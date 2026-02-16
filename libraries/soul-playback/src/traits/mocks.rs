//! Mock implementations of component traits for testing
//!
//! These mock implementations provide:
//! - Lightweight alternatives to full components
//! - Deterministic behavior for unit tests
//! - No I/O operations (files, audio devices, etc.)
//! - Simple state machines for testing logic
//!
//! # Usage
//!
//! ```ignore
//! use soul_playback::traits::mocks::*;
//!
//! #[test]
//! fn test_playback_flow() {
//!     let queue = MockQueue::with_tracks(vec![track1, track2]);
//!     let audio = MockAudio::new();
//!     let volume = MockVolume::new(80, false);
//!     let state = MockState::new();
//!     let fades = MockFade::new();
//!
//!     // Test coordinator logic without real I/O
//!     // coordinator.play().unwrap();
//!     // assert_eq!(state.state(), PlaybackState::Playing);
//! }
//! ```

use super::{AudioProcessing, FadeManagement, QueueOperations, StateTracking, VolumeControl};
use crate::crossfade::{CrossfadeSettings, CrossfadeState, FadeCurve};
use crate::error::{PlaybackError, Result};
use crate::events::PlaybackEvent;
use crate::fade_envelopes::FadeCompleteAction;
use crate::lazy_queue::{LazyQueueState, QueueContext};
use crate::source::AudioSource;
use crate::types::{PlaybackState, QueueTrack, RepeatMode, ShuffleMode};

use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;

// =============================================================================
// MockQueue - Minimal queue for testing
// =============================================================================

/// Mock queue implementation for testing
///
/// Provides deterministic queue behavior without shuffle complexity,
/// lazy loading, or advanced features.
#[cfg(test)]
pub struct MockQueue {
    tracks: VecDeque<QueueTrack>,
    history: Vec<QueueTrack>,
    current: Option<QueueTrack>,
    next: Option<QueueTrack>,
    repeat: RepeatMode,
    shuffle: ShuffleMode,
}

#[cfg(test)]
impl MockQueue {
    /// Create a new empty mock queue
    pub fn new() -> Self {
        Self {
            tracks: VecDeque::new(),
            history: Vec::new(),
            current: None,
            next: None,
            repeat: RepeatMode::Off,
            shuffle: ShuffleMode::Off,
        }
    }

    /// Create a mock queue with pre-loaded tracks
    pub fn with_tracks(tracks: Vec<QueueTrack>) -> Self {
        Self {
            tracks: tracks.into(),
            history: Vec::new(),
            current: None,
            next: None,
            repeat: RepeatMode::Off,
            shuffle: ShuffleMode::Off,
        }
    }

    /// Create a mock queue with repeat mode
    pub fn with_repeat(mut self, repeat: RepeatMode) -> Self {
        self.repeat = repeat;
        self
    }
}

#[cfg(test)]
impl QueueOperations for MockQueue {
    fn current_track(&self) -> Option<&QueueTrack> {
        self.current.as_ref()
    }

    fn set_current_track(&mut self, track: Option<QueueTrack>) {
        self.current = track;
    }

    fn take_current_track(&mut self) -> Option<QueueTrack> {
        self.current.take()
    }

    fn next_track(&self) -> Option<&QueueTrack> {
        self.next.as_ref()
    }

    fn set_next_track(&mut self, track: Option<QueueTrack>) {
        self.next = track;
    }

    fn take_next_track(&mut self) -> Option<QueueTrack> {
        self.next.take()
    }

    fn add_to_queue_next(&mut self, track: QueueTrack) {
        self.tracks.push_front(track);
    }

    fn add_to_queue_end(&mut self, track: QueueTrack) {
        self.tracks.push_back(track);
    }

    fn remove_from_queue(&mut self, index: usize) -> Result<QueueTrack> {
        self.tracks
            .remove(index)
            .ok_or(PlaybackError::IndexOutOfBounds(index))
    }

