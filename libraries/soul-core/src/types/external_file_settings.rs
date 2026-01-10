//! External file handling settings
//!
//! Controls behavior when opening or dropping audio files that are not part of the library.

use serde::{Deserialize, Serialize};

/// Action to take when opening files not in library
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ExternalFileAction {
    /// Always ask user what to do (default)
    #[default]
    Ask,
    /// Play without importing to library
    Play,
    /// Import to library
    Import,
}

impl ExternalFileAction {
    /// Convert to string representation for database storage
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ask => "ask",
            Self::Play => "play",
            Self::Import => "import",
        }
    }

    /// Parse from string
    #[must_use]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "ask" => Some(Self::Ask),
            "play" => Some(Self::Play),
            "import" => Some(Self::Import),
            _ => None,
        }
    }
}

impl std::fmt::Display for ExternalFileAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Destination for imported files
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ImportDestination {
    /// Import to managed library folder (default)
    #[default]
    Managed,
    /// Add to a watched folder (index without copying)
    Watched,
}

impl ImportDestination {
    /// Convert to string representation for database storage
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Managed => "managed",
            Self::Watched => "watched",
        }
    }

    /// Parse from string
    #[must_use]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "managed" => Some(Self::Managed),
            "watched" => Some(Self::Watched),
            _ => None,
        }
    }
}

impl std::fmt::Display for ImportDestination {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Settings for handling external (non-library) files
///
/// These settings control what happens when a user:
/// - Drags and drops audio files onto the player
/// - Double-clicks an audio file to open with Soul Player
/// - Opens a file from the command line
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalFileSettings {
    /// Database ID
    pub id: i64,

    /// Owner user ID
    pub user_id: String,

    /// Device these settings apply to
    pub device_id: String,

    /// What to do when opening files not in library
    pub default_action: ExternalFileAction,

    /// Where to import files (if importing)
    pub import_destination: ImportDestination,

    /// If importing to watched folder, which source ID to use (None = managed library)
    pub import_to_source_id: Option<i64>,

    /// Whether to show notification after importing files
    pub show_import_notification: bool,

    /// Created timestamp (Unix epoch seconds)
    pub created_at: i64,

    /// Last updated timestamp (Unix epoch seconds)
    pub updated_at: i64,
}

impl Default for ExternalFileSettings {
    fn default() -> Self {
        Self {
            id: 0,
            user_id: String::new(),
            device_id: String::new(),
            default_action: ExternalFileAction::Ask,
            import_destination: ImportDestination::Managed,
            import_to_source_id: None,
            show_import_notification: true,
            created_at: 0,
            updated_at: 0,
        }
    }
}

/// Request to create or update external file settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateExternalFileSettings {
    /// What to do when opening files not in library
    pub default_action: ExternalFileAction,

    /// Where to import files (if importing)
    pub import_destination: ImportDestination,

    /// If importing to watched folder, which source ID to use
    pub import_to_source_id: Option<i64>,

    /// Whether to show notification after importing files
    pub show_import_notification: bool,
}

impl Default for UpdateExternalFileSettings {
    fn default() -> Self {
        Self {
            default_action: ExternalFileAction::Ask,
            import_destination: ImportDestination::Managed,
            import_to_source_id: None,
            show_import_notification: true,
        }
    }
}
