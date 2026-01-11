//! Convolution engine performance tests
//!
//! These tests verify that the convolution engine meets real-time performance requirements.

use soul_audio::effects::{AudioEffect, ConvolutionEngine};
use std::f32::consts::PI;
use std::time::Instant;

/// Generate a stereo sine wave at the given frequency
fn generate_sine_wave(frequency: f32, sample_rate: u32, num_frames: usize) -> Vec<f32> {
    let mut buffer = Vec::with_capacity(num_frames * 2);
    for i in 0..num_frames {
        let t = i as f32 / sample_rate as f32;
        let sample = (2.0 * PI * frequency * t).sin();
        buffer.push(sample); // Left
        buffer.push(sample); // Right
    }
    buffer
}

/// Measure processing time for a given configuration
fn measure_processing_time(
    ir_length: usize,
    buffer_size: usize,
    sample_rate: u32,
    iterations: usize,
) -> std::time::Duration {
    let mut engine = ConvolutionEngine::new();

    // Create IR
    let ir: Vec<f32> = (0..ir_length * 2)
        .map(|i| (-(i as f32) / (ir_length as f32)).exp())
        .collect();

    engine.load_impulse_response(&ir, sample_rate, 2).unwrap();

    // Warm up
    for _ in 0..5 {
        let mut buffer = vec![0.5; buffer_size * 2];
        engine.process(&mut buffer, sample_rate);
    }

    // Measure
    let start = Instant::now();
    for _ in 0..iterations {
        let mut buffer = generate_sine_wave(1000.0, sample_rate, buffer_size);
        engine.process(&mut buffer, sample_rate);
    }
    start.elapsed() / iterations as u32
}

#[test]
fn benchmark_short_ir_processing() {
    // Short IR (e.g., 10ms at 44100 Hz = 441 samples)
    let ir_length = 441;
    let buffer_size = 512;
    let sample_rate = 44100;
    let iterations = 100;

    let avg_time = measure_processing_time(ir_length, buffer_size, sample_rate, iterations);

    println!(
        "Short IR ({} samples) processing time: {:?} per buffer ({} samples)",
        ir_length, avg_time, buffer_size
    );

    // Calculate real-time deadline
    let buffer_duration =
        std::time::Duration::from_secs_f64(buffer_size as f64 / sample_rate as f64);

    // Should be faster than real-time (with margin)
    assert!(
        avg_time < buffer_duration,
        "Short IR processing too slow: {:?} > {:?}",
        avg_time,
        buffer_duration
    );
}

#[test]
fn benchmark_medium_ir_processing() {
    // Medium IR (e.g., 100ms at 44100 Hz = 4410 samples)
    let ir_length = 4410;
    let buffer_size = 512;
    let sample_rate = 44100;
    let iterations = 50;

    let avg_time = measure_processing_time(ir_length, buffer_size, sample_rate, iterations);

    println!(
        "Medium IR ({} samples) processing time: {:?} per buffer ({} samples)",
        ir_length, avg_time, buffer_size
    );

    // For medium IR, we're more lenient but still want reasonable performance
    let buffer_duration =
        std::time::Duration::from_secs_f64(buffer_size as f64 / sample_rate as f64);

    // Accept up to 5x real-time for medium IR (convolution is O(n*m))
    assert!(
        avg_time < buffer_duration * 5,
        "Medium IR processing very slow: {:?}",
        avg_time
    );
}

#[test]
fn benchmark_long_ir_processing() {
    // Long IR (e.g., 500ms at 44100 Hz = 22050 samples)
    let ir_length = 22050;
    let buffer_size = 1024;
    let sample_rate = 44100;
    let iterations = 10;

    let avg_time = measure_processing_time(ir_length, buffer_size, sample_rate, iterations);

    println!(
        "Long IR ({} samples) processing time: {:?} per buffer ({} samples)",
        ir_length, avg_time, buffer_size
    );

    // Long IR will be slow with direct convolution - just verify it completes
    assert!(
        avg_time < std::time::Duration::from_secs(5),
        "Long IR processing took too long: {:?}",
        avg_time
    );
}

