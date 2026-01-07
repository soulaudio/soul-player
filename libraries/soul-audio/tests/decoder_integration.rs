/// Integration tests for audio decoder
///
/// These tests verify that the decoder correctly processes real audio files
/// and produces valid output buffers with proper sample values.
use soul_audio::SymphoniaDecoder;
use soul_core::AudioDecoder;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

/// Helper to create a simple WAV file for testing
///
/// Creates a 440 Hz sine wave (A4 note) for the specified duration
fn create_test_wav(path: &PathBuf, sample_rate: u32, duration_secs: f32, channels: u16) {
    let num_samples = (sample_rate as f32 * duration_secs) as usize;
    let frequency = 440.0; // A4 note

    // Create WAV header
    let mut file = File::create(path).expect("Failed to create test WAV file");

    let byte_rate = sample_rate * channels as u32 * 2; // 16-bit samples
    let block_align = channels * 2;
    let data_size = (num_samples * channels as usize * 2) as u32;
    let chunk_size = 36 + data_size;

    // Write RIFF header
    file.write_all(b"RIFF").unwrap();
    file.write_all(&chunk_size.to_le_bytes()).unwrap();
    file.write_all(b"WAVE").unwrap();

    // Write fmt chunk
    file.write_all(b"fmt ").unwrap();
    file.write_all(&16u32.to_le_bytes()).unwrap(); // fmt chunk size
    file.write_all(&1u16.to_le_bytes()).unwrap(); // PCM format
    file.write_all(&channels.to_le_bytes()).unwrap();
    file.write_all(&sample_rate.to_le_bytes()).unwrap();
    file.write_all(&byte_rate.to_le_bytes()).unwrap();
    file.write_all(&block_align.to_le_bytes()).unwrap();
    file.write_all(&16u16.to_le_bytes()).unwrap(); // bits per sample

    // Write data chunk
    file.write_all(b"data").unwrap();
    file.write_all(&data_size.to_le_bytes()).unwrap();

    // Generate sine wave samples
    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let sample_f = (2.0 * std::f32::consts::PI * frequency * t).sin();
        let sample_i16 = (sample_f * i16::MAX as f32) as i16;

        // Write for each channel
        for _ in 0..channels {
            file.write_all(&sample_i16.to_le_bytes()).unwrap();
        }
    }
}

#[test]
fn test_decode_stereo_wav() {
    // Create a temporary WAV file
    let temp_dir = tempfile::tempdir().unwrap();
    let wav_path = temp_dir.path().join("test_stereo.wav");

    // Create a 1-second stereo WAV at 44.1 kHz
    create_test_wav(&wav_path, 44100, 1.0, 2);

    // Decode the file
    let mut decoder = SymphoniaDecoder::new();
    let buffer = decoder
        .decode(&wav_path)
        .expect("Failed to decode stereo WAV");

    // Verify basic properties
    assert_eq!(buffer.format.sample_rate.0, 44100);
    assert_eq!(buffer.format.channels, 2);
    assert_eq!(buffer.format.bits_per_sample, 32); // f32 output

    // Verify sample count (stereo = 2 * num_samples)
    let expected_samples = 44100 * 2; // 1 second * sample rate * channels
    assert_eq!(buffer.samples.len(), expected_samples);

    // Verify all samples are in valid range [-1.0, 1.0]
    for sample in &buffer.samples {
        assert!(
            *sample >= -1.0 && *sample <= 1.0,
            "Sample out of range: {}",
            sample
        );
    }

    // Verify samples are not all zeros (actual audio content)
    let non_zero_samples: Vec<_> = buffer
        .samples
        .iter()
        .filter(|&&s| s.abs() > 0.001)
        .collect();
    assert!(
        non_zero_samples.len() > 1000,
        "Expected significant audio content"
    );
}

#[test]
fn test_decode_mono_wav() {
    let temp_dir = tempfile::tempdir().unwrap();
    let wav_path = temp_dir.path().join("test_mono.wav");

    // Create a 0.5-second mono WAV at 48 kHz
    create_test_wav(&wav_path, 48000, 0.5, 1);

    let mut decoder = SymphoniaDecoder::new();
    let buffer = decoder
        .decode(&wav_path)
        .expect("Failed to decode mono WAV");

    // Verify properties
    assert_eq!(buffer.format.sample_rate.0, 48000);
    assert_eq!(buffer.format.channels, 1);

    // Verify sample count (mono files are converted to stereo by duplicating)
    // Expected: 48000 * 0.5 = 24000 samples per channel -> 48000 interleaved
    let expected_samples = 48000; // Stereo output
    assert_eq!(buffer.samples.len(), expected_samples);

    // Verify all samples are in valid range
    for sample in &buffer.samples {
        assert!(
            *sample >= -1.0 && *sample <= 1.0,
            "Sample out of range: {}",
            sample
        );
    }
}

