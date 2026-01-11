//! Playback manager - core orchestration
//!
//! Coordinates queue, history, volume, shuffle, and audio processing

use crate::{
    crossfade::{CrossfadeEngine, CrossfadeSettings, CrossfadeState, FadeCurve},
    error::{PlaybackError, Result},
    events::{CrossfadeProgressTracker, PlaybackEvent},
    history::History,
    queue::Queue,
    shuffle::shuffle_queue,
    source::AudioSource,
    types::{PlaybackConfig, PlaybackState, QueueTrack, RepeatMode, ShuffleMode},
    volume::Volume,
};

/// Start/resume fade envelope for click-free playback transitions
///
/// Applies a short fade-in when playback starts or resumes to prevent
/// audible clicks/pops from sudden amplitude changes.
///
/// **Key feature**: The fade is AMPLITUDE-TRIGGERED, not time-based.
/// It waits for actual audio content (amplitude > threshold) before
/// starting the fade. This handles MP3 encoder delay (~26ms of silence)
/// that would otherwise "waste" a time-based fade.
///
/// The envelope includes:
/// 1. Wait for audio detection - outputs zeros until signal detected
/// 2. Fade-in period (20ms) - gradual amplitude increase with S-curve
/// 3. DC blocker - removes any DC offset from decoded audio
///
/// The envelope is applied BEFORE volume and effects to ensure proper click prevention.
struct StartFadeEnvelope {
    /// Whether fade-in is currently active
    active: bool,

    /// Whether we've detected actual audio content yet
    audio_detected: bool,

    /// Current position in the fade (in stereo samples, starts after audio detected)
    position_samples: usize,

    /// Total duration of fade (in stereo samples)
    duration_samples: usize,

    /// Sample rate for duration calculations
    sample_rate: u32,

    /// DC blocker state (left channel)
    dc_blocker_prev_input_l: f32,
    dc_blocker_prev_output_l: f32,

    /// DC blocker state (right channel)
    dc_blocker_prev_input_r: f32,
    dc_blocker_prev_output_r: f32,

    /// Samples processed while waiting for audio (for timeout)
    wait_samples: usize,

    /// Maximum wait time before forcing fade start (in samples)
    max_wait_samples: usize,
}

/// Default fade-in duration in milliseconds
const START_FADE_DURATION_MS: u32 = 30;

/// Audio detection threshold - amplitude above this triggers fade start
/// Set to -34dB (0.02) to catch real musical content, not encoder delay junk
const AUDIO_DETECT_THRESHOLD: f32 = 0.02;

/// Maximum wait time for audio detection (ms) before forcing fade start
/// Handles edge case of tracks that start with genuine silence
const MAX_WAIT_MS: u32 = 200;

/// DC blocker coefficient (0.995-0.9999, higher = less bass removal but slower response)
const DC_BLOCKER_COEFF: f32 = 0.9975;

impl StartFadeEnvelope {
    /// Create a new start fade envelope
    fn new(sample_rate: u32) -> Self {
        Self {
            active: false,
            audio_detected: false,
            position_samples: 0,
            duration_samples: Self::calculate_duration_samples(sample_rate, START_FADE_DURATION_MS),
            sample_rate,
            dc_blocker_prev_input_l: 0.0,
            dc_blocker_prev_output_l: 0.0,
            dc_blocker_prev_input_r: 0.0,
            dc_blocker_prev_output_r: 0.0,
            wait_samples: 0,
            max_wait_samples: Self::calculate_duration_samples(sample_rate, MAX_WAIT_MS),
        }
    }

    /// Calculate duration in stereo samples from milliseconds
    #[inline]
    fn calculate_duration_samples(sample_rate: u32, duration_ms: u32) -> usize {
        // duration_samples = sample_rate * duration_ms / 1000 * 2 (stereo)
        ((sample_rate as u64 * duration_ms as u64 * 2) / 1000) as usize
    }

