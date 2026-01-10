//! Resampling integration tests
//!
//! Tests for high-quality sample rate conversion with both backends.

use soul_audio::resampling::{Resampler, ResamplerBackend, ResamplingQuality};
use std::f32::consts::PI;

/// Generate a pure sine wave at given frequency
fn generate_sine_wave(frequency: f32, sample_rate: u32, duration_secs: f32, channels: usize) -> Vec<f32> {
    let num_samples = (sample_rate as f32 * duration_secs) as usize;
    let mut samples = Vec::with_capacity(num_samples * channels);

    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let value = (2.0 * PI * frequency * t).sin();

        // Interleave for all channels
        for _ in 0..channels {
            samples.push(value);
        }
    }

    samples
}

/// Calculate RMS amplitude of a signal
fn calculate_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }

    let sum_squares: f32 = samples.iter().map(|&s| s * s).sum();
    (sum_squares / samples.len() as f32).sqrt()
}

/// Extract mono channel from interleaved stereo
fn extract_mono(interleaved: &[f32], channel: usize, num_channels: usize) -> Vec<f32> {
    interleaved
        .iter()
        .skip(channel)
        .step_by(num_channels)
        .copied()
        .collect()
}

/// Test basic upsampling (44.1kHz → 96kHz)
#[test]
fn test_upsampling_44k_to_96k() {
    let input_rate = 44100;
    let output_rate = 96000;
    let channels = 2;

    let mut resampler = Resampler::new(
        ResamplerBackend::Auto,
        input_rate,
        output_rate,
        channels,
        ResamplingQuality::High,
    )
    .unwrap();

    // Generate 1kHz sine wave at 44.1kHz
    let input = generate_sine_wave(1000.0, input_rate, 0.1, channels);

    let output = resampler.process(&input).unwrap();

    // Verify output size is roughly correct
    let expected_ratio = output_rate as f32 / input_rate as f32;
    let expected_output_frames = (input.len() / channels) as f32 * expected_ratio;
    let actual_output_frames = output.len() / channels;

    // Allow 50% tolerance for chunk-based processing and internal buffering
    // Rubato resamplers have internal buffers that affect output size
    assert!(
        (actual_output_frames as f32 - expected_output_frames).abs() / expected_output_frames < 0.5,
        "Output size mismatch: expected ~{}, got {}",
        expected_output_frames,
        actual_output_frames
    );

    // Verify amplitude is preserved (within 30% due to buffering effects)
    // Rubato resamplers have latency that affects amplitude in one-shot processing
    let input_rms = calculate_rms(&input);
    let output_rms = calculate_rms(&output);
    let amplitude_ratio = output_rms / input_rms;

    assert!(
        (amplitude_ratio - 1.0).abs() < 0.3,
        "Amplitude not preserved: input RMS = {}, output RMS = {}, ratio = {}",
        input_rms,
        output_rms,
        amplitude_ratio
    );
}

/// Test basic downsampling (96kHz → 44.1kHz)
#[test]
fn test_downsampling_96k_to_44k() {
    let input_rate = 96000;
    let output_rate = 44100;
    let channels = 2;

    let mut resampler = Resampler::new(
        ResamplerBackend::Auto,
        input_rate,
        output_rate,
        channels,
        ResamplingQuality::High,
    )
    .unwrap();

    // Generate 1kHz sine wave at 96kHz
    let input = generate_sine_wave(1000.0, input_rate, 0.1, channels);

    let output = resampler.process(&input).unwrap();

    // Verify output size is roughly correct
    let expected_ratio = output_rate as f32 / input_rate as f32;
    let expected_output_frames = (input.len() / channels) as f32 * expected_ratio;
    let actual_output_frames = output.len() / channels;

    // Allow 50% tolerance for buffering
    assert!(
        (actual_output_frames as f32 - expected_output_frames).abs() / expected_output_frames < 0.5,
        "Output size mismatch: expected ~{}, got {}",
        expected_output_frames,
        actual_output_frames
    );

    // Verify amplitude is preserved (within 30%)
    let input_rms = calculate_rms(&input);
    let output_rms = calculate_rms(&output);
    let amplitude_ratio = output_rms / input_rms;

    assert!(
        (amplitude_ratio - 1.0).abs() < 0.3,
        "Amplitude not preserved: input RMS = {}, output RMS = {}, ratio = {}",
        input_rms,
        output_rms,
        amplitude_ratio
    );
}

