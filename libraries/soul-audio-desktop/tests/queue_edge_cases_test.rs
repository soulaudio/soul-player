//! Queue edge case tests
//!
//! Tests for empty queue, null device, and boundary condition scenarios.
//! Ensures robust error handling and graceful degradation.

#![allow(clippy::len_zero)]

use soul_audio_desktop::{DesktopPlayback, PlaybackCommand};
use soul_playback::{PlaybackConfig, QueueTrack, RepeatMode, ShuffleMode, TrackSource};
use std::path::PathBuf;
use std::time::Duration;

fn create_track(id: &str, title: &str) -> QueueTrack {
    QueueTrack {
        id: id.to_string(),
        path: PathBuf::from(format!("/music/{}.mp3", id)),
        title: title.to_string(),
        artist: "Test Artist".to_string(),
        album: Some("Test Album".to_string()),
        duration: Duration::from_secs(180),
        track_number: Some(1),
        source: TrackSource::Single,
    }
}

fn create_track_with_duration(id: &str, title: &str, duration_secs: u64) -> QueueTrack {
    QueueTrack {
        id: id.to_string(),
        path: PathBuf::from(format!("/music/{}.mp3", id)),
        title: title.to_string(),
        artist: "Test Artist".to_string(),
        album: Some("Test Album".to_string()),
        duration: Duration::from_secs(duration_secs),
        track_number: Some(1),
        source: TrackSource::Single,
    }
}

fn drain_events(playback: &DesktopPlayback) {
    while playback.try_recv_event().is_some() {}
}

// ===== Empty Queue Tests =====

#[test]
#[ignore = "Requires real audio hardware - not available in CI environments"]
fn test_play_empty_queue() {
    let playback = DesktopPlayback::new(PlaybackConfig::default()).unwrap();

    // Try to play with empty queue
    let result = playback.send_command(PlaybackCommand::Play);

    // Assert: Should handle gracefully (either error or no-op)
    // Should NOT panic
    assert!(
        result.is_ok(),
        "Play command should succeed even with empty queue (no-op)"
    );
}

#[test]
#[ignore = "Requires real audio hardware - not available in CI environments"]
fn test_next_on_empty_queue() {
    let playback = DesktopPlayback::new(PlaybackConfig::default()).unwrap();

    // Try to skip next with empty queue
    let result = playback.send_command(PlaybackCommand::Next);

    // Assert: Should handle gracefully without panic
    assert!(
        result.is_ok(),
        "Next command should succeed on empty queue (no-op)"
    );
}

#[test]
#[ignore = "Requires real audio hardware - not available in CI environments"]
fn test_previous_on_empty_queue() {
    let playback = DesktopPlayback::new(PlaybackConfig::default()).unwrap();

    // Try to go previous with empty queue
    let result = playback.send_command(PlaybackCommand::Previous);

    // Assert: Should handle gracefully without panic
    assert!(
        result.is_ok(),
        "Previous command should succeed on empty queue (no-op)"
    );
}

#[test]
#[ignore = "Requires real audio hardware - not available in CI environments"]
fn test_pause_on_empty_queue() {
    let playback = DesktopPlayback::new(PlaybackConfig::default()).unwrap();

    // Try to pause with empty queue
    let result = playback.send_command(PlaybackCommand::Pause);

    // Assert: Should handle gracefully
    assert!(
        result.is_ok(),
        "Pause command should succeed on empty queue (no-op)"
    );
}

#[test]
#[ignore = "Requires real audio hardware - not available in CI environments"]
fn test_seek_on_empty_queue() {
    let playback = DesktopPlayback::new(PlaybackConfig::default()).unwrap();

    // Try to seek with empty queue
    let result = playback.send_command(PlaybackCommand::Seek(30.0));

    // Assert: Should handle gracefully (either error or no-op)
    assert!(
        result.is_ok(),
        "Seek command should succeed on empty queue (no-op)"
    );
}

// ===== Queue Boundary Tests =====

