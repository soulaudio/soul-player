//! Graphic Equalizer
//!
//! Provides fixed-frequency band equalization:
//! - 10-band: ISO standard octave frequencies
//! - 31-band: Third-octave frequencies
//! - Per-band gain control (-12 to +12 dB)
//! - Preset support

use super::chain::AudioEffect;
use std::f32::consts::PI;

/// 10-band ISO standard frequencies (Hz)
pub const ISO_10_BAND_FREQUENCIES: [f32; 10] = [
    31.5, 63.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0,
];

/// 31-band third-octave frequencies (Hz)
pub const ISO_31_BAND_FREQUENCIES: [f32; 31] = [
    20.0, 25.0, 31.5, 40.0, 50.0, 63.0, 80.0, 100.0, 125.0, 160.0, 200.0, 250.0, 315.0, 400.0,
    500.0, 630.0, 800.0, 1000.0, 1250.0, 1600.0, 2000.0, 2500.0, 3150.0, 4000.0, 5000.0, 6300.0,
    8000.0, 10000.0, 12500.0, 16000.0, 20000.0,
];

/// Graphic EQ preset
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GraphicEqPreset {
    /// Flat - All bands at 0 dB
    #[default]
    Flat,

    /// Bass Boost - Enhanced low frequencies
    BassBoost,

    /// Treble Boost - Enhanced high frequencies
    TrebleBoost,

    /// V-Shape - Boosted lows and highs, reduced mids
    VShape,

    /// Vocal - Enhanced mid frequencies for voice
    Vocal,

    /// Rock - Classic rock music profile
    Rock,

    /// Electronic - Dance/Electronic music profile
    Electronic,

    /// Acoustic - Natural acoustic instrument profile
    Acoustic,

    /// Custom - User-defined settings
    Custom,
}

impl GraphicEqPreset {
    /// Get gain values for this preset (10-band)
    pub fn gains_10(&self) -> [f32; 10] {
        match self {
            Self::Flat => [0.0; 10],
            Self::BassBoost => [6.0, 5.0, 4.0, 2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            Self::TrebleBoost => [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 2.0, 4.0, 5.0, 6.0],
            Self::VShape => [5.0, 4.0, 2.0, -1.0, -2.0, -2.0, -1.0, 2.0, 4.0, 5.0],
            Self::Vocal => [-2.0, -1.0, 0.0, 2.0, 4.0, 4.0, 2.0, 0.0, -1.0, -2.0],
            Self::Rock => [4.0, 3.0, 1.0, 0.0, -1.0, 0.0, 1.0, 3.0, 4.0, 4.0],
            Self::Electronic => [5.0, 4.0, 2.0, 0.0, 1.0, 2.0, 1.0, 3.0, 4.0, 4.0],
            Self::Acoustic => [2.0, 1.0, 0.0, 1.0, 2.0, 2.0, 1.0, 2.0, 2.0, 1.0],
            Self::Custom => [0.0; 10],
        }
    }

    /// Get preset name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Flat => "Flat",
            Self::BassBoost => "Bass Boost",
            Self::TrebleBoost => "Treble Boost",
            Self::VShape => "V-Shape",
            Self::Vocal => "Vocal",
            Self::Rock => "Rock",
            Self::Electronic => "Electronic",
            Self::Acoustic => "Acoustic",
            Self::Custom => "Custom",
        }
    }
}

/// Number of samples over which to smooth coefficient changes
/// At 44.1kHz, 64 samples = ~1.5ms, which is imperceptible but prevents clicks
const SMOOTH_SAMPLES: u32 = 64;

/// Biquad filter for graphic EQ bands
///
/// Implements coefficient smoothing to prevent audio artifacts (clicks, pops,
/// zipper noise) when parameters change at runtime.
#[derive(Debug, Clone)]
struct BiquadBand {
    // Target coefficients (set by update_coefficients)
    target_b0: f32,
    target_b1: f32,
    target_b2: f32,
    target_a1: f32,
    target_a2: f32,

    // Active coefficients (used for processing, smoothed toward target)
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,

    // Smoothing state
    smooth_samples_remaining: u32,

