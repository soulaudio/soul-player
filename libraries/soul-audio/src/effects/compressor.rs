/// Dynamic Range Compressor
///
/// Reduces the dynamic range of audio by attenuating signals above a threshold.
/// Useful for making quiet parts louder and loud parts quieter.
use super::chain::AudioEffect;

/// Compressor settings
#[derive(Debug, Clone, Copy)]
pub struct CompressorSettings {
    /// Threshold in dB (-60 to 0)
    /// Signals above this level will be compressed
    pub threshold_db: f32,

    /// Ratio (1.0 to 20.0)
    /// Amount of compression (e.g., 4.0 means 4:1 compression)
    pub ratio: f32,

    /// Attack time in milliseconds (0.1 to 100)
    /// How quickly compression is applied when signal exceeds threshold
    pub attack_ms: f32,

    /// Release time in milliseconds (10 to 1000)
    /// How quickly compression is released when signal falls below threshold
    pub release_ms: f32,

    /// Knee width in dB (0 to 10)
    /// Softens the transition at the threshold (0 = hard knee, >0 = soft knee)
    pub knee_db: f32,

    /// Makeup gain in dB (0 to 24)
    /// Applied after compression to restore overall level
    pub makeup_gain_db: f32,
}

impl CompressorSettings {
    /// Create default compressor settings
    /// - Threshold: -20 dB
    /// - Ratio: 4:1
    /// - Attack: 5 ms
    /// - Release: 50 ms
    /// - Soft knee: 6 dB
    /// - Makeup gain: 0 dB
    pub fn new() -> Self {
        Self {
            threshold_db: -20.0,
            ratio: 4.0,
            attack_ms: 5.0,
            release_ms: 50.0,
            knee_db: 6.0,
            makeup_gain_db: 0.0,
        }
    }

    /// Create settings for gentle compression (vocals, acoustic)
    pub fn gentle() -> Self {
        Self {
            threshold_db: -15.0,
            ratio: 2.5,
            attack_ms: 10.0,
            release_ms: 100.0,
            knee_db: 8.0,
            makeup_gain_db: 3.0,
        }
    }

    /// Create settings for moderate compression (mix bus)
    pub fn moderate() -> Self {
        Self {
            threshold_db: -18.0,
            ratio: 4.0,
            attack_ms: 5.0,
            release_ms: 50.0,
            knee_db: 6.0,
            makeup_gain_db: 4.0,
        }
    }

    /// Create settings for aggressive compression (limiting)
    pub fn aggressive() -> Self {
        Self {
            threshold_db: -12.0,
            ratio: 10.0,
            attack_ms: 1.0,
            release_ms: 30.0,
            knee_db: 2.0,
            makeup_gain_db: 6.0,
        }
    }

    /// Validate and clamp settings to safe ranges
    pub fn validate(&mut self) {
        self.threshold_db = self.threshold_db.clamp(-60.0, 0.0);
        self.ratio = self.ratio.clamp(1.0, 20.0);
        self.attack_ms = self.attack_ms.clamp(0.1, 100.0);
        self.release_ms = self.release_ms.clamp(10.0, 1000.0);
        self.knee_db = self.knee_db.clamp(0.0, 10.0);
        self.makeup_gain_db = self.makeup_gain_db.clamp(0.0, 24.0);
    }
}

impl Default for CompressorSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// Dynamic Range Compressor
///
/// Uses a two-stage design for proper timing and low THD:
/// 1. Peak level detection with instant attack and slow release (peak hold)
///    This accurately tracks the signal level without per-cycle variation
/// 2. Gain smoothing with configurable attack/release for proper dynamics
///    This controls how fast compression responds
///
/// The key insight is that:
/// - Peak detection needs SLOW release to hold peaks across waveform cycles (reduces THD)
/// - Gain smoothing uses the user-configured attack/release times
/// - These are independent: peak detector affects level accuracy, gain smoother affects timing
pub struct Compressor {
    settings: CompressorSettings,
    enabled: bool,

