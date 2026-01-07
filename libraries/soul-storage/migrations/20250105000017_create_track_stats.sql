-- Track statistics (play counts, skip counts, ratings, etc.)

CREATE TABLE IF NOT EXISTS track_stats (
    track_id TEXT PRIMARY KEY,

    -- Play statistics
    play_count INTEGER NOT NULL DEFAULT 0,
    skip_count INTEGER NOT NULL DEFAULT 0,
    last_played_at TEXT,

    -- User ratings (1-5 stars, NULL = unrated)
    rating INTEGER CHECK (rating IS NULL OR (rating >= 1 AND rating <= 5)),

    -- Timestamps
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),

    FOREIGN KEY (track_id) REFERENCES tracks(id) ON DELETE CASCADE
);

-- Index for sorting by play count
CREATE INDEX IF NOT EXISTS idx_track_stats_play_count ON track_stats(play_count DESC);

-- Index for recently played
CREATE INDEX IF NOT EXISTS idx_track_stats_last_played ON track_stats(last_played_at DESC);

-- Index for ratings
CREATE INDEX IF NOT EXISTS idx_track_stats_rating ON track_stats(rating DESC);
