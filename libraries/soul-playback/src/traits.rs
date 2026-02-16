//! Trait-based component abstractions for testability and flexibility
//!
//! This module provides trait abstractions for all core playback components,
//! enabling:
//! - **Mock implementations** for isolated unit testing
//! - **Runtime component swapping** for different use cases
//! - **Alternative implementations** (e.g., lightweight for tests, full-featured for production)
//! - **Clear contracts** defining component boundaries and responsibilities
//! - **Conditional compilation** with feature flags
//!
//! # Architecture
//!
//! The PlaybackManager is composed of five core components, each with a trait:
//!
//! ```text
//! ┌─────────────────────────────────────────┐
//! │         PlaybackManager                 │
//! ├─────────────────────────────────────────┤
//! │ - QueueOperations                       │
//! │ - AudioProcessing                       │
//! │ - VolumeControl                         │
//! │ - StateTracking                         │
//! │ - FadeManagement                        │
//! └─────────────────────────────────────────┘
//! ```
//!
//! # Testing Benefits
//!
//! ## Before (concrete types):
//! ```ignore
//! // Hard to test in isolation - requires full component initialization
//! let manager = PlaybackManager::new(config);
//! manager.play()?; // Triggers audio I/O, file access, etc.
//! ```
//!
//! ## After (trait-based):
//! ```ignore
//! // Easy to test with mocks - no I/O, deterministic behavior
//! let queue = MockQueue::with_tracks(vec![track1, track2]);
//! let audio = MockAudio::new();
//! let coordinator = PlaybackCoordinator::with_components(queue, audio, ...);
//! coordinator.play()?; // Pure state machine, no I/O
//! ```
//!
//! # Example Usage
//!
//! ## Production (concrete implementations):
//! ```ignore
//! use soul_playback::components::*;
//! use soul_playback::traits::*;
//!
//! let queue: Box<dyn QueueOperations> = Box::new(QueueManager::new(100, shuffle, repeat));
//! let audio: Box<dyn AudioProcessing> = Box::new(AudioPipeline::new(&config));
//! // ... construct coordinator with trait objects
//! ```
//!
//! ## Testing (mock implementations):
//! ```ignore
//! use soul_playback::traits::*;
//!
//! struct MockQueue {
//!     tracks: VecDeque<QueueTrack>,
//! }
//!
//! impl QueueOperations for MockQueue {
//!     fn next_track(&mut self) -> Result<QueueTrack> {
//!         self.tracks.pop_front().ok_or(PlaybackError::QueueEmpty)
//!     }
//!     // ... simplified implementations
//! }
//!
//! #[test]
//! fn test_playback_with_mock() {
//!     let queue = MockQueue { tracks: vec![track1, track2].into() };
//!     // Test coordinator logic without real I/O
//! }
//! ```

use crate::crossfade::{CrossfadeSettings, CrossfadeState, FadeCurve};
use crate::error::Result;
use crate::events::PlaybackEvent;
use crate::fade_envelopes::FadeCompleteAction;
use crate::lazy_queue::{LazyQueueState, QueueContext};
use crate::source::AudioSource;
use crate::types::{PlaybackState, QueueTrack, RepeatMode, ShuffleMode};

#[cfg(feature = "effects")]
use soul_audio::effects::EffectChain;

use std::sync::Arc;
use std::time::Duration;

// =============================================================================
// Submodules
// =============================================================================

/// Mock implementations for testing
///
/// Provides lightweight test doubles for all component traits.
#[cfg(test)]
pub mod mocks;

// =============================================================================
// Queue Operations Trait
// =============================================================================

/// Queue management operations for playback
///
/// Manages the three-tier queue system (Play Next, Source, Add to Queue),
/// history tracking, shuffle/repeat modes, and lazy loading state.
///
/// # Responsibilities
/// - Track ordering and navigation (next/previous)
/// - History management for "previous" button
/// - Shuffle/repeat mode logic
/// - Lazy queue batch loading coordination
/// - Current/next track metadata
///
/// # Implementation Notes
/// - Methods should be deterministic for testability
/// - Queue modifications should be atomic
/// - History size should be bounded to prevent memory growth
///
/// # Example Mock Implementation
/// ```ignore
/// struct MockQueue {
///     tracks: VecDeque<QueueTrack>,
///     history: Vec<QueueTrack>,
///     current: Option<QueueTrack>,
/// }
///
/// impl QueueOperations for MockQueue {
///     fn next_track(&mut self) -> Result<QueueTrack> {
///         self.tracks.pop_front().ok_or(PlaybackError::QueueEmpty)
///     }
///     // ... other methods with simplified logic
/// }
/// ```
pub trait QueueOperations: Send {
    // ===== Current Track =====

