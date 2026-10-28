// Compile the internal modules in this test crate rather than exposing a
// path-authorization API from the application library for testing.
#[path = "../src/contracts.rs"]
mod contracts;
#[path = "../src/library.rs"]
mod library;
#[path = "../src/scan.rs"]
mod scan;
#[path = "../src/secure_fs.rs"]
mod secure_fs;
mod support;

use contracts::ErrorCode;
use library::LibraryState;
use std::{fs, os::unix::fs::symlink};
use support::Fixture;

#[test]
fn selection_cancellation_overlap_and_untrusted_ids() {
    let fixture = Fixture::new();
    let root = fixture.path().join("audio");
    fs::create_dir(&root).unwrap();
    fs::create_dir(root.join("nested")).unwrap();
    fs::create_dir(fixture.path().join("audio-other")).unwrap();
    let state = LibraryState::default();
    assert!(!state.picker_open.load(std::sync::atomic::Ordering::Acquire));
    assert_eq!(
        state.get("/etc").err().unwrap().code,
        ErrorCode::NotAuthorized
    );
    let info = state
        .authorize_selection(Some(root.clone()))
        .unwrap()
        .unwrap();
    assert_eq!(info.id.len(), 32);
    assert!(!info.id.contains("audio"));
    assert_eq!(state.authorize_selection(None).unwrap(), None);
    assert!(state.get(&info.id).is_ok());
    assert_eq!(
        state.authorize_selection(Some(root.clone())).unwrap(),
        Some(info.clone())
    );
    assert_eq!(
        state
            .authorize_selection(Some(root.join("nested")))
            .unwrap_err()
            .code,
        ErrorCode::InvalidInput
    );
    assert!(
        state
            .authorize_selection(Some(fixture.path().join("audio-other")))
            .is_ok()
    );
    assert!(LibraryState::default().get(&info.id).is_err());
}

#[test]
fn discovers_nested_candidates_and_keeps_ids_on_repeat_scans() {
    let fixture = Fixture::new();
    fs::create_dir(fixture.path().join("nested")).unwrap();
    for name in [
        "a.WaV",
        "nested/b.MP3",
        "nested/c.oGg",
        "ignored.flac",
        "not.wav.txt",
    ] {
        fs::write(fixture.path().join(name), b"synthetic audio candidate").unwrap();
    }
    symlink(
        fixture.path().join("a.WaV"),
        fixture.path().join("link.wav"),
    )
    .unwrap();
    symlink(fixture.path(), fixture.path().join("cycle")).unwrap();
    let state = LibraryState::default();
    let info = state
        .authorize_selection(Some(fixture.path().to_path_buf()))
        .unwrap()
        .unwrap();
    let library = state.get(&info.id).unwrap();
    assert_eq!(
        scan::begin(&library).unwrap().phase,
        contracts::ScanPhase::Running
    );
    assert_eq!(scan::begin(&library).unwrap_err().code, ErrorCode::Busy);
    assert_eq!(scan::assets(&library).unwrap_err().code, ErrorCode::Busy);
    scan::execute(&library).unwrap();
    let first = scan::assets(&library).unwrap();
    assert_eq!(first.assets.len(), 3);
    assert_eq!(
        first
            .assets
            .iter()
            .map(|asset| asset.extension.as_str())
            .collect::<Vec<_>>(),
        ["wav", "mp3", "ogg"]
    );
    assert_eq!(
        scan::status(&library).unwrap().phase,
        contracts::ScanPhase::Complete
    );
    scan::begin(&library).unwrap();
    scan::execute(&library).unwrap();
    let second = scan::assets(&library).unwrap();
    assert_ne!(first.scan_id, second.scan_id);
    assert_eq!(first.assets, second.assets);
}

