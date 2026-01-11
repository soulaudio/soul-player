//! Runtime Parameter Artifact Detection Tests
//!
//! Comprehensive E2E tests that detect audio artifacts (clicks, pops, discontinuities)
//! when DSP parameters are changed at runtime. These tests verify that all effects
//! properly smooth parameter transitions to avoid audible artifacts.
//!
//! ## Artifact Detection Methods
//!
//! 1. **Sample-to-sample discontinuity detection**: Measures maximum sample difference
//!    and flags values that exceed the expected maximum for the signal type.
//!
//! 2. **High-frequency energy analysis**: Clicks produce broadband energy spikes.
//!    By analyzing the high-frequency content, we can detect transient artifacts.
//!
//! 3. **RMS level tracking**: Sudden RMS changes indicate discontinuities.

use soul_audio::effects::{
    AudioEffect, Compressor, Crossfeed, CrossfeedPreset, EqBand, GraphicEq,
    Limiter, LimiterSettings, ParametricEq, StereoEnhancer,
};
use std::f32::consts::PI;

// ============================================================================
// Test Utilities - Audio Generation
// ============================================================================

/// Generate a sine wave (stereo interleaved)
fn generate_sine(frequency: f32, sample_rate: u32, duration_sec: f32, amplitude: f32) -> Vec<f32> {
    let num_samples = (sample_rate as f32 * duration_sec) as usize;
    let mut buffer = Vec::with_capacity(num_samples * 2);
    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let sample = (2.0 * PI * frequency * t).sin() * amplitude;
        buffer.push(sample); // Left
        buffer.push(sample); // Right
    }
    buffer
}

/// Generate pink noise approximation (stereo interleaved)
fn generate_pink_noise(sample_rate: u32, duration_sec: f32, amplitude: f32) -> Vec<f32> {
    let num_samples = (sample_rate as f32 * duration_sec) as usize;
    let mut buffer = Vec::with_capacity(num_samples * 2);

    // Use simple LCG for deterministic "random" values
    let mut seed: u32 = 12345;
    let mut next_rand = || -> f32 {
        seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
        ((seed >> 16) as f32 / 32767.0) * 2.0 - 1.0
    };

    // Pink noise filter state
    let mut b0 = 0.0f32;
    let mut b1 = 0.0f32;
    let mut b2 = 0.0f32;

    for _ in 0..num_samples {
        let white = next_rand();
        // Voss-McCartney pink noise approximation
        b0 = 0.99886 * b0 + white * 0.0555179;
        b1 = 0.99332 * b1 + white * 0.0750759;
        b2 = 0.96900 * b2 + white * 0.1538520;
        let pink = (b0 + b1 + b2 + white * 0.5362) * amplitude * 0.2;
        buffer.push(pink); // Left
        buffer.push(pink); // Right
    }
    buffer
}

// ============================================================================
// Test Utilities - Artifact Detection
// ============================================================================

/// Calculate expected maximum sample-to-sample difference for a sine wave
fn expected_max_diff_sine(frequency: f32, sample_rate: u32, amplitude: f32) -> f32 {
    amplitude * 2.0 * PI * frequency / sample_rate as f32
}

/// Measure sample-to-sample discontinuities in a buffer
/// Returns (max_diff, max_index, samples_above_threshold)
fn measure_discontinuities(buffer: &[f32], threshold: f32) -> (f32, usize, usize) {
    let mut max_diff = 0.0f32;
    let mut max_index = 0;
    let mut count = 0;

    for i in 1..buffer.len() {
        let diff = (buffer[i] - buffer[i - 1]).abs();
        if diff > max_diff {
            max_diff = diff;
            max_index = i;
        }
        if diff > threshold {
            count += 1;
        }
    }

    (max_diff, max_index, count)
}

/// Check if a buffer contains clicks (sudden amplitude changes)
/// Returns (has_clicks, max_diff_found, threshold_used)
fn detect_clicks(
    buffer: &[f32],
    signal_frequency: f32,
    sample_rate: u32,
    expected_amplitude: f32,
    tolerance_multiplier: f32,
) -> (bool, f32, f32) {
    let expected_max = expected_max_diff_sine(signal_frequency, sample_rate, expected_amplitude);
    let threshold = expected_max * tolerance_multiplier;
    let (max_diff, _, _) = measure_discontinuities(buffer, threshold);
    (max_diff > threshold, max_diff, threshold)
}