    fn reorder_queue(&mut self, from: usize, to: usize) -> Result<()> {
        if from >= self.tracks.len() || to >= self.tracks.len() {
            return Err(PlaybackError::InvalidOperation(
                "Index out of bounds".to_string(),
            ));
        }
        let track = self.tracks.remove(from).unwrap();
        self.tracks.insert(to, track);
        Ok(())
    }

    fn clear_play_next(&mut self) {
        self.tracks.clear();
    }

    fn clear_queued_later(&mut self) {
        self.tracks.clear();
    }

    fn clear_queue(&mut self) {
        self.tracks.clear();
    }

    fn get_queue(&self) -> Vec<&QueueTrack> {
        self.tracks.iter().collect()
    }

    fn queue_len(&self) -> usize {
        self.tracks.len()
    }

    fn is_queue_empty(&self) -> bool {
        self.tracks.is_empty()
    }

    fn get_source_total(&self) -> usize {
        self.tracks.len()
    }

    fn current_position_in_source(&self) -> usize {
        0
    }

    fn peek_next(&self) -> Option<&QueueTrack> {
        self.tracks.front()
    }

    fn peek_first_source_track(&self) -> Option<&QueueTrack> {
        self.tracks.front()
    }

    fn load_playlist(&mut self, tracks: Vec<QueueTrack>, _start_index: usize) {
        self.tracks = tracks.into();
        self.history.clear();
    }

    fn add_playlist_to_queue(&mut self, tracks: Vec<QueueTrack>) {
        self.tracks = tracks.into();
        self.history.clear();
    }

    fn append_to_queue(&mut self, tracks: Vec<QueueTrack>) {
        self.tracks.extend(tracks);
    }

    fn append_to_source(&mut self, tracks: Vec<QueueTrack>) {
        self.tracks.extend(tracks);
    }

    fn push_history(&mut self, track: QueueTrack) {
        self.history.push(track);
    }

    fn pop_history(&mut self) -> Option<QueueTrack> {
        self.history.pop()
    }

    fn is_history_empty(&self) -> bool {
        self.history.is_empty()
    }

    fn get_history(&self) -> Vec<&QueueTrack> {
        self.history.iter().collect()
    }

    fn clear_history(&mut self) {
        self.history.clear();
    }

    fn can_go_back(&self) -> bool {
        !self.history.is_empty()
    }

    fn go_back(&mut self) {
        // Mock: do nothing
    }

    fn get_next_track_from_queue(&mut self) -> Result<QueueTrack> {
        self.tracks
            .pop_front()
            .or_else(|| {
                if self.repeat == RepeatMode::One {
                    self.current.clone()
                } else {
                    None
                }
            })
            .ok_or(PlaybackError::QueueEmpty)
    }

    fn skip_to_index(&mut self, _index: usize) -> Option<Vec<QueueTrack>> {
        None
    }

    fn peek_next_queue_track(&self) -> Option<&QueueTrack> {
        self.tracks.front()
    }

    fn has_next(&self) -> bool {
        !self.tracks.is_empty() || self.repeat == RepeatMode::One
    }

    fn has_previous(&self) -> bool {
        !self.history.is_empty() || self.repeat == RepeatMode::One
    }

    fn shuffle(&self) -> ShuffleMode {
        self.shuffle
    }

    fn set_shuffle(&mut self, mode: ShuffleMode) {
        self.shuffle = mode;
    }

    fn cycle_shuffle(&mut self) -> ShuffleMode {
        self.shuffle = match self.shuffle {
            ShuffleMode::Off => ShuffleMode::Random,
            ShuffleMode::Random => ShuffleMode::Smart,
            ShuffleMode::Smart => ShuffleMode::Off,
        };
        self.shuffle
    }

    fn repeat(&self) -> RepeatMode {
        self.repeat
    }

    fn set_repeat(&mut self, mode: RepeatMode) {
        self.repeat = mode;
    }

    fn set_lazy_context(&mut self, _context: QueueContext, _shuffle_seed: Option<u64>) {
        // Mock: do nothing
    }

