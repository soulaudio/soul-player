//! Playback manager - core orchestration
//!
//! Coordinates queue, history, volume, shuffle, and audio processing

use crate::{
    crossfade::{CrossfadeEngine, CrossfadeSettings, CrossfadeState, FadeCurve},
    error::{PlaybackError, Result},
    events::{CrossfadeProgressTracker, PlaybackEvent},
    fade_envelopes::{
        FadeCompleteAction, StartFadeEnvelope, StopFadeEnvelope, DAC_KEEPALIVE_NOISE,
    },
    history::History,
    lazy_queue::LazyQueueState,
    queue::Queue,
    shuffle::shuffle_queue,
    source::AudioSource,
    types::{PlaybackConfig, PlaybackState, QueueTrack, RepeatMode, ShuffleMode},
    volume::Volume,
};

#[cfg(feature = "effects")]
use soul_audio::effects::EffectChain;

#[cfg(feature = "volume-leveling")]
use soul_loudness::{
    headroom::{HeadroomManager, HeadroomMode},
    LookaheadPreset, LoudnessNormalizer, NormalizationMode, TruePeakLimiter,
};

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

/// Central playback management
///
/// Orchestrates all playback functionality:
/// - Queue management (two-tier: explicit + source)
/// - History tracking (for "previous" button)
/// - Volume control (logarithmic, 0-100%)
/// - Shuffle modes (Off, Random, Smart)
/// - Repeat modes (Off, All, One)
/// - Audio effects processing
/// - Gapless playback support
#[allow(clippy::struct_excessive_bools)]
pub struct PlaybackManager {
    // State
    state: PlaybackState,
    /// Pending state transition waiting for audio callback to acknowledge
    /// Used to defer state changes until fades complete for deterministic behavior
    pending_state: Option<PlaybackState>,
    /// Flag indicating user explicitly paused playback
    /// Prevents set_audio_source() from overriding pause when audio loads late
    user_paused: bool,
    current_track: Option<QueueTrack>,

    // Queue and history
    queue: Queue,
    history: History,

    // Lazy queue state for on-demand loading
    lazy_state: Option<LazyQueueState>,

    // Settings
    volume: Volume,
    shuffle: ShuffleMode,
    repeat: RepeatMode,
    gapless_enabled: bool,

    // Audio processing
    #[cfg(feature = "effects")]
    effect_chain: EffectChain,
    #[cfg(feature = "volume-leveling")]
    loudness_normalizer: LoudnessNormalizer,
    #[cfg(feature = "volume-leveling")]
    headroom_manager: HeadroomManager,
    #[cfg(feature = "volume-leveling")]
    output_limiter: TruePeakLimiter,
    audio_source: Option<Box<dyn AudioSource>>,
    next_source: Option<Box<dyn AudioSource>>, // For gapless/crossfade
    next_track: Option<QueueTrack>,            // Metadata for next track

    // Crossfade engine
    crossfade: CrossfadeEngine,

    // Lazily-allocated buffers for crossfade (allocated on first use, freed when disabled)
    // This saves ~14.6MB of memory when crossfade is disabled
    outgoing_buffer: Option<Vec<f32>>,
    incoming_buffer: Option<Vec<f32>>,

    // Pre-allocated buffer for stereo conversion (mono/multichannel output)
    // Avoids heap allocation in audio callback - see CLAUDE.md rule #4
    stereo_conversion_buffer: Vec<f32>,

    // Sample rate (for effects processing)
    sample_rate: u32,

    // Output channels (1 = mono, 2 = stereo)
    output_channels: u16,

    // Track if we're in a manual skip (for crossfade on_skip setting)
    is_manual_skip: bool,

    // Event queue for UI synchronization
    pending_events: Vec<PlaybackEvent>,

    // Crossfade progress tracker for 50% metadata switch
    crossfade_progress: CrossfadeProgressTracker,

    // Start fade envelope for click-free playback start/resume
    start_fade: StartFadeEnvelope,

    // Stop fade envelope for click-free playback stop/transitions
    stop_fade: StopFadeEnvelope,

    // Pending source to be set after stop fade completes
    // This prevents race conditions during source transitions
    pending_source: Option<Box<dyn AudioSource>>,

    // Noise state for buffer underrun handling (DAC keep-alive)
    underrun_noise_state: u32,

    // Flag to track if source readiness has been verified for current track
    // When false, we wait for source.is_ready() before starting actual playback
    // This prevents clicks from playing a not-yet-buffered source
    source_ready_verified: bool,

    // Count of samples we've waited for source to become ready
    // Used for logging/debugging startup issues
    source_ready_wait_samples: usize,

    // Counter for throttled position updates
    // Accumulates samples processed until threshold reached
    position_update_samples: usize,

    // Last emitted state for duplicate suppression
    // Prevents flooding the event queue with redundant state events
    last_emitted_state: Option<PlaybackState>,

    // Last emitted crossfade progress for throttling
    // Only emit when progress changes by at least 5%
    last_emitted_crossfade_progress: f32,
}

/// Default buffer size for crossfade (10 seconds at max supported sample rate 192kHz stereo)
/// This ensures crossfade works correctly at all sample rates up to 192kHz
const CROSSFADE_BUFFER_SIZE: usize = 10 * 192000 * 2;

/// Number of samples between position update events (~250ms at 48kHz stereo)
const POSITION_UPDATE_SAMPLE_THRESHOLD: usize = 48000 / 4 * 2;

/// Maximum stereo buffer size for channel conversion (8192 frames * 2 channels)
/// This covers typical audio callback buffer sizes (256-4096 frames)
const MAX_STEREO_BUFFER_SIZE: usize = 8192 * 2;

/// Maximum number of pending events before overflow handling kicks in
/// Prevents unbounded memory growth if events aren't drained
const MAX_PENDING_EVENTS: usize = 1000;

/// Number of oldest events to drop when overflow occurs
const EVENT_OVERFLOW_DROP_COUNT: usize = 100;

impl PlaybackManager {
    /// Create new playback manager
    pub fn new(config: PlaybackConfig) -> Self {
        // Configure loudness normalizer to NOT use internal limiter
        // We use a separate output_limiter at the end of the chain
        #[cfg(feature = "volume-leveling")]
        let mut loudness_normalizer = LoudnessNormalizer::new(44100, 2);
        #[cfg(feature = "volume-leveling")]
        loudness_normalizer.set_use_internal_limiter(false);

        Self {
            state: PlaybackState::Stopped,
            pending_state: None,
            user_paused: false,
            current_track: None,
            queue: Queue::new(),
            history: History::new(config.history_size),
            lazy_state: None,
            volume: Volume::new(config.volume),
            shuffle: config.shuffle,
            repeat: config.repeat,
            gapless_enabled: config.gapless,
            #[cfg(feature = "effects")]
            effect_chain: EffectChain::new(),
            #[cfg(feature = "volume-leveling")]
            loudness_normalizer,
            #[cfg(feature = "volume-leveling")]
            headroom_manager: HeadroomManager::new(),
            #[cfg(feature = "volume-leveling")]
            output_limiter: TruePeakLimiter::new(44100, 2),
            audio_source: None,
            next_source: None,
            next_track: None,
            crossfade: CrossfadeEngine::with_settings(config.crossfade),
            outgoing_buffer: None,
            incoming_buffer: None,
            stereo_conversion_buffer: vec![0.0; MAX_STEREO_BUFFER_SIZE],
            sample_rate: 44100, // Default, will be updated by platform
            output_channels: 2, // Default stereo, will be updated by platform
            is_manual_skip: false,
            pending_events: Vec::with_capacity(64), // Pre-allocate to avoid early reallocations
            crossfade_progress: CrossfadeProgressTracker::new(),
            start_fade: StartFadeEnvelope::new(44100), // Will be updated by set_sample_rate
            stop_fade: StopFadeEnvelope::new(44100),   // Will be updated by set_sample_rate
            pending_source: None,
            underrun_noise_state: 0xDEAD_BEEF, // Seed for LFSR noise generator
            source_ready_verified: false,
            source_ready_wait_samples: 0,
            position_update_samples: 0,
            last_emitted_state: None,
            last_emitted_crossfade_progress: -1.0, // Sentinel: any valid progress (0-1) triggers first emit
        }
    }

    /// Fill buffer with DAC keep-alive noise to prevent power-save mode pops
    ///
    /// When buffer underrun occurs, we need to output SOMETHING to keep the DAC
    /// active. Pure zeros can cause some DACs to enter power-save mode, which
    /// creates an audible pop when audio resumes.
    ///
    /// This fills the buffer with -96dB noise (inaudible) that keeps the DAC active.
    #[inline]
    fn fill_underrun_buffer(&mut self, buffer: &mut [f32]) {
        for sample in buffer.iter_mut() {
            // Simple LFSR for pseudo-random noise
            self.underrun_noise_state ^= self.underrun_noise_state << 13;
            self.underrun_noise_state ^= self.underrun_noise_state >> 17;
            self.underrun_noise_state ^= self.underrun_noise_state << 5;

            // Convert to bipolar noise in range [-1, 1] then scale to keep-alive level
            *sample =
                ((self.underrun_noise_state & 0xFFFF) as f32 / 32768.0 - 1.0) * DAC_KEEPALIVE_NOISE;
        }
    }

    /// Handle the action when a stop fade completes
    fn handle_fade_complete_action(&mut self, action: FadeCompleteAction) -> Result<()> {
        match action {
            FadeCompleteAction::None => Ok(()),
            FadeCompleteAction::Stop => {
                self.state = PlaybackState::Stopped;
                self.audio_source = None;
                self.pending_state = None;
                self.emit_state_changed(PlaybackState::Stopped);
                Ok(())
            }
            FadeCompleteAction::Pause => {
                // Apply the pending state and emit event NOW (after fade completes)
                self.state = PlaybackState::Paused;
                self.pending_state = None;
                self.emit_state_changed(PlaybackState::Paused);
                Ok(())
            }
            FadeCompleteAction::TransitionToNext => {
                // Transition handled by pending_source mechanism
                // Just clear the old source
                self.audio_source = None;
                self.pending_state = None;
                Ok(())
            }
        }
    }

    // ===== Playback Control =====

    /// Start or resume playback
    pub fn play(&mut self) -> Result<()> {
        match self.state {
            PlaybackState::Paused => {
                // Clear pause flag when user explicitly resumes
                self.user_paused = false;

                // Cancel any active stop fade that hasn't completed yet
                // This prevents the fade from finishing and reverting state to Paused
                self.stop_fade.reset();
                self.pending_state = None;

                // Resume from pause
                self.state = PlaybackState::Playing;

                // Only start fade if source is ready
                // If source not ready yet (paused during startup), let the normal
                // startup logic in process_audio() handle the fade after ready check
                if self.source_ready_verified {
                    self.start_fade.start();
                }

                self.emit_state_changed(PlaybackState::Playing);
                Ok(())
            }
            PlaybackState::Stopped | PlaybackState::Loading => {
                // Clear pause flag on new playback start
                self.user_paused = false;

                // Start playing from queue
                self.play_next_in_queue()
            }
            PlaybackState::Playing => {
                // Already playing
                Ok(())
            }
        }
    }

    /// Pause playback
    pub fn pause(&mut self) {
        tracing::debug!(
            "[pause] Called: state={:?}, source_ready={}, has_source={}",
            self.state,
            self.source_ready_verified,
            self.audio_source.is_some()
        );

        // Can pause from Playing OR Loading states
        // Loading state happens when user clicks pause during track load
        if self.state == PlaybackState::Playing || self.state == PlaybackState::Loading {
            // Set user pause flag FIRST to prevent set_audio_source() from overriding
            self.user_paused = true;

            // CRITICAL: Freeze start_fade at current gain to prevent volume spike
            // When pause is clicked during fade-in, freezing prevents gain from continuing to increase
            // Both frozen start_fade and active stop_fade will multiply together smoothly
            if self.start_fade.is_active() {
                self.start_fade.freeze();
                tracing::info!("[pause] Froze start_fade at current position");
            } else {
                tracing::info!("[pause] start_fade not active, no freeze needed");
            }

            // Reset wait counter if paused during loading
            // This prevents timeout from carrying over when resuming
            if !self.source_ready_verified {
                self.source_ready_wait_samples = 0;
                tracing::debug!("[pause] Reset wait counter (source not ready yet)");
            }

            // Start smooth fade-out before pausing
            // Fade whenever we have an audio source (even if not verified)
            // This prevents pops when pausing after seek or during load
            if self.audio_source.is_some() && !self.stop_fade.is_active() {
                self.stop_fade.start(FadeCompleteAction::Pause);
                // Defer state change until fade completes (sample-accurate)
                self.pending_state = Some(PlaybackState::Paused);
                tracing::debug!(
                    "[pause] Started fade-out (source_ready={}), state change deferred",
                    self.source_ready_verified
                );
            } else {
                // No audio source or fade already active, change state immediately
                self.state = PlaybackState::Paused;
                self.emit_state_changed(PlaybackState::Paused);
                tracing::debug!("[pause] State changed to Paused (no fade needed)");
            }
        } else {
            tracing::debug!("[pause] Ignored (state is {:?})", self.state);
        }
    }

    /// Stop playback
    ///
    /// Stops playback and clears current track (but not queue).
    /// Uses smooth fade-out to prevent clicks.
    pub fn stop(&mut self) {
        // CRITICAL: Cancel any active stop_fade to prevent it from completing later
        // This is important for play_queue flow (stop -> load -> play)
        // Without this, the old fade can complete and override the new playback state
        self.stop_fade.reset();

        self.state = PlaybackState::Stopped;
        self.current_track = None;
        // Audio source cleared immediately since we're force-stopping
        self.audio_source = None;
        self.next_source = None;
        self.next_track = None;
        self.pending_source = None;
        self.pending_state = None; // Clear any pending state transition
        self.user_paused = false; // Clear pause flag when explicitly stopping
        self.crossfade.reset();
        self.crossfade_progress.reset();
        // Free crossfade buffers to reclaim ~15MB of memory
        self.free_crossfade_buffers();
        self.is_manual_skip = false;
        // Reset source ready state for clean next playback
        self.source_ready_verified = false;
        self.source_ready_wait_samples = 0;
        // Reset fade envelopes
        self.start_fade.reset();
        // Reset position update counter
        self.position_update_samples = 0;
        // Reset event suppression state to ensure next playback emits all events
        self.last_emitted_state = None;
        self.last_emitted_crossfade_progress = -1.0; // Sentinel: any valid progress triggers first emit
        self.emit_state_changed(PlaybackState::Stopped);
    }

    /// Reset source readiness tracking for a new track load
    ///
    /// Called when transitioning to a new track to ensure the source readiness
    /// check is performed again. This prevents playing an unbuffered source
    /// which would cause clicks/underruns at playback start.
    fn reset_position_tracking(&mut self) {
        self.source_ready_verified = false;
        self.source_ready_wait_samples = 0;
    }

    /// Skip to next track
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Result<()> {
        self.is_manual_skip = true;

        // Cancel any active stop_fade to prevent race conditions
        self.stop_fade.reset();
        self.pending_state = None;

        // CRITICAL: Clear user_paused flag when manually skipping
        // User expects next() to START the next track, not stay paused
        self.user_paused = false;

        // Save current track to history (if any)
        if let Some(track) = self.current_track.take() {
            self.history.push(track);
        }

        self.play_next_in_queue()
    }

