# SoundShelf

A local Linux audio asset manager using Tauri 2, Rust, React, TypeScript, and Vite. The current implementation selects directories and discovers WAV, MP3, and OGG candidates without changing original files. The SQLite index persists across restarts and supports filename/path search and pagination. Filesystem authorization remains session-only. Authorized local playback now provides a single audio player with pause, seeking, and recoverable errors. Native WebKitGTK playback regression tests pass; full application acceptance remains pending, and metadata editing remains a future milestone.

## Development

Use Linux with `openat2` support, the Tauri Linux native prerequisites, Rust, and Node.js 24 or later (the tests use built-in TypeScript stripping). Secure traversal has no compatibility fallback. Tauri already uses GTK 3; the directory picker uses the same GTK dependency.

```sh
npm run tauri -- dev
npm test
npm run build
```

From `src-tauri/`:

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
```

Build an executable with bundled frontend assets without creating installers:

```sh
npm run tauri -- build --debug --no-bundle
```

Select an audio directory, then choose **Start scan**. **Refresh scan** repeats discovery. Canceling the picker keeps the current view. Search stored filenames and relative paths with **Search**, then use **Previous** and **Next** to browse 50 results per page. Wildcard characters are literal; ASCII matching ignores case, while full Unicode case folding is unavailable.

The index lives at `soundshelf.sqlite3` in Tauri's application-data directory (normally `$XDG_DATA_HOME/dev.soundshelf.app`, with the platform fallback when unset). No index or sidecars are created in selected audio directories. After restarting, saved libraries and results remain visible but inactive; select directories again through the native picker to authorize scans and playback. Complete scans mark absent files missing; partial and failed scans retain earlier metadata. Files replaced at the same relative path keep the path's metadata; moved files create new entries.

Choose **Play** beside an asset to use the single audio player. Codec support depends on WebKitGTK/GStreamer; an extension alone does not establish playable audio. No original file is changed.

See [playback implementation and Linux acceptance steps](docs/MILESTONE_3.md), [the architecture plan](docs/PLAN.md) and [discovery verification and limitations](docs/MILESTONE_1.md), and [persistent-library verification](docs/MILESTONE_2.md). Native GUI acceptance checks remain separate from automated tests.
