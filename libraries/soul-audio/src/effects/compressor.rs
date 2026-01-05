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
pub struct Compressor {
    settings: CompressorSettings,
    enabled: bool,

    // Envelope followers (per channel)
    envelope_l: f32,
    envelope_r: f32,

    // Coefficient cache
    attack_coeff: f32,
    release_coeff: f32,
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
            envelope_l: 0.0,
            envelope_r: 0.0,
            attack_coeff: 0.0,
            release_coeff: 0.0,
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

        // Calculate attack/release coefficients
        // Formula: 1 - e^(-1 / (time_ms * sample_rate / 1000))
        self.attack_coeff = (-1.0 / (self.settings.attack_ms * sr / 1000.0)).exp();
        self.release_coeff = (-1.0 / (self.settings.release_ms * sr / 1000.0)).exp();

        // Convert makeup gain from dB to linear
        self.makeup_gain_linear = 10.0_f32.powf(self.settings.makeup_gain_db / 20.0);

        self.needs_update = false;
    }

    /// Compute gain reduction for a given input level
    #[inline]
    fn compute_gain_reduction(&self, input_db: f32) -> f32 {
        let threshold = self.settings.threshold_db;
        let ratio = self.settings.ratio;
        let knee = self.settings.knee_db;

        if input_db < threshold - knee / 2.0 {
            // Below knee - no compression
            0.0
        } else if input_db > threshold + knee / 2.0 {
            // Above knee - full compression
            (threshold - input_db) + (input_db - threshold) / ratio
        } else {
            // Within knee - soft transition
            let knee_start = threshold - knee / 2.0;
            let knee_factor = (input_db - knee_start) / knee;
            let knee_gain = knee_factor * knee_factor / 2.0;
            -knee_gain * knee * (1.0 - 1.0 / ratio)
        }
    }

    /// Process a single sample for one channel
    #[inline]
    fn process_sample(&mut self, sample: f32, envelope: &mut f32) -> f32 {
        // Convert to dB (with floor to avoid log(0))
        let input_level = sample.abs().max(0.000001);
        let input_db = 20.0 * input_level.log10();

        // Envelope follower with attack/release
        let coeff = if input_db > *envelope {
            self.attack_coeff // Attack
        } else {
            self.release_coeff // Release
        };

        *envelope = coeff * *envelope + (1.0 - coeff) * input_db;

        // Compute gain reduction
        let gain_reduction_db = self.compute_gain_reduction(*envelope);
        let gain = 10.0_f32.powf(gain_reduction_db / 20.0);

        // Apply gain and makeup gain
        sample * gain * self.makeup_gain_linear
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

        // Process interleaved stereo buffer
        // Extract envelopes to avoid borrow checker issues
        let mut env_l = self.envelope_l;
        let mut env_r = self.envelope_r;

        for chunk in buffer.chunks_exact_mut(2) {
            chunk[0] = self.process_sample(chunk[0], &mut env_l);
            chunk[1] = self.process_sample(chunk[1], &mut env_r);
        }

        // Store back
        self.envelope_l = env_l;
        self.envelope_r = env_r;
    }

    fn reset(&mut self) {
        self.envelope_l = 0.0;
        self.envelope_r = 0.0;
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
            ratio: 50.0,           // Out of range
            attack_ms: 0.01,       // Out of range
            release_ms: 5000.0,    // Out of range
            knee_db: 20.0,         // Out of range
            makeup_gain_db: 50.0,  // Out of range
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

        // Envelopes should be zero
        assert_eq!(comp.envelope_l, 0.0);
        assert_eq!(comp.envelope_r, 0.0);
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
}