    /// Get currently playing track metadata
    fn current_track(&self) -> Option<&QueueTrack>;

    /// Set the current track
    fn set_current_track(&mut self, track: Option<QueueTrack>);

    /// Take the current track (removes it)
    fn take_current_track(&mut self) -> Option<QueueTrack>;

    // ===== Next Track =====

    /// Get next track metadata (for gapless/crossfade pre-loading)
    fn next_track(&self) -> Option<&QueueTrack>;

    /// Set the next track
    fn set_next_track(&mut self, track: Option<QueueTrack>);

    /// Take the next track (removes it)
    fn take_next_track(&mut self) -> Option<QueueTrack>;

    // ===== Queue Modifications =====

    /// Add track to play next (highest priority, LIFO)
    fn add_to_queue_next(&mut self, track: QueueTrack);

    /// Add track to queue end (lowest priority, FIFO)
    fn add_to_queue_end(&mut self, track: QueueTrack);

    /// Remove track from queue by index
    fn remove_from_queue(&mut self, index: usize) -> Result<QueueTrack>;

    /// Reorder track in queue
    fn reorder_queue(&mut self, from: usize, to: usize) -> Result<()>;

    /// Clear specific queue tiers
    fn clear_play_next(&mut self);
    fn clear_queued_later(&mut self);
    fn clear_queue(&mut self);

    // ===== Queue Inspection =====

    /// Get all tracks in priority order
    fn get_queue(&self) -> Vec<&QueueTrack>;

    /// Get queue length
    fn queue_len(&self) -> usize;

    /// Check if queue is empty
    fn is_queue_empty(&self) -> bool;

    /// Get total source queue length (for pagination)
    fn get_source_total(&self) -> usize;

    /// Get current position in source queue
    fn current_position_in_source(&self) -> usize;

    /// Peek at the next track without advancing
    fn peek_next(&self) -> Option<&QueueTrack>;

    /// Peek at the first source track (for repeat all)
    fn peek_first_source_track(&self) -> Option<&QueueTrack>;

    // ===== Playlist Loading =====

    /// Load playlist/album to source queue with start index
    fn load_playlist(&mut self, tracks: Vec<QueueTrack>, start_index: usize);

    /// Load playlist/album to source queue (starts from beginning)
    fn add_playlist_to_queue(&mut self, tracks: Vec<QueueTrack>);

    /// Append tracks to source queue
    fn append_to_queue(&mut self, tracks: Vec<QueueTrack>);

    /// Append tracks to source queue (raw, no shuffle)
    fn append_to_source(&mut self, tracks: Vec<QueueTrack>);

    // ===== History =====

    /// Push track to history
    fn push_history(&mut self, track: QueueTrack);

    /// Pop track from history
    fn pop_history(&mut self) -> Option<QueueTrack>;

    /// Check if history is empty
    fn is_history_empty(&self) -> bool;

    /// Get all history tracks
    fn get_history(&self) -> Vec<&QueueTrack>;

    /// Clear history
    fn clear_history(&mut self);

    /// Check if queue can go back
    fn can_go_back(&self) -> bool;

    /// Go back in queue (decrement source index)
    fn go_back(&mut self);

    // ===== Next Track Logic =====

    /// Get next track from queue considering repeat mode
    fn get_next_track_from_queue(&mut self) -> Result<QueueTrack>;

    /// Skip to track at index in queue (returns skipped tracks)
    fn skip_to_index(&mut self, index: usize) -> Option<Vec<QueueTrack>>;

    /// Peek at the next track that would play (considers repeat)
    fn peek_next_queue_track(&self) -> Option<&QueueTrack>;

    /// Check if there is a next track
    fn has_next(&self) -> bool;

    /// Check if there is a previous track
    fn has_previous(&self) -> bool;

    // ===== Shuffle & Repeat =====

    /// Get current shuffle mode
    fn shuffle(&self) -> ShuffleMode;

    /// Set shuffle mode
    fn set_shuffle(&mut self, mode: ShuffleMode);

