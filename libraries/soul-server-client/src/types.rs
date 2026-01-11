//! Types for Soul Player Server API requests and responses.

use serde::{Deserialize, Serialize};

/// Configuration for connecting to a Soul Player server.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Base URL of the server (e.g., "https://music.example.com")
    pub url: String,
    /// Current access token (if authenticated)
    pub access_token: Option<String>,
    /// Refresh token for obtaining new access tokens
    pub refresh_token: Option<String>,
}

impl ServerConfig {
    /// Create a new server config with just the URL.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            access_token: None,
            refresh_token: None,
        }
    }

    /// Create a config with existing tokens.
    pub fn with_tokens(
        url: impl Into<String>,
        access_token: impl Into<String>,
        refresh_token: Option<String>,
    ) -> Self {
        Self {
            url: url.into(),
            access_token: Some(access_token.into()),
            refresh_token,
        }
    }
}

// =============================================================================
// Authentication Types
// =============================================================================

/// Request body for login endpoint.
#[derive(Debug, Serialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// Response from successful login.
#[derive(Debug, Deserialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    /// Token validity in seconds
    pub expires_in: u64,
    pub user_id: String,
    pub username: String,
}

/// Request body for token refresh.
#[derive(Debug, Serialize)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
}

/// Response from token refresh.
#[derive(Debug, Deserialize)]
pub struct RefreshTokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
}

/// Current user info.
#[derive(Debug, Deserialize)]
pub struct UserInfo {
    pub id: String,
    pub username: String,
    pub email: Option<String>,
    pub created_at: String,
}

// =============================================================================
// Server Info Types
// =============================================================================

/// Information about the Soul Player server.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
    pub features: Vec<String>,
    /// Whether the server requires authentication
    pub requires_auth: bool,
    /// Maximum upload size in bytes (if limited)
    pub max_upload_size: Option<u64>,
}

// =============================================================================
// Library Types
// =============================================================================

/// A track as returned by the server.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerTrack {
    pub id: String,
    pub title: String,
    pub artist: Option<String>,
    pub artist_id: Option<String>,
    pub album: Option<String>,
    pub album_id: Option<String>,
    pub album_artist: Option<String>,
    pub track_number: Option<i32>,
    pub disc_number: Option<i32>,
    pub year: Option<i32>,
    pub duration_seconds: Option<f64>,
    pub file_format: String,
    pub bitrate: Option<i32>,
    pub sample_rate: Option<i32>,
    pub channels: Option<i32>,
    pub file_size: i64,
    pub content_hash: String,
    pub server_path: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Album as returned by the server.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerAlbum {
    pub id: String,
    pub title: String,
    pub artist: Option<String>,
    pub artist_id: Option<String>,
    pub year: Option<i32>,
    pub track_count: i32,
    pub cover_art_url: Option<String>,
}

/// Artist as returned by the server.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerArtist {
    pub id: String,
    pub name: String,
    pub album_count: i32,
    pub track_count: i32,
}

/// Delta sync response - changes since last sync.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SyncDelta {
    /// New tracks added since last sync
    pub new_tracks: Vec<ServerTrack>,
    /// Tracks that were updated
    pub updated_tracks: Vec<ServerTrack>,
    /// IDs of tracks that were deleted
    pub deleted_track_ids: Vec<String>,
    /// Server's current timestamp for next delta sync
    pub server_timestamp: i64,
    /// Sync token for next request
    pub sync_token: String,
}

/// Full library response (for initial sync).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LibraryResponse {
    pub tracks: Vec<ServerTrack>,
    pub albums: Vec<ServerAlbum>,
    pub artists: Vec<ServerArtist>,
    pub server_timestamp: i64,
    pub sync_token: String,
}

// =============================================================================
// Upload Types
// =============================================================================

/// Metadata to send with track upload.
#[derive(Debug, Clone, Serialize)]
pub struct UploadMetadata {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub track_number: Option<i32>,
    pub disc_number: Option<i32>,
    pub year: Option<i32>,
    pub content_hash: String,
}

/// Response from successful upload.
#[derive(Debug, Clone, Deserialize)]
pub struct UploadResponse {
    pub track: ServerTrack,
    /// True if track already existed (deduplicated)
    pub already_existed: bool,
}

/// Progress information during upload.
#[derive(Debug, Clone)]
pub struct UploadProgress {
    pub track_index: usize,
    pub total_tracks: usize,
    pub current_file: String,
    pub bytes_sent: u64,
    pub bytes_total: u64,
}

// =============================================================================
// Download Types
// =============================================================================

/// Progress information during download.
#[derive(Debug, Clone)]
pub struct DownloadProgress {
    pub track_id: String,
    pub bytes_received: u64,
    pub bytes_total: Option<u64>,
    /// Progress as 0.0 to 1.0
    pub progress: f32,
}

/// Stream URL response.
#[derive(Debug, Clone, Deserialize)]
pub struct StreamUrlResponse {
    pub url: String,
    /// URL validity in seconds
    pub expires_in: u64,
}

// =============================================================================
// Error Types
// =============================================================================

/// API error response from server.
#[derive(Debug, Clone, Deserialize)]
pub struct ApiError {
    pub error: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
}
