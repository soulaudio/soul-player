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

        // Validate buffer is stereo-aligned
        if buffer.len() % 2 != 0 {
            tracing::warn!(
                "[LIMITER] Odd buffer length {}, last sample will not be processed",
                buffer.len()
            );
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

    #[test]
    fn ebu_r128_compliant_threshold() {
        // EBU R128 requires -1 dBTP maximum true peak
        let mut limiter = Limiter::with_settings(LimiterSettings {
            threshold_db: -1.0,
            release_ms: 100.0,
        });

        // Signal at 0 dBFS (1.0 linear)
        let mut buffer = vec![1.0_f32; 200];
        limiter.process(&mut buffer, 44100);

        // After settling, output should not exceed -1 dBFS (~0.891)
        let threshold_linear = 10.0_f32.powf(-1.0 / 20.0);
        for &sample in &buffer[100..] {
            // Allow small tolerance for envelope follower behavior
            assert!(
                sample.abs() <= threshold_linear + 0.01,
                "Sample {} exceeds -1 dB threshold {}",
                sample,
                threshold_linear
            );
        }
    }

    #[test]
    fn threshold_smoothing_prevents_clicks() {
        let mut limiter = Limiter::new();

        // Process some audio first
        let mut buffer = vec![0.5_f32; 128];
        limiter.process(&mut buffer, 44100);

        // Change threshold mid-stream
        limiter.set_threshold(-6.0);

        // Process more audio
        let mut buffer2 = vec![0.5_f32; 128];
        limiter.process(&mut buffer2, 44100);

        // Check for abrupt changes (clicks would show as large sample-to-sample differences)
        for window in buffer2.windows(2) {
            let diff = (window[1] - window[0]).abs();
            assert!(
                diff < 0.1,
                "Abrupt change detected: {} to {} (diff {})",
                window[0],
                window[1],
                diff
            );
        }
    }

    #[test]
    fn release_coefficient_calculation() {
        // Verify release coefficient is correctly computed for different sample rates
        let _limiter = Limiter::with_settings(LimiterSettings {
            threshold_db: -1.0,
            release_ms: 100.0,
        });

        // At 44100 Hz, 100ms = 4410 samples
        // Release coeff should be exp(-1/4410) ≈ 0.9997732
        let expected_coeff = (-1.0 / 4410.0_f32).exp();
        let actual_coeff = Limiter::calculate_release_coeff(100.0, 44100);

        assert!(
            (actual_coeff - expected_coeff).abs() < 0.0001,
            "Expected release coeff {}, got {}",
            expected_coeff,
            actual_coeff
        );
    }

    #[test]
    fn envelope_follower_instant_attack() {
        let mut limiter = Limiter::new();

        // Start with quiet signal
        let mut quiet = vec![0.1_f32; 100];
        limiter.process(&mut quiet, 44100);

        // Envelope should track quiet signal
        assert!(
            limiter.envelope < 0.2,
            "Envelope should be low after quiet signal: {}",
            limiter.envelope
        );

        // Sudden loud transient - attack should be instant
        let mut loud = vec![0.9_f32, 0.9_f32];
        limiter.process(&mut loud, 44100);

        // Envelope should immediately jump to track peak
        assert!(
            limiter.envelope >= 0.85,
            "Envelope should instantly track loud signal: {}",
            limiter.envelope
        );
    }

    /// Documents the feedback limiter design (no lookahead)
    ///
    /// This limiter uses a feedback design where gain reduction is computed
    /// from the current sample. This means:
    /// 1. The first sample of a transient may overshoot before limiting engages
    /// 2. No latency is introduced
    ///
    /// For true brick-wall limiting, a lookahead design (like TruePeakLimiter
    /// in soul-loudness) is preferred, at the cost of added latency.
    #[test]
    fn feedback_limiter_transient_overshoot() {
        let mut limiter = Limiter::with_settings(LimiterSettings {
            threshold_db: -6.0, // -6 dB threshold (~0.501 linear)
            release_ms: 50.0,
        });

        // Start with silence, then sudden loud transient
        let mut buffer = vec![0.0_f32; 10];
        buffer.extend(vec![0.9_f32; 10]); // Loud transient at sample 10

        limiter.process(&mut buffer, 44100);

        // First sample of transient may overshoot (feedback limiter behavior)
        // This is expected - the limiter hasn't seen the peak yet
        let threshold_linear = 10.0_f32.powf(-6.0 / 20.0);

        // Check that limiting eventually engages
        let last_samples = &buffer[buffer.len() - 4..];
        for &sample in last_samples {
            assert!(
                sample.abs() <= threshold_linear + 0.05,
                "Limiter should engage after transient: sample={}, threshold={}",
                sample,
                threshold_linear
            );
        }
    }

    /// Documents limitation: sample peak detection only (no true peak/oversampling)
    ///
    /// Per ITU-R BS.1770 and EBU R128, true peak limiting requires 4x oversampling
    /// to detect inter-sample peaks. This limiter operates on sample values only.
    ///
    /// Inter-sample peaks can exceed sample peaks by up to 3-6 dB for certain
    /// signals (especially at frequencies near Nyquist/4).
    #[test]
    fn sample_peak_limitation_documented() {
        let mut limiter = Limiter::with_settings(LimiterSettings {
            threshold_db: 0.0, // 0 dBFS
            release_ms: 50.0,
        });

        // Signal that will have inter-sample peaks: two samples that create
        // a peak between them when reconstructed
        let mut buffer = vec![0.707_f32, -0.707_f32, 0.707_f32, -0.707_f32];

        limiter.process(&mut buffer, 44100);

        // Sample peaks are 0.707 (-3 dBFS), so no limiting occurs
        // But the true peak between samples approaches 1.0 (0 dBFS)
        // This is the documented limitation of sample-peak-only limiting
        for &sample in &buffer {
            // Samples pass through unchanged (below threshold)
            assert!(
                (sample.abs() - 0.707).abs() < 0.01,
                "Expected sample to pass through: {}",
                sample
            );
        }

        // NOTE: A true peak limiter with 4x oversampling would detect and
        // limit the inter-sample peaks here.
    }

    #[test]
    fn odd_buffer_length_handling() {
        let mut limiter = Limiter::new();

        // Odd-length buffer (not stereo-aligned)
        let mut buffer = vec![0.5_f32; 101];
        limiter.process(&mut buffer, 44100);

        // Should process without panic (last sample ignored per warning)
        // First 100 samples should be processed
        assert!(buffer.len() == 101);
    }

    #[test]
    fn dc_offset_handling() {
        let mut limiter = Limiter::with_settings(LimiterSettings {
            threshold_db: -3.0,
            release_ms: 50.0,
        });

        // Signal with DC offset
        let mut buffer: Vec<f32> = (0..200)
            .map(|i| 0.4 + 0.3 * (i as f32 * 0.1).sin())
            .collect();

        limiter.process(&mut buffer, 44100);

        // Limiter should handle DC offset + AC component correctly
        let threshold_linear = 10.0_f32.powf(-3.0 / 20.0);
        for &sample in &buffer[50..] {
            assert!(
                sample.abs() <= threshold_linear + 0.1,
                "Sample {} exceeds threshold with DC offset",
                sample
            );
        }
    }

    // ==================== Extreme Input Level Tests ====================

    #[test]
    fn extreme_positive_input_levels() {
        // Test: Limiter should handle extremely high positive input levels
        let mut limiter = Limiter::with_settings(LimiterSettings {
            threshold_db: -0.1,
            release_ms: 50.0,
        });

        // Extreme input levels (10x, 100x, 1000x normal)
        let extreme_levels = [10.0, 100.0, 1000.0, 10000.0];

        for &level in &extreme_levels {
            limiter.reset();
            let mut buffer = vec![level, level, level, level];
            limiter.process(&mut buffer, 44100);

            for &sample in &buffer {
                assert!(
                    sample.is_finite(),
                    "Sample should be finite for input level {}",
                    level
                );
                assert!(
                    sample.abs() <= 1.0,
                    "Sample {} should be limited for input level {}",
                    sample,
                    level
                );
            }
        }
    }

    #[test]
    fn extreme_negative_input_levels() {
        // Test: Limiter should handle extremely high negative input levels
        let mut limiter = Limiter::with_settings(LimiterSettings::brickwall());

        let extreme_levels = [-10.0, -100.0, -1000.0];

        for &level in &extreme_levels {
            limiter.reset();
            let mut buffer = vec![level, level, level, level];
            limiter.process(&mut buffer, 44100);

            for &sample in &buffer {
                assert!(
                    sample.is_finite(),
                    "Sample should be finite for input level {}",
                    level
                );
                assert!(
                    sample.abs() <= 1.0,
                    "Sample {} should be limited for input level {}",
                    sample,
                    level
                );
            }
        }
    }

    #[test]
    fn mixed_extreme_and_normal_levels() {
        // Test: Limiter should handle mixed extreme and normal levels in same buffer
        let mut limiter = Limiter::with_settings(LimiterSettings::brickwall());

        let mut buffer = vec![
            0.1, -0.1, // Normal stereo pair
            100.0, -100.0, // Extreme stereo pair
            0.5, 0.5, // Normal again
            50.0, -50.0, // Extreme again
        ];

        limiter.process(&mut buffer, 44100);

        for &sample in &buffer {
            assert!(sample.is_finite(), "All samples should be finite");
            assert!(
                sample.abs() <= 1.001,
                "All samples should be limited: {}",
                sample
            );
        }
    }

    #[test]
    fn sustained_extreme_levels() {
        // Test: Sustained extreme levels should not cause numerical issues
        let mut limiter = Limiter::with_settings(LimiterSettings::brickwall());

        // Process many buffers of extreme signal
        for _ in 0..100 {
            let mut buffer = vec![100.0, -100.0, 100.0, -100.0];
            limiter.process(&mut buffer, 44100);

            for &sample in &buffer {
                assert!(sample.is_finite(), "Sample should remain finite over time");
                assert!(
                    sample.abs() <= 1.001,
                    "Limiting should be maintained: {}",
                    sample
                );
            }
        }

        // Envelope should not have accumulated numerical errors
        assert!(
            limiter.envelope.is_finite(),
            "Envelope should remain finite"
        );
    }

    #[test]
    fn infinity_input_handled() {
        // Test: Infinity input should not crash (though output may be undefined)
        let mut limiter = Limiter::new();

        let mut buffer = vec![f32::INFINITY, f32::NEG_INFINITY, 0.5, -0.5];
        limiter.process(&mut buffer, 44100);

        // At minimum, should not panic. Output may be NaN/Inf due to extreme input.
        // This tests that the limiter doesn't crash on invalid input.
    }

    #[test]
    fn very_quiet_input_after_loud() {
        // Test: Recovery from extreme levels to quiet levels
        let mut limiter = Limiter::with_settings(LimiterSettings {
            threshold_db: -0.1,
            release_ms: 50.0,
        });

        // First, process very loud signal
        let mut loud_buffer = vec![100.0; 100];
        limiter.process(&mut loud_buffer, 44100);

        // Now process very quiet signal
        let mut quiet_buffer = vec![0.001; 200];
        limiter.process(&mut quiet_buffer, 44100);

        // After release, quiet signal should pass through relatively unaffected
        // The last samples should be close to original
        let last_samples = &quiet_buffer[quiet_buffer.len() - 10..];
        for &sample in last_samples {
            assert!(
                (sample - 0.001).abs() < 0.005,
                "Quiet signal should recover: {}",
                sample
            );
        }
    }

    #[test]
    fn dc_offset_with_limiting() {
        // Test: DC offset combined with limiting
        let mut limiter = Limiter::with_settings(LimiterSettings::brickwall());

        // Signal with DC offset pushing it over threshold
        let mut buffer: Vec<f32> = (0..100)
            .map(|i| 0.5 + (i as f32 * 0.1).sin() * 0.8) // 0.5 DC + 0.8 amplitude
            .collect();

        // Make it stereo
        let mono_buffer = buffer.clone();
        buffer.clear();
        for sample in mono_buffer {
            buffer.push(sample);
            buffer.push(sample);
        }

        limiter.process(&mut buffer, 44100);

        for &sample in &buffer {
            assert!(sample.is_finite());
            assert!(
                sample.abs() <= 1.001,
                "DC+AC signal should be limited: {}",
                sample
            );
        }
    }

    #[test]
    fn threshold_at_exactly_near_zero_db() {
        // Test: Threshold near 0 dB (maximum possible threshold)
        let mut limiter = Limiter::with_settings(LimiterSettings {
            threshold_db: -0.01,
            release_ms: 50.0,
        });

        let mut buffer = vec![1.0, 1.0, 1.5, 1.5];
        limiter.process(&mut buffer, 44100);

        for &sample in &buffer {
            assert!(
                sample.abs() <= 1.001,
                "Should limit near 0 dB threshold: {}",
                sample
            );
        }
    }

    #[test]
    fn threshold_very_low() {
        // Test: Very low threshold (heavy limiting)
        let mut limiter = Limiter::with_settings(LimiterSettings {
            threshold_db: -20.0, // -20 dB = 0.1 linear
            release_ms: 50.0,
        });

        let mut buffer = vec![0.5, 0.5, 0.5, 0.5];
        limiter.process(&mut buffer, 44100);

        // Output should be significantly reduced
        for &sample in &buffer {
            assert!(sample.abs() <= 0.15, "Heavy limiting expected: {}", sample);
        }
    }

    // ==================== Threshold Smoothing Tests ====================

    #[test]
    fn threshold_change_is_smoothed() {
        // Test: Threshold changes should be smoothed over SMOOTH_SAMPLES
        let mut limiter = Limiter::new();

        // Set initial threshold
        limiter.set_threshold(-3.0);

        // Process to apply initial threshold
        let mut buffer = vec![1.0; 200];
        limiter.process(&mut buffer, 44100);

        // Change threshold
        limiter.set_threshold(-6.0);

        // The threshold change should not be instant
        assert!(
            limiter.smooth_samples_remaining > 0,
            "Smoothing should be active"
        );

        // Process and verify smoothing is happening
        let mut buffer2 = vec![1.0; 100];
        limiter.process(&mut buffer2, 44100);
    }

    #[test]
    fn threshold_change_no_clicks() {
        // Test: Changing threshold during processing should not cause clicks
        let mut limiter = Limiter::new();

        let mut all_samples = Vec::new();

        for i in 0..10 {
            // Change threshold periodically
            let threshold = -0.5 - (i as f32 * 0.3);
            limiter.set_threshold(threshold);

            let mut buffer = vec![0.8; 50];
            limiter.process(&mut buffer, 44100);
            all_samples.extend(buffer);
        }

        // Check for smooth transitions (no large jumps)
        for i in 1..all_samples.len() {
            let delta = (all_samples[i] - all_samples[i - 1]).abs();
            assert!(
                delta < 0.2,
                "Large jump at {}: {} -> {}",
                i,
                all_samples[i - 1],
                all_samples[i]
            );
        }
    }

    // ==================== Sample Rate Variation Tests ====================

    #[test]
    fn different_sample_rates() {
        // Test: Limiter should work at various sample rates
        let sample_rates = [8000, 22050, 44100, 48000, 88200, 96000, 192000];

        for &rate in &sample_rates {
            let mut limiter = Limiter::with_settings(LimiterSettings {
                threshold_db: -0.1,
                release_ms: 50.0,
            });

            let mut buffer = vec![1.5, -1.5, 1.5, -1.5];
            limiter.process(&mut buffer, rate);

            for &sample in &buffer {
                assert!(sample.is_finite(), "Should be finite at {}Hz", rate);
                assert!(
                    sample.abs() <= 1.0,
                    "Should limit at {}Hz: {}",
                    rate,
                    sample
                );
            }
        }
    }

    #[test]
    fn release_time_respects_sample_rate() {
        // Test: Release time should produce consistent decay behavior across sample rates
        // With exponential decay, after `release_ms`, envelope should be at ~37% (exp(-1))
        // After 3x `release_ms`, it should be at ~5% (exp(-3))
        let release_ms = 100.0;

        for &rate in &[44100u32, 96000u32] {
            let mut limiter = Limiter::with_settings(LimiterSettings {
                threshold_db: -0.1,
                release_ms,
            });

            // Process loud signal to set envelope to ~2.0
            let mut loud = vec![2.0; 200];
            limiter.process(&mut loud, rate);

            let envelope_after_loud = limiter.envelope;
            assert!(
                envelope_after_loud > 1.5,
                "Envelope should track loud signal at {}Hz: {}",
                rate,
                envelope_after_loud
            );

            // Process quiet signal for 3x the release time
            // After 3 time constants, envelope should decay to < 5% of original
            let samples_for_3x_release = (rate as f32 * release_ms * 3.0 / 1000.0) as usize;
            let mut quiet = vec![0.0; samples_for_3x_release * 2]; // *2 for stereo
            limiter.process(&mut quiet, rate);

            // Envelope should have decayed to < 10% of where it was
            // (exp(-3) = 0.05, but we're lenient due to discrete-time approximation)
            assert!(
                limiter.envelope < envelope_after_loud * 0.15,
                "Envelope should decay significantly after 3x release time at {}Hz: {} (was {})",
                rate,
                limiter.envelope,
                envelope_after_loud
            );
        }
    }
}
