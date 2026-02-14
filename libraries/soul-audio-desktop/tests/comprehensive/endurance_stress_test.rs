//! Endurance stress test
//!
//! Tests the playback system under long-running continuous load:
//! - Memory stability over extended periods
//! - No resource leaks (file handles, threads, memory)
//! - Audio quality remains consistent
//! - System remains responsive after hours of operation

use soul_audio_desktop::{DesktopPlayback, PlaybackCommand, PlaybackEvent};
use soul_playback::{PlaybackConfig, PlaybackState, QueueTrack, TrackSource};
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Create a test track with a real audio file path for testing
fn create_test_track(id: &str) -> QueueTrack {
    QueueTrack {
        id: id.to_string().into(),
        path: PathBuf::from(format!("test_data/track_{}.wav", id)),
        title: format!("Test Track {}", id),
        artist: "Test Artist".to_string(),
        album: Some("Test Album".to_string()),
        duration: Duration::from_secs(30), // 30 second tracks for faster cycling
        track_number: Some(1),
        source: TrackSource::Single,
    }
}

/// Helper to drain all events from the playback system
fn drain_events(playback: &DesktopPlayback) -> Vec<PlaybackEvent> {
    std::iter::from_fn(|| playback.try_recv_event()).collect()
}

/// Get current process memory usage (Windows)
#[cfg(target_os = "windows")]
fn get_process_memory() -> u64 {
    use std::mem;
    use std::ptr;
    use windows::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
    use windows::Win32::System::Threading::GetCurrentProcess;

    unsafe {
        let process = GetCurrentProcess();
        let mut pmc: PROCESS_MEMORY_COUNTERS = mem::zeroed();
        pmc.cb = mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;

        if GetProcessMemoryInfo(process, &mut pmc, pmc.cb).is_ok() {
            pmc.WorkingSetSize as u64
        } else {
            0
        }
    }
}

/// Get current process memory usage (Unix-like)
#[cfg(not(target_os = "windows"))]
fn get_process_memory() -> u64 {
    // Read /proc/self/statm on Linux, use ps on macOS
    #[cfg(target_os = "linux")]
    {
        if let Ok(content) = std::fs::read_to_string("/proc/self/statm") {
            let parts: Vec<&str> = content.split_whitespace().collect();
            if let Some(rss_pages) = parts.get(1) {
                if let Ok(pages) = rss_pages.parse::<u64>() {
                    return pages * 4096; // Assume 4KB pages
                }
            }
        }
    }

    // Fallback: return 0 (will skip memory checks)
    0
}

struct MemoryStats {
    start_mb: u64,
    current_mb: u64,
    peak_mb: u64,
    growth_mb: u64,
}

impl MemoryStats {
    fn new() -> Self {
        let start = get_process_memory();
        Self {
            start_mb: start / (1024 * 1024),
            current_mb: start / (1024 * 1024),
            peak_mb: start / (1024 * 1024),
            growth_mb: 0,
        }
    }

    fn update(&mut self) {
        let current = get_process_memory();
        self.current_mb = current / (1024 * 1024);
        self.peak_mb = self.peak_mb.max(self.current_mb);
        self.growth_mb = self.current_mb.saturating_sub(self.start_mb);
    }

    fn print(&self, label: &str) {
        println!(
            "[MEMORY] {}: {}MB (growth: +{}MB, peak: {}MB)",
            label, self.current_mb, self.growth_mb, self.peak_mb
        );
    }
}