#[test]
fn replaced_root_revokes_old_authorization_permanently() {
    let fixture = Fixture::new();
    let root = fixture.path().join("audio");
    fs::create_dir(&root).unwrap();
    let state = LibraryState::default();
    let info = state
        .authorize_selection(Some(root.clone()))
        .unwrap()
        .unwrap();
    fs::rename(&root, fixture.path().join("old")).unwrap();
    fs::create_dir(&root).unwrap();
    assert_eq!(
        state.get(&info.id).err().unwrap().code,
        ErrorCode::NotAuthorized
    );
    fs::remove_dir(&root).unwrap();
    fs::rename(fixture.path().join("old"), &root).unwrap();
    assert!(state.get(&info.id).is_err());
    let replacement = state.authorize_selection(Some(root)).unwrap().unwrap();
    assert_ne!(replacement.id, info.id);
}

#[test]
fn picker_selection_rejects_symlinks_and_missing_paths() {
    let fixture = Fixture::new();
    let root = fixture.path().join("audio");
    fs::create_dir(&root).unwrap();
    symlink(&root, fixture.path().join("link")).unwrap();
    let state = LibraryState::default();
    assert!(
        state
            .authorize_selection(Some(fixture.path().join("link")))
            .is_err()
    );
    assert!(
        state
            .authorize_selection(Some(fixture.path().join("absent")))
            .is_err()
    );
    assert!(
        state
            .authorize_selection(Some(root.join("../audio")))
            .is_err()
    );
}

fn selected_library(fixture: &Fixture) -> (LibraryState, std::sync::Arc<library::Library>) {
    let state = LibraryState::default();
    let info = state
        .authorize_selection(Some(fixture.path().join("audio")))
        .unwrap()
        .unwrap();
    let library = state.get(&info.id).unwrap();
    (state, library)
}

#[test]
fn traversal_prefix_collisions_and_static_symlinks_cannot_escape() {
    use rustix::fs::OFlags;
    use std::path::Path;
    let fixture = Fixture::new();
    let root = fixture.path().join("audio");
    let outside = fixture.path().join("audio-other");
    fs::create_dir(&root).unwrap();
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join("private.wav"), b"outside").unwrap();
    symlink(&outside, root.join("link-dir")).unwrap();
    symlink(outside.join("private.wav"), root.join("link-file.wav")).unwrap();
    fs::write(root.join("inside.wav"), b"inside").unwrap();
    symlink(root.join("inside.wav"), root.join("internal-link.wav")).unwrap();
    let (_state, library) = selected_library(&fixture);
    for path in [
        "../audio-other/private.wav",
        "/etc/passwd",
        "link-dir/private.wav",
        "link-file.wav",
        "internal-link.wav",
        "",
        ".",
        "inside.wav/../inside.wav",
    ] {
        assert!(
            library
                .root
                .open_relative(Path::new(path), OFlags::RDONLY)
                .is_err(),
            "accepted {path}"
        );
    }
    scan::begin(&library).unwrap();
    scan::execute(&library).unwrap();
    let assets = scan::assets(&library).unwrap().assets;
    assert_eq!(assets.len(), 1);
    assert_eq!(assets[0].name, "inside.wav");
    assert_eq!(fs::read(outside.join("private.wav")).unwrap(), b"outside");
}

#[test]
fn file_symlink_replacement_races_are_rejected_at_both_open_boundaries() {
    use scan::ScanPoint;
    use std::path::Path;
    for boundary in [ScanPoint::Inspect, ScanPoint::ReadOpen] {
        let fixture = Fixture::new();
        let root = fixture.path().join("audio");
        fs::create_dir(&root).unwrap();
        fs::write(root.join("target.wav"), b"inside").unwrap();
        let secret = fixture.path().join("secret.wav");
        fs::write(&secret, b"outside").unwrap();
        let (_state, library) = selected_library(&fixture);
        scan::begin(&library).unwrap();
        let mut replaced = false;
        scan::execute_with_hook(&library, |path, point| {
            if path == Path::new("target.wav") && point == boundary {
                fs::rename(root.join(path), fixture.path().join("original.wav")).unwrap();
                symlink(&secret, root.join(path)).unwrap();
                replaced = true;
            }
        })
        .unwrap();
        assert!(replaced);
        assert!(scan::assets(&library).unwrap().assets.is_empty());
        assert_eq!(
            scan::status(&library).unwrap().phase,
            contracts::ScanPhase::Partial
        );
        assert_eq!(fs::read(&secret).unwrap(), b"outside");
    }
}

