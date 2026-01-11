//! Tests for continuous parameter changes (dragging sliders, automation)
//!
//! These tests simulate real-world scenarios where parameters are continuously
//! changed over time, like dragging an EQ band or moving a slider slowly.
//! This can cause sizzle/artifacts that don't show up in single-change tests.

use soul_audio::effects::{
    AudioEffect, Crossfeed, CrossfeedPreset, EqBand, GraphicEq, Limiter, ParametricEq,
    StereoEnhancer,
};

const SAMPLE_RATE: u32 = 44100;
const CHUNK_SIZE: usize = 512; // Typical audio buffer size

/// Generate a sine wave chunk
fn generate_sine_chunk(frequency: f32, sample_rate: u32, start_sample: usize, num_samples: usize) -> Vec<f32> {
    let mut buffer = Vec::with_capacity(num_samples * 2);
    for i in 0..num_samples {
        let t = (start_sample + i) as f32 / sample_rate as f32;
        let sample = (2.0 * std::f32::consts::PI * frequency * t).sin() * 0.5;
        buffer.push(sample); // Left
        buffer.push(sample); // Right
    }
    buffer
}

/// Detect high-frequency artifacts (sizzle) in audio
/// Returns the amount of high-frequency energy relative to the fundamental
fn detect_sizzle(buffer: &[f32], fundamental_freq: f32, sample_rate: u32) -> f32 {
    // Simple approach: look for rapid sample-to-sample changes that exceed
    // what we'd expect from a smooth sine wave

    let mut max_derivative = 0.0_f32;
    let mut total_derivative = 0.0_f32;
    let mut count = 0;

    // Expected max derivative for a sine wave at this frequency
    // d/dt[A*sin(2*pi*f*t)] = A*2*pi*f*cos(2*pi*f*t)
    // Max value = A * 2 * pi * f / sample_rate
    let amplitude = 0.5;
    let expected_max_derivative = amplitude * 2.0 * std::f32::consts::PI * fundamental_freq / sample_rate as f32;

    // Check left channel derivatives
    for i in 1..(buffer.len() / 2) {
        let left_curr = buffer[i * 2];
        let left_prev = buffer[(i - 1) * 2];
        let derivative = (left_curr - left_prev).abs();

        max_derivative = max_derivative.max(derivative);
        total_derivative += derivative;
        count += 1;
    }

    let avg_derivative = if count > 0 { total_derivative / count as f32 } else { 0.0 };

    // Return ratio of max derivative to expected - values > 2-3x indicate artifacts
    max_derivative / expected_max_derivative
}

/// Detect discontinuities (clicks/pops) in audio
fn detect_discontinuities(buffer: &[f32], threshold: f32) -> Vec<usize> {
    let mut discontinuities = Vec::new();

    for i in 1..(buffer.len() / 2) {
        let left_curr = buffer[i * 2];
        let left_prev = buffer[(i - 1) * 2];
        let diff = (left_curr - left_prev).abs();

        if diff > threshold {
            discontinuities.push(i);
        }
    }

    discontinuities
}

/// Calculate THD+N (Total Harmonic Distortion + Noise) using simple time-domain analysis
fn calculate_distortion(buffer: &[f32], fundamental_freq: f32, sample_rate: u32) -> f32 {
    // Extract left channel
    let left: Vec<f32> = buffer.iter().step_by(2).copied().collect();

    if left.len() < 256 {
        return 0.0;
    }

    // Generate ideal sine wave at same frequency and phase-match
    let mut ideal = Vec::with_capacity(left.len());
    let period_samples = sample_rate as f32 / fundamental_freq;

    // Find best phase alignment by checking first zero crossing
    let mut best_phase = 0.0_f32;
    let mut min_error = f32::MAX;

    for phase_step in 0..32 {
        let phase = phase_step as f32 * std::f32::consts::PI / 16.0;
        let mut error = 0.0;
        for (i, &sample) in left.iter().take(256).enumerate() {
            let t = i as f32 / sample_rate as f32;
            let ideal_sample = (2.0 * std::f32::consts::PI * fundamental_freq * t + phase).sin() * 0.5;
            error += (sample - ideal_sample).powi(2);
        }
        if error < min_error {
            min_error = error;
            best_phase = phase;
        }
    }

    // Generate phase-aligned ideal signal
    for i in 0..left.len() {
        let t = i as f32 / sample_rate as f32;
        let ideal_sample = (2.0 * std::f32::consts::PI * fundamental_freq * t + best_phase).sin() * 0.5;
        ideal.push(ideal_sample);
    }

    // Calculate error (distortion + noise)
    let mut signal_power = 0.0_f32;
    let mut error_power = 0.0_f32;

    for (i, &sample) in left.iter().enumerate() {
        signal_power += ideal[i].powi(2);
        error_power += (sample - ideal[i]).powi(2);
    }

    if signal_power > 0.0 {
        (error_power / signal_power).sqrt() * 100.0 // Return as percentage
    } else {
        0.0
    }
}

