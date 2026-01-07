-- Fix tracks table to use INTEGER id instead of TEXT
-- This is a breaking change but necessary for proper functioning

-- Drop and recreate tracks table with INTEGER id
DROP TABLE IF EXISTS tracks;

CREATE TABLE tracks (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    title TEXT NOT NULL,
    artist_id INTEGER REFERENCES artists(id) ON DELETE SET NULL,
    album_id INTEGER REFERENCES albums(id) ON DELETE SET NULL,
    album_artist_id INTEGER REFERENCES artists(id) ON DELETE SET NULL,
    track_number INTEGER,
    disc_number INTEGER,
    year INTEGER,
    duration_seconds REAL,
    bitrate INTEGER,
    sample_rate INTEGER,
    channels INTEGER,
    file_format TEXT,
    file_hash TEXT,
    origin_source_id INTEGER NOT NULL DEFAULT 1,
    musicbrainz_recording_id TEXT,
    fingerprint TEXT,
    metadata_source TEXT DEFAULT 'file',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Recreate indices
CREATE INDEX IF NOT EXISTS idx_tracks_title ON tracks(title);
CREATE INDEX IF NOT EXISTS idx_tracks_artist_id ON tracks(artist_id);
CREATE INDEX IF NOT EXISTS idx_tracks_album_id ON tracks(album_id);
CREATE INDEX IF NOT EXISTS idx_tracks_album_artist_id ON tracks(album_artist_id);
CREATE INDEX IF NOT EXISTS idx_tracks_file_hash ON tracks(file_hash);
CREATE INDEX IF NOT EXISTS idx_tracks_duration_seconds ON tracks(duration_seconds);
