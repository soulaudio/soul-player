-- Library sources table - watched folders for library scanning
-- This is separate from the remote 'sources' table which handles local vs server sync

CREATE TABLE IF NOT EXISTS library_sources (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT NOT NULL,
    device_id TEXT NOT NULL,
    name TEXT NOT NULL,
    -- 'watched' = monitor folder, never modify files
    -- 'managed' = organized folder where imports go (deprecated, use managed_library_settings)
    source_type TEXT NOT NULL DEFAULT 'watched' CHECK (source_type IN ('watched')),
    path TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    -- When files disappear, mark them unavailable (soft delete) vs remove from DB
    sync_deletes INTEGER NOT NULL DEFAULT 1,
    last_scan_at INTEGER,
    -- 'idle' | 'scanning' | 'error'
    scan_status TEXT DEFAULT 'idle' CHECK (scan_status IN ('idle', 'scanning', 'error')),
    error_message TEXT,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    -- Note: user_id and device_id are string identifiers, not foreign keys
    -- This allows flexibility for desktop (single user) and server (multi-user) modes
    UNIQUE(user_id, device_id, path)
);

CREATE INDEX IF NOT EXISTS idx_library_sources_user_device
    ON library_sources(user_id, device_id);
CREATE INDEX IF NOT EXISTS idx_library_sources_enabled
    ON library_sources(enabled) WHERE enabled = 1;
