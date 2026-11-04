# Tags, favorites, and MVP implementation

Milestone 4 implements the remaining MVP features: persistent tag management, asset annotations, combined search/filtering, and accessible editing controls. Automated verification and the real WebKitGTK playback regression pass. Full development/bundled application GUI acceptance, keyboard/screen-reader interaction, and audible playback still require the manual checks below. This is an implementation completion record, not a claim that all native acceptance gates have passed.

## 4A — Tag management

Tags are global to the local index. `list_tags`, `create_tag`, `rename_tag`, and `delete_tag` are explicit, main-window-only commands dispatched to blocking workers. They use the existing `tags` and `asset_tags` tables; schema version 1 remains unchanged. No migration or dependency was needed.

Names are trimmed using Rust Unicode whitespace rules. The resulting name must contain 1–64 Unicode scalar values and no control characters. Interior whitespace is preserved. The display name preserves case; `str::to_lowercase()` supplies the unique normalized key. This is Unicode lowercase mapping, not full case folding or NFC/NFKC normalization: composed/decomposed characters and some case-fold equivalents can remain distinct. A case-only rename of the same tag succeeds. Duplicate names return `duplicateTag`, invalid names return `invalidInput`, absent IDs return `notFound`, and database failures return a generic `databaseError` without raw SQL/path details.

Create, rename, and delete run in transactions. Deletion removes associations and the tag atomically, keeping assets and favorites. A failed delete restores its associations. The unique normalized-name constraint and application database lock prevent ordinary duplicate creation. Values are bound SQL parameters. Tag names containing quotes, `%`, `_`, backslashes, Japanese text, and SQL-looking strings remain data.

## 4B — Asset annotations

`set_asset_tags` accepts `{ libraryId, assetId, tagIds }`; `set_asset_favorite` accepts `{ libraryId, assetId, favorite }`. Requests reject unknown fields. Tags are replaced as one transaction, after checking asset/library membership and all tag IDs. Empty selection removes every tag. At most 32 distinct tags are allowed per asset; duplicate IDs, empty/oversized IDs and NULs are invalid. Associations also have a composite primary key. Favorite setting is explicit and idempotent.

Metadata operations address stable indexed asset IDs and never open audio or reconstruct paths. They work for inactive saved libraries, missing files, and inaccessible records. This does not grant filesystem authority. Scan reconciliation still updates observed file fields without overwriting favorites or associations. Restart retains annotations but clears directory authorization. Metadata remains path-based: replacing a file at the same relative path keeps its metadata; moving a file produces a different entry without automatic reassociation.

Only the application-data SQLite index is written. No embedded tags, sidecars, media bytes, names, modes, or intentional timestamps are written. Normal OS reads may update atime.

## 4C — Search and filters

`AssetQuery` now requires `{ libraryId, search, tagIds, favoritesOnly, offset, limit }`. Every returned asset includes `favorite` and ordered `tags: [{ id, name }]`. Rust DTOs, TypeScript interfaces, runtime response parsers, JSON fixtures, and contract tests change together.

Search retains literal filename/path substring matching and adds tag display names. Backslash, `%`, and `_` are escaped before parameterized `LIKE`. SQLite matching remains ASCII case-insensitive without full Unicode case folding. Tag duplicate normalization and free-text search intentionally have different case semantics. Selecting tag IDs matches exact IDs and requires **all** selected tags; favorites-only is an additional AND condition. Filters do not depend on session root access.

Correlated `EXISTS` predicates avoid duplicate asset rows when multiple tags match. The count, page, and annotations are read within one transaction. Sorting remains binary display path followed by stable asset ID. Existing bounds remain: 1–200 rows, offset up to 1,000,000, search up to 1,024 UTF-8 bytes without NULs. Tag-filter IDs use the same 32-distinct-ID bound. An unknown filter ID returns no matches, rather than granting access or dropping the filter. Separate page requests can observe intervening edits/scans; they are not a cross-request snapshot.

## 4D — React UI

`TagEditor.tsx` supplies create/rename/delete forms and global loading/status/error feedback. `AssetMetadata.tsx` supplies an `aria-pressed` favorite button and a collapsible, labeled checkbox fieldset for tag assignment. The library has favorite and all-selected-tag filters with a clear-filters button. Search, filter changes and tag deletion reset pagination appropriately; the existing shrinking-page recovery also handles favorite/tag removals from filtered results.

