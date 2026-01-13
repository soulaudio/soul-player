-- Seed 500 dummy tracks for e2e testing
-- Run with: sqlite3 path/to/dev.db < scripts/seed-test-tracks.sql

-- Insert test artist
INSERT OR IGNORE INTO artists (id, name) VALUES (9999, 'Test Artist');

-- Insert test album
INSERT OR IGNORE INTO albums (id, title, artist_id, year)
VALUES (9999, 'Test Album', 9999, 2024);

-- Insert test source (local)
INSERT OR IGNORE INTO sources (id, name, source_type)
VALUES (1, 'Local Library', 'local');

-- Insert 500 test tracks
WITH RECURSIVE cnt(x) AS (
  SELECT 1
  UNION ALL
  SELECT x+1 FROM cnt
  LIMIT 500
)
INSERT OR IGNORE INTO tracks (
  id,
  title,
  artist_id,
  album_id,
  track_number,
  disc_number,
  duration_seconds,
  file_format,
  bit_rate,
  sample_rate,
  channels
)
SELECT
  10000 + x,
  'Test Track ' || x,
  9999,
  9999,
  x,
  1,
  180.0,
  'mp3',
  320,
  44100,
  2
FROM cnt;

-- Insert track availability (mark all as available from local source)
WITH RECURSIVE cnt(x) AS (
  SELECT 1
  UNION ALL
  SELECT x+1 FROM cnt
  LIMIT 500
)
INSERT OR IGNORE INTO track_availability (
  track_id,
  source_id,
  status,
  local_file_path
)
SELECT
  10000 + x,
  1,
  'available',
  'test/track_' || x || '.mp3'
FROM cnt;

-- Verify insertion
SELECT COUNT(*) as total_test_tracks FROM tracks WHERE id >= 10000 AND id < 10500;