    fn clear_lazy_context(&mut self) {
        // Mock: do nothing
    }

    fn get_lazy_state(&self) -> Option<&LazyQueueState> {
        None
    }

    fn check_batch_loading(&mut self) -> Option<(usize, usize)> {
        None
    }

    fn check_jump_loading(&mut self, _target_index: usize) -> Option<(usize, usize)> {
        None
    }

    fn transition_to_next(&mut self) -> Option<QueueTrack> {
        if let Some(track) = self.current.take() {
            self.history.push(track);
        }
        self.current = self.next.take();
        self.current.clone()
    }
}

// =============================================================================
// MockVolume - Simple volume control for testing
// =============================================================================

/// Mock volume controller for testing
///
/// Provides simple linear volume scaling without logarithmic curve.
#[cfg(test)]
pub struct MockVolume {
    level: u8,
    muted: bool,
}

#[cfg(test)]
impl MockVolume {
    /// Create a new mock volume controller
    pub fn new(level: u8, muted: bool) -> Self {
        Self { level, muted }
    }
}

#[cfg(test)]
impl VolumeControl for MockVolume {
    fn set_volume(&mut self, level: u8) {
        self.level = level.min(100);
    }

    fn get_volume(&self) -> u8 {
        self.level
    }

    fn mute(&mut self) {
        self.muted = true;
    }

    fn unmute(&mut self) {
        self.muted = false;
    }

    fn toggle_mute(&mut self) {
        self.muted = !self.muted;
    }

    fn is_muted(&self) -> bool {
        self.muted
    }

    fn apply(&mut self, buffer: &mut [f32]) {
        if self.muted {
            buffer.fill(0.0);
        } else {
            let gain = (self.level as f32) / 100.0;
            for sample in buffer {
                *sample *= gain;
            }
        }
    }
}

// =============================================================================
// MockState - Simple state tracker for testing
// =============================================================================

/// Mock state manager for testing
///
/// Provides event tracking without duplicate suppression or throttling.
#[cfg(test)]
pub struct MockState {
    state: PlaybackState,
    pending_state: Option<PlaybackState>,
    user_paused: bool,
    events: Vec<PlaybackEvent>,
    position_samples: usize,
    crossfade_metadata_switched: bool,
}

#[cfg(test)]
impl MockState {
    /// Create a new mock state manager
    pub fn new() -> Self {
        Self {
            state: PlaybackState::Stopped,
            pending_state: None,
            user_paused: false,
            events: Vec::new(),
            position_samples: 0,
            crossfade_metadata_switched: false,
        }
    }
}

#[cfg(test)]
impl StateTracking for MockState {
    fn state(&self) -> PlaybackState {
        self.state
    }

    fn set_state(&mut self, state: PlaybackState) {
        self.state = state;
    }

    fn pending_state(&self) -> Option<PlaybackState> {
        self.pending_state
    }

    fn set_pending_state(&mut self, state: Option<PlaybackState>) {
        self.pending_state = state;
    }

    fn user_paused(&self) -> bool {
        self.user_paused
    }

    fn set_user_paused(&mut self, paused: bool) {
        self.user_paused = paused;
    }

    fn push_event(&mut self, event: PlaybackEvent) {
        self.events.push(event);
    }

    fn drain_events(&mut self) -> Vec<PlaybackEvent> {
        std::mem::take(&mut self.events)
    }

    fn has_pending_events(&self) -> bool {
        !self.events.is_empty()
    }

    fn emit_state_changed(&mut self, state: PlaybackState) {
        self.push_event(PlaybackEvent::StateChanged {
            state: state.into(),
        });
    }

    fn emit_track_changed(&mut self, track_id: Arc<str>, previous_track_id: Option<Arc<str>>) {
        self.push_event(PlaybackEvent::TrackChanged {
            track_id,
            previous_track_id,
        });
    }

