use super::*;
use crate::support::Fixture;
use crate::{
    contracts::{AssetQuery, LibraryInfo},
    scan,
};
use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
};

pub(crate) fn fixture() -> (Fixture, LibraryState, LibraryInfo, PlaybackRequest) {
    let fixture = Fixture::new();
    let root = fixture.path().join("audio");
    fs::create_dir_all(root.join("nested")).unwrap();
    // Synthetic PCM WAV: 1 channel, 8 kHz, 16 bits, 1 second of silence.
    let mut audio = Vec::from(&b"RIFF"[..]);
    audio.extend(16036u32.to_le_bytes());
    audio.extend(b"WAVEfmt ");
    audio.extend(16u32.to_le_bytes());
    audio.extend(1u16.to_le_bytes());
    audio.extend(1u16.to_le_bytes());
    audio.extend(8000u32.to_le_bytes());
    audio.extend(16000u32.to_le_bytes());
    audio.extend(2u16.to_le_bytes());
    audio.extend(16u16.to_le_bytes());
    audio.extend(b"data");
    audio.extend(16000u32.to_le_bytes());
    audio.resize(16044, 0);
    fs::write(root.join("nested/track.wav"), audio).unwrap();
    let state = LibraryState::new(
        crate::db::Database::open(&fixture.path().join("index.sqlite3")).unwrap(),
    );
    let info = state.authorize_selection(Some(root)).unwrap().unwrap();
    let library = state.get(&info.id).unwrap();
    scan::begin(&library).unwrap();
    scan::execute(&library).unwrap();
    let page = lock(&state.database)
        .unwrap()
        .query(&AssetQuery {
            library_id: info.id.clone(),
            search: String::new(),
            offset: 0,
            limit: 10,
        })
        .unwrap();
    let request = PlaybackRequest {
        library_id: info.id.clone(),
        asset_id: page.assets[0].id.clone(),
    };
    (fixture, state, info, request)
}
fn header<'a>(response: &'a AudioResponse, name: &str) -> &'a str {
    &response
        .headers
        .iter()
        .find(|(key, _)| *key == name)
        .unwrap()
        .1
}
fn bytes(mut response: AudioResponse) -> Vec<u8> {
    let mut bytes = Vec::new();
    response
        .stream
        .as_mut()
        .unwrap()
        .read_to_end(&mut bytes)
        .unwrap();
    bytes
}

#[test]
fn playback_headers_ranges_head_and_integrity() {
    let (fixture, state, _, request) = fixture();
    let path = fixture.path().join("audio/nested/track.wav");
    let original = fs::read(&path).unwrap();
    let before = fs::metadata(&path).unwrap();
    let url = state.prepare_playback(request).unwrap().url;
    assert!(!url.contains("track"));
    let full = state.audio_response(&url, "GET", None);
    assert_eq!(full.status, 200);
    assert_eq!(header(&full, "Content-Type"), "audio/wav");
    assert_eq!(header(&full, "Content-Length"), "16044");
    assert_eq!(header(&full, "Accept-Ranges"), "bytes");
    assert_eq!(header(&full, "Cache-Control"), "no-store");
    assert_eq!(bytes(full), original);
    for (range, start, end) in [
        ("bytes=0-3", 0, 4),
        ("bytes=16000-", 16000, 16044),
        ("bytes=-44", 16000, 16044),
        ("bytes=16000-99999", 16000, 16044),
    ] {
        let response = state.audio_response(&url, "GET", Some(range));
        assert_eq!(response.status, 206);
        assert_eq!(
            header(&response, "Content-Length"),
            (end - start).to_string()
        );
        assert_eq!(
            header(&response, "Content-Range"),
            format!("bytes {start}-{}/16044", end - 1)
        );
        assert_eq!(bytes(response), original[start..end]);
    }
    for range in [
        "bytes=16044-",
        "bytes=1-0",
        "bytes=-0",
        "bytes=0-1,2-3",
        "bytes=x-y",
    ] {
        let response = state.audio_response(&url, "GET", Some(range));
        assert_eq!(response.status, 416);
        assert_eq!(header(&response, "Content-Range"), "bytes */16044");
        assert_eq!(header(&response, "Content-Length"), "0");
        assert!(response.stream.is_none());
    }
    let head = state.audio_response(&url, "HEAD", None);
    assert_eq!(head.status, 200);
    assert_eq!(header(&head, "Content-Length"), "16044");
    assert!(head.stream.is_none());
    assert_eq!(state.audio_response(&url, "POST", None).status, 405);
    let after = fs::metadata(&path).unwrap();
    assert_eq!(original, fs::read(&path).unwrap());
    assert_eq!(
        (before.mode(), before.mtime(), before.mtime_nsec()),
        (after.mode(), after.mtime(), after.mtime_nsec())
    );
    assert_eq!(
        fs::read_dir(fixture.path().join("audio/nested"))
            .unwrap()
            .count(),
        1
    );
}