// =============================================================================
// CONTINUOUS PARAMETER CHANGE TESTS
// =============================================================================

#[test]
fn test_eq_continuous_frequency_drag() {
    // Simulate dragging an EQ band frequency from 200Hz to 2000Hz over 2 seconds
    let mut eq = ParametricEq::new();
    eq.set_band(0, EqBand::peaking(500.0, 6.0, 1.0));

    let duration_seconds = 2.0;
    let total_samples = (SAMPLE_RATE as f32 * duration_seconds) as usize;
    let num_chunks = total_samples / CHUNK_SIZE;

    let start_freq = 200.0_f32;
    let end_freq = 2000.0_f32;

    let test_tone = 440.0; // A4
    let mut max_sizzle = 0.0_f32;
    let mut total_distortion = 0.0_f32;
    let mut all_discontinuities = Vec::new();

    println!("EQ Frequency Drag Test: {} Hz -> {} Hz over {} chunks", start_freq, end_freq, num_chunks);

    for chunk_idx in 0..num_chunks {
        // Calculate current frequency (linear interpolation)
        let progress = chunk_idx as f32 / num_chunks as f32;
        let current_freq = start_freq + (end_freq - start_freq) * progress;

        // Update EQ band frequency (simulating continuous drag)
        eq.set_band(0, EqBand::peaking(current_freq, 6.0, 1.0));

        // Generate and process audio
        let start_sample = chunk_idx * CHUNK_SIZE;
        let mut buffer = generate_sine_chunk(test_tone, SAMPLE_RATE, start_sample, CHUNK_SIZE);

        eq.process(&mut buffer, SAMPLE_RATE);

        // Analyze for artifacts
        let sizzle = detect_sizzle(&buffer, test_tone, SAMPLE_RATE);
        let distortion = calculate_distortion(&buffer, test_tone, SAMPLE_RATE);
        let discontinuities = detect_discontinuities(&buffer, 0.3);

        max_sizzle = max_sizzle.max(sizzle);
        total_distortion += distortion;

        if !discontinuities.is_empty() {
            all_discontinuities.extend(discontinuities.iter().map(|&d| chunk_idx * CHUNK_SIZE + d));
        }

        // Debug output for high sizzle chunks
        if sizzle > 3.0 {
            println!("  Chunk {}: freq={:.0}Hz, sizzle={:.2}x, distortion={:.2}%",
                     chunk_idx, current_freq, sizzle, distortion);
        }
    }

    let avg_distortion = total_distortion / num_chunks as f32;

    println!("\nResults:");
    println!("  Max sizzle ratio: {:.2}x expected", max_sizzle);
    println!("  Avg distortion: {:.2}% (note: includes intentional EQ effect)", avg_distortion);
    println!("  Discontinuities: {}", all_discontinuities.len());

    // Thresholds based on acceptable audio quality
    // Sizzle > 5x indicates serious high-frequency artifacts
    assert!(max_sizzle < 5.0,
            "Excessive sizzle during frequency drag: {:.2}x (max 5.0x)", max_sizzle);
    // Note: distortion metric includes intended EQ amplitude changes as the band sweeps
    // through the test frequency, so we use a higher threshold. The key metric is sizzle.
    assert!(avg_distortion < 50.0,
            "Excessive distortion during frequency drag: {:.2}% (max 50%)", avg_distortion);
    assert!(all_discontinuities.len() < 10,
            "Too many discontinuities during frequency drag: {} (max 10)", all_discontinuities.len());
}

