//! Playback capabilities are bound to one authorized root generation and file identity.
use crate::{
    contracts::{AppError, ErrorCode, PlaybackRequest, PlaybackSource},
    library::{Library, LibraryState, lock, opaque_id},
    secure_fs::same_file,
};
use rustix::fs::OFlags;
use std::{
    collections::HashMap,
    fs::{File, Metadata},
    io::{Read, Seek, SeekFrom},
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, Weak,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

const PREFIX: &str = "soundshelf-audio://localhost/";
const TOKEN_LIFETIME: Duration = Duration::from_secs(3600);
const MAX_TOKENS: usize = 32;
pub(crate) const READ_CHUNK: usize = 64 * 1024;

#[derive(Default)]
pub(crate) struct PlaybackTokens(Mutex<HashMap<String, Arc<Grant>>>);

struct Grant {
    library: Weak<Library>,
    asset_id: String,
    path: PathBuf,
    mime: &'static str,
    identity: Identity,
    expires: Instant,
    revoked: AtomicBool,
}

#[derive(PartialEq)]
struct Identity {
    device: u64,
    inode: u64,
    size: u64,
    modified: (i64, i64),
    changed: (i64, i64),
}
impl Identity {
    fn of(metadata: &Metadata) -> Self {
        Self {
            device: metadata.dev(),
            inode: metadata.ino(),
            size: metadata.len(),
            modified: (metadata.mtime(), metadata.mtime_nsec()),
            changed: (metadata.ctime(), metadata.ctime_nsec()),
        }
    }
}

fn mime(extension: &str) -> Result<&'static str, AppError> {
    match extension.to_ascii_lowercase().as_str() {
        "wav" => Ok("audio/wav"),
        "mp3" => Ok("audio/mpeg"),
        "ogg" => Ok("audio/ogg"),
        _ => Err(AppError::unavailable(
            "This audio extension is unsupported.",
        )),
    }
}

fn path_mime(path: &Path) -> Result<&'static str, AppError> {
    mime(
        path.extension()
            .and_then(|extension| extension.to_str())
            .ok_or_else(unavailable)?,
    )
}

fn denied() -> AppError {
    AppError::new(
        ErrorCode::NotAuthorized,
        "Playback authorization expired. Select the directory again and retry playback.",
    )
}
fn unavailable() -> AppError {
    AppError::unavailable(
        "The audio file is empty, unavailable, or changed. Refresh the scan and retry playback.",
    )
}

// Inspect before read-open so devices/FIFOs are not opened as audio. Both opens
// resolve from the root; a replacement between them must retain the same inode.
fn open_audio(library: &Library, path: &Path) -> Result<(File, Identity), AppError> {
    open_audio_with_hook(library, path, || {})
}

fn open_audio_with_hook(
    library: &Library,
    path: &Path,
    before_read_open: impl FnOnce(),
) -> Result<(File, Identity), AppError> {
    let inspected = library
        .root
        .open_relative(path, OFlags::PATH)
        .map_err(|_| unavailable())?;
    let metadata = inspected.metadata().map_err(|_| unavailable())?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > i64::MAX as u64 {
        return Err(unavailable());
    }
    before_read_open();
    let file = library
        .root
        .open_relative(path, OFlags::RDONLY)
        .map_err(|_| unavailable())?;
    let metadata = file.metadata().map_err(|_| unavailable())?;
    if !same_file(&inspected, &file).map_err(|_| unavailable())?
        || !metadata.is_file()
        || metadata.len() == 0
        || metadata.len() > i64::MAX as u64
    {
        return Err(unavailable());
    }
    library.root.validate()?;
    Ok((file, Identity::of(&metadata)))
}