Controls use native buttons, inputs, details/summary, labels and fieldsets, with visible keyboard focus. Status updates use polite live regions; errors use alerts and retain inputs for retry. Invalid tag input is validated authoritatively in Rust. Metadata writes are serialized in the frontend while pending, with controls disabled to prevent competing replacements. Database errors preserve current results and offer reload/retry. A tag-list failure at startup remains visible after saved-library initialization.

Query generations discard obsolete search/filter/page results. Separate tag-load and mutation generations discard results after cleanup and prevent older tag loads from replacing newer lists. Metadata completion refreshes the current query, including after navigation, without replacing the current search/filter selection. Errors for an asset edit in a previously selected library do not overwrite the new library view. A committed write cannot be canceled by navigation; only its obsolete UI publication is suppressed.

The existing audio player/controller is unchanged. Search, filters and metadata refresh do not stop playback. Switching libraries retains the existing stop behavior. Saved Library CSS rules and their contrast regression tests are unchanged. React component review followed the `vercel:react-best-practices` skill for hooks, side effects, typing and accessibility; no framework/state-management package was introduced.

## 4E — Automated verification

Executed from `src-tauri/`:

```sh
cargo fmt --check
cargo test --offline
cargo clippy --offline --all-targets -- -D warnings
```

Executed from the repository root:

```sh
npm test
npm run build
npm run tauri -- build --debug --no-bundle -- --offline
cargo test --offline --manifest-path src-tauri/Cargo.toml --lib native_webkit_plays_pauses_and_seeks_loopback_audio -- --ignored --test-threads=1 --nocapture
```

Results:

- Rust formatting and Clippy with warnings denied passed.
- Normal Rust suite: 21 library unit tests passed; one optional desktop test ignored by default; 44 integration-binary tests passed. The integration binary recompiles private core modules and includes repeated core unit tests.
- All 35 Node frontend tests passed, including existing player and Saved Library contrast regressions.
- TypeScript/Vite production build and Tauri debug executable build with bundled frontend assets passed.
- The optional real WebKitGTK regression was run explicitly and passed: disposable WAV, MP3 and OGG loaded duration, played muted, paused, resumed and sought from development and production document origins. It also exercises loopback HTTP ranges, headers, rejected requests, and unchanged WAV bytes. It does not operate the full SoundShelf application or establish audible output.
- Tauri emitted its pre-existing macOS `.app` identifier advisory. WebKit emitted pre-existing missing subtitle/video-helper plugin warnings; audio regression checks passed. No dependency installation was performed.

Added coverage checks validation, trimming/lowercase normalization, duplicate names/IDs, case-only rename, tag CRUD, absent tags/assets, wrong library membership, favorite/unfavorite, assignment/removal, missing/inaccessible record annotations, combined filters/search and deterministic pages, literal wildcard/quote/backslash/non-ASCII tag searches, and no duplicate rows. Injected SQLite triggers prove assignment and delete rollback after an actual SQL failure. Updated restart tests use the real annotation operations and check stable IDs and metadata after reopen, rescans and disappearance/reappearance. A shared integrity snapshot checks SHA-256, directory entries, modes and nanosecond mtime before/after metadata edits and rescanning.

All existing Milestone 1–3 regression tests remain included: authorization, traversal/symlink/race containment, real unprivileged permission failures, partial/failed scans, migration rollback, IPC serialization, playback tokens/ranges/bounded streaming, and file integrity. New frontend tests cover annotation requests, favorites, combined filtering/pagination, late query/tag/mutation results, write serialization, invalid/duplicate/database errors, retry and initial tag-loading errors. These are controller/runtime-contract tests, not mounted-browser accessibility or native GUI tests.

## Security review and files

Production `secure_fs.rs`, `library.rs`, `playback.rs`, audio player/controller, and CSP are unchanged. The only `playback_protocol.rs` / `playback_tests.rs` changes add the new query fields to test fixtures. `scan.rs` initializes the new DTO fields; reconciliation still preserves existing annotations. The six new command permissions are restricted to the main window. There is no unrestricted filesystem permission, shell, remote content, account, external service, telemetry, or API key.

