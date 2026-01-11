//! Decoder edge case tests
//!
//! Tests for:
//! - Corrupted audio files
//! - Truncated files
//! - Zero-length files
//! - Invalid metadata
//! - Unusual channel configurations
//! - Very short files
//! - Large file handling

use soul_audio::SymphoniaDecoder;
use soul_core::AudioDecoder;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

/// Helper to create a simple WAV file for testing
fn create_test_wav(path: &PathBuf, sample_rate: u32, duration_secs: f32, channels: u16) {
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
    file.write_all(&1u16.to_le_bytes()).unwrap();
    file.write_all(&channels.to_le_bytes()).unwrap();
    file.write_all(&sample_rate.to_le_bytes()).unwrap();
    file.write_all(&byte_rate.to_le_bytes()).unwrap();
    file.write_all(&block_align.to_le_bytes()).unwrap();
    file.write_all(&16u16.to_le_bytes()).unwrap();

    // Write data chunk
    file.write_all(b"data").unwrap();
    file.write_all(&data_size.to_le_bytes()).unwrap();

    // Generate sine wave samples
    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let sample_f = (2.0 * std::f32::consts::PI * frequency * t).sin();
        let sample_i16 = (sample_f * i16::MAX as f32) as i16;

        for _ in 0..channels {
            file.write_all(&sample_i16.to_le_bytes()).unwrap();
        }
    }
}

/// Create a WAV file with only header (no data)
fn create_empty_data_wav(path: &PathBuf, sample_rate: u32, channels: u16) {
    let mut file = File::create(path).expect("Failed to create test WAV file");

    let byte_rate = sample_rate * channels as u32 * 2;
    let block_align = channels * 2;
    let data_size = 0u32; // No data
    let chunk_size = 36 + data_size;

    file.write_all(b"RIFF").unwrap();
    file.write_all(&chunk_size.to_le_bytes()).unwrap();
    file.write_all(b"WAVE").unwrap();
    file.write_all(b"fmt ").unwrap();
    file.write_all(&16u32.to_le_bytes()).unwrap();
    file.write_all(&1u16.to_le_bytes()).unwrap();
    file.write_all(&channels.to_le_bytes()).unwrap();
    file.write_all(&sample_rate.to_le_bytes()).unwrap();
    file.write_all(&byte_rate.to_le_bytes()).unwrap();
    file.write_all(&block_align.to_le_bytes()).unwrap();
    file.write_all(&16u16.to_le_bytes()).unwrap();
    file.write_all(b"data").unwrap();
    file.write_all(&data_size.to_le_bytes()).unwrap();
}

// ============================================================================
// CORRUPTED FILE TESTS
// ============================================================================

#[test]
fn test_random_garbage_data() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("garbage.wav");

    let mut file = File::create(&path).unwrap();
    // Write random garbage
    for i in 0..1024 {
        file.write_all(&[(i % 256) as u8]).unwrap();
    }
    drop(file);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    // Should return error, not panic
    assert!(result.is_err(), "Random garbage should fail to decode");
}

#[test]
fn test_truncated_riff_header() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("truncated_header.wav");

    let mut file = File::create(&path).unwrap();
    // Write partial RIFF header
    file.write_all(b"RIF").unwrap(); // Missing the 'F'
    drop(file);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    assert!(result.is_err(), "Truncated header should fail to decode");
}

#[test]
fn test_truncated_fmt_chunk() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("truncated_fmt.wav");

    let mut file = File::create(&path).unwrap();
    file.write_all(b"RIFF").unwrap();
    file.write_all(&100u32.to_le_bytes()).unwrap();
    file.write_all(b"WAVE").unwrap();
    file.write_all(b"fmt ").unwrap();
    file.write_all(&16u32.to_le_bytes()).unwrap();
    // Missing fmt chunk data
    drop(file);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    assert!(result.is_err(), "Truncated fmt chunk should fail to decode");
}