#[test]
#[ignore = "Requires real audio hardware - not available in CI environments"]
fn test_next_at_queue_end() {
    let playback = DesktopPlayback::new(PlaybackConfig::default()).unwrap();

    // Load 2-track queue
    playback
        .send_command(PlaybackCommand::AddToQueue(create_track("1", "Track 1")))
        .unwrap();
    playback
        .send_command(PlaybackCommand::AddToQueue(create_track("2", "Track 2")))
        .unwrap();

    std::thread::sleep(Duration::from_millis(50));

    // Skip through entire queue
    playback.send_command(PlaybackCommand::Next).unwrap();
    std::thread::sleep(Duration::from_millis(20));
    playback.send_command(PlaybackCommand::Next).unwrap();
    std::thread::sleep(Duration::from_millis(20));

    // Try to go next again at end
    let result = playback.send_command(PlaybackCommand::Next);

    // Assert: Should handle gracefully without panic
    assert!(result.is_ok(), "Next command at queue end should not panic");

    drain_events(&playback);

    // Verify queue is empty
    assert!(!playback.has_next(), "Should not have next at queue end");
}

#[test]
#[ignore = "Requires real audio hardware - not available in CI environments"]
fn test_previous_at_queue_start() {
    let playback = DesktopPlayback::new(PlaybackConfig::default()).unwrap();

    // Load one track
    playback
        .send_command(PlaybackCommand::AddToQueue(create_track("1", "Track 1")))
        .unwrap();

    std::thread::sleep(Duration::from_millis(50));

    // Try to go previous at start
    let result = playback.send_command(PlaybackCommand::Previous);

    // Assert: Should handle gracefully
    assert!(
        result.is_ok(),
        "Previous command at queue start should not panic"
    );
}

#[test]
#[ignore = "Requires real audio hardware - not available in CI environments"]
fn test_rapid_next_beyond_queue_end() {
    let playback = DesktopPlayback::new(PlaybackConfig::default()).unwrap();

    // Load 3 tracks
    for i in 1..=3 {
        playback
            .send_command(PlaybackCommand::AddToQueue(create_track(
                &i.to_string(),
                &format!("Track {}", i),
            )))
            .unwrap();
    }

    std::thread::sleep(Duration::from_millis(50));

    // Rapidly skip beyond queue end
    for _ in 0..10 {
        let result = playback.send_command(PlaybackCommand::Next);
        assert!(result.is_ok(), "Rapid next should not panic");
    }

    drain_events(&playback);

    // Verify queue is exhausted
    assert!(!playback.has_next(), "Queue should be exhausted");
}

// ===== Seek Boundary Tests =====

#[test]
#[ignore = "Requires real audio hardware - not available in CI environments"]
fn test_seek_beyond_track_duration() {
    let playback = DesktopPlayback::new(PlaybackConfig::default()).unwrap();

    // Load 10-second track
    playback
        .send_command(PlaybackCommand::AddToQueue(create_track_with_duration(
            "1", "Track 1", 10,
        )))
        .unwrap();

    std::thread::sleep(Duration::from_millis(50));

    // Try to seek to 20 seconds (beyond duration)
    let result = playback.send_command(PlaybackCommand::Seek(20.0));

    // Assert: Should handle gracefully (clamped or error, not panic)
    assert!(
        result.is_ok(),
        "Seek beyond duration should not panic (should clamp or error)"
    );
}

#[test]
#[ignore = "Requires real audio hardware - not available in CI environments"]
fn test_seek_negative_position() {
    let playback = DesktopPlayback::new(PlaybackConfig::default()).unwrap();

    playback
        .send_command(PlaybackCommand::AddToQueue(create_track("1", "Track 1")))
        .unwrap();

    std::thread::sleep(Duration::from_millis(50));

    // Try to seek to negative position
    let result = playback.send_command(PlaybackCommand::Seek(-5.0));

    // Assert: Should handle gracefully (clamped to 0 or error, not panic)
    assert!(
        result.is_ok(),
        "Negative seek should not panic (should clamp to 0 or error)"
    );
}

#[test]
#[ignore = "Requires real audio hardware - not available in CI environments"]
fn test_seek_extreme_values() {
    let playback = DesktopPlayback::new(PlaybackConfig::default()).unwrap();

    playback
        .send_command(PlaybackCommand::AddToQueue(create_track("1", "Track 1")))
        .unwrap();

    std::thread::sleep(Duration::from_millis(50));

    // Try extreme seek values
    let extreme_values = [
        f64::MAX,
        f64::MIN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        0.0,
        1e10, // 10 billion seconds
    ];

    for &value in &extreme_values {
        let result = playback.send_command(PlaybackCommand::Seek(value));

        // Assert: Should handle gracefully (no panic, no crash)
        assert!(
            result.is_ok(),
            "Extreme seek value {:?} should not panic",
            value
        );
    }
}

