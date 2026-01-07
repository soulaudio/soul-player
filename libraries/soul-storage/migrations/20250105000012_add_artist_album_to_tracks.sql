-- Add foreign key columns to tracks table
ALTER TABLE tracks ADD COLUMN artist_id INTEGER REFERENCES artists(id) ON DELETE SET NULL;
ALTER TABLE tracks ADD COLUMN album_id INTEGER REFERENCES albums(id) ON DELETE SET NULL;

-- Create indices for the new foreign keys
CREATE INDEX IF NOT EXISTS idx_tracks_artist_id ON tracks(artist_id);
CREATE INDEX IF NOT EXISTS idx_tracks_album_id ON tracks(album_id);
