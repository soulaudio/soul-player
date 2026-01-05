-- Create users table
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

-- Create index on name for lookups
CREATE INDEX IF NOT EXISTS idx_users_name ON users(name);
