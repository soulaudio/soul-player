//! Crossfeed (Bauer stereophonic-to-binaural DSP)
//!
//! Reduces stereo width and adds interaural crosstalk to reduce listener fatigue
//! when using headphones. Based on the Bauer stereophonic-to-binaural DSP algorithm.
//!
//! The crossfeed effect simulates how sound from speakers reaches both ears,
//! making headphone listening more natural and comfortable for extended sessions.

use super::chain::AudioEffect;
use std::f32::consts::PI;

/// Crossfeed preset
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CrossfeedPreset {
    /// Natural: Subtle crossfeed for a more natural soundstage
    /// Level: -4.5 dB, Cutoff: 700 Hz
    #[default]
    Natural,

    /// Relaxed: Moderate crossfeed for casual listening
    /// Level: -6 dB, Cutoff: 650 Hz
    Relaxed,

    /// Meier: Based on Jan Meier's crossfeed algorithm
    /// Level: -9 dB, Cutoff: 550 Hz
    Meier,

    /// Custom: User-defined settings
    Custom,
}

impl CrossfeedPreset {
    /// Get crossfeed level in dB for this preset
    pub fn level_db(&self) -> f32 {
        match self {
            Self::Natural => -4.5,
            Self::Relaxed => -6.0,
            Self::Meier => -9.0,
            Self::Custom => -6.0, // Default for custom
        }
    }

    /// Get cutoff frequency in Hz for this preset
    pub fn cutoff_hz(&self) -> f32 {
        match self {
            Self::Natural => 700.0,
            Self::Relaxed => 650.0,
            Self::Meier => 550.0,
            Self::Custom => 650.0, // Default for custom
        }
    }
}

/// Crossfeed settings
#[derive(Debug, Clone)]
pub struct CrossfeedSettings {
    /// Crossfeed level in dB (negative values, -3 to -12 dB typical)
    pub level_db: f32,

    /// Low-pass filter cutoff frequency in Hz (300-1000 Hz typical)
    pub cutoff_hz: f32,

    /// Preset being used
    pub preset: CrossfeedPreset,
}

impl Default for CrossfeedSettings {
    fn default() -> Self {
        Self {
            level_db: CrossfeedPreset::Natural.level_db(),
            cutoff_hz: CrossfeedPreset::Natural.cutoff_hz(),
            preset: CrossfeedPreset::Natural,
        }
    }
}

impl CrossfeedSettings {
    /// Create settings from a preset
    pub fn from_preset(preset: CrossfeedPreset) -> Self {
        Self {
            level_db: preset.level_db(),
            cutoff_hz: preset.cutoff_hz(),
            preset,
        }
    }

    /// Create custom settings
    pub fn custom(level_db: f32, cutoff_hz: f32) -> Self {
        Self {
            level_db: level_db.clamp(-12.0, -3.0),
            cutoff_hz: cutoff_hz.clamp(300.0, 1000.0),
            preset: CrossfeedPreset::Custom,
        }
    }
}

/// Single-pole low-pass filter for crossfeed
#[derive(Debug, Clone)]
struct LowPassFilter {
    coefficient: f32,
    state: f32,
}

impl LowPassFilter {
    fn new() -> Self {
        Self {
            coefficient: 0.5,
            state: 0.0,
        }
    }

    fn set_cutoff(&mut self, cutoff_hz: f32, sample_rate: f32) {
        // Single-pole IIR low-pass filter coefficient
        let omega = 2.0 * PI * cutoff_hz / sample_rate;
        self.coefficient = omega / (omega + 1.0);
    }

    #[inline]
    fn process(&mut self, input: f32) -> f32 {
        // y[n] = y[n-1] + coefficient * (x[n] - y[n-1])
        self.state += self.coefficient * (input - self.state);
        self.state
    }

    fn reset(&mut self) {
        self.state = 0.0;
    }
}

