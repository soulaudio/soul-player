//! Core types for playback management

use crate::crossfade::CrossfadeSettings;
use crate::source::AudioSource;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

// Re-export crossfade types for convenience
pub use crate::crossfade::{CrossfadeSettings as CrossfadeConfig, FadeCurve};

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
}

/// Source state machine - replaces triple-source pattern
///
/// This enum consolidates audio_source, next_source, and pending_source
/// into a single type-safe state machine. Makes illegal states unrepresentable
/// (e.g., can't have pending_source without audio_source).
#[allow(clippy::large_enum_variant)]
pub enum SourceState {
    /// No audio loaded
    Empty,

    /// Single track playing
    Playing {
        source: Box<dyn AudioSource>,
        track: QueueTrack,
    },

    /// Two sources ready for crossfade transition
    Transitioning {
        outgoing: Box<dyn AudioSource>,
        outgoing_track: QueueTrack,
        incoming: Box<dyn AudioSource>,
        incoming_track: QueueTrack,
        /// Crossfade progress (0.0 = start, 1.0 = complete)
        /// None = not in crossfade (gapless mode)
        crossfade_progress: Option<f32>,
    },
}

impl SourceState {
    /// Get immutable reference to current playing source
    pub fn current_source(&self) -> Option<&dyn AudioSource> {
        match self {
            Self::Empty => None,
            Self::Playing { source, .. } => Some(source.as_ref()),
            Self::Transitioning { outgoing, .. } => Some(outgoing.as_ref()),
        }
    }

    /// Get mutable reference to current playing source
    pub fn current_source_mut(&mut self) -> Option<&mut dyn AudioSource> {
        match self {
            Self::Empty => None,
            Self::Playing { source, .. } => Some(source.as_mut()),
            Self::Transitioning { outgoing, .. } => Some(outgoing.as_mut()),
        }
    }

    /// Get immutable reference to incoming source (if transitioning)
    pub fn incoming_source(&self) -> Option<&dyn AudioSource> {
        match self {
            Self::Transitioning { incoming, .. } => Some(incoming.as_ref()),
            _ => None,
        }
    }

    /// Get mutable reference to incoming source (if transitioning)
    pub fn incoming_source_mut(&mut self) -> Option<&mut dyn AudioSource> {
        match self {
            Self::Transitioning { incoming, .. } => Some(incoming.as_mut()),
            _ => None,
        }
    }

    /// Get current track metadata
    pub fn current_track(&self) -> Option<&QueueTrack> {
        match self {
            Self::Empty => None,
            Self::Playing { track, .. } => Some(track),
            Self::Transitioning { outgoing_track, .. } => Some(outgoing_track),
        }
    }

    /// Get incoming track metadata (if transitioning)
    pub fn incoming_track(&self) -> Option<&QueueTrack> {
        match self {
            Self::Transitioning { incoming_track, .. } => Some(incoming_track),
            _ => None,
        }
    }

    /// Check if source is ready for playback
    pub fn is_ready(&self) -> bool {
        match self {
            Self::Empty => false,
            Self::Playing { source, .. } => source.is_ready(),
            Self::Transitioning { outgoing, .. } => outgoing.is_ready(),
        }
    }

    /// Check if currently transitioning
    pub fn is_transitioning(&self) -> bool {
        matches!(self, Self::Transitioning { .. })
    }

    /// Complete transition by dropping outgoing source and promoting incoming
    ///
    /// Panics if not in Transitioning state.
    #[must_use]
    pub fn complete_transition(self) -> Self {
        match self {
            Self::Transitioning {
                incoming,
                incoming_track,
                ..
            } => Self::Playing {
                source: incoming,
                track: incoming_track,
            },
            _ => panic!("complete_transition called on non-transitioning state"),
        }
    }

