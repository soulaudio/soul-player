//! Queue navigation integration tests
//!
//! Tests for `has_next()`, `has_previous()`, and queue retrieval functionality.
//! Focuses on real-world usage scenarios from UI perspective.

use soul_audio_desktop::{DesktopPlayback, PlaybackCommand};
use soul_playback::{PlaybackConfig, QueueTrack, RepeatMode, TrackSource};
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

fn drain_events(playback: &DesktopPlayback) {
    while playback.try_recv_event().is_some() {}
}

// ===== has_next() / has_previous() Tests =====

#[test]
fn test_has_next_with_tracks_in_queue() {
    let playback = DesktopPlayback::new(PlaybackConfig::default()).unwrap();

    // Add tracks to queue
    playback
        .send_command(PlaybackCommand::AddToQueue(create_track("1", "Track 1")))
        .unwrap();
    playback
        .send_command(PlaybackCommand::AddToQueue(create_track("2", "Track 2")))
        .unwrap();

    std::thread::sleep(Duration::from_millis(50));
    drain_events(&playback);

    // Should have next track
    assert!(playback.has_next(), "Should have next with tracks in queue");
}

#[test]
fn test_has_next_empty_queue() {
    let playback = DesktopPlayback::new(PlaybackConfig::default()).unwrap();

    // Empty queue
    assert!(
        !playback.has_next(),
        "Should not have next with empty queue"
    );
}

#[test]
fn test_has_next_after_queue_consumed() {
    let playback = DesktopPlayback::new(PlaybackConfig::default()).unwrap();

    // Add one track
    playback
        .send_command(PlaybackCommand::AddToQueue(create_track("1", "Track 1")))
        .unwrap();

    std::thread::sleep(Duration::from_millis(50));

    // Initially has next
    assert!(playback.has_next());

    // Skip to next (consumes track)
    playback.send_command(PlaybackCommand::Next).unwrap();
    std::thread::sleep(Duration::from_millis(50));
    drain_events(&playback);

    // No longer has next
    assert!(
        !playback.has_next(),
        "Should not have next after consuming queue"
    );
}

#[test]
fn test_has_next_with_repeat_one() {
    let mut config = PlaybackConfig::default();
    config.repeat = RepeatMode::One;
    let playback = DesktopPlayback::new(config).unwrap();

    playback
        .send_command(PlaybackCommand::AddToQueue(create_track("1", "Track 1")))
        .unwrap();

    std::thread::sleep(Duration::from_millis(50));

    // With repeat one, always has next (same track)
    assert!(playback.has_next(), "Should have next with repeat one");

    // Even after consuming
    playback.send_command(PlaybackCommand::Next).unwrap();
    std::thread::sleep(Duration::from_millis(50));

    assert!(
        playback.has_next(),
        "Should still have next with repeat one after skip"
    );
}

#[test]
fn test_has_previous_initially_false() {
    let playback = DesktopPlayback::new(PlaybackConfig::default()).unwrap();

    playback
        .send_command(PlaybackCommand::AddToQueue(create_track("1", "Track 1")))
        .unwrap();

    std::thread::sleep(Duration::from_millis(50));

    // No history yet
    assert!(
        !playback.has_previous(),
        "Should not have previous initially"
    );
}

#[test]
fn test_has_previous_after_playing_tracks() {
    let playback = DesktopPlayback::new(PlaybackConfig::default()).unwrap();

    // Add two tracks
    playback
        .send_command(PlaybackCommand::AddToQueue(create_track("1", "Track 1")))
        .unwrap();
    playback
        .send_command(PlaybackCommand::AddToQueue(create_track("2", "Track 2")))
        .unwrap();

    std::thread::sleep(Duration::from_millis(50));

    // Play first, then next
    playback.send_command(PlaybackCommand::Next).unwrap();
    std::thread::sleep(Duration::from_millis(50));
    drain_events(&playback);

    // Should have previous now (Track 1 in history)
    assert!(
        playback.has_previous(),
        "Should have previous after playing tracks"
    );
}

#[test]
fn test_has_previous_with_repeat_one() {
    let mut config = PlaybackConfig::default();
    config.repeat = RepeatMode::One;
    let playback = DesktopPlayback::new(config).unwrap();

    playback
        .send_command(PlaybackCommand::AddToQueue(create_track("1", "Track 1")))
        .unwrap();

    std::thread::sleep(Duration::from_millis(50));

    // With repeat one, always has previous (same track)
    assert!(
        playback.has_previous(),
        "Should have previous with repeat one"
    );
}

// ===== Queue Retrieval Tests =====

