-- Track availability across multiple sources
-- A track may be available from local files, remote servers, or both

CREATE TABLE IF NOT EXISTS track_sources (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    track_id TEXT NOT NULL,
    source_id INTEGER NOT NULL,

    -- Availability status
    status TEXT NOT NULL,  -- 'local_file', 'cached', 'stream_only', 'unavailable'

    -- Local storage
    local_file_path TEXT,              -- If downloaded/cached
    local_file_size INTEGER,
    downloaded_at TEXT,

    -- Server storage
    server_path TEXT,                  -- Path on server for streaming
    server_file_id TEXT,               -- Server's track ID

    -- Sync metadata
    last_verified_at TEXT,             -- Last time we checked availability

    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),

    FOREIGN KEY (track_id) REFERENCES tracks(id) ON DELETE CASCADE,
    FOREIGN KEY (source_id) REFERENCES sources(id) ON DELETE CASCADE,

    UNIQUE(track_id, source_id)
);

-- Indexes for common queries
CREATE INDEX IF NOT EXISTS idx_track_sources_track ON track_sources(track_id);
CREATE INDEX IF NOT EXISTS idx_track_sources_source ON track_sources(source_id);
CREATE INDEX IF NOT EXISTS idx_track_sources_status ON track_sources(status);
