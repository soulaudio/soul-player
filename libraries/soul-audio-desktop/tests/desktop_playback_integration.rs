//! Integration tests for DesktopPlayback
//!
//! These tests verify the complete playback flow including command processing,
//! event emission, and integration with PlaybackManager.

use soul_audio_desktop::{DesktopPlayback, PlaybackCommand, PlaybackEvent};
use soul_playback::{PlaybackConfig, QueueTrack, RepeatMode, ShuffleMode, TrackSource};
use std::path::PathBuf;
use std::time::Duration;

/// Helper to create a test queue track
fn create_test_track(id: &str, title: &str, duration_secs: u64) -> QueueTrack {
    QueueTrack {
        id: id.to_string(),
        path: PathBuf::from(format!("/test/{}.mp3", id)),
        title: title.to_string(),
        artist: "Test Artist".to_string(),
        album: Some("Test Album".to_string()),
        duration: Duration::from_secs(duration_secs),
        track_number: Some(1),
        source: TrackSource::Single,
    }
}

#[test]
fn test_desktop_playback_creation() {
    let config = PlaybackConfig::default();
    let playback = DesktopPlayback::new(config);

    assert!(
        playback.is_ok(),
        "Should create DesktopPlayback: {:?}",
        playback.err()
    );
}

#[test]
fn test_command_sending_does_not_block() {
    let config = PlaybackConfig::default();
    let playback = DesktopPlayback::new(config).unwrap();

    // Send multiple commands rapidly - should not block
    let start = std::time::Instant::now();

    for _ in 0..100 {
        playback.send_command(PlaybackCommand::Play).unwrap();
        playback.send_command(PlaybackCommand::Pause).unwrap();
    }

    let elapsed = start.elapsed();

    // Should complete almost instantly (commands are async)
    assert!(
        elapsed < Duration::from_millis(100),
        "Command sending should not block, took {:?}",
        elapsed
    );
}

#[test]
fn test_event_reception_after_commands() {
    let config = PlaybackConfig::default();
    let playback = DesktopPlayback::new(config).unwrap();

    // Send play command
    playback.send_command(PlaybackCommand::Play).unwrap();

    // Give audio thread time to process
    std::thread::sleep(Duration::from_millis(50));

    // Try to receive event
    let event = playback.try_recv_event();

    // Should receive state changed event (or no event if queue is empty)
    // This tests that event channel is working
    if let Some(event) = event {
        match event {
            PlaybackEvent::StateChanged(_) => {
                // Expected
            }
            PlaybackEvent::Error(_) => {
                // Also acceptable (no audio source set)
            }
            _ => {}
        }
    }
}

#[test]
fn test_volume_command_processing() {
    let config = PlaybackConfig::default();
    let playback = DesktopPlayback::new(config).unwrap();

    // Set volume to 50
    playback
        .send_command(PlaybackCommand::SetVolume(50))
        .unwrap();

    // Give time to process
    std::thread::sleep(Duration::from_millis(50));

    // Check for volume changed event
    let mut found_volume_event = false;
    for _ in 0..10 {
        if let Some(PlaybackEvent::VolumeChanged(vol)) = playback.try_recv_event() {
            assert_eq!(vol, 50, "Volume should be 50");
            found_volume_event = true;
            break;
        }
    }

    assert!(
        found_volume_event,
        "Should receive volume changed event"
    );
}

#[test]
fn test_mute_unmute_commands() {
    let config = PlaybackConfig::default();
    let playback = DesktopPlayback::new(config).unwrap();

    // Mute
    playback.send_command(PlaybackCommand::Mute).unwrap();
    std::thread::sleep(Duration::from_millis(50));

    // Unmute
    playback.send_command(PlaybackCommand::Unmute).unwrap();
    std::thread::sleep(Duration::from_millis(50));

    // Should receive volume changed events
    let events: Vec<_> = std::iter::from_fn(|| playback.try_recv_event())
        .take(10)
        .collect();

    let has_volume_events = events
        .iter()
        .any(|e| matches!(e, PlaybackEvent::VolumeChanged(_)));

    assert!(
        has_volume_events,
        "Should receive volume events from mute/unmute"
    );
}

#[test]
fn test_shuffle_mode_commands() {
    let config = PlaybackConfig::default();
    let playback = DesktopPlayback::new(config).unwrap();

    // Test all shuffle modes
    for mode in [ShuffleMode::Off, ShuffleMode::Random, ShuffleMode::Smart] {
        playback
            .send_command(PlaybackCommand::SetShuffle(mode.clone()))
            .unwrap();
        std::thread::sleep(Duration::from_millis(20));
    }

    // Should complete without errors
}

#[test]
fn test_repeat_mode_commands() {
    let config = PlaybackConfig::default();
    let playback = DesktopPlayback::new(config).unwrap();

    // Test all repeat modes
    for mode in [RepeatMode::Off, RepeatMode::All, RepeatMode::One] {
        playback
            .send_command(PlaybackCommand::SetRepeat(mode))
            .unwrap();
        std::thread::sleep(Duration::from_millis(20));
    }

    // Should complete without errors
}

