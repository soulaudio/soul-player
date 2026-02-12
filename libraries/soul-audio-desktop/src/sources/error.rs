//! Audio source error types with fast-fail validation
//!
//! This module provides structured error types for audio source operations,
//! enabling fast error detection (<1ms) and user-friendly error messages.

use std::path::PathBuf;
use thiserror::Error;

/// Audio source errors with detailed context
///
/// These errors are designed for fast-fail validation - they can be detected
/// in <1ms by pre-validating files before attempting to decode them.
#[derive(Debug, Error, Clone)]
pub enum AudioSourceError {
    /// File does not exist at the given path
    #[error("File not found: {path}")]
    FileNotFound {
        /// Path that was not found
        path: PathBuf,
    },

    /// Permission denied when trying to access file
    #[error("Permission denied: {path}")]
    PermissionDenied {
        /// Path that was inaccessible
        path: PathBuf,
    },

    /// File exists but has an unsupported format
    #[error("Unsupported format: {path}")]
    UnsupportedFormat {
        /// Path to the unsupported file
        path: PathBuf,
        /// Details about why the format is unsupported
        details: String,
    },

    /// File is corrupted or malformed
    #[error("Corrupted file: {path}")]
    CorruptedFile {
        /// Path to the corrupted file
        path: PathBuf,
        /// Details about the corruption
        details: String,
    },

    /// File read error (other I/O errors)
    #[error("Failed to read file: {path}")]
    FileReadError {
        /// Path that failed to read
        path: PathBuf,
        /// Reason for the failure
        reason: String,
    },

    /// Decoder failed to initialize or decode
    #[error("Decoder failed: {reason}")]
    DecoderFailed {
        /// Reason for decoder failure
        reason: String,
    },

    /// Resampler failed to initialize
    #[error("Resampler initialization failed: {reason}")]
    ResamplerFailed {
        /// Reason for resampler failure
        reason: String,
    },

    /// No audio tracks found in the file
    #[error("No audio tracks found in file: {path}")]
    NoAudioTracks {
        /// Path to the file with no tracks
        path: PathBuf,
    },

    /// Probe failed to identify file format
    #[error("Failed to probe file format: {path}")]
    ProbeFailed {
        /// Path that failed to probe
        path: PathBuf,
        /// Reason for probe failure
        reason: String,
    },
}

impl AudioSourceError {
    /// Get a user-friendly error message
    ///
    /// Returns a localized, user-friendly message suitable for display in the UI.
    /// This should be translated in the frontend.
    pub fn user_message(&self) -> String {
        match self {
            Self::FileNotFound { path } => {
                format!("The file could not be found: {}", path.display())
            }
            Self::PermissionDenied { path } => {
                format!(
                    "Permission denied when trying to access: {}",
                    path.display()
                )
            }
            Self::UnsupportedFormat { path, details } => {
                format!("Unsupported audio format: {} ({})", path.display(), details)
            }
            Self::CorruptedFile { path, details } => {
                format!(
                    "The file is corrupted or malformed: {} ({})",
                    path.display(),
                    details
                )
            }
            Self::FileReadError { path, reason } => {
                format!("Failed to read file: {} ({})", path.display(), reason)
            }
            Self::DecoderFailed { reason } => {
                format!("Audio decoder failed: {}", reason)
            }
            Self::ResamplerFailed { reason } => {
                format!("Audio resampler failed: {}", reason)
            }
            Self::NoAudioTracks { path } => {
                format!("No audio tracks found in: {}", path.display())
            }
            Self::ProbeFailed { path, reason } => {
                format!(
                    "Failed to identify file format: {} ({})",
                    path.display(),
                    reason
                )
            }
        }
    }

    /// Check if this error is recoverable
    ///
    /// Recoverable errors might succeed if retried (e.g., temporary file locks).
    /// Unrecoverable errors will never succeed (e.g., file not found, corrupted file).
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            Self::FileReadError { .. } | Self::PermissionDenied { .. }
        )
    }

    /// Get the severity level of this error
    ///
    /// Used for logging and UI display priority.
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            Self::FileNotFound { .. } => ErrorSeverity::Warning,
            Self::PermissionDenied { .. } => ErrorSeverity::Warning,
            Self::UnsupportedFormat { .. } => ErrorSeverity::Warning,
            Self::CorruptedFile { .. } => ErrorSeverity::Error,
            Self::FileReadError { .. } => ErrorSeverity::Error,
            Self::DecoderFailed { .. } => ErrorSeverity::Error,
            Self::ResamplerFailed { .. } => ErrorSeverity::Error,
            Self::NoAudioTracks { .. } => ErrorSeverity::Warning,
            Self::ProbeFailed { .. } => ErrorSeverity::Error,
        }
    }
}

/// Error severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    /// Warning - user should be notified but playback can continue
    Warning,
    /// Error - playback cannot continue for this track
    Error,
}

/// Convert std::io::Error to AudioSourceError
///
/// This provides fast-fail validation by categorizing I/O errors
/// into specific audio source error types.
pub fn io_error_to_audio_source_error(
    path: &std::path::Path,
    error: std::io::Error,
) -> AudioSourceError {
    use std::io::ErrorKind;

    match error.kind() {
        ErrorKind::NotFound => AudioSourceError::FileNotFound {
            path: path.to_path_buf(),
        },
        ErrorKind::PermissionDenied => AudioSourceError::PermissionDenied {
            path: path.to_path_buf(),
        },
        _ => AudioSourceError::FileReadError {
            path: path.to_path_buf(),
            reason: error.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_message_generation() {
        let err = AudioSourceError::FileNotFound {
            path: PathBuf::from("/path/to/music.mp3"),
        };
        let msg = err.user_message();
        assert!(msg.contains("could not be found"));
        assert!(msg.contains("music.mp3"));
    }

    #[test]
    fn test_recoverable_errors() {
        let recoverable = AudioSourceError::PermissionDenied {
            path: PathBuf::from("/test.mp3"),
        };
        assert!(recoverable.is_recoverable());

        let unrecoverable = AudioSourceError::FileNotFound {
            path: PathBuf::from("/test.mp3"),
        };
        assert!(!unrecoverable.is_recoverable());
    }

    #[test]
    fn test_severity_levels() {
        let warning_err = AudioSourceError::FileNotFound {
            path: PathBuf::from("/test.mp3"),
        };
        assert_eq!(warning_err.severity(), ErrorSeverity::Warning);

        let error_err = AudioSourceError::DecoderFailed {
            reason: "test".to_string(),
        };
        assert_eq!(error_err.severity(), ErrorSeverity::Error);
    }

    #[test]
    fn test_io_error_conversion() {
        use std::io::{Error, ErrorKind};

        let path = std::path::Path::new("/test.mp3");

        let not_found = Error::new(ErrorKind::NotFound, "not found");
        let err = io_error_to_audio_source_error(path, not_found);
        assert!(matches!(err, AudioSourceError::FileNotFound { .. }));

        let permission = Error::new(ErrorKind::PermissionDenied, "denied");
        let err = io_error_to_audio_source_error(path, permission);
        assert!(matches!(err, AudioSourceError::PermissionDenied { .. }));

        let other = Error::new(ErrorKind::Other, "other error");
        let err = io_error_to_audio_source_error(path, other);
        assert!(matches!(err, AudioSourceError::FileReadError { .. }));
    }
}
