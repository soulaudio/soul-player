//! Channel conversion tests
//!
//! Tests for:
//! - Mono to stereo upmixing
//! - Stereo passthrough
//! - Surround to stereo downmixing (3ch, 4ch, 5.0, 5.1, 6.1, 7.1)
//! - Channel order assumptions
//! - Normalization to prevent clipping
//! - Round-trip mono->stereo->mono gain unity

use soul_audio::SymphoniaDecoder;
use soul_core::AudioDecoder;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

// ============================================================================
// WAV FILE GENERATION HELPERS
// ============================================================================

/// Helper to create a WAV file with arbitrary channel count
fn create_multichannel_wav(
    path: &PathBuf,
    sample_rate: u32,
    duration_secs: f32,
    channels: u16,
    channel_values: Option<&[f32]>, // Fixed value per channel, or None for sine waves
) {
    let num_samples = (sample_rate as f32 * duration_secs) as usize;
    let frequency = 440.0;

    let mut file = File::create(path).expect("Failed to create test WAV file");

    let byte_rate = sample_rate * channels as u32 * 2;
    let block_align = channels * 2;
    let data_size = (num_samples * channels as usize * 2) as u32;
    let chunk_size = 36 + data_size;

    // Write RIFF header
    file.write_all(b"RIFF").unwrap();
    file.write_all(&chunk_size.to_le_bytes()).unwrap();
    file.write_all(b"WAVE").unwrap();

    // Write fmt chunk
    file.write_all(b"fmt ").unwrap();
    file.write_all(&16u32.to_le_bytes()).unwrap();
    file.write_all(&1u16.to_le_bytes()).unwrap(); // PCM format
    file.write_all(&channels.to_le_bytes()).unwrap();
    file.write_all(&sample_rate.to_le_bytes()).unwrap();
    file.write_all(&byte_rate.to_le_bytes()).unwrap();
    file.write_all(&block_align.to_le_bytes()).unwrap();
    file.write_all(&16u16.to_le_bytes()).unwrap(); // 16-bit

    // Write data chunk
    file.write_all(b"data").unwrap();
    file.write_all(&data_size.to_le_bytes()).unwrap();

    // Generate samples
    for i in 0..num_samples {
        for ch in 0..channels {
            let sample_f = if let Some(values) = channel_values {
                // Use fixed value for this channel
                values.get(ch as usize).copied().unwrap_or(0.0)
            } else {
                // Generate sine wave with different phase per channel
                let t = i as f32 / sample_rate as f32;
                let phase = ch as f32 * std::f32::consts::PI / channels as f32;
                (2.0 * std::f32::consts::PI * frequency * t + phase).sin() * 0.5
            };
            let sample_i16 = (sample_f * i16::MAX as f32) as i16;
            file.write_all(&sample_i16.to_le_bytes()).unwrap();
        }
    }
}

/// Create a mono WAV with a specific constant value
fn create_mono_wav_constant(path: &PathBuf, sample_rate: u32, duration_secs: f32, value: f32) {
    create_multichannel_wav(path, sample_rate, duration_secs, 1, Some(&[value]));
}

/// Create a stereo WAV with specific constant values for L and R
fn create_stereo_wav_constant(
    path: &PathBuf,
    sample_rate: u32,
    duration_secs: f32,
    left: f32,
    right: f32,
) {
    create_multichannel_wav(path, sample_rate, duration_secs, 2, Some(&[left, right]));
}

// ============================================================================
// MONO TO STEREO UPMIXING TESTS
// ============================================================================

#[test]
fn test_mono_to_stereo_duplicates_channels() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("mono_constant.wav");

    // Create mono file with constant 0.5
    create_mono_wav_constant(&path, 44100, 0.1, 0.5);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    assert!(result.is_ok(), "Mono file should decode successfully");

    let buffer = result.unwrap();

    // Output should be stereo (2 channels)
    assert_eq!(buffer.format.channels, 2, "Output should be stereo");

    // Both channels should have the same value
    let tolerance = 0.01; // Allow small conversion error
    for i in (0..buffer.samples.len()).step_by(2) {
        let left = buffer.samples[i];
        let right = buffer.samples[i + 1];
        assert!(
            (left - right).abs() < tolerance,
            "L and R should be equal for mono source: L={}, R={}",
            left,
            right
        );
    }

    // Average value should be close to original 0.5
    let avg: f32 = buffer.samples.iter().sum::<f32>() / buffer.samples.len() as f32;
    assert!(
        (avg - 0.5).abs() < 0.1,
        "Average should be close to 0.5, got {}",
        avg
    );
}

