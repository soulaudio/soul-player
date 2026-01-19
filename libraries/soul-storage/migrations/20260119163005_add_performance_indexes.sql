-- Add performance indexes for foreign keys to improve query performance
--
-- This migration adds indexes for commonly-queried foreign key columns that were
-- missing indexes. Most foreign keys already have indexes from previous migrations,
-- but these were identified as missing and important for query performance.

-- Add index on playlist_tracks.track_id for reverse playlist lookups
-- (finding which playlists contain a specific track)
CREATE INDEX IF NOT EXISTS idx_playlist_tracks_track_id ON playlist_tracks(track_id);

-- Add index on playlist_shares.playlist_id for finding all users a playlist is shared with
CREATE INDEX IF NOT EXISTS idx_playlist_shares_playlist_id ON playlist_shares(playlist_id);
