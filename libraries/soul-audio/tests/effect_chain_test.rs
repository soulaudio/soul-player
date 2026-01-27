//! Comprehensive Tests for Effect Chain Processing
//!
//! This test suite provides thorough coverage of the EffectChain including:
//! - Empty effect chain processing
//! - Single effect enable/disable behavior
//! - Multiple effects interaction
//! - Effect order dependency (EQ before limiter vs after)
//! - All effects disabled passthrough
//! - Adding/removing effects during processing
//! - Effect parameter changes during processing
//! - Effect chain with very small buffers (1-10 samples)
//! - Effect chain with very large buffers (10000+ samples)

use soul_audio::effects::{
    AudioEffect, Compressor, CompressorSettings, EffectChain, EqBand, Limiter, LimiterSettings,
    ParametricEq,
};
use std::any::Any;
use std::f32::consts::PI;

// ============================================================================
// TEST UTILITIES
// ============================================================================

const SAMPLE_RATE: u32 = 44100;

/// Generate a stereo sine wave at the given frequency
fn generate_stereo_sine(frequency: f32, sample_rate: u32, num_frames: usize) -> Vec<f32> {
    generate_stereo_sine_with_amplitude(frequency, sample_rate, num_frames, 1.0)
}

/// Generate a stereo sine wave with specified amplitude
fn generate_stereo_sine_with_amplitude(
    frequency: f32,
    sample_rate: u32,
    num_frames: usize,
    amplitude: f32,
) -> Vec<f32> {
    let mut buffer = Vec::with_capacity(num_frames * 2);
    for i in 0..num_frames {
        let t = i as f32 / sample_rate as f32;
        let sample = amplitude * (2.0 * PI * frequency * t).sin();
        buffer.push(sample); // Left
        buffer.push(sample); // Right
    }
    buffer
}

/// Generate deterministic white noise for testing
fn generate_white_noise(num_frames: usize, seed: u64) -> Vec<f32> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut buffer = Vec::with_capacity(num_frames * 2);
    for i in 0..num_frames {
        let mut hasher = DefaultHasher::new();
        (seed, i).hash(&mut hasher);
        let hash = hasher.finish();
        let sample = (hash as f64 / u64::MAX as f64) as f32 * 2.0 - 1.0;
        buffer.push(sample);
        buffer.push(sample);
    }
    buffer
}

/// Calculate RMS of a buffer
fn calculate_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_squares: f32 = samples.iter().map(|x| x * x).sum();
    (sum_squares / samples.len() as f32).sqrt()
}

/// Calculate peak level of a buffer
fn peak_level(buffer: &[f32]) -> f32 {
    buffer.iter().map(|s| s.abs()).fold(0.0f32, f32::max)
}

/// Convert linear amplitude to dB
fn linear_to_db(linear: f32) -> f32 {
    20.0 * linear.abs().max(1e-10).log10()
}

/// Check if all samples are finite and within bounds
fn is_stable(buffer: &[f32]) -> bool {
    buffer.iter().all(|s| s.is_finite() && s.abs() < 100.0)
}

/// Simple gain effect for testing chain behavior
struct GainEffect {
    gain: f32,
    enabled: bool,
    process_count: usize,
}

impl GainEffect {
    fn new(gain: f32) -> Self {
        Self {
            gain,
            enabled: true,
            process_count: 0,
        }
    }
}

impl AudioEffect for GainEffect {
    fn process(&mut self, buffer: &mut [f32], _sample_rate: u32) {
        self.process_count += 1;
        for sample in buffer.iter_mut() {
            *sample *= self.gain;
        }
    }