    /// Go to previous track
    ///
    /// If >3 seconds into current track, restarts current track.
    /// Otherwise, uses index-based navigation to go back without reordering the queue.
    pub fn previous(&mut self) -> Result<()> {
        // Cancel any active stop_fade to prevent race conditions
        self.stop_fade.reset();
        self.pending_state = None;

        // CRITICAL: Clear user_paused flag when manually going to previous track
        // User expects previous() to START the previous track, not stay paused
        self.user_paused = false;

        // Check position in current track
        if let Some(ref source) = self.audio_source {
            if source.position() > Duration::from_secs(3) {
                // Restart current track
                if let Some(ref mut src) = self.audio_source {
                    src.reset()?;
                    // Start fade-in for click-free restart
                    self.start_fade.start();
                }
                return Ok(());
            }
        }

        // Go to previous track from history
        if let Some(prev_track) = self.history.pop() {
            // IMPORTANT: Don't add current track back to queue!
            // The queue uses index-based navigation, so the track is still there.
            // We just need to decrement the source_index to "un-consume" it.
            if self.current_track.is_some() {
                // Decrement source index to restore queue position
                // This keeps the queue order intact
                if self.queue.can_go_back() {
                    self.queue.go_back();
                }
            }

            // Load previous track
            self.current_track = Some(prev_track);
            self.state = PlaybackState::Loading;
            self.reset_position_tracking();
            // Platform will need to call load_current_track()
            Ok(())
        } else {
            // No history, restart current track
            if let Some(ref mut source) = self.audio_source {
                source.reset()?;
                // Start fade-in for click-free restart
                self.start_fade.start();
            }
            Ok(())
        }
    }

    /// Internal: Play next track from queue
    fn play_next_in_queue(&mut self) -> Result<()> {
        // Handle repeat one
        if self.repeat == RepeatMode::One && self.current_track.is_some() {
            // Restart current track
            if let Some(ref mut source) = self.audio_source {
                source.reset()?;
                // Start fade-in for click-free restart
                self.start_fade.start();
                self.state = PlaybackState::Playing;
                return Ok(());
            }
        }

        // CRITICAL: Check if we need to load next batch BEFORE trying to get next track
        // Otherwise, if the track doesn't exist yet, we return QueueEmpty before batch loading!
        if let Some((offset, limit)) = self.check_batch_loading() {
            tracing::info!(
                offset = offset,
                limit = limit,
                "[PlaybackManager] Forward pagination triggered"
            );
            self.pending_events
                .push(PlaybackEvent::BatchLoadRequested { offset, limit });

            // Set loading state and wait for batch to arrive
            // The batch handler will call play_next_in_queue again after loading
            self.state = PlaybackState::Loading;
            return Ok(());
        }

        // Get next track from queue
        let next_track = self.get_next_track_from_queue()?;

        // Save current track to history
        if let Some(track) = self.current_track.take() {
            self.history.push(track);
        }

        // Load next track
        self.current_track = Some(next_track);
        self.state = PlaybackState::Loading;
        self.reset_position_tracking();
        // Platform will need to call load_current_track()

        Ok(())
    }

    /// Get next track considering repeat mode
    fn get_next_track_from_queue(&mut self) -> Result<QueueTrack> {
        // If starting playback (no history), skip play_next queue
        // Play Next tracks should play AFTER the first track, not instead of it
        let track = if self.history.is_empty() {
            self.queue.pop_next_skip_play_next()
        } else {
            self.queue.pop_next()
        };

        if let Some(track) = track {
            return Ok(track);
        }

        // Queue reached end - check repeat mode
        match self.repeat {
            RepeatMode::All => {
                // Reload source queue from original and try again
                self.queue.reload_source(self.shuffle);

                // Try to get the first track from reloaded queue
                self.queue.pop_next().ok_or(PlaybackError::QueueEmpty)
            }
            RepeatMode::Off | RepeatMode::One => Err(PlaybackError::QueueEmpty),
        }
    }

    // ===== Seek =====

    /// Seek to position in current track (by duration)
    pub fn seek_to(&mut self, position: Duration) -> Result<()> {
        // Guard: Cannot seek while Loading (source may not be fully initialized)
        // Allow seeking only in Playing or Paused states
        if self.state == PlaybackState::Loading || self.state == PlaybackState::Stopped {
            return Err(PlaybackError::NoTrackLoaded);
        }

        // CRITICAL: If crossfade is active, cancel it before seeking
        // Seeking during crossfade would cause stale mixing state and audio glitches
        // Note: This must be done before borrowing audio_source to avoid borrow conflicts
        if self.crossfade.is_active() {
            tracing::info!("[PLAYBACK] Cancelling active crossfade due to seek operation");
            self.crossfade.reset();
            self.crossfade_progress.reset();
            // Clear preloaded next track since we're staying on current track
            self.next_source = None;
            self.next_track = None;
            // Free crossfade buffers to reclaim ~15MB of memory
            self.free_crossfade_buffers();
        }

        // Cancel any active stop fade to prevent race conditions
        // (e.g., seeking during fade-out should cancel the fade)
        if self.stop_fade.is_active() {
            tracing::debug!("[seek_to] Cancelling active stop fade due to seek");
            self.stop_fade.reset();
            self.pending_state = None;
        }

        if let Some(ref mut source) = self.audio_source {
            // Clamp position to avoid seeking exactly to end (which would trigger EOF)
            // Leave 1ms margin before duration to ensure we can still read samples.
            let duration = source.duration();
            let max_seek_position = duration.saturating_sub(Duration::from_millis(1));
            let clamped_position = position.min(max_seek_position);

            // Log if we clamped the position (only for near-end seeks)
            if clamped_position != position && position > Duration::ZERO {
                tracing::debug!(
                    "[seek_to] Clamped seek near end: {:?} -> {:?} (duration: {:?})",
                    position,
                    clamped_position,
                    duration
                );
            }

            source.seek(clamped_position)?;

            // CRITICAL: Mark source as not ready after seek
            // This prevents the audio callback from reading 0 samples and thinking track finished
            // We wait for source.is_ready() before continuing playback (same as track load)
            self.source_ready_verified = false;
            self.source_ready_wait_samples = 0;

            // NOTE: Fade-in is started in process_audio() after source_ready_verified
            // becomes true, not here. This ensures the fade doesn't begin until the
            // source has buffered enough samples for glitch-free playback.

            Ok(())
        } else {
            Err(PlaybackError::NoTrackLoaded)
        }
    }

    /// Seek to position in current track (by percentage)
    pub fn seek_to_percent(&mut self, percent: f32) -> Result<()> {
        let percent = percent.clamp(0.0, 1.0);

        if let Some(ref source) = self.audio_source {
            let duration = source.duration();
            let position = duration.mul_f32(percent);
            self.seek_to(position)
        } else {
            Err(PlaybackError::NoTrackLoaded)
        }
    }

    // ===== Volume =====

    /// Set volume (0-100)
    pub fn set_volume(&mut self, level: u8) {
        self.volume.set_level(level);
    }

    /// Get current volume level (0-100)
    pub fn get_volume(&self) -> u8 {
        self.volume.level()
    }

    /// Mute audio
    pub fn mute(&mut self) {
        self.volume.mute();
    }

    /// Unmute audio
    pub fn unmute(&mut self) {
        self.volume.unmute();
    }

    /// Toggle mute state
    pub fn toggle_mute(&mut self) {
        self.volume.toggle_mute();
    }

    /// Check if muted
    pub fn is_muted(&self) -> bool {
        self.volume.is_muted()
    }

    // ===== Queue Management =====

    /// Add track to play next (top of explicit queue)
    pub fn add_to_queue_next(&mut self, track: QueueTrack) {
        self.queue.add_next(track);
    }

    /// Add track to end of explicit queue
    pub fn add_to_queue_end(&mut self, track: QueueTrack) {
        self.queue.add_to_end(track);
    }

    /// Clear Play Next queue only
    pub fn clear_play_next(&mut self) {
        self.queue.clear_play_next();
    }

    /// Clear Add to Queue only
    pub fn clear_queued_later(&mut self) {
        self.queue.clear_queued_later();
    }

    /// Clear Add to Queue only (alias for clear_queued_later)
    pub fn clear_add_to_queue(&mut self) {
        self.queue.clear_queued_later();
    }

    /// Cycle shuffle mode: Off → Random → Smart → Off
    ///
    /// Returns the new shuffle mode after cycling.
    pub fn cycle_shuffle(&mut self) -> ShuffleMode {
        let new_mode = self.shuffle.cycle();
        self.set_shuffle(new_mode);
        new_mode
    }

    /// Get current shuffle mode
    pub fn get_shuffle_mode(&self) -> ShuffleMode {
        self.shuffle
    }

    /// Load playlist/album to source queue with start index
    ///
    /// Replaces the entire queue and clears history for a fresh start.
    /// Starts playback from the specified index (Bug #1 fix).
    pub fn load_playlist(&mut self, mut tracks: Vec<QueueTrack>, start_index: usize) {
        // Apply shuffle if enabled
        if self.shuffle != ShuffleMode::Off {
            shuffle_queue(&mut tracks, self.shuffle);
        }

        self.queue.set_source(tracks);

        // Remove consecutive duplicates to prevent same track playing twice (Bug #5 fix)
        self.queue.remove_consecutive_duplicates();

        // Skip to start index if specified (Bug #1 fix)
        if start_index > 0
            && start_index < self.queue.get_source_total()
            && self.queue.skip_to_index(start_index).is_none()
        {
            tracing::warn!(
                start_index = start_index,
                queue_size = self.queue.get_source_total(),
                "[PlaybackManager] Failed to skip to start index - index may be out of bounds"
            );
        }

        // IMPORTANT: Clear history when loading a new playlist
        // This ensures navigation starts fresh without old history interfering
        self.history.clear();
    }

    /// Load playlist/album to source queue
    ///
    /// Replaces the entire queue and clears history for a fresh start.
    /// This ensures clicking a track in the playlist starts from scratch.
    pub fn add_playlist_to_queue(&mut self, mut tracks: Vec<QueueTrack>) {
        // Apply shuffle if enabled
        if self.shuffle != ShuffleMode::Off {
            shuffle_queue(&mut tracks, self.shuffle);
        }

        self.queue.set_source(tracks);

        // Remove consecutive duplicates to prevent same track playing twice
        self.queue.remove_consecutive_duplicates();

        // IMPORTANT: Clear history when loading a new playlist
        // This ensures navigation starts fresh without old history interfering
        self.history.clear();
    }

    /// Append tracks to source queue
    pub fn append_to_queue(&mut self, mut tracks: Vec<QueueTrack>) {
        // Apply shuffle if enabled
        if self.shuffle != ShuffleMode::Off {
            shuffle_queue(&mut tracks, self.shuffle);
        }

        self.queue.append_to_source(tracks);

        // Remove consecutive duplicates to prevent same track playing twice
        self.queue.remove_consecutive_duplicates();
    }

    /// Append tracks to source queue without shuffling (for lazy loading)
    ///
    /// Unlike `append_to_queue()`, this does NOT apply shuffle or remove duplicates.
    /// Used for lazy loading where tracks are already in the correct order (seed-based shuffle).
    pub fn append_to_source(&mut self, tracks: Vec<QueueTrack>) {
        self.queue.append_to_source(tracks);
    }

    // ===== Lazy Queue Management =====

    /// Set lazy context for on-demand track loading
    ///
    /// This enables automatic batch loading for large collections.
    /// When the queue approaches the end of loaded tracks, the system
    /// will emit events to trigger loading the next batch.
    pub fn set_lazy_context(
        &mut self,
        context: crate::lazy_queue::QueueContext,
        shuffle_seed: Option<u64>,
    ) {
        use crate::lazy_queue::{LazyQueueState, DEFAULT_WINDOW_SIZE};

        let mut state = LazyQueueState::new(context, 0);
        state.shuffle_seed = shuffle_seed;
        state.window_end = DEFAULT_WINDOW_SIZE; // Initial batch loaded

        self.lazy_state = Some(state);
    }

    /// Clear lazy context (disable lazy loading)
    pub fn clear_lazy_context(&mut self) {
        self.lazy_state = None;
    }

    /// Get lazy queue state (for batch loading)
    pub fn get_lazy_state(&self) -> Option<&LazyQueueState> {
        self.lazy_state.as_ref()
    }

    /// Check if we need to load the next batch (forward pagination)
    ///
    /// Returns Some((offset, limit)) if batch loading is needed, None otherwise.
    pub fn check_batch_loading(&mut self) -> Option<(usize, usize)> {
        if let Some(ref mut lazy_state) = self.lazy_state {
            let current_pos = self.queue.current_position_in_source();

            if lazy_state.should_load_next_batch(current_pos) {
                let (offset, limit) = lazy_state.next_batch_range();

                // Update window boundaries
                lazy_state.extend_window(limit);

                return Some((offset, limit));
            }
        }
        None
    }

    /// Check if jumping to index requires loading new batch
    ///
    /// Returns Some((offset, limit)) for batch containing target index, None if already loaded.
    pub fn check_jump_loading(&mut self, target_index: usize) -> Option<(usize, usize)> {
        use crate::lazy_queue::DEFAULT_WINDOW_SIZE;

        if let Some(ref mut lazy_state) = self.lazy_state {
            // If target is beyond current window, load batch containing it
            if target_index >= lazy_state.window_end {
                tracing::info!(
                    target_index = target_index,
                    window_end = lazy_state.window_end,
                    "[PlaybackManager] Jump beyond window"
                );

                // Calculate which batch contains target_index
                let batch_number = target_index / DEFAULT_WINDOW_SIZE;
                let offset = batch_number * DEFAULT_WINDOW_SIZE;
                let limit = DEFAULT_WINDOW_SIZE;

                // Update window to new position
                lazy_state.window_start = offset;
                lazy_state.window_end = offset + limit;

                return Some((offset, limit));
            }
            // Also trigger forward pagination if jumping near end of window
            else if lazy_state.should_load_next_batch(target_index) {
                tracing::info!(
                    target_index = target_index,
                    window_end = lazy_state.window_end,
                    "[PlaybackManager] Jump near window end, triggering forward pagination"
                );

                let (offset, limit) = lazy_state.next_batch_range();
                lazy_state.extend_window(limit);

                return Some((offset, limit));
            }
        }
        None
    }

    /// Remove track from queue by index
    pub fn remove_from_queue(&mut self, index: usize) -> Result<QueueTrack> {
        self.queue
            .remove(index)
            .ok_or(PlaybackError::IndexOutOfBounds(index))
    }

    /// Reorder track in queue
    pub fn reorder_queue(&mut self, from: usize, to: usize) -> Result<()> {
        self.queue
            .reorder(from, to)
            .map_err(PlaybackError::InvalidOperation)
    }

    /// Clear entire queue
    pub fn clear_queue(&mut self) {
        self.queue.clear();
    }

    /// Get all tracks in queue
    pub fn get_queue(&self) -> Vec<&QueueTrack> {
        self.queue.get_all()
    }

    /// Get queue length
    pub fn queue_len(&self) -> usize {
        self.queue.len()
    }

    /// Skip to track at index in queue
    ///
    /// Skips to the track at the specified index. Only the currently playing track
    /// (if any) is added to history - skipped-over tracks are NOT added since they
    /// were never actually played.
    pub fn skip_to_queue_index(&mut self, index: usize) -> Result<()> {
        // Check if jumping to index requires loading new batch
        if let Some((offset, limit)) = self.check_jump_loading(index) {
            self.pending_events
                .push(PlaybackEvent::JumpLoadRequested { offset, limit });
            // Set loading state while batch is fetched
            self.state = PlaybackState::Loading;

            // IMPORTANT: Return early - wait for batch to load
            // The batch handler will call skip_to_queue_index again after loading
            return Ok(());
        }

        if index >= self.queue.len() {
            return Err(PlaybackError::QueueEmpty);
        }

        // CRITICAL: Clear user_paused flag - user explicitly selected a track to play
        // This ensures clicking a queue item in paused state starts playback
        self.user_paused = false;

        // Reset any active fades
        self.stop_fade.reset();
        self.pending_state = None;

        // Save current track to history (if any) - only actually-played tracks
        if let Some(track) = self.current_track.take() {
            self.history.push(track);
        }

        // Skip to target index - we intentionally discard the skipped tracks
        // because they were never played and shouldn't appear in history
        let _skipped_tracks = self
            .queue
            .skip_to_index(index)
            .ok_or(PlaybackError::QueueEmpty)?;

        // Play the next track (now at index 0)
        self.play_next_in_queue()
    }

