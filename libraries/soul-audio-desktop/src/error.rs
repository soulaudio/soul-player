/// Audio output errors
use thiserror::Error;

/// Result type for audio operations
pub type Result<T> = std::result::Result<T, AudioError>;

/// Audio errors
#[derive(Debug, Error)]
pub enum AudioError {
    /// Device not found
    #[error("Audio device not found")]
    DeviceNotFound,

    /// Device error
    #[error("Device error: {0}")]
    DeviceError(String),

    /// Failed to build output stream
    #[error("Failed to build output stream: {0}")]
    StreamBuildError(String),

    /// Failed to play stream
    #[error("Failed to play stream: {0}")]
    PlayError(String),

    /// Failed to pause stream
    #[error("Failed to pause stream: {0}")]
    PauseError(String),

    /// Invalid volume level
    #[error("Invalid volume: {0}. Must be between 0.0 and 1.0")]
    InvalidVolume(f32),

    /// Sample rate conversion error
    #[error("Sample rate conversion error: {0}")]
    ResampleError(String),

    /// No audio buffer available
    #[error("No audio buffer available")]
    NoBuffer,

    /// Unsupported audio format
    #[error("Unsupported audio format: {0}")]
    UnsupportedFormat(String),

    /// Playback error
    #[error("Playback error: {0}")]
    PlaybackError(String),

    /// CPAL error
    #[error("CPAL error: {0}")]
    CpalError(String),
}

// Backwards compatibility alias
pub type AudioOutputError = AudioError;

impl From<cpal::BuildStreamError> for AudioError {
    fn from(err: cpal::BuildStreamError) -> Self {
        AudioError::StreamBuildError(err.to_string())
    }
}

impl From<cpal::PlayStreamError> for AudioError {
    fn from(err: cpal::PlayStreamError) -> Self {
        AudioError::PlayError(err.to_string())
    }
}

impl From<cpal::PauseStreamError> for AudioError {
    fn from(err: cpal::PauseStreamError) -> Self {
        AudioError::PauseError(err.to_string())
    }
}

impl From<cpal::DefaultStreamConfigError> for AudioError {
    fn from(err: cpal::DefaultStreamConfigError) -> Self {
        AudioError::CpalError(err.to_string())
    }
}

impl From<soul_playback::PlaybackError> for AudioError {
    fn from(err: soul_playback::PlaybackError) -> Self {
        AudioError::PlaybackError(err.to_string())
    }
}

impl From<AudioOutputError> for soul_core::SoulError {
    fn from(err: AudioOutputError) -> Self {
        soul_core::SoulError::audio(err.to_string())
    }
}
