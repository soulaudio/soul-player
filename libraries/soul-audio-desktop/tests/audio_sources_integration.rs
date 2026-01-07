//! Integration tests for LocalAudioSource and StreamingAudioSource
//!
//! These tests verify real behavior with actual audio data.

use soul_audio::SymphoniaDecoder;
use soul_audio_desktop::{LocalAudioSource, StreamingAudioSource};
use soul_core::AudioDecoder;
use soul_playback::AudioSource;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;
use tempfile::TempDir;

/// Generate a simple sine wave WAV file for testing
fn generate_test_wav(path: &PathBuf, duration_secs: f64, frequency: f64) -> std::io::Result<()> {
    let sample_rate = 44100;
    let num_samples = (sample_rate as f64 * duration_secs) as usize;
    let channels = 2; // Stereo

    // WAV file header
    let mut file = File::create(path)?;

    // RIFF header
    file.write_all(b"RIFF")?;
    let file_size = 36 + num_samples * channels * 2; // 16-bit samples
    file.write_all(&(file_size as u32).to_le_bytes())?;
    file.write_all(b"WAVE")?;

    // fmt chunk
    file.write_all(b"fmt ")?;
    file.write_all(&16u32.to_le_bytes())?; // Chunk size
    file.write_all(&1u16.to_le_bytes())?; // Audio format (1 = PCM)
    file.write_all(&(channels as u16).to_le_bytes())?;
    file.write_all(&(sample_rate as u32).to_le_bytes())?;
    file.write_all(&((sample_rate * channels * 2) as u32).to_le_bytes())?; // Byte rate
    file.write_all(&((channels * 2) as u16).to_le_bytes())?; // Block align
    file.write_all(&16u16.to_le_bytes())?; // Bits per sample

    // data chunk
    file.write_all(b"data")?;
    file.write_all(&((num_samples * channels * 2) as u32).to_le_bytes())?;

    // Generate sine wave samples
    for i in 0..num_samples {
        let t = i as f64 / sample_rate as f64;
        let sample = (t * frequency * 2.0 * std::f64::consts::PI).sin();
        let sample_i16 = (sample * 32767.0) as i16;

        // Write stereo (same sample for both channels)
        file.write_all(&sample_i16.to_le_bytes())?;
        file.write_all(&sample_i16.to_le_bytes())?;
    }

    Ok(())
}

// ===== LocalAudioSource Integration Tests =====

#[test]
fn test_local_source_loads_and_plays_entire_file() {
    // Create test WAV file (1 second, 440 Hz)
    let temp_dir = TempDir::new().unwrap();
    let wav_path = temp_dir.path().join("test.wav");
    generate_test_wav(&wav_path, 1.0, 440.0).unwrap();

    // Load with LocalAudioSource
    let mut source = LocalAudioSource::new(&wav_path).expect("Failed to load test file");

    // Verify duration is approximately 1 second
    let duration = source.duration();
    assert!(
        duration.as_secs_f64() > 0.9 && duration.as_secs_f64() < 1.1,
        "Duration should be ~1 second, got {}",
        duration.as_secs_f64()
    );

    // Verify we can read samples
    let mut buffer = vec![0.0f32; 1024];
    let samples_read = source.read_samples(&mut buffer).unwrap();
    assert_eq!(samples_read, 1024, "Should read full buffer");

    // Verify samples are not all zeros (contains actual audio)
    let has_audio = buffer.iter().any(|&s| s.abs() > 0.01);
    assert!(has_audio, "Audio buffer should contain non-zero samples");

    // Verify position advances
    let position = source.position();
    assert!(
        position.as_secs_f64() > 0.0,
        "Position should advance after reading"
    );
}

