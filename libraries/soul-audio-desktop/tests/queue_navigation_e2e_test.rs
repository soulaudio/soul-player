//! Comprehensive E2E tests for queue navigation, rewind, and next/previous logic
//!
//! **Tests cover:**
//! - Rewind/previous logic (< 3 seconds vs > 3 seconds into track)
//! - Next track logic with different queue states
//! - Loop modes: Off, All, One
//! - Shuffle modes: Off, Random, Smart
//! - Edge cases and combinations
//!
//! **IMPORTANT:** All tests use real audio files from test_data/ directory.
//! No mocks are used to ensure accurate testing of timing-sensitive navigation.

use soul_audio_desktop::{DesktopPlayback, PlaybackCommand, PlaybackEvent};
use soul_playback::{
    PlaybackConfig, PlaybackState, QueueTrack, RepeatMode, ShuffleMode, TrackSource,
};
use std::path::PathBuf;
use std::time::Duration;

// ===== Test Helpers =====

/// Create a test track with real audio file path
fn create_test_track(id: &str) -> QueueTrack {
    QueueTrack {
        id: id.to_string().into(),
        path: PathBuf::from(format!(
            "test_data/track_{}.wav",
            if id == "1" { "1" } else { "2" }
        )),
        title: format!("Test Track {}", id),
        artist: "Test Artist".to_string(),
        album: Some("Test Album".to_string()),
        duration: Duration::from_secs(10), // Track 1 is 10s, Track 2 is 30s
        track_number: Some(id.parse().unwrap_or(1)),
        source: TrackSource::Single,
    }
}

/// Drain all events from the playback system
fn drain_events(playback: &DesktopPlayback) -> Vec<PlaybackEvent> {
    std::iter::from_fn(|| playback.try_recv_event()).collect()
}

/// Get the latest StateChanged event
fn get_latest_state(events: &[PlaybackEvent]) -> Option<PlaybackState> {
    events.iter().rev().find_map(|e| {
        if let PlaybackEvent::StateChanged(state) = e {
            Some(*state)
        } else {
            None
        }
    })
}

/// Get the latest TrackChanged event
fn get_latest_track(events: &[PlaybackEvent]) -> Option<String> {
    events.iter().rev().find_map(|e| {
        if let PlaybackEvent::TrackChanged(Some(track)) = e {
            Some(track.id.to_string())
        } else {
            None
        }
    })
}

// ===== Rewind/Previous Logic Tests =====

#[test]
#[ignore = "Requires real audio hardware - not available in CI environments"]
fn test_previous_within_3_seconds_goes_to_previous_track() {
    // **SCENARIO:** User presses "previous" within first 3 seconds of track
    // **EXPECTED:** Should go to previous track, not restart current track

    let playback =
        DesktopPlayback::new(PlaybackConfig::default()).expect("Failed to create playback");

    println!("\n[TEST] Testing previous() within 3 seconds goes to previous track");

    // Load 3 tracks
    let tracks = vec![
        create_test_track("1"),
        create_test_track("2"),
        create_test_track("3"),
    ];
    playback
        .send_command(PlaybackCommand::LoadPlaylist {
            tracks: tracks,
            start_index: 0,
        })
        .unwrap();
    playback.send_command(PlaybackCommand::Play).unwrap();

    // Let first track start playing
    std::thread::sleep(Duration::from_millis(150));
    drain_events(&playback);

    // Skip to track 2
    playback.send_command(PlaybackCommand::Next).unwrap();
    std::thread::sleep(Duration::from_millis(150));
    drain_events(&playback);

    // Now we're on track 2, less than 3 seconds in
    // Press previous - should go back to track 1
    playback.send_command(PlaybackCommand::Previous).unwrap();
    std::thread::sleep(Duration::from_millis(150));

    let events = drain_events(&playback);
    let track = get_latest_track(&events);

    println!("[TEST] Current track after previous: {:?}", track);

    // Should be back on track 1
    assert_eq!(
        track.as_deref(),
        Some("1"),
        "Should go to previous track (1) when pressing previous within 3 seconds"
    );
}

