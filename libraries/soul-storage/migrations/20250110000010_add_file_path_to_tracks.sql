-- Add file_path column back to tracks for library scanning
-- This tracks the local file path for tracks imported via library sources (watched folders)
-- Complements the library_source_id, file_size, file_mtime, content_hash columns

ALTER TABLE tracks ADD COLUMN file_path TEXT;

CREATE INDEX IF NOT EXISTS idx_tracks_file_path ON tracks(file_path) WHERE file_path IS NOT NULL;