#[test]
fn test_get_queue_returns_all_tracks() {
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
    drain_events(&playback);

    let queue = playback.get_queue();

    assert_eq!(queue.len(), 3, "Queue should have 3 tracks");
    assert_eq!(queue[0].id, "1");
    assert_eq!(queue[1].id, "2");
    assert_eq!(queue[2].id, "3");
}

#[test]
fn test_get_queue_order_preserved() {
    let playback = DesktopPlayback::new(PlaybackConfig::default()).unwrap();

    let track_ids = ["a", "b", "c", "d", "e"];

    for id in track_ids {
        playback
            .send_command(PlaybackCommand::AddToQueue(create_track(
                id,
                &format!("Track {}", id),
            )))
            .unwrap();
    }

    std::thread::sleep(Duration::from_millis(100));
    drain_events(&playback);

    let queue = playback.get_queue();

    // Verify order
    for (i, track) in queue.iter().enumerate() {
        assert_eq!(
            track.id, track_ids[i],
            "Track order should be preserved at index {}",
            i
        );
    }
}

#[test]
fn test_get_queue_after_removal() {
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

    // Remove track at index 2
    playback
        .send_command(PlaybackCommand::RemoveFromQueue(2))
        .unwrap();

    std::thread::sleep(Duration::from_millis(50));
    drain_events(&playback);

    let queue = playback.get_queue();

    assert_eq!(queue.len(), 4, "Queue should have 4 tracks after removal");
    // Track 3 should be removed
    assert_eq!(queue[0].id, "1");
    assert_eq!(queue[1].id, "2");
    assert_eq!(queue[2].id, "4"); // Track 3 skipped
    assert_eq!(queue[3].id, "5");
}

#[test]
fn test_get_queue_after_clear() {
    let playback = DesktopPlayback::new(PlaybackConfig::default()).unwrap();

    // Add tracks
    for i in 1..=3 {
        playback
            .send_command(PlaybackCommand::AddToQueue(create_track(
                &i.to_string(),
                &format!("Track {}", i),
            )))
            .unwrap();
    }

    std::thread::sleep(Duration::from_millis(50));

    // Clear
    playback.send_command(PlaybackCommand::ClearQueue).unwrap();

    std::thread::sleep(Duration::from_millis(50));
    drain_events(&playback);

    let queue = playback.get_queue();

    assert_eq!(queue.len(), 0, "Queue should be empty after clear");
}

#[test]
fn test_get_queue_empty_initially() {
    let playback = DesktopPlayback::new(PlaybackConfig::default()).unwrap();

    let queue = playback.get_queue();

    assert_eq!(queue.len(), 0, "Queue should be empty initially");
}

// ===== Queue Navigation State Tests =====

#[test]
fn test_navigation_state_through_queue() {
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

    // Initially: has next, no previous
    assert!(playback.has_next(), "Should have next initially");
    assert!(
        !playback.has_previous(),
        "Should not have previous initially"
    );

    // After first next: has next, has previous
    playback.send_command(PlaybackCommand::Next).unwrap();
    std::thread::sleep(Duration::from_millis(50));
    drain_events(&playback);

    assert!(playback.has_next(), "Should have next after first skip");
    assert!(
        playback.has_previous(),
        "Should have previous after first skip"
    );

    // After second next: has next, has previous
    playback.send_command(PlaybackCommand::Next).unwrap();
    std::thread::sleep(Duration::from_millis(50));
    drain_events(&playback);

    assert!(playback.has_next(), "Should have next after second skip");
    assert!(
        playback.has_previous(),
        "Should have previous after second skip"
    );

    // After third next: no next, has previous
    playback.send_command(PlaybackCommand::Next).unwrap();
    std::thread::sleep(Duration::from_millis(50));
    drain_events(&playback);

    assert!(
        !playback.has_next(),
        "Should not have next after consuming queue"
    );
    assert!(playback.has_previous(), "Should still have previous at end");
}

#[test]
fn test_navigation_state_after_adding_tracks() {
    let playback = DesktopPlayback::new(PlaybackConfig::default()).unwrap();

    // Start with empty queue
    assert!(!playback.has_next());

    // Add one track
    playback
        .send_command(PlaybackCommand::AddToQueue(create_track("1", "Track 1")))
        .unwrap();

    std::thread::sleep(Duration::from_millis(50));
    drain_events(&playback);

    // Should now have next
    assert!(playback.has_next(), "Should have next after adding track");

    // Consume track
    playback.send_command(PlaybackCommand::Next).unwrap();
    std::thread::sleep(Duration::from_millis(50));
    drain_events(&playback);

    assert!(!playback.has_next(), "Should not have next after consuming");

    // Add another track
    playback
        .send_command(PlaybackCommand::AddToQueue(create_track("2", "Track 2")))
        .unwrap();

    std::thread::sleep(Duration::from_millis(50));
    drain_events(&playback);

    // Should have next again
    assert!(
        playback.has_next(),
        "Should have next after adding new track"
    );
}

