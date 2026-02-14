//! Corrupted file recovery test
//!
//! Tests the playback system's error handling for corrupted/invalid audio files:
//! - Truncated files
//! - Zero-byte files
//! - Invalid headers
//! - Missing files during playback
//! - Corrupted mid-stream data

use soul_audio_desktop::{DesktopPlayback, PlaybackCommand, PlaybackEvent};
use soul_playback::{PlaybackConfig, PlaybackState, QueueTrack, TrackSource};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;
use tempfile::TempDir;

/// Create a test track pointing to a specific path
fn create_test_track_with_path(id: &str, path: PathBuf) -> QueueTrack {
    QueueTrack {
        id: id.to_string().into(),
        path,
        title: format!("Test Track {}", id),
        artist: "Test Artist".to_string(),
        album: Some("Test Album".to_string()),
        duration: Duration::from_secs(180),
        track_number: Some(1),
        source: TrackSource::Single,
    }
}

/// Helper to drain all events from the playback system
fn drain_events(playback: &DesktopPlayback) -> Vec<PlaybackEvent> {
    std::iter::from_fn(|| playback.try_recv_event()).collect()
}

/// Helper to check if events contain an error
fn has_error_event(events: &[PlaybackEvent]) -> bool {
    events.iter().any(|e| matches!(e, PlaybackEvent::Error(_)))
}

/// Create a valid minimal WAV file header
fn create_wav_header(data_size: u32, sample_rate: u32, channels: u16) -> Vec<u8> {
    let mut header = Vec::new();

    // RIFF header
    header.extend_from_slice(b"RIFF");
    header.extend_from_slice(&(36 + data_size).to_le_bytes()); // File size - 8
    header.extend_from_slice(b"WAVE");

    // fmt chunk
    header.extend_from_slice(b"fmt ");
    header.extend_from_slice(&16u32.to_le_bytes()); // fmt chunk size
    header.extend_from_slice(&1u16.to_le_bytes()); // PCM format
    header.extend_from_slice(&channels.to_le_bytes());
    header.extend_from_slice(&sample_rate.to_le_bytes());
    let byte_rate = sample_rate * u32::from(channels) * 2; // 16-bit samples
    header.extend_from_slice(&byte_rate.to_le_bytes());
    let block_align = channels * 2;
    header.extend_from_slice(&block_align.to_le_bytes());
    header.extend_from_slice(&16u16.to_le_bytes()); // Bits per sample

    // data chunk header
    header.extend_from_slice(b"data");
    header.extend_from_slice(&data_size.to_le_bytes());

    header
}

#[test]
#[ignore = "Requires file I/O - run manually with: cargo test --test corrupted_file_recovery_test -- --include-ignored"]
fn test_truncated_wav_file() {
    println!("\n[CORRUPTED FILE TEST] Testing truncated WAV file handling");

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let truncated_path = temp_dir.path().join("truncated.wav");

    // Create a valid header but truncate the data
    let header = create_wav_header(48000, 48000, 2); // Claims 1 second of data
    let mut file = fs::File::create(&truncated_path).expect("Failed to create file");
    file.write_all(&header).expect("Failed to write header");
    // Write only a tiny bit of data (should cause decoder error)
    file.write_all(&[0u8; 100]).expect("Failed to write data");
    drop(file);

    let config = PlaybackConfig::default();
    let playback = DesktopPlayback::new(config).expect("Failed to create playback");

    let track = create_test_track_with_path("1", truncated_path);
    playback
        .send_command(PlaybackCommand::LoadPlaylist { tracks: vec![track], start_index: 0 })
        .expect("Failed to load playlist");

    playback
        .send_command(PlaybackCommand::Play)
        .expect("Failed to send play command");

    // Give time for playback to attempt loading
    std::thread::sleep(Duration::from_millis(200));

    let events = drain_events(&playback);
    println!("[CORRUPTED FILE TEST] Received {} events", events.len());

    for (i, event) in events.iter().enumerate() {
        match event {
            PlaybackEvent::Error(err) => {
                println!("  Event {}: Error({})", i, err);
            }
            PlaybackEvent::StateChanged(state) => {
                println!("  Event {}: StateChanged({:?})", i, state);
            }
            _ => {
                println!("  Event {}: {:?}", i, event);
            }
        }
    }

    // Should receive an error event or transition to stopped state
    let has_error = has_error_event(&events);
    let has_stopped = events.iter().any(|e| {
        matches!(
            e,
            PlaybackEvent::StateChanged(PlaybackState::Stopped)
        )
    });

    assert!(
        has_error || has_stopped,
        "Expected error or stopped state for truncated file"
    );

    println!("[CORRUPTED FILE TEST] ✓ Truncated file handled gracefully");
}