    /// Cycle shuffle mode: Off -> Random -> Smart -> Off
    fn cycle_shuffle(&mut self) -> ShuffleMode;

    /// Get current repeat mode
    fn repeat(&self) -> RepeatMode;

    /// Set repeat mode
    fn set_repeat(&mut self, mode: RepeatMode);

    // ===== Lazy Queue Management =====

    /// Set lazy context for on-demand track loading
    fn set_lazy_context(&mut self, context: QueueContext, shuffle_seed: Option<u64>);

    /// Clear lazy context
    fn clear_lazy_context(&mut self);

    /// Get lazy queue state
    fn get_lazy_state(&self) -> Option<&LazyQueueState>;

    /// Check if we need to load the next batch (forward pagination)
    fn check_batch_loading(&mut self) -> Option<(usize, usize)>;

    /// Check if jumping to index requires loading new batch
    fn check_jump_loading(&mut self, target_index: usize) -> Option<(usize, usize)>;

    // ===== Track Transitions =====

    /// Save current track to history and move to next track
    ///
    /// Returns the next track's metadata after it becomes the current track.
    /// Used during transitions (gapless, crossfade completion).
    fn transition_to_next(&mut self) -> Option<QueueTrack>;
}

// =============================================================================
// Audio Processing Trait
// =============================================================================

/// Audio processing operations for playback
///
/// Manages the audio pipeline from source to output:
/// Source -> Crossfade -> Effects -> Volume Leveling -> Output
///
/// # Responsibilities
/// - Audio source lifecycle (current + next for gapless)
/// - Crossfade engine and buffer management
/// - Effects chain (feature-gated)
/// - Loudness normalization (feature-gated)
/// - Headroom management (feature-gated)
/// - Output limiting (feature-gated)
/// - Channel conversion (mono/stereo/multichannel)
///
/// # Performance Requirements
/// - `process_*` methods MUST NOT allocate
/// - All buffers MUST be pre-allocated during initialization
/// - Buffer operations MUST be bounded (no unbounded loops)
///
/// # Example Mock Implementation
/// ```ignore
/// struct MockAudio {
///     current_source: Option<Box<dyn AudioSource>>,
///     crossfade_active: bool,
/// }
///
/// impl AudioProcessing for MockAudio {
///     fn read_source_into_stereo_buffer(&mut self, max_frames: usize) -> Result<Option<usize>> {
///         // Return fake data for testing
///         Ok(Some(max_frames * 2))
///     }
///     // ... other methods with test doubles
/// }
/// ```
pub trait AudioProcessing: Send {
    // ===== Audio Source Management =====

    /// Get reference to current audio source
    fn audio_source(&self) -> Option<&dyn AudioSource>;

    /// Get mutable reference to current audio source
    fn audio_source_mut(&mut self) -> Option<&mut Box<dyn AudioSource>>;

    /// Check if there is a current audio source
    fn has_audio_source(&self) -> bool;

    /// Set audio source directly
    fn set_audio_source(&mut self, source: Option<Box<dyn AudioSource>>);

    /// Take audio source (removes it)
    fn take_audio_source(&mut self) -> Option<Box<dyn AudioSource>>;

    // ===== Source Operations =====

    /// Read from current source into internal stereo conversion buffer
    ///
    /// Returns number of samples read, or None if no source.
    /// This method resolves borrow checker conflicts when reading + processing.
    fn read_source_into_stereo_buffer(&mut self, max_output_frames: usize)
        -> Result<Option<usize>>;

    /// Get source position and duration without holding mutable borrow
    fn source_position_duration(&self) -> Option<(Duration, Duration)>;

    /// Get mutable slice of stereo conversion buffer
    fn stereo_buffer_slice_mut(&mut self, range: std::ops::Range<usize>) -> &mut [f32];

    /// Convert stereo samples to mono (writes to output, returns frames written)
    fn convert_stereo_to_mono(&self, output: &mut [f32], samples_read: usize) -> usize;

    /// Upmix stereo samples to multichannel (writes to output, returns samples written)
    fn upmix_stereo_to_multichannel(&self, output: &mut [f32], samples_read: usize) -> usize;

    // ===== Crossfade =====

    /// Start crossfade transition
    ///
    /// Returns true if crossfade started, false if conditions not met.
    fn start_crossfade(&mut self) -> bool;

