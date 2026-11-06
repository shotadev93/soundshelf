# SoundShelf v0.1.0 release checklist

Status: **Final package acceptance pending; not published.** On 2026-11-04 the user approved the original folder/waveform icon replacement and MIT licensing of their own screenshot image rights. Those two rights/branding blockers are resolved. Final package installation/launcher acceptance and explicit publication approval remain required.

## Approved assets follow-up — 2026-11-04

- Original MIT-licensed vector source: `src-tauri/icons/soundshelf.svg`. All 16 existing PNG/ICO/ICNS assets were regenerated with the installed Tauri CLI; no default Tauri application icon is retained in the current icon directory.
- User confirmation applies to their own image rights in both README screenshots, not third-party material depicted in them. See the [license review](THIRD_PARTY_LICENSE_REVIEW.md).
- A fresh Debian build completed under `src-tauri/target/approved-assets-20261104/`; the final artifact evidence is recorded below. Earlier packages and manifests are retained as historical candidates.
- No application code, playback authorization, CSP, identifier, database location or dependency versions changed in this follow-up.
- User-reported application acceptance in section B remains valid. The new icon/legal package has not been installed by Codex; inspect its launcher icon and perform a focused same-version reinstall/startup/playback/data-preservation smoke check before approving publication.

### Current artifact and executed verification

Build command (repository root):

```sh
CARGO_TARGET_DIR="$PWD/src-tauri/target/approved-assets-20261104" RUSTFLAGS="--remap-path-prefix=$HOME=/build --remap-path-prefix=$PWD=/soundshelf" npm run tauri -- build --bundles deb -- --offline --locked
```

Actual artifact: `src-tauri/target/approved-assets-20261104/release/bundle/deb/soundshelf_0.1.0_amd64.deb`, **3,474,466 bytes (3.31 MiB)**, package `soundshelf` version `0.1.0`, architecture `amd64`. The package and matching `SHA256SUMS` are staged in `src-tauri/target/approved-assets-20261104/release/release-assets/v0.1.0/`.

```text
dc069f499998738c4ea7a0a815a681c1dd29ebea7718290c9a65df7753f9905e  soundshelf_0.1.0_amd64.deb
```

Checksum verification passed. The 29-file payload includes the original folder/waveform icons and legal/source-compliance materials. All three packaged PNGs match the generated source assets; the bundled license, notices, review and entire licenses directory match the repository inputs. The executable embeds the current `index-DLFp0Ojs.js` frontend, with no old frontend or debug sections; maximum imported glibc version remains 2.39. Scoped static checks found no private home paths, known binary credential patterns, personal media/database/environment/source-map files or copied shared libraries. This is not proof of complete security. `apt-get --simulate install --reinstall` succeeded with one same-version replacement, zero new packages and zero removals; no package was installed.

- Follow-up checks passed: `cargo fmt --check`; `cargo test --offline` (21 library + 44 integration tests; optional desktop test ignored by default); `cargo clippy --offline --all-targets -- -D warnings`; `npm test` (44 passes); production frontend and Debian builds; `git diff --check` and documentation links.
- This follow-up changes artwork/licensing only. Earlier explicit debug/release native probes are historical executed evidence, not rerun claims. No new behavior tests were added for the reversible asset replacement. The two screenshot files remain byte-identical to HEAD.
- Tauri repeated its existing macOS `.app` identifier advisory. The Linux identifier stays unchanged. AppImage remains unavailable for the previously recorded missing-patchelf reason; it was not retried here.

### Final package smoke acceptance — human tester, not executed by Codex

1. Close SoundShelf and stop Vite normally. Follow the current verification/install commands in the manual reference below, using the **approved-assets** directory and `--reinstall`; retain all existing application data.
2. Check the desktop launcher and running application use the navy/teal folder/waveform icon. If the desktop retains a cached old icon, log out/in and report the desktop behavior; do not delete application-data directories.
3. Launch `/usr/bin/soundshelf` without Vite. Prefer the isolated XDG test data from the reference procedure. Confirm existing test metadata remains, select the disposable audio root again, play/pause a fixture and repeat pause → seek → resume → pause.
4. Close/restart and confirm test metadata persists and picker reauthorization is required. Record the installed artifact hash and results. Earlier user-reported full functional acceptance is preserved; this is focused verification of the final packaging changes.

**Current verdict:** the two rights/branding blockers are resolved. Final package smoke acceptance and explicit publication approval remain pending; do not tag/upload this candidate before those gates pass. No commit, push, tag, upload, publication, system-package installation or user-data cleanup was performed.

## Historical pre-release audit — before asset approval

The following audit records the earlier blocked candidate. Its icon/image decisions and artifact paths are superseded by the approved-assets follow-up above; historical results are retained for provenance.

### Initial audit record — 2026-11-04

### A. Checks executed by Codex

