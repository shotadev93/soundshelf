mod commands;
mod contracts;
mod db;
mod library;
mod scan;
mod secure_fs;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let result = tauri::Builder::default()
        .setup(|app| {
            use tauri::Manager;
            let directory = app.path().app_data_dir()?;
            std::fs::create_dir_all(&directory)?;
            let database = db::Database::open(&directory.join("soundshelf.sqlite3"))
                .map_err(|error| std::io::Error::other(error.message))?;
            app.manage(std::sync::Arc::new(library::LibraryState::new(database)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::select_directory,
            commands::scan_library,
            commands::get_scan_status,
            commands::list_assets,
            commands::list_libraries
        ])
        .run(tauri::generate_context!());
    if result.is_err() {
        eprintln!(
            "SoundShelf could not start. Check application-data access and database compatibility."
        );
        std::process::exit(1);
    }
}
