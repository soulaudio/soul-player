/// Playlist domain types
use crate::types::{OldPlaylistId as PlaylistId, OldTrackId as TrackId, OldUserId as UserId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Playlist
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Playlist {
    /// Unique playlist identifier
    pub id: PlaylistId,

    /// Owner user ID
    pub owner_id: UserId,

    /// Playlist name
    pub name: String,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

impl Playlist {
    /// Create a new playlist
    pub fn new(owner_id: UserId, name: impl Into<String>) -> Self {
        Self {
            id: PlaylistId::generate(),
            owner_id,
            name: name.into(),
            created_at: Utc::now(),
        }
    }

    /// Create a playlist with a specific ID (for database loading)
    pub fn with_id(
        id: PlaylistId,
        owner_id: UserId,
        name: impl Into<String>,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            owner_id,
            name: name.into(),
            created_at,
        }
    }
}

/// Playlist track association
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaylistTrack {
    /// Playlist ID
    pub playlist_id: PlaylistId,

    /// Track ID
    pub track_id: TrackId,

    /// Position in the playlist (0-indexed)
    pub position: u32,

    /// When the track was added to the playlist
    pub added_at: DateTime<Utc>,
}

impl PlaylistTrack {
    /// Create a new playlist track association
    pub fn new(playlist_id: PlaylistId, track_id: TrackId, position: u32) -> Self {
        Self {
            playlist_id,
            track_id,
            position,
            added_at: Utc::now(),
        }
    }
}

/// Permission level for shared playlists
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Permission {
    /// Can only view/listen
    Read,
    /// Can add/remove tracks
    Write,
}

impl Permission {
    /// Convert permission to string for database storage
    pub fn as_str(&self) -> &'static str {
        match self {
            Permission::Read => "read",
            Permission::Write => "write",
        }
    }

    /// Parse permission from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "read" => Some(Permission::Read),
            "write" => Some(Permission::Write),
            _ => None,
        }
    }
}

/// Playlist sharing information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaylistShare {
    /// Playlist ID
    pub playlist_id: PlaylistId,

    /// User ID this playlist is shared with
    pub shared_with_user_id: UserId,

    /// Permission level
    pub permission: Permission,

    /// When the playlist was shared
    pub shared_at: DateTime<Utc>,
}

impl PlaylistShare {
    /// Create a new playlist share
    pub fn new(
        playlist_id: PlaylistId,
        shared_with_user_id: UserId,
        permission: Permission,
    ) -> Self {
        Self {
            playlist_id,
            shared_with_user_id,
            permission,
            shared_at: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn playlist_creation() {
        let user_id = UserId::new("user-1");
        let playlist = Playlist::new(user_id.clone(), "My Favorites");

        assert_eq!(playlist.owner_id, user_id);
        assert_eq!(playlist.name, "My Favorites");
        assert!(playlist.created_at <= Utc::now());
    }

    #[test]
    fn permission_string_conversion() {
        assert_eq!(Permission::Read.as_str(), "read");
        assert_eq!(Permission::Write.as_str(), "write");

        assert_eq!(Permission::from_str("read"), Some(Permission::Read));
        assert_eq!(Permission::from_str("write"), Some(Permission::Write));
        assert_eq!(Permission::from_str("invalid"), None);
    }

    #[test]
    fn playlist_track_ordering() {
        let playlist_id = PlaylistId::new("playlist-1");
        let track_id = TrackId::new("track-1");

        let pt = PlaylistTrack::new(playlist_id, track_id, 5);
        assert_eq!(pt.position, 5);
    }
}