    /// Process active crossfade mixing
    ///
    /// Returns (samples_processed, completed).
    fn process_active_crossfade(&mut self, output: &mut [f32]) -> Result<(usize, bool)>;

    /// Move next source to current (for track transition)
    fn transition_sources(&mut self);

    /// Check if crossfade is currently active
    fn is_crossfading(&self) -> bool;

    /// Get crossfade progress (0.0 to 1.0)
    fn get_crossfade_progress(&self) -> f32;

    /// Get crossfade state
    fn get_crossfade_state(&self) -> CrossfadeState;

    /// Check source compatibility for gapless/crossfade
    fn is_source_compatible(&self, source: &dyn AudioSource) -> bool;

    /// Get time remaining until crossfade should start
    fn time_until_crossfade(&self) -> Option<Duration>;

    // ===== Crossfade Settings =====

    /// Set crossfade settings
    fn set_crossfade_settings(&mut self, settings: CrossfadeSettings);

    /// Get current crossfade settings
    fn get_crossfade_settings(&self) -> &CrossfadeSettings;

    /// Enable or disable crossfade
    fn set_crossfade_enabled(&mut self, enabled: bool);

    /// Check if crossfade is enabled
    fn is_crossfade_enabled(&self) -> bool;

    /// Set crossfade duration in milliseconds
    fn set_crossfade_duration(&mut self, duration_ms: u32);

    /// Get crossfade duration in milliseconds
    fn get_crossfade_duration(&self) -> u32;

    /// Set crossfade curve type
    fn set_crossfade_curve(&mut self, curve: FadeCurve);

    /// Get crossfade curve type
    fn get_crossfade_curve(&self) -> FadeCurve;

    /// Set whether crossfade applies on manual skip
    fn set_crossfade_on_skip(&mut self, on_skip: bool);

    // ===== Audio Processing Chain =====

    /// Apply full processing chain (loudness -> headroom -> effects -> volume -> limiter)
    ///
    /// Volume is applied via the provided volume controller.
    /// MUST NOT allocate - all processing done in-place.
    fn apply_processing_chain(&mut self, buffer: &mut [f32], volume: &mut dyn VolumeControl);

    /// Apply processing chain on stereo conversion buffer
    fn apply_processing_chain_on_stereo_buffer(
        &mut self,
        samples_read: usize,
        volume: &mut dyn VolumeControl,
    );

    // ===== Configuration =====

    /// Get sample rate
    fn sample_rate(&self) -> u32;

    /// Set sample rate
    fn set_sample_rate(&mut self, sample_rate: u32);

    /// Get output channels
    fn output_channels(&self) -> u16;

    /// Set output channels
    fn set_output_channels(&mut self, channels: u16);

    /// Get manual skip flag
    fn is_manual_skip(&self) -> bool;

    /// Set manual skip flag
    fn set_manual_skip(&mut self, is_manual: bool);

    /// Check if gapless is enabled
    fn gapless_enabled(&self) -> bool;

    /// Set gapless enabled
    fn set_gapless_enabled(&mut self, enabled: bool);

    // ===== Buffer Management =====

    /// Allocate crossfade buffers based on current config and sample rate
    fn allocate_crossfade_buffers(&mut self);

    /// Free crossfade buffers to save memory
    fn free_crossfade_buffers(&mut self);

    // ===== Effects (feature-gated) =====

    /// Get mutable reference to effect chain
    #[cfg(feature = "effects")]
    fn effect_chain_mut(&mut self) -> &mut EffectChain;

    // ===== Volume Leveling (feature-gated) =====

    #[cfg(feature = "volume-leveling")]
    fn set_volume_leveling_mode(&mut self, mode: NormalizationMode);

    #[cfg(feature = "volume-leveling")]
    fn get_volume_leveling_mode(&self) -> NormalizationMode;

    #[cfg(feature = "volume-leveling")]
    fn set_track_gain(&mut self, gain_db: f64, peak_dbfs: f64);

    #[cfg(feature = "volume-leveling")]
    fn set_album_gain(&mut self, gain_db: f64, peak_dbfs: f64);

    #[cfg(feature = "volume-leveling")]
    fn clear_loudness_gains(&mut self);

    #[cfg(feature = "volume-leveling")]
    fn set_loudness_preamp(&mut self, preamp_db: f64);

    #[cfg(feature = "volume-leveling")]
    fn get_loudness_preamp(&self) -> f64;