#[test]
#[ignore = "Requires real audio hardware - not available in CI environments"]
fn test_previous_after_3_seconds_restarts_current_track() {
    // **SCENARIO:** User presses "previous" after 3+ seconds into track
    // **EXPECTED:** Should restart current track, not go to previous

    let playback =
        DesktopPlayback::new(PlaybackConfig::default()).expect("Failed to create playback");

    println!("\n[TEST] Testing previous() after 3 seconds restarts current track");

    let tracks = vec![create_test_track("1"), create_test_track("2")];
    playback
        .send_command(PlaybackCommand::LoadPlaylist {
            tracks: tracks,
            start_index: 0,
        })
        .unwrap();
    playback.send_command(PlaybackCommand::Play).unwrap();

    // Let first track play for over 3 seconds
    std::thread::sleep(Duration::from_millis(3500));
    drain_events(&playback);

    // Press previous - should restart current track (track 1)
    playback.send_command(PlaybackCommand::Previous).unwrap();
    std::thread::sleep(Duration::from_millis(150));

    let events = drain_events(&playback);
    let track = get_latest_track(&events);

    println!("[TEST] Current track after previous: {:?}", track);

    // Should still be on track 1 (restarted)
    // Note: TrackChanged event may not fire for restart, so check state
    let state = get_latest_state(&events);
    assert!(
        matches!(state, Some(PlaybackState::Playing)),
        "Should be playing/loading after restart"
    );
}

#[test]
#[ignore = "Requires real audio hardware - not available in CI environments"]
fn test_previous_at_beginning_of_queue() {
    // **SCENARIO:** User presses "previous" on the first track (no history)
    // **EXPECTED:** Should restart current track

    let playback =
        DesktopPlayback::new(PlaybackConfig::default()).expect("Failed to create playback");

    println!("\n[TEST] Testing previous() at beginning of queue");

    let tracks = vec![create_test_track("1"), create_test_track("2")];
    playback
        .send_command(PlaybackCommand::LoadPlaylist {
            tracks: tracks,
            start_index: 0,
        })
        .unwrap();
    playback.send_command(PlaybackCommand::Play).unwrap();

    std::thread::sleep(Duration::from_millis(150));
    drain_events(&playback);

    // Press previous while on first track - should restart
    playback.send_command(PlaybackCommand::Previous).unwrap();
    std::thread::sleep(Duration::from_millis(150));

    let events = drain_events(&playback);
    let state = get_latest_state(&events);

    println!("[TEST] State after previous at start: {:?}", state);

    // Should be playing (restarted first track)
    assert!(
        matches!(state, Some(PlaybackState::Playing)),
        "Should restart first track when pressing previous at queue start"
    );
}

#[test]
#[ignore = "Requires real audio hardware - not available in CI environments"]
fn test_rapid_previous_presses() {
    // **SCENARIO:** User rapidly presses "previous" multiple times
    // **EXPECTED:** Should navigate backwards through history reliably

    let playback =
        DesktopPlayback::new(PlaybackConfig::default()).expect("Failed to create playback");

    println!("\n[TEST] Testing rapid previous() presses");

    let tracks = vec![
        create_test_track("1"),
        create_test_track("2"),
        create_test_track("3"),
        create_test_track("4"),
    ];
    playback
        .send_command(PlaybackCommand::LoadPlaylist {
            tracks: tracks,
            start_index: 0,
        })
        .unwrap();
    playback.send_command(PlaybackCommand::Play).unwrap();

    std::thread::sleep(Duration::from_millis(150));

    // Skip through all tracks
    for _ in 0..3 {
        playback.send_command(PlaybackCommand::Next).unwrap();
        std::thread::sleep(Duration::from_millis(100));
    }

    drain_events(&playback);
    println!("[TEST] Now on track 4, pressing previous rapidly...");

    // Rapidly press previous 3 times (should go: 4 -> 3 -> 2 -> 1)
    for _ in 0..3 {
        playback.send_command(PlaybackCommand::Previous).unwrap();
        std::thread::sleep(Duration::from_millis(50)); // Short delay to stay under 3 seconds
    }

    std::thread::sleep(Duration::from_millis(150));
    let events = drain_events(&playback);
    let track = get_latest_track(&events);

    println!("[TEST] Track after 3x previous: {:?}", track);

    // Should be on track 1
    assert_eq!(
        track.as_deref(),
        Some("1"),
        "Should navigate back to track 1 after 3x previous"
    );
}

// ===== Next Track Logic Tests =====

