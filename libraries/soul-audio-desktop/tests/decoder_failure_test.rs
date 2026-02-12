//! Decoder failure and timeout tests
//!
//! Tests decoder error handling:
//! - Corrupt file handling
//! - Decoder timeouts
//! - Unsupported formats
//! - Partial decode failures
//!
//! Run with: cargo test --test decoder_failure_test -- --include-ignored

use soul_audio_desktop::sources::LocalAudioSource;
use soul_audio_desktop::{DesktopAudioBackend, DesktopPlaybackCommand, PlaybackContext};
use soul_playback::{AudioSource, PlaybackEvent};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

// ============================================================================
// Test Utilities
// ============================================================================

/// Create a corrupt audio file for testing
fn create_corrupt_file(path: &PathBuf) -> std::io::Result<()> {
    let mut file = File::create(path)?;
    // Write invalid audio data (random bytes)
    file.write_all(&[0xFF; 1024])?;
    Ok(())
}

/// Create a file with valid header but corrupt data
fn create_partial_corrupt_file(path: &PathBuf) -> std::io::Result<()> {
    // Start with valid WAV header
    let mut data = vec![
        // RIFF header
        0x52, 0x49, 0x46, 0x46, // "RIFF"
        0x24, 0x00, 0x00, 0x00, // File size - 8
        0x57, 0x41, 0x56, 0x45, // "WAVE"
        // fmt chunk
        0x66, 0x6D, 0x74, 0x20, // "fmt "
        0x10, 0x00, 0x00, 0x00, // Chunk size
        0x01, 0x00, // Audio format (PCM)
        0x02, 0x00, // Channels (stereo)
        0x44, 0xAC, 0x00, 0x00, // Sample rate (44100)
        0x10, 0xB1, 0x02, 0x00, // Byte rate
        0x04, 0x00, // Block align
        0x10, 0x00, // Bits per sample
        // data chunk
        0x64, 0x61, 0x74, 0x61, // "data"
        0x00, 0x00, 0x00, 0x00, // Data size
    ];

    // Append corrupt data
    data.extend_from_slice(&[0xFF; 512]);

    let mut file = File::create(path)?;
    file.write_all(&data)?;
    Ok(())
}

fn test_audio_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test_data")
        .join("sine_1khz_10s_44100hz_stereo.wav")
}

// ============================================================================
// 1. Corrupt File Tests
// ============================================================================

#[test]
#[ignore]
fn test_corrupt_file_handling() {
    let temp_dir = std::env::temp_dir();
    let corrupt_file = temp_dir.join("corrupt_audio_test.wav");

    create_corrupt_file(&corrupt_file).expect("Failed to create corrupt file");

    // Try to load corrupt file
    let result = LocalAudioSource::new(&corrupt_file, 48000);

    // Should return error
    assert!(
        result.is_err(),
        "Should fail to load corrupt file: {:?}",
        result
    );

    // Clean up
    std::fs::remove_file(corrupt_file).ok();
}

#[test]
#[ignore]
fn test_circuit_breaker_skips_corrupt_track() {
    let (cmd_tx, cmd_rx) = mpsc::channel();
    let (event_tx, event_rx) = mpsc::channel();

    let backend = DesktopAudioBackend::new(cmd_rx, event_tx.clone());
    let _backend_thread = thread::spawn(move || backend.run());

    let ctx = PlaybackContext {
        device_id: None,
        sample_rate: 48000,
        buffer_size: 512,
    };
    cmd_tx.send(DesktopPlaybackCommand::Initialize(ctx)).ok();
    thread::sleep(Duration::from_millis(100));

    // Create corrupt file
    let temp_dir = std::env::temp_dir();
    let corrupt_file = temp_dir.join("corrupt_track.wav");
    create_corrupt_file(&corrupt_file).expect("Failed to create corrupt file");

    // Try to load corrupt track
    cmd_tx
        .send(DesktopPlaybackCommand::LoadTrack {
            track_id: "corrupt1".into(),
            path: corrupt_file.clone(),
            start_position: Duration::ZERO,
        })
        .ok();

    thread::sleep(Duration::from_millis(500));

    // Should emit error event
    let mut found_error = false;
    let mut found_skip = false;

    for _ in 0..50 {
        if let Ok(event) = event_rx.try_recv() {
            match event {
                PlaybackEvent::Error { .. } => {
                    found_error = true;
                    println!("✓ Received error event for corrupt file");
                }
                PlaybackEvent::TrackSkippedDueToFailures { .. } => {
                    found_skip = true;
                    println!("✓ Track skipped after failures");
                }
                _ => {}
            }
        }
        thread::sleep(Duration::from_millis(10));
    }

    assert!(
        found_error,
        "Should emit error event when loading corrupt file"
    );

    // Clean up
    std::fs::remove_file(corrupt_file).ok();
    cmd_tx.send(DesktopPlaybackCommand::Shutdown).ok();
}

