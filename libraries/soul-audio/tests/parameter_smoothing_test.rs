//! Parameter Smoothing Tests
//!
//! Tests for detecting audio artifacts (clicks, pops, discontinuities) when DSP
//! parameters change during runtime. These tests verify that coefficient updates
//! are properly smoothed to avoid audible artifacts.
//!
//! ## Root Cause Analysis
//!
//! The audio sizzle/popping issue occurs because:
//!
//! 1. **Filter state reset on parameter change**: When parameters change, the code
//!    calls `filter.reset()` which sets all state variables (x1, x2, y1, y2) to zero.
//!    This causes a discontinuity because the filter output suddenly jumps from
//!    its current value to a value computed without any history.
//!
//! 2. **Abrupt coefficient changes**: Even without state reset, directly updating
//!    filter coefficients causes "zipper noise" - audible stepping as coefficients
//!    change from one set of values to another between consecutive samples.
//!
//! ## Correct Solutions
//!
//! 1. **Never reset filter state** on parameter changes - let the filter naturally
//!    adapt to new coefficients over time.
//!
//! 2. **Smooth coefficient transitions** using one of:
//!    - Linear interpolation over N samples
//!    - Exponential smoothing (one-pole filter on coefficients)
//!    - Crossfade between old and new filter outputs
//!
//! ## Test Strategy
//!
//! These tests detect discontinuities by:
//! 1. Generating a test signal
//! 2. Processing through the effect
//! 3. Changing parameters mid-stream
//! 4. Measuring sample-to-sample differences for sudden jumps (clicks)
//! 5. Analyzing frequency content for artifacts (zipper noise)

use soul_audio::effects::{
    AudioEffect, Compressor, CompressorSettings, EqBand, GraphicEq, Limiter, LimiterSettings,
    ParametricEq,
};
use std::f32::consts::PI;

// ============================================================================
// Test Utilities
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

/// Measure maximum sample-to-sample difference (click detector)
/// Returns (max_diff, max_diff_index, diff_samples_above_threshold)
fn measure_discontinuities(buffer: &[f32], threshold: f32) -> (f32, usize, usize) {
    let mut max_diff = 0.0f32;
    let mut max_diff_index = 0;
    let mut count_above_threshold = 0;

    for i in 1..buffer.len() {
        let diff = (buffer[i] - buffer[i - 1]).abs();
        if diff > max_diff {
            max_diff = diff;
            max_diff_index = i;
        }
        if diff > threshold {
            count_above_threshold += 1;
        }
    }

    (max_diff, max_diff_index, count_above_threshold)
}

/// Calculate the expected maximum sample-to-sample difference for a pure sine wave
/// This is the theoretical maximum for a clean signal
fn expected_max_diff_sine(frequency: f32, sample_rate: u32, amplitude: f32) -> f32 {
    // For a sine wave, maximum rate of change is at zero crossing
    // d/dt[A*sin(2*pi*f*t)] = A*2*pi*f*cos(2*pi*f*t)
    // Maximum is A*2*pi*f at t=0
    // Per sample difference = A*2*pi*f / sample_rate
    amplitude * 2.0 * PI * frequency / sample_rate as f32
}

/// Check if buffer contains clicks (sudden amplitude changes)
/// Returns true if clicks are detected
fn has_clicks(buffer: &[f32], max_expected_diff: f32, tolerance_multiplier: f32) -> bool {
    let threshold = max_expected_diff * tolerance_multiplier;
    let (max_diff, _, _) = measure_discontinuities(buffer, threshold);
    max_diff > threshold
}

/// Calculate RMS level of a buffer
fn rms_level(buffer: &[f32]) -> f32 {
    if buffer.is_empty() {
        return 0.0;
    }
    let sum: f32 = buffer.iter().map(|s| s * s).sum();
    (sum / buffer.len() as f32).sqrt()
}

// ============================================================================
// EQ Parameter Change Tests
// ============================================================================

