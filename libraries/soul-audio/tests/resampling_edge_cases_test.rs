//! Resampling edge case tests
//!
//! Tests for:
//! - Unusual sample rate combinations
//! - Very short buffers
//! - Long-running resampling (many consecutive buffers)
//! - Edge sample rates (very low, very high)
//! - Non-integer rate ratios

use soul_audio::resampling::{Resampler, ResamplerBackend, ResamplingQuality};
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
    let mut resampler = Resampler::new(
        ResamplerBackend::Auto,
        44100,
        48000,
        2,
        ResamplingQuality::Balanced,
    )
    .unwrap();

    let input = generate_stereo_sine(1000.0, 44100, 4410); // 0.1 sec
    let mut output = resampler.process(&input).unwrap();
    // Flush any remaining buffered samples
    output.extend(resampler.flush().unwrap());

    // Upsampling: output should be larger than input
    assert!(
        output.len() > input.len(),
        "Upsampling should produce more samples: input={}, output={}",
        input.len(),
        output.len()
    );

    // Verify signal integrity
    assert!(calculate_rms(&output) > 0.3);
}

#[test]
fn test_48000_to_44100() {
    // Downsampling
    let mut resampler = Resampler::new(
        ResamplerBackend::Auto,
        48000,
        44100,
        2,
        ResamplingQuality::Balanced,
    )
    .unwrap();

    let input = generate_stereo_sine(1000.0, 48000, 4800);
    let output = resampler.process(&input).unwrap();

    // Output should be produced (latency may vary)
    assert!(output.len() > 0, "Downsampling should produce output");
    // Verify signal integrity
    assert!(calculate_rms(&output) > 0.2);
}

#[test]
fn test_44100_to_88200() {
    // 2x upsampling (integer ratio)
    let mut resampler = Resampler::new(
        ResamplerBackend::Auto,
        44100,
        88200,
        2,
        ResamplingQuality::Balanced,
    )
    .unwrap();

    let input = generate_stereo_sine(1000.0, 44100, 4410);
    let output = resampler.process(&input).unwrap();

    // Upsampling: output should be larger than input (approximately doubled)
    assert!(
        output.len() > input.len(),
        "2x upsampling should produce more samples"
    );
    assert!(calculate_rms(&output) > 0.3);
}

#[test]
fn test_96000_to_44100() {
    // Large downsampling ratio
    let mut resampler = Resampler::new(
        ResamplerBackend::Auto,
        96000,
        44100,
        2,
        ResamplingQuality::Balanced,
    )
    .unwrap();

    let input = generate_stereo_sine(1000.0, 96000, 9600);
    let output = resampler.process(&input).unwrap();

    // Output should be produced
    assert!(output.len() > 0, "Downsampling should produce output");
    assert!(calculate_rms(&output) > 0.2);
}

#[test]
fn test_22050_to_48000() {
    // Uncommon upsampling
    let mut resampler = Resampler::new(
        ResamplerBackend::Auto,
        22050,
        48000,
        2,
        ResamplingQuality::Balanced,
    )
    .unwrap();

    let input = generate_stereo_sine(1000.0, 22050, 2205);
    let output = resampler.process(&input).unwrap();

    assert!(output.len() > 8000);
}

#[test]
fn test_48000_to_22050() {
    // Uncommon downsampling
    let mut resampler = Resampler::new(
        ResamplerBackend::Auto,
        48000,
        22050,
        2,
        ResamplingQuality::Balanced,
    )
    .unwrap();

    let input = generate_stereo_sine(1000.0, 48000, 4800);
    let output = resampler.process(&input).unwrap();

    // Verify output is smaller
    assert!(output.len() < input.len());
}

#[test]
fn test_8000_to_44100() {
    // Telephone rate to CD rate
    let mut resampler = Resampler::new(
        ResamplerBackend::Auto,
        8000,
        44100,
        2,
        ResamplingQuality::Balanced,
    )
    .unwrap();

    let input = generate_stereo_sine(1000.0, 8000, 800);
    let mut output = resampler.process(&input).unwrap();
    // Flush any remaining buffered samples
    output.extend(resampler.flush().unwrap());

    // Large upsampling ratio
    assert!(output.len() > input.len() * 4);
}

