-- User playback state - shared across all devices for a user
-- Enables multi-device sync where all devices see the same playback state
CREATE TABLE IF NOT EXISTS user_playback_state (
    user_id TEXT PRIMARY KEY NOT NULL,
    active_device_id TEXT,
    is_playing INTEGER NOT NULL DEFAULT 0,
    current_track_id TEXT,
    position_ms INTEGER NOT NULL DEFAULT 0,
    volume INTEGER NOT NULL DEFAULT 80 CHECK (volume >= 0 AND volume <= 100),
    shuffle_enabled INTEGER NOT NULL DEFAULT 0,
    repeat_mode TEXT NOT NULL DEFAULT 'off' CHECK (repeat_mode IN ('off', 'all', 'one')),
    queue_json TEXT,
    updated_at INTEGER NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (active_device_id) REFERENCES devices(id) ON DELETE SET NULL
);
