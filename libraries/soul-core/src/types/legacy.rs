//! Legacy types for old Database implementation
//! These are kept for backward compatibility but not actively used

use serde::{Deserialize, Serialize};

/// Permission level (legacy)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Permission {
    Read,
    Write,
}

/// Playlist share (legacy)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistShare {
    pub playlist_id: String,
    pub shared_with_user_id: String,
    pub permission: Permission,
}

/// Track metadata (legacy)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TrackMetadata {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration_ms: Option<u64>,
}