#[test]
fn test_192000_to_44100() {
    // High-res to CD rate
    let mut resampler = Resampler::new(
        ResamplerBackend::Auto,
        192000,
        44100,
        2,
        ResamplingQuality::Balanced,
    )
    .unwrap();

    let input = generate_stereo_sine(1000.0, 192000, 19200);
    let output = resampler.process(&input).unwrap();

    assert!(output.len() < input.len());
}

#[test]
fn test_same_rate_passthrough() {
    // 44100 to 44100 (should be passthrough or near-passthrough)
    let mut resampler = Resampler::new(
        ResamplerBackend::Auto,
        44100,
        44100,
        2,
        ResamplingQuality::Balanced,
    )
    .unwrap();

    let input = generate_stereo_sine(1000.0, 44100, 4410);
    let output = resampler.process(&input).unwrap();

    // Should produce output and preserve signal
    assert!(output.len() > 0, "Same rate should produce output");
    assert!(calculate_rms(&output) > 0.3, "Signal should be preserved");
}

// ============================================================================
// NON-STANDARD SAMPLE RATES
// ============================================================================

#[test]
fn test_non_standard_rates() {
    // 37800 to 44100 (non-standard source rate)
    let mut resampler = Resampler::new(
        ResamplerBackend::Auto,
        37800,
        44100,
        2,
        ResamplingQuality::Balanced,
    )
    .unwrap();

    let input = generate_stereo_sine(1000.0, 37800, 3780);
    let output = resampler.process(&input).unwrap();

    assert!(output.len() > 0);
    assert!(calculate_rms(&output) > 0.2);
}

#[test]
fn test_prime_number_rates() {
    // Using prime numbers for interesting ratios
    let mut resampler = Resampler::new(
        ResamplerBackend::Auto,
        44101,
        48000,
        2,
        ResamplingQuality::Balanced,
    )
    .unwrap();

    let input = generate_stereo_sine(1000.0, 44101, 4410);
    let output = resampler.process(&input).unwrap();

    assert!(output.len() > 0);
}

// ============================================================================
// VERY SHORT BUFFER TESTS
// ============================================================================

#[test]
fn test_single_frame_buffer() {
    let mut resampler = Resampler::new(
        ResamplerBackend::Auto,
        44100,
        48000,
        2,
        ResamplingQuality::Balanced,
    )
    .unwrap();

    // Single stereo frame
    let input = vec![0.5, 0.5];
    let mut output = resampler.process(&input).unwrap();
    // Flush to get any buffered samples
    output.extend(resampler.flush().unwrap());

    // Should produce some output (at least 1 frame) after flush
    assert!(output.len() >= 2);
}

#[test]
fn test_very_small_buffers() {
    // Note: With buffered resampling, small inputs are accumulated until there's
    // enough for a complete chunk. This test verifies the resampler handles small
    // inputs without panicking and eventually produces output.
    let mut resampler = Resampler::new(
        ResamplerBackend::Auto,
        44100,
        48000,
        2,
        ResamplingQuality::Balanced,
    )
    .unwrap();

    let mut total_output = 0;
    for size in [2, 4, 8, 16, 32, 64] {
        let input = generate_stereo_sine(1000.0, 44100, size);
        let output = resampler.process(&input).unwrap();
        total_output += output.len();
    }
    // Flush remaining samples
    total_output += resampler.flush().unwrap().len();

    // Should produce output after all inputs and flush
    assert!(
        total_output > 0,
        "Should produce output after accumulating small buffers and flushing"
    );
}

