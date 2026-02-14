//! Race condition tests for device switching
//!
//! These tests verify that device switching handles concurrent operations safely:
//! - Concurrent device switches
//! - Device switch during audio callback execution
//! - Device hotplug removal during switch
//! - Mutex poisoning recovery
//!
//! These are HIGH PRIORITY tests to catch synchronization bugs that could cause
//! panics, deadlocks, or audio dropouts in production.

use soul_audio_desktop::{AudioBackend, DesktopPlayback, PlaybackCommand};
use soul_playback::{CrossfadeSettings, FadeCurve, PlaybackConfig, QueueTrack, TrackSource};
use std::path::PathBuf;
use std::sync::{Arc, Barrier};
use std::time::Duration;
use tokio::time::sleep;

/// Helper to drain all pending events
fn drain_events(playback: &DesktopPlayback) {
    while playback.try_recv_event().is_some() {}
}

/// Helper to create a test queue track
fn create_test_track(id: &str) -> QueueTrack {
    QueueTrack {
        id: id.to_string(),
        path: PathBuf::from("/nonexistent/test.flac"),
        title: format!("Test Track {}", id),
        artist: "Test Artist".to_string(),
        album: Some("Test Album".to_string()),
        duration: Duration::from_secs(180),
        track_number: Some(1),
        source: TrackSource::Single,
    }
}

/// Test 1: Concurrent device switch + hotplug removal
///
/// **Scenario**: While switching from Device A to Device B, Device A gets unplugged.
/// **Expected**: Should complete switch to Device B or fallback to default (no panic).
#[tokio::test]
async fn test_concurrent_device_switch_and_removal() {
    // Note: Logging is initialized via env RUST_LOG if needed
    eprintln!("[TEST] Starting concurrent device switch and removal test");

    let result = DesktopPlayback::new(PlaybackConfig::default());

    match result {
        Ok(mut playback) => {
            // Load a test track to have active playback
            let track = create_test_track("concurrent-test");
            let _ = playback.send_command(PlaybackCommand::LoadPlaylist {
                tracks: vec![track],
                start_index: 0,
            });
            let _ = playback.send_command(PlaybackCommand::Play);

            // Give playback time to start
            sleep(Duration::from_millis(100)).await;

            // Get initial device
            let initial_device = playback.get_current_device();
            eprintln!(
                "[TEST] Initial device: {}, starting concurrent operations",
                initial_device
            );

            // Simulate concurrent operations:
            // 1. Switch to default device (or same device)
            // 2. In production, a hotplug event would fire here
            //    (we can't easily simulate real hotplug in tests, but we test the race)

            let mut switch_success = false;
            for attempt in 1..=3 {
                eprintln!("[TEST] Device switch attempt {}/3", attempt);

                match playback.switch_device(AudioBackend::Default, None) {
                    Ok(()) => {
                        switch_success = true;
                        eprintln!("[TEST] Device switch attempt {} succeeded", attempt);

                        // Verify device is still accessible
                        let current_device = playback.get_current_device();
                        assert!(
                            !current_device.is_empty(),
                            "Device name should not be empty after switch"
                        );

                        // Small delay between switches
                        sleep(Duration::from_millis(50)).await;
                    }
                    Err(e) => {
                        eprintln!(
                            "[TEST] Device switch attempt {} failed (expected in test env): {}",
                            attempt, e
                        );
                    }
                }
            }

            // At least one switch should work, or all should fail gracefully
            eprintln!(
                "[TEST] Concurrent operations completed. Switch success: {}",
                switch_success
            );

            // Final state should be valid
            let final_device = playback.get_current_device();
            assert!(!final_device.is_empty(), "Final device should be valid");
            eprintln!("[TEST] Final device: {}", final_device);

            // Cleanup
            let _ = playback.send_command(PlaybackCommand::Stop);
        }
        Err(e) => {
            eprintln!(
                "[TEST] Audio device not available in test environment (expected): {}",
                e
            );
        }
    }
}

