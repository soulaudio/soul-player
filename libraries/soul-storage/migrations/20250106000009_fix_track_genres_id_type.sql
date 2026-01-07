-- Fix track_genres table to use INTEGER track_id instead of TEXT

DROP TABLE IF EXISTS track_genres;

CREATE TABLE track_genres (
    track_id INTEGER NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    genre_id INTEGER NOT NULL REFERENCES genres(id) ON DELETE CASCADE,
    PRIMARY KEY (track_id, genre_id)
);

-- Recreate indices
CREATE INDEX IF NOT EXISTS idx_track_genres_track_id ON track_genres(track_id);
CREATE INDEX IF NOT EXISTS idx_track_genres_genre_id ON track_genres(genre_id);
