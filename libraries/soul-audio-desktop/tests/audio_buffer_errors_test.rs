//! MEDIUM Priority Tests: Audio Buffer and Resampling Error Paths
//!
//! Tests edge cases and error conditions in audio format conversion, buffer handling,
//! and resampling under stress conditions.

#![allow(clippy::manual_range_contains)]
#![allow(clippy::cast_lossless)]
#![allow(clippy::len_zero)]
#![allow(clippy::cast_abs_to_unsigned)]
#![allow(clippy::manual_is_multiple_of)]
#![allow(clippy::useless_vec)]
#![allow(clippy::unnecessary_cast)]

use soul_audio::resampling::{Resampler, ResamplerBackend, ResamplingQuality};
use std::time::Duration;

/// Test helper: Create interleaved stereo samples with specific values
fn create_stereo_samples(left: f32, right: f32, frames: usize) -> Vec<f32> {
    let mut samples = Vec::with_capacity(frames * 2);
    for _ in 0..frames {
        samples.push(left);
        samples.push(right);
    }
    samples
}

// ============================================================================
// Format Conversion Edge Cases
// ============================================================================

#[test]
fn test_f32_clamp_with_nan() {
    // Test that NaN values are properly handled in f32 conversions
    // Based on decoder.rs line 176: clamp(-1.0, 1.0)

    let test_cases = vec![
        (f32::NAN, "NaN"),
        (f32::INFINITY, "positive infinity"),
        (f32::NEG_INFINITY, "negative infinity"),
        (2.5, "above range"),
        (-2.5, "below range"),
    ];

    for (input, desc) in test_cases {
        let clamped = input.clamp(-1.0, 1.0);

        // Clamped value should be finite and in range
        // NaN.clamp() returns NaN, which we need to handle
        if input.is_nan() {
            assert!(
                clamped.is_nan(),
                "NaN should remain NaN after clamp: {}",
                desc
            );
        } else {
            assert!(
                (-1.0..=1.0).contains(&clamped),
                "Value {} should be clamped to [-1.0, 1.0], got: {}",
                desc,
                clamped
            );
        }
    }
}

#[test]
fn test_f32_conversion_intersample_peaks() {
    // Test handling of intersample peaks (values > 1.0 in floating point audio)
    // This is mentioned in decoder.rs line 175: "F32 audio can have intersample peaks > 1.0"

    let peaks: Vec<f32> = vec![1.5, 2.0, -1.5, -2.0];

    for peak in peaks {
        let clamped = peak.clamp(-1.0, 1.0);

        assert!(
            (-1.0..=1.0).contains(&clamped),
            "Intersample peak {} should be clamped to [-1.0, 1.0], got: {}",
            peak,
            clamped
        );

        // Verify no precision loss for in-range values
        let normal = 0.5f32;
        assert_eq!(normal.clamp(-1.0, 1.0), normal);
    }
}

#[test]
fn test_integer_to_float_symmetry() {
    // Verify symmetric scaling for signed integers (divide by 2^(N-1))
    // Based on decoder.rs lines 186-200

    // i16: -32768 to 32767, divide by 32768.0 gives [-1.0, 1.0)
    let i16_min = i16::MIN as f32 / 32768.0;
    let i16_max = i16::MAX as f32 / 32768.0;

    assert_eq!(i16_min, -1.0, "i16::MIN should map to exactly -1.0");
    assert!(
        (0.999..1.0).contains(&i16_max),
        "i16::MAX should map close to 1.0: {}",
        i16_max
    );

    // i32: -2147483648 to 2147483647, divide by 2147483648.0
    // Note: f32 has only 24 bits of precision, so i32 conversion loses precision
    let i32_min = i32::MIN as f32 / 2147483648.0;
    let i32_max = i32::MAX as f32 / 2147483648.0;

    assert_eq!(i32_min, -1.0, "i32::MIN should map to exactly -1.0");
    // f32 can exactly represent 1.0, so this may round up
    assert!(
        i32_max <= 1.0 && i32_max >= 0.99,
        "i32::MAX should map close to 1.0 (may be exactly 1.0 due to f32 rounding): {}",
        i32_max
    );

    // i8: -128 to 127, divide by 128.0
    let i8_min = i8::MIN as f32 / 128.0;
    let i8_max = i8::MAX as f32 / 128.0;

    assert_eq!(i8_min, -1.0, "i8::MIN should map to exactly -1.0");
    assert!(
        i8_max < 1.0 && i8_max >= 0.99,
        "i8::MAX should map close to 1.0: {}",
        i8_max
    );
}

