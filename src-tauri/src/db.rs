//! The index grants no filesystem authority. Native paths are stored as BLOBs.
use crate::{
    contracts::{
        AppError, AssetPage, AssetQuery, AudioAsset, Availability, ErrorCode, LibraryInfo,
        ScanPhase, ScanStatus, Tag,
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

    pub(crate) fn playback_asset(
        &self,
        library_id: &str,
        asset_id: &str,
    ) -> Result<(PathBuf, String), AppError> {
        self.0
            .query_row(
                "SELECT relative_path,extension FROM assets WHERE library_id=?1 AND id=?2",
                params![library_id, asset_id],
                |row| {
                    let bytes: Vec<u8> = row.get(0)?;
                    Ok((PathBuf::from(OsString::from_vec(bytes)), row.get(1)?))
                },
            )
            .optional()
            .map_err(database_error)?
            .ok_or_else(|| AppError::new(ErrorCode::NotFound, "The audio asset was not found."))
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

    pub(crate) fn tags(&self) -> Result<Vec<Tag>, AppError> {
        read_tags(&self.0, None)
    }

    pub(crate) fn create_tag(&mut self, name: &str) -> Result<Tag, AppError> {
        let (name, normalized) = normalize_tag(name)?;
        let tx = self.0.transaction().map_err(database_error)?;
        check_duplicate(&tx, &normalized, "")?;
        let tag = Tag {
            id: opaque_id()?,
            name,
        };
        tx.execute(
            "INSERT INTO tags(id,normalized_name,display_name) VALUES(?1,?2,?3)",
            params![tag.id, normalized, tag.name],
        )
        .map_err(database_error)?;
        tx.commit().map_err(database_error)?;
        Ok(tag)
    }

    pub(crate) fn rename_tag(&mut self, id: &str, name: &str) -> Result<Tag, AppError> {
        let (name, normalized) = normalize_tag(name)?;
        let tx = self.0.transaction().map_err(database_error)?;
        require_tag(&tx, id)?;
        check_duplicate(&tx, &normalized, id)?;
        tx.execute(
            "UPDATE tags SET normalized_name=?2,display_name=?3 WHERE id=?1",
            params![id, normalized, name],
        )
        .map_err(database_error)?;
        tx.commit().map_err(database_error)?;
        Ok(Tag {
            id: id.into(),
            name,
        })
    }

    pub(crate) fn delete_tag(&mut self, id: &str) -> Result<(), AppError> {
        let tx = self.0.transaction().map_err(database_error)?;
        require_tag(&tx, id)?;
        tx.execute("DELETE FROM asset_tags WHERE tag_id=?1", [id])
            .map_err(database_error)?;
        tx.execute("DELETE FROM tags WHERE id=?1", [id])
            .map_err(database_error)?;
        tx.commit().map_err(database_error)
    }

    pub(crate) fn set_asset_tags(
        &mut self,
        library: &str,
        asset: &str,
        ids: &[String],
    ) -> Result<(), AppError> {
        validate_tag_ids(ids)?;
        let tx = self.0.transaction().map_err(database_error)?;
        require_asset(&tx, library, asset)?;
        for id in ids {
            require_tag(&tx, id)?;
        }
        tx.execute("DELETE FROM asset_tags WHERE asset_id=?1", [asset])
            .map_err(database_error)?;
        for id in ids {
            tx.execute(
                "INSERT INTO asset_tags(asset_id,tag_id) VALUES(?1,?2)",
                params![asset, id],
            )
            .map_err(database_error)?;
        }
        tx.commit().map_err(database_error)
    }

    pub(crate) fn set_asset_favorite(
        &mut self,
        library: &str,
        asset: &str,
        favorite: bool,
    ) -> Result<(), AppError> {
        let tx = self.0.transaction().map_err(database_error)?;
        require_asset(&tx, library, asset)?;
        tx.execute(
            "UPDATE assets SET favorite=?3 WHERE library_id=?1 AND id=?2",
            params![library, asset, favorite],
        )
        .map_err(database_error)?;
        tx.commit().map_err(database_error)
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
        validate_tag_ids(&query.tag_ids)?;
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
        let mut filter = String::from(
            r"a.library_id=?1 AND (a.display_name LIKE ?2 ESCAPE '\' OR a.display_path LIKE ?2 ESCAPE '\' OR EXISTS (SELECT 1 FROM asset_tags at JOIN tags t ON t.id=at.tag_id WHERE at.asset_id=a.id AND t.display_name LIKE ?2 ESCAPE '\'))",
        );
        if query.favorites_only {
            filter.push_str(" AND a.favorite=1");
        }
        // Only fixed SQL and numbered placeholders are composed; values are always bound.
        for index in 0..query.tag_ids.len() {
            filter.push_str(&format!(
                " AND EXISTS (SELECT 1 FROM asset_tags f WHERE f.asset_id=a.id AND f.tag_id=?{})",
                index + 3
            ));
        }
        let mut values: Vec<rusqlite::types::Value> =
            vec![query.library_id.clone().into(), pattern.into()];
        values.extend(query.tag_ids.iter().cloned().map(Into::into));
        let total: u32 = transaction
            .query_row(
                &format!("SELECT count(*) FROM assets a WHERE {filter}"),
                rusqlite::params_from_iter(values.iter()),
                |row| row.get(0),
            )
            .map_err(database_error)?;
        let assets = {
            let mut statement = transaction.prepare(&format!("SELECT a.id,a.display_name,a.display_path,a.extension,a.availability,a.favorite FROM assets a WHERE {filter} ORDER BY a.display_path COLLATE BINARY,a.id LIMIT ?{} OFFSET ?{}", values.len()+1, values.len()+2)).map_err(database_error)?;
            values.push(query.limit.into());
            values.push(query.offset.into());
            let mut assets = statement
                .query_map(rusqlite::params_from_iter(values.iter()), |row| {
                    let state: String = row.get(4)?;
                    Ok(AudioAsset {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        relative_display_path: row.get(2)?,
                        extension: row.get(3)?,
                        duration_seconds: None,
                        favorite: row.get(5)?,
                        tags: Vec::new(),
                        availability: match state.as_str() {
                            "available" => Availability::Available,
                            "missing" => Availability::Missing,
                            "inaccessible" => Availability::Inaccessible,
                            _ => Availability::Unknown,
                        },
                    })
                })
                .map_err(database_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(database_error)?;
            for asset in &mut assets {
                asset.tags = read_tags(&transaction, Some(&asset.id))?;
            }
            assets
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

fn normalize_tag(input: &str) -> Result<(String, String), AppError> {
    let name = input.trim();
    if name.is_empty() || name.chars().count() > 64 || name.chars().any(char::is_control) {
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            "Use a tag name of 1–64 characters without control characters.",
        ));
    }
    Ok((name.into(), name.to_lowercase()))
}

fn validate_tag_ids(ids: &[String]) -> Result<(), AppError> {
    let unique: std::collections::HashSet<_> = ids.iter().collect();
    if ids.len() > 32
        || unique.len() != ids.len()
        || ids
            .iter()
            .any(|id| id.is_empty() || id.len() > 128 || id.contains('\0'))
    {
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            "Select up to 32 distinct tag IDs.",
        ));
    }
    Ok(())
}
fn require_tag(db: &Connection, id: &str) -> Result<(), AppError> {
    if !db
        .query_row("SELECT EXISTS(SELECT 1 FROM tags WHERE id=?1)", [id], |r| {
            r.get::<_, bool>(0)
        })
        .map_err(database_error)?
    {
        return Err(AppError::new(ErrorCode::NotFound, "The tag was not found."));
    }
    Ok(())
}
fn require_asset(db: &Connection, library: &str, asset: &str) -> Result<(), AppError> {
    if !db
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM assets WHERE library_id=?1 AND id=?2)",
            params![library, asset],
            |r| r.get::<_, bool>(0),
        )
        .map_err(database_error)?
    {
        return Err(AppError::new(
            ErrorCode::NotFound,
            "The audio asset was not found.",
        ));
    }
    Ok(())
}
fn check_duplicate(db: &Connection, normalized: &str, id: &str) -> Result<(), AppError> {
    if db
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM tags WHERE normalized_name=?1 AND id<>?2)",
            params![normalized, id],
            |r| r.get::<_, bool>(0),
        )
        .map_err(database_error)?
    {
        return Err(AppError::new(
            ErrorCode::DuplicateTag,
            "A tag with this name already exists.",
        ));
    }
    Ok(())
}
fn read_tags(db: &Connection, asset: Option<&str>) -> Result<Vec<Tag>, AppError> {
    let mut stmt = db.prepare("SELECT t.id,t.display_name FROM tags t WHERE (?1 IS NULL OR EXISTS (SELECT 1 FROM asset_tags a WHERE a.tag_id=t.id AND a.asset_id=?1)) ORDER BY t.normalized_name COLLATE BINARY,t.id").map_err(database_error)?;
    stmt.query_map([asset], |r| {
        Ok(Tag {
            id: r.get(0)?,
            name: r.get(1)?,
        })
    })
    .map_err(database_error)?
    .collect::<Result<_, _>>()
    .map_err(database_error)
}

