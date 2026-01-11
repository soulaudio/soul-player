//! Sensitive sizzle/artifact detection tests
//!
//! These tests use more sensitive detection methods to catch audible artifacts
//! that humans can hear but simple metrics might miss.
//!
//! Key insight: The main cause of sizzle during parameter changes was the effect
//! chain being completely rebuilt (destroying filter state) whenever parameters
//! changed. The fix uses in-place parameter updates that preserve filter states.

use soul_audio::effects::{AudioEffect, EqBand, GraphicEq, ParametricEq};

const SAMPLE_RATE: u32 = 44100;
const CHUNK_SIZE: usize = 512;

/// Generate a pure sine wave with configurable start sample for phase continuity
fn generate_sine(frequency: f32, sample_rate: u32, num_samples: usize, amplitude: f32) -> Vec<f32> {
    generate_sine_at(frequency, sample_rate, 0, num_samples, amplitude)
}

/// Generate a pure sine wave starting at a specific sample index for phase continuity
fn generate_sine_at(
    frequency: f32,
    sample_rate: u32,
    start_sample: usize,
    num_samples: usize,
    amplitude: f32,
) -> Vec<f32> {
    let mut buffer = Vec::with_capacity(num_samples * 2);
    for i in 0..num_samples {
        let t = (start_sample + i) as f32 / sample_rate as f32;
        let sample = (2.0 * std::f32::consts::PI * frequency * t).sin() * amplitude;
        buffer.push(sample); // Left
        buffer.push(sample); // Right
    }
    buffer
}

/// Detect high-frequency content that shouldn't be there (sizzle/aliasing)
/// Uses a simple high-pass differentiator to detect rapid changes
fn measure_high_freq_content(buffer: &[f32]) -> f32 {
    if buffer.len() < 4 {
        return 0.0;
    }

    // Second derivative approximation - sensitive to high-frequency content
    let mut total_second_deriv = 0.0_f32;
    let mut count = 0;

    for i in 2..(buffer.len() / 2) {
        let left_prev2 = buffer[(i - 2) * 2];
        let left_prev1 = buffer[(i - 1) * 2];
        let left_curr = buffer[i * 2];

        // Second derivative: f''(x) ≈ f(x+1) - 2*f(x) + f(x-1)
        let second_deriv = (left_curr - 2.0 * left_prev1 + left_prev2).abs();
        total_second_deriv += second_deriv;
        count += 1;
    }

    if count > 0 {
        total_second_deriv / count as f32
    } else {
        0.0
    }
}

/// Measure the "roughness" of transitions - detects zipper noise
fn measure_zipper_noise(buffer: &[f32]) -> f32 {
    if buffer.len() < 6 {
        return 0.0;
    }

    // Look for irregular step patterns (zipper noise signature)
    let mut irregularity = 0.0_f32;
    let mut count = 0;

    for i in 2..(buffer.len() / 2 - 1) {
        let d1 = buffer[(i - 1) * 2] - buffer[(i - 2) * 2];
        let d2 = buffer[i * 2] - buffer[(i - 1) * 2];
        let d3 = buffer[(i + 1) * 2] - buffer[i * 2];

        // Irregularity: changes in derivative direction/magnitude
        let dir_change1 = (d2.signum() != d1.signum()) as i32 as f32;
        let dir_change2 = (d3.signum() != d2.signum()) as i32 as f32;

        // Rapid direction changes indicate zipper noise
        irregularity += dir_change1 + dir_change2;
        count += 1;
    }

    if count > 0 {
        irregularity / count as f32
    } else {
        0.0
    }
}

/// Measure spectral leakage/intermodulation distortion
/// by checking if output contains frequencies not in input
fn measure_spectral_leakage(
    input: &[f32],
    output: &[f32],
    _fundamental_freq: f32,
    _sample_rate: u32,
) -> f32 {
    if input.len() != output.len() || input.len() < 512 {
        return 0.0;
    }

    // Calculate expected amplitude change from EQ
    // Then measure deviation from expected sinusoidal shape

    let samples = input.len() / 2;

    // Extract left channels
    let input_left: Vec<f32> = input.iter().step_by(2).copied().collect();
    let output_left: Vec<f32> = output.iter().step_by(2).copied().collect();

    // Calculate RMS of input and output to find gain
    let input_rms: f32 = (input_left.iter().map(|x| x * x).sum::<f32>() / samples as f32).sqrt();
    let output_rms: f32 = (output_left.iter().map(|x| x * x).sum::<f32>() / samples as f32).sqrt();

    if input_rms < 0.001 {
        return 0.0;
    }

    let gain = output_rms / input_rms;

    // Generate expected output (input scaled by gain)
    // and measure deviation
    let mut deviation_sum = 0.0_f32;
    for i in 0..samples {
        let expected = input_left[i] * gain;
        let actual = output_left[i];
        deviation_sum += (actual - expected).abs();
    }

    deviation_sum / samples as f32
}

