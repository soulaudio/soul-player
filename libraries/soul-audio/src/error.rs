/// Audio-specific errors
use thiserror::Error;

/// Result type alias using `AudioError`
pub type Result<T> = std::result::Result<T, AudioError>;

/// Audio error types
#[derive(Error, Debug)]
pub enum AudioError {
    /// File not found
    #[error("File not found: {0}")]
    FileNotFound(String),

    /// Unsupported format
    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),

    /// Decoding error
    #[error("Decode error: {0}")]
    DecodeError(String),

    /// Playback error
    #[error("Playback error: {0}")]
    PlaybackError(String),

    /// Invalid audio buffer
    #[error("Invalid audio buffer: {0}")]
    InvalidBuffer(String),

    /// Seek error
    #[error("Seek error: {0}")]
    SeekError(String),

    /// No file is currently open
    #[error("No file open for streaming decode")]
    NoFileOpen,

    /// I/O error
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// Symphonia error
    #[error("Symphonia error: {0}")]
    Symphonia(String),

    /// Fingerprinting error
    #[error("Fingerprint error: {0}")]
    Fingerprint(String),
}

impl From<AudioError> for soul_core::SoulError {
    fn from(err: AudioError) -> Self {
        soul_core::SoulError::audio(err.to_string())
    }
}