#[test]
fn test_decode_different_sample_rates() {
    let temp_dir = tempfile::tempdir().unwrap();

    let test_cases = vec![
        (22050, "22khz.wav"),
        (44100, "44khz.wav"),
        (48000, "48khz.wav"),
        (96000, "96khz.wav"),
    ];

    for (sample_rate, filename) in test_cases {
        let wav_path = temp_dir.path().join(filename);
        create_test_wav(&wav_path, sample_rate, 0.1, 2);

        let mut decoder = SymphoniaDecoder::new();
        let buffer = decoder
            .decode(&wav_path)
            .unwrap_or_else(|_| panic!("Failed to decode WAV at {} Hz", sample_rate));

        // Verify sample rate is preserved
        assert_eq!(buffer.format.sample_rate.0, sample_rate);

        // Verify sample count matches duration
        let expected_samples = (sample_rate as f32 * 0.1 * 2.0) as usize; // 0.1s * 2 channels
        assert_eq!(buffer.samples.len(), expected_samples);
    }
}

#[test]
fn test_decode_nonexistent_file() {
    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&PathBuf::from("/nonexistent/file.wav"));

    assert!(result.is_err());
}

#[test]
fn test_decode_invalid_file() {
    // Create a file with invalid content
    let temp_dir = tempfile::tempdir().unwrap();
    let invalid_path = temp_dir.path().join("invalid.wav");

    let mut file = File::create(&invalid_path).unwrap();
    file.write_all(b"This is not a valid WAV file").unwrap();
    drop(file);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&invalid_path);

    assert!(result.is_err());
}

#[test]
fn test_supports_format() {
    let decoder = SymphoniaDecoder::new();

    // Supported formats
    assert!(decoder.supports_format(&PathBuf::from("test.mp3")));
    assert!(decoder.supports_format(&PathBuf::from("test.flac")));
    assert!(decoder.supports_format(&PathBuf::from("test.ogg")));
    assert!(decoder.supports_format(&PathBuf::from("test.opus")));
    assert!(decoder.supports_format(&PathBuf::from("test.wav")));
    assert!(decoder.supports_format(&PathBuf::from("test.m4a")));
    assert!(decoder.supports_format(&PathBuf::from("test.aac")));

    // Case insensitive
    assert!(decoder.supports_format(&PathBuf::from("TEST.MP3")));
    assert!(decoder.supports_format(&PathBuf::from("Test.Flac")));

    // Unsupported formats
    assert!(!decoder.supports_format(&PathBuf::from("test.txt")));
    assert!(!decoder.supports_format(&PathBuf::from("test.pdf")));
    assert!(!decoder.supports_format(&PathBuf::from("test")));
}

#[test]
fn test_signal_properties() {
    // Verify that decoded audio maintains signal properties
    let temp_dir = tempfile::tempdir().unwrap();
    let wav_path = temp_dir.path().join("sine_wave.wav");

    create_test_wav(&wav_path, 44100, 0.1, 2);

    let mut decoder = SymphoniaDecoder::new();
    let buffer = decoder.decode(&wav_path).unwrap();

    // Check that we have alternating left/right channel samples (interleaved)
    // Both channels should have the same content (sine wave)
    let left_samples: Vec<f32> = buffer.samples.iter().step_by(2).copied().collect();
    let right_samples: Vec<f32> = buffer.samples.iter().skip(1).step_by(2).copied().collect();

    assert_eq!(left_samples.len(), right_samples.len());

    // Verify both channels have similar content (sine wave)
    // They should be very close since we wrote the same data to both channels
    for (left, right) in left_samples.iter().zip(right_samples.iter()) {
        let diff = (left - right).abs();
        assert!(
            diff < 0.01,
            "Channels should contain similar content, diff: {}",
            diff
        );
    }

    // Verify peak amplitude is close to 1.0 (sine wave should reach near maximum)
    let max_amplitude = buffer
        .samples
        .iter()
        .map(|s| s.abs())
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();

    assert!(
        max_amplitude > 0.9,
        "Peak amplitude should be close to 1.0, got: {}",
        max_amplitude
    );
}

#[test]
fn test_zero_crossing_detection() {
    // Test that we can detect zero crossings in a sine wave
    // This verifies that the audio signal is properly decoded
    let temp_dir = tempfile::tempdir().unwrap();
    let wav_path = temp_dir.path().join("sine_440hz.wav");

    create_test_wav(&wav_path, 44100, 0.1, 1);

    let mut decoder = SymphoniaDecoder::new();
    let buffer = decoder.decode(&wav_path).unwrap();

    // Count zero crossings (sign changes)
    let mut zero_crossings = 0;
    for i in 1..buffer.samples.len() {
        if (buffer.samples[i - 1] < 0.0 && buffer.samples[i] >= 0.0)
            || (buffer.samples[i - 1] >= 0.0 && buffer.samples[i] < 0.0)
        {
            zero_crossings += 1;
        }
    }

    // For a 440 Hz sine wave over 0.1 seconds, we expect ~44 zero crossings
    // (440 Hz * 0.1s * 2 crossings per cycle)
    // Allow some tolerance for quantization effects
    assert!(
        zero_crossings > 70 && zero_crossings < 100,
        "Expected ~88 zero crossings (440 Hz sine), got: {}",
        zero_crossings
    );
}