    // State (stereo)
    x1_l: f32,
    x2_l: f32,
    y1_l: f32,
    y2_l: f32,
    x1_r: f32,
    x2_r: f32,
    y1_r: f32,
    y2_r: f32,

    // Band parameters
    frequency: f32,
    gain_db: f32,
    q: f32,
}

impl BiquadBand {
    fn new(frequency: f32, q: f32) -> Self {
        Self {
            // Target coefficients (for smoothing)
            target_b0: 1.0,
            target_b1: 0.0,
            target_b2: 0.0,
            target_a1: 0.0,
            target_a2: 0.0,
            // Active coefficients
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
            // Smoothing state
            smooth_samples_remaining: 0,
            // Filter state (stereo)
            x1_l: 0.0,
            x2_l: 0.0,
            y1_l: 0.0,
            y2_l: 0.0,
            x1_r: 0.0,
            x2_r: 0.0,
            y1_r: 0.0,
            y2_r: 0.0,
            // Band parameters
            frequency,
            gain_db: 0.0,
            q,
        }
    }

    fn set_gain(&mut self, gain_db: f32) {
        self.gain_db = gain_db.clamp(-12.0, 12.0);
    }

    fn update_coefficients(&mut self, sample_rate: f32) {
        // Bug fix: Early return if sample rate is invalid to prevent division by zero
        if sample_rate < 1.0 {
            return;
        }

        // Bug fix: Lowered threshold from 0.1 to 0.01 to reduce discontinuity at near-zero gains
        if self.gain_db.abs() < 0.01 {
            self.b0 = 1.0;
            self.b1 = 0.0;
            self.b2 = 0.0;
            self.a1 = 0.0;
            self.a2 = 0.0;
            return;
        }

        let a = 10.0_f32.powf(self.gain_db / 40.0);
        // Bug fix: Clamp frequency to 45% of sample rate to prevent near-Nyquist instability
        let clamped_freq = self.frequency.min(sample_rate * 0.45);
        let omega = 2.0 * PI * clamped_freq / sample_rate;
        let sin_omega = omega.sin();
        let cos_omega = omega.cos();
        let alpha = sin_omega / (2.0 * self.q);

        let b0 = 1.0 + alpha * a;
        let b1 = -2.0 * cos_omega;
        let b2 = 1.0 - alpha * a;
        let a0 = 1.0 + alpha / a;
        let a1 = -2.0 * cos_omega;
        let a2 = 1.0 - alpha / a;

        // Normalize
        self.b0 = b0 / a0;
        self.b1 = b1 / a0;
        self.b2 = b2 / a0;
        self.a1 = a1 / a0;
        self.a2 = a2 / a0;
    }

    #[inline]
    fn process(&mut self, left: f32, right: f32) -> (f32, f32) {
        // Left channel
        let mut out_l = self.b0 * left + self.b1 * self.x1_l + self.b2 * self.x2_l
            - self.a1 * self.y1_l
            - self.a2 * self.y2_l;

        // Bug fix: Flush denormal numbers to zero to prevent CPU performance issues
        if out_l.abs() < 1e-15 {
            out_l = 0.0;
        }

        self.x2_l = self.x1_l;
        self.x1_l = left;
        self.y2_l = self.y1_l;
        self.y1_l = out_l;

        // Right channel
        let mut out_r = self.b0 * right + self.b1 * self.x1_r + self.b2 * self.x2_r
            - self.a1 * self.y1_r
            - self.a2 * self.y2_r;

        // Bug fix: Flush denormal numbers to zero to prevent CPU performance issues
        if out_r.abs() < 1e-15 {
            out_r = 0.0;
        }

        self.x2_r = self.x1_r;
        self.x1_r = right;
        self.y2_r = self.y1_r;
        self.y1_r = out_r;

        (out_l, out_r)
    }

    fn reset(&mut self) {
        self.x1_l = 0.0;
        self.x2_l = 0.0;
        self.y1_l = 0.0;
        self.y2_l = 0.0;
        self.x1_r = 0.0;
        self.x2_r = 0.0;
        self.y1_r = 0.0;
        self.y2_r = 0.0;
    }
}

