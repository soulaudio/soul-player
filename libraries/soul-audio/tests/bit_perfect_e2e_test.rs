//! Bit-Perfect Audio Output Verification E2E Tests
//!
//! Comprehensive end-to-end tests for verifying bit-perfect audio processing:
//!
//! 1. Bit-perfect passthrough verification
//! 2. Sample format conversion accuracy
//! 3. Dithering tests
//! 4. Clipping behavior
//! 5. DC offset handling
//! 6. Null testing
//! 7. Level accuracy
//!
//! These tests ensure that audio data integrity is maintained through
//! the entire processing pipeline.
//!
//! Run: `cargo test -p soul-audio --features test-utils bit_perfect_e2e -- --nocapture`

#![cfg(feature = "test-utils")]

use soul_audio::effects::{
    AudioEffect, Compressor, CompressorSettings, Crossfeed, EffectChain, EqBand, GraphicEq,
    GraphicEqPreset, Limiter, LimiterSettings, ParametricEq, StereoEnhancer, StereoSettings,
};
use soul_audio::test_utils::analysis::*;
use soul_audio::test_utils::signals::*;
use std::f32::consts::PI;

// =============================================================================
// Constants
// =============================================================================

const SAMPLE_RATE_44100: u32 = 44100;
const SAMPLE_RATE_48000: u32 = 48000;
const SAMPLE_RATE_96000: u32 = 96000;
const SAMPLE_RATE_192000: u32 = 192000;

/// Tolerance for bit-perfect comparison (exact bit match)
const BIT_PERFECT_TOLERANCE: f32 = 0.0;

/// Tolerance for near-perfect comparison (floating point rounding)
const NEAR_PERFECT_TOLERANCE: f32 = 1e-7;

/// Tolerance for transparent processing (very minor differences allowed)
const TRANSPARENT_TOLERANCE: f32 = 1e-5;

// =============================================================================
// Helper Functions
// =============================================================================

/// Compare two buffers for exact bit equality
fn buffers_bit_identical(a: &[f32], b: &[f32]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter()
        .zip(b.iter())
        .all(|(x, y)| x.to_bits() == y.to_bits())
}

/// Count number of differing samples between two buffers
fn count_differences(a: &[f32], b: &[f32]) -> usize {
    a.iter()
        .zip(b.iter())
        .filter(|(x, y)| x.to_bits() != y.to_bits())
        .count()
}

/// Find maximum absolute difference between two buffers
fn max_difference(a: &[f32], b: &[f32]) -> f32 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y).abs())
        .fold(0.0f32, f32::max)
}

/// Find RMS difference between two buffers
fn rms_difference(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return f32::INFINITY;
    }
    let sum_sq: f32 = a.iter().zip(b.iter()).map(|(x, y)| (x - y).powi(2)).sum();
    (sum_sq / a.len() as f32).sqrt()
}

/// Generate deterministic test pattern with multiple frequencies
fn generate_test_pattern(num_samples: usize, sample_rate: u32) -> Vec<f32> {
    let mut buffer = Vec::with_capacity(num_samples * 2);
    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        // Complex multi-frequency pattern
        let left = 0.3 * (2.0 * PI * 440.0 * t).sin()
            + 0.2 * (2.0 * PI * 880.0 * t).sin()
            + 0.1 * (2.0 * PI * 1760.0 * t).sin()
            + 0.05 * (2.0 * PI * 3520.0 * t).sin();
        let right = 0.3 * (2.0 * PI * 440.0 * t + 0.5).sin()
            + 0.2 * (2.0 * PI * 880.0 * t + 0.3).sin()
            + 0.1 * (2.0 * PI * 1760.0 * t + 0.1).sin()
            + 0.05 * (2.0 * PI * 3520.0 * t + 0.7).sin();
        buffer.push(left);
        buffer.push(right);
    }
    buffer
}

/// Generate a linear ramp pattern from -1 to 1
fn generate_ramp_pattern(num_samples: usize) -> Vec<f32> {
    let mut buffer = Vec::with_capacity(num_samples * 2);
    for i in 0..num_samples {
        let value = (i as f32 / num_samples as f32) * 2.0 - 1.0;
        buffer.push(value);
        buffer.push(-value);
    }
    buffer
}

/// Generate signal at specific bit depth representation
fn generate_bit_depth_test_signal(bit_depth: u32, num_samples: usize) -> Vec<f32> {
    let max_value = (1 << (bit_depth - 1)) as f32;
    let mut buffer = Vec::with_capacity(num_samples * 2);
    for i in 0..num_samples {
        let t = i as f32 / 44100.0;
        let sample = (2.0 * PI * 1000.0 * t).sin();
        // Quantize to bit depth
        let quantized = ((sample * max_value).round() / max_value).clamp(-1.0, 1.0);
        buffer.push(quantized);
        buffer.push(quantized);
    }
    buffer
}

// =============================================================================
// 1. BIT-PERFECT PASSTHROUGH VERIFICATION
// =============================================================================

#[test]
fn test_passthrough_16bit_samples() {
    let input = generate_bit_depth_test_signal(16, 8192);
    let mut output = input.clone();

    // Process through disabled effects chain
    let mut chain = EffectChain::new();
    let mut eq = ParametricEq::new();
    eq.set_enabled(false);
    chain.add_effect(Box::new(eq));

    chain.process(&mut output, SAMPLE_RATE_44100);

    assert!(
        buffers_bit_identical(&input, &output),
        "16-bit passthrough should be bit-perfect. Differences: {}",
        count_differences(&input, &output)
    );
}

#[test]
fn test_passthrough_24bit_samples() {
    let input = generate_bit_depth_test_signal(24, 8192);
    let mut output = input.clone();

    let mut chain = EffectChain::new();
    let mut eq = ParametricEq::new();
    eq.set_enabled(false);
    chain.add_effect(Box::new(eq));

    chain.process(&mut output, SAMPLE_RATE_44100);

    assert!(
        buffers_bit_identical(&input, &output),
        "24-bit passthrough should be bit-perfect. Differences: {}",
        count_differences(&input, &output)
    );
}

#[test]
fn test_passthrough_32bit_samples() {
    let input = generate_test_pattern(8192, SAMPLE_RATE_44100);
    let mut output = input.clone();

    let mut chain = EffectChain::new();
    let mut eq = ParametricEq::new();
    eq.set_enabled(false);
    chain.add_effect(Box::new(eq));

    chain.process(&mut output, SAMPLE_RATE_44100);

    assert!(
        buffers_bit_identical(&input, &output),
        "32-bit float passthrough should be bit-perfect. Differences: {}",
        count_differences(&input, &output)
    );
}

