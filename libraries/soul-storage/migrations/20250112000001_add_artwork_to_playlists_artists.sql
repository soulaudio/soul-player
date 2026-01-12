-- Add cover_art_path to playlists table
ALTER TABLE playlists ADD COLUMN cover_art_path TEXT;

-- Add cover_art_path to artists table
ALTER TABLE artists ADD COLUMN cover_art_path TEXT;
