//! Bit-perfect verification tests
//!
//! Tests to verify that:
//! - Bypass modes are truly bit-perfect (not just "close")
//! - Disabled effects don't modify the signal
//! - Passthrough configurations preserve exact values

use soul_audio::effects::{
    AudioEffect, Compressor, Crossfeed, EqBand, GraphicEq, GraphicEqPreset, Limiter, ParametricEq,
    StereoEnhancer, StereoSettings,
};
use std::f32::consts::PI;

const SAMPLE_RATE: u32 = 44100;

/// Generate a deterministic test pattern
fn generate_test_pattern(num_samples: usize) -> Vec<f32> {
    let mut buffer = Vec::with_capacity(num_samples * 2);
    for i in 0..num_samples {
        // Use a complex pattern that's unlikely to accidentally match
        let t = i as f32 / SAMPLE_RATE as f32;
        let left = 0.3 * (2.0 * PI * 440.0 * t).sin()
            + 0.2 * (2.0 * PI * 880.0 * t).sin()
            + 0.1 * (2.0 * PI * 1760.0 * t).sin();
        let right = 0.3 * (2.0 * PI * 440.0 * t + 0.5).sin()
            + 0.2 * (2.0 * PI * 880.0 * t + 0.3).sin()
            + 0.1 * (2.0 * PI * 1760.0 * t + 0.1).sin();
        buffer.push(left);
        buffer.push(right);
    }
    buffer
}

/// Generate a simple ramp pattern for easy verification
fn generate_ramp_pattern(num_samples: usize) -> Vec<f32> {
    let mut buffer = Vec::with_capacity(num_samples * 2);
    for i in 0..num_samples {
        let value = (i as f32 / num_samples as f32) * 2.0 - 1.0; // -1 to 1
        buffer.push(value);
        buffer.push(-value); // Opposite for right channel
    }
    buffer
}

/// Compare two buffers for exact equality
fn buffers_identical(a: &[f32], b: &[f32]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter()
        .zip(b.iter())
        .all(|(x, y)| x.to_bits() == y.to_bits())
}

/// Compare two buffers for near-equality (within epsilon)
fn buffers_nearly_equal(a: &[f32], b: &[f32], epsilon: f32) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b.iter()).all(|(x, y)| (x - y).abs() < epsilon)
}

/// Find maximum difference between two buffers
fn max_difference(a: &[f32], b: &[f32]) -> f32 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y).abs())
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or(0.0)
}

// ============================================================================
// PARAMETRIC EQ BIT-PERFECT TESTS
// ============================================================================

#[test]
fn test_eq_disabled_is_bit_perfect() {
    let mut eq = ParametricEq::new();
    eq.set_low_band(EqBand::low_shelf(100.0, 6.0));
    eq.set_mid_band(EqBand::peaking(1000.0, 6.0, 1.0));
    eq.set_high_band(EqBand::high_shelf(10000.0, 6.0));
    eq.set_enabled(false);

    let original = generate_test_pattern(4096);
    let mut buffer = original.clone();

    eq.process(&mut buffer, SAMPLE_RATE);

    assert!(
        buffers_identical(&buffer, &original),
        "Disabled EQ should be bit-perfect. Max diff: {}",
        max_difference(&buffer, &original)
    );
}

#[test]
fn test_eq_zero_gain_nearly_transparent() {
    let mut eq = ParametricEq::new();
    eq.set_low_band(EqBand::low_shelf(100.0, 0.0));
    eq.set_mid_band(EqBand::peaking(1000.0, 0.0, 1.0));
    eq.set_high_band(EqBand::high_shelf(10000.0, 0.0));
    eq.set_enabled(true);

    let original = generate_test_pattern(4096);
    let mut buffer = original.clone();

    eq.process(&mut buffer, SAMPLE_RATE);

    // Zero gain should be very close to transparent
    // (may have tiny differences due to filter implementation)
    assert!(
        buffers_nearly_equal(&buffer, &original, 0.001),
        "Zero-gain EQ should be nearly transparent. Max diff: {}",
        max_difference(&buffer, &original)
    );
}