#[test]
fn directory_symlink_replacement_after_inspection_is_rejected() {
    use scan::ScanPoint;
    use std::path::Path;
    let fixture = Fixture::new();
    let root = fixture.path().join("audio");
    fs::create_dir_all(root.join("nested")).unwrap();
    let outside = fixture.path().join("outside");
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join("private.ogg"), b"outside").unwrap();
    let (_state, library) = selected_library(&fixture);
    scan::begin(&library).unwrap();
    let mut replaced = false;
    scan::execute_with_hook(&library, |path, point| {
        if path == Path::new("nested") && point == ScanPoint::DirectoryOpen {
            fs::rename(root.join(path), fixture.path().join("old-nested")).unwrap();
            symlink(&outside, root.join(path)).unwrap();
            replaced = true;
        }
    })
    .unwrap();
    assert!(replaced);
    assert!(scan::assets(&library).unwrap().assets.is_empty());
    assert_eq!(
        scan::status(&library).unwrap().phase,
        contracts::ScanPhase::Partial
    );
}

#[test]
fn a_directory_moved_outside_after_open_cannot_be_used_as_a_traversal_anchor() {
    use scan::ScanPoint;
    use std::path::Path;
    let fixture = Fixture::new();
    let root = fixture.path().join("audio");
    fs::create_dir_all(root.join("nested")).unwrap();
    fs::write(root.join("nested/track.mp3"), b"candidate").unwrap();
    let (_state, library) = selected_library(&fixture);
    scan::begin(&library).unwrap();
    let mut moved = false;
    scan::execute_with_hook(&library, |path, point| {
        if path == Path::new("nested/track.mp3") && point == ScanPoint::Inspect {
            fs::rename(root.join("nested"), fixture.path().join("moved-outside")).unwrap();
            moved = true;
        }
    })
    .unwrap();
    assert!(moved);
    assert!(scan::assets(&library).unwrap().assets.is_empty());
    assert_eq!(
        scan::status(&library).unwrap().phase,
        contracts::ScanPhase::Partial
    );
}

#[test]
fn root_replacement_before_publication_revokes_results() {
    let fixture = Fixture::new();
    let root = fixture.path().join("audio");
    fs::create_dir(&root).unwrap();
    fs::write(root.join("track.wav"), b"candidate").unwrap();
    let (state, library) = selected_library(&fixture);
    scan::begin(&library).unwrap();
    let error = scan::execute_with_hook(&library, |_, point| {
        if point == scan::ScanPoint::Finish {
            fs::rename(&root, fixture.path().join("old-root")).unwrap();
            fs::create_dir(&root).unwrap();
        }
    })
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::NotAuthorized);
    scan::fail(&library, error);
    let data = library.scan.lock().unwrap();
    assert_eq!(data.status.phase, contracts::ScanPhase::Failed);
    assert!(data.assets.is_empty());
    assert!(state.get(&library.info.id).is_err());
}

#[test]
fn non_utf8_paths_remain_lossless_and_have_distinct_stable_ids() {
    use std::{ffi::OsString, os::unix::ffi::OsStringExt};
    let fixture = Fixture::new();
    let root = fixture.path().join("audio");
    fs::create_dir(&root).unwrap();
    for byte in [0xfe, 0xff] {
        fs::write(
            root.join(OsString::from_vec(vec![byte, b'.', b'W', b'A', b'V'])),
            b"candidate",
        )
        .unwrap();
    }
    let (_state, library) = selected_library(&fixture);
    scan::begin(&library).unwrap();
    scan::execute(&library).unwrap();
    let first = scan::assets(&library).unwrap().assets;
    assert_eq!(first.len(), 2);
    assert_eq!(first[0].name, first[1].name);
    assert_ne!(first[0].id, first[1].id);
    let paths = library
        .scan
        .lock()
        .unwrap()
        .assets
        .iter()
        .map(|entry| entry.relative_path.clone())
        .collect::<Vec<_>>();
    assert_ne!(paths[0], paths[1]);
    scan::begin(&library).unwrap();
    scan::execute(&library).unwrap();
    assert_eq!(scan::assets(&library).unwrap().assets, first);
}

