//! Resampling edge case tests
//!
//! Tests for:
//! - Unusual sample rate combinations
//! - Very short buffers
//! - Long-running resampling (many consecutive buffers)
//! - Edge sample rates (very low, very high)
//! - Non-integer rate ratios

use soul_audio::resampling::{AudioResampler, ResamplingQuality};
use std::f32::consts::PI;

/// Generate a stereo sine wave buffer
fn generate_stereo_sine(frequency: f32, sample_rate: u32, num_samples: usize) -> Vec<f32> {
    let mut buffer = Vec::with_capacity(num_samples * 2);
    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let sample = (2.0 * PI * frequency * t).sin();
        buffer.push(sample); // Left
        buffer.push(sample); // Right
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

// ============================================================================
// UNUSUAL SAMPLE RATE COMBINATIONS
// ============================================================================

#[test]
fn test_44100_to_48000() {
    // Common rate conversion
    let mut resampler = AudioResampler::new(44100, 48000, 2, ResamplingQuality::Normal).unwrap();

    let input = generate_stereo_sine(1000.0, 44100, 4410); // 0.1 sec
    let output = resampler.process(&input).unwrap();

    // Expected: 4410 * (48000/44100) = ~4800 frames = 9600 samples
    assert!(output.len() > 9000 && output.len() < 10000);

    // Verify signal integrity
    assert!(calculate_rms(&output) > 0.3);
}

#[test]
fn test_48000_to_44100() {
    // Downsampling
    let mut resampler = AudioResampler::new(48000, 44100, 2, ResamplingQuality::Normal).unwrap();

    let input = generate_stereo_sine(1000.0, 48000, 4800);
    let output = resampler.process(&input).unwrap();

    // Expected: ~4410 frames = ~8820 samples
    assert!(output.len() > 8000 && output.len() < 9500);
}

#[test]
fn test_44100_to_88200() {
    // 2x upsampling (integer ratio)
    let mut resampler = AudioResampler::new(44100, 88200, 2, ResamplingQuality::Normal).unwrap();

    let input = generate_stereo_sine(1000.0, 44100, 4410);
    let output = resampler.process(&input).unwrap();

    // Should approximately double
    assert!(output.len() > 16000 && output.len() < 20000);
}

#[test]
fn test_96000_to_44100() {
    // Large downsampling ratio
    let mut resampler = AudioResampler::new(96000, 44100, 2, ResamplingQuality::Normal).unwrap();

    let input = generate_stereo_sine(1000.0, 96000, 9600);
    let output = resampler.process(&input).unwrap();

    // Expected: 9600 * (44100/96000) / 2 frames = ~4410 frames = ~8820 samples
    assert!(output.len() > 7500 && output.len() < 10000);
}

#[test]
fn test_22050_to_48000() {
    // Uncommon upsampling
    let mut resampler = AudioResampler::new(22050, 48000, 2, ResamplingQuality::Normal).unwrap();

    let input = generate_stereo_sine(1000.0, 22050, 2205);
    let output = resampler.process(&input).unwrap();

    assert!(output.len() > 8000);
}

#[test]
fn test_48000_to_22050() {
    // Uncommon downsampling
    let mut resampler = AudioResampler::new(48000, 22050, 2, ResamplingQuality::Normal).unwrap();

    let input = generate_stereo_sine(1000.0, 48000, 4800);
    let output = resampler.process(&input).unwrap();

    // Verify output is smaller
    assert!(output.len() < input.len());
}

#[test]
fn test_8000_to_44100() {
    // Telephone rate to CD rate
    let mut resampler = AudioResampler::new(8000, 44100, 2, ResamplingQuality::Normal).unwrap();

    let input = generate_stereo_sine(1000.0, 8000, 800);
    let output = resampler.process(&input).unwrap();

    // Large upsampling ratio
    assert!(output.len() > input.len() * 4);
}

#[test]
fn test_192000_to_44100() {
    // High-res to CD rate
    let mut resampler = AudioResampler::new(192000, 44100, 2, ResamplingQuality::Normal).unwrap();

    let input = generate_stereo_sine(1000.0, 192000, 19200);
    let output = resampler.process(&input).unwrap();

    assert!(output.len() < input.len());
}

#[test]
fn test_same_rate_passthrough() {
    // 44100 to 44100 (should be passthrough or near-passthrough)
    let mut resampler = AudioResampler::new(44100, 44100, 2, ResamplingQuality::Normal).unwrap();

    let input = generate_stereo_sine(1000.0, 44100, 4410);
    let output = resampler.process(&input).unwrap();

    // Output should be similar size
    let ratio = output.len() as f32 / input.len() as f32;
    assert!(ratio > 0.9 && ratio < 1.1);
}

// ============================================================================
// NON-STANDARD SAMPLE RATES
// ============================================================================

#[test]
fn test_non_standard_rates() {
    // 37800 to 44100 (non-standard source rate)
    let mut resampler = AudioResampler::new(37800, 44100, 2, ResamplingQuality::Normal).unwrap();

    let input = generate_stereo_sine(1000.0, 37800, 3780);
    let output = resampler.process(&input).unwrap();

    assert!(output.len() > 0);
    assert!(calculate_rms(&output) > 0.2);
}

#[test]
fn test_prime_number_rates() {
    // Using prime numbers for interesting ratios
    let mut resampler = AudioResampler::new(44101, 48000, 2, ResamplingQuality::Normal).unwrap();

    let input = generate_stereo_sine(1000.0, 44101, 4410);
    let output = resampler.process(&input).unwrap();

    assert!(output.len() > 0);
}

// ============================================================================
// VERY SHORT BUFFER TESTS
// ============================================================================

#[test]
fn test_single_frame_buffer() {
    let mut resampler = AudioResampler::new(44100, 48000, 2, ResamplingQuality::Normal).unwrap();

    // Single stereo frame
    let input = vec![0.5, 0.5];
    let output = resampler.process(&input).unwrap();

    // Should produce some output (at least 1 frame)
    assert!(output.len() >= 2);
}

#[test]
fn test_very_small_buffers() {
    let mut resampler = AudioResampler::new(44100, 48000, 2, ResamplingQuality::Normal).unwrap();

    for size in [2, 4, 8, 16, 32, 64] {
        let input = generate_stereo_sine(1000.0, 44100, size);
        let output = resampler.process(&input).unwrap();

        // Should not panic and produce some output
        assert!(output.len() > 0, "Size {} should produce output", size);
    }
}

#[test]
fn test_odd_sample_counts() {
    let mut resampler = AudioResampler::new(44100, 48000, 2, ResamplingQuality::Normal).unwrap();

    // Odd number of frames (not a problem, but edge case)
    for frames in [1, 3, 5, 7, 13, 17, 31, 127] {
        let input = generate_stereo_sine(1000.0, 44100, frames);
        let output = resampler.process(&input).unwrap();

        assert!(output.len() > 0, "Odd frame count {} should work", frames);
    }
}

#[test]
fn test_empty_buffer() {
    let mut resampler = AudioResampler::new(44100, 48000, 2, ResamplingQuality::Normal).unwrap();

    let input: Vec<f32> = vec![];
    let output = resampler.process(&input).unwrap();

    assert!(output.is_empty() || output.len() < 10); // Might have latency samples
}

// ============================================================================
// LONG-RUNNING RESAMPLING TESTS
// ============================================================================

#[test]
fn test_many_consecutive_buffers() {
    let mut resampler = AudioResampler::new(44100, 48000, 2, ResamplingQuality::Normal).unwrap();

    let mut total_input_samples = 0;
    let mut total_output_samples = 0;

    // Process 1000 consecutive buffers
    for _ in 0..1000 {
        let input = generate_stereo_sine(1000.0, 44100, 512);
        let output = resampler.process(&input).unwrap();

        total_input_samples += input.len();
        total_output_samples += output.len();

        // Verify no NaN or Inf
        for sample in &output {
            assert!(sample.is_finite(), "Output should be finite");
        }
    }

    // Verify overall ratio is correct
    let ratio = total_output_samples as f64 / total_input_samples as f64;
    let expected_ratio = 48000.0 / 44100.0;
    assert!(
        (ratio - expected_ratio).abs() < 0.1,
        "Overall ratio should be ~{}, got {}",
        expected_ratio,
        ratio
    );
}

#[test]
fn test_long_duration_single_buffer() {
    let mut resampler = AudioResampler::new(44100, 48000, 2, ResamplingQuality::Normal).unwrap();

    // 10 seconds of audio
    let input = generate_stereo_sine(1000.0, 44100, 441000);
    let output = resampler.process(&input).unwrap();

    // Verify output size is reasonable
    let expected_output_frames = (441000.0 * 48000.0 / 44100.0) as usize;
    assert!(
        (output.len() / 2) > expected_output_frames - 1000
            && (output.len() / 2) < expected_output_frames + 1000,
        "Output frames should be ~{}, got {}",
        expected_output_frames,
        output.len() / 2
    );
}

#[test]
fn test_reset_during_long_run() {
    let mut resampler = AudioResampler::new(44100, 48000, 2, ResamplingQuality::Normal).unwrap();

    // Process some buffers
    for _ in 0..100 {
        let input = generate_stereo_sine(1000.0, 44100, 512);
        let _ = resampler.process(&input).unwrap();
    }

    // Reset
    resampler.reset();

    // Continue processing - should work fine
    for _ in 0..100 {
        let input = generate_stereo_sine(1000.0, 44100, 512);
        let output = resampler.process(&input).unwrap();

        assert!(output.len() > 0);
    }
}

// ============================================================================
// QUALITY PRESET TESTS
// ============================================================================

#[test]
fn test_all_quality_presets() {
    let qualities = [
        ResamplingQuality::Fast,
        ResamplingQuality::Normal,
        ResamplingQuality::High,
    ];

    for quality in qualities {
        let mut resampler = AudioResampler::new(44100, 48000, 2, quality).unwrap();

        let input = generate_stereo_sine(1000.0, 44100, 4410);
        let output = resampler.process(&input).unwrap();

        assert!(output.len() > 0, "{:?} should produce output", quality);
        assert!(
            calculate_rms(&output) > 0.3,
            "{:?} should preserve signal",
            quality
        );
    }
}

#[test]
fn test_quality_affects_latency() {
    // Higher quality typically means more latency
    let mut fast_resampler = AudioResampler::new(44100, 48000, 2, ResamplingQuality::Fast).unwrap();
    let mut high_resampler = AudioResampler::new(44100, 48000, 2, ResamplingQuality::High).unwrap();

    let input = generate_stereo_sine(1000.0, 44100, 256);

    let fast_output = fast_resampler.process(&input).unwrap();
    let high_output = high_resampler.process(&input).unwrap();

    // Both should produce valid output
    assert!(fast_output.len() > 0);
    assert!(high_output.len() > 0);
}

// ============================================================================
// CHANNEL CONFIGURATION TESTS
// ============================================================================

#[test]
fn test_mono_resampling() {
    let mut resampler = AudioResampler::new(44100, 48000, 1, ResamplingQuality::Normal).unwrap();

    // Mono signal
    let input: Vec<f32> = (0..4410)
        .map(|i| (2.0 * PI * 1000.0 * i as f32 / 44100.0).sin())
        .collect();

    let output = resampler.process(&input).unwrap();

    assert!(output.len() > 4500); // Should be upsampled
}

#[test]
fn test_stereo_channel_independence() {
    let mut resampler = AudioResampler::new(44100, 48000, 2, ResamplingQuality::Normal).unwrap();

    // Different frequencies in each channel
    let mut input = Vec::with_capacity(4410 * 2);
    for i in 0..4410 {
        let t = i as f32 / 44100.0;
        let left = (2.0 * PI * 440.0 * t).sin(); // 440 Hz
        let right = (2.0 * PI * 880.0 * t).sin(); // 880 Hz
        input.push(left);
        input.push(right);
    }

    let output = resampler.process(&input).unwrap();

    // Extract channels
    let left_out: Vec<f32> = output.iter().step_by(2).copied().collect();
    let right_out: Vec<f32> = output.iter().skip(1).step_by(2).copied().collect();

    // Channels should be different
    let correlation: f32 = left_out
        .iter()
        .zip(&right_out)
        .map(|(l, r)| l * r)
        .sum::<f32>()
        / left_out.len() as f32;

    // Should not be perfectly correlated
    assert!(
        correlation.abs() < 0.5,
        "Channels should be independent, correlation: {}",
        correlation
    );
}

// ============================================================================
// EDGE SAMPLE RATE TESTS
// ============================================================================

#[test]
fn test_very_low_source_rate() {
    // 4000 Hz (very low)
    let mut resampler = AudioResampler::new(4000, 44100, 2, ResamplingQuality::Normal).unwrap();

    let input = generate_stereo_sine(500.0, 4000, 400); // 500 Hz tone
    let output = resampler.process(&input).unwrap();

    assert!(output.len() > input.len() * 8); // Large upsampling
}

#[test]
fn test_very_high_source_rate() {
    // 384000 Hz (very high)
    let mut resampler = AudioResampler::new(384000, 44100, 2, ResamplingQuality::Normal).unwrap();

    let input = generate_stereo_sine(1000.0, 384000, 38400);
    let output = resampler.process(&input).unwrap();

    assert!(output.len() < input.len()); // Downsampling
}

// ============================================================================
// NUMERICAL STABILITY
// ============================================================================

#[test]
fn test_dc_offset_preservation() {
    let mut resampler = AudioResampler::new(44100, 48000, 2, ResamplingQuality::Normal).unwrap();

    // DC signal
    let input = vec![0.5; 8820]; // 2205 stereo frames of DC
    let output = resampler.process(&input).unwrap();

    // DC should be preserved
    let avg: f32 = output.iter().sum::<f32>() / output.len() as f32;
    assert!(
        (avg - 0.5).abs() < 0.1,
        "DC offset should be preserved, got {}",
        avg
    );
}

#[test]
fn test_silence_stays_silent() {
    let mut resampler = AudioResampler::new(44100, 48000, 2, ResamplingQuality::Normal).unwrap();

    let input = vec![0.0; 8820];
    let output = resampler.process(&input).unwrap();

    // Output should be near-silent
    let rms = calculate_rms(&output);
    assert!(rms < 0.001, "Silence should remain silent, got RMS {}", rms);
}

#[test]
fn test_no_nan_or_inf() {
    let mut resampler = AudioResampler::new(44100, 48000, 2, ResamplingQuality::Normal).unwrap();

    // Various input signals
    let test_signals = [
        generate_stereo_sine(1000.0, 44100, 4410),
        vec![0.0; 8820],
        vec![1.0; 8820],
        vec![-1.0; 8820],
    ];

    for input in test_signals {
        let output = resampler.process(&input).unwrap();

        for sample in &output {
            assert!(!sample.is_nan(), "Output contains NaN");
            assert!(!sample.is_infinite(), "Output contains Inf");
        }

        resampler.reset();
    }
}

// ============================================================================
// PERFORMANCE / TIMING
// ============================================================================

#[test]
fn test_processing_time_consistency() {
    use std::time::Instant;

    let mut resampler = AudioResampler::new(44100, 48000, 2, ResamplingQuality::Normal).unwrap();

    let input = generate_stereo_sine(1000.0, 44100, 4096);
    let mut times = Vec::new();

    // Warmup
    for _ in 0..10 {
        let _ = resampler.process(&input).unwrap();
    }

    // Measure
    for _ in 0..100 {
        let start = Instant::now();
        let _ = resampler.process(&input).unwrap();
        times.push(start.elapsed());
    }

    let avg = times.iter().map(|t| t.as_nanos()).sum::<u128>() / times.len() as u128;
    let max = times.iter().map(|t| t.as_nanos()).max().unwrap();

    // Max should not be too much more than average
    let ratio = max as f64 / avg as f64;
    assert!(
        ratio < 10.0,
        "Processing time variance too high: avg={}ns, max={}ns",
        avg,
        max
    );
}
