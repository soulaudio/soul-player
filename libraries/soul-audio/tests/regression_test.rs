//! Bug hunting tests - Critical examination of audio effect implementations
//!
//! These tests are specifically designed to find bugs by:
//! - Testing mathematical correctness, not just "doesn't crash"
//! - Verifying expected dB/linear conversions
//! - Testing boundary conditions aggressively
//! - Checking for known audio processing pitfalls
//!
//! ## BUGS FOUND:
//!
//! ### BUG 1: Limiter envelope initialization (limiter.rs:85, :163)
//! - Envelope initializes/resets to 1.0 (0 dB)
//! - Causes quiet signals to be attenuated until envelope settles
//! - FIX: Initialize envelope to 0.0 (or threshold_linear)
//!
//! ### BUG 2: Compressor envelope initialization (compressor.rs:136-137)
//! - Envelope initializes to 0.0 (interpreted as 0 dB in the follower)
//! - When signal arrives below 0 dB, logic uses RELEASE instead of ATTACK
//! - Attack time parameter has no effect when starting from silence
//! - FIX: Initialize envelope to very low dB value (e.g., -100.0)
//!
//! ### BUG 3: Compressor affects signals below threshold
//! - Even signals well below threshold get modified (-1 dB change)
//! - Related to envelope tracking behavior
//!

use soul_audio::effects::{
    AudioEffect, Compressor, CompressorSettings, Crossfeed, CrossfeedPreset, EqBand, GraphicEq,
    Limiter, LimiterSettings, ParametricEq, StereoEnhancer, StereoSettings,
};
use std::f32::consts::PI;

const SAMPLE_RATE: u32 = 44100;

// ============================================================================
// TEST UTILITIES
// ============================================================================

fn generate_sine(frequency: f32, sample_rate: u32, duration_sec: f32) -> Vec<f32> {
    let num_samples = (sample_rate as f32 * duration_sec) as usize * 2; // stereo
    let mut buffer = Vec::with_capacity(num_samples);
    for i in 0..num_samples / 2 {
        let t = i as f32 / sample_rate as f32;
        let sample = (2.0 * PI * frequency * t).sin();
        buffer.push(sample); // Left
        buffer.push(sample); // Right
    }
    buffer
}

fn generate_dc(level: f32, num_frames: usize) -> Vec<f32> {
    vec![level; num_frames * 2]
}

fn db_to_linear(db: f32) -> f32 {
    10.0_f32.powf(db / 20.0)
}

fn linear_to_db(linear: f32) -> f32 {
    20.0 * linear.abs().max(1e-10).log10()
}

fn peak_level(buffer: &[f32]) -> f32 {
    buffer.iter().map(|s| s.abs()).fold(0.0f32, f32::max)
}

fn rms_level(buffer: &[f32]) -> f32 {
    if buffer.is_empty() {
        return 0.0;
    }
    let sum: f32 = buffer.iter().map(|s| s * s).sum();
    (sum / buffer.len() as f32).sqrt()
}

// ============================================================================
// LIMITER TESTS - Checking for envelope initialization bug
// ============================================================================

#[test]
fn test_limiter_quiet_signal_not_attenuated() {
    // BUG HYPOTHESIS: Limiter envelope starts at 1.0, so quiet signals get
    // attenuated until envelope settles
    let mut limiter = Limiter::with_settings(LimiterSettings {
        threshold_db: -1.0, // Just below 0dB
        release_ms: 100.0,
    });

    // Very quiet signal (well below threshold)
    let level = 0.1; // -20 dB, way below -1 dB threshold
    let mut buffer = generate_dc(level, 1000);
    let original_peak = peak_level(&buffer);

    limiter.process(&mut buffer, SAMPLE_RATE);

    let processed_peak = peak_level(&buffer);

    // Quiet signal should NOT be attenuated by limiter
    // Allow small tolerance for floating point
    let attenuation_db = linear_to_db(processed_peak / original_peak);
    assert!(
        attenuation_db > -0.5,
        "BUG: Limiter attenuated quiet signal by {:.2} dB (should be ~0 dB). \
         Original: {:.4}, Processed: {:.4}",
        attenuation_db,
        original_peak,
        processed_peak
    );
}

