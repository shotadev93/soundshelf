mod commands;
mod contracts;
mod library;
mod scan;
mod secure_fs;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(std::sync::Arc::new(library::LibraryState::default()))
        .invoke_handler(tauri::generate_handler![
            commands::select_directory,
            commands::scan_library,
            commands::get_scan_status,
            commands::list_assets
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
