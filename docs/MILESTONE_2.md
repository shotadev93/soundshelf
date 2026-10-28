# Persistent, searchable library

Milestone 2 implements SQLite indexing, scan reconciliation, saved-library browsing, and filename/path search. Automated checks pass. User-reported Linux GUI testing confirms scanning, folder selection, clearing, saved-library selection, and English/Japanese search. GUI results and the completed Saved Library contrast follow-up are recorded below. Playback and metadata editing have not been implemented; Milestone 3 has not started.

## Phase reviews

### 2A — Database

`db.rs` owns a single `rusqlite::Connection`, shared through `Arc<Mutex<Database>>`. `rusqlite` 0.41 with the `bundled` feature supplies a maintained SQLite binding and a consistent SQLite implementation without a new system SQLite prerequisite. See the [upstream documentation](https://docs.rs/rusqlite/0.41.0/rusqlite/). No frontend dependency was added.

Tauri resolves its application-data directory at startup, creates it if needed, and opens `soundshelf.sqlite3` there. The Linux location is normally `$XDG_DATA_HOME/dev.soundshelf.app`, with Tauri's platform fallback when the variable is unset. The database is independent of selected audio directories. Database files and SQLite journal companions are ignored by Git. Database initialization errors stop startup with a generic diagnostic and a failure exit code instead of a panic.

`migrations/001_initial.sql` defines:

- `libraries`: persistent opaque ID, unique canonical root bytes, display path, UTC creation/last-success timestamps, latest scan ID and phase.
- `assets`: persistent opaque ID, library foreign key, native relative path, display name/path, extension, byte size, modification time, availability, last-seen scan ID, and favorite flag. `(library_id, relative_path)` is unique.
- `tags`: opaque ID, unique normalized name, and display name.
- `asset_tags`: foreign keys and a composite primary key.

Root and relative paths are BLOBs containing native Linux bytes. Display strings are lossy UTF-8 text used only for presentation/search. Paths that display identically can remain distinct. Byte sizes and modification seconds are decimal strings; nanoseconds are stored separately. Creation and successful-scan timestamps are UTC strings.

Every application connection enables foreign keys and a bounded busy timeout. `PRAGMA user_version` selects ordered migrations; all pending migrations and version updates run in one transaction. A failed migration rolls back schema and version changes. A database from a newer schema is rejected. SQL values are bound parameters; dynamically composed query fragments are fixed application-owned SQL, never user input.

Review and tests cover fresh creation, reopening, retained rows, schema upgrades, failed-migration rollback, newer-schema rejection, and foreign-key rejection.

### 2B — Persistence

The existing `openat2` scanner and its containment flags are unchanged. Selection still uses the trusted native picker. A selected canonical root reconnects its persistent library ID; the database never authorizes a path. The session authorization map starts empty, and saved libraries appear inactive after restart. Index queries remain available while inactive, but scans require picker authorization. Revoked directory handles remain revoked even after the same root is explicitly reselected.

Before traversal, a short database operation loads existing native-path/ID pairs. The connection lock is then released. Discovery opens original resources read-only through the existing authorized-root handles. After traversal and root validation, a transaction reconciles the collected results. A test queries the index at traversal hooks to verify that traversal holds no database lock. Scan start, reconciliation, and queries run on blocking workers; React does not access SQLite or the filesystem.

Upserts retain IDs and annotations at unchanged relative paths, including when a missing path reappears. They update observed file metadata without duplicating rows. Complete scans mark unseen indexed paths `missing` and advance the last-success timestamp. Any warning makes the entire scan partial: observed candidates may be updated, but unseen records retain their previous metadata and availability. Failed discovery publishes no candidates. A failed reconciliation rolls back all asset changes, and a subsequent scan can recover. Nothing automatically prunes unavailable records.

The latest scan ID and phase distinguish complete, partial, running, and failed attempts. Startup converts interrupted `running` attempts to `failed` without changing asset availability. Scan IDs guard reconciliation and failure publication so a revoked old worker cannot overwrite a newer scan after reselection. Detailed warnings and progress remain session data; the UI does not replay them after restart. `AssetPage.scanId` identifies the latest attempt, while individual rows can retain observations from older scans.

Metadata is path-based: replacing a file at the same relative path retains that path's metadata. A moved file creates a new entry and leaves the old record unavailable after a complete scan. No content matching or move reassociation is performed. Annotations seeded directly into test databases survive rescans and unavailable states, but there is no annotation command or editor in this milestone.

Review and integration tests cover repeated scans, stable IDs, disappearance/reappearance, partial and failed scans, unavailable roots, transaction failure, interrupted-scan recovery, stale workers, inactive restart state, trusted reselection, and lossless non-UTF-8 roots/relative paths. Existing containment, deterministic symlink-race, permission-denied, and file-integrity tests still run.

### 2C — Search and UI

The main-window capability adds only `list_libraries`; `list_assets` now accepts an explicit paginated request:

```ts
{ libraryId: string, search: string, offset: number, limit: number }
```

It returns `{ scanId, assets, total, offset, limit }`. `list_libraries` returns saved library information and an in-memory `active` flag. Neither query grants filesystem access. Rust DTOs, TypeScript interfaces/runtime parsers, and shared serialization fixtures were updated together. Legacy discovery snapshots exist only in the Rust test harness; the production asset command is paginated.

Filename and relative display-path search use parameterized SQLite `LIKE`. Backslash, percent, and underscore are escaped as literal characters. Quotes and SQL-looking text remain ordinary input. Matching is substring-based and ASCII case-insensitive; SQLite's default `LIKE` does not provide full Unicode case folding or Unicode normalization. Tag search is deferred with metadata editing.

Requests require a limit of 1–200, an offset of 0–1,000,000, and a search of at most 1,024 UTF-8 bytes without NUL characters. Results sort by binary display path, then persistent asset ID. Count and page are read within one transaction. Separate requests can observe intervening completed scans; pagination is not a cross-request snapshot.

React loads saved libraries on startup, offers explicit picker reauthorization, shows all four availability labels, and searches on form submission. Pages contain 50 assets. Loading, no libraries, empty index, no matches, partial scans, query errors, retry, and inactive/unavailable roots have visible states. Navigation and query generations reject late responses from previous libraries/searches. A shrinking last page returns to a valid page. Scan failures preserve the stored view. Help text explains historical availability, literal matching, and path-based metadata identity.

Review and tests cover request/response fixtures, nullable fields, all availability/error variants, SQL input, literal wildcards/backslashes, non-ASCII names, deterministic pages, invalid bounds, stale responses, query recovery, restart browsing, reauthorization, and picker cancellation during scans.

## Automated verification

Executed from `src-tauri/`:

```sh
cargo fmt
cargo fmt --check
cargo test --offline
cargo clippy --offline --all-targets -- -D warnings
```

Executed from the repository root:

```sh
npm test
npm run build
npm run tauri -- build --debug --no-bundle -- --offline
```

Results: formatting passed; Rust tests passed (4 unit tests and 27 tests in the private-core integration binary, which also runs three core unit tests); Clippy passed with warnings denied; all 14 Node tests passed; TypeScript/Vite production build passed. The Tauri debug executable build with bundled frontend assets also passed. Tauri emitted the existing warning that the identifier ends in `.app` (a macOS bundle-name recommendation); the Linux identifier was left unchanged.

Tests use disposable synthetic audio trees and temporary or in-memory databases, never personal libraries. Permission-denied tests require an unprivileged user and assert actual OS access denial. Integrity snapshots compare SHA-256 hashes, names, modes, and modification times and check that no sidecars appear in audio directories. Normal reads may update OS access times. Restart tests discard application state/connections and reopen an on-disk index; this is a simulated application restart, not a GUI end-to-end test.

The original Milestone 2 implementation session did not perform native GUI interaction. The user subsequently reported the successful GUI checks below. Automated controller tests simulate native responses and do not establish native popup rendering, keyboard focus, or full WebView IPC/CSP acceptance. The earlier GTK startup limitation is historical and does not negate the user-reported results.

## GUI verification and Saved Library contrast follow-up

The user confirmed that scanning, folder selection, clearing, saved-library selection, and English/Japanese search work correctly in Linux GUI testing. The user subsequently completed post-fix visual testing and judged the overall appearance acceptable, with no issues reported. The Saved Library styling fix is visually accepted. These are user-reported results, not an automated desktop test; runtime versions and individual results for the broader acceptance checklist were not supplied.

The reported visual defect was white text on a white background in the Saved Library selection area. `App.tsx` uses a native controlled `select`; its earlier CSS inherited the page's text color, left the background transparent, and did not declare a native color scheme. In dark mode, this could combine light text with a light native surface.

The fix is confined to `App.css`, a stylesheet regression test, and this document. The existing native select, label, keyboard behavior, and change handler are preserved. Scoped variables reuse the application's existing text, button-surface, hover, and blue-focus palette, with matching light/dark `color-scheme` declarations. The select and unselected options now receive explicit foreground/background pairs. Selected options use a contrasting blue surface and semibold text, hovered options have a separate surface (including selected-and-hovered), and the closed control has a hover border and a three-pixel keyboard-focus outline with an offset. Native form controls use `color-scheme` to follow the intended theme; see [WebKit's dark-mode guidance](https://webkit.org/blog/8840/dark-mode-support-in-webkit/).

Authored text contrast ratios, calculated using sRGB relative luminance:

| Theme | Unselected / closed | Hover | Selected | Selected + hover |
| --- | --- | --- | --- | --- |
| Light | 16.45:1 | 14.10:1 | 5.32:1 | 6.94:1 |
| Dark | 11.81:1 | 8.64:1 | 9.26:1 | 11.49:1 |

All authored text pairs exceed the [WCAG AA normal-text minimum of 4.5:1](https://www.w3.org/WAI/WCAG21/Understanding/contrast-minimum). `library-styles.test.mjs` checks these thresholds, distinct interaction colors, and at least 3:1 contrast for the control border against its surface and focus outline against the page. It uses the existing Node test runner with no new dependencies. These checks validate the authored palettes, not rendered pixels. [Native option styling varies by browser and operating system](https://developer.mozilla.org/en-US/docs/Web/HTML/Reference/Elements/option), so GTK/WebKit may render popup hover/keyboard highlights using native theme colors rather than every authored option rule.

Follow-up verification: `npm test` passed all 16 tests (including two new theme-contrast tests), and `npm run build` passed. The repository-required `cargo fmt --check`, `cargo test --offline` (4 unit tests and 27 integration-binary tests), and `cargo clippy --offline --all-targets -- -D warnings` also passed without Rust source changes. The user completed the post-fix visual recheck and reported no overall issues. This records user acceptance of the corrected appearance; the coding agent did not independently operate the native GUI. Rust, SQLite, filesystem security, and IPC behavior were not changed by this styling fix.

## Manual Linux GUI acceptance checklist

Use a disposable fixture directory and an isolated test application-data location. Do not use a personal audio library. Test both development and the built executable (`src-tauri/target/debug/soundshelf`).

1. Start with an empty test application-data directory. Verify the empty state, picker cancellation, selection, scan progress, and populated results. Confirm the SQLite file is in application data and no database or sidecar appears in the selected directory.
2. Include nested/mixed-case WAV, MP3, and OGG candidates, non-ASCII names, literal `%`, `_`, backslash, and quote characters, and more than 50 candidates. Submit searches with Enter and the Search button, clear them, and navigate first/last pages. Verify counts, stable ordering, no-match messages, and visible loading states.
3. Restart the application. Confirm stored results can be searched while the library is inactive, scanning is disabled, and native picker reselection reconnects the same records without duplicates. Canceling reselection must preserve the view.
4. Remove or rename only disposable files externally, then run a complete scan. Verify missing records remain visible. Recreate a fixture at the same relative path and verify its ID is retained. Move a fixture to another path and verify it produces a new entry.
5. Make a disposable subtree unreadable as an unprivileged user, then rescan. Confirm a partial warning and retained unseen records, with no false missing states. Restore fixture permissions and verify recovery. Remove/replace a selected root and confirm scanning requires reselection while stored results remain searchable.
6. Switch saved libraries while scans/queries run. Search and change pages rapidly, including a page that shrinks after a rescan. Confirm stale responses do not replace the current results and errors offer a usable retry.
7. Use only the keyboard for picker access, saved-library selection, search, Clear, page buttons, and retry. Check focus visibility, screen-reader status announcements, long paths, light/dark appearance, and window/input responsiveness during scanning.
8. Run the bundled executable without Vite. Check selection → scan → search → restart → reauthorize, and confirm no capability or production CSP errors. Compare fixture hashes and directory contents before/after. Playback is unavailable and is not part of this milestone's checks.

## Limits

Scanner resource limits and filesystem security scope remain as documented in Milestone 1. This is not an atomic filesystem snapshot or watcher. Partial scans conservatively avoid marking any unseen record missing, even in subtrees that were readable. Inactive libraries show historical availability rather than probing saved paths. Detailed scan warnings are not persisted. Search is bounded but uses ordinary `LIKE`, not FTS; no performance claim is made for an unmeasured library size. The current application assumes one owning application instance per index. The user-reported functional GUI checks and post-fix visual acceptance above are complete. Individual results for the broader manual acceptance checklist have not been recorded.