#[test]
fn test_truncated_audio_data() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("truncated_data.wav");

    // Create a valid WAV first
    create_test_wav(&path, 44100, 1.0, 2);

    // Now truncate the file to cut off audio data
    let file = File::options().write(true).open(&path).unwrap();
    file.set_len(100).unwrap(); // Keep only 100 bytes (header without data)
    drop(file);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    // Should either error or return partial data
    // The exact behavior depends on Symphonia's handling
    // We just verify it doesn't panic
    let _ = result; // We accept either success with partial data or error
}

#[test]
fn test_corrupted_sample_data() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("corrupted_samples.wav");

    let mut file = File::create(&path).unwrap();

    let sample_rate = 44100u32;
    let channels = 2u16;
    let num_samples = 4410; // 0.1 seconds
    let byte_rate = sample_rate * channels as u32 * 2;
    let block_align = channels * 2;
    let data_size = (num_samples * channels as usize * 2) as u32;
    let chunk_size = 36 + data_size;

    // Write valid header
    file.write_all(b"RIFF").unwrap();
    file.write_all(&chunk_size.to_le_bytes()).unwrap();
    file.write_all(b"WAVE").unwrap();
    file.write_all(b"fmt ").unwrap();
    file.write_all(&16u32.to_le_bytes()).unwrap();
    file.write_all(&1u16.to_le_bytes()).unwrap();
    file.write_all(&channels.to_le_bytes()).unwrap();
    file.write_all(&sample_rate.to_le_bytes()).unwrap();
    file.write_all(&byte_rate.to_le_bytes()).unwrap();
    file.write_all(&block_align.to_le_bytes()).unwrap();
    file.write_all(&16u16.to_le_bytes()).unwrap();
    file.write_all(b"data").unwrap();
    file.write_all(&data_size.to_le_bytes()).unwrap();

    // Write corrupted data (this is actually valid - just not audio)
    for i in 0..(num_samples * channels as usize) {
        let corrupted_sample = (i % 65536) as i16;
        file.write_all(&corrupted_sample.to_le_bytes()).unwrap();
    }
    drop(file);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    // This should succeed - the data is "valid" format-wise
    // but the audio will sound like garbage
    assert!(
        result.is_ok(),
        "Corrupted (but valid format) samples should decode"
    );

    let buffer = result.unwrap();
    // Verify all samples are in valid range
    for sample in &buffer.samples {
        assert!(sample.is_finite(), "Sample should be finite");
        assert!(
            *sample >= -1.0 && *sample <= 1.0,
            "Sample out of range: {}",
            sample
        );
    }
}

// ============================================================================
// ZERO-LENGTH AND EMPTY FILE TESTS
// ============================================================================

#[test]
fn test_zero_length_file() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("zero_length.wav");

    // Create empty file
    File::create(&path).unwrap();

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    assert!(result.is_err(), "Zero-length file should fail to decode");
}

#[test]
fn test_wav_with_empty_data_chunk() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("empty_data.wav");

    create_empty_data_wav(&path, 44100, 2);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    // Should either error or return empty buffer
    match result {
        Ok(buffer) => {
            assert!(
                buffer.samples.is_empty(),
                "Empty data chunk should produce empty samples"
            );
        }
        Err(_) => {
            // Also acceptable
        }
    }
}

#[test]
fn test_wav_with_only_header() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("header_only.wav");

    let mut file = File::create(&path).unwrap();
    file.write_all(b"RIFF").unwrap();
    file.write_all(&4u32.to_le_bytes()).unwrap();
    file.write_all(b"WAVE").unwrap();
    // No fmt or data chunks
    drop(file);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    assert!(result.is_err(), "Header-only WAV should fail to decode");
}

// ============================================================================
// UNUSUAL CHANNEL CONFIGURATIONS
// ============================================================================