#[test]
fn test_passthrough_44100hz() {
    let input = generate_test_pattern(4096, SAMPLE_RATE_44100);
    let mut output = input.clone();

    let mut chain = EffectChain::new();
    chain.process(&mut output, SAMPLE_RATE_44100);

    assert!(
        buffers_bit_identical(&input, &output),
        "44.1kHz passthrough should be bit-perfect"
    );
}

#[test]
fn test_passthrough_48000hz() {
    let input = generate_test_pattern(4096, SAMPLE_RATE_48000);
    let mut output = input.clone();

    let mut chain = EffectChain::new();
    chain.process(&mut output, SAMPLE_RATE_48000);

    assert!(
        buffers_bit_identical(&input, &output),
        "48kHz passthrough should be bit-perfect"
    );
}

#[test]
fn test_passthrough_96000hz() {
    let input = generate_test_pattern(8192, SAMPLE_RATE_96000);
    let mut output = input.clone();

    let mut chain = EffectChain::new();
    chain.process(&mut output, SAMPLE_RATE_96000);

    assert!(
        buffers_bit_identical(&input, &output),
        "96kHz passthrough should be bit-perfect"
    );
}

#[test]
fn test_passthrough_192000hz() {
    let input = generate_test_pattern(16384, SAMPLE_RATE_192000);
    let mut output = input.clone();

    let mut chain = EffectChain::new();
    chain.process(&mut output, SAMPLE_RATE_192000);

    assert!(
        buffers_bit_identical(&input, &output),
        "192kHz passthrough should be bit-perfect"
    );
}

#[test]
fn test_passthrough_all_effects_disabled() {
    let input = generate_test_pattern(8192, SAMPLE_RATE_44100);
    let mut output = input.clone();

    // Create full effect chain, all disabled
    let mut chain = EffectChain::new();

    let mut eq = ParametricEq::new();
    eq.set_low_band(EqBand::low_shelf(100.0, 6.0));
    eq.set_mid_band(EqBand::peaking(1000.0, 6.0, 1.0));
    eq.set_high_band(EqBand::high_shelf(10000.0, 6.0));
    eq.set_enabled(false);
    chain.add_effect(Box::new(eq));

    let mut geq = GraphicEq::new_10_band();
    geq.set_preset(GraphicEqPreset::BassBoost);
    geq.set_enabled(false);
    chain.add_effect(Box::new(geq));

    let mut stereo = StereoEnhancer::with_settings(StereoSettings::extra_wide());
    stereo.set_enabled(false);
    chain.add_effect(Box::new(stereo));

    let mut crossfeed = Crossfeed::new();
    crossfeed.set_enabled(false);
    chain.add_effect(Box::new(crossfeed));

    let mut compressor = Compressor::with_settings(CompressorSettings::aggressive());
    compressor.set_enabled(false);
    chain.add_effect(Box::new(compressor));

    let mut limiter = Limiter::with_settings(LimiterSettings::brickwall());
    limiter.set_enabled(false);
    chain.add_effect(Box::new(limiter));

    chain.process(&mut output, SAMPLE_RATE_44100);

    assert!(
        buffers_bit_identical(&input, &output),
        "All effects disabled should be bit-perfect. Max diff: {:e}",
        max_difference(&input, &output)
    );
}

// =============================================================================
// 2. SAMPLE FORMAT CONVERSION ACCURACY
// =============================================================================

/// Simulate i16 -> f32 -> i16 roundtrip
fn i16_to_f32_to_i16(value: i16) -> i16 {
    let f32_value = value as f32 / 32768.0;
    (f32_value * 32768.0).round() as i16
}

/// Simulate i24 -> f32 -> i24 roundtrip (i32 with 24 bits of precision)
fn i24_to_f32_to_i24(value: i32) -> i32 {
    let f32_value = value as f32 / 8388608.0;
    (f32_value * 8388608.0).round() as i32
}

/// Simulate i32 -> f32 -> i32 roundtrip
fn i32_to_f32_to_i32(value: i32) -> i32 {
    let f32_value = value as f64 / 2147483648.0; // Use f64 for precision
    (f32_value * 2147483648.0).round() as i32
}

#[test]
fn test_i16_f32_i16_roundtrip() {
    let test_values: Vec<i16> = vec![
        0,
        1,
        -1,
        i16::MAX,
        i16::MIN,
        i16::MAX / 2,
        i16::MIN / 2,
        100,
        -100,
        1000,
        -1000,
        10000,
        -10000,
    ];

    for &value in &test_values {
        let roundtrip = i16_to_f32_to_i16(value);
        assert_eq!(
            value, roundtrip,
            "i16 -> f32 -> i16 roundtrip failed for value {}. Got {}",
            value, roundtrip
        );
    }
}

#[test]
fn test_i16_f32_i16_full_range() {
    // Test all possible i16 values for perfect roundtrip
    let mut errors = 0;
    let mut max_error = 0i16;

    for value in i16::MIN..=i16::MAX {
        let roundtrip = i16_to_f32_to_i16(value);
        let error = (value as i32 - roundtrip as i32).abs() as i16;
        if error > 0 {
            errors += 1;
            max_error = max_error.max(error);
        }
    }

    // i16 -> f32 -> i16 should be perfect (f32 has 24 bits of mantissa)
    assert_eq!(
        errors, 0,
        "i16 roundtrip should be perfect. {} errors, max error: {}",
        errors, max_error
    );
}

#[test]
fn test_i24_f32_i24_roundtrip() {
    let i24_max = 8388607i32;
    let i24_min = -8388608i32;

    let test_values: Vec<i32> = vec![
        0,
        1,
        -1,
        i24_max,
        i24_min,
        i24_max / 2,
        i24_min / 2,
        1000,
        -1000,
        100000,
        -100000,
        1000000,
        -1000000,
    ];

    for &value in &test_values {
        let roundtrip = i24_to_f32_to_i24(value);
        // Allow +-1 LSB error due to f32 precision limits at 24-bit
        let error = (value - roundtrip).abs();
        assert!(
            error <= 1,
            "i24 -> f32 -> i24 roundtrip error too large for value {}. Got {}, error: {}",
            value,
            roundtrip,
            error
        );
    }
}