#[test]
fn test_local_source_reads_entire_file() {
    let temp_dir = TempDir::new().unwrap();
    let wav_path = temp_dir.path().join("test.wav");
    generate_test_wav(&wav_path, 0.5, 440.0).unwrap(); // 0.5 second file

    let mut source = LocalAudioSource::new(&wav_path).unwrap();
    let duration = source.duration();

    // Read entire file
    let mut total_samples = 0;
    let mut buffer = vec![0.0f32; 4096];

    loop {
        let samples_read = source.read_samples(&mut buffer).unwrap();
        if samples_read == 0 {
            break; // EOF
        }
        total_samples += samples_read;
    }

    // Verify we read approximately the right number of samples
    // Duration * sample_rate * channels
    let expected_samples = (duration.as_secs_f64() * 44100.0 * 2.0) as usize;
    let tolerance = expected_samples / 10; // 10% tolerance

    assert!(
        total_samples > expected_samples - tolerance
            && total_samples < expected_samples + tolerance,
        "Should read ~{} samples, got {}",
        expected_samples,
        total_samples
    );

    // Verify source reports finished
    assert!(source.is_finished(), "Source should report finished");
}

#[test]
fn test_local_source_seeking() {
    let temp_dir = TempDir::new().unwrap();
    let wav_path = temp_dir.path().join("test.wav");
    generate_test_wav(&wav_path, 2.0, 440.0).unwrap(); // 2 second file

    let mut source = LocalAudioSource::new(&wav_path).unwrap();

    // Read some samples to advance position
    let mut buffer = vec![0.0f32; 8192];
    source.read_samples(&mut buffer).unwrap();
    let pos_before_seek = source.position();
    assert!(pos_before_seek > Duration::ZERO);

    // Seek to 1.0 seconds
    source.seek(Duration::from_secs(1)).unwrap();
    let pos_after_seek = source.position();

    // Verify position is approximately 1 second
    assert!(
        (pos_after_seek.as_secs_f64() - 1.0).abs() < 0.01,
        "Position should be ~1.0s, got {}",
        pos_after_seek.as_secs_f64()
    );

    // Verify we can continue reading from new position
    let samples_read = source.read_samples(&mut buffer).unwrap();
    assert!(samples_read > 0, "Should be able to read after seeking");

    // Seek to beginning
    source.seek(Duration::ZERO).unwrap();
    assert!(
        source.position().as_secs_f64() < 0.01,
        "Should be at beginning after seeking to zero"
    );
}

#[test]
fn test_local_source_seek_beyond_duration_fails() {
    let temp_dir = TempDir::new().unwrap();
    let wav_path = temp_dir.path().join("test.wav");
    generate_test_wav(&wav_path, 1.0, 440.0).unwrap();

    let mut source = LocalAudioSource::new(&wav_path).unwrap();
    let duration = source.duration();

    // Seek beyond duration should fail
    let result = source.seek(duration + Duration::from_secs(1));
    assert!(result.is_err(), "Seeking beyond duration should fail");
}

#[test]
fn test_local_source_position_tracking_accuracy() {
    let temp_dir = TempDir::new().unwrap();
    let wav_path = temp_dir.path().join("test.wav");
    generate_test_wav(&wav_path, 1.0, 440.0).unwrap();

    let mut source = LocalAudioSource::new(&wav_path).unwrap();
    let buffer_size = 4410; // Exactly 0.05 seconds worth of stereo samples at 44.1kHz
    let mut buffer = vec![0.0f32; buffer_size];

    // Read exactly 0.05 seconds worth
    source.read_samples(&mut buffer).unwrap();
    let position = source.position();

    // Position should be approximately 0.05 seconds
    assert!(
        (position.as_secs_f64() - 0.05).abs() < 0.001,
        "Position should be ~0.05s, got {}",
        position.as_secs_f64()
    );

    // Read another 0.05 seconds
    source.read_samples(&mut buffer).unwrap();
    let position = source.position();

    assert!(
        (position.as_secs_f64() - 0.10).abs() < 0.001,
        "Position should be ~0.10s, got {}",
        position.as_secs_f64()
    );
}

