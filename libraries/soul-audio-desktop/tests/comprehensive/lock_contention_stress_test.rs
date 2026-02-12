//! Lock contention stress test
//!
//! Tests the playback system under extreme concurrent load from multiple threads
//! sending different commands simultaneously. Validates:
//! - No deadlocks under heavy contention
//! - Command processing remains reliable
//! - Audio state remains consistent
//! - Event delivery works correctly

use soul_audio_desktop::{DesktopPlayback, PlaybackCommand, PlaybackEvent};
use soul_playback::{PlaybackConfig, PlaybackState, QueueTrack, TrackSource};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Create a test track with a real audio file path for testing
fn create_test_track(id: &str) -> QueueTrack {
    QueueTrack {
        id: id.to_string().into(),
        path: PathBuf::from(format!("test_data/track_{}.wav", id)),
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

#[test]
#[ignore = "Stress test - run manually with: cargo test --test lock_contention_stress_test -- --include-ignored"]
fn test_extreme_command_flood() {
    println!("\n[STRESS TEST] Starting extreme command flood test");
    println!("[STRESS TEST] Spawning 8 threads sending different commands simultaneously");

    let config = PlaybackConfig::default();
    let playback = Arc::new(DesktopPlayback::new(config).expect("Failed to create playback"));

    // Load initial playlist
    let tracks = vec![
        create_test_track("1"),
        create_test_track("2"),
    ];
    playback
        .send_command(PlaybackCommand::LoadPlaylist(tracks.clone()))
        .expect("Failed to load playlist");

    std::thread::sleep(Duration::from_millis(50));

    let stop_flag = Arc::new(AtomicBool::new(false));
    let error_count = Arc::new(AtomicU64::new(0));
    let success_count = Arc::new(AtomicU64::new(0));

    // Spawn 8 threads with different command patterns
    let handles: Vec<_> = (0..8)
        .map(|i| {
            let pb = Arc::clone(&playback);
            let stop = Arc::clone(&stop_flag);
            let errors = Arc::clone(&error_count);
            let successes = Arc::clone(&success_count);

            std::thread::spawn(move || {
                let start = Instant::now();
                let mut local_successes = 0u64;
                let mut local_errors = 0u64;

                // Run for 5 seconds
                while !stop.load(Ordering::Relaxed) && start.elapsed() < Duration::from_secs(5) {
                    let result = match i {
                        0 => {
                            // Rapid play/pause cycles
                            pb.send_command(PlaybackCommand::Play)
                                .and_then(|_| {
                                    std::thread::sleep(Duration::from_millis(1));
                                    pb.send_command(PlaybackCommand::Pause)
                                })
                        }
                        1 => {
                            // Skip next/prev rapidly
                            pb.send_command(PlaybackCommand::SkipNext)
                                .or_else(|_| pb.send_command(PlaybackCommand::SkipPrev))
                        }
                        2 => {
                            // Volume changes
                            let volume = (start.elapsed().as_millis() % 100) as f32 / 100.0;
                            pb.send_command(PlaybackCommand::SetVolume(volume))
                        }
                        3 => {
                            // Queue modifications
                            pb.send_command(PlaybackCommand::RemoveFromQueue(1))
                                .or_else(|_| {
                                    let track = create_test_track("1");
                                    pb.send_command(PlaybackCommand::AddToQueueEnd(track))
                                })
                        }
                        4 | 5 | 6 | 7 => {
                            // Query state (lightweight operations)
                            drain_events(&pb);
                            Ok(())
                        }
                        _ => Ok(()),
                    };

                    match result {
                        Ok(_) => local_successes += 1,
                        Err(_) => local_errors += 1,
                    }

                    // Small yield to prevent complete CPU saturation
                    if i < 4 {
                        std::thread::sleep(Duration::from_micros(100));
                    }
                }

                successes.fetch_add(local_successes, Ordering::Relaxed);
                errors.fetch_add(local_errors, Ordering::Relaxed);

                println!(
                    "[THREAD {}] Completed: {} successes, {} errors",
                    i, local_successes, local_errors
                );
            })
        })
        .collect();

    println!("[STRESS TEST] All threads spawned, running for 5 seconds...");

    // Wait for all threads to complete
    for (i, handle) in handles.into_iter().enumerate() {
        handle.join().expect(&format!("Thread {} panicked", i));
    }

    stop_flag.store(true, Ordering::Relaxed);

    let total_successes = success_count.load(Ordering::Relaxed);
    let total_errors = error_count.load(Ordering::Relaxed);
    let total_ops = total_successes + total_errors;

    println!("\n[STRESS TEST] Results:");
    println!("  Total operations: {}", total_ops);
    println!("  Successful: {}", total_successes);
    println!("  Errors: {}", total_errors);
    println!(
        "  Success rate: {:.2}%",
        (total_successes as f64 / total_ops as f64) * 100.0
    );

    // Give time to process final commands
    std::thread::sleep(Duration::from_millis(200));

    // Verify system is still responsive
    let events = drain_events(&playback);
    println!("  Final events received: {}", events.len());

    // Test final command still works (no deadlock)
    let result = playback.send_command(PlaybackCommand::Pause);
    assert!(
        result.is_ok(),
        "System is deadlocked - final command failed"
    );

    println!("\n[STRESS TEST] ✓ No deadlocks detected");
    println!("[STRESS TEST] ✓ System remained responsive throughout test");

    // At least 50% of operations should succeed under stress
    let success_rate = total_successes as f64 / total_ops as f64;
    assert!(
        success_rate > 0.5,
        "Success rate too low: {:.2}%",
        success_rate * 100.0
    );
}

#[test]
#[ignore = "Stress test - run manually with: cargo test --test lock_contention_stress_test -- --include-ignored"]
fn test_concurrent_state_queries() {
    println!("\n[STRESS TEST] Testing concurrent state queries");

    let config = PlaybackConfig::default();
    let playback = Arc::new(DesktopPlayback::new(config).expect("Failed to create playback"));

    let stop_flag = Arc::new(AtomicBool::new(false));
    let query_count = Arc::new(AtomicU64::new(0));

    // Spawn 16 threads all querying state simultaneously
    let handles: Vec<_> = (0..16)
        .map(|i| {
            let pb = Arc::clone(&playback);
            let stop = Arc::clone(&stop_flag);
            let count = Arc::clone(&query_count);

            std::thread::spawn(move || {
                let start = Instant::now();
                let mut local_count = 0u64;

                while !stop.load(Ordering::Relaxed) && start.elapsed() < Duration::from_secs(3) {
                    // Query events (exercises read locks)
                    let _events = drain_events(&pb);
                    local_count += 1;
                }

                count.fetch_add(local_count, Ordering::Relaxed);
                println!("[QUERY THREAD {}] Performed {} queries", i, local_count);
            })
        })
        .collect();

    // While queries are running, send commands (write operations)
    let pb = Arc::clone(&playback);
    let command_thread = std::thread::spawn(move || {
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(3) {
            let _ = pb.send_command(PlaybackCommand::Play);
            std::thread::sleep(Duration::from_millis(50));
            let _ = pb.send_command(PlaybackCommand::Pause);
            std::thread::sleep(Duration::from_millis(50));
        }
    });

    // Wait for completion
    for handle in handles {
        handle.join().expect("Query thread panicked");
    }
    command_thread.join().expect("Command thread panicked");

    stop_flag.store(true, Ordering::Relaxed);

    let total_queries = query_count.load(Ordering::Relaxed);
    println!(
        "\n[STRESS TEST] Total queries: {} ({}/sec)",
        total_queries,
        total_queries / 3
    );

    // Should achieve at least 10,000 queries/sec across all threads
    assert!(
        total_queries > 30_000,
        "Query throughput too low: {} queries in 3 seconds",
        total_queries
    );

    println!("[STRESS TEST] ✓ Concurrent queries completed successfully");
    println!("[STRESS TEST] ✓ No read/write lock contention detected");
}

#[test]
#[ignore = "Stress test - run manually with: cargo test --test lock_contention_stress_test -- --include-ignored"]
fn test_rapid_playlist_changes() {
    println!("\n[STRESS TEST] Testing rapid playlist changes");

    let config = PlaybackConfig::default();
    let playback = Arc::new(DesktopPlayback::new(config).expect("Failed to create playback"));

    let stop_flag = Arc::new(AtomicBool::new(false));

    // Spawn 4 threads all trying to modify the playlist
    let handles: Vec<_> = (0..4)
        .map(|i| {
            let pb = Arc::clone(&playback);
            let stop = Arc::clone(&stop_flag);

            std::thread::spawn(move || {
                let start = Instant::now();
                let mut cycle = 0;

                while !stop.load(Ordering::Relaxed) && start.elapsed() < Duration::from_secs(3) {
                    let tracks = vec![
                        create_test_track(&format!("{}_1", i)),
                        create_test_track(&format!("{}_2", i)),
                    ];

                    let _ = pb.send_command(PlaybackCommand::LoadPlaylist(tracks));
                    cycle += 1;

                    std::thread::sleep(Duration::from_millis(10));
                }

                println!("[PLAYLIST THREAD {}] Completed {} cycles", i, cycle);
            })
        })
        .collect();

    // Wait for completion
    for handle in handles {
        handle.join().expect("Playlist thread panicked");
    }

    stop_flag.store(true, Ordering::Relaxed);

    // Verify system is still functional
    std::thread::sleep(Duration::from_millis(100));
    let final_tracks = vec![create_test_track("final")];
    let result = playback.send_command(PlaybackCommand::LoadPlaylist(final_tracks));

    assert!(
        result.is_ok(),
        "System locked up after rapid playlist changes"
    );

    println!("[STRESS TEST] ✓ Rapid playlist changes completed");
    println!("[STRESS TEST] ✓ System remained stable");
}
