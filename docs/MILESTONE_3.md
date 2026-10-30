# Authorized local audio playback

Milestone 3 adds the playback implementation and automated security/controller coverage. **Full application acceptance remains pending for both development and the bundled executable; the native WebKit playback regression below now passes.** A successful build, GIO stream test, fixture generation, or installed codec is not evidence of native HTML audio playback. The initial implementation did not perform desktop GUI or audible playback checks. The subsequent fix runs a real, muted WebKit playback test; audible output and full application interaction remain manual checks. Milestone 4 is not implemented.

## Architecture and requirements review

The inspected Milestone 2 code already retained authorized root descriptors in memory, used Linux `openat2` for discovery, stored lossless asset paths/IDs in SQLite, restored saved roots inactive, and limited commands to the main window. Those mechanisms remain intact. The only existing working-tree modification before this work was `src-tauri/.gitignore`; it was preserved.

The original direct `soundshelf-audio://` adapter failed on the supported Linux runtime: a normal synthetic PCM WAV produced `NotSupportedError` and `MEDIA_ERR_SRC_NOT_SUPPORTED` (code 4). Registering the scheme as secure or CORS enabled did not fix it. WebKit's GStreamer source accepts HTTP, HTTPS and blob URLs; custom-scheme sniffing requests can reach the handler even though actual playback cannot start. See the [WebKit source](https://github.com/WebKit/WebKit/blob/main/Source/WebCore/platform/graphics/gstreamer/WebKitWebSourceGStreamer.cpp) and [upstream issue](https://bugs.webkit.org/show_bug.cgi?id=146351).

`playback_protocol.rs` now uses the existing libsoup dependency to bind an ephemeral port **only on IPv4 loopback (`127.0.0.1`)**. The endpoint serves only `/audio/<128-bit-random-token>`; it rejects other hosts, foreign explicit origins, queries and arbitrary paths. It does not send CORS permission headers, expose directories or accept filesystem paths. Playback requires the same in-memory grants and secure read-only opens as before. The listener closes with the main GTK window. No external network service or internet connection is used.

`prepare_playback` maps the internal grant locator to `http://127.0.0.1:<port>/audio/<token>`. The frontend validates this exact URL shape and port range. Production/development CSP allows this loopback route only under `media-src`; general frontend fetch access is not added.

libsoup serves each response incrementally with body accumulation disabled. It requests the next block only after the previous block is written. Database queries, secure opens and reads run on blocking workers; GTK/GLib retain the messages on the main thread. Each block is at most 64 KiB. Disconnecting releases the reader; a failed or revoked stream closes its socket rather than completing a truncated successful response.

## Authorization and file identity

`prepare_playback` accepts only `{ libraryId, assetId }` and rejects unknown DTO fields, including paths. It resolves the asset through a parameterized query constrained by both IDs. Neither a saved library nor a frontend path can grant access. Its response is `{ url: "http://127.0.0.1:<port>/audio/<128-bit-random-token>" }`, without a filename or filesystem path.

Grants live only in memory, expire after one hour measured with `Instant`, and reference the original `Library` generation through `Weak`. There are at most 32 retained grants. Preparing a 33rd evicts and revokes the oldest grant, including subsequent reads of its open streams. Expired/deactivated roots are pruned during preparation. Even before pruning, their tokens cannot authorize requests or stream reads. Restart/exit discards all grants. Reselecting a revoked root cannot reactivate an old grant, including when its persistent library ID is reused. Switching the UI library stops the frontend player; it does not deactivate other already authorized roots.

Preparation securely opens the file and records its device, inode, length, mtime and ctime. Each media request resolves the grant and DB ID mapping again, validates the original root generation, checks native path and indexed extension, and opens from that root with the existing `BENEATH | NO_SYMLINKS | NO_XDEV` behavior. It first inspects an `O_PATH` handle, requires a nonempty regular file, then opens read-only/nonblocking and compares the actual descriptors. Root identity is checked again after opening. The opened file identity must match preparation; replacing or modifying it requires a fresh preparation. No unrestricted path reopen is used.

Each stream read rechecks expiration/revocation and the authorized root. Linux openat2 support remains mandatory. Missing, unreadable, empty, replaced, unsupported and nonregular files fail without returning their contents. Error messages omit full filesystem paths. The application never writes media, tags, timestamps or sidecars. OS reads may update access times.

## Media responses

