# Third-party license and asset review — v0.1.0

Audit date: 2026-11-04. This is a repository/dependency/package review, not an independent legal opinion or security certification. SoundShelf's own code is MIT licensed; upstream works retain their own copyrights, licenses and trademark conditions.

## Project copyright and MIT adoption

`LICENSE` contains the complete standard MIT grant/disclaimer with `Copyright (c) 2026 shotadev93`. This public handle is established by all five repository commits through `6ef608f` and by the configured GitHub repository owner, `shotadev93/soundshelf`. A legal name, employer or different rights holder was not inferred. Cargo and npm metadata also declare MIT. This grant covers SoundShelf-authored material; it cannot relicense third-party logos, dependencies or material visible in screenshots.

## Dependency inventory and notices

`licenses/inventory.json` records exact versions, declared expressions, upstream source archives, notice hashes and lockfile hashes. `THIRD_PARTY_NOTICES.txt` reproduces the collected upstream texts. Run `python3 scripts/generate-third-party-notices.py` with the locked Cargo cache, installed npm dependencies and Rust toolchain documentation to regenerate them offline. The generator fails when an included package has no recovered notice text. Supplemental provenance is recorded in `licenses/supplemental/README.md`.

Notice hashes identify the original cached upstream files. Distributed texts preserve their terms and attribution while normalizing line endings and trailing whitespace; they are not byte-for-byte copies. No upstream program source was modified.

The inventory covers 268 Cargo packages in the Linux normal/build dependency tree and 26 installed npm packages, including build tools conservatively. It is not a linker map and does not assert that all inventoried packages are embedded. Metadata for another 149 Cargo packages outside that tree and 35 uninstalled npm platform packages is retained separately; those entries do not imply Linux binary distribution. Runtime JS in the application is React, React DOM, Scheduler and Tauri API code plus SoundShelf code. Vite, TypeScript, native CLI/Rolldown bindings and CSS tooling are build tools rather than shipped executables.

| Terms observed | Handling and findings |
| --- | --- |
| MIT / Apache-2.0 alternatives | Original texts and copyright notices retained; dual expressions remain alternatives, not a new project license. |
| Apache-2.0 only (including Tao) | Apache texts and available attribution/NOTICE/SPDX files retained. Tao's winit/Tauri attribution remains upstream-owned. |
| BSD-3-Clause, Zlib, ISC, Unlicense/CC0 options | Original notices/disclaimers retained. The alloc-stdlib crate omits its repository-root BSD text; the pinned upstream text was checked and supplied. |
| `BSD-3-Clause AND MIT` (brotli), `Apache-2.0 AND MIT` (dpi) | Both required sets of texts included; AND was not treated as OR. |
| Unicode-3.0 and unicode-ident's compound expression | Unicode notices included in addition to permissive code terms. |
| MPL-2.0: cssparser, cssparser-macros, dtoa-short, option-ext, selectors | Exact unmodified crate source archive URLs are provided in the notices. The project has not modified those sources. selectors' source header identifies MPL 2.0 even though its archive omits a standalone LICENSE. |
| MPL-2.0 npm Lightning CSS packages | Build-time tooling; upstream license retained, not reclassified as project MIT. |
| Rust standard library / compiler-builtins | Toolchain-provided `COPYRIGHT-library.html` and its license texts are included under `licenses/rust-standard-library/`. These retain their original exceptions and component terms; the conservative document includes material not necessarily linked. |
| SQLite 3.54.1 through bundled libsqlite3-sys | The wrapper is MIT; the amalgamation's copyright disclaimer/public-domain notice is reproduced separately. SQLCipher is not enabled. |

