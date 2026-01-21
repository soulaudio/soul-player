-- Add composite indexes for frequently-used multi-column queries
--
-- This migration adds composite indexes to optimize queries that filter and sort
-- by multiple columns. These indexes improve performance for common query patterns
-- identified through code analysis.

-- For devices query: WHERE user_id = ? ORDER BY last_seen_at DESC (devices/mod.rs:75)
-- The existing separate indexes on user_id and last_seen_at are not optimal for this pattern.
-- A composite index allows the database to filter by user_id and sort by last_seen_at
-- in a single index scan instead of merging two separate indexes.
CREATE INDEX IF NOT EXISTS idx_devices_user_last_seen
ON devices(user_id, last_seen_at DESC);

-- For playback contexts with type filter:
-- WHERE user_id = ? AND context_type = ? (playback_contexts/mod.rs:163)
-- Also benefits JOIN queries in albums/mod.rs that filter by context_type
-- This allows efficient lookups when filtering contexts by both user and type,
-- and can be extended with ORDER BY last_played_at for sorted results.
CREATE INDEX IF NOT EXISTS idx_playback_contexts_user_type
ON playback_contexts(user_id, context_type, last_played_at DESC);

-- Note: The existing idx_playback_contexts_user (user_id, last_played_at DESC) is kept
-- for queries that only filter by user_id. SQLite will choose the most appropriate index.
--
-- Note: track_stats already has composite indexes (user_id, play_count) and
-- (user_id, last_played_at) from migration 20250108000001, so no additional indexes needed.
--
-- Note: track_sources queries primarily use IN clauses on track_id, which benefit from
-- the existing idx_track_sources_track index. No composite index provides significant
-- additional benefit for these access patterns.

-- For albums by artist with date sorting:
-- Used by: Artist detail pages showing recent albums
-- Supports: WHERE artist_id = ? ORDER BY created_at DESC
-- Replaces separate scans on idx_albums_artist_id and idx_albums_created_at
CREATE INDEX IF NOT EXISTS idx_albums_artist_created
ON albums(artist_id, created_at DESC);

-- For tracks by album with track number ordering:
-- Used by: Album detail pages showing sorted track lists
-- Supports: WHERE album_id = ? ORDER BY track_number
-- The existing idx_tracks_album_id only covers album filtering, not sorting
CREATE INDEX IF NOT EXISTS idx_tracks_album_number
ON tracks(album_id, track_number);

-- For playlists by owner with favorite filtering:
-- Used by: Library page filtering/sorting playlists by favorite status
-- Supports: WHERE owner_id = ? ORDER BY is_favorite DESC
-- Allows efficient favorite-first sorting for user's playlists
CREATE INDEX IF NOT EXISTS idx_playlists_owner_favorite
ON playlists(owner_id, is_favorite DESC);

-- For tracks by artist with date sorting:
-- Used by: Artist detail pages showing recent tracks
-- Supports: WHERE artist_id = ? ORDER BY created_at DESC
-- Allows efficient chronological listing of artist's tracks
CREATE INDEX IF NOT EXISTS idx_tracks_artist_created
ON tracks(artist_id, created_at DESC);

-- Note: Playlist tracks already has idx_playlist_tracks_playlist (playlist_id, position)
-- from migration 20250105000004, which covers playlist track ordering.