#[test]
#[ignore = "Requires real audio hardware - not available in CI environments"]
fn test_next_advances_through_queue() {
    // **SCENARIO:** User presses "next" to advance through queue
    // **EXPECTED:** Should advance through all tracks in order

    let playback =
        DesktopPlayback::new(PlaybackConfig::default()).expect("Failed to create playback");

    println!("\n[TEST] Testing next() advances through queue");

    let tracks = vec![
        create_test_track("1"),
        create_test_track("2"),
        create_test_track("3"),
    ];
    playback
        .send_command(PlaybackCommand::LoadPlaylist {
            tracks: tracks,
            start_index: 0,
        })
        .unwrap();
    playback.send_command(PlaybackCommand::Play).unwrap();

    std::thread::sleep(Duration::from_millis(150));
    drain_events(&playback);

    // Track progression: 1 -> 2 -> 3
    for expected_track in ["2", "3"] {
        playback.send_command(PlaybackCommand::Next).unwrap();
        std::thread::sleep(Duration::from_millis(150));

        let events = drain_events(&playback);
        let track = get_latest_track(&events);

        println!("[TEST] After next, track: {:?}", track);
        assert_eq!(
            track.as_deref(),
            Some(expected_track),
            "Should advance to track {}",
            expected_track
        );
    }
}

#[test]
#[ignore = "Requires real audio hardware - not available in CI environments"]
fn test_next_at_end_of_queue_stops() {
    // **SCENARIO:** User presses "next" at end of queue (no repeat)
    // **EXPECTED:** Should stop playback

    let playback =
        DesktopPlayback::new(PlaybackConfig::default()).expect("Failed to create playback");

    println!("\n[TEST] Testing next() at end of queue stops");

    let tracks = vec![create_test_track("1"), create_test_track("2")];
    playback
        .send_command(PlaybackCommand::LoadPlaylist {
            tracks: tracks,
            start_index: 0,
        })
        .unwrap();
    playback.send_command(PlaybackCommand::Play).unwrap();

    std::thread::sleep(Duration::from_millis(150));

    // Skip to last track
    playback.send_command(PlaybackCommand::Next).unwrap();
    std::thread::sleep(Duration::from_millis(150));
    drain_events(&playback);

    // Try to go next from last track
    playback.send_command(PlaybackCommand::Next).unwrap();
    std::thread::sleep(Duration::from_millis(150));

    let events = drain_events(&playback);
    let state = get_latest_state(&events);

    println!("[TEST] State after next at end: {:?}", state);

    // Should stop when queue ends
    assert_eq!(
        state,
        Some(PlaybackState::Stopped),
        "Should stop when reaching end of queue"
    );
}

#[test]
#[ignore = "Requires real audio hardware - not available in CI environments"]
fn test_rapid_next_presses() {
    // **SCENARIO:** User rapidly presses "next" multiple times
    // **EXPECTED:** Should handle all presses without skipping tracks or crashing

    let playback =
        DesktopPlayback::new(PlaybackConfig::default()).expect("Failed to create playback");

    println!("\n[TEST] Testing rapid next() presses");

    let tracks = vec![
        create_test_track("1"),
        create_test_track("2"),
        create_test_track("3"),
        create_test_track("4"),
        create_test_track("5"),
    ];
    playback
        .send_command(PlaybackCommand::LoadPlaylist {
            tracks: tracks,
            start_index: 0,
        })
        .unwrap();
    playback.send_command(PlaybackCommand::Play).unwrap();

    std::thread::sleep(Duration::from_millis(150));
    drain_events(&playback);

    // Rapidly press next 10 times (more than queue size)
    for _ in 0..10 {
        playback.send_command(PlaybackCommand::Next).unwrap();
        std::thread::sleep(Duration::from_millis(20));
    }

    std::thread::sleep(Duration::from_millis(200));
    let events = drain_events(&playback);
    let state = get_latest_state(&events);

    println!("[TEST] State after 10x rapid next: {:?}", state);

    // Should either be stopped (queue exhausted) or playing last track
    assert!(
        matches!(
            state,
            Some(PlaybackState::Stopped) | Some(PlaybackState::Playing)
        ),
        "Should handle rapid next presses gracefully"
    );
}

// ===== Loop Mode: Off Tests =====

#[test]
#[ignore = "Requires real audio hardware - not available in CI environments"]
fn test_loop_off_stops_at_end() {
    // **SCENARIO:** Queue ends with loop mode off
    // **EXPECTED:** Playback should stop

    let mut config = PlaybackConfig::default();
    config.repeat = RepeatMode::Off;
    let playback = DesktopPlayback::new(config).expect("Failed to create playback");

    println!("\n[TEST] Testing loop off stops at end");

    let tracks = vec![create_test_track("1"), create_test_track("2")];
    playback
        .send_command(PlaybackCommand::LoadPlaylist {
            tracks: tracks,
            start_index: 0,
        })
        .unwrap();
    playback.send_command(PlaybackCommand::Play).unwrap();

    std::thread::sleep(Duration::from_millis(150));

    // Skip to end of queue
    playback.send_command(PlaybackCommand::Next).unwrap();
    std::thread::sleep(Duration::from_millis(150));
    playback.send_command(PlaybackCommand::Next).unwrap();
    std::thread::sleep(Duration::from_millis(150));

    let events = drain_events(&playback);
    let state = get_latest_state(&events);

    println!("[TEST] State at queue end (loop off): {:?}", state);

    assert_eq!(
        state,
        Some(PlaybackState::Stopped),
        "Should stop at end with loop off"
    );
}

