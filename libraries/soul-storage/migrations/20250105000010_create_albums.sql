-- Create albums table
CREATE TABLE IF NOT EXISTS albums (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,
    artist_id INTEGER REFERENCES artists(id) ON DELETE SET NULL,
    year INTEGER,
    cover_art_path TEXT,
    musicbrainz_id TEXT UNIQUE,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Create indices for common queries
CREATE INDEX IF NOT EXISTS idx_albums_title ON albums(title);
CREATE INDEX IF NOT EXISTS idx_albums_artist_id ON albums(artist_id);
CREATE INDEX IF NOT EXISTS idx_albums_year ON albums(year);
CREATE INDEX IF NOT EXISTS idx_albums_musicbrainz_id ON albums(musicbrainz_id);

-- Composite index for album matching by title and artist
CREATE INDEX IF NOT EXISTS idx_albums_title_artist ON albums(title, artist_id);