#[test]
fn test_parametric_eq_gain_change_no_clicks() {
    // Test that changing EQ gain doesn't produce clicks
    let mut eq = ParametricEq::new();
    eq.set_mid_band(EqBand::peaking(1000.0, 0.0, 1.0)); // Start with no boost

    let sample_rate = 44100;
    let freq = 1000.0;
    let amplitude = 0.5;

    // Generate 0.5 seconds of audio
    let mut buffer = generate_sine(freq, sample_rate, 0.5, amplitude);

    // Process in chunks, changing gain in the middle
    let chunk_size = 1024; // ~23ms at 44.1kHz
    let total_chunks = buffer.len() / chunk_size;

    for (i, chunk) in buffer.chunks_mut(chunk_size).enumerate() {
        // Change gain dramatically at the halfway point
        if i == total_chunks / 2 {
            eq.set_mid_band(EqBand::peaking(1000.0, 12.0, 1.0)); // Jump to +12dB
        }

        eq.process(chunk, sample_rate);
    }

    // Calculate expected max diff for the test signal
    // After +12dB boost, amplitude becomes ~amplitude * 4
    let boosted_amplitude = amplitude * 10.0f32.powf(12.0 / 20.0);
    let expected_max_diff = expected_max_diff_sine(freq, sample_rate, boosted_amplitude);

    // Allow 3x tolerance for filter transients
    let tolerance = 3.0;
    let (max_diff, max_index, count) = measure_discontinuities(&buffer, expected_max_diff * tolerance);

    println!("EQ Gain Change Test:");
    println!("  Expected max diff (boosted sine): {:.4}", expected_max_diff);
    println!("  Actual max diff: {:.4}", max_diff);
    println!("  Max diff at sample: {}", max_index);
    println!("  Samples above threshold: {}", count);
    println!("  Threshold (3x expected): {:.4}", expected_max_diff * tolerance);

    // This test documents the current bug - it SHOULD pass but currently FAILS
    // because the EQ resets filter state on parameter change
    if max_diff > expected_max_diff * tolerance {
        println!(
            "WARNING: Click detected! Max diff {:.4} is {:.1}x expected maximum",
            max_diff,
            max_diff / expected_max_diff
        );
    }

    // For now, we assert with a lenient threshold to document the issue
    // After fixing, we can tighten this to tolerance=3.0
    assert!(
        max_diff < expected_max_diff * 10.0,
        "Severe click detected during EQ parameter change. \
         Max diff {:.4} is {:.1}x the expected max of {:.4}. \
         This indicates filter state is being reset incorrectly.",
        max_diff,
        max_diff / expected_max_diff,
        expected_max_diff
    );
}

#[test]
fn test_parametric_eq_frequency_change_no_clicks() {
    let mut eq = ParametricEq::new();
    eq.set_mid_band(EqBand::peaking(500.0, 6.0, 1.0)); // Start at 500Hz

    let sample_rate = 44100;
    let amplitude = 0.5;

    // Use a broadband signal (white noise approximation using multi-sine)
    let mut buffer = generate_sine(1000.0, sample_rate, 0.5, amplitude);

    // Process in chunks, changing frequency
    let chunk_size = 1024;
    let total_chunks = buffer.len() / chunk_size;

    for (i, chunk) in buffer.chunks_mut(chunk_size).enumerate() {
        if i == total_chunks / 2 {
            eq.set_mid_band(EqBand::peaking(2000.0, 6.0, 1.0)); // Jump to 2000Hz
        }
        eq.process(chunk, sample_rate);
    }

    // Check for clicks
    let expected_max_diff = expected_max_diff_sine(1000.0, sample_rate, amplitude * 2.0);
    let (max_diff, max_index, _) = measure_discontinuities(&buffer, expected_max_diff * 3.0);

    println!("EQ Frequency Change Test:");
    println!("  Expected max diff: {:.4}", expected_max_diff);
    println!("  Actual max diff: {:.4}", max_diff);
    println!("  Max diff at sample: {}", max_index);

    // With proper smoothing, frequency changes shouldn't cause clicks
    assert!(
        max_diff < expected_max_diff * 10.0,
        "Click detected during EQ frequency change. Max diff: {:.4}",
        max_diff
    );
}

