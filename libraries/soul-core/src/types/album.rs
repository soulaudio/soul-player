//! Album types

use super::ArtistId;
use serde::{Deserialize, Serialize};

pub type AlbumId = i64;

/// An album
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Album {
    pub id: AlbumId,
    pub title: String,
    pub artist_id: Option<ArtistId>,
    pub artist_name: Option<String>, // Denormalized
    pub year: Option<i32>,
    pub cover_art_path: Option<String>,
    /// Artwork source: 'soul_storage', 'folder', or 'embedded'
    pub artwork_source: Option<String>,
    pub musicbrainz_id: Option<String>,
    /// Filesystem folder containing this album's tracks (direct parent directory).
    /// Used to enforce strict folder isolation: same title + artist in different
    /// folders → distinct albums, no cross-folder merging.
    pub folder_path: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Data for creating a new album
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAlbum {
    pub title: String,
    pub artist_id: Option<ArtistId>,
    pub year: Option<i32>,
    pub musicbrainz_id: Option<String>,
    /// Filesystem folder containing this album's tracks.
    pub folder_path: String,
}
