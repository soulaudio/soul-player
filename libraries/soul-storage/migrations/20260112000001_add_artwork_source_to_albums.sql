-- Add artwork source tracking to albums table
-- Tracks where artwork comes from for priority system

-- Artwork source type
-- Values: NULL (not set), 'soul_storage' (custom via UI), 'folder' (cover.jpg), 'embedded' (from track)
ALTER TABLE albums ADD COLUMN artwork_source TEXT CHECK (artwork_source IN ('soul_storage', 'folder', 'embedded'));

-- Create index for artwork source lookups
CREATE INDEX IF NOT EXISTS idx_albums_artwork_source ON albums(artwork_source) WHERE artwork_source IS NOT NULL;
