-- Fix all tables that reference tracks to use INTEGER track_id

-- Fix playlist_tracks
DROP TABLE IF EXISTS playlist_tracks;

CREATE TABLE playlist_tracks (
    playlist_id TEXT NOT NULL,
    track_id INTEGER NOT NULL,
    position INTEGER NOT NULL,
    added_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (playlist_id, track_id),
    FOREIGN KEY (playlist_id) REFERENCES playlists(id) ON DELETE CASCADE,
    FOREIGN KEY (track_id) REFERENCES tracks(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_playlist_tracks_playlist ON playlist_tracks(playlist_id, position);

-- Fix track_variants
DROP TABLE IF EXISTS track_variants;

CREATE TABLE track_variants (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    track_id INTEGER NOT NULL,
    quality TEXT NOT NULL,
    format TEXT NOT NULL,
    file_path TEXT NOT NULL,
    file_size INTEGER NOT NULL,
    bitrate INTEGER,
    sample_rate INTEGER,
    channels INTEGER,
    transcoded_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (track_id) REFERENCES tracks(id) ON DELETE CASCADE,
    UNIQUE(track_id, quality, format)
);

CREATE INDEX IF NOT EXISTS idx_track_variants_track_id ON track_variants(track_id);
CREATE INDEX IF NOT EXISTS idx_track_variants_quality ON track_variants(quality);
CREATE INDEX IF NOT EXISTS idx_track_variants_format ON track_variants(format);

-- Fix track_sources
DROP TABLE IF EXISTS track_sources;

CREATE TABLE track_sources (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    track_id INTEGER NOT NULL,
    source_id INTEGER NOT NULL,
    status TEXT NOT NULL,
    local_file_path TEXT,
    local_file_size INTEGER,
    downloaded_at TEXT,
    server_path TEXT,
    server_file_id TEXT,
    last_verified_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (track_id) REFERENCES tracks(id) ON DELETE CASCADE,
    FOREIGN KEY (source_id) REFERENCES sources(id) ON DELETE CASCADE,
    UNIQUE(track_id, source_id)
);

CREATE INDEX IF NOT EXISTS idx_track_sources_track ON track_sources(track_id);
CREATE INDEX IF NOT EXISTS idx_track_sources_source ON track_sources(source_id);
CREATE INDEX IF NOT EXISTS idx_track_sources_status ON track_sources(status);

-- Fix track_stats
DROP TABLE IF EXISTS track_stats;

CREATE TABLE track_stats (
    track_id INTEGER PRIMARY KEY,
    play_count INTEGER NOT NULL DEFAULT 0,
    skip_count INTEGER NOT NULL DEFAULT 0,
    last_played_at TEXT,
    rating INTEGER CHECK (rating IS NULL OR (rating >= 1 AND rating <= 5)),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (track_id) REFERENCES tracks(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_track_stats_play_count ON track_stats(play_count DESC);
CREATE INDEX IF NOT EXISTS idx_track_stats_last_played ON track_stats(last_played_at DESC);
CREATE INDEX IF NOT EXISTS idx_track_stats_rating ON track_stats(rating DESC);
