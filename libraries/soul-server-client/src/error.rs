//! Error types for the Soul Server client.

use thiserror::Error;

/// Errors that can occur when interacting with a Soul Player server.
#[derive(Error, Debug)]
pub enum ServerClientError {
    /// HTTP request failed
    #[error("HTTP request failed: {0}")]
    Request(#[from] reqwest::Error),

    /// Server returned an error response
    #[error("Server error ({status}): {message}")]
    ServerError { status: u16, message: String },

    /// Authentication required but no token available
    #[error("Authentication required")]
    AuthRequired,

    /// Authentication failed (invalid credentials or expired token)
    #[error("Authentication failed: {0}")]
    AuthFailed(String),

    /// Token refresh failed
    #[error("Token refresh failed: {0}")]
    TokenRefreshFailed(String),

    /// Invalid server URL
    #[error("Invalid server URL: {0}")]
    InvalidUrl(String),

    /// Failed to parse server response
    #[error("Failed to parse response: {0}")]
    ParseError(String),

    /// File not found for upload
    #[error("File not found: {0}")]
    FileNotFound(String),

    /// IO error during upload/download
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Upload was cancelled
    #[error("Upload cancelled")]
    UploadCancelled,

    /// Download was cancelled
    #[error("Download cancelled")]
    DownloadCancelled,

    /// Server is offline or unreachable
    #[error("Server unreachable: {0}")]
    ServerUnreachable(String),

    /// Server version incompatible
    #[error("Server version {server_version} is incompatible (requires {required_version}+)")]
    IncompatibleVersion {
        server_version: String,
        required_version: String,
    },

    /// Rate limited by server
    #[error("Rate limited, retry after {retry_after_secs} seconds")]
    RateLimited { retry_after_secs: u64 },
}

/// Result type for server client operations.
pub type Result<T> = std::result::Result<T, ServerClientError>;