    /// Start a new fade-in
    #[inline]
    fn start(&mut self) {
        self.active = true;
        self.audio_detected = false;
        self.position_samples = 0;
        self.wait_samples = 0;
        // Reset DC blocker state for clean start
        self.dc_blocker_prev_input_l = 0.0;
        self.dc_blocker_prev_output_l = 0.0;
        self.dc_blocker_prev_input_r = 0.0;
        self.dc_blocker_prev_output_r = 0.0;
    }

    /// Reset the envelope (stop any active fade)
    #[inline]
    fn reset(&mut self) {
        self.active = false;
        self.audio_detected = false;
        self.position_samples = 0;
        self.wait_samples = 0;
    }

    /// Update sample rate and recalculate duration
    fn set_sample_rate(&mut self, sample_rate: u32) {
        if self.sample_rate != sample_rate {
            self.sample_rate = sample_rate;
            self.duration_samples =
                Self::calculate_duration_samples(sample_rate, START_FADE_DURATION_MS);
            self.max_wait_samples = Self::calculate_duration_samples(sample_rate, MAX_WAIT_MS);
        }
    }

    /// Check if fade is currently active
    #[inline]
    fn is_active(&self) -> bool {
        self.active
    }

    /// Apply DC blocker to remove DC offset (first-order highpass)
    /// Formula: y[n] = gain * (x[n] - x[n-1]) + beta * y[n-1]
    #[inline]
    fn dc_block_sample(&mut self, input_l: f32, input_r: f32) -> (f32, f32) {
        const GAIN: f32 = (1.0 + DC_BLOCKER_COEFF) / 2.0;

        let output_l = GAIN * (input_l - self.dc_blocker_prev_input_l)
            + DC_BLOCKER_COEFF * self.dc_blocker_prev_output_l;
        let output_r = GAIN * (input_r - self.dc_blocker_prev_input_r)
            + DC_BLOCKER_COEFF * self.dc_blocker_prev_output_r;

        self.dc_blocker_prev_input_l = input_l;
        self.dc_blocker_prev_output_l = output_l;
        self.dc_blocker_prev_input_r = input_r;
        self.dc_blocker_prev_output_r = output_r;

        (output_l, output_r)
    }

    /// Check if a sample pair contains actual audio content
    #[inline]
    fn is_audio_content(&self, left: f32, right: f32) -> bool {
        left.abs() > AUDIO_DETECT_THRESHOLD || right.abs() > AUDIO_DETECT_THRESHOLD
    }