// ===== Loop Mode: All Tests =====

#[test]
#[ignore = "Requires real audio hardware - not available in CI environments"]
fn test_loop_all_wraps_to_beginning() {
    // **SCENARIO:** Queue ends with loop mode all
    // **EXPECTED:** Should wrap to first track

    let mut config = PlaybackConfig::default();
    config.repeat = RepeatMode::All;
    let playback = DesktopPlayback::new(config).expect("Failed to create playback");

    println!("\n[TEST] Testing loop all wraps to beginning");

    let tracks = vec![create_test_track("1"), create_test_track("2")];
    playback
        .send_command(PlaybackCommand::LoadPlaylist {
            tracks: tracks,
            start_index: 0,
        })
        .unwrap();
    playback.send_command(PlaybackCommand::Play).unwrap();

    std::thread::sleep(Duration::from_millis(150));

    // Skip to end and beyond
    playback.send_command(PlaybackCommand::Next).unwrap();
    std::thread::sleep(Duration::from_millis(150));
    playback.send_command(PlaybackCommand::Next).unwrap();
    std::thread::sleep(Duration::from_millis(150));

    let events = drain_events(&playback);
    let track = get_latest_track(&events);
    let state = get_latest_state(&events);

    println!(
        "[TEST] Track at queue end (loop all): {:?}, State: {:?}",
        track, state
    );

    // Should wrap to track 1
    assert_eq!(
        track.as_deref(),
        Some("1"),
        "Should wrap to first track with loop all"
    );
    assert!(
        matches!(state, Some(PlaybackState::Playing)),
        "Should be playing after wrapping"
    );
}

#[test]
#[ignore = "Requires real audio hardware - not available in CI environments"]
fn test_loop_all_multiple_cycles() {
    // **SCENARIO:** Loop through queue multiple times
    // **EXPECTED:** Should cycle indefinitely

    let mut config = PlaybackConfig::default();
    config.repeat = RepeatMode::All;
    let playback = DesktopPlayback::new(config).expect("Failed to create playback");

    println!("\n[TEST] Testing loop all multiple cycles");

    let tracks = vec![create_test_track("1"), create_test_track("2")];
    playback
        .send_command(PlaybackCommand::LoadPlaylist {
            tracks: tracks,
            start_index: 0,
        })
        .unwrap();
    playback.send_command(PlaybackCommand::Play).unwrap();

    std::thread::sleep(Duration::from_millis(150));

    // Go through 2 full cycles (4 next presses for 2-track queue)
    for i in 0..4 {
        playback.send_command(PlaybackCommand::Next).unwrap();
        std::thread::sleep(Duration::from_millis(100));

        let events = drain_events(&playback);
        let track = get_latest_track(&events);
        let expected = if i % 2 == 0 { "2" } else { "1" };

        println!("[TEST] Cycle iteration {}: track {:?}", i, track);

        assert_eq!(
            track.as_deref(),
            Some(expected),
            "Should cycle correctly at iteration {}",
            i
        );
    }
}

// ===== Loop Mode: One Tests =====

#[test]
#[ignore = "Requires real audio hardware - not available in CI environments"]
fn test_loop_one_repeats_current_track() {
    // **SCENARIO:** Next pressed with loop one enabled
    // **EXPECTED:** Should restart same track

    let mut config = PlaybackConfig::default();
    config.repeat = RepeatMode::One;
    let playback = DesktopPlayback::new(config).expect("Failed to create playback");

    println!("\n[TEST] Testing loop one repeats current track");

    let tracks = vec![create_test_track("1"), create_test_track("2")];
    playback
        .send_command(PlaybackCommand::LoadPlaylist {
            tracks: tracks,
            start_index: 0,
        })
        .unwrap();
    playback.send_command(PlaybackCommand::Play).unwrap();

    std::thread::sleep(Duration::from_millis(150));
    drain_events(&playback);

    // Press next multiple times - should stay on track 1
    for i in 0..3 {
        playback.send_command(PlaybackCommand::Next).unwrap();
        std::thread::sleep(Duration::from_millis(150));

        let events = drain_events(&playback);
        let state = get_latest_state(&events);

        println!("[TEST] Iteration {}: state {:?}", i, state);

        // Should still be playing (restarting track 1)
        assert!(
            matches!(state, Some(PlaybackState::Playing)),
            "Should keep playing with loop one at iteration {}",
            i
        );
    }
}

