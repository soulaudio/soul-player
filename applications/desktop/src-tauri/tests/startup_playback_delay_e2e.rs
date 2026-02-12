//! E2E test simulating app startup and immediate playback
//!
//! This test verifies the actual delay from app start to audio playback,
//! measuring each step to identify bottlenecks.

use soul_audio_desktop::{DesktopPlayback, PlaybackCommand, PlaybackEvent};
use soul_playback::{PlaybackConfig, QueueTrack, TrackSource};
use std::fs::{self, File};
use std::io::Write as IoWrite;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::OnceCell;

/// Generate a minimal test WAV file
fn generate_test_wav(path: &PathBuf, duration_secs: f64, frequency: f64) -> std::io::Result<()> {
    let sample_rate = 44100;
    let num_samples = (sample_rate as f64 * duration_secs) as usize;
    let channels = 2;

    let mut file = File::create(path)?;

    // RIFF header
    file.write_all(b"RIFF")?;
    let file_size = 36 + num_samples * channels * 2;
    file.write_all(&(file_size as u32).to_le_bytes())?;
    file.write_all(b"WAVE")?;

    // fmt chunk
    file.write_all(b"fmt ")?;
    file.write_all(&16u32.to_le_bytes())?;
    file.write_all(&1u16.to_le_bytes())?;
    file.write_all(&(channels as u16).to_le_bytes())?;
    file.write_all(&(sample_rate as u32).to_le_bytes())?;
    file.write_all(&((sample_rate * channels * 2) as u32).to_le_bytes())?;
    file.write_all(&((channels * 2) as u16).to_le_bytes())?;
    file.write_all(&16u16.to_le_bytes())?;

    // data chunk
    file.write_all(b"data")?;
    file.write_all(&((num_samples * channels * 2) as u32).to_le_bytes())?;

    for i in 0..num_samples {
        let t = i as f64 / sample_rate as f64;
        let sample = (t * frequency * 2.0 * std::f64::consts::PI).sin();
        let sample_i16 = (sample * 32767.0) as i16;

        file.write_all(&sample_i16.to_le_bytes())?;
        file.write_all(&sample_i16.to_le_bytes())?;
    }

    Ok(())
}

/// Simulates the actual PlaybackManager wrapper from main.rs
struct PlaybackManager {
    playback: DesktopPlayback,
}

impl PlaybackManager {
    /// Simulates PlaybackManager::new() from main.rs
    fn new() -> Result<Self, String> {
        let config = PlaybackConfig::default();
        let playback = DesktopPlayback::new(config).map_err(|e| e.to_string())?;

        Ok(Self { playback })
    }

    fn play(&self) -> Result<(), String> {
        self.playback
            .send_command(PlaybackCommand::Play)
            .map_err(|e| e.to_string())
    }

    fn stop(&self) -> Result<(), String> {
        self.playback
            .send_command(PlaybackCommand::Stop)
            .map_err(|e| e.to_string())
    }

    fn load_playlist(&self, tracks: Vec<QueueTrack>) -> Result<(), String> {
        self.playback
            .send_command(PlaybackCommand::LoadPlaylist(tracks))
            .map_err(|e| e.to_string())
    }

    fn try_recv_event(&self) -> Option<PlaybackEvent> {
        self.playback.try_recv_event()
    }
}

