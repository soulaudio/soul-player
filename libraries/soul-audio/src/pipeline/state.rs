//! Pipeline State Machine
//!
//! Defines the playback state machine with proper state transitions
//! and event emission for UI synchronization.

use std::time::Duration;

/// Track transition information
#[derive(Debug, Clone)]
pub struct TrackTransition {
    /// Track being transitioned from (outgoing)
    pub from_track_id: Option<String>,
    /// Track being transitioned to (incoming)
    pub to_track_id: String,
    /// Whether this is a crossfade transition
    pub is_crossfade: bool,
}

/// Crossfade progress information
#[derive(Debug, Clone, Copy)]
pub struct CrossfadeProgress {
    /// Progress from 0.0 (just started) to 1.0 (complete)
    pub progress: f32,
    /// Total duration of the crossfade
    pub duration_ms: u32,
    /// Whether the metadata switch point (50%) has been reached
    pub metadata_switched: bool,
}

impl CrossfadeProgress {
    /// Create a new crossfade progress tracker
    pub fn new(duration_ms: u32) -> Self {
        Self {
            progress: 0.0,
            duration_ms,
            metadata_switched: false,
        }
    }

    /// Update progress
    pub fn update(&mut self, progress: f32) {
        self.progress = progress.clamp(0.0, 1.0);
    }

    /// Check if we should switch metadata (at 50% progress)
    pub fn should_switch_metadata(&self) -> bool {
        !self.metadata_switched && self.progress >= 0.5
    }

    /// Mark metadata as switched
    pub fn mark_metadata_switched(&mut self) {
        self.metadata_switched = true;
    }

    /// Check if crossfade is complete
    pub fn is_complete(&self) -> bool {
        self.progress >= 1.0
    }
}

/// Events emitted by the pipeline state machine
#[derive(Debug, Clone)]
pub enum PipelineEvent {
    /// Playback state changed
    StateChanged(PipelineState),

    /// Track changed (emitted when audio transition happens, NOT after load)
    /// For crossfade: emitted at 50% progress
    /// For gapless: emitted immediately on transition
    TrackChanged {
        track_id: String,
        /// Previous track ID (if any)
        previous_track_id: Option<String>,
    },

    /// Crossfade started
    CrossfadeStarted {
        from_track_id: String,
        to_track_id: String,
        duration_ms: u32,
    },

    /// Crossfade progress update (for UI animations)
    CrossfadeProgress {
        progress: f32,
        /// Whether metadata has been switched
        metadata_switched: bool,
    },

    /// Crossfade completed
    CrossfadeCompleted,

    /// Track finished naturally (reached end)
    TrackFinished { track_id: String },

    /// Error occurred
    Error { message: String },

    /// Position update (periodic)
    PositionUpdate {
        position: Duration,
        duration: Duration,
    },

    /// Next track prepared (pre-decoded and ready)
    NextTrackPrepared { track_id: String },
}

/// Pipeline playback states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelineState {
    /// No track loaded, no playback
    Stopped,
    /// Track is being loaded/decoded
    Loading,
    /// Playing audio
    Playing,
    /// Playback paused
    Paused,
    /// Crossfading between two tracks
    Crossfading,
}

impl Default for PipelineState {
    fn default() -> Self {
        Self::Stopped
    }
}

/// Pipeline state machine for managing playback states and transitions
///
/// This state machine ensures:
/// 1. Valid state transitions only
/// 2. Proper event emission at the right times
/// 3. Crossfade metadata switch at 50%
/// 4. TrackChanged emitted on audio transition, not after load
#[derive(Debug)]
pub struct PipelineStateMachine {
    /// Current state
    state: PipelineState,
    /// Current track ID
    current_track_id: Option<String>,
    /// Next track ID (for gapless/crossfade)
    next_track_id: Option<String>,
    /// Crossfade progress (if crossfading)
    crossfade_progress: Option<CrossfadeProgress>,
    /// Pending events to be consumed
    pending_events: Vec<PipelineEvent>,
}

impl Default for PipelineStateMachine {
    fn default() -> Self {
        Self::new()
    }
}

impl PipelineStateMachine {
    /// Create a new state machine in Stopped state
    pub fn new() -> Self {
        Self {
            state: PipelineState::Stopped,
            current_track_id: None,
            next_track_id: None,
            crossfade_progress: None,
            pending_events: Vec::new(),
        }
    }

    /// Get current state
    pub fn state(&self) -> PipelineState {
        self.state
    }

    /// Get current track ID
    pub fn current_track_id(&self) -> Option<&str> {
        self.current_track_id.as_deref()
    }

    /// Get next track ID (if prepared)
    pub fn next_track_id(&self) -> Option<&str> {
        self.next_track_id.as_deref()
    }

    /// Check if crossfading
    pub fn is_crossfading(&self) -> bool {
        self.state == PipelineState::Crossfading
    }