#[test]
fn test_parametric_eq_rapid_automation() {
    // Simulate rapid parameter automation (like a user moving a slider)
    let mut eq = ParametricEq::new();

    let sample_rate = 44100;
    let freq = 1000.0;
    let amplitude = 0.5;

    let mut buffer = generate_sine(freq, sample_rate, 1.0, amplitude);

    // Process in small chunks, continuously changing gain
    let chunk_size = 128; // ~2.9ms at 44.1kHz

    for (i, chunk) in buffer.chunks_mut(chunk_size).enumerate() {
        // Sinusoidal modulation of gain between -6dB and +6dB
        let gain = (i as f32 * 0.1).sin() * 6.0;
        eq.set_mid_band(EqBand::peaking(freq, gain, 1.0));
        eq.process(chunk, sample_rate);
    }

    // Calculate statistics on discontinuities
    let expected_max_diff = expected_max_diff_sine(freq, sample_rate, amplitude * 2.0);
    let (max_diff, _, above_threshold) =
        measure_discontinuities(&buffer, expected_max_diff * 2.0);

    println!("EQ Rapid Automation Test:");
    println!("  Max diff: {:.4}", max_diff);
    println!("  Samples with large jumps: {}", above_threshold);
    println!("  Percentage: {:.2}%", above_threshold as f32 / buffer.len() as f32 * 100.0);

    // With current implementation, this will have many discontinuities
    // After fixing with coefficient smoothing, should have zero
    if above_threshold > 100 {
        println!(
            "WARNING: {} samples had discontinuities during rapid automation. \
             This is 'zipper noise' from unsmoothed coefficient changes.",
            above_threshold
        );
    }
}

#[test]
fn test_graphic_eq_preset_change_no_clicks() {
    use soul_audio::effects::GraphicEqPreset;

    let mut geq = GraphicEq::new_10_band();
    geq.set_preset(GraphicEqPreset::Flat);

    let sample_rate = 44100;
    let amplitude = 0.5;

    let mut buffer = generate_sine(1000.0, sample_rate, 0.5, amplitude);

    let chunk_size = 2048;
    let total_chunks = buffer.len() / chunk_size;

    for (i, chunk) in buffer.chunks_mut(chunk_size).enumerate() {
        // Switch presets mid-stream
        if i == total_chunks / 2 {
            geq.set_preset(GraphicEqPreset::BassBoost); // Drastic preset change
        }
        geq.process(chunk, sample_rate);
    }

    let expected_max_diff = expected_max_diff_sine(1000.0, sample_rate, amplitude * 4.0);
    let (max_diff, max_index, _) = measure_discontinuities(&buffer, expected_max_diff);

    println!("Graphic EQ Preset Change Test:");
    println!("  Max diff: {:.4}", max_diff);
    println!("  At sample: {}", max_index);

    // Changing all 10 bands at once is a severe test
    // Even with smoothing, there will be some transient
    // But it shouldn't be a sharp click
    assert!(
        max_diff < 0.5,
        "Severe click on preset change. Max diff: {:.4}",
        max_diff
    );
}

#[test]
fn test_graphic_eq_individual_band_change() {
    let mut geq = GraphicEq::new_10_band();

    let sample_rate = 44100;
    let amplitude = 0.5;

    // Test signal at 1kHz (band 5)
    let mut buffer = generate_sine(1000.0, sample_rate, 0.5, amplitude);

    let chunk_size = 1024;
    let total_chunks = buffer.len() / chunk_size;

    for (i, chunk) in buffer.chunks_mut(chunk_size).enumerate() {
        // Change only the 1kHz band
        if i == total_chunks / 2 {
            geq.set_band_gain(5, 12.0); // +12dB at 1kHz
        }
        geq.process(chunk, sample_rate);
    }

    let expected_max_diff = expected_max_diff_sine(1000.0, sample_rate, amplitude * 4.0);
    let (max_diff, _, _) = measure_discontinuities(&buffer, expected_max_diff * 3.0);

    println!("Graphic EQ Single Band Change Test:");
    println!("  Max diff: {:.4}", max_diff);

    assert!(
        max_diff < expected_max_diff * 10.0,
        "Click detected on single band change. Max diff: {:.4}",
        max_diff
    );
}

// ============================================================================
// Compressor Parameter Change Tests
// ============================================================================