#[test]
fn test_mono_file() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("mono.wav");

    create_test_wav(&path, 44100, 0.1, 1);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    assert!(result.is_ok(), "Mono file should decode successfully");

    let buffer = result.unwrap();
    // Decoder should output stereo by duplicating mono channel
    assert!(buffer.samples.len() > 0);
}

#[test]
fn test_stereo_file() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("stereo.wav");

    create_test_wav(&path, 44100, 0.1, 2);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    assert!(result.is_ok(), "Stereo file should decode successfully");

    let buffer = result.unwrap();
    assert_eq!(buffer.format.channels, 2);
}

// ============================================================================
// VERY SHORT FILES
// ============================================================================

#[test]
fn test_single_sample_file() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("single_sample.wav");

    let mut file = File::create(&path).unwrap();

    let sample_rate = 44100u32;
    let channels = 2u16;
    let data_size = (channels as u32) * 2; // Single stereo sample
    let chunk_size = 36 + data_size;

    file.write_all(b"RIFF").unwrap();
    file.write_all(&chunk_size.to_le_bytes()).unwrap();
    file.write_all(b"WAVE").unwrap();
    file.write_all(b"fmt ").unwrap();
    file.write_all(&16u32.to_le_bytes()).unwrap();
    file.write_all(&1u16.to_le_bytes()).unwrap();
    file.write_all(&channels.to_le_bytes()).unwrap();
    file.write_all(&sample_rate.to_le_bytes()).unwrap();
    file.write_all(&(sample_rate * channels as u32 * 2).to_le_bytes())
        .unwrap();
    file.write_all(&(channels * 2).to_le_bytes()).unwrap();
    file.write_all(&16u16.to_le_bytes()).unwrap();
    file.write_all(b"data").unwrap();
    file.write_all(&data_size.to_le_bytes()).unwrap();

    // Single stereo sample
    file.write_all(&1000i16.to_le_bytes()).unwrap();
    file.write_all(&(-1000i16).to_le_bytes()).unwrap();
    drop(file);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    assert!(result.is_ok(), "Single sample file should decode");

    let buffer = result.unwrap();
    assert_eq!(buffer.samples.len(), 2); // One stereo frame = 2 samples
}

#[test]
fn test_very_short_file_10ms() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("short_10ms.wav");

    create_test_wav(&path, 44100, 0.01, 2); // 10ms

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    assert!(result.is_ok(), "10ms file should decode");

    let buffer = result.unwrap();
    // 44100 * 0.01 * 2 = 882 samples
    assert!(buffer.samples.len() >= 800 && buffer.samples.len() <= 900);
}

#[test]
fn test_very_short_file_1ms() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("short_1ms.wav");

    create_test_wav(&path, 44100, 0.001, 2); // 1ms

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    assert!(result.is_ok(), "1ms file should decode");

    let buffer = result.unwrap();
    // 44100 * 0.001 * 2 = ~88 samples
    assert!(buffer.samples.len() >= 80 && buffer.samples.len() <= 100);
}

// ============================================================================
// SAMPLE RATE EDGE CASES
// ============================================================================

#[test]
fn test_very_low_sample_rate() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("low_rate_8000.wav");

    create_test_wav(&path, 8000, 0.1, 2);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    assert!(result.is_ok(), "8kHz file should decode");

    let buffer = result.unwrap();
    assert_eq!(buffer.format.sample_rate.0, 8000);
}

#[test]
fn test_high_sample_rate_192khz() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("high_rate_192000.wav");

    create_test_wav(&path, 192000, 0.1, 2);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    assert!(result.is_ok(), "192kHz file should decode");

    let buffer = result.unwrap();
    assert_eq!(buffer.format.sample_rate.0, 192000);
}

#[test]
fn test_non_standard_sample_rate() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("odd_rate.wav");

    create_test_wav(&path, 37800, 0.1, 2); // Non-standard rate

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    assert!(result.is_ok(), "Non-standard sample rate should decode");

    let buffer = result.unwrap();
    assert_eq!(buffer.format.sample_rate.0, 37800);
}

