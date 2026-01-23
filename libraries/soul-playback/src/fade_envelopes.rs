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
    pub(crate) fn set_sample_rate(&mut self, sample_rate: u32) {
        if self.sample_rate != sample_rate {
            self.sample_rate = sample_rate;
            self.duration_samples =
                Self::calculate_duration_samples(sample_rate, START_FADE_DURATION_MS);
            self.max_wait_samples = Self::calculate_duration_samples(sample_rate, MAX_WAIT_MS);
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
    pub(crate) fn set_sample_rate(&mut self, sample_rate: u32) {
        if self.sample_rate != sample_rate {
            self.sample_rate = sample_rate;
            self.duration_samples =
                Self::calculate_duration_samples(sample_rate, STOP_FADE_DURATION_MS);
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
}
