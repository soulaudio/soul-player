-- Backfill track_sources from tracks.file_path for tracks missing availability entries
-- This fixes tracks that were created before the multisource architecture was implemented

INSERT INTO track_sources (track_id, source_id, status, local_file_path)
SELECT
    t.id,
    t.origin_source_id,
    'local_file',
    t.file_path
FROM tracks t
WHERE
    t.file_path IS NOT NULL
    AND t.file_path != ''
    AND NOT EXISTS (
        SELECT 1 FROM track_sources ts
        WHERE ts.track_id = t.id
    );
