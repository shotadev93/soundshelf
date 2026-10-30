mod commands;
mod contracts;
mod db;
mod library;
mod playback;
mod playback_protocol;
mod scan;
mod secure_fs;
#[cfg(test)]
#[path = "../tests/support/mod.rs"]
mod support;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let result = tauri::Builder::default()
        .setup(|app| {
            use tauri::Manager;
            let directory = app.path().app_data_dir()?;
            std::fs::create_dir_all(&directory)?;
            let database = db::Database::open(&directory.join("soundshelf.sqlite3"))
                .map_err(|error| std::io::Error::other(error.message))?;
            let state = std::sync::Arc::new(library::LibraryState::new(database));
            let window = app
                .get_webview_window("main")
                .ok_or_else(|| std::io::Error::other("Main window unavailable."))?;
            let endpoint = playback_protocol::install(&window, std::sync::Arc::clone(&state))?;
            app.manage(endpoint);
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::select_directory,
            commands::scan_library,
            commands::get_scan_status,
            commands::list_assets,
            commands::list_libraries,
            commands::prepare_playback
        ])
        .run(tauri::generate_context!());
    if result.is_err() {
        eprintln!(
            "SoundShelf could not start. Check application-data access and database compatibility."
        );
        std::process::exit(1);
    }
}
