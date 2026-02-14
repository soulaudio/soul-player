//! Integration tests for seek functionality
//!
//! These tests verify that seeking works correctly in actual playback scenarios,
//! including position verification, seeking to various positions, and handling
//! edge cases like seeking near the end of a track or with different file formats.

use soul_audio_desktop::{DesktopPlayback, PlaybackCommand, PlaybackEvent};
use soul_playback::{PlaybackConfig, PlaybackState, QueueTrack, TrackSource};
use std::path::PathBuf;
use std::time::Duration;

/// Helper to create a test queue track
fn create_test_track(id: &str, title: &str, path: &str, duration_secs: u64) -> QueueTrack {
    QueueTrack {
        id: id.to_string(),
        path: PathBuf::from(path),
        title: title.to_string(),
        artist: "Test Artist".to_string(),
        album: Some("Test Album".to_string()),
        duration: Duration::from_secs(duration_secs),
        track_number: Some(1),
        source: TrackSource::Single,
    }
}

/// Helper to drain events from playback until timeout
fn drain_events(playback: &DesktopPlayback, timeout_ms: u64) -> Vec<PlaybackEvent> {
    let start = std::time::Instant::now();
    let mut events = Vec::new();

    while start.elapsed() < Duration::from_millis(timeout_ms) {
        if let Some(event) = playback.try_recv_event() {
            events.push(event);
        } else {
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    events
}

/// Helper to find the last position update event
fn find_last_position_event(events: &[PlaybackEvent]) -> Option<f64> {
    events.iter().rev().find_map(|e| {
        if let PlaybackEvent::PositionChanged(pos) = e {
            Some(*pos)
        } else {
            None
        }
    })
}

#[test]
#[ignore = "Requires real audio files and hardware - run manually with: cargo test seek_to_middle_of_track -- --ignored"]
fn test_seek_to_middle_of_track() {
    // This test requires a real audio file at this path
    // Adjust the path to point to a valid test file
    let test_file = "tests/data/test_track.mp3";

    if !PathBuf::from(test_file).exists() {
        eprintln!("Test file not found: {}", test_file);
        eprintln!("Skipping test - please provide a valid test audio file");
        return;
    }

    let config = PlaybackConfig::default();
    let playback = DesktopPlayback::new(config).expect("Failed to create playback");

    // Load a track (assume 180 second duration for testing)
    let track = create_test_track("test1", "Test Track", test_file, 180);
    playback
        .send_command(PlaybackCommand::LoadQueue(vec![track], 0))
        .expect("Failed to load queue");

    // Wait for track to load
    std::thread::sleep(Duration::from_millis(200));

    // Start playback
    playback
        .send_command(PlaybackCommand::Play)
        .expect("Failed to start playback");

    // Wait for playback to start
    std::thread::sleep(Duration::from_millis(200));

    // Drain initial events
    drain_events(&playback, 100);

    // Seek to middle (90 seconds)
    let target_position = 90.0;
    playback
        .send_command(PlaybackCommand::Seek(target_position))
        .expect("Failed to seek");

    // Wait for seek to complete and position updates
    std::thread::sleep(Duration::from_millis(300));

    // Collect position events
    let events = drain_events(&playback, 200);

    // Find last position event
    if let Some(position) = find_last_position_event(&events) {
        // Position should be close to target (within 1 second tolerance)
        assert!(
            (position - target_position).abs() < 1.0,
            "Position after seek should be close to {}, got {}",
            target_position,
            position
        );
    } else {
        panic!("No position events received after seek");
    }
}

#[test]
#[ignore = "Requires real audio files and hardware - run manually with: cargo test seek_to_beginning -- --ignored"]
fn test_seek_to_beginning() {
    let test_file = "tests/data/test_track.mp3";

    if !PathBuf::from(test_file).exists() {
        eprintln!("Test file not found, skipping test");
        return;
    }

    let config = PlaybackConfig::default();
    let playback = DesktopPlayback::new(config).expect("Failed to create playback");

    let track = create_test_track("test1", "Test Track", test_file, 180);
    playback
        .send_command(PlaybackCommand::LoadQueue(vec![track], 0))
        .expect("Failed to load queue");

    std::thread::sleep(Duration::from_millis(200));

    playback
        .send_command(PlaybackCommand::Play)
        .expect("Failed to start playback");

    std::thread::sleep(Duration::from_millis(200));

    // Let track play for a bit
    std::thread::sleep(Duration::from_millis(500));

    drain_events(&playback, 100);

    // Seek to beginning
    playback
        .send_command(PlaybackCommand::Seek(0.0))
        .expect("Failed to seek");

    std::thread::sleep(Duration::from_millis(300));

    let events = drain_events(&playback, 200);

    if let Some(position) = find_last_position_event(&events) {
        assert!(
            position < 1.0,
            "Position after seeking to start should be near 0, got {}",
            position
        );
    } else {
        panic!("No position events received after seek");
    }
}

#[test]
#[ignore = "Requires real audio files and hardware - run manually with: cargo test seek_near_end -- --ignored"]
fn test_seek_near_end_of_track() {
    let test_file = "tests/data/test_track.mp3";

    if !PathBuf::from(test_file).exists() {
        eprintln!("Test file not found, skipping test");
        return;
    }

    let config = PlaybackConfig::default();
    let playback = DesktopPlayback::new(config).expect("Failed to create playback");

    let track = create_test_track("test1", "Test Track", test_file, 180);
    playback
        .send_command(PlaybackCommand::LoadQueue(vec![track], 0))
        .expect("Failed to load queue");

    std::thread::sleep(Duration::from_millis(200));

    playback
        .send_command(PlaybackCommand::Play)
        .expect("Failed to start playback");

    std::thread::sleep(Duration::from_millis(200));

    drain_events(&playback, 100);

    // Seek to 2 seconds before end
    let target_position = 178.0;
    playback
        .send_command(PlaybackCommand::Seek(target_position))
        .expect("Failed to seek");

    std::thread::sleep(Duration::from_millis(300));

    let events = drain_events(&playback, 200);

    if let Some(position) = find_last_position_event(&events) {
        assert!(
            (position - target_position).abs() < 2.0,
            "Position after seek should be close to {}, got {}",
            target_position,
            position
        );
    } else {
        panic!("No position events received after seek");
    }
}

#[test]
#[ignore = "Requires real audio files and hardware - run manually with: cargo test multiple_rapid_seeks -- --ignored"]
fn test_multiple_rapid_seeks() {
    let test_file = "tests/data/test_track.mp3";

    if !PathBuf::from(test_file).exists() {
        eprintln!("Test file not found, skipping test");
        return;
    }

    let config = PlaybackConfig::default();
    let playback = DesktopPlayback::new(config).expect("Failed to create playback");

    let track = create_test_track("test1", "Test Track", test_file, 180);
    playback
        .send_command(PlaybackCommand::LoadQueue(vec![track], 0))
        .expect("Failed to load queue");

    std::thread::sleep(Duration::from_millis(200));

    playback
        .send_command(PlaybackCommand::Play)
        .expect("Failed to start playback");

    std::thread::sleep(Duration::from_millis(200));

    drain_events(&playback, 100);

    // Send multiple rapid seek commands (simulating user clicking multiple times)
    let positions = [30.0, 60.0, 90.0, 120.0, 150.0];
    for pos in positions.iter() {
        playback
            .send_command(PlaybackCommand::Seek(*pos))
            .expect("Failed to seek");
        std::thread::sleep(Duration::from_millis(50));
    }

    // Wait for all seeks to settle
    std::thread::sleep(Duration::from_millis(500));

    let events = drain_events(&playback, 300);

    // Should end up at the last seek position
    if let Some(position) = find_last_position_event(&events) {
        let last_target = positions.last().unwrap();
        assert!(
            (position - last_target).abs() < 2.0,
            "Final position should be close to last seek target {}, got {}",
            last_target,
            position
        );
    } else {
        panic!("No position events received after multiple seeks");
    }
}

#[test]
#[ignore = "Requires real audio files and hardware - run manually with: cargo test seek_while_paused -- --ignored"]
fn test_seek_while_paused() {
    let test_file = "tests/data/test_track.mp3";

    if !PathBuf::from(test_file).exists() {
        eprintln!("Test file not found, skipping test");
        return;
    }

    let config = PlaybackConfig::default();
    let playback = DesktopPlayback::new(config).expect("Failed to create playback");

    let track = create_test_track("test1", "Test Track", test_file, 180);
    playback
        .send_command(PlaybackCommand::LoadQueue(vec![track], 0))
        .expect("Failed to load queue");

    std::thread::sleep(Duration::from_millis(200));

    // Start playback
    playback
        .send_command(PlaybackCommand::Play)
        .expect("Failed to start playback");

    std::thread::sleep(Duration::from_millis(200));

    // Pause
    playback
        .send_command(PlaybackCommand::Pause)
        .expect("Failed to pause");

    std::thread::sleep(Duration::from_millis(200));

    drain_events(&playback, 100);

    // Seek while paused
    let target_position = 60.0;
    playback
        .send_command(PlaybackCommand::Seek(target_position))
        .expect("Failed to seek while paused");

    std::thread::sleep(Duration::from_millis(300));

    let events = drain_events(&playback, 200);

    if let Some(position) = find_last_position_event(&events) {
        assert!(
            (position - target_position).abs() < 1.0,
            "Position after seek while paused should be close to {}, got {}",
            target_position,
            position
        );
    } else {
        panic!("No position events received after seek while paused");
    }

    // Resume playback to verify seek worked
    playback
        .send_command(PlaybackCommand::Play)
        .expect("Failed to resume");

    std::thread::sleep(Duration::from_millis(300));

    let events = drain_events(&playback, 200);

    // Check that playback continues from seek position
    if let Some(position) = find_last_position_event(&events) {
        // Position should have advanced slightly from target
        assert!(
            position >= target_position && position < target_position + 2.0,
            "Position after resume should be near seek position, got {}",
            position
        );
    }
}

#[test]
#[ignore = "Requires real FLAC file and hardware - run manually with: cargo test seek_flac_file -- --ignored"]
fn test_seek_flac_file() {
    // Test seeking with FLAC format specifically
    let test_file = "tests/data/test_track.flac";

    if !PathBuf::from(test_file).exists() {
        eprintln!("FLAC test file not found, skipping test");
        return;
    }

    let config = PlaybackConfig::default();
    let playback = DesktopPlayback::new(config).expect("Failed to create playback");

    let track = create_test_track("test1", "Test FLAC Track", test_file, 180);
    playback
        .send_command(PlaybackCommand::LoadQueue(vec![track], 0))
        .expect("Failed to load queue");

    std::thread::sleep(Duration::from_millis(300)); // FLAC may take longer to decode

    playback
        .send_command(PlaybackCommand::Play)
        .expect("Failed to start playback");

    std::thread::sleep(Duration::from_millis(200));

    drain_events(&playback, 100);

    // Seek to middle
    let target_position = 90.0;
    playback
        .send_command(PlaybackCommand::Seek(target_position))
        .expect("Failed to seek in FLAC");

    std::thread::sleep(Duration::from_millis(400)); // FLAC seeking may be slower

    let events = drain_events(&playback, 300);

    if let Some(position) = find_last_position_event(&events) {
        assert!(
            (position - target_position).abs() < 1.5,
            "Position after FLAC seek should be close to {}, got {}",
            target_position,
            position
        );
    } else {
        panic!("No position events received after FLAC seek");
    }
}

#[test]
#[ignore = "Requires real MP3 file and hardware - run manually with: cargo test seek_mp3_file -- --ignored"]
fn test_seek_mp3_file() {
    // Test seeking with MP3 format specifically
    let test_file = "tests/data/test_track.mp3";

    if !PathBuf::from(test_file).exists() {
        eprintln!("MP3 test file not found, skipping test");
        return;
    }

    let config = PlaybackConfig::default();
    let playback = DesktopPlayback::new(config).expect("Failed to create playback");

    let track = create_test_track("test1", "Test MP3 Track", test_file, 180);
    playback
        .send_command(PlaybackCommand::LoadQueue(vec![track], 0))
        .expect("Failed to load queue");

    std::thread::sleep(Duration::from_millis(200));

    playback
        .send_command(PlaybackCommand::Play)
        .expect("Failed to start playback");

    std::thread::sleep(Duration::from_millis(200));

    drain_events(&playback, 100);

    // Seek to middle
    let target_position = 90.0;
    playback
        .send_command(PlaybackCommand::Seek(target_position))
        .expect("Failed to seek in MP3");

    std::thread::sleep(Duration::from_millis(300));

    let events = drain_events(&playback, 200);

    if let Some(position) = find_last_position_event(&events) {
        assert!(
            (position - target_position).abs() < 1.0,
            "Position after MP3 seek should be close to {}, got {}",
            target_position,
            position
        );
    } else {
        panic!("No position events received after MP3 seek");
    }
}

#[test]
#[ignore = "Requires real audio files and hardware - run manually with: cargo test seek_then_skip_track -- --ignored"]
fn test_seek_then_skip_to_next_track() {
    let test_file1 = "tests/data/test_track_1.mp3";
    let test_file2 = "tests/data/test_track_2.mp3";

    if !PathBuf::from(test_file1).exists() || !PathBuf::from(test_file2).exists() {
        eprintln!("Test files not found, skipping test");
        return;
    }

    let config = PlaybackConfig::default();
    let playback = DesktopPlayback::new(config).expect("Failed to create playback");

    // Load queue with two tracks
    let tracks = vec![
        create_test_track("test1", "Track 1", test_file1, 180),
        create_test_track("test2", "Track 2", test_file2, 180),
    ];

    playback
        .send_command(PlaybackCommand::LoadQueue(tracks, 0))
        .expect("Failed to load queue");

    std::thread::sleep(Duration::from_millis(200));

    playback
        .send_command(PlaybackCommand::Play)
        .expect("Failed to start playback");

    std::thread::sleep(Duration::from_millis(200));

    // Seek in first track
    playback
        .send_command(PlaybackCommand::Seek(60.0))
        .expect("Failed to seek");

    std::thread::sleep(Duration::from_millis(300));

    drain_events(&playback, 100);

    // Skip to next track
    playback
        .send_command(PlaybackCommand::Next)
        .expect("Failed to skip to next");

    std::thread::sleep(Duration::from_millis(400));

    let events = drain_events(&playback, 300);

    // Should have track changed event
    let has_track_change = events
        .iter()
        .any(|e| matches!(e, PlaybackEvent::TrackChanged(_)));

    assert!(has_track_change, "Should have track changed event");

    // Position should be near beginning of new track
    if let Some(position) = find_last_position_event(&events) {
        assert!(
            position < 5.0,
            "Position after skip should be near start of new track, got {}",
            position
        );
    }
}

#[test]
#[ignore = "Requires real audio files and hardware - run manually with: cargo test seek_boundary_conditions -- --ignored"]
fn test_seek_boundary_conditions() {
    let test_file = "tests/data/test_track.mp3";

    if !PathBuf::from(test_file).exists() {
        eprintln!("Test file not found, skipping test");
        return;
    }

    let config = PlaybackConfig::default();
    let playback = DesktopPlayback::new(config).expect("Failed to create playback");

    let track = create_test_track("test1", "Test Track", test_file, 180);
    playback
        .send_command(PlaybackCommand::LoadQueue(vec![track], 0))
        .expect("Failed to load queue");

    std::thread::sleep(Duration::from_millis(200));

    playback
        .send_command(PlaybackCommand::Play)
        .expect("Failed to start playback");

    std::thread::sleep(Duration::from_millis(200));

    // Test seeking to exactly 0.0
    playback
        .send_command(PlaybackCommand::Seek(0.0))
        .expect("Failed to seek to 0.0");

    std::thread::sleep(Duration::from_millis(300));
    drain_events(&playback, 100);

    // Test seeking to a fractional position
    playback
        .send_command(PlaybackCommand::Seek(45.5))
        .expect("Failed to seek to fractional position");

    std::thread::sleep(Duration::from_millis(300));

    let events = drain_events(&playback, 200);

    if let Some(position) = find_last_position_event(&events) {
        assert!(
            (position - 45.5).abs() < 1.0,
            "Should handle fractional seek positions, got {}",
            position
        );
    }

    // Test seeking to very near end (179.9 seconds)
    playback
        .send_command(PlaybackCommand::Seek(179.9))
        .expect("Failed to seek near end");

    std::thread::sleep(Duration::from_millis(300));

    let events = drain_events(&playback, 500);

    // Track might end and go to next track or stop
    // Just verify we don't crash
    let has_any_events = !events.is_empty();
    assert!(
        has_any_events,
        "Should receive events after seeking near end"
    );
}