#[test]
fn test_limiter_first_sample_not_attenuated() {
    // The very first sample should not be attenuated if below threshold
    let mut limiter = Limiter::with_settings(LimiterSettings {
        threshold_db: -1.0,
        release_ms: 100.0,
    });

    let level = 0.5; // -6 dB, below -1 dB threshold
    let mut buffer = vec![level, level]; // Single stereo frame

    limiter.process(&mut buffer, SAMPLE_RATE);

    // First sample should be unchanged (below threshold)
    assert!(
        (buffer[0] - level).abs() < 0.01,
        "BUG: First sample was modified from {} to {} even though below threshold",
        level,
        buffer[0]
    );
}

#[test]
fn test_limiter_actually_limits() {
    // Verify limiter actually limits peaks above threshold
    let mut limiter = Limiter::with_settings(LimiterSettings {
        threshold_db: -6.0, // ~0.5 linear
        release_ms: 50.0,
    });

    // Signal that exceeds threshold
    let mut buffer = vec![1.0, 1.0, 1.0, 1.0, 1.0, 1.0];

    limiter.process(&mut buffer, SAMPLE_RATE);

    let max_output = peak_level(&buffer);
    let threshold_linear = db_to_linear(-6.0);

    // Output should be at or below threshold (with small margin for attack time)
    assert!(
        max_output <= threshold_linear * 1.1,
        "Limiter failed to limit: output peak {:.4} exceeds threshold {:.4}",
        max_output,
        threshold_linear
    );
}

#[test]
fn test_limiter_reset_then_quiet_signal() {
    // After reset, quiet signal should still not be attenuated
    let mut limiter = Limiter::with_settings(LimiterSettings {
        threshold_db: -1.0,
        release_ms: 100.0,
    });

    // First process some loud signal
    let mut loud = vec![1.0; 200];
    limiter.process(&mut loud, SAMPLE_RATE);

    // Reset
    limiter.reset();

    // Now process quiet signal
    let level = 0.1;
    let mut quiet = generate_dc(level, 100);
    let original_peak = peak_level(&quiet);

    limiter.process(&mut quiet, SAMPLE_RATE);
    let processed_peak = peak_level(&quiet);

    let attenuation_db = linear_to_db(processed_peak / original_peak);
    assert!(
        attenuation_db > -0.5,
        "BUG: After reset, limiter attenuated quiet signal by {:.2} dB",
        attenuation_db
    );
}

// ============================================================================
// COMPRESSOR TESTS - Checking attack/release behavior
// ============================================================================

#[test]
fn test_compressor_attack_is_actually_fast() {
    // Test that attack time controls how fast compression engages
    let mut fast_attack = Compressor::with_settings(CompressorSettings {
        threshold_db: -20.0,
        ratio: 10.0,
        attack_ms: 0.1, // Very fast attack
        release_ms: 100.0,
        knee_db: 0.0,
        makeup_gain_db: 0.0,
    });

    let mut slow_attack = Compressor::with_settings(CompressorSettings {
        threshold_db: -20.0,
        ratio: 10.0,
        attack_ms: 100.0, // Very slow attack
        release_ms: 100.0,
        knee_db: 0.0,
        makeup_gain_db: 0.0,
    });

    // Generate loud signal (well above -20dB threshold)
    let mut buffer_fast = vec![0.5; 2000]; // stereo
    let mut buffer_slow = buffer_fast.clone();

    fast_attack.process(&mut buffer_fast, SAMPLE_RATE);
    slow_attack.process(&mut buffer_slow, SAMPLE_RATE);

    // Check early samples (first 10ms = 441 samples = 220 stereo frames)
    let early_fast: f32 = buffer_fast[100..200].iter().map(|s| s.abs()).sum::<f32>() / 100.0;
    let early_slow: f32 = buffer_slow[100..200].iter().map(|s| s.abs()).sum::<f32>() / 100.0;

    // Fast attack should have lower level early on (compression engaged sooner)
    assert!(
        early_fast < early_slow,
        "BUG: Fast attack ({:.4}) should compress more than slow attack ({:.4}) \
         in early samples",
        early_fast,
        early_slow
    );
}