/// Test all quality presets
#[test]
fn test_quality_presets() {
    let input_rate = 44100;
    let output_rate = 96000;
    let channels = 2;

    for quality in [
        ResamplingQuality::Fast,
        ResamplingQuality::Balanced,
        ResamplingQuality::High,
        ResamplingQuality::Maximum,
    ] {
        let mut resampler =
            Resampler::new(ResamplerBackend::Auto, input_rate, output_rate, channels, quality)
                .unwrap();

        let input = generate_sine_wave(1000.0, input_rate, 0.05, channels);
        let output = resampler.process(&input).unwrap();

        assert!(
            !output.is_empty(),
            "Quality preset {:?} produced empty output",
            quality
        );

        // Verify amplitude is roughly preserved (30% tolerance for buffering)
        let input_rms = calculate_rms(&input);
        let output_rms = calculate_rms(&output);
        let amplitude_ratio = output_rms / input_rms;

        assert!(
            (amplitude_ratio - 1.0).abs() < 0.3,
            "Quality preset {:?}: amplitude ratio = {}",
            quality,
            amplitude_ratio
        );
    }
}

/// Test rubato backend explicitly
#[test]
fn test_rubato_backend() {
    let input_rate = 44100;
    let output_rate = 96000;
    let channels = 2;

    let mut resampler = Resampler::new(
        ResamplerBackend::Rubato,
        input_rate,
        output_rate,
        channels,
        ResamplingQuality::High,
    )
    .unwrap();

    let input = generate_sine_wave(1000.0, input_rate, 0.1, channels);
    let output = resampler.process(&input).unwrap();

    assert!(!output.is_empty(), "Rubato backend produced empty output");

    let input_rms = calculate_rms(&input);
    let output_rms = calculate_rms(&output);
    let amplitude_ratio = output_rms / input_rms;

    assert!(
        (amplitude_ratio - 1.0).abs() < 0.3,
        "Rubato: amplitude ratio = {}",
        amplitude_ratio
    );
}

/// Test r8brain backend explicitly (requires r8brain feature)
#[test]
#[cfg(feature = "r8brain")]
fn test_r8brain_backend() {
    let input_rate = 44100;
    let output_rate = 96000;
    let channels = 2;

    let mut resampler = Resampler::new(
        ResamplerBackend::R8Brain,
        input_rate,
        output_rate,
        channels,
        ResamplingQuality::High,
    )
    .unwrap();

    let input = generate_sine_wave(1000.0, input_rate, 0.1, channels);
    let output = resampler.process(&input).unwrap();

    assert!(!output.is_empty(), "r8brain backend produced empty output");

    let input_rms = calculate_rms(&input);
    let output_rms = calculate_rms(&output);
    let amplitude_ratio = output_rms / input_rms;

    assert!(
        (amplitude_ratio - 1.0).abs() < 0.1,
        "r8brain: amplitude ratio = {}",
        amplitude_ratio
    );
}

/// Test that r8brain backend returns error when feature not enabled
#[test]
#[cfg(not(feature = "r8brain"))]
fn test_r8brain_feature_disabled() {
    let result = Resampler::new(
        ResamplerBackend::R8Brain,
        44100,
        96000,
        2,
        ResamplingQuality::High,
    );

    assert!(result.is_err(), "r8brain should error when feature disabled");
}

/// Test mono resampling
#[test]
fn test_mono_resampling() {
    let input_rate = 44100;
    let output_rate = 48000;
    let channels = 1;

    let mut resampler = Resampler::new(
        ResamplerBackend::Auto,
        input_rate,
        output_rate,
        channels,
        ResamplingQuality::Balanced,
    )
    .unwrap();

    let input = generate_sine_wave(1000.0, input_rate, 0.1, channels);
    let output = resampler.process(&input).unwrap();

    assert!(!output.is_empty(), "Mono resampling produced empty output");

    let input_rms = calculate_rms(&input);
    let output_rms = calculate_rms(&output);
    let amplitude_ratio = output_rms / input_rms;

    assert!(
        (amplitude_ratio - 1.0).abs() < 0.3,
        "Mono: amplitude ratio = {}",
        amplitude_ratio
    );
}