#[test]
fn permission_failures_are_partial_for_an_unprivileged_user() {
    use std::os::unix::fs::PermissionsExt;
    let fixture = Fixture::new();
    let root = fixture.path().join("audio");
    fs::create_dir_all(root.join("locked-directory")).unwrap();
    fs::write(root.join("readable.wav"), b"candidate").unwrap();
    let locked = root.join("locked.mp3");
    fs::write(&locked, b"locked").unwrap();
    fs::write(root.join("locked-directory/hidden.ogg"), b"hidden").unwrap();
    let (_state, library) = selected_library(&fixture);
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o0)).unwrap();
    fs::set_permissions(
        root.join("locked-directory"),
        fs::Permissions::from_mode(0o0),
    )
    .unwrap();
    let file_denied = fs::File::open(&locked).is_err();
    let directory_denied = fs::read_dir(root.join("locked-directory")).is_err();
    scan::begin(&library).unwrap();
    let result = scan::execute(&library);
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o600)).unwrap();
    fs::set_permissions(
        root.join("locked-directory"),
        fs::Permissions::from_mode(0o700),
    )
    .unwrap();
    assert!(
        file_denied && directory_denied,
        "permission test must run without root privileges"
    );
    result.unwrap();
    let result = scan::assets(&library).unwrap();
    assert_eq!(result.assets.len(), 2);
    assert!(
        result
            .assets
            .iter()
            .any(|asset| asset.availability == contracts::Availability::Inaccessible)
    );
    let status = scan::status(&library).unwrap();
    assert_eq!(status.phase, contracts::ScanPhase::Partial);
    assert_eq!(status.warning_count, 2);
}

#[test]
fn missing_entries_report_partial_results_and_warnings_are_bounded() {
    let fixture = Fixture::new();
    let root = fixture.path().join("audio");
    fs::create_dir(&root).unwrap();
    for index in 0..110 {
        fs::write(root.join(format!("missing-{index}.wav")), b"candidate").unwrap();
    }
    fs::write(root.join("keep.ogg"), b"candidate").unwrap();
    let (_state, library) = selected_library(&fixture);
    scan::begin(&library).unwrap();
    scan::execute_with_hook(&library, |path, point| {
        if point == scan::ScanPoint::Inspect && path.to_string_lossy().starts_with("missing-") {
            fs::remove_file(root.join(path)).unwrap();
        }
    })
    .unwrap();
    let status = scan::status(&library).unwrap();
    assert_eq!(status.phase, contracts::ScanPhase::Partial);
    assert_eq!(status.warning_count, 110);
    assert_eq!(status.warnings.len(), 100);
    assert_eq!(scan::assets(&library).unwrap().assets[0].name, "keep.ogg");
}

#[test]
fn progress_is_visible_and_a_running_worker_rejects_overlapping_scans() {
    use std::sync::{Arc, Barrier};
    let fixture = Fixture::new();
    let root = fixture.path().join("audio");
    fs::create_dir(&root).unwrap();
    for index in 0..40 {
        fs::write(root.join(format!("{index}.wav")), b"candidate").unwrap();
    }
    let (_state, library) = selected_library(&fixture);
    let barrier = Arc::new(Barrier::new(2));
    let worker_barrier = Arc::clone(&barrier);
    let worker = Arc::clone(&library);
    scan::begin(&library).unwrap();
    let thread = std::thread::spawn(move || {
        scan::execute_with_hook(&worker, |_, point| {
            if point == scan::ScanPoint::Finish {
                worker_barrier.wait();
                worker_barrier.wait();
            }
        })
    });
    barrier.wait();
    let status = scan::status(&library).unwrap();
    assert_eq!(status.phase, contracts::ScanPhase::Running);
    assert!(status.visited_entries >= 32);
    assert_eq!(scan::begin(&library).unwrap_err().code, ErrorCode::Busy);
    barrier.wait();
    thread.join().unwrap().unwrap();
    assert_eq!(scan::assets(&library).unwrap().assets.len(), 40);
}