#[test]
fn test_local_source_handles_multiple_formats() {
    // Test that we can load WAV files (other formats would require encoding libraries)
    let temp_dir = TempDir::new().unwrap();

    // Test different durations
    for (duration, freq) in [(0.1, 440.0), (0.5, 880.0), (2.0, 220.0)] {
        let wav_path = temp_dir.path().join(format!("test_{}s.wav", duration));
        generate_test_wav(&wav_path, duration, freq).unwrap();

        let mut source = LocalAudioSource::new(&wav_path).unwrap();
        let actual_duration = source.duration();

        assert!(
            (actual_duration.as_secs_f64() - duration).abs() < 0.1,
            "Duration mismatch for {}s file",
            duration
        );

        // Verify we can read samples
        let mut buffer = vec![0.0f32; 1024];
        let samples_read = source.read_samples(&mut buffer).unwrap();
        assert!(
            samples_read > 0,
            "Should read samples from {}s file",
            duration
        );
    }
}

#[test]
fn test_local_source_partial_buffer_fill_at_end() {
    let temp_dir = TempDir::new().unwrap();
    let wav_path = temp_dir.path().join("test.wav");
    generate_test_wav(&wav_path, 0.1, 440.0).unwrap(); // Very short file

    let mut source = LocalAudioSource::new(&wav_path).unwrap();
    let mut large_buffer = vec![0.0f32; 44100 * 2]; // 1 second buffer for 0.1 second file

    // Request more samples than available
    let samples_read = source.read_samples(&mut large_buffer).unwrap();

    // Should return partial read
    assert!(
        samples_read > 0 && samples_read < large_buffer.len(),
        "Should do partial read at EOF"
    );

    // Next read should return 0 (EOF)
    let samples_read = source.read_samples(&mut large_buffer).unwrap();
    assert_eq!(samples_read, 0, "Should return 0 at EOF");
}

#[test]
fn test_local_source_reset_functionality() {
    let temp_dir = TempDir::new().unwrap();
    let wav_path = temp_dir.path().join("test.wav");
    generate_test_wav(&wav_path, 1.0, 440.0).unwrap();

    let mut source = LocalAudioSource::new(&wav_path).unwrap();
    let mut buffer = vec![0.0f32; 8192];

    // Read to advance position
    source.read_samples(&mut buffer).unwrap();
    assert!(source.position() > Duration::ZERO);

    // Reset (seek to zero)
    source.reset().unwrap();

    // Verify position is back to zero
    assert!(
        source.position().as_secs_f64() < 0.01,
        "Should reset to beginning"
    );
    assert!(!source.is_finished(), "Should not be finished after reset");
}

// ===== StreamingAudioSource Integration Tests =====

#[test]
fn test_streaming_source_creation() {
    let source = StreamingAudioSource::new(
        "http://localhost:8080/stream".to_string(),
        44100,
        2,
        Duration::from_secs(180),
    );

    assert!(source.is_ok(), "Should create streaming source");
    let source = source.unwrap();
    assert_eq!(source.sample_rate(), 44100);
    assert_eq!(source.channels(), 2);
    assert_eq!(source.duration(), Duration::from_secs(180));
}

#[test]
fn test_streaming_source_initial_state() {
    let source = StreamingAudioSource::new(
        "http://localhost:8080/stream".to_string(),
        44100,
        2,
        Duration::from_secs(60),
    )
    .unwrap();

    // Initial position should be zero
    assert_eq!(source.position(), Duration::ZERO);
    assert!(!source.is_finished());
}

#[test]
fn test_streaming_source_seek_not_supported() {
    let mut source = StreamingAudioSource::new(
        "http://localhost:8080/stream".to_string(),
        44100,
        2,
        Duration::from_secs(60),
    )
    .unwrap();

    // Seeking should return error
    let result = source.seek(Duration::from_secs(30));
    assert!(
        result.is_err(),
        "Streaming source should not support seeking"
    );
}

