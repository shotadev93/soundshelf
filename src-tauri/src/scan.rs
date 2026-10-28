//! Read-only, bounded discovery. No filesystem paths are accepted over IPC.
use crate::{
    contracts::{
        AppError, AssetList, AudioAsset, Availability, ErrorCode, ScanPhase, ScanStatus,
        ScanWarning,
    },
    library::{Library, lock, opaque_id},
    secure_fs::{AuthorizedRoot, same_file},
};
use rustix::{
    fs::{Dir, FileType, OFlags},
    io::Errno,
};
use std::{
    collections::HashMap,
    ffi::OsStr,
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

const MAX_DEPTH: usize = 64;
const MAX_ENTRIES: u32 = 100_000;
const MAX_ASSETS: usize = 10_000;
const MAX_WARNINGS: usize = 100;
const MAX_DURATION: Duration = Duration::from_secs(30);

pub(crate) struct StoredAsset {
    pub relative_path: PathBuf,
    pub asset: AudioAsset,
}

#[derive(Default)]
pub(crate) struct ScanData {
    pub status: ScanStatus,
    pub assets: Vec<StoredAsset>,
}

pub(crate) fn begin(library: &Library) -> Result<ScanStatus, AppError> {
    library.root.validate()?;
    let mut data = lock(&library.scan)?;
    if data.status.phase == ScanPhase::Running {
        return Err(AppError::new(
            ErrorCode::Busy,
            "A scan is already running for this library.",
        ));
    }
    data.status = ScanStatus {
        scan_id: Some(opaque_id()?),
        phase: ScanPhase::Running,
        ..ScanStatus::default()
    };
    Ok(data.status.clone())
}

pub(crate) fn status(library: &Library) -> Result<ScanStatus, AppError> {
    library.root.validate()?;
    Ok(lock(&library.scan)?.status.clone())
}

pub(crate) fn assets(library: &Library) -> Result<AssetList, AppError> {
    library.root.validate()?;
    let data = lock(&library.scan)?;
    if data.status.phase == ScanPhase::Running {
        return Err(AppError::new(
            ErrorCode::Busy,
            "Wait for the scan to finish before loading its results.",
        ));
    }
    Ok(AssetList {
        scan_id: data.status.scan_id.clone(),
        assets: data
            .assets
            .iter()
            .map(|entry| entry.asset.clone())
            .collect(),
    })
}

pub(crate) fn fail(library: &Library, error: AppError) {
    if let Ok(mut data) = lock(&library.scan) {
        data.status.phase = ScanPhase::Failed;
        data.status.error = Some(error);
        if library.root.validate().is_err() {
            data.assets.clear();
        }
    }
}

pub(crate) fn extension(path: &Path) -> Option<String> {
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();
    matches!(extension.as_str(), "wav" | "mp3" | "ogg").then_some(extension)
}

// Internal synchronization seam for deterministic race tests, never exposed via IPC.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum ScanPoint {
    Inspect,
    DirectoryOpen,
    ReadOpen,
    Finish,
}

struct Discovery<'a, F: FnMut(&Path, ScanPoint)> {
    root: &'a AuthorizedRoot,
    library: &'a Library,
    previous: HashMap<PathBuf, String>,
    result: Vec<StoredAsset>,
    status: ScanStatus,
    started: Instant,
    before_access: F,
    stopped: bool,
}

