-- Create user_credentials table for secure password storage
-- Separated from users table for better security practices
CREATE TABLE IF NOT EXISTS user_credentials (
    user_id TEXT PRIMARY KEY NOT NULL,
    password_hash TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- Index for efficient user lookups during authentication
CREATE INDEX IF NOT EXISTS idx_user_credentials_user_id ON user_credentials(user_id);