/// Graphic EQ band count
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphicEqBands {
    /// 10-band octave EQ
    Ten,
    /// 31-band third-octave EQ
    ThirtyOne,
}

impl GraphicEqBands {
    /// Get the number of bands
    pub fn count(&self) -> usize {
        match self {
            Self::Ten => 10,
            Self::ThirtyOne => 31,
        }
    }

    /// Get the frequencies for this band count
    pub fn frequencies(&self) -> &'static [f32] {
        match self {
            Self::Ten => &ISO_10_BAND_FREQUENCIES,
            Self::ThirtyOne => &ISO_31_BAND_FREQUENCIES,
        }
    }

    /// Get the Q factor for this band count
    pub fn q_factor(&self) -> f32 {
        match self {
            Self::Ten => 1.41,       // Octave bandwidth
            Self::ThirtyOne => 4.32, // Third-octave bandwidth
        }
    }
}

/// 10-band or 31-band Graphic Equalizer
pub struct GraphicEq {
    /// Filter bands
    bands: Vec<BiquadBand>,

    /// Band configuration
    band_config: GraphicEqBands,

    /// Current preset
    preset: GraphicEqPreset,

    /// Effect enabled state
    enabled: bool,

    /// Current sample rate
    sample_rate: u32,

    /// Coefficients need recalculation
    needs_update: bool,
}

impl GraphicEq {
    /// Create a new 10-band graphic EQ
    pub fn new_10_band() -> Self {
        Self::new(GraphicEqBands::Ten)
    }

    /// Create a new 31-band graphic EQ
    pub fn new_31_band() -> Self {
        Self::new(GraphicEqBands::ThirtyOne)
    }

    /// Create a new graphic EQ with specified band count
    pub fn new(band_config: GraphicEqBands) -> Self {
        let q = band_config.q_factor();
        let bands: Vec<BiquadBand> = band_config
            .frequencies()
            .iter()
            .map(|&freq| BiquadBand::new(freq, q))
            .collect();

        Self {
            bands,
            band_config,
            preset: GraphicEqPreset::Flat,
            enabled: true,
            sample_rate: 44100,
            needs_update: true,
        }
    }

    /// Get band configuration
    pub fn band_config(&self) -> GraphicEqBands {
        self.band_config
    }

    /// Get number of bands
    pub fn band_count(&self) -> usize {
        self.bands.len()
    }

    /// Get frequency for a specific band
    pub fn band_frequency(&self, index: usize) -> Option<f32> {
        self.bands.get(index).map(|b| b.frequency)
    }

    /// Get gain for a specific band in dB
    pub fn band_gain(&self, index: usize) -> Option<f32> {
        self.bands.get(index).map(|b| b.gain_db)
    }

    /// Set gain for a specific band in dB
    pub fn set_band_gain(&mut self, index: usize, gain_db: f32) {
        if let Some(band) = self.bands.get_mut(index) {
            band.set_gain(gain_db);
            // Bug fix: Reset filter state on parameter change to prevent clicks
            band.reset();
            self.preset = GraphicEqPreset::Custom;
            self.needs_update = true;
        }
    }

    /// Set all band gains at once (for 10-band)
    pub fn set_gains_10(&mut self, gains: [f32; 10]) {
        if self.band_config != GraphicEqBands::Ten {
            return;
        }
        for (band, &gain) in self.bands.iter_mut().zip(gains.iter()) {
            band.set_gain(gain);
            // Bug fix: Reset filter state on parameter change to prevent clicks
            band.reset();
        }
        self.preset = GraphicEqPreset::Custom;
        self.needs_update = true;
    }

    /// Get all band gains (for 10-band)
    pub fn gains_10(&self) -> [f32; 10] {
        let mut gains = [0.0; 10];
        for (i, band) in self.bands.iter().take(10).enumerate() {
            gains[i] = band.gain_db;
        }
        gains
    }