#[test]
fn test_eq_continuous_gain_drag() {
    // Simulate dragging EQ gain from -12dB to +12dB over 2 seconds
    let mut eq = ParametricEq::new();
    eq.set_band(0, EqBand::peaking(1000.0, 0.0, 1.0));

    let duration_seconds = 2.0;
    let total_samples = (SAMPLE_RATE as f32 * duration_seconds) as usize;
    let num_chunks = total_samples / CHUNK_SIZE;

    let start_gain = -12.0_f32;
    let end_gain = 12.0_f32;

    let test_tone = 1000.0; // Same as EQ center frequency for maximum effect
    let mut max_sizzle = 0.0_f32;
    let mut max_discontinuity = 0.0_f32;

    println!("EQ Gain Drag Test: {} dB -> {} dB over {} chunks", start_gain, end_gain, num_chunks);

    for chunk_idx in 0..num_chunks {
        let progress = chunk_idx as f32 / num_chunks as f32;
        let current_gain = start_gain + (end_gain - start_gain) * progress;

        eq.set_band(0, EqBand::peaking(1000.0, current_gain, 1.0));

        let start_sample = chunk_idx * CHUNK_SIZE;
        let mut buffer = generate_sine_chunk(test_tone, SAMPLE_RATE, start_sample, CHUNK_SIZE);

        eq.process(&mut buffer, SAMPLE_RATE);

        let sizzle = detect_sizzle(&buffer, test_tone, SAMPLE_RATE);
        max_sizzle = max_sizzle.max(sizzle);

        // Check for large sample-to-sample jumps
        for i in 1..(buffer.len() / 2) {
            let diff = (buffer[i * 2] - buffer[(i - 1) * 2]).abs();
            max_discontinuity = max_discontinuity.max(diff);
        }

        if sizzle > 3.0 {
            println!("  Chunk {}: gain={:.1}dB, sizzle={:.2}x", chunk_idx, current_gain, sizzle);
        }
    }

    println!("\nResults:");
    println!("  Max sizzle ratio: {:.2}x", max_sizzle);
    println!("  Max discontinuity: {:.4}", max_discontinuity);

    assert!(max_sizzle < 5.0, "Excessive sizzle during gain drag: {:.2}x", max_sizzle);
    assert!(max_discontinuity < 0.5, "Large discontinuity detected: {:.4}", max_discontinuity);
}

#[test]
fn test_eq_continuous_q_drag() {
    // Simulate dragging Q from 0.5 (wide) to 10.0 (narrow) over 2 seconds
    let mut eq = ParametricEq::new();
    eq.set_band(0, EqBand::peaking(1000.0, 6.0, 0.5));

    let duration_seconds = 2.0;
    let total_samples = (SAMPLE_RATE as f32 * duration_seconds) as usize;
    let num_chunks = total_samples / CHUNK_SIZE;

    let start_q = 0.5_f32;
    let end_q = 10.0_f32;

    let test_tone = 1000.0;
    let mut max_sizzle = 0.0_f32;

    println!("EQ Q Drag Test: Q={} -> Q={} over {} chunks", start_q, end_q, num_chunks);

    for chunk_idx in 0..num_chunks {
        let progress = chunk_idx as f32 / num_chunks as f32;
        let current_q = start_q + (end_q - start_q) * progress;

        eq.set_band(0, EqBand::peaking(1000.0, 6.0, current_q));

        let start_sample = chunk_idx * CHUNK_SIZE;
        let mut buffer = generate_sine_chunk(test_tone, SAMPLE_RATE, start_sample, CHUNK_SIZE);

        eq.process(&mut buffer, SAMPLE_RATE);

        let sizzle = detect_sizzle(&buffer, test_tone, SAMPLE_RATE);
        max_sizzle = max_sizzle.max(sizzle);

        if sizzle > 3.0 {
            println!("  Chunk {}: Q={:.2}, sizzle={:.2}x", chunk_idx, current_q, sizzle);
        }
    }

    println!("\nResults:");
    println!("  Max sizzle ratio: {:.2}x", max_sizzle);

    assert!(max_sizzle < 5.0, "Excessive sizzle during Q drag: {:.2}x", max_sizzle);
}

