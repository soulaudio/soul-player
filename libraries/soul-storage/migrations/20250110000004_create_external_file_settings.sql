-- External file handling settings (per user per device)
-- Controls behavior when opening/dropping files not in the library

CREATE TABLE IF NOT EXISTS external_file_settings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT NOT NULL,
    device_id TEXT NOT NULL,
    -- What to do when opening files not in library: 'ask' | 'play' | 'import'
    default_action TEXT NOT NULL DEFAULT 'ask' CHECK (default_action IN ('ask', 'play', 'import')),
    -- Where to import files: 'managed' | 'watched'
    import_destination TEXT NOT NULL DEFAULT 'managed' CHECK (import_destination IN ('managed', 'watched')),
    -- If importing to watched folder, which source to use (NULL = managed library)
    import_to_source_id INTEGER,
    -- Show notification after importing files
    show_import_notification INTEGER NOT NULL DEFAULT 1,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    -- Note: user_id and device_id are string identifiers, not foreign keys
    -- This allows flexibility for desktop (single user) and server (multi-user) modes
    FOREIGN KEY (import_to_source_id) REFERENCES sources(id) ON DELETE SET NULL,
    UNIQUE(user_id, device_id)
);

CREATE INDEX IF NOT EXISTS idx_external_file_settings_user_device
    ON external_file_settings(user_id, device_id);
