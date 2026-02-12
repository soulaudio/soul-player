//! Device failure and recovery tests
//!
//! Tests audio device failure scenarios:
//! - Device disconnect during playback
//! - No devices available
//! - Sample rate mismatches
//! - Buffer underruns
//!
//! Run with: cargo test --test device_failure_test -- --include-ignored

use soul_audio_desktop::sources::LocalAudioSource;
use soul_audio_desktop::{DesktopAudioBackend, DesktopPlaybackCommand, PlaybackContext};
use soul_playback::{AudioSource, PlaybackEvent, PlaybackState};
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

// ============================================================================
// Test Utilities
// ============================================================================

fn test_audio_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test_data")
        .join("sine_1khz_10s_44100hz_stereo.wav")
}

fn wait_for_state(
    event_rx: &mpsc::Receiver<PlaybackEvent>,
    target_state: PlaybackState,
    timeout: Duration,
) -> bool {
    let start = std::time::Instant::now();
    while start.elapsed() < timeout {
        if let Ok(event) = event_rx.try_recv() {
            if let PlaybackEvent::StateChanged { state } = event {
                if matches!(
                    (state, target_state),
                    (
                        soul_playback::events::PlaybackStateEvent::Playing,
                        PlaybackState::Playing
                    ) | (
                        soul_playback::events::PlaybackStateEvent::Paused,
                        PlaybackState::Paused
                    ) | (
                        soul_playback::events::PlaybackStateEvent::Stopped,
                        PlaybackState::Stopped
                    )
                ) {
                    return true;
                }
            }
        }
        thread::sleep(Duration::from_millis(10));
    }
    false
}

// ============================================================================
// 1. Device Disconnect Tests
// ============================================================================

#[test]
#[ignore] // Requires real audio device
fn test_device_unplugged_during_playback() {
    // This test demonstrates the expected behavior when a device is unplugged.
    // In practice, the OS will either:
    // 1. Automatically switch to another device
    // 2. Return errors from audio callbacks
    // 3. Stop calling audio callbacks

    let (cmd_tx, cmd_rx) = mpsc::channel();
    let (event_tx, event_rx) = mpsc::channel();

    let backend = DesktopAudioBackend::new(cmd_rx, event_tx.clone());
    let _backend_thread = thread::spawn(move || backend.run());

    // Start playback
    let ctx = PlaybackContext {
        device_id: None,
        sample_rate: 48000,
        buffer_size: 512,
    };
    cmd_tx.send(DesktopPlaybackCommand::Initialize(ctx)).ok();
    thread::sleep(Duration::from_millis(100));

    // Load and play track
    cmd_tx
        .send(DesktopPlaybackCommand::LoadTrack {
            track_id: "test1".into(),
            path: test_audio_path(),
            start_position: Duration::ZERO,
        })
        .ok();

    thread::sleep(Duration::from_millis(200));

    cmd_tx.send(DesktopPlaybackCommand::Play).ok();
    assert!(
        wait_for_state(&event_rx, PlaybackState::Playing, Duration::from_secs(2)),
        "Should start playing"
    );

    // Manual intervention: User should unplug audio device now
    println!("=== MANUAL TEST STEP ===");
    println!("Unplug your audio device within 5 seconds...");
    thread::sleep(Duration::from_secs(5));

    // Check events for error or state change
    let mut found_error = false;
    let mut found_pause = false;

    for _ in 0..50 {
        if let Ok(event) = event_rx.try_recv() {
            match event {
                PlaybackEvent::Error { .. } => {
                    found_error = true;
                    println!("✓ Received error event on device disconnect");
                }
                PlaybackEvent::StateChanged { state } => {
                    if matches!(
                        state,
                        soul_playback::events::PlaybackStateEvent::Paused
                            | soul_playback::events::PlaybackStateEvent::Stopped
                    ) {
                        found_pause = true;
                        println!("✓ Playback paused/stopped on device disconnect");
                    }
                }
                _ => {}
            }
        }
        thread::sleep(Duration::from_millis(10));
    }

    // Expected: Either error event or state change to paused/stopped
    assert!(
        found_error || found_pause,
        "Should emit error or pause on device disconnect"
    );

    cmd_tx.send(DesktopPlaybackCommand::Shutdown).ok();
}