#[test]
fn test_graphic_eq_continuous_band_drag() {
    // Simulate dragging a graphic EQ band up and down
    let mut eq = GraphicEq::new_10_band();

    let duration_seconds = 2.0;
    let total_samples = (SAMPLE_RATE as f32 * duration_seconds) as usize;
    let num_chunks = total_samples / CHUNK_SIZE;

    let band_index = 4; // 1kHz band
    let test_tone = 1000.0;
    let mut max_sizzle = 0.0_f32;

    println!("Graphic EQ Band Drag Test: Band {} (1kHz) sweep over {} chunks", band_index, num_chunks);

    for chunk_idx in 0..num_chunks {
        // Oscillate gain between -12 and +12 dB (like wiggling a slider)
        let progress = chunk_idx as f32 / num_chunks as f32;
        let gain = (progress * 4.0 * std::f32::consts::PI).sin() * 12.0;

        eq.set_band_gain(band_index, gain);

        let start_sample = chunk_idx * CHUNK_SIZE;
        let mut buffer = generate_sine_chunk(test_tone, SAMPLE_RATE, start_sample, CHUNK_SIZE);

        eq.process(&mut buffer, SAMPLE_RATE);

        let sizzle = detect_sizzle(&buffer, test_tone, SAMPLE_RATE);
        max_sizzle = max_sizzle.max(sizzle);

        if sizzle > 3.0 {
            println!("  Chunk {}: gain={:.1}dB, sizzle={:.2}x", chunk_idx, gain, sizzle);
        }
    }

    println!("\nResults:");
    println!("  Max sizzle ratio: {:.2}x", max_sizzle);

    assert!(max_sizzle < 5.0, "Excessive sizzle during graphic EQ drag: {:.2}x", max_sizzle);
}

#[test]
fn test_limiter_continuous_threshold_drag() {
    // Simulate dragging limiter threshold from -20dB to 0dB
    let mut limiter = Limiter::new();

    let duration_seconds = 2.0;
    let total_samples = (SAMPLE_RATE as f32 * duration_seconds) as usize;
    let num_chunks = total_samples / CHUNK_SIZE;

    let start_threshold = -20.0_f32;
    let end_threshold = -0.1_f32;

    let test_tone = 440.0;
    let mut max_sizzle = 0.0_f32;
    let mut max_discontinuity = 0.0_f32;

    println!("Limiter Threshold Drag Test: {} dB -> {} dB over {} chunks",
             start_threshold, end_threshold, num_chunks);

    for chunk_idx in 0..num_chunks {
        let progress = chunk_idx as f32 / num_chunks as f32;
        let current_threshold = start_threshold + (end_threshold - start_threshold) * progress;

        limiter.set_threshold(current_threshold);

        let start_sample = chunk_idx * CHUNK_SIZE;
        let mut buffer = generate_sine_chunk(test_tone, SAMPLE_RATE, start_sample, CHUNK_SIZE);

        limiter.process(&mut buffer, SAMPLE_RATE);

        let sizzle = detect_sizzle(&buffer, test_tone, SAMPLE_RATE);
        max_sizzle = max_sizzle.max(sizzle);

        for i in 1..(buffer.len() / 2) {
            let diff = (buffer[i * 2] - buffer[(i - 1) * 2]).abs();
            max_discontinuity = max_discontinuity.max(diff);
        }

        if sizzle > 3.0 {
            println!("  Chunk {}: threshold={:.1}dB, sizzle={:.2}x",
                     chunk_idx, current_threshold, sizzle);
        }
    }

    println!("\nResults:");
    println!("  Max sizzle ratio: {:.2}x", max_sizzle);
    println!("  Max discontinuity: {:.4}", max_discontinuity);

    assert!(max_sizzle < 5.0, "Excessive sizzle during threshold drag: {:.2}x", max_sizzle);
}

