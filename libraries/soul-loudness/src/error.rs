//! Error types for loudness analysis

use thiserror::Error;

/// Result type for loudness operations
pub type Result<T> = std::result::Result<T, LoudnessError>;

/// Errors that can occur during loudness analysis
#[derive(Error, Debug)]
pub enum LoudnessError {
    /// Invalid sample rate
    #[error("Invalid sample rate: {0} Hz (must be between 8000 and 384000)")]
    InvalidSampleRate(u32),

    /// Invalid channel count
    #[error("Invalid channel count: {0} (must be 1-8)")]
    InvalidChannelCount(u32),

    /// EBU R128 analysis error
    #[error("EBU R128 analysis failed: {0}")]
    AnalysisError(String),

    /// No samples were provided for analysis
    #[error("No audio samples provided for analysis")]
    NoSamples,

    /// Audio is completely silent
    #[error("Audio is silent (no loudness data available)")]
    SilentAudio,

    /// Tag reading error
    #[error("Failed to read audio tags: {0}")]
    TagReadError(String),

    /// Tag writing error
    #[error("Failed to write audio tags: {0}")]
    TagWriteError(String),

    /// File not found
    #[error("File not found: {0}")]
    FileNotFound(String),

    /// Unsupported file format
    #[error("Unsupported file format: {0}")]
    UnsupportedFormat(String),

    /// Resampling error
    #[error("Resampling failed: {0}")]
    ResampleError(String),

    /// Generic IO error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

impl From<ebur128::Error> for LoudnessError {
    fn from(err: ebur128::Error) -> Self {
        Self::AnalysisError(format!("{:?}", err))
    }
}

impl From<lofty::error::LoftyError> for LoudnessError {
    fn from(err: lofty::error::LoftyError) -> Self {
        Self::TagReadError(err.to_string())
    }
}
