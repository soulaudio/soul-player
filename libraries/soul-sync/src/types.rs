use serde::{Deserialize, Serialize};

/// Sync status representing the current state of the sync operation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SyncStatus {
    Idle,
    Scanning,
    Extracting,
    Validating,
    Cleaning,
    Error,
}

/// Sync phase representing the current phase of work
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncPhase {
    Scanning,
    MetadataExtraction,
    Validation,
    Cleanup,
}

/// What triggered the sync operation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncTrigger {
    Manual,              // User clicked sync button
    SchemaMigration,     // New migrations detected
    SourceActivation,    // Source became active
}

/// Progress information for an ongoing sync operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncProgress {
    pub status: SyncStatus,
    pub phase: Option<SyncPhase>,
    pub total_items: usize,
    pub processed_items: usize,
    pub successful_items: usize,
    pub failed_items: usize,
    pub current_item: Option<String>,
    pub percentage: f32,
}

/// Summary of a completed sync operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncSummary {
    pub session_id: String,
    pub started_at: String,
    pub completed_at: String,
    pub duration_seconds: u64,
    pub files_scanned: usize,
    pub tracks_updated: usize,
    pub errors_encountered: usize,
    pub orphans_cleaned: usize,
}

/// Error record from sync_errors table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncErrorRecord {
    pub id: i64,
    pub sync_session_id: String,
    pub occurred_at: String,
    pub phase: String,
    pub item_path: Option<String>,
    pub error_type: String,
    pub error_message: String,
    pub resolved: bool,
    pub resolved_at: Option<String>,
    pub resolution_notes: Option<String>,
}