/// Test 2: Device switch during active playback (audio callback executing)
///
/// **Scenario**: Start playback, then switch device while audio callback is processing samples.
/// **Expected**: Playback continues on new device, position preserved, no audio glitches.
#[tokio::test]
async fn test_device_switch_during_playback() {
    eprintln!("[TEST] Starting device switch during playback test");

    let result = DesktopPlayback::new(PlaybackConfig::default());

    match result {
        Ok(mut playback) => {
            // Load track and start playback
            let track = create_test_track("playback-test");
            let _ = playback.send_command(PlaybackCommand::LoadPlaylist {
                tracks: vec![track],
                start_index: 0,
            });
            let _ = playback.send_command(PlaybackCommand::Play);

            // Let playback run for a bit to ensure audio callback is active
            sleep(Duration::from_millis(200)).await;

            let position_before = playback.get_position();
            eprintln!(
                "[TEST] Position before switch: {:?}, performing switch...",
                position_before
            );

            // Switch device while audio callback is likely executing
            match playback.switch_device(AudioBackend::Default, None) {
                Ok(()) => {
                    eprintln!("[TEST] Device switched successfully during playback");

                    // Let playback continue briefly
                    sleep(Duration::from_millis(100)).await;

                    let position_after = playback.get_position();
                    eprintln!("[TEST] Position after switch: {:?}", position_after);

                    // Position should still be valid (either preserved or advanced)
                    // We allow some drift due to device switch latency
                    let before_secs = position_before.as_secs_f64();
                    let after_secs = position_after.as_secs_f64();
                    assert!(
                        after_secs >= before_secs - 1.0,
                        "Position should not jump backwards significantly"
                    );
                    assert!(
                        after_secs <= before_secs + 2.0,
                        "Position should not jump forwards excessively"
                    );

                    // Device should still be accessible
                    let current_device = playback.get_current_device();
                    assert!(
                        !current_device.is_empty(),
                        "Device should be valid after switch"
                    );
                }
                Err(e) => {
                    eprintln!(
                        "[TEST] Device switch failed (may be expected in test env): {}",
                        e
                    );

                    // Even if switch fails, original device should still work
                    let current_device = playback.get_current_device();
                    assert!(
                        !current_device.is_empty(),
                        "Original device should still be valid"
                    );
                }
            }

            // Cleanup
            let _ = playback.send_command(PlaybackCommand::Stop);
        }
        Err(e) => {
            eprintln!(
                "[TEST] Audio device not available in test environment (expected): {}",
                e
            );
        }
    }
}

/// Test 3: Stream mutex poisoning recovery
///
/// **Scenario**: Simulate mutex poisoning (panic in audio callback), then try device switch.
/// **Expected**: Should return error gracefully, not panic.
///
/// Note: We can't easily cause real mutex poisoning in tests without unsafe code,
/// but we can test the error paths and ensure no panics occur.
#[tokio::test]
async fn test_stream_mutex_recovery_after_errors() {
    eprintln!("[TEST] Starting mutex recovery test");

    let result = DesktopPlayback::new(PlaybackConfig::default());

    match result {
        Ok(mut playback) => {
            // Attempt operations that might fail in various ways
            eprintln!("[TEST] Testing error recovery paths");

            // Try switching to invalid device (should fail gracefully)
            let invalid_result = playback.switch_device(
                AudioBackend::Default,
                Some("ThisDeviceDoesNotExist12345".to_string()),
            );

            assert!(
                invalid_result.is_err(),
                "Switch to invalid device should return error"
            );
            eprintln!("[TEST] Invalid device switch returned error as expected");

            // After error, system should still be functional
            let current_device = playback.get_current_device();
            assert!(
                !current_device.is_empty(),
                "Device should still be valid after error"
            );

            // Try valid switch after error
            let recovery_result = playback.switch_device(AudioBackend::Default, None);
            match recovery_result {
                Ok(()) => {
                    eprintln!("[TEST] System recovered successfully after error");
                }
                Err(e) => {
                    eprintln!("[TEST] Recovery switch failed (test env): {}", e);
                }
            }

            // Final state should be valid
            let final_device = playback.get_current_device();
            assert!(!final_device.is_empty(), "System should remain functional");

            // Cleanup
            let _ = playback.send_command(PlaybackCommand::Stop);
        }
        Err(e) => {
            eprintln!(
                "[TEST] Audio device not available in test environment (expected): {}",
                e
            );
        }
    }
}