    fn emit_crossfade_started(
        &mut self,
        from_track_id: Arc<str>,
        to_track_id: Arc<str>,
        duration_ms: u32,
    ) {
        self.push_event(PlaybackEvent::CrossfadeStarted {
            from_track_id,
            to_track_id,
            duration_ms,
        });
    }

    fn emit_crossfade_progress(&mut self, progress: f32, metadata_switched: bool) {
        self.push_event(PlaybackEvent::CrossfadeProgress {
            progress,
            metadata_switched,
        });
    }

    fn emit_crossfade_completed(&mut self) {
        self.push_event(PlaybackEvent::CrossfadeCompleted);
    }

    fn emit_track_finished(&mut self, track_id: String) {
        self.push_event(PlaybackEvent::TrackFinished { track_id });
    }

    fn emit_volume_changed(&mut self, level: u8, is_muted: bool) {
        self.push_event(PlaybackEvent::VolumeChanged { level, is_muted });
    }

    fn emit_queue_changed(&mut self, length: usize) {
        self.push_event(PlaybackEvent::QueueChanged { length });
    }

    fn emit_error(&mut self, message: String) {
        self.push_event(PlaybackEvent::Error { message });
    }

    fn emit_next_track_prepared(&mut self, track_id: String) {
        self.push_event(PlaybackEvent::NextTrackPrepared { track_id });
    }

    fn position_update_samples(&self) -> usize {
        self.position_samples
    }

    fn set_position_update_samples(&mut self, samples: usize) {
        self.position_samples = samples;
    }

    fn add_position_update_samples(&mut self, samples: usize) {
        self.position_samples += samples;
    }

    fn crossfade_metadata_switched(&self) -> bool {
        self.crossfade_metadata_switched
    }

    fn set_crossfade_metadata_switched(&mut self, switched: bool) {
        self.crossfade_metadata_switched = switched;
    }

    fn reset_crossfade_progress_tracking(&mut self) {
        self.crossfade_metadata_switched = false;
    }

    fn reset_event_suppression(&mut self) {
        // Mock: do nothing
    }
}

// =============================================================================
// MockFade - Simple fade controller for testing
// =============================================================================

/// Mock fade controller for testing
///
/// Provides instant fades (no gradual envelope) for deterministic tests.
#[cfg(test)]
pub struct MockFade {
    start_active: bool,
    stop_active: bool,
    stop_action: Option<FadeCompleteAction>,
    source_ready: bool,
    source_wait_samples: usize,
    pending_source: Option<Box<dyn AudioSource>>,
}

#[cfg(test)]
impl MockFade {
    /// Create a new mock fade controller
    pub fn new() -> Self {
        Self {
            start_active: false,
            stop_active: false,
            stop_action: None,
            source_ready: false,
            source_wait_samples: 0,
            pending_source: None,
        }
    }
}

#[cfg(test)]
impl FadeManagement for MockFade {
    fn start_fade_in(&mut self, _preserve_dc_state: bool) {
        self.start_active = true;
    }

    fn is_start_fade_active(&self) -> bool {
        self.start_active
    }

    fn freeze_start_fade(&mut self) {
        self.start_active = false;
    }

    fn reset_start_fade(&mut self) {
        self.start_active = false;
    }

    fn process_start_fade(&mut self, _buffer: &mut [f32]) {
        // Mock: instant fade (no envelope)
        self.start_active = false;
    }

    fn start_fade_out(&mut self, action: FadeCompleteAction) {
        self.stop_active = true;
        self.stop_action = Some(action);
    }

    fn is_stop_fade_active(&self) -> bool {
        self.stop_active
    }

    fn reset_stop_fade(&mut self) {
        self.stop_active = false;
        self.stop_action = None;
    }

    fn process_stop_fade(&mut self, _buffer: &mut [f32]) -> Option<FadeCompleteAction> {
        // Mock: instant fade (no envelope)
        if self.stop_active {
            self.stop_active = false;
            self.stop_action.take()
        } else {
            None
        }
    }