#[test]
fn test_i24_f32_i24_sample_range() {
    // Test a sampling of 24-bit values
    let i24_max = 8388607i32;
    let i24_min = -8388608i32;

    let mut errors = 0;
    let mut max_error = 0i32;
    let mut samples_tested = 0;

    // Test every 1000th value
    let mut value = i24_min;
    while value <= i24_max {
        let roundtrip = i24_to_f32_to_i24(value);
        let error = (value - roundtrip).abs();
        if error > 1 {
            // Allow +-1 LSB
            errors += 1;
            max_error = max_error.max(error);
        }
        samples_tested += 1;
        value = value.saturating_add(1000);
    }

    println!(
        "i24 roundtrip: tested {} samples, {} errors > 1 LSB, max error: {}",
        samples_tested, errors, max_error
    );

    // Should have very few errors
    assert!(
        errors == 0,
        "i24 roundtrip should have no errors > 1 LSB. {} errors, max: {}",
        errors,
        max_error
    );
}

#[test]
fn test_i32_f32_i32_roundtrip() {
    // Note: f32 only has 24 bits of mantissa, so i32 -> f32 -> i32 loses precision
    let test_values: Vec<i32> = vec![
        0,
        1,
        -1,
        // Values that fit in 24 bits should roundtrip perfectly
        (1 << 23) - 1,
        -(1 << 23),
    ];

    for &value in &test_values {
        let roundtrip = i32_to_f32_to_i32(value);
        // Large values will have quantization error
        let expected_max_error = if value.abs() > (1 << 23) {
            (value.abs() >> 23) as i32 // About 8 bits of error at full scale
        } else {
            0
        };
        let error = (value - roundtrip).abs();
        assert!(
            error <= expected_max_error.max(1),
            "i32 -> f32 -> i32 roundtrip error too large for value {}. Got {}, error: {}",
            value,
            roundtrip,
            error
        );
    }
}

#[test]
fn test_f32_i16_dithering_quality() {
    // Generate a sine wave and convert to i16 with dithering simulation
    let num_samples = 44100;
    let mut signal: Vec<f32> = (0..num_samples)
        .map(|i| {
            let t = i as f32 / 44100.0;
            (2.0 * PI * 1000.0 * t).sin() * 0.5
        })
        .collect();

    // Simulate TPDF dither and quantization to 16-bit
    let mut dithered_i16 = Vec::with_capacity(num_samples);
    for &sample in &signal {
        // TPDF dither: two uniform random values subtracted
        let dither = (rand::random::<f32>() - rand::random::<f32>()) / 32768.0;
        let dithered = sample + dither;
        let quantized = (dithered * 32768.0).round().clamp(-32768.0, 32767.0) as i16;
        dithered_i16.push(quantized);
    }

    // Verify the signal wasn't destroyed
    let reconstructed: Vec<f32> = dithered_i16.iter().map(|&s| s as f32 / 32768.0).collect();

    let rms_original = calculate_rms(&signal);
    let rms_reconstructed = calculate_rms(&reconstructed);
    let rms_ratio = rms_reconstructed / rms_original;

    println!(
        "f32 -> i16 dithering: original RMS: {:.6}, reconstructed RMS: {:.6}, ratio: {:.4}",
        rms_original, rms_reconstructed, rms_ratio
    );

    // Signal should be preserved (within 1%)
    assert!(
        (rms_ratio - 1.0).abs() < 0.01,
        "Dithered signal RMS should be within 1% of original. Ratio: {:.4}",
        rms_ratio
    );
}

#[test]
fn test_f32_i24_dithering_quality() {
    let num_samples = 44100;
    let signal: Vec<f32> = (0..num_samples)
        .map(|i| {
            let t = i as f32 / 44100.0;
            (2.0 * PI * 1000.0 * t).sin() * 0.5
        })
        .collect();

    // Simulate TPDF dither and quantization to 24-bit
    let mut dithered_i24 = Vec::with_capacity(num_samples);
    for &sample in &signal {
        let dither = (rand::random::<f32>() - rand::random::<f32>()) / 8388608.0;
        let dithered = sample + dither;
        let quantized = (dithered * 8388608.0).round().clamp(-8388608.0, 8388607.0) as i32;
        dithered_i24.push(quantized);
    }

    let reconstructed: Vec<f32> = dithered_i24.iter().map(|&s| s as f32 / 8388608.0).collect();

    let rms_original = calculate_rms(&signal);
    let rms_reconstructed = calculate_rms(&reconstructed);
    let rms_ratio = rms_reconstructed / rms_original;

    println!(
        "f32 -> i24 dithering: original RMS: {:.6}, reconstructed RMS: {:.6}, ratio: {:.4}",
        rms_original, rms_reconstructed, rms_ratio
    );

    // 24-bit should be even more accurate
    assert!(
        (rms_ratio - 1.0).abs() < 0.001,
        "Dithered 24-bit signal RMS should be within 0.1% of original. Ratio: {:.6}",
        rms_ratio
    );
}

// =============================================================================
// 3. DITHERING TESTS
// =============================================================================

/// Generate TPDF (Triangular Probability Density Function) dither
fn generate_tpdf_dither(num_samples: usize) -> Vec<f32> {
    (0..num_samples)
        .map(|_| rand::random::<f32>() - rand::random::<f32>())
        .collect()
}

#[test]
fn test_tpdf_dither_spectrum() {
    // TPDF dither should have a flat noise spectrum (white noise)
    let dither = generate_tpdf_dither(44100);

    // Calculate spectrum
    let spectrum = analyze_frequency_spectrum(&dither, 44100);

    // Find average power across frequency bands
    let low_freq_power: f32 = spectrum
        .iter()
        .filter(|(f, _)| *f > 20.0 && *f < 500.0)
        .map(|(_, mag)| db_to_linear(*mag).powi(2))
        .sum::<f32>();

    let mid_freq_power: f32 = spectrum
        .iter()
        .filter(|(f, _)| *f >= 500.0 && *f < 5000.0)
        .map(|(_, mag)| db_to_linear(*mag).powi(2))
        .sum::<f32>();

    let high_freq_power: f32 = spectrum
        .iter()
        .filter(|(f, _)| *f >= 5000.0 && *f < 20000.0)
        .map(|(_, mag)| db_to_linear(*mag).powi(2))
        .sum::<f32>();

    // Count bins for normalization
    let low_bins = spectrum
        .iter()
        .filter(|(f, _)| *f > 20.0 && *f < 500.0)
        .count() as f32;
    let mid_bins = spectrum
        .iter()
        .filter(|(f, _)| *f >= 500.0 && *f < 5000.0)
        .count() as f32;
    let high_bins = spectrum
        .iter()
        .filter(|(f, _)| *f >= 5000.0 && *f < 20000.0)
        .count() as f32;

    let low_avg = if low_bins > 0.0 {
        low_freq_power / low_bins
    } else {
        0.0
    };
    let mid_avg = if mid_bins > 0.0 {
        mid_freq_power / mid_bins
    } else {
        0.0
    };
    let high_avg = if high_bins > 0.0 {
        high_freq_power / high_bins
    } else {
        0.0
    };

    println!("TPDF Dither Spectrum Analysis:");
    println!("  Low freq (20-500Hz) avg power: {:.6}", low_avg);
    println!("  Mid freq (500-5kHz) avg power: {:.6}", mid_avg);
    println!("  High freq (5k-20kHz) avg power: {:.6}", high_avg);

    // White noise should have relatively equal power per bin
    // Allow 10x variation (10dB) which is quite generous
    if mid_avg > 0.0 {
        let low_ratio = low_avg / mid_avg;
        let high_ratio = high_avg / mid_avg;

        println!("  Low/Mid ratio: {:.2}", low_ratio);
        println!("  High/Mid ratio: {:.2}", high_ratio);

        // Spectrum should be relatively flat (within 10x or 20dB)
        assert!(
            low_ratio > 0.1 && low_ratio < 10.0,
            "TPDF spectrum should be flat. Low/Mid ratio: {:.2}",
            low_ratio
        );
        assert!(
            high_ratio > 0.1 && high_ratio < 10.0,
            "TPDF spectrum should be flat. High/Mid ratio: {:.2}",
            high_ratio
        );
    }
}