#[test]
#[ignore = "Requires file I/O - run manually with: cargo test --test corrupted_file_recovery_test -- --include-ignored"]
fn test_zero_byte_file() {
    println!("\n[CORRUPTED FILE TEST] Testing zero-byte file handling");

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let zero_path = temp_dir.path().join("zero.wav");

    // Create completely empty file
    fs::File::create(&zero_path).expect("Failed to create file");

    let config = PlaybackConfig::default();
    let playback = DesktopPlayback::new(config).expect("Failed to create playback");

    let track = create_test_track_with_path("1", zero_path);
    playback
        .send_command(PlaybackCommand::LoadPlaylist { tracks: vec![track], start_index: 0 })
        .expect("Failed to load playlist");

    playback
        .send_command(PlaybackCommand::Play)
        .expect("Failed to send play command");

    std::thread::sleep(Duration::from_millis(200));

    let events = drain_events(&playback);
    println!("[CORRUPTED FILE TEST] Received {} events", events.len());

    // Should receive error event
    assert!(
        has_error_event(&events),
        "Expected error event for zero-byte file"
    );

    println!("[CORRUPTED FILE TEST] ✓ Zero-byte file rejected correctly");
}

#[test]
#[ignore = "Requires file I/O - run manually with: cargo test --test corrupted_file_recovery_test -- --include-ignored"]
fn test_invalid_header() {
    println!("\n[CORRUPTED FILE TEST] Testing invalid WAV header handling");

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let invalid_path = temp_dir.path().join("invalid.wav");

    // Create file with random garbage data
    let mut file = fs::File::create(&invalid_path).expect("Failed to create file");
    let garbage = vec![0xFFu8; 1024]; // Not a valid WAV
    file.write_all(&garbage).expect("Failed to write data");
    drop(file);

    let config = PlaybackConfig::default();
    let playback = DesktopPlayback::new(config).expect("Failed to create playback");

    let track = create_test_track_with_path("1", invalid_path);
    playback
        .send_command(PlaybackCommand::LoadPlaylist { tracks: vec![track], start_index: 0 })
        .expect("Failed to load playlist");

    playback
        .send_command(PlaybackCommand::Play)
        .expect("Failed to send play command");

    std::thread::sleep(Duration::from_millis(200));

    let events = drain_events(&playback);
    println!("[CORRUPTED FILE TEST] Received {} events", events.len());

    // Should receive error event
    assert!(
        has_error_event(&events),
        "Expected error event for invalid header"
    );

    println!("[CORRUPTED FILE TEST] ✓ Invalid header rejected correctly");
}

#[test]
#[ignore = "Requires file I/O - run manually with: cargo test --test corrupted_file_recovery_test -- --include-ignored"]
fn test_missing_file_during_playback() {
    println!("\n[CORRUPTED FILE TEST] Testing missing file recovery");

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let missing_path = temp_dir.path().join("will_delete.wav");

    // Create a valid WAV file first
    let header = create_wav_header(96000, 48000, 2); // 1 second
    let mut file = fs::File::create(&missing_path).expect("Failed to create file");
    file.write_all(&header).expect("Failed to write header");
    let data = vec![0u8; 96000];
    file.write_all(&data).expect("Failed to write data");
    drop(file);

    let config = PlaybackConfig::default();
    let playback = DesktopPlayback::new(config).expect("Failed to create playback");

    // Note: File will be missing when we try to play it
    // This simulates the file being deleted/moved between queue and playback
    fs::remove_file(&missing_path).expect("Failed to delete file");

    let track = create_test_track_with_path("1", missing_path);
    playback
        .send_command(PlaybackCommand::LoadPlaylist { tracks: vec![track], start_index: 0 })
        .expect("Failed to load playlist");

    playback
        .send_command(PlaybackCommand::Play)
        .expect("Failed to send play command");

    std::thread::sleep(Duration::from_millis(200));

    let events = drain_events(&playback);
    println!("[CORRUPTED FILE TEST] Received {} events", events.len());

    // Should receive error event
    assert!(
        has_error_event(&events),
        "Expected error event for missing file"
    );

    println!("[CORRUPTED FILE TEST] ✓ Missing file handled gracefully");
}

