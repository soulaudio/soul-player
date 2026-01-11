//! Crossfade engine for smooth track transitions
//!
//! Provides multiple fade curve types for seamless transitions between tracks:
//! - Linear: Simple linear fade (note: has 3dB volume dip at midpoint)
//! - SquareRoot: Faster rise than linear, natural-sounding transitions
//! - S-Curve: Smooth transitions with slow start/end
//! - Equal Power: Constant perceived loudness (best for music, default)

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
    #[default]
    EqualPower,
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
        }
    }

    /// Get a human-readable name for the curve
    #[allow(deprecated)]
    pub fn display_name(&self) -> &'static str {
        match self {
            FadeCurve::Linear => "Linear",
            FadeCurve::SquareRoot => "Square Root",
            FadeCurve::Logarithmic => "Square Root", // Deprecated alias
            FadeCurve::SCurve => "S-Curve",
            FadeCurve::EqualPower => "Equal Power",
        }
    }
}

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
            duration_ms: 3000, // 3 second default
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
    pub fn with_duration(duration_ms: u32) -> Self {
        Self {
            enabled: true,
            duration_ms: duration_ms.min(10000),
            curve: FadeCurve::EqualPower,
            on_skip: false,
        }
    }

    /// Get duration in samples for a given sample rate
    pub fn duration_samples(&self, sample_rate: u32) -> usize {
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
    pub fn set_sample_rate(&mut self, sample_rate: u32) {
        self.sample_rate = sample_rate;
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

        // Process stereo frames
        let frames = samples_to_process / 2;
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

        self.position_samples += samples_to_process;

        let completed = self.position_samples >= self.duration_samples;
        if completed {
            self.state = CrossfadeState::Completed;
        }

        (samples_to_process, completed)
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
        engine.set_sample_rate(1000); // 1000 Hz for easy math

        engine.start(false);

        // 100ms at 1000Hz = 100 samples, but stereo = 200 samples total
        let outgoing = vec![1.0f32; 200];
        let incoming = vec![0.0f32; 200];
        let mut output = vec![0.0f32; 200];

        let (samples, completed) = engine.process(&outgoing, &incoming, &mut output);

        assert_eq!(samples, 200);
        assert!(completed);
        assert_eq!(engine.state(), CrossfadeState::Completed);

        // First sample should be mostly outgoing (gain ~1.0)
        assert!(output[0] > 0.9, "First sample should be mostly outgoing");

        // Last sample should be mostly incoming (gain ~0.0)
        assert!(
            output[198] < 0.1,
            "Last sample should be mostly incoming, got {}",
            output[198]
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
        engine.set_sample_rate(1000); // 1000 Hz

        engine.start(false);
        assert!((engine.progress() - 0.0).abs() < 0.001);

        // Process half
        let outgoing = vec![1.0f32; 1000]; // 500 stereo frames = 1000 samples
        let incoming = vec![0.0f32; 1000];
        let mut output = vec![0.0f32; 1000];

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
}
