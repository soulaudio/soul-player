//! Concurrency stress tests for PlaybackManager
//!
//! Tests multi-threaded scenarios to detect race conditions, deadlocks, and memory issues.
//! Run with: cargo test --test concurrency_stress_test -- --include-ignored

use soul_playback::{
    AudioSource, PlaybackConfig, PlaybackError, PlaybackManager, PlaybackState, QueueTrack, Result,
    ShuffleMode, TrackSource,
};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

// ============================================================================
// Test Infrastructure
// ============================================================================

/// Mock audio source for concurrency testing
struct ConcurrencyMockSource {
    duration: Duration,
    position: Duration,
    sample_rate: u32,
    samples_per_second: u64,
    finished: bool,
    read_count: Arc<AtomicUsize>,
    /// Simulate slow operations
    delay_micros: u64,
}

impl ConcurrencyMockSource {
    fn new(duration: Duration, sample_rate: u32) -> Self {
        Self {
            duration,
            position: Duration::ZERO,
            sample_rate,
            samples_per_second: sample_rate as u64 * 2, // Stereo
            finished: false,
            read_count: Arc::new(AtomicUsize::new(0)),
            delay_micros: 0,
        }
    }

    fn with_delay(mut self, delay_micros: u64) -> Self {
        self.delay_micros = delay_micros;
        self
    }

    fn with_counters(mut self, read_count: Arc<AtomicUsize>) -> Self {
        self.read_count = read_count;
        self
    }
}

impl AudioSource for ConcurrencyMockSource {
    fn read_samples(&mut self, buffer: &mut [f32]) -> Result<usize> {
        self.read_count.fetch_add(1, Ordering::SeqCst);

        // Simulate processing delay
        if self.delay_micros > 0 {
            thread::sleep(Duration::from_micros(self.delay_micros));
        }

        if self.finished {
            return Ok(0);
        }

        let total_samples = (self.duration.as_secs_f64() * self.samples_per_second as f64) as u64;
        let current_sample = (self.position.as_secs_f64() * self.samples_per_second as f64) as u64;

        let remaining = (total_samples.saturating_sub(current_sample)) as usize;
        let to_read = remaining.min(buffer.len());

        if to_read == 0 {
            self.finished = true;
            return Ok(0);
        }

        // Generate audio pattern
        for (i, sample) in buffer.iter_mut().enumerate().take(to_read) {
            *sample = 0.5 * ((i % 2) as f32 - 0.5);
        }

        let samples_read_duration =
            Duration::from_secs_f64(to_read as f64 / self.samples_per_second as f64);
        self.position += samples_read_duration;

        Ok(to_read)
    }

    fn seek(&mut self, position: Duration) -> Result<()> {
        if position > self.duration {
            return Err(PlaybackError::InvalidSeekPosition(position));
        }
        self.position = position;
        self.finished = false;
        Ok(())
    }

    fn duration(&self) -> Duration {
        self.duration
    }

    fn position(&self) -> Duration {
        self.position
    }

    fn is_finished(&self) -> bool {
        self.finished
    }
}

fn create_test_track(id: &str, duration_secs: u64) -> QueueTrack {
    QueueTrack {
        id: id.into(),
        path: PathBuf::from(format!("/music/{}.mp3", id)),
        title: format!("Track {}", id),
        artist: "Artist".to_string(),
        album: Some("Album".to_string()),
        duration: Duration::from_secs(duration_secs),
        track_number: Some(1),
        source: TrackSource::Single,
    }
}

// ============================================================================
// 1. Multi-Threaded Chaos Tests
// ============================================================================