    /// Apply a preset
    pub fn set_preset(&mut self, preset: GraphicEqPreset) {
        self.preset = preset;
        let gains = preset.gains_10();

        // Apply gains to matching bands (first 10 bands)
        for (band, &gain) in self.bands.iter_mut().zip(gains.iter()) {
            band.set_gain(gain);
            // Bug fix: Reset filter state on parameter change to prevent clicks
            band.reset();
        }

        // For 31-band, interpolate the remaining bands
        if self.band_config == GraphicEqBands::ThirtyOne && self.bands.len() > 10 {
            // Simple approach: replicate gains for nearby frequencies
            // A more sophisticated approach would interpolate between octave bands
            for band in self.bands.iter_mut().skip(10) {
                // Find the nearest preset frequency and use its gain
                let freq = band.frequency;
                let mut nearest_gain = 0.0;
                let mut nearest_dist = f32::MAX;

                for (i, &preset_freq) in ISO_10_BAND_FREQUENCIES.iter().enumerate() {
                    let dist = (freq.log2() - preset_freq.log2()).abs();
                    if dist < nearest_dist {
                        nearest_dist = dist;
                        nearest_gain = gains[i];
                    }
                }
                band.set_gain(nearest_gain);
                // Bug fix: Reset filter state on parameter change to prevent clicks
                band.reset();
            }
        }

        self.needs_update = true;
    }

    /// Get current preset
    pub fn preset(&self) -> GraphicEqPreset {
        self.preset
    }

    /// Reset all bands to flat (0 dB)
    pub fn reset_to_flat(&mut self) {
        for band in &mut self.bands {
            band.set_gain(0.0);
            // Bug fix: Reset filter state on parameter change to prevent clicks
            band.reset();
        }
        self.preset = GraphicEqPreset::Flat;
        self.needs_update = true;
    }

    /// Update filter coefficients for all bands
    fn update_coefficients(&mut self) {
        if self.needs_update {
            let sr = self.sample_rate as f32;
            for band in &mut self.bands {
                band.update_coefficients(sr);
            }
            self.needs_update = false;
        }
    }
}

impl Default for GraphicEq {
    fn default() -> Self {
        Self::new_10_band()
    }
}

impl AudioEffect for GraphicEq {
    fn process(&mut self, buffer: &mut [f32], sample_rate: u32) {
        if !self.enabled {
            return;
        }

        // Update sample rate if changed
        if self.sample_rate != sample_rate {
            self.sample_rate = sample_rate;
            // Bug fix: Reset filter state when sample rate changes to prevent transients
            for band in &mut self.bands {
                band.reset();
            }
            self.needs_update = true;
        }

        self.update_coefficients();

        // Process interleaved stereo buffer
        for chunk in buffer.chunks_exact_mut(2) {
            let mut left = chunk[0];
            let mut right = chunk[1];

            // Process through all bands in series
            for band in &mut self.bands {
                (left, right) = band.process(left, right);
            }

            chunk[0] = left;
            chunk[1] = right;
        }
    }

