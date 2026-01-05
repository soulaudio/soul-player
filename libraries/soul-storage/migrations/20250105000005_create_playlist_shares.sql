-- Create playlist_shares table (collaboration)
CREATE TABLE IF NOT EXISTS playlist_shares (
    playlist_id TEXT NOT NULL,
    shared_with_user_id TEXT NOT NULL,
    permission TEXT NOT NULL CHECK (permission IN ('read', 'write')),
    shared_at INTEGER NOT NULL,
    PRIMARY KEY (playlist_id, shared_with_user_id),
    FOREIGN KEY (playlist_id) REFERENCES playlists(id) ON DELETE CASCADE,
    FOREIGN KEY (shared_with_user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- Create index on shared_with_user_id for efficient user share queries
CREATE INDEX IF NOT EXISTS idx_playlist_shares_user ON playlist_shares(shared_with_user_id);