#[test]
#[ignore = "Requires real audio hardware - not available in CI environments"]
fn test_loop_one_with_previous() {
    // **SCENARIO:** Previous pressed with loop one enabled
    // **EXPECTED:** Should restart same track (regardless of position)

    let mut config = PlaybackConfig::default();
    config.repeat = RepeatMode::One;
    let playback = DesktopPlayback::new(config).expect("Failed to create playback");

    println!("\n[TEST] Testing loop one with previous()");

    let tracks = vec![create_test_track("1"), create_test_track("2")];
    playback
        .send_command(PlaybackCommand::LoadPlaylist {
            tracks: tracks,
            start_index: 0,
        })
        .unwrap();
    playback.send_command(PlaybackCommand::Play).unwrap();

    // Let track play past 3 seconds
    std::thread::sleep(Duration::from_millis(3500));
    drain_events(&playback);

    // Press previous - with loop one, should restart current track
    playback.send_command(PlaybackCommand::Previous).unwrap();
    std::thread::sleep(Duration::from_millis(150));

    let events = drain_events(&playback);
    let state = get_latest_state(&events);

    println!("[TEST] State after previous (loop one): {:?}", state);

    // Should be playing (restarted)
    assert!(
        matches!(state, Some(PlaybackState::Playing)),
        "Should restart track with loop one + previous"
    );
}

// ===== Shuffle Mode Tests =====

#[test]
#[ignore = "Requires real audio hardware - not available in CI environments"]
fn test_shuffle_off_maintains_order() {
    // **SCENARIO:** Play through queue with shuffle off
    // **EXPECTED:** Tracks should play in original order

    let mut config = PlaybackConfig::default();
    config.shuffle = ShuffleMode::Off;
    let playback = DesktopPlayback::new(config).expect("Failed to create playback");

    println!("\n[TEST] Testing shuffle off maintains order");

    let tracks = vec![
        create_test_track("1"),
        create_test_track("2"),
        create_test_track("3"),
    ];
    playback
        .send_command(PlaybackCommand::LoadPlaylist {
            tracks: tracks,
            start_index: 0,
        })
        .unwrap();
    playback.send_command(PlaybackCommand::Play).unwrap();

    std::thread::sleep(Duration::from_millis(150));
    drain_events(&playback);

    let expected_order = ["2", "3"];
    for expected in expected_order {
        playback.send_command(PlaybackCommand::Next).unwrap();
        std::thread::sleep(Duration::from_millis(150));

        let events = drain_events(&playback);
        let track = get_latest_track(&events);

        println!("[TEST] Track with shuffle off: {:?}", track);
        assert_eq!(
            track.as_deref(),
            Some(expected),
            "Should play track {} in order",
            expected
        );
    }
}

#[test]
#[ignore = "Requires real audio hardware - not available in CI environments"]
fn test_shuffle_random_does_not_crash() {
    // **SCENARIO:** Play with shuffle random enabled
    // **EXPECTED:** Should play tracks without crashing (order may vary)

    let mut config = PlaybackConfig::default();
    config.shuffle = ShuffleMode::Random;
    let playback = DesktopPlayback::new(config).expect("Failed to create playback");

    println!("\n[TEST] Testing shuffle random doesn't crash");

    let tracks = vec![
        create_test_track("1"),
        create_test_track("2"),
        create_test_track("3"),
        create_test_track("4"),
        create_test_track("5"),
    ];
    playback
        .send_command(PlaybackCommand::LoadPlaylist {
            tracks: tracks,
            start_index: 0,
        })
        .unwrap();
    playback.send_command(PlaybackCommand::Play).unwrap();

    std::thread::sleep(Duration::from_millis(150));

    // Skip through several tracks
    for _ in 0..3 {
        playback.send_command(PlaybackCommand::Next).unwrap();
        std::thread::sleep(Duration::from_millis(100));
    }

    let events = drain_events(&playback);
    let state = get_latest_state(&events);

    println!("[TEST] State with shuffle random: {:?}", state);

    // Should be playing (order doesn't matter, just shouldn't crash)
    assert!(
        matches!(state, Some(PlaybackState::Playing)),
        "Should play with shuffle random without crashing"
    );
}