// ============================================================================
// GRAPHIC EQ BIT-PERFECT TESTS
// ============================================================================

#[test]
fn test_graphic_eq_disabled_is_bit_perfect() {
    let mut geq = GraphicEq::new_10_band();
    geq.set_preset(GraphicEqPreset::BassBoost);
    geq.set_enabled(false);

    let original = generate_test_pattern(4096);
    let mut buffer = original.clone();

    geq.process(&mut buffer, SAMPLE_RATE);

    assert!(
        buffers_identical(&buffer, &original),
        "Disabled Graphic EQ should be bit-perfect"
    );
}

#[test]
fn test_graphic_eq_flat_preset_nearly_transparent() {
    let mut geq = GraphicEq::new_10_band();
    geq.set_preset(GraphicEqPreset::Flat);
    geq.set_enabled(true);

    let original = generate_test_pattern(4096);
    let mut buffer = original.clone();

    geq.process(&mut buffer, SAMPLE_RATE);

    // Flat preset should be very close to transparent
    let max_diff = max_difference(&buffer, &original);
    assert!(
        max_diff < 0.01,
        "Flat Graphic EQ should be nearly transparent. Max diff: {}",
        max_diff
    );
}

// ============================================================================
// STEREO ENHANCER BIT-PERFECT TESTS
// ============================================================================

#[test]
fn test_stereo_enhancer_disabled_is_bit_perfect() {
    let mut enhancer = StereoEnhancer::with_settings(StereoSettings::extra_wide());
    enhancer.set_enabled(false);

    let original = generate_test_pattern(4096);
    let mut buffer = original.clone();

    enhancer.process(&mut buffer, SAMPLE_RATE);

    assert!(
        buffers_identical(&buffer, &original),
        "Disabled Stereo Enhancer should be bit-perfect"
    );
}

#[test]
fn test_stereo_enhancer_neutral_settings_is_transparent() {
    let mut enhancer = StereoEnhancer::new();
    // Default/neutral settings: width=1.0, mid_gain=0, side_gain=0, balance=0
    enhancer.set_enabled(true);

    let original = generate_test_pattern(4096);
    let mut buffer = original.clone();

    enhancer.process(&mut buffer, SAMPLE_RATE);

    // Neutral settings should be transparent or very close
    let max_diff = max_difference(&buffer, &original);
    assert!(
        max_diff < 0.0001,
        "Neutral Stereo Enhancer should be transparent. Max diff: {}",
        max_diff
    );
}

// ============================================================================
// CROSSFEED BIT-PERFECT TESTS
// ============================================================================

#[test]
fn test_crossfeed_disabled_is_bit_perfect() {
    let mut crossfeed = Crossfeed::new();
    crossfeed.set_enabled(false);

    let original = generate_test_pattern(4096);
    let mut buffer = original.clone();

    crossfeed.process(&mut buffer, SAMPLE_RATE);

    assert!(
        buffers_identical(&buffer, &original),
        "Disabled Crossfeed should be bit-perfect"
    );
}

// ============================================================================
// COMPRESSOR BIT-PERFECT TESTS
// ============================================================================

#[test]
fn test_compressor_disabled_is_bit_perfect() {
    let mut compressor = Compressor::new();
    compressor.set_enabled(false);

    let original = generate_test_pattern(4096);
    let mut buffer = original.clone();

    compressor.process(&mut buffer, SAMPLE_RATE);

    assert!(
        buffers_identical(&buffer, &original),
        "Disabled Compressor should be bit-perfect"
    );
}

#[test]
fn test_compressor_below_threshold_is_transparent() {
    let mut compressor = Compressor::new();
    compressor.set_threshold(0.0); // 0 dB threshold
    compressor.set_enabled(true);

    // Very quiet signal that won't trigger compression
    let mut buffer: Vec<f32> = generate_test_pattern(4096)
        .iter()
        .map(|s| s * 0.001) // -60 dB
        .collect();
    let original = buffer.clone();

    compressor.process(&mut buffer, SAMPLE_RATE);

    // Signal below threshold should be mostly unchanged
    // (may have tiny differences due to envelope detection)
    let max_diff = max_difference(&buffer, &original);
    assert!(
        max_diff < 0.0001,
        "Compressor below threshold should be transparent. Max diff: {}",
        max_diff
    );
}