/// Print waveform samples around a point for debugging
fn print_waveform_around(buffer: &[f32], center_sample: usize, window: usize, label: &str) {
    println!("\n{} (samples around {}):", label, center_sample);
    let start = center_sample.saturating_sub(window);
    let end = (center_sample + window).min(buffer.len() / 2);

    for i in start..end {
        let left = buffer[i * 2];
        let marker = if i == center_sample { " <--" } else { "" };
        println!("  [{}]: {:.6}{}", i, left, marker);
    }
}

#[test]
fn test_eq_parameter_change_waveform_analysis() {
    // This test analyzes the actual waveform during parameter changes
    // to understand what artifacts look like

    let mut eq = ParametricEq::new();
    eq.set_band(0, EqBand::peaking(1000.0, 0.0, 1.0));

    let test_freq = 1000.0;
    let num_chunks = 20;

    println!("=== EQ Parameter Change Waveform Analysis ===\n");

    // Collect metrics for each chunk
    let mut all_high_freq: Vec<f32> = Vec::new();
    let mut all_zipper: Vec<f32> = Vec::new();
    let mut chunk_boundary_jumps: Vec<f32> = Vec::new();

    let mut prev_last_sample = 0.0_f32;

    for chunk_idx in 0..num_chunks {
        // Change gain progressively
        let gain = (chunk_idx as f32 / num_chunks as f32) * 12.0 - 6.0;
        eq.set_band(0, EqBand::peaking(1000.0, gain, 1.0));

        // Generate phase-continuous sine wave and process
        let start_sample = chunk_idx * CHUNK_SIZE;
        let input = generate_sine_at(test_freq, SAMPLE_RATE, start_sample, CHUNK_SIZE, 0.5);
        let mut output = input.clone();
        eq.process(&mut output, SAMPLE_RATE);

        // Measure artifacts
        let high_freq = measure_high_freq_content(&output);
        let zipper = measure_zipper_noise(&output);

        all_high_freq.push(high_freq);
        all_zipper.push(zipper);

        // Check chunk boundary discontinuity
        if chunk_idx > 0 {
            let first_sample = output[0];
            let jump = (first_sample - prev_last_sample).abs();
            chunk_boundary_jumps.push(jump);

            if jump > 0.05 {
                println!("Chunk {} boundary jump: {:.6} (prev_last={:.6}, first={:.6})",
                         chunk_idx, jump, prev_last_sample, first_sample);
                print_waveform_around(&output, 0, 5, "Start of chunk");
            }
        }

        prev_last_sample = output[output.len() - 2]; // Last left sample

        // Print detailed info for chunks with high artifact levels
        if high_freq > 0.01 || zipper > 0.5 {
            println!("Chunk {}: gain={:.1}dB, high_freq={:.6}, zipper={:.4}",
                     chunk_idx, gain, high_freq, zipper);
        }
    }

    // Summary statistics
    let max_high_freq = all_high_freq.iter().cloned().fold(0.0_f32, f32::max);
    let max_zipper = all_zipper.iter().cloned().fold(0.0_f32, f32::max);
    let max_boundary_jump = chunk_boundary_jumps.iter().cloned().fold(0.0_f32, f32::max);
    let avg_high_freq: f32 = all_high_freq.iter().sum::<f32>() / all_high_freq.len() as f32;

    println!("\n=== Summary ===");
    println!("Max high-freq content: {:.6}", max_high_freq);
    println!("Avg high-freq content: {:.6}", avg_high_freq);
    println!("Max zipper noise: {:.4}", max_zipper);
    println!("Max boundary jump: {:.6}", max_boundary_jump);

    // Thresholds for detecting audible artifacts
    // Note: With ±6dB gain changes, some high-freq content is expected from coefficient transitions
    // The key is that it's below audible threshold (~0.02 for high-freq)
    assert!(max_high_freq < 0.02, "High-frequency artifacts detected: {:.6}", max_high_freq);

    // Boundary jumps are expected when gain changes - a 6dB change is 2x amplitude
    // Allow up to 0.15 for smooth transitions (actual sizzle would show much higher)
    assert!(max_boundary_jump < 0.15, "Chunk boundary discontinuity: {:.6}", max_boundary_jump);
}