#[test]
fn excessive_directory_depth_produces_bounded_partial_results() {
    let fixture = Fixture::new();
    let root = fixture.path().join("audio");
    fs::create_dir(&root).unwrap();
    let mut path = root.clone();
    for _ in 0..66 {
        path.push("d");
        fs::create_dir(&path).unwrap();
    }
    fs::write(path.join("too-deep.wav"), b"candidate").unwrap();
    let (_state, library) = selected_library(&fixture);
    scan::begin(&library).unwrap();
    scan::execute(&library).unwrap();
    assert!(scan::assets(&library).unwrap().assets.is_empty());
    assert_eq!(
        scan::status(&library).unwrap().phase,
        contracts::ScanPhase::Partial
    );
}

#[test]
fn scans_leave_original_hashes_names_modes_and_modification_times_unchanged() {
    use sha2::{Digest, Sha256};
    use std::{collections::BTreeMap, os::unix::fs::MetadataExt, path::Path};
    fn snapshot(root: &Path) -> BTreeMap<std::path::PathBuf, (Vec<u8>, u32, i64, i64)> {
        let mut result = BTreeMap::new();
        for entry in fs::read_dir(root).unwrap() {
            let entry = entry.unwrap();
            let metadata = entry.metadata().unwrap();
            let hash = if metadata.is_file() {
                Sha256::digest(fs::read(entry.path()).unwrap()).to_vec()
            } else {
                vec![]
            };
            result.insert(
                entry.path(),
                (
                    hash,
                    metadata.mode(),
                    metadata.mtime(),
                    metadata.mtime_nsec(),
                ),
            );
            if metadata.is_dir() {
                result.extend(snapshot(&entry.path()));
            }
        }
        result
    }
    let fixture = Fixture::new();
    let root = fixture.path().join("audio");
    fs::create_dir_all(root.join("nested")).unwrap();
    fs::write(root.join("one.wav"), b"RIFF synthetic WAV fixture").unwrap();
    fs::write(root.join("nested/two.OGG"), b"OggS synthetic OGG fixture").unwrap();
    fs::write(root.join("ignored.txt"), b"unsupported").unwrap();
    let before = snapshot(&root);
    let (_state, library) = selected_library(&fixture);
    scan::begin(&library).unwrap();
    scan::execute(&library).unwrap();
    assert_eq!(before, snapshot(&root));
}

