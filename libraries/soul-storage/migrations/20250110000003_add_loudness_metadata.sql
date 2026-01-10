-- Add loudness metadata columns to tracks table for ReplayGain and EBU R128 analysis
-- These values are populated by background analysis and read during playback

-- ReplayGain values (linear scale, not dB)
ALTER TABLE tracks ADD COLUMN replaygain_track_gain REAL;  -- Track gain in dB
ALTER TABLE tracks ADD COLUMN replaygain_track_peak REAL;  -- Track peak (linear, 0.0-1.0+)
ALTER TABLE tracks ADD COLUMN replaygain_album_gain REAL;  -- Album gain in dB
ALTER TABLE tracks ADD COLUMN replaygain_album_peak REAL;  -- Album peak (linear, 0.0-1.0+)

-- EBU R128 loudness values
ALTER TABLE tracks ADD COLUMN lufs_integrated REAL;        -- Integrated loudness in LUFS
ALTER TABLE tracks ADD COLUMN lufs_range REAL;             -- Loudness range in LU
ALTER TABLE tracks ADD COLUMN true_peak_dbfs REAL;         -- True peak in dBFS

-- Analysis metadata
ALTER TABLE tracks ADD COLUMN loudness_analyzed_at INTEGER; -- Timestamp of analysis (Unix epoch)
ALTER TABLE tracks ADD COLUMN loudness_version TEXT;        -- Version of analysis algorithm

-- Create album loudness table for album-level analysis
CREATE TABLE IF NOT EXISTS album_loudness (
    album_id INTEGER PRIMARY KEY NOT NULL REFERENCES albums(id) ON DELETE CASCADE,
    replaygain_gain REAL,              -- Album-level ReplayGain in dB
    replaygain_peak REAL,              -- Album-level peak (linear)
    lufs_integrated REAL,              -- Album integrated loudness in LUFS
    lufs_range REAL,                   -- Album loudness range in LU
    true_peak_dbfs REAL,               -- Album max true peak in dBFS
    track_count INTEGER NOT NULL,      -- Number of tracks analyzed
    analyzed_at INTEGER NOT NULL,      -- Timestamp of analysis
    version TEXT NOT NULL              -- Version of analysis algorithm
);

-- Create analysis queue table for background processing
CREATE TABLE IF NOT EXISTS loudness_analysis_queue (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    track_id INTEGER NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    priority INTEGER DEFAULT 0,        -- Higher = more urgent (e.g., currently playing = 100)
    status TEXT DEFAULT 'pending',     -- pending, processing, completed, failed
    error_message TEXT,                -- Error message if failed
    created_at INTEGER NOT NULL,       -- When added to queue
    started_at INTEGER,                -- When processing started
    completed_at INTEGER,              -- When processing completed
    UNIQUE(track_id)
);

-- Index for queue processing
CREATE INDEX IF NOT EXISTS idx_loudness_queue_status_priority
    ON loudness_analysis_queue(status, priority DESC, created_at ASC);

-- Index for finding tracks without loudness data
CREATE INDEX IF NOT EXISTS idx_tracks_loudness_analyzed
    ON tracks(loudness_analyzed_at) WHERE loudness_analyzed_at IS NULL;