    fn set_pending_source(&mut self, source: Box<dyn AudioSource>) {
        self.pending_source = Some(source);
    }

    fn take_pending_source(&mut self) -> Option<Box<dyn AudioSource>> {
        self.pending_source.take()
    }

    fn has_pending_source(&self) -> bool {
        self.pending_source.is_some()
    }

    fn clear_pending_source(&mut self) {
        self.pending_source = None;
    }

    fn source_ready_verified(&self) -> bool {
        self.source_ready
    }

    fn set_source_ready_verified(&mut self, verified: bool) {
        self.source_ready = verified;
    }

    fn source_ready_wait_samples(&self) -> usize {
        self.source_wait_samples
    }

    fn add_source_ready_wait_samples(&mut self, samples: usize) {
        self.source_wait_samples += samples;
    }

    fn reset_source_tracking(&mut self) {
        self.source_ready = false;
        self.source_wait_samples = 0;
    }

    fn fill_underrun_buffer(&mut self, buffer: &mut [f32]) {
        buffer.fill(0.0);
    }

    fn set_sample_rate(&mut self, _sample_rate: u32) {
        // Mock: do nothing
    }

    fn reset_all(&mut self) {
        self.start_active = false;
        self.stop_active = false;
        self.stop_action = None;
        self.source_ready = false;
        self.source_wait_samples = 0;
        self.pending_source = None;
    }
}

// =============================================================================
// MockAudio - Minimal audio pipeline for testing
// =============================================================================

/// Mock audio pipeline for testing
///
/// Provides no-op audio processing without real I/O.
/// Returns silence for all audio operations.
#[cfg(test)]
pub struct MockAudio {
    current_source: Option<Box<dyn AudioSource>>,
    crossfade_active: bool,
    crossfade_progress: f32,
    sample_rate: u32,
    output_channels: u16,
    manual_skip: bool,
    gapless_enabled: bool,
    crossfade_settings: CrossfadeSettings,
}

#[cfg(test)]
impl MockAudio {
    /// Create a new mock audio pipeline
    pub fn new() -> Self {
        Self {
            current_source: None,
            crossfade_active: false,
            crossfade_progress: 0.0,
            sample_rate: 44100,
            output_channels: 2,
            manual_skip: false,
            gapless_enabled: true,
            crossfade_settings: CrossfadeSettings::default(),
        }
    }
}

#[cfg(test)]
impl AudioProcessing for MockAudio {
    fn audio_source(&self) -> Option<&dyn AudioSource> {
        self.current_source.as_deref()
    }

    fn audio_source_mut(&mut self) -> Option<&mut Box<dyn AudioSource>> {
        self.current_source.as_mut()
    }

    fn has_audio_source(&self) -> bool {
        self.current_source.is_some()
    }

    fn set_audio_source(&mut self, source: Option<Box<dyn AudioSource>>) {
        self.current_source = source;
    }

    fn take_audio_source(&mut self) -> Option<Box<dyn AudioSource>> {
        self.current_source.take()
    }

    fn read_source_into_stereo_buffer(
        &mut self,
        max_output_frames: usize,
    ) -> Result<Option<usize>> {
        // Mock: return silence
        Ok(Some(max_output_frames * 2))
    }

    fn source_position_duration(&self) -> Option<(Duration, Duration)> {
        self.current_source
            .as_ref()
            .map(|s| (s.position(), s.duration()))
    }

    fn stereo_buffer_slice_mut(&mut self, _range: std::ops::Range<usize>) -> &mut [f32] {
        // Mock: return empty slice
        &mut []
    }

    fn convert_stereo_to_mono(&self, _output: &mut [f32], samples_read: usize) -> usize {
        samples_read / 2
    }

    fn upmix_stereo_to_multichannel(&self, _output: &mut [f32], samples_read: usize) -> usize {
        samples_read
    }

    fn start_crossfade(&mut self) -> bool {
        if self.crossfade_settings.enabled && self.has_next_source() {
            self.crossfade_active = true;
            self.crossfade_progress = 0.0;
            true
        } else {
            false
        }
    }

