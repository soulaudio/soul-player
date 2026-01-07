-- Create play_history table for tracking user play history
-- Migration generated during SQLx compile-time query migration

CREATE TABLE IF NOT EXISTS play_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT NOT NULL,
    track_id INTEGER NOT NULL,
    play_duration_seconds REAL,
    completed INTEGER NOT NULL DEFAULT 1,
    played_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),

    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (track_id) REFERENCES tracks(id) ON DELETE CASCADE
);

-- Index for efficient user history queries
CREATE INDEX IF NOT EXISTS idx_play_history_user ON play_history(user_id, played_at DESC);

-- Index for efficient track play count queries
CREATE INDEX IF NOT EXISTS idx_play_history_track ON play_history(track_id, played_at DESC);