#[test]
fn test_queue_commands() {
    let config = PlaybackConfig::default();
    let playback = DesktopPlayback::new(config).unwrap();

    // Add track to queue
    let track = create_test_track("test1", "Test Track 1", 180);
    playback
        .send_command(PlaybackCommand::AddToQueue(track))
        .unwrap();

    std::thread::sleep(Duration::from_millis(50));

    // Check for queue updated event
    let events: Vec<_> = std::iter::from_fn(|| playback.try_recv_event())
        .take(10)
        .collect();

    let has_queue_event = events
        .iter()
        .any(|e| matches!(e, PlaybackEvent::QueueUpdated));

    assert!(has_queue_event, "Should receive queue updated event");
}

#[test]
fn test_clear_queue_command() {
    let config = PlaybackConfig::default();
    let playback = DesktopPlayback::new(config).unwrap();

    // Add some tracks
    for i in 1..=3 {
        let track = create_test_track(&format!("test{}", i), &format!("Track {}", i), 180);
        playback
            .send_command(PlaybackCommand::AddToQueue(track))
            .unwrap();
    }

    std::thread::sleep(Duration::from_millis(50));

    // Clear queue
    playback.send_command(PlaybackCommand::ClearQueue).unwrap();

    std::thread::sleep(Duration::from_millis(50));

    // Should receive queue updated events
    let events: Vec<_> = std::iter::from_fn(|| playback.try_recv_event())
        .take(20)
        .collect();

    let queue_events_count = events
        .iter()
        .filter(|e| matches!(e, PlaybackEvent::QueueUpdated))
        .count();

    assert!(
        queue_events_count >= 2,
        "Should receive queue updated events (add + clear)"
    );
}

#[test]
fn test_seek_command() {
    let config = PlaybackConfig::default();
    let playback = DesktopPlayback::new(config).unwrap();

    // Seek to 30 seconds (even without active track, should not crash)
    let result = playback.send_command(PlaybackCommand::Seek(30.0));

    assert!(result.is_ok(), "Seek command should be accepted");
}

#[test]
fn test_playback_control_sequence() {
    let config = PlaybackConfig::default();
    let playback = DesktopPlayback::new(config).unwrap();

    // Simulate typical playback control sequence
    let commands = vec![
        PlaybackCommand::Play,
        PlaybackCommand::Pause,
        PlaybackCommand::Play,
        PlaybackCommand::Next,
        PlaybackCommand::Previous,
        PlaybackCommand::Stop,
    ];

    for command in commands {
        let result = playback.send_command(command);
        assert!(result.is_ok(), "All commands should be accepted");
        std::thread::sleep(Duration::from_millis(10));
    }

    // Drain events
    let _events: Vec<_> = std::iter::from_fn(|| playback.try_recv_event())
        .take(20)
        .collect();

    // If we got here without panic, sequence worked
}

#[test]
fn test_rapid_sequential_commands() {
    let config = PlaybackConfig::default();
    let playback = DesktopPlayback::new(config).unwrap();

    // Send many commands in rapid succession (simulates concurrent UI actions)
    for i in 0..100 {
        playback
            .send_command(PlaybackCommand::SetVolume((i % 100) as u8))
            .unwrap();
    }

    // Give time to process
    std::thread::sleep(Duration::from_millis(200));

    // Should handle rapid commands without errors
    let _events: Vec<_> = std::iter::from_fn(|| playback.try_recv_event())
        .take(100)
        .collect();
}

#[test]
fn test_event_order_preservation() {
    let config = PlaybackConfig::default();
    let playback = DesktopPlayback::new(config).unwrap();

    // Send sequence of volume changes
    for vol in [10, 20, 30, 40, 50] {
        playback
            .send_command(PlaybackCommand::SetVolume(vol))
            .unwrap();
        std::thread::sleep(Duration::from_millis(30));
    }

    // Collect volume changed events
    std::thread::sleep(Duration::from_millis(100));

    let mut volumes = Vec::new();
    while let Some(event) = playback.try_recv_event() {
        if let PlaybackEvent::VolumeChanged(vol) = event {
            volumes.push(vol);
        }
    }

    // Volumes should be in order (or at least increasing if some were dropped)
    if volumes.len() > 1 {
        assert!(
            volumes.windows(2).all(|w| w[0] <= w[1]),
            "Volume events should be in order: {:?}",
            volumes
        );
    }
}