#[test]
fn test_stereo_continuous_width_drag() {
    // Simulate dragging stereo width from 0 (mono) to 2.0 (wide)
    let mut stereo = StereoEnhancer::new();

    let duration_seconds = 2.0;
    let total_samples = (SAMPLE_RATE as f32 * duration_seconds) as usize;
    let num_chunks = total_samples / CHUNK_SIZE;

    let start_width = 0.0_f32;
    let end_width = 2.0_f32;

    let test_tone = 440.0;
    let mut max_sizzle = 0.0_f32;
    let mut max_discontinuity = 0.0_f32;

    println!("Stereo Width Drag Test: {} -> {} over {} chunks", start_width, end_width, num_chunks);

    for chunk_idx in 0..num_chunks {
        let progress = chunk_idx as f32 / num_chunks as f32;
        let current_width = start_width + (end_width - start_width) * progress;

        stereo.set_width(current_width);

        let start_sample = chunk_idx * CHUNK_SIZE;
        let mut buffer = generate_sine_chunk(test_tone, SAMPLE_RATE, start_sample, CHUNK_SIZE);

        stereo.process(&mut buffer, SAMPLE_RATE);

        let sizzle = detect_sizzle(&buffer, test_tone, SAMPLE_RATE);
        max_sizzle = max_sizzle.max(sizzle);

        for i in 1..(buffer.len() / 2) {
            let diff = (buffer[i * 2] - buffer[(i - 1) * 2]).abs();
            max_discontinuity = max_discontinuity.max(diff);
        }

        if sizzle > 3.0 {
            println!("  Chunk {}: width={:.2}, sizzle={:.2}x", chunk_idx, current_width, sizzle);
        }
    }

    println!("\nResults:");
    println!("  Max sizzle ratio: {:.2}x", max_sizzle);
    println!("  Max discontinuity: {:.4}", max_discontinuity);

    assert!(max_sizzle < 5.0, "Excessive sizzle during width drag: {:.2}x", max_sizzle);
}

#[test]
fn test_crossfeed_continuous_level_drag() {
    // Simulate dragging crossfeed level from -12dB to -3dB
    let mut crossfeed = Crossfeed::new();

    let duration_seconds = 2.0;
    let total_samples = (SAMPLE_RATE as f32 * duration_seconds) as usize;
    let num_chunks = total_samples / CHUNK_SIZE;

    let start_level = -12.0_f32;
    let end_level = -3.0_f32;

    let test_tone = 440.0;
    let mut max_sizzle = 0.0_f32;

    println!("Crossfeed Level Drag Test: {} dB -> {} dB over {} chunks",
             start_level, end_level, num_chunks);

    for chunk_idx in 0..num_chunks {
        let progress = chunk_idx as f32 / num_chunks as f32;
        let current_level = start_level + (end_level - start_level) * progress;

        crossfeed.set_level_db(current_level);

        let start_sample = chunk_idx * CHUNK_SIZE;
        let mut buffer = generate_sine_chunk(test_tone, SAMPLE_RATE, start_sample, CHUNK_SIZE);

        crossfeed.process(&mut buffer, SAMPLE_RATE);

        let sizzle = detect_sizzle(&buffer, test_tone, SAMPLE_RATE);
        max_sizzle = max_sizzle.max(sizzle);

        if sizzle > 3.0 {
            println!("  Chunk {}: level={:.1}dB, sizzle={:.2}x", chunk_idx, current_level, sizzle);
        }
    }

    println!("\nResults:");
    println!("  Max sizzle ratio: {:.2}x", max_sizzle);

    assert!(max_sizzle < 5.0, "Excessive sizzle during level drag: {:.2}x", max_sizzle);
}

