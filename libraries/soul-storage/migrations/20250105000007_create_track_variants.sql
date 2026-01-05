-- Create track_variants table for tracking transcoded quality versions
-- Each track can have multiple quality variants (original, high, medium, low)
CREATE TABLE IF NOT EXISTS track_variants (
    id TEXT PRIMARY KEY NOT NULL,
    track_id TEXT NOT NULL,
    quality TEXT NOT NULL, -- 'original', 'high', 'medium', 'low'
    format TEXT NOT NULL,  -- 'mp3', 'flac', 'ogg', 'opus', 'wav'
    file_path TEXT NOT NULL,
    file_size INTEGER NOT NULL,
    bitrate INTEGER,       -- in kbps, null for lossless
    sample_rate INTEGER,   -- in Hz
    channels INTEGER,      -- 1 = mono, 2 = stereo, etc.
    transcoded_at INTEGER NOT NULL,
    FOREIGN KEY (track_id) REFERENCES tracks(id) ON DELETE CASCADE,
    UNIQUE(track_id, quality, format)
);

-- Indices for efficient lookups
CREATE INDEX IF NOT EXISTS idx_track_variants_track_id ON track_variants(track_id);
CREATE INDEX IF NOT EXISTS idx_track_variants_quality ON track_variants(quality);
CREATE INDEX IF NOT EXISTS idx_track_variants_format ON track_variants(format);
CREATE INDEX IF NOT EXISTS idx_track_variants_file_path ON track_variants(file_path);
