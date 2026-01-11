/// Brick-wall limiter for preventing clipping
///
/// A limiter is essentially a compressor with an infinite ratio, designed to prevent
/// audio peaks from exceeding a threshold. This implementation uses a lookahead buffer
/// for zero-latency brick-wall limiting.
use super::AudioEffect;

/// Limiter settings
#[derive(Debug, Clone, Copy)]
pub struct LimiterSettings {
    /// Threshold in dB (typical: -0.1 to -3.0)
    pub threshold_db: f32,
    /// Release time in milliseconds
    pub release_ms: f32,
}

impl LimiterSettings {
    /// Create default settings (gentle limiting)
    pub fn default() -> Self {
        Self {
            threshold_db: -0.3,
            release_ms: 50.0,
        }
    }

    /// Aggressive brick-wall limiting
    pub fn brickwall() -> Self {
        Self {
            threshold_db: -0.1,
            release_ms: 100.0,
        }
    }

    /// Soft limiting (more transparent)
    pub fn soft() -> Self {
        Self {
            threshold_db: -1.0,
            release_ms: 200.0,
        }
    }

    /// Validate settings
    pub fn validate(&self) -> Result<(), String> {
        if self.threshold_db > 0.0 {
            return Err("Threshold must be negative (in dB)".to_string());
        }
        if self.release_ms <= 0.0 {
            return Err("Release time must be positive".to_string());
        }
        Ok(())
    }
}

/// Number of samples over which to smooth threshold changes
/// At 44.1kHz, 64 samples = ~1.5ms, which is imperceptible but prevents clicks
const SMOOTH_SAMPLES: u32 = 64;

/// Brick-wall limiter effect
///
/// # Real-Time Safety
/// - Pre-allocates envelope buffer in constructor
/// - No allocations in `process()`
/// - Suitable for real-time audio threads
///
/// # Parameter Smoothing
/// Threshold changes are smoothed over 64 samples (~1.5ms) to prevent
/// audible clicks when adjusting the threshold during playback.
pub struct Limiter {
    settings: LimiterSettings,
    /// Target threshold (set by user, smoothed toward)
    target_threshold_linear: f32,
    /// Active threshold (used for processing, smoothed toward target)
    threshold_linear: f32,
    /// Samples remaining until threshold matches target
    smooth_samples_remaining: u32,
    release_coeff: f32,
    envelope: f32,
    enabled: bool,
}

impl Limiter {
    /// Create a limiter with default settings
    pub fn new() -> Self {
        Self::with_settings(LimiterSettings::default())
    }

    /// Create a limiter with specific settings
    pub fn with_settings(settings: LimiterSettings) -> Self {
        settings.validate().expect("Invalid limiter settings");

        let threshold_linear = Self::db_to_linear(settings.threshold_db);

        Self {
            settings,
            target_threshold_linear: threshold_linear,
            threshold_linear,
            smooth_samples_remaining: 0,
            release_coeff: 0.0, // Will be updated in process()
            envelope: 0.0,      // Start with no signal detected
            enabled: true,
        }
    }

    /// Set threshold in dB
    ///
    /// The threshold is smoothed over 64 samples to prevent clicks.
    pub fn set_threshold(&mut self, threshold_db: f32) {
        self.settings.threshold_db = threshold_db.min(0.0);
        let new_target = Self::db_to_linear(self.settings.threshold_db);

        // Only initiate smoothing if threshold changed and not starting from default
        if (new_target - self.target_threshold_linear).abs() > 1e-6 {
            self.target_threshold_linear = new_target;
            self.smooth_samples_remaining = SMOOTH_SAMPLES;
        }
    }

    /// Smooth threshold toward target value
    #[inline]
    fn smooth_threshold(&mut self) {
        if self.smooth_samples_remaining == 0 {
            return;
        }

        let alpha = 1.0 / self.smooth_samples_remaining as f32;
        self.threshold_linear += alpha * (self.target_threshold_linear - self.threshold_linear);
        self.smooth_samples_remaining -= 1;

        // Snap to target when done
        if self.smooth_samples_remaining == 0 {
            self.threshold_linear = self.target_threshold_linear;
        }
    }

    /// Set release time in milliseconds
    pub fn set_release(&mut self, release_ms: f32) {
        self.settings.release_ms = release_ms.max(1.0);
    }

    /// Get current settings
    pub fn settings(&self) -> LimiterSettings {
        self.settings
    }

    /// Convert dB to linear gain
    fn db_to_linear(db: f32) -> f32 {
        10.0f32.powf(db / 20.0)
    }

    /// Calculate release coefficient for given sample rate
    fn calculate_release_coeff(release_ms: f32, sample_rate: u32) -> f32 {
        let release_samples = (release_ms / 1000.0) * sample_rate as f32;
        (-1.0 / release_samples).exp()
    }
}