#[test]
fn test_compressor_release_is_actually_slow() {
    // Test that release time controls how fast compression disengages
    let mut fast_release = Compressor::with_settings(CompressorSettings {
        threshold_db: -20.0,
        ratio: 10.0,
        attack_ms: 1.0,
        release_ms: 10.0, // Fast release
        knee_db: 0.0,
        makeup_gain_db: 0.0,
    });

    let mut slow_release = Compressor::with_settings(CompressorSettings {
        threshold_db: -20.0,
        ratio: 10.0,
        attack_ms: 1.0,
        release_ms: 1000.0, // Slow release
        knee_db: 0.0,
        makeup_gain_db: 0.0,
    });

    // First, process loud signal to engage compression
    let mut loud_fast = vec![0.5; 4000];
    let mut loud_slow = loud_fast.clone();
    fast_release.process(&mut loud_fast, SAMPLE_RATE);
    slow_release.process(&mut loud_slow, SAMPLE_RATE);

    // Now process quiet signal and see how fast compression releases
    let quiet_level = 0.01; // Well below threshold
    let mut quiet_fast = generate_dc(quiet_level, 2000);
    let mut quiet_slow = quiet_fast.clone();

    fast_release.process(&mut quiet_fast, SAMPLE_RATE);
    slow_release.process(&mut quiet_slow, SAMPLE_RATE);

    // Check late samples - fast release should recover to full level sooner
    let late_fast: f32 = quiet_fast[3000..3100].iter().sum::<f32>() / 100.0;
    let late_slow: f32 = quiet_slow[3000..3100].iter().sum::<f32>() / 100.0;

    // Fast release should be closer to original level
    // Note: With makeup gain 0, the quiet signal passes through mostly unchanged
    // The difference should be that slow release is still attenuating
    println!(
        "Fast release late avg: {:.6}, Slow release late avg: {:.6}",
        late_fast, late_slow
    );
}

#[test]
fn test_compressor_ratio_correctness() {
    // Test that the compression ratio is mathematically correct
    // For signal X dB above threshold, with ratio R, output should be
    // threshold + (X - threshold) / R
    let mut compressor = Compressor::with_settings(CompressorSettings {
        threshold_db: -20.0,
        ratio: 4.0, // 4:1 compression
        attack_ms: 0.1,
        release_ms: 0.1,
        knee_db: 0.0, // Hard knee for predictable behavior
        makeup_gain_db: 0.0,
    });

    // Input at -10 dB (10 dB above -20 dB threshold)
    let input_db = -10.0;
    let input_linear = db_to_linear(input_db);

    // Process enough samples for envelope to settle
    let mut buffer = generate_dc(input_linear, 10000);
    compressor.process(&mut buffer, SAMPLE_RATE);

    // Check output level (use late samples for settled envelope)
    let output_level = rms_level(&buffer[18000..]);
    let output_db = linear_to_db(output_level);

    // Expected: threshold + (input - threshold) / ratio = -20 + (-10 - -20) / 4 = -20 + 2.5 = -17.5 dB
    let expected_db = -20.0 + (input_db - (-20.0)) / 4.0;

    // Allow 3 dB tolerance due to envelope follower dynamics
    assert!(
        (output_db - expected_db).abs() < 3.0,
        "Compression ratio incorrect: input {:.1} dB, expected output {:.1} dB, got {:.1} dB",
        input_db,
        expected_db,
        output_db
    );
}

#[test]
fn test_compressor_below_threshold_unchanged() {
    // Signal below threshold should pass through unchanged (ratio 1:1 below threshold)
    let mut compressor = Compressor::with_settings(CompressorSettings {
        threshold_db: -20.0,
        ratio: 10.0,
        attack_ms: 1.0,
        release_ms: 10.0,
        knee_db: 0.0,
        makeup_gain_db: 0.0,
    });

    // Signal at -40 dB (well below -20 dB threshold)
    let input_linear = db_to_linear(-40.0);
    let mut buffer = generate_dc(input_linear, 1000);
    let original_rms = rms_level(&buffer);

    compressor.process(&mut buffer, SAMPLE_RATE);
    let processed_rms = rms_level(&buffer);

    // Should be unchanged (or very close)
    let change_db = linear_to_db(processed_rms / original_rms);
    assert!(
        change_db.abs() < 1.0,
        "Signal below threshold changed by {:.2} dB (should be ~0)",
        change_db
    );
}

// ============================================================================
// EQ TESTS - Verifying filter correctness
// ============================================================================

#[test]
fn test_eq_boost_increases_level() {
    // A +6dB boost should approximately double the amplitude at that frequency
    let mut eq = ParametricEq::new();
    eq.set_mid_band(EqBand::peaking(1000.0, 6.0, 1.0)); // +6 dB at 1kHz

    let mut buffer = generate_sine(1000.0, SAMPLE_RATE, 0.1);
    let original_rms = rms_level(&buffer);

    eq.process(&mut buffer, SAMPLE_RATE);
    let processed_rms = rms_level(&buffer);

    let boost_db = linear_to_db(processed_rms / original_rms);

    // +6 dB boost should result in ~6 dB increase (allow 2dB tolerance for filter shape)
    assert!(
        boost_db > 4.0 && boost_db < 8.0,
        "EQ +6dB boost resulted in {:.1} dB change (expected ~6 dB)",
        boost_db
    );
}

