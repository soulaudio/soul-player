-- Create user settings table for storing user preferences
-- Multi-user aware: each user has their own settings
CREATE TABLE IF NOT EXISTS user_settings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT NOT NULL,
    key TEXT NOT NULL,
    value TEXT NOT NULL,  -- JSON-serialized value for flexibility
    updated_at INTEGER NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    UNIQUE(user_id, key)
);

-- Index for efficient user lookups
CREATE INDEX IF NOT EXISTS idx_user_settings_user ON user_settings(user_id);

-- Index for efficient key lookups
CREATE INDEX IF NOT EXISTS idx_user_settings_user_key ON user_settings(user_id, key);