    /// Apply fade envelope to audio buffer (in-place)
    ///
    /// AMPLITUDE-TRIGGERED fade:
    /// 1. Wait phase - outputs zeros until audio detected (amplitude > threshold)
    /// 2. Fade phase - gradual amplitude increase with S-curve
    /// 3. DC blocking throughout - removes any DC offset
    ///
    /// This handles MP3 encoder delay (~26ms silence) that would otherwise
    /// "waste" a time-based fade.
    ///
    /// MUST be called BEFORE volume/effects processing.
    ///
    /// Returns the number of samples processed.
    #[inline]
    fn process(&mut self, buffer: &mut [f32]) -> usize {
        if !self.active {
            return buffer.len();
        }

        // Debug: log first process call
        if self.wait_samples == 0 && self.position_samples == 0 {
            eprintln!(
                "[StartFade] Starting amplitude-triggered fade: fade duration {} samples ({:.1}ms), threshold {:.6}",
                self.duration_samples,
                self.duration_samples as f32 / (self.sample_rate as f32 * 2.0) * 1000.0,
                AUDIO_DETECT_THRESHOLD
            );
            if buffer.len() >= 4 {
                eprintln!(
                    "[StartFade] Input samples: [{:.6}, {:.6}, {:.6}, {:.6}]",
                    buffer[0], buffer[1], buffer[2], buffer[3]
                );
            }
        }

        // Process stereo frames (2 samples per frame)
        let frames = buffer.len() / 2;

        for frame in 0..frames {
            let left_idx = frame * 2;
            let right_idx = frame * 2 + 1;

            let input_l = buffer[left_idx];
            let input_r = buffer[right_idx];

            // Apply DC blocker first
            let (blocked_l, blocked_r) = self.dc_block_sample(input_l, input_r);

            if !self.audio_detected {
                // WAIT PHASE: Looking for actual audio content
                // Check if this sample has audio content OR if we've waited too long
                let timeout = self.wait_samples >= self.max_wait_samples;
                let has_audio = self.is_audio_content(blocked_l, blocked_r);

                if has_audio || timeout {
                    // Audio detected (or timeout)! Start the fade
                    self.audio_detected = true;
                    if has_audio {
                        eprintln!(
                            "[StartFade] Audio DETECTED at sample {}, amplitude: L={:.6} R={:.6}",
                            self.wait_samples, blocked_l.abs(), blocked_r.abs()
                        );
                    } else {
                        eprintln!(
                            "[StartFade] Timeout after {} samples ({:.1}ms), forcing fade start",
                            self.wait_samples,
                            self.wait_samples as f32 / (self.sample_rate as f32 * 2.0) * 1000.0
                        );
                    }
                    // Apply fade gain = 0 for first sample
                    buffer[left_idx] = 0.0;
                    buffer[right_idx] = 0.0;
                    self.position_samples = 2; // Next frame starts at position 2
                } else {
                    // Still waiting - output silence
                    buffer[left_idx] = 0.0;
                    buffer[right_idx] = 0.0;
                    self.wait_samples += 2;
                }
            } else {
                // FADE PHASE: Apply gradual fade-in
                let progress = self.position_samples as f32 / self.duration_samples as f32;

                if progress >= 1.0 {
                    // Fade complete - pass through with DC blocking only
                    buffer[left_idx] = blocked_l;
                    buffer[right_idx] = blocked_r;
                } else {
                    // S-curve: (1 - cos(π * t)) / 2 - smooth at start and end
                    let gain = (1.0 - (std::f32::consts::PI * progress).cos()) * 0.5;
                    buffer[left_idx] = blocked_l * gain;
                    buffer[right_idx] = blocked_r * gain;
                    self.position_samples += 2;
                }
            }
        }

        // Check if fade completed
        if self.audio_detected && self.position_samples >= self.duration_samples {
            self.active = false;
            eprintln!(
                "[StartFade] Fade COMPLETED: waited {} samples ({:.1}ms), faded {} samples ({:.1}ms)",
                self.wait_samples,
                self.wait_samples as f32 / (self.sample_rate as f32 * 2.0) * 1000.0,
                self.position_samples,
                self.position_samples as f32 / (self.sample_rate as f32 * 2.0) * 1000.0
            );
        }

        buffer.len()
    }
}

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
pub struct PlaybackManager {
    // State
    state: PlaybackState,
    current_track: Option<QueueTrack>,

    // Queue and history
    queue: Queue,
    history: History,

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

    // Pre-allocated buffers for crossfade (to avoid allocation in audio callback)
    outgoing_buffer: Vec<f32>,
    incoming_buffer: Vec<f32>,

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
}

/// Default buffer size for crossfade (10 seconds at max supported sample rate 192kHz stereo)
/// This ensures crossfade works correctly at all sample rates up to 192kHz
const CROSSFADE_BUFFER_SIZE: usize = 10 * 192000 * 2;

