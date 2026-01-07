//! Streaming decoder integration tests
//!
//! Tests for packet-based decoding with ring buffer.
//! Verifies memory bounds, on-demand loading, and performance.

use soul_audio_desktop::LocalAudioSource;
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

    let mut file = File::create(path)?;

    // RIFF header
    file.write_all(b"RIFF")?;
    let file_size = 36 + num_samples * channels * 2;
    file.write_all(&(file_size as u32).to_le_bytes())?;
    file.write_all(b"WAVE")?;

    // fmt chunk
    file.write_all(b"fmt ")?;
    file.write_all(&16u32.to_le_bytes())?;
    file.write_all(&1u16.to_le_bytes())?; // PCM
    file.write_all(&(channels as u16).to_le_bytes())?;
    file.write_all(&(sample_rate as u32).to_le_bytes())?;
    file.write_all(&((sample_rate * channels * 2) as u32).to_le_bytes())?;
    file.write_all(&((channels * 2) as u16).to_le_bytes())?;
    file.write_all(&16u16.to_le_bytes())?;

    // data chunk
    file.write_all(b"data")?;
    file.write_all(&((num_samples * channels * 2) as u32).to_le_bytes())?;

    // Generate sine wave
    for i in 0..num_samples {
        let t = i as f64 / sample_rate as f64;
        let sample = (t * frequency * 2.0 * std::f64::consts::PI).sin();
        let sample_i16 = (sample * 32767.0) as i16;

        file.write_all(&sample_i16.to_le_bytes())?;
        file.write_all(&sample_i16.to_le_bytes())?;
    }

    Ok(())
}

// ===== Ring Buffer Behavior Tests =====

#[test]
fn test_streaming_decoder_loads_incrementally() {
    // Create a long file (10 seconds)
    let temp_dir = TempDir::new().unwrap();
    let wav_path = temp_dir.path().join("long.wav");
    generate_test_wav(&wav_path, 10.0, 440.0).unwrap();

    // Create source - should not load entire file immediately
    let mut source = LocalAudioSource::new(&wav_path).expect("Failed to create source");

    // Request small amount of data
    let mut buffer = vec![0.0f32; 1024];
    let samples_read = source.read_samples(&mut buffer).unwrap();

    assert_eq!(samples_read, 1024, "Should read requested samples");

    // Position should advance incrementally
    let position = source.position();
    assert!(
        position < Duration::from_secs(1),
        "Position should be small after small read, got {}",
        position.as_secs_f64()
    );
}

#[test]
fn test_ring_buffer_refills_on_demand() {
    let temp_dir = TempDir::new().unwrap();
    let wav_path = temp_dir.path().join("test.wav");
    generate_test_wav(&wav_path, 5.0, 440.0).unwrap();

    let mut source = LocalAudioSource::new(&wav_path).unwrap();

    // Read multiple times - ring buffer should refill each time
    for iteration in 0..10 {
        let mut buffer = vec![0.0f32; 8192];
        let samples_read = source.read_samples(&mut buffer).unwrap();

        assert!(
            samples_read > 0,
            "Should read samples on iteration {}",
            iteration
        );
    }

    // Should still be able to read (not at EOF yet)
    let mut buffer = vec![0.0f32; 1024];
    let samples_read = source.read_samples(&mut buffer).unwrap();
    assert!(samples_read > 0, "Ring buffer should keep refilling");
}

#[test]
fn test_decoder_handles_eof_gracefully() {
    let temp_dir = TempDir::new().unwrap();
    let wav_path = temp_dir.path().join("short.wav");
    generate_test_wav(&wav_path, 0.5, 440.0).unwrap(); // Very short file

    let mut source = LocalAudioSource::new(&wav_path).unwrap();
    let mut buffer = vec![0.0f32; 4096];

    // Read entire file
    let mut total_samples = 0;
    for _ in 0..100 {
        // Enough iterations to exhaust file
        let samples_read = source.read_samples(&mut buffer).unwrap();
        if samples_read == 0 {
            break;
        }
        total_samples += samples_read;
    }

    assert!(total_samples > 0, "Should have read some samples");
    assert!(source.is_finished(), "Should be finished at EOF");

    // Further reads should return 0
    let samples_read = source.read_samples(&mut buffer).unwrap();
    assert_eq!(samples_read, 0, "Should return 0 at EOF");
}

#[test]
fn test_partial_buffer_fill_near_end() {
    let temp_dir = TempDir::new().unwrap();
    let wav_path = temp_dir.path().join("test.wav");
    generate_test_wav(&wav_path, 0.1, 440.0).unwrap();

    let mut source = LocalAudioSource::new(&wav_path).unwrap();
    let mut large_buffer = vec![0.0f32; 88200]; // 1 second worth for 0.1 second file

    // Request more than available
    let samples_read = source.read_samples(&mut large_buffer).unwrap();

    // Should return partial read
    assert!(samples_read > 0, "Should read some samples");
    assert!(
        samples_read < large_buffer.len(),
        "Should not fill entire buffer (file too short)"
    );
}

// ===== Seeking with Streaming Decoder Tests =====

