# Tags, favorites, and MVP implementation

Milestones 1–4 are implemented, completing the planned MVP feature scope. Milestone 4 adds persistent tag management, asset annotations, combined search/filtering, and accessible editing controls. The recorded automated verification and real WebKitGTK playback regression pass. The user reports that all nine items in their Milestone 4 Linux GUI checklist passed (9/9), as recorded below. These are user-reported results, separate from the automated verification; the GUI test environment and separate development/bundled run results have not been supplied. The acceptance evidence and remaining recording gaps are summarized below.

The documentation review baseline is checkpoint `3ac9faa` (`feat: implement SoundShelf milestone 4 features`). Earlier milestone documents and the architecture plan retain implementation-time pending/planned wording; this record supplies the current overall MVP status. No application code, dependencies or tests were changed or rerun for this documentation update.

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

## MVP acceptance evidence

The Milestone 4 acceptance criteria in `docs/PLAN.md` are reviewed below. Existing automated results remain implementation-time evidence; the user's latest report adds nine itemized Linux GUI checks, all reported passed. An absent record is not a failed test.

| Acceptance criterion | Recorded evidence | Evidence still not individually recorded |
| --- | --- | --- |
| Tags and favorites persist after restart and remain attached to unavailable assets | Rust restart/rescan and missing/inaccessible-record tests described above; user reports persistence after rescan/restart and normal operation with deleted audio | GUI confirmation of annotations on unavailable records and picker reauthorization, tied to a specific build/environment |
| Validation/normalization prevent duplicate associations; updates are transactional and parameterized | Rust validation, duplicate, SQL-input and trigger-induced rollback tests; code inspection | Individual GUI boundary-input and database-error recovery results |
| Tag search and favorite filtering combine with search and pagination | Rust query tests and frontend controller tests; user reports tag search, favorite filtering and combined search/filter/page navigation passed | Rapid-change races and last-page recovery in the native GUI |
| Metadata edits preserve original bytes and create no sidecars | Automated SHA-256, directory-entry, mode and nanosecond-mtime snapshots using disposable fixtures | A manual before/after integrity comparison and its retained output |
| Keyboard-only selection, browsing, metadata editing and playback; clean-install flow through restart and reauthorization | Accessible controls and controller tests; muted native WebKit playback regression; user reports WAV/MP3/OGG playback, keyboard operation and light/dark themes passed | Separate development/bundled and clean-install results, the complete keyboard-only flow, screen-reader observations, explicit audible-output confirmation and picker reauthorization |
| Final capability/CSP review excludes broad filesystem access, remote content, telemetry and accounts | Recorded configuration/source review and retained authorization/containment regressions | Itemized production GUI IPC/CSP observations; no independent security audit is recorded or implied |