// ============================================================================
// FILE PATH EDGE CASES
// ============================================================================

#[test]
fn test_nonexistent_file() {
    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&PathBuf::from("/nonexistent/path/to/file.wav"));

    assert!(result.is_err(), "Nonexistent file should fail");
}

#[test]
fn test_directory_instead_of_file() {
    let temp_dir = tempfile::tempdir().unwrap();

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(temp_dir.path());

    assert!(result.is_err(), "Directory path should fail");
}

#[test]
fn test_unicode_filename() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir
        .path()
        .join("test_\u{4E2D}\u{6587}_\u{65E5}\u{672C}\u{8A9E}.wav");

    create_test_wav(&path, 44100, 0.1, 2);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    assert!(result.is_ok(), "Unicode filename should work");
}

#[test]
fn test_filename_with_spaces() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("test file with spaces.wav");

    create_test_wav(&path, 44100, 0.1, 2);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    assert!(result.is_ok(), "Filename with spaces should work");
}

// ============================================================================
// FORMAT DETECTION
// ============================================================================

#[test]
fn test_wrong_extension() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("audio.txt"); // Wrong extension

    create_test_wav(&path, 44100, 0.1, 2);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    // Symphonia should still detect the format from content
    // Note: this might fail if Symphonia relies on extension hints
    let _ = result; // Accept either outcome
}

#[test]
fn test_no_extension() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("audio_file_no_extension");

    create_test_wav(&path, 44100, 0.1, 2);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    // Accept either success (content-based detection) or failure
    let _ = result;
}

// ============================================================================
// LARGE FILE SIMULATION
// ============================================================================

#[test]
fn test_moderately_large_file() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("large_file.wav");

    // Create a 30-second file (~2.6 MB at 44.1kHz stereo 16-bit)
    create_test_wav(&path, 44100, 30.0, 2);

    let mut decoder = SymphoniaDecoder::new();
    let result = decoder.decode(&path);

    assert!(result.is_ok(), "30-second file should decode");

    let buffer = result.unwrap();
    // 44100 * 30 * 2 = 2,646,000 samples
    let expected = (44100.0 * 30.0 * 2.0) as usize;
    assert!(
        (buffer.samples.len() as i64 - expected as i64).abs() < 1000,
        "Expected ~{} samples, got {}",
        expected,
        buffer.samples.len()
    );
}

// ============================================================================
// CONCURRENT DECODING
// ============================================================================

#[test]
fn test_multiple_decoders_concurrent() {
    use std::thread;

    let temp_dir = tempfile::tempdir().unwrap();

    // Create multiple test files
    let paths: Vec<_> = (0..4)
        .map(|i| {
            let path = temp_dir.path().join(format!("concurrent_{}.wav", i));
            create_test_wav(&path, 44100, 0.5, 2);
            path
        })
        .collect();

    // Decode concurrently
    let handles: Vec<_> = paths
        .into_iter()
        .map(|path| {
            thread::spawn(move || {
                let mut decoder = SymphoniaDecoder::new();
                decoder.decode(&path)
            })
        })
        .collect();

    // All should succeed
    for handle in handles {
        let result = handle.join().expect("Thread panicked");
        assert!(result.is_ok(), "Concurrent decoding should succeed");
    }
}

#[test]
fn test_sequential_decode_many_files() {
    let temp_dir = tempfile::tempdir().unwrap();
    let mut decoder = SymphoniaDecoder::new();

    // Decode 20 files sequentially
    for i in 0..20 {
        let path = temp_dir.path().join(format!("sequential_{}.wav", i));
        create_test_wav(&path, 44100, 0.1, 2);

        let result = decoder.decode(&path);
        assert!(result.is_ok(), "Sequential decode {} should succeed", i);
    }
}