    /// Get crossfade progress
    pub fn crossfade_progress(&self) -> Option<&CrossfadeProgress> {
        self.crossfade_progress.as_ref()
    }

    /// Drain pending events
    pub fn drain_events(&mut self) -> Vec<PipelineEvent> {
        std::mem::take(&mut self.pending_events)
    }

    /// Check if there are pending events
    pub fn has_pending_events(&self) -> bool {
        !self.pending_events.is_empty()
    }

    // ===== State Transitions =====

    /// Start loading a track
    pub fn start_loading(&mut self, track_id: String) {
        let old_state = self.state;
        self.state = PipelineState::Loading;

        // Don't change current_track_id yet - that happens on actual playback start
        self.next_track_id = Some(track_id);

        if old_state != PipelineState::Loading {
            self.pending_events
                .push(PipelineEvent::StateChanged(PipelineState::Loading));
        }
    }

    /// Track loaded and ready to play
    pub fn track_ready(&mut self, track_id: String) {
        let previous_track_id = self.current_track_id.take();
        self.current_track_id = Some(track_id.clone());
        self.next_track_id = None;
        self.state = PipelineState::Playing;

        // Emit TrackChanged - this is the key fix: emit when audio starts, not after load
        self.pending_events.push(PipelineEvent::TrackChanged {
            track_id,
            previous_track_id,
        });

        self.pending_events
            .push(PipelineEvent::StateChanged(PipelineState::Playing));
    }

    /// Start playback (from paused or stopped)
    pub fn play(&mut self) {
        if self.state == PipelineState::Paused || self.state == PipelineState::Stopped {
            self.state = PipelineState::Playing;
            self.pending_events
                .push(PipelineEvent::StateChanged(PipelineState::Playing));
        }
    }

    /// Pause playback
    pub fn pause(&mut self) {
        if self.state == PipelineState::Playing {
            self.state = PipelineState::Paused;
            self.pending_events
                .push(PipelineEvent::StateChanged(PipelineState::Paused));
        }
    }

    /// Stop playback
    pub fn stop(&mut self) {
        self.state = PipelineState::Stopped;
        self.current_track_id = None;
        self.next_track_id = None;
        self.crossfade_progress = None;

        self.pending_events
            .push(PipelineEvent::StateChanged(PipelineState::Stopped));
    }

    /// Prepare next track for gapless/crossfade
    pub fn prepare_next_track(&mut self, track_id: String) {
        self.next_track_id = Some(track_id.clone());
        self.pending_events
            .push(PipelineEvent::NextTrackPrepared { track_id });
    }

    /// Start crossfade transition
    pub fn start_crossfade(&mut self, duration_ms: u32) {
        if self.state != PipelineState::Playing {
            return;
        }

        let Some(to_track_id) = self.next_track_id.clone() else {
            return;
        };

        let from_track_id = self.current_track_id.clone().unwrap_or_default();

        self.state = PipelineState::Crossfading;
        self.crossfade_progress = Some(CrossfadeProgress::new(duration_ms));

        self.pending_events.push(PipelineEvent::CrossfadeStarted {
            from_track_id,
            to_track_id,
            duration_ms,
        });

        self.pending_events
            .push(PipelineEvent::StateChanged(PipelineState::Crossfading));
    }

    /// Update crossfade progress
    ///
    /// Returns true if metadata should be switched (at 50% progress)
    pub fn update_crossfade_progress(&mut self, progress: f32) -> bool {
        let Some(ref mut cf_progress) = self.crossfade_progress else {
            return false;
        };

        cf_progress.update(progress);

        // Check if we should switch metadata (at 50%)
        let should_switch = cf_progress.should_switch_metadata();

        if should_switch {
            cf_progress.mark_metadata_switched();

            // Switch current track to next track
            if let Some(next_id) = self.next_track_id.clone() {
                let previous_id = self.current_track_id.take();
                self.current_track_id = Some(next_id.clone());

                // Emit TrackChanged at 50% crossfade
                self.pending_events.push(PipelineEvent::TrackChanged {
                    track_id: next_id,
                    previous_track_id: previous_id,
                });
            }
        }

        // Emit progress event
        self.pending_events.push(PipelineEvent::CrossfadeProgress {
            progress,
            metadata_switched: cf_progress.metadata_switched,
        });

        should_switch
    }

    /// Complete crossfade transition
    pub fn complete_crossfade(&mut self) {
        self.crossfade_progress = None;
        self.next_track_id = None;
        self.state = PipelineState::Playing;

        self.pending_events.push(PipelineEvent::CrossfadeCompleted);
        self.pending_events
            .push(PipelineEvent::StateChanged(PipelineState::Playing));
    }

