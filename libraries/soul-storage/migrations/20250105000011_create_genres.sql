-- Create genres table
CREATE TABLE IF NOT EXISTS genres (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    canonical_name TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Create track_genres junction table (many-to-many)
CREATE TABLE IF NOT EXISTS track_genres (
    track_id TEXT NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    genre_id INTEGER NOT NULL REFERENCES genres(id) ON DELETE CASCADE,
    PRIMARY KEY (track_id, genre_id)
);

-- Create indices
CREATE INDEX IF NOT EXISTS idx_genres_name ON genres(name);
CREATE INDEX IF NOT EXISTS idx_genres_canonical_name ON genres(canonical_name);
CREATE INDEX IF NOT EXISTS idx_track_genres_track_id ON track_genres(track_id);
CREATE INDEX IF NOT EXISTS idx_track_genres_genre_id ON track_genres(genre_id);

-- Create unique index for case-insensitive name matching
CREATE UNIQUE INDEX IF NOT EXISTS idx_genres_name_lower ON genres(LOWER(name));
