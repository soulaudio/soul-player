-- Directory-level mtime tracking for incremental scanning.
-- Stores the last-known mtime of each directory so unchanged
-- directories can be skipped entirely during rescan.

CREATE TABLE IF NOT EXISTS scanned_directories (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    library_source_id INTEGER NOT NULL REFERENCES library_sources(id) ON DELETE CASCADE,
    path TEXT NOT NULL,
    dir_mtime INTEGER NOT NULL DEFAULT 0,
    file_count INTEGER NOT NULL DEFAULT 0,
    last_scanned_at INTEGER NOT NULL DEFAULT 0,
    UNIQUE(library_source_id, path)
);

CREATE INDEX idx_scanned_dirs_source ON scanned_directories(library_source_id);