// ===== Integration Tests with Play Commands =====

#[test]
fn test_queue_state_after_play_command() {
    let playback = DesktopPlayback::new(PlaybackConfig::default()).unwrap();

    // Add track and play
    playback
        .send_command(PlaybackCommand::AddToQueue(create_track("1", "Track 1")))
        .unwrap();
    playback.send_command(PlaybackCommand::Play).unwrap();

    std::thread::sleep(Duration::from_millis(100));
    drain_events(&playback);

    // Queue should still be queryable
    let queue = playback.get_queue();

    // Depending on implementation, queue may have been consumed or not
    // This tests that get_queue doesn't crash during playback
    assert!(queue.len() <= 1, "Queue should have at most 1 track");
}

#[test]
fn test_navigation_state_persists_across_pause_resume() {
    let playback = DesktopPlayback::new(PlaybackConfig::default()).unwrap();

    // Add tracks
    playback
        .send_command(PlaybackCommand::AddToQueue(create_track("1", "Track 1")))
        .unwrap();
    playback
        .send_command(PlaybackCommand::AddToQueue(create_track("2", "Track 2")))
        .unwrap();

    std::thread::sleep(Duration::from_millis(50));

    let has_next_before = playback.has_next();

    // Pause and resume
    playback.send_command(PlaybackCommand::Pause).unwrap();
    std::thread::sleep(Duration::from_millis(20));
    playback.send_command(PlaybackCommand::Play).unwrap();
    std::thread::sleep(Duration::from_millis(20));
    drain_events(&playback);

    let has_next_after = playback.has_next();

    assert_eq!(
        has_next_before, has_next_after,
        "Navigation state should persist across pause/resume"
    );
}

// ===== Stress and Edge Cases =====

#[test]
fn test_rapid_queue_queries() {
    let playback = DesktopPlayback::new(PlaybackConfig::default()).unwrap();

    // Add tracks
    for i in 1..=10 {
        playback
            .send_command(PlaybackCommand::AddToQueue(create_track(
                &i.to_string(),
                &format!("Track {}", i),
            )))
            .unwrap();
    }

    std::thread::sleep(Duration::from_millis(50));

    // Rapidly query queue and navigation state
    for _ in 0..100 {
        let _ = playback.get_queue();
        let _ = playback.has_next();
        let _ = playback.has_previous();
    }

    // Should handle rapid queries without issues
}

#[test]
fn test_queue_query_during_modifications() {
    let playback = DesktopPlayback::new(PlaybackConfig::default()).unwrap();

    // Add tracks while querying
    for i in 1..=10 {
        playback
            .send_command(PlaybackCommand::AddToQueue(create_track(
                &i.to_string(),
                &format!("Track {}", i),
            )))
            .unwrap();

        // Query between adds
        let _ = playback.get_queue();
        let _ = playback.has_next();

        std::thread::sleep(Duration::from_millis(5));
    }

    // Final query should succeed
    let queue = playback.get_queue();
    assert!(!queue.is_empty(), "Queue should have tracks");
}

#[test]
fn test_large_queue_navigation_performance() {
    let playback = DesktopPlayback::new(PlaybackConfig::default()).unwrap();

    // Add 1000 tracks
    for i in 1..=1000 {
        playback
            .send_command(PlaybackCommand::AddToQueue(create_track(
                &i.to_string(),
                &format!("Track {}", i),
            )))
            .unwrap();

        if i % 100 == 0 {
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    std::thread::sleep(Duration::from_millis(200));
    drain_events(&playback);

    let start = std::time::Instant::now();

    // Query should be fast even with large queue
    let queue = playback.get_queue();
    let has_next = playback.has_next();
    let _has_previous = playback.has_previous();

    let elapsed = start.elapsed();

    assert_eq!(queue.len(), 1000, "Queue should have all tracks");
    assert!(has_next, "Should have next with large queue");
    assert!(
        elapsed < Duration::from_millis(100),
        "Large queue query should be fast, took {:?}",
        elapsed
    );
}

#[test]
fn test_concurrent_navigation_and_playback_commands() {
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

    // Mix playback commands with navigation queries
    for _ in 0..20 {
        playback.send_command(PlaybackCommand::Play).unwrap();
        let _ = playback.has_next();

        playback.send_command(PlaybackCommand::Pause).unwrap();
        let _ = playback.get_queue();

        playback.send_command(PlaybackCommand::Next).unwrap();
        let _ = playback.has_previous();

        std::thread::sleep(Duration::from_millis(10));
    }

    drain_events(&playback);

    // Should handle concurrent operations without crashing
}