    // ===== Shuffle & Repeat =====

    /// Set shuffle mode
    ///
    /// When toggling shuffle during playback:
    /// - Turning ON: Only shuffles remaining (unplayed) tracks, preserving current position
    /// - Turning OFF: Restores original order, keeping current track's position in original order
    pub fn set_shuffle(&mut self, mode: ShuffleMode) {
        if self.shuffle == mode {
            return;
        }

        let old_mode = self.shuffle;
        self.shuffle = mode;

        match mode {
            ShuffleMode::Off => {
                // Restore original order while preserving current track position
                self.queue.restore_original_order();
                tracing::debug!(
                    "[set_shuffle] Restored original order, new source_index: {}",
                    self.queue.current_source_index()
                );
            }
            ShuffleMode::Random | ShuffleMode::Smart => {
                if old_mode == ShuffleMode::Off {
                    // First time enabling shuffle - use the new method that preserves position
                    self.queue.apply_shuffle(mode);
                } else {
                    // Switching between Random and Smart - reshuffle remaining tracks
                    self.queue.apply_shuffle(mode);
                }

                // Remove consecutive duplicates after shuffling
                self.queue.remove_consecutive_duplicates();

                tracing::debug!(
                    "[set_shuffle] Applied shuffle mode {:?}, source_index: {}",
                    mode,
                    self.queue.current_source_index()
                );
            }
        }
    }

    /// Get current shuffle mode
    pub fn get_shuffle(&self) -> ShuffleMode {
        self.shuffle
    }

    /// Set repeat mode
    ///
    /// If a crossfade is active and mode changes to RepeatOne, the crossfade
    /// is cancelled since RepeatOne will repeat the current track instead of
    /// transitioning to the next track.
    pub fn set_repeat(&mut self, mode: RepeatMode) {
        let old_mode = self.repeat;
        self.repeat = mode;

        // If switching to RepeatOne during an active crossfade, cancel it
        // RepeatOne means we should repeat the current track, not transition to next
        if mode == RepeatMode::One && old_mode != RepeatMode::One && self.crossfade.is_active() {
            tracing::info!("[PLAYBACK] Cancelling active crossfade due to RepeatOne mode change");
            self.crossfade.reset();
            self.crossfade_progress.reset();
            // Clear the preloaded next track since we'll repeat current track
            self.next_source = None;
            self.next_track = None;
            // Free crossfade buffers to reclaim ~15MB of memory
            self.free_crossfade_buffers();
        }
    }

    /// Get current repeat mode
    pub fn get_repeat(&self) -> RepeatMode {
        self.repeat
    }

    // ===== State Queries =====

    /// Get current playback state
    pub fn get_state(&self) -> PlaybackState {
        self.state
    }

    /// Get currently playing track
    pub fn get_current_track(&self) -> Option<&QueueTrack> {
        self.current_track.as_ref()
    }

    /// Get current playback position
    ///
    /// During crossfade, returns the incoming track's position to avoid
    /// a jarring position jump when the transition completes.
    pub fn get_position(&self) -> Duration {
        // During crossfade, report incoming track position
        if self.crossfade.is_active() {
            if let Some(ref next_source) = self.next_source {
                return next_source.position();
            }
        }

        // Normal playback - report current source position
        self.audio_source
            .as_ref()
            .map(|s| s.position())
            .unwrap_or(Duration::ZERO)
    }

    /// Get current track duration
    ///
    /// During crossfade, returns the incoming track's duration to match
    /// the position reporting.
    pub fn get_duration(&self) -> Option<Duration> {
        // During crossfade, report incoming track duration
        if self.crossfade.is_active() {
            if let Some(ref next_source) = self.next_source {
                return Some(next_source.duration());
            }
        }

        // Normal playback
        self.audio_source.as_ref().map(|s| s.duration())
    }

    /// Get playback history
    pub fn get_history(&self) -> Vec<&QueueTrack> {
        self.history.get_all()
    }

    /// Get total queue length
    pub fn get_queue_length(&self) -> usize {
        self.queue.len()
    }

    /// Check if there is a next track
    pub fn has_next(&self) -> bool {
        // Queue has tracks
        if !self.queue.is_empty() {
            return true;
        }

        // Repeat One always has next (same track)
        if self.repeat == RepeatMode::One {
            return true;
        }

        // Repeat All has next if source queue exists (Bug #7 fix)
        if self.repeat == RepeatMode::All && self.queue.get_source_total() > 0 {
            return true;
        }

        false
    }

    /// Check if there is a previous track
    pub fn has_previous(&self) -> bool {
        !self.history.get_all().is_empty() || self.repeat == RepeatMode::One
    }

    /// Peek at the next track in queue without advancing
    ///
    /// Returns the next track that would play when current track finishes.
    /// Used by platform code to pre-load the next track for crossfade/gapless.
    ///
    /// NOTE: For RepeatAll mode when queue is exhausted, this returns the first
    /// track from the original source (the track that would play after reload).
    /// This enables pre-loading for seamless loop transitions.
    pub fn peek_next_queue_track(&self) -> Option<&QueueTrack> {
        // If repeat one is enabled, return current track
        if self.repeat == RepeatMode::One {
            return self.current_track.as_ref();
        }

        // Otherwise peek at the queue
        if let Some(track) = self.queue.peek_next() {
            Some(track)
        } else if self.repeat == RepeatMode::All && self.queue.get_source_total() > 0 {
            // Queue exhausted but repeat all is enabled - would loop back to first track
            // Return the first track from original source for pre-loading
            // NOTE: We peek at source[0] directly since after reload that's what plays
            self.queue.peek_first_source_track()
        } else {
            None
        }
    }

    // ===== Audio Processing =====

    /// Process audio samples for output
    ///
    /// Called by platform audio callback. Applies effects and volume.
    /// Returns number of samples written to output buffer.
    ///
    /// # Arguments
    /// * `output` - Output buffer (interleaved, channel count matches output_channels)
    ///
    /// # Returns
    /// Number of samples written (0 = no audio available)
    pub fn process_audio(&mut self, output: &mut [f32]) -> Result<usize> {
        // Debug logging (first few calls only)
        static CALL_COUNT: AtomicUsize = AtomicUsize::new(0);
        let count = CALL_COUNT.fetch_add(1, Ordering::Relaxed);
        if count < 3 {
            tracing::debug!("[process_audio] Call #{}", count + 1);
            tracing::debug!("  - Output buffer size: {} samples", output.len());
            tracing::debug!("  - Output channels: {}", self.output_channels);
            tracing::debug!(
                "  - Expected frames: {}",
                output.len() / self.output_channels as usize
            );
            tracing::debug!("  - Sample rate: {} Hz", self.sample_rate);
        }

        // === PHASE 1: Handle stop fade and pending source activation ===
        // Check if we have a pending source to activate after stop fade
        if self.pending_source.is_some() && !self.stop_fade.is_active() {
            // Stop fade completed (or wasn't needed), activate pending source
            self.audio_source = self.pending_source.take();
            self.state = PlaybackState::Playing;
            // Don't start fade yet - wait for source to be ready first
            self.source_ready_verified = false;
            self.source_ready_wait_samples = 0;
            tracing::debug!("[process_audio] Activated pending source, waiting for ready");
        }

        // === PHASE 1.5: Universal fade processing (CRITICAL FIX) ===
        // Process stop_fade REGARDLESS of state to ensure immediate response to pause/stop commands
        // This fixes the race condition where pause() is called while state is still Playing
        if self.stop_fade.is_active() {
            if let Some(ref mut source) = self.audio_source {
                // Read audio and apply fades
                let samples_read = source.read_samples(output)?;
                if samples_read > 0 {
                    // CRITICAL: Apply start_fade FIRST if active/frozen
                    // When pause is clicked during fade-in, frozen start_fade maintains constant gain
                    // Then stop_fade multiplies on top to create smooth fade-out
                    if self.start_fade.is_active() {
                        self.start_fade.process(&mut output[..samples_read]);
                    }

                    // Then apply stop_fade on top
                    if let Some(action) = self.stop_fade.process(&mut output[..samples_read]) {
                        tracing::info!(
                            "[process_audio] stop_fade completed, transitioning to paused state"
                        );
                        self.handle_fade_complete_action(action)?;
                    }

                    // CRITICAL: Apply the same processing chain as normal playback
                    // This prevents volume jumps when transitioning to/from stop_fade

                    // Apply loudness normalization (gain only, no internal limiter)
                    #[cfg(feature = "volume-leveling")]
                    self.loudness_normalizer
                        .process(&mut output[..samples_read]);

                    // Apply headroom attenuation BEFORE effects to prevent clipping in DSP chain
                    #[cfg(feature = "volume-leveling")]
                    self.headroom_manager.process(&mut output[..samples_read]);

                    // Apply effects (if feature enabled)
                    #[cfg(feature = "effects")]
                    self.effect_chain
                        .process(&mut output[..samples_read], self.sample_rate);

                    // Apply volume
                    self.volume.apply(&mut output[..samples_read]);

                    // Apply output limiter AFTER volume to catch ALL peaks
                    #[cfg(feature = "volume-leveling")]
                    self.output_limiter.process(&mut output[..samples_read]);
                }
                // Fill remainder with keepalive noise
                if samples_read < output.len() {
                    self.fill_underrun_buffer(&mut output[samples_read..]);
                }
                return Ok(output.len());
            }
        }

        // === PHASE 2: State-based processing ===
        match self.state {
            PlaybackState::Stopped => {
                // Stopped: output DAC keepalive noise (not raw silence)
                self.fill_underrun_buffer(output);
                return Ok(output.len());
            }
            PlaybackState::Paused => {
                // Paused: output keepalive noise (stop_fade handled in Phase 1.5)
                self.fill_underrun_buffer(output);
                return Ok(output.len());
            }
            PlaybackState::Loading => {
                // Loading: output keepalive noise while waiting (stop_fade handled in Phase 1.5)
                self.fill_underrun_buffer(output);
                return Ok(output.len());
            }
            PlaybackState::Playing => {
                // Fall through to normal processing below
            }
        }

        // === PHASE 2.5: Source readiness check (only for new sources) ===
        // Wait for source to report ready before starting actual playback
        // This prevents clicks from playing a not-yet-buffered source
        if !self.source_ready_verified {
            if let Some(ref source) = self.audio_source {
                if source.is_ready() {
                    // Source is ready - start the fade and proceed
                    self.source_ready_verified = true;
                    self.start_fade.start();
                    let wait_ms =
                        (self.source_ready_wait_samples as f64 / self.sample_rate as f64) * 1000.0;
                    tracing::debug!(
                        "[process_audio] Source ready after {} samples ({:.1}ms wait), starting playback",
                        self.source_ready_wait_samples, wait_ms
                    );
                } else {
                    // Source not ready yet - output keepalive noise and wait
                    self.source_ready_wait_samples += output.len();

                    // Log periodically (every ~500ms worth of samples)
                    let log_interval = self.sample_rate as usize; // ~1 second
                    if self.source_ready_wait_samples % log_interval < output.len() {
                        let wait_ms = (self.source_ready_wait_samples as f64
                            / self.sample_rate as f64)
                            * 1000.0;
                        tracing::debug!(
                            "[process_audio] Waiting for source ready... ({:.0}ms elapsed)",
                            wait_ms
                        );
                    }

                    // Timeout after 2 seconds - proceed anyway with warning
                    let timeout_samples = self.sample_rate as usize * 2; // 2 seconds
                    if self.source_ready_wait_samples >= timeout_samples {
                        tracing::warn!(
                            "[process_audio] Source ready timeout after 2 seconds, proceeding anyway"
                        );
                        self.source_ready_verified = true;
                        self.start_fade.start();
                    } else {
                        self.fill_underrun_buffer(output);
                        return Ok(output.len());
                    }
                }
            }
        }

        // === PHASE 3: Normal playback processing (state == Playing) ===
        let Some(ref mut source) = self.audio_source else {
            // No audio source - output keepalive noise instead of raw silence
            self.fill_underrun_buffer(output);
            return Ok(output.len());
        };

        // Audio source always outputs stereo (2 channels)
        // If device is mono, we need to convert
        if self.output_channels == 1 {
            // Mono output - read stereo, convert to mono
            // Use pre-allocated buffer to avoid heap allocation in audio callback
            let stereo_samples = (output.len() * 2).min(self.stereo_conversion_buffer.len());

            let samples_read =
                source.read_samples(&mut self.stereo_conversion_buffer[..stereo_samples])?;

            if samples_read == 0 {
                // CRITICAL: Check if track actually finished or just buffering
                // After seek, source may return 0 samples temporarily while buffering
                let position = source.position();
                let duration = source.duration();
                if position >= duration {
                    // Track actually finished - position at or past duration
                    self.handle_track_finished()?;
                    return Ok(0);
                }
                // Still buffering after seek - output keepalive noise and continue
                tracing::debug!(
                    "[process_audio] Mono: Source returned 0 samples but pos={:?} < dur={:?}, buffering",
                    position, duration
                );
                self.fill_underrun_buffer(output);
                return Ok(output.len());
            }

            // Apply start fade envelope for click-free playback start/resume
            // This must come BEFORE any other processing
            self.start_fade
                .process(&mut self.stereo_conversion_buffer[..samples_read]);

            // Apply loudness normalization to stereo buffer (before channel conversion)
            #[cfg(feature = "volume-leveling")]
            self.loudness_normalizer
                .process(&mut self.stereo_conversion_buffer[..samples_read]);

            // Apply headroom attenuation BEFORE effects to prevent clipping in DSP chain
            #[cfg(feature = "volume-leveling")]
            self.headroom_manager
                .process(&mut self.stereo_conversion_buffer[..samples_read]);

            // Convert stereo to mono by averaging L and R channels
            let frames = samples_read / 2;
            for (i, out_sample) in output.iter_mut().enumerate().take(frames) {
                let left = self.stereo_conversion_buffer[i * 2];
                let right = self.stereo_conversion_buffer[i * 2 + 1];
                *out_sample = (left + right) * 0.5; // Average and write to mono output
            }

            // Apply effects (if feature enabled)
            #[cfg(feature = "effects")]
            self.effect_chain
                .process(&mut output[..frames], self.sample_rate);

            // Apply volume
            self.volume.apply(&mut output[..frames]);

            // Apply output limiter AFTER volume to catch ALL peaks
            #[cfg(feature = "volume-leveling")]
            self.output_limiter.process(&mut output[..frames]);

            // Handle buffer underrun: fill remainder with DAC keep-alive noise
            if frames < output.len() {
                self.fill_underrun_buffer(&mut output[frames..]);
            }

            Ok(frames)
        } else if self.output_channels == 2 {
            // Stereo output - with crossfade support
            let samples_read = self.process_stereo_with_crossfade(output)?;

            if samples_read == 0 {
                // CRITICAL: Check if track actually finished or just buffering
                // process_stereo_with_crossfade should handle this, but double-check here
                if let Some(ref source) = self.audio_source {
                    let position = source.position();
                    let duration = source.duration();
                    if position >= duration {
                        // Track actually finished
                        self.handle_track_finished()?;
                        return Ok(0);
                    }
                    // Still buffering - this shouldn't happen but handle it gracefully
                    tracing::warn!(
                        "[process_audio] Stereo: Unexpected 0 samples with pos={:?} < dur={:?}",
                        position,
                        duration
                    );
                    self.fill_underrun_buffer(output);
                    return Ok(output.len());
                }
                // No source, track actually finished
                self.handle_track_finished()?;
                return Ok(0);
            }

            // Apply start fade envelope for click-free playback start/resume
            // Only apply when NOT crossfading (crossfade has its own fade curves)
            if !self.crossfade.is_active() {
                self.start_fade.process(&mut output[..samples_read]);
            }

            // Apply loudness normalization (gain only, no internal limiter)
            #[cfg(feature = "volume-leveling")]
            self.loudness_normalizer
                .process(&mut output[..samples_read]);

            // Apply headroom attenuation BEFORE effects to prevent clipping in DSP chain
            #[cfg(feature = "volume-leveling")]
            self.headroom_manager.process(&mut output[..samples_read]);

            // Apply effects (if feature enabled)
            #[cfg(feature = "effects")]
            self.effect_chain
                .process(&mut output[..samples_read], self.sample_rate);

            // Apply volume
            self.volume.apply(&mut output[..samples_read]);

            // Apply output limiter AFTER volume to catch ALL peaks
            // This is the correct DSP chain order for preventing clipping
            #[cfg(feature = "volume-leveling")]
            self.output_limiter.process(&mut output[..samples_read]);

            // Handle buffer underrun: fill remainder with DAC keep-alive noise
            // This prevents DAC power-save mode pops when audio resumes
            if samples_read < output.len() {
                self.fill_underrun_buffer(&mut output[samples_read..]);
            }

            Ok(samples_read)
        } else {
            // Multi-channel output (e.g., ASIO with 6 channels)
            // Read stereo, then upmix to fill all output channels
            // Use pre-allocated buffer to avoid heap allocation in audio callback
            let frames = output.len() / self.output_channels as usize;
            let stereo_samples = (frames * 2).min(self.stereo_conversion_buffer.len());

            let samples_read =
                source.read_samples(&mut self.stereo_conversion_buffer[..stereo_samples])?;

            if samples_read == 0 {
                // Track finished
                self.handle_track_finished()?;
                return Ok(0);
            }

            let frames_read = samples_read / 2;

            // Apply start fade envelope for click-free playback start/resume
            // This must come BEFORE any other processing
            self.start_fade
                .process(&mut self.stereo_conversion_buffer[..samples_read]);

            // Apply loudness normalization to stereo buffer
            #[cfg(feature = "volume-leveling")]
            self.loudness_normalizer
                .process(&mut self.stereo_conversion_buffer[..samples_read]);

            // Apply headroom attenuation BEFORE effects to prevent clipping in DSP chain
            #[cfg(feature = "volume-leveling")]
            self.headroom_manager
                .process(&mut self.stereo_conversion_buffer[..samples_read]);

            // Apply effects to stereo buffer (if feature enabled)
            #[cfg(feature = "effects")]
            self.effect_chain.process(
                &mut self.stereo_conversion_buffer[..samples_read],
                self.sample_rate,
            );

            // Apply volume to stereo buffer
            self.volume
                .apply(&mut self.stereo_conversion_buffer[..samples_read]);

            // Apply output limiter AFTER volume to catch ALL peaks
            #[cfg(feature = "volume-leveling")]
            self.output_limiter
                .process(&mut self.stereo_conversion_buffer[..samples_read]);

            // Upmix stereo to multi-channel: put L/R in first two channels, silence in rest
            for frame in 0..frames_read {
                let left = self.stereo_conversion_buffer[frame * 2];
                let right = self.stereo_conversion_buffer[frame * 2 + 1];
                let out_offset = frame * self.output_channels as usize;

                // First two channels get stereo audio
                output[out_offset] = left;
                output[out_offset + 1] = right;

                // Remaining channels get silence
                for ch in 2..self.output_channels as usize {
                    output[out_offset + ch] = 0.0;
                }
            }

            // Handle buffer underrun: fill remainder with DAC keep-alive noise
            let samples_written = frames_read * self.output_channels as usize;
            if samples_written < output.len() {
                self.fill_underrun_buffer(&mut output[samples_written..]);
            }

            Ok(samples_written)
        }
    }