/// Maximum stereo buffer size for channel conversion (8192 frames * 2 channels)
/// This covers typical audio callback buffer sizes (256-4096 frames)
const MAX_STEREO_BUFFER_SIZE: usize = 8192 * 2;

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
            current_track: None,
            queue: Queue::new(),
            history: History::new(config.history_size),
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
            outgoing_buffer: vec![0.0; CROSSFADE_BUFFER_SIZE],
            incoming_buffer: vec![0.0; CROSSFADE_BUFFER_SIZE],
            stereo_conversion_buffer: vec![0.0; MAX_STEREO_BUFFER_SIZE],
            sample_rate: 44100, // Default, will be updated by platform
            output_channels: 2, // Default stereo, will be updated by platform
            is_manual_skip: false,
            pending_events: Vec::new(),
            crossfade_progress: CrossfadeProgressTracker::new(),
            start_fade: StartFadeEnvelope::new(44100), // Will be updated by set_sample_rate
        }
    }

    // ===== Playback Control =====

    /// Start or resume playback
    pub fn play(&mut self) -> Result<()> {
        match self.state {
            PlaybackState::Paused => {
                // Resume from pause
                self.state = PlaybackState::Playing;
                // Start fade-in for click-free resume
                self.start_fade.start();
                self.emit_state_changed(PlaybackState::Playing);
                Ok(())
            }
            PlaybackState::Stopped | PlaybackState::Loading => {
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
        if self.state == PlaybackState::Playing {
            self.state = PlaybackState::Paused;
            self.emit_state_changed(PlaybackState::Paused);
        }
    }

    /// Stop playback
    ///
    /// Stops playback and clears current track (but not queue)
    pub fn stop(&mut self) {
        self.state = PlaybackState::Stopped;
        self.current_track = None;
        self.audio_source = None;
        self.next_source = None;
        self.next_track = None;
        self.crossfade.reset();
        self.crossfade_progress.reset();
        self.is_manual_skip = false;
        self.emit_state_changed(PlaybackState::Stopped);
    }

    /// Skip to next track
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Result<()> {
        self.is_manual_skip = true;

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

        // Get next track from queue
        let next_track = self.get_next_track_from_queue()?;

        // Save current track to history
        if let Some(track) = self.current_track.take() {
            self.history.push(track);
        }

        // Load next track
        self.current_track = Some(next_track);
        self.state = PlaybackState::Loading;
        // Platform will need to call load_current_track()

        Ok(())
    }

    /// Get next track considering repeat mode
    fn get_next_track_from_queue(&mut self) -> Result<QueueTrack> {
        if let Some(track) = self.queue.pop_next() {
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
        if let Some(ref mut source) = self.audio_source {
            source.seek(position)?;
            // Start fade-in for click-free seek
            self.start_fade.start();
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
        if index >= self.queue.len() {
            return Err(PlaybackError::QueueEmpty);
        }

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
    pub fn set_shuffle(&mut self, mode: ShuffleMode) {
        if self.shuffle == mode {
            return;
        }

        let old_mode = self.shuffle;
        self.shuffle = mode;

        match mode {
            ShuffleMode::Off => {
                // Restore original order
                self.queue.restore_original_order();
            }
            ShuffleMode::Random | ShuffleMode::Smart => {
                // Apply shuffle to source queue
                if old_mode == ShuffleMode::Off {
                    // Save current order before shuffling
                    self.queue.update_original_source();
                }

                let source = self.queue.source_mut();
                shuffle_queue(source, mode);
                self.queue.set_shuffled(true);

                // Remove consecutive duplicates after shuffling
                self.queue.remove_consecutive_duplicates();
            }
        }
    }

    /// Get current shuffle mode
    pub fn get_shuffle(&self) -> ShuffleMode {
        self.shuffle
    }

    /// Set repeat mode
    pub fn set_repeat(&mut self, mode: RepeatMode) {
        self.repeat = mode;
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
        !self.queue.is_empty() || self.repeat == RepeatMode::One
    }

    /// Check if there is a previous track
    pub fn has_previous(&self) -> bool {
        !self.history.get_all().is_empty() || self.repeat == RepeatMode::One
    }

    /// Peek at the next track in queue without advancing
    ///
    /// Returns the next track that would play when current track finishes.
    /// Used by platform code to pre-load the next track for crossfade/gapless.
    pub fn peek_next_queue_track(&self) -> Option<&QueueTrack> {
        // If repeat one is enabled, return current track
        if self.repeat == RepeatMode::One {
            return self.current_track.as_ref();
        }

        // Otherwise peek at the queue
        if let Some(track) = self.queue.peek_next() {
            Some(track)
        } else if self.repeat == RepeatMode::All && !self.queue.is_empty() {
            // If queue is empty but repeat all, would loop back to first track
            // For pre-loading purposes, we don't handle this case
            None
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
            eprintln!("[process_audio] Call #{}", count + 1);
            eprintln!("  - Output buffer size: {} samples", output.len());
            eprintln!("  - Output channels: {}", self.output_channels);
            eprintln!(
                "  - Expected frames: {}",
                output.len() / self.output_channels as usize
            );
            eprintln!("  - Sample rate: {} Hz", self.sample_rate);
        }

        if self.state != PlaybackState::Playing {
            // Not playing - output silence
            output.fill(0.0);
            return Ok(output.len());
        }

        let Some(ref mut source) = self.audio_source else {
            // No audio source - output silence
            output.fill(0.0);
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
                // Track finished
                self.handle_track_finished()?;
                return Ok(0);
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
            for i in 0..frames {
                let left = self.stereo_conversion_buffer[i * 2];
                let right = self.stereo_conversion_buffer[i * 2 + 1];
                output[i] = (left + right) * 0.5; // Average and write to mono output
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

            Ok(frames)
        } else if self.output_channels == 2 {
            // Stereo output - with crossfade support
            let samples_read = self.process_stereo_with_crossfade(output)?;

            if samples_read == 0 {
                // Track finished (no crossfade or crossfade completed)
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

            Ok(frames_read * self.output_channels as usize)
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

        // Normal playback - check if we should start crossfade
        let source = self
            .audio_source
            .as_mut()
            .ok_or(PlaybackError::NoTrackLoaded)?;

        // Check if we're approaching the crossfade window
        let position = source.position();
        let duration = source.duration();
        let crossfade_duration_ms = self.crossfade.settings().duration_ms;
        let crossfade_duration = Duration::from_millis(crossfade_duration_ms as u64);
        let remaining = duration.saturating_sub(position);

        // Should we start crossfade?
        let should_crossfade = self.crossfade.settings().enabled
            && self.next_source.is_some()
            && remaining <= crossfade_duration;

        if should_crossfade {
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

                self.crossfade_progress
                    .start(from_track_id.clone(), to_track_id.clone(), crossfade_duration_ms);
                self.emit_crossfade_started(from_track_id, to_track_id, crossfade_duration_ms);

                return self.process_active_crossfade(output);
            }
        }

        // Check for gapless transition (crossfade disabled but gapless enabled)
        let should_gapless = !self.crossfade.settings().enabled
            && self.gapless_enabled
            && self.next_source.is_some();

        // Normal playback
        let samples_read = source.read_samples(output)?;

        if samples_read == 0 {
            // Track finished
            if should_gapless {
                // Seamless transition to next track
                self.transition_to_next_track()?;
                // Try to read from new source
                if let Some(ref mut new_source) = self.audio_source {
                    return new_source.read_samples(output);
                }
            }
            return Ok(0);
        }

        Ok(samples_read)
    }

    /// Process audio during active crossfade
    fn process_active_crossfade(&mut self, output: &mut [f32]) -> Result<usize> {
        let buffer_len = output.len();

        // Read from outgoing (current) track
        let outgoing_samples = if let Some(ref mut source) = self.audio_source {
            let len = buffer_len.min(self.outgoing_buffer.len());
            source
                .read_samples(&mut self.outgoing_buffer[..len])
                .unwrap_or(0)
        } else {
            // Fill with silence if no outgoing source
            self.outgoing_buffer[..buffer_len].fill(0.0);
            buffer_len
        };

        // Read from incoming (next) track
        let incoming_samples = if let Some(ref mut source) = self.next_source {
            let len = buffer_len.min(self.incoming_buffer.len());
            source
                .read_samples(&mut self.incoming_buffer[..len])
                .unwrap_or(0)
        } else {
            // Fill with silence if no incoming source
            self.incoming_buffer[..buffer_len].fill(0.0);
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
            &self.outgoing_buffer[..samples_to_process],
            &self.incoming_buffer[..samples_to_process],
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
        }

        Ok(processed)
    }

    /// Transition from current track to next track
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

        // Emit track changed for gapless (non-crossfade) transitions
        // Note: For crossfade, TrackChanged is emitted at 50% in process_active_crossfade
        if !self.crossfade_progress.is_active() {
            if let Some(track_id) = next_track_id {
                self.emit_track_changed(track_id, previous_track_id);
            }
        }

        // Reset loudness normalizer for new track
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
    pub fn set_audio_source(&mut self, source: Box<dyn AudioSource>) {
        let previous_track_id = self.current_track.as_ref().map(|t| t.id.clone());

        // IMPORTANT: Start fade BEFORE setting audio source to prevent race condition
        // where audio callback reads samples before fade is active
        self.start_fade.start();

        self.audio_source = Some(source);
        self.state = PlaybackState::Playing;
        self.is_manual_skip = false;

        // Emit track changed event (for non-crossfade transitions)
        if let Some(ref track) = self.current_track {
            self.emit_track_changed(track.id.clone(), previous_track_id);
        }
        self.emit_state_changed(PlaybackState::Playing);
    }

    // ===== Crossfade Settings =====

    /// Set crossfade settings
    pub fn set_crossfade_settings(&mut self, settings: CrossfadeSettings) {
        self.crossfade.set_settings(settings);
    }

    /// Get current crossfade settings
    pub fn get_crossfade_settings(&self) -> &CrossfadeSettings {
        self.crossfade.settings()
    }

    /// Enable or disable crossfade
    pub fn set_crossfade_enabled(&mut self, enabled: bool) {
        let mut settings = self.crossfade.settings().clone();
        settings.enabled = enabled;
        self.crossfade.set_settings(settings);
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
    /// Called by platform when pre-decoding the next track
    pub fn set_next_source(&mut self, source: Box<dyn AudioSource>, track: QueueTrack) {
        let track_id = track.id.clone();
        self.next_source = Some(source);
        self.next_track = Some(track);
        self.emit_next_track_prepared(track_id);
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
            Some(remaining - crossfade_duration)
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

    /// Emit a state changed event
    fn emit_state_changed(&mut self, state: PlaybackState) {
        self.pending_events.push(PlaybackEvent::StateChanged {
            state: state.into(),
        });
    }

    /// Emit a track changed event
    fn emit_track_changed(&mut self, track_id: String, previous_track_id: Option<String>) {
        self.pending_events.push(PlaybackEvent::TrackChanged {
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
        self.pending_events.push(PlaybackEvent::CrossfadeStarted {
            from_track_id,
            to_track_id,
            duration_ms,
        });
    }

    /// Emit a crossfade progress event
    fn emit_crossfade_progress(&mut self, progress: f32, metadata_switched: bool) {
        self.pending_events.push(PlaybackEvent::CrossfadeProgress {
            progress,
            metadata_switched,
        });
    }

    /// Emit a crossfade completed event
    fn emit_crossfade_completed(&mut self) {
        self.pending_events.push(PlaybackEvent::CrossfadeCompleted);
    }

    /// Emit a track finished event
    fn emit_track_finished(&mut self, track_id: String) {
        self.pending_events.push(PlaybackEvent::TrackFinished { track_id });
    }

    /// Emit a volume changed event
    fn emit_volume_changed(&mut self) {
        self.pending_events.push(PlaybackEvent::VolumeChanged {
            level: self.volume.level(),
            is_muted: self.volume.is_muted(),
        });
    }

    /// Emit a queue changed event
    fn emit_queue_changed(&mut self) {
        self.pending_events.push(PlaybackEvent::QueueChanged {
            length: self.queue.len(),
        });
    }

    /// Emit an error event
    fn emit_error(&mut self, message: String) {
        self.pending_events.push(PlaybackEvent::Error { message });
    }

    /// Emit a next track prepared event
    fn emit_next_track_prepared(&mut self, track_id: String) {
        self.pending_events
            .push(PlaybackEvent::NextTrackPrepared { track_id });
    }

    /// Emit a position update event
    pub fn emit_position_update(&mut self) {
        if let Some(ref source) = self.audio_source {
            self.pending_events.push(PlaybackEvent::PositionUpdate {
                position_ms: source.position().as_millis() as u64,
                duration_ms: source.duration().as_millis() as u64,
            });
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

        // Should output silence
        assert_eq!(buffer[0], 0.0);
        assert_eq!(buffer[1023], 0.0);
    }

    #[test]
    fn set_audio_source_changes_state() {
        let mut manager = PlaybackManager::default();

        let source = Box::new(DummyAudioSource::new(Duration::from_secs(10), 44100));
        manager.set_audio_source(source);

        assert_eq!(manager.get_state(), PlaybackState::Playing);
    }
}
