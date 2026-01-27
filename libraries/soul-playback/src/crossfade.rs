//! Crossfade engine for smooth track transitions
//!
//! Provides multiple fade curve types for seamless transitions between tracks:
//! - Linear: Simple linear fade (note: has 3dB volume dip at midpoint)
//! - SquareRoot: Faster rise than linear, natural-sounding transitions
//! - S-Curve: Smooth transitions with slow start/end
//! - Equal Power: Constant perceived loudness (best for music, default)
//! - Exponential: dB-linear fade matching human hearing perception
//!
//! # Industry Standards
//!
//! This implementation follows audio industry best practices:
//! - **Equal Power** (default): Standard for professional DAWs and music crossfades.
//!   Maintains constant power (sin²(x) + cos²(x) = 1) ensuring -3dB at midpoint.
//! - **Crossfade duration**: Default 3 seconds, range 0-10 seconds (Spotify uses 1-12s).
//! - **Gapless support**: 0ms crossfade for seamless transitions without mixing.
//!
//! # References
//! - Sound on Sound: "Should I use linear or constant-power crossfades?"
//! - Audacity Manual: "Fade and Crossfade" - equal power for music
//! - KVR Audio DSP Forum: Equal power crossfading discussions

use std::f32::consts::PI;

/// Crossfade curve type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FadeCurve {
    /// Linear fade: simple and predictable
    ///
    /// **Note**: Linear crossfade has a 3dB volume dip at the midpoint because
    /// it maintains constant amplitude sum (0.5 + 0.5 = 1.0) but not constant power.
    /// At the midpoint: power = 0.5^2 + 0.5^2 = 0.5 (-3dB).
    /// For music, prefer `EqualPower` which maintains constant perceived loudness.
    Linear,

    /// Square root fade: faster rise than linear, creates natural-sounding transitions
    ///
    /// Uses t^0.5 (square root) curve. This creates a curve that rises faster
    /// initially then slows down, which sounds more natural than linear.
    SquareRoot,

    /// Logarithmic fade (alias for SquareRoot for backwards compatibility)
    ///
    /// **Deprecated**: Use `SquareRoot` instead. This uses t^0.5 (square root),
    /// not a true logarithmic curve. The name was misleading.
    #[deprecated(
        since = "0.2.0",
        note = "Use SquareRoot instead - this is actually a square root curve, not logarithmic"
    )]
    Logarithmic,

    /// S-Curve fade: slow start, fast middle, slow end
    SCurve,

    /// Equal power fade: maintains perceived loudness
    /// This is the default and best choice for music crossfades
    ///
    /// Uses sine/cosine relationship: at any point, gain_in² + gain_out² = 1
    /// This ensures constant total power (perceived loudness) during the transition.
    /// At midpoint: both gains are √(0.5) ≈ 0.707 (-3dB each, 0dB combined).
    #[default]
    EqualPower,

    /// Exponential (dB-linear) fade: matches human hearing perception
    ///
    /// This curve is linear in the dB domain, meaning it sounds like a steady,
    /// constant-rate volume change to human ears. Uses the formula:
    /// gain = 10^(t * range_db / 20) mapped to [0, 1]
    ///
    /// Best for:
    /// - Fade-outs that mimic natural acoustic decay
    /// - Transitions where perceptual linearity matters
    /// - Speech and dialogue content
    ///
    /// Note: This is NOT equal-power - use `EqualPower` for music crossfades.
    Exponential,
}