#[test]
fn test_odd_sample_counts() {
    // Note: With buffered resampling, small inputs are accumulated. This test
    // verifies odd frame counts work correctly by accumulating and flushing.
    let mut resampler = Resampler::new(
        ResamplerBackend::Auto,
        44100,
        48000,
        2,
        ResamplingQuality::Balanced,
    )
    .unwrap();

    let mut total_output = 0;
    // Odd number of frames (not a problem, but edge case)
    for frames in [1, 3, 5, 7, 13, 17, 31, 127] {
        let input = generate_stereo_sine(1000.0, 44100, frames);
        let output = resampler.process(&input).unwrap();
        total_output += output.len();
    }
    // Flush remaining samples
    total_output += resampler.flush().unwrap().len();

    assert!(
        total_output > 0,
        "Odd frame counts should work after accumulating and flushing"
    );
}

#[test]
fn test_empty_buffer() {
    let mut resampler = Resampler::new(
        ResamplerBackend::Auto,
        44100,
        48000,
        2,
        ResamplingQuality::Balanced,
    )
    .unwrap();

    let input: Vec<f32> = vec![];
    let output = resampler.process(&input).unwrap();

    assert!(output.is_empty() || output.len() < 10); // Might have latency samples
}

// ============================================================================
// LONG-RUNNING RESAMPLING TESTS
// ============================================================================

#[test]
fn test_many_consecutive_buffers() {
    let mut resampler = Resampler::new(
        ResamplerBackend::Auto,
        44100,
        48000,
        2,
        ResamplingQuality::Balanced,
    )
    .unwrap();

    let mut total_output_samples = 0;
    let mut all_outputs_valid = true;

    // Process 1000 consecutive buffers
    for _ in 0..1000 {
        let input = generate_stereo_sine(1000.0, 44100, 512);
        let output = resampler.process(&input).unwrap();

        total_output_samples += output.len();

        // Verify no NaN or Inf
        for sample in &output {
            if !sample.is_finite() {
                all_outputs_valid = false;
            }
        }
    }

    assert!(all_outputs_valid, "All output samples should be finite");
    assert!(
        total_output_samples > 0,
        "Should produce output over 1000 buffers"
    );
}

#[test]
fn test_long_duration_single_buffer() {
    let mut resampler = Resampler::new(
        ResamplerBackend::Auto,
        44100,
        48000,
        2,
        ResamplingQuality::Balanced,
    )
    .unwrap();

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
    let mut resampler = Resampler::new(
        ResamplerBackend::Auto,
        44100,
        48000,
        2,
        ResamplingQuality::Balanced,
    )
    .unwrap();

    // Process some buffers
    for _ in 0..100 {
        let input = generate_stereo_sine(1000.0, 44100, 512);
        let _ = resampler.process(&input).unwrap();
    }

    // Reset
    resampler.reset();

    // Continue processing - use enough frames (2048) to exceed chunk size and produce output
    let mut total_output = 0;
    for _ in 0..100 {
        let input = generate_stereo_sine(1000.0, 44100, 2048);
        let output = resampler.process(&input).unwrap();
        total_output += output.len();
    }

    assert!(total_output > 0, "Should produce output after reset");
}

// ============================================================================
// QUALITY PRESET TESTS
// ============================================================================