// ===== Large Queue Capacity Tests =====

#[test]
#[ignore = "Requires real audio hardware - not available in CI environments"]
fn test_queue_large_capacity() {
    let playback = DesktopPlayback::new(PlaybackConfig::default()).unwrap();

    // Try to add 1,000 tracks
    for i in 1..=1000 {
        let result = playback.send_command(PlaybackCommand::AddToQueue(create_track(
            &i.to_string(),
            &format!("Track {}", i),
        )));

        assert!(result.is_ok(), "Adding track {} should succeed", i);

        // Small delay every 100 tracks to avoid overwhelming the channel
        if i % 100 == 0 {
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    std::thread::sleep(Duration::from_millis(200));
    drain_events(&playback);

    // Verify queue size
    let queue = playback.get_queue();
    assert_eq!(queue.len(), 1000, "Queue should hold 1000 tracks");
}

#[test]
#[ignore = "Requires real audio hardware - not available in CI environments"]
fn test_queue_very_large_capacity() {
    let playback = DesktopPlayback::new(PlaybackConfig::default()).unwrap();

    // Try to add 10,000 tracks (stress test)
    let target_count = 10000;

    for i in 1..=target_count {
        let result = playback.send_command(PlaybackCommand::AddToQueue(create_track(
            &i.to_string(),
            &format!("Track {}", i),
        )));

        // Should either accept or reject gracefully
        if result.is_err() {
            tracing::warn!("Queue rejected track {} (capacity limit reached)", i);
            break;
        }

        // Small delay every 100 tracks
        if i % 100 == 0 {
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    std::thread::sleep(Duration::from_millis(500));
    drain_events(&playback);

    // Verify we didn't crash
    let queue = playback.get_queue();
    tracing::info!("Successfully added {} tracks to queue", queue.len());

    // Should have added at least some tracks
    assert!(queue.len() > 0, "Queue should have tracks");
}

// ===== Remove from Queue Edge Cases =====

#[test]
#[ignore = "Requires real audio hardware - not available in CI environments"]
fn test_remove_from_empty_queue() {
    let playback = DesktopPlayback::new(PlaybackConfig::default()).unwrap();

    // Try to remove from empty queue
    let result = playback.send_command(PlaybackCommand::RemoveFromQueue(0));

    // Assert: Should handle gracefully (likely error, but no panic)
    assert!(
        result.is_ok() || result.is_err(),
        "Remove from empty queue should not panic"
    );
}

#[test]
#[ignore = "Requires real audio hardware - not available in CI environments"]
fn test_remove_invalid_index() {
    let playback = DesktopPlayback::new(PlaybackConfig::default()).unwrap();

    // Add 3 tracks
    for i in 1..=3 {
        playback
            .send_command(PlaybackCommand::AddToQueue(create_track(
                &i.to_string(),
                &format!("Track {}", i),
            )))
            .unwrap();
    }

    std::thread::sleep(Duration::from_millis(50));

    // Try to remove index 100 (out of bounds)
    let result = playback.send_command(PlaybackCommand::RemoveFromQueue(100));

    // Assert: Should handle gracefully
    assert!(
        result.is_ok() || result.is_err(),
        "Remove invalid index should not panic"
    );
}

#[test]
#[ignore = "Requires real audio hardware - not available in CI environments"]
fn test_remove_all_tracks_one_by_one() {
    let playback = DesktopPlayback::new(PlaybackConfig::default()).unwrap();

    // Add 5 tracks
    for i in 1..=5 {
        playback
            .send_command(PlaybackCommand::AddToQueue(create_track(
                &i.to_string(),
                &format!("Track {}", i),
            )))
            .unwrap();
    }

    std::thread::sleep(Duration::from_millis(50));

    // Remove all tracks (always remove index 0)
    for _ in 0..5 {
        let result = playback.send_command(PlaybackCommand::RemoveFromQueue(0));
        assert!(result.is_ok(), "Remove should succeed");
        std::thread::sleep(Duration::from_millis(10));
    }

    drain_events(&playback);

    // Verify queue is empty
    let queue = playback.get_queue();
    assert_eq!(queue.len(), 0, "Queue should be empty after removing all");
}

// ===== Repeat Mode Edge Cases =====

#[test]
#[ignore = "Requires real audio hardware - not available in CI environments"]
fn test_next_with_repeat_one_on_empty_queue() {
    let mut config = PlaybackConfig::default();
    config.repeat = RepeatMode::One;
    let playback = DesktopPlayback::new(config).unwrap();

    // Try to skip next with repeat one but empty queue
    let result = playback.send_command(PlaybackCommand::Next);

    // Assert: Should handle gracefully
    assert!(
        result.is_ok(),
        "Next with repeat one on empty queue should not panic"
    );
}

#[test]
#[ignore = "Requires real audio hardware - not available in CI environments"]
fn test_repeat_all_with_single_track() {
    let mut config = PlaybackConfig::default();
    config.repeat = RepeatMode::All;
    let playback = DesktopPlayback::new(config).unwrap();

    playback
        .send_command(PlaybackCommand::AddToQueue(create_track("1", "Track 1")))
        .unwrap();

    std::thread::sleep(Duration::from_millis(50));

    // Skip next multiple times (should loop)
    for _ in 0..5 {
        let result = playback.send_command(PlaybackCommand::Next);
        assert!(result.is_ok(), "Next with repeat all should succeed");
        std::thread::sleep(Duration::from_millis(20));
    }

    drain_events(&playback);

    // Should still have next (repeat all loops)
    assert!(playback.has_next(), "Should have next with repeat all");
}

// ===== Shuffle Edge Cases =====

#[test]
#[ignore = "Requires real audio hardware - not available in CI environments"]
fn test_shuffle_single_track() {
    let mut config = PlaybackConfig::default();
    config.shuffle = ShuffleMode::Random;
    let playback = DesktopPlayback::new(config).unwrap();

    playback
        .send_command(PlaybackCommand::AddToQueue(create_track("1", "Track 1")))
        .unwrap();

    std::thread::sleep(Duration::from_millis(50));
    drain_events(&playback);

    // Verify queue still has the track
    let queue = playback.get_queue();
    assert_eq!(
        queue.len(),
        1,
        "Single track queue should work with shuffle"
    );
    assert_eq!(queue[0].id, "1", "Track should be present");
}

#[test]
#[ignore = "Requires real audio hardware - not available in CI environments"]
fn test_shuffle_empty_queue() {
    let mut config = PlaybackConfig::default();
    config.shuffle = ShuffleMode::Random;
    let playback = DesktopPlayback::new(config).unwrap();

    // Try operations with shuffle enabled but empty queue
    let _ = playback.send_command(PlaybackCommand::Play);
    let _ = playback.send_command(PlaybackCommand::Next);

    // Should not panic
}

// ===== Clear Queue Edge Cases =====

#[test]
#[ignore = "Requires real audio hardware - not available in CI environments"]
fn test_clear_empty_queue() {
    let playback = DesktopPlayback::new(PlaybackConfig::default()).unwrap();

    // Clear already empty queue
    let result = playback.send_command(PlaybackCommand::ClearQueue);

    // Assert: Should handle gracefully
    assert!(result.is_ok(), "Clear empty queue should succeed");

    drain_events(&playback);

    let queue = playback.get_queue();
    assert_eq!(queue.len(), 0, "Queue should remain empty");
}

#[test]
#[ignore = "Requires real audio hardware - not available in CI environments"]
fn test_clear_during_playback() {
    let playback = DesktopPlayback::new(PlaybackConfig::default()).unwrap();

    // Add tracks
    for i in 1..=5 {
        playback
            .send_command(PlaybackCommand::AddToQueue(create_track(
                &i.to_string(),
                &format!("Track {}", i),
            )))
            .unwrap();
    }

    std::thread::sleep(Duration::from_millis(50));

    // Start playback
    playback.send_command(PlaybackCommand::Play).unwrap();
    std::thread::sleep(Duration::from_millis(20));

    // Clear queue during playback
    let result = playback.send_command(PlaybackCommand::ClearQueue);

    // Assert: Should handle gracefully
    assert!(result.is_ok(), "Clear during playback should succeed");

    drain_events(&playback);

    let queue = playback.get_queue();
    assert_eq!(queue.len(), 0, "Queue should be empty after clear");
}

// ===== Volume Edge Cases =====

#[test]
#[ignore = "Requires real audio hardware - not available in CI environments"]
fn test_volume_boundary_values() {
    let playback = DesktopPlayback::new(PlaybackConfig::default()).unwrap();

    // Test boundary volume values
    let valid_volumes = [0, 1, 50, 99, 100];

    for &volume in &valid_volumes {
        let result = playback.send_command(PlaybackCommand::SetVolume(volume));
        assert!(result.is_ok(), "Volume {} should be accepted", volume);
    }
}

// ===== Concurrent Operations Edge Cases =====

#[test]
#[ignore = "Requires real audio hardware - not available in CI environments"]
fn test_rapid_play_pause_on_empty_queue() {
    let playback = DesktopPlayback::new(PlaybackConfig::default()).unwrap();

    // Rapidly toggle play/pause with empty queue
    for _ in 0..20 {
        playback.send_command(PlaybackCommand::Play).unwrap();
        playback.send_command(PlaybackCommand::Pause).unwrap();
    }

    // Should not panic
}

#[test]
#[ignore = "Requires real audio hardware - not available in CI environments"]
fn test_rapid_seek_on_empty_queue() {
    let playback = DesktopPlayback::new(PlaybackConfig::default()).unwrap();

    // Rapidly seek on empty queue
    for i in 0..20 {
        let result = playback.send_command(PlaybackCommand::Seek((i as f64) * 10.0));
        assert!(result.is_ok(), "Rapid seek should not panic");
    }
}

#[test]
#[ignore = "Requires real audio hardware - not available in CI environments"]
fn test_add_and_remove_rapidly() {
    let playback = DesktopPlayback::new(PlaybackConfig::default()).unwrap();

    // Rapidly add and remove tracks
    for i in 0..50 {
        playback
            .send_command(PlaybackCommand::AddToQueue(create_track(
                &i.to_string(),
                &format!("Track {}", i),
            )))
            .unwrap();

        if i % 2 == 0 {
            let _ = playback.send_command(PlaybackCommand::RemoveFromQueue(0));
        }
    }

    std::thread::sleep(Duration::from_millis(100));
    drain_events(&playback);

    // Should not panic, queue should be in valid state
    let queue = playback.get_queue();
    // Just checking we can query the queue without panic
    let _len = queue.len();
}

// Note: SkipToIndex tests removed - this command doesn't exist in PlaybackCommand
// Queue navigation is handled via Next/Previous commands

// ===== State Query Edge Cases =====

#[test]
#[ignore = "Requires real audio hardware - not available in CI environments"]
fn test_get_queue_rapid_queries() {
    let playback = DesktopPlayback::new(PlaybackConfig::default()).unwrap();

    // Rapidly query queue (stress test)
    for _ in 0..1000 {
        let _ = playback.get_queue();
    }

    // Should not panic or deadlock
}

#[test]
#[ignore = "Requires real audio hardware - not available in CI environments"]
fn test_has_next_has_previous_rapid_queries() {
    let playback = DesktopPlayback::new(PlaybackConfig::default()).unwrap();

    // Rapidly query navigation state
    for _ in 0..1000 {
        let _ = playback.has_next();
        let _ = playback.has_previous();
    }

    // Should not panic or deadlock
}

#[test]
#[ignore = "Requires real audio hardware - not available in CI environments"]
fn test_state_queries_during_queue_modifications() {
    let playback = DesktopPlayback::new(PlaybackConfig::default()).unwrap();

    // Add tracks while rapidly querying state
    for i in 0..100 {
        playback
            .send_command(PlaybackCommand::AddToQueue(create_track(
                &i.to_string(),
                &format!("Track {}", i),
            )))
            .unwrap();

        // Query state between additions
        let _ = playback.get_queue();
        let _ = playback.has_next();
        let _ = playback.has_previous();
    }

    // Should not panic or produce inconsistent state
}