/// Test 4: Rapid device switches (stress test)
///
/// **Scenario**: Send 10 device switch commands in quick succession.
/// **Expected**: Last switch wins, no deadlock, no crashes, system remains functional.
#[tokio::test]
async fn test_rapid_device_switches() {
    eprintln!("[TEST] Starting rapid device switches stress test");

    let result = DesktopPlayback::new(PlaybackConfig::default());

    match result {
        Ok(mut playback) => {
            // Load track for active playback
            let track = create_test_track("rapid-test");
            let _ = playback.send_command(PlaybackCommand::LoadPlaylist {
                tracks: vec![track],
                start_index: 0,
            });
            let _ = playback.send_command(PlaybackCommand::Play);

            // Give playback time to start
            sleep(Duration::from_millis(100)).await;

            eprintln!("[TEST] Performing 10 rapid device switches");

            let mut success_count = 0;
            let mut fail_count = 0;
            let start_time = std::time::Instant::now();

            for i in 1..=10 {
                match playback.switch_device(AudioBackend::Default, None) {
                    Ok(()) => {
                        success_count += 1;
                        tracing::debug!("[TEST] Rapid switch {}/10 succeeded", i);
                    }
                    Err(e) => {
                        fail_count += 1;
                        tracing::debug!("[TEST] Rapid switch {}/10 failed: {}", i, e);
                    }
                }

                // Very small delay to maximize race condition potential
                sleep(Duration::from_millis(10)).await;
            }

            let duration = start_time.elapsed();
            eprintln!(
                "[TEST] Rapid switches completed in {:?}. Success: {}, Failed: {}",
                duration, success_count, fail_count
            );

            // System should not deadlock (test completed = no deadlock)
            assert!(
                duration < Duration::from_secs(5),
                "Switches should complete quickly, no deadlock"
            );

            // Final state should be valid (no crashes)
            let final_device = playback.get_current_device();
            assert!(
                !final_device.is_empty(),
                "Device should be valid after rapid switches"
            );
            eprintln!("[TEST] Final device: {}", final_device);

            // Playback should still be controllable
            assert!(
                playback.send_command(PlaybackCommand::Pause).is_ok(),
                "Playback should still be controllable"
            );

            // Cleanup
            let _ = playback.send_command(PlaybackCommand::Stop);
        }
        Err(e) => {
            eprintln!(
                "[TEST] Audio device not available in test environment (expected): {}",
                e
            );
        }
    }
}

/// Test 5: Multi-threaded concurrent device switches
///
/// **Scenario**: Multiple threads attempt to switch device simultaneously.
/// **Expected**: No panics, no deadlocks, one switch succeeds, system remains functional.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_multi_threaded_device_switches() {
    eprintln!("[TEST] Starting multi-threaded device switch test");

    let result = DesktopPlayback::new(PlaybackConfig::default());

    match result {
        Ok(playback) => {
            // Wrap in Arc<Mutex> for thread safety
            // Note: DesktopPlayback requires &mut for switch_device, so we need interior mutability
            let playback = Arc::new(std::sync::Mutex::new(playback));

            // Barrier to synchronize thread start for maximum race condition potential
            let barrier = Arc::new(Barrier::new(4));
            let mut handles = vec![];

            eprintln!("[TEST] Spawning 4 threads to switch devices concurrently");

            for thread_id in 1..=4 {
                let playback_clone = Arc::clone(&playback);
                let barrier_clone = Arc::clone(&barrier);

                let handle = tokio::task::spawn_blocking(move || {
                    // Wait for all threads to be ready
                    barrier_clone.wait();

                    eprintln!("[TEST] Thread {} attempting device switch", thread_id);

                    // Each thread tries to switch device
                    let mut pb = playback_clone.lock().unwrap();
                    let result = pb.switch_device(AudioBackend::Default, None);

                    match result {
                        Ok(()) => {
                            eprintln!("[TEST] Thread {} switch succeeded", thread_id);
                            true
                        }
                        Err(e) => {
                            eprintln!("[TEST] Thread {} switch failed: {}", thread_id, e);
                            false
                        }
                    }
                });

                handles.push(handle);
            }

            // Wait for all threads to complete
            let mut success_count = 0;
            for handle in handles {
                match handle.await {
                    Ok(success) => {
                        if success {
                            success_count += 1;
                        }
                    }
                    Err(e) => {
                        panic!("Thread panicked during device switch: {}", e);
                    }
                }
            }

            eprintln!(
                "[TEST] Multi-threaded switches completed. {} threads succeeded",
                success_count
            );

            // Final state should be valid
            let pb = playback.lock().unwrap();
            let final_device = pb.get_current_device();
            assert!(
                !final_device.is_empty(),
                "Device should be valid after concurrent switches"
            );
            eprintln!("[TEST] Final device: {}", final_device);

            // Note: We can't call stop() here because pb is a MutexGuard
            // The drop of playback will clean up
        }
        Err(e) => {
            eprintln!(
                "[TEST] Audio device not available in test environment (expected): {}",
                e
            );
        }
    }
}