/// Calculate RMS in a sliding window and detect sudden changes
fn detect_rms_discontinuities(buffer: &[f32], window_size: usize, threshold_ratio: f32) -> usize {
    let mut discontinuity_count = 0;
    let mut prev_rms = 0.0f32;

    for window in buffer.chunks(window_size) {
        let rms: f32 = (window.iter().map(|s| s * s).sum::<f32>() / window.len() as f32).sqrt();

        if prev_rms > 0.01 && rms > 0.01 {
            let ratio = if rms > prev_rms {
                rms / prev_rms
            } else {
                prev_rms / rms
            };
            if ratio > threshold_ratio {
                discontinuity_count += 1;
            }
        }
        prev_rms = rms;
    }

    discontinuity_count
}

// ============================================================================
// Limiter Parameter Change Tests
// ============================================================================

#[test]
fn test_limiter_threshold_change_detects_artifacts() {
    let mut limiter = Limiter::with_settings(LimiterSettings {
        threshold_db: 0.0, // Start with no limiting
        release_ms: 50.0,
    });

    let sample_rate = 44100;
    let amplitude = 0.9;

    // Generate 0.5 seconds of loud signal
    let mut buffer = generate_sine(440.0, sample_rate, 0.5, amplitude);

    let chunk_size = 256;
    let total_chunks = buffer.len() / chunk_size;

    for (i, chunk) in buffer.chunks_mut(chunk_size).enumerate() {
        // Change threshold mid-stream - this should be smoothed
        if i == total_chunks / 2 {
            limiter.set_threshold(-12.0); // Suddenly limit to -12dB
        }
        limiter.process(chunk, sample_rate);
    }

    // Detect artifacts
    let expected_max = expected_max_diff_sine(440.0, sample_rate, amplitude);
    let (max_diff, max_index, above_count) = measure_discontinuities(&buffer, expected_max * 3.0);

    eprintln!("Limiter Threshold Change Test:");
    eprintln!("  Expected max diff (sine): {:.4}", expected_max);
    eprintln!("  Actual max diff: {:.4}", max_diff);
    eprintln!("  Max diff at sample: {}", max_index);
    eprintln!("  Samples above 3x threshold: {}", above_count);

    // The limiter should smooth threshold changes
    // If max_diff is very high (>10x expected), there's a click
    assert!(
        max_diff < expected_max * 15.0,
        "Limiter threshold change caused click: max_diff={:.4} ({:.1}x expected)",
        max_diff,
        max_diff / expected_max
    );
}

#[test]
fn test_limiter_rapid_threshold_automation() {
    let mut limiter = Limiter::new();
    let sample_rate = 44100;
    let amplitude = 0.9;

    let mut buffer = generate_sine(440.0, sample_rate, 1.0, amplitude);

    let chunk_size = 64; // Small chunks for rapid automation

    for (i, chunk) in buffer.chunks_mut(chunk_size).enumerate() {
        // Oscillate threshold between 0 and -12dB
        let threshold = -6.0 + (i as f32 * 0.2).sin() * 6.0;
        limiter.set_threshold(threshold);
        limiter.process(chunk, sample_rate);
    }

    let expected_max = expected_max_diff_sine(440.0, sample_rate, amplitude);
    let (max_diff, _, above_count) = measure_discontinuities(&buffer, expected_max * 5.0);

    eprintln!("Limiter Rapid Automation Test:");
    eprintln!("  Max diff: {:.4}", max_diff);
    eprintln!("  Samples above threshold: {}", above_count);

    // Many discontinuities indicate zipper noise
    assert!(
        above_count < 500,
        "Limiter automation has excessive zipper noise: {} samples",
        above_count
    );
}

// ============================================================================
// Stereo Enhancer Parameter Change Tests
// ============================================================================