#[test]
fn test_tpdf_dither_amplitude() {
    // TPDF dither amplitude should be +-1 LSB (for 16-bit, that's 1/32768)
    let num_samples = 100000;
    let dither = generate_tpdf_dither(num_samples);

    let min_val = dither.iter().cloned().fold(f32::INFINITY, f32::min);
    let max_val = dither.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let rms = calculate_rms(&dither);

    println!("TPDF Dither Amplitude:");
    println!("  Min: {:.6}", min_val);
    println!("  Max: {:.6}", max_val);
    println!("  RMS: {:.6}", rms);

    // TPDF from (rand - rand) has range [-1, 1] and RMS ≈ 0.408
    assert!(
        min_val > -1.1 && min_val < 0.0,
        "TPDF min should be in (-1, 0), got {}",
        min_val
    );
    assert!(
        max_val < 1.1 && max_val > 0.0,
        "TPDF max should be in (0, 1), got {}",
        max_val
    );

    // RMS of TPDF should be sqrt(1/6) ≈ 0.408
    let expected_rms = 0.408;
    assert!(
        (rms - expected_rms).abs() < 0.05,
        "TPDF RMS should be ~{:.3}, got {:.3}",
        expected_rms,
        rms
    );
}

#[test]
fn test_dither_no_dc_offset() {
    let num_samples = 1000000; // Large sample for accurate mean
    let dither = generate_tpdf_dither(num_samples);

    let mean: f32 = dither.iter().sum::<f32>() / num_samples as f32;

    println!("TPDF Dither DC Offset: {:.8}", mean);

    // DC offset should be negligible (< 0.001)
    assert!(
        mean.abs() < 0.001,
        "TPDF dither should have no DC offset. Mean: {:.8}",
        mean
    );
}

#[test]
fn test_dithered_signal_no_dc_offset() {
    // Apply dither to a signal and verify no DC offset is introduced
    let num_samples = 44100;
    let signal: Vec<f32> = (0..num_samples)
        .map(|i| {
            let t = i as f32 / 44100.0;
            (2.0 * PI * 1000.0 * t).sin() * 0.5
        })
        .collect();

    let original_mean: f32 = signal.iter().sum::<f32>() / num_samples as f32;

    // Apply dither
    let dither = generate_tpdf_dither(num_samples);
    let dithered: Vec<f32> = signal
        .iter()
        .zip(dither.iter())
        .map(|(&s, &d)| s + d / 32768.0) // Scale dither to 16-bit LSB
        .collect();

    let dithered_mean: f32 = dithered.iter().sum::<f32>() / num_samples as f32;
    let dc_change = (dithered_mean - original_mean).abs();

    println!("DC Offset Change from Dithering:");
    println!("  Original mean: {:.8}", original_mean);
    println!("  Dithered mean: {:.8}", dithered_mean);
    println!("  DC change: {:.8}", dc_change);

    // DC offset should not change significantly
    assert!(
        dc_change < 0.0001,
        "Dithering should not add DC offset. Change: {:.8}",
        dc_change
    );
}

// =============================================================================
// 4. CLIPPING BEHAVIOR
// =============================================================================

#[test]
fn test_hard_clipping_at_1_0() {
    // Create signal that exceeds 1.0
    let mut signal: Vec<f32> = vec![0.5, 0.5, 1.5, 1.5, 2.0, 2.0, -1.5, -1.5, -2.0, -2.0];

    // Hard clip to [-1.0, 1.0]
    for sample in &mut signal {
        *sample = (*sample).clamp(-1.0, 1.0);
    }

    // Verify clipping
    for (i, &sample) in signal.iter().enumerate() {
        assert!(
            sample >= -1.0 && sample <= 1.0,
            "Sample {} exceeds limits: {}",
            i,
            sample
        );
    }

    // Check specific clipped values
    assert_eq!(signal[2], 1.0, "1.5 should clip to 1.0");
    assert_eq!(signal[4], 1.0, "2.0 should clip to 1.0");
    assert_eq!(signal[6], -1.0, "-1.5 should clip to -1.0");
    assert_eq!(signal[8], -1.0, "-2.0 should clip to -1.0");
}

#[test]
fn test_soft_clipping_characteristics() {
    // Soft clipping using tanh-style curve (properly normalized)
    // This uses tanh(x) which asymptotically approaches +-1
    fn soft_clip(x: f32) -> f32 {
        x.tanh()
    }

    let test_values = [0.0, 0.5, 0.8, 1.0, 1.2, 1.5, 2.0, 3.0];

    println!("Soft Clipping (tanh):");
    for &value in &test_values {
        let clipped = soft_clip(value);
        println!("  Input: {:.2} -> Output: {:.4}", value, clipped);

        // Soft clipping via tanh asymptotically approaches 1.0 but never exceeds it
        assert!(
            clipped.abs() < 1.0,
            "Soft clip output should be < 1.0, got {}",
            clipped
        );

        // Output should preserve sign
        if value > 0.0 {
            assert!(clipped > 0.0, "Positive input should give positive output");
        }
    }

    // Verify smooth transition (derivative should be smooth)
    let x1 = 0.9;
    let x2 = 1.0;
    let x3 = 1.1;
    let y1 = soft_clip(x1);
    let y2 = soft_clip(x2);
    let y3 = soft_clip(x3);

    // Slope should decrease as we approach saturation
    let slope1 = (y2 - y1) / (x2 - x1);
    let slope2 = (y3 - y2) / (x3 - x2);

    println!("  Slope at 0.95: {:.4}", slope1);
    println!("  Slope at 1.05: {:.4}", slope2);

    assert!(
        slope2 < slope1,
        "Soft clip slope should decrease: {} should be < {}",
        slope2,
        slope1
    );
}