#[test]
#[ignore] // Run with --include-ignored
fn stress_concurrent_play_pause_100_cycles() {
    let manager = Arc::new(Mutex::new(PlaybackManager::default()));

    // Add test tracks
    {
        let mut mgr = manager.lock().unwrap();
        for i in 0..10 {
            mgr.add_to_queue_end(create_test_track(&i.to_string(), 180));
        }
        mgr.play().ok();
        mgr.activate_source(Box::new(ConcurrencyMockSource::new(
            Duration::from_secs(180),
            44100,
        )));
    }

    let handles: Vec<_> = (0..4)
        .map(|thread_id| {
            let mgr = Arc::clone(&manager);
            thread::spawn(move || {
                for i in 0..100 {
                    {
                        let mut m = mgr.lock().unwrap();
                        m.play().ok();
                    }
                    thread::sleep(Duration::from_micros(100));

                    {
                        let mut m = mgr.lock().unwrap();
                        m.pause();
                    }
                    thread::sleep(Duration::from_micros(100));

                    // Verify state is valid
                    let state = mgr.lock().unwrap().get_state();
                    assert!(
                        matches!(
                            state,
                            PlaybackState::Playing | PlaybackState::Paused | PlaybackState::Stopped
                        ),
                        "Thread {} cycle {}: invalid state {:?}",
                        thread_id,
                        i,
                        state
                    );
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().expect("Thread panicked");
    }

    // Verify manager is still functional
    let state = manager.lock().unwrap().get_state();
    assert!(matches!(
        state,
        PlaybackState::Paused | PlaybackState::Playing
    ));
}

#[test]
#[ignore]
fn stress_concurrent_skip_and_queue_modify() {
    let manager = Arc::new(Mutex::new(PlaybackManager::default()));

    // Initial tracks - start with more to ensure queue doesn't empty
    {
        let mut mgr = manager.lock().unwrap();
        for i in 0..100 {
            mgr.add_to_queue_end(create_test_track(&i.to_string(), 180));
        }
    }

    let skip_handle = {
        let mgr = Arc::clone(&manager);
        thread::spawn(move || {
            for _ in 0..30 {
                mgr.lock().unwrap().next().ok();
                thread::sleep(Duration::from_millis(10));
            }
        })
    };

    let add_handle = {
        let mgr = Arc::clone(&manager);
        thread::spawn(move || {
            for i in 100..150 {
                mgr.lock()
                    .unwrap()
                    .add_to_queue_end(create_test_track(&i.to_string(), 180));
                thread::sleep(Duration::from_millis(10));
            }
        })
    };

    let remove_handle = {
        let mgr = Arc::clone(&manager);
        thread::spawn(move || {
            for _ in 0..20 {
                let mut m = mgr.lock().unwrap();
                if m.queue_len() > 5 {
                    // Leave some buffer
                    m.remove_from_queue(0).ok();
                }
                drop(m);
                thread::sleep(Duration::from_millis(10));
            }
        })
    };

    skip_handle.join().expect("Skip thread panicked");
    add_handle.join().expect("Add thread panicked");
    remove_handle.join().expect("Remove thread panicked");

    // Verify manager state is consistent (should have plenty remaining)
    let mgr = manager.lock().unwrap();
    let queue_len = mgr.queue_len();
    assert!(
        queue_len > 0,
        "Queue should not be empty after operations (had {})",
        queue_len
    );
}

#[test]
#[ignore]
fn stress_shuffle_during_playback() {
    let manager = Arc::new(Mutex::new(PlaybackManager::default()));

    // Setup
    {
        let mut mgr = manager.lock().unwrap();
        for i in 0..100 {
            mgr.add_to_queue_end(create_test_track(&i.to_string(), 180));
        }
        mgr.play().ok();
        mgr.activate_source(Box::new(ConcurrencyMockSource::new(
            Duration::from_secs(180),
            44100,
        )));
    }

    let shuffle_thread = {
        let mgr = Arc::clone(&manager);
        thread::spawn(move || {
            for i in 0..50 {
                let mode = match i % 3 {
                    0 => ShuffleMode::Off,
                    1 => ShuffleMode::Random,
                    _ => ShuffleMode::Smart,
                };
                mgr.lock().unwrap().set_shuffle(mode);
                thread::sleep(Duration::from_millis(20));
            }
        })
    };

    let process_thread = {
        let mgr = Arc::clone(&manager);
        thread::spawn(move || {
            let mut buffer = vec![0.0f32; 1024];
            for _ in 0..100 {
                mgr.lock().unwrap().process_audio(&mut buffer).ok();
                thread::sleep(Duration::from_millis(10));
            }
        })
    };

    shuffle_thread.join().expect("Shuffle thread panicked");
    process_thread.join().expect("Process thread panicked");

    // Verify queue is intact
    let queue_len = manager.lock().unwrap().queue_len();
    assert!(queue_len > 0, "Queue should not be corrupted");
}

#[test]
#[ignore]
fn stress_volume_changes_during_playback() {
    let manager = Arc::new(Mutex::new(PlaybackManager::default()));

    {
        let mut mgr = manager.lock().unwrap();
        mgr.add_to_queue_end(create_test_track("1", 180));
        mgr.play().ok();
        mgr.activate_source(Box::new(ConcurrencyMockSource::new(
            Duration::from_secs(180),
            44100,
        )));
    }

    let volume_thread = {
        let mgr = Arc::clone(&manager);
        thread::spawn(move || {
            for i in 0..200 {
                // Use saturating_mul to prevent overflow in debug mode
                let volume = ((i as u32).saturating_mul(7) % 101) as u8;
                mgr.lock().unwrap().set_volume(volume);
                thread::sleep(Duration::from_micros(500));
            }
        })
    };

    let mute_thread = {
        let mgr = Arc::clone(&manager);
        thread::spawn(move || {
            for _ in 0..100 {
                mgr.lock().unwrap().toggle_mute();
                thread::sleep(Duration::from_millis(1));
            }
        })
    };

    let process_thread = {
        let mgr = Arc::clone(&manager);
        thread::spawn(move || {
            let mut buffer = vec![0.0f32; 512];
            for _ in 0..100 {
                mgr.lock().unwrap().process_audio(&mut buffer).ok();
                thread::sleep(Duration::from_millis(1));
            }
        })
    };

    volume_thread.join().expect("Volume thread panicked");
    mute_thread.join().expect("Mute thread panicked");
    process_thread.join().expect("Process thread panicked");

    // Volume should be valid
    let volume = manager.lock().unwrap().get_volume();
    assert!(volume <= 100, "Volume should be valid");
}

#[test]
#[ignore]
fn stress_seek_from_multiple_threads() {
    let manager = Arc::new(Mutex::new(PlaybackManager::default()));

    {
        let mut mgr = manager.lock().unwrap();
        mgr.add_to_queue_end(create_test_track("1", 180));
        mgr.play().ok();
        mgr.activate_source(Box::new(ConcurrencyMockSource::new(
            Duration::from_secs(180),
            44100,
        )));
    }

    let handles: Vec<_> = (0..3)
        .map(|thread_id| {
            let mgr = Arc::clone(&manager);
            thread::spawn(move || {
                for i in 0..50 {
                    let position = Duration::from_secs(((thread_id * 50 + i) % 179) as u64);
                    mgr.lock().unwrap().seek_to(position).ok();
                    thread::sleep(Duration::from_millis(10));
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().expect("Seek thread panicked");
    }

    // Position should be valid
    let position = manager.lock().unwrap().get_position();
    let duration = manager.lock().unwrap().get_duration().unwrap_or_default();
    assert!(
        position <= duration,
        "Position should be within track duration"
    );
}

// ============================================================================
// 2. Race Condition Tests
// ============================================================================

#[test]
#[ignore]
fn race_pause_during_track_load() {
    let manager = Arc::new(Mutex::new(PlaybackManager::default()));

    {
        let mut mgr = manager.lock().unwrap();
        for i in 0..10 {
            mgr.add_to_queue_end(create_test_track(&i.to_string(), 180));
        }
    }

    // Simulate rapid play/pause during track loading
    for _ in 0..20 {
        {
            let mut mgr = manager.lock().unwrap();
            mgr.play().ok();
            // Simulate async track loading delay
            thread::sleep(Duration::from_micros(100));
        }

        // Pause before source is set (simulating pause during loading)
        {
            let mut mgr = manager.lock().unwrap();
            mgr.pause();
        }

        // Now set source
        {
            let mut mgr = manager.lock().unwrap();
            mgr.activate_source(Box::new(ConcurrencyMockSource::new(
                Duration::from_secs(180),
                44100,
            )));
        }

        // Verify state is paused (not auto-playing)
        let state = manager.lock().unwrap().get_state();
        assert!(
            matches!(state, PlaybackState::Paused),
            "Should respect pause during loading"
        );
    }
}

#[test]
#[ignore]
fn race_skip_during_crossfade() {
    let config = PlaybackConfig {
        crossfade: soul_playback::CrossfadeSettings::with_duration(1000),
        ..Default::default()
    };
    let manager = Arc::new(Mutex::new(PlaybackManager::new(config)));

    {
        let mut mgr = manager.lock().unwrap();
        mgr.set_crossfade_enabled(true);
        for i in 0..5 {
            mgr.add_to_queue_end(create_test_track(&i.to_string(), 10));
        }
        mgr.play().ok();
        mgr.activate_source(Box::new(ConcurrencyMockSource::new(
            Duration::from_secs(10),
            44100,
        )));
    }

    // Process audio to trigger crossfade
    let skip_thread = {
        let mgr = Arc::clone(&manager);
        thread::spawn(move || {
            for _ in 0..10 {
                thread::sleep(Duration::from_millis(50));
                mgr.lock().unwrap().next().ok();
            }
        })
    };

    let process_thread = {
        let mgr = Arc::clone(&manager);
        thread::spawn(move || {
            let mut buffer = vec![0.0f32; 4096];
            for _ in 0..100 {
                mgr.lock().unwrap().process_audio(&mut buffer).ok();
                thread::sleep(Duration::from_millis(10));
            }
        })
    };

    skip_thread.join().expect("Skip thread panicked");
    process_thread.join().expect("Process thread panicked");

    // Should not crash
    let state = manager.lock().unwrap().get_state();
    assert!(matches!(
        state,
        PlaybackState::Playing | PlaybackState::Stopped
    ));
}

#[test]
#[ignore]
fn race_queue_clear_during_playback() {
    let manager = Arc::new(Mutex::new(PlaybackManager::default()));

    {
        let mut mgr = manager.lock().unwrap();
        for i in 0..50 {
            mgr.add_to_queue_end(create_test_track(&i.to_string(), 180));
        }
        mgr.play().ok();
        mgr.activate_source(Box::new(ConcurrencyMockSource::new(
            Duration::from_secs(180),
            44100,
        )));
    }

    let clear_thread = {
        let mgr = Arc::clone(&manager);
        thread::spawn(move || {
            for _ in 0..10 {
                thread::sleep(Duration::from_millis(20));
                mgr.lock().unwrap().clear_queue();
            }
        })
    };

    let process_thread = {
        let mgr = Arc::clone(&manager);
        thread::spawn(move || {
            let mut buffer = vec![0.0f32; 1024];
            for _ in 0..50 {
                mgr.lock().unwrap().process_audio(&mut buffer).ok();
                thread::sleep(Duration::from_millis(10));
            }
        })
    };

    clear_thread.join().expect("Clear thread panicked");
    process_thread.join().expect("Process thread panicked");

    // Current track should still be playing
    let state = manager.lock().unwrap().get_state();
    assert!(matches!(
        state,
        PlaybackState::Playing | PlaybackState::Stopped
    ));
}

// ============================================================================
// 3. Deadlock Prevention Tests
// ============================================================================

#[test]
#[ignore]
fn deadlock_prevention_nested_operations() {
    let manager = Arc::new(Mutex::new(PlaybackManager::default()));

    {
        let mut mgr = manager.lock().unwrap();
        for i in 0..20 {
            mgr.add_to_queue_end(create_test_track(&i.to_string(), 180));
        }
    }

    // Multiple threads performing complex operations
    let handles: Vec<_> = (0..4)
        .map(|_| {
            let mgr = Arc::clone(&manager);
            thread::spawn(move || {
                for i in 0..50 {
                    // Lock contention scenario
                    let mut m = mgr.lock().unwrap();

                    match i % 5 {
                        0 => {
                            m.play().ok();
                        }
                        1 => {
                            m.pause();
                        }
                        2 => {
                            m.next().ok();
                        }
                        3 => {
                            m.add_to_queue_end(create_test_track(&i.to_string(), 180));
                        }
                        _ => {
                            m.set_volume(50);
                        }
                    }

                    drop(m);
                    thread::sleep(Duration::from_micros(100));
                }
            })
        })
        .collect();

    // All threads should complete without deadlock
    for handle in handles {
        handle
            .join()
            .expect("Thread should complete without deadlock");
    }
}

// ============================================================================
// 4. Memory Safety Tests
// ============================================================================

#[test]
#[ignore]
fn memory_safety_concurrent_queue_modifications() {
    let manager = Arc::new(Mutex::new(PlaybackManager::default()));

    let add_thread = {
        let mgr = Arc::clone(&manager);
        thread::spawn(move || {
            for i in 0..1000 {
                mgr.lock()
                    .unwrap()
                    .add_to_queue_end(create_test_track(&i.to_string(), 180));
                if i % 100 == 0 {
                    thread::sleep(Duration::from_micros(10));
                }
            }
        })
    };

    let remove_thread = {
        let mgr = Arc::clone(&manager);
        thread::spawn(move || {
            for _ in 0..500 {
                let mut m = mgr.lock().unwrap();
                if m.queue_len() > 0 {
                    m.remove_from_queue(0).ok();
                }
                drop(m);
                thread::sleep(Duration::from_micros(20));
            }
        })
    };

    let query_thread = {
        let mgr = Arc::clone(&manager);
        thread::spawn(move || {
            for _ in 0..200 {
                let _queue = mgr.lock().unwrap().get_queue();
                thread::sleep(Duration::from_micros(50));
            }
        })
    };

    add_thread.join().expect("Add thread panicked");
    remove_thread.join().expect("Remove thread panicked");
    query_thread.join().expect("Query thread panicked");

    // Verify queue is in valid state (no panics)
    let _queue_len = manager.lock().unwrap().queue_len();
}

// ============================================================================
// 5. Stress Test - All Operations Combined
// ============================================================================

#[test]
#[ignore]
fn stress_all_operations_combined_chaos() {
    let manager = Arc::new(Mutex::new(PlaybackManager::default()));
    let stop_flag = Arc::new(AtomicBool::new(false));

    // Initial setup
    {
        let mut mgr = manager.lock().unwrap();
        for i in 0..50 {
            mgr.add_to_queue_end(create_test_track(&i.to_string(), 180));
        }
        mgr.play().ok();
        mgr.activate_source(Box::new(ConcurrencyMockSource::new(
            Duration::from_secs(180),
            44100,
        )));
    }

    let playback_thread = {
        let mgr = Arc::clone(&manager);
        let stop = Arc::clone(&stop_flag);
        thread::spawn(move || {
            let mut buffer = vec![0.0f32; 512];
            while !stop.load(Ordering::Relaxed) {
                mgr.lock().unwrap().process_audio(&mut buffer).ok();
                thread::sleep(Duration::from_micros(100));
            }
        })
    };

    let control_thread = {
        let mgr = Arc::clone(&manager);
        let stop = Arc::clone(&stop_flag);
        thread::spawn(move || {
            for i in 0..100 {
                if stop.load(Ordering::Relaxed) {
                    break;
                }
                let mut m = mgr.lock().unwrap();
                if i % 2 == 0 {
                    m.play().ok();
                } else {
                    m.pause();
                }
                drop(m);
                thread::sleep(Duration::from_millis(10));
            }
        })
    };

    let queue_thread = {
        let mgr = Arc::clone(&manager);
        let stop = Arc::clone(&stop_flag);
        thread::spawn(move || {
            for i in 100..150 {
                if stop.load(Ordering::Relaxed) {
                    break;
                }
                mgr.lock()
                    .unwrap()
                    .add_to_queue_end(create_test_track(&i.to_string(), 180));
                thread::sleep(Duration::from_millis(20));
            }
        })
    };

    let volume_thread = {
        let mgr = Arc::clone(&manager);
        let stop = Arc::clone(&stop_flag);
        thread::spawn(move || {
            for i in 0..100 {
                if stop.load(Ordering::Relaxed) {
                    break;
                }
                // Use saturating_mul to prevent overflow in debug mode
                let volume = ((i as u32).saturating_mul(13) % 101) as u8;
                mgr.lock().unwrap().set_volume(volume);
                thread::sleep(Duration::from_millis(10));
            }
        })
    };

    // Let chaos run for a bit
    thread::sleep(Duration::from_secs(2));

    // Signal stop
    stop_flag.store(true, Ordering::Relaxed);

    // Wait for all threads
    playback_thread.join().expect("Playback thread panicked");
    control_thread.join().expect("Control thread panicked");
    queue_thread.join().expect("Queue thread panicked");
    volume_thread.join().expect("Volume thread panicked");

    // Verify manager is still functional
    let mgr = manager.lock().unwrap();
    let state = mgr.get_state();
    assert!(
        matches!(
            state,
            PlaybackState::Playing | PlaybackState::Paused | PlaybackState::Stopped
        ),
        "Manager should be in valid state after chaos"
    );
}