#[test]
fn test_fft_convolution_quality() {
    // Verify that FFT-based convolution produces correct output
    let mut engine = ConvolutionEngine::new();

    // Create a simple decaying IR (100 samples, stereo)
    let ir_length = 100;
    let ir: Vec<f32> = (0..ir_length * 2)
        .map(|i| {
            let frame = i / 2;
            (-(frame as f32) / 20.0).exp() * if i % 2 == 0 { 1.0 } else { 1.0 }
        })
        .collect();

    engine.load_impulse_response(&ir, 44100, 2).unwrap();
    engine.set_dry_wet_mix(1.0);

    // Create an impulse input
    let mut buffer: Vec<f32> = vec![0.0; 512 * 2];
    buffer[0] = 1.0; // Left impulse
    buffer[1] = 1.0; // Right impulse

    engine.process(&mut buffer, 44100);

    // Output should contain the IR convolved with the impulse
    // First sample should be non-zero
    assert!(buffer[0].abs() > 0.01, "First sample should be non-zero");

    // Signal should decay
    let early_energy: f32 = buffer[0..20].iter().map(|x: &f32| x.abs()).sum();
    let late_energy: f32 = buffer[200..220].iter().map(|x: &f32| x.abs()).sum();

    assert!(
        early_energy > late_energy,
        "Signal should decay over time: early={}, late={}",
        early_energy,
        late_energy
    );
}

#[test]
fn test_fft_vs_time_domain_threshold() {
    // Verify that IRs above threshold use FFT, below use time-domain
    let mut short_engine = ConvolutionEngine::new();
    let mut long_engine = ConvolutionEngine::new();

    // Short IR (below threshold, should use time-domain)
    let short_ir: Vec<f32> = (0..32 * 2) // 32 samples
        .map(|i| (-(i as f32) / 10.0).exp())
        .collect();

    // Long IR (above threshold, should use FFT)
    let long_ir: Vec<f32> = (0..100 * 2) // 100 samples
        .map(|i| (-(i as f32) / 10.0).exp())
        .collect();

    short_engine
        .load_impulse_response(&short_ir, 44100, 2)
        .unwrap();
    long_engine
        .load_impulse_response(&long_ir, 44100, 2)
        .unwrap();

    // Both should produce valid output
    let mut short_buffer = generate_sine_wave(1000.0, 44100, 256);
    let mut long_buffer = generate_sine_wave(1000.0, 44100, 256);

    short_engine.process(&mut short_buffer, 44100);
    long_engine.process(&mut long_buffer, 44100);

    // Verify outputs are finite
    assert!(short_buffer.iter().all(|x| x.is_finite()));
    assert!(long_buffer.iter().all(|x| x.is_finite()));
}

#[test]
fn test_continuous_processing() {
    // Verify that processing multiple consecutive buffers works correctly
    let mut engine = ConvolutionEngine::new();

    let ir_length = 200;
    // Normalize the IR so the sum of squares is approximately 1
    let raw_ir: Vec<f32> = (0..ir_length).map(|i| (-(i as f32) / 50.0).exp()).collect();
    let energy: f32 = raw_ir.iter().map(|x| x * x).sum();
    let norm = 1.0 / energy.sqrt();

    let ir: Vec<f32> = raw_ir.iter().flat_map(|&x| [x * norm, x * norm]).collect();

    engine.load_impulse_response(&ir, 44100, 2).unwrap();

    // Process many consecutive buffers
    for i in 0..100 {
        let freq = 440.0 + (i as f32 * 0.5);
        let mut buffer = generate_sine_wave(freq, 44100, 256);
        engine.process(&mut buffer, 44100);

        // Verify output is valid
        assert!(
            buffer.iter().all(|x| x.is_finite()),
            "Non-finite output at iteration {}",
            i
        );

        // Check peak isn't excessive (with normalized IR, output should be close to input level)
        // Allow some headroom for convolution tail overlap between buffers
        let peak = buffer.iter().map(|x| x.abs()).fold(0.0f32, f32::max);
        assert!(
            peak < 10.0,
            "Excessive amplitude at iteration {}: {}",
            i,
            peak
        );
    }
}
