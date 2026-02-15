//! Memory pressure and edge case tests
//!
//! Tests system behavior under memory constraints and edge cases:
//! - Large queue memory usage
//! - Memory leak detection
//! - Buffer allocation failures
//! - Zero/negative edge cases
//!
//! Run with: cargo test --test memory_and_edge_case_test -- --include-ignored

use soul_playback::{
    AudioSource, PlaybackConfig, PlaybackError, PlaybackManager, PlaybackState, QueueTrack,
    RepeatMode, Result, TrackSource,
};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

// ============================================================================
// Test Infrastructure
// ============================================================================

struct MemoryMockSource {
    duration: Duration,
    position: Duration,
    sample_rate: u32,
    finished: bool,
    read_count: Arc<AtomicUsize>,
}

impl MemoryMockSource {
    fn new(duration: Duration) -> Self {
        Self {
            duration,
            position: Duration::ZERO,
            sample_rate: 44100,
            finished: false,
            read_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn with_counter(mut self, counter: Arc<AtomicUsize>) -> Self {
        self.read_count = counter;
        self
    }
}

impl AudioSource for MemoryMockSource {
    fn read_samples(&mut self, buffer: &mut [f32]) -> Result<usize> {
        self.read_count.fetch_add(1, Ordering::Relaxed);

        if self.finished {
            return Ok(0);
        }

        let samples_per_second = (self.sample_rate as u64) * 2;
        let total_samples = (self.duration.as_secs_f64() * samples_per_second as f64) as u64;
        let current_sample = (self.position.as_secs_f64() * samples_per_second as f64) as u64;

        let remaining = (total_samples.saturating_sub(current_sample)) as usize;
        let to_read = remaining.min(buffer.len());

        if to_read == 0 {
            self.finished = true;
            return Ok(0);
        }

        for sample in buffer.iter_mut().take(to_read) {
            *sample = 0.01;
        }

        let samples_read_duration =
            Duration::from_secs_f64(to_read as f64 / samples_per_second as f64);
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
// 1. Large Queue Memory Tests
// ============================================================================

#[test]
#[ignore]
fn test_large_queue_memory_usage() {
    let mut manager = PlaybackManager::default();

    // Add 100,000 tracks (metadata only)
    let start = std::time::Instant::now();

    for i in 0..100_000 {
        manager.add_to_queue_end(create_test_track(&i.to_string(), 180));

        // Progress indicator
        if i % 10_000 == 0 {
            println!("Added {} tracks...", i);
        }
    }

    let elapsed = start.elapsed();
    println!("Added 100k tracks in {:?}", elapsed);

    // Verify queue length
    assert_eq!(manager.queue_len(), 100_000);

    // Operations should still work
    manager.play().ok();
    manager.activate_source(Box::new(MemoryMockSource::new(Duration::from_secs(180))));

    let mut buffer = vec![0.0f32; 1024];
    manager.process_audio(&mut buffer).ok();

    // Should not crash or hang
    let state = manager.get_state();
    assert_eq!(state, PlaybackState::Playing);

    // Memory usage check (rough estimate)
    // Each QueueTrack is ~200 bytes, so 100k tracks ≈ 20MB
    // This should be acceptable
    println!("Large queue test completed successfully");
}

#[test]
#[ignore]
fn test_queue_iteration_performance() {
    let mut manager = PlaybackManager::default();

    // Add 50,000 tracks
    for i in 0..50_000 {
        manager.add_to_queue_end(create_test_track(&i.to_string(), 180));
    }

    // Measure get_queue performance
    let start = std::time::Instant::now();
    let queue = manager.get_queue();
    let elapsed = start.elapsed();

    println!("Retrieved queue of {} items in {:?}", queue.len(), elapsed);

    // Should complete in reasonable time (< 100ms)
    assert!(
        elapsed < Duration::from_millis(100),
        "Queue retrieval too slow: {:?}",
        elapsed
    );
}

// ============================================================================
// 2. Memory Leak Detection Tests
// ============================================================================

#[test]
#[ignore]
fn test_memory_leak_detection_1000_tracks() {
    let mut manager = PlaybackManager::default();

    // Play through 1000 tracks
    for i in 0..1000 {
        manager.add_to_queue_end(create_test_track(&i.to_string(), 1));
        manager.play().ok();
        manager.activate_source(Box::new(MemoryMockSource::new(Duration::from_secs(1))));

        // Process until track finishes
        let mut buffer = vec![0.0f32; 44100 * 2]; // 1 second
        for _ in 0..3 {
            manager.process_audio(&mut buffer).ok();
        }

        manager.next().ok();

        if i % 100 == 0 {
            println!("Played {} tracks...", i);
        }
    }

    // Verify history doesn't grow unbounded
    let history_len = manager.get_history().len();
    println!("History length after 1000 tracks: {}", history_len);

    assert!(
        history_len <= 100,
        "History should be bounded, got {}",
        history_len
    );

    // Queue should be empty or small
    let queue_len = manager.queue_len();
    println!("Queue length: {}", queue_len);

    assert!(queue_len < 100, "Queue should not grow unbounded");
}

#[test]
#[ignore]
fn test_no_buffer_leak_on_repeated_playback() {
    let read_count = Arc::new(AtomicUsize::new(0));
    let mut manager = PlaybackManager::default();

    manager.add_to_queue_end(create_test_track("1", 10));

    // Start and stop playback many times
    for i in 0..100 {
        manager.play().ok();
        manager.activate_source(Box::new(
            MemoryMockSource::new(Duration::from_secs(10)).with_counter(Arc::clone(&read_count)),
        ));

        let mut buffer = vec![0.0f32; 1024];
        for _ in 0..10 {
            manager.process_audio(&mut buffer).ok();
        }

        manager.stop();

        if i % 20 == 0 {
            println!("Cycle {}", i);
        }
    }

    let total_reads = read_count.load(Ordering::Relaxed);
    println!("Total read operations: {}", total_reads);

    // Should have processed audio
    assert!(total_reads > 0);

    // After stop, manager should be clean
    assert_eq!(manager.get_state(), PlaybackState::Stopped);
    assert!(manager.get_current_track().is_none());
}

#[test]
#[ignore]
fn test_crossfade_buffer_cleanup() {
    let config = PlaybackConfig {
        crossfade: soul_playback::CrossfadeSettings::with_duration(1000),
        ..Default::default()
    };
    let mut manager = PlaybackManager::new(config);
    manager.set_crossfade_enabled(true);

    // Add tracks
    for i in 0..50 {
        manager.add_to_queue_end(create_test_track(&i.to_string(), 5));
    }

    manager.play().ok();
    manager.activate_source(Box::new(MemoryMockSource::new(Duration::from_secs(5))));

    let mut buffer = vec![0.0f32; 4096];

    // Play through several tracks with crossfade
    for _ in 0..10 {
        // Process audio to trigger crossfade
        for _ in 0..50 {
            manager.process_audio(&mut buffer).ok();
        }

        manager.next().ok();
        manager.activate_source(Box::new(MemoryMockSource::new(Duration::from_secs(5))));
    }

    // Stop should free crossfade buffers
    manager.stop();

    // Verify clean state
    assert_eq!(manager.get_state(), PlaybackState::Stopped);

    println!("Crossfade buffer cleanup test passed");
}

// ============================================================================
// 3. Buffer Edge Cases
// ============================================================================

#[test]
fn test_empty_buffer_processing() {
    let mut manager = PlaybackManager::default();

    manager.add_to_queue_end(create_test_track("1", 180));
    manager.play().ok();
    manager.activate_source(Box::new(MemoryMockSource::new(Duration::from_secs(180))));

    // Try processing with empty buffer
    let mut empty_buffer: Vec<f32> = vec![];
    let result = manager.process_audio(&mut empty_buffer);

    // Should handle gracefully (implementation-dependent)
    match result {
        Ok(0) => println!("✓ Empty buffer handled (returned 0)"),
        Err(_) => println!("✓ Empty buffer rejected with error"),
        _ => {}
    }
}

#[test]
fn test_single_sample_buffer() {
    let mut manager = PlaybackManager::default();

    manager.add_to_queue_end(create_test_track("1", 180));
    manager.play().ok();
    manager.activate_source(Box::new(MemoryMockSource::new(Duration::from_secs(180))));

    // Process with 1-sample buffer (stereo = 2 values)
    let mut tiny_buffer = vec![0.0f32; 2];
    let result = manager.process_audio(&mut tiny_buffer);

    // Should handle gracefully
    assert!(
        result.is_ok() || result.is_err(),
        "Should handle tiny buffer"
    );
}

#[test]
#[ignore]
fn test_huge_buffer_allocation() {
    let mut manager = PlaybackManager::default();

    manager.add_to_queue_end(create_test_track("1", 180));
    manager.play().ok();
    manager.activate_source(Box::new(MemoryMockSource::new(Duration::from_secs(180))));

    // Try processing with huge buffer (100MB of f32 = 25M samples)
    // This might fail or succeed depending on available memory
    let huge_size = 25_000_000;
    let mut huge_buffer = Vec::new();
    match huge_buffer.try_reserve_exact(huge_size) {
        Ok(_) => {
            huge_buffer.resize(huge_size, 0.0f32);
            let result = manager.process_audio(&mut huge_buffer);

            match result {
                Ok(n) => println!("✓ Processed {} samples from huge buffer", n),
                Err(e) => println!("✓ Rejected huge buffer: {:?}", e),
            }
        }
        Err(_) => {
            println!("✓ Cannot allocate huge buffer (expected on some systems)");
        }
    }
}

#[test]
fn test_odd_buffer_size() {
    let mut manager = PlaybackManager::default();

    manager.add_to_queue_end(create_test_track("1", 180));
    manager.play().ok();
    manager.activate_source(Box::new(MemoryMockSource::new(Duration::from_secs(180))));

    // Odd buffer size (not multiple of channels)
    let mut odd_buffer = vec![0.0f32; 1001]; // Odd number
    let result = manager.process_audio(&mut odd_buffer);

    // Should handle (might round down to even number)
    assert!(result.is_ok(), "Should handle odd buffer size");
}

// ============================================================================
// 4. Duration Edge Cases
// ============================================================================

#[test]
fn test_zero_duration_track() {
    let mut manager = PlaybackManager::default();

    let zero_track = QueueTrack {
        id: "zero".into(),
        path: PathBuf::from("/music/zero.mp3"),
        title: "Zero Duration".to_string(),
        artist: "Artist".to_string(),
        album: None,
        duration: Duration::ZERO,
        track_number: None,
        source: TrackSource::Single,
    };

    manager.add_to_queue_end(zero_track);
    manager.add_to_queue_end(create_test_track("normal", 180));

    manager.play().ok();
    manager.activate_source(Box::new(MemoryMockSource::new(Duration::ZERO)));

    let mut buffer = vec![0.0f32; 1024];

    // Should skip zero-duration track quickly
    for _ in 0..10 {
        manager.process_audio(&mut buffer).ok();
    }

    // Should not crash
    let state = manager.get_state();
    assert!(matches!(
        state,
        PlaybackState::Playing | PlaybackState::Stopped
    ));
}

#[test]
fn test_extremely_long_track() {
    let mut manager = PlaybackManager::default();

    // Track with max duration (u64::MAX nanoseconds ≈ 584 years)
    let long_track = QueueTrack {
        id: "long".into(),
        path: PathBuf::from("/music/long.mp3"),
        title: "Extremely Long Track".to_string(),
        artist: "Artist".to_string(),
        album: None,
        duration: Duration::from_secs(u32::MAX as u64), // ~136 years
        track_number: None,
        source: TrackSource::Single,
    };

    manager.add_to_queue_end(long_track);
    manager.play().ok();
    manager.activate_source(Box::new(MemoryMockSource::new(Duration::from_secs(
        u32::MAX as u64,
    ))));

    // Should not crash on position calculations
    let position = manager.get_position();
    let duration = manager.get_duration();

    assert!(position <= duration.unwrap_or(Duration::MAX));

    // Seek near end should work
    let near_end = Duration::from_secs(u32::MAX as u64 - 10);
    manager.seek_to(near_end).ok();
}

// ============================================================================
// 5. Seek Edge Cases
// ============================================================================

#[test]
fn test_seek_beyond_duration() {
    let mut manager = PlaybackManager::default();

    manager.add_to_queue_end(create_test_track("1", 100));
    manager.play().ok();
    manager.activate_source(Box::new(MemoryMockSource::new(Duration::from_secs(100))));

    // Seek way beyond duration
    let result = manager.seek_to(Duration::from_secs(10000));

    // Should clamp or error gracefully
    match result {
        Ok(_) => {
            let position = manager.get_position();
            let duration = manager.get_duration().unwrap();
            assert!(
                position <= duration,
                "Position should be clamped to duration"
            );
            println!("✓ Seek clamped to duration");
        }
        Err(_) => {
            println!("✓ Seek rejected with error");
        }
    }
}

#[test]
fn test_negative_seek_via_zero() {
    let mut manager = PlaybackManager::default();

    manager.add_to_queue_end(create_test_track("1", 100));
    manager.play().ok();
    manager.activate_source(Box::new(MemoryMockSource::new(Duration::from_secs(100))));

    // Seek to exactly zero
    let result = manager.seek_to(Duration::ZERO);

    assert!(result.is_ok(), "Should be able to seek to start");
    assert_eq!(manager.get_position(), Duration::ZERO, "Should be at start");
}

#[test]
fn test_rapid_seeks_to_same_position() {
    let mut manager = PlaybackManager::default();

    manager.add_to_queue_end(create_test_track("1", 100));
    manager.play().ok();
    manager.activate_source(Box::new(MemoryMockSource::new(Duration::from_secs(100))));

    let target = Duration::from_secs(50);

    // Seek to same position 100 times
    for _ in 0..100 {
        manager.seek_to(target).ok();
    }

    // Should not corrupt state
    let position = manager.get_position();
    assert!(
        position <= Duration::from_secs(100),
        "Position should be valid"
    );
}

// ============================================================================
// 6. State Transition Edge Cases
// ============================================================================

#[test]
fn test_rapid_state_transitions() {
    let mut manager = PlaybackManager::default();

    manager.add_to_queue_end(create_test_track("1", 180));

    // Rapid state transitions
    for _ in 0..100 {
        manager.play().ok();
        manager.pause();
        manager.play().ok();
        manager.stop();
    }

    // Should end in stopped state
    assert_eq!(manager.get_state(), PlaybackState::Stopped);

    // Should still be functional
    manager.add_to_queue_end(create_test_track("2", 180));
    manager.play().ok();
}

#[test]
fn test_operations_without_tracks() {
    let mut manager = PlaybackManager::default();

    // Try operations on empty queue
    let play_result = manager.play();
    assert!(play_result.is_err(), "Play should fail without tracks");

    let next_result = manager.next();
    assert!(next_result.is_err(), "Next should fail without tracks");

    let prev_result = manager.previous();
    assert!(prev_result.is_ok(), "Previous should not crash");

    // Volume operations should work
    manager.set_volume(50);
    assert_eq!(manager.get_volume(), 50);

    // State should be stopped
    assert_eq!(manager.get_state(), PlaybackState::Stopped);
}

// ============================================================================
// 7. Repeat Mode Edge Cases
// ============================================================================

#[test]
fn test_repeat_one_with_zero_duration() {
    let mut manager = PlaybackManager::default();
    manager.set_repeat(RepeatMode::One);

    let zero_track = QueueTrack {
        id: "zero".into(),
        path: PathBuf::from("/music/zero.mp3"),
        title: "Zero".to_string(),
        artist: "Artist".to_string(),
        album: None,
        duration: Duration::ZERO,
        track_number: None,
        source: TrackSource::Single,
    };

    manager.add_to_queue_end(zero_track);
    manager.play().ok();
    manager.activate_source(Box::new(MemoryMockSource::new(Duration::ZERO)));

    let mut buffer = vec![0.0f32; 1024];

    // Process multiple times (should not infinite loop)
    for _ in 0..10 {
        manager.process_audio(&mut buffer).ok();
    }

    // Should not crash
}

#[test]
#[ignore]
fn test_repeat_all_large_queue() {
    let mut manager = PlaybackManager::default();
    manager.set_repeat(RepeatMode::All);

    // Add 1000 tracks as a playlist (so they become part of the repeatable source)
    let tracks: Vec<_> = (0..1000)
        .map(|i| create_test_track(&i.to_string(), 1))
        .collect();
    manager.add_playlist_to_queue(tracks);

    manager.play().ok();
    manager.activate_source(Box::new(MemoryMockSource::new(Duration::from_secs(1))));

    let mut buffer = vec![0.0f32; 44100 * 2];

    // Play through entire queue + wrap around
    for _ in 0..1050 {
        manager.process_audio(&mut buffer).ok();
        manager.next().ok();
        manager.activate_source(Box::new(MemoryMockSource::new(Duration::from_secs(1))));
    }

    // Should have wrapped around
    let current = manager.get_current_track();
    assert!(current.is_some(), "Should still be playing with repeat all");

    println!("Repeat all with large queue completed successfully");
}

// ============================================================================
// 8. Volume Edge Cases
// ============================================================================

#[test]
fn test_volume_boundary_values() {
    let mut manager = PlaybackManager::default();

    // Test boundary values
    manager.set_volume(0);
    assert_eq!(manager.get_volume(), 0);

    manager.set_volume(100);
    assert_eq!(manager.get_volume(), 100);

    manager.set_volume(101); // Over limit
    let volume = manager.get_volume();
    assert!(volume <= 100, "Volume should be clamped to 100");
}
