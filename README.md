# SoundShelf

A local Linux audio asset manager using Tauri 2, Rust, React, TypeScript, and Vite. The current implementation selects directories and discovers WAV, MP3, and OGG candidates without changing original files. The SQLite index persists across restarts and supports filename/path/tag search, pagination, tags, and favorites. Filesystem authorization remains session-only. Authorized local playback now provides a single audio player with pause, seeking, and recoverable errors. The MVP feature implementation is complete. Automated tests and native WebKitGTK playback regressions pass; full development/bundled application GUI, keyboard/screen-reader, and audible playback acceptance remain pending.

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
cargo test --offline
cargo clippy --offline --all-targets -- -D warnings
```

Build an executable with bundled frontend assets without creating installers:

```sh
npm run tauri -- build --debug --no-bundle -- --offline
```

Select an audio directory, then choose **Start scan**. **Refresh scan** repeats discovery. Canceling the picker keeps the current view. Search stored filenames, relative paths and tag names with **Search**, then use **Previous** and **Next** to browse 50 results per page. Wildcard characters are literal; ASCII matching ignores case, while full Unicode case folding is unavailable.

Use **Tags** to create, rename and delete global tags, and **Edit tags** beside an asset to assign/remove them. Names are trimmed, limited to 64 Unicode characters, and deduplicated by Unicode lowercase mapping (without Unicode normalization). Each asset supports up to 32 tags. **Favorite**, **Favorites only**, and tag filters work together with search and pagination. Selecting multiple tags requires all of them. Annotations remain editable for inactive, missing and inaccessible assets; deleting a tag removes its associations across all libraries. All metadata lives in SQLite, never embedded audio tags or sidecars.

The index lives at `soundshelf.sqlite3` in Tauri's application-data directory (normally `$XDG_DATA_HOME/dev.soundshelf.app`, with the platform fallback when unset). No index or sidecars are created in selected audio directories. After restarting, saved libraries and results remain visible but inactive; select directories again through the native picker to authorize scans and playback. Complete scans mark absent files missing; partial and failed scans retain earlier metadata. Files replaced at the same relative path keep the path's metadata; moved files create new entries.

Choose **Play** beside an asset to use the single audio player. Codec support depends on WebKitGTK/GStreamer; an extension alone does not establish playable audio. No original file is changed.

See [MVP annotations, verification and manual acceptance](docs/MILESTONE_4.md), [playback implementation and Linux acceptance steps](docs/MILESTONE_3.md), [the architecture plan](docs/PLAN.md) and [discovery verification and limitations](docs/MILESTONE_1.md), and [persistent-library verification](docs/MILESTONE_2.md). Native GUI acceptance checks remain separate from automated tests.
