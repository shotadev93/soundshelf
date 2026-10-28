//! Session authorization. Paths enter here only from the native picker, never IPC.
use crate::contracts::{AppError, ErrorCode, LibraryInfo, SavedLibrary};
use crate::secure_fs::AuthorizedRoot;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex, MutexGuard},
};

pub struct LibraryState {
    pub(crate) database: Arc<Mutex<crate::db::Database>>,
    libraries: Mutex<HashMap<String, Arc<Library>>>,
    pub(crate) picker_open: std::sync::atomic::AtomicBool,
}

pub(crate) struct Library {
    pub info: LibraryInfo,
    pub database: Arc<Mutex<crate::db::Database>>,
    pub root: AuthorizedRoot,
    pub scan: Mutex<crate::scan::ScanData>,
}

pub(crate) fn lock<T>(mutex: &Mutex<T>) -> Result<MutexGuard<'_, T>, AppError> {
    mutex.lock().map_err(|_| {
        AppError::new(
            ErrorCode::InternalError,
            "Application state is unavailable. Restart SoundShelf.",
        )
    })
}

pub(crate) fn opaque_id() -> Result<String, AppError> {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes)
        .map_err(|_| AppError::unavailable("Secure identifiers are unavailable."))?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

#[cfg(test)]
impl Default for LibraryState {
    fn default() -> Self {
        Self::new(crate::db::Database::memory())
    }
}

impl LibraryState {
    pub(crate) fn new(database: crate::db::Database) -> Self {
        Self {
            database: Arc::new(Mutex::new(database)),
            libraries: Mutex::new(HashMap::new()),
            picker_open: std::sync::atomic::AtomicBool::new(false),
        }
    }

    pub(crate) fn saved_libraries(&self) -> Result<Vec<SavedLibrary>, AppError> {
        let saved = lock(&self.database)?.libraries()?;
        let active = lock(&self.libraries)?;
        Ok(saved
            .into_iter()
            .map(|library| {
                let authorized = active
                    .get(&library.id)
                    .is_some_and(|entry| entry.root.validate().is_ok());
                SavedLibrary {
                    library,
                    active: authorized,
                }
            })
            .collect())
    }

    pub(crate) fn authorize_selection(
        &self,
        selected: Option<PathBuf>,
    ) -> Result<Option<LibraryInfo>, AppError> {
        let Some(path) = selected else {
            return Ok(None);
        };
        let root = AuthorizedRoot::open(&path)?;
        let mut libraries = lock(&self.libraries)?;
        // Discard revoked roots before checking component-aware overlap.
        libraries.retain(|_, library| library.root.validate().is_ok());
        for library in libraries.values() {
            if root.path() == library.root.path() {
                return Ok(Some(library.info.clone()));
            }
            if root.path().starts_with(library.root.path())
                || library.root.path().starts_with(root.path())
            {
                return Err(AppError::new(
                    ErrorCode::InvalidInput,
                    "This directory overlaps an active library.",
                ));
            }
        }
        if libraries.len() >= 16 {
            return Err(AppError::unavailable(
                "The session library limit was reached. Restart to select other directories.",
            ));
        }
        let info = lock(&self.database)?.register(root.path())?;
        libraries.insert(
            info.id.clone(),
            Arc::new(Library {
                info: info.clone(),
                root,
                database: Arc::clone(&self.database),
                scan: Mutex::new(crate::scan::ScanData::default()),
            }),
        );
        Ok(Some(info))
    }

    pub(crate) fn get(&self, id: &str) -> Result<Arc<Library>, AppError> {
        let library = lock(&self.libraries)?.get(id).cloned().ok_or_else(|| {
            AppError::new(
                ErrorCode::NotAuthorized,
                "Select an audio directory to authorize access.",
            )
        })?;
        library.root.validate()?;
        Ok(library)
    }
}
