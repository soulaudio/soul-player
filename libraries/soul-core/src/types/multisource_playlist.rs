//! Multi-source playlist types

use super::ids::{PlaylistId, TrackId, UserId};
use serde::{Deserialize, Serialize};

/// Playlist with multi-user support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Playlist {
    pub id: PlaylistId,
    pub name: String,
    pub description: Option<String>,
    pub owner_id: UserId,
    pub is_public: bool,
    pub is_favorite: bool,
    pub created_at: String,
    pub updated_at: String,

    /// Tracks in playlist (optional, populated when requested)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tracks: Option<Vec<PlaylistTrack>>,
}

/// Data for creating a new playlist
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePlaylist {
    pub name: String,
    pub description: Option<String>,
    pub owner_id: UserId,
    pub is_favorite: bool,
}

/// Track in a playlist with denormalized data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistTrack {
    pub track_id: TrackId,
    pub position: i32,
    pub added_at: String,

    /// Denormalized fields for display
    pub title: Option<String>,
    pub artist_name: Option<String>,
    pub duration_seconds: Option<f64>,
}