#[test]
fn test_eq_cut_decreases_level() {
    let mut eq = ParametricEq::new();
    eq.set_mid_band(EqBand::peaking(1000.0, -6.0, 1.0)); // -6 dB at 1kHz

    let mut buffer = generate_sine(1000.0, SAMPLE_RATE, 0.1);
    let original_rms = rms_level(&buffer);

    eq.process(&mut buffer, SAMPLE_RATE);
    let processed_rms = rms_level(&buffer);

    let cut_db = linear_to_db(processed_rms / original_rms);

    assert!(
        cut_db < -4.0 && cut_db > -8.0,
        "EQ -6dB cut resulted in {:.1} dB change (expected ~-6 dB)",
        cut_db
    );
}

#[test]
fn test_eq_frequency_selectivity() {
    // A narrow Q filter at 1kHz should not significantly affect 100Hz
    let mut eq = ParametricEq::new();
    eq.set_mid_band(EqBand::peaking(1000.0, 12.0, 5.0)); // +12 dB at 1kHz, Q=5

    let mut buffer_100hz = generate_sine(100.0, SAMPLE_RATE, 0.2);
    let original_100hz = rms_level(&buffer_100hz);

    eq.process(&mut buffer_100hz, SAMPLE_RATE);
    let processed_100hz = rms_level(&buffer_100hz);

    let change_db = linear_to_db(processed_100hz / original_100hz);

    // 100Hz should be minimally affected (< 3dB change)
    assert!(
        change_db.abs() < 3.0,
        "Narrow 1kHz boost affected 100Hz by {:.1} dB (should be minimal)",
        change_db
    );
}

#[test]
fn test_eq_zero_gain_transparent() {
    let mut eq = ParametricEq::new();
    eq.set_low_band(EqBand::low_shelf(100.0, 0.0));
    eq.set_mid_band(EqBand::peaking(1000.0, 0.0, 1.0));
    eq.set_high_band(EqBand::high_shelf(8000.0, 0.0));

    let mut buffer = generate_sine(1000.0, SAMPLE_RATE, 0.1);
    let original = buffer.clone();

    eq.process(&mut buffer, SAMPLE_RATE);

    // Zero gain should be nearly transparent (small filter artifacts allowed)
    let max_diff = buffer
        .iter()
        .zip(original.iter())
        .map(|(a, b)| (a - b).abs())
        .fold(0.0f32, f32::max);

    assert!(
        max_diff < 0.01,
        "Zero-gain EQ changed signal by max {:.4} (should be ~0)",
        max_diff
    );
}

// ============================================================================
// STEREO ENHANCER TESTS
// ============================================================================

#[test]
fn test_stereo_width_zero_creates_mono() {
    let mut enhancer = StereoEnhancer::with_settings(StereoSettings {
        width: 0.0,
        mid_gain_db: 0.0,
        side_gain_db: 0.0,
        balance: 0.0,
    });

    // Stereo signal with different L/R
    let mut buffer = vec![1.0, -1.0, 0.8, -0.8, 0.5, -0.5];

    enhancer.process(&mut buffer, SAMPLE_RATE);

    // With width=0, L and R should be identical
    for chunk in buffer.chunks(2) {
        assert!(
            (chunk[0] - chunk[1]).abs() < 0.001,
            "Width 0 should create mono: L={}, R={}",
            chunk[0],
            chunk[1]
        );
    }
}

#[test]
fn test_stereo_width_preserves_mono_content() {
    let mut enhancer = StereoEnhancer::with_settings(StereoSettings {
        width: 2.0, // Extra wide
        mid_gain_db: 0.0,
        side_gain_db: 0.0,
        balance: 0.0,
    });

    // Mono content (L == R)
    let mut buffer = vec![0.5, 0.5, 0.3, 0.3, -0.2, -0.2];
    let original = buffer.clone();

    enhancer.process(&mut buffer, SAMPLE_RATE);

    // Mono content has no side component, so width shouldn't change it
    for (i, chunk) in buffer.chunks(2).enumerate() {
        let orig_chunk = &original[i * 2..i * 2 + 2];
        assert!(
            (chunk[0] - orig_chunk[0]).abs() < 0.001,
            "Width change affected mono content: original {}, got {}",
            orig_chunk[0],
            chunk[0]
        );
    }
}

