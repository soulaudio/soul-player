/// Playback state types for multi-device sync
use serde::{Deserialize, Serialize};

/// Repeat mode for playback
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RepeatMode {
    #[default]
    Off,
    All,
    One,
}

impl RepeatMode {
    /// Convert to string representation
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::All => "all",
            Self::One => "one",
        }
    }

    /// Parse from string
    #[must_use]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "off" => Some(Self::Off),
            "all" => Some(Self::All),
            "one" => Some(Self::One),
            _ => None,
        }
    }
}

impl std::fmt::Display for RepeatMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// User's playback state - shared across all devices
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaybackState {
    /// User ID this state belongs to
    pub user_id: String,

    /// Currently active device ID (the one playing audio)
    pub active_device_id: Option<String>,

    /// Whether playback is currently playing
    pub is_playing: bool,

    /// Current track ID (if any)
    pub current_track_id: Option<String>,

    /// Current position in milliseconds
    pub position_ms: i64,

    /// Volume level (0-100)
    pub volume: i32,

    /// Whether shuffle is enabled
    pub shuffle_enabled: bool,

    /// Repeat mode
    pub repeat_mode: RepeatMode,

    /// Queue as JSON array of track IDs
    pub queue_json: Option<String>,

    /// Last update timestamp (Unix epoch seconds)
    pub updated_at: i64,
}

impl Default for PlaybackState {
    fn default() -> Self {
        Self {
            user_id: String::new(),
            active_device_id: None,
            is_playing: false,
            current_track_id: None,
            position_ms: 0,
            volume: 80,
            shuffle_enabled: false,
            repeat_mode: RepeatMode::Off,
            queue_json: None,
            updated_at: 0,
        }
    }
}

/// Request to update playback state
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdatePlaybackState {
    /// Set playing state
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_playing: Option<bool>,

    /// Set current track
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_track_id: Option<String>,

    /// Set position in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position_ms: Option<i64>,

    /// Set volume (0-100)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume: Option<i32>,

    /// Set shuffle mode
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shuffle_enabled: Option<bool>,

    /// Set repeat mode
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repeat_mode: Option<RepeatMode>,

    /// Set queue (JSON array of track IDs)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub queue_json: Option<String>,
}

/// Transfer playback request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferPlayback {
    /// Target device ID to transfer playback to
    pub device_id: String,

    /// Whether to start playing immediately on the new device
    #[serde(default = "default_play")]
    pub play: bool,
}

fn default_play() -> bool {
    true
}
