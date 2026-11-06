# SoundShelf development instructions

## Scope and stack

SoundShelf is a local Linux desktop audio asset manager built with Tauri 2, Rust, React, TypeScript, and Vite. Read `docs/PLAN.md` before implementing features. SoundShelf has completed its initial MVP implementation
(Milestones 1–4). Consult the milestone reports for
implemented functionality, verification results,
and known limitations.

Keep documentation in English. Explain unfamiliar Rust concepts in Japanese when discussing implementation with the user, including a short practical example when useful (for example, ownership, borrowing, `Result`, lifetimes, or `Send`).

## Coding conventions

- Make small, focused changes. Avoid speculative abstractions and unrelated refactors.
- Use Rust formatting and idiomatic snake_case; use PascalCase for types. Use camelCase in TypeScript and serialized IPC fields, and PascalCase for React components and interfaces.
- Keep filesystem access, path validation, scanning, and SQLite operations in Rust. React manages presentation, interaction, and an authorized local audio player.
- Use explicit request and response DTOs at the command boundary. Map Rust names with serde as needed. Update the Rust DTO, TypeScript definition, and contract test together whenever IPC changes.
- Represent expected failures with typed results and stable error codes. Do not panic or use `unwrap()` for user input, filesystem operations, database operations, or decoding failures.
- Use strict TypeScript types; avoid `any`. Keep components accessible and side effects outside rendering.
- Prefer existing dependencies and standard library facilities. Add a dependency only for a concrete need and explain its purpose. Do not introduce generic repository, event-bus, or plugin layers for the MVP.
- Keep long scans and database work off the UI thread. Keep modules organized by their actual responsibilities.

## Security and data integrity

- Never write, rename, delete, or alter original audio files, including embedded tags and timestamps deliberately set by the application. Store all SoundShelf metadata in its own SQLite database in the OS application-data directory. Normal OS read access may update access times.
- Grant access only through a trusted native directory picker. A frontend-supplied path or stored database path is not authorization. Restore saved roots only as inactive entries until the user selects them again after restart.
- Validate containment in Rust for every scan and playback request. Reject traversal and symlinks; do not follow directory symlinks. Validate actual opened resources against authorized roots and account for path replacement races. Do not rely on string-prefix checks or canonicalization alone.
- Use library IDs and asset IDs in ordinary commands instead of accepting arbitrary paths. Treat names, tags, paths, and IPC arguments as untrusted input.
- Use a narrowly scoped playback route that revalidates authorization when opening audio. Never enable unrestricted filesystem access or a broad asset-protocol scope.
- Use parameterized SQL, transactions, and versioned migrations. Preserve metadata when files become unavailable; never interpret a failed or incomplete scan as deletion.
- Handle missing, moved, unreadable, corrupt, and unsupported files without crashing or losing user metadata. Never automatically match moved files solely by filename, size, or timestamp.
- Keep Tauri capabilities and content security policy minimal. No shell access, remote content, cloud services, accounts, telemetry, or network requests are required. Do not log audio contents or full library paths by default.
- Do not commit databases, scanned media, secrets, or user-specific paths.

## Verification requirements

- For implementation changes, run relevant Rust tests, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `npm run build`. Run frontend tests when configured and affected. Report checks that could not run and their reasons.
- Test behavior and security boundaries rather than mirroring implementation. Use disposable fixtures and temporary databases; never scan or modify a developer's real library in tests.
- Cover extension filtering, nested directories, traversal, symlinks, authorization, missing and unreadable files, partial scans, repeat scans, SQL input, metadata persistence, and IPC serialization as those features are introduced.
- Add tests demonstrating that tags and favorites do not alter original file bytes. Test playback authorization and range responses, plus playback manually on the supported Linux runtime.
- Check keyboard navigation and loading, empty, error, and unavailable-file states in the UI.
- Documentation-only work does not require application builds or dependency installation. Do not claim checks passed unless actually run.

## Standard development workflow

For every implementation task:

1. Read this file and relevant project documentation.
2. Inspect Git status and the existing implementation.
3. Identify scope, dependencies, and regression risks.
4. Implement in small, reviewable steps.
5. Make straightforward decisions independently.
6. Add or update tests.
7. Run relevant verification commands.
8. Review the final diff.

Do not ask for confirmation about ordinary implementation details.
Request approval for destructive, security-sensitive, or externally visible actions.

## Git and external actions

- Do not commit, push, tag, force-push, or rewrite Git history without explicit authorization.
- Do not publish releases or change GitHub permissions without approval.
- Do not run sudo or install system packages automatically.
- Do not upload user data or source code to external services without permission.
- Preserve uncommitted changes.
- Do not add cloud services, authentication, or telemetry unless requested.

## Verification protocol

For implementation changes, run applicable checks:

- cargo fmt --check
- cargo test --offline
- cargo clippy --offline --all-targets -- -D warnings
- npm test
- npm run build

For release tasks, also build relevant Tauri distribution artifacts.

Report every failed or skipped check.
Do not equate compilation with native GUI or audio playback verification.

For documentation-only changes, review the diff and run git diff --check.

## Documentation and reporting

- Keep code, UI text, and documentation in English.
- Write final reports in Japanese.
- Explain important new Rust concepts in Japanese.
- Update documentation when behavior or architecture changes.
- Never invent verification results or benchmarks.

Final report structure:

1. Summary of changes
2. Files created or modified
3. Tests and build results
4. Dependencies or configuration changes
5. Known limitations
6. Manual verification required
7. Recommended next steps

Keep reports concise for small tasks and detailed for milestones.

## Release preparation rules

- Verify version consistency across manifests.
- Inspect production configuration and dependencies.
- Build requested distribution artifacts when feasible.
- Generate SHA-256 checksums for actual artifacts.
- Prepare release notes and acceptance checks.
- Do not claim builds prove successful installation.
- Stop before committing, tagging, uploading, or publishing without authorization.
- Never delete existing application data during verification.

## Completion criteria

A task is complete when requested deliverables exist,
relevant checks have passed or failures are reported,
and the final changes have been reviewed.

When native GUI acceptance is pending, report that separately.