#[test]
fn test_compressor_ratio_one_is_transparent() {
    let mut compressor = Compressor::new();
    compressor.set_ratio(1.0); // 1:1 ratio = no compression
    compressor.set_enabled(true);

    let original = generate_test_pattern(4096);
    let mut buffer = original.clone();

    compressor.process(&mut buffer, SAMPLE_RATE);

    // 1:1 ratio should be transparent
    let max_diff = max_difference(&buffer, &original);
    assert!(
        max_diff < 0.01,
        "Compressor with 1:1 ratio should be transparent. Max diff: {}",
        max_diff
    );
}

// ============================================================================
// LIMITER BIT-PERFECT TESTS
// ============================================================================

#[test]
fn test_limiter_disabled_is_bit_perfect() {
    let mut limiter = Limiter::new();
    limiter.set_enabled(false);

    let original = generate_test_pattern(4096);
    let mut buffer = original.clone();

    limiter.process(&mut buffer, SAMPLE_RATE);

    assert!(
        buffers_identical(&buffer, &original),
        "Disabled Limiter should be bit-perfect"
    );
}

#[test]
fn test_limiter_below_ceiling_is_transparent() {
    let mut limiter = Limiter::new();
    limiter.set_threshold(0.0); // 0 dB ceiling
    limiter.set_enabled(true);

    // Signal that peaks at 0.5 (-6 dB), well below 0 dB ceiling
    let mut buffer: Vec<f32> = generate_test_pattern(4096)
        .iter()
        .map(|s| s * 0.5)
        .collect();
    let original = buffer.clone();

    limiter.process(&mut buffer, SAMPLE_RATE);

    // Signal below ceiling should be mostly unchanged
    let max_diff = max_difference(&buffer, &original);
    assert!(
        max_diff < 0.01,
        "Limiter below ceiling should be transparent. Max diff: {}",
        max_diff
    );
}

// ============================================================================
// EFFECT CHAIN BIT-PERFECT TESTS
// ============================================================================

#[test]
fn test_all_effects_disabled_is_bit_perfect() {
    let mut eq = ParametricEq::new();
    eq.set_low_band(EqBand::low_shelf(100.0, 6.0));
    eq.set_enabled(false);

    let mut geq = GraphicEq::new_10_band();
    geq.set_preset(GraphicEqPreset::Rock);
    geq.set_enabled(false);

    let mut stereo = StereoEnhancer::with_settings(StereoSettings::extra_wide());
    stereo.set_enabled(false);

    let mut crossfeed = Crossfeed::new();
    crossfeed.set_enabled(false);

    let mut compressor = Compressor::new();
    compressor.set_enabled(false);

    let mut limiter = Limiter::new();
    limiter.set_enabled(false);

    let original = generate_test_pattern(4096);
    let mut buffer = original.clone();

    // Process through all disabled effects
    eq.process(&mut buffer, SAMPLE_RATE);
    geq.process(&mut buffer, SAMPLE_RATE);
    stereo.process(&mut buffer, SAMPLE_RATE);
    crossfeed.process(&mut buffer, SAMPLE_RATE);
    compressor.process(&mut buffer, SAMPLE_RATE);
    limiter.process(&mut buffer, SAMPLE_RATE);

    assert!(
        buffers_identical(&buffer, &original),
        "All disabled effects should be bit-perfect"
    );
}

// ============================================================================
// SAMPLE VALUE PRESERVATION TESTS
// ============================================================================

#[test]
fn test_silence_preserved() {
    let mut eq = ParametricEq::new();
    eq.set_mid_band(EqBand::peaking(1000.0, 6.0, 1.0));
    eq.set_enabled(false);

    let original = vec![0.0f32; 8192];
    let mut buffer = original.clone();

    eq.process(&mut buffer, SAMPLE_RATE);

    // Silence should remain exactly silent when bypassed
    for sample in &buffer {
        assert_eq!(*sample, 0.0, "Silence should be preserved exactly");
    }
}

