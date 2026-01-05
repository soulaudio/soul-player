//! Core types for playback management

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// Track information for queue management
///
/// Contains all metadata needed for playback and display.
/// This is eagerly loaded from storage to avoid I/O during playback.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueueTrack {
    /// Unique track identifier from storage
    pub id: String,

    /// File path for audio decoding
    pub path: PathBuf,

    /// Track title
    pub title: String,

    /// Artist name
    pub artist: String,

    /// Album name (optional)
    pub album: Option<String>,

    /// Track duration
    pub duration: Duration,

    /// Track number in album (optional)
    pub track_number: Option<u32>,

    /// Source context for shuffle scope
    pub source: TrackSource,
}

/// Source context for a track
///
/// Used to determine shuffle scope (e.g., shuffle within album only)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TrackSource {
    /// Track from a playlist
    Playlist { id: String, name: String },

    /// Track from an album
    Album { id: String, name: String },

    /// Track from artist discography
    Artist { id: String, name: String },

    /// Individual track (no context)
    Single,
}

/// Playback state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlaybackState {
    /// No track loaded
    Stopped,

    /// Currently playing
    Playing,

    /// Paused mid-track
    Paused,

    /// Loading/buffering next track
    Loading,
}

/// Repeat mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RepeatMode {
    /// Stop when queue ends
    Off,

    /// Loop entire queue
    All,

    /// Loop current track only
    One,
}

/// Shuffle mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShuffleMode {
    /// No shuffling
    Off,

    /// Pure random shuffle
    Random,

    /// Smart shuffle (avoid recently played, distribute artists)
    Smart,
}

/// Configuration for playback manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybackConfig {
    /// Maximum history size (default: 50)
    pub history_size: usize,

    /// Initial volume (0-100, default: 80)
    pub volume: u8,

    /// Initial shuffle mode (default: Off)
    pub shuffle: ShuffleMode,

    /// Initial repeat mode (default: Off)
    pub repeat: RepeatMode,

    /// Gapless playback enabled (default: true)
    pub gapless: bool,
}

impl Default for PlaybackConfig {
    fn default() -> Self {
        Self {
            history_size: 50,
            volume: 80,
            shuffle: ShuffleMode::Off,
            repeat: RepeatMode::Off,
            gapless: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = PlaybackConfig::default();
        assert_eq!(config.history_size, 50);
        assert_eq!(config.volume, 80);
        assert_eq!(config.shuffle, ShuffleMode::Off);
        assert_eq!(config.repeat, RepeatMode::Off);
        assert!(config.gapless);
    }

    #[test]
    fn queue_track_creation() {
        let track = QueueTrack {
            id: "track1".to_string(),
            path: PathBuf::from("/music/song.mp3"),
            title: "Test Song".to_string(),
            artist: "Test Artist".to_string(),
            album: Some("Test Album".to_string()),
            duration: Duration::from_secs(180),
            track_number: Some(1),
            source: TrackSource::Album {
                id: "album1".to_string(),
                name: "Test Album".to_string(),
            },
        };

        assert_eq!(track.id, "track1");
        assert_eq!(track.title, "Test Song");
    }
}
