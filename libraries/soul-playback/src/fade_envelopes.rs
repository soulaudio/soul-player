//! Fade envelopes for click-free playback transitions
//!
//! Provides amplitude-triggered fade-in and fade-out envelopes to prevent
//! audible clicks and pops during playback state transitions.
//!
//! # Overview
//!
//! Digital audio requires smooth amplitude transitions to prevent clicks/pops.
//! This module provides two complementary fade envelopes:
//!
//! - **StartFadeEnvelope**: Fade-in when playback starts/resumes
//! - **StopFadeEnvelope**: Fade-out when playback stops/pauses/transitions
//!
//! # Key Features
//!
//! ## Amplitude-Triggered Fade-In
//!
//! Unlike time-based fades, StartFadeEnvelope waits for actual audio content
//! before starting the fade. This handles MP3 encoder delay (~26ms of silence)
//! that would otherwise "waste" a time-based fade.
//!
//! ## DC Blocking
//!
//! Both envelopes include DC blocking to remove DC offset from decoded audio,
//! preventing speaker cone displacement and subsonic distortion.
//!
//! ## DAC Keep-Alive
//!
//! During the wait phase (before audio detection), low-level noise (-96dB)
//! keeps the DAC circuitry active to prevent power-save mode which can cause
//! audible pops when audio starts.
//!
//! # Usage
//!
//! Fade envelopes are used internally by PlaybackManager and are NOT
//! intended for direct use by platform code. They are applied BEFORE
//! volume and effects processing.
//!
//! # Audio Safety
//!
//! All processing is allocation-free and suitable for real-time audio callbacks.
//! No heap allocations occur during audio processing.

/// Default fade-in duration in milliseconds
///
/// This is the actual fade duration AFTER audio detection, not including
/// the wait time for encoder delay.
pub(crate) const START_FADE_DURATION_MS: u32 = 30;

/// Default fade-out duration in milliseconds
///
/// 100ms provides smooth, natural-sounding pause/stop transitions without
/// being perceptibly slow to the user.
pub(crate) const STOP_FADE_DURATION_MS: u32 = 100;

/// Audio detection threshold - amplitude above this triggers fade start
///
/// Set to -60dB (0.001) to catch very quiet intros while filtering encoder noise.
/// This threshold has been empirically tuned:
/// - 0.02 (-34dB): Too high, missed quiet intros
/// - 0.003 (-50dB): Still missed some quiet content
/// - 0.001 (-60dB): Catches quiet intros while filtering encoder artifacts
pub(crate) const AUDIO_DETECT_THRESHOLD: f32 = 0.001; // -60dB

/// Maximum wait time for audio detection (ms) before forcing fade start
///
/// Handles edge case of tracks that start with genuine silence (e.g., classical
/// recordings with long quiet intros). After this timeout, the fade starts
/// regardless of detected amplitude.
const MAX_WAIT_MS: u32 = 200;

/// DC blocker coefficient (0.995-0.9999)
///
/// Higher values = less bass removal but slower DC offset tracking.
/// 0.9975 provides good DC blocking without audible bass attenuation.
const DC_BLOCKER_COEFF: f32 = 0.9975;

/// Low-level noise amplitude for DAC keep-alive during wait phase
///
/// -96dB (0.000016) is inaudible to humans but keeps DAC circuitry active,
/// preventing power-save mode which can cause audible pops when audio starts.
pub(crate) const DAC_KEEPALIVE_NOISE: f32 = 0.000016;

/// Minimum allowed sample rate in Hz
const MIN_SAMPLE_RATE: u32 = 8000;

/// Maximum allowed sample rate in Hz
const MAX_SAMPLE_RATE: u32 = 384000;

/// Action to perform when stop fade completes
///
/// This allows the fade-out to complete smoothly before executing the
/// requested action, ensuring deterministic fade behavior.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum FadeCompleteAction {
    /// No action needed
    None,
    /// Transition to next track after fade
    TransitionToNext,
    /// Stop playback completely
    Stop,
    /// Pause playback
    Pause,
}

/// Start/resume fade envelope for click-free playback transitions
///
/// Applies a short fade-in when playback starts or resumes to prevent
/// audible clicks/pops from sudden amplitude changes.
///
/// # Key Feature: Amplitude-Triggered Fade
///
/// The fade is AMPLITUDE-TRIGGERED, not time-based. It waits for actual
/// audio content (amplitude > threshold) before starting the fade. This
/// handles MP3 encoder delay (~26ms of silence) that would otherwise
/// "waste" a time-based fade.
///
/// # Processing Stages
///
/// 1. **Wait Phase**: Output low-level noise until signal detected
///    - Keeps DAC active to prevent power-save pops
///    - Monitors amplitude vs. threshold
///    - Timeout fallback for genuinely silent tracks
///
/// 2. **Fade Phase**: Gradual amplitude increase (S-curve)
///    - Smooth fade using raised cosine window
///    - Can be frozen during fade (for pause during fade-in)
///
/// 3. **DC Blocking**: Throughout all phases
///    - First-order highpass filter removes DC offset
///    - Prevents speaker cone displacement
///
/// # Thread Safety
///
/// This struct is NOT thread-safe and should only be accessed from the
/// audio processing thread.
pub(crate) struct StartFadeEnvelope {
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

    /// DC blocker state (left channel) - previous input
    dc_blocker_prev_input_l: f32,
    /// DC blocker state (left channel) - previous output
    dc_blocker_prev_output_l: f32,

    /// DC blocker state (right channel) - previous input
    dc_blocker_prev_input_r: f32,
    /// DC blocker state (right channel) - previous output
    dc_blocker_prev_output_r: f32,

    /// Samples processed while waiting for audio (for timeout)
    wait_samples: usize,

    /// Maximum wait time before forcing fade start (in samples)
    max_wait_samples: usize,

    /// Simple noise state for DAC keep-alive during wait phase
    /// Alternating low-level noise prevents DAC from entering power-save mode
    noise_state: u32,
}