#[cfg(test)]
mod annotation_tests {
    use super::*;

    fn seeded() -> (Database, LibraryInfo) {
        let mut db = Database::memory();
        let library = db.register(Path::new("/synthetic/audio")).unwrap();
        for (id, state) in [("a", "available"), ("b", "missing"), ("c", "inaccessible")] {
            db.0.execute("INSERT INTO assets(id,library_id,relative_path,display_name,display_path,extension,size_bytes,availability,last_seen_scan_id) VALUES(?1,?2,?3,?1,?1,'wav','1',?4,'scan')", params![id,library.id,id.as_bytes(),state]).unwrap();
        }
        (db, library)
    }
    fn query(id: &str) -> AssetQuery {
        AssetQuery {
            library_id: id.into(),
            search: String::new(),
            tag_ids: vec![],
            favorites_only: false,
            offset: 0,
            limit: 1,
        }
    }

    #[test]
    fn validation_normalization_duplicates_and_sql_input() {
        let mut db = Database::memory();
        for input in ["", " \t ", "a\0b", "a\nb", &"雨".repeat(65)] {
            assert_eq!(
                db.create_tag(input).unwrap_err().code,
                ErrorCode::InvalidInput
            );
        }
        let tag = db.create_tag("  Ä RAIN 雨  ").unwrap();
        assert_eq!(tag.name, "Ä RAIN 雨");
        assert_eq!(
            db.create_tag("ä rain 雨").unwrap_err().code,
            ErrorCode::DuplicateTag
        );
        let other = db.create_tag("Other").unwrap();
        assert_eq!(
            db.rename_tag(&other.id, "ä rain 雨").unwrap_err().code,
            ErrorCode::DuplicateTag
        );
        assert_eq!(
            db.rename_tag("absent", "OK").unwrap_err().code,
            ErrorCode::NotFound
        );
        assert_eq!(
            db.delete_tag("absent").unwrap_err().code,
            ErrorCode::NotFound
        );
        assert_eq!(db.rename_tag(&tag.id, "Ä Rain 雨").unwrap().id, tag.id);
        db.create_tag("%_\\'; DROP TABLE tags; --").unwrap();
        db.create_tag(&"雨".repeat(64)).unwrap();
        assert_eq!(db.tags().unwrap().len(), 4);
    }