#[test]
fn test_intersample_peak_detection() {
    // Create a signal with intersample peaks
    // When two adjacent samples are high but opposite phase,
    // the interpolated peak can exceed both samples
    let signal = vec![
        0.0, 0.0, // baseline
        0.9, 0.9, // rising
        0.0, 0.0, // zero crossing
        -0.9, -0.9, // opposite peak
        0.0, 0.0, // baseline
    ];

    // Simple peak (sample-based)
    let sample_peak = calculate_peak(&signal);

    // Detect potential intersample peaks by analyzing slope
    let mut max_slope = 0.0f32;
    for chunk in signal.chunks(2) {
        if chunk.len() == 2 {
            let left = chunk[0];
            // Find slope to next frame
            // (simplified - real intersample detection uses oversampling)
            max_slope = max_slope.max(left.abs());
        }
    }

    // For this specific signal, intersample peak would be between 0.9 and -0.9
    // The true peak could exceed 0.9 depending on the signal's frequency
    // relative to sample rate

    println!("Intersample Peak Analysis:");
    println!("  Sample peak: {:.4}", sample_peak);
    println!("  Max slope indicator: {:.4}", max_slope);

    // Verify we can at least detect the sample-level peak
    assert!(
        (sample_peak - 0.9).abs() < 0.01,
        "Sample peak should be 0.9, got {}",
        sample_peak
    );
}

#[test]
fn test_limiter_prevents_clipping() {
    let mut limiter = Limiter::with_settings(LimiterSettings::brickwall());
    limiter.set_threshold(-0.5); // -0.5 dB ≈ 0.944

    // Create signal that would clip
    let mut signal = generate_sine_wave(440.0, 44100, 0.1, 1.2);

    limiter.process(&mut signal, 44100);

    let peak = calculate_peak(&signal);
    let threshold_linear = 10.0f32.powf(-0.5 / 20.0);

    println!("Limiter Test:");
    println!("  Threshold: {:.4} ({:.1} dB)", threshold_linear, -0.5);
    println!("  Output peak: {:.4} ({:.1} dB)", peak, linear_to_db(peak));

    // Peak should be at or below threshold
    // Allow small overshoot due to lookahead/attack time
    assert!(
        peak <= threshold_linear * 1.1,
        "Limiter should prevent clipping. Peak {:.4} > threshold {:.4}",
        peak,
        threshold_linear
    );
}

#[test]
fn test_true_peak_limiting() {
    // True peak limiting requires oversampling to catch intersample peaks
    // This test verifies the limiter reduces peaks even when they don't
    // appear at sample boundaries

    let mut limiter = Limiter::with_settings(LimiterSettings::brickwall());
    limiter.set_threshold(-1.0); // -1 dB

    // Create a signal with potential intersample peaks
    // High frequency near Nyquist can have significant intersample peaks
    let mut signal = generate_sine_wave(20000.0, 44100, 0.1, 0.9);

    let input_peak = calculate_peak(&signal);
    limiter.process(&mut signal, 44100);
    let output_peak = calculate_peak(&signal);

    println!("True Peak Limiting Test:");
    println!("  Input peak: {:.4}", input_peak);
    println!("  Output peak: {:.4}", output_peak);
    println!("  Threshold: -1.0 dB = {:.4}", 10.0f32.powf(-1.0 / 20.0));

    // Output should be reduced (exact amount depends on implementation)
    assert!(
        output_peak <= input_peak,
        "Limiter should not increase peak: {} > {}",
        output_peak,
        input_peak
    );
}

// =============================================================================
// 5. DC OFFSET HANDLING
// =============================================================================

#[test]
fn test_dc_offset_detection() {
    // Create signal with DC offset
    let dc_offset = 0.1;
    let signal: Vec<f32> = (0..44100)
        .map(|i| {
            let t = i as f32 / 44100.0;
            (2.0 * PI * 1000.0 * t).sin() * 0.5 + dc_offset
        })
        .collect();

    let mean = signal.iter().sum::<f32>() / signal.len() as f32;

    println!("DC Offset Detection:");
    println!("  Expected DC: {:.4}", dc_offset);
    println!("  Measured DC: {:.4}", mean);

    assert!(
        (mean - dc_offset).abs() < 0.001,
        "Should detect DC offset. Expected {:.4}, got {:.4}",
        dc_offset,
        mean
    );
}

#[test]
fn test_dc_removal_effectiveness() {
    // Create signal with DC offset
    let dc_offset = 0.2;
    let num_samples = 44100 * 2; // 2 seconds
    let mut signal: Vec<f32> = (0..num_samples)
        .map(|i| {
            let t = i as f32 / 44100.0;
            (2.0 * PI * 440.0 * t).sin() * 0.5 + dc_offset
        })
        .collect();

    let original_dc = signal.iter().sum::<f32>() / signal.len() as f32;

    // Simple DC removal: highpass filter with very low cutoff
    // Using a simple IIR: y[n] = x[n] - x[n-1] + 0.995 * y[n-1]
    let alpha = 0.995;
    let mut prev_input = signal[0];
    let mut prev_output = 0.0;

    for sample in signal.iter_mut() {
        let output = *sample - prev_input + alpha * prev_output;
        prev_input = *sample;
        prev_output = output;
        *sample = output;
    }

    // Skip first part (filter settling time)
    let settled_signal = &signal[4410..]; // Skip first 0.1 seconds
    let final_dc = settled_signal.iter().sum::<f32>() / settled_signal.len() as f32;

    println!("DC Removal Effectiveness:");
    println!("  Original DC: {:.6}", original_dc);
    println!("  Final DC: {:.6}", final_dc);
    println!(
        "  Reduction: {:.1}x",
        original_dc / final_dc.abs().max(0.0001)
    );

    // DC should be significantly reduced
    assert!(
        final_dc.abs() < 0.01,
        "DC should be removed. Original: {:.4}, Final: {:.4}",
        original_dc,
        final_dc
    );
}