/// Crossfeed effect (Bauer stereophonic-to-binaural DSP)
///
/// Adds controlled crosstalk between stereo channels with low-pass filtering
/// to simulate natural speaker listening through headphones.
pub struct Crossfeed {
    /// Crossfeed mix level (0.0 to 1.0, derived from dB)
    level: f32,

    /// Low-pass filter for left-to-right crossfeed
    lpf_l_to_r: LowPassFilter,

    /// Low-pass filter for right-to-left crossfeed
    lpf_r_to_l: LowPassFilter,

    /// Current settings
    settings: CrossfeedSettings,

    /// Effect enabled state
    enabled: bool,

    /// Current sample rate
    sample_rate: u32,

    /// Settings need recalculation
    needs_update: bool,
}

impl Crossfeed {
    /// Create a new crossfeed effect with default settings
    pub fn new() -> Self {
        Self::with_settings(CrossfeedSettings::default())
    }

    /// Create a crossfeed effect with a specific preset
    pub fn with_preset(preset: CrossfeedPreset) -> Self {
        Self::with_settings(CrossfeedSettings::from_preset(preset))
    }

    /// Create a crossfeed effect with custom settings
    pub fn with_settings(settings: CrossfeedSettings) -> Self {
        Self {
            level: 0.0,
            lpf_l_to_r: LowPassFilter::new(),
            lpf_r_to_l: LowPassFilter::new(),
            settings,
            enabled: true,
            sample_rate: 44100,
            needs_update: true,
        }
    }

    /// Set crossfeed preset
    pub fn set_preset(&mut self, preset: CrossfeedPreset) {
        self.settings = CrossfeedSettings::from_preset(preset);
        self.needs_update = true;
    }

    /// Set custom crossfeed level in dB
    pub fn set_level_db(&mut self, level_db: f32) {
        self.settings.level_db = level_db.clamp(-12.0, -3.0);
        self.settings.preset = CrossfeedPreset::Custom;
        self.needs_update = true;
    }

    /// Set custom cutoff frequency in Hz
    pub fn set_cutoff_hz(&mut self, cutoff_hz: f32) {
        self.settings.cutoff_hz = cutoff_hz.clamp(300.0, 1000.0);
        self.settings.preset = CrossfeedPreset::Custom;
        self.needs_update = true;
    }

    /// Get current settings
    pub fn settings(&self) -> &CrossfeedSettings {
        &self.settings
    }

    /// Get current preset
    pub fn preset(&self) -> CrossfeedPreset {
        self.settings.preset
    }

    /// Update internal parameters based on settings
    fn update_parameters(&mut self) {
        if self.needs_update {
            // Convert dB to linear level
            self.level = 10.0_f32.powf(self.settings.level_db / 20.0);

            // Update filter cutoffs
            let sr = self.sample_rate as f32;
            self.lpf_l_to_r.set_cutoff(self.settings.cutoff_hz, sr);
            self.lpf_r_to_l.set_cutoff(self.settings.cutoff_hz, sr);

            self.needs_update = false;
        }
    }
}

impl Default for Crossfeed {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioEffect for Crossfeed {
    fn process(&mut self, buffer: &mut [f32], sample_rate: u32) {
        if !self.enabled {
            return;
        }

        // Update sample rate if changed
        if self.sample_rate != sample_rate {
            self.sample_rate = sample_rate;
            self.needs_update = true;
        }

        self.update_parameters();

        // Process interleaved stereo buffer
        for chunk in buffer.chunks_exact_mut(2) {
            let left = chunk[0];
            let right = chunk[1];

            // Apply low-pass filtering to crossfeed signals
            let crossfeed_to_right = self.lpf_l_to_r.process(left);
            let crossfeed_to_left = self.lpf_r_to_l.process(right);

            // Mix original with crossfeed
            // The crossfeed signal is inverted (negative phase) to simulate
            // the delay and cancellation effects of speaker listening
            let new_left = left - self.level * crossfeed_to_left;
            let new_right = right - self.level * crossfeed_to_right;

            // Apply gain compensation to maintain overall level
            // When mixing in crossfeed, we need to normalize
            let compensation = 1.0 / (1.0 + self.level * 0.5);

            chunk[0] = new_left * compensation;
            chunk[1] = new_right * compensation;
        }
    }

