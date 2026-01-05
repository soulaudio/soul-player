//! Error types for playback management

use thiserror::Error;

/// Playback errors
#[derive(Debug, Error)]
pub enum PlaybackError {
    /// No track is currently loaded
    #[error("No track loaded")]
    NoTrackLoaded,

    /// Queue is empty
    #[error("Queue is empty")]
    QueueEmpty,

    /// Invalid seek position
    #[error("Invalid seek position: {0:?}")]
    InvalidSeekPosition(std::time::Duration),

    /// Audio source error
    #[error("Audio source error: {0}")]
    AudioSource(String),

    /// Index out of bounds
    #[error("Index out of bounds: {0}")]
    IndexOutOfBounds(usize),

    /// Invalid operation
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result type for playback operations
pub type Result<T> = std::result::Result<T, PlaybackError>;