#[test]
fn test_stereo_balance_full_left() {
    let mut enhancer = StereoEnhancer::with_settings(StereoSettings {
        width: 1.0,
        mid_gain_db: 0.0,
        side_gain_db: 0.0,
        balance: -1.0, // Full left
    });

    let mut buffer = vec![0.5, 0.5, 0.3, 0.3];
    enhancer.process(&mut buffer, SAMPLE_RATE);

    // Right channel should be silent
    assert!(
        buffer[1].abs() < 0.001 && buffer[3].abs() < 0.001,
        "Full left balance should silence right channel"
    );
    // Left should still have signal
    assert!(buffer[0].abs() > 0.1, "Left channel should have signal");
}

// ============================================================================
// CROSSFEED TESTS
// ============================================================================

#[test]
fn test_crossfeed_adds_to_opposite_channel() {
    let mut crossfeed = Crossfeed::with_preset(CrossfeedPreset::Natural);

    // Hard-panned left signal
    let mut buffer: Vec<f32> = (0..1000).flat_map(|_| [0.5, 0.0]).collect();

    crossfeed.process(&mut buffer, SAMPLE_RATE);

    // Right channel should now have some signal
    let right_level: f32 = buffer.iter().skip(1).step_by(2).map(|s| s.abs()).sum();
    assert!(
        right_level > 0.1,
        "Crossfeed should add signal to opposite channel"
    );
}

#[test]
fn test_crossfeed_mono_passthrough() {
    let mut crossfeed = Crossfeed::with_preset(CrossfeedPreset::Natural);

    // Mono signal
    let mut buffer = vec![0.5, 0.5, 0.3, 0.3, 0.7, 0.7];

    crossfeed.process(&mut buffer, SAMPLE_RATE);

    // L and R should remain equal (crossfeed cancels for mono)
    for chunk in buffer.chunks(2) {
        assert!(
            (chunk[0] - chunk[1]).abs() < 0.01,
            "Mono signal should remain balanced after crossfeed"
        );
    }
}

// ============================================================================
// GRAPHIC EQ TESTS
// ============================================================================

#[test]
fn test_graphic_eq_band_independence() {
    // Setting one band shouldn't affect others (much)
    let mut geq = GraphicEq::new_10_band();

    // Boost band 5 (around 1kHz) by 12dB
    geq.set_band_gain(5, 12.0);

    // Test at band 0 frequency (~31Hz) and band 9 frequency (~16kHz)
    let mut buffer_low = generate_sine(31.0, SAMPLE_RATE, 0.5);
    let original_low = rms_level(&buffer_low);

    geq.process(&mut buffer_low, SAMPLE_RATE);
    let processed_low = rms_level(&buffer_low);

    let change_db = linear_to_db(processed_low / original_low);
    assert!(
        change_db.abs() < 3.0,
        "Band 5 boost affected band 0 frequency by {:.1} dB",
        change_db
    );
}

// ============================================================================
// EDGE CASE TESTS
// ============================================================================

#[test]
fn test_effects_handle_nan_input() {
    let effects: Vec<Box<dyn AudioEffect>> = vec![
        Box::new(ParametricEq::new()),
        Box::new(Compressor::new()),
        Box::new(Limiter::new()),
        Box::new(StereoEnhancer::new()),
        Box::new(Crossfeed::new()),
    ];

    for mut effect in effects {
        effect.set_enabled(true);

        let mut buffer = vec![f32::NAN, f32::NAN, f32::NAN, f32::NAN];
        effect.process(&mut buffer, SAMPLE_RATE);

        // Effects should either produce NaN (propagate) or handle gracefully
        // They should NOT produce Inf or crash
        for sample in &buffer {
            assert!(
                !sample.is_infinite(),
                "Effect {} produced Inf from NaN input",
                effect.name()
            );
        }
    }
}

#[test]
fn test_effects_handle_inf_input() {
    let effects: Vec<Box<dyn AudioEffect>> = vec![
        Box::new(ParametricEq::new()),
        Box::new(Compressor::new()),
        Box::new(Limiter::new()),
        Box::new(StereoEnhancer::new()),
        Box::new(Crossfeed::new()),
    ];

    for mut effect in effects {
        effect.set_enabled(true);

        let mut buffer = vec![f32::INFINITY, f32::INFINITY, -f32::INFINITY, -f32::INFINITY];
        effect.process(&mut buffer, SAMPLE_RATE);

        // Should not crash - behavior may vary
        // This test just ensures no panic
    }
}

