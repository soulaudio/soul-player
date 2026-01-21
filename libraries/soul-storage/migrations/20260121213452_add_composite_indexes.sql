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
