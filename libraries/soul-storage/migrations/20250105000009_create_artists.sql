-- Create artists table
CREATE TABLE IF NOT EXISTS artists (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    sort_name TEXT,
    musicbrainz_id TEXT UNIQUE,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Create indices for common queries
CREATE INDEX IF NOT EXISTS idx_artists_name ON artists(name);
CREATE INDEX IF NOT EXISTS idx_artists_sort_name ON artists(sort_name);
CREATE INDEX IF NOT EXISTS idx_artists_musicbrainz_id ON artists(musicbrainz_id);

-- Create unique index for case-insensitive name matching (fuzzy matching helper)
CREATE UNIQUE INDEX IF NOT EXISTS idx_artists_name_lower ON artists(LOWER(name));