    #[test]
    fn annotations_filters_pages_and_unavailable_records() {
        let (mut db, library) = seeded();
        let rain = db.create_tag("雨%_\\").unwrap();
        let wind = db.create_tag("Wind").unwrap();
        for asset in ["a", "b", "c"] {
            db.set_asset_tags(&library.id, asset, &[rain.id.clone(), wind.id.clone()])
                .unwrap();
            db.set_asset_favorite(&library.id, asset, true).unwrap();
        }
        let mut request = query(&library.id);
        request.search = "雨%_\\".into();
        request.tag_ids = vec![rain.id.clone(), wind.id.clone()];
        request.favorites_only = true;
        let first = db.query(&request).unwrap();
        assert_eq!(first.total, 3);
        assert_eq!(first.assets[0].id, "a");
        request.offset = 1;
        let second = db.query(&request).unwrap();
        assert_eq!(second.assets[0].id, "b");
        assert_eq!(second.assets[0].availability, Availability::Missing);
        assert!(second.assets[0].favorite);
        assert_eq!(second.assets[0].tags.len(), 2);
        request.offset = 2;
        assert_eq!(
            db.query(&request).unwrap().assets[0].availability,
            Availability::Inaccessible
        );
        db.set_asset_favorite(&library.id, "b", false).unwrap();
        db.set_asset_tags(&library.id, "c", std::slice::from_ref(&rain.id))
            .unwrap();
        request.offset = 0;
        assert_eq!(db.query(&request).unwrap().total, 1);
        db.delete_tag(&wind.id).unwrap();
        assert_eq!(db.query(&request).unwrap().total, 0);
        request.tag_ids = vec![rain.id];
        assert_eq!(db.query(&request).unwrap().total, 2);
        request.search = "' OR 1=1 --".into();
        assert_eq!(db.query(&request).unwrap().total, 0);
        assert_eq!(
            db.set_asset_favorite("wrong", "a", true).unwrap_err().code,
            ErrorCode::NotFound
        );
        assert_eq!(
            db.set_asset_tags(&library.id, "absent", &[])
                .unwrap_err()
                .code,
            ErrorCode::NotFound
        );
    }

