-- Create window state table for persisting window position and size
-- One entry per user
CREATE TABLE IF NOT EXISTS window_state (
    user_id TEXT PRIMARY KEY NOT NULL,
    x INTEGER,
    y INTEGER,
    width INTEGER NOT NULL,
    height INTEGER NOT NULL,
    maximized INTEGER NOT NULL DEFAULT 0,
    last_route TEXT,
    updated_at INTEGER NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);
