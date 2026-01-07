-- Add album_artist_id column to tracks table
ALTER TABLE tracks ADD COLUMN album_artist_id INTEGER REFERENCES artists(id) ON DELETE SET NULL;

-- Create index for the new foreign key
CREATE INDEX IF NOT EXISTS idx_tracks_album_artist_id ON tracks(album_artist_id);