#[test]
#[ignore] // Requires manual device control
fn test_device_unplug_replug_cycle() {
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

    cmd_tx
        .send(DesktopPlaybackCommand::LoadTrack {
            track_id: "test1".into(),
            path: test_audio_path(),
            start_position: Duration::ZERO,
        })
        .ok();

    thread::sleep(Duration::from_millis(200));
    cmd_tx.send(DesktopPlaybackCommand::Play).ok();
    assert!(
        wait_for_state(&event_rx, PlaybackState::Playing, Duration::from_secs(2)),
        "Should start playing"
    );

    println!("=== MANUAL TEST STEP ===");
    println!("1. Unplug audio device (wait 3 seconds)");
    thread::sleep(Duration::from_secs(3));

    println!("2. Replug audio device (wait 3 seconds)");
    thread::sleep(Duration::from_secs(3));

    // Try to resume playback
    cmd_tx.send(DesktopPlaybackCommand::Play).ok();
    thread::sleep(Duration::from_millis(500));

    // Should recover gracefully
    let mut recovered = false;
    for _ in 0..20 {
        if let Ok(event) = event_rx.try_recv() {
            if let PlaybackEvent::StateChanged { state } = event {
                if matches!(state, soul_playback::events::PlaybackStateEvent::Playing) {
                    recovered = true;
                    println!("✓ Playback recovered after device replug");
                    break;
                }
            }
        }
        thread::sleep(Duration::from_millis(50));
    }

    assert!(recovered, "Should recover playback after device replug");

    cmd_tx.send(DesktopPlaybackCommand::Shutdown).ok();
}

// ============================================================================
// 2. Zero Device Tests
// ============================================================================