    /// Start a transition from current playing state to a new source
    ///
    /// Panics if not in Playing state.
    #[must_use]
    pub fn start_transition(
        self,
        incoming: Box<dyn AudioSource>,
        incoming_track: QueueTrack,
        crossfade_progress: Option<f32>,
    ) -> Self {
        match self {
            Self::Playing { source, track } => Self::Transitioning {
                outgoing: source,
                outgoing_track: track,
                incoming,
                incoming_track,
                crossfade_progress,
            },
            _ => panic!("start_transition called on non-playing state"),
        }
    }

    /// Take the current source out of the state, leaving Empty
    pub fn take(self) -> (Option<Box<dyn AudioSource>>, Option<QueueTrack>) {
        match self {
            Self::Empty => (None, None),
            Self::Playing { source, track } => (Some(source), Some(track)),
            Self::Transitioning {
                outgoing,
                outgoing_track,
                ..
            } => (Some(outgoing), Some(outgoing_track)),
        }
    }

    /// Take the current track out, consuming and replacing the state with Empty
    ///
    /// This is useful when saving the current track to history before loading a new one.
    pub fn take_current_track(&mut self) -> Option<QueueTrack> {
        let old_state = std::mem::replace(self, Self::Empty);
        let (_, track) = old_state.take();
        track
    }
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

impl RepeatMode {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::All => "all",
            Self::One => "one",
        }
    }
}

/// Shuffle mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ShuffleMode {
    /// No shuffling
    #[default]
    Off,

    /// Pure random shuffle
    Random,

    /// Smart shuffle (avoid recently played, distribute artists)
    Smart,
}

impl ShuffleMode {
    /// Convert shuffle mode to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Random => "random",
            Self::Smart => "smart",
        }
    }

    /// Parse shuffle mode from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "off" => Some(Self::Off),
            "random" => Some(Self::Random),
            "smart" => Some(Self::Smart),
            _ => None,
        }
    }

    /// Cycle to next shuffle mode
    ///
    /// Off → Random → Smart → Off
    #[must_use]
    pub fn cycle(&self) -> Self {
        match self {
            Self::Off => Self::Random,
            Self::Random => Self::Smart,
            Self::Smart => Self::Off,
        }
    }
}

/// Minimum allowed history size
pub const MIN_HISTORY_SIZE: usize = 1;

/// Maximum allowed history size
pub const MAX_HISTORY_SIZE: usize = 1000;

/// Maximum allowed volume level
pub const MAX_VOLUME: u8 = 100;

/// Default volume level
pub const DEFAULT_VOLUME: u8 = 80;

/// Default history size
pub const DEFAULT_HISTORY_SIZE: usize = 50;

/// Configuration validation error
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigValidationError {
    /// History size is out of valid range (1-1000)
    HistorySizeOutOfRange {
        value: usize,
        min: usize,
        max: usize,
    },
    /// Volume is out of valid range (0-100)
    VolumeOutOfRange { value: u8, max: u8 },
    /// Crossfade duration is out of valid range (0-10000ms)
    CrossfadeDurationOutOfRange { value: u32, max: u32 },
}

impl std::fmt::Display for ConfigValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HistorySizeOutOfRange { value, min, max } => {
                write!(
                    f,
                    "History size {} is out of valid range ({}-{})",
                    value, min, max
                )
            }
            Self::VolumeOutOfRange { value, max } => {
                write!(f, "Volume {} exceeds maximum value {}", value, max)
            }
            Self::CrossfadeDurationOutOfRange { value, max } => {
                write!(
                    f,
                    "Crossfade duration {}ms exceeds maximum {}ms",
                    value, max
                )
            }
        }
    }
}

impl std::error::Error for ConfigValidationError {}

/// Configuration for playback manager
#[derive(Debug, Clone)]
pub struct PlaybackConfig {
    /// Maximum history size (default: 50, range: 1-1000)
    pub history_size: usize,

    /// Initial volume (0-100, default: 80)
    pub volume: u8,