#[test]
#[ignore = "Requires real audio hardware - not available in CI environments"]
fn test_shuffle_random_with_loop_all() {
    // **SCENARIO:** Shuffle random + loop all
    // **EXPECTED:** Should cycle through shuffled queue indefinitely

    let mut config = PlaybackConfig::default();
    config.shuffle = ShuffleMode::Random;
    config.repeat = RepeatMode::All;
    let playback = DesktopPlayback::new(config).expect("Failed to create playback");

    println!("\n[TEST] Testing shuffle random with loop all");

    let tracks = vec![
        create_test_track("1"),
        create_test_track("2"),
        create_test_track("3"),
    ];
    playback
        .send_command(PlaybackCommand::LoadPlaylist {
            tracks: tracks,
            start_index: 0,
        })
        .unwrap();
    playback.send_command(PlaybackCommand::Play).unwrap();

    std::thread::sleep(Duration::from_millis(150));

    // Go through more tracks than queue size
    for i in 0..6 {
        playback.send_command(PlaybackCommand::Next).unwrap();
        std::thread::sleep(Duration::from_millis(100));

        let events = drain_events(&playback);
        let state = get_latest_state(&events);

        println!("[TEST] Iteration {}: state {:?}", i, state);

        // Should keep playing
        assert!(
            matches!(state, Some(PlaybackState::Playing)),
            "Should keep playing with shuffle + loop all"
        );
    }
}

// ===== Edge Cases & Combinations =====

#[test]
#[ignore = "Requires real audio hardware - not available in CI environments"]
fn test_previous_then_next_restores_position() {
    // **SCENARIO:** Go next, then previous, then next again
    // **EXPECTED:** Should navigate correctly through history and queue

    let playback =
        DesktopPlayback::new(PlaybackConfig::default()).expect("Failed to create playback");

    println!("\n[TEST] Testing previous then next navigation");

    let tracks = vec![
        create_test_track("1"),
        create_test_track("2"),
        create_test_track("3"),
    ];
    playback
        .send_command(PlaybackCommand::LoadPlaylist {
            tracks: tracks,
            start_index: 0,
        })
        .unwrap();
    playback.send_command(PlaybackCommand::Play).unwrap();

    std::thread::sleep(Duration::from_millis(150));

    // 1 -> 2
    playback.send_command(PlaybackCommand::Next).unwrap();
    std::thread::sleep(Duration::from_millis(150));
    drain_events(&playback);

    // 2 -> 1 (previous)
    playback.send_command(PlaybackCommand::Previous).unwrap();
    std::thread::sleep(Duration::from_millis(150));
    let events = drain_events(&playback);
    let track = get_latest_track(&events);

    println!("[TEST] After previous: {:?}", track);
    assert_eq!(track.as_deref(), Some("1"), "Should go back to track 1");

    // 1 -> 2 (next again)
    playback.send_command(PlaybackCommand::Next).unwrap();
    std::thread::sleep(Duration::from_millis(150));
    let events = drain_events(&playback);
    let track = get_latest_track(&events);

    println!("[TEST] After next again: {:?}", track);
    assert_eq!(
        track.as_deref(),
        Some("2"),
        "Should advance to track 2 again"
    );
}

#[test]
#[ignore = "Requires real audio hardware - not available in CI environments"]
fn test_mixed_next_previous_navigation() {
    // **SCENARIO:** Complex navigation pattern: next, next, previous, next, previous, previous
    // **EXPECTED:** Should handle all navigation correctly

    let playback =
        DesktopPlayback::new(PlaybackConfig::default()).expect("Failed to create playback");

    println!("\n[TEST] Testing mixed next/previous navigation");

    let tracks = vec![
        create_test_track("1"),
        create_test_track("2"),
        create_test_track("3"),
        create_test_track("4"),
    ];
    playback
        .send_command(PlaybackCommand::LoadPlaylist {
            tracks: tracks,
            start_index: 0,
        })
        .unwrap();
    playback.send_command(PlaybackCommand::Play).unwrap();

    std::thread::sleep(Duration::from_millis(150));
    drain_events(&playback);

    // Pattern: N, N, P, N, P, P
    let commands = [
        (PlaybackCommand::Next, "2"),     // 1 -> 2
        (PlaybackCommand::Next, "3"),     // 2 -> 3
        (PlaybackCommand::Previous, "2"), // 3 -> 2
        (PlaybackCommand::Next, "3"),     // 2 -> 3
        (PlaybackCommand::Previous, "2"), // 3 -> 2
        (PlaybackCommand::Previous, "1"), // 2 -> 1
    ];

    for (i, (cmd, expected)) in commands.iter().enumerate() {
        playback.send_command(cmd.clone()).unwrap();
        std::thread::sleep(Duration::from_millis(100));

        let events = drain_events(&playback);
        let track = get_latest_track(&events);

        println!("[TEST] Step {}: expected {}, got {:?}", i, expected, track);

        assert_eq!(
            track.as_deref(),
            Some(*expected),
            "Step {}: should be on track {}",
            i,
            expected
        );
    }
}