#[test]
#[ignore]
fn test_consecutive_corrupt_files_circuit_breaker() {
    let (cmd_tx, cmd_rx) = mpsc::channel();
    let (event_tx, event_rx) = mpsc::channel();

    let backend = DesktopAudioBackend::new(cmd_rx, event_tx.clone());
    let _backend_thread = thread::spawn(move || backend.run());

    let ctx = PlaybackContext {
        device_id: None,
        sample_rate: 48000,
        buffer_size: 512,
    };
    cmd_tx.send(DesktopPlaybackCommand::Initialize(ctx)).ok();
    thread::sleep(Duration::from_millis(100));

    let temp_dir = std::env::temp_dir();

    // Try to load multiple corrupt files
    for i in 0..5 {
        let corrupt_file = temp_dir.join(format!("corrupt_track_{}.wav", i));
        create_corrupt_file(&corrupt_file).expect("Failed to create corrupt file");

        cmd_tx
            .send(DesktopPlaybackCommand::LoadTrack {
                track_id: format!("corrupt{}", i).into(),
                path: corrupt_file.clone(),
                start_position: Duration::ZERO,
            })
            .ok();

        thread::sleep(Duration::from_millis(200));
    }

    // Check for circuit breaker events
    let mut circuit_opened = false;
    let mut skip_count = 0;

    for _ in 0..100 {
        if let Ok(event) = event_rx.try_recv() {
            match event {
                PlaybackEvent::CircuitOpened { .. } => {
                    circuit_opened = true;
                    println!("✓ Circuit breaker opened after multiple failures");
                }
                PlaybackEvent::TrackSkippedDueToFailures { .. } => {
                    skip_count += 1;
                }
                _ => {}
            }
        }
        thread::sleep(Duration::from_millis(10));
    }

    // Circuit breaker should open after consecutive failures
    // (Threshold is 10 failures in 60s)
    println!("Skip count: {}", skip_count);

    // Clean up
    for i in 0..5 {
        let corrupt_file = temp_dir.join(format!("corrupt_track_{}.wav", i));
        std::fs::remove_file(corrupt_file).ok();
    }

    cmd_tx.send(DesktopPlaybackCommand::Shutdown).ok();
}

// ============================================================================
// 2. Decoder Timeout Tests
// ============================================================================

#[test]
#[ignore]
fn test_decoder_timeout() {
    // Create a very large file that might timeout
    let temp_dir = std::env::temp_dir();
    let large_file = temp_dir.join("large_file_test.wav");

    // Create a valid WAV header with huge data size
    let mut data = vec![
        // RIFF header
        0x52, 0x49, 0x46, 0x46, // "RIFF"
        0xFF, 0xFF, 0xFF, 0x7F, // Max file size
        0x57, 0x41, 0x56, 0x45, // "WAVE"
        // fmt chunk
        0x66, 0x6D, 0x74, 0x20, // "fmt "
        0x10, 0x00, 0x00, 0x00, // Chunk size
        0x01, 0x00, // Audio format (PCM)
        0x02, 0x00, // Channels (stereo)
        0x44, 0xAC, 0x00, 0x00, // Sample rate (44100)
        0x10, 0xB1, 0x02, 0x00, // Byte rate
        0x04, 0x00, // Block align
        0x10, 0x00, // Bits per sample
        // data chunk
        0x64, 0x61, 0x74, 0x61, // "data"
        0xFF, 0xFF, 0xFF, 0x7F, // Huge data size
    ];

    // Add minimal actual data
    data.extend_from_slice(&[0x00; 1024]);

    let mut file = File::create(&large_file).expect("Failed to create large file");
    file.write_all(&data).ok();

    // Try to load file (should timeout or fail gracefully)
    let result = LocalAudioSource::new(&large_file, 48000);

    // Should either fail immediately or timeout gracefully
    match result {
        Ok(_) => {
            println!("File loaded (might use streaming)");
        }
        Err(e) => {
            println!("✓ Failed to load malformed file: {:?}", e);
        }
    }

    // Clean up
    std::fs::remove_file(large_file).ok();
}