#[test]
fn test_stereo_width_change_detects_artifacts() {
    let mut enhancer = StereoEnhancer::new();

    let sample_rate = 44100;
    let amplitude = 0.5;

    let mut buffer = generate_sine(1000.0, sample_rate, 0.5, amplitude);

    let chunk_size = 256;
    let total_chunks = buffer.len() / chunk_size;

    for (i, chunk) in buffer.chunks_mut(chunk_size).enumerate() {
        // Change width mid-stream
        if i == total_chunks / 2 {
            enhancer.set_width(2.0); // Jump to extra wide
        }
        enhancer.process(chunk, sample_rate);
    }

    let expected_max = expected_max_diff_sine(1000.0, sample_rate, amplitude);
    let (max_diff, max_index, _) = measure_discontinuities(&buffer, expected_max * 3.0);

    eprintln!("Stereo Width Change Test:");
    eprintln!("  Expected max diff: {:.4}", expected_max);
    eprintln!("  Actual max diff: {:.4}", max_diff);
    eprintln!("  At sample: {}", max_index);

    assert!(
        max_diff < expected_max * 10.0,
        "Stereo width change caused click: max_diff={:.4}",
        max_diff
    );
}

#[test]
fn test_stereo_gain_change_detects_artifacts() {
    let mut enhancer = StereoEnhancer::new();

    let sample_rate = 44100;
    let amplitude = 0.5;

    let mut buffer = generate_sine(1000.0, sample_rate, 0.5, amplitude);

    let chunk_size = 256;
    let total_chunks = buffer.len() / chunk_size;

    for (i, chunk) in buffer.chunks_mut(chunk_size).enumerate() {
        if i == total_chunks / 2 {
            enhancer.set_mid_gain_db(12.0); // Jump to +12dB
        }
        enhancer.process(chunk, sample_rate);
    }

    // After boost, amplitude is higher
    let boosted_amplitude = amplitude * 10.0f32.powf(12.0 / 20.0);
    let expected_max = expected_max_diff_sine(1000.0, sample_rate, boosted_amplitude);
    let (max_diff, _, _) = measure_discontinuities(&buffer, expected_max * 3.0);

    eprintln!("Stereo Gain Change Test:");
    eprintln!("  Max diff: {:.4}", max_diff);

    assert!(
        max_diff < expected_max * 10.0,
        "Stereo gain change caused click: max_diff={:.4}",
        max_diff
    );
}

#[test]
fn test_stereo_rapid_automation() {
    let mut enhancer = StereoEnhancer::new();
    let sample_rate = 44100;
    let amplitude = 0.5;

    let mut buffer = generate_sine(1000.0, sample_rate, 1.0, amplitude);

    let chunk_size = 64;

    for (i, chunk) in buffer.chunks_mut(chunk_size).enumerate() {
        // Rapidly change width
        let width = 1.0 + (i as f32 * 0.1).sin() * 0.5;
        enhancer.set_width(width);
        enhancer.process(chunk, sample_rate);
    }

    let expected_max = expected_max_diff_sine(1000.0, sample_rate, amplitude * 1.5);
    let (max_diff, _, above_count) = measure_discontinuities(&buffer, expected_max * 3.0);

    eprintln!("Stereo Rapid Automation Test:");
    eprintln!("  Max diff: {:.4}", max_diff);
    eprintln!("  Samples above threshold: {}", above_count);

    assert!(
        above_count < 500,
        "Stereo automation has excessive artifacts: {} samples",
        above_count
    );
}

// ============================================================================
// Crossfeed Parameter Change Tests
// ============================================================================

#[test]
fn test_crossfeed_level_change_detects_artifacts() {
    let mut crossfeed = Crossfeed::with_preset(CrossfeedPreset::Natural);

    let sample_rate = 44100;
    let amplitude = 0.5;

    let mut buffer = generate_sine(1000.0, sample_rate, 0.5, amplitude);

    let chunk_size = 256;
    let total_chunks = buffer.len() / chunk_size;

    for (i, chunk) in buffer.chunks_mut(chunk_size).enumerate() {
        if i == total_chunks / 2 {
            crossfeed.set_level_db(-12.0); // Change level
        }
        crossfeed.process(chunk, sample_rate);
    }

    let expected_max = expected_max_diff_sine(1000.0, sample_rate, amplitude);
    let (max_diff, _, _) = measure_discontinuities(&buffer, expected_max * 3.0);

    eprintln!("Crossfeed Level Change Test:");
    eprintln!("  Max diff: {:.4}", max_diff);

    assert!(
        max_diff < expected_max * 10.0,
        "Crossfeed level change caused click: max_diff={:.4}",
        max_diff
    );
}