#[test]
fn test_eq_slow_drag_simulation() {
    // Simulate a very slow drag (like a user carefully adjusting)
    // This tests whether the smoothing window is appropriate
    let mut eq = ParametricEq::new();
    eq.set_band(0, EqBand::peaking(1000.0, 0.0, 1.0));

    // Simulate 5 seconds of slow dragging, changing by small amounts each frame
    let duration_seconds = 5.0;
    let total_samples = (SAMPLE_RATE as f32 * duration_seconds) as usize;
    let num_chunks = total_samples / CHUNK_SIZE;

    let test_tone = 1000.0;
    let mut prev_output_sample = 0.0_f32;
    let mut max_chunk_boundary_jump = 0.0_f32;
    let mut sizzle_events = 0;

    println!("EQ Slow Drag Simulation: {} chunks, small incremental changes", num_chunks);

    for chunk_idx in 0..num_chunks {
        // Slowly ramp gain from 0 to +6dB over entire duration
        let progress = chunk_idx as f32 / num_chunks as f32;
        let current_gain = progress * 6.0;

        eq.set_band(0, EqBand::peaking(1000.0, current_gain, 1.0));

        let start_sample = chunk_idx * CHUNK_SIZE;
        let mut buffer = generate_sine_chunk(test_tone, SAMPLE_RATE, start_sample, CHUNK_SIZE);

        eq.process(&mut buffer, SAMPLE_RATE);

        // Check for discontinuity at chunk boundary
        if chunk_idx > 0 {
            let first_sample = buffer[0];
            let boundary_jump = (first_sample - prev_output_sample).abs();
            max_chunk_boundary_jump = max_chunk_boundary_jump.max(boundary_jump);

            // Expected max jump for smooth sine at 1kHz
            let expected_max = 0.5 * 2.0 * std::f32::consts::PI * test_tone / SAMPLE_RATE as f32;
            if boundary_jump > expected_max * 3.0 {
                sizzle_events += 1;
                if sizzle_events <= 5 {
                    println!("  Chunk boundary {}: jump={:.4} (expected max {:.4})",
                             chunk_idx, boundary_jump, expected_max);
                }
            }
        }

        // Save last sample for next chunk boundary check
        prev_output_sample = buffer[buffer.len() - 2]; // Last left sample
    }

    println!("\nResults:");
    println!("  Max chunk boundary jump: {:.4}", max_chunk_boundary_jump);
    println!("  Sizzle events: {}", sizzle_events);

    // During slow dragging, we shouldn't have many sizzle events
    assert!(sizzle_events < 20,
            "Too many sizzle events during slow drag: {} (max 20)", sizzle_events);
}

#[test]
fn test_eq_rapid_wiggle_simulation() {
    // Simulate rapidly wiggling a parameter (like nervous mouse movement)
    let mut eq = ParametricEq::new();
    eq.set_band(0, EqBand::peaking(1000.0, 0.0, 1.0));

    let duration_seconds = 2.0;
    let total_samples = (SAMPLE_RATE as f32 * duration_seconds) as usize;
    let num_chunks = total_samples / CHUNK_SIZE;

    let test_tone = 1000.0;
    let mut max_sizzle = 0.0_f32;

    println!("EQ Rapid Wiggle Test: fast oscillating changes over {} chunks", num_chunks);

    for chunk_idx in 0..num_chunks {
        // Rapid oscillation of gain (simulating jittery mouse)
        let t = chunk_idx as f32 * 0.1;
        let gain = (t * 20.0).sin() * 3.0 + (t * 7.0).sin() * 2.0; // Complex oscillation

        eq.set_band(0, EqBand::peaking(1000.0, gain, 1.0));

        let start_sample = chunk_idx * CHUNK_SIZE;
        let mut buffer = generate_sine_chunk(test_tone, SAMPLE_RATE, start_sample, CHUNK_SIZE);

        eq.process(&mut buffer, SAMPLE_RATE);

        let sizzle = detect_sizzle(&buffer, test_tone, SAMPLE_RATE);
        max_sizzle = max_sizzle.max(sizzle);
    }

    println!("\nResults:");
    println!("  Max sizzle ratio: {:.2}x", max_sizzle);

    // Even with rapid changes, sizzle should be controlled by smoothing
    assert!(max_sizzle < 8.0, "Excessive sizzle during rapid wiggle: {:.2}x", max_sizzle);
}