impl LibraryState {
    pub(crate) fn prepare_playback(
        &self,
        request: PlaybackRequest,
    ) -> Result<PlaybackSource, AppError> {
        let library = self.get(&request.library_id)?;
        let (path, extension) =
            lock(&self.database)?.playback_asset(&request.library_id, &request.asset_id)?;
        let mime = mime(&extension)?;
        if mime != path_mime(&path)? {
            return Err(unavailable());
        }
        let (_, identity) = open_audio(&library, &path)?;
        let now = Instant::now();
        let mut tokens = lock(&self.playback.0)?;
        tokens.retain(|_, grant| {
            grant.expires > now
                && grant
                    .library
                    .upgrade()
                    .is_some_and(|library| library.root.validate().is_ok())
        });
        if tokens.len() >= MAX_TOKENS
            && let Some(oldest) = tokens
                .iter()
                .min_by_key(|(_, grant)| grant.expires)
                .map(|(token, _)| token.clone())
            && let Some(grant) = tokens.remove(&oldest)
        {
            grant.revoked.store(true, Ordering::Release);
        }
        let token = opaque_id()?;
        tokens.insert(
            token.clone(),
            Arc::new(Grant {
                library: Arc::downgrade(&library),
                asset_id: request.asset_id,
                path,
                mime,
                identity,
                expires: now + TOKEN_LIFETIME,
                revoked: AtomicBool::new(false),
            }),
        );
        Ok(PlaybackSource {
            url: format!("{PREFIX}{token}"),
        })
    }

    pub(crate) fn audio_response(
        &self,
        uri: &str,
        method: &str,
        range: Option<&str>,
    ) -> AudioResponse {
        match self.open_response(uri, method, range) {
            Ok(response) => response,
            Err(error) => AudioResponse::empty(match error.code {
                ErrorCode::NotAuthorized => 403,
                ErrorCode::NotFound => 404,
                ErrorCode::InvalidInput => 400,
                ErrorCode::Busy => 503,
                _ => 410,
            }),
        }
    }

    fn open_response(
        &self,
        uri: &str,
        method: &str,
        range: Option<&str>,
    ) -> Result<AudioResponse, AppError> {
        if method != "GET" && method != "HEAD" {
            let mut response = AudioResponse::empty(405);
            response.headers.push(("Allow", "GET, HEAD".into()));
            return Ok(response);
        }
        let token = uri
            .strip_prefix(PREFIX)
            .filter(|token| token.len() == 32 && token.bytes().all(|b| b.is_ascii_hexdigit()))
            .ok_or_else(denied)?;
        let grant = lock(&self.playback.0)?
            .get(token)
            .cloned()
            .ok_or_else(denied)?;
        let library = grant.authorized()?;
        // Resolve IDs again; a stored path remains data, never authorization.
        let (path, extension) =
            lock(&self.database)?.playback_asset(&library.info.id, &grant.asset_id)?;
        if path != grant.path || mime(&extension)? != grant.mime || path_mime(&path)? != grant.mime
        {
            return Err(unavailable());
        }
        let (mut file, identity) = open_audio(&library, &path)?;
        if identity != grant.identity {
            return Err(unavailable());
        }
        let size = identity.size;
        let (start, length, partial) =
            match byte_range(if method == "GET" { range } else { None }, size) {
                Ok(value) => value,
                Err(()) => {
                    let mut response = AudioResponse::empty(416);
                    response
                        .headers
                        .push(("Content-Range", format!("bytes */{size}")));
                    return Ok(response);
                }
            };
        file.seek(SeekFrom::Start(start))
            .map_err(|_| unavailable())?;
        grant.authorized()?;
        let mut headers = vec![
            ("Content-Type", grant.mime.into()),
            ("Content-Length", length.to_string()),
            ("Accept-Ranges", "bytes".into()),
            ("Cache-Control", "no-store".into()),
            ("X-Content-Type-Options", "nosniff".into()),
        ];
        if partial {
            headers.push((
                "Content-Range",
                format!("bytes {start}-{}/{size}", start + length - 1),
            ));
        }
        Ok(AudioResponse {
            status: if partial { 206 } else { 200 },
            headers,
            length: if method == "HEAD" { 0 } else { length },
            stream: if method == "HEAD" {
                None
            } else {
                Some(AudioStream {
                    file,
                    grant,
                    remaining: length,
                })
            },
        })
    }
}

