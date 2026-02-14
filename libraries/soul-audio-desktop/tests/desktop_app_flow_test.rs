//! Test that reproduces the actual desktop app flow
//! This test mimics how the Tauri desktop app uses PlaybackManager

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
fn test_desktop_app_playback_flow() {
    println!("\n=== Testing Desktop App Playback Flow ===\n");

    // Get absolute path to test file
    let base_dir = std::env::current_dir().expect("Failed to get current directory");
    let track_path = base_dir.join("test_data/track_1.wav");
    assert!(
        track_path.exists(),
        "Test file not found: {}",
        track_path.display()
    );

    // Create PlaybackManager with default config (same as desktop app)
    let config = PlaybackConfig::default();
    let playback = DesktopPlayback::new(config).expect("Failed to create DesktopPlayback");

    // === Simulate Desktop App Flow ===

    // 1. User loads a track (e.g., clicking on album)
    let tracks = vec![create_test_track("1", track_path.to_str().unwrap())];

    println!("[1] LoadPlaylist command");
    playback
        .send_command(PlaybackCommand::LoadPlaylist {
            tracks,
            start_index: 0,
        })
        .expect("Failed to load playlist");

    // Small delay to let command process
    std::thread::sleep(Duration::from_millis(10));

    // 2. User clicks play button (triggers invoke('play'))
    println!("[2] Play command");
    let play_start = Instant::now();
    playback
        .send_command(PlaybackCommand::Play)
        .expect("Failed to play");

    // === Monitor Events (like TauriPlayerCommandsProvider does) ===
    let mut playing_events = Vec::new();
    let mut track_changed_events = Vec::new();
    let mut all_events_with_timing = Vec::new();

    // Monitor for 3 seconds
    let monitor_start = Instant::now();
    while monitor_start.elapsed() < Duration::from_secs(3) {
        std::thread::sleep(Duration::from_millis(5));

        while let Some(event) = playback.try_recv_event() {
            let elapsed = play_start.elapsed();

            match &event {
                PlaybackEvent::StateChanged(PlaybackState::Playing) => {
                    playing_events.push(elapsed);
                    println!(
                        "[T+{:?}] StateChanged(Playing) #{}",
                        elapsed,
                        playing_events.len()
                    );
                }
                PlaybackEvent::StateChanged(state) => {
                    println!("[T+{:?}] StateChanged({:?})", elapsed, state);
                }
                PlaybackEvent::TrackChanged(Some(track)) => {
                    track_changed_events.push((elapsed, track.id.clone()));
                    println!(
                        "[T+{:?}] TrackChanged: {} (id: {})",
                        elapsed, track.title, track.id
                    );
                }
                PlaybackEvent::TrackChanged(None) => {
                    println!("[T+{:?}] TrackChanged: None", elapsed);
                }
                PlaybackEvent::Error(e) => {
                    println!("[T+{:?}] Error: {}", elapsed, e);
                }
                _ => {}
            }

            all_events_with_timing.push((elapsed, format!("{:?}", event)));
        }
    }

    // Stop playback
    playback.send_command(PlaybackCommand::Stop).ok();
    std::thread::sleep(Duration::from_millis(100));

    // === Analysis ===
    println!("\n=== ANALYSIS ===");
    println!("Total Playing events: {}", playing_events.len());
    println!("Total TrackChanged events: {}", track_changed_events.len());

    if !playing_events.is_empty() {
        println!("\nPlaying event timings:");
        for (i, timing) in playing_events.iter().enumerate() {
            println!("  Event {}: {:?}", i + 1, timing);
        }

        if playing_events.len() > 1 {
            println!("\nGaps between Playing events:");
            for i in 1..playing_events.len() {
                let gap = playing_events[i] - playing_events[i - 1];
                println!("  Gap {}-{}: {:?}", i, i + 1, gap);
            }
        }
    }

    if !track_changed_events.is_empty() {
        println!("\nTrackChanged event timings:");
        for (i, (timing, track_id)) in track_changed_events.iter().enumerate() {
            println!("  Event {}: {:?} (id: {})", i + 1, timing, track_id);
        }
    }

    // === Assertions ===
    assert!(
        !playing_events.is_empty(),
        "No Playing events received - playback never started!"
    );

    if playing_events.len() > 1 {
        println!("\n❌ FAIL: Multiple Playing events detected!");
        println!("   This reproduces the desktop app bug.");
        println!("   Expected: 1 Playing event");
        println!("   Actual: {} Playing events", playing_events.len());

        println!("\n=== ALL EVENTS (for debugging) ===");
        for (timing, event) in &all_events_with_timing {
            println!("[T+{:?}] {}", timing, event);
        }

        panic!(
            "Desktop app bug reproduced: {} Playing events instead of 1",
            playing_events.len()
        );
    } else {
        println!("\n✓ PASS: Single Playing event (desktop app flow correct)");
    }
}

#[test]
fn test_play_queue_command() {
    println!("\n=== Testing play_queue Command (Desktop UI Pattern) ===\n");

    let base_dir = std::env::current_dir().expect("Failed to get current directory");
    let track_path = base_dir.join("test_data/track_1.wav");
    assert!(
        track_path.exists(),
        "Test file not found: {}",
        track_path.display()
    );

    let config = PlaybackConfig::default();
    let playback = DesktopPlayback::new(config).expect("Failed to create DesktopPlayback");

    // Desktop app pattern: LoadPlaylist + immediate Play in quick succession
    let tracks = vec![create_test_track("1", track_path.to_str().unwrap())];

    println!("[1] LoadPlaylist + Play (rapid fire)");
    let start = Instant::now();
    playback
        .send_command(PlaybackCommand::LoadPlaylist {
            tracks,
            start_index: 0,
        })
        .expect("Failed to load");
    playback
        .send_command(PlaybackCommand::Play)
        .expect("Failed to play");

    // Monitor events
    let mut playing_count = 0;
    let timeout = Duration::from_secs(2);

    while start.elapsed() < timeout && playing_count < 2 {
        std::thread::sleep(Duration::from_millis(10));

        while let Some(event) = playback.try_recv_event() {
            if let PlaybackEvent::StateChanged(PlaybackState::Playing) = event {
                playing_count += 1;
                println!("[T+{:?}] Playing event #{}", start.elapsed(), playing_count);

                if playing_count > 1 {
                    println!("❌ DUPLICATE Playing event detected!");
                    playback.send_command(PlaybackCommand::Stop).ok();
                    panic!("Multiple Playing events in rapid play scenario");
                }
            }
        }
    }

    playback.send_command(PlaybackCommand::Stop).ok();

    assert_eq!(
        playing_count, 1,
        "Expected exactly 1 Playing event, got {}",
        playing_count
    );
    println!("✓ PASS: Single Playing event in rapid play scenario");
}