impl FadeCurve {
    /// Calculate the fade gain at a given position
    ///
    /// # Arguments
    /// * `position` - Normalized position in the fade (0.0 to 1.0)
    /// * `fade_out` - If true, calculates fade-out gain; if false, fade-in gain
    ///
    /// # Returns
    /// Gain multiplier (0.0 to 1.0)
    #[inline]
    pub fn calculate_gain(&self, position: f32, fade_out: bool) -> f32 {
        let position = position.clamp(0.0, 1.0);
        let t = if fade_out { 1.0 - position } else { position };

        match self {
            FadeCurve::Linear => t,

            #[allow(deprecated)]
            FadeCurve::SquareRoot | FadeCurve::Logarithmic => {
                // Square root curve: faster rise than linear, sounds natural
                // t^0.5 creates a curve that rises quickly at first then slows
                if t <= 0.0 {
                    0.0
                } else {
                    t.powf(0.5)
                }
            }

            FadeCurve::SCurve => {
                // Smooth S-curve using sine: slow start, fast middle, slow end
                // Maps t ∈ [0,1] through sin to create S-shape
                (1.0 - (PI * t).cos()) * 0.5
            }

            FadeCurve::EqualPower => {
                // Equal power crossfade maintains constant perceived loudness
                // Uses sine/cosine relationship: sin²(x) + cos²(x) = 1
                // This ensures the sum of powers remains constant during fade
                (t * PI * 0.5).sin()
            }

            FadeCurve::Exponential => {
                // dB-linear (exponential) fade: sounds perceptually linear
                // Human hearing is logarithmic, so this curve sounds "steady"
                //
                // Uses the formula: gain = (a^t - 1) / (a - 1)
                // where 'a' controls the curve steepness
                //
                // With a > 1, the curve rises slowly at first, then rapidly at the end
                // This matches how humans perceive loudness (logarithmically)
                //
                // At t=0: gain = 0 (silence)
                // At t=1: gain = 1 (full volume)
                if t <= 0.0 {
                    0.0
                } else if t >= 1.0 {
                    1.0
                } else {
                    // Use a=1000 for a curve spanning roughly 60dB range
                    // This gives good perceptual linearity without extreme values
                    const A: f32 = 1000.0;
                    (A.powf(t) - 1.0) / (A - 1.0)
                }
            }
        }
    }

    /// Get a human-readable name for the curve
    #[allow(deprecated)]
    pub fn display_name(&self) -> &'static str {
        match self {
            FadeCurve::Linear => "Linear",
            FadeCurve::SquareRoot | FadeCurve::Logarithmic => "Square Root", // Logarithmic is deprecated alias
            FadeCurve::SCurve => "S-Curve",
            FadeCurve::EqualPower => "Equal Power",
            FadeCurve::Exponential => "Exponential (dB-linear)",
        }
    }
}

/// Maximum allowed crossfade duration in milliseconds
pub const MAX_CROSSFADE_DURATION_MS: u32 = 10000;

/// Default crossfade duration in milliseconds
pub const DEFAULT_CROSSFADE_DURATION_MS: u32 = 3000;

/// Minimum allowed sample rate in Hz
pub const MIN_SAMPLE_RATE: u32 = 8000;

/// Maximum allowed sample rate in Hz
pub const MAX_SAMPLE_RATE: u32 = 384000;

/// Crossfade settings
#[derive(Debug, Clone)]
pub struct CrossfadeSettings {
    /// Whether crossfade is enabled
    pub enabled: bool,

    /// Crossfade duration in milliseconds (0 = gapless, max 10000)
    pub duration_ms: u32,

    /// Fade curve type
    pub curve: FadeCurve,

    /// Trigger crossfade on manual skip (vs only auto-advance)
    pub on_skip: bool,
}

impl Default for CrossfadeSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            duration_ms: DEFAULT_CROSSFADE_DURATION_MS,
            curve: FadeCurve::EqualPower,
            on_skip: false,
        }
    }
}

impl CrossfadeSettings {
    /// Create settings with gapless playback (no crossfade, 0ms transition)
    pub fn gapless() -> Self {
        Self {
            enabled: true,
            duration_ms: 0,
            curve: FadeCurve::Linear, // Doesn't matter for 0ms
            on_skip: false,
        }
    }

    /// Create settings with a specific duration
    ///
    /// Duration is clamped to maximum of 10000ms.
    pub fn with_duration(duration_ms: u32) -> Self {
        Self {
            enabled: true,
            duration_ms: duration_ms.min(MAX_CROSSFADE_DURATION_MS),
            curve: FadeCurve::EqualPower,
            on_skip: false,
        }
    }

    /// Create settings with a specific duration and curve
    ///
    /// Duration is clamped to maximum of 10000ms.
    pub fn with_duration_and_curve(duration_ms: u32, curve: FadeCurve) -> Self {
        Self {
            enabled: true,
            duration_ms: duration_ms.min(MAX_CROSSFADE_DURATION_MS),
            curve,
            on_skip: false,
        }
    }