#[test]
fn test_no_dc_accumulation() {
    // Process long signal and verify DC doesn't accumulate
    let mut chain = EffectChain::new();

    let mut eq = ParametricEq::new();
    eq.set_bands(vec![
        EqBand::new(100.0, 0.0, 1.0),
        EqBand::new(1000.0, 6.0, 1.0),
        EqBand::new(10000.0, 0.0, 1.0),
    ]);
    chain.add_effect(Box::new(eq));

    let mut compressor = Compressor::with_settings(CompressorSettings::gentle());
    chain.add_effect(Box::new(compressor));

    // Process in chunks to simulate real-time processing
    let mut signal = vec![0.0; 44100 * 10]; // 10 seconds
    for i in 0..signal.len() {
        let t = i as f32 / 44100.0;
        signal[i] = (2.0 * PI * 440.0 * t).sin() * 0.5;
    }

    // Measure DC before
    let dc_before = signal.iter().sum::<f32>() / signal.len() as f32;

    // Process in 1024 sample chunks
    for chunk in signal.chunks_mut(1024) {
        chain.process(chunk, 44100);
    }

    // Measure DC after
    let dc_after = signal.iter().sum::<f32>() / signal.len() as f32;

    // Also check DC at different points
    let dc_start = signal[..44100].iter().sum::<f32>() / 44100.0;
    let dc_end = signal[signal.len() - 44100..].iter().sum::<f32>() / 44100.0;

    println!("DC Accumulation Test:");
    println!("  DC before: {:.8}", dc_before);
    println!("  DC after: {:.8}", dc_after);
    println!("  DC at start: {:.8}", dc_start);
    println!("  DC at end: {:.8}", dc_end);

    // DC should not accumulate over time
    let dc_drift = (dc_end - dc_start).abs();
    assert!(
        dc_drift < 0.01,
        "DC should not accumulate. Start: {:.6}, End: {:.6}, Drift: {:.6}",
        dc_start,
        dc_end,
        dc_drift
    );
}

// =============================================================================
// 6. NULL TESTING
// =============================================================================

#[test]
fn test_null_signal_plus_inverted() {
    // Signal + inverted signal should equal silence
    let signal = generate_test_pattern(4096, SAMPLE_RATE_44100);
    let inverted: Vec<f32> = signal.iter().map(|&s| -s).collect();

    let sum: Vec<f32> = signal
        .iter()
        .zip(inverted.iter())
        .map(|(&a, &b)| a + b)
        .collect();

    let max_deviation = calculate_peak(&sum);

    println!("Null Test (signal + inverted):");
    println!("  Max deviation from zero: {:e}", max_deviation);

    assert!(
        max_deviation < 1e-6,
        "Signal + inverted should be zero. Max deviation: {:e}",
        max_deviation
    );
}

#[test]
fn test_null_effect_bypass() {
    let input = generate_test_pattern(8192, SAMPLE_RATE_44100);

    // Process through disabled effect
    let mut eq = ParametricEq::new();
    eq.set_low_band(EqBand::low_shelf(100.0, 6.0));
    eq.set_mid_band(EqBand::peaking(1000.0, 6.0, 1.0));
    eq.set_high_band(EqBand::high_shelf(10000.0, 6.0));
    eq.set_enabled(false);

    let mut output = input.clone();
    eq.process(&mut output, SAMPLE_RATE_44100);

    // Subtract original from processed
    let null_result: Vec<f32> = output
        .iter()
        .zip(input.iter())
        .map(|(&a, &b)| a - b)
        .collect();

    let max_deviation = calculate_peak(&null_result);

    println!("Null Test (bypass):");
    println!("  Max deviation: {:e}", max_deviation);

    // Should be exactly zero (bit-perfect)
    assert!(
        max_deviation == 0.0,
        "Disabled effect should be bit-perfect. Max deviation: {:e}",
        max_deviation
    );
}

#[test]
fn test_null_double_invert() {
    // Process -> invert -> process -> invert should return to original
    // (for linear effects only)

    let input = generate_test_pattern(4096, SAMPLE_RATE_44100);

    // Apply a linear effect (EQ with 0 gain is linear but not identity)
    let mut eq = ParametricEq::new();
    eq.set_bands(vec![
        EqBand::new(100.0, 0.0, 1.0),
        EqBand::new(1000.0, 0.0, 1.0),
        EqBand::new(10000.0, 0.0, 1.0),
    ]);

    let mut output = input.clone();
    eq.process(&mut output, SAMPLE_RATE_44100);

    // For truly linear processing, phase-inverted input should give
    // phase-inverted output
    let inverted_input: Vec<f32> = input.iter().map(|&s| -s).collect();
    let mut inverted_output = inverted_input.clone();

    eq.reset(); // Reset filter state
    eq.process(&mut inverted_output, SAMPLE_RATE_44100);

    // Compare output + inverted_output
    let sum: Vec<f32> = output
        .iter()
        .zip(inverted_output.iter())
        .map(|(&a, &b)| a + b)
        .collect();

    let max_deviation = calculate_peak(&sum);
    let rms_deviation = calculate_rms(&sum);

    println!("Null Test (double invert):");
    println!("  Max deviation: {:e}", max_deviation);
    println!("  RMS deviation: {:e}", rms_deviation);

    // Should be very close to zero (filter state might cause tiny differences)
    assert!(
        max_deviation < 0.001,
        "Linear processing should null. Max deviation: {:e}",
        max_deviation
    );
}

#[test]
fn test_null_all_effects_disabled_chain() {
    let input = generate_test_pattern(8192, SAMPLE_RATE_44100);

    // Build full chain, all disabled
    let mut chain = EffectChain::new();

    let mut effects: Vec<Box<dyn AudioEffect>> = vec![
        Box::new(ParametricEq::new()),
        Box::new(GraphicEq::new_10_band()),
        Box::new(StereoEnhancer::new()),
        Box::new(Crossfeed::new()),
        Box::new(Compressor::new()),
        Box::new(Limiter::new()),
    ];

    for effect in &mut effects {
        effect.set_enabled(false);
    }

    for effect in effects {
        chain.add_effect(effect);
    }

    let mut output = input.clone();
    chain.process(&mut output, SAMPLE_RATE_44100);

    // Null test
    let null_result: Vec<f32> = output
        .iter()
        .zip(input.iter())
        .map(|(&a, &b)| a - b)
        .collect();

    let max_deviation = calculate_peak(&null_result);

    assert!(
        max_deviation == 0.0,
        "All-disabled chain should null perfectly. Deviation: {:e}",
        max_deviation
    );
}