New files: `src/components/{TagEditor,AssetMetadata}.tsx`, six generated tag/annotation command permission TOMLs, and this document. Updated files: Rust `build.rs`, capabilities, `src/{commands,contracts,db,lib,scan,playback_protocol,playback_tests}.rs`, integration tests; frontend App TSX/CSS, library controller/tests, IPC, types, runtime contracts/tests and shared fixture; README and the plan's implementation status. Dependency manifests, lockfiles and the database migration are unchanged.

No new security issue was identified within the tested scope. Existing host-filesystem and playback capability limits still apply as documented in Milestones 1–3; this is not a general security audit.

## Manual Linux GUI acceptance

Run every step with development and the built executable, using only disposable media and separate isolated application-data directories. These steps remain **pending** for Milestone 4.

1. Generate disposable tones with `python3 scripts/create-playback-fixtures.py`; include a separate set of more than 50 candidates for pagination. Record a hash/name/mode/mtime baseline outside the audio root. Start development with an isolated `XDG_DATA_HOME` as described in Milestone 3. Confirm empty states, picker cancellation, selection, scanning, search and playback.
2. Create tags such as `Rain`, `雨`, `100%`, `under_score`, a quote/backslash name, and a 64-character name. Test blank/whitespace-only input, internal control characters, 65 characters, and a duplicate differing only by case. Verify clear errors and recovery. Rename a tag, including a case-only change; its assignments must stay attached. Delete an assigned tag and check that it disappears from every asset.
3. Use favorite/unfavorite and assign/remove multiple tags, including while audio is playing. The player must continue. Search by tag, filename and path, including literal wildcards. Combine favorite-only and two selected tags: both tags must be present. Browse pages, remove a favorite/tag from the last matching page, clear filters and confirm deterministic counts and usable pagination.
4. Close and restart with the same test data directory. Confirm tags/favorites and filtered results survive. Saved roots must be inactive. Search/filter and metadata editing must work while Play and scans require native picker reselection. Reselect and rescan; annotations and IDs must persist.
5. Using separate disposable copies, externally remove one file and make another unreadable. Rescan and confirm retained annotations on missing/inaccessible or conservatively preserved historical rows. Restore and rescan. Never interpret a partial/failed scan as deletion. Repeat the malicious symlink/root replacement playback checks in Milestone 3.
6. Change search, tags, pages and libraries rapidly while results/writes load. Only the latest query may display; writes must not apply to another asset/library. Check loading, no matches, empty tags, invalid input and recovery. Test database-error recovery only with an isolated disposable index (for example an injected test-only SQLite trigger); never damage a real index.
7. Use Tab/Shift+Tab, Enter/Space and native range keys for directory selection, Saved Library, search, filters, tag create/rename/delete, disclosure/assignment controls, favorites, pagination and playback. Check focus visibility, names/pressed/checked states, loading/status/error announcements with a screen reader, narrow windows and long/non-ASCII text. Recheck Saved Library text contrast in light/dark native themes.
8. Build with the command above, stop Vite, launch `src-tauri/target/debug/soundshelf` with a different isolated `XDG_DATA_HOME`, and repeat the complete select → scan → search → play → annotate → restart → reauthorize flow. Check production CSP/IPC permissions and audible WAV/MP3/OGG playback at low volume. A successful executable build does not establish this acceptance result.
9. On fresh fixtures without deliberate external mutations, compare hashes, name set, modes and nanosecond mtime with the baseline. No sidecars may appear. Atime is excluded. Record distribution/WebKit/GStreamer versions and dev/bundled results before declaring full GUI acceptance.

## Limits and next steps

Tags are global and deletion removes associations across all libraries. No undo, tag merging, full Unicode normalization/case-folded search, filesystem watcher, content identity, move matching, or waveform/analysis feature was added. The index assumes one owning application instance. Queries use ordinary SQLite LIKE and bounded per-page annotation reads; large-library performance has not been benchmarked. Metadata follows paths and filters/search are session UI state rather than persisted preferences.

Prioritize the manual acceptance checklist, record any runtime/accessibility failures, and verify database backup/recovery procedures before expanding scope. Measure real library sizes before considering search/index optimizations. Post-MVP features remain deferred.
