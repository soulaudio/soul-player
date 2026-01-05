-- Create playlists table (user-owned)
CREATE TABLE IF NOT EXISTS playlists (
    id TEXT PRIMARY KEY NOT NULL,
    owner_id TEXT NOT NULL,
    name TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (owner_id) REFERENCES users(id) ON DELETE CASCADE
);

-- Create index on owner for efficient user playlist queries
CREATE INDEX IF NOT EXISTS idx_playlists_owner ON playlists(owner_id);