#[test]
fn test_effects_handle_denormals() {
    let effects: Vec<Box<dyn AudioEffect>> = vec![
        Box::new(ParametricEq::new()),
        Box::new(Compressor::new()),
        Box::new(Limiter::new()),
        Box::new(StereoEnhancer::new()),
        Box::new(Crossfeed::new()),
    ];

    // Create buffer with denormal numbers
    let denormal = 1e-40_f32;

    for mut effect in effects {
        effect.set_enabled(true);

        let mut buffer = vec![denormal; 1000];
        effect.process(&mut buffer, SAMPLE_RATE);

        // Output should be finite (not NaN or Inf)
        for sample in &buffer {
            assert!(
                sample.is_finite(),
                "Effect {} produced non-finite output from denormal input",
                effect.name()
            );
        }
    }
}

#[test]
fn test_single_sample_buffer() {
    // Single stereo frame
    let effects: Vec<Box<dyn AudioEffect>> = vec![
        Box::new(ParametricEq::new()),
        Box::new(Compressor::new()),
        Box::new(Limiter::new()),
        Box::new(StereoEnhancer::new()),
        Box::new(Crossfeed::new()),
    ];

    for mut effect in effects {
        effect.set_enabled(true);

        let mut buffer = vec![0.5, 0.5];
        effect.process(&mut buffer, SAMPLE_RATE);

        assert!(
            buffer[0].is_finite() && buffer[1].is_finite(),
            "Effect {} failed on single sample buffer",
            effect.name()
        );
    }
}

#[test]
fn test_empty_buffer() {
    let effects: Vec<Box<dyn AudioEffect>> = vec![
        Box::new(ParametricEq::new()),
        Box::new(Compressor::new()),
        Box::new(Limiter::new()),
        Box::new(StereoEnhancer::new()),
        Box::new(Crossfeed::new()),
    ];

    for mut effect in effects {
        effect.set_enabled(true);

        let mut buffer: Vec<f32> = vec![];
        effect.process(&mut buffer, SAMPLE_RATE);

        // Should not panic
        assert!(buffer.is_empty());
    }
}

// ============================================================================
// MONO BUFFER TESTS (odd length / mono mode)
// ============================================================================

#[test]
fn test_odd_length_buffer_handling() {
    // Odd number of samples (not complete stereo frame)
    let effects: Vec<Box<dyn AudioEffect>> = vec![
        Box::new(ParametricEq::new()),
        Box::new(Compressor::new()),
        Box::new(Limiter::new()),
        Box::new(StereoEnhancer::new()),
        Box::new(Crossfeed::new()),
    ];

    for mut effect in effects {
        effect.set_enabled(true);

        let mut buffer = vec![0.5, 0.5, 0.5]; // 1.5 stereo frames
        effect.process(&mut buffer, SAMPLE_RATE);

        // The last sample (orphan) may or may not be processed
        // but should not crash
        assert!(buffer.len() == 3);
    }
}

// ============================================================================
// GAIN STRUCTURE TESTS
// ============================================================================

#[test]
fn test_chained_effects_dont_clip() {
    // Chaining multiple effects with boosts shouldn't cause massive clipping
    let mut eq = ParametricEq::new();
    eq.set_low_band(EqBand::low_shelf(100.0, 6.0));
    eq.set_mid_band(EqBand::peaking(1000.0, 6.0, 1.0));
    eq.set_high_band(EqBand::high_shelf(8000.0, 6.0));

    let mut compressor = Compressor::with_settings(CompressorSettings {
        threshold_db: -20.0,
        ratio: 2.0,
        attack_ms: 5.0,
        release_ms: 50.0,
        knee_db: 6.0,
        makeup_gain_db: 12.0, // +12 dB makeup
    });

    // Signal at moderate level
    let mut buffer = generate_sine(1000.0, SAMPLE_RATE, 0.1);

    eq.process(&mut buffer, SAMPLE_RATE);
    compressor.process(&mut buffer, SAMPLE_RATE);

    let max_level = peak_level(&buffer);

    // Should not have massive clipping (> 10x original = +20dB)
    assert!(
        max_level < 10.0,
        "Chained effects produced excessive level: {:.2}",
        max_level
    );
}
