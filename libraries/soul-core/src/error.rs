/// Core error types for Soul Player
use thiserror::Error;
use crate::types::{PlaylistId, TrackId, UserId};

/// Result type alias using `SoulError`
pub type Result<T> = std::result::Result<T, SoulError>;

/// Core error type for Soul Player
#[derive(Error, Debug)]
pub enum SoulError {
    /// Storage-related errors
    #[error("Storage error: {0}")]
    Storage(String),

    /// Audio decoding/playback errors
    #[error("Audio error: {0}")]
    Audio(String),

    /// Metadata parsing errors
    #[error("Metadata error: {0}")]
    Metadata(String),

    /// Entity not found
    #[error("{entity} not found: {id}")]
    NotFound { entity: String, id: String },

    /// Track not found
    #[error("Track not found: {0}")]
    TrackNotFound(TrackId),

    /// Artist not found
    #[error("Artist not found: {0}")]
    ArtistNotFound(i64),

    /// Album not found
    #[error("Album not found: {0}")]
    AlbumNotFound(i64),

    /// Playlist not found
    #[error("Playlist not found: {0}")]
    PlaylistNotFound(PlaylistId),

    /// Source not found
    #[error("Source not found: {0}")]
    SourceNotFound(i64),

    /// User not found
    #[error("User not found: {0}")]
    UserNotFound(UserId),

    /// Permission denied
    #[error("Permission denied")]
    PermissionDenied,

    /// Permission denied with context
    #[error("Permission denied: {0}")]
    PermissionDeniedWithContext(String),

    /// Duplicate entry
    #[error("Duplicate entry: {0}")]
    Duplicate(String),

    /// Invalid input
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Network error
    #[error("Network error: {0}")]
    Network(String),

    /// I/O errors
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// Serialization errors
    #[error(transparent)]
    Serialization(#[from] serde_json::Error),

    /// Database errors (for storage implementations)
    #[error("Database error: {0}")]
    Database(String),

    /// Other errors
    #[error("{0}")]
    Other(String),
}

impl SoulError {
    /// Create a storage error
    pub fn storage(msg: impl Into<String>) -> Self {
        Self::Storage(msg.into())
    }

    /// Create an audio error
    pub fn audio(msg: impl Into<String>) -> Self {
        Self::Audio(msg.into())
    }

    /// Create a metadata error
    pub fn metadata(msg: impl Into<String>) -> Self {
        Self::Metadata(msg.into())
    }

    /// Create a not found error
    pub fn not_found(entity: impl Into<String>, id: impl Into<String>) -> Self {
        Self::NotFound {
            entity: entity.into(),
            id: id.into(),
        }
    }

    /// Create a permission denied error
    pub fn permission_denied(msg: impl Into<String>) -> Self {
        Self::PermissionDeniedWithContext(msg.into())
    }

    /// Create an invalid input error
    pub fn invalid_input(msg: impl Into<String>) -> Self {
        Self::InvalidInput(msg.into())
    }
}

#[cfg(feature = "sqlx-support")]
impl From<sqlx::Error> for SoulError {
    fn from(err: sqlx::Error) -> Self {
        Self::Database(err.to_string())
    }
}
