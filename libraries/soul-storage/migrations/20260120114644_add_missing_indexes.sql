-- Add missing indexes for performance optimization

-- Index for track_sources status filtering (availability queries)
CREATE INDEX IF NOT EXISTS idx_track_sources_status ON track_sources(status);

-- Index for albums created_at (Recently Added sorting)
CREATE INDEX IF NOT EXISTS idx_albums_created_at ON albums(created_at DESC);

-- Index for artists sort_name (alphabetical sorting)
CREATE INDEX IF NOT EXISTS idx_artists_sort_name ON artists(sort_name COLLATE NOCASE);

-- Index for track_stats user_id (per-user filtering)
CREATE INDEX IF NOT EXISTS idx_track_stats_user_id ON track_stats(user_id);
