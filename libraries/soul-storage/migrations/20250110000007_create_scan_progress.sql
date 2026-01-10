-- Scan progress tracking - monitors ongoing library scans

CREATE TABLE IF NOT EXISTS scan_progress (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    library_source_id INTEGER NOT NULL,
    started_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    completed_at INTEGER,
    total_files INTEGER,
    processed_files INTEGER NOT NULL DEFAULT 0,
    new_files INTEGER NOT NULL DEFAULT 0,
    updated_files INTEGER NOT NULL DEFAULT 0,
    removed_files INTEGER NOT NULL DEFAULT 0,
    errors INTEGER NOT NULL DEFAULT 0,
    -- 'running' | 'completed' | 'failed' | 'cancelled'
    status TEXT NOT NULL DEFAULT 'running' CHECK (status IN ('running', 'completed', 'failed', 'cancelled')),
    error_message TEXT,
    FOREIGN KEY (library_source_id) REFERENCES library_sources(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_scan_progress_source
    ON scan_progress(library_source_id);
CREATE INDEX IF NOT EXISTS idx_scan_progress_status
    ON scan_progress(status) WHERE status = 'running';