#[test]
fn test_crossfeed_preset_change_detects_artifacts() {
    let mut crossfeed = Crossfeed::with_preset(CrossfeedPreset::Natural);

    let sample_rate = 44100;
    let amplitude = 0.5;

    let mut buffer = generate_sine(1000.0, sample_rate, 0.5, amplitude);

    let chunk_size = 256;
    let total_chunks = buffer.len() / chunk_size;

    for (i, chunk) in buffer.chunks_mut(chunk_size).enumerate() {
        if i == total_chunks / 2 {
            crossfeed.set_preset(CrossfeedPreset::Meier); // Change preset
        }
        crossfeed.process(chunk, sample_rate);
    }

    let expected_max = expected_max_diff_sine(1000.0, sample_rate, amplitude);
    let (max_diff, _, _) = measure_discontinuities(&buffer, expected_max * 3.0);

    eprintln!("Crossfeed Preset Change Test:");
    eprintln!("  Max diff: {:.4}", max_diff);

    assert!(
        max_diff < expected_max * 10.0,
        "Crossfeed preset change caused click: max_diff={:.4}",
        max_diff
    );
}

// ============================================================================
// Combined Effects Chain Tests
// ============================================================================

#[test]
fn test_all_effects_rapid_parameter_changes() {
    // Test all effects together with rapid parameter changes

    let sample_rate = 44100;
    let amplitude = 0.5;
    let mut buffer = generate_pink_noise(sample_rate, 2.0, amplitude);

    let chunk_size = 128;
    let mut i = 0;

    // Create all effects
    let mut eq = ParametricEq::new();
    let mut geq = GraphicEq::new_10_band();
    let mut comp = Compressor::new();
    let mut limiter = Limiter::new();
    let mut stereo = StereoEnhancer::new();
    let mut crossfeed = Crossfeed::new();

    for chunk in buffer.chunks_mut(chunk_size) {
        // Vary parameters on all effects
        let phase = i as f32 * 0.05;

        // EQ
        eq.set_mid_band(EqBand::peaking(1000.0, phase.sin() * 6.0, 1.0));
        eq.process(chunk, sample_rate);

        // Graphic EQ
        geq.set_band_gain(5, phase.cos() * 6.0);
        geq.process(chunk, sample_rate);

        // Compressor
        comp.set_threshold(-20.0 + phase.sin() * 10.0);
        comp.process(chunk, sample_rate);

        // Limiter
        limiter.set_threshold(-1.0 + phase.cos() * 0.5);
        limiter.process(chunk, sample_rate);

        // Stereo
        stereo.set_width(1.0 + phase.sin() * 0.3);
        stereo.process(chunk, sample_rate);

        // Crossfeed
        crossfeed.set_level_db(-6.0 + phase.cos() * 2.0);
        crossfeed.process(chunk, sample_rate);

        i += 1;
    }

    // Check for RMS discontinuities
    let rms_discontinuities = detect_rms_discontinuities(&buffer, 256, 3.0);

    eprintln!("All Effects Stress Test:");
    eprintln!("  RMS discontinuities detected: {}", rms_discontinuities);

    // Allow some discontinuities but not excessive
    assert!(
        rms_discontinuities < 20,
        "Excessive RMS discontinuities during parameter automation: {}",
        rms_discontinuities
    );
}

#[test]
fn test_effect_enable_disable_no_clicks() {
    let sample_rate = 44100;
    let amplitude = 0.5;

    // Test each effect's enable/disable for clicks
    let effects: Vec<Box<dyn AudioEffect>> = vec![
        Box::new(ParametricEq::new()),
        Box::new(GraphicEq::new_10_band()),
        Box::new(Compressor::new()),
        Box::new(Limiter::new()),
        Box::new(StereoEnhancer::new()),
        Box::new(Crossfeed::new()),
    ];

    for mut effect in effects {
        let name = effect.name().to_string();
        let mut buffer = generate_sine(1000.0, sample_rate, 0.5, amplitude);

        let chunk_size = 512;
        let total_chunks = buffer.len() / chunk_size;

        for (i, chunk) in buffer.chunks_mut(chunk_size).enumerate() {
            // Toggle enable/disable
            effect.set_enabled(i % 4 < 2);
            effect.process(chunk, sample_rate);
        }

        let expected_max = expected_max_diff_sine(1000.0, sample_rate, amplitude * 2.0);
        let (max_diff, _, _) = measure_discontinuities(&buffer, expected_max);

        eprintln!("{} Enable/Disable Test: max_diff={:.4}", name, max_diff);

        assert!(
            max_diff < 1.0,
            "{} enable/disable caused click: max_diff={:.4}",
            name,
            max_diff
        );
    }
}

