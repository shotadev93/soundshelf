# SoundShelf

A local Linux audio asset manager using Tauri 2, Rust, React, TypeScript, and Vite. The current implementation selects directories and discovers WAV, MP3, and OGG candidates without changing original files. Discovery is session-only; persistence, playback, tags, and favorites are future milestones.

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

Select an audio directory, then choose **Start scan**. **Refresh scan** repeats discovery. Canceling the picker keeps the current view. After restarting, select directories again to authorize access.

See [the architecture plan](docs/PLAN.md) and [discovery verification and limitations](docs/MILESTONE_1.md). Native GUI acceptance checks remain separate from automated tests.