- Read the current source/configuration and the plan, all milestone reports, README, release notes and playback investigation; preserve existing uncommitted changes and historical milestone counts.
- `cargo fmt --check`, `cargo test --offline` (21 library + 44 integration-binary passes; one optional desktop test ignored by default), `cargo clippy --offline --all-targets -- -D warnings`, `npm test` (44 passes) and `npm run build`: passed in this audit.
- The optional real WebKit test was explicitly run and passed all six debug codec/origin cases, using the actual controller and disposable fixtures. Missing WebVTT/fakevideosink warnings persisted; no system packages were installed. This is muted playback, not independent audible/GUI testing.
- The release-profile actual-controller native probe also passed all three production-origin codecs in this audit. Together the explicit desktop runs covered nine format/origin cases; the normal suite still ignores the optional desktop test by default.
- The user-tested corrected Debian package is `caa2c45780ca3b6578d3097ae3a96ca269bf0a7a898a7ce686453cbbcd1e93e1` (3,085,674 bytes). Its embedded `index-DLFp0Ojs.js` matches the current production frontend; it contains no old pre-fix `index-s7oNAVJm.js`, and the application fixes predate the package. This confirms freshness beyond the version number alone.
- The prior Debian payload has only the executable, desktop entry and three icons, with no license notices. Legal-notice bundling is now configured and verified in the separately built audit candidate, retaining every previous artifact/checksum. Its actual bytes are recorded below.
- Full standard MIT License added for `2026 shotadev93`, supported by all five commit author identities and repository ownership. npm/Cargo license fields agree. No legal name was invented.
- Third-party notices cover 268 Linux Cargo normal/build packages and 26 installed npm packages. No included package lacks recovered license text. Original MPL source archives are supplied with lockfile-verified hashes; Rust library notices and SQLite's separate disclaimer are included. The first notice-generation pass flagged a missing native Rolldown package notice; it was resolved using the matching-version parent package's distributed notices.
- Unused starter SVG logos removed from source/distribution after import checks. Locked Linux dependency MSRVs reach Rust 1.90; README now states that requirement rather than only edition 2024. Actual toolchain testing remains Rust 1.99.1.

### B. Manual verification reported by the user

The user explicitly reports the following results for the **fixed installed Debian build**. These were not independently executed or observed by Codex. The user did not supply a separate measured package hash, test date/runtime version log or integrity snapshot; do not invent them.

- [x] Fixed Debian package installed and tested.
- [x] Playback regression checks passed **6/6**, including pause → seek → resume → pause, repeated seeks, rapid playback toggles and track switching. The named cases are those supplied by the user; do not invent extra checklist labels to reach six.
- [x] WAV, MP3 and OGG produced audible output in the installed application.
- [x] Installed application worked without the Vite development server.
- [x] Scanning, search, tags, favorites, persistence and filesystem reauthorization passed.
- [x] Reinstalling the Debian package preserved existing application data.

These reports supersede earlier pending statements for those exact functional cases. They do not claim screen-reader testing, a manual before/after media integrity snapshot, an explicit uninstall/remove test, other distributions/architectures, AppImage support or acceptance of later packaging/asset changes. The user does not need to repeat already accepted functional checks merely to fill historical documentation gaps.

### C. Remaining findings and release decision

| Severity | Finding | Required resolution |
| --- | --- | --- |
| Blocker | Application icon is still the Tauri default logo. Its terms are separate from MIT and official trademark guidance says not to ship it as the application icon. | User decision requested: use the prepared original folder/waveform design or supply an authorized replacement; regenerate configured icons and rebuild/inspect the final Debian. |
| Blocker / rights decision | The two user-provided screenshot captures have no recorded image-license decision. Root MIT must not silently relicense them or the depicted third-party material. | User confirmation requested: apply MIT to their own image rights or retain an explicit separate permission/license for the existing README images. |
| Resolved and package-verified | The user-tested package omitted project/dependency license notices. | Audit candidate now contains project copyright, third-party texts, inventory, Rust library notices and five lock-verified original MPL source archives. |
| Warning | No AppImage exists; patchelf was unavailable during GStreamer bundling. AppImage would require a separate bundled-library/codec review. | Publish Debian only; provision and test AppImage separately if desired. This does not block a Debian-only release. |
| Warning | Only the recorded Zorin OS 18.1 x86_64 build/runtime is tested; glibc 2.39 is required. | Keep support claims narrow. Do not infer older-distribution or architecture support. |
| Optional improvement | No retained manual file-integrity snapshot, screen-reader record or explicit uninstall/remove test was supplied in this report. | Keep the reference procedures below and record these checks when performed; automated integrity tests remain separate evidence. |

**Verdict: BLOCKED** while the icon and image-rights decisions remain open. The application regression itself has user-reported acceptance, and MIT copyright attribution is established. Do not tag or publish a package merely because the application tests pass. Once distribution blockers are resolved, regenerate/check the final artifact and manifest, review the final diff/notice bundle and request explicit publication approval. No commit, tag, push, upload, system-package installation or data cleanup was performed.

### Audit candidate artifact — distribution still blocked

Production command executed from the repository root:

```sh
CARGO_TARGET_DIR="$PWD/src-tauri/target/pre-release-audit-20261104" RUSTFLAGS="--remap-path-prefix=$HOME=/build --remap-path-prefix=$PWD=/soundshelf" npm run tauri -- build --bundles deb -- --offline --locked
```

- Actual build output: `src-tauri/target/pre-release-audit-20261104/release/bundle/deb/soundshelf_0.1.0_amd64.deb`.
- Staged package and manifest: `src-tauri/target/pre-release-audit-20261104/release/release-assets/v0.1.0/` (only `.deb` and `SHA256SUMS`).
- Size: **3,473,178 bytes (3.31 MiB)**; package `soundshelf`, version `0.1.0`, architecture `amd64`; expected runtime dependencies unchanged.
- SHA-256: `e6cae9b0ef2ab35e862de52fdcf35d372c91340af82ed1c34f468edbe3532fbd`. Manifest verification passed against actual bytes.
- Extracted payload has **29 files** plus two control files. It includes the executable/desktop entry/three existing icons and intentional legal/source-compliance materials. No maintainer scripts remove data. Project license, third-party notice text and inventory match the source files; the five MPL archives match Cargo.lock hashes.
- The ELF has no debug sections, embeds the current `index-DLFp0Ojs.js` frontend, and imports at most GLIBC_2.39 symbols. Static scans detected no private home paths, known credential signatures, audio/database/environment/source-map files or copied shared libraries. This does not prove complete security or absence of arbitrary secrets.
- `apt-get --simulate install --reinstall` resolved the new local candidate successfully: one same-version `soundshelf` replacement, zero new packages and zero removals. This was a simulation, not installation.
- Unused starter SVGs are absent from the frontend distribution. The default Tauri application icons are still present deliberately as an unresolved blocker; do not mistake this candidate for an approved public release.
- The prior `50b24b...` pre-fix and `caa2c457...` corrected application candidates and their manifests are retained. User-reported installed acceptance applies to the fixed application candidate, not to independent installation of this new legal-packaging candidate.

After the icon/image-rights decisions are resolved, build into a fresh target directory (preserve these audit artifacts), inspect again and update the authoritative paths/size/hash before publication. New assets/legal packaging need a focused package smoke check; accepted application functionality need not be reclassified as untested merely because the packaging changed.

## Baseline and scope

- [x] Read `AGENTS.md`, `README.md`, `docs/PLAN.md`, and all four milestone records; inspect source, manifests, locks, capabilities and Git state.
- [x] Initial working tree clean; baseline HEAD `6ef608f` (`docs: finalize MVP documentation and add screenshots`). Remote: `https://github.com/shotadev93/soundshelf.git`.
- [x] Keep package.json, package-lock.json (root/package entry), Cargo.toml, Cargo.lock (application entry) and tauri.conf.json at `0.1.0`.
- [x] Keep identifier `dev.soundshelf.app` and `soundshelf.sqlite3` in the existing Tauri application-data directory. No migration or schema change.
- [x] Replace starter package description/author and window/document titles; use SoundShelf as window/document title, retain the existing `soundshelf` product/executable/package name. Keep existing valid PNG icons (32, 128 and 256 pixels) and cross-platform icon files. No new branding asset was created.
- [x] Restrict default bundle targets to Debian and AppImage; retain existing release optimization, LTO, abort-on-panic and stripping settings. Add Debian media dependencies and enable AppImage media framework bundling.
- [x] No new application features, dependency version changes, user data changes, commits, tags, pushes or uploads. `AGENTS.md` was modified separately during preparation; that user-owned change was read and preserved, not authored or reverted here.

## Build environment and reproducibility

Preparation date: 2026-11-04. Build host: Zorin OS 18.1 (Ubuntu noble base), Linux 7.0.0-42-generic, x86_64; glibc 2.39. GTK 3.24.41, WebKitGTK 2.52.6, libsoup 3.4.4, GStreamer tools 1.24.2. Rust/Cargo 1.99.1, Tauri CLI 2.12.4, Node 24.12.0, npm 11.21.1. Exact resolved dependencies are in the existing npm/Cargo lockfiles. Build tools include cc, ld, ar, strip, pkg-config, dpkg-deb and mksquashfs; standalone patchelf, linuxdeploy and appimagetool were not found on PATH, nor were Tauri AppImage helpers found in the existing cache. GStreamer runtime tools/plugins are installed; its development pkg-config file is absent (not needed for the Debian Rust build).

Rebuild from the reviewed source tree, not baseline HEAD alone. For a future post-blocker rebuild, use a fresh target directory (the example `final-reviewed-v0.1.0` must not already hold artifacts). On a compatible build host with prerequisites and dependencies provisioned:

```sh
# Repository root; npm ci requires registry access or a populated npm cache.
npm ci
npm test
npm run build
cd src-tauri
cargo fmt --check
cargo test --offline
cargo clippy --offline --all-targets -- -D warnings
cd ..
CARGO_TARGET_DIR="$PWD/src-tauri/target/final-reviewed-v0.1.0" RUSTFLAGS="--remap-path-prefix=$HOME=/build --remap-path-prefix=$PWD=/soundshelf" npm run tauri -- build --bundles deb -- --offline --locked
# Debian-only publication does not require retrying the currently blocked AppImage target.
```