    // Peak level detector (in dB)
    // Uses instant attack, slow release to hold peaks across cycles
    peak_level_db: f32,

    // Smoothed gain reduction in dB
    // This is what gets smoothed with user-configured attack/release
    gain_reduction_db: f32,

    // Coefficient cache
    peak_release_coeff: f32,  // Slow release for peak hold (~100ms)
    gr_attack_coeff: f32,     // User-configured attack
    gr_release_coeff: f32,    // User-configured release
    makeup_gain_linear: f32,

    sample_rate: u32,
    needs_update: bool,
}

impl Compressor {
    /// Create a new compressor with default settings
    pub fn new() -> Self {
        Self::with_settings(CompressorSettings::new())
    }

    /// Create compressor with specific settings
    pub fn with_settings(settings: CompressorSettings) -> Self {
        let mut comp = Self {
            settings,
            enabled: true,
            peak_level_db: -120.0,   // Start with very low level
            gain_reduction_db: 0.0,  // Start with no gain reduction
            peak_release_coeff: 0.0,
            gr_attack_coeff: 0.0,
            gr_release_coeff: 0.0,
            makeup_gain_linear: 1.0,
            sample_rate: 44100,
            needs_update: true,
        };
        comp.update_coefficients();
        comp
    }

    /// Update compressor settings
    pub fn set_settings(&mut self, mut settings: CompressorSettings) {
        settings.validate();
        self.settings = settings;
        self.needs_update = true;
    }

    /// Get current settings
    pub fn settings(&self) -> CompressorSettings {
        self.settings
    }

    /// Set threshold (in dB)
    pub fn set_threshold(&mut self, threshold_db: f32) {
        self.settings.threshold_db = threshold_db.clamp(-60.0, 0.0);
        self.needs_update = true;
    }

    /// Set ratio
    pub fn set_ratio(&mut self, ratio: f32) {
        self.settings.ratio = ratio.clamp(1.0, 20.0);
    }

    /// Set attack time (in ms)
    pub fn set_attack(&mut self, attack_ms: f32) {
        self.settings.attack_ms = attack_ms.clamp(0.1, 100.0);
        self.needs_update = true;
    }

    /// Set release time (in ms)
    pub fn set_release(&mut self, release_ms: f32) {
        self.settings.release_ms = release_ms.clamp(10.0, 1000.0);
        self.needs_update = true;
    }

    /// Set makeup gain (in dB)
    pub fn set_makeup_gain(&mut self, gain_db: f32) {
        self.settings.makeup_gain_db = gain_db.clamp(0.0, 24.0);
        self.needs_update = true;
    }

    /// Update internal coefficients based on settings and sample rate
    fn update_coefficients(&mut self) {
        if !self.needs_update {
            return;
        }

        let sr = self.sample_rate as f32;

        // Peak level detector coefficient
        // Use a relatively long hold time to capture true peaks
        // This ensures the peak level is stable within each waveform cycle
        // A 50ms release means peaks are held for about 50 cycles at 1kHz
        // This is enough to accurately measure steady-state level
        // while still responding to level changes within ~100ms
        let peak_release_ms = 50.0;
        let peak_release_samples = peak_release_ms * sr / 1000.0;
        self.peak_release_coeff = (-1.0 / peak_release_samples).exp();

        // Gain reduction smoothing coefficients
        // These are the user-configurable attack/release times
        // Using the standard formula: coeff = exp(-1 / (time_ms * sample_rate / 1000))
        // This gives 63.2% (1 - 1/e) response at the specified time
        let attack_samples = self.settings.attack_ms * sr / 1000.0;
        let release_samples = self.settings.release_ms * sr / 1000.0;

        self.gr_attack_coeff = (-1.0 / attack_samples).exp();
        self.gr_release_coeff = (-1.0 / release_samples).exp();

        // Convert makeup gain from dB to linear
        self.makeup_gain_linear = 10.0_f32.powf(self.settings.makeup_gain_db / 20.0);

        self.needs_update = false;
    }

