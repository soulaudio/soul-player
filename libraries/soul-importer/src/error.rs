//! Error types for the importer

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ImportError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Storage error: {0}")]
    Storage(#[from] soul_core::SoulError),

    #[error("Storage layer error: {0}")]
    StorageLayer(#[from] soul_storage::StorageError),

    #[error("Metadata error: {0}")]
    Metadata(String),

    #[error("Unsupported file format: {0}")]
    UnsupportedFormat(String),

    #[error("Duplicate file: {0}")]
    Duplicate(String),

    #[error("Invalid file path: {0}")]
    InvalidPath(String),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Import cancelled")]
    Cancelled,

    #[error("Unknown error: {0}")]
    Unknown(String),
}