impl<F: FnMut(&Path, ScanPoint)> Discovery<'_, F> {
    fn warning(&mut self, path: Option<&Path>, message: &str) {
        self.status.warning_count = self.status.warning_count.saturating_add(1);
        if self.status.warnings.len() < MAX_WARNINGS {
            self.status.warnings.push(ScanWarning {
                message: message.into(),
                relative_display_path: path.map(|path| path.to_string_lossy().into_owned()),
            });
        }
    }

    fn walk(&mut self, path: &Path, depth: usize) -> Result<(), AppError> {
        if depth > MAX_DEPTH {
            self.warning(Some(path), "The directory depth limit was reached.");
            return Ok(());
        }
        self.root.validate()?;
        let directory = match self.root.open_directory(path).and_then(Dir::new) {
            Ok(directory) => directory,
            Err(_) => {
                self.warning(Some(path), "This directory could not be read securely.");
                return Ok(());
            }
        };
        for entry in directory {
            if self.stopped {
                break;
            }
            if self.started.elapsed() >= MAX_DURATION
                || self.status.visited_entries >= MAX_ENTRIES
                || self.result.len() >= MAX_ASSETS
            {
                self.warning(None, "A scan resource limit was reached. Results are partial; select a smaller directory.");
                self.stopped = true;
                break;
            }
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => {
                    self.warning(Some(path), "A directory entry could not be read.");
                    break;
                }
            };
            let bytes = entry.file_name().to_bytes();
            if bytes == b"." || bytes == b".." {
                continue;
            }
            self.status.visited_entries += 1;
            let relative = path.join(OsStr::from_bytes(bytes));
            // openat2 also rejects unknown types and links replaced after enumeration.
            if entry.file_type() != FileType::Symlink {
                self.inspect(&relative, depth)?;
            }
            if self.status.visited_entries.is_multiple_of(32) {
                self.root.validate()?;
                lock(&self.library.scan)?.status = self.status.clone();
            }
        }
        Ok(())
    }

    fn inspect(&mut self, path: &Path, depth: usize) -> Result<(), AppError> {
        (self.before_access)(path, ScanPoint::Inspect);
        let node = match self.root.open_relative(path, OFlags::PATH) {
            Ok(node) => node,
            Err(_) => {
                self.warning(
                    Some(path),
                    "The entry disappeared, became inaccessible, or failed secure path validation.",
                );
                return Ok(());
            }
        };
        let metadata = match node.metadata() {
            Ok(metadata) => metadata,
            Err(_) => {
                self.warning(Some(path), "The entry metadata is unavailable.");
                return Ok(());
            }
        };
        if metadata.is_dir() {
            (self.before_access)(path, ScanPoint::DirectoryOpen);
            match self.root.open_directory(path) {
                Ok(directory) if same_file(&node, &directory).unwrap_or(false) => {
                    // walk reopens from the root, not through a potentially moved child fd.
                    self.walk(path, depth + 1)?;
                }
                _ => self.warning(
                    Some(path),
                    "The directory changed or could not be opened securely.",
                ),
            }
        } else if metadata.is_file() {
            let Some(extension) = extension(path) else {
                return Ok(());
            };
            (self.before_access)(path, ScanPoint::ReadOpen);
            let availability = match self.root.open_relative(path, OFlags::RDONLY) {
                Ok(file)
                    if same_file(&node, &file).unwrap_or(false)
                        && file.metadata().is_ok_and(|metadata| metadata.is_file()) =>
                {
                    Availability::Available
                }
                Err(Errno::ACCESS | Errno::PERM) => {
                    self.warning(Some(path), "The audio candidate cannot be read.");
                    Availability::Inaccessible
                }
                _ => {
                    self.warning(
                        Some(path),
                        "The audio candidate changed or failed secure path validation.",
                    );
                    return Ok(());
                }
            };
            let id = match self.previous.get(path) {
                Some(id) => id.clone(),
                None => opaque_id()?,
            };
            self.result.push(StoredAsset {
                relative_path: path.to_path_buf(),
                asset: AudioAsset {
                    id,
                    name: path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned(),
                    relative_display_path: path.to_string_lossy().into_owned(),
                    duration_seconds: None,
                    extension,
                    availability,
                },
            });
            self.status.discovered_assets = self.result.len() as u32;
        }
        Ok(())
    }
}

pub(crate) fn execute(library: &Library) -> Result<(), AppError> {
    execute_with_hook(library, |_, _| {})
}

pub(crate) fn execute_with_hook(
    library: &Library,
    before_access: impl FnMut(&Path, ScanPoint),
) -> Result<(), AppError> {
    let (status, previous) = {
        let data = lock(&library.scan)?;
        if data.status.phase != ScanPhase::Running {
            return Err(AppError::new(
                ErrorCode::InvalidInput,
                "Start a scan before running discovery.",
            ));
        }
        (
            data.status.clone(),
            data.assets
                .iter()
                .map(|entry| (entry.relative_path.clone(), entry.asset.id.clone()))
                .collect(),
        )
    };
    let mut discovery = Discovery {
        root: &library.root,
        library,
        previous,
        result: Vec::new(),
        status,
        started: Instant::now(),
        before_access,
        stopped: false,
    };
    discovery.walk(Path::new(""), 0)?;
    (discovery.before_access)(Path::new(""), ScanPoint::Finish);
    library.root.validate()?;
    discovery
        .result
        .sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    discovery.status.phase = if discovery.status.warning_count == 0 {
        ScanPhase::Complete
    } else {
        ScanPhase::Partial
    };
    let mut data = lock(&library.scan)?;
    data.assets = discovery.result;
    data.status = discovery.status;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extensions_are_candidates_not_codec_validation() {
        for path in ["x.wav", "x.WaV", "folder/x.MP3", "x.ogg"] {
            assert!(extension(Path::new(path)).is_some());
        }
        for path in ["wav", "x.wav.txt", "x.flac", "x"] {
            assert!(extension(Path::new(path)).is_none());
        }
    }
}