    /// Initial shuffle mode (default: Off)
    pub shuffle: ShuffleMode,

    /// Initial repeat mode (default: Off)
    pub repeat: RepeatMode,

    /// Gapless playback enabled (default: true)
    pub gapless: bool,

    /// Crossfade settings
    pub crossfade: CrossfadeSettings,
}

impl Default for PlaybackConfig {
    fn default() -> Self {
        Self {
            history_size: DEFAULT_HISTORY_SIZE,
            volume: DEFAULT_VOLUME,
            shuffle: ShuffleMode::Off,
            repeat: RepeatMode::Off,
            gapless: true,
            crossfade: CrossfadeSettings::default(),
        }
    }
}

impl PlaybackConfig {
    /// Create config with gapless playback (no crossfade)
    pub fn gapless() -> Self {
        Self {
            crossfade: CrossfadeSettings::gapless(),
            ..Default::default()
        }
    }

    /// Create config with crossfade
    ///
    /// Duration is clamped to maximum of 10000ms.
    pub fn with_crossfade(duration_ms: u32, curve: FadeCurve) -> Self {
        Self {
            crossfade: CrossfadeSettings::with_duration_and_curve(duration_ms, curve),
            ..Default::default()
        }
    }

    /// Validate the configuration and return any errors
    ///
    /// Returns `Ok(())` if the configuration is valid, otherwise returns
    /// a list of all validation errors found.
    pub fn validate(&self) -> Result<(), Vec<ConfigValidationError>> {
        let mut errors = Vec::new();

        // Validate history size
        if self.history_size < MIN_HISTORY_SIZE || self.history_size > MAX_HISTORY_SIZE {
            errors.push(ConfigValidationError::HistorySizeOutOfRange {
                value: self.history_size,
                min: MIN_HISTORY_SIZE,
                max: MAX_HISTORY_SIZE,
            });
        }

        // Validate volume
        if self.volume > MAX_VOLUME {
            errors.push(ConfigValidationError::VolumeOutOfRange {
                value: self.volume,
                max: MAX_VOLUME,
            });
        }

        // Validate crossfade settings
        if let Err(crossfade_errors) = self.crossfade.validate() {
            errors.extend(crossfade_errors);
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Create a validated configuration, clamping values to valid ranges
    ///
    /// This ensures the configuration is always valid by clamping out-of-range
    /// values instead of returning errors.
    #[must_use]
    pub fn validated(mut self) -> Self {
        // Clamp history size
        self.history_size = self.history_size.clamp(MIN_HISTORY_SIZE, MAX_HISTORY_SIZE);

        // Clamp volume
        self.volume = self.volume.min(MAX_VOLUME);

        // Ensure crossfade is validated
        self.crossfade = self.crossfade.validated();

        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = PlaybackConfig::default();
        assert_eq!(config.history_size, DEFAULT_HISTORY_SIZE);
        assert_eq!(config.volume, DEFAULT_VOLUME);
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

    // ========================================
    // Configuration Validation Tests
    // ========================================

    #[test]
    fn config_validation_default_is_valid() {
        let config = PlaybackConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn config_validation_history_size_zero() {
        let config = PlaybackConfig {
            history_size: 0,
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            errors[0],
            ConfigValidationError::HistorySizeOutOfRange { value: 0, .. }
        ));
    }

    #[test]
    fn config_validation_history_size_too_large() {
        let config = PlaybackConfig {
            history_size: MAX_HISTORY_SIZE + 1,
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(matches!(
            errors[0],
            ConfigValidationError::HistorySizeOutOfRange { .. }
        ));
    }

    #[test]
    fn config_validation_volume_too_high() {
        let config = PlaybackConfig {
            volume: 150,
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(matches!(
            errors[0],
            ConfigValidationError::VolumeOutOfRange {
                value: 150,
                max: 100
            }
        ));
    }

    #[test]
    fn config_validation_volume_at_max_is_valid() {
        let config = PlaybackConfig {
            volume: MAX_VOLUME,
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn config_validation_crossfade_duration_too_high() {
        let config = PlaybackConfig {
            crossfade: CrossfadeSettings {
                duration_ms: 15000,
                ..Default::default()
            },
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(matches!(
            errors[0],
            ConfigValidationError::CrossfadeDurationOutOfRange {
                value: 15000,
                max: 10000
            }
        ));
    }

    #[test]
    fn config_validation_multiple_errors() {
        let config = PlaybackConfig {
            history_size: 0,
            volume: 200,
            crossfade: CrossfadeSettings {
                duration_ms: 20000,
                ..Default::default()
            },
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 3);
    }

    #[test]
    fn config_validated_clamps_values() {
        let config = PlaybackConfig {
            history_size: 0,
            volume: 200,
            crossfade: CrossfadeSettings {
                duration_ms: 20000,
                ..Default::default()
            },
            ..Default::default()
        };

        let validated = config.validated();

        // Values should be clamped to valid ranges
        assert_eq!(validated.history_size, MIN_HISTORY_SIZE);
        assert_eq!(validated.volume, MAX_VOLUME);
        assert_eq!(
            validated.crossfade.duration_ms,
            crate::crossfade::MAX_CROSSFADE_DURATION_MS
        );

        // Validated config should pass validation
        assert!(validated.validate().is_ok());
    }

    #[test]
    fn config_validated_preserves_valid_values() {
        let config = PlaybackConfig {
            history_size: 25,
            volume: 50,
            shuffle: ShuffleMode::Random,
            repeat: RepeatMode::All,
            gapless: false,
            crossfade: CrossfadeSettings::with_duration(5000),
        };

        let validated = config.validated();

        assert_eq!(validated.history_size, 25);
        assert_eq!(validated.volume, 50);
        assert_eq!(validated.shuffle, ShuffleMode::Random);
        assert_eq!(validated.repeat, RepeatMode::All);
        assert!(!validated.gapless);
        assert_eq!(validated.crossfade.duration_ms, 5000);
    }

    #[test]
    fn config_with_crossfade_clamps_duration() {
        let config = PlaybackConfig::with_crossfade(20000, FadeCurve::Linear);
        assert_eq!(
            config.crossfade.duration_ms,
            crate::crossfade::MAX_CROSSFADE_DURATION_MS
        );
    }

    #[test]
    fn config_validation_error_display() {
        let error = ConfigValidationError::HistorySizeOutOfRange {
            value: 0,
            min: 1,
            max: 1000,
        };
        assert!(error.to_string().contains('0'));
        assert!(error.to_string().contains("1-1000"));

        let error = ConfigValidationError::VolumeOutOfRange {
            value: 150,
            max: 100,
        };
        assert!(error.to_string().contains("150"));
        assert!(error.to_string().contains("100"));

        let error = ConfigValidationError::CrossfadeDurationOutOfRange {
            value: 20000,
            max: 10000,
        };
        assert!(error.to_string().contains("20000ms"));
        assert!(error.to_string().contains("10000ms"));
    }

    #[test]
    fn shuffle_mode_cycle() {
        let mode = ShuffleMode::Off;
        assert_eq!(mode.cycle(), ShuffleMode::Random);
        assert_eq!(ShuffleMode::Random.cycle(), ShuffleMode::Smart);
        assert_eq!(ShuffleMode::Smart.cycle(), ShuffleMode::Off);
    }

    #[test]
    fn shuffle_mode_from_str() {
        assert_eq!(ShuffleMode::from_str("off"), Some(ShuffleMode::Off));
        assert_eq!(ShuffleMode::from_str("random"), Some(ShuffleMode::Random));
        assert_eq!(ShuffleMode::from_str("smart"), Some(ShuffleMode::Smart));
        assert_eq!(ShuffleMode::from_str("invalid"), None);
    }
}
