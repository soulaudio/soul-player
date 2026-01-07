-- Create sources table for multi-source architecture
-- Sources represent where music comes from (local files, remote servers, etc.)

CREATE TABLE IF NOT EXISTS sources (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    source_type TEXT NOT NULL,  -- 'local' or 'server'

    -- Server config (NULL for local)
    server_url TEXT,
    server_username TEXT,
    server_token TEXT,  -- Cached auth token

    is_active BOOLEAN NOT NULL DEFAULT 0,
    is_online BOOLEAN NOT NULL DEFAULT 1,
    last_sync_at TEXT,

    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),

    -- Only one active server at a time
    CHECK (
        source_type = 'local' OR
        (source_type = 'server' AND server_url IS NOT NULL)
    )
);

-- Ensure only one active server
CREATE UNIQUE INDEX IF NOT EXISTS idx_active_server
ON sources(is_active)
WHERE source_type = 'server' AND is_active = 1;

-- Always have exactly one local source (ID = 1)
INSERT INTO sources (id, name, source_type, is_active, is_online)
VALUES (1, 'Local Files', 'local', 1, 1);