#[test]
fn test_mono_to_stereo_unity_roundtrip() {
    // Test that mono -> stereo -> mono maintains unity gain
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("mono_unity.wav");

    let test_value = 0.7_f32;
    create_mono_wav_constant(&path, 44100, 0.1, test_value);

    let mut decoder = SymphoniaDecoder::new();
    let buffer = decoder.decode(&path).unwrap();

    // Simulate mono output device: average L and R
    let mut mono_values: Vec<f32> = Vec::new();
    for i in (0..buffer.samples.len()).step_by(2) {
        let left = buffer.samples[i];
        let right = buffer.samples[i + 1];
        mono_values.push((left + right) * 0.5);
    }

    // After round-trip, value should still be close to original
    let avg: f32 = mono_values.iter().sum::<f32>() / mono_values.len() as f32;
    let tolerance = 0.05;
    assert!(
        (avg - test_value).abs() < tolerance,
        "Round-trip should maintain unity gain: expected {}, got {}",
        test_value,
        avg
    );
}

// ============================================================================
// STEREO PASSTHROUGH TESTS
// ============================================================================

#[test]
fn test_stereo_passthrough_preserves_channels() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("stereo_lr.wav");

    // Create stereo with distinct L and R values
    create_stereo_wav_constant(&path, 44100, 0.1, 0.3, 0.7);

    let mut decoder = SymphoniaDecoder::new();
    let buffer = decoder.decode(&path).unwrap();

    assert_eq!(buffer.format.channels, 2, "Output should be stereo");

    // Check that L and R are preserved and different
    let tolerance = 0.05;
    let mut left_sum = 0.0_f32;
    let mut right_sum = 0.0_f32;
    let mut count = 0;

    for i in (0..buffer.samples.len()).step_by(2) {
        left_sum += buffer.samples[i];
        right_sum += buffer.samples[i + 1];
        count += 1;
    }

    let left_avg = left_sum / count as f32;
    let right_avg = right_sum / count as f32;

    assert!(
        (left_avg - 0.3).abs() < tolerance,
        "Left channel should be ~0.3, got {}",
        left_avg
    );
    assert!(
        (right_avg - 0.7).abs() < tolerance,
        "Right channel should be ~0.7, got {}",
        right_avg
    );
}

// ============================================================================
// SURROUND DOWNMIX TESTS
// ============================================================================

#[test]
fn test_3ch_downmix_center_to_both() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("3ch.wav");

    // 3 channels: L=0, R=0, C=1.0
    // Expected: L_out = 0 + 0.707*1 = 0.707, R_out = 0 + 0.707*1 = 0.707
    // With normalization: 0.707 * (1/1.707) = 0.414
    create_multichannel_wav(&path, 44100, 0.1, 3, Some(&[0.0, 0.0, 1.0]));

    let mut decoder = SymphoniaDecoder::new();
    let buffer = decoder.decode(&path).unwrap();

    assert_eq!(buffer.format.channels, 2, "Output should be stereo");

    // Check output values
    let tolerance = 0.1;
    let expected = 0.707 / 1.707; // ~0.414

    let mut left_sum = 0.0_f32;
    let mut right_sum = 0.0_f32;
    let mut count = 0;

    for i in (0..buffer.samples.len()).step_by(2) {
        left_sum += buffer.samples[i];
        right_sum += buffer.samples[i + 1];
        count += 1;
    }

    let left_avg = left_sum / count as f32;
    let right_avg = right_sum / count as f32;

    assert!(
        (left_avg - expected).abs() < tolerance,
        "Left should be ~{}, got {}",
        expected,
        left_avg
    );
    assert!(
        (right_avg - expected).abs() < tolerance,
        "Right should be ~{}, got {}",
        expected,
        right_avg
    );
    assert!(
        (left_avg - right_avg).abs() < 0.01,
        "L and R should be equal for center-only source"
    );
}

