//! The index grants no filesystem authority. Native paths are stored as BLOBs.
use crate::{
    contracts::{
        AppError, AssetPage, AssetQuery, AudioAsset, Availability, ErrorCode, LibraryInfo,
        ScanPhase, ScanStatus,
    },
    library::opaque_id,
    scan::StoredAsset,
};
use rusqlite::{Connection, OptionalExtension, params};
use std::{
    collections::HashMap,
    ffi::OsString,
    os::unix::ffi::{OsStrExt, OsStringExt},
    path::{Path, PathBuf},
    time::Duration,
};

pub(crate) struct Database(Connection);

fn database_error(_: rusqlite::Error) -> AppError {
    AppError::new(
        ErrorCode::DatabaseError,
        "The library database could not be accessed.",
    )
}

impl Database {
    pub(crate) fn open(path: &Path) -> Result<Self, AppError> {
        Self::initialize(Connection::open(path).map_err(database_error)?)
    }

    #[cfg(test)]
    pub(crate) fn memory() -> Self {
        Self::initialize(Connection::open_in_memory().expect("temporary database")).expect("schema")
    }

    fn initialize(mut connection: Connection) -> Result<Self, AppError> {
        connection
            .busy_timeout(Duration::from_secs(5))
            .map_err(database_error)?;
        connection
            .pragma_update(None, "foreign_keys", true)
            .map_err(database_error)?;
        migrate(
            &mut connection,
            &[include_str!("../migrations/001_initial.sql")],
        )?;
        // Interrupted workers must never be mistaken for completed scans.
        connection
            .execute(
                "UPDATE libraries SET last_scan_phase='failed' WHERE last_scan_phase='running'",
                [],
            )
            .map_err(database_error)?;
        Ok(Self(connection))
    }

    pub(crate) fn register(&mut self, root: &Path) -> Result<LibraryInfo, AppError> {
        let bytes = root.as_os_str().as_bytes();
        let existing = self
            .0
            .query_row(
                "SELECT id, display_path FROM libraries WHERE root_path=?1",
                [bytes],
                |row| {
                    Ok(LibraryInfo {
                        id: row.get(0)?,
                        display_path: row.get(1)?,
                    })
                },
            )
            .optional()
            .map_err(database_error)?;
        if let Some(info) = existing {
            return Ok(info);
        }
        let info = LibraryInfo {
            id: opaque_id()?,
            display_path: root.to_string_lossy().into_owned(),
        };
        self.0
            .execute(
                "INSERT INTO libraries(id,root_path,display_path) VALUES(?1,?2,?3)",
                params![info.id, bytes, info.display_path],
            )
            .map_err(database_error)?;
        Ok(info)
    }

    pub(crate) fn libraries(&self) -> Result<Vec<LibraryInfo>, AppError> {
        let mut statement = self
            .0
            .prepare("SELECT id,display_path FROM libraries ORDER BY display_path,id")
            .map_err(database_error)?;
        statement
            .query_map([], |row| {
                Ok(LibraryInfo {
                    id: row.get(0)?,
                    display_path: row.get(1)?,
                })
            })
            .map_err(database_error)?
            .collect::<Result<_, _>>()
            .map_err(database_error)
    }

    pub(crate) fn identities(
        &self,
        library_id: &str,
    ) -> Result<HashMap<PathBuf, String>, AppError> {
        let mut statement = self
            .0
            .prepare("SELECT relative_path,id FROM assets WHERE library_id=?1")
            .map_err(database_error)?;
        statement
            .query_map([library_id], |row| {
                let bytes: Vec<u8> = row.get(0)?;
                Ok((PathBuf::from(OsString::from_vec(bytes)), row.get(1)?))
            })
            .map_err(database_error)?
            .collect::<Result<_, _>>()
            .map_err(database_error)
    }

    pub(crate) fn record_status(&self, id: &str, status: &ScanStatus) -> Result<(), AppError> {
        self.0
            .execute(
                "UPDATE libraries SET last_scan_id=?2,last_scan_phase=?3 WHERE id=?1 AND (?3='running' OR last_scan_id=?2)",
                params![id, status.scan_id, phase(&status.phase)],
            )
            .map_err(database_error)?;
        Ok(())
    }