    fn process_active_crossfade(&mut self, output: &mut [f32]) -> Result<(usize, bool)> {
        if !self.crossfade_active {
            return Ok((0, false));
        }

        // Mock: instant crossfade completion
        output.fill(0.0);
        self.crossfade_active = false;
        self.crossfade_progress = 1.0;
        Ok((output.len(), true))
    }

    fn transition_sources(&mut self) {
        self.current_source = self.next_source.take();
        self.manual_skip = false;
    }

    fn is_crossfading(&self) -> bool {
        self.crossfade_active
    }

    fn get_crossfade_progress(&self) -> f32 {
        self.crossfade_progress
    }

    fn get_crossfade_state(&self) -> CrossfadeState {
        if self.crossfade_active {
            CrossfadeState::Active
        } else {
            CrossfadeState::Inactive
        }
    }

    fn is_source_compatible(&self, _source: &dyn AudioSource) -> bool {
        true
    }

    fn time_until_crossfade(&self) -> Option<Duration> {
        None
    }

    fn set_crossfade_settings(&mut self, settings: CrossfadeSettings) {
        self.crossfade_settings = settings;
    }

    fn get_crossfade_settings(&self) -> &CrossfadeSettings {
        &self.crossfade_settings
    }

    fn set_crossfade_enabled(&mut self, enabled: bool) {
        self.crossfade_settings.enabled = enabled;
    }

    fn is_crossfade_enabled(&self) -> bool {
        self.crossfade_settings.enabled
    }

    fn set_crossfade_duration(&mut self, duration_ms: u32) {
        self.crossfade_settings.duration_ms = duration_ms;
    }

    fn get_crossfade_duration(&self) -> u32 {
        self.crossfade_settings.duration_ms
    }

    fn set_crossfade_curve(&mut self, curve: FadeCurve) {
        self.crossfade_settings.curve = curve;
    }

    fn get_crossfade_curve(&self) -> FadeCurve {
        self.crossfade_settings.curve
    }

    fn set_crossfade_on_skip(&mut self, on_skip: bool) {
        self.crossfade_settings.on_skip = on_skip;
    }

    fn apply_processing_chain(&mut self, _buffer: &mut [f32], _volume: &mut dyn VolumeControl) {
        // Mock: do nothing
    }

    fn apply_processing_chain_on_stereo_buffer(
        &mut self,
        _samples_read: usize,
        _volume: &mut dyn VolumeControl,
    ) {
        // Mock: do nothing
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn set_sample_rate(&mut self, sample_rate: u32) {
        self.sample_rate = sample_rate;
    }

    fn output_channels(&self) -> u16 {
        self.output_channels
    }

    fn set_output_channels(&mut self, channels: u16) {
        self.output_channels = channels;
    }

    fn is_manual_skip(&self) -> bool {
        self.manual_skip
    }

    fn set_manual_skip(&mut self, is_manual: bool) {
        self.manual_skip = is_manual;
    }

    fn gapless_enabled(&self) -> bool {
        self.gapless_enabled
    }

    fn set_gapless_enabled(&mut self, enabled: bool) {
        self.gapless_enabled = enabled;
    }

    fn allocate_crossfade_buffers(&mut self) {
        // Mock: do nothing
    }

    fn free_crossfade_buffers(&mut self) {
        // Mock: do nothing
    }

    #[cfg(feature = "effects")]
    fn effect_chain_mut(&mut self) -> &mut soul_audio::effects::EffectChain {
        unimplemented!("Mock does not support effects")
    }

    // Volume-leveling feature removed - replaced by ReplayGain
    // See soul-playback/src/replay_gain.rs for new implementation

    fn reset_all(&mut self) {
        self.current_source = None;
        self.next_source = None;
        self.crossfade_active = false;
        self.crossfade_progress = 0.0;
        self.manual_skip = false;
    }
}
