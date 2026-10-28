CREATE TABLE libraries (
    id TEXT PRIMARY KEY,
    root_path BLOB NOT NULL UNIQUE,
    display_path TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    last_successful_scan_at TEXT,
    last_scan_id TEXT,
    last_scan_phase TEXT NOT NULL DEFAULT 'idle' CHECK(last_scan_phase IN ('idle','running','complete','partial','failed'))
);
CREATE TABLE assets (
    id TEXT PRIMARY KEY,
    library_id TEXT NOT NULL REFERENCES libraries(id),
    relative_path BLOB NOT NULL,
    display_name TEXT NOT NULL,
    display_path TEXT NOT NULL,
    extension TEXT NOT NULL,
    size_bytes TEXT NOT NULL,
    modified_seconds TEXT,
    modified_nanoseconds INTEGER,
    availability TEXT NOT NULL CHECK(availability IN ('available','missing','inaccessible','unknown')),
    last_seen_scan_id TEXT NOT NULL,
    favorite INTEGER NOT NULL DEFAULT 0 CHECK(favorite IN (0,1)),
    UNIQUE(library_id, relative_path)
);
CREATE INDEX assets_library_order ON assets(library_id, display_path, id);
CREATE TABLE tags (
    id TEXT PRIMARY KEY,
    normalized_name TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL
);
CREATE TABLE asset_tags (
    asset_id TEXT NOT NULL REFERENCES assets(id),
    tag_id TEXT NOT NULL REFERENCES tags(id),
    PRIMARY KEY(asset_id, tag_id)
);