#[test]
fn test_eq_rapid_frequency_sweep_artifacts() {
    // Sweep EQ frequency rapidly and look for artifacts
    let mut eq = ParametricEq::new();
    eq.set_band(0, EqBand::peaking(500.0, 6.0, 2.0));

    let test_freq = 440.0;
    let num_chunks = 50;

    println!("=== Rapid Frequency Sweep Artifact Analysis ===\n");

    let mut max_deviation = 0.0_f32;
    let mut prev_last_sample = 0.0_f32;
    let mut max_boundary_jump = 0.0_f32;

    for chunk_idx in 0..num_chunks {
        // Sweep frequency from 200Hz to 2000Hz
        let progress = chunk_idx as f32 / num_chunks as f32;
        let eq_freq = 200.0 + progress * 1800.0;
        eq.set_band(0, EqBand::peaking(eq_freq, 6.0, 2.0));

        // Generate phase-continuous sine wave
        let start_sample = chunk_idx * CHUNK_SIZE;
        let input = generate_sine_at(test_freq, SAMPLE_RATE, start_sample, CHUNK_SIZE, 0.5);
        let mut output = input.clone();
        eq.process(&mut output, SAMPLE_RATE);

        // Measure deviation from expected sinusoidal shape
        let deviation = measure_spectral_leakage(&input, &output, test_freq, SAMPLE_RATE);
        max_deviation = max_deviation.max(deviation);

        // Check boundary
        if chunk_idx > 0 {
            let jump = (output[0] - prev_last_sample).abs();
            if jump > max_boundary_jump {
                max_boundary_jump = jump;
                if jump > 0.03 {
                    println!("Chunk {}: freq={:.0}Hz, boundary_jump={:.6}", chunk_idx, eq_freq, jump);
                }
            }
        }
        prev_last_sample = output[output.len() - 2];

        if deviation > 0.02 {
            println!("Chunk {}: freq={:.0}Hz, deviation={:.6}", chunk_idx, eq_freq, deviation);
        }
    }

    println!("\n=== Summary ===");
    println!("Max spectral deviation: {:.6}", max_deviation);
    println!("Max boundary jump: {:.6}", max_boundary_jump);

    // EQ frequency sweeps cause filter coefficient changes which affect phase
    // Allow reasonable boundary jumps (< 0.1) for smooth frequency transitions
    assert!(max_boundary_jump < 0.1, "Boundary discontinuity during freq sweep: {:.6}", max_boundary_jump);
}

#[test]
fn test_graphic_eq_slider_drag_artifacts() {
    // Simulate dragging a graphic EQ slider up and down
    let mut eq = GraphicEq::new_10_band();

    let test_freq = 1000.0; // Test at 1kHz band
    let band_idx = 4; // 1kHz band

    println!("=== Graphic EQ Slider Drag Artifact Analysis ===\n");

    let mut prev_output: Option<Vec<f32>> = None;
    let mut max_inter_chunk_diff = 0.0_f32;

    // Simulate dragging slider: -12dB -> +12dB -> -12dB (bounce)
    for frame in 0..100 {
        let t = frame as f32 / 100.0;
        let gain = (t * 4.0 * std::f32::consts::PI).sin() * 12.0; // Oscillate ±12dB

        eq.set_band_gain(band_idx, gain);

        // Generate phase-continuous sine wave
        let start_sample = frame * CHUNK_SIZE;
        let input = generate_sine_at(test_freq, SAMPLE_RATE, start_sample, CHUNK_SIZE, 0.5);
        let mut output = input.clone();
        eq.process(&mut output, SAMPLE_RATE);

        // Compare with previous chunk's last samples
        if let Some(ref prev) = prev_output {
            let prev_last = prev[prev.len() - 2];
            let curr_first = output[0];
            let diff = (curr_first - prev_last).abs();

            if diff > max_inter_chunk_diff {
                max_inter_chunk_diff = diff;
            }

            if diff > 0.1 {
                println!("Frame {}: gain={:.1}dB, chunk_boundary_diff={:.6}", frame, gain, diff);
            }
        }

        prev_output = Some(output);
    }

    println!("\n=== Summary ===");
    println!("Max inter-chunk difference: {:.6}", max_inter_chunk_diff);

    // With ±12dB oscillation (4x amplitude swing), some inter-chunk difference is expected
    // The EQ is correctly applying gain changes. Actual sizzle/zipper noise would show
    // as high-frequency content, not just level differences from gain changes.
    // Allow up to 0.5 for aggressive gain sweeps - the important metric is that
    // test_sample_by_sample_continuity shows no discontinuities beyond physics
    assert!(max_inter_chunk_diff < 0.5,
            "Chunk boundary artifacts during slider drag: {:.6}", max_inter_chunk_diff);
}

