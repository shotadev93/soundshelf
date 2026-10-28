# SoundShelf MVP architecture plan

This document describes the target architecture and acceptance criteria. Discovery implementation and verification status are recorded in `docs/MILESTONE_1.md`; persistent indexing and search are recorded in `docs/MILESTONE_2.md`. Native GUI acceptance checks remain outstanding; milestones 3–4 remain planned. Follow the four milestones in order, and do not assume a proposed module exists until its implementation is verified.

## Architecture overview

SoundShelf is a single-user, offline Linux desktop application. React presents a searchable audio library and an HTML audio player. Rust owns directory authorization, read-only scanning, playback access validation, and a local SQLite database. Tauri connects the two through a small command interface.

Flow: native directory selection → Rust session authorization → background scan → SQLite index → typed library query → React results → authorized local playback.

The MVP supports recursive discovery of WAV, MP3, and OGG files, filename/path/tag search, favorites, and one active audio player. Extension matching is case-insensitive. An extension identifies a candidate, not proof that it is valid or playable. Corrupt files and unsupported codecs produce a visible playback error. Waveforms, editing, embedded-tag extraction, automatic move detection, filesystem watching, and cloud synchronization are outside the MVP.

Keep the implementation direct: one application state, small Rust modules, explicit commands, and a single SQLite database. Choose a maintained SQLite binding and a native directory-picker integration during implementation after checking Linux/Tauri compatibility. Prefer the platform audio element before adding an audio engine. No ORM, search service, state-management framework, or type generator is initially required.

## Suggested directory structure

```text
src/
  main.tsx
  App.tsx
  App.css
  components/
    LibraryToolbar.tsx
    LibraryList.tsx
    AudioPlayer.tsx
    AssetMetadata.tsx
  lib/
    ipc.ts                 # Typed command wrappers and error mapping
    types.ts               # DTOs matching Rust serialization
src-tauri/
  src/
    main.rs                # Existing native entry point
    lib.rs                 # State, command and protocol registration
    commands.rs            # Thin IPC handlers
    contracts.rs           # Requests, responses and stable error codes
    library.rs             # Directory authorization and file validation
    scan.rs                # Recursive read-only discovery and reconciliation
    playback.rs            # Authorized local streaming and byte ranges
    db.rs                  # Connection ownership, queries and migrations
  migrations/
    001_initial.sql
  tests/
    library_integration.rs
    contract_serialization.rs
  capabilities/default.json
  tauri.conf.json
docs/
  PLAN.md
AGENTS.md
```

Create modules only when a milestone needs them. Keep small helpers beside their callers. Place frontend tests next to the corresponding component or IPC module when a test runner is introduced.

## Rust and TypeScript responsibility boundaries

| Rust | React / TypeScript |
| --- | --- |
| Open the native directory picker and register selected roots | Initiate selection and show the selected directory |
| Validate roots, containment, file types, and read access | Render library, progress, warnings, and retry controls |
| Walk directories on a worker and reconcile scan results | Submit search text and display paginated results |
| Own migrations, SQL queries, tags, and favorites | Edit metadata through typed commands |
| Open authorized files and serve bounded audio responses | Control one HTML audio element and display playback errors |
| Return structured failures and availability states | Present actionable messages without losing current state |

Proposed commands: `select_directory`, `scan_library`, `get_scan_status`, `list_assets`, `set_asset_tags`, `set_asset_favorite`, and `prepare_playback`. Selection runs the trusted picker in the native layer; it does not accept a path from JavaScript. Commands use opaque library/asset IDs. Serialize scans per library; poll scan status for the MVP instead of adding an event framework. Reject duplicate starts with a stable busy error.

Define Rust DTOs with explicit serde naming and matching TypeScript types in `src/lib/types.ts`. Use camelCase JSON fields, string IDs, UTC timestamp strings, and explicit nullable fields. Avoid transmitting Rust integers that may exceed JavaScript's safe integer range; transmit large byte sizes as decimal strings. Return errors shaped as `{ code, message }`, with codes such as `notAuthorized`, `notFound`, `unavailable`, `invalidInput`, `busy`, and `databaseError`. Do not expose raw database errors to the UI.