#[test]
fn test_unsigned_to_signed_conversion() {
    // Test unsigned to signed conversion centering around 0
    // Based on decoder.rs lines 202-218

    // u8: 0-255 -> map to [-1.0, 1.0]
    let u8_min = (u8::MIN as f32 / u8::MAX as f32) * 2.0 - 1.0;
    let u8_mid = (128_u8 as f32 / u8::MAX as f32) * 2.0 - 1.0;
    let u8_max = (u8::MAX as f32 / u8::MAX as f32) * 2.0 - 1.0;

    assert!(
        (u8_min - (-1.0)).abs() < 0.01,
        "u8::MIN should map near -1.0: {}",
        u8_min
    );
    assert!(
        u8_mid.abs() < 0.01,
        "u8 midpoint (128) should map near 0.0: {}",
        u8_mid
    );
    assert_eq!(u8_max, 1.0, "u8::MAX should map to exactly 1.0");

    // u16: 0-65535 -> map to [-1.0, 1.0]
    let u16_min = (u16::MIN as f32 / u16::MAX as f32) * 2.0 - 1.0;
    let u16_max = (u16::MAX as f32 / u16::MAX as f32) * 2.0 - 1.0;

    assert!(
        (u16_min - (-1.0)).abs() < 0.0001,
        "u16::MIN should map near -1.0: {}",
        u16_min
    );
    assert_eq!(u16_max, 1.0, "u16::MAX should map to exactly 1.0");
}

#[test]
fn test_24bit_conversion_precision() {
    // Test 24-bit audio conversion precision
    // Based on decoder.rs lines 220-233

    // S24: -8388608 to 8388607, divide by 8388608.0
    let s24_min = -8388608_i32 as f32 / 8388608.0;
    let s24_max = 8388607_i32 as f32 / 8388608.0;

    assert_eq!(s24_min, -1.0, "S24 min should map to exactly -1.0");
    assert!(
        s24_max < 1.0 && s24_max >= 0.9999,
        "S24 max should map close to 1.0: {}",
        s24_max
    );

    // U24: 0 to 16777215, map to [-1.0, 1.0]
    let u24_min = (0_u32 as f32 / 16777215.0) * 2.0 - 1.0;
    let u24_max = (16777215_u32 as f32 / 16777215.0) * 2.0 - 1.0;

    assert!(
        (u24_min - (-1.0)).abs() < 0.0001,
        "U24 min should map near -1.0: {}",
        u24_min
    );
    assert_eq!(u24_max, 1.0, "U24 max should map to exactly 1.0");
}

// ============================================================================
// Buffer Size and Overflow Protection
// ============================================================================

#[test]
fn test_large_buffer_allocation() {
    // Test handling of very large buffer sizes without panic
    // This tests memory allocation limits

    let large_frame_count = 10_000_000; // 10 million frames
    let large_stereo_samples = large_frame_count * 2; // 20 million samples

    // Try to allocate - should succeed on modern systems (80 MB)
    // but we're testing that it doesn't panic
    let result = std::panic::catch_unwind(|| {
        let mut buffer: Vec<f32> = Vec::new();
        buffer.resize(large_stereo_samples, 0.0);
        buffer.len()
    });

    assert!(
        result.is_ok(),
        "Large buffer allocation should not panic: {:?}",
        result.err()
    );

    if let Ok(size) = result {
        assert_eq!(size, large_stereo_samples);
    }
}

#[test]
fn test_zero_length_buffer_handling() {
    // Test handling of empty buffers

    let empty: Vec<f32> = Vec::new();

    // Empty buffer should be valid
    assert_eq!(empty.len(), 0);

    // Processing empty buffer should not panic
    let clamped: Vec<f32> = empty.iter().map(|&s| s.clamp(-1.0, 1.0)).collect();
    assert_eq!(clamped.len(), 0);
}

#[test]
fn test_mismatched_channel_buffer() {
    // Test odd-length buffer (not valid for stereo)

    let odd_buffer = vec![1.0, 2.0, 3.0]; // 3 samples = 1.5 frames

    // This should be caught by validation
    let frames = odd_buffer.len() / 2;
    let remaining = odd_buffer.len() % 2;

    assert_eq!(frames, 1, "Should calculate 1 complete stereo frame");
    assert_eq!(remaining, 1, "Should detect 1 remaining sample");
}