    #[cfg(feature = "volume-leveling")]
    fn set_prevent_clipping(&mut self, prevent: bool);

    #[cfg(feature = "volume-leveling")]
    fn get_effective_gain_db(&mut self) -> f64;

    #[cfg(feature = "volume-leveling")]
    fn reset_loudness_normalizer(&mut self);

    // ===== Output Limiter (feature-gated) =====

    #[cfg(feature = "volume-leveling")]
    fn set_output_limiter_lookahead(&mut self, preset: LookaheadPreset);

    #[cfg(feature = "volume-leveling")]
    fn get_output_limiter_lookahead(&self) -> LookaheadPreset;

    #[cfg(feature = "volume-leveling")]
    fn set_output_limiter_lookahead_ms(&mut self, lookahead_ms: f32);

    #[cfg(feature = "volume-leveling")]
    fn set_output_limiter_threshold_db(&mut self, threshold_db: f32);

    #[cfg(feature = "volume-leveling")]
    fn get_output_limiter_gain_reduction_db(&self) -> f32;

    #[cfg(feature = "volume-leveling")]
    fn get_output_limiter_latency(&self) -> usize;

    #[cfg(feature = "volume-leveling")]
    fn reset_output_limiter(&mut self);

    // ===== Headroom Management (feature-gated) =====

    #[cfg(feature = "volume-leveling")]
    fn set_headroom_mode(&mut self, mode: HeadroomMode);

    #[cfg(feature = "volume-leveling")]
    fn get_headroom_mode(&self) -> HeadroomMode;

    #[cfg(feature = "volume-leveling")]
    fn set_headroom_replaygain_db(&mut self, gain_db: f64);

    #[cfg(feature = "volume-leveling")]
    fn set_headroom_preamp_db(&mut self, preamp_db: f64);

    #[cfg(feature = "volume-leveling")]
    fn set_headroom_eq_boost_db(&mut self, boost_db: f64);

    #[cfg(feature = "volume-leveling")]
    fn set_headroom_additional_gain_db(&mut self, gain_db: f64);

    #[cfg(feature = "volume-leveling")]
    fn get_headroom_total_gain_db(&self) -> f64;

    #[cfg(feature = "volume-leveling")]
    fn get_headroom_attenuation_db(&mut self) -> f64;

    #[cfg(feature = "volume-leveling")]
    fn set_headroom_enabled(&mut self, enabled: bool);

    #[cfg(feature = "volume-leveling")]
    fn is_headroom_enabled(&self) -> bool;

    #[cfg(feature = "volume-leveling")]
    fn reset_headroom(&mut self);

    #[cfg(feature = "volume-leveling")]
    fn clear_headroom_track_gains(&mut self);

    // ===== Reset =====

    /// Reset all audio pipeline state
    fn reset_all(&mut self);
}

// =============================================================================
// Volume Control Trait
// =============================================================================

/// Volume control operations for playback
///
/// Manages volume level (0-100) with logarithmic scaling and mute state.
///
/// # Responsibilities
/// - Volume level control (0-100 range)
/// - Logarithmic volume curve for perceptual linearity
/// - Mute/unmute state management
/// - In-place buffer gain application
///
/// # Performance Requirements
/// - `apply()` MUST NOT allocate
/// - `apply()` MUST be optimized for SIMD when possible
/// - Volume curve calculation MUST be cached (no per-sample computation)
///
/// # Example Mock Implementation
/// ```ignore
/// struct MockVolume {
///     level: u8,
///     muted: bool,
/// }
///
/// impl VolumeControl for MockVolume {
///     fn apply(&mut self, buffer: &mut [f32]) {
///         if self.muted {
///             buffer.fill(0.0);
///         } else {
///             let gain = (self.level as f32) / 100.0;
///             for sample in buffer {
///                 *sample *= gain;
///             }
///         }
///     }
///     // ... other methods
/// }
/// ```
pub trait VolumeControl: Send {
    /// Set volume level (0-100)
    fn set_volume(&mut self, level: u8);

    /// Get current volume level (0-100)
    fn get_volume(&self) -> u8;

    /// Mute audio
    fn mute(&mut self);

    /// Unmute audio
    fn unmute(&mut self);

    /// Toggle mute state
    fn toggle_mute(&mut self);

    /// Check if muted
    fn is_muted(&self) -> bool;