    /// Compute the desired output level for a given input level (in dB)
    /// Returns the output level in dB
    #[inline]
    fn compute_output_level(&self, input_db: f32) -> f32 {
        let threshold = self.settings.threshold_db;
        let ratio = self.settings.ratio;
        let knee = self.settings.knee_db;

        if knee <= 0.0 {
            // Hard knee
            if input_db <= threshold {
                input_db // No compression below threshold
            } else {
                // Above threshold: output = threshold + (input - threshold) / ratio
                threshold + (input_db - threshold) / ratio
            }
        } else {
            // Soft knee
            let half_knee = knee / 2.0;
            let knee_start = threshold - half_knee;
            let knee_end = threshold + half_knee;

            if input_db <= knee_start {
                // Below knee region - no compression
                input_db
            } else if input_db >= knee_end {
                // Above knee region - full compression
                threshold + (input_db - threshold) / ratio
            } else {
                // Within knee region - smooth quadratic transition
                // This implements the standard soft-knee formula
                let x = input_db - knee_start;
                let slope_change = (1.0 - 1.0 / ratio) / (2.0 * knee);
                input_db - slope_change * x * x
            }
        }
    }

    /// Compute gain reduction for a given input level (in dB)
    /// Returns gain reduction in dB (negative value means gain reduction)
    #[inline]
    fn compute_gain_reduction(&self, input_db: f32) -> f32 {
        self.compute_output_level(input_db) - input_db
    }

    /// Update peak level detector
    /// Uses instant attack (new peaks are captured immediately)
    /// and slow release (peaks decay toward noise floor, not input)
    #[inline]
    fn update_peak_level(&mut self, input_db: f32) {
        if input_db > self.peak_level_db {
            // Instant attack - capture new peaks immediately
            self.peak_level_db = input_db;
        } else {
            // Slow release - decay toward noise floor at fixed rate
            // We don't decay toward input because input can be -inf at zero crossings
            // This holds peaks properly across waveform cycles
            // Decay by (1 - coeff) * range_to_floor per sample
            // Effectively: peak = peak * coeff (multiplicative decay in linear, fixed dB/s in log)
            const NOISE_FLOOR_DB: f32 = -120.0;
            self.peak_level_db = self.peak_release_coeff * (self.peak_level_db - NOISE_FLOOR_DB)
                + NOISE_FLOOR_DB;
        }
    }

    /// Smooth gain reduction with attack/release
    /// Attack = time to increase gain reduction (compress)
    /// Release = time to decrease gain reduction (release)
    #[inline]
    fn smooth_gain_reduction(&mut self, target_gr_db: f32) {
        // Attack means more negative gain reduction (more compression)
        // Release means less negative gain reduction (less compression)
        let coeff = if target_gr_db < self.gain_reduction_db {
            self.gr_attack_coeff // Target is more negative = attacking
        } else {
            self.gr_release_coeff // Target is less negative = releasing
        };

        self.gain_reduction_db = coeff * self.gain_reduction_db
            + (1.0 - coeff) * target_gr_db;
    }
}