    /// Process stereo audio with crossfade support
    ///
    /// Handles:
    /// - Normal playback (no crossfade)
    /// - Crossfade initiation (when approaching end of track)
    /// - Crossfade mixing (when active)
    /// - Gapless transition (0ms crossfade)
    fn process_stereo_with_crossfade(&mut self, output: &mut [f32]) -> Result<usize> {
        // Check if crossfade is currently active
        if self.crossfade.is_active() {
            return self.process_active_crossfade(output);
        }

        // Get position and duration without holding borrow across decision points
        // This allows us to call &mut self methods for crossfade setup
        let (position, duration) = {
            let source = self
                .audio_source
                .as_ref()
                .ok_or(PlaybackError::NoTrackLoaded)?;
            (source.position(), source.duration())
        };

        // Check if we're approaching the crossfade window
        let crossfade_duration_ms = self.crossfade.settings().duration_ms;
        let crossfade_duration = Duration::from_millis(crossfade_duration_ms as u64);
        let remaining = duration.saturating_sub(position);

        // Should we start crossfade?
        // NOTE: RepeatOne mode should NOT crossfade because:
        // 1. It would crossfade the same track to itself (sounds weird)
        // 2. RepeatOne should seamlessly restart from the beginning
        let should_crossfade = self.crossfade.settings().enabled
            && self.next_source.is_some()
            && remaining <= crossfade_duration
            && self.repeat != RepeatMode::One;

        if should_crossfade {
            // CRITICAL: Ensure buffers are allocated BEFORE starting crossfade
            // This moves the allocation to the transition point (once per crossfade)
            // rather than inside the hot audio processing loop
            self.ensure_crossfade_buffers_allocated();

            // Start crossfade
            let started = self.crossfade.start(self.is_manual_skip);
            if started {
                // Initialize crossfade progress tracker
                let from_track_id = self
                    .current_track
                    .as_ref()
                    .map(|t| t.id.clone())
                    .unwrap_or_default();
                let to_track_id = self
                    .next_track
                    .as_ref()
                    .map(|t| t.id.clone())
                    .unwrap_or_default();

                self.crossfade_progress.start(
                    from_track_id.clone(),
                    to_track_id.clone(),
                    crossfade_duration_ms,
                );
                // Reset progress tracking for new crossfade
                // Use sentinel (-1.0) so first progress (0.0) always emits
                self.last_emitted_crossfade_progress = -1.0;
                self.emit_crossfade_started(from_track_id, to_track_id, crossfade_duration_ms);

                return self.process_active_crossfade(output);
            }
        }

        // Check for gapless transition (crossfade disabled but gapless enabled)
        let should_gapless = !self.crossfade.settings().enabled
            && self.gapless_enabled
            && self.next_source.is_some();

        // Normal playback - now get mutable reference for reading samples
        let source = self
            .audio_source
            .as_mut()
            .ok_or(PlaybackError::NoTrackLoaded)?;
        let samples_read = source.read_samples(output)?;

        if samples_read == 0 {
            // CRITICAL: Check if track actually finished or just buffering
            // After seek, source may return 0 samples temporarily while buffering
            if position >= duration {
                // Track actually finished - position at or past duration
                if should_gapless {
                    // CRITICAL: Verify next source is ready before gapless transition
                    // This prevents audio glitches from reading an unbuffered source
                    let next_ready = self
                        .next_source
                        .as_ref()
                        .map(|s| s.is_ready())
                        .unwrap_or(false);

                    if !next_ready {
                        // Next source not ready yet - output keepalive noise and wait
                        // This rare case can happen with slow storage or large files
                        tracing::debug!(
                            "[process_stereo_with_crossfade] Gapless: next source not ready, waiting"
                        );
                        self.fill_underrun_buffer(output);
                        return Ok(output.len());
                    }

                    // Seamless transition to next track
                    self.transition_to_next_track()?;
                    // Try to read from new source
                    if let Some(ref mut new_source) = self.audio_source {
                        return new_source.read_samples(output);
                    }
                }
                return Ok(0);
            }
            // Still buffering after seek - output keepalive noise and continue
            tracing::debug!(
                "[process_stereo_with_crossfade] Source returned 0 samples but pos={:?} < dur={:?}, buffering",
                position, duration
            );
            // Fill with keepalive noise and return length to continue
            self.fill_underrun_buffer(output);
            return Ok(output.len());
        }

        Ok(samples_read)
    }