#[test]
fn test_dc_offset_preserved_when_disabled() {
    let mut eq = ParametricEq::new();
    eq.set_low_band(EqBand::low_shelf(100.0, 6.0));
    eq.set_enabled(false);

    let dc_value = 0.5f32;
    let original = vec![dc_value; 8192];
    let mut buffer = original.clone();

    eq.process(&mut buffer, SAMPLE_RATE);

    // DC should be preserved exactly when bypassed
    for sample in &buffer {
        assert_eq!(
            *sample, dc_value,
            "DC should be preserved exactly when disabled"
        );
    }
}

#[test]
fn test_full_scale_preserved_when_disabled() {
    let mut limiter = Limiter::new();
    limiter.set_enabled(false);

    // Alternating +1 and -1
    let mut original = Vec::with_capacity(8192);
    for i in 0..4096 {
        original.push(if i % 2 == 0 { 1.0 } else { -1.0 });
        original.push(if i % 2 == 0 { -1.0 } else { 1.0 });
    }
    let mut buffer = original.clone();

    limiter.process(&mut buffer, SAMPLE_RATE);

    assert!(
        buffers_identical(&buffer, &original),
        "Full-scale should be preserved when disabled"
    );
}

// ============================================================================
// SPECIFIC VALUE TESTS
// ============================================================================

#[test]
fn test_specific_values_preserved() {
    let mut eq = ParametricEq::new();
    eq.set_enabled(false);

    // Test specific edge-case values
    let test_values: Vec<f32> = vec![
        0.0,
        1.0,
        -1.0,
        0.5,
        -0.5,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        0.99999,
        -0.99999,
        0.00001,
        -0.00001,
    ];

    for &value in &test_values {
        let mut buffer = vec![value, value]; // Stereo pair
        let original = buffer.clone();

        eq.process(&mut buffer, SAMPLE_RATE);

        assert!(
            buffer[0].to_bits() == original[0].to_bits()
                && buffer[1].to_bits() == original[1].to_bits(),
            "Value {} should be preserved exactly",
            value
        );
    }
}

#[test]
fn test_ramp_pattern_preserved_when_disabled() {
    let mut eq = ParametricEq::new();
    eq.set_low_band(EqBand::peaking(1000.0, 12.0, 0.5));
    eq.set_enabled(false);

    let original = generate_ramp_pattern(4096);
    let mut buffer = original.clone();

    eq.process(&mut buffer, SAMPLE_RATE);

    assert!(
        buffers_identical(&buffer, &original),
        "Ramp pattern should be preserved exactly when disabled"
    );
}

// ============================================================================
// DOUBLE PROCESSING TESTS
// ============================================================================

#[test]
fn test_double_processing_disabled_still_bit_perfect() {
    let mut eq = ParametricEq::new();
    eq.set_mid_band(EqBand::peaking(1000.0, 6.0, 1.0));
    eq.set_enabled(false);

    let original = generate_test_pattern(4096);
    let mut buffer = original.clone();

    // Process twice
    eq.process(&mut buffer, SAMPLE_RATE);
    eq.process(&mut buffer, SAMPLE_RATE);

    assert!(
        buffers_identical(&buffer, &original),
        "Double processing while disabled should still be bit-perfect"
    );
}

#[test]
fn test_enable_disable_returns_to_bit_perfect() {
    let mut eq = ParametricEq::new();
    eq.set_mid_band(EqBand::peaking(1000.0, 6.0, 1.0));

    let original = generate_test_pattern(4096);

    // Enable and process
    eq.set_enabled(true);
    let mut buffer1 = original.clone();
    eq.process(&mut buffer1, SAMPLE_RATE);

    // Buffer1 is now modified

    // Disable and process new buffer
    eq.set_enabled(false);
    eq.reset(); // Reset filter state

    let mut buffer2 = original.clone();
    eq.process(&mut buffer2, SAMPLE_RATE);

    // buffer2 should be identical to original
    assert!(
        buffers_identical(&buffer2, &original),
        "After disable, processing should be bit-perfect"
    );
}
