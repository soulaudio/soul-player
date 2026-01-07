-- Add source tracking and file format to tracks table

-- Track where the file came from (origin)
ALTER TABLE tracks ADD COLUMN origin_source_id INTEGER REFERENCES sources(id) ON DELETE CASCADE DEFAULT 1;

-- Track the file format (mp3, flac, etc.)
ALTER TABLE tracks ADD COLUMN file_format TEXT;

-- Create index for origin source lookups
CREATE INDEX IF NOT EXISTS idx_tracks_origin_source ON tracks(origin_source_id);