    /// Process audio during active crossfade
    ///
    /// IMPORTANT: Buffers MUST be allocated before calling this function.
    /// Call `ensure_crossfade_buffers_allocated()` when starting crossfade,
    /// NOT inside this hot loop to avoid latency-inducing allocations.
    fn process_active_crossfade(&mut self, output: &mut [f32]) -> Result<usize> {
        let buffer_len = output.len();

        // Get mutable references to the buffers (guaranteed to exist - allocated at crossfade start)
        let outgoing_buffer = self
            .outgoing_buffer
            .as_mut()
            .expect("Crossfade buffers must be allocated before calling process_active_crossfade");
        let incoming_buffer = self
            .incoming_buffer
            .as_mut()
            .expect("Crossfade buffers must be allocated before calling process_active_crossfade");

        // Read from outgoing (current) track
        let outgoing_samples = if let Some(ref mut source) = self.audio_source {
            let len = buffer_len.min(outgoing_buffer.len());
            match source.read_samples(&mut outgoing_buffer[..len]) {
                Ok(samples) => samples,
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "[Crossfade] Failed to read from outgoing track - using silence"
                    );
                    outgoing_buffer[..buffer_len].fill(0.0);
                    0
                }
            }
        } else {
            // Fill with silence if no outgoing source
            outgoing_buffer[..buffer_len].fill(0.0);
            buffer_len
        };

        // Read from incoming (next) track
        let incoming_samples = if let Some(ref mut source) = self.next_source {
            let len = buffer_len.min(incoming_buffer.len());
            match source.read_samples(&mut incoming_buffer[..len]) {
                Ok(samples) => samples,
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "[Crossfade] Failed to read from incoming track - using silence"
                    );
                    incoming_buffer[..buffer_len].fill(0.0);
                    0
                }
            }
        } else {
            // Fill with silence if no incoming source
            incoming_buffer[..buffer_len].fill(0.0);
            buffer_len
        };

        // Use the minimum of available samples
        let samples_to_process = outgoing_samples.min(incoming_samples).min(buffer_len);

        if samples_to_process == 0 {
            // Both sources exhausted
            self.crossfade.reset();
            self.crossfade_progress.reset();
            return Ok(0);
        }

        // Process crossfade mixing
        let (processed, completed) = self.crossfade.process(
            &outgoing_buffer[..samples_to_process],
            &incoming_buffer[..samples_to_process],
            &mut output[..samples_to_process],
        );

        // Update crossfade progress and check for metadata switch
        let progress = self.crossfade.progress();
        let should_switch_metadata = self.crossfade_progress.update(progress);

        // Emit TrackChanged at 50% crossfade (metadata switch point)
        if should_switch_metadata {
            if let (Some(from_id), Some(to_id)) = (
                self.crossfade_progress.from_track_id().map(String::from),
                self.crossfade_progress.to_track_id().map(String::from),
            ) {
                self.emit_track_changed(to_id, Some(from_id));
            }
        }

        // Emit crossfade progress event
        self.emit_crossfade_progress(progress, self.crossfade_progress.metadata_switched());

        if completed {
            // Crossfade completed - transition to next track
            self.transition_to_next_track()?;
            self.crossfade.reset();
            self.crossfade_progress.reset();
            self.emit_crossfade_completed();

            // Free crossfade buffers to reclaim ~15MB of memory
            // Buffers will be re-allocated lazily on next crossfade
            self.free_crossfade_buffers();
        }

        Ok(processed)
    }

    /// Transition from current track to next track
    ///
    /// For gapless playback (non-crossfade), this also starts a brief start fade
    /// to prevent clicks from amplitude discontinuities between tracks.
    fn transition_to_next_track(&mut self) -> Result<()> {
        // Get track IDs before moving
        let previous_track_id = self.current_track.as_ref().map(|t| t.id.clone());
        let next_track_id = self.next_track.as_ref().map(|t| t.id.clone());

        // Save current track to history
        if let Some(track) = self.current_track.take() {
            self.history.push(track);
        }

        // Move next source to current
        self.audio_source = self.next_source.take();
        self.current_track = self.next_track.take();
        self.is_manual_skip = false;
        self.reset_position_tracking();

        // CRITICAL: For gapless transitions (non-crossfade), apply a start fade
        // to prevent clicks from amplitude discontinuities between tracks.
        // This handles cases where tracks don't end/start at zero amplitude.
        // Crossfade handles its own smooth transition, so we skip this for crossfade.
        if !self.crossfade_progress.is_active() {
            // Source was pre-loaded for gapless, mark as ready
            self.source_ready_verified = true;
            self.source_ready_wait_samples = 0;
            // Start fade-in to smooth the transition
            self.start_fade.start();
            tracing::debug!(
                "[transition_to_next_track] Gapless transition: starting fade-in for new track"
            );
        }

        // Emit track changed for gapless (non-crossfade) transitions
        // Note: For crossfade, TrackChanged is emitted at 50% in process_active_crossfade
        if !self.crossfade_progress.is_active() {
            if let Some(track_id) = next_track_id {
                self.emit_track_changed(track_id, previous_track_id);
            }
        }

        // Reset loudness normalizer for new track
        // Note: Platform layer should call set_track_gain()/set_album_gain()
        // before the next audio callback to avoid brief volume discrepancy.
        #[cfg(feature = "volume-leveling")]
        self.loudness_normalizer.reset();

        Ok(())
    }

    /// Handle track finished
    fn handle_track_finished(&mut self) -> Result<()> {
        self.is_manual_skip = false;

        // Emit track finished event
        if let Some(ref track) = self.current_track {
            self.emit_track_finished(track.id.clone());
        }

        // Auto-advance to next track
        self.next()
    }

    /// Set sample rate (called by platform)
    pub fn set_sample_rate(&mut self, sample_rate: u32) {
        self.sample_rate = sample_rate;
        self.crossfade.set_sample_rate(sample_rate);
        self.start_fade.set_sample_rate(sample_rate);
        self.stop_fade.set_sample_rate(sample_rate);

        // Update effect chain sample rate for correct filter frequencies
        #[cfg(feature = "effects")]
        self.effect_chain.set_sample_rate(sample_rate);
    }

    /// Get sample rate
    pub fn get_sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Set output channels (called by platform)
    pub fn set_output_channels(&mut self, channels: u16) {
        self.output_channels = channels;
    }

    /// Get effect chain (for adding/configuring effects)
    #[cfg(feature = "effects")]
    pub fn effect_chain_mut(&mut self) -> &mut EffectChain {
        &mut self.effect_chain
    }

    // ===== Volume Leveling =====

    /// Set volume leveling mode (ReplayGain track/album, EBU R128, etc.)
    #[cfg(feature = "volume-leveling")]
    pub fn set_volume_leveling_mode(&mut self, mode: NormalizationMode) {
        self.loudness_normalizer.set_mode(mode);
    }

    /// Get current volume leveling mode
    #[cfg(feature = "volume-leveling")]
    pub fn get_volume_leveling_mode(&self) -> NormalizationMode {
        self.loudness_normalizer.mode()
    }

    /// Set track gain for current track (called when loading track)
    ///
    /// # Arguments
    /// * `gain_db` - ReplayGain value in dB
    /// * `peak_dbfs` - Peak value in dBFS (for clipping prevention)
    #[cfg(feature = "volume-leveling")]
    pub fn set_track_gain(&mut self, gain_db: f64, peak_dbfs: f64) {
        self.loudness_normalizer.set_track_gain(gain_db, peak_dbfs);
    }

    /// Set album gain for current track (called when loading track)
    ///
    /// # Arguments
    /// * `gain_db` - Album ReplayGain value in dB
    /// * `peak_dbfs` - Album peak value in dBFS
    #[cfg(feature = "volume-leveling")]
    pub fn set_album_gain(&mut self, gain_db: f64, peak_dbfs: f64) {
        self.loudness_normalizer.set_album_gain(gain_db, peak_dbfs);
    }

    /// Clear gain values (for new track without loudness data)
    #[cfg(feature = "volume-leveling")]
    pub fn clear_loudness_gains(&mut self) {
        self.loudness_normalizer.clear_gains();
    }

    /// Set pre-amp gain for volume leveling (-12 to +12 dB)
    #[cfg(feature = "volume-leveling")]
    pub fn set_loudness_preamp(&mut self, preamp_db: f64) {
        self.loudness_normalizer.set_preamp_db(preamp_db);
    }

    /// Get pre-amp gain
    #[cfg(feature = "volume-leveling")]
    pub fn get_loudness_preamp(&self) -> f64 {
        self.loudness_normalizer.preamp_db()
    }

    /// Set whether clipping prevention is enabled
    #[cfg(feature = "volume-leveling")]
    pub fn set_prevent_clipping(&mut self, prevent: bool) {
        self.loudness_normalizer.set_prevent_clipping(prevent);
    }

    /// Get the effective gain being applied in dB
    #[cfg(feature = "volume-leveling")]
    pub fn get_effective_gain_db(&mut self) -> f64 {
        self.loudness_normalizer.effective_gain_db()
    }

    /// Reset loudness normalizer state (e.g., between tracks)
    #[cfg(feature = "volume-leveling")]
    pub fn reset_loudness_normalizer(&mut self) {
        self.loudness_normalizer.reset();
    }

    // ===== Output Limiter =====

    /// Set output limiter lookahead preset
    ///
    /// The limiter runs after volume to catch all peaks from the DSP chain.
    /// - Instant (0ms): No latency, may cause distortion on transients
    /// - Balanced (1.5ms): Good tradeoff between latency and transparency
    /// - Transparent (5ms): Minimal audible artifacts
    #[cfg(feature = "volume-leveling")]
    pub fn set_output_limiter_lookahead(&mut self, preset: LookaheadPreset) {
        self.output_limiter.set_lookahead(preset);
    }

    /// Get current output limiter lookahead preset
    #[cfg(feature = "volume-leveling")]
    pub fn get_output_limiter_lookahead(&self) -> LookaheadPreset {
        self.output_limiter.lookahead_preset()
    }

    /// Set output limiter lookahead in milliseconds (0-10ms)
    #[cfg(feature = "volume-leveling")]
    pub fn set_output_limiter_lookahead_ms(&mut self, lookahead_ms: f32) {
        self.output_limiter.set_lookahead_ms(lookahead_ms);
    }

    /// Set output limiter threshold in dB (0 dB = 0 dBFS, use negative for headroom)
    #[cfg(feature = "volume-leveling")]
    pub fn set_output_limiter_threshold_db(&mut self, threshold_db: f32) {
        self.output_limiter.set_threshold_db(threshold_db);
    }

    /// Get current output limiter gain reduction in dB (0 = no limiting)
    #[cfg(feature = "volume-leveling")]
    pub fn get_output_limiter_gain_reduction_db(&self) -> f32 {
        self.output_limiter.gain_reduction_db()
    }

    /// Get output limiter latency in samples
    #[cfg(feature = "volume-leveling")]
    pub fn get_output_limiter_latency(&self) -> usize {
        self.output_limiter.latency_samples()
    }

    /// Reset output limiter state
    #[cfg(feature = "volume-leveling")]
    pub fn reset_output_limiter(&mut self) {
        self.output_limiter.reset();
    }

    // ===== Headroom Management =====

    /// Set headroom mode
    ///
    /// Controls how headroom attenuation is calculated:
    /// - Auto: Calculates from ReplayGain + preamp + EQ boost
    /// - Manual(dB): Fixed headroom reserve (e.g., -6 dB)
    /// - Disabled: No headroom attenuation
    #[cfg(feature = "volume-leveling")]
    pub fn set_headroom_mode(&mut self, mode: HeadroomMode) {
        self.headroom_manager.set_mode(mode);
    }

    /// Get current headroom mode
    #[cfg(feature = "volume-leveling")]
    pub fn get_headroom_mode(&self) -> HeadroomMode {
        self.headroom_manager.mode()
    }

    /// Set ReplayGain value for headroom calculation (in dB)
    #[cfg(feature = "volume-leveling")]
    pub fn set_headroom_replaygain_db(&mut self, gain_db: f64) {
        self.headroom_manager.set_replaygain_db(gain_db);
    }

    /// Set pre-amp gain for headroom calculation (in dB)
    #[cfg(feature = "volume-leveling")]
    pub fn set_headroom_preamp_db(&mut self, preamp_db: f64) {
        self.headroom_manager.set_preamp_db(preamp_db);
    }

    /// Set maximum EQ boost for headroom calculation (in dB)
    ///
    /// This should be the maximum positive gain from any EQ band.
    /// Call this whenever EQ settings change.
    #[cfg(feature = "volume-leveling")]
    pub fn set_headroom_eq_boost_db(&mut self, boost_db: f64) {
        self.headroom_manager.set_eq_max_boost_db(boost_db);
    }

    /// Set additional DSP gain for headroom calculation (in dB)
    #[cfg(feature = "volume-leveling")]
    pub fn set_headroom_additional_gain_db(&mut self, gain_db: f64) {
        self.headroom_manager.set_additional_gain_db(gain_db);
    }

    /// Get total potential gain in dB (for UI display)
    #[cfg(feature = "volume-leveling")]
    pub fn get_headroom_total_gain_db(&self) -> f64 {
        self.headroom_manager.total_potential_gain_db()
    }

    /// Get current headroom attenuation in dB (for UI display)
    #[cfg(feature = "volume-leveling")]
    pub fn get_headroom_attenuation_db(&mut self) -> f64 {
        self.headroom_manager.attenuation_db()
    }

    /// Enable or disable headroom management
    #[cfg(feature = "volume-leveling")]
    pub fn set_headroom_enabled(&mut self, enabled: bool) {
        self.headroom_manager.set_enabled(enabled);
    }

    /// Check if headroom management is enabled
    #[cfg(feature = "volume-leveling")]
    pub fn is_headroom_enabled(&self) -> bool {
        self.headroom_manager.is_enabled()
    }

    /// Reset headroom manager state (e.g., for new track)
    #[cfg(feature = "volume-leveling")]
    pub fn reset_headroom(&mut self) {
        self.headroom_manager.reset();
    }

    /// Clear track-specific headroom values (ReplayGain) but keep settings
    #[cfg(feature = "volume-leveling")]
    pub fn clear_headroom_track_gains(&mut self) {
        self.headroom_manager.clear_track_gains();
    }

    /// Set audio source (called by platform after loading track)
    ///
    /// Uses pending source pattern for smooth transitions:
    /// - If currently playing: fades out current audio, then fades in new source
    /// - If not playing: directly sets the source with fade-in
    pub fn set_audio_source(&mut self, source: Box<dyn AudioSource>) {
        let previous_track_id = self.current_track.as_ref().map(|t| t.id.clone());

        // Check if we need to fade out current audio before switching
        let has_active_audio = self.audio_source.is_some() && self.state == PlaybackState::Playing;

        if has_active_audio && !self.stop_fade.is_active() {
            // Currently playing - use pending source pattern for smooth transition
            // 1. Start stop fade to fade out current audio
            // 2. Set new source as pending (will be activated when fade completes)
            tracing::info!("[set_audio_source] Using pending source pattern for smooth transition");
            self.pending_source = Some(source);
            self.stop_fade.start(FadeCompleteAction::TransitionToNext);
            // State stays Playing during the fade-out, then becomes Playing again after fade-in
        } else {
            // Not currently playing or already transitioning - directly set the source
            tracing::info!(
                "[set_audio_source] Direct source set (no active audio or already transitioning)"
            );
            self.pending_source = None; // Clear any pending source
            self.audio_source = Some(source);

            // CRITICAL FIX: Respect user_paused flag
            // If user explicitly paused, DON'T override their command even if state is Loading
            let should_play = !self.user_paused
                && (self.state == PlaybackState::Playing || self.state == PlaybackState::Loading);

            if should_play {
                tracing::info!("[set_audio_source] Setting state to Playing");
                self.state = PlaybackState::Playing;
                self.source_ready_verified = false;
                self.source_ready_wait_samples = 0;
                self.stop_fade.reset(); // Cancel any active stop fade
            } else {
                // User has paused/stopped - keep current state
                tracing::info!(
                    "[set_audio_source] Keeping state={:?} (user_paused={}, original_state={:?})",
                    self.state,
                    self.user_paused,
                    self.state
                );
                self.source_ready_verified = false;
                self.source_ready_wait_samples = 0;
                self.stop_fade.reset();
                self.start_fade.reset(); // Also cancel start fade to prevent audio from starting
            }
        }

        self.is_manual_skip = false;

        // Emit track changed event (for non-crossfade transitions)
        if let Some(ref track) = self.current_track {
            self.emit_track_changed(track.id.clone(), previous_track_id);
        }

        // CRITICAL: Only emit Playing state if we're actually playing
        // Don't emit if user has already paused
        if self.state == PlaybackState::Playing {
            self.emit_state_changed(PlaybackState::Playing);
        }
    }

    // ===== Crossfade Settings =====

    /// Ensure crossfade buffers are allocated (called before first use)
    /// This is safe to call outside audio callback as allocation happens on settings change
    fn ensure_crossfade_buffers_allocated(&mut self) {
        if self.outgoing_buffer.is_none() {
            tracing::debug!("[crossfade] Allocating buffers (~14.6MB) for crossfade processing");
            self.outgoing_buffer = Some(vec![0.0; CROSSFADE_BUFFER_SIZE]);
            self.incoming_buffer = Some(vec![0.0; CROSSFADE_BUFFER_SIZE]);
        }
    }

    /// Free crossfade buffers to save memory when crossfade is disabled
    fn free_crossfade_buffers(&mut self) {
        if self.outgoing_buffer.is_some() {
            tracing::debug!("[crossfade] Freeing buffers (~14.6MB) as crossfade is disabled");
            self.outgoing_buffer = None;
            self.incoming_buffer = None;
        }
    }

    /// Set crossfade settings
    pub fn set_crossfade_settings(&mut self, settings: CrossfadeSettings) {
        let was_enabled = self.crossfade.settings().enabled;
        let new_enabled = settings.enabled; // Get value before move
        self.crossfade.set_settings(settings);

        // Pre-allocate buffers if crossfade is being enabled
        // CRITICAL: This MUST happen outside the audio callback to avoid
        // allocating ~14.6MB in the real-time audio path
        if !was_enabled && new_enabled {
            self.ensure_crossfade_buffers_allocated();
        }

        // Free buffers if crossfade is being disabled
        if was_enabled && !new_enabled {
            self.free_crossfade_buffers();
        }
    }

    /// Get current crossfade settings
    pub fn get_crossfade_settings(&self) -> &CrossfadeSettings {
        self.crossfade.settings()
    }

    /// Enable or disable crossfade
    pub fn set_crossfade_enabled(&mut self, enabled: bool) {
        let was_enabled = self.crossfade.settings().enabled;
        let mut settings = self.crossfade.settings().clone();
        settings.enabled = enabled;
        self.crossfade.set_settings(settings);

        // Pre-allocate buffers if crossfade is being enabled
        // CRITICAL: This MUST happen outside the audio callback to avoid
        // allocating ~14.6MB in the real-time audio path
        if !was_enabled && enabled {
            self.ensure_crossfade_buffers_allocated();
        }

        // Free buffers if crossfade is being disabled
        if was_enabled && !enabled {
            self.free_crossfade_buffers();
        }
    }

    /// Check if crossfade is enabled
    pub fn is_crossfade_enabled(&self) -> bool {
        self.crossfade.settings().enabled
    }

    /// Set crossfade duration in milliseconds (0-10000)
    pub fn set_crossfade_duration(&mut self, duration_ms: u32) {
        let mut settings = self.crossfade.settings().clone();
        settings.duration_ms = duration_ms.min(10000);
        self.crossfade.set_settings(settings);
    }

    /// Get crossfade duration in milliseconds
    pub fn get_crossfade_duration(&self) -> u32 {
        self.crossfade.settings().duration_ms
    }

    /// Set crossfade curve type
    pub fn set_crossfade_curve(&mut self, curve: FadeCurve) {
        let mut settings = self.crossfade.settings().clone();
        settings.curve = curve;
        self.crossfade.set_settings(settings);
    }

    /// Get crossfade curve type
    pub fn get_crossfade_curve(&self) -> FadeCurve {
        self.crossfade.settings().curve
    }

    /// Set whether crossfade applies on manual skip
    pub fn set_crossfade_on_skip(&mut self, on_skip: bool) {
        let mut settings = self.crossfade.settings().clone();
        settings.on_skip = on_skip;
        self.crossfade.set_settings(settings);
    }

    /// Check crossfade state
    pub fn get_crossfade_state(&self) -> CrossfadeState {
        self.crossfade.state()
    }

    /// Check if crossfade is currently active
    pub fn is_crossfading(&self) -> bool {
        self.crossfade.is_active()
    }

    /// Get crossfade progress (0.0 to 1.0)
    pub fn get_crossfade_progress(&self) -> f32 {
        self.crossfade.progress()
    }

    // ===== Pre-decode / Gapless Support =====

    /// Set the next audio source for gapless/crossfade playback
    ///
    /// Called by platform when pre-decoding the next track.
    ///
    /// IMPORTANT: The source MUST be resampled to match the manager's sample rate.
    /// If sample rates don't match, a warning is logged but the source is still set
    /// (the platform layer is responsible for proper resampling).
    pub fn set_next_source(&mut self, source: Box<dyn AudioSource>, track: QueueTrack) {
        // Validate sample rate compatibility
        if let Some(source_rate) = source.sample_rate() {
            if source_rate != self.sample_rate {
                tracing::warn!(
                    "[GAPLESS] Sample rate mismatch: next source is {}Hz but manager expects {}Hz. \
                     This may cause audio glitches unless the platform resamples correctly.",
                    source_rate,
                    self.sample_rate
                );
            }
        }

        let track_id = track.id.clone();
        self.next_source = Some(source);
        self.next_track = Some(track);
        self.emit_next_track_prepared(track_id);
    }

    /// Validate that a source is compatible with gapless/crossfade playback
    ///
    /// Returns true if the source can be used for seamless transition.
    /// Checks sample rate compatibility (sources must match manager's rate).
    ///
    /// NOTE: Channel count is not checked because all sources are expected
    /// to output stereo (2 channels) after decoding.
    pub fn is_source_compatible(&self, source: &dyn AudioSource) -> bool {
        match source.sample_rate() {
            Some(rate) => rate == self.sample_rate,
            None => true, // Assume compatible if sample rate unknown
        }
    }

    /// Check if next source is ready
    pub fn has_next_source(&self) -> bool {
        self.next_source.is_some()
    }

    /// Get metadata for the next pre-decoded track
    pub fn get_next_track(&self) -> Option<&QueueTrack> {
        self.next_track.as_ref()
    }

    /// Get time remaining until crossfade should start (if applicable)
    ///
    /// Returns None if crossfade is disabled or position can't be determined.
    /// Returns Some(duration) with the time before crossfade should trigger.
    pub fn time_until_crossfade(&self) -> Option<Duration> {
        if !self.crossfade.settings().enabled {
            return None;
        }

        let source = self.audio_source.as_ref()?;
        let position = source.position();
        let duration = source.duration();
        let crossfade_duration =
            Duration::from_millis(self.crossfade.settings().duration_ms as u64);

        // Crossfade starts when: remaining_time <= crossfade_duration
        let remaining = duration.saturating_sub(position);

        if remaining <= crossfade_duration {
            Some(Duration::ZERO)
        } else {
            Some(remaining.checked_sub(crossfade_duration).unwrap())
        }
    }

    /// Check if we should start preparing the next track for crossfade
    ///
    /// Returns true when we're approaching the crossfade window
    /// and should pre-decode the next track.
    pub fn should_prepare_next_track(&self) -> bool {
        if !self.crossfade.settings().enabled && !self.gapless_enabled {
            return false;
        }

        // RepeatOne doesn't need pre-loading - it resets the same track
        // Pre-loading would be wasteful since we already have the source
        if self.repeat == RepeatMode::One {
            return false;
        }

        // If we already have the next source ready, no need to prepare
        if self.next_source.is_some() {
            return false;
        }

        // Check if queue has next track
        if self.queue.is_empty() && self.repeat != RepeatMode::All {
            return false;
        }

        // Check time remaining
        if let Some(time_until) = self.time_until_crossfade() {
            // Start preparing 5 seconds before crossfade
            // or immediately if crossfade is disabled (gapless mode)
            time_until <= Duration::from_secs(5)
        } else if self.gapless_enabled {
            // For gapless without crossfade, prepare when within 2 seconds
            if let Some(ref source) = self.audio_source {
                let remaining = source.duration().saturating_sub(source.position());
                remaining <= Duration::from_secs(2)
            } else {
                false
            }
        } else {
            false
        }
    }

    // ===== Events =====

    /// Drain all pending events
    ///
    /// Returns all events that have been emitted since the last drain.
    /// The UI should call this periodically (e.g., every frame or on audio callback)
    /// to synchronize with playback state.
    pub fn drain_events(&mut self) -> Vec<PlaybackEvent> {
        std::mem::take(&mut self.pending_events)
    }

    /// Check if there are pending events
    pub fn has_pending_events(&self) -> bool {
        !self.pending_events.is_empty()
    }

    /// Get the crossfade progress tracker
    pub fn crossfade_progress_tracker(&self) -> &CrossfadeProgressTracker {
        &self.crossfade_progress
    }

    /// Get the track ID that should be displayed in the UI
    ///
    /// During crossfade before 50%: returns outgoing track ID
    /// During crossfade after 50%: returns incoming track ID
    /// Otherwise: returns current track ID
    pub fn display_track_id(&self) -> Option<&str> {
        if self.crossfade_progress.is_active() {
            self.crossfade_progress.display_track_id()
        } else {
            self.current_track.as_ref().map(|t| t.id.as_str())
        }
    }

    /// Push an event to the pending events queue with overflow protection
    ///
    /// If the queue exceeds MAX_PENDING_EVENTS, drops the oldest events to make room.
    /// This prevents unbounded memory growth if events aren't drained promptly.
    fn push_event(&mut self, event: PlaybackEvent) {
        if self.pending_events.len() >= MAX_PENDING_EVENTS {
            // Drop oldest events to make room, keeping recent ones
            self.pending_events.drain(0..EVENT_OVERFLOW_DROP_COUNT);
            tracing::warn!(
                "[PLAYBACK] Event queue overflow, dropped {} oldest events",
                EVENT_OVERFLOW_DROP_COUNT
            );
        }
        self.pending_events.push(event);
    }

    /// Emit a state changed event
    /// Emit a state changed event with duplicate suppression
    ///
    /// Only emits if the state has actually changed from the last emitted state.
    /// This prevents flooding the event queue with redundant state updates.
    fn emit_state_changed(&mut self, state: PlaybackState) {
        // Suppress duplicate state events
        if self.last_emitted_state == Some(state) {
            return;
        }
        self.last_emitted_state = Some(state);
        self.push_event(PlaybackEvent::StateChanged {
            state: state.into(),
        });
    }

    /// Emit a track changed event
    fn emit_track_changed(&mut self, track_id: String, previous_track_id: Option<String>) {
        self.push_event(PlaybackEvent::TrackChanged {
            track_id,
            previous_track_id,
        });
    }

    /// Emit a crossfade started event
    fn emit_crossfade_started(
        &mut self,
        from_track_id: String,
        to_track_id: String,
        duration_ms: u32,
    ) {
        self.push_event(PlaybackEvent::CrossfadeStarted {
            from_track_id,
            to_track_id,
            duration_ms,
        });
    }

    /// Emit a crossfade progress event with throttling
    ///
    /// Only emits if progress has changed by at least 5% or if metadata_switched is true.
    /// This prevents flooding the event queue during crossfade while still allowing
    /// smooth UI updates.
    fn emit_crossfade_progress(&mut self, progress: f32, metadata_switched: bool) {
        // Always emit if metadata just switched (this is a significant event)
        // Otherwise, only emit if progress changed by at least 5%
        let progress_delta = (progress - self.last_emitted_crossfade_progress).abs();
        if !metadata_switched && progress_delta < 0.05 {
            return;
        }

        self.last_emitted_crossfade_progress = progress;
        self.push_event(PlaybackEvent::CrossfadeProgress {
            progress,
            metadata_switched,
        });
    }

    /// Emit a crossfade completed event
    fn emit_crossfade_completed(&mut self) {
        self.push_event(PlaybackEvent::CrossfadeCompleted);
    }

    /// Emit a track finished event
    fn emit_track_finished(&mut self, track_id: String) {
        self.push_event(PlaybackEvent::TrackFinished { track_id });
    }

    /// Emit a volume changed event
    fn emit_volume_changed(&mut self) {
        self.push_event(PlaybackEvent::VolumeChanged {
            level: self.volume.level(),
            is_muted: self.volume.is_muted(),
        });
    }

    /// Emit a queue changed event
    fn emit_queue_changed(&mut self) {
        self.push_event(PlaybackEvent::QueueChanged {
            length: self.queue.len(),
        });
    }

    /// Emit an error event
    fn emit_error(&mut self, message: String) {
        self.push_event(PlaybackEvent::Error { message });
    }

    /// Emit a next track prepared event
    fn emit_next_track_prepared(&mut self, track_id: String) {
        self.push_event(PlaybackEvent::NextTrackPrepared { track_id });
    }

    /// Emit a position update event
    pub fn emit_position_update(&mut self) {
        if let Some(ref source) = self.audio_source {
            self.push_event(PlaybackEvent::PositionUpdate {
                position_ms: source.position().as_millis() as u64,
                duration_ms: source.duration().as_millis() as u64,
            });
        }
    }

    /// Maybe emit a position update event (throttled based on samples processed)
    ///
    /// Position updates are throttled to avoid flooding the event queue.
    /// Updates are emitted approximately every 250ms based on sample count.
    ///
    /// # Arguments
    /// * `samples_processed` - Number of samples processed in this callback
    pub fn maybe_emit_position_update(&mut self, samples_processed: usize) {
        // Accumulate samples
        self.position_update_samples += samples_processed;

        // Calculate threshold: emit approximately every 250ms
        // At 48kHz stereo, 250ms = 48000 * 0.25 * 2 = 24000 samples
        let threshold = (self.sample_rate as usize * 2) / 4; // 250ms

        if self.position_update_samples >= threshold {
            self.emit_position_update();
            self.position_update_samples = 0;
        }
    }
}