#[test]
fn tokens_are_opaque_expire_and_do_not_survive_restart() {
    let (fixture, state, _, request) = fixture();
    for uri in [
        "soundshelf-audio://localhost/../../etc/passwd",
        "soundshelf-audio://other/00000000000000000000000000000000",
        "soundshelf-audio://localhost/%2e%2e",
        "file:///etc/passwd",
        "soundshelf-audio://localhost/00000000000000000000000000000000",
    ] {
        assert_eq!(state.audio_response(uri, "GET", None).status, 403);
    }
    let url = state.prepare_playback(request).unwrap().url;
    {
        let mut tokens = lock(&state.playback.0).unwrap();
        let grant = Arc::get_mut(tokens.values_mut().next().unwrap()).unwrap();
        grant.expires = Instant::now() - Duration::from_secs(1);
    }
    assert_eq!(state.audio_response(&url, "GET", None).status, 403);
    let restarted = LibraryState::new(
        crate::db::Database::open(&fixture.path().join("index.sqlite3")).unwrap(),
    );
    assert_eq!(restarted.audio_response(&url, "GET", None).status, 403);
}

#[test]
fn roots_revoke_existing_tokens_streams_and_old_generation_after_reselection() {
    let (fixture, state, info, request) = fixture();
    let url = state.prepare_playback(request.clone()).unwrap().url;
    let mut stream = state.audio_response(&url, "GET", None).stream.unwrap();
    let root = fixture.path().join("audio");
    fs::rename(&root, fixture.path().join("old-root")).unwrap();
    assert_eq!(state.audio_response(&url, "GET", None).status, 403);
    assert!(stream.read(&mut [0; 10]).is_err());
    fs::rename(fixture.path().join("old-root"), &root).unwrap();
    let selected = state.authorize_selection(Some(root)).unwrap().unwrap();
    assert_eq!(selected.id, info.id);
    assert_eq!(state.audio_response(&url, "GET", None).status, 403);
    assert!(state.prepare_playback(request).is_ok());
}

#[test]
fn missing_empty_unreadable_replaced_and_symlink_files_fail_closed() {
    for attack in [
        "missing",
        "empty",
        "unreadable",
        "file-link",
        "directory-link",
        "replacement",
        "fifo",
    ] {
        let (fixture, state, _, request) = fixture();
        let path = fixture.path().join("audio/nested/track.wav");
        let url = state.prepare_playback(request).unwrap().url;
        fs::write(fixture.path().join("secret.wav"), b"secret outside root").unwrap();
        match attack {
            "missing" => fs::remove_file(&path).unwrap(),
            "empty" => fs::write(&path, b"").unwrap(),
            "unreadable" => {
                fs::set_permissions(&path, fs::Permissions::from_mode(0o0)).unwrap();
                assert!(File::open(&path).is_err(), "run without root privileges");
            }
            "file-link" => {
                fs::remove_file(&path).unwrap();
                symlink(fixture.path().join("secret.wav"), &path).unwrap();
            }
            "directory-link" => {
                fs::rename(
                    fixture.path().join("audio/nested"),
                    fixture.path().join("outside"),
                )
                .unwrap();
                symlink(
                    fixture.path().join("outside"),
                    fixture.path().join("audio/nested"),
                )
                .unwrap();
            }
            "replacement" => {
                fs::rename(&path, fixture.path().join("original.wav")).unwrap();
                fs::write(&path, b"different inode").unwrap();
            }
            "fifo" => {
                fs::remove_file(&path).unwrap();
                rustix::fs::mkfifoat(rustix::fs::CWD, &path, rustix::fs::Mode::RUSR).unwrap();
            }
            _ => unreachable!(),
        }
        let response = state.audio_response(&url, "GET", None);
        assert_eq!(response.status, 410, "{attack}");
        assert!(response.stream.is_none());
    }
}

#[test]
fn database_paths_and_wrong_library_ids_cannot_grant_access() {
    let (fixture, state, _, request) = fixture();
    let mut wrong = request.clone();
    wrong.library_id = "untrusted".into();
    assert_eq!(
        state.prepare_playback(wrong).unwrap_err().code,
        ErrorCode::NotAuthorized
    );
    let mut wrong = request.clone();
    wrong.asset_id = "' OR 1=1 --".into();
    assert_eq!(
        state.prepare_playback(wrong).unwrap_err().code,
        ErrorCode::NotFound
    );
    let connection = rusqlite::Connection::open(fixture.path().join("index.sqlite3")).unwrap();
    for path in [
        b"../secret.wav".as_slice(),
        b"/etc/passwd".as_slice(),
        b"nested/../../secret.wav".as_slice(),
    ] {
        connection
            .execute(
                "UPDATE assets SET relative_path=?1 WHERE id=?2",
                rusqlite::params![path, request.asset_id],
            )
            .unwrap();
        assert!(state.prepare_playback(request.clone()).is_err());
    }
    connection
        .execute(
            "UPDATE assets SET relative_path=?1,extension='txt' WHERE id=?2",
            rusqlite::params![b"nested/track.wav".as_slice(), request.asset_id],
        )
        .unwrap();
    assert!(state.prepare_playback(request).is_err());
}