| Request | Response |
| --- | --- |
| GET without Range | 200 and full file streamed incrementally |
| `bytes=start-end` | 206, inclusive bounds; end clamped to file length |
| `bytes=start-` | 206 through the end of the file |
| `bytes=-count` | 206 for the suffix, clamped to file length |
| Malformed, unsatisfiable, zero suffix, overflowing, or multiple ranges | 416 with `Content-Range: bytes */<length>` and an empty body |
| HEAD | 200, full Content-Length and no body; Range ignored |
| Other methods | 405 and `Allow: GET, HEAD` |
| Invalid, expired, revoked, or previous-session token | 403 |
| Asset no longer in the index | 404 |
| Unavailable/changed/empty/unsupported file | 410 |

Successful responses include Content-Type, Content-Length, Accept-Ranges, Cache-Control: no-store, and X-Content-Type-Options: nosniff; 206 also includes Content-Range. Types are `audio/wav`, `audio/mpeg`, and `audio/ogg`. Multiple ranges intentionally do not produce multipart bodies.

`AudioStream` reads at most 64 KiB per call and stops at the selected range boundary, even if another process grows the file. A shortened file reports an unexpected EOF. No media bytes pass through IPC; no application buffer scales with file length or requested range length. An 8 GiB sparse-file test checks response construction and the read ceiling, and a GIO adapter test checks incremental reads and EOF. These tests establish the application's read behavior, not a bound on WebKit/GStreamer's internal buffering or a process-wide memory measurement.

## Player and UI

`AudioPlayer.tsx` renders exactly one HTML audio element. `PlaybackController` owns its event handling and provides play, pause, resume, stop, seek, current time, duration and retry. Selecting an asset immediately stops/removes the old source before awaiting preparation. Selection generations guard both preparation results and play promises. Library changes, inactive authorization and component cleanup clear pending playback. Search/pagination do not stop an already selected asset.

The player displays loading and recoverable errors and retains the selected name for retry. Media loading, decoding and unsupported-source failures have distinct messages. A rejected play promise cannot replace an earlier media error with a generic IPC error. Typed preparation errors retain their backend message. Browser autoplay rejection keeps a prepared source paused so the explicit Play button can satisfy gesture policy. Unknown/nonfinite duration disables seeking. Metadata duration remains nullable in the index; the player's duration comes from the media element.

Controls have visible focus, native button/range keyboard behavior, accessible labels and seek time text. Loading/play/pause status uses a polite live region; errors use alerts. Play selection is disabled for inactive saved roots, but historical missing/inaccessible rows can be retried after picker authorization because availability is rechecked in Rust. Existing Saved Library colors and their contrast tests are preserved. Rendered native themes, keyboard/screen-reader behavior and audio controls still need manual acceptance.

## Files and dependencies

New files:

- `src-tauri/src/playback.rs`: capabilities, secure opens, stream reader, byte-range parsing and core range unit tests.
- `src-tauri/src/playback_protocol.rs`: libsoup loopback streaming adapter and optional native WebKit regression test.
- `src-tauri/src/playback_tests.rs`: disposable filesystem/database playback tests.
- `src-tauri/permissions/autogenerated/prepare_playback.toml`: generated command permission.
- `src/components/AudioPlayer.tsx`, `src/lib/playback-controller.ts`, `src/lib/playback-controller.test.mjs`.
- `scripts/create-playback-fixtures.py`: optional synthetic GUI fixture generation.
- `docs/MILESTONE_3.md`.

Updated Rust files: `src/{commands,contracts,db,lib,library}.rs`, `tests/library_integration.rs`, Cargo manifest/lock, command permission generation, main-window capability and CSP. Updated frontend files: App TSX/CSS, IPC/types/runtime contracts, shared fixture and contract tests. README and the plan/Milestone 2 status links now point here. There is no migration or index schema change.

Direct Rust dependencies added: `webkit2gtk` 2.0.2 with `v2_40` for the native WebKit regression test, and `soup3` 0.5 (Rust crate name `soup`) for the local playback listener and response headers. Both were already present transitively in the existing Tauri lockfile/cache; no new native library or frontend dependency was introduced. The optional fixture script uses Python's standard library and installed GStreamer tools/encoders. It is not an application runtime dependency.

## Linux playback failure regression

