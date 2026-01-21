//! Playback manager - core orchestration
//!
//! Coordinates queue, history, volume, shuffle, and audio processing

use crate::{
    crossfade::{CrossfadeEngine, CrossfadeSettings, CrossfadeState, FadeCurve},
    error::{PlaybackError, Result},
    events::{CrossfadeProgressTracker, PlaybackEvent},
    history::History,
    lazy_queue::LazyQueueState,
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

    /// Whether the fade is frozen at current gain (prevents progression)
    frozen: bool,

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

    /// Simple noise state for DAC keep-alive during wait phase
    /// Alternating low-level noise prevents DAC from entering power-save mode
    noise_state: u32,
}

/// Default fade-in duration in milliseconds
const START_FADE_DURATION_MS: u32 = 30;

/// Default fade-out duration in milliseconds (100ms for smooth, natural-sounding pause)
const STOP_FADE_DURATION_MS: u32 = 100;

/// Audio detection threshold - amplitude above this triggers fade start
/// Set to -60dB (0.001) to catch very quiet intros while filtering encoder noise
/// Previous values: 0.02 (-34dB) too high, 0.003 (-50dB) still missed quiet content
const AUDIO_DETECT_THRESHOLD: f32 = 0.001; // -60dB

/// Maximum wait time for audio detection (ms) before forcing fade start
/// Handles edge case of tracks that start with genuine silence
const MAX_WAIT_MS: u32 = 200;

/// DC blocker coefficient (0.995-0.9999, higher = less bass removal but slower response)
const DC_BLOCKER_COEFF: f32 = 0.9975;

/// Low-level noise amplitude for DAC keep-alive during wait phase
/// -96dB (0.000016) is inaudible but keeps DAC circuitry active
const DAC_KEEPALIVE_NOISE: f32 = 0.000016;

/// Stop/transition fade envelope for click-free playback transitions
///
/// Applies a short fade-out when playback stops or transitions to prevent
/// audible clicks from sudden amplitude drops.
///
/// This is the INVERSE of StartFadeEnvelope and works symmetrically.
struct StopFadeEnvelope {
    /// Whether fade-out is currently active
    active: bool,

    /// Current position in the fade (in stereo samples)
    position_samples: usize,

    /// Total duration of fade (in stereo samples)
    duration_samples: usize,

    /// Sample rate for duration calculations
    sample_rate: u32,

    /// Callback to execute when fade completes (deferred action)
    /// This allows us to finish the fade before changing state
    fade_complete_action: FadeCompleteAction,
}

/// Action to perform when stop fade completes
#[derive(Clone, Copy, Debug, PartialEq)]
enum FadeCompleteAction {
    /// No action needed
    None,
    /// Transition to next track after fade
    TransitionToNext,
    /// Stop playback completely
    Stop,
    /// Pause playback
    Pause,
}

impl StopFadeEnvelope {
    /// Create a new stop fade envelope
    fn new(sample_rate: u32) -> Self {
        Self {
            active: false,
            position_samples: 0,
            duration_samples: Self::calculate_duration_samples(sample_rate, STOP_FADE_DURATION_MS),
            sample_rate,
            fade_complete_action: FadeCompleteAction::None,
        }
    }

    /// Calculate duration in stereo samples from milliseconds
    #[inline]
    fn calculate_duration_samples(sample_rate: u32, duration_ms: u32) -> usize {
        ((sample_rate as u64 * duration_ms as u64 * 2) / 1000) as usize
    }

    /// Start a fade-out with specified completion action
    #[inline]
    fn start(&mut self, action: FadeCompleteAction) {
        self.active = true;
        self.position_samples = 0;
        self.fade_complete_action = action;
    }

    /// Reset the envelope (cancel any active fade)
    #[inline]
    fn reset(&mut self) {
        self.active = false;
        self.position_samples = 0;
        self.fade_complete_action = FadeCompleteAction::None;
    }

    /// Update sample rate and recalculate duration
    fn set_sample_rate(&mut self, sample_rate: u32) {
        if self.sample_rate != sample_rate {
            self.sample_rate = sample_rate;
            self.duration_samples =
                Self::calculate_duration_samples(sample_rate, STOP_FADE_DURATION_MS);
        }
    }

    /// Check if fade is currently active
    #[inline]
    fn is_active(&self) -> bool {
        self.active
    }