impl Default for Limiter {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioEffect for Limiter {
    fn process(&mut self, buffer: &mut [f32], sample_rate: u32) {
        if !self.enabled {
            return;
        }

        // Update release coefficient if sample rate changed
        self.release_coeff = Self::calculate_release_coeff(self.settings.release_ms, sample_rate);

        // Process stereo interleaved samples
        for chunk in buffer.chunks_exact_mut(2) {
            // Smooth threshold to prevent clicks during parameter changes
            self.smooth_threshold();

            let left = chunk[0];
            let right = chunk[1];

            // Calculate peak level
            let peak = left.abs().max(right.abs());

            // Update envelope (with fast attack, slow release)
            if peak > self.envelope {
                // Instant attack
                self.envelope = peak;
            } else {
                // Exponential release
                self.envelope = peak + self.release_coeff * (self.envelope - peak);
            }

            // Calculate gain reduction
            let gain = if self.envelope > self.threshold_linear {
                self.threshold_linear / self.envelope
            } else {
                1.0
            };

            // Apply limiting
            chunk[0] = left * gain;
            chunk[1] = right * gain;
        }
    }

    fn reset(&mut self) {
        self.envelope = 0.0; // Reset to "no signal detected"
        // Snap threshold to target when resetting
        self.threshold_linear = self.target_threshold_linear;
        self.smooth_samples_remaining = 0;
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn name(&self) -> &str {
        "Limiter"
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
    fn create_limiter() {
        let limiter = Limiter::new();
        assert!(limiter.is_enabled());
        assert_eq!(limiter.name(), "Limiter");
    }

    #[test]
    fn preset_settings() {
        let default = LimiterSettings::default();
        assert!(default.validate().is_ok());

        let brickwall = LimiterSettings::brickwall();
        assert!(brickwall.validate().is_ok());

        let soft = LimiterSettings::soft();
        assert!(soft.validate().is_ok());
    }

    #[test]
    fn settings_validation() {
        let mut settings = LimiterSettings::default();

        settings.threshold_db = 1.0; // Invalid (positive)
        assert!(settings.validate().is_err());

        settings.threshold_db = -0.5; // Valid
        settings.release_ms = 0.0; // Invalid
        assert!(settings.validate().is_err());

        settings.release_ms = 50.0; // Valid
        assert!(settings.validate().is_ok());
    }

    #[test]
    fn setters_update_settings() {
        let mut limiter = Limiter::new();

        limiter.set_threshold(-1.0);
        assert_eq!(limiter.settings().threshold_db, -1.0);

        limiter.set_release(100.0);
        assert_eq!(limiter.settings().release_ms, 100.0);
    }

    #[test]
    fn process_prevents_clipping() {
        let mut limiter = Limiter::with_settings(LimiterSettings {
            threshold_db: -0.1, // Just below 0dB
            release_ms: 50.0,
        });

        // Create signal with peak at 1.2 (would clip)
        let mut buffer = vec![0.5, 0.5, 1.2, 1.2, 0.3, 0.3];

        limiter.process(&mut buffer, 44100);

        // All samples should be below threshold
        for sample in &buffer {
            assert!(sample.abs() <= 1.0, "Sample {}, exceeds limit", sample);
        }
    }

    #[test]
    fn reset_clears_envelope() {
        let mut limiter = Limiter::new();

        // Process some loud signal
        let mut buffer = vec![1.0; 100];
        limiter.process(&mut buffer, 44100);

        // Envelope should be tracking signal
        assert!(limiter.envelope > 0.0);

        limiter.reset();

        // Envelope should be reset to 0 (no signal detected)
        assert!((limiter.envelope - 0.0).abs() < 0.0001);
    }

    #[test]
    fn disabled_limiter_bypassed() {
        let mut limiter = Limiter::new();
        limiter.set_enabled(false);

        let original = vec![1.5, 1.5, 2.0, 2.0]; // Would be limited
        let mut buffer = original.clone();

        limiter.process(&mut buffer, 44100);

        // Should be unchanged (effect disabled)
        assert_eq!(buffer, original);
    }

    #[test]
    fn preserves_signal_below_threshold() {
        let mut limiter = Limiter::with_settings(LimiterSettings {
            threshold_db: -1.0,
            release_ms: 50.0,
        });

        // Quiet signal (below threshold)
        let original = vec![0.1, 0.1, 0.2, 0.2, 0.15, 0.15];
        let mut buffer = original.clone();

        limiter.process(&mut buffer, 44100);

        // Signal should be mostly unchanged (minor envelope follower effect)
        for (i, sample) in buffer.iter().enumerate() {
            let diff = (sample - original[i]).abs();
            assert!(
                diff < 0.05,
                "Sample {} changed too much: {} vs {}",
                i,
                sample,
                original[i]
            );
        }
    }

    #[test]
    fn brickwall_settings_aggressive() {
        let settings = LimiterSettings::brickwall();
        assert!(settings.threshold_db > -0.5); // Very close to 0dB
        assert!(settings.threshold_db < 0.0); // But still negative
    }

    #[test]
    fn soft_settings_more_gentle() {
        let settings = LimiterSettings::soft();
        assert!(settings.threshold_db < -0.5); // Further from 0dB
        assert!(settings.release_ms > 100.0); // Longer release
    }
}