    fn reset(&mut self) {
        self.lpf_l_to_r.reset();
        self.lpf_r_to_l.reset();
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn name(&self) -> &str {
        "Crossfeed"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preset_values() {
        assert_eq!(CrossfeedPreset::Natural.level_db(), -4.5);
        assert_eq!(CrossfeedPreset::Natural.cutoff_hz(), 700.0);

        assert_eq!(CrossfeedPreset::Meier.level_db(), -9.0);
        assert_eq!(CrossfeedPreset::Meier.cutoff_hz(), 550.0);
    }

    #[test]
    fn test_create_with_preset() {
        let crossfeed = Crossfeed::with_preset(CrossfeedPreset::Relaxed);
        assert_eq!(crossfeed.preset(), CrossfeedPreset::Relaxed);
        assert_eq!(crossfeed.settings().level_db, -6.0);
        assert_eq!(crossfeed.settings().cutoff_hz, 650.0);
    }

    #[test]
    fn test_custom_settings() {
        let mut crossfeed = Crossfeed::new();
        crossfeed.set_level_db(-8.0);
        crossfeed.set_cutoff_hz(600.0);

        assert_eq!(crossfeed.preset(), CrossfeedPreset::Custom);
        assert_eq!(crossfeed.settings().level_db, -8.0);
        assert_eq!(crossfeed.settings().cutoff_hz, 600.0);
    }

    #[test]
    fn test_settings_clamping() {
        let settings = CrossfeedSettings::custom(-20.0, 100.0);
        assert_eq!(settings.level_db, -12.0); // Clamped to -12
        assert_eq!(settings.cutoff_hz, 300.0); // Clamped to 300
    }

    #[test]
    fn test_process_stereo() {
        let mut crossfeed = Crossfeed::with_preset(CrossfeedPreset::Natural);

        // Hard-panned signal (full left)
        let mut buffer: Vec<f32> = vec![1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0];

        crossfeed.process(&mut buffer, 44100);

        // After crossfeed, right channel should have some signal
        // (crossfeed from left)
        let processed_right = buffer[7]; // Last right sample
        assert!(processed_right.abs() > 0.01, "Crossfeed should add signal to silent channel");
    }

    #[test]
    fn test_mono_signal_unchanged() {
        let mut crossfeed = Crossfeed::with_preset(CrossfeedPreset::Natural);

        // Mono signal (same on both channels)
        let mut buffer: Vec<f32> = vec![0.5, 0.5, 0.5, 0.5];

        crossfeed.process(&mut buffer, 44100);

        // Mono signal should remain mono (crossfeed cancels out)
        assert!(
            (buffer[0] - buffer[1]).abs() < 0.01,
            "Mono signal should remain balanced"
        );
    }

    #[test]
    fn test_disabled_bypass() {
        let mut crossfeed = Crossfeed::new();
        crossfeed.set_enabled(false);

        let mut buffer = vec![1.0, 0.0, 1.0, 0.0];
        let original = buffer.clone();

        crossfeed.process(&mut buffer, 44100);

        assert_eq!(buffer, original, "Disabled effect should bypass");
    }

    #[test]
    fn test_reset() {
        let mut crossfeed = Crossfeed::new();

        // Process some samples
        let mut buffer: Vec<f32> = (0..100).flat_map(|_| [1.0, 0.0]).collect();
        crossfeed.process(&mut buffer, 44100);

        // Reset
        crossfeed.reset();

        // Process again - filter state should be reset
        let mut buffer2: Vec<f32> = (0..100).flat_map(|_| [1.0, 0.0]).collect();
        crossfeed.process(&mut buffer2, 44100);

        // Results should be deterministic after reset
        assert_eq!(buffer, buffer2);
    }

    #[test]
    fn test_name() {
        let crossfeed = Crossfeed::new();
        assert_eq!(crossfeed.name(), "Crossfeed");
    }
}