    pub(crate) fn reconcile(
        &mut self,
        id: &str,
        status: &ScanStatus,
        assets: &[StoredAsset],
    ) -> Result<(), AppError> {
        let transaction = self.0.transaction().map_err(database_error)?;
        // A revoked worker must not publish over a new scan after trusted reselection.
        let updated = transaction.execute(
            "UPDATE libraries SET last_scan_phase=?3 WHERE id=?1 AND last_scan_id=?2 AND last_scan_phase='running'",
            params![id,status.scan_id,phase(&status.phase)],
        ).map_err(database_error)?;
        if updated != 1 {
            return Err(AppError::new(
                ErrorCode::Busy,
                "A newer scan replaced these results.",
            ));
        }
        for entry in assets {
            let asset = &entry.asset;
            transaction.execute("INSERT INTO assets(id,library_id,relative_path,display_name,display_path,extension,size_bytes,modified_seconds,modified_nanoseconds,availability,last_seen_scan_id)
                VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)
                ON CONFLICT(library_id,relative_path) DO UPDATE SET display_name=excluded.display_name,display_path=excluded.display_path,extension=excluded.extension,size_bytes=excluded.size_bytes,modified_seconds=excluded.modified_seconds,modified_nanoseconds=excluded.modified_nanoseconds,availability=excluded.availability,last_seen_scan_id=excluded.last_seen_scan_id",
                params![asset.id,id,entry.relative_path.as_os_str().as_bytes(),asset.name,asset.relative_display_path,asset.extension,entry.size_bytes,entry.modified_seconds,entry.modified_nanoseconds,availability(&asset.availability),status.scan_id]).map_err(database_error)?;
        }
        if status.phase == ScanPhase::Complete {
            transaction.execute("UPDATE assets SET availability='missing' WHERE library_id=?1 AND last_seen_scan_id<>?2", params![id,status.scan_id]).map_err(database_error)?;
            transaction.execute("UPDATE libraries SET last_successful_scan_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?1", [id]).map_err(database_error)?;
        }
        transaction
            .execute(
                "UPDATE libraries SET last_scan_id=?2,last_scan_phase=?3 WHERE id=?1",
                params![id, status.scan_id, phase(&status.phase)],
            )
            .map_err(database_error)?;
        transaction.commit().map_err(database_error)
    }

    pub(crate) fn query(&mut self, query: &AssetQuery) -> Result<AssetPage, AppError> {
        if query.limit == 0
            || query.limit > 200
            || query.offset > 1_000_000
            || query.search.len() > 1024
            || query.search.contains('\0')
        {
            return Err(AppError::new(
                ErrorCode::InvalidInput,
                "Use a page size of 1–200, offset up to 1000000, and search up to 1024 bytes without NUL characters.",
            ));
        }
        let pattern = format!(
            "%{}%",
            query
                .search
                .replace('\\', "\\\\")
                .replace('%', "\\%")
                .replace('_', "\\_")
        );
        let transaction = self.0.transaction().map_err(database_error)?;
        let scan_id: Option<String> = transaction
            .query_row(
                "SELECT last_scan_id FROM libraries WHERE id=?1",
                [&query.library_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(database_error)?
            .ok_or_else(|| {
                AppError::new(ErrorCode::NotFound, "The saved library was not found.")
            })?;
        let filter = r"library_id=?1 AND (display_name LIKE ?2 ESCAPE '\' OR display_path LIKE ?2 ESCAPE '\')";
        let total: u32 = transaction
            .query_row(
                &format!("SELECT count(*) FROM assets WHERE {filter}"),
                params![query.library_id, pattern],
                |row| row.get(0),
            )
            .map_err(database_error)?;
        let assets = {
            let mut statement = transaction.prepare(&format!("SELECT id,display_name,display_path,extension,availability FROM assets WHERE {filter} ORDER BY display_path COLLATE BINARY,id LIMIT ?3 OFFSET ?4")).map_err(database_error)?;
            statement
                .query_map(
                    params![query.library_id, pattern, query.limit, query.offset],
                    |row| {
                        let state: String = row.get(4)?;
                        Ok(AudioAsset {
                            id: row.get(0)?,
                            name: row.get(1)?,
                            relative_display_path: row.get(2)?,
                            extension: row.get(3)?,
                            duration_seconds: None,
                            availability: match state.as_str() {
                                "available" => Availability::Available,
                                "missing" => Availability::Missing,
                                "inaccessible" => Availability::Inaccessible,
                                _ => Availability::Unknown,
                            },
                        })
                    },
                )
                .map_err(database_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(database_error)?
        };
        transaction.commit().map_err(database_error)?;
        Ok(AssetPage {
            scan_id,
            assets,
            total,
            offset: query.offset,
            limit: query.limit,
        })
    }
}

fn phase(value: &ScanPhase) -> &'static str {
    match value {
        ScanPhase::Idle => "idle",
        ScanPhase::Running => "running",
        ScanPhase::Complete => "complete",
        ScanPhase::Partial => "partial",
        ScanPhase::Failed => "failed",
    }
}
fn availability(value: &Availability) -> &'static str {
    match value {
        Availability::Available => "available",
        Availability::Missing => "missing",
        Availability::Inaccessible => "inaccessible",
        Availability::Unknown => "unknown",
    }
}