#[test]
#[ignore = "Requires real audio hardware - not available in CI environments"]
fn test_loop_one_does_not_affect_manual_navigation() {
    // **SCENARIO:** Loop one enabled, but user manually navigates with next/previous
    // **EXPECTED:** Manual navigation should work (loop one only affects auto-advance)

    let mut config = PlaybackConfig::default();
    config.repeat = RepeatMode::One;
    let playback = DesktopPlayback::new(config).expect("Failed to create playback");

    println!("\n[TEST] Testing manual navigation with loop one");

    let tracks = vec![
        create_test_track("1"),
        create_test_track("2"),
        create_test_track("3"),
    ];
    playback
        .send_command(PlaybackCommand::LoadPlaylist {
            tracks: tracks,
            start_index: 0,
        })
        .unwrap();
    playback.send_command(PlaybackCommand::Play).unwrap();

    std::thread::sleep(Duration::from_millis(150));
    drain_events(&playback);

    // With loop one, next should still restart current track
    playback.send_command(PlaybackCommand::Next).unwrap();
    std::thread::sleep(Duration::from_millis(150));

    let events = drain_events(&playback);
    let state = get_latest_state(&events);

    println!("[TEST] State after next with loop one: {:?}", state);

    // Should be playing (loop one behavior)
    assert!(
        matches!(state, Some(PlaybackState::Playing)),
        "Loop one should restart current track on next"
    );
}

#[test]
#[ignore = "Requires real audio hardware - not available in CI environments"]
fn test_empty_queue_navigation() {
    // **SCENARIO:** Try next/previous on empty queue
    // **EXPECTED:** Should handle gracefully without crashing

    let playback =
        DesktopPlayback::new(PlaybackConfig::default()).expect("Failed to create playback");

    println!("\n[TEST] Testing navigation on empty queue");

    // Try next and previous with empty queue
    playback.send_command(PlaybackCommand::Next).unwrap();
    playback.send_command(PlaybackCommand::Previous).unwrap();

    std::thread::sleep(Duration::from_millis(100));

    let events = drain_events(&playback);
    let state = get_latest_state(&events);

    println!("[TEST] State after empty queue navigation: {:?}", state);

    // Should handle gracefully (likely stays stopped)
    assert!(
        state.is_none() || matches!(state, Some(PlaybackState::Stopped)),
        "Should handle empty queue navigation gracefully"
    );
}

#[test]
#[ignore = "Requires real audio hardware - not available in CI environments"]
fn test_single_track_queue_with_loop_off() {
    // **SCENARIO:** Single track, loop off, press next
    // **EXPECTED:** Should stop after track ends

    let mut config = PlaybackConfig::default();
    config.repeat = RepeatMode::Off;
    let playback = DesktopPlayback::new(config).expect("Failed to create playback");

    println!("\n[TEST] Testing single track with loop off");

    let tracks = vec![create_test_track("1")];
    playback
        .send_command(PlaybackCommand::LoadPlaylist {
            tracks: tracks,
            start_index: 0,
        })
        .unwrap();
    playback.send_command(PlaybackCommand::Play).unwrap();

    std::thread::sleep(Duration::from_millis(150));
    drain_events(&playback);

    // Press next on single track
    playback.send_command(PlaybackCommand::Next).unwrap();
    std::thread::sleep(Duration::from_millis(150));

    let events = drain_events(&playback);
    let state = get_latest_state(&events);

    println!("[TEST] State after next on single track: {:?}", state);

    // Should stop (no more tracks)
    assert_eq!(
        state,
        Some(PlaybackState::Stopped),
        "Should stop after single track with loop off"
    );
}