    /// Apply volume to audio buffer in-place
    ///
    /// # Performance
    /// MUST NOT allocate. Optimized implementations should use SIMD.
    ///
    /// # Arguments
    /// * `buffer` - Audio samples to apply volume to (modified in-place)
    fn apply(&mut self, buffer: &mut [f32]);
}

// =============================================================================
// State Tracking Trait
// =============================================================================

/// State tracking and event emission for playback
///
/// Manages playback state transitions, pending state changes,
/// event queue, and duplicate suppression.
///
/// # Responsibilities
/// - Playback state lifecycle (Stopped, Loading, Playing, Paused)
/// - Pending state transitions (for fade completion)
/// - User pause flag tracking
/// - Event emission with duplicate suppression
/// - Event queue overflow protection
/// - Crossfade progress tracking
///
/// # Event System Design
/// - Events are queued until drained (pull-based)
/// - Duplicate suppression prevents event spam
/// - Overflow protection prevents unbounded growth
/// - Throttling for high-frequency events (position, crossfade progress)
///
/// # Example Mock Implementation
/// ```ignore
/// struct MockState {
///     state: PlaybackState,
///     events: Vec<PlaybackEvent>,
/// }
///
/// impl StateTracking for MockState {
///     fn emit_state_changed(&mut self, state: PlaybackState) {
///         self.events.push(PlaybackEvent::StateChanged { state: state.into() });
///     }
///
///     fn drain_events(&mut self) -> Vec<PlaybackEvent> {
///         std::mem::take(&mut self.events)
///     }
///     // ... other methods
/// }
/// ```
pub trait StateTracking: Send {
    // ===== State Management =====

    /// Get current playback state
    fn state(&self) -> PlaybackState;

    /// Set playback state directly
    fn set_state(&mut self, state: PlaybackState);

    /// Get pending state transition
    fn pending_state(&self) -> Option<PlaybackState>;

    /// Set pending state transition
    fn set_pending_state(&mut self, state: Option<PlaybackState>);

    /// Check if user has explicitly paused
    fn user_paused(&self) -> bool;

    /// Set user paused flag
    fn set_user_paused(&mut self, paused: bool);

    // ===== Event Emission =====

    /// Push an event to the pending events queue
    fn push_event(&mut self, event: PlaybackEvent);

    /// Drain all pending events
    ///
    /// Returns all events that have been emitted since the last drain.
    fn drain_events(&mut self) -> Vec<PlaybackEvent>;

    /// Check if there are pending events
    fn has_pending_events(&self) -> bool;

    /// Emit a state changed event (with duplicate suppression)
    fn emit_state_changed(&mut self, state: PlaybackState);

    /// Emit a track changed event (zero-allocation: uses Arc)
    fn emit_track_changed(&mut self, track_id: Arc<str>, previous_track_id: Option<Arc<str>>);

    /// Emit a crossfade started event (zero-allocation: uses Arc)
    fn emit_crossfade_started(
        &mut self,
        from_track_id: Arc<str>,
        to_track_id: Arc<str>,
        duration_ms: u32,
    );

    /// Emit a crossfade progress event (with throttling)
    fn emit_crossfade_progress(&mut self, progress: f32, metadata_switched: bool);

    /// Emit a crossfade completed event
    fn emit_crossfade_completed(&mut self);

    /// Emit a track finished event
    fn emit_track_finished(&mut self, track_id: String);

    /// Emit a volume changed event
    fn emit_volume_changed(&mut self, level: u8, is_muted: bool);

    /// Emit a queue changed event
    fn emit_queue_changed(&mut self, length: usize);

    /// Emit an error event
    fn emit_error(&mut self, message: String);

    /// Emit a next track prepared event
    fn emit_next_track_prepared(&mut self, track_id: String);

    // ===== Position Tracking =====

    /// Get position update sample counter
    fn position_update_samples(&self) -> usize;

    /// Set position update sample counter
    fn set_position_update_samples(&mut self, samples: usize);

    /// Add samples to position update counter
    fn add_position_update_samples(&mut self, samples: usize);

    // ===== Crossfade Progress =====

    /// Check if crossfade metadata has been switched
    fn crossfade_metadata_switched(&self) -> bool;

    /// Set crossfade metadata switched flag
    fn set_crossfade_metadata_switched(&mut self, switched: bool);

    /// Reset crossfade progress tracking for new crossfade
    fn reset_crossfade_progress_tracking(&mut self);

