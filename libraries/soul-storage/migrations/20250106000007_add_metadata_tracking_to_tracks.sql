-- Add metadata tracking columns to tracks table
ALTER TABLE tracks ADD COLUMN metadata_source TEXT DEFAULT 'Manual';
ALTER TABLE tracks ADD COLUMN created_at TEXT NOT NULL DEFAULT (datetime('now'));
ALTER TABLE tracks ADD COLUMN updated_at TEXT NOT NULL DEFAULT (datetime('now'));

-- Create index for metadata queries
CREATE INDEX IF NOT EXISTS idx_tracks_metadata_source ON tracks(metadata_source);
CREATE INDEX IF NOT EXISTS idx_tracks_updated_at ON tracks(updated_at);
