-- Create DSP presets table
-- Stores user-created presets for effect chains

CREATE TABLE IF NOT EXISTS dsp_presets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    is_builtin BOOLEAN NOT NULL DEFAULT 0,
    effect_chain TEXT NOT NULL, -- JSON array of effects
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    UNIQUE(user_id, name)
);

-- Create index for user lookups
CREATE INDEX IF NOT EXISTS idx_dsp_presets_user ON dsp_presets(user_id);

-- Note: Built-in presets will be seeded by the application on first run
-- This allows them to be created per-user when users are actually created