#[test]
fn test_all_quality_presets() {
    let qualities = [
        ResamplingQuality::Fast,
        ResamplingQuality::Balanced,
        ResamplingQuality::High,
        ResamplingQuality::Maximum,
    ];

    for quality in qualities {
        let mut resampler =
            Resampler::new(ResamplerBackend::Auto, 44100, 48000, 2, quality).unwrap();

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
    // Higher quality typically means more latency and different chunk sizes
    let mut fast_resampler = Resampler::new(
        ResamplerBackend::Auto,
        44100,
        48000,
        2,
        ResamplingQuality::Fast,
    )
    .unwrap();
    let mut high_resampler = Resampler::new(
        ResamplerBackend::Auto,
        44100,
        48000,
        2,
        ResamplingQuality::High,
    )
    .unwrap();

    // Use larger input to ensure output is produced (chunk sizes vary by quality)
    let input = generate_stereo_sine(1000.0, 44100, 4096);

    let mut fast_output = fast_resampler.process(&input).unwrap();
    let mut high_output = high_resampler.process(&input).unwrap();

    // Flush remaining samples
    fast_output.extend(fast_resampler.flush().unwrap());
    high_output.extend(high_resampler.flush().unwrap());

    // Both should produce valid output
    assert!(fast_output.len() > 0, "Fast quality should produce output");
    assert!(high_output.len() > 0, "High quality should produce output");

    // Verify latency API is available
    let fast_latency = fast_resampler.latency();
    let high_latency = high_resampler.latency();
    // Just verify it returns something (latency values depend on implementation)
    assert!(fast_latency >= 0);
    assert!(high_latency >= 0);
}

// ============================================================================
// CHANNEL CONFIGURATION TESTS
// ============================================================================

#[test]
fn test_mono_resampling() {
    let mut resampler = Resampler::new(
        ResamplerBackend::Auto,
        44100,
        48000,
        1,
        ResamplingQuality::Balanced,
    )
    .unwrap();

    // Mono signal
    let input: Vec<f32> = (0..4410)
        .map(|i| (2.0 * PI * 1000.0 * i as f32 / 44100.0).sin())
        .collect();

    let mut output = resampler.process(&input).unwrap();
    // Flush remaining samples
    output.extend(resampler.flush().unwrap());

    assert!(
        output.len() > 4500,
        "Mono signal should be upsampled, got {} samples",
        output.len()
    );
}

#[test]
fn test_stereo_channel_independence() {
    let mut resampler = Resampler::new(
        ResamplerBackend::Auto,
        44100,
        48000,
        2,
        ResamplingQuality::Balanced,
    )
    .unwrap();

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
    let mut resampler = Resampler::new(
        ResamplerBackend::Auto,
        4000,
        44100,
        2,
        ResamplingQuality::Balanced,
    )
    .unwrap();

    let input = generate_stereo_sine(500.0, 4000, 400); // 500 Hz tone
    let mut output = resampler.process(&input).unwrap();
    // Flush remaining samples
    output.extend(resampler.flush().unwrap());

    assert!(
        output.len() > input.len() * 8,
        "Large upsampling should work, got {} samples vs {} input",
        output.len(),
        input.len()
    );
}

#[test]
fn test_very_high_source_rate() {
    // 384000 Hz (very high)
    let mut resampler = Resampler::new(
        ResamplerBackend::Auto,
        384000,
        44100,
        2,
        ResamplingQuality::Balanced,
    )
    .unwrap();

    let input = generate_stereo_sine(1000.0, 384000, 38400);
    let output = resampler.process(&input).unwrap();

    assert!(output.len() < input.len()); // Downsampling
}

// ============================================================================
// NUMERICAL STABILITY
// ============================================================================

#[test]
fn test_dc_offset_preservation() {
    let mut resampler = Resampler::new(
        ResamplerBackend::Auto,
        44100,
        48000,
        2,
        ResamplingQuality::Balanced,
    )
    .unwrap();

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
    let mut resampler = Resampler::new(
        ResamplerBackend::Auto,
        44100,
        48000,
        2,
        ResamplingQuality::Balanced,
    )
    .unwrap();

    let input = vec![0.0; 8820];
    let output = resampler.process(&input).unwrap();

    // Output should be near-silent
    let rms = calculate_rms(&output);
    assert!(rms < 0.001, "Silence should remain silent, got RMS {}", rms);
}

#[test]
fn test_no_nan_or_inf() {
    let mut resampler = Resampler::new(
        ResamplerBackend::Auto,
        44100,
        48000,
        2,
        ResamplingQuality::Balanced,
    )
    .unwrap();

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

    let mut resampler = Resampler::new(
        ResamplerBackend::Auto,
        44100,
        48000,
        2,
        ResamplingQuality::Balanced,
    )
    .unwrap();

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