    // ===== Reset =====

    /// Reset all event suppression state
    ///
    /// Called during stop() to ensure next playback emits all events.
    fn reset_event_suppression(&mut self);
}

// =============================================================================
// Fade Management Trait
// =============================================================================

/// Fade envelope management for click-free playback
///
/// Manages start/stop fade envelopes, pending source activation,
/// buffer underrun noise, and source readiness tracking.
///
/// # Responsibilities
/// - Start fade-in envelope (playback start/resume)
/// - Stop fade-out envelope (pause/stop transitions)
/// - Pending source activation (smooth transitions)
/// - Buffer underrun noise (DAC keep-alive)
/// - Source readiness verification (prevents clicks)
///
/// # Fade Types
/// - **Start fade**: Quick fade-in (30ms) to prevent clicks at playback start
/// - **Stop fade**: Smooth fade-out (100ms) before pause/stop
///
/// # Source Readiness
/// Sources with background decoding need time to buffer audio before playback.
/// This component tracks when a source is "ready" to prevent stuttering/clicks.
///
/// # Example Mock Implementation
/// ```ignore
/// struct MockFade {
///     start_active: bool,
///     stop_active: bool,
///     source_ready: bool,
/// }
///
/// impl FadeManagement for MockFade {
///     fn process_start_fade(&mut self, buffer: &mut [f32]) {
///         if self.start_active {
///             // Apply simple linear fade for testing
///             for (i, sample) in buffer.iter_mut().enumerate() {
///                 let gain = (i as f32) / (buffer.len() as f32);
///                 *sample *= gain;
///             }
///             self.start_active = false;
///         }
///     }
///     // ... other methods
/// }
/// ```
pub trait FadeManagement: Send {
    // ===== Start Fade =====

    /// Start a fade-in envelope
    ///
    /// # Arguments
    ///
    /// * `preserve_dc_state` - If true, preserves DC blocker state (for resume from pause).
    ///   If false, resets DC blocker state (for fresh start).
    fn start_fade_in(&mut self, preserve_dc_state: bool);

    /// Check if start fade is active
    fn is_start_fade_active(&self) -> bool;

    /// Freeze start fade at current position (for pause during fade-in)
    fn freeze_start_fade(&mut self);

    /// Reset start fade
    fn reset_start_fade(&mut self);

    /// Process start fade envelope on buffer
    ///
    /// MUST NOT allocate. Modifies buffer in-place.
    fn process_start_fade(&mut self, buffer: &mut [f32]);

    // ===== Stop Fade =====

    /// Start a fade-out with the given completion action
    fn start_fade_out(&mut self, action: FadeCompleteAction);

    /// Check if stop fade is active
    fn is_stop_fade_active(&self) -> bool;

    /// Reset stop fade
    fn reset_stop_fade(&mut self);

    /// Process stop fade envelope on buffer
    ///
    /// Returns the completion action if fade just completed.
    /// MUST NOT allocate. Modifies buffer in-place.
    fn process_stop_fade(&mut self, buffer: &mut [f32]) -> Option<FadeCompleteAction>;

    // ===== Pending Source =====

    /// Set a pending source (for smooth transitions)
    fn set_pending_source(&mut self, source: Box<dyn AudioSource>);

    /// Take the pending source (returns and clears it)
    fn take_pending_source(&mut self) -> Option<Box<dyn AudioSource>>;

    /// Check if there is a pending source
    fn has_pending_source(&self) -> bool;

    /// Clear pending source
    fn clear_pending_source(&mut self);

    // ===== Source Readiness =====

    /// Check if source readiness has been verified
    fn source_ready_verified(&self) -> bool;

    /// Set source readiness state
    fn set_source_ready_verified(&mut self, verified: bool);

    /// Get number of samples waited for source readiness
    fn source_ready_wait_samples(&self) -> usize;

    /// Add samples to wait counter
    fn add_source_ready_wait_samples(&mut self, samples: usize);

    /// Reset source readiness tracking for a new track load
    fn reset_source_tracking(&mut self);

    // ===== Underrun Handling =====

    /// Fill buffer with DAC keep-alive noise
    ///
    /// When buffer underrun occurs, fills buffer with -96dB noise
    /// to prevent DAC power-save mode pops.
    /// MUST NOT allocate. Modifies buffer in-place.
    fn fill_underrun_buffer(&mut self, buffer: &mut [f32]);