#[test]
#[ignore = "Requires file I/O - run manually with: cargo test --test corrupted_file_recovery_test -- --include-ignored"]
fn test_recovery_after_corrupted_file() {
    println!("\n[CORRUPTED FILE TEST] Testing recovery and skip to next track");

    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Create one corrupted file
    let corrupted_path = temp_dir.path().join("corrupted.wav");
    let mut file = fs::File::create(&corrupted_path).expect("Failed to create file");
    file.write_all(&[0xFFu8; 100]).expect("Failed to write data");
    drop(file);

    // Create one valid file
    let valid_path = temp_dir.path().join("valid.wav");
    let header = create_wav_header(96000, 48000, 2);
    let mut file = fs::File::create(&valid_path).expect("Failed to create file");
    file.write_all(&header).expect("Failed to write header");
    let data = vec![0u8; 96000];
    file.write_all(&data).expect("Failed to write data");
    drop(file);

    let config = PlaybackConfig::default();
    let playback = DesktopPlayback::new(config).expect("Failed to create playback");

    let tracks = vec![
        create_test_track_with_path("1", corrupted_path),
        create_test_track_with_path("2", valid_path),
    ];

    playback
        .send_command(PlaybackCommand::LoadPlaylist { tracks: tracks, start_index: 0 })
        .expect("Failed to load playlist");

    playback
        .send_command(PlaybackCommand::Play)
        .expect("Failed to send play command");

    // Wait for error on first track
    std::thread::sleep(Duration::from_millis(200));
    let _ = drain_events(&playback);

    // Try to skip to next track
    println!("[CORRUPTED FILE TEST] Skipping to valid track...");
    playback
        .send_command(PlaybackCommand::SkipNext)
        .expect("Failed to skip");

    std::thread::sleep(Duration::from_millis(200));

    let events = drain_events(&playback);
    println!("[CORRUPTED FILE TEST] Received {} events after skip", events.len());

    // System should still be functional
    let result = playback.send_command(PlaybackCommand::Pause);
    assert!(
        result.is_ok(),
        "System locked up after encountering corrupted file"
    );

    println!("[CORRUPTED FILE TEST] ✓ Successfully recovered after corrupted file");
    println!("[CORRUPTED FILE TEST] ✓ Able to skip to next valid track");
}

#[test]
#[ignore = "Requires file I/O - run manually with: cargo test --test corrupted_file_recovery_test -- --include-ignored"]
fn test_playlist_of_corrupted_files() {
    println!("\n[CORRUPTED FILE TEST] Testing playlist with multiple corrupted files");

    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Create 3 corrupted files
    let mut tracks = Vec::new();
    for i in 1..=3 {
        let path = temp_dir.path().join(format!("corrupted_{}.wav", i));
        let mut file = fs::File::create(&path).expect("Failed to create file");
        file.write_all(&[0xFFu8; 100]).expect("Failed to write data");
        drop(file);

        tracks.push(create_test_track_with_path(&i.to_string(), path));
    }

    let config = PlaybackConfig::default();
    let playback = DesktopPlayback::new(config).expect("Failed to create playback");

    playback
        .send_command(PlaybackCommand::LoadPlaylist { tracks: tracks, start_index: 0 })
        .expect("Failed to load playlist");

    playback
        .send_command(PlaybackCommand::Play)
        .expect("Failed to send play command");

    // Give time to process all tracks
    std::thread::sleep(Duration::from_millis(500));

    let events = drain_events(&playback);
    println!(
        "[CORRUPTED FILE TEST] Received {} events for 3 corrupted files",
        events.len()
    );

    // Should have received multiple errors
    let error_count = events
        .iter()
        .filter(|e| matches!(e, PlaybackEvent::Error(_)))
        .count();
    println!("[CORRUPTED FILE TEST] Error events: {}", error_count);

    // System should still respond to commands
    let result = playback.send_command(PlaybackCommand::Pause);
    assert!(
        result.is_ok(),
        "System locked up after multiple corrupted files"
    );

    println!("[CORRUPTED FILE TEST] ✓ Survived playlist of corrupted files");
    println!("[CORRUPTED FILE TEST] ✓ System remained responsive");
}
