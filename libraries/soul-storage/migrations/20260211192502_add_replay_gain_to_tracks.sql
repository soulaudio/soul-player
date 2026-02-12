-- Add ReplayGain columns to tracks table
-- ReplayGain is a standard for normalizing audio volume (simpler than LUFS)
-- Values are stored in dB (relative to -18 LUFS reference)

-- Track gain: ReplayGain adjustment for individual track
ALTER TABLE tracks ADD COLUMN replay_gain_track_db REAL;

-- Album gain: ReplayGain adjustment for full album (preserves relative volume)
ALTER TABLE tracks ADD COLUMN replay_gain_album_db REAL;

-- Track peak: Maximum sample value (0.0 to 1.0+) for clipping prevention
ALTER TABLE tracks ADD COLUMN replay_gain_track_peak REAL;

-- Create index for querying tracks with ReplayGain data
CREATE INDEX IF NOT EXISTS idx_tracks_replay_gain
ON tracks(replay_gain_track_db)
WHERE replay_gain_track_db IS NOT NULL;