    // ===== Configuration =====

    /// Update sample rate for fade envelopes
    fn set_sample_rate(&mut self, sample_rate: u32);

    // ===== Reset =====

    /// Full reset of all fade state
    fn reset_all(&mut self);
}

// =============================================================================
// Benefits of Trait-Based Architecture
// =============================================================================
//
// 1. Mock Implementations for Testing
//
// Traits enable lightweight mock implementations that avoid I/O and provide
// deterministic behavior for unit tests.
//
// ```ignore
// struct MockQueue {
//     tracks: VecDeque<QueueTrack>,
// }
//
// impl QueueOperations for MockQueue {
//     fn next_track(&mut self) -> Result<QueueTrack> {
//         self.tracks.pop_front().ok_or(PlaybackError::QueueEmpty)
//     }
//     // No file I/O, no shuffle complexity, just pure logic
// }
//
// #[test]
// fn test_queue_exhaustion() {
//     let mut queue = MockQueue { tracks: VecDeque::new() };
//     assert!(queue.next_track().is_err()); // Deterministic test
// }
// ```
//
// 2. Runtime Component Swapping
//
// Swap implementations at runtime based on use case:
//
// ```ignore
// let queue: Box<dyn QueueOperations> = if cfg!(test) {
//     Box::new(MockQueue::new())
// } else {
//     Box::new(QueueManager::new(100, shuffle, repeat))
// };
// ```
//
// ## 3. Alternative Implementations
//
// Create specialized implementations for different scenarios:
//
// ```ignore
// // Lightweight for tests
// struct TestQueue { /* minimal state */ }
//
// // Full-featured for production
// struct ProductionQueue { /* complete implementation */ }
//
// // Memory-constrained for embedded
// struct EmbeddedQueue { /* optimized for size */ }
//
// // All implement QueueOperations
// ```
//
// ## 4. Clear Contracts and Boundaries
//
// Traits define explicit contracts between components:
// - What each component is responsible for
// - What operations are guaranteed to be available
// - What state is accessible vs. internal
//
// This prevents tight coupling and makes refactoring safer.
//
// ## 5. Conditional Compilation (Features)
//
// Traits enable clean feature-gated implementations:
//
// ```ignore
// #[cfg(feature = "advanced-queue")]
// impl QueueOperations for AdvancedQueue { /* ... */ }
//
// #[cfg(not(feature = "advanced-queue"))]
// impl QueueOperations for BasicQueue { /* ... */ }
// ```
//
// ## 6. Dependency Injection
//
// Traits enable constructor injection for better testability:
//
// ```ignore
// struct PlaybackCoordinator {
//     queue: Box<dyn QueueOperations>,
//     audio: Box<dyn AudioProcessing>,
//     // ...
// }
//
// impl PlaybackCoordinator {
//     pub fn new(
//         queue: Box<dyn QueueOperations>,
//         audio: Box<dyn AudioProcessing>,
//         // ...
//     ) -> Self {
//         Self { queue, audio, /* ... */ }
//     }
// }
// ```
//
// ## 7. Isolated Unit Testing
//
// Test components in complete isolation without initializing entire system:
//
// ```ignore
// #[test]
// fn test_volume_application() {
//     let mut volume = MockVolume::new(50, false);
//     let mut buffer = vec![1.0; 100];
//     volume.apply(&mut buffer);
//     assert_eq!(buffer[0], 0.5); // Isolated test, no audio I/O
// }
// ```
//
// ## 8. Performance Benchmarking
//
// Benchmark individual components without full system overhead:
//
// ```ignore
// #[bench]
// fn bench_crossfade_processing(b: &mut Bencher) {
//     let mut audio = MockAudio::new();
//     let mut output = vec![0.0; 8192];
//     b.iter(|| {
//         audio.process_active_crossfade(&mut output).unwrap();
//     });
// }
// ```
//
// ## 9. Documentation and Discoverability
//
// Traits serve as live documentation of component capabilities.
// IDEs can show all available operations on a component via trait methods.
//
// ## 10. Future-Proofing
//
// Adding new implementations is easy without modifying existing code:
// - New queue strategies (priority queue, weighted shuffle, etc.)
// - New audio processors (spectral effects, ML-based normalization, etc.)
// - New volume curves (exponential, S-curve, etc.)
//
// All while maintaining backward compatibility with existing consumers.
