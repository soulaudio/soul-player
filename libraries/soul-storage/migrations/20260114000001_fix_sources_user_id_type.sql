-- Fix sources.user_id and source_sync_state.user_id types to match users.id (TEXT instead of INTEGER)
-- The original migration used INTEGER but users.id is TEXT (UUID)

-- SQLite doesn't support ALTER COLUMN TYPE, so we need to:
-- 1. Create new table with correct schema
-- 2. Copy data
-- 3. Drop old table
-- 4. Rename new table

-- ==========================
-- Fix sources table
-- ==========================

-- Create new sources table with TEXT user_id
CREATE TABLE sources_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    source_type TEXT NOT NULL,
    server_url TEXT,
    server_username TEXT,
    server_token TEXT,
    is_active BOOLEAN NOT NULL DEFAULT 0,
    is_online BOOLEAN NOT NULL DEFAULT 1,
    last_sync_at TEXT,
    user_id TEXT REFERENCES users(id),  -- Changed from INTEGER to TEXT
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    CHECK (
        source_type = 'local' OR
        (source_type = 'server' AND server_url IS NOT NULL)
    )
);

-- Copy existing data (user_id will be NULL for existing sources)
INSERT INTO sources_new (id, name, source_type, server_url, server_username, server_token,
                         is_active, is_online, last_sync_at, user_id, created_at, updated_at)
SELECT id, name, source_type, server_url, server_username, server_token,
       is_active, is_online, last_sync_at, NULL, created_at, updated_at
FROM sources;

-- Drop old table
DROP TABLE sources;

-- Rename new table
ALTER TABLE sources_new RENAME TO sources;

-- Recreate indices
CREATE UNIQUE INDEX IF NOT EXISTS idx_active_server
ON sources(is_active)
WHERE source_type = 'server' AND is_active = 1;

CREATE INDEX IF NOT EXISTS idx_sources_user_id ON sources(user_id);

-- ==========================
-- Fix source_sync_state table
-- ==========================

-- Create new source_sync_state table with TEXT user_id
CREATE TABLE source_sync_state_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_id INTEGER NOT NULL REFERENCES sources(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id),  -- Changed from INTEGER to TEXT
    last_sync_at INTEGER,
    last_sync_direction TEXT,
    sync_status TEXT NOT NULL DEFAULT 'idle',
    current_operation TEXT,
    current_item TEXT,
    total_items INTEGER DEFAULT 0,
    processed_items INTEGER DEFAULT 0,
    tracks_uploaded INTEGER DEFAULT 0,
    tracks_downloaded INTEGER DEFAULT 0,
    tracks_updated INTEGER DEFAULT 0,
    tracks_deleted INTEGER DEFAULT 0,
    error_message TEXT,
    server_sync_token TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(source_id, user_id)
);

-- Copy existing data (if any exists, it will be dropped as user_id can't be converted from INTEGER to TEXT)
-- Since this table is for sync state which is transient, it's safe to drop existing data
-- INSERT INTO source_sync_state_new SELECT * FROM source_sync_state WHERE 0 = 1;

-- Drop old table
DROP TABLE IF EXISTS source_sync_state;

-- Rename new table
ALTER TABLE source_sync_state_new RENAME TO source_sync_state;

-- Recreate indices
CREATE INDEX IF NOT EXISTS idx_source_sync_state_source ON source_sync_state(source_id);
CREATE INDEX IF NOT EXISTS idx_source_sync_state_user ON source_sync_state(user_id);
CREATE INDEX IF NOT EXISTS idx_source_sync_state_status ON source_sync_state(sync_status);
