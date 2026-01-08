-- Create sync state tracking tables
-- These tables support the sync/doctor functionality with persistent state

-- Global sync state (single row, always id=1)
CREATE TABLE IF NOT EXISTS sync_state (
    id INTEGER PRIMARY KEY CHECK (id = 1),

    -- Current sync status
    status TEXT NOT NULL DEFAULT 'idle',  -- 'idle', 'scanning', 'extracting', 'validating', 'cleaning', 'error'

    -- Progress tracking
    phase TEXT,                            -- Current phase name
    total_items INTEGER DEFAULT 0,
    processed_items INTEGER DEFAULT 0,
    successful_items INTEGER DEFAULT 0,
    failed_items INTEGER DEFAULT 0,

    -- Current operation
    current_item TEXT,                     -- File/track being processed

    -- Timestamps
    started_at TEXT,
    completed_at TEXT,
    last_updated_at TEXT NOT NULL DEFAULT (datetime('now')),

    -- Last known migration version (for detecting schema changes)
    last_migration_version TEXT,

    -- Error tracking
    error_message TEXT
);

-- Insert initial row
INSERT INTO sync_state (id, status) VALUES (1, 'idle');

-- Sync errors log
CREATE TABLE IF NOT EXISTS sync_errors (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    sync_session_id TEXT NOT NULL,         -- UUID for each sync run
    occurred_at TEXT NOT NULL DEFAULT (datetime('now')),

    -- Error details
    phase TEXT NOT NULL,                   -- Which phase failed
    item_path TEXT,                        -- File/track that failed
    error_type TEXT NOT NULL,              -- 'file_missing', 'metadata_corrupt', 'database_error', etc.
    error_message TEXT NOT NULL,

    -- Resolution status
    resolved INTEGER NOT NULL DEFAULT 0,
    resolved_at TEXT,
    resolution_notes TEXT
);

-- Create indexes for efficient queries
CREATE INDEX IF NOT EXISTS idx_sync_errors_session ON sync_errors(sync_session_id);
CREATE INDEX IF NOT EXISTS idx_sync_errors_occurred_at ON sync_errors(occurred_at DESC);
CREATE INDEX IF NOT EXISTS idx_sync_errors_resolved ON sync_errors(resolved);