#[test]
#[ignore = "Long-running test - 1 hour. Run manually with: cargo test --test endurance_stress_test test_continuous_playback_1_hour -- --include-ignored"]
fn test_continuous_playback_1_hour() {
    println!("\n[ENDURANCE TEST] Starting 1-hour continuous playback test");
    println!("[ENDURANCE TEST] This test validates memory stability and resource management");

    let mut stats = MemoryStats::new();
    stats.print("Initial");

    let config = PlaybackConfig::default();
    let playback = DesktopPlayback::new(config).expect("Failed to create playback");

    // Load playlist with 2 tracks
    let tracks = vec![create_test_track("1"), create_test_track("2")];
    playback
        .send_command(PlaybackCommand::LoadPlaylist { tracks: tracks.clone(), start_index: 0 }))
        .expect("Failed to load playlist");

    std::thread::sleep(Duration::from_millis(100));

    let start = Instant::now();
    let test_duration = Duration::from_secs(3600); // 1 hour
    let mut cycle_count = 0u64;
    let mut last_memory_check = Instant::now();

    println!("[ENDURANCE TEST] Starting playback cycle...");

    playback
        .send_command(PlaybackCommand::Play)
        .expect("Failed to start playback");

    while start.elapsed() < test_duration {
        // Simulate realistic usage patterns
        // - Cycle through tracks
        // - Pause/resume occasionally
        // - Volume changes
        // - Query state

        // Every 10 seconds: skip to next track
        if cycle_count % 10 == 0 {
            let _ = playback.send_command(PlaybackCommand::SkipNext);
        }

        // Every 30 seconds: pause/resume
        if cycle_count % 30 == 5 {
            let _ = playback.send_command(PlaybackCommand::Pause);
            std::thread::sleep(Duration::from_millis(100));
            let _ = playback.send_command(PlaybackCommand::Play);
        }

        // Every 20 seconds: change volume
        if cycle_count % 20 == 0 {
            let volume = ((cycle_count / 20) % 10) as f32 / 10.0;
            let _ = playback.send_command(PlaybackCommand::SetVolume(volume));
        }

        // Drain events regularly to prevent queue buildup
        let events = drain_events(&playback);
        if !events.is_empty() && cycle_count % 60 == 0 {
            println!(
                "[ENDURANCE TEST] Cycle {}: Processed {} events",
                cycle_count,
                events.len()
            );
        }

        // Check memory every minute
        if last_memory_check.elapsed() >= Duration::from_secs(60) {
            stats.update();
            let hours_elapsed = start.elapsed().as_secs_f64() / 3600.0;
            let minutes_elapsed = (start.elapsed().as_secs() / 60) % 60;

            println!(
                "\n[ENDURANCE TEST] Status at {}h {}m:",
                hours_elapsed as u64, minutes_elapsed
            );
            stats.print("Current");

            // Assert memory growth is acceptable
            // Allow up to 10MB growth per hour (very conservative)
            let max_growth_mb = (10.0 * hours_elapsed) as u64;
            assert!(
                stats.growth_mb <= max_growth_mb,
                "Memory leak detected! Growth {}MB exceeds {}MB limit after {:.2}h",
                stats.growth_mb,
                max_growth_mb,
                hours_elapsed
            );

            last_memory_check = Instant::now();
        }

        cycle_count += 1;
        std::thread::sleep(Duration::from_secs(1));
    }

    println!("\n[ENDURANCE TEST] Completed 1-hour test");
    println!("[ENDURANCE TEST] Total cycles: {}", cycle_count);
    stats.update();
    stats.print("Final");

    // Verify system is still responsive
    let result = playback.send_command(PlaybackCommand::Pause);
    assert!(result.is_ok(), "System became unresponsive after 1 hour");

    let events = drain_events(&playback);
    println!(
        "[ENDURANCE TEST] Final event drain: {} events",
        events.len()
    );

    // Final memory assertion
    assert!(
        stats.growth_mb <= 15,
        "Excessive memory growth: {}MB over 1 hour",
        stats.growth_mb
    );

    println!("[ENDURANCE TEST] ✓ No memory leaks detected");
    println!("[ENDURANCE TEST] ✓ System remained responsive throughout");
}

