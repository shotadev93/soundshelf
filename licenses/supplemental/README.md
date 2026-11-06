# Supplemental upstream notices

These are upstream texts, not SoundShelf-authored MIT material.

- `dlopen2-LICENSE`: exact upstream repository license for both dlopen2 0.8.2 and dlopen2_derive 0.4.3, missing from their cached crate archives. The archive `.cargo_vcs_info.json` files identify commit `cc80e4a0a90d499b677fdf7743699b4b3a43a989`. Source: https://raw.githubusercontent.com/OpenByteDev/dlopen2/cc80e4a0a90d499b677fdf7743699b4b3a43a989/LICENSE
- alloc-stdlib 0.3.0 omits its repository-root BSD notice. The notice in the companion alloc-no-stdlib 3.0.0 archive was compared with https://raw.githubusercontent.com/dropbox/rust-alloc-no-stdlib/0a81fd6928ea3b33c8cd484aa4575d50ffb98012/LICENSE (alloc-stdlib's recorded commit), and is reproduced in the generated third-party notices.
- selectors 0.38.0 omits a standalone LICENSE file but its source headers explicitly name MPL 2.0. The identical standard MPL text shipped by cssparser 0.37.0 is supplied with selectors' source header and the exact selectors source-archive URL in the generated notices.

Optional/non-built libappindicator-sys inherits repository-root notices from the companion libappindicator crate at the same recorded commit; native npm Tauri CLI tooling inherits its matching-version parent CLI notices. These build tools/optional components do not establish that they are embedded in the Debian binary.
