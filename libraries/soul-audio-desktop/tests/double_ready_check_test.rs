//! Test for double ready check bug
//!
//! Bug: When TrackLoader returns a ready source, set_audio_source() resets
//! source_ready_verified to false, causing playback to wait/check again.
//! This manifests as a stutter or "false start" at the beginning of playback.

use soul_audio_desktop::{DesktopPlayback, PlaybackCommand, PlaybackEvent};
use soul_playback::{PlaybackConfig, PlaybackState, QueueTrack, TrackSource};
use std::path::PathBuf;
use std::time::{Duration, Instant};

fn create_test_track(id: &str, filename: &str) -> QueueTrack {
    QueueTrack {
        id: id.to_string().into(),
        path: PathBuf::from(filename),
        title: format!("Test Track {}", id),
        artist: "Test Artist".to_string(),
        album: Some("Test Album".to_string()),
        duration: Duration::from_secs(180),
        track_number: Some(1),
        source: TrackSource::Single,
    }
}

#[test]
fn test_no_double_ready_check() {
    println!("\n=== Testing for Double Ready Check Bug ===\n");

    // Get absolute path to test file
    let base_dir = std::env::current_dir().expect("Failed to get current directory");
    let track_path = base_dir.join("test_data/track_1.wav");
    assert!(
        track_path.exists(),
        "Test file not found: {}",
        track_path.display()
    );

    let config = PlaybackConfig::default();
    let playback = DesktopPlayback::new(config).expect("Failed to create DesktopPlayback");

    let tracks = vec![create_test_track("1", track_path.to_str().unwrap())];

    println!("[1] Loading playlist...");
    playback
        .send_command(PlaybackCommand::LoadPlaylist(tracks))
        .expect("Failed to load playlist");

    println!("[2] Starting playback...");
    let play_start = Instant::now();
    playback
        .send_command(PlaybackCommand::Play)
        .expect("Failed to play");

    // Track timing of state changes
    let mut first_playing_time = None;
    let mut loading_to_playing_count = 0;
    let mut all_events = Vec::new();

    // Monitor events for 3 seconds
    let monitor_start = Instant::now();
    while monitor_start.elapsed() < Duration::from_secs(3) {
        std::thread::sleep(Duration::from_millis(10));

        while let Some(event) = playback.try_recv_event() {
            let elapsed = play_start.elapsed();

            match &event {
                PlaybackEvent::StateChanged(PlaybackState::Playing) => {
                    loading_to_playing_count += 1;
                    if first_playing_time.is_none() {
                        first_playing_time = Some(elapsed);
                        println!("[T+{:?}] ✓ First Playing event", elapsed);
                    } else {
                        println!(
                            "[T+{:?}] ⚠️  DUPLICATE Playing event #{}",
                            elapsed, loading_to_playing_count
                        );
                    }
                }
                PlaybackEvent::StateChanged(state) => {
                    println!("[T+{:?}] StateChanged({:?})", elapsed, state);
                }
                PlaybackEvent::TrackChanged(Some(track)) => {
                    println!(
                        "[T+{:?}] TrackChanged: {} (id: {})",
                        elapsed, track.title, track.id
                    );
                }
                PlaybackEvent::Error(e) => {
                    println!("[T+{:?}] Error: {}", elapsed, e);
                }
                _ => {}
            }

            all_events.push((elapsed, event));
        }
    }

    // Stop playback
    playback.send_command(PlaybackCommand::Stop).ok();
    std::thread::sleep(Duration::from_millis(100));

    // Analysis
    println!("\n=== ANALYSIS ===");
    println!("Playing events emitted: {}", loading_to_playing_count);

    if let Some(first_time) = first_playing_time {
        println!("First Playing event at: {:?}", first_time);

        if first_time > Duration::from_millis(500) {
            println!("⚠️  WARNING: Took > 500ms to reach Playing state");
            println!("   This suggests the source ready check is causing delay");
        }
    }

    // Check for duplicate Playing events (symptom of double ready check)
    if loading_to_playing_count > 1 {
        println!("\n❌ FAIL: Multiple Playing events detected!");
        println!("   This indicates the ready check is happening multiple times.");
        println!("   Expected: 1 Playing event");
        println!("   Actual: {} Playing events", loading_to_playing_count);

        // Show timing between Playing events
        let playing_events: Vec<_> = all_events
            .iter()
            .filter(|(_, e)| matches!(e, PlaybackEvent::StateChanged(PlaybackState::Playing)))
            .collect();

        if playing_events.len() > 1 {
            for i in 1..playing_events.len() {
                let gap = playing_events[i].0 - playing_events[i - 1].0;
                println!("   Gap between event {} and {}: {:?}", i, i + 1, gap);
            }
        }

        panic!("Double ready check bug detected");
    } else if loading_to_playing_count == 1 {
        println!("\n✓ PASS: Single Playing event (no double ready check)");
    } else {
        println!("\n❌ FAIL: No Playing event received!");
        panic!("Playback never started");
    }
}

#[test]
fn test_ready_check_timing() {
    println!("\n=== Testing Ready Check Timing ===\n");

    let base_dir = std::env::current_dir().expect("Failed to get current directory");
    let track_path = base_dir.join("test_data/track_1.wav");
    assert!(
        track_path.exists(),
        "Test file not found: {}",
        track_path.display()
    );

    let config = PlaybackConfig::default();
    let playback = DesktopPlayback::new(config).expect("Failed to create DesktopPlayback");

    let tracks = vec![create_test_track("1", track_path.to_str().unwrap())];

    playback
        .send_command(PlaybackCommand::LoadPlaylist(tracks))
        .expect("Failed to load playlist");

    let play_start = Instant::now();
    playback
        .send_command(PlaybackCommand::Play)
        .expect("Failed to play");

    // Measure time to first Playing event
    let mut playing_received = false;
    let timeout = Duration::from_secs(2);

    while play_start.elapsed() < timeout && !playing_received {
        std::thread::sleep(Duration::from_millis(5));

        while let Some(event) = playback.try_recv_event() {
            if let PlaybackEvent::StateChanged(PlaybackState::Playing) = event {
                let delay = play_start.elapsed();
                println!("✓ Playing event received after {:?}", delay);

                // Check if delay is reasonable
                if delay > Duration::from_millis(300) {
                    println!("⚠️  WARNING: Delay > 300ms suggests redundant ready check");
                    println!("   Expected: < 200ms (buffer already ready from TrackLoader)");
                    println!("   Actual: {:?}", delay);
                }

                playing_received = true;
                break;
            }
        }
    }

    playback.send_command(PlaybackCommand::Stop).ok();
    std::thread::sleep(Duration::from_millis(100));

    assert!(
        playing_received,
        "Never received Playing event within {:?}",
        timeout
    );
}