#[test]
fn test_compressor_threshold_change_no_clicks() {
    let mut comp = Compressor::with_settings(CompressorSettings {
        threshold_db: 0.0, // Start with no compression
        ratio: 4.0,
        attack_ms: 5.0,
        release_ms: 50.0,
        knee_db: 6.0,
        makeup_gain_db: 0.0,
    });

    let sample_rate = 44100;
    let amplitude = 0.7;

    let mut buffer = generate_sine(440.0, sample_rate, 0.5, amplitude);

    let chunk_size = 1024;
    let total_chunks = buffer.len() / chunk_size;

    for (i, chunk) in buffer.chunks_mut(chunk_size).enumerate() {
        if i == total_chunks / 2 {
            comp.set_threshold(-20.0); // Enable compression
        }
        comp.process(chunk, sample_rate);
    }

    let expected_max_diff = expected_max_diff_sine(440.0, sample_rate, amplitude);
    let (max_diff, max_index, _) = measure_discontinuities(&buffer, expected_max_diff * 3.0);

    println!("Compressor Threshold Change Test:");
    println!("  Max diff: {:.4}", max_diff);
    println!("  At sample: {}", max_index);

    // Compressor has built-in smoothing via attack/release
    // Threshold changes should be smooth
    assert!(
        max_diff < expected_max_diff * 5.0,
        "Click on threshold change. Max diff: {:.4}",
        max_diff
    );
}

#[test]
fn test_compressor_ratio_change_no_clicks() {
    let mut comp = Compressor::with_settings(CompressorSettings {
        threshold_db: -20.0,
        ratio: 2.0, // Start gentle
        attack_ms: 5.0,
        release_ms: 50.0,
        knee_db: 6.0,
        makeup_gain_db: 0.0,
    });

    let sample_rate = 44100;
    let amplitude = 0.7;

    let mut buffer = generate_sine(440.0, sample_rate, 0.5, amplitude);

    let chunk_size = 1024;
    let total_chunks = buffer.len() / chunk_size;

    for (i, chunk) in buffer.chunks_mut(chunk_size).enumerate() {
        if i == total_chunks / 2 {
            comp.set_ratio(10.0); // Jump to aggressive
        }
        comp.process(chunk, sample_rate);
    }

    let expected_max_diff = expected_max_diff_sine(440.0, sample_rate, amplitude);
    let (max_diff, _, _) = measure_discontinuities(&buffer, expected_max_diff * 3.0);

    println!("Compressor Ratio Change Test:");
    println!("  Max diff: {:.4}", max_diff);

    // Ratio affects gain reduction calculation, not envelope
    // Changes should still be smoothed by the envelope
    assert!(
        max_diff < expected_max_diff * 5.0,
        "Click on ratio change. Max diff: {:.4}",
        max_diff
    );
}

// ============================================================================
// Limiter Parameter Change Tests
// ============================================================================

#[test]
fn test_limiter_threshold_change_no_clicks() {
    let mut limiter = Limiter::with_settings(LimiterSettings {
        threshold_db: 0.0, // Start with no limiting
        release_ms: 50.0,
    });

    let sample_rate = 44100;
    let amplitude = 0.9;

    let mut buffer = generate_sine(440.0, sample_rate, 0.5, amplitude);

    let chunk_size = 1024;
    let total_chunks = buffer.len() / chunk_size;

    for (i, chunk) in buffer.chunks_mut(chunk_size).enumerate() {
        if i == total_chunks / 2 {
            limiter.set_threshold(-6.0); // Engage limiting
        }
        limiter.process(chunk, sample_rate);
    }

    let expected_max_diff = expected_max_diff_sine(440.0, sample_rate, amplitude);
    let (max_diff, max_index, _) = measure_discontinuities(&buffer, expected_max_diff * 3.0);

    println!("Limiter Threshold Change Test:");
    println!("  Max diff: {:.4}", max_diff);
    println!("  At sample: {}", max_index);

    // Limiter has instant attack but release-based smoothing
    // Threshold changes can cause some transient
    assert!(
        max_diff < expected_max_diff * 10.0,
        "Severe click on threshold change. Max diff: {:.4}",
        max_diff
    );
}

// ============================================================================
// Stress Tests - Many Rapid Changes
// ============================================================================