#[test]
fn test_multiple_effects_continuous_changes() {
    // Test multiple effects being adjusted simultaneously
    let mut eq = ParametricEq::new();
    eq.set_band(0, EqBand::peaking(1000.0, 0.0, 1.0));

    let mut stereo = StereoEnhancer::new();
    let mut limiter = Limiter::new();

    let duration_seconds = 2.0;
    let total_samples = (SAMPLE_RATE as f32 * duration_seconds) as usize;
    let num_chunks = total_samples / CHUNK_SIZE;

    let test_tone = 440.0;
    let mut max_sizzle = 0.0_f32;

    println!("Multiple Effects Continuous Test: {} chunks", num_chunks);

    for chunk_idx in 0..num_chunks {
        let progress = chunk_idx as f32 / num_chunks as f32;

        // Change all effects simultaneously
        eq.set_band(0, EqBand::peaking(1000.0, progress * 6.0 - 3.0, 1.0));
        stereo.set_width(0.5 + progress);
        limiter.set_threshold(-10.0 + progress * 9.0);

        let start_sample = chunk_idx * CHUNK_SIZE;
        let mut buffer = generate_sine_chunk(test_tone, SAMPLE_RATE, start_sample, CHUNK_SIZE);

        // Process through all effects
        eq.process(&mut buffer, SAMPLE_RATE);
        stereo.process(&mut buffer, SAMPLE_RATE);
        limiter.process(&mut buffer, SAMPLE_RATE);

        let sizzle = detect_sizzle(&buffer, test_tone, SAMPLE_RATE);
        max_sizzle = max_sizzle.max(sizzle);

        if sizzle > 4.0 {
            println!("  Chunk {}: sizzle={:.2}x", chunk_idx, sizzle);
        }
    }

    println!("\nResults:");
    println!("  Max sizzle ratio: {:.2}x", max_sizzle);

    assert!(max_sizzle < 8.0, "Excessive sizzle with multiple effects: {:.2}x", max_sizzle);
}

#[test]
fn test_eq_frequency_sweep_both_directions() {
    // Test sweeping frequency both up and down to catch asymmetric issues
    let mut eq = ParametricEq::new();

    let test_tone = 440.0;
    let mut max_sizzle_up = 0.0_f32;
    let mut max_sizzle_down = 0.0_f32;

    // Sweep up
    println!("EQ Frequency Sweep Up Test");
    eq.reset();
    eq.set_band(0, EqBand::peaking(200.0, 6.0, 1.0));

    for chunk_idx in 0..100 {
        let freq = 200.0 + chunk_idx as f32 * 18.0; // 200 -> 2000 Hz
        eq.set_band(0, EqBand::peaking(freq, 6.0, 1.0));

        let mut buffer = generate_sine_chunk(test_tone, SAMPLE_RATE, chunk_idx * CHUNK_SIZE, CHUNK_SIZE);
        eq.process(&mut buffer, SAMPLE_RATE);

        let sizzle = detect_sizzle(&buffer, test_tone, SAMPLE_RATE);
        max_sizzle_up = max_sizzle_up.max(sizzle);
    }

    // Sweep down
    println!("EQ Frequency Sweep Down Test");
    eq.reset();
    eq.set_band(0, EqBand::peaking(2000.0, 6.0, 1.0));

    for chunk_idx in 0..100 {
        let freq = 2000.0 - chunk_idx as f32 * 18.0; // 2000 -> 200 Hz
        eq.set_band(0, EqBand::peaking(freq, 6.0, 1.0));

        let mut buffer = generate_sine_chunk(test_tone, SAMPLE_RATE, chunk_idx * CHUNK_SIZE, CHUNK_SIZE);
        eq.process(&mut buffer, SAMPLE_RATE);

        let sizzle = detect_sizzle(&buffer, test_tone, SAMPLE_RATE);
        max_sizzle_down = max_sizzle_down.max(sizzle);
    }

    println!("\nResults:");
    println!("  Max sizzle (sweep up): {:.2}x", max_sizzle_up);
    println!("  Max sizzle (sweep down): {:.2}x", max_sizzle_down);

    assert!(max_sizzle_up < 5.0, "Excessive sizzle during upward sweep: {:.2}x", max_sizzle_up);
    assert!(max_sizzle_down < 5.0, "Excessive sizzle during downward sweep: {:.2}x", max_sizzle_down);
}
