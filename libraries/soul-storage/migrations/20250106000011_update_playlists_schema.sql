-- Add missing columns to playlists table to match multisource Playlist struct
-- Migration generated during SQLx compile-time query migration

-- Add description column (nullable)
ALTER TABLE playlists ADD COLUMN description TEXT;

-- Add is_public column (default 0)
ALTER TABLE playlists ADD COLUMN is_public INTEGER NOT NULL DEFAULT 0;

-- Add is_favorite column (default 0)
ALTER TABLE playlists ADD COLUMN is_favorite INTEGER NOT NULL DEFAULT 0;

-- Add updated_at column (default to created_at value)
ALTER TABLE playlists ADD COLUMN updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'));