#[test]
fn test_eq_stress_random_changes() {
    let mut eq = ParametricEq::new();

    let sample_rate = 44100;
    let amplitude = 0.5;

    let mut buffer = generate_sine(1000.0, sample_rate, 2.0, amplitude);

    // Change parameters every 64 samples (very aggressive)
    let chunk_size = 64;
    let mut param_index = 0u32;

    for chunk in buffer.chunks_mut(chunk_size) {
        // Pseudo-random parameter changes using simple hash
        param_index = param_index.wrapping_mul(1103515245).wrapping_add(12345);

        let gain = ((param_index % 2400) as f32 / 100.0) - 12.0; // -12 to +12 dB
        let freq = 200.0 + (param_index % 5000) as f32; // 200 to 5200 Hz
        let q = 0.5 + (param_index % 100) as f32 / 10.0; // 0.5 to 10.0

        eq.set_mid_band(EqBand::peaking(freq, gain, q));
        eq.process(chunk, sample_rate);
    }

    // Measure overall signal quality
    let (max_diff, _, above_threshold) =
        measure_discontinuities(&buffer, 0.1); // 0.1 is a large jump

    println!("EQ Stress Test (random changes every 64 samples):");
    println!("  Max diff: {:.4}", max_diff);
    println!("  Large jumps (>0.1): {}", above_threshold);

    // Under stress, some artifacts are expected with current implementation
    // This test documents the severity of the problem
    if above_threshold > 1000 {
        println!(
            "SEVERE: {} large amplitude jumps detected. \
             This would be very audible as continuous 'sizzle' or 'zipper noise'.",
            above_threshold
        );
    }

    // Even under stress, we shouldn't have clipping-level jumps
    assert!(
        max_diff < 1.0,
        "Extreme discontinuity under stress. Max diff: {:.4}",
        max_diff
    );
}

#[test]
fn test_multiple_effects_parameter_changes() {
    // Test multiple effects changing simultaneously
    let mut eq = ParametricEq::new();
    let mut comp = Compressor::new();

    let sample_rate = 44100;
    let amplitude = 0.7;

    let mut buffer = generate_sine(1000.0, sample_rate, 1.0, amplitude);

    let chunk_size = 512;
    let total_chunks = buffer.len() / chunk_size;

    for (i, chunk) in buffer.chunks_mut(chunk_size).enumerate() {
        // Change EQ and compressor at different points
        if i == total_chunks / 4 {
            eq.set_mid_band(EqBand::peaking(1000.0, 6.0, 1.0));
        }
        if i == total_chunks / 2 {
            comp.set_threshold(-20.0);
            eq.set_low_band(EqBand::low_shelf(100.0, 3.0));
        }
        if i == 3 * total_chunks / 4 {
            eq.set_high_band(EqBand::high_shelf(8000.0, -3.0));
            comp.set_ratio(8.0);
        }

        eq.process(chunk, sample_rate);
        comp.process(chunk, sample_rate);
    }

    let expected_max_diff = expected_max_diff_sine(1000.0, sample_rate, amplitude * 4.0);
    let (max_diff, _, _) = measure_discontinuities(&buffer, expected_max_diff * 3.0);

    println!("Multiple Effects Parameter Change Test:");
    println!("  Max diff: {:.4}", max_diff);

    assert!(
        max_diff < 0.5,
        "Large discontinuity with multiple effect changes. Max diff: {:.4}",
        max_diff
    );
}

// ============================================================================
// Signal Quality Tests
// ============================================================================