impl Default for PlaybackManager {
    fn default() -> Self {
        Self::new(PlaybackConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::DummyAudioSource;
    use crate::types::TrackSource;
    use std::path::PathBuf;

    fn create_test_track(id: &str) -> QueueTrack {
        QueueTrack {
            id: id.to_string(),
            path: PathBuf::from(format!("/music/{}.mp3", id)),
            title: format!("Track {}", id),
            artist: "Test Artist".to_string(),
            album: Some("Test Album".to_string()),
            duration: Duration::from_secs(180),
            track_number: Some(1),
            source: TrackSource::Single,
        }
    }

    #[test]
    fn create_playback_manager() {
        let manager = PlaybackManager::new(PlaybackConfig::default());
        assert_eq!(manager.get_state(), PlaybackState::Stopped);
        assert_eq!(manager.get_volume(), 80);
        assert!(manager.get_queue().is_empty());
    }

    #[test]
    fn set_volume() {
        let mut manager = PlaybackManager::default();

        manager.set_volume(50);
        assert_eq!(manager.get_volume(), 50);

        manager.set_volume(100);
        assert_eq!(manager.get_volume(), 100);
    }

    #[test]
    fn mute_unmute() {
        let mut manager = PlaybackManager::default();

        assert!(!manager.is_muted());

        manager.mute();
        assert!(manager.is_muted());

        manager.unmute();
        assert!(!manager.is_muted());
    }

    #[test]
    fn add_to_queue() {
        let mut manager = PlaybackManager::default();

        manager.add_to_queue_next(create_test_track("1"));
        manager.add_to_queue_end(create_test_track("2"));

        assert_eq!(manager.queue_len(), 2);
    }

    #[test]
    fn shuffle_modes() {
        let mut manager = PlaybackManager::default();

        // Add some tracks
        manager.add_playlist_to_queue(vec![
            create_test_track("1"),
            create_test_track("2"),
            create_test_track("3"),
        ]);

        assert_eq!(manager.get_shuffle(), ShuffleMode::Off);

        // Enable shuffle
        manager.set_shuffle(ShuffleMode::Random);
        assert_eq!(manager.get_shuffle(), ShuffleMode::Random);

        // Disable shuffle (should restore original order)
        manager.set_shuffle(ShuffleMode::Off);
        assert_eq!(manager.get_shuffle(), ShuffleMode::Off);
    }

    #[test]
    fn repeat_modes() {
        let mut manager = PlaybackManager::default();

        assert_eq!(manager.get_repeat(), RepeatMode::Off);

        manager.set_repeat(RepeatMode::All);
        assert_eq!(manager.get_repeat(), RepeatMode::All);

        manager.set_repeat(RepeatMode::One);
        assert_eq!(manager.get_repeat(), RepeatMode::One);
    }

    #[test]
    fn process_audio_when_stopped() {
        let mut manager = PlaybackManager::default();
        let mut buffer = [1.0f32; 1024];

        let result = manager.process_audio(&mut buffer);
        assert!(result.is_ok());

        // Should output near-silence (DAC keepalive noise at ~-96dB is acceptable)
        let dac_keepalive_threshold = 0.0001; // ~-80dB, well above DAC keepalive noise
        assert!(
            buffer[0].abs() < dac_keepalive_threshold,
            "Expected near-silence, got {}",
            buffer[0]
        );
        assert!(
            buffer[1023].abs() < dac_keepalive_threshold,
            "Expected near-silence, got {}",
            buffer[1023]
        );
    }

    #[test]
    fn set_audio_source_respects_current_state() {
        // Test that set_audio_source() respects the current state
        // and doesn't unconditionally force Playing state

        let mut manager = PlaybackManager::default();
        assert_eq!(manager.get_state(), PlaybackState::Stopped);

        // Case 1: Setting source while Stopped should keep Stopped
        let source1 = Box::new(DummyAudioSource::new(Duration::from_secs(10), 44100));
        manager.set_audio_source(source1);
        assert_eq!(
            manager.get_state(),
            PlaybackState::Stopped,
            "Should stay Stopped when source set without play command"
        );

        // Case 2: Setting source after playing from Loading should become Playing
        // This simulates the normal flow: play() → Loading → set_audio_source() → Playing
        let mut manager2 = PlaybackManager::default();
        let track = QueueTrack {
            id: "test".to_string(),
            path: std::path::PathBuf::from("test.mp3"),
            title: "Test".to_string(),
            artist: "Test Artist".to_string(),
            album: None,
            duration: Duration::from_secs(180),
            track_number: None,
            source: TrackSource::Single,
        };
        manager2.load_playlist(vec![track], 0);
        manager2.play().unwrap(); // Sets state to Loading

        let source2 = Box::new(DummyAudioSource::new(Duration::from_secs(10), 44100));
        manager2.set_audio_source(source2);
        assert_eq!(
            manager2.get_state(),
            PlaybackState::Playing,
            "Should transition to Playing when coming from Loading"
        );

        // Case 3: Setting source after pausing should keep Paused
        let mut manager3 = PlaybackManager::default();
        manager3.load_playlist(
            vec![QueueTrack {
                id: "test2".to_string(),
                path: std::path::PathBuf::from("test2.mp3"),
                title: "Test 2".to_string(),
                artist: "Test Artist".to_string(),
                album: None,
                duration: Duration::from_secs(180),
                track_number: None,
                source: TrackSource::Single,
            }],
            0,
        );
        manager3.play().unwrap(); // Loading
        manager3.pause(); // Paused

        let source3 = Box::new(DummyAudioSource::new(Duration::from_secs(10), 44100));
        manager3.set_audio_source(source3);
        assert_eq!(
            manager3.get_state(),
            PlaybackState::Paused,
            "Should keep Paused when user has paused during loading"
        );
    }

    #[test]
    fn stop_resets_source_ready_state() {
        // Test that stop() properly resets source_ready_verified and source_ready_wait_samples
        // This prevents stale state from affecting the next playback start
        let mut manager = PlaybackManager::default();

        // Setup: Load a playlist and start playing
        manager.load_playlist(vec![create_test_track("1")], 0);
        manager.play().unwrap(); // Loading

        // Simulate source becoming ready
        let source = Box::new(DummyAudioSource::new(Duration::from_secs(10), 44100));
        manager.set_audio_source(source);

        // Verify initial Playing state
        assert_eq!(manager.get_state(), PlaybackState::Playing);

        // Now stop playback
        manager.stop();

        // Verify stopped state and that source_ready_verified is reset
        assert_eq!(manager.get_state(), PlaybackState::Stopped);
        assert!(
            !manager.source_ready_verified,
            "source_ready_verified should be reset after stop()"
        );
        assert_eq!(
            manager.source_ready_wait_samples, 0,
            "source_ready_wait_samples should be reset after stop()"
        );
    }

    #[test]
    fn seek_returns_error_when_loading() {
        // Test that seek_to() returns NoTrackLoaded error when in Loading state
        // This prevents seeking while the source may not be fully initialized
        let mut manager = PlaybackManager::default();

        // Setup: Start playing but don't set source (stays in Loading)
        manager.load_playlist(vec![create_test_track("1")], 0);
        manager.play().unwrap();
        assert_eq!(manager.get_state(), PlaybackState::Loading);

        // Try to seek while in Loading state
        let result = manager.seek_to(Duration::from_secs(5));
        assert!(
            result.is_err(),
            "seek_to() should return error when in Loading state"
        );
        if let Err(PlaybackError::NoTrackLoaded) = result {
            // Expected error
        } else {
            panic!("Expected NoTrackLoaded error, got {:?}", result);
        }
    }

    #[test]
    fn seek_returns_error_when_stopped() {
        // Test that seek_to() returns NoTrackLoaded error when in Stopped state
        let mut manager = PlaybackManager::default();

        assert_eq!(manager.get_state(), PlaybackState::Stopped);

        // Try to seek while in Stopped state
        let result = manager.seek_to(Duration::from_secs(5));
        assert!(
            result.is_err(),
            "seek_to() should return error when in Stopped state"
        );
        if let Err(PlaybackError::NoTrackLoaded) = result {
            // Expected error
        } else {
            panic!("Expected NoTrackLoaded error, got {:?}", result);
        }
    }

    #[test]
    fn play_from_paused_cancels_stop_fade() {
        // Test that play() from Paused state cancels any active stop fade
        // This prevents the fade from completing and reverting state to Paused
        let mut manager = PlaybackManager::default();

        // Setup: Load playlist and start playing without audio source
        // When there's no audio source, pause() immediately transitions to Paused
        manager.load_playlist(vec![create_test_track("1")], 0);
        manager.play().unwrap(); // Loading

        // Pause during loading (no audio source, so immediate state change to Paused)
        manager.pause();
        assert_eq!(
            manager.get_state(),
            PlaybackState::Paused,
            "Should be Paused after pause() during Loading"
        );

        // Now set audio source - should stay Paused because user_paused is true
        let source = Box::new(DummyAudioSource::new(Duration::from_secs(10), 44100));
        manager.set_audio_source(source);
        assert_eq!(manager.get_state(), PlaybackState::Paused);

        // Verify user_paused is set
        assert!(manager.user_paused, "user_paused should be true");

        // Now resume playback
        manager.play().unwrap();

        // Verify we're Playing and user_paused is cleared
        assert_eq!(
            manager.get_state(),
            PlaybackState::Playing,
            "Should be Playing after play() from Paused"
        );
        assert!(
            !manager.user_paused,
            "user_paused should be cleared after play()"
        );
        assert!(
            !manager.stop_fade.is_active(),
            "stop_fade should not be active after play() from Paused"
        );
    }

    #[test]
    fn play_from_paused_with_pending_fade_cancels_fade() {
        // Test that play() properly handles the case where pause() has started a fade
        // but hasn't completed yet. When state defers to fade completion, calling
        // play() while state is still Playing should still work correctly.
        let mut manager = PlaybackManager::default();

        // Setup: Get to Playing state with audio source
        manager.load_playlist(vec![create_test_track("1")], 0);
        manager.play().unwrap(); // Loading

        let source = Box::new(DummyAudioSource::new(Duration::from_secs(10), 44100));
        manager.set_audio_source(source);
        manager.source_ready_verified = true;
        assert_eq!(manager.get_state(), PlaybackState::Playing);

        // Pause - this starts a stop fade with deferred state change
        manager.pause();

        // State is still Playing during fade, but stop_fade is active
        // and pending_state is Some(Paused)
        assert!(
            manager.stop_fade.is_active() || manager.get_state() == PlaybackState::Paused,
            "Either stop_fade is active (deferred) or state is already Paused"
        );

        // Rapid toggle: if state is Paused, play() will cancel the fade
        // if state is still Playing, it returns early (already playing)
        if manager.get_state() == PlaybackState::Paused {
            manager.play().unwrap();
            assert_eq!(manager.get_state(), PlaybackState::Playing);
            assert!(
                !manager.stop_fade.is_active(),
                "stop_fade should be cancelled"
            );
        }
        // If still Playing with active fade, the fade will complete and
        // transition to Paused. This is expected behavior - the pause command
        // takes precedence since it was the last user action.
    }

    #[test]
    fn stop_clears_all_playback_state() {
        // Test that stop() properly clears all playback-related state
        let mut manager = PlaybackManager::default();

        // Setup a complex state
        manager.load_playlist(
            vec![
                create_test_track("1"),
                create_test_track("2"),
                create_test_track("3"),
            ],
            0,
        );
        manager.play().unwrap();

        // Set audio source
        let source = Box::new(DummyAudioSource::new(Duration::from_secs(10), 44100));
        manager.set_audio_source(source);

        // Set next source for crossfade
        let next_source = Box::new(DummyAudioSource::new(Duration::from_secs(10), 44100));
        manager.set_next_source(next_source, create_test_track("2"));

        // Now stop
        manager.stop();

        // Verify all state is cleared
        assert_eq!(manager.get_state(), PlaybackState::Stopped);
        assert!(
            manager.audio_source.is_none(),
            "audio_source should be None"
        );
        assert!(manager.next_source.is_none(), "next_source should be None");
        assert!(manager.next_track.is_none(), "next_track should be None");
        assert!(
            manager.pending_source.is_none(),
            "pending_source should be None"
        );
        assert!(
            manager.current_track.is_none(),
            "current_track should be None"
        );
        assert!(!manager.user_paused, "user_paused should be false");
        assert!(
            !manager.source_ready_verified,
            "source_ready_verified should be false"
        );
    }

    #[test]
    fn pause_during_loading_sets_user_paused() {
        // Test that pause() during Loading state properly sets user_paused
        // so that set_audio_source() respects the pause
        let mut manager = PlaybackManager::default();

        // Start loading
        manager.load_playlist(vec![create_test_track("1")], 0);
        manager.play().unwrap();
        assert_eq!(manager.get_state(), PlaybackState::Loading);

        // Pause during loading
        manager.pause();
        assert!(
            manager.user_paused,
            "user_paused should be true after pause during Loading"
        );

        // Now set audio source - should stay Paused
        let source = Box::new(DummyAudioSource::new(Duration::from_secs(10), 44100));
        manager.set_audio_source(source);
        assert_eq!(
            manager.get_state(),
            PlaybackState::Paused,
            "Should be Paused after set_audio_source when user_paused is true"
        );
    }

    #[test]
    fn next_clears_user_paused() {
        // Test that next() clears user_paused flag
        // User expects next() to START the next track, not stay paused
        let mut manager = PlaybackManager::default();

        // Setup
        manager.load_playlist(vec![create_test_track("1"), create_test_track("2")], 0);
        manager.play().unwrap();

        // Set source and pause
        let source = Box::new(DummyAudioSource::new(Duration::from_secs(10), 44100));
        manager.set_audio_source(source);
        manager.source_ready_verified = true;
        manager.pause();
        assert!(
            manager.user_paused,
            "user_paused should be true after pause"
        );

        // Skip to next
        let _ = manager.next();
        assert!(
            !manager.user_paused,
            "user_paused should be false after next()"
        );
    }

    // ===== Gapless Playback Edge Case Tests =====

    #[test]
    fn peek_next_queue_track_with_repeat_all_at_queue_end() {
        let mut manager = PlaybackManager::default();

        // Add tracks to queue
        let tracks = vec![
            create_test_track("1"),
            create_test_track("2"),
            create_test_track("3"),
        ];
        manager.load_playlist(tracks, 0);

        // Enable repeat all
        manager.set_repeat(RepeatMode::All);

        // Consume all tracks to exhaust the queue
        let _ = manager.play(); // Starts track 1

        // Set audio source to simulate playback
        let source = Box::new(DummyAudioSource::new(Duration::from_secs(10), 44100));
        manager.set_audio_source(source);

        // Advance through queue
        let _ = manager.next(); // Now on track 2
        let source2 = Box::new(DummyAudioSource::new(Duration::from_secs(10), 44100));
        manager.set_audio_source(source2);

        let _ = manager.next(); // Now on track 3
        let source3 = Box::new(DummyAudioSource::new(Duration::from_secs(10), 44100));
        manager.set_audio_source(source3);

        // Now queue is exhausted, but repeat all is on
        // peek_next_queue_track should return track 1 (first track for loop)
        let next = manager.peek_next_queue_track();
        assert!(
            next.is_some(),
            "With RepeatAll, should return first track when queue exhausted"
        );
        assert_eq!(
            next.unwrap().id,
            "1",
            "Should return first track from original source for RepeatAll loop"
        );
    }

    #[test]
    fn peek_next_queue_track_returns_none_when_repeat_off_and_queue_empty() {
        let mut manager = PlaybackManager::default();

        // Add single track
        manager.load_playlist(vec![create_test_track("1")], 0);

        // Repeat is off by default
        assert_eq!(manager.get_repeat(), RepeatMode::Off);

        // Start playback
        let _ = manager.play();
        let source = Box::new(DummyAudioSource::new(Duration::from_secs(10), 44100));
        manager.set_audio_source(source);

        // Queue is now exhausted (only 1 track, and it's now playing)
        let next = manager.peek_next_queue_track();
        assert!(
            next.is_none(),
            "With RepeatOff and empty queue, should return None"
        );
    }

    #[test]
    fn peek_next_queue_track_returns_current_with_repeat_one() {
        let mut manager = PlaybackManager::default();

        // Add tracks
        manager.load_playlist(vec![create_test_track("1"), create_test_track("2")], 0);

        // Enable repeat one
        manager.set_repeat(RepeatMode::One);

        // Start playback
        let _ = manager.play();
        let source = Box::new(DummyAudioSource::new(Duration::from_secs(10), 44100));
        manager.set_audio_source(source);

        // With repeat one, peek should return current track
        let next = manager.peek_next_queue_track();
        assert!(
            next.is_some(),
            "With RepeatOne, should return current track"
        );
        assert_eq!(
            next.unwrap().id,
            "1",
            "RepeatOne should return current track for pre-loading"
        );
    }

    #[test]
    fn is_source_compatible_validates_sample_rate() {
        let manager = PlaybackManager::default();

        // Manager defaults to 44100 Hz
        assert_eq!(manager.get_sample_rate(), 44100);

        // Compatible source (same rate)
        let compatible = DummyAudioSource::new(Duration::from_secs(10), 44100);
        assert!(
            manager.is_source_compatible(&compatible),
            "Source with matching sample rate should be compatible"
        );

        // Incompatible source (different rate)
        let incompatible = DummyAudioSource::new(Duration::from_secs(10), 48000);
        assert!(
            !manager.is_source_compatible(&incompatible),
            "Source with different sample rate should be incompatible"
        );
    }

    #[test]
    fn set_next_source_accepts_mismatched_sample_rate_with_warning() {
        let mut manager = PlaybackManager::default();

        // Set manager to 44100 Hz
        manager.set_sample_rate(44100);

        // Add tracks to queue first
        manager.load_playlist(vec![create_test_track("1"), create_test_track("2")], 0);

        // Set a source with different sample rate
        // Note: The manager logs a warning but still accepts the source
        // (platform layer is responsible for resampling)
        let source = Box::new(DummyAudioSource::new(Duration::from_secs(10), 48000));
        manager.set_next_source(source, create_test_track("2"));

        // Source should still be set despite mismatch
        assert!(
            manager.has_next_source(),
            "Next source should be set even with sample rate mismatch"
        );
    }

    #[test]
    fn gapless_enabled_affects_should_prepare_next_track() {
        let mut manager = PlaybackManager::new(PlaybackConfig {
            gapless: false,
            ..PlaybackConfig::default()
        });

        // Add tracks
        manager.load_playlist(vec![create_test_track("1"), create_test_track("2")], 0);

        // Start playing
        let _ = manager.play();

        // With gapless disabled and crossfade disabled, should not prepare
        assert!(
            !manager.should_prepare_next_track(),
            "With gapless disabled, should not prepare next track"
        );

        // Enable gapless
        manager.gapless_enabled = true;

        // Create a source near end (within 2 second gapless window)
        let mut source = DummyAudioSource::new(Duration::from_secs(3), 44100);
        source.seek(Duration::from_millis(2500)).unwrap();
        manager.set_audio_source(Box::new(source));

        // With track near end and gapless enabled, should prepare
        assert!(
            manager.should_prepare_next_track(),
            "With gapless enabled and track near end, should prepare next track"
        );
    }

    #[test]
    fn transition_to_next_track_moves_sources_correctly() {
        let mut manager = PlaybackManager::default();

        // Setup: Load playlist, start playback
        manager.load_playlist(vec![create_test_track("1"), create_test_track("2")], 0);
        let _ = manager.play();

        // Set current source
        let source1 = Box::new(DummyAudioSource::new(Duration::from_secs(10), 44100));
        manager.set_audio_source(source1);

        // Set next source for gapless
        let source2 = Box::new(DummyAudioSource::new(Duration::from_secs(10), 44100));
        manager.set_next_source(source2, create_test_track("2"));

        // Verify next source is set
        assert!(manager.has_next_source());
        assert_eq!(manager.get_next_track().unwrap().id, "2");

        // Perform transition
        manager.transition_to_next_track().unwrap();

        // Verify next_source moved to audio_source
        assert!(manager.audio_source.is_some());
        assert!(manager.next_source.is_none());
        assert!(manager.next_track.is_none());

        // Current track should now be track 2
        assert_eq!(manager.get_current_track().unwrap().id, "2");

        // Track 1 should be in history
        let history = manager.get_history();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].id, "1");
    }