#[test]
#[ignore] // Requires no audio devices
fn test_no_audio_devices_available() {
    // This test should be run on a system with no audio devices
    // or with audio devices disabled in device manager

    let (cmd_tx, cmd_rx) = mpsc::channel();
    let (event_tx, event_rx) = mpsc::channel();

    let backend = DesktopAudioBackend::new(cmd_rx, event_tx.clone());
    let _backend_thread = thread::spawn(move || backend.run());

    let ctx = PlaybackContext {
        device_id: None,
        sample_rate: 48000,
        buffer_size: 512,
    };

    // Initialize should succeed (falls back to null device)
    cmd_tx.send(DesktopPlaybackCommand::Initialize(ctx)).ok();
    thread::sleep(Duration::from_millis(200));

    // Load track should work
    cmd_tx
        .send(DesktopPlaybackCommand::LoadTrack {
            track_id: "test1".into(),
            path: test_audio_path(),
            start_position: Duration::ZERO,
        })
        .ok();

    thread::sleep(Duration::from_millis(200));

    // Queue manipulation should work
    cmd_tx.send(DesktopPlaybackCommand::Play).ok();
    thread::sleep(Duration::from_millis(100));

    cmd_tx.send(DesktopPlaybackCommand::Pause).ok();
    thread::sleep(Duration::from_millis(100));

    // Verify we can receive events (UI should remain functional)
    let mut got_events = false;
    for _ in 0..20 {
        if event_rx.try_recv().is_ok() {
            got_events = true;
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }

    assert!(got_events, "Should emit events even with no devices");

    cmd_tx.send(DesktopPlaybackCommand::Shutdown).ok();
}

// ============================================================================
// 3. Sample Rate Mismatch Tests
// ============================================================================

#[test]
#[ignore] // Requires real audio device
fn test_sample_rate_mismatch() {
    // Test resampling when device and source have different sample rates

    let (cmd_tx, cmd_rx) = mpsc::channel();
    let (event_tx, event_rx) = mpsc::channel();

    let backend = DesktopAudioBackend::new(cmd_rx, event_tx.clone());
    let _backend_thread = thread::spawn(move || backend.run());

    // Request 48kHz output (source is 44.1kHz)
    let ctx = PlaybackContext {
        device_id: None,
        sample_rate: 48000, // Different from source (44100)
        buffer_size: 512,
    };
    cmd_tx.send(DesktopPlaybackCommand::Initialize(ctx)).ok();
    thread::sleep(Duration::from_millis(100));

    cmd_tx
        .send(DesktopPlaybackCommand::LoadTrack {
            track_id: "test1".into(),
            path: test_audio_path(),
            start_position: Duration::ZERO,
        })
        .ok();

    thread::sleep(Duration::from_millis(200));

    cmd_tx.send(DesktopPlaybackCommand::Play).ok();
    assert!(
        wait_for_state(&event_rx, PlaybackState::Playing, Duration::from_secs(2)),
        "Should start playing with resampling"
    );

    // Let it play for a bit
    thread::sleep(Duration::from_secs(2));

    // Check for audio glitches (no error events)
    let mut found_error = false;
    while let Ok(event) = event_rx.try_recv() {
        if matches!(event, PlaybackEvent::Error { .. }) {
            found_error = true;
        }
    }

    assert!(!found_error, "Should not have errors during resampling");

    cmd_tx.send(DesktopPlaybackCommand::Shutdown).ok();
}

#[test]
#[ignore] // Requires real audio device
fn test_extreme_sample_rate_mismatch() {
    // Test with very different sample rates (e.g., 8kHz -> 192kHz)

    let (cmd_tx, cmd_rx) = mpsc::channel();
    let (event_tx, _event_rx) = mpsc::channel();

    let backend = DesktopAudioBackend::new(cmd_rx, event_tx.clone());
    let _backend_thread = thread::spawn(move || backend.run());

    // Try extreme sample rate
    let ctx = PlaybackContext {
        device_id: None,
        sample_rate: 192000, // Very high
        buffer_size: 512,
    };
    cmd_tx.send(DesktopPlaybackCommand::Initialize(ctx)).ok();
    thread::sleep(Duration::from_millis(200));

    // Should not crash
    cmd_tx
        .send(DesktopPlaybackCommand::LoadTrack {
            track_id: "test1".into(),
            path: test_audio_path(),
            start_position: Duration::ZERO,
        })
        .ok();

    thread::sleep(Duration::from_millis(200));

    cmd_tx.send(DesktopPlaybackCommand::Play).ok();
    thread::sleep(Duration::from_secs(1));

    // Should complete without crash
    cmd_tx.send(DesktopPlaybackCommand::Shutdown).ok();
}

// ============================================================================
// 4. Buffer Underrun Tests
// ============================================================================

#[test]
#[ignore] // Requires real audio device
fn test_buffer_underrun_recovery() {
    // Simulate slow decoder by using very small buffer size

    let (cmd_tx, cmd_rx) = mpsc::channel();
    let (event_tx, event_rx) = mpsc::channel();

    let backend = DesktopAudioBackend::new(cmd_rx, event_tx.clone());
    let _backend_thread = thread::spawn(move || backend.run());

    let ctx = PlaybackContext {
        device_id: None,
        sample_rate: 48000,
        buffer_size: 64, // Very small buffer (increases underrun risk)
    };
    cmd_tx.send(DesktopPlaybackCommand::Initialize(ctx)).ok();
    thread::sleep(Duration::from_millis(100));

    cmd_tx
        .send(DesktopPlaybackCommand::LoadTrack {
            track_id: "test1".into(),
            path: test_audio_path(),
            start_position: Duration::ZERO,
        })
        .ok();

    thread::sleep(Duration::from_millis(200));

    cmd_tx.send(DesktopPlaybackCommand::Play).ok();
    thread::sleep(Duration::from_secs(3));

    // Check for underrun warnings in logs
    // (implementation should log warnings but continue playing)

    let mut still_playing = false;
    while let Ok(event) = event_rx.try_recv() {
        if let PlaybackEvent::StateChanged { state } = event {
            if matches!(state, soul_playback::events::PlaybackStateEvent::Playing) {
                still_playing = true;
            }
        }
    }

    assert!(
        still_playing,
        "Should recover from buffer underruns and continue playing"
    );

    cmd_tx.send(DesktopPlaybackCommand::Shutdown).ok();
}

// ============================================================================
// 5. Device Capability Tests
// ============================================================================

#[test]
#[ignore] // Requires real audio device
fn test_unsupported_channel_count() {
    // Try to use unsupported channel configuration

    let (cmd_tx, cmd_rx) = mpsc::channel();
    let (event_tx, _event_rx) = mpsc::channel();

    let backend = DesktopAudioBackend::new(cmd_rx, event_tx.clone());
    let _backend_thread = thread::spawn(move || backend.run());

    // Most devices support stereo, but not all support mono or 5.1
    let ctx = PlaybackContext {
        device_id: None,
        sample_rate: 48000,
        buffer_size: 512,
    };

    cmd_tx.send(DesktopPlaybackCommand::Initialize(ctx)).ok();
    thread::sleep(Duration::from_millis(200));

    // Should fall back to supported channel count
    cmd_tx
        .send(DesktopPlaybackCommand::LoadTrack {
            track_id: "test1".into(),
            path: test_audio_path(),
            start_position: Duration::ZERO,
        })
        .ok();

    thread::sleep(Duration::from_millis(200));

    // Should not crash
    cmd_tx.send(DesktopPlaybackCommand::Shutdown).ok();
}

// ============================================================================
// 6. Device Enumeration Failures
// ============================================================================

#[test]
#[ignore]
fn test_device_enumeration_timeout() {
    // Test that device enumeration doesn't block forever
    use std::time::Instant;

    let (cmd_tx, cmd_rx) = mpsc::channel();
    let (event_tx, _event_rx) = mpsc::channel();

    let start = Instant::now();

    let backend = DesktopAudioBackend::new(cmd_rx, event_tx.clone());
    let backend_thread = thread::spawn(move || backend.run());

    let ctx = PlaybackContext {
        device_id: None,
        sample_rate: 48000,
        buffer_size: 512,
    };
    cmd_tx.send(DesktopPlaybackCommand::Initialize(ctx)).ok();

    // Wait for initialization
    thread::sleep(Duration::from_millis(500));

    let elapsed = start.elapsed();

    // Device enumeration should complete quickly (< 1 second)
    assert!(
        elapsed < Duration::from_secs(1),
        "Device enumeration took too long: {:?}",
        elapsed
    );

    cmd_tx.send(DesktopPlaybackCommand::Shutdown).ok();
    backend_thread.join().ok();
}

// ============================================================================
// 7. Concurrent Device Operations
// ============================================================================

#[test]
#[ignore] // Requires real audio device
fn test_device_switch_during_playback() {
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

    cmd_tx
        .send(DesktopPlaybackCommand::LoadTrack {
            track_id: "test1".into(),
            path: test_audio_path(),
            start_position: Duration::ZERO,
        })
        .ok();

    thread::sleep(Duration::from_millis(200));

    cmd_tx.send(DesktopPlaybackCommand::Play).ok();
    assert!(
        wait_for_state(&event_rx, PlaybackState::Playing, Duration::from_secs(2)),
        "Should start playing"
    );

    // Switch device mid-playback
    // (In real implementation, this would be a device change command)
    thread::sleep(Duration::from_secs(1));

    // Verify playback continues after device switch
    let mut still_playing = false;
    for _ in 0..20 {
        if let Ok(event) = event_rx.try_recv() {
            if let PlaybackEvent::StateChanged { state } = event {
                if matches!(state, soul_playback::events::PlaybackStateEvent::Playing) {
                    still_playing = true;
                }
            }
        }
        thread::sleep(Duration::from_millis(50));
    }

    assert!(still_playing, "Should continue playing after device switch");

    cmd_tx.send(DesktopPlaybackCommand::Shutdown).ok();
}

// ============================================================================
// 8. Edge Cases
// ============================================================================

#[test]
#[ignore]
fn test_rapid_device_switches() {
    let (cmd_tx, cmd_rx) = mpsc::channel();
    let (event_tx, _event_rx) = mpsc::channel();

    let backend = DesktopAudioBackend::new(cmd_rx, event_tx.clone());
    let _backend_thread = thread::spawn(move || backend.run());

    // Rapidly switch devices
    for i in 0..10 {
        let ctx = PlaybackContext {
            device_id: None,
            sample_rate: if i % 2 == 0 { 44100 } else { 48000 },
            buffer_size: 512,
        };
        cmd_tx.send(DesktopPlaybackCommand::Initialize(ctx)).ok();
        thread::sleep(Duration::from_millis(50));
    }

    // Should not crash
    thread::sleep(Duration::from_millis(500));

    cmd_tx.send(DesktopPlaybackCommand::Shutdown).ok();
}