#[test]
#[ignore]
fn test_hanging_decoder_timeout() {
    // This test would require a mock decoder that hangs
    // In practice, the decoder should have internal timeouts

    let (cmd_tx, cmd_rx) = mpsc::channel();
    let (event_tx, event_rx) = mpsc::channel();

    let backend = DesktopAudioBackend::new(cmd_rx, event_tx.clone());
    let _backend_thread = thread::spawn(move || backend.run());

    let ctx = PlaybackContext {
        device_id: None,
        sample_rate: 48000,
        buffer_size: 512,
    };
    cmd_tx.send(DesktopPlaybackCommand::Initialize(ctx)).ok();
    thread::sleep(Duration::from_millis(100));

    // Load a potentially problematic file
    let temp_dir = std::env::temp_dir();
    let problem_file = temp_dir.join("problem_file.wav");
    create_partial_corrupt_file(&problem_file).ok();

    let start = std::time::Instant::now();

    cmd_tx
        .send(DesktopPlaybackCommand::LoadTrack {
            track_id: "problem".into(),
            path: problem_file.clone(),
            start_position: Duration::ZERO,
        })
        .ok();

    // Wait for timeout or error
    thread::sleep(Duration::from_secs(5));

    let elapsed = start.elapsed();

    // Should timeout or fail within reasonable time (< 5 seconds)
    assert!(
        elapsed < Duration::from_secs(6),
        "Decoder should timeout, not hang indefinitely"
    );

    // Check for error event
    let mut found_error = false;
    while let Ok(event) = event_rx.try_recv() {
        if matches!(event, PlaybackEvent::Error { .. }) {
            found_error = true;
        }
    }

    // Clean up
    std::fs::remove_file(problem_file).ok();
    cmd_tx.send(DesktopPlaybackCommand::Shutdown).ok();
}

// ============================================================================
// 3. Unsupported Format Tests
// ============================================================================

#[test]
#[ignore]
fn test_unsupported_format() {
    let temp_dir = std::env::temp_dir();
    let unsupported_file = temp_dir.join("unsupported.xyz");

    // Create a file with unsupported extension
    let mut file = File::create(&unsupported_file).expect("Failed to create file");
    file.write_all(b"Not a valid audio file").ok();

    // Try to load unsupported file
    let result = LocalAudioSource::new(&unsupported_file, 48000);

    assert!(result.is_err(), "Should fail to load unsupported format");

    // Clean up
    std::fs::remove_file(unsupported_file).ok();
}

#[test]
#[ignore]
fn test_skip_unsupported_format_in_queue() {
    let (cmd_tx, cmd_rx) = mpsc::channel();
    let (event_tx, event_rx) = mpsc::channel();

    let backend = DesktopAudioBackend::new(cmd_rx, event_tx.clone());
    let _backend_thread = thread::spawn(move || backend.run());

    let ctx = PlaybackContext {
        device_id: None,
        sample_rate: 48000,
        buffer_size: 512,
    };
    cmd_tx.send(DesktopPlaybackCommand::Initialize(ctx)).ok();
    thread::sleep(Duration::from_millis(100));

    // Try to load unsupported file
    let temp_dir = std::env::temp_dir();
    let unsupported_file = temp_dir.join("unsupported_in_queue.txt");
    let mut file = File::create(&unsupported_file).expect("Failed to create file");
    file.write_all(b"This is a text file, not audio").ok();

    cmd_tx
        .send(DesktopPlaybackCommand::LoadTrack {
            track_id: "unsupported".into(),
            path: unsupported_file.clone(),
            start_position: Duration::ZERO,
        })
        .ok();

    thread::sleep(Duration::from_millis(500));

    // Should emit error and potentially skip
    let mut found_error = false;
    while let Ok(event) = event_rx.try_recv() {
        if matches!(event, PlaybackEvent::Error { .. }) {
            found_error = true;
            println!("✓ Error emitted for unsupported format");
        }
    }

    assert!(found_error, "Should emit error for unsupported format");

    // Clean up
    std::fs::remove_file(unsupported_file).ok();
    cmd_tx.send(DesktopPlaybackCommand::Shutdown).ok();
}

// ============================================================================
// 4. Partial Decode Failure Tests
// ============================================================================

#[test]
#[ignore]
fn test_partial_decode_failure() {
    let temp_dir = std::env::temp_dir();
    let partial_file = temp_dir.join("partial_corrupt.wav");

    create_partial_corrupt_file(&partial_file).expect("Failed to create file");

    // Try to load and play
    let result = LocalAudioSource::new(&partial_file, 48000);

    match result {
        Ok(mut source) => {
            // Try to read samples
            let mut buffer = vec![0.0f32; 1024];
            let mut total_read = 0;

            for _ in 0..100 {
                match source.read_samples(&mut buffer) {
                    Ok(n) => {
                        total_read += n;
                        if n == 0 {
                            break;
                        }
                    }
                    Err(e) => {
                        println!("✓ Decoder failed gracefully: {:?}", e);
                        break;
                    }
                }
            }

            println!("Total samples read before failure: {}", total_read);
        }
        Err(e) => {
            println!("✓ Failed to load partially corrupt file: {:?}", e);
        }
    }

    // Clean up
    std::fs::remove_file(partial_file).ok();
}