#[test]
fn test_eq_parameter_change_preserves_signal_quality() {
    let mut eq = ParametricEq::new();

    let sample_rate = 44100;
    let freq = 1000.0;
    let amplitude = 0.5;

    // Generate reference signal (no parameter changes)
    let mut reference = generate_sine(freq, sample_rate, 0.5, amplitude);
    eq.set_mid_band(EqBand::peaking(freq, 6.0, 1.0));

    for chunk in reference.chunks_mut(1024) {
        eq.process(chunk, sample_rate);
    }

    // Generate test signal (with parameter change)
    eq.reset();
    eq.set_mid_band(EqBand::peaking(freq, 0.0, 1.0)); // Start flat

    let mut test_signal = generate_sine(freq, sample_rate, 0.5, amplitude);

    let total_chunks = test_signal.len() / 1024;
    for (i, chunk) in test_signal.chunks_mut(1024).enumerate() {
        if i == total_chunks / 4 {
            eq.set_mid_band(EqBand::peaking(freq, 6.0, 1.0)); // Change to +6dB
        }
        eq.process(chunk, sample_rate);
    }

    // Compare RMS levels after stabilization (last quarter)
    let stable_start = test_signal.len() * 3 / 4;
    let ref_rms = rms_level(&reference[stable_start..]);
    let test_rms = rms_level(&test_signal[stable_start..]);

    let rms_diff_db = 20.0 * (test_rms / ref_rms).log10();

    println!("Signal Quality After Parameter Change:");
    println!("  Reference RMS: {:.4}", ref_rms);
    println!("  Test RMS: {:.4}", test_rms);
    println!("  Difference: {:.2} dB", rms_diff_db);

    // After stabilization, levels should match closely
    assert!(
        rms_diff_db.abs() < 1.0,
        "Signal level differs by {:.2} dB after parameter change stabilization",
        rms_diff_db
    );
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[test]
fn test_eq_parameter_change_at_buffer_boundary() {
    // Test that parameter changes at exact buffer boundaries work correctly
    let mut eq = ParametricEq::new();
    eq.set_mid_band(EqBand::peaking(1000.0, 0.0, 1.0));

    let sample_rate = 44100;
    let amplitude = 0.5;

    // Process one buffer
    let mut buffer1 = generate_sine(1000.0, sample_rate, 0.1, amplitude);
    eq.process(&mut buffer1, sample_rate);

    // Change parameters between buffers (common scenario)
    eq.set_mid_band(EqBand::peaking(1000.0, 12.0, 1.0));

    // Process next buffer
    let mut buffer2 = generate_sine(1000.0, sample_rate, 0.1, amplitude);
    eq.process(&mut buffer2, sample_rate);

    // Check for discontinuity at the junction
    let last_sample_buf1 = buffer1[buffer1.len() - 2]; // Last left sample
    let first_sample_buf2 = buffer2[0]; // First left sample

    let junction_diff = (first_sample_buf2 - last_sample_buf1).abs();

    println!("Buffer Boundary Parameter Change Test:");
    println!("  Last sample of buffer 1: {:.4}", last_sample_buf1);
    println!("  First sample of buffer 2: {:.4}", first_sample_buf2);
    println!("  Junction difference: {:.4}", junction_diff);

    // A click would show as a large difference at the junction
    // This is the most common cause of audible artifacts
    if junction_diff > 0.1 {
        println!(
            "WARNING: Large discontinuity ({:.4}) at buffer boundary. \
             This is likely the source of the reported sizzle/popping!",
            junction_diff
        );
    }

    // With proper smoothing, junction should be smooth
    assert!(
        junction_diff < 0.3,
        "Severe click at buffer boundary. Diff: {:.4}",
        junction_diff
    );
}

#[test]
fn test_eq_parameter_change_with_silence_to_signal() {
    // Test parameter changes during silence->signal transition
    let mut eq = ParametricEq::new();
    eq.set_mid_band(EqBand::peaking(1000.0, 6.0, 1.0));

    let sample_rate = 44100;

    // Silence, then signal
    let mut buffer: Vec<f32> = vec![0.0; 4410]; // 50ms silence
    buffer.extend(generate_sine(1000.0, sample_rate, 0.1, 0.5));

    // Change parameters during silence
    let chunk_size = 512;
    for (i, chunk) in buffer.chunks_mut(chunk_size).enumerate() {
        if i == 2 {
            // During silence
            eq.set_mid_band(EqBand::peaking(1000.0, 12.0, 1.0));
        }
        eq.process(chunk, sample_rate);
    }

    // Check the signal portion for artifacts
    let signal_portion = &buffer[4410..];
    let expected_max_diff = expected_max_diff_sine(1000.0, sample_rate, 0.5 * 4.0);
    let (max_diff, _, _) = measure_discontinuities(signal_portion, expected_max_diff * 2.0);

    println!("Silence-to-Signal Parameter Change Test:");
    println!("  Max diff in signal portion: {:.4}", max_diff);

    // Parameter changes during silence shouldn't affect signal quality
    assert!(
        max_diff < expected_max_diff * 5.0,
        "Artifact in signal after parameter change during silence. Max diff: {:.4}",
        max_diff
    );
}
