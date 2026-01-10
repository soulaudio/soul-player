//! Managed library settings types
//!
//! Controls how files are organized when imported to the managed library folder.

use serde::{Deserialize, Serialize};

/// Settings for the managed library folder
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManagedLibrarySettings {
    /// Unique ID
    pub id: i64,

    /// Owner user ID
    pub user_id: String,

    /// Device these settings apply to
    pub device_id: String,

    /// Path to the managed library folder
    pub library_path: String,

    /// Path template for organizing files
    /// Available placeholders: {AlbumArtist}, {Artist}, {Album}, {Year}, {TrackNo}, {DiscNo}, {Title}, {Genre}, {Composer}
    pub path_template: String,

    /// What to do when importing files
    pub import_action: ImportAction,

    /// Created timestamp (Unix epoch seconds)
    pub created_at: i64,

    /// Last updated timestamp (Unix epoch seconds)
    pub updated_at: i64,
}

impl Default for ManagedLibrarySettings {
    fn default() -> Self {
        Self {
            id: 0,
            user_id: String::new(),
            device_id: String::new(),
            library_path: String::new(),
            path_template: "{AlbumArtist}/{Year} - {Album}/{TrackNo} - {Title}".to_string(),
            import_action: ImportAction::Copy,
            created_at: 0,
            updated_at: 0,
        }
    }
}

/// What to do when importing files to managed library
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ImportAction {
    /// Copy files (preserve originals)
    #[default]
    Copy,
    /// Move files (relocate to managed library)
    Move,
}

impl ImportAction {
    /// Convert to string for database storage
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Copy => "copy",
            Self::Move => "move",
        }
    }

    /// Parse from string
    #[must_use]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "copy" => Some(Self::Copy),
            "move" => Some(Self::Move),
            _ => None,
        }
    }
}

impl std::fmt::Display for ImportAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Request to create or update managed library settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateManagedLibrarySettings {
    /// Path to the managed library folder
    pub library_path: String,

    /// Path template for organizing files
    #[serde(default = "default_template")]
    pub path_template: String,

    /// What to do when importing files
    #[serde(default)]
    pub import_action: ImportAction,
}

fn default_template() -> String {
    "{AlbumArtist}/{Year} - {Album}/{TrackNo} - {Title}".to_string()
}

impl Default for UpdateManagedLibrarySettings {
    fn default() -> Self {
        Self {
            library_path: String::new(),
            path_template: default_template(),
            import_action: ImportAction::Copy,
        }
    }
}

/// Available path template presets
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PathTemplatePreset {
    /// Audiophile: {AlbumArtist}/{Year} - {Album}/{TrackNo} - {Title}
    Audiophile,
    /// Simple: {AlbumArtist}/{Album}/{TrackNo} - {Title}
    Simple,
    /// Genre-first: {Genre}/{AlbumArtist}/{Album}/{TrackNo} - {Title}
    GenreFirst,
}

impl PathTemplatePreset {
    /// Get the template string for this preset
    #[must_use]
    pub fn template(&self) -> &'static str {
        match self {
            Self::Audiophile => "{AlbumArtist}/{Year} - {Album}/{TrackNo} - {Title}",
            Self::Simple => "{AlbumArtist}/{Album}/{TrackNo} - {Title}",
            Self::GenreFirst => "{Genre}/{AlbumArtist}/{Album}/{TrackNo} - {Title}",
        }
    }

    /// Get human-readable name
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Audiophile => "Audiophile",
            Self::Simple => "Simple",
            Self::GenreFirst => "Genre-first",
        }
    }
}