    #[test]
    fn crossfade_cancelled_when_repeat_one_enabled() {
        let mut manager = PlaybackManager::default();

        // Enable crossfade
        manager.set_crossfade_enabled(true);
        manager.set_crossfade_duration(3000);

        // Load playlist
        manager.load_playlist(vec![create_test_track("1"), create_test_track("2")], 0);
        let _ = manager.play();

        // Set current and next sources
        let source1 = Box::new(DummyAudioSource::new(Duration::from_secs(10), 44100));
        manager.set_audio_source(source1);
        let source2 = Box::new(DummyAudioSource::new(Duration::from_secs(10), 44100));
        manager.set_next_source(source2, create_test_track("2"));

        // Manually start crossfade (simulating approaching end of track)
        manager.crossfade.set_sample_rate(44100);
        manager.crossfade.start(false);
        assert!(manager.crossfade.is_active());

        // Now enable repeat one - crossfade should be cancelled
        manager.set_repeat(RepeatMode::One);

        // Crossfade should be reset
        assert!(!manager.crossfade.is_active());
        assert!(manager.next_source.is_none());
        assert!(manager.next_track.is_none());
    }

    #[test]
    fn crossfade_buffers_preallocated_on_enable() {
        // THREAD SAFETY TEST: Verifies that crossfade buffers are pre-allocated
        // when crossfade is enabled, rather than during the audio callback.
        // This is critical for avoiding ~14.6MB heap allocations in the real-time
        // audio path which would cause latency spikes and potential buffer underruns.
        let mut manager = PlaybackManager::default();

        // Initially crossfade is disabled
        assert!(!manager.is_crossfade_enabled());
        // Buffers should not be allocated
        assert!(
            manager.outgoing_buffer.is_none(),
            "Buffers should not be allocated when crossfade is disabled"
        );
        assert!(
            manager.incoming_buffer.is_none(),
            "Buffers should not be allocated when crossfade is disabled"
        );

        // Enable crossfade via set_crossfade_enabled
        manager.set_crossfade_enabled(true);
        assert!(manager.is_crossfade_enabled());

        // Buffers should now be pre-allocated (not lazily in audio callback)
        assert!(
            manager.outgoing_buffer.is_some(),
            "Outgoing buffer should be pre-allocated when crossfade is enabled"
        );
        assert!(
            manager.incoming_buffer.is_some(),
            "Incoming buffer should be pre-allocated when crossfade is enabled"
        );
        assert_eq!(
            manager.outgoing_buffer.as_ref().unwrap().len(),
            CROSSFADE_BUFFER_SIZE,
            "Buffer should have correct size"
        );

        // Disable crossfade
        manager.set_crossfade_enabled(false);
        assert!(!manager.is_crossfade_enabled());

        // Buffers should be freed to save ~14.6MB of memory
        assert!(
            manager.outgoing_buffer.is_none(),
            "Buffers should be freed when crossfade is disabled"
        );
        assert!(
            manager.incoming_buffer.is_none(),
            "Buffers should be freed when crossfade is disabled"
        );
    }

    #[test]
    fn crossfade_buffers_preallocated_via_settings() {
        // Same test but using set_crossfade_settings instead of set_crossfade_enabled
        let mut manager = PlaybackManager::default();

        // Initially buffers not allocated
        assert!(manager.outgoing_buffer.is_none());

        // Enable via settings
        let mut settings = manager.get_crossfade_settings().clone();
        settings.enabled = true;
        settings.duration_ms = 3000;
        manager.set_crossfade_settings(settings);

        // Buffers should be pre-allocated
        assert!(
            manager.outgoing_buffer.is_some(),
            "Buffers should be pre-allocated when crossfade is enabled via settings"
        );
        assert!(
            manager.incoming_buffer.is_some(),
            "Buffers should be pre-allocated when crossfade is enabled via settings"
        );

        // Disable via settings
        let mut settings = manager.get_crossfade_settings().clone();
        settings.enabled = false;
        manager.set_crossfade_settings(settings);

        // Buffers should be freed
        assert!(
            manager.outgoing_buffer.is_none(),
            "Buffers should be freed when crossfade is disabled via settings"
        );
    }

    // ===== Repeat Mode + Crossfade Interaction Tests =====

