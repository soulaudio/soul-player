-- Drop legacy TEXT columns (no backwards compatibility needed)
-- artist, album, and genre are now normalized via foreign keys

-- Drop indices first
DROP INDEX IF EXISTS idx_tracks_artist;
DROP INDEX IF EXISTS idx_tracks_album;

-- Drop columns
ALTER TABLE tracks DROP COLUMN artist;
ALTER TABLE tracks DROP COLUMN album;
ALTER TABLE tracks DROP COLUMN album_artist;
ALTER TABLE tracks DROP COLUMN genre;