#[test]
#[ignore = "Requires real audio hardware - not available in CI environments"]
fn test_single_track_queue_with_loop_all() {
    // **SCENARIO:** Single track, loop all, press next
    // **EXPECTED:** Should restart the same track

    let mut config = PlaybackConfig::default();
    config.repeat = RepeatMode::All;
    let playback = DesktopPlayback::new(config).expect("Failed to create playback");

    println!("\n[TEST] Testing single track with loop all");

    let tracks = vec![create_test_track("1")];
    playback
        .send_command(PlaybackCommand::LoadPlaylist {
            tracks: tracks,
            start_index: 0,
        })
        .unwrap();
    playback.send_command(PlaybackCommand::Play).unwrap();

    std::thread::sleep(Duration::from_millis(150));
    drain_events(&playback);

    // Press next - should wrap to same track
    playback.send_command(PlaybackCommand::Next).unwrap();
    std::thread::sleep(Duration::from_millis(150));

    let events = drain_events(&playback);
    let track = get_latest_track(&events);
    let state = get_latest_state(&events);

    println!(
        "[TEST] Track/state after next on single track (loop all): {:?} / {:?}",
        track, state
    );

    // Should restart track 1
    assert_eq!(
        track.as_deref(),
        Some("1"),
        "Should restart track 1 with loop all"
    );
    assert!(
        matches!(state, Some(PlaybackState::Playing)),
        "Should be playing after restart"
    );
}

#[test]
#[ignore = "Requires real audio hardware - not available in CI environments"]
fn test_pause_resume_preserves_navigation_state() {
    // **SCENARIO:** Navigate through queue, pause, resume, continue navigating
    // **EXPECTED:** Navigation history should be preserved

    let playback =
        DesktopPlayback::new(PlaybackConfig::default()).expect("Failed to create playback");

    println!("\n[TEST] Testing pause/resume preserves navigation");

    let tracks = vec![
        create_test_track("1"),
        create_test_track("2"),
        create_test_track("3"),
    ];
    playback
        .send_command(PlaybackCommand::LoadPlaylist {
            tracks: tracks,
            start_index: 0,
        })
        .unwrap();
    playback.send_command(PlaybackCommand::Play).unwrap();

    std::thread::sleep(Duration::from_millis(150));

    // Navigate: 1 -> 2
    playback.send_command(PlaybackCommand::Next).unwrap();
    std::thread::sleep(Duration::from_millis(150));
    drain_events(&playback);

    // Pause
    playback.send_command(PlaybackCommand::Pause).unwrap();
    std::thread::sleep(Duration::from_millis(150));

    // Resume
    playback.send_command(PlaybackCommand::Play).unwrap();
    std::thread::sleep(Duration::from_millis(150));
    drain_events(&playback);

    // Go previous - should remember we were on track 2
    playback.send_command(PlaybackCommand::Previous).unwrap();
    std::thread::sleep(Duration::from_millis(150));

    let events = drain_events(&playback);
    let track = get_latest_track(&events);

    println!("[TEST] Track after pause/resume/previous: {:?}", track);

    // Should go back to track 1
    assert_eq!(
        track.as_deref(),
        Some("1"),
        "Should preserve navigation history across pause/resume"
    );
}

#[test]
#[ignore = "Requires real audio hardware - not available in CI environments"]
fn test_rewind_bug_reproduction() {
    // **BUG REPRODUCTION:** Sometimes rewind seems to skip a track
    // **SCENARIO:** Navigate forward, rewind immediately (within 3s), repeat
    // **EXPECTED:** Should navigate backwards correctly, not skip tracks

    let playback =
        DesktopPlayback::new(PlaybackConfig::default()).expect("Failed to create playback");

    println!("\n[TEST] Reproducing rewind skip bug");

    let tracks = vec![
        create_test_track("1"),
        create_test_track("2"),
        create_test_track("3"),
        create_test_track("4"),
    ];
    playback
        .send_command(PlaybackCommand::LoadPlaylist {
            tracks: tracks,
            start_index: 0,
        })
        .unwrap();
    playback.send_command(PlaybackCommand::Play).unwrap();

    std::thread::sleep(Duration::from_millis(150));

    // Pattern that might trigger bug: Next -> immediate Previous -> Next -> immediate Previous
    for cycle in 0..2 {
        println!("[TEST] Cycle {}: Going next...", cycle);
        playback.send_command(PlaybackCommand::Next).unwrap();
        std::thread::sleep(Duration::from_millis(100));

        drain_events(&playback);

        println!("[TEST] Cycle {}: Going previous immediately...", cycle);
        playback.send_command(PlaybackCommand::Previous).unwrap();
        std::thread::sleep(Duration::from_millis(150));

        let events = drain_events(&playback);
        let track = get_latest_track(&events);

        println!(
            "[TEST] Cycle {}: After immediate previous, track: {:?}",
            cycle, track
        );

        // Should be back to original track (1 on first cycle, 2 on second)
        let expected = if cycle == 0 { "1" } else { "2" };
        assert_eq!(
            track.as_deref(),
            Some(expected),
            "Cycle {}: Should not skip tracks during immediate previous",
            cycle
        );
    }
}