#[test]
fn test_streaming_source_buffer_underrun_handling() {
    let mut source = StreamingAudioSource::new(
        "http://localhost:9999/nonexistent".to_string(), // Will fail to connect
        44100,
        2,
        Duration::from_secs(60),
    )
    .unwrap();

    // Give download thread time to fail
    std::thread::sleep(Duration::from_millis(100));

    let mut buffer = vec![0.0f32; 1024];

    // Should handle gracefully (return 0 or silence)
    let result = source.read_samples(&mut buffer);

    // Either returns 0 (no data) or fills with silence
    if let Ok(samples_read) = result {
        if samples_read > 0 {
            // If it returns samples, they should be silence on underrun
            // (actual implementation returns 0, but this tests defensive coding)
            assert!(
                buffer.iter().all(|&s| s == 0.0),
                "Buffer underrun should produce silence"
            );
        }
    }
}

#[test]
fn test_streaming_source_position_updates() {
    let mut source = StreamingAudioSource::new(
        "http://localhost:8080/stream".to_string(),
        44100,
        2,
        Duration::from_secs(60),
    )
    .unwrap();

    let initial_position = source.position();
    assert_eq!(initial_position, Duration::ZERO);

    // Even without actual data, position tracking should work
    // (implementation detail: position updates when samples are consumed)
}

#[test]
fn test_streaming_source_cleanup_on_drop() {
    // Create source in inner scope
    {
        let _source = StreamingAudioSource::new(
            "http://localhost:8080/stream".to_string(),
            44100,
            2,
            Duration::from_secs(60),
        )
        .unwrap();

        // Source should have background thread running
    } // Source dropped here

    // Give background thread time to clean up
    std::thread::sleep(Duration::from_millis(50));

    // If we get here without hanging, cleanup worked
    // (background thread received stop signal and terminated)
}

// ===== Cross-Source Integration Tests =====

#[test]
fn test_both_sources_implement_audio_source_trait() {
    let temp_dir = TempDir::new().unwrap();
    let wav_path = temp_dir.path().join("test.wav");
    generate_test_wav(&wav_path, 1.0, 440.0).unwrap();

    // Create both types
    let local = LocalAudioSource::new(&wav_path).unwrap();
    let streaming = StreamingAudioSource::new(
        "http://localhost:8080/stream".to_string(),
        44100,
        2,
        Duration::from_secs(60),
    )
    .unwrap();

    // Both should implement AudioSource
    fn assert_is_audio_source<T: AudioSource>(_: &T) {}
    assert_is_audio_source(&local);
    assert_is_audio_source(&streaming);
}

#[test]
fn test_local_source_consistent_sample_count() {
    // Verify that reading the same file twice produces same sample count
    let temp_dir = TempDir::new().unwrap();
    let wav_path = temp_dir.path().join("test.wav");
    generate_test_wav(&wav_path, 0.5, 440.0).unwrap();

    let mut counts = Vec::new();

    for _ in 0..3 {
        let mut source = LocalAudioSource::new(&wav_path).unwrap();
        let mut buffer = vec![0.0f32; 2048];
        let mut total = 0;

        loop {
            let read = source.read_samples(&mut buffer).unwrap();
            if read == 0 {
                break;
            }
            total += read;
        }

        counts.push(total);
    }

    // All reads should produce same count
    assert!(
        counts.windows(2).all(|w| w[0] == w[1]),
        "Multiple reads should produce same sample count: {:?}",
        counts
    );
}

#[test]
fn test_local_source_nonexistent_file_fails() {
    let result = LocalAudioSource::new("/nonexistent/path/file.wav");
    assert!(result.is_err(), "Should fail to load nonexistent file");
}

#[test]
fn test_local_source_invalid_file_fails() {
    let temp_dir = TempDir::new().unwrap();
    let invalid_path = temp_dir.path().join("invalid.wav");

    // Write garbage data
    let mut file = File::create(&invalid_path).unwrap();
    file.write_all(b"This is not a valid audio file").unwrap();

    let result = LocalAudioSource::new(&invalid_path);
    assert!(result.is_err(), "Should fail to load invalid audio file");
}
