use thiserror::Error;

/// Errors that can occur during sync operations
#[derive(Error, Debug)]
pub enum SyncError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Sync already in progress")]
    AlreadySyncing,

    #[error("Sync not running")]
    NotSyncing,

    #[error("Importer error: {0}")]
    Importer(#[from] soul_importer::ImportError),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Invalid sync state: {0}")]
    InvalidState(String),

    #[error("Sync was cancelled")]
    Cancelled,

    #[error("Unknown error: {0}")]
    Unknown(String),
}

pub type Result<T> = std::result::Result<T, SyncError>;
