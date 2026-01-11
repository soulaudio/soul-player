-- Enhance sources table for full server source support
-- Adds user ownership, proper auth token management, and per-source sync state

-- Add user_id to sources for multi-user support
-- NULL user_id means the source is device-wide (like local source ID=1)
ALTER TABLE sources ADD COLUMN user_id INTEGER REFERENCES users(id);

-- Add index for user source lookups
CREATE INDEX IF NOT EXISTS idx_sources_user_id ON sources(user_id);

-- Server authentication tokens table
-- Separate from sources for security (tokens can be refreshed independently)
CREATE TABLE IF NOT EXISTS server_auth_tokens (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_id INTEGER NOT NULL REFERENCES sources(id) ON DELETE CASCADE,
    access_token TEXT NOT NULL,
    refresh_token TEXT,
    token_expires_at INTEGER,  -- Unix timestamp when access_token expires
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(source_id)
);

-- Per-source, per-user sync state tracking
-- Different from sync_state which is for local library scanning
CREATE TABLE IF NOT EXISTS source_sync_state (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_id INTEGER NOT NULL REFERENCES sources(id) ON DELETE CASCADE,
    user_id INTEGER NOT NULL REFERENCES users(id),

    -- Sync progress
    last_sync_at INTEGER,                    -- Unix timestamp
    last_sync_direction TEXT,                -- 'upload', 'download', 'both'
    sync_status TEXT NOT NULL DEFAULT 'idle', -- 'idle', 'syncing', 'error', 'cancelled'

    -- Progress tracking for ongoing sync
    current_operation TEXT,                  -- 'uploading', 'downloading', 'comparing'
    current_item TEXT,                       -- Current track being processed
    total_items INTEGER DEFAULT 0,
    processed_items INTEGER DEFAULT 0,

    -- Results from last sync
    tracks_uploaded INTEGER DEFAULT 0,
    tracks_downloaded INTEGER DEFAULT 0,
    tracks_updated INTEGER DEFAULT 0,
    tracks_deleted INTEGER DEFAULT 0,

    -- Error tracking
    error_message TEXT,

    -- Server's sync token for delta sync
    server_sync_token TEXT,

    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),

    UNIQUE(source_id, user_id)
);

-- Index for efficient sync state lookups
CREATE INDEX IF NOT EXISTS idx_source_sync_state_source ON source_sync_state(source_id);
CREATE INDEX IF NOT EXISTS idx_source_sync_state_user ON source_sync_state(user_id);
CREATE INDEX IF NOT EXISTS idx_source_sync_state_status ON source_sync_state(sync_status);
