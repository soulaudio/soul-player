-- Quick hash (SHA256 of first 64KB) for fast new-file deduplication.
-- Avoids reading entire file content for relocation detection.

ALTER TABLE tracks ADD COLUMN quick_hash TEXT;
CREATE INDEX idx_tracks_quick_hash ON tracks(quick_hash);