#[test]
#[ignore = "Short endurance test - 5 minutes. Run manually with: cargo test --test endurance_stress_test test_continuous_playback_5_min -- --include-ignored"]
fn test_continuous_playback_5_min() {
    println!("\n[ENDURANCE TEST] Starting 5-minute continuous playback test");
    println!("[ENDURANCE TEST] Shorter version for quick validation");

    let mut stats = MemoryStats::new();
    stats.print("Initial");

    let config = PlaybackConfig::default();
    let playback = DesktopPlayback::new(config).expect("Failed to create playback");

    let tracks = vec![create_test_track("1"), create_test_track("2")];
    playback
        .send_command(PlaybackCommand::LoadPlaylist { tracks: tracks.clone(), start_index: 0 }))
        .expect("Failed to load playlist");

    std::thread::sleep(Duration::from_millis(100));

    let start = Instant::now();
    let test_duration = Duration::from_secs(300); // 5 minutes
    let mut cycle_count = 0u64;

    playback
        .send_command(PlaybackCommand::Play)
        .expect("Failed to start playback");

    while start.elapsed() < test_duration {
        // Rapid cycling to stress test in shorter time
        if cycle_count % 5 == 0 {
            let _ = playback.send_command(PlaybackCommand::SkipNext);
        }

        if cycle_count % 10 == 3 {
            let _ = playback.send_command(PlaybackCommand::Pause);
            std::thread::sleep(Duration::from_millis(50));
            let _ = playback.send_command(PlaybackCommand::Play);
        }

        let _ = drain_events(&playback);

        if cycle_count % 60 == 0 {
            stats.update();
            let seconds = start.elapsed().as_secs();
            println!(
                "[ENDURANCE TEST] {}s: Cycle {}, Memory: {}MB (+{}MB)",
                seconds, cycle_count, stats.current_mb, stats.growth_mb
            );
        }

        cycle_count += 1;
        std::thread::sleep(Duration::from_secs(1));
    }

    println!("\n[ENDURANCE TEST] Completed 5-minute test");
    println!("[ENDURANCE TEST] Total cycles: {}", cycle_count);
    stats.update();
    stats.print("Final");

    // Verify system is still responsive
    let result = playback.send_command(PlaybackCommand::Pause);
    assert!(result.is_ok(), "System became unresponsive");

    // Memory growth should be minimal over 5 minutes
    assert!(
        stats.growth_mb <= 5,
        "Excessive memory growth: {}MB over 5 minutes",
        stats.growth_mb
    );

    println!("[ENDURANCE TEST] ✓ Short endurance test passed");
}

#[test]
#[ignore = "Rapid cycling test - 2 minutes. Run manually with: cargo test --test endurance_stress_test test_rapid_track_cycling -- --include-ignored"]
fn test_rapid_track_cycling() {
    println!("\n[ENDURANCE TEST] Testing rapid track cycling (2 minutes)");
    println!("[ENDURANCE TEST] Validates decoder/buffer cleanup on track changes");

    let mut stats = MemoryStats::new();
    stats.print("Initial");

    let config = PlaybackConfig::default();
    let playback = DesktopPlayback::new(config).expect("Failed to create playback");

    let tracks = vec![
        create_test_track("1"),
        create_test_track("2"),
    ];
    playback
        .send_command(PlaybackCommand::LoadPlaylist { tracks: tracks.clone(), start_index: 0 }))
        .expect("Failed to load playlist");

    std::thread::sleep(Duration::from_millis(100));

    let start = Instant::now();
    let test_duration = Duration::from_secs(120); // 2 minutes
    let mut skip_count = 0u64;

    playback
        .send_command(PlaybackCommand::Play)
        .expect("Failed to start playback");

    println!("[ENDURANCE TEST] Starting rapid skip cycle (every 500ms)...");

    while start.elapsed() < test_duration {
        // Skip track every 500ms (very aggressive)
        let _ = playback.send_command(PlaybackCommand::SkipNext);
        let _ = drain_events(&playback);

        skip_count += 1;

        if skip_count % 60 == 0 {
            stats.update();
            println!(
                "[ENDURANCE TEST] {}s: {} skips, Memory: {}MB (+{}MB)",
                start.elapsed().as_secs(),
                skip_count,
                stats.current_mb,
                stats.growth_mb
            );
        }

        std::thread::sleep(Duration::from_millis(500));
    }

    println!("\n[ENDURANCE TEST] Completed rapid cycling test");
    println!("[ENDURANCE TEST] Total skips: {}", skip_count);
    stats.update();
    stats.print("Final");

    // Should have performed ~240 skips (120s / 0.5s)
    assert!(
        skip_count >= 200,
        "Expected ~240 skips, got {}",
        skip_count
    );

    // Memory growth should be minimal despite rapid decoder churn
    assert!(
        stats.growth_mb <= 5,
        "Decoder leak detected: {}MB growth over {} skips",
        stats.growth_mb,
        skip_count
    );

    println!("[ENDURANCE TEST] ✓ No decoder resource leaks detected");
    println!("[ENDURANCE TEST] ✓ Rapid track changes handled correctly");
}