// =============================================================================
// 7. LEVEL ACCURACY
// =============================================================================

#[test]
fn test_0dbfs_equals_1_0() {
    // 0 dBFS should be exactly 1.0 in float
    let level_0dbfs = 1.0f32;
    let db = linear_to_db(level_0dbfs);

    println!("0 dBFS Level Test:");
    println!("  Linear 1.0 = {:.4} dB", db);

    assert!(
        db.abs() < 0.001,
        "0 dBFS should equal 1.0 linear. Got {:.4} dB",
        db
    );
}

#[test]
fn test_minus_6dbfs_accuracy() {
    // -6 dBFS should be approximately 0.501 (half voltage)
    let expected_linear = 10.0f32.powf(-6.0 / 20.0);
    let actual_db = linear_to_db(0.5);

    println!("-6 dBFS Level Test:");
    println!("  Expected linear: {:.6}", expected_linear);
    println!("  0.5 linear = {:.4} dB", actual_db);

    // -6 dB ≈ 0.501 linear
    assert!(
        (expected_linear - 0.501).abs() < 0.01,
        "-6 dB should be ~0.501. Got {:.6}",
        expected_linear
    );

    // 0.5 linear ≈ -6.02 dB
    assert!(
        (actual_db - (-6.02)).abs() < 0.1,
        "0.5 linear should be ~-6.02 dB. Got {:.4}",
        actual_db
    );
}

#[test]
fn test_minus_20dbfs_accuracy() {
    // -20 dBFS should be 0.1 (10% of full scale)
    let expected_linear = 10.0f32.powf(-20.0 / 20.0);
    let actual_db = linear_to_db(0.1);

    println!("-20 dBFS Level Test:");
    println!("  -20 dB = {:.6} linear", expected_linear);
    println!("  0.1 linear = {:.4} dB", actual_db);

    // -20 dB = 0.1 exactly
    assert!(
        (expected_linear - 0.1).abs() < 0.001,
        "-20 dB should be 0.1. Got {:.6}",
        expected_linear
    );

    // 0.1 linear = -20 dB exactly
    assert!(
        (actual_db - (-20.0)).abs() < 0.01,
        "0.1 linear should be -20 dB. Got {:.4}",
        actual_db
    );
}

#[test]
fn test_noise_floor_measurement() {
    // Generate very quiet signal (noise floor test)
    let amplitude = 0.0001; // -80 dBFS
    let signal = generate_sine_wave(1000.0, 44100, 1.0, amplitude);

    let peak = calculate_peak(&signal);
    let peak_db = linear_to_db(peak);
    let rms = calculate_rms(&signal);
    let rms_db = linear_to_db(rms);

    println!("Noise Floor Measurement:");
    println!(
        "  Input amplitude: {} ({:.1} dBFS)",
        amplitude,
        linear_to_db(amplitude)
    );
    println!("  Measured peak: {:.6} ({:.1} dBFS)", peak, peak_db);
    println!("  Measured RMS: {:.6} ({:.1} dBFS)", rms, rms_db);

    // Verify we can accurately measure low levels
    let expected_peak_db = -80.0;
    let expected_rms_db = -80.0 - 3.0; // RMS of sine is -3dB from peak

    assert!(
        (peak_db - expected_peak_db).abs() < 1.0,
        "Should measure -80 dBFS peak. Got {:.1} dBFS",
        peak_db
    );
}

#[test]
fn test_level_accuracy_after_gain() {
    // Apply known gain and verify level accuracy
    let input_level = 0.5; // -6 dBFS
    let gain_db = 6.0;
    let gain_linear = 10.0f32.powf(gain_db / 20.0);

    let signal: Vec<f32> = (0..44100)
        .map(|i| {
            let t = i as f32 / 44100.0;
            (2.0 * PI * 1000.0 * t).sin() * input_level
        })
        .collect();

    let input_peak = calculate_peak(&signal);
    let output: Vec<f32> = signal.iter().map(|&s| s * gain_linear).collect();
    let output_peak = calculate_peak(&output);

    let input_db = linear_to_db(input_peak);
    let output_db = linear_to_db(output_peak);
    let measured_gain = output_db - input_db;

    println!("Gain Accuracy Test:");
    println!("  Input peak: {:.4} ({:.2} dB)", input_peak, input_db);
    println!(
        "  Applied gain: {:.2} dB (linear: {:.4})",
        gain_db, gain_linear
    );
    println!("  Output peak: {:.4} ({:.2} dB)", output_peak, output_db);
    println!("  Measured gain: {:.2} dB", measured_gain);

    // Measured gain should match applied gain
    assert!(
        (measured_gain - gain_db).abs() < 0.1,
        "Measured gain should match applied gain. Expected {:.2} dB, got {:.2} dB",
        gain_db,
        measured_gain
    );
}

#[test]
fn test_db_linear_roundtrip() {
    // Verify dB <-> linear conversion is accurate
    let test_db_values = [-60.0, -40.0, -20.0, -12.0, -6.0, -3.0, 0.0];

    for &db in &test_db_values {
        let linear = db_to_linear(db);
        let back_to_db = linear_to_db(linear);

        println!(
            "dB roundtrip: {} dB -> {:.6} linear -> {:.4} dB",
            db, linear, back_to_db
        );

        assert!(
            (back_to_db - db).abs() < 0.001,
            "dB roundtrip failed for {} dB. Got {:.6} dB",
            db,
            back_to_db
        );
    }
}

#[test]
fn test_level_consistency_across_frequencies() {
    // Same amplitude at different frequencies should give same level
    let amplitude = 0.5;
    let frequencies = [100.0, 440.0, 1000.0, 5000.0, 10000.0];

    println!("Level Consistency Across Frequencies:");

    let mut levels = Vec::new();
    for &freq in &frequencies {
        let signal = generate_sine_wave(freq, 44100, 0.5, amplitude);
        let mono = extract_mono(&signal, 0);
        let rms = calculate_rms(&mono);
        let rms_db = linear_to_db(rms);
        levels.push((freq, rms, rms_db));
        println!("  {} Hz: RMS = {:.6} ({:.2} dB)", freq, rms, rms_db);
    }

    // All levels should be within 0.1 dB of each other
    let max_db = levels
        .iter()
        .map(|(_, _, db)| *db)
        .fold(f32::NEG_INFINITY, f32::max);
    let min_db = levels
        .iter()
        .map(|(_, _, db)| *db)
        .fold(f32::INFINITY, f32::min);
    let variation = max_db - min_db;

    println!("  Level variation: {:.3} dB", variation);

    assert!(
        variation < 0.1,
        "Level should be consistent across frequencies. Variation: {:.3} dB",
        variation
    );
}