Contract fixtures must cover representative requests, responses, enums, null values, and errors. Rust verifies actual serialized output; TypeScript checks fixture shapes against its declarations. A boundary change updates both sides and fixtures in one change. Introduce generated bindings only if maintaining these contracts becomes a demonstrated problem.

## Data model

Store `soundshelf.sqlite3` in Tauri's resolved application-data directory, separate from all audio roots. Use schema-versioned SQL migrations, foreign keys, parameterized statements, and transactions. A single owned database connection behind a simple serialized access boundary is sufficient initially; do not hold its lock during filesystem traversal or audio streaming.

| Table | Main fields and constraints |
| --- | --- |
| `libraries` | `id` primary key, canonical root path stored losslessly, display path, created timestamp, last successful scan timestamp |
| `assets` | `id` primary key, `library_id` foreign key, lossless relative path, display name/path, extension, size, modification time, availability, last-seen scan ID, favorite flag; unique `(library_id, relative_path)` |
| `tags` | `id` primary key, unique normalized name, display name |
| `asset_tags` | `asset_id` and `tag_id` foreign keys, composite primary key |

Linux paths may contain non-UTF-8 bytes. Store canonical root and relative paths as BLOBs using native path bytes; expose a lossy display string separately. Never reconstruct an OS path from display text. IDs remain stable across rescans at the same relative path. Preserve tags and favorites when an asset is unavailable. A file replaced at the same path retains the path's metadata in this MVP; content-based identity is deferred and this limitation should be documented in the UI help.

Session authorization is held in memory, not inferred from `libraries`. Persisted directories appear inactive after restart until reselected through the picker. Reselecting the same canonical root reconnects its existing library metadata. Reject overlapping active roots for the MVP to avoid duplicate indexing and ambiguous authorization.

Availability values are `available`, `missing`, `inaccessible`, and `unknown`. Mark an asset missing only when its absence is established by a complete traversal of its containing scope. Permission errors, interrupted scans, and unavailable roots must not mark unseen assets missing. Record scan warnings and keep previous metadata. Recheck availability at playback time regardless of the last scan result.

A moved file appears as an unavailable old entry and a new entry after rescan. Moving an entire root requires selecting it as a new root; automatic metadata reassociation is deferred. Do not silently transfer favorites or tags based on names or file size. There is no automatic pruning of unavailable records.

Start search with escaped, parameterized SQLite `LIKE` over display name, relative display path, and tags; treat `%` and `_` as literal input unless a future UI explicitly offers wildcard search. Use bounded pagination and a deterministic secondary sort by ID. Define tag trimming, case normalization, length limits, and maximum tags per asset in Rust. Keep matching semantics simple and documented; SQLite's default case folding is not full Unicode search. Add FTS only if measured library sizes justify it.

## Tauri security considerations

- Audit the starter's `core:default` and `opener:default` permissions during implementation; retain only what the UI needs. The MVP does not require arbitrary URL opening, shell execution, or general frontend filesystem APIs.
- Bind capabilities to the main application window. Treat every command argument as untrusted even when the caller is that window. Configure explicit production CSP directives for bundled scripts, styles, and the local audio route; prohibit remote content. Scope development allowances to Vite development only.
- Only a trusted native picker may create session root authorization. Canonicalize and verify selected directories. Reject traversal, absolute relative paths, and symlinks during scans and playback. Never authorize with a textual path-prefix comparison.
- Address time-of-check/time-of-use races: on Linux, traverse and open relative to an authorized directory handle using beneath-root/no-symlink resolution. Select an appropriate supported OS primitive or small focused library during implementation. Canonicalization followed by an unrestricted path open is insufficient. If this cannot be enforced on a target runtime, fail closed rather than weakening containment.
- Open audio read-only. Store annotations only in SQLite. Do not write sidecars, change embedded tags, or clean up user media. Recursion must have bounded resource usage and skip symlinks. Root disappearance or replacement invalidates authorization and requires reselection.
- Prefer a dedicated Tauri local protocol route for playback over exposing filesystem paths. `prepare_playback` returns a session-scoped opaque URL/token for an authorized asset. The route resolves the token to its library and asset, checks authorization again, securely opens the file, and streams from that handle. Revoke tokens on root deactivation and app restart. Never embed a raw path in the URL or enable a global asset scope.
- Verify the chosen protocol supports the Linux WebView's media loading and HTTP-style byte ranges. Implement bounded reads, correct media types, range validation, partial-content headers, and invalid-range responses. Do not load whole audio files into IPC or memory. Never fall back to unrestricted `file://` URLs when playback fails.
- No cloud services, accounts, telemetry, or application network traffic. Keep user-facing diagnostics useful without logging full paths or media contents by default.

