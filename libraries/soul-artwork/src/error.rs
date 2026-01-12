use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur during artwork operations
#[derive(Debug, Error)]
pub enum ArtworkError {
    /// File not found
    #[error("File not found: {0}")]
    FileNotFound(PathBuf),

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Lofty error
    #[error("Metadata error: {0}")]
    Lofty(#[from] lofty::error::LoftyError),

    /// No artwork found in file
    #[error("No artwork found in file")]
    NoArtwork,

    /// Artwork too large
    #[error("Artwork too large: {0} bytes (max {1} bytes)")]
    TooLarge(usize, usize),

    /// No tag found in file (for writing)
    #[error("No tag found in file to write artwork")]
    NoTag,

    /// Failed to save artwork to file
    #[error("Failed to save artwork: {0}")]
    SaveFailed(String),
}

/// Result type for artwork operations
pub type Result<T> = std::result::Result<T, ArtworkError>;
