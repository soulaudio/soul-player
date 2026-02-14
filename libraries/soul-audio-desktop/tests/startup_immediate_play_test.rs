//! E2E test: Measure delay from initialization to first audio playback
//!
//! This test simulates the app startup flow:
//! 1. Initialize DesktopPlayback (like PlaybackManager::new())
//! 2. Optionally warm up the audio device
//! 3. Load queue and play immediately
//! 4. Measure time to actual audio output

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

fn wait_for_event<F>(playback: &DesktopPlayback, mut predicate: F, timeout: Duration) -> bool
where
    F: FnMut(&PlaybackEvent) -> bool,
{
    let start = Instant::now();
    while start.elapsed() < timeout {
        while let Some(event) = playback.try_recv_event() {
            if predicate(&event) {
                return true;
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    false
}

#[test]
fn test_cold_start_immediate_play() {
    println!("\n=== E2E Test: COLD START - Immediate Play After Init ===\n");

    let total_start = Instant::now();

    // Get absolute paths to test files
    // Test runs from crate root (libraries/soul-audio-desktop), not repo root
    let base_dir = std::env::current_dir().expect("Failed to get current directory");
    let track1_path = base_dir.join("test_data/track_1.wav");
    let track2_path = base_dir.join("test_data/track_2.wav");

    // Verify files exist
    assert!(
        track1_path.exists(),
        "Test file not found: {}",
        track1_path.display()
    );
    assert!(
        track2_path.exists(),
        "Test file not found: {}",
        track2_path.display()
    );
    println!("✓ Test files found:");
    println!("  - {}", track1_path.display());
    println!("  - {}", track2_path.display());

    // Phase 1: Initialize audio engine (simulates PlaybackManager::new())
    println!("[T+0ms] Initializing audio engine...");
    let init_start = Instant::now();

    let config = PlaybackConfig::default();
    let playback = DesktopPlayback::new(config).expect("Failed to create DesktopPlayback");

    let init_duration = init_start.elapsed();
    println!(
        "[T+{:?}] ✓ Audio engine initialized (took {:?})",
        total_start.elapsed(),
        init_duration
    );

    // Phase 2: Load tracks
    println!("[T+{:?}] Loading tracks...", total_start.elapsed());
    let load_start = Instant::now();

    let tracks = vec![
        create_test_track("1", track1_path.to_str().unwrap()),
        create_test_track("2", track2_path.to_str().unwrap()),
    ];

    playback
        .send_command(PlaybackCommand::LoadPlaylist {
            tracks: tracks,
            start_index: 0,
        })
        .expect("Failed to load playlist");

    let load_duration = load_start.elapsed();
    println!(
        "[T+{:?}] ✓ Tracks loaded (took {:?})",
        total_start.elapsed(),
        load_duration
    );

    // Phase 3: Start playback
    println!("[T+{:?}] Calling play()...", total_start.elapsed());
    let play_start = Instant::now();

    playback
        .send_command(PlaybackCommand::Play)
        .expect("Failed to play");

    let play_call_duration = play_start.elapsed();
    println!(
        "[T+{:?}] ✓ play() returned (took {:?})",
        total_start.elapsed(),
        play_call_duration
    );

    // Phase 4: Wait for actual audio to start
    println!(
        "[T+{:?}] Waiting for audio to start...",
        total_start.elapsed()
    );
    let audio_wait_start = Instant::now();

    let audio_started = wait_for_event(
        &playback,
        |event| matches!(event, PlaybackEvent::StateChanged(PlaybackState::Playing)),
        Duration::from_secs(5),
    );

    let audio_wait_duration = audio_wait_start.elapsed();
    let total_duration = total_start.elapsed();

    // Results
    println!("\n=== TIMING BREAKDOWN ===");
    println!("1. Audio engine init: {:?}", init_duration);
    println!("2. Load tracks:       {:?}", load_duration);
    println!("3. play() call:       {:?}", play_call_duration);
    println!("4. Wait for audio:    {:?}", audio_wait_duration);
    println!("---");
    println!("TOTAL (init → audio): {:?}", total_duration);

    if audio_started {
        println!("\n✓ SUCCESS: Audio started playing");

        if total_duration > Duration::from_secs(2) {
            println!("⚠️  WARNING: Total time exceeds 2 seconds!");
            println!("   Expected: < 2s for immediate playback");
            println!("   This indicates a delay issue.");
        } else if total_duration > Duration::from_millis(500) {
            println!("⚠️  NOTE: Total time > 500ms");
            println!("   For instant-feel playback, aim for < 500ms");
        } else {
            println!("✓ Excellent: Total time < 500ms (instant playback feel)");
        }
    } else {
        println!("\n✗ FAILURE: Audio never started (timeout after 5s)");
    }

    // Cleanup
    playback.send_command(PlaybackCommand::Stop).ok();
    std::thread::sleep(Duration::from_millis(200));

    assert!(
        audio_started,
        "Audio should have started playing within 5 seconds"
    );
}

#[test]
fn test_warm_start_immediate_play() {
    println!("\n=== E2E Test: WARM START - With Device Pre-Warming ===\n");

    let total_start = Instant::now();

    // Get absolute paths to test files
    // Test runs from crate root (libraries/soul-audio-desktop), not repo root
    let base_dir = std::env::current_dir().expect("Failed to get current directory");
    let track1_path = base_dir.join("test_data/track_1.wav");
    let track2_path = base_dir.join("test_data/track_2.wav");

    assert!(
        track1_path.exists(),
        "Test file not found: {}",
        track1_path.display()
    );
    assert!(
        track2_path.exists(),
        "Test file not found: {}",
        track2_path.display()
    );

    // Phase 1: Initialize audio engine
    println!("[T+0ms] Initializing audio engine...");
    let init_start = Instant::now();

    let config = PlaybackConfig::default();
    let playback = DesktopPlayback::new(config).expect("Failed to create DesktopPlayback");

    let init_duration = init_start.elapsed();
    println!(
        "[T+{:?}] ✓ Audio engine initialized (took {:?})",
        total_start.elapsed(),
        init_duration
    );

    // Phase 2: Warm-up cycle (simulates main.rs warm-up)
    println!("[T+{:?}] Running warm-up cycle...", total_start.elapsed());
    let warmup_start = Instant::now();

    // Trigger device initialization by trying to play (will fail with no tracks)
    let _ = playback.send_command(PlaybackCommand::Play);
    std::thread::sleep(Duration::from_millis(50));
    let _ = playback.send_command(PlaybackCommand::Stop);

    let warmup_duration = warmup_start.elapsed();
    println!(
        "[T+{:?}] ✓ Warm-up complete (took {:?})",
        total_start.elapsed(),
        warmup_duration
    );

    // Phase 3: Load tracks
    println!("[T+{:?}] Loading tracks...", total_start.elapsed());
    let load_start = Instant::now();

    let tracks = vec![
        create_test_track("1", track1_path.to_str().unwrap()),
        create_test_track("2", track2_path.to_str().unwrap()),
    ];

    playback
        .send_command(PlaybackCommand::LoadPlaylist {
            tracks: tracks,
            start_index: 0,
        })
        .expect("Failed to load playlist");

    let load_duration = load_start.elapsed();
    println!(
        "[T+{:?}] ✓ Tracks loaded (took {:?})",
        total_start.elapsed(),
        load_duration
    );

    // Phase 4: Start playback
    println!("[T+{:?}] Calling play()...", total_start.elapsed());
    let play_start = Instant::now();

    playback
        .send_command(PlaybackCommand::Play)
        .expect("Failed to play");

    let play_call_duration = play_start.elapsed();
    println!(
        "[T+{:?}] ✓ play() returned (took {:?})",
        total_start.elapsed(),
        play_call_duration
    );

    // Phase 5: Wait for actual audio to start
    println!(
        "[T+{:?}] Waiting for audio to start...",
        total_start.elapsed()
    );
    let audio_wait_start = Instant::now();

    let audio_started = wait_for_event(
        &playback,
        |event| matches!(event, PlaybackEvent::StateChanged(PlaybackState::Playing)),
        Duration::from_secs(5),
    );

    let audio_wait_duration = audio_wait_start.elapsed();
    let total_duration = total_start.elapsed();

    // Results
    println!("\n=== TIMING BREAKDOWN ===");
    println!("1. Audio engine init: {:?}", init_duration);
    println!("2. Warm-up cycle:     {:?}", warmup_duration);
    println!("3. Load tracks:       {:?}", load_duration);
    println!("4. play() call:       {:?}", play_call_duration);
    println!("5. Wait for audio:    {:?}", audio_wait_duration);
    println!("---");
    println!("TOTAL (init → audio): {:?}", total_duration);

    if audio_started {
        println!("\n✓ SUCCESS: Audio started playing");

        if total_duration > Duration::from_secs(2) {
            println!("⚠️  WARNING: Total time exceeds 2 seconds even with warm-up!");
            println!("   Expected: < 2s with device pre-warming");
            println!("   Warm-up may not be effective.");
        } else if total_duration > Duration::from_millis(500) {
            println!("⚠️  NOTE: Total time > 500ms even with warm-up");
            println!("   Consider optimizing load or play phases");
        } else {
            println!("✓ Excellent: Total time < 500ms with warm-up");
        }
    } else {
        println!("\n✗ FAILURE: Audio never started (timeout after 5s)");
    }

    // Cleanup
    playback.send_command(PlaybackCommand::Stop).ok();
    std::thread::sleep(Duration::from_millis(200));

    assert!(
        audio_started,
        "Audio should have started playing within 5 seconds"
    );
}

#[test]
fn test_user_immediate_play_simulation() {
    println!("\n=== E2E Test: Simulating Impatient User (Play at T+100ms) ===\n");
    println!("This simulates a user clicking play immediately after app starts");

    let app_start = Instant::now();

    // Get absolute path to test file
    // Test runs from crate root (libraries/soul-audio-desktop), not repo root
    let base_dir = std::env::current_dir().expect("Failed to get current directory");
    let track1_path = base_dir.join("test_data/track_1.wav");
    assert!(
        track1_path.exists(),
        "Test file not found: {}",
        track1_path.display()
    );

    // Simulate async initialization
    println!("[T+0ms] App starting, spawning async audio init...");

    let config = PlaybackConfig::default();
    let playback = DesktopPlayback::new(config).expect("Failed to create DesktopPlayback");

    println!("[T+{:?}] Audio engine created", app_start.elapsed());

    // Warm-up in background
    let warmup_start = Instant::now();
    let _ = playback.send_command(PlaybackCommand::Play);
    std::thread::sleep(Duration::from_millis(50));
    let _ = playback.send_command(PlaybackCommand::Stop);
    println!(
        "[T+{:?}] Warm-up complete (took {:?})",
        app_start.elapsed(),
        warmup_start.elapsed()
    );

    // User clicks play at T+100ms (very impatient!)
    std::thread::sleep(Duration::from_millis(100));
    println!(
        "\n[T+{:?}] 👤 USER ACTION: Clicks play button",
        app_start.elapsed()
    );

    let user_action_time = Instant::now();

    // Load and play
    let tracks = vec![create_test_track("1", track1_path.to_str().unwrap())];

    playback
        .send_command(PlaybackCommand::LoadPlaylist {
            tracks: tracks,
            start_index: 0,
        })
        .expect("Failed to load playlist");

    playback
        .send_command(PlaybackCommand::Play)
        .expect("Failed to play");

    println!(
        "[T+{:?}] Commands sent, waiting for audio...",
        app_start.elapsed()
    );

    // Wait for audio
    let audio_started = wait_for_event(
        &playback,
        |event| matches!(event, PlaybackEvent::StateChanged(PlaybackState::Playing)),
        Duration::from_secs(5),
    );

    let user_perceived_delay = user_action_time.elapsed();
    let total_time = app_start.elapsed();

    println!("\n=== RESULTS ===");
    println!("User clicked play at: T+100ms");
    println!("Audio started at:     T+{:?}", total_time);
    println!("---");
    println!("USER PERCEIVED DELAY: {:?}", user_perceived_delay);

    if audio_started {
        println!("\n✓ Audio is playing");

        if user_perceived_delay > Duration::from_secs(3) {
            println!("❌ CRITICAL: User waited > 3 seconds!");
            println!("   This is unacceptable UX - feels broken");
        } else if user_perceived_delay > Duration::from_secs(1) {
            println!("⚠️  WARNING: User waited > 1 second");
            println!("   This feels slow and frustrating");
        } else if user_perceived_delay > Duration::from_millis(500) {
            println!("⚠️  OK: User waited > 500ms");
            println!("   Noticeable delay but acceptable");
        } else {
            println!("✓ EXCELLENT: User waited < 500ms");
            println!("   Feels instant and responsive");
        }
    } else {
        println!("\n✗ FAILURE: Audio never started");
    }

    // Cleanup
    playback.send_command(PlaybackCommand::Stop).ok();
    std::thread::sleep(Duration::from_millis(200));

    assert!(audio_started, "Audio should have started");
    assert!(
        user_perceived_delay < Duration::from_secs(3),
        "User perceived delay should be < 3s for acceptable UX"
    );
}