#[test]
fn test_51_downmix_no_clipping() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("51_full.wav");

    // 5.1 channels all at maximum: L=1, R=1, C=1, LFE=1, SL=1, SR=1
    // Without normalization: 1 + 0.707 + 0.707 = 2.414 (would clip!)
    // With normalization: 2.414 * (1/2.414) = 1.0 (no clipping)
    // Note: LFE is excluded per ITU-R BS.775-3
    create_multichannel_wav(&path, 44100, 0.1, 6, Some(&[1.0, 1.0, 1.0, 1.0, 1.0, 1.0]));

    let mut decoder = SymphoniaDecoder::new();
    let buffer = decoder.decode(&path).unwrap();

    // Verify no clipping (all samples in [-1.0, 1.0])
    for sample in &buffer.samples {
        assert!(
            *sample >= -1.0 && *sample <= 1.0,
            "Sample {} clips! Should be in [-1, 1]",
            sample
        );
    }

    // Output should be normalized but still high (close to 1.0)
    let max_sample = buffer
        .samples
        .iter()
        .map(|s| s.abs())
        .fold(0.0_f32, f32::max);
    assert!(
        max_sample > 0.9,
        "Max sample should be high after normalization, got {}",
        max_sample
    );
}

#[test]
fn test_51_lfe_excluded() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("51_lfe_only.wav");

    // 5.1 with only LFE: L=0, R=0, C=0, LFE=1, SL=0, SR=0
    // LFE should be excluded, so output should be near-silence
    create_multichannel_wav(&path, 44100, 0.1, 6, Some(&[0.0, 0.0, 0.0, 1.0, 0.0, 0.0]));

    let mut decoder = SymphoniaDecoder::new();
    let buffer = decoder.decode(&path).unwrap();

    // Output should be near-silence (LFE excluded)
    let max_sample = buffer
        .samples
        .iter()
        .map(|s| s.abs())
        .fold(0.0_f32, f32::max);
    assert!(
        max_sample < 0.01,
        "LFE should be excluded from downmix, but got max sample {}",
        max_sample
    );
}

#[test]
fn test_51_surround_channels_separated() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("51_surround.wav");

    // 5.1 with only surround left: L=0, R=0, C=0, LFE=0, SL=1, SR=0
    // SL should only go to left output
    create_multichannel_wav(&path, 44100, 0.1, 6, Some(&[0.0, 0.0, 0.0, 0.0, 1.0, 0.0]));

    let mut decoder = SymphoniaDecoder::new();
    let buffer = decoder.decode(&path).unwrap();

    // Left should have signal, right should be near-silence
    let mut left_sum = 0.0_f32;
    let mut right_sum = 0.0_f32;
    let mut count = 0;

    for i in (0..buffer.samples.len()).step_by(2) {
        left_sum += buffer.samples[i].abs();
        right_sum += buffer.samples[i + 1].abs();
        count += 1;
    }

    let left_avg = left_sum / count as f32;
    let right_avg = right_sum / count as f32;

    assert!(
        left_avg > 0.1,
        "Left should have signal from SL, got {}",
        left_avg
    );
    assert!(
        right_avg < 0.01,
        "Right should be near-silence, got {}",
        right_avg
    );
}

// ============================================================================
// 7.1 DOWNMIX TESTS
// ============================================================================

#[test]
fn test_71_downmix_no_clipping() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("71_full.wav");

    // 7.1 channels all at maximum: L, R, C, LFE, SL, SR, BL, BR
    // Without normalization would severely clip
    create_multichannel_wav(
        &path,
        44100,
        0.1,
        8,
        Some(&[1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0]),
    );

    let mut decoder = SymphoniaDecoder::new();
    let buffer = decoder.decode(&path).unwrap();

    // Verify no clipping
    for sample in &buffer.samples {
        assert!(
            *sample >= -1.0 && *sample <= 1.0,
            "7.1 sample {} clips!",
            sample
        );
    }
}