After inspecting a successfully rebuilt Debian package, stage only that final package and generate a fresh manifest (do not blindly include old/intermediate bundles):

```sh
mkdir -p src-tauri/target/final-reviewed-v0.1.0/release/release-assets/v0.1.0
cp src-tauri/target/final-reviewed-v0.1.0/release/bundle/deb/soundshelf_0.1.0_amd64.deb src-tauri/target/final-reviewed-v0.1.0/release/release-assets/v0.1.0/
cd src-tauri/target/final-reviewed-v0.1.0/release/release-assets/v0.1.0
sha256sum soundshelf_0.1.0_amd64.deb > SHA256SUMS
sha256sum -c SHA256SUMS
cd -
```

Add an AppImage filename to the checksum command only after that format succeeds and passes inspection. Update release-note size/hash records after every rebuild. Use the fresh reviewed staging directory; keep all earlier artifact directories untouched. Only a successfully built and inspected package plus its manifest belong in the new staging directory.

Cargo offline mode requires the locked crates to be cached. The Tauri bundler can separately need network access for AppImage helpers; Cargo offline mode does not disable those downloads. Do not use sudo or weaken containment/WebKit security to complete a build. The path-remapping flags remove build-user paths from Rust diagnostic strings; keep them for final release builds and recheck the resulting payload. For checkout/home paths containing whitespace, use the equivalent CARGO_ENCODED_RUSTFLAGS form rather than whitespace-separated RUSTFLAGS. The detected executable requires GLIBC_2.39, so the Debian dependency explicitly requires libc6 >= 2.39 and libgcc-s1 as well. This is a repeatable build procedure, not a claim of byte-identical reproducible builds: host libraries, toolchains, bundler helpers and timestamps can affect output. Build on the oldest intended supported base and test it before adding distribution claims. See upstream [Debian](https://v2.tauri.app/distribute/debian/) and [AppImage](https://v2.tauri.app/distribute/appimage/) guidance.

## Historical initial preparation verification record

These initial counts/artifacts are historical. The final audit and current verdict at the top supersede them.

AppImage retry provenance: Tauri downloaded AppRun, linuxdeploy 1-alpha commit `07333c6` (build 368), and the AppImage/GTK/GStreamer helpers into its user cache. The linuxdeploy file SHA-256 was `bd9521cd5ff3ca351fecb78cd9b236a4e37b3621014ac05cac0c2d767ffa0474`; the GStreamer helper was `2a15ce9da8de6e20159e1ab27861a7a5ef8758c81a6278ba4ab30cefa1d74c9f`. These identify tooling used in the failed attempt, not distributable application artifacts. Continuous helper downloads can change; record versions/hashes again when retrying.

- [x] `cargo fmt --check`: passed.
- [x] `cargo test --offline`: 21 library tests and 44 integration-binary tests passed; one optional desktop test ignored by default. Integration tests repeat some private-core unit tests. Binary/doc targets contain no tests.
- [x] `cargo clippy --offline --all-targets -- -D warnings`: passed.
- [x] `npm test`: 35 passed.
- [x] `npm run build`: TypeScript/Vite production build passed.
- [x] Final Debian release build with path remapping and locked offline Cargo dependencies: passed; package `soundshelf`, version `0.1.0`, architecture `amd64`, 3,085,590 bytes.
- [x] Final extracted payload/control inspection and checksum verification: passed. Known credential/private-home-path patterns and media/database files absent; stripped ELF has no debug sections. The raw executable comparison initially differed; inspection confirmed only Tauri's expected three-byte DEB/UNK bundle marker. `apt-get --simulate install` resolved dependencies with one new package and no upgrades/removals; no installation occurred.
- [ ] AppImage: unavailable. Sandbox bundling hit a read-only filesystem; authorized retries reached linuxdeploy but failed because the GStreamer plugin requires `patchelf` (`exit code: 2`), which is absent. No system packages were installed. Keep `bundleMediaFramework` enabled; provision a suitable build host and retry before claiming this format.
- Optional native WebKit test and final packaging evidence: see the completed artifact/issue record in [v0.1.0 notes](releases/v0.1.0.md).
- [x] The user reports installing and accepting the corrected application candidate; see section B above. Final asset/legal packaging approval remains separate.
- [ ] Test any successfully built AppImage independently.
- [ ] Optional evidence completion: retain runtime logs, measured installed-artifact checksum, screen-reader observations and a manual integrity comparison when available. Current user-reported audible/functional acceptance is already recorded in section B.

### Playback duration follow-up

The original `invalid duration 0` failure was reproduced with event diagnostics and traced to a test assertion on transient MP3 metadata. Corrected tests require a finite positive duration within the unchanged 0.3-second tolerance, actual playback advancement, stable pause and successful seek/resume. Full/range HTTP bodies and unchanged bytes are now checked for all three codecs. See the [investigation record](PLAYBACK_DURATION_INVESTIGATION.md), including initial failed experiments, fixture metadata and remaining GUI checks.

- [x] Corrected debug desktop probe: all three formats from both development and production document origins passed in two consecutive runs (12 successful format/origin cases in total); the first run included zero metadata recovering at canplay.
- [x] Deterministic duration-gate/controller regressions: five additions, bringing `npm test` to 40 passes. Persistent zero, NaN, infinity, wrong duration and media errors remain failures.
- [x] `cargo fmt --check`, `cargo test --offline` (21 library + 44 integration tests, one desktop test ignored by default), offline Clippy with warnings denied and `npm run build`: passed during the investigation.
- [x] Release-profile loopback route test: passed, including explicit rejection of the development Origin. No authorization/CSP/containment behavior changed.
- [x] Release-profile native desktop probe: all three formats from the production document origin passed. Together with the two debug runs, the corrected probe completed 15 format/origin cases. The probe uses muted audio and does not establish audible output or full application GUI acceptance.

Run the corrected probes from the repository root in a working desktop session:

```sh
cargo test --offline --manifest-path src-tauri/Cargo.toml --lib native_webkit_plays_pauses_and_seeks_loopback_audio -- --ignored --test-threads=1 --nocapture
RUSTFLAGS="--remap-path-prefix=$HOME=/build --remap-path-prefix=$PWD=/soundshelf" cargo test --release --offline --manifest-path src-tauri/Cargo.toml --lib native_webkit_plays_pauses_and_seeks_loopback_audio -- --ignored --test-threads=1 --nocapture
RUSTFLAGS="--remap-path-prefix=$HOME=/build --remap-path-prefix=$PWD=/soundshelf" cargo test --release --offline --manifest-path src-tauri/Cargo.toml --lib loopback_route_rejects_remote_hosts_origins_and_arbitrary_paths
```

The original release-preparation package and SHA256SUMS remain unchanged. This initial duration investigation modified test-only Rust/JavaScript and documentation, not application behavior. The subsequent user-reported installed-package pause regression is tracked separately below. AppImage remains unavailable due to the separately recorded missing-patchelf issue. Add the investigation/test files to the maintainer's reviewed commit selection when preparing the eventual release commit.

### Pause after seek/resume regression

The user confirmed the original installed Debian package cannot reliably pause after play → pause → seek → resume. Before changing production code, deterministic controller tests and a WebKit run using the actual application controller reproduced the failure. Pending play promises/readiness were conflated with the ability to pause; old same-track promises and queued events could also overwrite newer intent. See the [root cause and evidence](PLAYBACK_DURATION_INVESTIGATION.md#confirmed-application-pause-regression).

The frontend now separates `playRequested` from loading/readiness and gives each play attempt its own cancellation guard. Pause stays available while play() is pending or buffering. Queued events are checked against current media state, while real errors still stop playback and are reported. Backend authorization, CSP and IPC are unchanged. No new dependency was added.

- [x] Four new controller regressions pass, including the exact reported sequence and repeated rapid actions with out-of-order promise results. `npm test`: 44 passed.
- [x] `cargo fmt --check`, offline Rust tests (21 library + 44 integration-binary passes, one desktop test ignored by default), offline Clippy with warnings denied, and `npm run build`: passed.
- [x] Upgraded native WebKit probe using the actual controller: all six debug codec/origin cases passed, including repeated pause/seek/resume and stable time after pausing again.
- [x] Release-profile native WebKit probe using the actual controller: all three production-origin codec cases passed, with the exact sequence and rapid repetitions. Across debug and release, nine format/origin cases passed. Tests are muted and do not establish rendered React GUI or audible acceptance.
- [x] Corrected Debian production build (offline, locked, path remapping, independent target directory): passed. Output: `src-tauri/target/pause-seek-fix-20261104/release/bundle/deb/soundshelf_0.1.0_amd64.deb`, 3,085,674 bytes (2.94 MiB).
- [x] Corrected package inspection: five payload files and two control files; no media/database/environment files, private home paths, known credential signatures or ELF debug sections detected. The packaged ELF differs from the working build only by Tauri's expected DEB/UNK marker. SHA256SUMS verification passed.
- [x] `apt-get --simulate install --reinstall` for the corrected package: dependency resolution succeeded, with zero upgrades/removals/new packages and one reinstall. This was a simulation; the corrected package was not installed by the coding agent.
- [x] User-reported retest of the corrected installed application: playback 6/6, audible three-format output, no-Vite use, library features, persistence/reauthorization and data-preserving reinstall passed. Not independently executed by Codex.

The original `.deb` and its SHA256SUMS are retained unchanged as historical evidence, **not a publishable fixed package**. The corrected candidate is built in a separate target directory, `src-tauri/target/pause-seek-fix-20261104`, so it does not replace or delete the original release artifacts. Record the new artifact checksum before installing; both packages still report version `0.1.0`, so the checksum distinguishes them.

The corrected package and its new `SHA256SUMS` are staged in `src-tauri/target/pause-seek-fix-20261104/release/release-assets/v0.1.0/`. The new checksum is:

```text
caa2c45780ca3b6578d3097ae3a96ca269bf0a7a898a7ce686453cbbcd1e93e1  soundshelf_0.1.0_amd64.deb
```

The original files still match `50b24b34561750ac813b7fa7e2f9fcb8890496a46aedf624356df045f2374619`. Neither old nor corrected package was installed by the coding agent. The new candidate needs the manual retest below; a successful build and muted controller probe do not establish installation acceptance.

Required manual retest, using disposable WAV/MP3/OGG tones and isolated XDG data/config/cache as described below:

1. Close every running SoundShelf instance. Verify the corrected candidate's SHA256SUMS, then install that exact file with the graphical installer or `pkexec apt install --reinstall "$(realpath path/to/corrected/soundshelf_0.1.0_amd64.deb)"`. The reinstall option is needed because the original and corrected candidates share version 0.1.0. Do not remove application data. Record the checksum and installed package version.
2. Stop Vite and launch `/usr/bin/soundshelf` with the isolated XDG variables. Select/scan the synthetic directory through the native picker.
3. For **each format**, play until time advances, pause, seek forwards while paused, resume, then pause again. Check the button stays enabled and reads **Pause** while a play request is pending/loading. Audible output and currentTime must stop; waiting a second must not produce a false error or restart sound. Resume again and repeat with backwards and near-end seeking.
4. Repeat the full sequence at least ten times per format. Rapidly alternate Play/Pause and several seek positions, including clicking Pause immediately after Resume. The final action must win; no stuck disabled button, stale error or unintended restart may occur. Repeat using keyboard controls.
5. Repeat while buffering/loading if observed. Test Stop and switching tracks/libraries during pending play; old completions must not restart or overwrite the current selection. Confirm corrupt/missing media still produces a recoverable error and a valid file can play afterward.
6. Close/restart using the same isolated data directory. Confirm metadata persistence and native-picker reauthorization. Compare untouched fixture bytes/names/modes/mtimes with the baseline. Check light/dark focus and seek controls. Record audible results separately from muted automated tests.
7. Repeat the exact sequence in the development application with a separate isolated data directory. Complete the existing non-destructive uninstall/reinstall checks before approving publication. Do not delete or publish historical release artifacts.

## Distribution and security review

Production CSP permits bundled content, Tauri IPC and only the `http://127.0.0.1:*/audio/` media route. Development Vite allowances remain confined to `devCsp`. There is no registered custom audio URI scheme in production: Linux playback uses a libsoup IPv4 loopback listener because WebKitGTK does not decode the previous custom scheme. The route checks hosts/origins, uses opaque expiring tokens and revalidates authorization/file identity. The main-window capability grants only the twelve explicit application commands; no general filesystem, shell, opener, remote API or broad asset-protocol access is granted.

Native picker authorization and openat2 beneath-root/no-symlink/no-cross-mount containment remain unchanged. Restart restores saved entries inactive. Annotation writes target SQLite only. No cloud service, telemetry, account or external runtime API was added. Local loopback traffic is required for playback. A local process possessing an active bearer token can read that authorized stream; already buffered audio cannot be revoked. This review and regression suite are not an independent security audit.

The Debian payload contains compiled frontend assets, the stripped production executable, desktop entry, icons and intentional legal/source-compliance materials under `/usr/share/doc/soundshelf/`; no extra application resources, media sidecars or external binaries are configured. Inspect actual payload and metadata before publication, including SQLite/media/credentials and build-machine path strings. Do not upload `target/debug`, an AppDir, generated fixtures, the database or the entire target directory. Upload only reviewed final packages plus `SHA256SUMS`.

No project LICENSE file was present at the original baseline. The user has now selected MIT; the complete root LICENSE and manifest fields implement that decision for SoundShelf-authored code. Third-party terms are separately preserved in the license review/notices. The original icon replacement and MIT licensing of the user-owned screenshot image rights were explicitly approved and implemented; see the follow-up at the top. A future AppImage requires its own bundled-runtime/codec review; successful bundling alone would not resolve those obligations.

## Manual packaged Linux acceptance reference

Use an unprivileged desktop user and disposable synthetic audio. Close all SoundShelf instances before each run; one instance owns an index. Stop any Vite/Tauri development process normally and confirm no server is listening on 1420/1421 (`ss -ltn`). Do not kill unrelated applications. Record commands/output and exceptions outside the selected audio directory.

### 1. Verify and install Debian

From the repository root (or substitute the directory containing the reviewed artifacts):

```sh
cd src-tauri/target/approved-assets-20261104/release/release-assets/v0.1.0
sha256sum -c SHA256SUMS
dpkg-deb --info soundshelf_0.1.0_amd64.deb
pkexec apt install --reinstall "$(realpath soundshelf_0.1.0_amd64.deb)"
dpkg-query -W -f='${Package} ${Version} ${Architecture}\n' soundshelf
cd -
```

The `pkexec` command is for the human tester's authenticated package-manager session; it has not been executed by the coding agent. A graphical package installer can be used if the system does not permit pkexec apt. Resolve dependencies using the distribution package manager. Do not force dependency installation failures or assume support for an untested distribution.

### 2. Isolate data and prepare integrity evidence

```sh
soundshelf_acceptance=$(mktemp -d /tmp/soundshelf-release-acceptance-XXXXXX)
mkdir -p "$soundshelf_acceptance/data" "$soundshelf_acceptance/config" "$soundshelf_acceptance/cache"
python3 scripts/create-playback-fixtures.py
# Paste ONLY the newly printed audio-directory path:
read -r soundshelf_audio_root
```

Keep this shell and the generated directories for restart/reinstall checks. The generator supplies WAV, MP3 and OGG tones, corrupt/empty candidates, and an outside-root symlink. Do not select a personal library. Create a snapshot helper outside the audio root:

```sh
cat > "$soundshelf_acceptance/snapshot.py" <<'PY'
import hashlib, json, os, stat, sys
from pathlib import Path
root = Path(sys.argv[1])
result = {}
for parent, directories, files in os.walk(root, followlinks=False):
    for name in directories + files:
        path = Path(parent) / name
        info = path.lstat()
        entry = {"mode": info.st_mode, "mtimeNs": info.st_mtime_ns}
        if stat.S_ISREG(info.st_mode):
            entry["sha256"] = hashlib.sha256(path.read_bytes()).hexdigest()
        elif stat.S_ISLNK(info.st_mode):
            entry["target"] = os.readlink(path)
        result[str(path.relative_to(root))] = entry
print(json.dumps(result, sort_keys=True, indent=2))
PY
python3 "$soundshelf_acceptance/snapshot.py" "$soundshelf_audio_root" > "$soundshelf_acceptance/before.json"
XDG_DATA_HOME="$soundshelf_acceptance/data" XDG_CONFIG_HOME="$soundshelf_acceptance/config" XDG_CACHE_HOME="$soundshelf_acceptance/cache" /usr/bin/soundshelf
```

Run without Vite or an internet connection. Confirm bundled UI opens, application identity/icons appear, no CSP/IPC failures occur and directory selection works. Node, npm and Cargo must not be needed to run the installed application. For a desktop-menu launch, separately verify the launcher uses the installed binary; avoid indexing personal files in its normal data directory.

### 3. Exercise the installed application

1. Confirm the empty state and picker cancellation. Select the printed audio root through **Select audio directory**, then **Start scan**. Nested and mixed-case WAV/MP3/OGG candidates appear, the symlink is skipped, and scanning/loading/error states remain usable.
2. At low volume play each valid tone and explicitly confirm audible WAV, MP3 and OGG output and duration. Pause/resume, seek forwards/backwards near the middle/end and replay. Switch WAV → MP3 → OGG while playing/loading: only the current selection plays. Stop works. Corrupt/empty candidates show recoverable errors; a valid tone can then play. Record format-specific errors and runtime versions.
3. Search filenames and relative paths. Create/rename/delete tags, assign/remove tags, toggle favorites, search by a tag, and combine Favorites only with multiple tag filters. Only assets with every selected tag match. Clear filters and confirm counts and empty/no-match states. For pagination, use a separate disposable fixture set with more than 50 candidates and snapshot that set before testing.
4. Use Tab/Shift+Tab, Enter/Space and seek arrow/Home/End keys throughout. Check visible focus, native picker, status/errors and light/dark appearance. Record any screen-reader observations separately.
5. Close the app normally and check the isolated index exists at `$soundshelf_acceptance/data/dev.soundshelf.app/soundshelf.sqlite3`, outside the audio root. Relaunch with the exact same XDG command. Saved records, tags and favorites persist. Roots must be inactive; search and annotation editing work, while scans/playback require native picker reselection. Reselect the same root, rescan and confirm no duplicate records or lost annotations.
6. After closing, inspect the isolated database read-only (Python standard library, no sqlite3 CLI dependency):

   ```sh
   python3 - "$soundshelf_acceptance/data/dev.soundshelf.app/soundshelf.sqlite3" <<'PY'
   import sqlite3, sys
   from pathlib import Path
   with sqlite3.connect(Path(sys.argv[1]).as_uri() + "?mode=ro", uri=True) as db:
       print("integrity:", db.execute("PRAGMA integrity_check").fetchall())
       print("schema:", db.execute("PRAGMA user_version").fetchone())
       for table in ("libraries", "assets", "tags", "asset_tags"):
           print(table, db.execute("SELECT count(*) FROM " + table).fetchone()[0])
   PY
   ```

7. Compare the original fixture snapshot after scanning, playback, metadata changes and restart:

   ```sh
   python3 "$soundshelf_acceptance/snapshot.py" "$soundshelf_audio_root" > "$soundshelf_acceptance/after.json"
   diff -u "$soundshelf_acceptance/before.json" "$soundshelf_acceptance/after.json"
   ```

   Expect no difference: original bytes, file/directory names, modes, mtimes and symlink targets remain unchanged; no sidecars appear. Atime is intentionally excluded. Test missing/unreadable/replaced media only on a separate disposable copy so deliberate external mutations do not invalidate this evidence. Confirm unavailable records preserve tags/favorites and do not crash the app.

### 4. AppImage acceptance, only when a reviewed AppImage exists

No AppImage was produced during this preparation: the GStreamer bundling helper could not find patchelf. The following is a future conditional procedure, not an acceptance result or an assertion that a download exists.

No AppImage run is accepted merely because Debian passed. Check its own SHA256SUMS, then `chmod u+x ./soundshelf_0.1.0_amd64.AppImage` (substitute the actual generated filename) and launch it with a **new** isolated XDG data/config/cache directory. Repeat the entire flow above without Vite, including each codec, audible output, seeking, persistence, reauthorization and integrity comparison. If FUSE is unavailable, try its supported `--appimage-extract-and-run` mode and record that distinction. Do not disable the WebKit sandbox. Record missing runtime requirements; do not claim universal portability.

### 5. Uninstall/reinstall without deleting data

With SoundShelf closed, retain the isolated test directories and take a copy of the test database outside its application-data directory. Record tag/favorite counts. Authenticate `pkexec apt remove soundshelf` and verify `dpkg-query -W -f='${db:Status-Status}\n' soundshelf` reports a non-installed state (or the package is absent); do not purge or remove any XDG directory. Inspect that the isolated database remains. Reinstall the same reviewed Debian file using step 1 and launch using the same isolated XDG variables. Metadata must persist and saved roots must again require picker reauthorization. Repeat the integrity comparison. Keep logs/fixtures until reviewed; do not delete personal data or run broad cleanup commands.

## Publication gate (maintainer only; not executed)

- [ ] Review final diff, artifact contents/checksums, license/redistribution decision and any packaging issues.
- [ ] Complete and record installation/reinstallation acceptance on each format intended for upload.
- [ ] Commit the reviewed release preparation explicitly; rebuild from that exact commit if build inputs changed. Update recorded checksums whenever artifacts change.
- [ ] Confirm `v0.1.0` does not already exist locally/remotely and that the intended commit is checked out.
- [ ] Only after approval, create/push the annotated tag and create the release with the exact reviewed assets (commands in the final preparation report).
- [ ] Replace draft/preparation wording in release notes with actual publication/acceptance facts when appropriate; never invent download URLs or supported distributions.

After all gates pass and publishing is explicitly approved, the maintainer can use the following commands from the repository root. They were **not executed** during preparation. Review the separately modified `AGENTS.md` independently rather than automatically including it in the release commit.

```sh
git diff --check
git add -A -- LICENSE THIRD_PARTY_NOTICES.txt licenses README.md docs/PLAN.md docs/THIRD_PARTY_LICENSE_REVIEW.md package.json package-lock.json index.html public/tauri.svg public/vite.svg src/assets/react.svg src-tauri/Cargo.toml src-tauri/tauri.conf.json src-tauri/icons src-tauri/src/playback_protocol.rs src-tauri/tests/support/webkit-playback.mjs src/lib/playback-controller.ts src/components/AudioPlayer.tsx src/lib/playback-controller.test.mjs src/lib/webkit-playback.test.mjs scripts/build-playback-test-module.mjs scripts/generate-third-party-notices.py docs/PLAYBACK_DURATION_INVESTIGATION.md docs/RELEASE_CHECKLIST.md docs/releases/v0.1.0.md
git commit -m "chore: prepare SoundShelf v0.1.0 Linux release"
git status --short
git tag --list v0.1.0
git ls-remote --tags origin refs/tags/v0.1.0
# Stop if the tag exists or the intended release inputs differ from the reviewed artifacts.
# Record the intended commit; use a clean reviewed checkout if unrelated changes remain.
git rev-parse HEAD
git tag -a v0.1.0 -m "SoundShelf v0.1.0"
git push origin HEAD
git push origin refs/tags/v0.1.0
# Execute only after final package smoke acceptance and explicit publication approval.
gh release create v0.1.0 --verify-tag --repo shotadev93/soundshelf --title "SoundShelf v0.1.0" --notes-file docs/releases/v0.1.0.md src-tauri/target/approved-assets-20261104/release/release-assets/v0.1.0/soundshelf_0.1.0_amd64.deb src-tauri/target/approved-assets-20261104/release/release-assets/v0.1.0/SHA256SUMS
```

The command intentionally lists only the successfully built Debian package and its manifest. Add an AppImage only after it is successfully built, inspected, checksummed and manually accepted; regenerate a flat `SHA256SUMS` covering every uploaded package. Review publication wording before executing `gh release create`. The GitHub CLI (`gh`) is not installed in this environment; run that step on an authenticated publishing machine with the CLI available, or create the release through GitHub’s web interface with the same tag, notes and exact two reviewed files. No CLI installation was attempted.