## Implementation milestones and acceptance criteria

### 1. Directory selection and safe discovery

Implement the following five steps in order. Keep discovery results and root authorization in memory until milestone 2; do not add SQLite in this milestone. Add focused checks with each step, then expand security and integration coverage in 1E. Intermediate steps are not a security-complete scanner. Milestone 1 is complete only after containment and symlink-race protections are implemented and tested, including the 1E acceptance criteria. If secure traversal is unavailable, fail closed; do not temporarily substitute canonicalization followed by unrestricted path access.

#### 1A. Return typed sample asset data from Rust

Define the initial asset DTO, structured error contract, and matching TypeScript types. Add a small Rust command returning deterministic synthetic assets, and a typed frontend wrapper. This step performs no filesystem access and establishes the IPC contract before native integration.

Acceptance criteria:
- The frontend can invoke the command and receive typed sample assets with string IDs, display fields, and explicit availability values.
- Rust serialization fixtures match the TypeScript declarations, including camelCase fields, nullable values, and structured errors. No `any` or unchecked type assertion substitutes for the contract check.
- Sample data uses no real user paths and grants no filesystem authorization. Keep the sample command isolated for removal when discovered assets replace it in 1D.

#### 1B. Implement a native directory picker

Add the trusted native picker and in-memory authorization for a selected root. Return an opaque library ID and display information, not a path that JavaScript can use to grant access. Establish the authorized directory handle and root identity needed for secure discovery; this step does not scan.

Acceptance criteria:
- Selection grants session access only to a verified directory selected through the native picker. Frontend-supplied paths cannot authorize roots.
- Canceling selection preserves the current selection; invalid, inaccessible, and overlapping active roots return structured failures without crashing.
- Root disappearance or replacement invalidates authorization and requires reselection. Restart clears session authorization.
- Capabilities are limited to the required window and picker behavior; no broad frontend filesystem permission is introduced.

#### 1C. Implement read-only audio file discovery

Implement recursive discovery behind the authorized library ID on a worker, with structured status and warnings. Enforce containment while traversing and opening resources relative to the authorized directory handle, using beneath-root/no-symlink resolution. Canonicalization alone is insufficient. Retain lossless native paths internally and expose display strings separately.

Acceptance criteria:
- Discovery finds nested WAV, MP3, and OGG candidates with case-insensitive extensions, skips unrelated files and all symlinks, and uses bounded traversal resources.
- Reject unauthorized IDs, traversal, absolute relative paths, and attempts to escape the root. Do not reopen validated resources through unrestricted paths or follow replaced symlinks.
- Missing or unreadable entries and unavailable roots produce structured warnings or failures without crashing. Incomplete results are explicitly identified as partial.
- Scanning is read-only, does not block the UI thread, and prevents overlapping scans for one library. Discovery results remain in memory.
- Focused discovery and containment tests pass; race-resistance validation remains an explicit completion gate in 1E. Do not describe the scanner as security-complete at this step.

#### 1D. Display discovered assets in React

Connect directory selection, scan initiation, status polling, and discovered results through typed IPC wrappers. Replace the sample-data path from 1A and remove its temporary command. Display a minimal accessible library list without adding persistence, search, playback, or annotation features from later milestones.

Acceptance criteria:
- Selecting a directory and starting a scan displays real discovered assets with useful display names and availability information.
- Loading, empty, partial-result, and error states are visible. Canceling selection preserves the view, and stale responses cannot overwrite results for a newly selected library.
- The UI remains responsive during scanning; keyboard users can select a directory, start discovery, and browse results.
- React uses opaque IDs for discovery requests and never performs filesystem operations or treats display paths as authorization. Rust DTOs, TypeScript types, and fixtures remain consistent.

#### 1E. Add security and integration tests

Complete the security gate using disposable directory trees and native integration checks. Fix any gaps found before declaring Milestone 1 complete; ordinary successful scans are not evidence of containment or race resistance.