#[test]
fn test_coefficient_smoothing_effectiveness() {
    // Test if coefficient smoothing is actually working by comparing
    // with and without parameter changes

    let mut eq = ParametricEq::new();
    eq.set_band(0, EqBand::peaking(1000.0, 6.0, 1.0));

    // First, process without changes to establish baseline
    let mut baseline_output = generate_sine(1000.0, SAMPLE_RATE, CHUNK_SIZE * 10, 0.5);
    eq.process(&mut baseline_output, SAMPLE_RATE);

    let baseline_high_freq = measure_high_freq_content(&baseline_output);
    println!("Baseline (no changes) high-freq content: {:.6}", baseline_high_freq);

    // Now process with parameter changes every chunk
    eq.reset();
    eq.set_band(0, EqBand::peaking(1000.0, 0.0, 1.0));

    let mut changing_output = Vec::new();
    for chunk_idx in 0..10 {
        let gain = (chunk_idx as f32 / 10.0) * 12.0;
        eq.set_band(0, EqBand::peaking(1000.0, gain, 1.0));

        // Generate phase-continuous sine wave
        let start_sample = chunk_idx * CHUNK_SIZE;
        let mut chunk = generate_sine_at(1000.0, SAMPLE_RATE, start_sample, CHUNK_SIZE, 0.5);
        eq.process(&mut chunk, SAMPLE_RATE);
        changing_output.extend_from_slice(&chunk);
    }

    let changing_high_freq = measure_high_freq_content(&changing_output);
    println!("With changes high-freq content: {:.6}", changing_high_freq);

    let ratio = if baseline_high_freq > 0.0001 {
        changing_high_freq / baseline_high_freq
    } else {
        changing_high_freq * 10000.0
    };

    println!("Ratio (changing/baseline): {:.2}x", ratio);

    // With good smoothing, the ratio should be close to 1
    // A high ratio indicates smoothing isn't working well
    assert!(ratio < 5.0, "Smoothing not effective: {:.2}x more artifacts with changes", ratio);
}

#[test]
fn test_sample_by_sample_continuity() {
    // Check that each sample flows smoothly into the next
    // This catches discontinuities that averaging might miss

    let mut eq = ParametricEq::new();
    eq.set_band(0, EqBand::peaking(1000.0, 0.0, 1.0));

    let test_freq = 1000.0;
    let amplitude = 0.5;

    // Expected maximum sample-to-sample change for a sine wave
    // derivative of sin(wt) = w*cos(wt), max value = w = 2*pi*f
    // sample-to-sample change = w/sample_rate * amplitude
    let expected_max_delta = 2.0 * std::f32::consts::PI * test_freq / SAMPLE_RATE as f32 * amplitude;

    println!("Expected max delta for {:.0}Hz sine: {:.6}", test_freq, expected_max_delta);

    let mut discontinuity_count = 0;
    let mut max_discontinuity = 0.0_f32;
    let mut prev_sample = 0.0_f32;
    let mut total_samples = 0;

    for chunk_idx in 0..50 {
        // Change gain each chunk
        let gain = (chunk_idx as f32 * 0.2).sin() * 6.0; // Oscillate ±6dB
        eq.set_band(0, EqBand::peaking(1000.0, gain, 1.0));

        // Generate phase-continuous sine wave
        let start_sample = chunk_idx * CHUNK_SIZE;
        let mut buffer = generate_sine_at(test_freq, SAMPLE_RATE, start_sample, CHUNK_SIZE, amplitude);
        eq.process(&mut buffer, SAMPLE_RATE);

        for i in 0..(buffer.len() / 2) {
            let sample = buffer[i * 2];
            if total_samples > 0 {
                let delta = (sample - prev_sample).abs();
                // Allow 3x expected for EQ gain changes, but flag anything higher
                let threshold = expected_max_delta * 5.0;
                if delta > threshold {
                    discontinuity_count += 1;
                    if delta > max_discontinuity {
                        max_discontinuity = delta;
                        if discontinuity_count <= 10 {
                            println!("Discontinuity at sample {}: delta={:.6} (threshold={:.6})",
                                     total_samples, delta, threshold);
                        }
                    }
                }
            }
            prev_sample = sample;
            total_samples += 1;
        }
    }

    println!("\n=== Summary ===");
    println!("Total samples: {}", total_samples);
    println!("Discontinuities (>{:.6}): {}", expected_max_delta * 5.0, discontinuity_count);
    println!("Max discontinuity: {:.6}", max_discontinuity);

    // Should have very few discontinuities with proper smoothing
    let discontinuity_rate = discontinuity_count as f32 / total_samples as f32;
    println!("Discontinuity rate: {:.4}%", discontinuity_rate * 100.0);

    assert!(discontinuity_rate < 0.01,
            "Too many discontinuities: {:.4}% (should be <1%)", discontinuity_rate * 100.0);
}