The direct custom scheme failed with a generated valid WAV in WebKitGTK 2.52.6. The loopback adapter passed the optional native regression on the same Linux desktop: synthetic WAV, MP3 and Vorbis/OGG each loaded duration, played, paused, resumed and sought from both development (`http://localhost:1420`) and production (`tauri://localhost`) document origins. Media is muted; these checks do not establish audible speaker output or full Tauri application integration.

The same test checks actual loopback HTTP full and range bodies (including a response spanning multiple 64 KiB blocks), suffix ranges, 416, HEAD, unknown tokens, and a foreign Origin. It compares the WAV bytes after playback. Normal unit tests check Host/origin/path restrictions. Run explicitly in a desktop session (requires installed GStreamer encoders to generate disposable MP3/OGG fixtures):

```sh
cargo test --offline --manifest-path src-tauri/Cargo.toml --lib native_webkit_plays_pauses_and_seeks_loopback_audio -- --ignored --test-threads=1 --nocapture
```

Fix verification: the normal Rust suite passed (16 library tests and 38 integration tests, with the desktop test skipped by default), all 27 frontend tests passed, and formatting, Clippy with warnings denied, and the TypeScript/Vite build passed. The desktop regression was also run explicitly and passed.

WebKit emitted warnings about missing subtitle/video helper plugins; the audio checks passed. The native test uses the real Rust authorization, SQLite index, libsoup response adapter and WebKit/GStreamer pipeline. It uses only disposable fixtures. Keyboard navigation, native picker interaction, audible output and the bundled executable remain manual acceptance items.

## Automated verification

Initial implementation results (before the playback-route fix): `cargo fmt --check` passed; `cargo test --offline` passed (15 library unit tests and 38 tests in the private-core integration binary, including repeated core unit tests); Clippy passed with warnings denied; all 25 Node tests passed; TypeScript/Vite production build passed; the Tauri debug executable build with bundled assets passed. Synthetic WAV/MP3/OGG fixture generation also passed. The Tauri CLI emitted the pre-existing `.app` bundle-identifier advisory for macOS. Commands use the repository's established offline Cargo approach:

```sh
# src-tauri/
cargo fmt --check
cargo test --offline
cargo clippy --offline --all-targets -- -D warnings

# repository root
npm test
npm run build
npm run tauri -- build --debug --no-bundle -- --offline
python3 scripts/create-playback-fixtures.py
```

Coverage includes retained Milestone 1/2 tests; playback DTO serialization/extra-field rejection; random/unknown/expired/restarted tokens; bounded token storage/eviction; revoked/reselected roots and open streams; missing/empty/unreadable files; malicious stored traversal/absolute paths; symlinks and inode/root replacements at controlled inspect-to-open boundaries; FIFO replacements; byte ranges and exact headers/bodies; read ceilings/EOF via Rust and GIO; and unchanged original bytes, directory names, mode and mtime. The fixtures are disposable synthetic data, including a generated PCM WAV, not personal libraries. Unreadable tests assert real OS permission denial under an unprivileged user.

Frontend tests exercise play/pause/resume, time/duration, clamped seeking, switching, pending request cancellation, stale preparation success/failure, stale play success/rejection, authorization/media errors and retry, cleanup, unknown duration, replay after end and autoplay policy. They use an EventTarget media double; they do not mount React in a browser or decode audio. Existing saved-library contrast tests still run.

Verification environment: Zorin OS 18.1, GTK 3.24.41, WebKitGTK 2.52.6. GStreamer's installed tools generated the three synthetic tone formats successfully. This records available APIs and fixture creation, not tested browser codec support. Native distribution/runtime/codec acceptance results have not yet been recorded.

## Manual Linux acceptance gate

Perform all steps twice: once with `npm run tauri -- dev` and once with the bundled executable. Use only generated/disposable files, an isolated test data directory, and low speaker volume.