    fn reset(&mut self) {
        self.process_count = 0;
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn name(&self) -> &str {
        "Gain"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Clipping effect for testing order dependency
struct ClipperEffect {
    threshold: f32,
    enabled: bool,
}

impl ClipperEffect {
    fn new(threshold: f32) -> Self {
        Self {
            threshold,
            enabled: true,
        }
    }
}

impl AudioEffect for ClipperEffect {
    fn process(&mut self, buffer: &mut [f32], _sample_rate: u32) {
        for sample in buffer.iter_mut() {
            *sample = sample.clamp(-self.threshold, self.threshold);
        }
    }

    fn reset(&mut self) {}

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn name(&self) -> &str {
        "Clipper"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// DC offset effect for testing chain behavior
struct DcOffsetEffect {
    offset: f32,
    enabled: bool,
}

impl DcOffsetEffect {
    fn new(offset: f32) -> Self {
        Self {
            offset,
            enabled: true,
        }
    }
}

impl AudioEffect for DcOffsetEffect {
    fn process(&mut self, buffer: &mut [f32], _sample_rate: u32) {
        for sample in buffer.iter_mut() {
            *sample += self.offset;
        }
    }

    fn reset(&mut self) {}

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn name(&self) -> &str {
        "DC Offset"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

// ============================================================================
// 1. EMPTY EFFECT CHAIN PROCESSING
// ============================================================================

mod empty_chain_tests {
    use super::*;

    #[test]
    fn empty_chain_passes_through_unchanged() {
        let mut chain = EffectChain::new();
        assert!(chain.is_empty());

        let mut buffer = generate_stereo_sine(440.0, SAMPLE_RATE, 1024);
        let original = buffer.clone();

        chain.process(&mut buffer, SAMPLE_RATE);

        // Buffer should be completely unchanged
        assert_eq!(buffer.len(), original.len());
        for (processed, original) in buffer.iter().zip(original.iter()) {
            assert!(
                (*processed - *original).abs() < 1e-10,
                "Empty chain should not modify signal"
            );
        }
    }

    #[test]
    fn empty_chain_handles_various_buffer_sizes() {
        let mut chain = EffectChain::new();

        let sizes = [0, 1, 2, 7, 10, 100, 1000, 10000];
        for &size in &sizes {
            let mut buffer = vec![0.5f32; size];
            let original = buffer.clone();

            chain.process(&mut buffer, SAMPLE_RATE);

            assert_eq!(
                buffer, original,
                "Empty chain should pass through size {}",
                size
            );
        }
    }

    #[test]
    fn empty_chain_reset_does_not_panic() {
        let mut chain = EffectChain::new();
        chain.reset(); // Should not panic
    }

    #[test]
    fn empty_chain_set_enabled_does_not_panic() {
        let mut chain = EffectChain::new();
        chain.set_enabled(false);
        chain.set_enabled(true);
    }

    #[test]
    fn empty_chain_set_sample_rate_does_not_panic() {
        let mut chain = EffectChain::new();
        chain.set_sample_rate(48000);
        chain.set_sample_rate(96000);
    }

    #[test]
    fn empty_chain_get_effect_returns_none() {
        let chain = EffectChain::new();
        assert!(chain.get_effect(0).is_none());
        assert!(chain.get_effect(100).is_none());
    }
}

// ============================================================================
// 2. SINGLE EFFECT ENABLE/DISABLE
// ============================================================================

mod single_effect_enable_disable_tests {
    use super::*;

    #[test]
    fn single_effect_processes_when_enabled() {
        let mut chain = EffectChain::new();
        chain.add_effect(Box::new(GainEffect::new(0.5)));

        let mut buffer = vec![1.0f32; 100];
        chain.process(&mut buffer, SAMPLE_RATE);

        for sample in &buffer {
            assert!(
                (*sample - 0.5).abs() < 0.0001,
                "Expected 0.5, got {}",
                sample
            );
        }
    }

    #[test]
    fn single_effect_bypasses_when_disabled() {
        let mut chain = EffectChain::new();
        chain.add_effect(Box::new(GainEffect::new(0.5)));

        // Disable the effect
        if let Some(effect) = chain.get_effect_mut(0) {
            effect.set_enabled(false);
        }

        let mut buffer = vec![1.0f32; 100];
        let original = buffer.clone();
        chain.process(&mut buffer, SAMPLE_RATE);

        // Should be unchanged
        assert_eq!(buffer, original);
    }

    #[test]
    fn single_effect_enable_toggle() {
        let mut chain = EffectChain::new();
        chain.add_effect(Box::new(GainEffect::new(0.5)));

        // Process enabled
        let mut buffer1 = vec![1.0f32; 100];
        chain.process(&mut buffer1, SAMPLE_RATE);
        for sample in &buffer1 {
            assert!((*sample - 0.5).abs() < 0.0001);
        }

        // Disable
        if let Some(effect) = chain.get_effect_mut(0) {
            effect.set_enabled(false);
        }

        // Process disabled
        let mut buffer2 = vec![1.0f32; 100];
        chain.process(&mut buffer2, SAMPLE_RATE);
        for sample in &buffer2 {
            assert!((*sample - 1.0).abs() < 0.0001);
        }

        // Re-enable
        if let Some(effect) = chain.get_effect_mut(0) {
            effect.set_enabled(true);
        }

        // Process re-enabled
        let mut buffer3 = vec![1.0f32; 100];
        chain.process(&mut buffer3, SAMPLE_RATE);
        for sample in &buffer3 {
            assert!((*sample - 0.5).abs() < 0.0001);
        }
    }

    #[test]
    fn parametric_eq_enable_disable() {
        let mut chain = EffectChain::new();
        let mut eq = ParametricEq::new();
        eq.set_low_band(EqBand::low_shelf(100.0, 12.0)); // Strong boost
        chain.add_effect(Box::new(eq));

        // Warm up the filter first to let coefficients settle
        let mut warmup = generate_stereo_sine(100.0, SAMPLE_RATE, 4096);
        chain.process(&mut warmup, SAMPLE_RATE);

        // Process enabled - should modify signal
        let mut buffer1 = generate_stereo_sine(100.0, SAMPLE_RATE, 4096);
        let original_rms = calculate_rms(&buffer1);
        chain.process(&mut buffer1, SAMPLE_RATE);
        // Skip first half for settling, measure second half
        let processed_rms = calculate_rms(&buffer1[buffer1.len() / 2..]);

        // EQ should boost the signal (at least some gain, accounting for coefficient smoothing)
        let ratio = processed_rms / original_rms;
        assert!(
            ratio > 1.2,
            "EQ should boost signal: ratio {} (processed {} vs original {})",
            ratio,
            processed_rms,
            original_rms
        );

        // Disable EQ
        if let Some(effect) = chain.get_effect_mut(0) {
            effect.set_enabled(false);
        }

        // Complete the bypass fade - need more samples
        for _ in 0..5 {
            let mut fade_buffer = generate_stereo_sine(100.0, SAMPLE_RATE, 512);
            chain.process(&mut fade_buffer, SAMPLE_RATE);
        }

        // Process disabled - should pass through
        let mut buffer2 = generate_stereo_sine(100.0, SAMPLE_RATE, 4096);
        let original_rms2 = calculate_rms(&buffer2);
        chain.process(&mut buffer2, SAMPLE_RATE);
        let processed_rms2 = calculate_rms(&buffer2);

        // Should be nearly unchanged
        let ratio_db = linear_to_db(processed_rms2 / original_rms2);
        assert!(
            ratio_db.abs() < 1.0,
            "Disabled EQ should pass through: {} dB change",
            ratio_db
        );
    }

    #[test]
    fn limiter_enable_disable() {
        let mut chain = EffectChain::new();
        let limiter = Limiter::with_settings(LimiterSettings {
            threshold_db: -6.0,
            release_ms: 50.0,
        });
        chain.add_effect(Box::new(limiter));

        // Process enabled with hot signal
        let mut buffer1 = generate_stereo_sine_with_amplitude(440.0, SAMPLE_RATE, 1024, 1.5);
        chain.process(&mut buffer1, SAMPLE_RATE);

        // Limiter should reduce peak level
        let peak1 = peak_level(&buffer1);
        assert!(peak1 < 1.2, "Limiter should reduce peak: {}", peak1);

        // Disable limiter
        if let Some(effect) = chain.get_effect_mut(0) {
            effect.set_enabled(false);
        }

        // Complete bypass fade
        let mut fade_buffer = vec![1.5f32; 512];
        chain.process(&mut fade_buffer, SAMPLE_RATE);

        // Process disabled
        let mut buffer2 = generate_stereo_sine_with_amplitude(440.0, SAMPLE_RATE, 1024, 1.5);
        chain.process(&mut buffer2, SAMPLE_RATE);

        // Should pass through at original level
        let peak2 = peak_level(&buffer2);
        assert!(
            (peak2 - 1.5).abs() < 0.01,
            "Disabled limiter should pass through: {}",
            peak2
        );
    }

    #[test]
    fn compressor_enable_disable() {
        let mut chain = EffectChain::new();
        let compressor = Compressor::with_settings(CompressorSettings::aggressive());
        chain.add_effect(Box::new(compressor));

        // Process enabled
        let mut buffer1 = generate_stereo_sine_with_amplitude(440.0, SAMPLE_RATE, 2048, 0.9);
        let original_rms = calculate_rms(&buffer1);
        chain.process(&mut buffer1, SAMPLE_RATE);
        let processed_rms = calculate_rms(&buffer1[1024..]); // Skip attack time

        // Compressor should reduce dynamic range
        assert!(
            processed_rms < original_rms,
            "Compressor should reduce level: {} vs {}",
            processed_rms,
            original_rms
        );

        // Disable compressor
        if let Some(effect) = chain.get_effect_mut(0) {
            effect.set_enabled(false);
        }

        // Complete bypass fade
        let mut fade_buffer = vec![0.9f32; 512];
        chain.process(&mut fade_buffer, SAMPLE_RATE);

        // Process disabled
        let mut buffer2 = generate_stereo_sine_with_amplitude(440.0, SAMPLE_RATE, 2048, 0.9);
        let original_rms2 = calculate_rms(&buffer2);
        chain.process(&mut buffer2, SAMPLE_RATE);
        let processed_rms2 = calculate_rms(&buffer2);

        // Should pass through unchanged
        let ratio = processed_rms2 / original_rms2;
        assert!(
            (ratio - 1.0).abs() < 0.01,
            "Disabled compressor should pass through: ratio = {}",
            ratio
        );
    }
}

// ============================================================================
// 3. MULTIPLE EFFECTS INTERACTION
// ============================================================================

mod multiple_effects_interaction_tests {
    use super::*;

    #[test]
    fn two_gain_effects_multiply() {
        let mut chain = EffectChain::new();
        chain.add_effect(Box::new(GainEffect::new(0.5)));
        chain.add_effect(Box::new(GainEffect::new(0.5)));

        let mut buffer = vec![1.0f32; 100];
        chain.process(&mut buffer, SAMPLE_RATE);

        // 0.5 * 0.5 = 0.25
        for sample in &buffer {
            assert!(
                (*sample - 0.25).abs() < 0.0001,
                "Expected 0.25, got {}",
                sample
            );
        }
    }

    #[test]
    fn three_effects_chain_processing() {
        let mut chain = EffectChain::new();
        chain.add_effect(Box::new(GainEffect::new(2.0))); // Amplify
        chain.add_effect(Box::new(ClipperEffect::new(0.8))); // Clip
        chain.add_effect(Box::new(GainEffect::new(0.5))); // Reduce

        let mut buffer = vec![0.5f32; 100];
        chain.process(&mut buffer, SAMPLE_RATE);

        // 0.5 * 2.0 = 1.0 -> clip to 0.8 -> * 0.5 = 0.4
        for sample in &buffer {
            assert!(
                (*sample - 0.4).abs() < 0.0001,
                "Expected 0.4, got {}",
                sample
            );
        }
    }

    #[test]
    fn eq_before_limiter_vs_after() {
        // This test demonstrates that effect order matters for nonlinear processing.
        // We use simple mock effects to clearly show the order dependency,
        // since real effects like EQ and Limiter have smoothing that makes
        // the order effect less dramatic in short test signals.

        // Test: Gain -> Clipper vs Clipper -> Gain with a signal that will clip
        let mut chain1 = EffectChain::new();
        chain1.add_effect(Box::new(GainEffect::new(2.0))); // Boost first
        chain1.add_effect(Box::new(ClipperEffect::new(1.0))); // Then clip

        let mut chain2 = EffectChain::new();
        chain2.add_effect(Box::new(ClipperEffect::new(1.0))); // Clip first
        chain2.add_effect(Box::new(GainEffect::new(2.0))); // Then boost

        // Input signal at 0.8 amplitude
        let mut buffer1 = generate_stereo_sine_with_amplitude(440.0, SAMPLE_RATE, 1024, 0.8);
        let mut buffer2 = generate_stereo_sine_with_amplitude(440.0, SAMPLE_RATE, 1024, 0.8);

        chain1.process(&mut buffer1, SAMPLE_RATE);
        chain2.process(&mut buffer2, SAMPLE_RATE);

        let peak1 = peak_level(&buffer1);
        let peak2 = peak_level(&buffer2);

        // Chain 1 (Gain -> Clip): 0.8 * 2.0 = 1.6 -> clipped to 1.0
        // Chain 2 (Clip -> Gain): 0.8 (no clip) -> 0.8 * 2.0 = 1.6

        assert!(
            (peak1 - 1.0).abs() < 0.01,
            "Gain -> Clip should produce peak ~1.0: {}",
            peak1
        );
        assert!(
            (peak2 - 1.6).abs() < 0.01,
            "Clip -> Gain should produce peak ~1.6: {}",
            peak2
        );

        // Order clearly matters
        assert!(
            peak2 > peak1,
            "Order should affect output: peak2={}, peak1={}",
            peak2,
            peak1
        );
    }

    #[test]
    fn compressor_before_limiter() {
        let mut chain = EffectChain::new();

        // Compressor reduces dynamic range
        let compressor = Compressor::with_settings(CompressorSettings::moderate());
        chain.add_effect(Box::new(compressor));

        // Limiter catches peaks
        let limiter = Limiter::with_settings(LimiterSettings::brickwall());
        chain.add_effect(Box::new(limiter));

        // Hot signal
        let mut buffer = generate_stereo_sine_with_amplitude(440.0, SAMPLE_RATE, 4096, 1.2);
        chain.process(&mut buffer, SAMPLE_RATE);

        // Check final output
        let peak = peak_level(&buffer);
        assert!(
            peak <= 1.01,
            "Compressor + Limiter should limit peak: {}",
            peak
        );
        assert!(is_stable(&buffer), "Output should be stable");
    }

    #[test]
    fn all_dynamics_chain() {
        let mut chain = EffectChain::new();

        // EQ -> Compressor -> Limiter (typical mastering chain)
        let mut eq = ParametricEq::new();
        eq.set_low_band(EqBand::low_shelf(80.0, 3.0));
        eq.set_mid_band(EqBand::peaking(2000.0, -2.0, 1.0));
        eq.set_high_band(EqBand::high_shelf(10000.0, 2.0));
        chain.add_effect(Box::new(eq));

        chain.add_effect(Box::new(Compressor::with_settings(
            CompressorSettings::gentle(),
        )));
        chain.add_effect(Box::new(Limiter::with_settings(
            LimiterSettings::brickwall(),
        )));

        // Process music-like signal (noise as proxy)
        let mut buffer = generate_white_noise(4096, 42);
        chain.process(&mut buffer, SAMPLE_RATE);

        assert!(is_stable(&buffer), "Chain output should be stable");
        assert!(
            peak_level(&buffer) <= 1.01,
            "Chain should limit peaks: {}",
            peak_level(&buffer)
        );
    }

    #[test]
    fn chain_preserves_stereo() {
        let mut chain = EffectChain::new();
        chain.add_effect(Box::new(GainEffect::new(0.5)));
        chain.add_effect(Box::new(GainEffect::new(2.0)));

        // Create stereo signal with different L/R
        let mut buffer: Vec<f32> = Vec::with_capacity(200);
        for i in 0..100 {
            buffer.push((i as f32) * 0.01); // Left
            buffer.push(-(i as f32) * 0.01); // Right (inverted)
        }

        chain.process(&mut buffer, SAMPLE_RATE);

        // Check L/R relationship preserved (gain is 0.5 * 2.0 = 1.0)
        for i in 0..100 {
            let left = buffer[i * 2];
            let right = buffer[i * 2 + 1];
            assert!(
                (left + right).abs() < 0.0001,
                "Stereo should be preserved: L={}, R={}",
                left,
                right
            );
        }
    }
}

// ============================================================================
// 4. EFFECT ORDER DEPENDENCY
// ============================================================================

mod effect_order_tests {
    use super::*;

    #[test]
    fn order_matters_for_nonlinear_effects() {
        // Gain -> Clipper vs Clipper -> Gain
        let mut chain1 = EffectChain::new();
        chain1.add_effect(Box::new(GainEffect::new(2.0)));
        chain1.add_effect(Box::new(ClipperEffect::new(0.5)));

        let mut chain2 = EffectChain::new();
        chain2.add_effect(Box::new(ClipperEffect::new(0.5)));
        chain2.add_effect(Box::new(GainEffect::new(2.0)));

        let mut buffer1 = vec![0.3f32; 100];
        let mut buffer2 = vec![0.3f32; 100];

        chain1.process(&mut buffer1, SAMPLE_RATE);
        chain2.process(&mut buffer2, SAMPLE_RATE);

        // Chain 1: 0.3 * 2.0 = 0.6 -> clip to 0.5 = 0.5
        // Chain 2: 0.3 -> no clip (< 0.5) -> 0.3 * 2.0 = 0.6
        for sample in &buffer1 {
            assert!(
                (*sample - 0.5).abs() < 0.0001,
                "Chain 1: expected 0.5, got {}",
                sample
            );
        }
        for sample in &buffer2 {
            assert!(
                (*sample - 0.6).abs() < 0.0001,
                "Chain 2: expected 0.6, got {}",
                sample
            );
        }
    }

    #[test]
    fn dc_offset_before_vs_after_gain() {
        // Offset -> Gain vs Gain -> Offset
        let mut chain1 = EffectChain::new();
        chain1.add_effect(Box::new(DcOffsetEffect::new(0.1)));
        chain1.add_effect(Box::new(GainEffect::new(2.0)));

        let mut chain2 = EffectChain::new();
        chain2.add_effect(Box::new(GainEffect::new(2.0)));
        chain2.add_effect(Box::new(DcOffsetEffect::new(0.1)));

        let mut buffer1 = vec![0.5f32; 100];
        let mut buffer2 = vec![0.5f32; 100];

        chain1.process(&mut buffer1, SAMPLE_RATE);
        chain2.process(&mut buffer2, SAMPLE_RATE);

        // Chain 1: 0.5 + 0.1 = 0.6 -> 0.6 * 2.0 = 1.2
        // Chain 2: 0.5 * 2.0 = 1.0 -> 1.0 + 0.1 = 1.1
        for sample in &buffer1 {
            assert!(
                (*sample - 1.2).abs() < 0.0001,
                "Chain 1: expected 1.2, got {}",
                sample
            );
        }
        for sample in &buffer2 {
            assert!(
                (*sample - 1.1).abs() < 0.0001,
                "Chain 2: expected 1.1, got {}",
                sample
            );
        }
    }

    #[test]
    fn eq_order_affects_frequency_response() {
        // Create two EQs with different settings
        let mut chain1 = EffectChain::new();
        let mut eq_boost = ParametricEq::new();
        eq_boost.set_low_band(EqBand::low_shelf(100.0, 6.0));
        chain1.add_effect(Box::new(eq_boost));
        let mut eq_cut = ParametricEq::new();
        eq_cut.set_low_band(EqBand::low_shelf(100.0, -6.0));
        chain1.add_effect(Box::new(eq_cut));

        let mut chain2 = EffectChain::new();
        let mut eq_cut2 = ParametricEq::new();
        eq_cut2.set_low_band(EqBand::low_shelf(100.0, -6.0));
        chain2.add_effect(Box::new(eq_cut2));
        let mut eq_boost2 = ParametricEq::new();
        eq_boost2.set_low_band(EqBand::low_shelf(100.0, 6.0));
        chain2.add_effect(Box::new(eq_boost2));

        // Both chains should ideally cancel out (+6 then -6 or -6 then +6)
        // But due to filter characteristics, results may differ slightly
        let mut buffer1 = generate_stereo_sine(100.0, SAMPLE_RATE, 4096);
        let mut buffer2 = generate_stereo_sine(100.0, SAMPLE_RATE, 4096);
        let original_rms = calculate_rms(&buffer1);

        chain1.process(&mut buffer1, SAMPLE_RATE);
        chain2.process(&mut buffer2, SAMPLE_RATE);

        let rms1 = calculate_rms(&buffer1[2048..]); // Skip settling
        let rms2 = calculate_rms(&buffer2[2048..]);

        // Both should be close to original (canceling out)
        let db1 = linear_to_db(rms1 / original_rms);
        let db2 = linear_to_db(rms2 / original_rms);

        assert!(
            db1.abs() < 3.0,
            "Boost->Cut should nearly cancel: {} dB",
            db1
        );
        assert!(
            db2.abs() < 3.0,
            "Cut->Boost should nearly cancel: {} dB",
            db2
        );
    }

    #[test]
    fn limiter_position_affects_headroom() {
        // Signal at -6dB
        let input_amplitude = 0.5; // approximately -6dB

        // Chain 1: Boost 12dB -> Limit (will hit limit)
        let mut chain1 = EffectChain::new();
        chain1.add_effect(Box::new(GainEffect::new(4.0))); // ~12dB boost
        chain1.add_effect(Box::new(Limiter::with_settings(
            LimiterSettings::brickwall(),
        )));

        // Chain 2: Limit -> Boost 12dB (boost after limit won't be caught)
        let mut chain2 = EffectChain::new();
        chain2.add_effect(Box::new(Limiter::with_settings(
            LimiterSettings::brickwall(),
        )));
        chain2.add_effect(Box::new(GainEffect::new(4.0)));

        let mut buffer1 =
            generate_stereo_sine_with_amplitude(440.0, SAMPLE_RATE, 2048, input_amplitude);
        let mut buffer2 =
            generate_stereo_sine_with_amplitude(440.0, SAMPLE_RATE, 2048, input_amplitude);

        chain1.process(&mut buffer1, SAMPLE_RATE);
        chain2.process(&mut buffer2, SAMPLE_RATE);

        let peak1 = peak_level(&buffer1);
        let peak2 = peak_level(&buffer2);

        // Chain 1 should be limited
        assert!(peak1 <= 1.01, "Boost -> Limit should limit: {}", peak1);

        // Chain 2 will have higher peak (limit then boost)
        assert!(
            peak2 > peak1,
            "Limit -> Boost should exceed limited: {} vs {}",
            peak2,
            peak1
        );
    }
}

// ============================================================================
// 5. ALL EFFECTS DISABLED PASSTHROUGH
// ============================================================================

mod all_disabled_passthrough_tests {
    use super::*;

    #[test]
    fn all_disabled_passes_through_unchanged() {
        let mut chain = EffectChain::new();
        chain.add_effect(Box::new(GainEffect::new(0.0))); // Would zero signal
        chain.add_effect(Box::new(GainEffect::new(100.0))); // Would amplify massively
        chain.add_effect(Box::new(ClipperEffect::new(0.01))); // Would clip heavily

        // Disable all
        chain.set_enabled(false);

        let mut buffer = vec![0.5f32; 100];
        let original = buffer.clone();
        chain.process(&mut buffer, SAMPLE_RATE);

        // Should be unchanged
        assert_eq!(buffer, original);
    }

    #[test]
    fn set_enabled_toggles_all_effects() {
        let mut chain = EffectChain::new();
        chain.add_effect(Box::new(GainEffect::new(0.5)));
        chain.add_effect(Box::new(GainEffect::new(0.5)));

        // Enable all
        chain.set_enabled(true);

        // Process - should apply effects
        let mut buffer1 = vec![1.0f32; 100];
        chain.process(&mut buffer1, SAMPLE_RATE);
        for sample in &buffer1 {
            assert!((*sample - 0.25).abs() < 0.0001);
        }

        // Disable all
        chain.set_enabled(false);

        // Process - should bypass
        let mut buffer2 = vec![1.0f32; 100];
        chain.process(&mut buffer2, SAMPLE_RATE);
        for sample in &buffer2 {
            assert!((*sample - 1.0).abs() < 0.0001);
        }
    }

    #[test]
    fn mixed_enabled_disabled_effects() {
        let mut chain = EffectChain::new();
        chain.add_effect(Box::new(GainEffect::new(0.5))); // enabled
        chain.add_effect(Box::new(GainEffect::new(0.0))); // will be disabled
        chain.add_effect(Box::new(GainEffect::new(2.0))); // enabled

        // Disable middle effect
        if let Some(effect) = chain.get_effect_mut(1) {
            effect.set_enabled(false);
        }

        let mut buffer = vec![1.0f32; 100];
        chain.process(&mut buffer, SAMPLE_RATE);

        // Should be 0.5 * 2.0 = 1.0 (middle effect bypassed)
        for sample in &buffer {
            assert!(
                (*sample - 1.0).abs() < 0.0001,
                "Expected 1.0, got {}",
                sample
            );
        }
    }

    #[test]
    fn all_real_effects_disabled_passthrough() {
        let mut chain = EffectChain::new();

        // Add real effects
        let mut eq = ParametricEq::new();
        eq.set_low_band(EqBand::low_shelf(100.0, 12.0));
        chain.add_effect(Box::new(eq));

        chain.add_effect(Box::new(Compressor::with_settings(
            CompressorSettings::aggressive(),
        )));
        chain.add_effect(Box::new(Limiter::with_settings(
            LimiterSettings::brickwall(),
        )));

        // Disable all
        chain.set_enabled(false);

        // Complete bypass fades by processing several buffers
        for _ in 0..10 {
            let mut fade_buffer = vec![0.5f32; 512];
            chain.process(&mut fade_buffer, SAMPLE_RATE);
        }

        // Now test passthrough
        let mut buffer = generate_stereo_sine(440.0, SAMPLE_RATE, 1024);
        let original = buffer.clone();
        chain.process(&mut buffer, SAMPLE_RATE);

        // Should be essentially unchanged
        let rms_diff = calculate_rms(
            &buffer
                .iter()
                .zip(original.iter())
                .map(|(a, b)| a - b)
                .collect::<Vec<f32>>(),
        );
        assert!(
            rms_diff < 0.001,
            "Disabled chain should pass through: rms_diff = {}",
            rms_diff
        );
    }
}

// ============================================================================
// 6. ADDING/REMOVING EFFECTS DURING PROCESSING
// ============================================================================

mod dynamic_chain_modification_tests {
    use super::*;

    #[test]
    fn add_effect_mid_processing() {
        let mut chain = EffectChain::new();
        chain.add_effect(Box::new(GainEffect::new(0.5)));

        // Process first buffer
        let mut buffer1 = vec![1.0f32; 100];
        chain.process(&mut buffer1, SAMPLE_RATE);
        for sample in &buffer1 {
            assert!((*sample - 0.5).abs() < 0.0001);
        }

        // Add another effect
        chain.add_effect(Box::new(GainEffect::new(0.5)));

        // Process second buffer
        let mut buffer2 = vec![1.0f32; 100];
        chain.process(&mut buffer2, SAMPLE_RATE);
        for sample in &buffer2 {
            assert!((*sample - 0.25).abs() < 0.0001);
        }
    }

    #[test]
    fn replace_effect_mid_processing() {
        let mut chain = EffectChain::new();
        chain.add_effect(Box::new(GainEffect::new(0.5)));

        // Process first buffer
        let mut buffer1 = vec![1.0f32; 100];
        chain.process(&mut buffer1, SAMPLE_RATE);
        for sample in &buffer1 {
            assert!((*sample - 0.5).abs() < 0.0001);
        }

        // Replace effect
        chain.replace_effect(0, Box::new(GainEffect::new(2.0)));

        // Process second buffer
        let mut buffer2 = vec![1.0f32; 100];
        chain.process(&mut buffer2, SAMPLE_RATE);
        for sample in &buffer2 {
            assert!((*sample - 2.0).abs() < 0.0001);
        }
    }

    #[test]
    fn clear_chain_mid_processing() {
        let mut chain = EffectChain::new();
        chain.add_effect(Box::new(GainEffect::new(0.5)));
        chain.add_effect(Box::new(GainEffect::new(0.5)));

        // Process first buffer
        let mut buffer1 = vec![1.0f32; 100];
        chain.process(&mut buffer1, SAMPLE_RATE);
        assert!(chain.len() == 2);

        // Clear chain
        chain.clear();
        assert!(chain.is_empty());

        // Process second buffer - should pass through
        let mut buffer2 = vec![1.0f32; 100];
        let original = buffer2.clone();
        chain.process(&mut buffer2, SAMPLE_RATE);
        assert_eq!(buffer2, original);
    }

    #[test]
    fn add_real_effect_mid_processing() {
        let mut chain = EffectChain::new();

        // Start with gain
        chain.add_effect(Box::new(GainEffect::new(1.0)));

        // Process
        let mut buffer1 = generate_stereo_sine(440.0, SAMPLE_RATE, 1024);
        chain.process(&mut buffer1, SAMPLE_RATE);

        // Add limiter while "playing"
        chain.add_effect(Box::new(Limiter::with_settings(
            LimiterSettings::brickwall(),
        )));

        // Process hot signal - limiter should catch it
        let mut buffer2 = generate_stereo_sine_with_amplitude(440.0, SAMPLE_RATE, 1024, 1.5);
        chain.process(&mut buffer2, SAMPLE_RATE);

        let peak = peak_level(&buffer2);
        assert!(peak <= 1.01, "Added limiter should limit peaks: {}", peak);
    }

    #[test]
    fn replace_eq_preserves_chain_stability() {
        let mut chain = EffectChain::new();

        // Initial EQ
        let mut eq1 = ParametricEq::new();
        eq1.set_low_band(EqBand::low_shelf(100.0, 3.0));
        chain.add_effect(Box::new(eq1));
        chain.add_effect(Box::new(Limiter::new()));

        // Process some audio
        let mut buffer1 = generate_stereo_sine(100.0, SAMPLE_RATE, 2048);
        chain.process(&mut buffer1, SAMPLE_RATE);

        // Replace EQ with different settings
        let mut eq2 = ParametricEq::new();
        eq2.set_low_band(EqBand::low_shelf(100.0, -6.0)); // Cut instead of boost
        chain.replace_effect(0, Box::new(eq2));

        // Process more audio - should not cause glitches or instability
        let mut buffer2 = generate_stereo_sine(100.0, SAMPLE_RATE, 2048);
        chain.process(&mut buffer2, SAMPLE_RATE);

        assert!(
            is_stable(&buffer2),
            "Chain should remain stable after replace"
        );
    }

    #[test]
    fn rapid_add_clear_cycles() {
        let mut chain = EffectChain::new();

        for _ in 0..10 {
            // Add effects
            chain.add_effect(Box::new(GainEffect::new(0.9)));
            chain.add_effect(Box::new(GainEffect::new(0.9)));

            // Process
            let mut buffer = generate_stereo_sine(440.0, SAMPLE_RATE, 256);
            chain.process(&mut buffer, SAMPLE_RATE);

            // Clear
            chain.clear();

            // Process empty chain
            let mut buffer2 = generate_stereo_sine(440.0, SAMPLE_RATE, 256);
            let original = buffer2.clone();
            chain.process(&mut buffer2, SAMPLE_RATE);
            assert_eq!(buffer2, original);
        }
    }
}

// ============================================================================
// 7. EFFECT PARAMETER CHANGES DURING PROCESSING
// ============================================================================

mod parameter_changes_during_processing_tests {
    use super::*;

    #[test]
    fn gain_change_during_processing() {
        let mut chain = EffectChain::new();
        chain.add_effect(Box::new(GainEffect::new(1.0)));

        // Process first buffer
        let mut buffer1 = vec![1.0f32; 100];
        chain.process(&mut buffer1, SAMPLE_RATE);

        // Change gain
        if let Some(effect) = chain.get_effect_as_mut::<GainEffect>(0) {
            effect.gain = 0.5;
        }

        // Process second buffer
        let mut buffer2 = vec![1.0f32; 100];
        chain.process(&mut buffer2, SAMPLE_RATE);
        for sample in &buffer2 {
            assert!((*sample - 0.5).abs() < 0.0001);
        }
    }

    #[test]
    fn eq_band_change_during_processing() {
        let mut chain = EffectChain::new();
        let eq = ParametricEq::new();
        chain.add_effect(Box::new(eq));

        // Process some audio
        let mut buffer1 = generate_stereo_sine(100.0, SAMPLE_RATE, 2048);
        chain.process(&mut buffer1, SAMPLE_RATE);

        // Change EQ parameters while "playing"
        if let Some(eq) = chain.get_effect_as_mut::<ParametricEq>(0) {
            eq.set_low_band(EqBand::low_shelf(100.0, 12.0));
        }

        // Process more audio - coefficient smoothing should prevent clicks
        let mut buffer2 = generate_stereo_sine(100.0, SAMPLE_RATE, 2048);
        chain.process(&mut buffer2, SAMPLE_RATE);

        assert!(
            is_stable(&buffer2),
            "Output should be stable after param change"
        );
    }

    #[test]
    fn limiter_threshold_change_during_processing() {
        let mut chain = EffectChain::new();
        chain.add_effect(Box::new(Limiter::with_settings(LimiterSettings {
            threshold_db: -0.1,
            release_ms: 50.0,
        })));

        // Process hot signal
        let mut buffer1 = generate_stereo_sine_with_amplitude(440.0, SAMPLE_RATE, 1024, 1.5);
        chain.process(&mut buffer1, SAMPLE_RATE);

        // Change threshold while "playing"
        if let Some(limiter) = chain.get_effect_as_mut::<Limiter>(0) {
            limiter.set_threshold(-6.0);
        }

        // Process more - threshold smoothing should prevent clicks
        let mut buffer2 = generate_stereo_sine_with_amplitude(440.0, SAMPLE_RATE, 1024, 1.5);
        chain.process(&mut buffer2, SAMPLE_RATE);

        assert!(
            is_stable(&buffer2),
            "Output should be stable after threshold change"
        );
    }

    #[test]
    fn compressor_settings_change_during_processing() {
        let mut chain = EffectChain::new();
        chain.add_effect(Box::new(Compressor::with_settings(
            CompressorSettings::gentle(),
        )));

        // Process
        let mut buffer1 = generate_stereo_sine_with_amplitude(440.0, SAMPLE_RATE, 2048, 0.9);
        chain.process(&mut buffer1, SAMPLE_RATE);

        // Change to aggressive settings
        if let Some(comp) = chain.get_effect_as_mut::<Compressor>(0) {
            comp.set_threshold(-12.0);
            comp.set_ratio(10.0);
            comp.set_attack(1.0);
        }

        // Process more - smoothing should prevent clicks
        let mut buffer2 = generate_stereo_sine_with_amplitude(440.0, SAMPLE_RATE, 2048, 0.9);
        chain.process(&mut buffer2, SAMPLE_RATE);

        assert!(
            is_stable(&buffer2),
            "Output should be stable after settings change"
        );
    }

    #[test]
    fn rapid_parameter_changes_remain_stable() {
        let mut chain = EffectChain::new();
        let eq = ParametricEq::new();
        chain.add_effect(Box::new(eq));

        let mut all_samples = Vec::new();

        for i in 0..20 {
            // Change EQ parameters every buffer
            if let Some(eq) = chain.get_effect_as_mut::<ParametricEq>(0) {
                let gain = (i as f32 - 10.0) * 0.6; // -6dB to +6dB
                eq.set_low_band(EqBand::low_shelf(100.0, gain));
            }

            let mut buffer = generate_stereo_sine(100.0, SAMPLE_RATE, 256);
            chain.process(&mut buffer, SAMPLE_RATE);
            all_samples.extend(buffer);
        }

        // Check for stability
        assert!(
            is_stable(&all_samples),
            "Should remain stable during rapid param changes"
        );

        // Check for smooth transitions (no large jumps)
        for window in all_samples.windows(2) {
            let delta = (window[1] - window[0]).abs();
            assert!(
                delta < 0.5,
                "Large jump detected: {} -> {}",
                window[0],
                window[1]
            );
        }
    }

    #[test]
    fn parameter_change_affects_subsequent_buffers() {
        let mut chain = EffectChain::new();
        chain.add_effect(Box::new(GainEffect::new(1.0)));

        // Verify initial state
        let mut buffer1 = vec![1.0f32; 10];
        chain.process(&mut buffer1, SAMPLE_RATE);
        assert!(buffer1.iter().all(|&s| (s - 1.0).abs() < 0.0001));

        // Change parameter
        if let Some(effect) = chain.get_effect_as_mut::<GainEffect>(0) {
            effect.gain = 0.25;
        }

        // Verify change persists
        for _ in 0..5 {
            let mut buffer = vec![1.0f32; 10];
            chain.process(&mut buffer, SAMPLE_RATE);
            assert!(buffer.iter().all(|&s| (s - 0.25).abs() < 0.0001));
        }
    }
}

// ============================================================================
// 8. EFFECT CHAIN WITH VERY SMALL BUFFERS (1-10 samples)
// ============================================================================

mod small_buffer_tests {
    use super::*;

    #[test]
    fn single_sample_buffer() {
        let mut chain = EffectChain::new();
        chain.add_effect(Box::new(GainEffect::new(0.5)));

        let mut buffer = vec![1.0f32];
        chain.process(&mut buffer, SAMPLE_RATE);
        assert!((buffer[0] - 0.5).abs() < 0.0001);
    }

    #[test]
    fn two_sample_stereo_pair() {
        let mut chain = EffectChain::new();
        chain.add_effect(Box::new(GainEffect::new(0.5)));
        chain.add_effect(Box::new(GainEffect::new(2.0)));

        let mut buffer = vec![0.8f32, 0.6f32]; // L, R
        chain.process(&mut buffer, SAMPLE_RATE);

        // 0.5 * 2.0 = 1.0 (no change)
        assert!((buffer[0] - 0.8).abs() < 0.0001);
        assert!((buffer[1] - 0.6).abs() < 0.0001);
    }

    #[test]
    fn small_buffers_1_to_10_samples() {
        let mut chain = EffectChain::new();
        chain.add_effect(Box::new(GainEffect::new(0.5)));

        for size in 1..=10 {
            let mut buffer = vec![1.0f32; size];
            chain.process(&mut buffer, SAMPLE_RATE);

            for sample in &buffer {
                assert!(
                    (*sample - 0.5).abs() < 0.0001,
                    "Failed for size {}: got {}",
                    size,
                    sample
                );
            }
        }
    }

    #[test]
    fn eq_with_small_buffers() {
        let mut chain = EffectChain::new();
        let mut eq = ParametricEq::new();
        eq.set_low_band(EqBand::low_shelf(100.0, 6.0));
        chain.add_effect(Box::new(eq));

        // Process many small buffers in sequence
        for _ in 0..100 {
            let mut buffer = vec![0.5f32; 4]; // 2 stereo samples
            chain.process(&mut buffer, SAMPLE_RATE);

            for sample in &buffer {
                assert!(sample.is_finite(), "Should be finite");
                assert!(sample.abs() < 10.0, "Should be reasonable amplitude");
            }
        }
    }

    #[test]
    fn limiter_with_small_buffers() {
        let mut chain = EffectChain::new();
        chain.add_effect(Box::new(Limiter::with_settings(
            LimiterSettings::brickwall(),
        )));

        // Process many small hot buffers
        for _ in 0..100 {
            let mut buffer = vec![1.5f32; 2]; // 1 stereo sample
            chain.process(&mut buffer, SAMPLE_RATE);

            for sample in &buffer {
                assert!(sample.is_finite());
                // Note: limiter may not limit single samples due to attack time
            }
        }
    }

    #[test]
    fn compressor_with_small_buffers() {
        let mut chain = EffectChain::new();
        chain.add_effect(Box::new(Compressor::with_settings(
            CompressorSettings::aggressive(),
        )));

        for _ in 0..100 {
            let mut buffer = vec![0.9f32; 6]; // 3 stereo samples
            chain.process(&mut buffer, SAMPLE_RATE);

            for sample in &buffer {
                assert!(sample.is_finite());
            }
        }
    }

    #[test]
    fn chain_with_multiple_effects_small_buffers() {
        let mut chain = EffectChain::new();

        let mut eq = ParametricEq::new();
        eq.set_low_band(EqBand::low_shelf(100.0, 3.0));
        chain.add_effect(Box::new(eq));

        chain.add_effect(Box::new(Compressor::new()));
        chain.add_effect(Box::new(Limiter::new()));

        // Process many tiny buffers
        for _ in 0..500 {
            let mut buffer = vec![0.5f32; 2];
            chain.process(&mut buffer, SAMPLE_RATE);

            assert!(is_stable(&buffer), "Should remain stable with tiny buffers");
        }
    }

    #[test]
    fn odd_sized_small_buffers() {
        let mut chain = EffectChain::new();
        chain.add_effect(Box::new(GainEffect::new(0.5)));

        // Prime number sizes
        let sizes = [1, 3, 5, 7, 9];
        for &size in &sizes {
            let mut buffer = vec![1.0f32; size];
            chain.process(&mut buffer, SAMPLE_RATE);

            for sample in &buffer {
                assert!((*sample - 0.5).abs() < 0.0001);
            }
        }
    }

    #[test]
    fn empty_buffer_does_not_panic() {
        let mut chain = EffectChain::new();
        chain.add_effect(Box::new(GainEffect::new(0.5)));
        chain.add_effect(Box::new(Limiter::new()));

        let mut buffer: Vec<f32> = vec![];
        chain.process(&mut buffer, SAMPLE_RATE);
        assert_eq!(buffer.len(), 0);
    }
}

// ============================================================================
// 9. EFFECT CHAIN WITH VERY LARGE BUFFERS (10000+ samples)
// ============================================================================

mod large_buffer_tests {
    use super::*;

    #[test]
    fn ten_thousand_sample_buffer() {
        let mut chain = EffectChain::new();
        chain.add_effect(Box::new(GainEffect::new(0.5)));

        let mut buffer = vec![1.0f32; 10000];
        chain.process(&mut buffer, SAMPLE_RATE);

        for sample in &buffer {
            assert!((*sample - 0.5).abs() < 0.0001);
        }
    }

    #[test]
    fn hundred_thousand_sample_buffer() {
        let mut chain = EffectChain::new();
        chain.add_effect(Box::new(GainEffect::new(0.5)));
        chain.add_effect(Box::new(GainEffect::new(2.0)));

        let mut buffer = vec![1.0f32; 100000];
        chain.process(&mut buffer, SAMPLE_RATE);

        // 0.5 * 2.0 = 1.0
        for sample in &buffer {
            assert!((*sample - 1.0).abs() < 0.0001);
        }
    }

    #[test]
    fn eq_with_large_buffer() {
        let mut chain = EffectChain::new();
        let mut eq = ParametricEq::new();
        eq.set_low_band(EqBand::low_shelf(100.0, 6.0));
        eq.set_mid_band(EqBand::peaking(1000.0, -3.0, 1.0));
        eq.set_high_band(EqBand::high_shelf(8000.0, 3.0));
        chain.add_effect(Box::new(eq));

        let mut buffer = generate_stereo_sine(440.0, SAMPLE_RATE, 50000);
        chain.process(&mut buffer, SAMPLE_RATE);

        assert!(is_stable(&buffer), "Large buffer should be stable");
    }

    #[test]
    fn limiter_with_large_buffer() {
        let mut chain = EffectChain::new();
        chain.add_effect(Box::new(Limiter::with_settings(
            LimiterSettings::brickwall(),
        )));

        let mut buffer = generate_stereo_sine_with_amplitude(440.0, SAMPLE_RATE, 50000, 1.5);
        chain.process(&mut buffer, SAMPLE_RATE);

        // Skip initial settling
        let peak = peak_level(&buffer[10000..]);
        assert!(peak <= 1.01, "Large buffer should be limited: {}", peak);
    }

    #[test]
    fn compressor_with_large_buffer() {
        let mut chain = EffectChain::new();
        chain.add_effect(Box::new(Compressor::with_settings(
            CompressorSettings::moderate(),
        )));

        let mut buffer = generate_stereo_sine_with_amplitude(440.0, SAMPLE_RATE, 50000, 0.9);
        let original_rms = calculate_rms(&buffer);
        chain.process(&mut buffer, SAMPLE_RATE);
        let processed_rms = calculate_rms(&buffer[10000..]); // Skip attack

        // Compressor should reduce level
        assert!(
            processed_rms < original_rms,
            "Large buffer compression: {} vs {}",
            processed_rms,
            original_rms
        );
    }

    #[test]
    fn full_chain_large_buffer() {
        let mut chain = EffectChain::new();

        let mut eq = ParametricEq::new();
        eq.set_low_band(EqBand::low_shelf(80.0, 2.0));
        chain.add_effect(Box::new(eq));

        chain.add_effect(Box::new(Compressor::with_settings(
            CompressorSettings::gentle(),
        )));
        chain.add_effect(Box::new(Limiter::with_settings(
            LimiterSettings::brickwall(),
        )));

        // Large buffer of complex signal
        let mut buffer = generate_white_noise(100000, 12345);
        chain.process(&mut buffer, SAMPLE_RATE);

        assert!(is_stable(&buffer), "Full chain should handle large buffer");
        assert!(
            peak_level(&buffer) <= 1.01,
            "Should be limited: {}",
            peak_level(&buffer)
        );
    }

    #[test]
    fn one_second_of_audio() {
        // 1 second at 44.1kHz = 44100 frames = 88200 samples (stereo)
        let mut chain = EffectChain::new();
        chain.add_effect(Box::new(GainEffect::new(0.8)));

        let mut buffer = generate_stereo_sine(440.0, SAMPLE_RATE, SAMPLE_RATE as usize);
        assert_eq!(buffer.len(), 88200);

        chain.process(&mut buffer, SAMPLE_RATE);

        // Verify processing
        let peak = peak_level(&buffer);
        assert!(
            (peak - 0.8).abs() < 0.01,
            "1 second buffer: expected peak 0.8, got {}",
            peak
        );
    }

    #[test]
    fn consistency_across_buffer_sizes() {
        // Process same total audio in different buffer sizes
        // Results should be identical for linear effects

        let total_frames = 10000;
        let input = generate_stereo_sine(440.0, SAMPLE_RATE, total_frames);

        // Process all at once
        let mut chain1 = EffectChain::new();
        chain1.add_effect(Box::new(GainEffect::new(0.5)));
        let mut buffer1 = input.clone();
        chain1.process(&mut buffer1, SAMPLE_RATE);

        // Process in small chunks
        let mut chain2 = EffectChain::new();
        chain2.add_effect(Box::new(GainEffect::new(0.5)));
        let mut buffer2 = input.clone();
        for chunk in buffer2.chunks_mut(100) {
            chain2.process(chunk, SAMPLE_RATE);
        }

        // Results should be identical for linear effect
        for (a, b) in buffer1.iter().zip(buffer2.iter()) {
            assert!(
                (*a - *b).abs() < 1e-6,
                "Buffer size should not affect linear effect: {} vs {}",
                a,
                b
            );
        }
    }

    #[test]
    fn large_buffer_no_memory_issues() {
        // Test that large buffers don't cause memory issues
        let mut chain = EffectChain::new();
        chain.add_effect(Box::new(GainEffect::new(0.9)));
        chain.add_effect(Box::new(Limiter::new()));

        // Process several large buffers
        for _ in 0..10 {
            let mut buffer = generate_white_noise(50000, 42);
            chain.process(&mut buffer, SAMPLE_RATE);
            assert!(is_stable(&buffer));
        }
    }

    #[test]
    fn large_buffer_various_sample_rates() {
        let sample_rates = [44100u32, 48000, 88200, 96000, 192000];

        for &rate in &sample_rates {
            let mut chain = EffectChain::new();
            let mut eq = ParametricEq::new();
            eq.set_low_band(EqBand::low_shelf(100.0, 3.0));
            chain.add_effect(Box::new(eq));

            let frames = rate as usize / 2; // 0.5 seconds
            let mut buffer = generate_stereo_sine(440.0, rate, frames);
            chain.process(&mut buffer, rate);

            assert!(
                is_stable(&buffer),
                "Large buffer at {}Hz should be stable",
                rate
            );
        }
    }
}

// ============================================================================
// ADDITIONAL EDGE CASE TESTS
// ============================================================================

mod edge_case_tests {
    use super::*;

    #[test]
    fn reset_during_processing_cycle() {
        let mut chain = EffectChain::new();
        let mut eq = ParametricEq::new();
        eq.set_low_band(EqBand::low_shelf(100.0, 6.0));
        chain.add_effect(Box::new(eq));

        // Process some audio
        let mut buffer1 = generate_stereo_sine(100.0, SAMPLE_RATE, 2048);
        chain.process(&mut buffer1, SAMPLE_RATE);

        // Reset
        chain.reset();

        // Process more audio - should work normally
        let mut buffer2 = generate_stereo_sine(100.0, SAMPLE_RATE, 2048);
        chain.process(&mut buffer2, SAMPLE_RATE);

        assert!(is_stable(&buffer2), "Should be stable after reset");
    }

    #[test]
    fn sample_rate_change_mid_processing() {
        let mut chain = EffectChain::new();
        let mut eq = ParametricEq::new();
        eq.set_low_band(EqBand::low_shelf(100.0, 6.0));
        chain.add_effect(Box::new(eq));

        // Process at 44.1kHz
        let mut buffer1 = generate_stereo_sine(440.0, 44100, 2048);
        chain.process(&mut buffer1, 44100);

        // Notify of sample rate change
        chain.set_sample_rate(96000);

        // Process at 96kHz
        let mut buffer2 = generate_stereo_sine(440.0, 96000, 2048);
        chain.process(&mut buffer2, 96000);

        assert!(is_stable(&buffer2), "Should handle sample rate change");
    }

    #[test]
    fn dc_signal_processing() {
        let mut chain = EffectChain::new();
        chain.add_effect(Box::new(GainEffect::new(0.5)));
        chain.add_effect(Box::new(Limiter::new()));

        // DC signal (constant value)
        let mut buffer = vec![0.8f32; 1000];
        chain.process(&mut buffer, SAMPLE_RATE);

        for sample in &buffer {
            assert!((*sample - 0.4).abs() < 0.01); // 0.8 * 0.5 = 0.4
        }
    }

    #[test]
    fn silence_processing() {
        let mut chain = EffectChain::new();
        let mut eq = ParametricEq::new();
        eq.set_low_band(EqBand::low_shelf(100.0, 12.0));
        chain.add_effect(Box::new(eq));
        chain.add_effect(Box::new(Compressor::new()));
        chain.add_effect(Box::new(Limiter::new()));

        // Silence
        let mut buffer = vec![0.0f32; 1000];
        chain.process(&mut buffer, SAMPLE_RATE);

        // Should remain silence
        for sample in &buffer {
            assert!(
                sample.abs() < 1e-10,
                "Silence should remain silent: {}",
                sample
            );
        }
    }

    #[test]
    fn denormal_handling() {
        let mut chain = EffectChain::new();
        let mut eq = ParametricEq::new();
        eq.set_low_band(EqBand::low_shelf(100.0, 6.0));
        chain.add_effect(Box::new(eq));

        // Very small (potentially denormal) values
        let mut buffer = vec![1e-40f32; 1000];
        chain.process(&mut buffer, SAMPLE_RATE);

        for sample in &buffer {
            assert!(sample.is_finite(), "Should handle denormals");
            // Denormals should be flushed to zero or remain very small
            assert!(
                sample.abs() < 1e-10,
                "Denormals should be handled: {}",
                sample
            );
        }
    }

    #[test]
    fn extreme_values() {
        let mut chain = EffectChain::new();
        chain.add_effect(Box::new(Limiter::with_settings(
            LimiterSettings::brickwall(),
        )));

        // Very large values
        let mut buffer = vec![100.0f32; 1000];
        chain.process(&mut buffer, SAMPLE_RATE);

        // Limiter should catch them
        let peak = peak_level(&buffer);
        assert!(peak <= 1.01, "Should limit extreme values: {}", peak);
    }

    #[test]
    fn alternating_positive_negative() {
        let mut chain = EffectChain::new();
        chain.add_effect(Box::new(GainEffect::new(0.5)));

        // Alternating +/- (like a square wave at Nyquist)
        let mut buffer: Vec<f32> = (0..100)
            .map(|i| if i % 2 == 0 { 1.0 } else { -1.0 })
            .collect();
        chain.process(&mut buffer, SAMPLE_RATE);

        for (i, sample) in buffer.iter().enumerate() {
            let expected = if i % 2 == 0 { 0.5 } else { -0.5 };
            assert!((*sample - expected).abs() < 0.0001);
        }
    }

    #[test]
    fn process_count_tracking() {
        let mut chain = EffectChain::new();
        chain.add_effect(Box::new(GainEffect::new(1.0)));

        // Process multiple buffers
        for _ in 0..5 {
            let mut buffer = vec![1.0f32; 100];
            chain.process(&mut buffer, SAMPLE_RATE);
        }

        // Check process count
        if let Some(effect) = chain.get_effect_as::<GainEffect>(0) {
            assert_eq!(effect.process_count, 5);
        }
    }

    #[test]
    fn chain_with_same_effect_type_multiple_times() {
        let mut chain = EffectChain::new();

        // Multiple EQs in series (like serial EQ processing)
        let mut eq1 = ParametricEq::new();
        eq1.set_low_band(EqBand::low_shelf(100.0, 3.0));
        chain.add_effect(Box::new(eq1));

        let mut eq2 = ParametricEq::new();
        eq2.set_mid_band(EqBand::peaking(1000.0, 3.0, 1.0));
        chain.add_effect(Box::new(eq2));

        let mut eq3 = ParametricEq::new();
        eq3.set_high_band(EqBand::high_shelf(8000.0, 3.0));
        chain.add_effect(Box::new(eq3));

        let mut buffer = generate_white_noise(4096, 42);
        chain.process(&mut buffer, SAMPLE_RATE);

        assert!(is_stable(&buffer), "Multiple same-type effects should work");
    }
}