**Current status:** the MVP implementation is complete, with all nine user-supplied Milestone 4 GUI checks reported passed. Automated evidence is recorded above. User-supplied screenshots of the player and favorites/tag view are available in the [README](../README.md#screenshots). Remaining recording gaps are runtime versions, tested build modes, fixtures, test logs, and the specific acceptance cases listed in the table. The nine reported checks do not imply that every subcase in the more detailed reference procedure below was exercised. Historical Milestone 1–3 acceptance items are covered only to the extent explicitly included in the report.

## Manual Linux GUI acceptance

### User-reported results — 9/9 passed

The user supplied the following completed checklist. Each checked item records a user-reported pass; these GUI checks were not independently performed by the coding agent. The report does not specify the test date, runtime versions, exact tested build or whether development and bundled modes were both exercised.

- [x] Tags can be created, renamed and deleted.
- [x] Tags can be assigned to and removed from audio assets.
- [x] Assets can be added to and removed from favorites.
- [x] Tag search and favorites-only filtering work.
- [x] Search, filters and page navigation work together.
- [x] Tags and favorites persist after rescanning and restarting the application.
- [x] The application continues to operate normally with corrupt files or deleted audio files.
- [x] WAV, MP3 and OGG playback continues to work normally.
- [x] Keyboard operation and light/dark themes work normally.

The report confirms these functional outcomes. It does not add manual file-integrity snapshots, database-error injection results, screen-reader results or an independent security audit. Existing automated results and known limitations remain unchanged.

### Detailed reference procedure

The retained checklist below is a reproducible procedure and a template for recording additional acceptance detail; its numbered steps are distinct from the nine reported results above and are not individually marked passed. It does not require repeating the completed checks. Record which steps were exercised, the checkpoint, Linux/WebKit/GStreamer versions, development versus bundled mode, and any exceptions when that information becomes available. Use disposable media and isolated application-data directories for any follow-up checks.

1. Generate disposable tones with `python3 scripts/create-playback-fixtures.py`; include a separate set of more than 50 candidates for pagination. Record a hash/name/mode/mtime baseline outside the audio root. Start development with an isolated `XDG_DATA_HOME` as described in Milestone 3. Confirm empty states, picker cancellation, selection, scanning, search and playback.
2. Create tags such as `Rain`, `雨`, `100%`, `under_score`, a quote/backslash name, and a 64-character name. Test blank/whitespace-only input, internal control characters, 65 characters, and a duplicate differing only by case. Verify clear errors and recovery. Rename a tag, including a case-only change; its assignments must stay attached. Delete an assigned tag and check that it disappears from every asset.
3. Use favorite/unfavorite and assign/remove multiple tags, including while audio is playing. The player must continue. Search by tag, filename and path, including literal wildcards. Combine favorite-only and two selected tags: both tags must be present. Browse pages, remove a favorite/tag from the last matching page, clear filters and confirm deterministic counts and usable pagination.
4. Close and restart with the same test data directory. Confirm tags/favorites persist and can be searched and filtered again; search/filter selections themselves are not persisted. Saved roots must be inactive. Search/filter and metadata editing must work while Play and scans require native picker reselection. Reselect and rescan; annotations and IDs must persist.
5. Using separate disposable copies, externally remove one file and make another unreadable. Rescan and confirm retained annotations on missing/inaccessible or conservatively preserved historical rows. Restore and rescan. Never interpret a partial/failed scan as deletion. Repeat the malicious symlink/root replacement playback checks in Milestone 3.
6. Change search, tags, pages and libraries rapidly while results/writes load. Only the latest query may display; writes must not apply to another asset/library. Check loading, no matches, empty tags, invalid input and recovery. Test database-error recovery only with an isolated disposable index (for example an injected test-only SQLite trigger); never damage a real index.
7. Use Tab/Shift+Tab, Enter/Space and native range keys for directory selection, Saved Library, search, filters, tag create/rename/delete, disclosure/assignment controls, favorites, pagination and playback. Check focus visibility, names/pressed/checked states, loading/status/error announcements with a screen reader, narrow windows and long/non-ASCII text. Recheck Saved Library text contrast in light/dark native themes.
8. Build with the command above, stop Vite, launch `src-tauri/target/debug/soundshelf` with a different isolated `XDG_DATA_HOME`, and repeat the complete select → scan → search → play → annotate → restart → reauthorize flow. Check production CSP/IPC permissions and audible WAV/MP3/OGG playback at low volume. A successful executable build does not establish this acceptance result.
9. On fresh fixtures without deliberate external mutations, compare hashes, name set, modes and nanosecond mtime with the baseline. No sidecars may appear. Atime is excluded. Record distribution/WebKit/GStreamer versions, dev/bundled results and integrity output to substantiate item-level GUI acceptance.

## Limits and next steps

Tags are global and deletion removes associations across all libraries. No undo, tag merging, full Unicode normalization/case-folded search, filesystem watcher, content identity, move matching, or waveform/analysis feature was added. The index assumes one owning application instance. Queries use ordinary SQLite LIKE and bounded per-page annotation reads; large-library performance has not been benchmarked. Metadata follows paths and filters/search are session UI state rather than persisted preferences.

Prioritize recording the environment and any remaining acceptance details for the completed user-reported GUI testing, following up only where evidence is missing or a problem is reported. Record any runtime/accessibility failures and verify database backup/recovery procedures before expanding scope. Measure real library sizes before considering search/index optimizations. Post-MVP features remain deferred.
