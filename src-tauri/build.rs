fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "select_directory",
            "scan_library",
            "get_scan_status",
            "list_assets",
            "list_libraries",
            "prepare_playback",
            "list_tags",
            "create_tag",
            "rename_tag",
            "delete_tag",
            "set_asset_tags",
            "set_asset_favorite",
        ]),
    ))
    .expect("failed to build Tauri command permissions")
}
