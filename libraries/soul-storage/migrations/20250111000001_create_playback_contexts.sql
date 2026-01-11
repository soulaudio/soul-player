-- Create playback_contexts table for tracking what context (album, playlist, etc.)
-- a user is playing from. Used for "Jump Back Into" and "Now Playing" context display.

CREATE TABLE IF NOT EXISTS playback_contexts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT NOT NULL,
    context_type TEXT NOT NULL,  -- 'album', 'playlist', 'artist', 'genre', 'tracks'
    context_id TEXT,             -- ID of the album/playlist/artist/genre (NULL for 'tracks')
    context_name TEXT,           -- Cached name for display (avoids joins)
    context_artwork_path TEXT,   -- Cached artwork path for display
    last_played_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),

    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- Index for efficient user history queries (most recent first)
CREATE INDEX IF NOT EXISTS idx_playback_contexts_user ON playback_contexts(user_id, last_played_at DESC);

-- Unique constraint: one entry per context per user (upsert on conflict)
CREATE UNIQUE INDEX IF NOT EXISTS idx_playback_contexts_unique ON playback_contexts(user_id, context_type, COALESCE(context_id, ''));
