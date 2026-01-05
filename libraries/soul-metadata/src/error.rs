/// Metadata-specific errors
use thiserror::Error;

/// Result type alias using `MetadataError`
pub type Result<T> = std::result::Result<T, MetadataError>;

/// Metadata error types
#[derive(Error, Debug)]
pub enum MetadataError {
    /// File not found
    #[error("File not found: {0}")]
    FileNotFound(String),

    /// Unsupported format
    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),

    /// Tag parsing error
    #[error("Tag parsing error: {0}")]
    ParseError(String),

    /// Tag writing error
    #[error("Tag writing error: {0}")]
    WriteError(String),

    /// I/O error
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// Lofty error
    #[error(transparent)]
    Lofty(#[from] lofty::error::LoftyError),
}

impl From<MetadataError> for soul_core::SoulError {
    fn from(err: MetadataError) -> Self {
        soul_core::SoulError::metadata(err.to_string())
    }
}