Acceptance criteria:
- Tests reject unauthorized access, traversal, prefix-collision paths, static file/directory symlinks, and replacements of files or directories with symlinks between validation and access. Use controlled synchronization to exercise race windows rather than relying only on timing loops.
- Tests cover selected-root removal/replacement and authorization invalidation, non-UTF-8 names, nested mixed-case audio extensions, missing entries, repeat scans, and partial failures. Permission-denied cases run as an unprivileged user.
- Hash comparisons and directory snapshots show that scanning changes no original file bytes and creates no sidecars or other files in selected roots.
- IPC serialization checks and end-to-end selection → authorization → scan → React display checks pass. Verify the native picker and responsive UI on the supported Linux Tauri runtime, including cancellation and error recovery.
- Relevant Rust tests, formatting, Clippy, and the TypeScript/Vite build pass. Record tested security guarantees and any unsupported runtime behavior; unsupported secure traversal fails closed. Describe discovery as security-complete only within this tested scope after all required protections pass.

### 2. Persistent, searchable library

Add the SQLite schema, migrations, scan reconciliation, paginated queries, filename/path search, and availability indicators.

Acceptance criteria:
- Repeated scans do not create duplicate entries; IDs remain stable at the same relative path.
- Search and pagination return deterministic results and handle quotes, wildcard characters, and non-ASCII display names safely.
- Complete scans detect absent files; partial scans and inaccessible roots preserve existing records and report uncertainty.
- The index persists after restart, while filesystem access stays inactive until the root is reselected.
- Fresh database creation, reopening, foreign keys, and migration rollback on failure are covered by temporary-database tests.

### 3. Authorized local playback

Add a single audio player, opaque playback URLs, validated streaming, play/pause, seeking, and error states.

Acceptance criteria:
- Known supported WAV, MP3, and OGG fixtures play and seek on the supported Linux WebView runtime. Record the tested distribution/runtime and codec limitations.
- Selecting another asset stops the previous one; pause and seek behave correctly.
- Removed, unreadable, corrupt, and unsupported audio produces a recoverable error without losing library metadata.
- Requests without valid session authorization, expired tokens, traversal inputs, and symlink replacements cannot read audio or arbitrary files.
- Streaming tests cover valid, open-ended, suffix, and invalid ranges, bounded memory use, and correct response headers. Playback never changes original bytes.

### 4. Tags, favorites, and MVP completion

Add tag editing, favorites, tag search, a favorite filter, and accessible loading/empty/error states.

Acceptance criteria:
- Tags and favorites persist after restart and remain attached to unavailable assets.
- Tag validation and normalization prevent duplicate associations; updates are transactional and SQL input is parameterized.
- Search finds tags; the favorite filter combines correctly with search and pagination.
- Hash comparisons confirm metadata edits leave original audio bytes unchanged and create no sidecars in audio directories.
- Keyboard-only directory selection, browsing, metadata editing, and playback controls work. A clean installation completes selection → scan → search → play → annotate → restart → reauthorize with persisted metadata.
- Final capability/CSP review confirms no broad filesystem access, remote content, telemetry, or account dependency.

## Testing strategy

Use Rust unit tests for extension filtering, path handling, tag normalization, search escaping, and DTO serialization. Use integration tests with disposable directory trees and SQLite databases for authorization, containment, races, scan reconciliation, transactions, migrations, and metadata retention. Include non-UTF-8 paths and moved/deleted files. Run permission-denied tests as an unprivileged user so they exercise real failures.

Test the playback handler separately from the UI with temporary media fixtures and range requests. Compare hashes before and after scans, playback, and annotations; also assert that no files were created in the selected media directory. Use synthetic or redistributable audio fixtures only.

Introduce a small frontend test runner when interactive behavior warrants it. Test search state, stale request handling, metadata failures, filters, player selection, and visible unavailable-file errors. Use real serialized fixtures for IPC contract checks, not only mocked return values.

For each code milestone run the relevant tests, Rust formatting and Clippy, and the TypeScript/Vite production build. Manually verify native picker, Linux codec support, seeking, keyboard accessibility, and production CSP in a packaged Tauri build: browser-only tests cannot establish native integration or codec support. Do not claim performance for an unmeasured library size; measure scanning and search against a documented representative fixture set before optimizing.
