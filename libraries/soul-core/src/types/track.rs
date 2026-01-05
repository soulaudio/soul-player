/// Track domain type
use crate::types::OldTrackId as TrackId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// Audio track
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Track {
    /// Unique track identifier
    pub id: TrackId,

    /// Track title
    pub title: String,

    /// Artist name
    pub artist: Option<String>,

    /// Album name
    pub album: Option<String>,

    /// Album artist
    pub album_artist: Option<String>,

    /// Track number
    pub track_number: Option<u32>,

    /// Disc number
    pub disc_number: Option<u32>,

    /// Release year
    pub year: Option<u32>,

    /// Genre
    pub genre: Option<String>,

    /// Track duration in milliseconds
    pub duration_ms: Option<u64>,

    /// File path on disk
    pub file_path: PathBuf,

    /// File hash (for deduplication)
    pub file_hash: Option<String>,

    /// When the track was added to the library
    pub added_at: DateTime<Utc>,
}

impl Track {
    /// Create a new track with minimal metadata
    pub fn new(title: impl Into<String>, file_path: PathBuf) -> Self {
        Self {
            id: TrackId::generate(),
            title: title.into(),
            artist: None,
            album: None,
            album_artist: None,
            track_number: None,
            disc_number: None,
            year: None,
            genre: None,
            duration_ms: None,
            file_path,
            file_hash: None,
            added_at: Utc::now(),
        }
    }

    /// Get the track duration as a Duration
    pub fn duration(&self) -> Option<Duration> {
        self.duration_ms.map(Duration::from_millis)
    }

    /// Set the track duration from a Duration
    pub fn set_duration(&mut self, duration: Duration) {
        self.duration_ms = Some(duration.as_millis() as u64);
    }
}

/// Track metadata extracted from file tags
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TrackMetadata {
    /// Track title
    pub title: Option<String>,

    /// Artist name
    pub artist: Option<String>,

    /// Album name
    pub album: Option<String>,

    /// Album artist
    pub album_artist: Option<String>,

    /// Track number
    pub track_number: Option<u32>,

    /// Disc number
    pub disc_number: Option<u32>,

    /// Release year
    pub year: Option<u32>,

    /// Genre
    pub genre: Option<String>,

    /// Duration in milliseconds
    pub duration_ms: Option<u64>,
}

impl TrackMetadata {
    /// Create empty metadata
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if metadata has any useful information
    pub fn is_empty(&self) -> bool {
        self.title.is_none()
            && self.artist.is_none()
            && self.album.is_none()
            && self.album_artist.is_none()
            && self.track_number.is_none()
            && self.disc_number.is_none()
            && self.year.is_none()
            && self.genre.is_none()
            && self.duration_ms.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn track_creation() {
        let track = Track::new("Test Song", PathBuf::from("/music/song.mp3"));
        assert_eq!(track.title, "Test Song");
        assert_eq!(track.file_path, PathBuf::from("/music/song.mp3"));
        assert!(track.artist.is_none());
    }

    #[test]
    fn track_duration_conversion() {
        let mut track = Track::new("Song", PathBuf::from("/song.mp3"));
        track.set_duration(Duration::from_secs(180));

        assert_eq!(track.duration_ms, Some(180_000));
        assert_eq!(track.duration(), Some(Duration::from_secs(180)));
    }

    #[test]
    fn metadata_is_empty() {
        let empty = TrackMetadata::new();
        assert!(empty.is_empty());

        let mut filled = TrackMetadata::new();
        filled.title = Some("Title".to_string());
        assert!(!filled.is_empty());
    }
}
