-- Fix track_stats to be per-user
-- Previously, track statistics (play_count, rating, etc.) were global
-- Now they are per-user to support multi-user functionality

-- Create new per-user track_stats table
CREATE TABLE IF NOT EXISTS track_stats_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT NOT NULL,
    track_id TEXT NOT NULL,

    -- Play statistics
    play_count INTEGER NOT NULL DEFAULT 0,
    skip_count INTEGER NOT NULL DEFAULT 0,
    last_played_at TEXT,

    -- User ratings (1-5 stars, NULL = unrated)
    rating INTEGER CHECK (rating IS NULL OR (rating >= 1 AND rating <= 5)),

    -- Timestamps
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),

    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (track_id) REFERENCES tracks(id) ON DELETE CASCADE,
    UNIQUE(user_id, track_id)
);

-- Migrate existing data to user_id='1' (default desktop user)
INSERT INTO track_stats_new (user_id, track_id, play_count, skip_count, last_played_at, rating, created_at, updated_at)
SELECT '1', track_id, play_count, skip_count, last_played_at, rating, created_at, updated_at
FROM track_stats;

-- Drop old table and rename new one
DROP TABLE track_stats;
ALTER TABLE track_stats_new RENAME TO track_stats;

-- Create indexes for efficient queries
CREATE INDEX IF NOT EXISTS idx_track_stats_user_track ON track_stats(user_id, track_id);
CREATE INDEX IF NOT EXISTS idx_track_stats_play_count ON track_stats(user_id, play_count DESC);
CREATE INDEX IF NOT EXISTS idx_track_stats_last_played ON track_stats(user_id, last_played_at DESC);
CREATE INDEX IF NOT EXISTS idx_track_stats_rating ON track_stats(user_id, rating DESC);