#[tokio::test]
async fn test_startup_to_first_playback_delay() {
    println!("\n=== E2E Test: App Startup → First Playback ===\n");

    // Setup: Create test audio file
    let test_dir = PathBuf::from("test_startup_audio");
    fs::create_dir_all(&test_dir).expect("Failed to create test directory");

    let track_path = test_dir.join("test_track.wav");
    generate_test_wav(&track_path, 2.0, 440.0).expect("Failed to generate test WAV");

    let test_track = QueueTrack {
        id: "test_1".to_string(),
        path: track_path.clone(),
        title: "Test Track".to_string(),
        artist: "Test Artist".to_string(),
        album: Some("Test Album".to_string()),
        duration: Duration::from_secs(2),
        track_number: Some(1),
        source: TrackSource::Single,
    };

    // === PHASE 1: App Startup (Simulating main.rs async initialization) ===
    let app_start = Instant::now();
    println!("[T+0ms] App startup begins...");

    // Simulate the Arc<OnceCell> pattern from main.rs
    let playback_cell = Arc::new(OnceCell::<PlaybackManager>::new());
    let cell_for_init = playback_cell.clone();

    // Simulate async initialization spawned in main.rs
    println!(
        "[T+{:?}] Spawning async audio engine initialization...",
        app_start.elapsed()
    );
    let init_start = Instant::now();

    let init_handle = tokio::spawn(async move {
        println!(
            "[INIT T+{:?}] Starting PlaybackManager::new()...",
            init_start.elapsed()
        );
        let pm_creation_start = Instant::now();

        let result = PlaybackManager::new();
        let pm_creation_duration = pm_creation_start.elapsed();

        match result {
            Ok(pm) => {
                println!(
                    "[INIT T+{:?}] PlaybackManager created successfully (took {:?})",
                    init_start.elapsed(),
                    pm_creation_duration
                );

                // Store in OnceCell
                if let Err(_) = cell_for_init.set(pm) {
                    eprintln!("[INIT] Failed to set PlaybackManager in OnceCell");
                    return Err("Failed to set in OnceCell".to_string());
                }

                // Simulate warm-up (play/stop cycle from main.rs)
                println!(
                    "[INIT T+{:?}] Starting warm-up cycle...",
                    init_start.elapsed()
                );
                let warmup_start = Instant::now();

                if let Some(pm) = cell_for_init.get() {
                    let _ = pm.play(); // Will fail (no tracks), but initializes device
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    let _ = pm.stop();
                }

                let warmup_duration = warmup_start.elapsed();
                println!(
                    "[INIT T+{:?}] Warm-up complete (took {:?})",
                    init_start.elapsed(),
                    warmup_duration
                );

                Ok(init_start.elapsed())
            }
            Err(e) => {
                eprintln!(
                    "[INIT T+{:?}] Failed to initialize: {}",
                    init_start.elapsed(),
                    e
                );
                Err(e)
            }
        }
    });

    // === PHASE 2: User Clicks Play (Immediately, simulating impatient user) ===
    // Wait a bit to simulate realistic startup, but still very fast
    tokio::time::sleep(Duration::from_millis(100)).await;

    println!(
        "\n[T+{:?}] USER ACTION: Click play button",
        app_start.elapsed()
    );
    let play_command_time = Instant::now();

    // Try to get playback manager (simulating the command handler)
    let playback = match playback_cell.get() {
        Some(pm) => {
            println!(
                "[T+{:?}] ✓ Audio engine ready (instant access from OnceCell)",
                app_start.elapsed()
            );
            pm
        }
        None => {
            println!(
                "[T+{:?}] ⚠️  Audio engine still initializing, waiting...",
                app_start.elapsed()
            );

            // Wait for init to complete
            while playback_cell.get().is_none() {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }

            let wait_duration = play_command_time.elapsed();
            println!(
                "[T+{:?}] ✓ Audio engine ready after waiting {:?}",
                app_start.elapsed(),
                wait_duration
            );

            playback_cell.get().unwrap()
        }
    };

    // === PHASE 3: Load Queue and Start Playback ===
    println!(
        "\n[T+{:?}] Loading track into queue...",
        app_start.elapsed()
    );
    let queue_load_start = Instant::now();

    playback
        .load_playlist(vec![test_track])
        .expect("Failed to load playlist");
    let queue_load_duration = queue_load_start.elapsed();

    println!(
        "[T+{:?}] Queue loaded (took {:?})",
        app_start.elapsed(),
        queue_load_duration
    );

    println!("[T+{:?}] Calling play()...", app_start.elapsed());
    let play_call_start = Instant::now();

    playback.play().expect("Failed to start playback");
    let play_call_duration = play_call_start.elapsed();

    println!(
        "[T+{:?}] play() returned (took {:?})",
        app_start.elapsed(),
        play_call_duration
    );

    // === PHASE 4: Wait for Audio to Actually Start ===
    println!(
        "[T+{:?}] Waiting for StateChanged(Playing) event...",
        app_start.elapsed()
    );
    let mut audio_started = false;
    let event_wait_start = Instant::now();

    for _ in 0..100 {
        tokio::time::sleep(Duration::from_millis(10)).await;

        while let Some(event) = playback.try_recv_event() {
            match event {
                PlaybackEvent::StateChanged(state) => {
                    println!(
                        "[T+{:?}] Event: StateChanged({:?})",
                        app_start.elapsed(),
                        state
                    );
                    if matches!(state, soul_playback::PlaybackState::Playing) {
                        audio_started = true;
                        break;
                    }
                }
                PlaybackEvent::Error(e) => {
                    println!("[T+{:?}] Event: Error({})", app_start.elapsed(), e);
                }
                _ => {
                    println!("[T+{:?}] Event: {:?}", app_start.elapsed(), event);
                }
            }
        }

        if audio_started {
            break;
        }
    }

    let audio_start_duration = event_wait_start.elapsed();
    let total_duration = app_start.elapsed();

    // === RESULTS ===
    println!("\n=== TIMING BREAKDOWN ===");

    let init_result = init_handle.await.expect("Init task panicked");
    if let Ok(init_duration) = init_result {
        println!("1. Audio engine initialization: {:?}", init_duration);
    }
    println!(
        "2. User clicked play at: T+{:?}",
        play_command_time.duration_since(app_start)
    );
    println!("3. Queue load time: {:?}", queue_load_duration);
    println!("4. play() call time: {:?}", play_call_duration);
    println!("5. Wait for audio to start: {:?}", audio_start_duration);
    println!(
        "\n🎵 TOTAL TIME (app start → audio playing): {:?}",
        total_duration
    );

    if audio_started {
        println!("✓ SUCCESS: Audio started playing");
    } else {
        println!(
            "✗ FAILURE: Audio never started (timeout after {:?})",
            audio_start_duration
        );
    }

    // Assertions
    assert!(audio_started, "Audio should have started playing");

    // Check if total time is reasonable (should be < 2s for immediate play)
    if total_duration > Duration::from_secs(2) {
        println!(
            "\n⚠️  WARNING: Total time ({:?}) exceeds 2 seconds!",
            total_duration
        );
        println!("   This indicates a delay issue that needs investigation.");
    } else {
        println!("\n✓ Total time is acceptable (< 2s)");
    }

    // Cleanup
    playback.stop().ok();
    tokio::time::sleep(Duration::from_millis(100)).await;
    fs::remove_dir_all(&test_dir).ok();
}

