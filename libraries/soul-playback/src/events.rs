//! Playback Events
//!
//! Event-based communication for UI synchronization during playback.
//! Events are emitted at key points:
//! - State changes (play/pause/stop)
//! - Track changes (at 50% crossfade or immediately for gapless)
//! - Crossfade progress updates
//! - Position updates (periodic)

use serde::{Deserialize, Serialize};

/// Events emitted by the playback system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlaybackEvent {
    /// Playback state changed (playing, paused, stopped, etc.)
    StateChanged {
        /// The new playback state
        state: PlaybackStateEvent,
    },

    /// Track changed - emitted when audio transition happens
    ///
    /// For crossfade: emitted at 50% progress (metadata switch point)
    /// For gapless: emitted immediately on transition
    /// For manual skip: emitted immediately
    TrackChanged {
        /// ID of the new (current) track
        track_id: String,
        /// ID of the previous track (if any)
        previous_track_id: Option<String>,
    },

    /// Crossfade started between two tracks
    CrossfadeStarted {
        /// ID of the outgoing track
        from_track_id: String,
        /// ID of the incoming track
        to_track_id: String,
        /// Duration of the crossfade in milliseconds
        duration_ms: u32,
    },

    /// Crossfade progress update (for UI animations)
    CrossfadeProgress {
        /// Progress from 0.0 (just started) to 1.0 (complete)
        progress: f32,
        /// Whether the metadata has been switched (at 50%)
        metadata_switched: bool,
    },

    /// Crossfade completed
    CrossfadeCompleted,

    /// Track finished playing naturally (reached end)
    TrackFinished {
        /// ID of the finished track
        track_id: String,
    },

    /// Position update (periodic, typically every 500ms-1s)
    PositionUpdate {
        /// Current playback position
        position_ms: u64,
        /// Total track duration
        duration_ms: u64,
    },

    /// Next track has been prepared (pre-decoded and ready)
    NextTrackPrepared {
        /// ID of the prepared track
        track_id: String,
    },

    /// Volume changed
    VolumeChanged {
        /// New volume level (0-100)
        level: u8,
        /// Whether audio is muted
        is_muted: bool,
    },

    /// Queue changed (tracks added/removed/reordered)
    QueueChanged {
        /// New queue length
        length: usize,
    },

    /// Error occurred during playback
    Error {
        /// Error message
        message: String,
    },
}

/// Playback state for events
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlaybackStateEvent {
    /// No track loaded
    Stopped,
    /// Currently loading a track
    Loading,
    /// Playing audio
    Playing,
    /// Paused mid-track
    Paused,
    /// Crossfading between two tracks
    Crossfading,
}

impl From<crate::types::PlaybackState> for PlaybackStateEvent {
    fn from(state: crate::types::PlaybackState) -> Self {
        match state {
            crate::types::PlaybackState::Stopped => PlaybackStateEvent::Stopped,
            crate::types::PlaybackState::Loading => PlaybackStateEvent::Loading,
            crate::types::PlaybackState::Playing => PlaybackStateEvent::Playing,
            crate::types::PlaybackState::Paused => PlaybackStateEvent::Paused,
        }
    }
}

/// Crossfade progress tracker
///
/// Tracks crossfade state and determines when to switch metadata.
#[derive(Debug, Clone)]
pub struct CrossfadeProgressTracker {
    /// Progress from 0.0 to 1.0
    progress: f32,
    /// Duration of the crossfade in milliseconds
    duration_ms: u32,
    /// Whether the metadata switch has occurred (at 50%)
    metadata_switched: bool,
    /// Whether crossfade is currently active
    active: bool,
    /// Outgoing track ID
    from_track_id: Option<String>,
    /// Incoming track ID
    to_track_id: Option<String>,
}

impl Default for CrossfadeProgressTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl CrossfadeProgressTracker {
    /// Create a new crossfade progress tracker
    pub fn new() -> Self {
        Self {
            progress: 0.0,
            duration_ms: 0,
            metadata_switched: false,
            active: false,
            from_track_id: None,
            to_track_id: None,
        }
    }