impl Default for Compressor {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioEffect for Compressor {
    fn process(&mut self, buffer: &mut [f32], sample_rate: u32) {
        // Bypass if disabled
        if !self.enabled {
            return;
        }

        // Update sample rate if changed
        if self.sample_rate != sample_rate {
            self.sample_rate = sample_rate;
            self.needs_update = true;
        }

        // Update coefficients if needed
        self.update_coefficients();

        // Process interleaved stereo buffer with linked stereo detection
        for chunk in buffer.chunks_exact_mut(2) {
            // For linked stereo, use the louder channel (peak of both)
            let max_sample = chunk[0].abs().max(chunk[1].abs());

            // Convert instantaneous level to dB
            let input_db = if max_sample > 1e-10 {
                20.0 * max_sample.log10()
            } else {
                -200.0 // Very quiet
            };

            // Stage 1: Update peak level detector
            // This holds peaks across waveform cycles for accurate level measurement
            self.update_peak_level(input_db);

            // Compute target gain reduction based on peak level
            let target_gr_db = self.compute_gain_reduction(self.peak_level_db);

            // Stage 2: Smooth the gain reduction with attack/release
            // This is where the user-configurable timing happens
            self.smooth_gain_reduction(target_gr_db);

            // Convert smoothed gain reduction to linear
            let gain = 10.0_f32.powf(self.gain_reduction_db / 20.0);

            // Apply same gain to both channels (linked stereo)
            // This preserves stereo image
            chunk[0] = chunk[0] * gain * self.makeup_gain_linear;
            chunk[1] = chunk[1] * gain * self.makeup_gain_linear;
        }
    }

