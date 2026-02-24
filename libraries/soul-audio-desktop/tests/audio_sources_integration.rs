//! Integration tests for `LocalAudioSource` and `StreamingAudioSource`
//!
//! These tests verify real behavior with actual audio data.

use soul_audio_desktop::{LocalAudioSource, StreamingAudioSource};
use soul_playback::AudioSource;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;
use tempfile::TempDir;

/// Helper to wait for an async seek to complete by polling position
/// Returns true if position reached target within timeout, false otherwise
fn wait_for_seek_complete(source: &dyn AudioSource, target_secs: f64, tolerance: f64) -> bool {
    for _ in 0..100 {
        let pos = source.position().as_secs_f64();
        if (pos - target_secs).abs() < tolerance {
            return true;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    false
}

/// Helper to wait for source to be ready for playback
/// The background decoder thread needs time to fill the buffer
/// Returns true if source became ready within timeout, false otherwise
fn wait_for_ready(source: &dyn AudioSource, timeout_ms: u64) -> bool {
    let iterations = timeout_ms / 10;
    for _ in 0..iterations {
        if source.is_ready() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    false
}

/// Helper function to assert a type implements `AudioSource` trait
fn assert_is_audio_source<T: AudioSource>(_: &T) {}

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
    let mut source = LocalAudioSource::new(&wav_path, 44100).expect("Failed to load test file");

    // Wait for background decoder to fill buffer (max 1 second)
    assert!(
        wait_for_ready(&source, 1000),
        "Source should become ready within 1 second"
    );

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

    let mut source = LocalAudioSource::new(&wav_path, 44100).unwrap();

    // Wait for background decoder to fill buffer
    assert!(
        wait_for_ready(&source, 1000),
        "Source should become ready within 1 second"
    );

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

    // Verify source reports finished.
    // is_eof is set by the decoder thread after the last samples are pushed to
    // the ring buffer, so there is a small window where all samples have been
    // consumed but is_eof has not yet been stored.  Retry briefly.
    for _ in 0..20 {
        if source.is_finished() {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(source.is_finished(), "Source should report finished");
}

#[test]
fn test_local_source_seeking() {
    let temp_dir = TempDir::new().unwrap();
    let wav_path = temp_dir.path().join("test.wav");
    generate_test_wav(&wav_path, 2.0, 440.0).unwrap(); // 2 second file

    let mut source = LocalAudioSource::new(&wav_path, 44100).unwrap();

    // Wait for background decoder to fill buffer
    assert!(
        wait_for_ready(&source, 1000),
        "Source should become ready within 1 second"
    );

    // Read some samples to advance position
    let mut buffer = vec![0.0f32; 8192];
    source.read_samples(&mut buffer).unwrap();
    let pos_before_seek = source.position();
    assert!(pos_before_seek > Duration::ZERO);

    // Seek to 1.0 seconds (async operation - need to wait for completion)
    source.seek(Duration::from_secs(1)).unwrap();

    // Wait for seek to complete (decoder thread processes asynchronously)
    assert!(
        wait_for_seek_complete(&source, 1.0, 0.1),
        "Position should be ~1.0s after seek, got {}",
        source.position().as_secs_f64()
    );

    // Verify we can continue reading from new position.
    // After a seek, two drain rounds may be needed before actual samples are
    // available: one for seek_pending=true (position updated before flag cleared)
    // and one for the generation-mismatch path.  Retry with a wait_for_ready
    // between attempts to ensure the buffer is refilled before each read.
    let mut samples_read = 0;
    for _ in 0..5 {
        samples_read = source.read_samples(&mut buffer).unwrap();
        if samples_read > 0 {
            break;
        }
        assert!(
            wait_for_ready(&source, 1000),
            "Buffer should refill within 1 second after seek"
        );
    }
    assert!(samples_read > 0, "Should be able to read after seeking");

    // Seek to beginning
    source.seek(Duration::ZERO).unwrap();
    assert!(
        wait_for_seek_complete(&source, 0.0, 0.1),
        "Should be at beginning after seeking to zero"
    );
}

#[test]
fn test_local_source_seek_beyond_duration_fails() {
    let temp_dir = TempDir::new().unwrap();
    let wav_path = temp_dir.path().join("test.wav");
    generate_test_wav(&wav_path, 1.0, 440.0).unwrap();

    let mut source = LocalAudioSource::new(&wav_path, 44100).unwrap();
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

    let mut source = LocalAudioSource::new(&wav_path, 44100).unwrap();

    // Wait for background decoder to fill buffer
    assert!(
        wait_for_ready(&source, 1000),
        "Source should become ready within 1 second"
    );

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

        let source = LocalAudioSource::new(&wav_path, 44100).unwrap();

        // Wait for background decoder to fill buffer
        assert!(
            wait_for_ready(&source, 1000),
            "Source should become ready within 1 second for {}s file",
            duration
        );

        let actual_duration = source.duration();

        assert!(
            (actual_duration.as_secs_f64() - duration).abs() < 0.1,
            "Duration mismatch for {}s file",
            duration
        );

        // Verify we can read samples
        let mut source = source; // Make mutable for read_samples
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

    let mut source = LocalAudioSource::new(&wav_path, 44100).unwrap();

    // Wait for background decoder to fill buffer (or reach EOF for short file)
    assert!(
        wait_for_ready(&source, 1000),
        "Source should become ready within 1 second"
    );

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

    let mut source = LocalAudioSource::new(&wav_path, 44100).unwrap();

    // Wait for background decoder to fill buffer
    assert!(
        wait_for_ready(&source, 1000),
        "Source should become ready within 1 second"
    );

    let mut buffer = vec![0.0f32; 8192];

    // Read to advance position
    source.read_samples(&mut buffer).unwrap();
    assert!(source.position() > Duration::ZERO);

    // Reset (seek to zero) - async operation
    source.reset().unwrap();

    // Wait for reset to complete (decoder thread processes asynchronously)
    assert!(
        wait_for_seek_complete(&source, 0.0, 0.1),
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
    assert_eq!(source.sample_rate(), Some(44100));
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
    let source = StreamingAudioSource::new(
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
    let local = LocalAudioSource::new(&wav_path, 44100).unwrap();
    let streaming = StreamingAudioSource::new(
        "http://localhost:8080/stream".to_string(),
        44100,
        2,
        Duration::from_secs(60),
    )
    .unwrap();

    // Both should implement AudioSource
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
        let mut source = LocalAudioSource::new(&wav_path, 44100).unwrap();
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

// ===== Position Accuracy Tests =====

#[test]
fn test_local_source_position_no_drift_long_playback() {
    // Test that position doesn't drift over extended playback
    // This catches issues with floating-point accumulation or sample counting errors
    let temp_dir = TempDir::new().unwrap();
    let wav_path = temp_dir.path().join("test.wav");
    generate_test_wav(&wav_path, 5.0, 440.0).unwrap(); // 5 second file

    let mut source = LocalAudioSource::new(&wav_path, 44100).unwrap();

    // Wait for background decoder to fill buffer
    assert!(
        wait_for_ready(&source, 1000),
        "Source should become ready within 1 second"
    );

    let mut buffer = vec![0.0f32; 8820]; // 0.1 seconds of stereo at 44.1kHz

    // Read for ~2 seconds worth of data (20 reads)
    let mut total_reads = 0;
    let mut last_position = 0.0;
    let mut cumulative_samples = 0;

    for i in 0..20 {
        let samples_read = source.read_samples(&mut buffer).unwrap();
        if samples_read == 0 {
            // try_lock returned 0 (lock contention) - wait and retry
            std::thread::sleep(Duration::from_millis(10));
            continue;
        }
        total_reads += 1;
        cumulative_samples += samples_read;

        let position = source.position().as_secs_f64();
        // Expected position based on actual samples read (stereo at 44.1kHz)
        let expected_position = cumulative_samples as f64 / (44100.0 * 2.0);

        // Position should be within 50ms of expected
        // Account for: encoder delay skipping, resampler artifacts, async operations
        let tolerance = 0.05;
        assert!(
            (position - expected_position).abs() < tolerance,
            "Position drift detected at read {}: expected ~{:.3}s, got {:.3}s (samples: {})",
            i + 1,
            expected_position,
            position,
            cumulative_samples
        );

        // Position should always increase monotonically
        assert!(
            position >= last_position,
            "Position went backwards: {} -> {}",
            last_position,
            position
        );
        last_position = position;
    }

    assert!(
        total_reads >= 15,
        "Should have completed at least 15 reads, got {}",
        total_reads
    );
}

#[test]
fn test_local_source_seek_position_accuracy() {
    // Test that position after seek matches the actual seek target
    let temp_dir = TempDir::new().unwrap();
    let wav_path = temp_dir.path().join("test.wav");
    generate_test_wav(&wav_path, 3.0, 440.0).unwrap(); // 3 second file

    let mut source = LocalAudioSource::new(&wav_path, 44100).unwrap();

    // Test various seek positions
    let seek_positions = [0.5, 1.0, 2.0, 0.0, 1.5];

    for &target_secs in &seek_positions {
        source.seek(Duration::from_secs_f64(target_secs)).unwrap();

        // Wait for async seek to complete
        assert!(
            wait_for_seek_complete(&source, target_secs, 0.1),
            "Seek to {}s failed: position is {}s",
            target_secs,
            source.position().as_secs_f64()
        );

        // Read some samples to verify playback continues correctly.
        // After a seek, the seek_pending=true path returns 0 while the decoder
        // drains stale data from the ring buffer.  Once seek_pending is cleared,
        // fresh samples may not yet be available if the decoder hasn't had time
        // to fill the buffer; retry with a short sleep to allow it to catch up.
        let mut buffer = vec![0.0f32; 4410]; // 0.05 seconds
        let mut got_samples = false;
        for _ in 0..20usize {
            let n = source.read_samples(&mut buffer).unwrap();
            if n == buffer.len() {
                got_samples = true;
                break;
            }
            // Either a drain (0) or partial read; wait for decoder to fill.
            std::thread::sleep(Duration::from_millis(50));
        }
        assert!(
            got_samples,
            "Should read a full buffer after seek to {}s (position: {:.3}s)",
            target_secs,
            source.position().as_secs_f64()
        );

        // Position should have advanced by ~0.05 seconds
        let position_after_read = source.position().as_secs_f64();
        let expected_after_read = target_secs + 0.05;
        assert!(
            (position_after_read - expected_after_read).abs() < 0.02,
            "Position after seek+read should be ~{:.2}s, got {:.3}s",
            expected_after_read,
            position_after_read
        );
    }
}

#[test]
fn test_local_source_position_stable_during_pause_simulation() {
    // Simulate pause by not reading samples, verify position doesn't drift
    let temp_dir = TempDir::new().unwrap();
    let wav_path = temp_dir.path().join("test.wav");
    generate_test_wav(&wav_path, 2.0, 440.0).unwrap();

    let mut source = LocalAudioSource::new(&wav_path, 44100).unwrap();
    let mut buffer = vec![0.0f32; 8820]; // 0.1 seconds

    // Read some samples to advance position
    source.read_samples(&mut buffer).unwrap();
    let position_before_pause = source.position();

    // Simulate pause by waiting without reading
    std::thread::sleep(Duration::from_millis(200));

    // Position should not have changed
    let position_after_pause = source.position();
    assert_eq!(
        position_before_pause, position_after_pause,
        "Position should not drift during pause: before={:?}, after={:?}",
        position_before_pause, position_after_pause
    );

    // Resume reading
    source.read_samples(&mut buffer).unwrap();
    let position_after_resume = source.position();

    // Position should have advanced by approximately the read duration
    assert!(
        position_after_resume > position_after_pause,
        "Position should advance after resume"
    );
}

#[test]
fn test_local_source_position_accuracy_with_resampling() {
    // Test position accuracy when resampling is involved
    let temp_dir = TempDir::new().unwrap();
    let wav_path = temp_dir.path().join("test.wav");
    generate_test_wav(&wav_path, 2.0, 440.0).unwrap(); // 44.1kHz source

    // Load with 48kHz target (requires resampling)
    let mut source = LocalAudioSource::new(&wav_path, 48000).unwrap();
    let buffer_size = 9600; // 0.1 seconds of stereo at 48kHz
    let mut buffer = vec![0.0f32; buffer_size];
    let expected_position_per_read = 0.1; // seconds

    // Read 10 buffers (1 second of audio)
    for i in 0..10 {
        let samples_read = source.read_samples(&mut buffer).unwrap();
        if samples_read == 0 {
            break;
        }

        let position = source.position().as_secs_f64();
        let expected = (i + 1) as f64 * expected_position_per_read;

        // Position should be accurate even with resampling
        // Allow slightly more tolerance for resampling
        let tolerance = if i < 2 { 0.05 } else { 0.02 };
        assert!(
            (position - expected).abs() < tolerance,
            "Position inaccurate with resampling at read {}: expected ~{:.3}s, got {:.3}s",
            i + 1,
            expected,
            position
        );
    }
}

#[test]
fn test_local_source_nonexistent_file_fails() {
    let result = LocalAudioSource::new("/nonexistent/path/file.wav", 44100);
    assert!(result.is_err(), "Should fail to load nonexistent file");
}

#[test]
fn test_local_source_invalid_file_fails() {
    let temp_dir = TempDir::new().unwrap();
    let invalid_path = temp_dir.path().join("invalid.wav");

    // Write garbage data
    let mut file = File::create(&invalid_path).unwrap();
    file.write_all(b"This is not a valid audio file").unwrap();

    let result = LocalAudioSource::new(&invalid_path, 44100);
    assert!(result.is_err(), "Should fail to load invalid audio file");
}