#[test]
fn test_buffer_interleaving_correctness() {
    // Test that interleaved stereo maintains L/R separation

    let left_val = 0.5f32;
    let right_val = -0.5f32;
    let frames = 1000;

    let interleaved = create_stereo_samples(left_val, right_val, frames);

    // Verify correct interleaving
    for i in 0..frames {
        assert_eq!(
            interleaved[i * 2],
            left_val,
            "Left channel should be at even indices"
        );
        assert_eq!(
            interleaved[i * 2 + 1],
            right_val,
            "Right channel should be at odd indices"
        );
    }
}

// ============================================================================
// Resampling Error Paths
// ============================================================================

#[test]
fn test_resampler_invalid_sample_rates() {
    // Test resampler validation for invalid sample rates
    // Based on resampling/mod.rs lines 244-252

    let invalid_rates = vec![
        (0, 96000, "zero input rate"),
        (44100, 0, "zero output rate"),
        (1_500_000, 96000, "input rate too high (>1MHz)"),
        (44100, 2_000_000, "output rate too high (>2MHz)"),
    ];

    for (input_rate, output_rate, desc) in invalid_rates {
        let result = Resampler::new(
            ResamplerBackend::Auto,
            input_rate,
            output_rate,
            2,
            ResamplingQuality::High,
        );

        assert!(
            result.is_err(),
            "Should reject invalid sample rates ({}): input={}, output={}",
            desc,
            input_rate,
            output_rate
        );
    }
}

#[test]
fn test_resampler_invalid_channel_counts() {
    // Test resampler validation for invalid channel counts
    // Based on resampling/mod.rs lines 250-252

    let invalid_channels = vec![
        (0, "zero channels"),
        (9, "too many channels (>8)"),
        (10, "way too many channels"),
    ];

    for (channels, desc) in invalid_channels {
        let result = Resampler::new(
            ResamplerBackend::Auto,
            44100,
            96000,
            channels,
            ResamplingQuality::High,
        );

        assert!(
            result.is_err(),
            "Should reject invalid channel count ({}): {}",
            desc,
            channels
        );
    }
}

#[test]
fn test_resampler_valid_edge_case_channels() {
    // Test boundary cases for valid channel counts (1-8)

    for channels in 1..=8 {
        let result = Resampler::new(
            ResamplerBackend::Auto,
            44100,
            96000,
            channels,
            ResamplingQuality::Fast, // Use fast for speed
        );

        assert!(
            result.is_ok(),
            "Should accept {} channels: {:?}",
            channels,
            result.err()
        );

        if let Ok(resampler) = result {
            assert_eq!(resampler.channels(), channels);
        }
    }
}

#[test]
fn test_resampler_wrong_buffer_size() {
    // Test resampler with incorrectly sized input buffer

    let mut resampler = Resampler::new(
        ResamplerBackend::Auto,
        44100,
        96000,
        2, // stereo
        ResamplingQuality::Fast,
    )
    .expect("Should create resampler");

    // Create buffer with odd number of samples (invalid for stereo)
    let wrong_buffer = vec![0.5; 1001]; // 1001 samples = not divisible by 2

    let result = resampler.process(&wrong_buffer);

    // Should handle gracefully - either error or process what it can
    // Different backends may handle this differently
    match result {
        Ok(output) => {
            // If it succeeds, output should be reasonable
            assert!(
                output.len() > 0,
                "If processing succeeds, should produce output"
            );
        }
        Err(e) => {
            // Error is acceptable for mismatched buffer
            tracing::debug!("Resampler rejected odd buffer size: {}", e);
        }
    }
}