    fn reset(&mut self) {
        self.peak_level_db = -120.0;
        self.gain_reduction_db = 0.0;
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn name(&self) -> &str {
        "Dynamic Range Compressor"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_compressor() {
        let comp = Compressor::new();
        assert!(comp.is_enabled());
        assert_eq!(comp.name(), "Dynamic Range Compressor");
    }

    #[test]
    fn settings_validation() {
        let mut settings = CompressorSettings {
            threshold_db: -100.0, // Out of range
            ratio: 50.0,          // Out of range
            attack_ms: 0.01,      // Out of range
            release_ms: 5000.0,   // Out of range
            knee_db: 20.0,        // Out of range
            makeup_gain_db: 50.0, // Out of range
        };

        settings.validate();

        assert!(settings.threshold_db >= -60.0 && settings.threshold_db <= 0.0);
        assert!(settings.ratio >= 1.0 && settings.ratio <= 20.0);
        assert!(settings.attack_ms >= 0.1 && settings.attack_ms <= 100.0);
        assert!(settings.release_ms >= 10.0 && settings.release_ms <= 1000.0);
        assert!(settings.knee_db >= 0.0 && settings.knee_db <= 10.0);
        assert!(settings.makeup_gain_db >= 0.0 && settings.makeup_gain_db <= 24.0);
    }

    #[test]
    fn preset_settings() {
        let gentle = CompressorSettings::gentle();
        assert_eq!(gentle.ratio, 2.5);

        let moderate = CompressorSettings::moderate();
        assert_eq!(moderate.ratio, 4.0);

        let aggressive = CompressorSettings::aggressive();
        assert_eq!(aggressive.ratio, 10.0);
    }

    #[test]
    fn process_reduces_peaks() {
        let mut comp = Compressor::with_settings(CompressorSettings::aggressive());

        // Generate loud signal
        let mut buffer = vec![0.8; 1000]; // Loud signal

        comp.process(&mut buffer, 44100);

        // Signal should be compressed (reduced)
        // First samples might not be fully compressed due to attack time
        let avg = buffer.iter().skip(100).sum::<f32>() / 900.0;
        assert!(avg < 0.8, "Signal should be compressed");
    }

    #[test]
    fn reset_clears_envelope() {
        let mut comp = Compressor::new();

        // Process some loud samples to build up envelope
        let mut buffer = vec![0.9; 100];
        comp.process(&mut buffer, 44100);

        // Reset
        comp.reset();

        // Peak level detector should be reset to very low value (-120 dB)
        assert_eq!(comp.peak_level_db, -120.0);
        // Gain reduction should be reset to 0 dB (no reduction)
        assert_eq!(comp.gain_reduction_db, 0.0);
    }

    #[test]
    fn disabled_compressor_bypassed() {
        let mut comp = Compressor::with_settings(CompressorSettings::aggressive());
        comp.set_enabled(false);

        let mut buffer = vec![0.8; 100];
        let original = buffer.clone();

        comp.process(&mut buffer, 44100);

        // Should be unchanged (compressor disabled)
        assert_eq!(buffer, original);
    }

    #[test]
    fn setters_update_settings() {
        let mut comp = Compressor::new();

        comp.set_threshold(-30.0);
        assert_eq!(comp.settings().threshold_db, -30.0);

        comp.set_ratio(8.0);
        assert_eq!(comp.settings().ratio, 8.0);

        comp.set_attack(2.0);
        assert_eq!(comp.settings().attack_ms, 2.0);

        comp.set_release(100.0);
        assert_eq!(comp.settings().release_ms, 100.0);

        comp.set_makeup_gain(6.0);
        assert_eq!(comp.settings().makeup_gain_db, 6.0);
    }

    #[test]
    fn makeup_gain_boosts_signal() {
        let mut comp = Compressor::new();
        comp.set_threshold(-20.0); // Normal threshold
        comp.set_ratio(2.0); // Gentle ratio
        comp.set_makeup_gain(12.0); // +12 dB makeup gain

        // Use a signal that will be compressed but with enough makeup gain
        // the result should still be louder
        let mut buffer = vec![0.2; 1000]; // Moderate signal
        let original_avg = buffer.iter().sum::<f32>() / buffer.len() as f32;

        comp.process(&mut buffer, 44100);

        let processed_avg = buffer.iter().skip(100).sum::<f32>() / 900.0; // Skip attack time

        // With makeup gain, signal should be louder even after compression
        assert!(
            processed_avg > original_avg * 1.1,
            "Expected processed ({}) > original ({})",
            processed_avg,
            original_avg
        );
    }

    #[test]
    fn gain_reduction_calculation() {
        let settings = CompressorSettings {
            threshold_db: -20.0,
            ratio: 4.0,
            attack_ms: 5.0,
            release_ms: 50.0,
            knee_db: 0.0, // Hard knee
            makeup_gain_db: 0.0,
        };
        let comp = Compressor::with_settings(settings);

        // Test below threshold - no compression
        assert_eq!(comp.compute_gain_reduction(-30.0), 0.0);
        assert_eq!(comp.compute_gain_reduction(-25.0), 0.0);
        assert_eq!(comp.compute_gain_reduction(-20.0), 0.0);

        // Test above threshold - should compress
        // At -16dB (4dB above threshold), 4:1 ratio means 3dB reduction
        let gr = comp.compute_gain_reduction(-16.0);
        assert!(
            (gr - (-3.0)).abs() < 0.01,
            "Expected -3.0dB gain reduction, got {}",
            gr
        );

        // At -10dB (10dB above threshold), 4:1 ratio means 7.5dB reduction
        let gr = comp.compute_gain_reduction(-10.0);
        assert!(
            (gr - (-7.5)).abs() < 0.01,
            "Expected -7.5dB gain reduction, got {}",
            gr
        );
    }

    #[test]
    fn output_level_calculation() {
        let settings = CompressorSettings {
            threshold_db: -20.0,
            ratio: 4.0,
            attack_ms: 5.0,
            release_ms: 50.0,
            knee_db: 0.0,
            makeup_gain_db: 0.0,
        };
        let comp = Compressor::with_settings(settings);

        // Below threshold: output = input
        assert_eq!(comp.compute_output_level(-30.0), -30.0);
        assert_eq!(comp.compute_output_level(-20.0), -20.0);

        // Above threshold: output = threshold + (input - threshold) / ratio
        // At -16dB: output = -20 + (-16 - -20) / 4 = -20 + 1 = -19
        let output = comp.compute_output_level(-16.0);
        assert!(
            (output - (-19.0)).abs() < 0.01,
            "Expected -19.0dB output, got {}",
            output
        );

        // At -10dB: output = -20 + (-10 - -20) / 4 = -20 + 2.5 = -17.5
        let output = comp.compute_output_level(-10.0);
        assert!(
            (output - (-17.5)).abs() < 0.01,
            "Expected -17.5dB output, got {}",
            output
        );
    }
}