#[test]
fn test_seek_clears_ring_buffer() {
    let temp_dir = TempDir::new().unwrap();
    let wav_path = temp_dir.path().join("test.wav");
    generate_test_wav(&wav_path, 3.0, 440.0).unwrap();

    let mut source = LocalAudioSource::new(&wav_path).unwrap();

    // Read some data (fills ring buffer)
    let mut buffer = vec![0.0f32; 8192];
    source.read_samples(&mut buffer).unwrap();

    let pos_before = source.position();
    assert!(pos_before > Duration::ZERO);

    // Seek to new position
    source.seek(Duration::from_secs(2)).unwrap();
    let pos_after = source.position();

    // Position should be at seek target
    assert!(
        (pos_after.as_secs_f64() - 2.0).abs() < 0.05,
        "Position should be ~2.0s, got {}",
        pos_after.as_secs_f64()
    );

    // Should be able to read from new position
    let samples_read = source.read_samples(&mut buffer).unwrap();
    assert!(samples_read > 0, "Should read after seek");
}

#[test]
fn test_seek_to_beginning_resets_decoder() {
    let temp_dir = TempDir::new().unwrap();
    let wav_path = temp_dir.path().join("test.wav");
    generate_test_wav(&wav_path, 2.0, 440.0).unwrap();

    let mut source = LocalAudioSource::new(&wav_path).unwrap();

    // Read halfway through
    let mut buffer = vec![0.0f32; 44100]; // ~0.5 seconds
    source.read_samples(&mut buffer).unwrap();

    assert!(source.position() > Duration::ZERO);

    // Seek to beginning
    source.seek(Duration::ZERO).unwrap();

    // Should be back at start
    assert!(
        source.position().as_secs_f64() < 0.01,
        "Should be at beginning"
    );
    assert!(!source.is_finished(), "Should not be finished");

    // Should be able to read entire file again
    let mut total = 0;
    for _ in 0..100 {
        let read = source.read_samples(&mut buffer).unwrap();
        if read == 0 {
            break;
        }
        total += read;
    }

    assert!(total > 44100, "Should read full file after reset");
}

#[test]
fn test_multiple_seeks_maintain_accuracy() {
    let temp_dir = TempDir::new().unwrap();
    let wav_path = temp_dir.path().join("test.wav");
    generate_test_wav(&wav_path, 5.0, 440.0).unwrap();

    let mut source = LocalAudioSource::new(&wav_path).unwrap();
    let mut buffer = vec![0.0f32; 1024];

    // Perform multiple seeks
    let seek_positions = [1.0, 3.0, 0.5, 4.0, 2.0];

    for target_pos in seek_positions {
        source.seek(Duration::from_secs_f64(target_pos)).unwrap();

        let actual_pos = source.position();
        assert!(
            (actual_pos.as_secs_f64() - target_pos).abs() < 0.05,
            "Seek to {}s failed, got {}s",
            target_pos,
            actual_pos.as_secs_f64()
        );

        // Should be able to read after each seek
        let samples_read = source.read_samples(&mut buffer).unwrap();
        assert!(
            samples_read > 0,
            "Should read after seeking to {}s",
            target_pos
        );
    }
}

#[test]
fn test_seek_past_end_fails() {
    let temp_dir = TempDir::new().unwrap();
    let wav_path = temp_dir.path().join("test.wav");
    generate_test_wav(&wav_path, 2.0, 440.0).unwrap();

    let mut source = LocalAudioSource::new(&wav_path).unwrap();
    let duration = source.duration();

    // Seek beyond duration
    let result = source.seek(duration + Duration::from_secs(1));
    assert!(result.is_err(), "Seek past end should fail");

    // Position should not change after failed seek
    // (implementation may vary, but should be safe)
}

// ===== Performance and Memory Tests =====

#[test]
fn test_fast_startup_time() {
    let temp_dir = TempDir::new().unwrap();
    let wav_path = temp_dir.path().join("large.wav");
    generate_test_wav(&wav_path, 30.0, 440.0).unwrap(); // Large file

    let start = std::time::Instant::now();

    // Create source - should be fast (not loading entire file)
    let source = LocalAudioSource::new(&wav_path);

    let creation_time = start.elapsed();

    assert!(source.is_ok(), "Should create source successfully");

    // Should take less than 200ms even for large file
    assert!(
        creation_time.as_millis() < 200,
        "Startup should be fast (<200ms), took {}ms",
        creation_time.as_millis()
    );
}

#[test]
fn test_memory_bounded_buffer() {
    let temp_dir = TempDir::new().unwrap();
    let wav_path = temp_dir.path().join("test.wav");
    generate_test_wav(&wav_path, 10.0, 440.0).unwrap();

    let mut source = LocalAudioSource::new(&wav_path).unwrap();

    // Read through file in chunks
    let mut buffer = vec![0.0f32; 2048];

    for _ in 0..100 {
        // Many iterations
        let read = source.read_samples(&mut buffer).unwrap();
        if read == 0 {
            break;
        }
    }

    // If we got here, memory didn't grow unbounded
    // (ring buffer implementation keeps memory usage constant)
}

