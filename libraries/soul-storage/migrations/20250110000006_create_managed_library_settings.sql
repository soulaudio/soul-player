-- Managed library settings - configuration for the organized library folder
-- One per user per device. Files imported here are copied/organized automatically.

CREATE TABLE IF NOT EXISTS managed_library_settings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT NOT NULL,
    device_id TEXT NOT NULL,
    -- Where the managed library lives (e.g., ~/Music/Soul Player)
    library_path TEXT NOT NULL,
    -- Path template for organizing files
    -- Available placeholders: {AlbumArtist}, {Artist}, {Album}, {Year}, {TrackNo}, {DiscNo}, {Title}, {Genre}, {Composer}
    path_template TEXT NOT NULL DEFAULT '{AlbumArtist}/{Year} - {Album}/{TrackNo} - {Title}',
    -- What to do when importing: 'copy' (preserve original) or 'move' (relocate)
    import_action TEXT NOT NULL DEFAULT 'copy' CHECK (import_action IN ('copy', 'move')),
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    -- Note: user_id and device_id are string identifiers, not foreign keys
    -- This allows flexibility for desktop (single user) and server (multi-user) modes
    UNIQUE(user_id, device_id)
);

CREATE INDEX IF NOT EXISTS idx_managed_library_settings_user_device
    ON managed_library_settings(user_id, device_id);
