//! Multi-source track types
//!
//! Tracks with support for multiple sources and availability tracking

use super::ids::TrackId;
use super::{AlbumId, ArtistId, SourceId};
use serde::{Deserialize, Serialize};

/// Track with multi-source support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Track {
    pub id: TrackId,
    pub title: String,
    pub artist_id: Option<ArtistId>,
    pub artist_name: Option<String>, // Denormalized
    pub album_id: Option<AlbumId>,
    pub album_title: Option<String>, // Denormalized
    pub album_artist_id: Option<ArtistId>,
    pub track_number: Option<i32>,
    pub disc_number: Option<i32>,
    pub year: Option<i32>,
    pub duration_seconds: Option<f64>,
    pub bitrate: Option<i32>,
    pub sample_rate: Option<i32>,
    pub channels: Option<i32>,
    pub file_format: String,
    pub origin_source_id: SourceId,
    pub musicbrainz_recording_id: Option<String>,
    pub fingerprint: Option<String>,
    pub metadata_source: MetadataSource,
    pub created_at: String,
    pub updated_at: String,

    /// Availability across sources (optional, populated when needed)
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub availability: Vec<TrackAvailability>,
}

/// Data for creating a new track
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTrack {
    pub title: String,
    pub artist_id: Option<ArtistId>,
    pub album_id: Option<AlbumId>,
    pub album_artist_id: Option<ArtistId>,
    pub track_number: Option<i32>,
    pub disc_number: Option<i32>,
    pub year: Option<i32>,
    pub duration_seconds: Option<f64>,
    pub bitrate: Option<i32>,
    pub sample_rate: Option<i32>,
    pub channels: Option<i32>,
    pub file_format: String,
    pub file_hash: Option<String>,
    pub origin_source_id: SourceId,
    pub local_file_path: Option<String>,
    pub musicbrainz_recording_id: Option<String>,
    pub fingerprint: Option<String>,
}

/// Data for updating a track (all fields optional)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateTrack {
    pub title: Option<String>,
    pub artist_id: Option<ArtistId>,
    pub album_id: Option<AlbumId>,
    pub album_artist_id: Option<ArtistId>,
    pub track_number: Option<i32>,
    pub disc_number: Option<i32>,
    pub year: Option<i32>,
    pub duration_seconds: Option<f64>,
    pub bitrate: Option<i32>,
    pub sample_rate: Option<i32>,
    pub channels: Option<i32>,
    pub musicbrainz_recording_id: Option<String>,
    pub fingerprint: Option<String>,
    pub metadata_source: Option<MetadataSource>,
}

/// Track availability on a specific source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackAvailability {
    pub source_id: SourceId,
    pub status: AvailabilityStatus,
    pub local_file_path: Option<String>,
    pub server_path: Option<String>,
    pub local_file_size: Option<i64>,
}

/// Availability status for a track on a source
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AvailabilityStatus {
    LocalFile,   // Original file on local filesystem
    Cached,      // Downloaded from server and cached locally
    StreamOnly,  // Available for streaming from server
    Unavailable, // Source is offline or file deleted
}

/// Source of track metadata
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetadataSource {
    File,       // From file tags
    Enriched,   // From MusicBrainz/external services
    UserEdited, // User manually edited
}