#[test]
fn large_streams_are_read_in_bounded_chunks_and_tokens_are_bounded() {
    let (fixture, state, _, request) = fixture();
    let path = fixture.path().join("audio/nested/track.wav");
    // A sparse synthetic file verifies that response construction never reads it.
    fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .unwrap()
        .set_len(8 * 1024 * 1024 * 1024)
        .unwrap();
    let url = state.prepare_playback(request.clone()).unwrap().url;
    let mut response = state.audio_response(&url, "GET", None);
    assert_eq!(response.length, 8 * 1024 * 1024 * 1024);
    let mut buffer = vec![0; READ_CHUNK * 2];
    let stream = response.stream.as_mut().unwrap();
    assert_eq!(stream.read(&mut buffer).unwrap(), READ_CHUNK);
    for _ in 0..MAX_TOKENS {
        state.prepare_playback(request.clone()).unwrap();
    }
    assert_eq!(lock(&state.playback.0).unwrap().len(), MAX_TOKENS);
    assert_eq!(state.audio_response(&url, "GET", None).status, 403);
    assert!(stream.read(&mut buffer).is_err());
}

#[test]
fn media_types_and_corruption_are_left_to_the_decoder() {
    assert_eq!(mime("MP3").unwrap(), "audio/mpeg");
    assert_eq!(mime("OGG").unwrap(), "audio/ogg");
    assert!(mime("exe").is_err());
    let (fixture, state, _, request) = fixture();
    fs::write(
        fixture.path().join("audio/nested/track.wav"),
        b"corrupt synthetic candidate",
    )
    .unwrap();
    let url = state.prepare_playback(request).unwrap().url;
    assert_eq!(state.audio_response(&url, "GET", None).status, 200);
}

#[test]
fn playback_ipc_contract_matches_typescript_and_rejects_paths() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("../../src/lib/fixtures/ipc.json")).unwrap();
    let request: PlaybackRequest =
        serde_json::from_value(fixture["playbackRequest"].clone()).unwrap();
    assert_eq!(
        serde_json::to_value(request).unwrap(),
        fixture["playbackRequest"]
    );
    let source = PlaybackSource {
        url: "http://127.0.0.1:43123/audio/0123456789abcdef0123456789abcdef".into(),
    };
    assert_eq!(
        serde_json::to_value(source).unwrap(),
        fixture["playbackSource"]
    );
    assert!(
        serde_json::from_value::<PlaybackRequest>(
            serde_json::json!({"libraryId":"id","assetId":"id","path":"/etc/passwd"})
        )
        .is_err()
    );
}

#[test]
fn symlink_and_inode_replacements_between_inspection_and_read_are_rejected() {
    for attack in ["file-link", "directory-link", "inode", "root"] {
        let (fixture, state, info, _) = fixture();
        let library = state.get(&info.id).unwrap();
        let root = fixture.path().join("audio");
        let path = root.join("nested/track.wav");
        fs::write(fixture.path().join("secret.wav"), b"outside secret").unwrap();
        let result =
            open_audio_with_hook(&library, Path::new("nested/track.wav"), || match attack {
                "file-link" => {
                    fs::rename(&path, fixture.path().join("original.wav")).unwrap();
                    symlink(fixture.path().join("secret.wav"), &path).unwrap();
                }
                "directory-link" => {
                    fs::rename(root.join("nested"), fixture.path().join("outside")).unwrap();
                    symlink(fixture.path().join("outside"), root.join("nested")).unwrap();
                }
                "inode" => {
                    fs::rename(&path, fixture.path().join("original.wav")).unwrap();
                    fs::write(&path, b"new inode").unwrap();
                }
                "root" => {
                    fs::rename(&root, fixture.path().join("old-root")).unwrap();
                    fs::create_dir_all(root.join("nested")).unwrap();
                    fs::write(&path, b"new root").unwrap();
                }
                _ => unreachable!(),
            });
        assert!(result.is_err(), "{attack}");
    }
}

#[test]
fn gio_adapter_reads_the_same_bounded_stream_without_buffering_the_response() {
    use gtk::gio::prelude::InputStreamExtManual;
    let (fixture, state, _, request) = fixture();
    fs::OpenOptions::new()
        .write(true)
        .open(fixture.path().join("audio/nested/track.wav"))
        .unwrap()
        .set_len(2 * READ_CHUNK as u64 + 5)
        .unwrap();
    let url = state.prepare_playback(request).unwrap().url;
    let response = state.audio_response(&url, "GET", Some("bytes=0-"));
    let reader = gtk::gio::ReadInputStream::new(response.stream.unwrap());
    let mut buffer = vec![0u8; READ_CHUNK * 2];
    assert_eq!(
        reader
            .read(&mut buffer, gtk::gio::Cancellable::NONE)
            .unwrap(),
        READ_CHUNK
    );
    assert_eq!(
        reader
            .read(&mut buffer, gtk::gio::Cancellable::NONE)
            .unwrap(),
        READ_CHUNK
    );
    assert_eq!(
        reader
            .read(&mut buffer, gtk::gio::Cancellable::NONE)
            .unwrap(),
        5
    );
    assert_eq!(
        reader
            .read(&mut buffer, gtk::gio::Cancellable::NONE)
            .unwrap(),
        0
    );
}