impl Grant {
    fn authorized(&self) -> Result<Arc<Library>, AppError> {
        if self.revoked.load(Ordering::Acquire) || Instant::now() >= self.expires {
            return Err(denied());
        }
        let library = self.library.upgrade().ok_or_else(denied)?;
        library.root.validate()?;
        Ok(library)
    }
}

pub(crate) struct AudioResponse {
    pub status: u32,
    pub headers: Vec<(&'static str, String)>,
    pub length: u64,
    pub stream: Option<AudioStream>,
}
impl AudioResponse {
    fn empty(status: u32) -> Self {
        Self {
            status,
            headers: vec![
                ("Content-Length", "0".into()),
                ("Accept-Ranges", "bytes".into()),
                ("Cache-Control", "no-store".into()),
            ],
            length: 0,
            stream: None,
        }
    }
}

pub(crate) struct AudioStream {
    file: File,
    grant: Arc<Grant>,
    remaining: u64,
}
impl Read for AudioStream {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        if buffer.is_empty() || self.remaining == 0 {
            return Ok(0);
        }
        self.grant.authorized().map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "Playback authorization expired.",
            )
        })?;
        let count = buffer
            .len()
            .min(READ_CHUNK)
            .min(self.remaining.min(READ_CHUNK as u64) as usize);
        let read = self.file.read(&mut buffer[..count])?;
        if read == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "Audio changed during playback.",
            ));
        }
        self.remaining -= read as u64;
        Ok(read)
    }
}

// A single range is supported. Multiple ranges are deliberately rejected with
// 416 instead of constructing multipart bodies. No read size depends on input.
fn byte_range(range: Option<&str>, size: u64) -> Result<(u64, u64, bool), ()> {
    let Some(range) = range else {
        return Ok((0, size, false));
    };
    let range = range.strip_prefix("bytes=").ok_or(())?;
    let (start, end) = range.split_once('-').ok_or(())?;
    let number = |text: &str| -> Result<u64, ()> {
        if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(());
        }
        text.parse().map_err(|_| ())
    };
    if size == 0 {
        return Err(());
    }
    if start.is_empty() {
        let length = number(end)?.min(size);
        if length == 0 {
            return Err(());
        }
        return Ok((size - length, length, true));
    }
    let start = number(start)?;
    let end = if end.is_empty() {
        size - 1
    } else {
        number(end)?.min(size - 1)
    };
    if start >= size || end < start {
        return Err(());
    }
    Ok((start, end - start + 1, true))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ranges_cover_full_open_suffix_and_invalid_inputs() {
        for (range, expected) in [
            (None, (0, 10, false)),
            (Some("bytes=2-5"), (2, 4, true)),
            (Some("bytes=4-"), (4, 6, true)),
            (Some("bytes=-3"), (7, 3, true)),
            (Some("bytes=-99"), (0, 10, true)),
            (Some("bytes=0-99"), (0, 10, true)),
        ] {
            assert_eq!(byte_range(range, 10), Ok(expected));
        }
        for range in [
            "bytes=",
            "bytes=-0",
            "bytes=10-",
            "bytes=4-3",
            "bytes=0-1,4-5",
            "items=0-1",
            "bytes=+1-2",
            "bytes=1-18446744073709551616",
            "bytes=../secret",
            "bytes= 0-1",
        ] {
            assert!(byte_range(Some(range), 10).is_err(), "{range}");
        }
        assert!(byte_range(Some("bytes=0-"), 0).is_err());
    }
}

#[cfg(test)]
#[path = "playback_tests.rs"]
pub(crate) mod playback_tests;