    /// Validate the crossfade settings
    ///
    /// Returns `Ok(())` if valid, otherwise returns a list of validation errors.
    pub fn validate(&self) -> Result<(), Vec<crate::types::ConfigValidationError>> {
        let mut errors = Vec::new();

        if self.duration_ms > MAX_CROSSFADE_DURATION_MS {
            errors.push(
                crate::types::ConfigValidationError::CrossfadeDurationOutOfRange {
                    value: self.duration_ms,
                    max: MAX_CROSSFADE_DURATION_MS,
                },
            );
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Create a validated copy, clamping values to valid ranges
    #[must_use]
    pub fn validated(mut self) -> Self {
        self.duration_ms = self.duration_ms.min(MAX_CROSSFADE_DURATION_MS);
        self
    }

    /// Get duration in samples for a given sample rate
    ///
    /// # Panics
    ///
    /// This function will not panic. If sample_rate is 0, returns 0 samples.
    pub fn duration_samples(&self, sample_rate: u32) -> usize {
        if sample_rate == 0 {
            return 0;
        }
        ((self.duration_ms as u64 * sample_rate as u64) / 1000) as usize
    }
}

/// Crossfade state during transition
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrossfadeState {
    /// No crossfade active, normal playback
    Inactive,

    /// Crossfade in progress
    Active,

    /// Crossfade completed, waiting for cleanup
    Completed,
}

/// Crossfade engine
///
/// Handles mixing of outgoing and incoming tracks during transitions.
pub struct CrossfadeEngine {
    /// Current settings
    settings: CrossfadeSettings,

    /// Current crossfade state
    state: CrossfadeState,

    /// Current position in crossfade (in samples)
    position_samples: usize,

    /// Total duration of current crossfade (in samples)
    duration_samples: usize,

    /// Sample rate for calculations
    sample_rate: u32,
}

impl CrossfadeEngine {
    /// Create a new crossfade engine with default settings
    pub fn new() -> Self {
        Self::with_settings(CrossfadeSettings::default())
    }

    /// Create a crossfade engine with specific settings
    pub fn with_settings(settings: CrossfadeSettings) -> Self {
        Self {
            settings,
            state: CrossfadeState::Inactive,
            position_samples: 0,
            duration_samples: 0,
            sample_rate: 44100,
        }
    }

    /// Update settings
    pub fn set_settings(&mut self, settings: CrossfadeSettings) {
        self.settings = settings;
    }

    /// Get current settings
    pub fn settings(&self) -> &CrossfadeSettings {
        &self.settings
    }

    /// Set sample rate
    ///
    /// Sample rate is clamped to valid range (8000 - 384000 Hz).
    /// Values outside this range are clamped to the nearest valid value.
    pub fn set_sample_rate(&mut self, sample_rate: u32) {
        self.sample_rate = sample_rate.clamp(MIN_SAMPLE_RATE, MAX_SAMPLE_RATE);
    }

    /// Get current crossfade state
    pub fn state(&self) -> CrossfadeState {
        self.state
    }

    /// Check if crossfade is currently active
    pub fn is_active(&self) -> bool {
        self.state == CrossfadeState::Active
    }

    /// Start a crossfade transition
    ///
    /// # Arguments
    /// * `is_manual_skip` - Whether this was triggered by manual skip (vs auto-advance)
    ///
    /// # Returns
    /// True if crossfade was started, false if skipped (e.g., disabled or on_skip=false)
    pub fn start(&mut self, is_manual_skip: bool) -> bool {
        if !self.settings.enabled {
            return false;
        }

        if is_manual_skip && !self.settings.on_skip {
            return false;
        }

        self.duration_samples = self.settings.duration_samples(self.sample_rate) * 2; // * 2 for stereo
        self.position_samples = 0;
        self.state = CrossfadeState::Active;

        true
    }

    /// Cancel current crossfade
    pub fn cancel(&mut self) {
        self.state = CrossfadeState::Inactive;
        self.position_samples = 0;
    }

    /// Reset crossfade state (after transition completes)
    pub fn reset(&mut self) {
        self.state = CrossfadeState::Inactive;
        self.position_samples = 0;
    }

    /// Get crossfade progress (0.0 to 1.0)
    pub fn progress(&self) -> f32 {
        if self.duration_samples == 0 {
            return 1.0;
        }
        (self.position_samples as f32) / (self.duration_samples as f32)
    }

    /// Get remaining crossfade duration in samples
    pub fn remaining_samples(&self) -> usize {
        self.duration_samples.saturating_sub(self.position_samples)
    }

    /// Process crossfade mixing
    ///
    /// Mixes outgoing and incoming track samples according to the fade curve.
    /// This is the main processing function called from the audio callback.
    ///
    /// # Arguments
    /// * `outgoing` - Samples from the outgoing (ending) track
    /// * `incoming` - Samples from the incoming (starting) track
    /// * `output` - Output buffer to write mixed result
    ///
    /// # Returns
    /// Number of samples written to output, and whether crossfade completed
    pub fn process(
        &mut self,
        outgoing: &[f32],
        incoming: &[f32],
        output: &mut [f32],
    ) -> (usize, bool) {
        if self.state != CrossfadeState::Active {
            // Not active - copy incoming directly
            let len = output.len().min(incoming.len());
            output[..len].copy_from_slice(&incoming[..len]);
            return (len, false);
        }

        // Gapless mode (0 duration) - instant switch
        if self.duration_samples == 0 {
            let len = output.len().min(incoming.len());
            output[..len].copy_from_slice(&incoming[..len]);
            self.state = CrossfadeState::Completed;
            return (len, true);
        }

        let samples_to_process = output
            .len()
            .min(outgoing.len())
            .min(incoming.len())
            .min(self.remaining_samples());

        // Align to stereo frame boundary (2 samples per frame)
        // This prevents processing partial frames which could cause audio artifacts
        let aligned_samples = (samples_to_process / 2) * 2;

        // Process stereo frames
        let frames = aligned_samples / 2;
        let curve = self.settings.curve;

        for frame in 0..frames {
            let sample_pos = self.position_samples + (frame * 2);
            let progress = (sample_pos as f32) / (self.duration_samples as f32);

            let out_gain = curve.calculate_gain(progress, true);
            let in_gain = curve.calculate_gain(progress, false);

            let left_idx = frame * 2;
            let right_idx = frame * 2 + 1;

            // Mix outgoing and incoming
            output[left_idx] = outgoing[left_idx] * out_gain + incoming[left_idx] * in_gain;
            output[right_idx] = outgoing[right_idx] * out_gain + incoming[right_idx] * in_gain;
        }

        self.position_samples += aligned_samples;

        let completed = self.position_samples >= self.duration_samples;
        if completed {
            self.state = CrossfadeState::Completed;
        }

        (aligned_samples, completed)
    }
}

impl Default for CrossfadeEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fade_curve_linear() {
        let curve = FadeCurve::Linear;

        // Fade in
        assert!((curve.calculate_gain(0.0, false) - 0.0).abs() < 0.001);
        assert!((curve.calculate_gain(0.5, false) - 0.5).abs() < 0.001);
        assert!((curve.calculate_gain(1.0, false) - 1.0).abs() < 0.001);

        // Fade out
        assert!((curve.calculate_gain(0.0, true) - 1.0).abs() < 0.001);
        assert!((curve.calculate_gain(0.5, true) - 0.5).abs() < 0.001);
        assert!((curve.calculate_gain(1.0, true) - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_fade_curve_equal_power() {
        let curve = FadeCurve::EqualPower;

        // At boundaries
        assert!((curve.calculate_gain(0.0, false) - 0.0).abs() < 0.001);
        assert!((curve.calculate_gain(1.0, false) - 1.0).abs() < 0.001);

        // At midpoint, both should be ~0.707 (1/sqrt(2))
        let mid_in = curve.calculate_gain(0.5, false);
        let mid_out = curve.calculate_gain(0.5, true);

        // Equal power check: in² + out² ≈ 1
        let sum_of_squares = mid_in * mid_in + mid_out * mid_out;
        assert!(
            (sum_of_squares - 1.0).abs() < 0.01,
            "Equal power: sum of squares = {}, expected ~1.0",
            sum_of_squares
        );
    }

    #[test]
    fn test_fade_curve_scurve() {
        let curve = FadeCurve::SCurve;

        // At boundaries
        assert!((curve.calculate_gain(0.0, false) - 0.0).abs() < 0.001);
        assert!((curve.calculate_gain(1.0, false) - 1.0).abs() < 0.001);

        // At midpoint should be ~0.5
        assert!((curve.calculate_gain(0.5, false) - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_crossfade_settings_default() {
        let settings = CrossfadeSettings::default();
        assert!(!settings.enabled);
        assert_eq!(settings.duration_ms, 3000);
        assert_eq!(settings.curve, FadeCurve::EqualPower);
        assert!(!settings.on_skip);
    }

    #[test]
    fn test_crossfade_settings_gapless() {
        let settings = CrossfadeSettings::gapless();
        assert!(settings.enabled);
        assert_eq!(settings.duration_ms, 0);
    }

    #[test]
    fn test_crossfade_settings_duration_samples() {
        let settings = CrossfadeSettings::with_duration(1000); // 1 second
        assert_eq!(settings.duration_samples(44100), 44100);
        assert_eq!(settings.duration_samples(48000), 48000);
    }

    #[test]
    fn test_crossfade_engine_start() {
        let mut engine = CrossfadeEngine::new();
        engine.set_settings(CrossfadeSettings {
            enabled: true,
            duration_ms: 1000,
            curve: FadeCurve::Linear,
            on_skip: false,
        });

        // Should start on auto-advance
        assert!(engine.start(false));
        assert_eq!(engine.state(), CrossfadeState::Active);

        engine.reset();

        // Should not start on manual skip (on_skip = false)
        assert!(!engine.start(true));
        assert_eq!(engine.state(), CrossfadeState::Inactive);
    }

    #[test]
    fn test_crossfade_engine_disabled() {
        let mut engine = CrossfadeEngine::new();
        engine.set_settings(CrossfadeSettings {
            enabled: false,
            ..Default::default()
        });

        // Should not start when disabled
        assert!(!engine.start(false));
        assert_eq!(engine.state(), CrossfadeState::Inactive);
    }

    #[test]
    fn test_crossfade_process_linear() {
        let mut engine = CrossfadeEngine::with_settings(CrossfadeSettings {
            enabled: true,
            duration_ms: 100, // 100ms for quick test
            curve: FadeCurve::Linear,
            on_skip: true,
        });
        // Use a valid sample rate (minimum is 8000 Hz)
        engine.set_sample_rate(8000);

        engine.start(false);

        // 100ms at 8000Hz = 800 samples, but stereo = 1600 samples total
        let outgoing = vec![1.0f32; 1600];
        let incoming = vec![0.0f32; 1600];
        let mut output = vec![0.0f32; 1600];

        let (samples, completed) = engine.process(&outgoing, &incoming, &mut output);

        assert_eq!(samples, 1600);
        assert!(completed);
        assert_eq!(engine.state(), CrossfadeState::Completed);

        // First sample should be mostly outgoing (gain ~1.0)
        assert!(output[0] > 0.9, "First sample should be mostly outgoing");

        // Last sample should be mostly incoming (gain ~0.0)
        assert!(
            output[1598] < 0.1,
            "Last sample should be mostly incoming, got {}",
            output[1598]
        );
    }

    #[test]
    fn test_crossfade_gapless_instant_switch() {
        let mut engine = CrossfadeEngine::with_settings(CrossfadeSettings::gapless());
        engine.set_sample_rate(44100);

        engine.start(false);

        let outgoing = vec![1.0f32; 100];
        let incoming = vec![0.5f32; 100];
        let mut output = vec![0.0f32; 100];

        let (samples, completed) = engine.process(&outgoing, &incoming, &mut output);

        assert_eq!(samples, 100);
        assert!(completed);

        // Gapless should copy incoming directly
        for sample in &output {
            assert!((sample - 0.5).abs() < 0.001);
        }
    }

    #[test]
    fn test_crossfade_progress() {
        let mut engine = CrossfadeEngine::with_settings(CrossfadeSettings::with_duration(1000));
        // Use valid sample rate (minimum is 8000 Hz)
        engine.set_sample_rate(8000);

        engine.start(false);
        assert!((engine.progress() - 0.0).abs() < 0.001);

        // At 8000 Hz, 1000ms crossfade = 8000 samples * 2 (stereo) = 16000 samples total
        // Process half = 8000 samples
        let outgoing = vec![1.0f32; 8000];
        let incoming = vec![0.0f32; 8000];
        let mut output = vec![0.0f32; 8000];

        engine.process(&outgoing, &incoming, &mut output);

        assert!(
            (engine.progress() - 0.5).abs() < 0.01,
            "Progress should be ~0.5, got {}",
            engine.progress()
        );
    }

    #[test]
    fn test_crossfade_cancel() {
        let mut engine = CrossfadeEngine::with_settings(CrossfadeSettings::with_duration(1000));
        engine.set_sample_rate(44100);

        engine.start(false);
        assert_eq!(engine.state(), CrossfadeState::Active);

        engine.cancel();
        assert_eq!(engine.state(), CrossfadeState::Inactive);
    }

    #[test]
    fn test_fade_curve_display_names() {
        assert_eq!(FadeCurve::Linear.display_name(), "Linear");
        assert_eq!(FadeCurve::SquareRoot.display_name(), "Square Root");
        assert_eq!(FadeCurve::SCurve.display_name(), "S-Curve");
        assert_eq!(FadeCurve::EqualPower.display_name(), "Equal Power");
    }

    #[test]
    fn test_square_root_curve() {
        let curve = FadeCurve::SquareRoot;

        // At boundaries
        assert!((curve.calculate_gain(0.0, false) - 0.0).abs() < 0.001);
        assert!((curve.calculate_gain(1.0, false) - 1.0).abs() < 0.001);

        // SquareRoot should rise faster at the start than linear
        let sqrt_mid = curve.calculate_gain(0.5, false);
        let linear_mid = FadeCurve::Linear.calculate_gain(0.5, false);

        // sqrt(0.5) ≈ 0.707, which is > 0.5
        assert!(
            sqrt_mid > linear_mid,
            "SquareRoot should rise faster: {} vs {}",
            sqrt_mid,
            linear_mid
        );
    }

    #[test]
    #[allow(deprecated)]
    fn test_logarithmic_alias_works() {
        // Test that deprecated Logarithmic alias still works
        let curve = FadeCurve::Logarithmic;
        let sqrt_curve = FadeCurve::SquareRoot;

        // Both should produce the same result
        assert_eq!(
            curve.calculate_gain(0.5, false),
            sqrt_curve.calculate_gain(0.5, false)
        );
        assert_eq!(curve.display_name(), "Square Root");
    }

    // ===== Industry Standard Compliance Tests =====

    /// Test that Equal Power maintains constant power across the entire fade range
    /// This is the key property that makes it suitable for music crossfades.
    /// Industry standard: sin²(x) + cos²(x) = 1 at all points
    #[test]
    fn test_equal_power_constant_across_range() {
        let curve = FadeCurve::EqualPower;

        // Test at multiple points across the fade range
        for i in 0..=100 {
            let t = i as f32 / 100.0;
            let gain_in = curve.calculate_gain(t, false);
            let gain_out = curve.calculate_gain(t, true);

            // Equal power: gain_in² + gain_out² should always equal 1.0
            let sum_of_squares = gain_in * gain_in + gain_out * gain_out;
            assert!(
                (sum_of_squares - 1.0).abs() < 0.001,
                "Equal power violated at t={}: sum_of_squares={}, expected 1.0",
                t,
                sum_of_squares
            );
        }
    }

    /// Test that Equal Power has the correct midpoint values (-3dB each)
    /// Industry standard: at t=0.5, both gains should be √(0.5) ≈ 0.707
    #[test]
    fn test_equal_power_midpoint_is_minus_3db() {
        let curve = FadeCurve::EqualPower;

        let mid_in = curve.calculate_gain(0.5, false);
        let mid_out = curve.calculate_gain(0.5, true);

        // √(0.5) ≈ 0.7071 which is -3dB
        let expected = (0.5f32).sqrt();

        assert!(
            (mid_in - expected).abs() < 0.001,
            "Fade-in at midpoint: {} expected {} (-3dB)",
            mid_in,
            expected
        );
        assert!(
            (mid_out - expected).abs() < 0.001,
            "Fade-out at midpoint: {} expected {} (-3dB)",
            mid_out,
            expected
        );
    }

    /// Test the new Exponential (dB-linear) curve
    /// This curve should be linear in the dB domain for perceptual linearity
    #[test]
    fn test_exponential_curve_boundaries() {
        let curve = FadeCurve::Exponential;

        // At boundaries
        assert!(
            curve.calculate_gain(0.0, false).abs() < 0.001,
            "Exponential should start at 0"
        );
        assert!(
            (curve.calculate_gain(1.0, false) - 1.0).abs() < 0.001,
            "Exponential should end at 1"
        );
    }

    /// Test that Exponential curve rises slower at the start than linear
    /// This is the characteristic shape of dB-linear fades - they rise very
    /// slowly at first (because dB is logarithmic) and rapidly at the end.
    #[test]
    fn test_exponential_curve_shape() {
        let curve = FadeCurve::Exponential;
        let linear = FadeCurve::Linear;

        // At low values, exponential should be below linear (slower rise)
        let exp_25 = curve.calculate_gain(0.25, false);
        let lin_25 = linear.calculate_gain(0.25, false);
        assert!(
            exp_25 < lin_25,
            "Exponential should rise slower at start: exp={} vs lin={}",
            exp_25,
            lin_25
        );

        // At 50%, exponential should still be below linear for dB-linear curves
        // This is because most of the perceived loudness change happens in the upper range
        let exp_50 = curve.calculate_gain(0.5, false);
        let lin_50 = linear.calculate_gain(0.5, false);
        assert!(
            exp_50 < lin_50,
            "Exponential should still be below linear at midpoint: exp={} vs lin={}",
            exp_50,
            lin_50
        );

        // Near the end (95%+), exponential rapidly catches up
        let exp_95 = curve.calculate_gain(0.95, false);
        assert!(
            exp_95 > 0.5,
            "Exponential should be above 0.5 at t=0.95: exp={}",
            exp_95
        );
    }

    /// Test Exponential curve is monotonically increasing (never decreases)
    #[test]
    fn test_exponential_monotonic() {
        let curve = FadeCurve::Exponential;

        let mut prev = 0.0f32;
        for i in 0..=100 {
            let t = i as f32 / 100.0;
            let gain = curve.calculate_gain(t, false);
            assert!(
                gain >= prev,
                "Exponential should be monotonically increasing: at t={}, gain {} < prev {}",
                t,
                gain,
                prev
            );
            prev = gain;
        }
    }

    /// Test Exponential curve display name
    #[test]
    fn test_exponential_display_name() {
        assert_eq!(
            FadeCurve::Exponential.display_name(),
            "Exponential (dB-linear)"
        );
    }

    /// Test that Linear crossfade has the documented 3dB dip at midpoint
    /// This verifies our documentation is accurate about Linear's limitation
    #[test]
    fn test_linear_has_3db_dip_at_midpoint() {
        let curve = FadeCurve::Linear;

        let mid_in = curve.calculate_gain(0.5, false);
        let mid_out = curve.calculate_gain(0.5, true);

        // Linear: 0.5 + 0.5 = 1.0 amplitude sum
        // But power = 0.5² + 0.5² = 0.5 (-3dB)
        let sum_of_squares = mid_in * mid_in + mid_out * mid_out;

        assert!(
            (sum_of_squares - 0.5).abs() < 0.01,
            "Linear should have 3dB dip: sum_of_squares={}, expected 0.5 (-3dB)",
            sum_of_squares
        );
    }

    /// Test that all curves reach true silence at t=0 and full volume at t=1
    #[test]
    fn test_all_curves_boundary_values() {
        let curves = [
            FadeCurve::Linear,
            FadeCurve::SquareRoot,
            FadeCurve::SCurve,
            FadeCurve::EqualPower,
            FadeCurve::Exponential,
        ];

        for curve in &curves {
            let start = curve.calculate_gain(0.0, false);
            let end = curve.calculate_gain(1.0, false);

            assert!(
                start.abs() < 0.001,
                "{:?} should start at 0, got {}",
                curve,
                start
            );
            assert!(
                (end - 1.0).abs() < 0.001,
                "{:?} should end at 1, got {}",
                curve,
                end
            );
        }
    }

    /// Test that all curves are monotonically increasing for fade-in
    #[test]
    fn test_all_curves_monotonic() {
        let curves = [
            FadeCurve::Linear,
            FadeCurve::SquareRoot,
            FadeCurve::SCurve,
            FadeCurve::EqualPower,
            FadeCurve::Exponential,
        ];

        for curve in &curves {
            let mut prev = 0.0f32;
            for i in 0..=100 {
                let t = i as f32 / 100.0;
                let gain = curve.calculate_gain(t, false);
                assert!(
                    gain >= prev - 0.0001, // Small tolerance for floating point
                    "{:?} should be monotonic: at t={}, gain {} < prev {}",
                    curve,
                    t,
                    gain,
                    prev
                );
                prev = gain;
            }
        }
    }

    /// Test default crossfade duration is within industry norms (1-5 seconds typical)
    #[test]
    fn test_default_duration_industry_standard() {
        let settings = CrossfadeSettings::default();

        // Industry standard: most music players use 1-5 seconds, with 2-3 being common
        // Spotify recommends 5 seconds as optimal
        assert!(
            settings.duration_ms >= 1000 && settings.duration_ms <= 5000,
            "Default duration {}ms should be within industry norm of 1-5 seconds",
            settings.duration_ms
        );
    }

    /// Test that Equal Power is the default curve (industry best practice for music)
    #[test]
    fn test_equal_power_is_default() {
        let settings = CrossfadeSettings::default();
        assert_eq!(
            settings.curve,
            FadeCurve::EqualPower,
            "Equal Power should be default as it's the industry standard for music"
        );
    }

    // ========================================
    // Configuration Validation Tests
    // ========================================

    #[test]
    fn test_crossfade_settings_validation_valid() {
        let settings = CrossfadeSettings::default();
        assert!(settings.validate().is_ok());

        let settings = CrossfadeSettings::with_duration(5000);
        assert!(settings.validate().is_ok());

        let settings = CrossfadeSettings::gapless();
        assert!(settings.validate().is_ok());
    }

    #[test]
    fn test_crossfade_settings_validation_duration_too_high() {
        let settings = CrossfadeSettings {
            duration_ms: 15000,
            ..Default::default()
        };
        let result = settings.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn test_crossfade_settings_validated_clamps_duration() {
        let settings = CrossfadeSettings {
            duration_ms: 20000,
            ..Default::default()
        };
        let validated = settings.validated();
        assert_eq!(validated.duration_ms, MAX_CROSSFADE_DURATION_MS);
        assert!(validated.validate().is_ok());
    }

    #[test]
    fn test_crossfade_with_duration_clamps() {
        // Normal duration
        let settings = CrossfadeSettings::with_duration(5000);
        assert_eq!(settings.duration_ms, 5000);

        // Excessive duration should be clamped
        let settings = CrossfadeSettings::with_duration(50000);
        assert_eq!(settings.duration_ms, MAX_CROSSFADE_DURATION_MS);
    }

    #[test]
    fn test_crossfade_with_duration_and_curve() {
        let settings = CrossfadeSettings::with_duration_and_curve(3000, FadeCurve::Linear);
        assert_eq!(settings.duration_ms, 3000);
        assert_eq!(settings.curve, FadeCurve::Linear);
        assert!(settings.enabled);

        // Excessive duration should be clamped
        let settings = CrossfadeSettings::with_duration_and_curve(50000, FadeCurve::SCurve);
        assert_eq!(settings.duration_ms, MAX_CROSSFADE_DURATION_MS);
    }

    #[test]
    fn test_duration_samples_with_zero_sample_rate() {
        let settings = CrossfadeSettings::with_duration(1000);
        // Should not panic, returns 0
        let samples = settings.duration_samples(0);
        assert_eq!(samples, 0);
    }

    #[test]
    fn test_duration_samples_calculation() {
        let settings = CrossfadeSettings::with_duration(1000); // 1 second

        // At 44100 Hz, 1 second = 44100 samples
        assert_eq!(settings.duration_samples(44100), 44100);

        // At 48000 Hz, 1 second = 48000 samples
        assert_eq!(settings.duration_samples(48000), 48000);

        // At 96000 Hz, 1 second = 96000 samples
        assert_eq!(settings.duration_samples(96000), 96000);
    }

    #[test]
    fn test_crossfade_engine_sample_rate_clamping() {
        let mut engine = CrossfadeEngine::new();

        // Very low sample rate should be clamped
        engine.set_sample_rate(100);
        assert_eq!(engine.sample_rate, MIN_SAMPLE_RATE);

        // Very high sample rate should be clamped
        engine.set_sample_rate(1000000);
        assert_eq!(engine.sample_rate, MAX_SAMPLE_RATE);

        // Valid sample rate should pass through
        engine.set_sample_rate(48000);
        assert_eq!(engine.sample_rate, 48000);
    }

    #[test]
    fn test_crossfade_engine_settings_update() {
        let mut engine = CrossfadeEngine::new();

        // Update settings
        let new_settings = CrossfadeSettings::with_duration(5000);
        engine.set_settings(new_settings);

        assert_eq!(engine.settings().duration_ms, 5000);
        assert!(engine.settings().enabled);
    }

    #[test]
    fn test_crossfade_engine_start_respects_enabled() {
        let mut engine = CrossfadeEngine::new();

        // Default is disabled
        assert!(!engine.start(false));
        assert!(!engine.is_active());

        // Enable crossfade
        engine.set_settings(CrossfadeSettings::with_duration(1000));
        assert!(engine.start(false));
        assert!(engine.is_active());
    }

    #[test]
    fn test_crossfade_engine_on_skip_setting() {
        let mut engine = CrossfadeEngine::new();
        engine.set_settings(CrossfadeSettings {
            enabled: true,
            duration_ms: 1000,
            curve: FadeCurve::Linear,
            on_skip: false,
        });

        // Manual skip should not start crossfade when on_skip is false
        assert!(!engine.start(true));
        assert!(!engine.is_active());

        // Auto-advance should still work
        assert!(engine.start(false));
        assert!(engine.is_active());

        engine.reset();

        // Enable on_skip
        let mut settings = engine.settings().clone();
        settings.on_skip = true;
        engine.set_settings(settings);

        // Now manual skip should start crossfade
        assert!(engine.start(true));
        assert!(engine.is_active());
    }
}