/// Test stereo channel independence
#[test]
fn test_stereo_channel_independence() {
    let input_rate = 44100;
    let output_rate = 96000;
    let channels = 2;

    let mut resampler = Resampler::new(
        ResamplerBackend::Auto,
        input_rate,
        output_rate,
        channels,
        ResamplingQuality::High,
    )
    .unwrap();

    // Generate different frequencies for L and R channels
    let num_samples = (input_rate as f32 * 0.1) as usize;
    let mut input = Vec::with_capacity(num_samples * 2);

    for i in 0..num_samples {
        let t = i as f32 / input_rate as f32;
        // Left: 1kHz
        let left = (2.0 * PI * 1000.0 * t).sin();
        // Right: 2kHz
        let right = (2.0 * PI * 2000.0 * t).sin();
        input.push(left);
        input.push(right);
    }

    let output = resampler.process(&input).unwrap();

    // Extract channels
    let left_in = extract_mono(&input, 0, 2);
    let right_in = extract_mono(&input, 1, 2);
    let left_out = extract_mono(&output, 0, 2);
    let right_out = extract_mono(&output, 1, 2);

    // Verify amplitudes preserved independently (30% tolerance)
    let left_ratio = calculate_rms(&left_out) / calculate_rms(&left_in);
    let right_ratio = calculate_rms(&right_out) / calculate_rms(&right_in);

    assert!(
        (left_ratio - 1.0).abs() < 0.3,
        "Left channel amplitude ratio = {}",
        left_ratio
    );
    assert!(
        (right_ratio - 1.0).abs() < 0.3,
        "Right channel amplitude ratio = {}",
        right_ratio
    );
}

/// Test reset functionality
#[test]
fn test_reset() {
    let input_rate = 44100;
    let output_rate = 96000;
    let channels = 2;

    let mut resampler = Resampler::new(
        ResamplerBackend::Auto,
        input_rate,
        output_rate,
        channels,
        ResamplingQuality::High,
    )
    .unwrap();

    let input = generate_sine_wave(1000.0, input_rate, 0.05, channels);

    // Process once
    let output1 = resampler.process(&input).unwrap();

    // Reset and process again - should give same result
    resampler.reset();
    let output2 = resampler.process(&input).unwrap();

    assert_eq!(
        output1.len(),
        output2.len(),
        "Reset changed output size"
    );

    // Verify outputs are similar (within 1% tolerance)
    let rms1 = calculate_rms(&output1);
    let rms2 = calculate_rms(&output2);

    assert!(
        (rms1 - rms2).abs() / rms1 < 0.01,
        "Reset changed output: RMS1 = {}, RMS2 = {}",
        rms1,
        rms2
    );
}

/// Test common audiophile sample rates
#[test]
fn test_audiophile_sample_rates() {
    let test_cases = vec![
        (44100, 48000),   // CD → 48kHz
        (44100, 88200),   // CD → 2x CD
        (44100, 96000),   // CD → 96kHz
        (44100, 176400),  // CD → 4x CD
        (44100, 192000),  // CD → 192kHz
        (48000, 96000),   // 48kHz → 96kHz
        (96000, 192000),  // 96kHz → 192kHz
        (192000, 96000),  // 192kHz → 96kHz (downsampling)
        (96000, 44100),   // 96kHz → CD (downsampling)
    ];

    for (input_rate, output_rate) in test_cases {
        let mut resampler = Resampler::new(
            ResamplerBackend::Auto,
            input_rate,
            output_rate,
            2,
            ResamplingQuality::High,
        )
        .unwrap();

        let input = generate_sine_wave(1000.0, input_rate, 0.05, 2);
        let output = resampler.process(&input).unwrap();

        assert!(
            !output.is_empty(),
            "{}Hz → {}Hz produced empty output",
            input_rate,
            output_rate
        );

        // Verify amplitude preserved (30% tolerance)
        let input_rms = calculate_rms(&input);
        let output_rms = calculate_rms(&output);
        let ratio = output_rms / input_rms;

        assert!(
            (ratio - 1.0).abs() < 0.3,
            "{}Hz → {}Hz: amplitude ratio = {}",
            input_rate,
            output_rate,
            ratio
        );
    }
}

/// Test output size calculation
#[test]
fn test_output_size_calculation() {
    let resampler = Resampler::new(
        ResamplerBackend::Auto,
        44100,
        96000,
        2,
        ResamplingQuality::High,
    )
    .unwrap();

    // 44.1kHz → 96kHz = ~2.177x ratio
    let input_samples = 2048;
    let expected = (input_samples as f64 * (96000.0 / 44100.0)).ceil() as usize;

    assert_eq!(
        resampler.calculate_output_size(input_samples),
        expected
    );
}
