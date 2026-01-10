-- Add library source tracking columns to tracks table
-- Enables tracking where files came from and their availability

-- Reference to the library source (watched folder) this track belongs to
ALTER TABLE tracks ADD COLUMN library_source_id INTEGER REFERENCES library_sources(id) ON DELETE SET NULL;

-- File metadata for change detection
ALTER TABLE tracks ADD COLUMN file_size INTEGER;
ALTER TABLE tracks ADD COLUMN file_mtime INTEGER;

-- Content hash for deduplication and relocation detection
-- Note: file_hash already exists but may have different semantics
-- content_hash is specifically SHA256 of full file bytes
ALTER TABLE tracks ADD COLUMN content_hash TEXT;

-- Codec details as JSON (bitrate, bits_per_sample, encoder, etc.)
ALTER TABLE tracks ADD COLUMN codec_details TEXT;

-- Availability tracking for soft delete when files are missing
ALTER TABLE tracks ADD COLUMN is_available INTEGER NOT NULL DEFAULT 1;
ALTER TABLE tracks ADD COLUMN unavailable_since INTEGER;

-- Indexes for efficient lookups
CREATE INDEX IF NOT EXISTS idx_tracks_library_source ON tracks(library_source_id);
CREATE INDEX IF NOT EXISTS idx_tracks_content_hash ON tracks(content_hash) WHERE content_hash IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_tracks_is_available ON tracks(is_available);
CREATE INDEX IF NOT EXISTS idx_tracks_file_size_mtime ON tracks(file_size, file_mtime);