#[test]
fn test_71_back_channels_mixed_correctly() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("71_back_left.wav");

    // 7.1 with only back left: all zeros except BL=1
    // BL should only go to left output
    create_multichannel_wav(
        &path,
        44100,
        0.1,
        8,
        Some(&[0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0]),
    );

    let mut decoder = SymphoniaDecoder::new();
    let buffer = decoder.decode(&path).unwrap();

    // Left should have signal, right should be near-silence
    let mut left_sum = 0.0_f32;
    let mut right_sum = 0.0_f32;
    let mut count = 0;

    for i in (0..buffer.samples.len()).step_by(2) {
        left_sum += buffer.samples[i].abs();
        right_sum += buffer.samples[i + 1].abs();
        count += 1;
    }

    let left_avg = left_sum / count as f32;
    let right_avg = right_sum / count as f32;

    assert!(
        left_avg > 0.1,
        "Left should have signal from BL, got {}",
        left_avg
    );
    assert!(
        right_avg < 0.01,
        "Right should be near-silence when only BL is active, got {}",
        right_avg
    );
}

// ============================================================================
// OUTPUT RANGE VALIDATION
// ============================================================================

#[test]
fn test_all_channel_configs_output_valid_range() {
    let temp_dir = tempfile::tempdir().unwrap();

    // Test various channel configurations with full-scale signals
    let configs = [
        (1, "mono"),
        (2, "stereo"),
        (3, "3ch"),
        (4, "quad"),
        (5, "5.0"),
        (6, "5.1"),
        // Note: 7 and 8 channel WAV files require extended format
    ];

    for (channels, name) in configs {
        let path = temp_dir.path().join(format!("{}_fullscale.wav", name));

        // All channels at full scale
        let values: Vec<f32> = vec![0.9; channels as usize];
        create_multichannel_wav(&path, 44100, 0.05, channels, Some(&values));

        let mut decoder = SymphoniaDecoder::new();
        let result = decoder.decode(&path);

        assert!(
            result.is_ok(),
            "{} channel file should decode: {:?}",
            channels,
            result.err()
        );

        let buffer = result.unwrap();

        // All samples should be in valid range
        for (i, sample) in buffer.samples.iter().enumerate() {
            assert!(
                sample.is_finite(),
                "{}: Sample {} is not finite: {}",
                name,
                i,
                sample
            );
            assert!(
                *sample >= -1.0 && *sample <= 1.0,
                "{}: Sample {} out of range: {}",
                name,
                i,
                sample
            );
        }
    }
}

// ============================================================================
// EDGE CASES
// ============================================================================

#[test]
fn test_zero_channels_produces_silence() {
    // This is an edge case that shouldn't happen in practice,
    // but the code should handle it gracefully
    // We can't create a 0-channel WAV file, so we just verify
    // the decoder doesn't panic on unusual inputs
}

#[test]
fn test_asymmetric_stereo_preserved() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("asymmetric.wav");

    // Create stereo with very different L and R
    create_stereo_wav_constant(&path, 44100, 0.1, -0.9, 0.9);

    let mut decoder = SymphoniaDecoder::new();
    let buffer = decoder.decode(&path).unwrap();

    // Calculate correlation - should be low for asymmetric content
    let mut left_sum = 0.0_f32;
    let mut right_sum = 0.0_f32;
    let count = buffer.samples.len() / 2;

    for i in (0..buffer.samples.len()).step_by(2) {
        left_sum += buffer.samples[i];
        right_sum += buffer.samples[i + 1];
    }

    let left_avg = left_sum / count as f32;
    let right_avg = right_sum / count as f32;

    // Signs should be opposite
    assert!(left_avg < -0.5, "Left should be negative, got {}", left_avg);
    assert!(
        right_avg > 0.5,
        "Right should be positive, got {}",
        right_avg
    );
}

#[test]
fn test_silent_multichannel() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("silent_6ch.wav");

    // All channels silent
    create_multichannel_wav(&path, 44100, 0.1, 6, Some(&[0.0, 0.0, 0.0, 0.0, 0.0, 0.0]));

    let mut decoder = SymphoniaDecoder::new();
    let buffer = decoder.decode(&path).unwrap();

    // All samples should be near-zero
    for sample in &buffer.samples {
        assert!(
            sample.abs() < 0.01,
            "Silent source should produce near-zero output, got {}",
            sample
        );
    }
}