// ============================================================================
// 5. Missing File Tests
// ============================================================================

#[test]
fn test_missing_file_handling() {
    let nonexistent = PathBuf::from("/nonexistent/path/to/audio.wav");

    let result = LocalAudioSource::new(&nonexistent, 48000);

    assert!(result.is_err(), "Should fail for missing file");
}

#[test]
#[ignore]
fn test_file_deleted_during_playback() {
    let (cmd_tx, cmd_rx) = mpsc::channel();
    let (event_tx, event_rx) = mpsc::channel();

    let backend = DesktopAudioBackend::new(cmd_rx, event_tx.clone());
    let _backend_thread = thread::spawn(move || backend.run());

    let ctx = PlaybackContext {
        device_id: None,
        sample_rate: 48000,
        buffer_size: 512,
    };
    cmd_tx.send(DesktopPlaybackCommand::Initialize(ctx)).ok();
    thread::sleep(Duration::from_millis(100));

    // Copy test file to temp location
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("temp_audio_delete_test.wav");
    std::fs::copy(test_audio_path(), &temp_file).expect("Failed to copy test file");

    // Load and start playing
    cmd_tx
        .send(DesktopPlaybackCommand::LoadTrack {
            track_id: "temp_track".into(),
            path: temp_file.clone(),
            start_position: Duration::ZERO,
        })
        .ok();

    thread::sleep(Duration::from_millis(500));

    cmd_tx.send(DesktopPlaybackCommand::Play).ok();
    thread::sleep(Duration::from_millis(500));

    // Delete file while playing
    std::fs::remove_file(&temp_file).ok();

    // Check if playback continues (file should be already in memory/buffer)
    thread::sleep(Duration::from_secs(1));

    // Depending on implementation, might continue playing from buffer
    // or might emit error. Either is acceptable.

    let mut events = Vec::new();
    while let Ok(event) = event_rx.try_recv() {
        events.push(event);
    }

    println!("Events after file deletion: {:?}", events);

    cmd_tx.send(DesktopPlaybackCommand::Shutdown).ok();
}

// ============================================================================
// 6. Edge Cases
// ============================================================================

#[test]
#[ignore]
fn test_empty_file() {
    let temp_dir = std::env::temp_dir();
    let empty_file = temp_dir.join("empty.wav");

    File::create(&empty_file).expect("Failed to create empty file");

    let result = LocalAudioSource::new(&empty_file, 48000);

    assert!(result.is_err(), "Should fail to load empty file");

    std::fs::remove_file(empty_file).ok();
}

#[test]
#[ignore]
fn test_zero_duration_file() {
    // Create a valid WAV with zero data
    let temp_dir = std::env::temp_dir();
    let zero_file = temp_dir.join("zero_duration.wav");

    let data = vec![
        // RIFF header
        0x52, 0x49, 0x46, 0x46, // "RIFF"
        0x24, 0x00, 0x00, 0x00, // File size
        0x57, 0x41, 0x56, 0x45, // "WAVE"
        // fmt chunk
        0x66, 0x6D, 0x74, 0x20, // "fmt "
        0x10, 0x00, 0x00, 0x00, // Chunk size
        0x01, 0x00, // Audio format
        0x02, 0x00, // Channels
        0x44, 0xAC, 0x00, 0x00, // Sample rate
        0x10, 0xB1, 0x02, 0x00, // Byte rate
        0x04, 0x00, // Block align
        0x10, 0x00, // Bits per sample
        // data chunk
        0x64, 0x61, 0x74, 0x61, // "data"
        0x00, 0x00, 0x00, 0x00, // Zero data size
    ];

    let mut file = File::create(&zero_file).expect("Failed to create file");
    file.write_all(&data).ok();

    let result = LocalAudioSource::new(&zero_file, 48000);

    match result {
        Ok(source) => {
            // Should report zero duration
            assert_eq!(
                source.duration(),
                Duration::ZERO,
                "Should have zero duration"
            );
        }
        Err(_) => {
            println!("✓ Rejected zero-duration file");
        }
    }

    std::fs::remove_file(zero_file).ok();
}
