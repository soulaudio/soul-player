//! Quick test for initialization delay using real audio files

use soul_audio_desktop::{DesktopPlayback, PlaybackCommand, PlaybackEvent};
use soul_playback::{PlaybackConfig, PlaybackState, QueueTrack, TrackSource};
use std::path::PathBuf;
use std::time::{Duration, Instant};

fn create_test_track(id: &str, filename: &str) -> QueueTrack {
    QueueTrack {
        id: id.to_string(),
        path: PathBuf::from(filename),
        title: format!("Test Track {}", id),
        artist: "Test Artist".to_string(),
        album: Some("Test Album".to_string()),
        duration: Duration::from_secs(180),
        track_number: Some(1),
        source: TrackSource::Single,
    }
}

fn main() {
    println!("\n=== Testing Audio Initialization Delay ===\n");

    let config = PlaybackConfig::default();
    let playback = DesktopPlayback::new(config).expect("Failed to create playback");

    let tracks = vec![
        create_test_track("1", "libraries/soul-audio-desktop/test_data/track_1.wav"),
        create_test_track("2", "libraries/soul-audio-desktop/test_data/track_2.wav"),
    ];

    println!("[1] Loading playlist...");
    playback.send_command(PlaybackCommand::LoadPlaylist(tracks)).unwrap();

    println!("[2] Starting playback...");
    let play_time = Instant::now();
    playback.send_command(PlaybackCommand::Play).unwrap();

    // Wait for audio to actually start
    std::thread::sleep(Duration::from_millis(100));

    // Collect events
    let mut events = Vec::new();
    while let Some(event) = playback.try_recv_event() {
        events.push(event);
    }

    let init_delay = play_time.elapsed();

    println!("\n[RESULTS]");
    println!("  Time from Play command to processing: {:?}", init_delay);
    println!("  Events received: {}", events.len());

    for (i, event) in events.iter().enumerate() {
        match event {
            PlaybackEvent::StateChanged(state) => {
                println!("    Event {}: StateChanged({:?})", i, state);
            }
            PlaybackEvent::Error(err) => {
                println!("    Event {}: Error({})", i, err);
            }
            _ => {
                println!("    Event {}: {:?}", i, event);
            }
        }
    }

    if init_delay > Duration::from_millis(200) {
        println!("\n⚠️  INITIALIZATION DELAY DETECTED: {:?}", init_delay);
        println!("  Expected: < 200ms");
        println!("  This confirms the initialization delay issue!");
    } else {
        println!("\n✓ Initialization time is good: {:?}", init_delay);
    }
}