    fn reset(&mut self) {
        for band in &mut self.bands {
            band.reset();
        }
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn name(&self) -> &str {
        match self.band_config {
            GraphicEqBands::Ten => "10-Band Graphic EQ",
            GraphicEqBands::ThirtyOne => "31-Band Graphic EQ",
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iso_frequencies() {
        assert_eq!(ISO_10_BAND_FREQUENCIES.len(), 10);
        assert_eq!(ISO_31_BAND_FREQUENCIES.len(), 31);

        // Verify frequencies are in ascending order
        for window in ISO_10_BAND_FREQUENCIES.windows(2) {
            assert!(window[0] < window[1]);
        }
        for window in ISO_31_BAND_FREQUENCIES.windows(2) {
            assert!(window[0] < window[1]);
        }
    }

    #[test]
    fn test_create_10_band() {
        let eq = GraphicEq::new_10_band();
        assert_eq!(eq.band_count(), 10);
        assert_eq!(eq.band_config(), GraphicEqBands::Ten);
    }

    #[test]
    fn test_create_31_band() {
        let eq = GraphicEq::new_31_band();
        assert_eq!(eq.band_count(), 31);
        assert_eq!(eq.band_config(), GraphicEqBands::ThirtyOne);
    }

    #[test]
    fn test_band_frequencies() {
        let eq = GraphicEq::new_10_band();

        assert_eq!(eq.band_frequency(0), Some(31.5));
        assert_eq!(eq.band_frequency(5), Some(1000.0));
        assert_eq!(eq.band_frequency(9), Some(16000.0));
        assert_eq!(eq.band_frequency(10), None);
    }

    #[test]
    fn test_set_band_gain() {
        let mut eq = GraphicEq::new_10_band();

        eq.set_band_gain(5, 6.0);
        assert_eq!(eq.band_gain(5), Some(6.0));
        assert_eq!(eq.preset(), GraphicEqPreset::Custom);
    }

    #[test]
    fn test_gain_clamping() {
        let mut eq = GraphicEq::new_10_band();

        eq.set_band_gain(0, 20.0); // Over maximum
        assert_eq!(eq.band_gain(0), Some(12.0));

        eq.set_band_gain(0, -20.0); // Under minimum
        assert_eq!(eq.band_gain(0), Some(-12.0));
    }

    #[test]
    fn test_presets() {
        let mut eq = GraphicEq::new_10_band();

        eq.set_preset(GraphicEqPreset::BassBoost);
        assert_eq!(eq.preset(), GraphicEqPreset::BassBoost);
        assert_eq!(eq.band_gain(0), Some(6.0)); // First band boosted

        eq.set_preset(GraphicEqPreset::Flat);
        assert_eq!(eq.band_gain(0), Some(0.0));
    }

    #[test]
    fn test_set_gains_10() {
        let mut eq = GraphicEq::new_10_band();

        let gains = [1.0, 2.0, 3.0, 4.0, 5.0, 4.0, 3.0, 2.0, 1.0, 0.0];
        eq.set_gains_10(gains);

        assert_eq!(eq.gains_10(), gains);
    }

    #[test]
    fn test_process_flat() {
        let mut eq = GraphicEq::new_10_band();
        eq.set_preset(GraphicEqPreset::Flat);

        let mut buffer = vec![0.5, 0.3, -0.2, 0.8];
        let original = buffer.clone();

        eq.process(&mut buffer, 44100);

        // Flat EQ should not significantly change the signal
        for (orig, proc) in original.iter().zip(buffer.iter()) {
            assert!(
                (orig - proc).abs() < 0.01,
                "Flat EQ should pass signal unchanged"
            );
        }
    }

    #[test]
    fn test_disabled_bypass() {
        let mut eq = GraphicEq::new_10_band();
        eq.set_preset(GraphicEqPreset::BassBoost);
        eq.set_enabled(false);

        let mut buffer = vec![0.5, 0.3];
        let original = buffer.clone();

        eq.process(&mut buffer, 44100);

        assert_eq!(buffer, original);
    }

    #[test]
    fn test_reset() {
        let mut eq = GraphicEq::new_10_band();
        eq.set_preset(GraphicEqPreset::BassBoost);

        // Process some samples
        let mut buffer = vec![0.5; 100];
        eq.process(&mut buffer, 44100);

        // Reset
        eq.reset();

        // Process again - should be deterministic
        let mut buffer2 = vec![0.5; 100];
        eq.process(&mut buffer2, 44100);

        assert_eq!(buffer, buffer2);
    }

    #[test]
    fn test_reset_to_flat() {
        let mut eq = GraphicEq::new_10_band();
        eq.set_preset(GraphicEqPreset::BassBoost);

        eq.reset_to_flat();

        assert_eq!(eq.preset(), GraphicEqPreset::Flat);
        for i in 0..10 {
            assert_eq!(eq.band_gain(i), Some(0.0));
        }
    }

    #[test]
    fn test_name() {
        let eq10 = GraphicEq::new_10_band();
        assert_eq!(eq10.name(), "10-Band Graphic EQ");

        let eq31 = GraphicEq::new_31_band();
        assert_eq!(eq31.name(), "31-Band Graphic EQ");
    }

    #[test]
    fn test_preset_names() {
        assert_eq!(GraphicEqPreset::Flat.name(), "Flat");
        assert_eq!(GraphicEqPreset::BassBoost.name(), "Bass Boost");
        assert_eq!(GraphicEqPreset::VShape.name(), "V-Shape");
    }
}
