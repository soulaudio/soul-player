//! Diagnostic test to capture exact event sequence
//! Run this and copy the output to help debug the stutter issue

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
fn diagnostic_event_sequence() {
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  EVENT SEQUENCE DIAGNOSTIC - Copy this output to debug      ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    let base_dir = std::env::current_dir().expect("Failed to get current directory");
    let track_path = base_dir.join("test_data/track_1.wav");

    if !track_path.exists() {
        println!("❌ Test file not found: {}", track_path.display());
        println!("   Run this test from the workspace root");
        return;
    }

    let config = PlaybackConfig::default();
    let playback = DesktopPlayback::new(config).expect("Failed to create DesktopPlayback");
    let tracks = vec![create_test_track("1", track_path.to_str().unwrap())];

    println!("┌─ COMMAND: LoadPlaylist");
    playback
        .send_command(PlaybackCommand::LoadPlaylist {
            tracks,
            start_index: 0,
        })
        .expect("Failed to load");
    std::thread::sleep(Duration::from_millis(10));

    println!("┌─ COMMAND: Play");
    let start = Instant::now();
    playback
        .send_command(PlaybackCommand::Play)
        .expect("Failed to play");

    // Collect ALL events for 5 seconds
    let mut all_events = Vec::new();
    let monitor_duration = Duration::from_secs(5);

    while start.elapsed() < monitor_duration {
        std::thread::sleep(Duration::from_millis(5));

        while let Some(event) = playback.try_recv_event() {
            let elapsed_ms = start.elapsed().as_millis();
            all_events.push((elapsed_ms, event));
        }
    }

    // Stop
    playback.send_command(PlaybackCommand::Stop).ok();

    // Print full timeline
    println!("\n┌─ EVENT TIMELINE (5 second capture)");
    println!("│");

    for (ms, event) in &all_events {
        let marker = match event {
            PlaybackEvent::StateChanged(PlaybackState::Playing) => "🔴 ",
            PlaybackEvent::StateChanged(PlaybackState::Paused) => "⏸️  ",
            PlaybackEvent::StateChanged(PlaybackState::Stopped) => "⏹️  ",
            PlaybackEvent::TrackChanged(_) => "🎵 ",
            _ => "   ",
        };

        println!("│ T+{:5}ms {} {:?}", ms, marker, event);
    }

    println!("│");
    println!("└─ Total events: {}\n", all_events.len());

    // Analysis
    let state_changes: Vec<_> = all_events
        .iter()
        .filter_map(|(ms, e)| {
            if let PlaybackEvent::StateChanged(state) = e {
                Some((*ms, state))
            } else {
                None
            }
        })
        .collect();

    println!("┌─ STATE CHANGE SUMMARY");
    println!("│");
    for (i, (ms, state)) in state_changes.iter().enumerate() {
        let next_marker = if i + 1 < state_changes.len() {
            format!(" → {:?}", state_changes[i + 1].1)
        } else {
            String::from(" (final)")
        };
        println!("│  {:5}ms: {:?}{}", ms, state, next_marker);
    }
    println!("│");
    println!("└─ Total state changes: {}\n", state_changes.len());

    // Check for duplicate Playing states
    let playing_states: Vec<_> = state_changes
        .iter()
        .filter(|(_, state)| matches!(state, PlaybackState::Playing))
        .collect();

    if playing_states.len() > 1 {
        println!("⚠️  WARNING: Multiple Playing states detected!");
        println!("   Count: {}", playing_states.len());
        println!("   Timings:");
        for (i, (ms, _)) in playing_states.iter().enumerate() {
            println!("     Playing event #{}: {}ms", i + 1, ms);
            if i > 0 {
                let gap = *ms - playing_states[i - 1].0;
                println!("       (gap from previous: {}ms)", gap);
            }
        }
        println!("\n   ❌ THIS IS THE BUG - copy this output!");
    } else if playing_states.len() == 1 {
        println!("✅ GOOD: Single Playing state at {}ms", playing_states[0].0);
    } else {
        println!("❌ ERROR: No Playing state received!");
    }

    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  END OF DIAGNOSTIC - Copy everything above this line         ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
}