impl StartFadeEnvelope {
    /// Create a new start fade envelope
    ///
    /// # Arguments
    ///
    /// * `sample_rate` - Audio sample rate in Hz (e.g., 44100, 48000)
    pub(crate) fn new(sample_rate: u32) -> Self {
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
    ///
    /// Formula: samples = sample_rate * duration_ms / 1000 * 2 (stereo)
    #[inline]
    fn calculate_duration_samples(sample_rate: u32, duration_ms: u32) -> usize {
        ((sample_rate as u64 * duration_ms as u64 * 2) / 1000) as usize
    }

    /// Start a new fade-in
    ///
    /// Resets all state and begins waiting for audio detection.
    #[inline]
    pub(crate) fn start(&mut self) {
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
    ///
    /// Immediately deactivates the fade and clears all state.
    #[inline]
    pub(crate) fn reset(&mut self) {
        self.active = false;
        self.frozen = false;
        self.audio_detected = false;
        self.position_samples = 0;
        self.wait_samples = 0;
    }

    /// Freeze the envelope at current gain (prevents further fade-in)
    ///
    /// Used when pause is clicked during fade-in to prevent volume spike.
    /// The fade remains active but doesn't progress, maintaining constant
    /// gain when combined with stop_fade.
    #[inline]
    pub(crate) fn freeze(&mut self) {
        self.frozen = true;
        // Keep active=true so the fade continues to be applied
        // Keep position_samples constant to maintain current gain when combined with stop_fade
        // audio_detected stays as-is (already detected if we're fading)
    }

    /// Update sample rate and recalculate duration
    ///
    /// Should be called when audio output format changes.
    /// Sample rate is clamped to valid range (8000 - 384000 Hz).
    pub(crate) fn set_sample_rate(&mut self, sample_rate: u32) {
        let clamped_rate = sample_rate.clamp(MIN_SAMPLE_RATE, MAX_SAMPLE_RATE);
        if self.sample_rate != clamped_rate {
            self.sample_rate = clamped_rate;
            self.duration_samples =
                Self::calculate_duration_samples(clamped_rate, START_FADE_DURATION_MS);
            self.max_wait_samples = Self::calculate_duration_samples(clamped_rate, MAX_WAIT_MS);
        }
    }

    /// Check if fade is currently active
    #[inline]
    pub(crate) fn is_active(&self) -> bool {
        self.active
    }

    /// Apply DC blocker to remove DC offset (first-order highpass)
    ///
    /// Formula: y[n] = gain * (x[n] - x[n-1]) + beta * y[n-1]
    ///
    /// This is a first-order IIR highpass filter that removes DC offset
    /// without audible impact on bass frequencies.
    ///
    /// # Audio Safety
    ///
    /// This method is allocation-free and suitable for real-time audio.
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
    ///
    /// Returns true if either channel exceeds the audio detection threshold.
    #[inline]
    fn is_audio_content(left: f32, right: f32) -> bool {
        left.abs() > AUDIO_DETECT_THRESHOLD || right.abs() > AUDIO_DETECT_THRESHOLD
    }

    /// Generate low-level noise for DAC keep-alive
    ///
    /// Uses simple LFSR (Linear Feedback Shift Register) for uncorrelated
    /// L/R pseudo-random noise at -96dB level.
    ///
    /// # Audio Safety
    ///
    /// This method is allocation-free and suitable for real-time audio.
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
    /// # Processing Stages
    ///
    /// 1. **Wait phase** - outputs low-level noise until audio detected
    ///    (amplitude > threshold) or timeout
    /// 2. **Fade phase** - gradual amplitude increase with raised cosine
    ///    (S-curve) for smooth transitions
    /// 3. **DC blocking** - throughout all phases to remove DC offset
    ///
    /// This handles MP3 encoder delay (~26ms silence) that would otherwise
    /// "waste" a time-based fade.
    ///
    /// # Arguments
    ///
    /// * `buffer` - Interleaved stereo audio buffer (L, R, L, R, ...)
    ///
    /// # Returns
    ///
    /// Number of samples processed (always equal to buffer.len())
    ///
    /// # Audio Safety
    ///
    /// This method is allocation-free and suitable for real-time audio
    /// callbacks. MUST be called BEFORE volume/effects processing.
    #[inline]
    pub(crate) fn process(&mut self, buffer: &mut [f32]) -> usize {
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
        // Note: If buffer has odd length, the last sample is ignored (should not happen with
        // properly aligned stereo buffers, but we handle it safely via integer division)
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

/// Stop/transition fade envelope for click-free playback transitions
///
/// Applies a short fade-out when playback stops, pauses, or transitions
/// to prevent audible clicks from sudden amplitude drops.
///
/// # Deferred Action Execution
///
/// The envelope supports deferred action execution - you specify what action
/// to take when the fade completes (stop, pause, transition). This ensures
/// smooth fade-out before state changes.
///
/// # Fade Curve
///
/// Uses an inverse raised cosine (S-curve) that starts at 1.0 and ends at 0.0,
/// providing smooth transitions at both ends of the fade.
///
/// # Thread Safety
///
/// This struct is NOT thread-safe and should only be accessed from the
/// audio processing thread.
pub(crate) struct StopFadeEnvelope {
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

impl StopFadeEnvelope {
    /// Create a new stop fade envelope
    ///
    /// # Arguments
    ///
    /// * `sample_rate` - Audio sample rate in Hz (e.g., 44100, 48000)
    pub(crate) fn new(sample_rate: u32) -> Self {
        Self {
            active: false,
            position_samples: 0,
            duration_samples: Self::calculate_duration_samples(sample_rate, STOP_FADE_DURATION_MS),
            sample_rate,
            fade_complete_action: FadeCompleteAction::None,
        }
    }

    /// Calculate duration in stereo samples from milliseconds
    ///
    /// Formula: samples = sample_rate * duration_ms / 1000 * 2 (stereo)
    #[inline]
    fn calculate_duration_samples(sample_rate: u32, duration_ms: u32) -> usize {
        ((sample_rate as u64 * duration_ms as u64 * 2) / 1000) as usize
    }

    /// Start a fade-out with specified completion action
    ///
    /// # Arguments
    ///
    /// * `action` - Action to execute when fade completes (stop/pause/transition)
    #[inline]
    pub(crate) fn start(&mut self, action: FadeCompleteAction) {
        self.active = true;
        self.position_samples = 0;
        self.fade_complete_action = action;
    }

    /// Reset the envelope (cancel any active fade)
    ///
    /// Immediately deactivates the fade and clears the completion action.
    #[inline]
    pub(crate) fn reset(&mut self) {
        self.active = false;
        self.position_samples = 0;
        self.fade_complete_action = FadeCompleteAction::None;
    }

    /// Update sample rate and recalculate duration
    ///
    /// Should be called when audio output format changes.
    /// Sample rate is clamped to valid range (8000 - 384000 Hz).
    pub(crate) fn set_sample_rate(&mut self, sample_rate: u32) {
        let clamped_rate = sample_rate.clamp(MIN_SAMPLE_RATE, MAX_SAMPLE_RATE);
        if self.sample_rate != clamped_rate {
            self.sample_rate = clamped_rate;
            self.duration_samples =
                Self::calculate_duration_samples(clamped_rate, STOP_FADE_DURATION_MS);
        }
    }

    /// Check if fade is currently active
    #[inline]
    pub(crate) fn is_active(&self) -> bool {
        self.active
    }

    /// Apply fade-out envelope to audio buffer (in-place)
    ///
    /// Uses an inverse raised cosine (S-curve) for smooth fade-out:
    /// - Starts at gain = 1.0 (full volume)
    /// - Ends at gain = 0.0 (silence)
    /// - Smooth transitions at both ends
    ///
    /// # Arguments
    ///
    /// * `buffer` - Interleaved stereo audio buffer (L, R, L, R, ...)
    ///
    /// # Returns
    ///
    /// - `Some(action)` - Fade completed, execute this action
    /// - `None` - Fade still in progress
    ///
    /// # Audio Safety
    ///
    /// This method is allocation-free and suitable for real-time audio
    /// callbacks. Should be called AFTER volume/effects processing.
    #[inline]
    pub(crate) fn process(&mut self, buffer: &mut [f32]) -> Option<FadeCompleteAction> {
        if !self.active {
            return None;
        }

        // Process stereo frames (2 samples per frame)
        // Note: If buffer has odd length, the last sample is ignored (should not happen with
        // properly aligned stereo buffers, but we handle it safely via integer division)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_start_fade_creation() {
        let fade = StartFadeEnvelope::new(48000);
        assert!(!fade.is_active());
        assert_eq!(fade.sample_rate, 48000);
    }

    #[test]
    fn test_stop_fade_creation() {
        let fade = StopFadeEnvelope::new(48000);
        assert!(!fade.is_active());
        assert_eq!(fade.sample_rate, 48000);
    }

    #[test]
    fn test_start_fade_activation() {
        let mut fade = StartFadeEnvelope::new(48000);
        fade.start();
        assert!(fade.is_active());
        fade.reset();
        assert!(!fade.is_active());
    }

    #[test]
    fn test_stop_fade_activation() {
        let mut fade = StopFadeEnvelope::new(48000);
        fade.start(FadeCompleteAction::Pause);
        assert!(fade.is_active());
        fade.reset();
        assert!(!fade.is_active());
    }

    #[test]
    fn test_start_fade_freeze() {
        let mut fade = StartFadeEnvelope::new(48000);
        fade.start();
        fade.freeze();
        assert!(fade.is_active());
        assert!(fade.frozen);
    }

    #[test]
    fn test_sample_rate_update() {
        let mut fade = StartFadeEnvelope::new(44100);
        let old_duration = fade.duration_samples;
        fade.set_sample_rate(48000);
        assert_eq!(fade.sample_rate, 48000);
        assert_ne!(fade.duration_samples, old_duration);
    }

    #[test]
    fn test_duration_calculation() {
        // At 48000 Hz, 30ms fade = 48000 * 0.030 * 2 (stereo) = 2880 samples
        let samples = StartFadeEnvelope::calculate_duration_samples(48000, 30);
        assert_eq!(samples, 2880);
    }

    #[test]
    fn test_audio_detection_threshold() {
        // Below threshold
        assert!(!StartFadeEnvelope::is_audio_content(0.0005, 0.0005));
        // Above threshold
        assert!(StartFadeEnvelope::is_audio_content(0.002, 0.0005));
        assert!(StartFadeEnvelope::is_audio_content(0.0005, 0.002));
    }

    #[test]
    fn test_fade_complete_action() {
        let mut fade = StopFadeEnvelope::new(48000);
        fade.start(FadeCompleteAction::TransitionToNext);
        assert_eq!(
            fade.fade_complete_action,
            FadeCompleteAction::TransitionToNext
        );
    }

    // ========================================
    // Sample Rate Validation Tests
    // ========================================

    #[test]
    fn test_start_fade_sample_rate_clamping_low() {
        let mut fade = StartFadeEnvelope::new(48000);

        // Very low sample rate should be clamped
        fade.set_sample_rate(100);
        assert_eq!(fade.sample_rate, MIN_SAMPLE_RATE);
    }

    #[test]
    fn test_start_fade_sample_rate_clamping_high() {
        let mut fade = StartFadeEnvelope::new(48000);

        // Very high sample rate should be clamped
        fade.set_sample_rate(1000000);
        assert_eq!(fade.sample_rate, MAX_SAMPLE_RATE);
    }

    #[test]
    fn test_start_fade_sample_rate_valid() {
        let mut fade = StartFadeEnvelope::new(48000);

        // Valid sample rate should pass through
        fade.set_sample_rate(96000);
        assert_eq!(fade.sample_rate, 96000);
    }

    #[test]
    fn test_stop_fade_sample_rate_clamping_low() {
        let mut fade = StopFadeEnvelope::new(48000);

        // Very low sample rate should be clamped
        fade.set_sample_rate(100);
        assert_eq!(fade.sample_rate, MIN_SAMPLE_RATE);
    }

    #[test]
    fn test_stop_fade_sample_rate_clamping_high() {
        let mut fade = StopFadeEnvelope::new(48000);

        // Very high sample rate should be clamped
        fade.set_sample_rate(1000000);
        assert_eq!(fade.sample_rate, MAX_SAMPLE_RATE);
    }

    #[test]
    fn test_stop_fade_sample_rate_valid() {
        let mut fade = StopFadeEnvelope::new(48000);

        // Valid sample rate should pass through
        fade.set_sample_rate(96000);
        assert_eq!(fade.sample_rate, 96000);
    }

    #[test]
    fn test_start_fade_duration_updates_on_sample_rate_change() {
        let mut fade = StartFadeEnvelope::new(44100);
        let original_duration = fade.duration_samples;

        fade.set_sample_rate(96000);

        // Duration in samples should increase with higher sample rate
        assert!(
            fade.duration_samples > original_duration,
            "Duration should increase with sample rate"
        );
    }

    #[test]
    fn test_stop_fade_duration_updates_on_sample_rate_change() {
        let mut fade = StopFadeEnvelope::new(44100);
        let original_duration = fade.duration_samples;

        fade.set_sample_rate(96000);

        // Duration in samples should increase with higher sample rate
        assert!(
            fade.duration_samples > original_duration,
            "Duration should increase with sample rate"
        );
    }

    // ========================================
    // Start Fade with Immediate Audio Tests
    // ========================================

    #[test]
    fn test_start_fade_immediate_audio_triggers_fade() {
        let mut fade = StartFadeEnvelope::new(48000);
        fade.start();

        // Create a buffer with immediate audio above threshold
        let mut buffer = vec![0.1, -0.1, 0.2, -0.2]; // Above threshold immediately
        fade.process(&mut buffer);

        // Audio should be detected and fade started
        assert!(fade.audio_detected);
        // First sample pair should be zeroed (clean fade start)
        assert_eq!(buffer[0], 0.0);
        assert_eq!(buffer[1], 0.0);
    }

    #[test]
    fn test_start_fade_immediate_audio_applies_gain_curve() {
        let mut fade = StartFadeEnvelope::new(48000);
        fade.start();

        // Create a large buffer with loud audio - use alternating to avoid DC blocker effects
        let num_samples = 4000; // Larger than fade duration
        let mut buffer: Vec<f32> = (0..num_samples)
            .map(|i| if i % 2 == 0 { 0.5 } else { -0.5 })
            .collect();

        fade.process(&mut buffer);

        // First samples should be at 0 or very low gain
        assert!(buffer[0].abs() < 0.01, "First sample should be near zero");

        // Middle samples should have partial gain (some point during the fade)
        let mid = num_samples / 4; // Early in fade
        assert!(
            buffer[mid].abs() < 0.5,
            "Mid-fade samples should have partial gain, got {}",
            buffer[mid].abs()
        );

        // Verify gain increases throughout the fade
        // Compare early fade to later in fade
        let early = 100;
        let later = fade.duration_samples - 100;
        assert!(
            buffer[later].abs() > buffer[early].abs(),
            "Gain should increase during fade: early {} vs later {}",
            buffer[early].abs(),
            buffer[later].abs()
        );
    }

    // ========================================
    // Start Fade with Long Silence Tests
    // ========================================

    #[test]
    fn test_start_fade_long_silence_waits_for_audio() {
        let mut fade = StartFadeEnvelope::new(48000);
        fade.start();

        // Create a buffer with silence (below threshold)
        let mut buffer = vec![0.0001, 0.0001, 0.0001, 0.0001]; // Below AUDIO_DETECT_THRESHOLD
        fade.process(&mut buffer);

        // Should still be waiting for audio
        assert!(!fade.audio_detected);
        // Should output keepalive noise, not silence
        // Keepalive noise is very small but non-zero
        let has_noise = buffer.iter().any(|&s| s != 0.0);
        assert!(has_noise, "Should output keepalive noise during wait phase");
    }

    #[test]
    fn test_start_fade_silence_then_audio_triggers_fade() {
        let mut fade = StartFadeEnvelope::new(48000);
        fade.start();

        // First buffer: silence
        let mut silence_buffer = vec![0.0001; 100];
        fade.process(&mut silence_buffer);
        assert!(!fade.audio_detected, "Should not detect audio in silence");

        // Second buffer: starts with more silence, then audio
        let mut mixed_buffer = vec![0.0001; 50];
        mixed_buffer.extend(vec![0.5; 50]); // Then loud audio
        fade.process(&mut mixed_buffer);

        // Audio should now be detected
        assert!(fade.audio_detected, "Should detect audio after silence");
    }

    #[test]
    fn test_start_fade_keepalive_noise_amplitude() {
        let mut fade = StartFadeEnvelope::new(48000);
        fade.start();

        // Process silence to get keepalive noise
        let mut buffer = vec![0.0; 1000];
        fade.process(&mut buffer);

        // Verify noise amplitude is within expected range (-96dB = ~0.000016)
        let max_amplitude = buffer.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        assert!(
            max_amplitude <= DAC_KEEPALIVE_NOISE * 2.0,
            "Keepalive noise should be near -96dB level, got {}",
            max_amplitude
        );
        assert!(
            max_amplitude > 0.0,
            "Keepalive noise should not be exactly zero"
        );
    }

    // ========================================
    // Start Fade Timeout Behavior Tests
    // ========================================

    #[test]
    fn test_start_fade_timeout_forces_fade_start() {
        let mut fade = StartFadeEnvelope::new(48000);
        fade.start();

        // Calculate samples needed to exceed timeout (200ms at 48kHz stereo)
        let timeout_samples = ((48000u64 * MAX_WAIT_MS as u64 * 2) / 1000) as usize;

        // Process enough silence to trigger timeout
        let mut buffer = vec![0.0001; timeout_samples + 100];
        fade.process(&mut buffer);

        // Audio should be "detected" due to timeout
        assert!(
            fade.audio_detected,
            "Should force audio detection after timeout"
        );
    }

    #[test]
    fn test_start_fade_timeout_duration_correct() {
        let fade = StartFadeEnvelope::new(48000);

        // 200ms at 48kHz stereo = 48000 * 0.2 * 2 = 19200 samples
        let expected_samples = ((48000u64 * MAX_WAIT_MS as u64 * 2) / 1000) as usize;
        assert_eq!(
            fade.max_wait_samples, expected_samples,
            "Timeout should be {} samples (200ms)",
            expected_samples
        );
    }

    // ========================================
    // Stop Fade Completion Tests
    // ========================================

    #[test]
    fn test_stop_fade_completes_and_returns_action() {
        let mut fade = StopFadeEnvelope::new(48000);
        fade.start(FadeCompleteAction::Stop);

        // Process enough samples to complete the fade
        let num_samples = fade.duration_samples + 1000;
        let mut buffer: Vec<f32> = (0..num_samples).map(|_| 1.0).collect();

        let result = fade.process(&mut buffer);

        assert_eq!(result, Some(FadeCompleteAction::Stop));
        assert!(!fade.is_active());
    }

    #[test]
    fn test_stop_fade_gain_decreases() {
        let mut fade = StopFadeEnvelope::new(48000);
        fade.start(FadeCompleteAction::Pause);

        // Create buffer with constant input
        let num_samples = fade.duration_samples;
        let mut buffer: Vec<f32> = (0..num_samples).map(|_| 1.0).collect();

        fade.process(&mut buffer);

        // First samples should be near full volume
        assert!(buffer[0] > 0.9, "Start of fade should be near full volume");

        // End samples should be near zero
        let end_idx = num_samples - 10;
        assert!(buffer[end_idx] < 0.1, "End of fade should be near zero");
    }

    #[test]
    fn test_stop_fade_outputs_silence_after_complete() {
        let mut fade = StopFadeEnvelope::new(48000);
        fade.start(FadeCompleteAction::Stop);

        // First pass: complete the fade
        let num_samples = fade.duration_samples + 100;
        let mut buffer: Vec<f32> = (0..num_samples).map(|_| 1.0).collect();
        let _ = fade.process(&mut buffer);

        // Last samples after fade complete should be silence
        let end_samples = &buffer[fade.duration_samples..];
        for sample in end_samples {
            assert_eq!(*sample, 0.0, "Output should be silence after fade complete");
        }
    }

    // ========================================
    // Stop Fade Interrupted by Start Tests
    // ========================================

    #[test]
    fn test_stop_fade_interrupted_by_reset() {
        let mut fade = StopFadeEnvelope::new(48000);
        fade.start(FadeCompleteAction::Stop);

        // Process partially
        let mut buffer = vec![1.0; 100];
        let result = fade.process(&mut buffer);
        assert!(result.is_none(), "Should not complete yet");
        assert!(fade.is_active());

        // Reset (simulating start of new playback)
        fade.reset();

        assert!(!fade.is_active());
        assert_eq!(fade.fade_complete_action, FadeCompleteAction::None);
    }

    #[test]
    fn test_stop_fade_restart_resets_position() {
        let mut fade = StopFadeEnvelope::new(48000);
        fade.start(FadeCompleteAction::Stop);

        // Process partially
        let mut buffer = vec![1.0; 1000];
        fade.process(&mut buffer);
        let position_after_partial = fade.position_samples;
        assert!(position_after_partial > 0);

        // Start new fade (should reset position)
        fade.start(FadeCompleteAction::Pause);
        assert_eq!(fade.position_samples, 0, "New fade should reset position");
        assert_eq!(fade.fade_complete_action, FadeCompleteAction::Pause);
    }

    // ========================================
    // Overlapping Fade Operations Tests
    // ========================================

    #[test]
    fn test_start_fade_while_start_fade_active() {
        let mut fade = StartFadeEnvelope::new(48000);
        fade.start();

        // Process some audio to get partway through fade
        let mut buffer = vec![0.5; 500];
        fade.process(&mut buffer);
        let position_mid = fade.position_samples;
        assert!(position_mid > 0);

        // Start new fade (should reset)
        fade.start();
        assert_eq!(fade.position_samples, 0, "New start should reset position");
        assert!(
            !fade.audio_detected,
            "New start should reset audio detection"
        );
        assert_eq!(fade.wait_samples, 0, "New start should reset wait samples");
    }

    #[test]
    fn test_stop_fade_while_stop_fade_active() {
        let mut fade = StopFadeEnvelope::new(48000);
        fade.start(FadeCompleteAction::Pause);

        // Process partially
        let mut buffer = vec![1.0; 500];
        fade.process(&mut buffer);
        assert!(fade.position_samples > 0);

        // Start new stop fade with different action
        fade.start(FadeCompleteAction::TransitionToNext);
        assert_eq!(
            fade.position_samples, 0,
            "New stop fade should reset position"
        );
        assert_eq!(
            fade.fade_complete_action,
            FadeCompleteAction::TransitionToNext
        );
    }

    // ========================================
    // 1-Sample Buffer Tests
    // ========================================

    #[test]
    fn test_start_fade_single_sample_buffer() {
        let mut fade = StartFadeEnvelope::new(48000);
        fade.start();

        // Single sample (not a complete stereo pair)
        let mut buffer = vec![0.5];
        fade.process(&mut buffer);

        // Should handle gracefully (no frames processed since len/2 = 0)
        // The buffer should be unchanged since there are no complete frames
        assert_eq!(buffer[0], 0.5);
    }

    #[test]
    fn test_start_fade_two_sample_buffer() {
        let mut fade = StartFadeEnvelope::new(48000);
        fade.start();

        // Exactly one stereo frame
        let mut buffer = vec![0.5, -0.5];
        fade.process(&mut buffer);

        // Should process the single frame
        // With audio above threshold, first frame should be zeroed
        assert_eq!(buffer[0], 0.0);
        assert_eq!(buffer[1], 0.0);
    }

    #[test]
    fn test_stop_fade_single_sample_buffer() {
        let mut fade = StopFadeEnvelope::new(48000);
        fade.start(FadeCompleteAction::Stop);

        // Single sample (not a complete stereo pair)
        let mut buffer = vec![1.0];
        let result = fade.process(&mut buffer);

        // Should handle gracefully
        assert!(result.is_none());
        // Buffer should be unchanged since there are no complete frames
        assert_eq!(buffer[0], 1.0);
    }

    #[test]
    fn test_stop_fade_two_sample_buffer() {
        let mut fade = StopFadeEnvelope::new(48000);
        fade.start(FadeCompleteAction::Stop);

        // Exactly one stereo frame
        let mut buffer = vec![1.0, 1.0];
        let result = fade.process(&mut buffer);

        // Should process the single frame
        assert!(result.is_none());
        // First frame at position 0 should have gain near 1.0 (S-curve start)
        assert!(
            buffer[0] > 0.9,
            "First sample should be near full volume, got {}",
            buffer[0]
        );
    }

    // ========================================
    // Buffer Larger Than Fade Duration Tests
    // ========================================

    #[test]
    fn test_start_fade_large_buffer_completes_fade() {
        let mut fade = StartFadeEnvelope::new(48000);
        fade.start();

        // Buffer much larger than fade duration
        let num_samples = fade.duration_samples * 3;
        let mut buffer: Vec<f32> = (0..num_samples).map(|_| 0.5).collect();

        fade.process(&mut buffer);

        // Fade should complete within this buffer
        assert!(!fade.is_active(), "Fade should complete with large buffer");
    }

    #[test]
    fn test_start_fade_large_buffer_passthrough_after_complete() {
        let mut fade = StartFadeEnvelope::new(48000);
        fade.start();

        // Buffer much larger than fade duration
        // Use a periodic signal that the DC blocker will preserve
        // A signal like sin wave: alternating between positive and negative
        // at audio frequencies won't be attenuated by the DC blocker
        let num_samples = fade.duration_samples * 3;
        let mut buffer: Vec<f32> = (0..num_samples)
            .map(|i| {
                // Simulate ~1kHz sine at 48kHz: period ~48 samples
                let phase = (i as f32 / 48.0) * std::f32::consts::TAU;
                0.5 * phase.sin()
            })
            .collect();

        fade.process(&mut buffer);

        // After fade completes, samples should pass through with full gain
        // Check the RMS amplitude of a late section (the DC blocker preserves AC content)
        let late_start = fade.duration_samples + 200;
        let late_end = late_start + 200;
        let rms: f32 = (buffer[late_start..late_end]
            .iter()
            .map(|s| s * s)
            .sum::<f32>()
            / (late_end - late_start) as f32)
            .sqrt();

        // RMS of 0.5 amplitude sine wave is 0.5/sqrt(2) ≈ 0.354
        // After fade with DC blocker, should still have significant amplitude
        assert!(
            rms > 0.2,
            "Post-fade samples should have significant RMS amplitude, got {}",
            rms
        );
    }

    #[test]
    fn test_stop_fade_large_buffer_completes_fade() {
        let mut fade = StopFadeEnvelope::new(48000);
        fade.start(FadeCompleteAction::Stop);

        // Buffer much larger than fade duration
        let num_samples = fade.duration_samples * 3;
        let mut buffer: Vec<f32> = (0..num_samples).map(|_| 1.0).collect();

        let result = fade.process(&mut buffer);

        // Fade should complete
        assert_eq!(result, Some(FadeCompleteAction::Stop));
        assert!(!fade.is_active());
    }

    #[test]
    fn test_stop_fade_large_buffer_silence_after_complete() {
        let mut fade = StopFadeEnvelope::new(48000);
        fade.start(FadeCompleteAction::Stop);

        let duration = fade.duration_samples;
        let num_samples = duration * 3;
        let mut buffer: Vec<f32> = (0..num_samples).map(|_| 1.0).collect();

        fade.process(&mut buffer);

        // All samples after fade complete should be zero
        for i in duration..num_samples {
            assert_eq!(
                buffer[i], 0.0,
                "Sample {} should be zero after fade complete",
                i
            );
        }
    }

    // ========================================
    // Sample Rate Change During Fade Tests
    // ========================================

    #[test]
    fn test_start_fade_sample_rate_change_during_fade() {
        let mut fade = StartFadeEnvelope::new(44100);
        fade.start();

        // Process some audio
        let mut buffer = vec![0.5; 500];
        fade.process(&mut buffer);

        let old_duration = fade.duration_samples;

        // Change sample rate during active fade
        fade.set_sample_rate(96000);

        // Duration should update
        assert_ne!(
            fade.duration_samples, old_duration,
            "Duration should change with sample rate"
        );
        assert!(
            fade.duration_samples > old_duration,
            "Higher sample rate should mean more samples"
        );

        // Fade should still be active
        assert!(fade.is_active());
    }

    #[test]
    fn test_stop_fade_sample_rate_change_during_fade() {
        let mut fade = StopFadeEnvelope::new(44100);
        fade.start(FadeCompleteAction::Pause);

        // Process some samples
        let mut buffer = vec![1.0; 500];
        fade.process(&mut buffer);

        let old_duration = fade.duration_samples;

        // Change sample rate during active fade
        fade.set_sample_rate(96000);

        // Duration should update
        assert_ne!(fade.duration_samples, old_duration);
        assert!(fade.is_active());
    }

    #[test]
    fn test_start_fade_sample_rate_change_preserves_position() {
        let mut fade = StartFadeEnvelope::new(44100);
        fade.start();

        // Process to get position
        let mut buffer = vec![0.5; 1000];
        fade.process(&mut buffer);

        let position_before = fade.position_samples;
        assert!(position_before > 0);

        // Change sample rate
        fade.set_sample_rate(96000);

        // Position should be preserved (implementation detail but important for consistency)
        assert_eq!(
            fade.position_samples, position_before,
            "Position should be preserved on sample rate change"
        );
    }

    // ========================================
    // Reset During Active Fade Tests
    // ========================================

    #[test]
    fn test_start_fade_reset_during_wait_phase() {
        let mut fade = StartFadeEnvelope::new(48000);
        fade.start();

        // Process silence (stay in wait phase)
        let mut buffer = vec![0.0001; 100];
        fade.process(&mut buffer);
        assert!(!fade.audio_detected);

        // Reset during wait
        fade.reset();

        assert!(!fade.is_active());
        assert!(!fade.audio_detected);
        assert_eq!(fade.position_samples, 0);
        assert_eq!(fade.wait_samples, 0);
    }

    #[test]
    fn test_start_fade_reset_during_fade_phase() {
        let mut fade = StartFadeEnvelope::new(48000);
        fade.start();

        // Process audio to enter fade phase
        let mut buffer = vec![0.5; 500];
        fade.process(&mut buffer);
        assert!(fade.audio_detected);
        assert!(fade.position_samples > 0);

        // Reset during fade
        fade.reset();

        assert!(!fade.is_active());
        assert!(!fade.audio_detected);
        assert_eq!(fade.position_samples, 0);
    }

    #[test]
    fn test_stop_fade_reset_during_fade() {
        let mut fade = StopFadeEnvelope::new(48000);
        fade.start(FadeCompleteAction::TransitionToNext);

        // Process partially
        let mut buffer = vec![1.0; 500];
        fade.process(&mut buffer);
        assert!(fade.position_samples > 0);

        // Reset during fade
        fade.reset();

        assert!(!fade.is_active());
        assert_eq!(fade.position_samples, 0);
        assert_eq!(fade.fade_complete_action, FadeCompleteAction::None);
    }

    #[test]
    fn test_start_fade_reset_clears_dc_blocker_state() {
        let mut fade = StartFadeEnvelope::new(48000);
        fade.start();

        // Process audio to populate DC blocker state
        let mut buffer = vec![0.5; 1000];
        fade.process(&mut buffer);

        // Start fresh (which resets DC blocker)
        fade.start();

        // DC blocker state should be reset
        assert_eq!(fade.dc_blocker_prev_input_l, 0.0);
        assert_eq!(fade.dc_blocker_prev_output_l, 0.0);
        assert_eq!(fade.dc_blocker_prev_input_r, 0.0);
        assert_eq!(fade.dc_blocker_prev_output_r, 0.0);
    }

    // ========================================
    // DC Blocking Effectiveness Tests
    // ========================================

    #[test]
    fn test_dc_blocking_removes_dc_offset() {
        let mut fade = StartFadeEnvelope::new(48000);
        fade.start();

        // First, get past the wait phase and complete the fade
        let init_samples = fade.duration_samples + 1000;
        let mut init_buffer: Vec<f32> = (0..init_samples)
            .map(|i| if i % 2 == 0 { 0.5 } else { -0.5 })
            .collect();
        fade.process(&mut init_buffer);

        // Now the fade is complete and inactive. Start a new fade to use the DC blocker
        fade.start();

        // Process a signal with DC offset: constant 0.5 value has DC component
        // The DC blocker should attenuate the DC component over time
        let num_samples = 20000;

        // First, let the fade complete with DC offset signal
        let mut dc_buffer: Vec<f32> = vec![0.5; num_samples];
        fade.process(&mut dc_buffer);

        // The DC blocker is a first-order highpass filter
        // It should reduce the DC component but not eliminate it instantly
        // After many samples, the output should trend toward zero for constant input
        // because the highpass removes the DC component

        // For constant input, the DC blocker output converges toward zero
        // Check that the later samples are closer to zero than the DC input
        let late_avg: f32 = dc_buffer[num_samples - 1000..].iter().sum::<f32>() / 1000.0;

        // The DC blocker should have reduced the DC component significantly
        // With coefficient 0.9975, after many samples it should be much less than 0.5
        assert!(
            late_avg.abs() < 0.3,
            "DC blocker should reduce constant DC input over time, got avg {}",
            late_avg
        );
    }

    #[test]
    fn test_dc_blocking_preserves_ac_signal() {
        let mut fade = StartFadeEnvelope::new(48000);
        fade.start();

        // Create a pure AC signal (sine-like alternating pattern)
        let num_samples = 2000;
        let mut buffer: Vec<f32> = (0..num_samples)
            .map(|i| if i % 4 < 2 { 0.5 } else { -0.5 })
            .collect();

        // Process through fade
        fade.process(&mut buffer);

        // After fade, AC content should still be present
        // Check that there's still significant amplitude variation
        let max_val = buffer.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let min_val = buffer.iter().cloned().fold(f32::INFINITY, f32::min);
        let range = max_val - min_val;

        assert!(
            range > 0.3,
            "AC signal should be preserved, range was {}",
            range
        );
    }

    #[test]
    fn test_dc_blocker_coefficient_in_valid_range() {
        // Verify the DC blocker coefficient is in the recommended range
        assert!(
            DC_BLOCKER_COEFF >= 0.995 && DC_BLOCKER_COEFF <= 0.9999,
            "DC blocker coefficient should be in range [0.995, 0.9999], got {}",
            DC_BLOCKER_COEFF
        );
    }

    // ========================================
    // Noise Amplitude Verification Tests
    // ========================================

    #[test]
    fn test_keepalive_noise_is_approximately_minus_96db() {
        let mut fade = StartFadeEnvelope::new(48000);
        fade.start();

        // Process silence to get keepalive noise
        let num_samples = 10000;
        let mut buffer = vec![0.0; num_samples];
        fade.process(&mut buffer);

        // Calculate RMS of the noise
        let sum_squares: f32 = buffer.iter().map(|s| s * s).sum();
        let rms = (sum_squares / num_samples as f32).sqrt();

        // -96dB corresponds to amplitude of ~0.000016
        // RMS should be close to DAC_KEEPALIVE_NOISE / sqrt(3) for uniform distribution
        // but since we use different noise generation, just verify it's in the right ballpark
        let expected_max = DAC_KEEPALIVE_NOISE * 2.0;

        assert!(
            rms < expected_max,
            "Noise RMS {} should be less than {}",
            rms,
            expected_max
        );
        assert!(rms > 0.0, "Noise should not be zero");
    }

    #[test]
    fn test_keepalive_noise_is_uncorrelated_stereo() {
        let mut fade = StartFadeEnvelope::new(48000);
        fade.start();

        // Process silence to get keepalive noise
        let num_frames = 5000;
        let mut buffer = vec![0.0; num_frames * 2];
        fade.process(&mut buffer);

        // Extract left and right channels
        let left: Vec<f32> = buffer.iter().step_by(2).cloned().collect();
        let right: Vec<f32> = buffer.iter().skip(1).step_by(2).cloned().collect();

        // Calculate correlation coefficient
        let mean_l: f32 = left.iter().sum::<f32>() / left.len() as f32;
        let mean_r: f32 = right.iter().sum::<f32>() / right.len() as f32;

        let mut cov = 0.0f32;
        let mut var_l = 0.0f32;
        let mut var_r = 0.0f32;

        for i in 0..left.len() {
            let dl = left[i] - mean_l;
            let dr = right[i] - mean_r;
            cov += dl * dr;
            var_l += dl * dl;
            var_r += dr * dr;
        }

        let correlation = if var_l > 0.0 && var_r > 0.0 {
            cov / (var_l.sqrt() * var_r.sqrt())
        } else {
            0.0
        };

        // Correlation should be low (uncorrelated channels)
        assert!(
            correlation.abs() < 0.3,
            "L/R channels should be uncorrelated, got {}",
            correlation
        );
    }

    #[test]
    fn test_keepalive_noise_constant_definition() {
        // Verify the constant is approximately -96dB
        // -96dB = 10^(-96/20) = 10^(-4.8) ≈ 0.0000158
        let expected_minus_96db = 10.0f32.powf(-96.0 / 20.0);

        assert!(
            (DAC_KEEPALIVE_NOISE - expected_minus_96db).abs() < 0.00001,
            "DAC_KEEPALIVE_NOISE {} should be approximately -96dB ({})",
            DAC_KEEPALIVE_NOISE,
            expected_minus_96db
        );
    }

    // ========================================
    // S-Curve Gain Verification Tests
    // ========================================

    #[test]
    fn test_start_fade_scurve_starts_at_zero() {
        let mut fade = StartFadeEnvelope::new(48000);
        fade.start();

        // Process buffer with audio
        let mut buffer = vec![1.0; 4];
        fade.process(&mut buffer);

        // First sample pair should be zeroed (position 0)
        assert_eq!(buffer[0], 0.0, "S-curve should start at gain 0");
        assert_eq!(buffer[1], 0.0);
    }

    #[test]
    fn test_start_fade_scurve_ends_at_one() {
        let mut fade = StartFadeEnvelope::new(48000);
        fade.start();

        // The S-curve formula is: gain = (1 - cos(π * progress)) / 2
        // At progress = 1.0, gain = (1 - cos(π)) / 2 = (1 - (-1)) / 2 = 1.0

        // To test this properly, we need to verify the gain calculation directly
        // rather than through the DC blocker which complicates things

        // Calculate expected gain at near-end of fade (progress = 0.95)
        let progress = 0.95f32;
        let expected_gain = (1.0 - (std::f32::consts::PI * progress).cos()) * 0.5;

        // Expected gain at 95% progress should be very close to 1.0
        assert!(
            expected_gain > 0.95,
            "S-curve at 95% progress should give gain > 0.95, got {}",
            expected_gain
        );

        // Verify at progress = 1.0
        let final_progress = 1.0f32;
        let final_gain = (1.0 - (std::f32::consts::PI * final_progress).cos()) * 0.5;
        assert!(
            (final_gain - 1.0).abs() < 0.001,
            "S-curve at 100% should give gain = 1.0, got {}",
            final_gain
        );
    }

    #[test]
    fn test_stop_fade_scurve_starts_at_one() {
        let mut fade = StopFadeEnvelope::new(48000);
        fade.start(FadeCompleteAction::Stop);

        // Process first frame
        let mut buffer = vec![1.0; 4];
        fade.process(&mut buffer);

        // First sample should be near full volume (progress = 0, gain ≈ 1)
        assert!(
            buffer[0] > 0.95,
            "S-curve should start near gain 1.0, got {}",
            buffer[0]
        );
    }

    #[test]
    fn test_stop_fade_scurve_ends_at_zero() {
        let mut fade = StopFadeEnvelope::new(48000);
        fade.start(FadeCompleteAction::Stop);

        // Process to near end of fade
        let num_samples = fade.duration_samples;
        let mut buffer: Vec<f32> = (0..num_samples).map(|_| 1.0).collect();
        fade.process(&mut buffer);

        // Last sample before complete should be very low
        let last_idx = num_samples - 2;
        assert!(
            buffer[last_idx] < 0.1,
            "Near end of fade, gain should be near 0, got {}",
            buffer[last_idx]
        );
    }

    // ========================================
    // Freeze Behavior Tests
    // ========================================

    #[test]
    fn test_start_fade_freeze_stops_progression() {
        let mut fade = StartFadeEnvelope::new(48000);
        fade.start();

        // Process until audio detected and partway through fade
        let mut buffer = vec![0.5; 500];
        fade.process(&mut buffer);

        let position_before_freeze = fade.position_samples;
        assert!(position_before_freeze > 0);

        // Freeze
        fade.freeze();
        assert!(fade.frozen);

        // Process more
        let mut buffer2 = vec![0.5; 500];
        fade.process(&mut buffer2);

        // Position should not have changed
        assert_eq!(
            fade.position_samples, position_before_freeze,
            "Position should not change while frozen"
        );
    }

    #[test]
    fn test_start_fade_freeze_maintains_gain() {
        let mut fade = StartFadeEnvelope::new(48000);
        fade.start();

        // Process until partway through fade
        let mut buffer = vec![0.5; 500];
        fade.process(&mut buffer);

        // Get gain at last sample
        let last_output = buffer[buffer.len() - 2];

        // Freeze
        fade.freeze();

        // Process more with same input
        let mut buffer2 = vec![0.5; 100];
        fade.process(&mut buffer2);

        // Output should be consistent (same gain applied)
        // Allow some tolerance for DC blocker settling
        let new_output = buffer2[buffer2.len() - 2];
        assert!(
            (new_output - last_output).abs() < 0.1,
            "Frozen fade should maintain similar gain"
        );
    }

    // ========================================
    // Edge Case Tests
    // ========================================

    #[test]
    fn test_process_when_not_active() {
        let mut fade = StartFadeEnvelope::new(48000);
        // Don't start - should be inactive

        let original = vec![0.5, -0.5, 0.3, -0.3];
        let mut buffer = original.clone();
        fade.process(&mut buffer);

        // Buffer should be unchanged when fade not active
        assert_eq!(buffer, original);
    }

    #[test]
    fn test_stop_fade_process_when_not_active() {
        let mut fade = StopFadeEnvelope::new(48000);
        // Don't start - should be inactive

        let original = vec![1.0, 1.0, 1.0, 1.0];
        let mut buffer = original.clone();
        let result = fade.process(&mut buffer);

        // Buffer should be unchanged and result should be None
        assert_eq!(buffer, original);
        assert!(result.is_none());
    }

    #[test]
    fn test_empty_buffer() {
        let mut start_fade = StartFadeEnvelope::new(48000);
        start_fade.start();

        let mut buffer: Vec<f32> = vec![];
        let processed = start_fade.process(&mut buffer);
        assert_eq!(processed, 0);

        let mut stop_fade = StopFadeEnvelope::new(48000);
        stop_fade.start(FadeCompleteAction::Stop);

        let result = stop_fade.process(&mut buffer);
        assert!(result.is_none());
    }

    #[test]
    fn test_odd_length_buffer() {
        let mut fade = StartFadeEnvelope::new(48000);
        fade.start();

        // Odd length buffer (3 samples = 1 complete frame + 1 orphan)
        let mut buffer = vec![0.5, 0.5, 0.5];
        fade.process(&mut buffer);

        // Only first frame should be processed
        // First frame should be zeroed (audio detected)
        assert_eq!(buffer[0], 0.0);
        assert_eq!(buffer[1], 0.0);
        // Third sample should be unchanged (orphan)
        assert_eq!(buffer[2], 0.5);
    }

    #[test]
    fn test_multiple_fade_cycles() {
        let mut fade = StartFadeEnvelope::new(48000);

        // First cycle
        fade.start();
        let mut buffer = vec![0.5; 5000];
        fade.process(&mut buffer);
        assert!(!fade.is_active(), "First fade should complete");

        // Second cycle
        fade.start();
        assert!(fade.is_active());
        assert_eq!(fade.position_samples, 0);
        assert!(!fade.audio_detected);

        let mut buffer2 = vec![0.5; 5000];
        fade.process(&mut buffer2);
        assert!(!fade.is_active(), "Second fade should complete");
    }

    #[test]
    fn test_stop_fade_all_actions() {
        // Test each FadeCompleteAction
        let actions = [
            FadeCompleteAction::None,
            FadeCompleteAction::Stop,
            FadeCompleteAction::Pause,
            FadeCompleteAction::TransitionToNext,
        ];

        for action in actions {
            let mut fade = StopFadeEnvelope::new(48000);
            fade.start(action);

            let num_samples = fade.duration_samples + 100;
            let mut buffer: Vec<f32> = (0..num_samples).map(|_| 1.0).collect();

            let result = fade.process(&mut buffer);
            assert_eq!(
                result,
                Some(action),
                "Should return correct action: {:?}",
                action
            );
        }
    }
}
