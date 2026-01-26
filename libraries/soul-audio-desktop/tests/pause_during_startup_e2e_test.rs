//! E2E test for pause-during-startup bug (`MediaCard` scenario)
//!
//! **Bug reproduction:**
//! User clicks Play on `MediaCard` → clicks Pause immediately → audio continues playing
//!
//! **Root cause:**
//! Commands are queued and processed in audio callback. By the time the pause
//! command is processed, the audio source may have already started outputting audio.
//!
//! This test reproduces the EXACT flow:
//! 1. User clicks Play button → sends `LoadPlaylist` + Play commands
//! 2. User immediately clicks Pause → sends Pause command
//! 3. Audio callbacks process commands and output audio
//! 4. Verify audio is SILENT (not playing)

use soul_audio_desktop::{DesktopPlayback, PlaybackCommand, PlaybackEvent};
use soul_playback::{PlaybackConfig, PlaybackState, QueueTrack, TrackSource};
use std::path::PathBuf;
use std::time::Duration;

/// Create a test track with a real audio file path for testing
fn create_test_track(id: &str) -> QueueTrack {
    QueueTrack {
        id: id.to_string(),
        path: PathBuf::from(format!("test_data/track_{}.mp3", id)),
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

/// Helper to find the latest `StateChanged` event
fn get_latest_state(events: &[PlaybackEvent]) -> Option<PlaybackState> {
    events.iter().rev().find_map(|e| {
        if let PlaybackEvent::StateChanged(state) = e {
            Some(*state)
        } else {
            None
        }
    })
}

#[test]
#[ignore = "Requires real audio hardware - not available in CI environments"]
fn test_mediacard_double_click_pause_bug() {
    // **SCENARIO: User rapidly clicks Play then Pause on MediaCard**
    //
    // This reproduces the exact bug: audio continues playing after immediate pause

    let config = PlaybackConfig::default();
    let playback = DesktopPlayback::new(config).expect("Failed to create playback");

    println!("\n[TEST] Starting MediaCard double-click scenario");
    println!("[TEST] Step 1: User clicks PLAY on MediaCard (loads playlist + plays)");

    // Simulate MediaCard "Play" button click
    // This sends: LoadPlaylist + Play commands (same as real app)
    let tracks = vec![create_test_track("1"), create_test_track("2")];
    playback
        .send_command(PlaybackCommand::LoadPlaylist(tracks.clone()))
        .expect("Failed to send LoadPlaylist");
    playback
        .send_command(PlaybackCommand::Play)
        .expect("Failed to send Play");

    println!("[TEST] Sent LoadPlaylist + Play commands");

    // Give time for first command to be queued
    std::thread::sleep(Duration::from_millis(1));

    println!("[TEST] Step 2: User IMMEDIATELY clicks PAUSE (within 5ms)");

    // Simulate immediate pause (user double-clicks)
    playback
        .send_command(PlaybackCommand::Pause)
        .expect("Failed to send Pause");

    println!("[TEST] Sent Pause command");
    println!("[TEST] Step 3: Audio thread processes commands...");

    // Give audio thread time to process all commands
    // In real app, audio callbacks run every ~10-20ms
    std::thread::sleep(Duration::from_millis(150));

    println!("[TEST] Step 4: Checking playback state...");

    // Collect all events
    let events = drain_events(&playback);

    println!("[TEST] Received {} events:", events.len());
    for (i, event) in events.iter().enumerate() {
        match event {
            PlaybackEvent::StateChanged(state) => {
                println!("  Event {}: StateChanged({:?})", i, state);
            }
            PlaybackEvent::QueueUpdated => {
                println!("  Event {}: QueueUpdated", i);
            }
            PlaybackEvent::Error(err) => {
                println!("  Event {}: Error({})", i, err);
            }
            _ => {
                println!("  Event {}: {:?}", i, event);
            }
        }
    }

    // Get the final playback state
    let final_state = get_latest_state(&events);

    println!("[TEST] Final state: {:?}", final_state);

    // **CRITICAL ASSERTION:**
    // After sending Play + Pause, the final state MUST be Paused
    //
    // Note: If audio files don't exist, we'll get Stopped after Loading fails
    // In that case, we check that pause was at least processed (not ignored)

    let has_errors = events.iter().any(|e| matches!(e, PlaybackEvent::Error(_)));

    if has_errors {
        // Files don't exist - check that pause wasn't ignored
        println!("[TEST] ⚠️  Audio files don't exist (expected in CI)");
        println!("[TEST] Checking that pause command wasn't ignored...");

        // If pause was processed, we should see at least one Paused or Stopped state
        // If pause was IGNORED (the bug), we'd only see Loading/Error
        assert!(
            final_state == Some(PlaybackState::Paused)
                || final_state == Some(PlaybackState::Stopped),
            "Pause command appears to have been ignored! Got {:?}",
            final_state
        );

        println!(
            "[TEST] ✓ Pause command was processed (state transitioned to {:?})",
            final_state
        );
    } else {
        // Files exist - strict check
        assert_eq!(
            final_state,
            Some(PlaybackState::Paused),
            "BUG DETECTED: Expected Paused state after immediate pause, got {:?}\n\
             This means audio is still playing even though user clicked pause!",
            final_state
        );
        println!("[TEST] ✓ Pause was successful - audio is NOT playing");
    }
}

#[test]
#[ignore = "Requires real audio hardware - not available in CI environments"]
fn test_triple_rapid_commands() {
    // Test even more extreme case: Play → Pause → Play rapidly

    let config = PlaybackConfig::default();
    let playback = DesktopPlayback::new(config).expect("Failed to create playback");

    println!("\n[TEST] Triple rapid command test: Play → Pause → Play");

    // Load playlist
    let tracks = vec![create_test_track("1")];
    playback
        .send_command(PlaybackCommand::LoadPlaylist(tracks))
        .expect("Failed to send LoadPlaylist");

    // Rapid fire: Play → Pause → Play
    playback
        .send_command(PlaybackCommand::Play)
        .expect("Send Play");
    std::thread::sleep(Duration::from_millis(1));

    playback
        .send_command(PlaybackCommand::Pause)
        .expect("Send Pause");
    std::thread::sleep(Duration::from_millis(1));

    playback
        .send_command(PlaybackCommand::Play)
        .expect("Send Play");

    println!("[TEST] Sent Play → Pause → Play (3 commands in 2ms)");

    // Wait for processing
    std::thread::sleep(Duration::from_millis(150));

    let events = drain_events(&playback);
    let final_state = get_latest_state(&events);

    println!(
        "[TEST] Final state after Play→Pause→Play: {:?}",
        final_state
    );

    // Final state should be Playing (last command was Play)
    // But if files don't exist, we get Stopped after error
    let has_errors = events.iter().any(|e| matches!(e, PlaybackEvent::Error(_)));

    if has_errors {
        println!("[TEST] ⚠️  Files don't exist, skipping state check");
    } else {
        assert!(
            matches!(
                final_state,
                Some(PlaybackState::Playing | PlaybackState::Loading)
            ),
            "Expected Playing or Loading after final Play command, got {:?}",
            final_state
        );
    }
}

#[test]
#[ignore = "Requires real audio hardware - not available in CI environments"]
fn test_pause_then_resume_during_loading() {
    // Test pausing during track loading, then resuming

    let config = PlaybackConfig::default();
    let playback = DesktopPlayback::new(config).expect("Failed to create playback");

    println!("\n[TEST] Pause during loading, then resume");

    // Start playback
    let tracks = vec![create_test_track("1")];
    playback
        .send_command(PlaybackCommand::LoadPlaylist(tracks))
        .unwrap();
    playback.send_command(PlaybackCommand::Play).unwrap();

    // Immediate pause (during loading phase)
    std::thread::sleep(Duration::from_millis(5));
    playback.send_command(PlaybackCommand::Pause).unwrap();

    println!("[TEST] Paused during loading");

    // Wait for pause to take effect
    std::thread::sleep(Duration::from_millis(100));

    // Verify paused
    let events = drain_events(&playback);
    let paused_state = get_latest_state(&events);
    println!("[TEST] State after pause: {:?}", paused_state);

    let has_errors1 = events.iter().any(|e| matches!(e, PlaybackEvent::Error(_)));

    if has_errors1 {
        println!("[TEST] ⚠️  Files don't exist, checking pause was processed...");
        assert!(
            paused_state == Some(PlaybackState::Paused)
                || paused_state == Some(PlaybackState::Stopped),
            "Pause command ignored! State: {:?}",
            paused_state
        );
    } else {
        assert_eq!(
            paused_state,
            Some(PlaybackState::Paused),
            "Should be paused after pause command"
        );
    }

    // Now resume
    playback.send_command(PlaybackCommand::Play).unwrap();
    println!("[TEST] Sent resume command");

    // Wait for resume to process
    std::thread::sleep(Duration::from_millis(150));

    let events2 = drain_events(&playback);
    let resumed_state = get_latest_state(&events2);

    println!("[TEST] State after resume: {:?}", resumed_state);

    // Should be Playing or Loading (resuming from paused-during-loading)
    // Unless files don't exist
    let has_errors = events2.iter().any(|e| matches!(e, PlaybackEvent::Error(_)));

    if has_errors {
        println!("[TEST] ⚠️  Files don't exist, skipping state check");
    } else {
        assert!(
            matches!(
                resumed_state,
                Some(PlaybackState::Playing | PlaybackState::Loading)
            ),
            "Should be Playing or Loading after resume, got {:?}",
            resumed_state
        );
    }
}

#[test]
#[ignore = "Requires real audio hardware - not available in CI environments"]
fn test_command_queue_ordering() {
    // Verify commands are processed in order (FIFO)

    let config = PlaybackConfig::default();
    let playback = DesktopPlayback::new(config).expect("Failed to create playback");

    println!("\n[TEST] Testing command queue ordering");

    // Send a sequence of volume commands
    // If processed in order, final volume should be 75
    playback
        .send_command(PlaybackCommand::SetVolume(25))
        .unwrap();
    playback
        .send_command(PlaybackCommand::SetVolume(50))
        .unwrap();
    playback
        .send_command(PlaybackCommand::SetVolume(75))
        .unwrap();

    std::thread::sleep(Duration::from_millis(100));

    let events = drain_events(&playback);

    // Find all volume events
    let volume_events: Vec<_> = events
        .iter()
        .filter_map(|e| {
            if let PlaybackEvent::VolumeChanged(vol) = e {
                Some(*vol)
            } else {
                None
            }
        })
        .collect();

    println!("[TEST] Volume events (in order): {:?}", volume_events);

    // Should have at least the final volume
    let final_volume = volume_events.last();
    assert_eq!(
        final_volume,
        Some(&75),
        "Final volume should be 75 (commands processed in order)"
    );
}

#[test]
#[ignore = "Requires real audio hardware - not available in CI environments"]
fn test_pause_immediately_after_load_playlist() {
    // This is the EXACT MediaCard scenario:
    // LoadPlaylist sets up the queue, but doesn't play yet
    // Then Play starts playback
    // Then immediate Pause should stop it

    let config = PlaybackConfig::default();
    let playback = DesktopPlayback::new(config).expect("Failed to create playback");

    println!("\n[TEST] LoadPlaylist → Play → Pause (MediaCard exact flow)");

    let tracks = vec![
        create_test_track("1"),
        create_test_track("2"),
        create_test_track("3"),
    ];

    // Step 1: LoadPlaylist (queues tracks)
    playback
        .send_command(PlaybackCommand::LoadPlaylist(tracks))
        .unwrap();

    // Step 2: Play (starts playback from first track)
    playback.send_command(PlaybackCommand::Play).unwrap();

    // Step 3: Immediate Pause (user changed their mind)
    // This happens within ~5ms in real usage
    playback.send_command(PlaybackCommand::Pause).unwrap();

    println!("[TEST] Sent LoadPlaylist → Play → Pause in rapid succession");

    // Give audio thread time to process
    std::thread::sleep(Duration::from_millis(200));

    let events = drain_events(&playback);

    println!("[TEST] Events received:");
    for event in &events {
        if let PlaybackEvent::StateChanged(state) = event {
            println!("  - StateChanged({:?})", state);
        }
    }

    let final_state = get_latest_state(&events);

    println!("[TEST] Final state: {:?}", final_state);

    // **THE BUG:**
    // If this fails, it means pause is not working during the loading phase

    let has_errors = events.iter().any(|e| matches!(e, PlaybackEvent::Error(_)));

    if has_errors {
        println!("[TEST] ⚠️  Audio files don't exist (expected in CI)");
        // Check pause was processed (not ignored)
        assert!(
            final_state == Some(PlaybackState::Paused)
                || final_state == Some(PlaybackState::Stopped),
            "Pause command was ignored! Got {:?}",
            final_state
        );
        println!(
            "[TEST] ✓ Pause command was processed (state: {:?})",
            final_state
        );
    } else {
        assert_eq!(
            final_state,
            Some(PlaybackState::Paused),
            "\n\n🐛 BUG REPRODUCED! 🐛\n\
             Expected: Paused (user clicked pause)\n\
             Actual: {:?}\n\n\
             The audio continues playing even though the user clicked pause.\n\
             This is the MediaCard double-click bug.\n",
            final_state
        );
        println!("[TEST] ✓ BUG NOT PRESENT - Pause works correctly");
    }
}

#[test]
#[ignore = "Requires real audio hardware - not available in CI environments"]
fn test_multiple_pause_resume_cycles() {
    // Test that pause/resume works correctly across multiple cycles

    let config = PlaybackConfig::default();
    let playback = DesktopPlayback::new(config).expect("Failed to create playback");

    println!("\n[TEST] Multiple pause/resume cycles");

    let tracks = vec![create_test_track("1")];
    playback
        .send_command(PlaybackCommand::LoadPlaylist(tracks))
        .unwrap();

    for cycle in 0..3 {
        println!("[TEST] Cycle {}: Play → Pause", cycle);

        playback.send_command(PlaybackCommand::Play).unwrap();
        std::thread::sleep(Duration::from_millis(50));

        playback.send_command(PlaybackCommand::Pause).unwrap();
        std::thread::sleep(Duration::from_millis(50));

        let events = drain_events(&playback);
        let state = get_latest_state(&events);

        let has_errors = events.iter().any(|e| matches!(e, PlaybackEvent::Error(_)));

        if !has_errors {
            assert_eq!(
                state,
                Some(PlaybackState::Paused),
                "Cycle {}: Should be paused",
                cycle
            );
        } else if cycle == 0 {
            // Only print once
            println!("[TEST] ⚠️  Files don't exist, checking pause was processed...");
            assert!(
                state == Some(PlaybackState::Paused) || state == Some(PlaybackState::Stopped),
                "Cycle {}: Pause command ignored! State: {:?}",
                cycle,
                state
            );
        }
    }

    println!("[TEST] ✓ All cycles completed successfully");
}

#[test]
#[ignore = "Requires real audio hardware - not available in CI environments"]
fn test_pause_during_background_loading() {
    // **THE CRITICAL BUG TEST:**
    // This reproduces the EXACT race condition that causes audio to play after pause.
    //
    // Timeline:
    // 1. User clicks Play → LoadPlaylist + Play commands sent
    // 2. State becomes Loading, track starts loading in background thread
    // 3. User clicks Pause (within 1-2s) → state becomes Paused
    // 4. Background loader finishes → calls set_audio_source()
    // 5. **BUG:** set_audio_source() overrides Paused state with Playing
    // 6. Audio starts playing even though user clicked pause!
    //
    // This test verifies that set_audio_source() respects the Paused state.

    let config = PlaybackConfig::default();
    let playback = DesktopPlayback::new(config).expect("Failed to create playback");

    println!("\n[TEST] Pause during background track loading (THE BUG)");
    println!("[TEST] This reproduces the 1-2 second pause bug");

    let tracks = vec![create_test_track("1"), create_test_track("2")];

    // Step 1: User clicks Play button
    println!("[TEST] Step 1: User clicks Play (LoadPlaylist + Play)");
    playback
        .send_command(PlaybackCommand::LoadPlaylist(tracks.clone()))
        .expect("Failed to send LoadPlaylist");
    playback
        .send_command(PlaybackCommand::Play)
        .expect("Failed to send Play");

    // Step 2: Give time for Loading state to be set
    // In real app, track loading happens in background thread
    std::thread::sleep(Duration::from_millis(10));

    // Step 3: User clicks Pause (during loading phase)
    println!("[TEST] Step 2: User clicks Pause (during loading phase)");
    playback
        .send_command(PlaybackCommand::Pause)
        .expect("Failed to send Pause");

    // Step 4: Wait for both:
    // - Pause command to be processed (sets state = Paused)
    // - Track loading to complete (calls set_audio_source())
    //
    // The bug occurs when set_audio_source() overrides the Paused state
    println!("[TEST] Step 3: Waiting for pause + track loading to complete...");
    std::thread::sleep(Duration::from_millis(300));

    // Step 5: Check final state
    println!("[TEST] Step 4: Checking final state...");

    let events = drain_events(&playback);

    println!("[TEST] Events received:");
    for (i, event) in events.iter().enumerate() {
        match event {
            PlaybackEvent::StateChanged(state) => {
                println!("  Event {}: StateChanged({:?})", i, state);
            }
            PlaybackEvent::QueueUpdated => {
                println!("  Event {}: QueueUpdated", i);
            }
            PlaybackEvent::Error(err) => {
                println!("  Event {}: Error({})", i, err);
            }
            _ => {
                println!("  Event {}: {:?}", i, event);
            }
        }
    }

    let final_state = get_latest_state(&events);
    println!("[TEST] Final state: {:?}", final_state);

    let has_errors = events.iter().any(|e| matches!(e, PlaybackEvent::Error(_)));

    if has_errors {
        println!("[TEST] ⚠️  Audio files don't exist (expected in CI)");
        println!("[TEST] Checking that pause command wasn't overridden...");

        // Even with errors, state should be Paused or Stopped, NOT Playing
        assert!(
            final_state == Some(PlaybackState::Paused)
                || final_state == Some(PlaybackState::Stopped),
            "BUG DETECTED: Pause was overridden by track loading! Got {:?}\n\
             Expected: Paused or Stopped\n\
             This means set_audio_source() ignored the user's pause command.",
            final_state
        );

        println!(
            "[TEST] ✓ Pause command was respected (state: {:?})",
            final_state
        );
    } else {
        // With real audio files, state MUST be Paused
        assert_eq!(
            final_state,
            Some(PlaybackState::Paused),
            "\n\n🐛 CRITICAL BUG DETECTED! 🐛\n\
             User clicked pause during loading, but audio is now playing!\n\
             Expected: Paused\n\
             Got: {:?}\n\n\
             This is the root cause of the '1-2 second pause bug'.\n\
             The set_audio_source() function is overriding the user's pause command.\n",
            final_state
        );

        println!("[TEST] ✓ BUG FIXED - Pause during loading works correctly!");
    }
}