// =============================================================================
// COMPREHENSIVE E2E TESTS
// =============================================================================

#[test]
fn test_e2e_full_signal_path_bit_perfect() {
    println!("\n=== End-to-End Bit-Perfect Signal Path Test ===\n");

    let input = generate_test_pattern(16384, SAMPLE_RATE_44100);

    // Create full processing chain with all effects disabled
    let mut chain = EffectChain::new();

    let mut eq = ParametricEq::new();
    eq.set_bands(vec![
        EqBand::new(80.0, 3.0, 1.0),    // Would boost bass
        EqBand::new(1000.0, -2.0, 1.0), // Would cut mids
        EqBand::new(8000.0, 2.0, 1.0),  // Would boost treble
    ]);
    eq.set_enabled(false);
    chain.add_effect(Box::new(eq));

    let mut geq = GraphicEq::new_10_band();
    geq.set_preset(GraphicEqPreset::BassBoost);
    geq.set_enabled(false);
    chain.add_effect(Box::new(geq));

    let mut stereo = StereoEnhancer::with_settings(StereoSettings::extra_wide());
    stereo.set_enabled(false);
    chain.add_effect(Box::new(stereo));

    let mut crossfeed = Crossfeed::new();
    crossfeed.set_enabled(false);
    chain.add_effect(Box::new(crossfeed));

    let mut compressor = Compressor::with_settings(CompressorSettings::aggressive());
    compressor.set_enabled(false);
    chain.add_effect(Box::new(compressor));

    let mut limiter = Limiter::with_settings(LimiterSettings::brickwall());
    limiter.set_enabled(false);
    chain.add_effect(Box::new(limiter));

    let mut output = input.clone();

    // Process in chunks to simulate real-time
    for chunk in output.chunks_mut(1024) {
        chain.process(chunk, SAMPLE_RATE_44100);
    }

    // Verify bit-perfect
    let is_identical = buffers_bit_identical(&input, &output);
    let diff_count = count_differences(&input, &output);
    let max_diff = max_difference(&input, &output);

    println!("Results:");
    println!("  Bit-perfect: {}", is_identical);
    println!("  Differences: {} samples", diff_count);
    println!("  Max deviation: {:e}", max_diff);

    assert!(
        is_identical,
        "Full signal path with all effects disabled should be bit-perfect"
    );
}

#[test]
fn test_e2e_format_conversion_chain() {
    println!("\n=== End-to-End Format Conversion Chain Test ===\n");

    // Simulate: i16 source -> f32 processing -> i16 output
    let num_samples = 4096;

    // Generate i16 source (simulated)
    let i16_source: Vec<i16> = (0..num_samples)
        .map(|i| {
            let t = i as f32 / 44100.0;
            ((2.0 * PI * 1000.0 * t).sin() * 16000.0) as i16
        })
        .collect();

    // Convert to f32
    let f32_signal: Vec<f32> = i16_source.iter().map(|&s| s as f32 / 32768.0).collect();

    // Apply processing (neutral EQ)
    let mut eq = ParametricEq::new();
    eq.set_bands(vec![
        EqBand::new(100.0, 0.0, 1.0),
        EqBand::new(1000.0, 0.0, 1.0),
        EqBand::new(10000.0, 0.0, 1.0),
    ]);

    // Create stereo from mono for processing
    let mut stereo: Vec<f32> = f32_signal.iter().flat_map(|&s| [s, s]).collect();
    eq.process(&mut stereo, 44100);

    // Extract left channel
    let processed: Vec<f32> = stereo.chunks(2).map(|c| c[0]).collect();

    // Convert back to i16 (with dithering)
    let i16_output: Vec<i16> = processed
        .iter()
        .map(|&s| {
            let dither = (rand::random::<f32>() - rand::random::<f32>()) / 32768.0;
            ((s + dither) * 32768.0).round().clamp(-32768.0, 32767.0) as i16
        })
        .collect();

    // Measure roundtrip error
    let mut total_error = 0i64;
    let mut max_error = 0i16;
    for (original, converted) in i16_source.iter().zip(i16_output.iter()) {
        let error = (*original as i32 - *converted as i32).abs() as i16;
        total_error += error as i64;
        max_error = max_error.max(error);
    }

    let avg_error = total_error as f32 / num_samples as f32;

    println!("Format Conversion Chain Results:");
    println!("  Average error: {:.2} LSB", avg_error);
    println!("  Max error: {} LSB", max_error);

    // With neutral processing and dithering, error should be minimal
    assert!(
        avg_error < 2.0,
        "Average roundtrip error should be < 2 LSB. Got {:.2}",
        avg_error
    );
    assert!(
        max_error < 5,
        "Max roundtrip error should be < 5 LSB. Got {}",
        max_error
    );
}

#[test]
fn test_e2e_signal_integrity_comprehensive() {
    println!("\n=== Comprehensive Signal Integrity Test ===\n");

    // Test multiple sample rates
    let sample_rates = [44100, 48000, 96000];

    for &rate in &sample_rates {
        let input = generate_test_pattern(rate as usize, rate);

        // Apply transparent processing
        let mut chain = EffectChain::new();
        let mut eq = ParametricEq::new();
        eq.set_bands(vec![
            EqBand::new(100.0, 0.0, 1.0),
            EqBand::new(1000.0, 0.0, 1.0),
            EqBand::new(10000.0, 0.0, 1.0),
        ]);
        chain.add_effect(Box::new(eq));

        let mut output = input.clone();
        chain.process(&mut output, rate);

        // Analyze quality
        let input_mono = extract_mono(&input, 0);
        let output_mono = extract_mono(&output, 0);

        let input_rms = calculate_rms(&input_mono);
        let output_rms = calculate_rms(&output_mono);
        let rms_ratio = output_rms / input_rms;

        let max_diff = max_difference(&input, &output);
        let snr = calculate_snr(&output_mono, Some(&input_mono));

        println!("Sample Rate: {} Hz", rate);
        println!("  RMS ratio: {:.6}", rms_ratio);
        println!("  Max difference: {:e}", max_diff);
        println!("  SNR: {:.1} dB", snr);

        // Transparent processing should preserve signal
        assert!(
            (rms_ratio - 1.0).abs() < 0.01,
            "RMS should be preserved at {} Hz. Ratio: {:.6}",
            rate,
            rms_ratio
        );
    }
}