// ============================================================================
// Regression Tests for Specific Bug Reports
// ============================================================================

#[test]
fn test_eq_band_drag_no_popping() {
    // Simulates user dragging an EQ band frequency slider
    let mut eq = ParametricEq::new();
    let sample_rate = 44100;
    let amplitude = 0.5;

    let mut buffer = generate_sine(1000.0, sample_rate, 2.0, amplitude);

    let chunk_size = 64; // ~1.5ms chunks for responsive UI updates

    for (i, chunk) in buffer.chunks_mut(chunk_size).enumerate() {
        // Simulate dragging frequency from 500Hz to 2000Hz
        let freq = 500.0 + (i as f32 / 50.0).min(1.0) * 1500.0;
        eq.set_mid_band(EqBand::peaking(freq, 6.0, 1.0));
        eq.process(chunk, sample_rate);
    }

    let expected_max = expected_max_diff_sine(1000.0, sample_rate, amplitude * 2.0);
    let (max_diff, _, above_count) = measure_discontinuities(&buffer, expected_max * 3.0);

    eprintln!("EQ Band Drag Test:");
    eprintln!("  Max diff: {:.4}", max_diff);
    eprintln!("  Discontinuities: {}", above_count);

    assert!(
        above_count < 100,
        "EQ band drag has popping: {} discontinuities",
        above_count
    );
}

#[test]
fn test_limiter_ceiling_adjustment_no_popping() {
    // Simulates user adjusting limiter ceiling
    let mut limiter = Limiter::new();
    let sample_rate = 44100;
    let amplitude = 0.8;

    let mut buffer = generate_sine(440.0, sample_rate, 2.0, amplitude);

    let chunk_size = 64;

    for (i, chunk) in buffer.chunks_mut(chunk_size).enumerate() {
        // Simulate adjusting ceiling from 0dB to -6dB
        let ceiling = -((i as f32 / 100.0).min(1.0) * 6.0);
        limiter.set_threshold(ceiling);
        limiter.process(chunk, sample_rate);
    }

    let expected_max = expected_max_diff_sine(440.0, sample_rate, amplitude);
    let (max_diff, _, above_count) = measure_discontinuities(&buffer, expected_max * 5.0);

    eprintln!("Limiter Ceiling Adjustment Test:");
    eprintln!("  Max diff: {:.4}", max_diff);
    eprintln!("  Discontinuities: {}", above_count);

    assert!(
        above_count < 100,
        "Limiter ceiling adjustment has popping: {} discontinuities",
        above_count
    );
}

#[test]
fn test_stereo_width_slider_no_popping() {
    // Simulates user moving stereo width slider
    let mut enhancer = StereoEnhancer::new();
    let sample_rate = 44100;
    let amplitude = 0.5;

    let mut buffer = generate_sine(1000.0, sample_rate, 2.0, amplitude);

    let chunk_size = 64;

    for (i, chunk) in buffer.chunks_mut(chunk_size).enumerate() {
        // Simulate moving width from 1.0 to 2.0
        let width = 1.0 + (i as f32 / 100.0).min(1.0);
        enhancer.set_width(width);
        enhancer.process(chunk, sample_rate);
    }

    let expected_max = expected_max_diff_sine(1000.0, sample_rate, amplitude * 1.5);
    let (max_diff, _, above_count) = measure_discontinuities(&buffer, expected_max * 3.0);

    eprintln!("Stereo Width Slider Test:");
    eprintln!("  Max diff: {:.4}", max_diff);
    eprintln!("  Discontinuities: {}", above_count);

    assert!(
        above_count < 100,
        "Stereo width slider has popping: {} discontinuities",
        above_count
    );
}