    /// Apply fade-out envelope to audio buffer (in-place)
    ///
    /// Returns the FadeCompleteAction when fade finishes, None if still fading
    #[inline]
    fn process(&mut self, buffer: &mut [f32]) -> Option<FadeCompleteAction> {
        if !self.active {
            return None;
        }

        // Process stereo frames (2 samples per frame)
        let frames = buffer.len() / 2;

        for frame in 0..frames {
            let left_idx = frame * 2;
            let right_idx = frame * 2 + 1;

            let progress = self.position_samples as f32 / self.duration_samples as f32;

            if progress >= 1.0 {
                // Fade complete - output silence
                buffer[left_idx] = 0.0;
                buffer[right_idx] = 0.0;
            } else {
                // Inverse S-curve: starts at 1.0, ends at 0.0
                let gain = (1.0 + (std::f32::consts::PI * progress).cos()) * 0.5;
                buffer[left_idx] *= gain;
                buffer[right_idx] *= gain;
                self.position_samples += 2;
            }
        }

        // Check if fade completed
        if self.position_samples >= self.duration_samples {
            self.active = false;
            let action = self.fade_complete_action;
            self.fade_complete_action = FadeCompleteAction::None;
            return Some(action);
        }

        None
    }
}

impl StartFadeEnvelope {
    /// Create a new start fade envelope
    fn new(sample_rate: u32) -> Self {
        Self {
            active: false,
            frozen: false,
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
            noise_state: 0xACE1,
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
        self.frozen = false;
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
        self.frozen = false;
        self.audio_detected = false;
        self.position_samples = 0;
        self.wait_samples = 0;
    }

    /// Freeze the envelope at current gain (prevents further fade-in)
    /// Used when pause is clicked during fade-in to prevent volume spike
    #[inline]
    fn freeze(&mut self) {
        self.frozen = true;
        // Keep active=true so the fade continues to be applied
        // Keep position_samples constant to maintain current gain when combined with stop_fade
        // audio_detected stays as-is (already detected if we're fading)
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
        const GAIN: f32 = f32::midpoint(1.0, DC_BLOCKER_COEFF);

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
    fn is_audio_content(left: f32, right: f32) -> bool {
        left.abs() > AUDIO_DETECT_THRESHOLD || right.abs() > AUDIO_DETECT_THRESHOLD
    }

    /// Generate low-level noise for DAC keep-alive
    /// Uses simple LFSR for uncorrelated L/R noise
    #[inline]
    fn keepalive_noise(&mut self) -> (f32, f32) {
        // Simple LFSR for pseudo-random noise
        self.noise_state ^= self.noise_state << 13;
        self.noise_state ^= self.noise_state >> 17;
        self.noise_state ^= self.noise_state << 5;

        // Convert to bipolar noise in range [-1, 1] then scale to keep-alive level
        let noise_l = ((self.noise_state & 0xFFFF) as f32 / 32768.0 - 1.0) * DAC_KEEPALIVE_NOISE;
        self.noise_state ^= self.noise_state << 13;
        self.noise_state ^= self.noise_state >> 17;
        self.noise_state ^= self.noise_state << 5;
        let noise_r = ((self.noise_state & 0xFFFF) as f32 / 32768.0 - 1.0) * DAC_KEEPALIVE_NOISE;

        (noise_l, noise_r)
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

        // Debug: log first process call with detailed sample analysis
        if self.wait_samples == 0 && self.position_samples == 0 {
            tracing::debug!(
                "[StartFade] Starting amplitude-triggered fade: fade duration {} samples ({:.1}ms), threshold {:.6}",
                self.duration_samples,
                self.duration_samples as f32 / (self.sample_rate as f32 * 2.0) * 1000.0,
                AUDIO_DETECT_THRESHOLD
            );

            // Log first 20 samples to see resampler ramp-up pattern
            let samples_to_log = buffer.len().min(40);
            if samples_to_log >= 4 {
                tracing::debug!("[StartFade] First {} input samples:", samples_to_log);
                for i in (0..samples_to_log).step_by(4) {
                    if i + 3 < buffer.len() {
                        tracing::debug!(
                            "  [{:3}..{:3}]: L={:+.6} R={:+.6} | L={:+.6} R={:+.6}",
                            i,
                            i + 3,
                            buffer[i],
                            buffer[i + 1],
                            buffer[i + 2],
                            buffer[i + 3]
                        );
                    }
                }

                // Find max amplitude in first callback
                let max_amp = buffer
                    .iter()
                    .take(samples_to_log)
                    .map(|s| s.abs())
                    .fold(0.0f32, f32::max);
                tracing::debug!(
                    "[StartFade] Max amplitude in first {} samples: {:.6} (threshold: {:.6})",
                    samples_to_log,
                    max_amp,
                    AUDIO_DETECT_THRESHOLD
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

            if self.audio_detected {
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
                    // Only increment position if not frozen (maintains constant gain during pause)
                    if !self.frozen {
                        self.position_samples += 2;
                    }
                }
            } else {
                // WAIT PHASE: Looking for actual audio content
                // Check if this sample has audio content OR if we've waited too long
                let timeout = self.wait_samples >= self.max_wait_samples;
                let has_audio = Self::is_audio_content(blocked_l, blocked_r);

                if has_audio || timeout {
                    // Audio detected (or timeout)! Start the fade
                    self.audio_detected = true;
                    if has_audio {
                        tracing::debug!(
                            "[StartFade] Audio DETECTED at sample {}, amplitude: L={:.6} R={:.6}",
                            self.wait_samples,
                            blocked_l.abs(),
                            blocked_r.abs()
                        );
                    } else {
                        tracing::debug!(
                            "[StartFade] Timeout after {} samples ({:.1}ms), forcing fade start",
                            self.wait_samples,
                            self.wait_samples as f32 / (self.sample_rate as f32 * 2.0) * 1000.0
                        );
                    }
                    // Apply fade gain = 0 for first sample (true zero for clean fade start)
                    buffer[left_idx] = 0.0;
                    buffer[right_idx] = 0.0;
                    self.position_samples = 2; // Next frame starts at position 2
                } else {
                    // Still waiting - output low-level noise to keep DAC active
                    // This prevents DAC power-save mode which causes pops on wake
                    let (noise_l, noise_r) = self.keepalive_noise();
                    buffer[left_idx] = noise_l;
                    buffer[right_idx] = noise_r;
                    self.wait_samples += 2;
                }
            }
        }

        // Check if fade completed
        if self.audio_detected && self.position_samples >= self.duration_samples {
            self.active = false;
            tracing::debug!(
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
            pending_events: Vec::new(),
            crossfade_progress: CrossfadeProgressTracker::new(),
            start_fade: StartFadeEnvelope::new(44100), // Will be updated by set_sample_rate
            stop_fade: StopFadeEnvelope::new(44100),   // Will be updated by set_sample_rate
            pending_source: None,
            underrun_noise_state: 0xDEAD_BEEF, // Seed for LFSR noise generator
            source_ready_verified: false,
            source_ready_wait_samples: 0,
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
        self.is_manual_skip = false;
        self.emit_state_changed(PlaybackState::Stopped);
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
        if let Some(ref mut source) = self.audio_source {
            source.seek(position)?;

            // CRITICAL: Mark source as not ready after seek
            // This prevents the audio callback from reading 0 samples and thinking track finished
            // We wait for source.is_ready() before continuing playback (same as track load)
            self.source_ready_verified = false;
            self.source_ready_wait_samples = 0;

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
        if start_index > 0 && start_index < self.queue.get_source_total() {
            let _ = self.queue.skip_to_index(start_index);
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

                self.crossfade_progress.start(
                    from_track_id.clone(),
                    to_track_id.clone(),
                    crossfade_duration_ms,
                );
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
            // CRITICAL: Check if track actually finished or just buffering
            // After seek, source may return 0 samples temporarily while buffering
            if position >= duration {
                // Track actually finished - position at or past duration
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
    fn process_active_crossfade(&mut self, output: &mut [f32]) -> Result<usize> {
        // Ensure buffers are allocated before processing crossfade
        // This is safe because we're in the settings/state transition path, not the audio callback
        self.ensure_crossfade_buffers_allocated();

        let buffer_len = output.len();

        // Get mutable references to the buffers (guaranteed to exist after allocation)
        let outgoing_buffer = self
            .outgoing_buffer
            .as_mut()
            .expect("Buffer should be allocated");
        let incoming_buffer = self
            .incoming_buffer
            .as_mut()
            .expect("Buffer should be allocated");

        // Read from outgoing (current) track
        let outgoing_samples = if let Some(ref mut source) = self.audio_source {
            let len = buffer_len.min(outgoing_buffer.len());
            source
                .read_samples(&mut outgoing_buffer[..len])
                .unwrap_or(0)
        } else {
            // Fill with silence if no outgoing source
            outgoing_buffer[..buffer_len].fill(0.0);
            buffer_len
        };

        // Read from incoming (next) track
        let incoming_samples = if let Some(ref mut source) = self.next_source {
            let len = buffer_len.min(incoming_buffer.len());
            source
                .read_samples(&mut incoming_buffer[..len])
                .unwrap_or(0)
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
        self.stop_fade.set_sample_rate(sample_rate);
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
        self.pending_events
            .push(PlaybackEvent::TrackFinished { track_id });
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
}
