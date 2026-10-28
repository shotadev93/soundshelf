# SoundShelf development instructions

## Scope and stack

SoundShelf is a local Linux desktop audio asset manager built with Tauri 2, Rust, React, TypeScript, and Vite. Read `docs/PLAN.md` before implementing features. The current application is a working starter; the plan describes future implementation, not existing functionality.

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