#[tokio::test]
async fn test_cold_start_vs_warm_playback() {
    println!("\n=== E2E Test: Cold Start vs Warmed-Up Playback ===\n");

    // Setup
    let test_dir = PathBuf::from("test_warmup_audio");
    fs::create_dir_all(&test_dir).expect("Failed to create test directory");

    let track_path = test_dir.join("test_track.wav");
    generate_test_wav(&track_path, 1.0, 440.0).expect("Failed to generate test WAV");

    let test_track = QueueTrack {
        id: "warmup_test".to_string(),
        path: track_path.clone(),
        title: "Warmup Test".to_string(),
        artist: "Test".to_string(),
        album: None,
        duration: Duration::from_secs(1),
        track_number: None,
        source: TrackSource::Single,
    };

    // === TEST 1: Cold Start (no warm-up) ===
    println!("=== Test 1: COLD START (no warm-up) ===");
    let cold_start = Instant::now();

    let pm_cold = PlaybackManager::new().expect("Failed to create PlaybackManager");
    let cold_init_duration = cold_start.elapsed();
    println!("Cold init took: {:?}", cold_init_duration);

    pm_cold
        .load_playlist(vec![test_track.clone()])
        .expect("Failed to load");

    let cold_play_start = Instant::now();
    pm_cold.play().expect("Failed to play");

    // Wait for actual audio start
    tokio::time::sleep(Duration::from_millis(50)).await;
    let mut cold_audio_started = false;
    while let Some(event) = pm_cold.try_recv_event() {
        if matches!(
            event,
            PlaybackEvent::StateChanged(soul_playback::PlaybackState::Playing)
        ) {
            cold_audio_started = true;
            break;
        }
    }

    let cold_play_duration = cold_play_start.elapsed();
    println!("Cold play() → audio start: {:?}", cold_play_duration);

    pm_cold.stop().ok();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // === TEST 2: Warm Start (with warm-up cycle) ===
    println!("\n=== Test 2: WARM START (with warm-up) ===");
    let warm_start = Instant::now();

    let pm_warm = PlaybackManager::new().expect("Failed to create PlaybackManager");
    let warm_init_duration = warm_start.elapsed();
    println!("Warm init took: {:?}", warm_init_duration);

    // Warm-up cycle
    println!("Running warm-up cycle...");
    let warmup_start = Instant::now();
    let _ = pm_warm.play(); // No tracks, will error but initializes device
    tokio::time::sleep(Duration::from_millis(50)).await;
    let _ = pm_warm.stop();
    let warmup_duration = warmup_start.elapsed();
    println!("Warm-up cycle took: {:?}", warmup_duration);

    // Now actual playback
    pm_warm
        .load_playlist(vec![test_track.clone()])
        .expect("Failed to load");

    let warm_play_start = Instant::now();
    pm_warm.play().expect("Failed to play");

    // Wait for actual audio start
    tokio::time::sleep(Duration::from_millis(50)).await;
    let mut warm_audio_started = false;
    while let Some(event) = pm_warm.try_recv_event() {
        if matches!(
            event,
            PlaybackEvent::StateChanged(soul_playback::PlaybackState::Playing)
        ) {
            warm_audio_started = true;
            break;
        }
    }

    let warm_play_duration = warm_play_start.elapsed();
    println!("Warm play() → audio start: {:?}", warm_play_duration);

    // === COMPARISON ===
    println!("\n=== RESULTS ===");
    println!("Cold start playback delay: {:?}", cold_play_duration);
    println!("Warm start playback delay: {:?}", warm_play_duration);

    if warm_play_duration < cold_play_duration {
        let improvement = cold_play_duration - warm_play_duration;
        println!("✓ Warm-up improved startup by: {:?}", improvement);
    } else {
        println!("⚠️  Warm-up did not improve startup time");
    }

    assert!(
        cold_audio_started && warm_audio_started,
        "Both tests should have audio playing"
    );

    // Cleanup
    pm_warm.stop().ok();
    tokio::time::sleep(Duration::from_millis(100)).await;
    fs::remove_dir_all(&test_dir).ok();
}