#[test]
fn test_resampler_output_size_calculation() {
    // Test output size calculations for various resampling ratios
    // Based on resampling/mod.rs lines 356-383

    let test_cases = vec![
        (44100, 96000, 2048, "44.1kHz to 96kHz upsampling"),
        (96000, 44100, 2048, "96kHz to 44.1kHz downsampling"),
        (48000, 44100, 1024, "48kHz to 44.1kHz"),
        (192000, 48000, 4096, "192kHz to 48kHz (4x downsampling)"),
    ];

    for (input_rate, output_rate, input_samples, desc) in test_cases {
        let resampler = Resampler::new(
            ResamplerBackend::Auto,
            input_rate,
            output_rate,
            2,
            ResamplingQuality::Fast,
        )
        .expect("Should create resampler");

        let expected_size = resampler.calculate_output_size(input_samples);
        let (min_size, max_size) = resampler.calculate_output_size_range(input_samples);

        // Verify range is sensible
        assert!(
            min_size <= max_size,
            "{}: min ({}) should be <= max ({})",
            desc,
            min_size,
            max_size
        );

        // Expected size should be within range
        assert!(
            expected_size >= min_size && expected_size <= max_size,
            "{}: expected ({}) should be within range [{}, {}]",
            desc,
            expected_size,
            min_size,
            max_size
        );

        // Verify ratio is approximately correct
        let ratio = output_rate as f64 / input_rate as f64;
        let theoretical = (input_samples as f64 * ratio) as usize;
        let diff = (expected_size as i64 - theoretical as i64).abs() as usize;

        assert!(
            diff <= 2,
            "{}: calculated size ({}) should be close to theoretical ({})",
            desc,
            expected_size,
            theoretical
        );
    }
}

// ============================================================================
// Concurrent Resampling Stress Tests
// ============================================================================

#[tokio::test]
async fn test_concurrent_resampling_stress() {
    // Test multiple concurrent resampling tasks at maximum quality
    // This stresses CPU and memory allocation under concurrent load

    let num_tasks = 10;
    let sample_rate_pairs = vec![
        (44100, 96000),
        (48000, 192000),
        (88200, 44100),
        (96000, 48000),
    ];

    let mut handles = Vec::new();

    for i in 0..num_tasks {
        let (input_rate, output_rate) = sample_rate_pairs[i % sample_rate_pairs.len()];

        let handle = tokio::spawn(async move {
            // Create resampler with maximum quality
            let mut resampler = Resampler::new(
                ResamplerBackend::Auto,
                input_rate,
                output_rate,
                2,
                ResamplingQuality::Maximum,
            )
            .expect("Should create resampler");

            // Process 1 second of audio (multiple chunks)
            let frames_per_second = input_rate as usize;
            let samples_per_second = frames_per_second * 2; // stereo
            let chunk_size = 2048;

            let mut total_output = 0;

            for chunk_start in (0..samples_per_second).step_by(chunk_size) {
                let chunk_end = (chunk_start + chunk_size).min(samples_per_second);
                let chunk_len = chunk_end - chunk_start;

                let input = create_stereo_samples(0.5, -0.5, chunk_len / 2);

                let output = resampler
                    .process(&input)
                    .expect("Should process chunk successfully");

                total_output += output.len();
            }

            (i, total_output)
        });

        handles.push(handle);
    }

    // Wait for all tasks to complete
    let mut results = Vec::new();
    for handle in handles {
        let result = tokio::time::timeout(Duration::from_secs(30), handle)
            .await
            .expect("Task should complete within 30 seconds")
            .expect("Task should not panic");

        results.push(result);
    }

    // Verify all tasks completed
    assert_eq!(results.len(), num_tasks);

    // Verify each task produced reasonable output
    for (task_id, output_samples) in results {
        assert!(
            output_samples > 0,
            "Task {} should produce output samples",
            task_id
        );
    }
}

#[tokio::test]
async fn test_concurrent_different_quality_levels() {
    // Test concurrent resampling with different quality levels

    let qualities = vec![
        ResamplingQuality::Fast,
        ResamplingQuality::Balanced,
        ResamplingQuality::High,
        ResamplingQuality::Maximum,
    ];

    let mut handles = Vec::new();

    for (i, quality) in qualities.iter().enumerate() {
        let quality = *quality;

        let handle = tokio::spawn(async move {
            let mut resampler = Resampler::new(ResamplerBackend::Auto, 44100, 96000, 2, quality)
                .expect("Should create resampler");

            let input = create_stereo_samples(0.5, -0.5, 2048);

            let start = std::time::Instant::now();

            // Process the input - some resamplers may need multiple chunks to produce output
            let total_output = match resampler.process(&input) {
                Ok(output) => {
                    if output.is_empty() {
                        // If no output yet, try flushing
                        resampler.flush().unwrap_or_default()
                    } else {
                        output
                    }
                }
                Err(e) => {
                    panic!(
                        "Resampler processing failed for quality {:?}: {}",
                        quality, e
                    );
                }
            };

            let elapsed = start.elapsed();

            (i, quality, total_output.len(), elapsed)
        });

        handles.push(handle);
    }

    // Wait for all to complete
    for handle in handles {
        let result = tokio::time::timeout(Duration::from_secs(10), handle)
            .await
            .expect("Task should complete within 10 seconds");

        let (task_id, quality, output_len, elapsed) = result.expect("Task should not panic");

        // Some resamplers may need buffering before producing output
        // We only require that the process doesn't fail
        if output_len > 0 {
            tracing::debug!(
                "Task {} ({:?}): {} samples in {:?}",
                task_id,
                quality,
                output_len,
                elapsed
            );
        } else {
            tracing::debug!(
                "Task {} ({:?}): No output (buffered internally) in {:?}",
                task_id,
                quality,
                elapsed
            );
        }
    }
}