    #[test]
    fn repeat_one_prevents_crossfade_from_starting() {
        // Test that crossfade does NOT start when RepeatOne is enabled.
        // RepeatOne should seamlessly restart the same track, not crossfade to itself.
        let mut manager = PlaybackManager::default();

        // Enable crossfade
        manager.set_crossfade_enabled(true);
        manager.set_crossfade_duration(3000);

        // Enable repeat one BEFORE loading tracks
        manager.set_repeat(RepeatMode::One);

        // Load playlist
        manager.load_playlist(vec![create_test_track("1"), create_test_track("2")], 0);
        let _ = manager.play();

        // Set current source
        let source1 = Box::new(DummyAudioSource::new(Duration::from_secs(10), 44100));
        manager.set_audio_source(source1);
        manager.source_ready_verified = true;

        // Set next source (simulating platform pre-loading, which peek_next_queue_track
        // returns the same track for RepeatOne)
        let source2 = Box::new(DummyAudioSource::new(Duration::from_secs(10), 44100));
        manager.set_next_source(source2, create_test_track("1")); // Same track!

        // Verify crossfade.start() would return false because RepeatOne is enabled
        // We can't easily test process_stereo_with_crossfade directly, but we can verify
        // the should_prepare_next_track logic
        assert!(
            !manager.should_prepare_next_track(),
            "should_prepare_next_track() should return false for RepeatOne"
        );
    }

    #[test]
    fn repeat_one_does_not_need_preloading() {
        // Test that should_prepare_next_track returns false for RepeatOne mode
        // because the same track will be reset, not a new track loaded.
        let mut manager = PlaybackManager::default();

        // Enable crossfade and gapless
        manager.set_crossfade_enabled(true);
        manager.gapless_enabled = true;

        // Enable repeat one
        manager.set_repeat(RepeatMode::One);

        // Load playlist
        manager.load_playlist(vec![create_test_track("1")], 0);
        let _ = manager.play();

        // Set current source near the end
        let mut source = DummyAudioSource::new(Duration::from_secs(10), 44100);
        source.seek(Duration::from_secs(8)).unwrap(); // Near end
        manager.set_audio_source(Box::new(source));
        manager.source_ready_verified = true;

        // should_prepare_next_track should return false for RepeatOne
        assert!(
            !manager.should_prepare_next_track(),
            "RepeatOne mode should not trigger preloading"
        );
    }

    #[test]
    fn repeat_all_wraps_around_correctly() {
        // Test that RepeatAll correctly wraps around to the first track
        let mut manager = PlaybackManager::default();

        // Enable repeat all
        manager.set_repeat(RepeatMode::All);

        // Load 2-track playlist
        manager.load_playlist(vec![create_test_track("1"), create_test_track("2")], 0);
        let _ = manager.play();

        // Play track 1
        let source1 = Box::new(DummyAudioSource::new(Duration::from_secs(10), 44100));
        manager.set_audio_source(source1);

        // Skip to track 2
        let _ = manager.next();
        let source2 = Box::new(DummyAudioSource::new(Duration::from_secs(10), 44100));
        manager.set_audio_source(source2);
        assert_eq!(manager.get_current_track().unwrap().id, "2");

        // Skip again - should wrap to track 1
        let _ = manager.next();
        assert_eq!(
            manager.get_state(),
            PlaybackState::Loading,
            "Should be loading track 1 after wrap"
        );
        assert_eq!(
            manager.get_current_track().unwrap().id,
            "1",
            "Should wrap to track 1 with RepeatAll"
        );
    }

    #[test]
    fn repeat_all_with_shuffle_reshuffles_on_wrap() {
        // Test that RepeatAll + Shuffle re-shuffles the queue when wrapping
        let mut manager = PlaybackManager::default();

        // Enable repeat all and shuffle
        manager.set_repeat(RepeatMode::All);
        manager.set_shuffle(ShuffleMode::Random);

        // Load playlist
        let tracks = vec![
            create_test_track("1"),
            create_test_track("2"),
            create_test_track("3"),
        ];
        manager.load_playlist(tracks, 0);

        // Verify shuffle is applied
        assert_eq!(manager.get_shuffle(), ShuffleMode::Random);

        // Exhaust the queue
        let _ = manager.play();
        let source1 = Box::new(DummyAudioSource::new(Duration::from_secs(10), 44100));
        manager.set_audio_source(source1);

        let _ = manager.next();
        let source2 = Box::new(DummyAudioSource::new(Duration::from_secs(10), 44100));
        manager.set_audio_source(source2);

        let _ = manager.next();
        let source3 = Box::new(DummyAudioSource::new(Duration::from_secs(10), 44100));
        manager.set_audio_source(source3);

        // Queue should be exhausted, next should trigger wrap
        let _ = manager.next();

        // Should have looped back
        assert_eq!(manager.get_state(), PlaybackState::Loading);
        assert!(
            manager.get_current_track().is_some(),
            "Should have a track after RepeatAll wrap"
        );
    }

    #[test]
    fn repeat_one_with_single_track_queue() {
        // Test RepeatOne with a single-track queue
        let mut manager = PlaybackManager::default();

        manager.set_repeat(RepeatMode::One);
        manager.load_playlist(vec![create_test_track("only_track")], 0);
        let _ = manager.play();

        // Set source
        let source = Box::new(DummyAudioSource::new(Duration::from_secs(10), 44100));
        manager.set_audio_source(source);
        manager.source_ready_verified = true;

        // Current track should be the only track
        assert_eq!(manager.get_current_track().unwrap().id, "only_track");

        // peek_next_queue_track should return the same track
        let next = manager.peek_next_queue_track();
        assert!(next.is_some());
        assert_eq!(next.unwrap().id, "only_track");

        // has_next should return true (RepeatOne always has next)
        assert!(manager.has_next(), "RepeatOne should always have next");
    }

    #[test]
    fn repeat_mode_change_during_playback() {
        // Test changing repeat mode during playback
        let mut manager = PlaybackManager::default();

        // Start with RepeatOff
        manager.set_repeat(RepeatMode::Off);
        manager.load_playlist(vec![create_test_track("1"), create_test_track("2")], 0);
        let _ = manager.play();

        let source = Box::new(DummyAudioSource::new(Duration::from_secs(10), 44100));
        manager.set_audio_source(source);

        assert_eq!(manager.get_repeat(), RepeatMode::Off);

        // Change to RepeatAll during playback
        manager.set_repeat(RepeatMode::All);
        assert_eq!(manager.get_repeat(), RepeatMode::All);

        // Change to RepeatOne during playback
        manager.set_repeat(RepeatMode::One);
        assert_eq!(manager.get_repeat(), RepeatMode::One);

        // Change back to Off
        manager.set_repeat(RepeatMode::Off);
        assert_eq!(manager.get_repeat(), RepeatMode::Off);
    }

    #[test]
    fn repeat_all_peek_returns_first_track_at_queue_end() {
        // Test that peek_next_queue_track returns the first track when queue is exhausted
        // and RepeatAll is enabled. This enables seamless pre-loading for loop transitions.
        let mut manager = PlaybackManager::default();

        manager.set_repeat(RepeatMode::All);

        // Load 2-track playlist
        manager.load_playlist(vec![create_test_track("1"), create_test_track("2")], 0);
        let _ = manager.play();

        // Play through both tracks
        let source1 = Box::new(DummyAudioSource::new(Duration::from_secs(10), 44100));
        manager.set_audio_source(source1);

        let _ = manager.next();
        let source2 = Box::new(DummyAudioSource::new(Duration::from_secs(10), 44100));
        manager.set_audio_source(source2);

        // Now on track 2, queue exhausted
        // peek should return track 1 for pre-loading
        let next = manager.peek_next_queue_track();
        assert!(
            next.is_some(),
            "Should return first track for RepeatAll loop"
        );
        assert_eq!(
            next.unwrap().id,
            "1",
            "Should pre-load track 1 for RepeatAll loop"
        );
    }

    // ===== Audio Buffer Handling Tests =====

    #[test]
    fn underrun_buffer_fills_with_noise() {
        // Test that fill_underrun_buffer produces proper LFSR noise at -96dB level
        let mut manager = PlaybackManager::default();
        let mut buffer = [0.0f32; 1024];

        manager.fill_underrun_buffer(&mut buffer);

        // Verify all samples are non-zero (noise, not silence)
        let non_zero_count = buffer.iter().filter(|&&s| s != 0.0).count();
        assert!(
            non_zero_count > 0,
            "Underrun buffer should contain noise, not silence"
        );

        // Verify noise is at DAC keepalive level (~-96dB = 0.000016)
        // Allow for some variance since it's random
        let max_abs = buffer.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        assert!(
            max_abs < 0.001,
            "Noise should be at DAC keepalive level (~-96dB), got max amplitude {}",
            max_abs
        );
        assert!(
            max_abs > 0.0,
            "Should have some noise amplitude, got {}",
            max_abs
        );
    }

    #[test]
    fn underrun_noise_is_different_each_call() {
        // Test that LFSR noise is pseudo-random and different each call
        let mut manager = PlaybackManager::default();
        let mut buffer1 = [0.0f32; 256];
        let mut buffer2 = [0.0f32; 256];

        manager.fill_underrun_buffer(&mut buffer1);
        manager.fill_underrun_buffer(&mut buffer2);

        // Buffers should be different (not identical)
        let same_count = buffer1
            .iter()
            .zip(buffer2.iter())
            .filter(|(a, b)| (*a - *b).abs() < f32::EPSILON)
            .count();
        assert!(
            same_count < buffer1.len() / 2,
            "Sequential underrun buffers should be different (LFSR advancing)"
        );
    }

    #[test]
    fn maybe_emit_position_update_throttles_correctly() {
        // Test that position updates are throttled to ~250ms intervals
        let mut manager = PlaybackManager::default();

        // Setup: Load a track and set source
        manager.load_playlist(vec![create_test_track("1")], 0);
        let _ = manager.play();
        let source = Box::new(DummyAudioSource::new(Duration::from_secs(60), 48000));
        manager.set_audio_source(source);

        // Set sample rate (48kHz)
        manager.set_sample_rate(48000);

        // Drain any events from setup (play/load emit events)
        let _ = manager.drain_events();

        // Process less than 250ms worth of samples - should NOT emit position update
        // 250ms at 48kHz stereo = 48000 * 0.25 * 2 = 24000 samples
        manager.maybe_emit_position_update(10000);
        let events_mid = manager.drain_events();
        let position_updates_mid = events_mid
            .iter()
            .filter(|e| matches!(e, PlaybackEvent::PositionUpdate { .. }))
            .count();
        assert_eq!(
            position_updates_mid, 0,
            "Should not emit position update before threshold"
        );

        // Process enough to cross threshold
        manager.maybe_emit_position_update(20000); // Total: 30000 > 24000
        let events_after = manager.drain_events();

        // Should have emitted at least one position update
        let position_updates = events_after
            .iter()
            .filter(|e| matches!(e, PlaybackEvent::PositionUpdate { .. }))
            .count();
        assert!(
            position_updates >= 1,
            "Should emit position update after crossing threshold"
        );
    }

    #[test]
    fn reset_position_tracking_clears_state() {
        // Test that reset_position_tracking properly resets both flags
        let mut manager = PlaybackManager::default();

        // Set states to non-default values
        manager.source_ready_verified = true;
        manager.source_ready_wait_samples = 12345;

        // Reset
        manager.reset_position_tracking();

        // Verify reset
        assert!(
            !manager.source_ready_verified,
            "source_ready_verified should be false after reset"
        );
        assert_eq!(
            manager.source_ready_wait_samples, 0,
            "source_ready_wait_samples should be 0 after reset"
        );
    }

    // ===== Event Emission Timing Tests =====

    #[test]
    fn state_changed_suppresses_duplicates() {
        let mut manager = PlaybackManager::default();

        // Emit first state change
        manager.emit_state_changed(PlaybackState::Playing);
        let events = manager.drain_events();
        assert_eq!(events.len(), 1, "First state change should emit");

        // Emit same state again - should be suppressed
        manager.emit_state_changed(PlaybackState::Playing);
        let events = manager.drain_events();
        assert_eq!(
            events.len(),
            0,
            "Duplicate state change should be suppressed"
        );

        // Emit different state - should emit
        manager.emit_state_changed(PlaybackState::Paused);
        let events = manager.drain_events();
        assert_eq!(events.len(), 1, "Different state should emit");
    }

    #[test]
    fn stop_resets_last_emitted_state() {
        let mut manager = PlaybackManager::default();

        // Set up a track so play() has something to work with
        manager.add_to_queue_end(create_test_track("1"));
        let _ = manager.play();

        // Emit Playing state
        manager.emit_state_changed(PlaybackState::Playing);
        let _ = manager.drain_events();

        // Stop should reset last_emitted_state
        manager.stop();
        let _ = manager.drain_events();

        // Now Playing should emit again (not be suppressed)
        let _ = manager.play();
        manager.emit_state_changed(PlaybackState::Playing);
        let events = manager.drain_events();

        // Check for StateChanged event
        let has_state_changed = events.iter().any(|e| {
            matches!(
                e,
                PlaybackEvent::StateChanged {
                    state: crate::events::PlaybackStateEvent::Playing
                }
            )
        });
        assert!(
            has_state_changed,
            "Playing state should emit after stop reset"
        );
    }

    #[test]
    fn crossfade_progress_throttles_small_changes() {
        let mut manager = PlaybackManager::default();

        // Emit initial progress
        manager.emit_crossfade_progress(0.0, false);
        let events = manager.drain_events();
        assert_eq!(events.len(), 1, "Initial progress should emit");

        // Emit tiny progress change (< 5%) - should be suppressed
        manager.emit_crossfade_progress(0.01, false);
        let events = manager.drain_events();
        assert_eq!(
            events.len(),
            0,
            "Small progress change (1%) should be suppressed"
        );

        // Emit larger progress change (>= 5%) - should emit
        manager.emit_crossfade_progress(0.06, false);
        let events = manager.drain_events();
        assert_eq!(events.len(), 1, "Progress change >= 5% should emit");
    }

    #[test]
    fn crossfade_progress_always_emits_on_metadata_switch() {
        let mut manager = PlaybackManager::default();

        // Emit initial progress
        manager.emit_crossfade_progress(0.49, false);
        let _ = manager.drain_events();

        // Emit tiny progress change but with metadata_switched=true
        // Should emit despite small change
        manager.emit_crossfade_progress(0.50, true);
        let events = manager.drain_events();
        assert_eq!(
            events.len(),
            1,
            "Metadata switch should always emit regardless of progress delta"
        );

        // Verify it has metadata_switched=true
        match &events[0] {
            PlaybackEvent::CrossfadeProgress {
                metadata_switched, ..
            } => {
                assert!(*metadata_switched, "metadata_switched should be true");
            }
            _ => panic!("Expected CrossfadeProgress event"),
        }
    }

    #[test]
    fn event_queue_overflow_drops_oldest() {
        let mut manager = PlaybackManager::default();

        // Fill the event queue beyond MAX_PENDING_EVENTS
        for i in 0..(MAX_PENDING_EVENTS + 100) {
            manager.push_event(PlaybackEvent::Error {
                message: format!("error_{}", i),
            });
        }

        let events = manager.drain_events();

        // Should have dropped some events
        assert!(
            events.len() <= MAX_PENDING_EVENTS,
            "Event queue should not exceed max size"
        );

        // Should have kept recent events (dropped oldest)
        // The last event should be one of the later ones
        if let Some(PlaybackEvent::Error { message }) = events.last() {
            // Parse the index from the message
            let idx: usize = message.strip_prefix("error_").unwrap().parse().unwrap();
            assert!(
                idx > EVENT_OVERFLOW_DROP_COUNT,
                "Should have kept recent events, not oldest"
            );
        }
    }
}