MPL permits larger works under other terms while requiring covered source availability and preservation of its terms; see the [license](https://www.mozilla.org/MPL/2.0/) and [Mozilla distribution FAQ](https://www.mozilla.org/en-US/MPL/2.0/FAQ/). The five Linux Cargo MPL source archives are also supplied unchanged under `licenses/sources/`, verified against their Cargo.lock SHA-256 values, so recipients need not rely on the upstream download remaining available. Extract a `.crate` file with `tar -xzf filename.crate` into a disposable directory to inspect the source. These archives are intentional source-compliance materials, not application runtime resources. Modified upstream code would require a new source-compliance review. No incompatible mandatory copyleft term was identified in the audited Linux Cargo/npm dependency expressions. This is not a claim that a metadata-only scan proves compliance for every possible build.

The Debian packaging configuration installs project copyright, full third-party notices, this review and the inventory/toolchain notices under `/usr/share/doc/soundshelf/`. Before this audit the user-tested package contained only the executable, desktop entry and three icons, with no distributed license notices; it must be replaced by the audited packaging candidate for publication. Source and binary release distributions must preserve these notice files.

## Native libraries and packaging tools

The Debian package uses system GTK/GDK, WebKitGTK, libsoup, GLib, Cairo, GStreamer codecs, glibc and libgcc. Its ELF dependencies were inspected; it does not copy those shared libraries or codec plugins into the Debian payload. Host/distribution components retain LGPL/GPL-with-exceptions and other terms supplied by their package maintainers, normally documented under `/usr/share/doc/<package>/copyright`. SoundShelf's MIT grant does not cover or replace them. Installing a package dependency is distinct from redistributing its binary inside SoundShelf.

No AppImage was built successfully. AppImage would copy WebKit/GTK/GStreamer and other system binaries, so this Debian review is not an AppImage license clearance. Review every bundled library/plugin, source/relinking obligations and any codec/patent considerations before publishing that format. Tauri's [AppImage guide](https://v2.tauri.app/distribute/appimage/) specifically calls out multimedia bundling and plugin licensing. The missing-patchelf failure remains unresolved; no system packages were installed.

Tauri CLI, dpkg-deb and the downloaded AppImage helper programs are build tools, not assets in the Debian release. Helper provenance from the failed AppImage attempt remains in the release checklist. The MIT project grant does not relicense those tools.

## Assets and screenshots

- The initial application icons are the Tauri default logo, not original SoundShelf artwork. The [Tauri repository](https://github.com/tauri-apps/tauri#licenses) identifies separate CC-BY-NC-ND logo terms, and the [trademark guidelines](https://v2.tauri.app/about/trademark/) instruct applications not to ship the default icon. On 2026-11-04 the user approved replacing it with the original SoundShelf folder/waveform design. `src-tauri/icons/soundshelf.svg` is the MIT-licensed vector source; all existing PNG/ICO/ICNS files were regenerated with the installed Tauri icon command. The default logo is absent from the new package. Historical packages retain it and must not be published.
- Unused `public/tauri.svg`, `public/vite.svg` and `src/assets/react.svg` starter assets were removed after confirming they have no imports. This prevents unused logos from being distributed as embedded frontend resources and avoids claiming those third-party marks as SoundShelf MIT artwork. No UI feature was removed.
- `docs/screenshots/audio-player.png` and `favorites-and-tags.png` are user-provided captures of the actual application. Inspection found a displayed song filename/relative path, not embedded audio or a full home path. On 2026-11-04 the user explicitly approved MIT licensing of the image rights they own in both captures. Project MIT does not grant rights in third-party music, filenames, marks or desktop/font designs visible in a capture. Screenshots are documentation assets, not Debian resources.
- Synthetic test tones are generated in temporary directories and are not checked into Git or packaged. No personal audio or SQLite index is part of the release payload.

## Current decisions and limits

Project MIT adoption is complete and supported by repository evidence. Dependency notices and Debian notice bundling are prepared. The user approved the original icon replacement and MIT licensing of their own screenshot image rights on 2026-11-04. These two distribution blockers are resolved. The rebuilt legal/asset package still requires a focused installation/launcher smoke check and explicit publication approval; earlier user-reported application acceptance remains valid within its recorded scope. The final artifact hash and verification status are recorded in `docs/RELEASE_CHECKLIST.md` and the v0.1.0 notes. Human review is still appropriate for third-party source/trademark terms and any future change in build target or bundled runtimes.