#[test]
fn ipc_fixtures_are_actual_rust_serializations_and_requests_reject_extra_paths() {
    use contracts::*;
    use serde_json::{Value, json, to_value};
    let fixture: Value =
        serde_json::from_str(include_str!("../../src/lib/fixtures/ipc.json")).unwrap();
    let asset = AudioAsset {
        id: "asset-fixture".into(),
        name: "kick.WAV".into(),
        duration_seconds: None,
        relative_display_path: "drums/kick.WAV".into(),
        extension: "wav".into(),
        availability: Availability::Available,
    };
    assert_eq!(
        to_value(AssetList {
            scan_id: Some("scan-fixture".into()),
            assets: vec![asset.clone()]
        })
        .unwrap(),
        fixture["assets"]
    );
    assert_eq!(
        to_value(AudioAsset {
            availability: Availability::Inaccessible,
            ..asset
        })
        .unwrap(),
        fixture["inaccessible"]
    );
    assert_eq!(
        to_value(LibraryInfo {
            id: "library-fixture".into(),
            display_path: "/synthetic/audio".into()
        })
        .unwrap(),
        fixture["library"]
    );
    assert_eq!(
        to_value(None::<LibraryInfo>).unwrap(),
        fixture["cancellation"]
    );
    let request: LibraryRequest = serde_json::from_value(fixture["request"].clone()).unwrap();
    assert_eq!(to_value(request).unwrap(), fixture["request"]);
    assert!(
        serde_json::from_value::<LibraryRequest>(json!({ "libraryId": "x", "path": "/etc" }))
            .is_err()
    );
    assert!(serde_json::from_value::<LibraryRequest>(json!({ "library_id": "x" })).is_err());
    assert_eq!(to_value(ScanStatus::default()).unwrap(), fixture["idle"]);
    for (key, phase) in [
        ("running", ScanPhase::Running),
        ("complete", ScanPhase::Complete),
        ("partial", ScanPhase::Partial),
        ("failed", ScanPhase::Failed),
    ] {
        let partial = phase == ScanPhase::Partial;
        let failed = phase == ScanPhase::Failed;
        let status = ScanStatus {
            scan_id: Some("scan-fixture".into()),
            phase,
            visited_entries: 2,
            discovered_assets: 1,
            warning_count: u32::from(partial),
            warnings: if partial {
                vec![ScanWarning {
                    message: "The audio candidate cannot be read.".into(),
                    relative_display_path: Some("locked.mp3".into()),
                }]
            } else {
                vec![]
            },
            error: failed.then(|| {
                AppError::new(
                    ErrorCode::NotAuthorized,
                    "Select an audio directory to authorize access.",
                )
            }),
        };
        assert_eq!(to_value(status).unwrap(), fixture[key]);
    }
    for (code, expected) in [
        ErrorCode::NotAuthorized,
        ErrorCode::Unavailable,
        ErrorCode::InvalidInput,
        ErrorCode::Busy,
        ErrorCode::InternalError,
    ]
    .into_iter()
    .zip(fixture["errorCodes"].as_array().unwrap())
    {
        assert_eq!(to_value(&code).unwrap(), *expected);
        assert_eq!(
            to_value(AppError::new(
                code,
                "Select an audio directory to authorize access."
            ))
            .unwrap(),
            json!({ "code": expected, "message": "Select an audio directory to authorize access." })
        );
    }
}

#[test]
fn access_helpers_reject_write_flags_and_removed_roots() {
    use rustix::fs::OFlags;
    use std::path::Path;
    let fixture = Fixture::new();
    fs::create_dir(fixture.path().join("audio")).unwrap();
    fs::write(fixture.path().join("audio/track.wav"), b"untouched").unwrap();
    let (state, library) = selected_library(&fixture);
    for flags in [OFlags::WRONLY, OFlags::RDWR, OFlags::TRUNC] {
        assert!(
            library
                .root
                .open_relative(Path::new("track.wav"), flags)
                .is_err()
        );
    }
    assert_eq!(
        fs::read(fixture.path().join("audio/track.wav")).unwrap(),
        b"untouched"
    );
    fs::rename(fixture.path().join("audio"), fixture.path().join("gone")).unwrap();
    assert_eq!(
        state.get(&library.info.id).err().unwrap().code,
        ErrorCode::NotAuthorized
    );
}

#[test]
fn replacing_an_audio_candidate_with_a_fifo_does_not_block_or_publish_it() {
    use rustix::fs::{CWD, Mode, mkfifoat};
    use std::path::Path;
    let fixture = Fixture::new();
    let root = fixture.path().join("audio");
    fs::create_dir(&root).unwrap();
    fs::write(root.join("track.wav"), b"candidate").unwrap();
    let (_state, library) = selected_library(&fixture);
    scan::begin(&library).unwrap();
    let mut replaced = false;
    scan::execute_with_hook(&library, |path, point| {
        if path == Path::new("track.wav") && point == scan::ScanPoint::ReadOpen {
            fs::rename(root.join(path), root.join("original.bin")).unwrap();
            mkfifoat(CWD, root.join(path), Mode::RUSR | Mode::WUSR).unwrap();
            replaced = true;
        }
    })
    .unwrap();
    assert!(replaced);
    assert!(scan::assets(&library).unwrap().assets.is_empty());
    assert_eq!(
        scan::status(&library).unwrap().phase,
        contracts::ScanPhase::Partial
    );
}
