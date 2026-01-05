//! Artist types

use serde::{Deserialize, Serialize};

pub type ArtistId = i64;

/// An artist
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artist {
    pub id: ArtistId,
    pub name: String,
    pub sort_name: Option<String>,
    pub musicbrainz_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Data for creating a new artist
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateArtist {
    pub name: String,
    pub sort_name: Option<String>,
    pub musicbrainz_id: Option<String>,
}
