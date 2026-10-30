use crate::{
    contracts::{AppError, ErrorCode, LibraryInfo},
    library::LibraryState,
};
use crate::{
    contracts::{AssetPage, AssetQuery, LibraryRequest, SavedLibrary, ScanStatus},
    scan,
};
use gtk::prelude::*;
use std::{
    cell::RefCell,
    path::PathBuf,
    sync::{Arc, atomic::Ordering},
};
use tauri::Manager;

thread_local! {
    // GTK objects stay on the main thread and alive until their response signal.
    static PICKER: RefCell<Option<gtk::FileChooserNative>> = const { RefCell::new(None) };
}

fn check_window(window: &tauri::WebviewWindow) -> Result<(), AppError> {
    if window.label() != "main" {
        return Err(AppError::new(
            ErrorCode::NotAuthorized,
            "This window is not authorized.",
        ));
    }
    Ok(())
}

fn picker_selection(
    response: gtk::ResponseType,
    filename: Option<PathBuf>,
) -> Result<Option<PathBuf>, AppError> {
    match response {
        gtk::ResponseType::Accept => filename
            .map(Some)
            .ok_or_else(|| AppError::unavailable("The selection is not a local directory.")),
        gtk::ResponseType::Cancel | gtk::ResponseType::DeleteEvent | gtk::ResponseType::Close => {
            Ok(None)
        }
        _ => Err(AppError::unavailable("Directory selection failed.")),
    }
}

struct PickerGuard(Arc<LibraryState>);
impl Drop for PickerGuard {
    fn drop(&mut self) {
        self.0.picker_open.store(false, Ordering::Release);
    }
}

#[tauri::command]
pub(crate) async fn scan_library(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, Arc<LibraryState>>,
    request: LibraryRequest,
) -> Result<ScanStatus, AppError> {
    check_window(&window)?;
    let state = Arc::clone(state.inner());
    let (library, status) = tauri::async_runtime::spawn_blocking(move || {
        let library = state.get(&request.library_id)?;
        let status = scan::begin(&library)?;
        Ok::<_, AppError>((library, status))
    })
    .await
    .map_err(|_| AppError::unavailable("Library validation failed."))??;
    tauri::async_runtime::spawn(async move {
        let worker = Arc::clone(&library);
        match tauri::async_runtime::spawn_blocking(move || scan::execute(&worker)).await {
            Ok(Ok(())) => {}
            Ok(Err(error)) => scan::fail(&library, error),
            Err(_) => scan::fail(
                &library,
                AppError::unavailable("The scan worker stopped unexpectedly."),
            ),
        }
    });
    Ok(status)
}

#[tauri::command]
pub(crate) async fn get_scan_status(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, Arc<LibraryState>>,
    request: LibraryRequest,
) -> Result<ScanStatus, AppError> {
    check_window(&window)?;
    let state = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || {
        let library = state.get(&request.library_id)?;
        scan::status(&library)
    })
    .await
    .map_err(|_| AppError::unavailable("Scan status is unavailable."))?
}

#[tauri::command]
pub(crate) async fn list_assets(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, Arc<LibraryState>>,
    request: AssetQuery,
) -> Result<AssetPage, AppError> {
    check_window(&window)?;
    let state = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || {
        crate::library::lock(&state.database)?.query(&request)
    })
    .await
    .map_err(|_| AppError::unavailable("Library results are unavailable."))?
}

#[tauri::command]
pub(crate) async fn list_libraries(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, Arc<LibraryState>>,
) -> Result<Vec<SavedLibrary>, AppError> {
    check_window(&window)?;
    let state = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || state.saved_libraries())
        .await
        .map_err(|_| AppError::unavailable("Saved libraries are unavailable."))?
}

#[tauri::command]
pub(crate) async fn select_directory(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, Arc<LibraryState>>,
) -> Result<Option<LibraryInfo>, AppError> {
    check_window(&window)?;
    let state = Arc::clone(state.inner());
    if state.picker_open.swap(true, Ordering::AcqRel) {
        return Err(AppError::new(
            ErrorCode::Busy,
            "A directory picker is already open.",
        ));
    }
    let _guard = PickerGuard(Arc::clone(&state));
    let (sender, mut receiver) = tauri::async_runtime::channel(1);
    let app = window.app_handle().clone();
    app.run_on_main_thread(move || {
        let parent = match window.gtk_window() {
            Ok(parent) => parent,
            Err(_) => {
                let _ = sender.try_send(Err(AppError::unavailable(
                    "The directory picker cannot be opened.",
                )));
                return;
            }
        };
        let dialog = gtk::FileChooserNative::new(
            Some("Select an audio directory"),
            Some(&parent),
            gtk::FileChooserAction::SelectFolder,
            Some("Select"),
            Some("Cancel"),
        );
        dialog.set_modal(true);
        dialog.set_local_only(true);
        dialog.connect_response(move |dialog, response| {
            let result = picker_selection(response, dialog.filename());
            let _ = sender.try_send(result);
            dialog.destroy();
            PICKER.with(|picker| {
                picker.borrow_mut().take();
            });
        });
        PICKER.with(|picker| {
            *picker.borrow_mut() = Some(dialog.clone());
        });
        dialog.show();
    })
    .map_err(|_| AppError::unavailable("The directory picker cannot be opened."))?;
    let selected = receiver
        .recv()
        .await
        .ok_or_else(|| AppError::unavailable("The directory picker closed unexpectedly."))??;
    tauri::async_runtime::spawn_blocking(move || state.authorize_selection(selected))
        .await
        .map_err(|_| AppError::unavailable("Directory selection failed."))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canceled_native_responses_never_authorize_a_stale_filename() {
        for response in [
            gtk::ResponseType::Cancel,
            gtk::ResponseType::DeleteEvent,
            gtk::ResponseType::Close,
        ] {
            assert_eq!(
                picker_selection(response, Some(PathBuf::from("/synthetic/audio"))).unwrap(),
                None
            );
        }
        assert!(picker_selection(gtk::ResponseType::Accept, None).is_err());
        assert!(picker_selection(gtk::ResponseType::None, None).is_err());
        assert_eq!(
            picker_selection(
                gtk::ResponseType::Accept,
                Some(PathBuf::from("/synthetic/audio"))
            )
            .unwrap(),
            Some(PathBuf::from("/synthetic/audio"))
        );
    }
}

#[tauri::command]
pub(crate) async fn prepare_playback(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, Arc<LibraryState>>,
    request: crate::contracts::PlaybackRequest,
    endpoint: tauri::State<'_, crate::playback_protocol::PlaybackEndpoint>,
) -> Result<crate::contracts::PlaybackSource, AppError> {
    check_window(&window)?;
    let state = Arc::clone(state.inner());
    let source = tauri::async_runtime::spawn_blocking(move || state.prepare_playback(request))
        .await
        .map_err(|_| AppError::unavailable("Playback preparation failed."))??;
    Ok(endpoint.source(source))
}