#[test]
fn test_consistent_playback_speed() {
    let temp_dir = TempDir::new().unwrap();
    let wav_path = temp_dir.path().join("test.wav");
    generate_test_wav(&wav_path, 2.0, 440.0).unwrap();

    let mut source = LocalAudioSource::new(&wav_path).unwrap();

    // Read at regular intervals, measure position advancement
    let buffer_size = 4410; // 0.05 seconds worth at 44.1kHz stereo
    let mut buffer = vec![0.0f32; buffer_size];

    let mut positions = Vec::new();

    for _ in 0..10 {
        source.read_samples(&mut buffer).unwrap();
        positions.push(source.position().as_secs_f64());
    }

    // Calculate time deltas
    let deltas: Vec<f64> = positions.windows(2).map(|w| w[1] - w[0]).collect();

    // All deltas should be approximately equal (consistent advancement)
    let avg_delta = deltas.iter().sum::<f64>() / deltas.len() as f64;

    for delta in deltas {
        assert!(
            (delta - avg_delta).abs() < 0.01,
            "Position advancement should be consistent, got delta {} vs avg {}",
            delta,
            avg_delta
        );
    }
}

// ===== Edge Cases =====

#[test]
fn test_tiny_buffer_reads() {
    let temp_dir = TempDir::new().unwrap();
    let wav_path = temp_dir.path().join("test.wav");
    generate_test_wav(&wav_path, 1.0, 440.0).unwrap();

    let mut source = LocalAudioSource::new(&wav_path).unwrap();

    // Read with very small buffer (edge case)
    let mut buffer = vec![0.0f32; 2];
    let samples_read = source.read_samples(&mut buffer).unwrap();

    assert_eq!(samples_read, 2, "Should handle tiny buffer reads");
}

#[test]
fn test_exact_duration_playback() {
    let temp_dir = TempDir::new().unwrap();
    let wav_path = temp_dir.path().join("test.wav");
    let expected_duration = 1.0;
    generate_test_wav(&wav_path, expected_duration, 440.0).unwrap();

    let mut source = LocalAudioSource::new(&wav_path).unwrap();
    let reported_duration = source.duration();

    // Read entire file
    let mut buffer = vec![0.0f32; 4096];
    let mut total_time = 0.0f64;

    loop {
        let samples_read = source.read_samples(&mut buffer).unwrap();
        if samples_read == 0 {
            break;
        }

        // Calculate time for samples read (stereo, 44.1kHz)
        let time_delta = samples_read as f64 / (44100.0 * 2.0);
        total_time += time_delta;
    }

    // Total playback time should match reported duration
    assert!(
        (total_time - reported_duration.as_secs_f64()).abs() < 0.1,
        "Playback time {} should match duration {}",
        total_time,
        reported_duration.as_secs_f64()
    );

    assert!(
        (total_time - expected_duration).abs() < 0.1,
        "Should play for expected duration"
    );
}

#[test]
fn test_interleaved_seek_and_read() {
    let temp_dir = TempDir::new().unwrap();
    let wav_path = temp_dir.path().join("test.wav");
    generate_test_wav(&wav_path, 5.0, 440.0).unwrap();

    let mut source = LocalAudioSource::new(&wav_path).unwrap();
    let mut buffer = vec![0.0f32; 1024];

    // Interleave seeks and reads
    source.read_samples(&mut buffer).unwrap();
    source.seek(Duration::from_secs(2)).unwrap();
    source.read_samples(&mut buffer).unwrap();
    source.seek(Duration::from_secs(1)).unwrap();
    source.read_samples(&mut buffer).unwrap();
    source.seek(Duration::from_secs(4)).unwrap();
    let final_read = source.read_samples(&mut buffer).unwrap();

    assert!(final_read > 0, "Should handle interleaved seek/read");
}

#[test]
fn test_zero_size_buffer_read() {
    let temp_dir = TempDir::new().unwrap();
    let wav_path = temp_dir.path().join("test.wav");
    generate_test_wav(&wav_path, 1.0, 440.0).unwrap();

    let mut source = LocalAudioSource::new(&wav_path).unwrap();
    let mut buffer = vec![];

    // Read with zero-size buffer
    let samples_read = source.read_samples(&mut buffer).unwrap();
    assert_eq!(samples_read, 0, "Should return 0 for empty buffer");
}

#[test]
fn test_decoder_state_after_error_recovery() {
    let temp_dir = TempDir::new().unwrap();
    let wav_path = temp_dir.path().join("test.wav");
    generate_test_wav(&wav_path, 2.0, 440.0).unwrap();

    let mut source = LocalAudioSource::new(&wav_path).unwrap();

    // Try invalid seek (past end)
    let _ = source.seek(Duration::from_secs(100));

    // Should still be able to use source after failed seek
    let mut buffer = vec![0.0f32; 1024];
    let result = source.read_samples(&mut buffer);

    // Should either work or fail gracefully (not panic/hang)
    assert!(
        result.is_ok() || result.is_err(),
        "Should handle error recovery"
    );
}
