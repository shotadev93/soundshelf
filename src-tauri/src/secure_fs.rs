//! Linux-only, descriptor-relative access. No path-based fallback is permitted.
#[cfg(not(target_os = "linux"))]
compile_error!("SoundShelf discovery currently requires Linux openat2 containment support.");

use crate::contracts::{AppError, ErrorCode};
use rustix::fs::{Mode, OFlags, ResolveFlags, open, openat2};
use std::os::unix::fs::MetadataExt;
use std::{
    fs::File,
    path::{Component, Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

pub(crate) struct AuthorizedRoot {
    path: PathBuf,
    directory: File,
    active: AtomicBool,
}

fn absolute_directory(path: &Path) -> Result<File, AppError> {
    if !path.is_absolute()
        || path
            .components()
            .any(|part| matches!(part, Component::ParentDir | Component::CurDir))
    {
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            "Select an absolute directory without traversal components.",
        ));
    }
    let anchor = open(
        "/",
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|_| AppError::unavailable("The filesystem root cannot be opened."))?;
    let relative = path
        .strip_prefix("/")
        .map_err(|_| AppError::unavailable("Invalid directory selection."))?;
    let relative = if relative.as_os_str().is_empty() {
        Path::new(".")
    } else {
        relative
    };
    openat2(anchor, relative, OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC | OFlags::NONBLOCK,
        Mode::empty(), ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS)
        .map(File::from)
        .map_err(|_| AppError::unavailable("Secure directory access failed. Select a readable directory without symlinks; Linux openat2 support is required."))
}

pub(crate) fn valid_relative(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && path
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
}

pub(crate) fn same_file(left: &File, right: &File) -> std::io::Result<bool> {
    let left = left.metadata()?;
    let right = right.metadata()?;
    Ok(left.dev() == right.dev() && left.ino() == right.ino())
}

impl AuthorizedRoot {
    pub(crate) fn open(path: &Path) -> Result<Self, AppError> {
        let directory = absolute_directory(path)?;
        // Resolve display/overlap identity only after the no-symlink open. Recheck
        // identity below; canonicalization is never the access-control mechanism.
        let canonical = path
            .canonicalize()
            .map_err(|_| AppError::unavailable("The selected directory disappeared."))?;
        if canonical != path {
            return Err(AppError::unavailable(
                "The selected directory changed during selection.",
            ));
        }
        let path = canonical;
        let root = Self {
            path,
            directory,
            active: AtomicBool::new(true),
        };
        root.validate()?;
        Ok(root)
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn validate(&self) -> Result<(), AppError> {
        let valid = self.active.load(Ordering::Acquire)
            && absolute_directory(&self.path)
                .and_then(|current| {
                    same_file(&self.directory, &current).map_err(|_| {
                        AppError::unavailable("Cannot validate the selected directory.")
                    })
                })
                .unwrap_or(false);
        if !valid {
            self.active.store(false, Ordering::Release);
            return Err(AppError::new(
                ErrorCode::NotAuthorized,
                "The selected directory is unavailable or was replaced. Select it again.",
            ));
        }
        Ok(())
    }

    pub(crate) fn open_relative(
        &self,
        path: &Path,
        flags: OFlags,
    ) -> Result<File, rustix::io::Errno> {
        let read_only_flags = OFlags::RDONLY | OFlags::PATH | OFlags::DIRECTORY;
        if !valid_relative(path) || !read_only_flags.contains(flags) {
            return Err(rustix::io::Errno::INVAL);
        }
        if self.validate().is_err() {
            return Err(rustix::io::Errno::ACCESS);
        }
        // Resolve every full relative path from the selected root, not a cached
        // child fd: a child may have been renamed outside the authorized tree.
        // Linux rejects NONBLOCK together with O_PATH. Metadata handles never
        // open device streams; actual read handles must remain nonblocking.
        let flags = if flags.contains(OFlags::PATH) {
            flags
        } else {
            flags | OFlags::NONBLOCK
        };
        let file = openat2(
            &self.directory,
            path,
            flags | OFlags::CLOEXEC,
            Mode::empty(),
            ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_XDEV,
        )?;
        Ok(File::from(file))
    }

    pub(crate) fn open_directory(&self, path: &Path) -> Result<File, rustix::io::Errno> {
        if path.as_os_str().is_empty() {
            if self.validate().is_err() {
                return Err(rustix::io::Errno::ACCESS);
            }
            // A new open description gives each traversal an independent offset.
            return openat2(
                &self.directory,
                ".",
                OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
                Mode::empty(),
                ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_XDEV,
            )
            .map(File::from);
        }
        self.open_relative(path, OFlags::RDONLY | OFlags::DIRECTORY)
    }
}
