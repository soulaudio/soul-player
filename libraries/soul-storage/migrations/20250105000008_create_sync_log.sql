-- Create sync_log table for tracking sync operations
-- Used for delta sync between clients and server
CREATE TABLE IF NOT EXISTS sync_log (
    id TEXT PRIMARY KEY NOT NULL,
    entity_type TEXT NOT NULL,     -- 'track', 'playlist', 'user', etc.
    entity_id TEXT NOT NULL,
    operation TEXT NOT NULL,        -- 'create', 'update', 'delete'
    user_id TEXT,                   -- User who triggered the change (null for system)
    timestamp INTEGER NOT NULL,
    data TEXT,                      -- JSON-serialized change data
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE SET NULL
);

-- Indices for efficient sync queries
CREATE INDEX IF NOT EXISTS idx_sync_log_timestamp ON sync_log(timestamp);
CREATE INDEX IF NOT EXISTS idx_sync_log_entity_type ON sync_log(entity_type);
CREATE INDEX IF NOT EXISTS idx_sync_log_entity_id ON sync_log(entity_id);
CREATE INDEX IF NOT EXISTS idx_sync_log_user_id ON sync_log(user_id);
CREATE INDEX IF NOT EXISTS idx_sync_log_operation ON sync_log(operation);

-- Composite index for efficient delta sync queries
CREATE INDEX IF NOT EXISTS idx_sync_log_entity_timestamp ON sync_log(entity_type, timestamp);