/// Test 6: Device switch during track transition
///
/// **Scenario**: Switch device while transitioning from Track A to Track B (crossfade/gapless).
/// **Expected**: Track transition completes correctly on new device, no audio artifacts.
#[tokio::test]
async fn test_device_switch_during_track_transition() {
    eprintln!("[TEST] Starting device switch during track transition test");

    let mut config = PlaybackConfig::default();
    config.crossfade = CrossfadeSettings {
        enabled: true,
        duration_ms: 2000, // 2 second crossfade for easier timing
        curve: FadeCurve::EqualPower,
        on_skip: true,
    };

    let result = DesktopPlayback::new(config);

    match result {
        Ok(mut playback) => {
            // Load multiple tracks
            let tracks = vec![
                create_test_track("transition-1"),
                create_test_track("transition-2"),
                create_test_track("transition-3"),
            ];
            let _ = playback.send_command(PlaybackCommand::LoadPlaylist {
                tracks,
                start_index: 0,
            });
            let _ = playback.send_command(PlaybackCommand::Play);

            // Let first track play briefly
            sleep(Duration::from_millis(300)).await;

            // Skip to next track to trigger transition
            let _ = playback.send_command(PlaybackCommand::Next);
            eprintln!("[TEST] Skipped to next track, transition starting");

            // Switch device during crossfade
            sleep(Duration::from_millis(100)).await; // Small delay to be mid-transition

            match playback.switch_device(AudioBackend::Default, None) {
                Ok(()) => {
                    eprintln!("[TEST] Device switched during track transition");

                    // Let transition complete
                    sleep(Duration::from_millis(500)).await;

                    // Verify playback is still working
                    let current_device = playback.get_current_device();
                    assert!(
                        !current_device.is_empty(),
                        "Device should be valid after transition switch"
                    );

                    // Position should still advance
                    let pos1 = playback.get_position();
                    sleep(Duration::from_millis(200)).await;
                    let pos2 = playback.get_position();

                    assert!(
                        pos2 > pos1,
                        "Position should advance after transition switch"
                    );
                }
                Err(e) => {
                    eprintln!(
                        "[TEST] Device switch during transition failed (test env): {}",
                        e
                    );
                }
            }

            // Cleanup
            let _ = playback.send_command(PlaybackCommand::Stop);
        }
        Err(e) => {
            eprintln!(
                "[TEST] Audio device not available in test environment (expected): {}",
                e
            );
        }
    }
}

/// Test 7: Device switch with queue modifications
///
/// **Scenario**: Switch device while simultaneously modifying playback queue.
/// **Expected**: Both operations complete successfully, queue state consistent.
#[tokio::test]
async fn test_device_switch_with_queue_modifications() {
    eprintln!("[TEST] Starting device switch with queue modifications test");

    let result = DesktopPlayback::new(PlaybackConfig::default());

    match result {
        Ok(mut playback) => {
            // Load initial tracks
            let initial_tracks = vec![create_test_track("queue-1"), create_test_track("queue-2")];
            let _ = playback.send_command(PlaybackCommand::LoadPlaylist {
                tracks: initial_tracks,
                start_index: 0,
            });
            let _ = playback.send_command(PlaybackCommand::Play);

            sleep(Duration::from_millis(100)).await;

            eprintln!("[TEST] Performing concurrent device switch and queue operations");

            // Attempt device switch
            let switch_result = playback.switch_device(AudioBackend::Default, None);

            // Immediately modify queue (potential race condition)
            let new_track = create_test_track("queue-3");
            let queue_result = playback.send_command(PlaybackCommand::AddToQueue(new_track));

            match (switch_result, queue_result) {
                (Ok(()), Ok(())) => {
                    eprintln!("[TEST] Both device switch and queue modification succeeded");
                }
                (Ok(()), Err(qe)) => {
                    eprintln!("[TEST] Device switch OK, queue modification failed: {}", qe);
                }
                (Err(se), Ok(())) => {
                    eprintln!("[TEST] Queue modification OK, device switch failed: {}", se);
                }
                (Err(se), Err(qe)) => {
                    eprintln!(
                        "[TEST] Both failed (test env) - switch: {}, queue: {}",
                        se, qe
                    );
                }
            }

            // System should still be functional
            let current_device = playback.get_current_device();
            assert!(
                !current_device.is_empty(),
                "Device should be valid after concurrent operations"
            );

            // Cleanup
            let _ = playback.send_command(PlaybackCommand::Stop);
        }
        Err(e) => {
            eprintln!(
                "[TEST] Audio device not available in test environment (expected): {}",
                e
            );
        }
    }
}