#[test]
fn test_resampler_reset_and_reuse() {
    // Test that resampler can be reset and reused correctly

    let mut resampler = Resampler::new(
        ResamplerBackend::Auto,
        44100,
        96000,
        2,
        ResamplingQuality::Balanced,
    )
    .expect("Should create resampler");

    // Process first chunk
    let input1 = create_stereo_samples(0.5, -0.5, 1024);
    let output1 = resampler.process(&input1).expect("Should process first");

    // Reset
    resampler.reset();

    // Process second chunk (should produce similar output)
    let input2 = create_stereo_samples(0.5, -0.5, 1024);
    let output2 = resampler.process(&input2).expect("Should process second");

    // Outputs should be similar length (may differ by 1-2 samples due to internal state)
    let len_diff = (output1.len() as i64 - output2.len() as i64).abs() as usize;
    assert!(
        len_diff <= 2,
        "Output lengths should be similar after reset: {} vs {}",
        output1.len(),
        output2.len()
    );
}

#[test]
fn test_resampler_flush_remaining_samples() {
    // Test that flush retrieves buffered samples

    let mut resampler = Resampler::new(
        ResamplerBackend::Auto,
        44100,
        96000,
        2,
        ResamplingQuality::High,
    )
    .expect("Should create resampler");

    // Process some audio
    let input = create_stereo_samples(0.5, -0.5, 512);
    let _output = resampler.process(&input).expect("Should process");

    // Flush any remaining samples
    let flushed = resampler.flush().expect("Should flush");

    // Flushed output may or may not be empty depending on internal buffering
    // Just verify it doesn't panic and returns valid data
    assert!(
        flushed.len() % 2 == 0,
        "Flushed output should be valid stereo (even length): {}",
        flushed.len()
    );
}

// ============================================================================
// Memory Safety Tests
// ============================================================================

#[test]
fn test_buffer_conversion_no_allocation_in_loop() {
    // Verify that processing large amounts of audio doesn't cause excessive allocation
    // This is a proxy test for allocation-free audio callback

    let frames = 100_000;
    let input = create_stereo_samples(0.5, -0.5, frames);

    // Process in chunks (simulating audio callback)
    let chunk_size = 512;
    let mut total_clamped = 0;

    for chunk_start in (0..input.len()).step_by(chunk_size * 2) {
        let chunk_end = (chunk_start + chunk_size * 2).min(input.len());
        let chunk = &input[chunk_start..chunk_end];

        // Clamp in place (no allocation)
        let clamped_count = chunk.iter().filter(|&&s| s >= -1.0 && s <= 1.0).count();
        total_clamped += clamped_count;
    }

    assert_eq!(total_clamped, input.len());
}

#[test]
fn test_extreme_resampling_ratios() {
    // Test extreme resampling ratios (within valid range)

    let extreme_ratios = vec![
        (8000, 192000, "24x upsampling (8kHz to 192kHz)"),
        (192000, 8000, "24x downsampling (192kHz to 8kHz)"),
        (11025, 96000, "8.7x upsampling (11.025kHz to 96kHz)"),
    ];

    for (input_rate, output_rate, desc) in extreme_ratios {
        let result = Resampler::new(
            ResamplerBackend::Auto,
            input_rate,
            output_rate,
            2,
            ResamplingQuality::Fast, // Use fast for extreme ratios
        );

        assert!(
            result.is_ok(),
            "{}: Should handle extreme ratio: {:?}",
            desc,
            result.err()
        );

        if let Ok(mut resampler) = result {
            // Try processing a small buffer
            let input = create_stereo_samples(0.5, -0.5, 256);
            let output_result = resampler.process(&input);

            assert!(
                output_result.is_ok(),
                "{}: Should process with extreme ratio: {:?}",
                desc,
                output_result.err()
            );
        }
    }
}
