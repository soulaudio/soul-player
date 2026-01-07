-- Add duration_seconds column to tracks table
-- The table has duration_ms but code expects duration_seconds
ALTER TABLE tracks ADD COLUMN duration_seconds REAL;

-- Create index for duration queries
CREATE INDEX IF NOT EXISTS idx_tracks_duration_seconds ON tracks(duration_seconds);