fn migrate(connection: &mut Connection, migrations: &[&str]) -> Result<(), AppError> {
    let transaction = connection.transaction().map_err(database_error)?;
    let version: u32 = transaction
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(database_error)?;
    if version as usize > migrations.len() {
        return Err(AppError::new(
            ErrorCode::DatabaseError,
            "This database requires a newer SoundShelf version.",
        ));
    }
    for (index, sql) in migrations.iter().enumerate().skip(version as usize) {
        transaction.execute_batch(sql).map_err(database_error)?;
        transaction
            .pragma_update(None, "user_version", (index + 1) as u32)
            .map_err(database_error)?;
    }
    transaction.commit().map_err(database_error)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fresh_database_reopens_and_upgrades_without_recreating_tables() {
        let mut bytes = [0u8; 16];
        getrandom::fill(&mut bytes).unwrap();
        let path = std::env::temp_dir().join(format!("soundshelf-db-{:x?}.sqlite3", bytes));
        {
            let mut db = Database::open(&path).unwrap();
            db.register(Path::new("/synthetic/audio")).unwrap();
        }
        let db = Database::open(&path).unwrap();
        assert_eq!(db.libraries().unwrap().len(), 1);
        let mut connection = db.0;
        migrate(
            &mut connection,
            &[
                include_str!("../migrations/001_initial.sql"),
                "ALTER TABLE libraries ADD COLUMN test_upgrade TEXT;",
            ],
        )
        .unwrap();
        assert_eq!(
            connection
                .pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
                .unwrap(),
            2
        );
        assert_eq!(
            connection
                .query_row("SELECT count(*) FROM libraries", [], |r| r.get::<_, u32>(0))
                .unwrap(),
            1
        );
        drop(connection);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn migrations_are_versioned_atomic_and_enforce_foreign_keys() {
        let mut connection = Connection::open_in_memory().unwrap();
        assert!(
            migrate(
                &mut connection,
                &["CREATE TABLE example(id INTEGER);", "INVALID SQL"]
            )
            .is_err()
        );
        assert_eq!(
            connection
                .pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
                .unwrap(),
            0
        );
        assert!(connection.prepare("SELECT * FROM example").is_err());
        let db = Database::memory();
        assert!(
            db.0.execute("INSERT INTO asset_tags VALUES('missing','missing')", [])
                .is_err()
        );
        assert_eq!(
            db.0.pragma_query_value(None, "foreign_keys", |r| r.get::<_, u32>(0))
                .unwrap(),
            1
        );
        let mut newer = Connection::open_in_memory().unwrap();
        newer.pragma_update(None, "user_version", 99).unwrap();
        assert!(migrate(&mut newer, &[""]).is_err());
    }
}