    /// Start tracking a new crossfade
    pub fn start(&mut self, from_track_id: String, to_track_id: String, duration_ms: u32) {
        self.progress = 0.0;
        self.duration_ms = duration_ms;
        self.metadata_switched = false;
        self.active = true;
        self.from_track_id = Some(from_track_id);
        self.to_track_id = Some(to_track_id);
    }

    /// Update crossfade progress
    ///
    /// Returns true if metadata should be switched (first time crossing 50%)
    pub fn update(&mut self, progress: f32) -> bool {
        self.progress = progress.clamp(0.0, 1.0);

        // Check if we should switch metadata (at 50%)
        if !self.metadata_switched && self.progress >= 0.5 {
            self.metadata_switched = true;
            return true;
        }

        false
    }

    /// Reset the tracker (crossfade completed or cancelled)
    pub fn reset(&mut self) {
        self.progress = 0.0;
        self.duration_ms = 0;
        self.metadata_switched = false;
        self.active = false;
        self.from_track_id = None;
        self.to_track_id = None;
    }

    /// Check if crossfade is active
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Get current progress
    pub fn progress(&self) -> f32 {
        self.progress
    }

    /// Check if metadata has been switched
    pub fn metadata_switched(&self) -> bool {
        self.metadata_switched
    }

    /// Check if crossfade is complete
    pub fn is_complete(&self) -> bool {
        self.progress >= 1.0
    }

    /// Get the track ID that should be displayed
    ///
    /// Before 50%: returns outgoing track
    /// After 50%: returns incoming track
    pub fn display_track_id(&self) -> Option<&str> {
        if self.metadata_switched {
            self.to_track_id.as_deref()
        } else {
            self.from_track_id.as_deref()
        }
    }

    /// Get outgoing track ID
    pub fn from_track_id(&self) -> Option<&str> {
        self.from_track_id.as_deref()
    }

    /// Get incoming track ID
    pub fn to_track_id(&self) -> Option<&str> {
        self.to_track_id.as_deref()
    }

    /// Get duration in milliseconds
    pub fn duration_ms(&self) -> u32 {
        self.duration_ms
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crossfade_progress_tracker_metadata_switch() {
        let mut tracker = CrossfadeProgressTracker::new();
        tracker.start("track1".to_string(), "track2".to_string(), 3000);

        assert!(tracker.is_active());
        assert!(!tracker.metadata_switched());
        assert_eq!(tracker.display_track_id(), Some("track1"));

        // Update to 30% - no switch yet
        let switched = tracker.update(0.3);
        assert!(!switched);
        assert!(!tracker.metadata_switched());
        assert_eq!(tracker.display_track_id(), Some("track1"));

        // Update to 50% - should switch
        let switched = tracker.update(0.5);
        assert!(switched);
        assert!(tracker.metadata_switched());
        assert_eq!(tracker.display_track_id(), Some("track2"));

        // Update to 70% - already switched
        let switched = tracker.update(0.7);
        assert!(!switched); // Doesn't return true again
        assert!(tracker.metadata_switched());

        // Complete
        tracker.update(1.0);
        assert!(tracker.is_complete());
    }

    #[test]
    fn test_crossfade_progress_tracker_reset() {
        let mut tracker = CrossfadeProgressTracker::new();
        tracker.start("track1".to_string(), "track2".to_string(), 3000);
        tracker.update(0.6);
        assert!(tracker.metadata_switched());

        tracker.reset();
        assert!(!tracker.is_active());
        assert!(!tracker.metadata_switched());
        assert_eq!(tracker.progress(), 0.0);
    }

    #[test]
    fn test_playback_state_event_conversion() {
        use crate::types::PlaybackState;

        assert_eq!(
            PlaybackStateEvent::from(PlaybackState::Playing),
            PlaybackStateEvent::Playing
        );
        assert_eq!(
            PlaybackStateEvent::from(PlaybackState::Paused),
            PlaybackStateEvent::Paused
        );
        assert_eq!(
            PlaybackStateEvent::from(PlaybackState::Stopped),
            PlaybackStateEvent::Stopped
        );
    }
}