1. Generate fixtures with `python3 scripts/create-playback-fixtures.py`. It prints a new `/tmp/soundshelf-playback-.../audio` directory containing approximately 20-second synthetic PCM WAV, MP3 and Vorbis/OGG tones, corrupt/empty candidates and an outside-root symlink. It also writes `integrity-before.json` outside that audio root. The generator requires installed `gst-launch-1.0`, `audiotestsrc`, `wavenc`, `lamemp3enc`, `vorbisenc` and `oggmux`; missing encoders are a fixture-generation prerequisite, not evidence of a SoundShelf failure.
2. From the repository root run `soundshelf_dev_data=$(mktemp -d /tmp/soundshelf-dev-data-XXXXXX)`, then `XDG_DATA_HOME="$soundshelf_dev_data" npm run tauri -- dev`. Keep that shell variable for restart checks. Select the generated **audio** directory with the native picker and scan. Confirm the outside symlink is skipped. Play WAV, MP3 and OGG separately. Record audible output, media duration near 20 seconds, and any format-specific error with the installed OS/WebKitGTK/GStreamer versions. If policy blocks automatic start, press the player Play button.
3. For each format pause for several seconds (time/audio must stop), resume, seek near the middle and near the end, seek backwards and replay after ending. Confirm displayed time/duration and range-based seeking work. Use WebKit's inspector where available to check the `http://127.0.0.1:<port>/audio/<token>` route, Range requests and 206/Content-Range responses. Check that there are no capability, CSP, mixed-content or custom-scheme errors and that only the loopback listener is used and no file URL is used.
4. Switch WAV → MP3 → OGG rapidly while loading and while playing. Only the latest selection may play or display errors. Switch saved libraries; the old sound must stop. Search/page while playing; the player must remain usable. Cancel the native picker during playback; existing playback must remain usable. Verify loading/disabled controls and a responsive window.
5. Play corrupt and empty candidates; verify a visible error, preserved library/search results and usable Retry playback. For separate disposable copies, remove a file, make one unreadable, replace one with an outside-root symlink, and rename/replace the selected root. Retry and seek to force fresh requests: no outside bytes may play. Reauthorize using the picker and retry a restored valid fixture. Previously issued URLs must remain unusable after root revocation and after app restart. Buffered bytes can continue until cleared; they cannot be withdrawn by token revocation.
6. Restart with the same test XDG_DATA_HOME. Saved results must remain searchable but inactive; Play and scanning stay disabled until picker reselection. Confirm old URLs cannot restore authorization. For a long-running check, leave a token older than one hour, then seek/reload it; it must fail and fresh Retry playback must recover for an active root. Unit tests cover expiration without waiting.
7. Use only Tab/Shift+Tab, Enter/Space and the range input's arrow/Home/End keys. Verify labels, visible focus, paused/loading/error announcements and seek value text with a screen reader. Inspect long filenames, narrow windows, dark/light themes and the Saved Library popup's text contrast.
8. Build with `npm run tauri -- build --debug --no-bundle -- --offline`. Stop Vite. Run `soundshelf_prod_data=$(mktemp -d /tmp/soundshelf-prod-data-XXXXXX)`, then `XDG_DATA_HOME="$soundshelf_prod_data" ./src-tauri/target/debug/soundshelf` and repeat steps 2–7 using its separate test data directory. Retain that shell variable for restart checks. Verify bundled frontend loading and production CSP behavior without a Vite server. A successful executable build alone does not pass this step.
9. Using fresh, unmodified fixtures for the playback-only integrity check, compare each regular file against the generated baseline: SHA-256, relative name set, mode and nanosecond mtime must match; no sidecars may appear. Do this separately from the deliberate external mutations in step 5. Atime is excluded. Record dev/bundled outcomes and runtime versions here before declaring native playback accepted.

## Limits and security scope

The loopback endpoint is a bearer-capability route: another local process that obtains an active token can use it while its authorization remains valid. Tokens never include paths, are not logged, expire and are invalid after restart. HTTP Host validation prevents DNS rebinding to the listener; no CORS access is granted to external pages.

There is no watcher, atomic filesystem snapshot or immediate withdrawal of bytes already buffered by WebKit/GStreamer. Root removal/replacement is detected during validation; changes wholly between validations may not be seen. A successfully opened descriptor continues referring to its original file if another process renames that file afterward. In-place external writes during a stream are not frozen; new requests compare identity and fail until preparation is refreshed. Hard links remain regular files and containment cannot establish content provenance. These are existing host-filesystem boundaries, not permission to bypass containment.

One-hour grants and the 32-grant retention limit can interrupt unusually long playback or retained old URLs; retry creates a new grant. Files larger than the native signed stream-length limit and zero-length files are rejected. Multiple ranges, conditional range validators and codec detection are not implemented. Recognized extensions identify candidates only; actual WAV/MP3/OGG codec support depends on the installed WebKitGTK/GStreamer runtime. Failed decoding is recoverable and never triggers filesystem/server fallback. Tests are not a general security audit, and native acceptance remains outstanding.