    /// Handle gapless transition (no crossfade)
    pub fn gapless_transition(&mut self) {
        if let Some(next_id) = self.next_track_id.take() {
            let previous_id = self.current_track_id.take();
            self.current_track_id = Some(next_id.clone());

            // Emit TrackChanged immediately
            self.pending_events.push(PipelineEvent::TrackChanged {
                track_id: next_id,
                previous_track_id: previous_id,
            });
        }
    }

    /// Track finished naturally
    pub fn track_finished(&mut self) {
        if let Some(track_id) = self.current_track_id.clone() {
            self.pending_events
                .push(PipelineEvent::TrackFinished { track_id });
        }
    }

    /// Report an error
    pub fn error(&mut self, message: String) {
        self.pending_events.push(PipelineEvent::Error { message });
    }

    /// Emit position update
    pub fn position_update(&mut self, position: Duration, duration: Duration) {
        self.pending_events.push(PipelineEvent::PositionUpdate {
            position,
            duration,
        });
    }

    /// Get the track ID that should be displayed in UI
    ///
    /// During crossfade before 50%: returns outgoing track
    /// During crossfade after 50%: returns incoming track
    /// Otherwise: returns current track
    pub fn display_track_id(&self) -> Option<&str> {
        self.current_track_id.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state() {
        let sm = PipelineStateMachine::new();
        assert_eq!(sm.state(), PipelineState::Stopped);
        assert!(sm.current_track_id().is_none());
    }

    #[test]
    fn test_basic_playback_flow() {
        let mut sm = PipelineStateMachine::new();

        // Start loading
        sm.start_loading("track1".to_string());
        assert_eq!(sm.state(), PipelineState::Loading);

        // Track ready - should emit TrackChanged
        sm.track_ready("track1".to_string());
        assert_eq!(sm.state(), PipelineState::Playing);
        assert_eq!(sm.current_track_id(), Some("track1"));

        let events = sm.drain_events();
        let has_track_changed = events
            .iter()
            .any(|e| matches!(e, PipelineEvent::TrackChanged { .. }));
        assert!(has_track_changed);
    }

    #[test]
    fn test_pause_resume() {
        let mut sm = PipelineStateMachine::new();
        sm.track_ready("track1".to_string());
        sm.drain_events();

        sm.pause();
        assert_eq!(sm.state(), PipelineState::Paused);

        sm.play();
        assert_eq!(sm.state(), PipelineState::Playing);
    }

    #[test]
    fn test_crossfade_metadata_switch_at_50_percent() {
        let mut sm = PipelineStateMachine::new();
        sm.track_ready("track1".to_string());
        sm.prepare_next_track("track2".to_string());
        sm.drain_events();

        // Start crossfade
        sm.start_crossfade(3000);
        assert_eq!(sm.state(), PipelineState::Crossfading);
        assert_eq!(sm.current_track_id(), Some("track1"));

        // Update to 30% - no switch yet
        let switched = sm.update_crossfade_progress(0.3);
        assert!(!switched);
        assert_eq!(sm.current_track_id(), Some("track1"));

        // Update to 50% - should switch
        let switched = sm.update_crossfade_progress(0.5);
        assert!(switched);
        assert_eq!(sm.current_track_id(), Some("track2"));

        // Check TrackChanged event was emitted
        let events = sm.drain_events();
        let track_changed = events.iter().find(|e| {
            matches!(e, PipelineEvent::TrackChanged { track_id, .. } if track_id == "track2")
        });
        assert!(track_changed.is_some());
    }

    #[test]
    fn test_crossfade_completion() {
        let mut sm = PipelineStateMachine::new();
        sm.track_ready("track1".to_string());
        sm.prepare_next_track("track2".to_string());
        sm.start_crossfade(3000);
        sm.update_crossfade_progress(0.5);
        sm.drain_events();

        sm.complete_crossfade();
        assert_eq!(sm.state(), PipelineState::Playing);
        assert!(sm.crossfade_progress().is_none());
    }

    #[test]
    fn test_gapless_transition() {
        let mut sm = PipelineStateMachine::new();
        sm.track_ready("track1".to_string());
        sm.prepare_next_track("track2".to_string());
        sm.drain_events();

        sm.gapless_transition();
        assert_eq!(sm.current_track_id(), Some("track2"));

        let events = sm.drain_events();
        let has_track_changed = events.iter().any(|e| {
            matches!(e, PipelineEvent::TrackChanged { track_id, .. } if track_id == "track2")
        });
        assert!(has_track_changed);
    }

    #[test]
    fn test_stop_clears_state() {
        let mut sm = PipelineStateMachine::new();
        sm.track_ready("track1".to_string());
        sm.prepare_next_track("track2".to_string());
        sm.start_crossfade(3000);
        sm.drain_events();

        sm.stop();
        assert_eq!(sm.state(), PipelineState::Stopped);
        assert!(sm.current_track_id().is_none());
        assert!(sm.next_track_id().is_none());
        assert!(sm.crossfade_progress().is_none());
    }
}
