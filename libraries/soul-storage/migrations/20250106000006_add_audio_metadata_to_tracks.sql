-- Add audio metadata columns to tracks table (file_format already exists from 20250105000015)
ALTER TABLE tracks ADD COLUMN bitrate INTEGER;
ALTER TABLE tracks ADD COLUMN sample_rate INTEGER;
ALTER TABLE tracks ADD COLUMN channels INTEGER;
ALTER TABLE tracks ADD COLUMN musicbrainz_recording_id TEXT;
ALTER TABLE tracks ADD COLUMN fingerprint TEXT;

-- Create indices
CREATE INDEX IF NOT EXISTS idx_tracks_musicbrainz_recording_id ON tracks(musicbrainz_recording_id);
CREATE INDEX IF NOT EXISTS idx_tracks_fingerprint ON tracks(fingerprint);
