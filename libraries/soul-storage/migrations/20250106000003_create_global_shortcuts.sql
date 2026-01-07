-- Create global shortcuts table for keyboard shortcuts configuration
-- Multi-user aware: each user can customize shortcuts
CREATE TABLE IF NOT EXISTS global_shortcuts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT NOT NULL,
    action TEXT NOT NULL,
    accelerator TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    is_default INTEGER NOT NULL DEFAULT 0,
    updated_at INTEGER NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    UNIQUE(user_id, action)
);

-- Index for efficient user lookups
CREATE INDEX IF NOT EXISTS idx_shortcuts_user ON global_shortcuts(user_id);

-- Index for enabled shortcuts (used during registration)
CREATE INDEX IF NOT EXISTS idx_shortcuts_enabled ON global_shortcuts(user_id, enabled);