#[test]
fn test_queue_management_workflow() {
    let config = PlaybackConfig::default();
    let playback = DesktopPlayback::new(config).unwrap();

    // Add 5 tracks
    for i in 1..=5 {
        let track = create_test_track(&format!("track{}", i), &format!("Track {}", i), 180);
        playback
            .send_command(PlaybackCommand::AddToQueue(track))
            .unwrap();
        std::thread::sleep(Duration::from_millis(10));
    }

    // Remove track at index 2
    playback
        .send_command(PlaybackCommand::RemoveFromQueue(2))
        .unwrap();
    std::thread::sleep(Duration::from_millis(50));

    // Clear remaining queue
    playback.send_command(PlaybackCommand::ClearQueue).unwrap();
    std::thread::sleep(Duration::from_millis(50));

    // Collect events
    let events: Vec<_> = std::iter::from_fn(|| playback.try_recv_event())
        .take(30)
        .collect();

    // Should have multiple queue updated events
    let queue_event_count = events
        .iter()
        .filter(|e| matches!(e, PlaybackEvent::QueueUpdated))
        .count();

    assert!(
        queue_event_count >= 5,
        "Should have queue events for adds, remove, and clear"
    );
}

#[test]
fn test_rapid_play_pause_toggle() {
    let config = PlaybackConfig::default();
    let playback = DesktopPlayback::new(config).unwrap();

    // Rapidly toggle play/pause
    for _ in 0..50 {
        playback.send_command(PlaybackCommand::Play).unwrap();
        playback.send_command(PlaybackCommand::Pause).unwrap();
    }

    // Give time to process
    std::thread::sleep(Duration::from_millis(200));

    // Should handle without crashing or deadlocking
    // Drain events to verify system is still responsive
    let events: Vec<_> = std::iter::from_fn(|| playback.try_recv_event())
        .take(100)
        .collect();

    // Should have received some state changed events
    let has_state_events = events
        .iter()
        .any(|e| matches!(e, PlaybackEvent::StateChanged(_)));

    assert!(
        has_state_events,
        "Should process state change events even under rapid commands"
    );
}

#[test]
fn test_volume_bounds_enforcement() {
    let config = PlaybackConfig::default();
    let playback = DesktopPlayback::new(config).unwrap();

    // Try to set volume beyond 100
    playback
        .send_command(PlaybackCommand::SetVolume(150))
        .unwrap();

    std::thread::sleep(Duration::from_millis(50));

    // Check that volume was clamped
    if let Some(PlaybackEvent::VolumeChanged(vol)) = playback.recv_event() {
        assert!(vol <= 100, "Volume should be clamped to 100, got {}", vol);
    }
}

#[test]
fn test_no_events_without_commands() {
    let config = PlaybackConfig::default();
    let playback = DesktopPlayback::new(config).unwrap();

    // Don't send any commands
    std::thread::sleep(Duration::from_millis(100));

    // Should not have spurious events
    let event = playback.try_recv_event();

    // Either no event, or only initialization events
    if let Some(event) = event {
        // Should only be initialization-related events if any
        assert!(
            matches!(
                event,
                PlaybackEvent::StateChanged(_) | PlaybackEvent::VolumeChanged(_)
            ),
            "Should only have initialization events"
        );
    }
}

#[test]
fn test_playback_manager_survives_stress() {
    let config = PlaybackConfig::default();
    let playback = DesktopPlayback::new(config).unwrap();

    // Stress test with mixed commands
    use rand::Rng;
    let mut rng = rand::thread_rng();

    for _ in 0..200 {
        let command = match rng.gen_range(0..10) {
            0 => PlaybackCommand::Play,
            1 => PlaybackCommand::Pause,
            2 => PlaybackCommand::Stop,
            3 => PlaybackCommand::Next,
            4 => PlaybackCommand::Previous,
            5 => PlaybackCommand::SetVolume(rng.gen_range(0..=100)),
            6 => PlaybackCommand::Mute,
            7 => PlaybackCommand::Unmute,
            8 => PlaybackCommand::SetShuffle(ShuffleMode::Random),
            _ => PlaybackCommand::SetRepeat(RepeatMode::All),
        };

        playback.send_command(command).unwrap();

        if rng.gen_bool(0.3) {
            std::thread::sleep(Duration::from_micros(100));
        }
    }

    // Give time to process all commands
    std::thread::sleep(Duration::from_millis(200));

    // Drain all events
    while playback.try_recv_event().is_some() {}

    // If we got here, system survived stress test
}

#[test]
fn test_playback_instance_isolation() {
    // Create a playback instance, use it, then drop it
    {
        let config1 = PlaybackConfig {
            volume: 50,
            ..Default::default()
        };
        let playback1 = DesktopPlayback::new(config1).unwrap();
        playback1
            .send_command(PlaybackCommand::SetVolume(50))
            .unwrap();
        std::thread::sleep(Duration::from_millis(50));
    } // Drop first instance

    // Create a second instance - should work independently
    let config2 = PlaybackConfig {
        volume: 80,
        ..Default::default()
    };
    let playback2 = DesktopPlayback::new(config2).unwrap();
    playback2
        .send_command(PlaybackCommand::SetVolume(80))
        .unwrap();
    std::thread::sleep(Duration::from_millis(50));

    // Should work without interference from dropped instance
    let _ = playback2.try_recv_event();
}
