//! Library source types for watched folder management
//!
//! Library sources represent folders that Soul Player monitors for audio files.
//! This is distinct from "sources" (local vs server) used for multi-source sync.

use serde::{Deserialize, Serialize};

/// A library source (watched folder)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LibrarySource {
    /// Unique ID
    pub id: i64,

    /// Owner user ID
    pub user_id: String,

    /// Device these settings apply to
    pub device_id: String,

    /// Display name (e.g., "FLAC Collection", "Vinyl Rips")
    pub name: String,

    /// Path to the watched folder
    pub path: String,

    /// Whether this source is enabled for scanning
    pub enabled: bool,

    /// Whether to soft-delete tracks when files disappear
    pub sync_deletes: bool,

    /// Last scan timestamp (Unix epoch seconds)
    pub last_scan_at: Option<i64>,

    /// Current scan status
    pub scan_status: ScanStatus,

    /// Error message if scan_status is Error
    pub error_message: Option<String>,

    /// Created timestamp (Unix epoch seconds)
    pub created_at: i64,

    /// Last updated timestamp (Unix epoch seconds)
    pub updated_at: i64,
}

/// Scan status for a library source
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ScanStatus {
    /// No scan in progress
    #[default]
    Idle,
    /// Currently scanning
    Scanning,
    /// Last scan failed
    Error,
}

impl ScanStatus {
    /// Convert to string for database storage
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Scanning => "scanning",
            Self::Error => "error",
        }
    }

    /// Parse from string
    #[must_use]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "idle" => Some(Self::Idle),
            "scanning" => Some(Self::Scanning),
            "error" => Some(Self::Error),
            _ => None,
        }
    }
}

impl std::fmt::Display for ScanStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Request to create a new library source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateLibrarySource {
    /// Display name
    pub name: String,

    /// Path to the watched folder
    pub path: String,

    /// Whether to soft-delete tracks when files disappear (default: true)
    #[serde(default = "default_true")]
    pub sync_deletes: bool,
}

fn default_true() -> bool {
    true
}

/// Request to update a library source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateLibrarySource {
    /// Display name
    pub name: Option<String>,

    /// Whether this source is enabled
    pub enabled: Option<bool>,

    /// Whether to soft-delete tracks when files disappear
    pub sync_deletes: Option<bool>,
}

/// Progress of an ongoing scan
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanProgress {
    /// Unique ID
    pub id: i64,

    /// Library source being scanned
    pub library_source_id: i64,

    /// When the scan started (Unix epoch seconds)
    pub started_at: i64,

    /// When the scan completed (Unix epoch seconds)
    pub completed_at: Option<i64>,

    /// Total files to process (if known)
    pub total_files: Option<i64>,

    /// Files processed so far
    pub processed_files: i64,

    /// New files added
    pub new_files: i64,

    /// Files with updated metadata
    pub updated_files: i64,

    /// Files marked unavailable
    pub removed_files: i64,

    /// Files that caused errors
    pub errors: i64,

    /// Current status
    pub status: ScanProgressStatus,

    /// Error message if status is Failed
    pub error_message: Option<String>,
}

/// Status of a scan operation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ScanProgressStatus {
    /// Scan is in progress
    #[default]
    Running,
    /// Scan completed successfully
    Completed,
    /// Scan failed with an error
    Failed,
    /// Scan was cancelled by user
    Cancelled,
}

impl ScanProgressStatus {
    /// Convert to string for database storage
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    /// Parse from string
    #[must_use]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "running" => Some(Self::Running),
            "completed" => Some(Self::Completed),
            "failed" => Some(Self::Failed),
            "cancelled" => Some(Self::Cancelled),
            _ => None,
        }
    }
}

impl std::fmt::Display for ScanProgressStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