    #[test]
    fn failed_annotation_updates_and_deletes_roll_back() {
        let (mut db, library) = seeded();
        let one = db.create_tag("One").unwrap();
        let two = db.create_tag("Two").unwrap();
        db.set_asset_tags(&library.id, "a", std::slice::from_ref(&one.id))
            .unwrap();
        assert_eq!(
            db.set_asset_tags(&library.id, "a", &[two.id.clone(), "missing".into()])
                .unwrap_err()
                .code,
            ErrorCode::NotFound
        );
        assert_eq!(
            db.set_asset_tags(&library.id, "a", &[one.id.clone(), one.id.clone()])
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
        assert_eq!(
            db.set_asset_tags(&library.id, "a", &vec!["x".into(); 33])
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
        db.0.execute_batch("CREATE TRIGGER reject_assignment BEFORE INSERT ON asset_tags BEGIN SELECT RAISE(ABORT,'test'); END;").unwrap();
        assert_eq!(
            db.set_asset_tags(&library.id, "a", std::slice::from_ref(&two.id))
                .unwrap_err()
                .code,
            ErrorCode::DatabaseError
        );
        assert_eq!(read_tags(&db.0, Some("a")).unwrap(), vec![one.clone()]);
        db.0.execute_batch("CREATE TRIGGER reject_delete BEFORE DELETE ON tags BEGIN SELECT RAISE(ABORT,'test'); END;").unwrap();
        assert_eq!(
            db.delete_tag(&one.id).unwrap_err().code,
            ErrorCode::DatabaseError
        );
        assert_eq!(read_tags(&db.0, Some("a")).unwrap(), vec![one]);
    }

    #[test]
    fn tag_search_is_literal_and_never_duplicates_assets() {
        let (mut db, library) = seeded();
        let literal = db.create_tag("100%_\\' 日本語").unwrap();
        let similar = db.create_tag("100XYZ 日本語").unwrap();
        let another = db.create_tag("100 percent").unwrap();
        db.set_asset_tags(&library.id, "a", &[literal.id.clone(), another.id])
            .unwrap();
        db.set_asset_tags(&library.id, "b", &[similar.id]).unwrap();
        let mut request = query(&library.id);
        request.limit = 10;
        for text in ["%", "_", "\\", "'", "100%_\\'", "100%_\\' 日本語"] {
            request.search = text.into();
            let result = db.query(&request).unwrap();
            assert_eq!(result.total, 1, "{text}");
            assert_eq!(result.assets[0].id, "a");
        }
        request.search = "100".into();
        let result = db.query(&request).unwrap();
        assert_eq!(result.total, 2);
        assert_eq!(
            result
                .assets
                .iter()
                .map(|a| a.id.as_str())
                .collect::<Vec<_>>(),
            vec!["a", "b"]
        );
        request.tag_ids = vec![literal.id.clone(), literal.id];
        assert_eq!(
            db.query(&request).unwrap_err().code,
            ErrorCode::InvalidInput
        );
    }

    #[test]
    fn annotation_ipc_contracts() {
        use crate::contracts::*;
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("../../src/lib/fixtures/ipc.json")).unwrap();
        macro_rules! check {
            ($ty:ty,$key:literal) => {
                let value: $ty = serde_json::from_value(fixture[$key].clone()).unwrap();
                assert_eq!(serde_json::to_value(value).unwrap(), fixture[$key]);
                let mut invalid = fixture[$key].clone();
                invalid["path"] = "/etc/passwd".into();
                assert!(serde_json::from_value::<$ty>(invalid).is_err());
            };
        }
        check!(CreateTagRequest, "createTagRequest");
        check!(RenameTagRequest, "renameTagRequest");
        check!(DeleteTagRequest, "deleteTagRequest");
        check!(SetAssetTagsRequest, "setAssetTagsRequest");
        check!(SetAssetFavoriteRequest, "setAssetFavoriteRequest");
        assert_eq!(
            serde_json::to_value(AudioAsset {
                id: "asset-fixture".into(),
                name: "kick.WAV".into(),
                duration_seconds: None,
                relative_display_path: "drums/kick.WAV".into(),
                extension: "wav".into(),
                availability: Availability::Available,
                favorite: true,
                tags: vec![Tag {
                    id: "tag-fixture".into(),
                    name: "雨".into()
                }],
            })
            .unwrap(),
            fixture["annotatedAsset"]
        );
        assert_eq!(
            serde_json::to_value(Tag {
                id: "tag-fixture".into(),
                name: "雨".into()
            })
            .unwrap(),
            fixture["tag"]
        );
    }
}
