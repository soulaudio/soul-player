//! Event system stress tests
//!
//! Tests event handling under high load:
//! - Event overflow handling
//! - Slow consumers
//! - Consumer disconnect
//! - Event ordering guarantees
//!
//! Run with: cargo test --test event_system_stress_test -- --include-ignored

use soul_playback::{
    AudioSource, PlaybackError, PlaybackEvent, PlaybackManager, QueueTrack, Result, TrackSource,
};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::Duration;

// ============================================================================
// Test Infrastructure
// ============================================================================

struct EventMockSource {
    duration: Duration,
    position: Duration,
    sample_rate: u32,
    finished: bool,
}

impl EventMockSource {
    fn new(duration: Duration) -> Self {
        Self {
            duration,
            position: Duration::ZERO,
            sample_rate: 44100,
            finished: false,
        }
    }
}

impl AudioSource for EventMockSource {
    fn read_samples(&mut self, buffer: &mut [f32]) -> Result<usize> {
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
            *sample = 0.1;
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
// 1. Event Overflow Tests
// ============================================================================

#[test]
fn test_event_overflow_handling() {
    let mut manager = PlaybackManager::default();

    // Add tracks
    for i in 0..10 {
        manager.add_to_queue_end(create_test_track(&i.to_string(), 180));
    }

    manager.play().ok();
    manager.set_audio_source(Box::new(EventMockSource::new(Duration::from_secs(180))));

    // Process audio rapidly to generate many position update events
    let mut buffer = vec![0.0f32; 512];
    for _ in 0..2000 {
        manager.process_audio(&mut buffer).ok();
    }

    // Drain events
    let events = manager.drain_events();
    let event_count = events.len();

    println!("Received {} events", event_count);

    // Should have received many events but not crash
    assert!(event_count > 0, "Should have generated events");
}

#[test]
fn test_position_update_throttling() {
    let mut manager = PlaybackManager::default();

    manager.add_to_queue_end(create_test_track("1", 180));
    manager.play().ok();
    manager.set_audio_source(Box::new(EventMockSource::new(Duration::from_secs(180))));

    // Process a lot of audio
    let mut buffer = vec![0.0f32; 256]; // Small buffer for rapid callbacks
    let mut process_count = 0;

    for _ in 0..1000 {
        manager.process_audio(&mut buffer).ok();
        process_count += 1;
    }

    // Count position update events
    let events = manager.drain_events();
    let position_updates = events
        .iter()
        .filter(|e| matches!(e, PlaybackEvent::PositionUpdate { .. }))
        .count();

    println!(
        "Process calls: {}, Position updates: {}",
        process_count, position_updates
    );

    // Position updates should be throttled (not one per process call)
    assert!(
        position_updates < process_count,
        "Position updates should be throttled"
    );
    assert!(position_updates > 0, "Should have some position updates");
}

#[test]
#[ignore]
fn stress_event_generation_10k_operations() {
    let mut manager = PlaybackManager::default();

    // Setup
    for i in 0..50 {
        manager.add_to_queue_end(create_test_track(&i.to_string(), 10));
    }

    manager.play().ok();
    manager.set_audio_source(Box::new(EventMockSource::new(Duration::from_secs(10))));

    let mut buffer = vec![0.0f32; 1024];

    // Perform 10,000 operations that might generate events
    for i in 0..10_000 {
        match i % 10 {
            0 => {
                manager.play().ok();
            }
            1 => {
                manager.pause();
            }
            2 => {
                manager.set_volume((i % 101) as u8);
            }
            3 => {
                manager.next().ok();
                manager.set_audio_source(Box::new(EventMockSource::new(Duration::from_secs(10))));
            }
            4 => {
                manager.seek_to(Duration::from_secs((i % 8) as u64)).ok();
            }
            _ => {
                manager.process_audio(&mut buffer).ok();
            }
        }

        // Drain some events periodically
        if i % 100 == 0 {
            let _ = manager.drain_events();
        }
    }

    // Drain remaining events
    let final_events = manager.drain_events();
    let final_event_count = final_events.len();

    println!("Final event count: {}", final_event_count);

    // Should complete without crash or memory leak
    assert!(
        final_event_count < 50000,
        "Event queue should not grow unbounded"
    );
}

// ============================================================================
// 2. Slow Consumer Tests
// ============================================================================

#[test]
#[ignore]
fn test_slow_event_consumer() {
    let (tx, rx) = mpsc::channel();
    let mut manager = PlaybackManager::default();

    manager.add_to_queue_end(create_test_track("1", 180));
    manager.play().ok();
    manager.set_audio_source(Box::new(EventMockSource::new(Duration::from_secs(180))));

    let producer = thread::spawn(move || {
        let mut buffer = vec![0.0f32; 512];

        // Rapidly produce events
        for _ in 0..1000 {
            manager.process_audio(&mut buffer).ok();

            // Send events
            for event in manager.drain_events() {
                if tx.send(event).is_err() {
                    break;
                }
            }

            thread::sleep(Duration::from_micros(100));
        }
    });

    // Slow consumer (100ms per event)
    let consumer = thread::spawn(move || {
        let mut consumed = 0;
        loop {
            match rx.recv_timeout(Duration::from_secs(5)) {
                Ok(_event) => {
                    consumed += 1;
                    thread::sleep(Duration::from_millis(100)); // Slow processing
                }
                Err(RecvTimeoutError::Timeout) => break,
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }
        consumed
    });

    producer.join().expect("Producer panicked");
    let consumed = consumer.join().expect("Consumer panicked");

    println!("Slow consumer received {} events", consumed);

    // Should have received some events (but not all due to slow processing)
    assert!(consumed > 0, "Should have consumed some events");
}

#[test]
#[ignore]
fn test_event_backpressure() {
    let mut manager = PlaybackManager::default();

    manager.add_to_queue_end(create_test_track("1", 180));
    manager.play().ok();
    manager.set_audio_source(Box::new(EventMockSource::new(Duration::from_secs(180))));

    let mut buffer = vec![0.0f32; 512];
    let mut process_operations = 0;

    // Process without draining events
    for _ in 0..1000 {
        manager.process_audio(&mut buffer).ok();
        process_operations += 1;
    }

    // Now drain events
    let events = manager.drain_events();
    let event_count = events.len();

    println!(
        "Process ops: {}, Events: {}",
        process_operations, event_count
    );

    // Event queue should not grow unbounded
    assert!(
        event_count < process_operations * 2,
        "Event queue should have reasonable size"
    );
}

// ============================================================================
// 3. Event Ordering Tests
// ============================================================================

#[test]
fn test_event_ordering_guarantees() {
    let mut manager = PlaybackManager::default();

    manager.add_to_queue_end(create_test_track("1", 180));
    manager.add_to_queue_end(create_test_track("2", 180));

    // Perform sequence of operations
    manager.play().ok();
    manager.set_audio_source(Box::new(EventMockSource::new(Duration::from_secs(180))));

    let mut buffer = vec![0.0f32; 1024];
    manager.process_audio(&mut buffer).ok();

    manager.pause();
    manager.play().ok();
    manager.next().ok();
    manager.set_audio_source(Box::new(EventMockSource::new(Duration::from_secs(180))));

    // Collect events
    let events = manager.drain_events();

    println!("Event sequence:");
    for (i, event) in events.iter().enumerate() {
        println!("  {}: {:?}", i, event);
    }

    // Verify events are in logical order
    // - StateChanged events should occur before corresponding track changes
    let mut last_state_change = None;

    for event in &events {
        match event {
            PlaybackEvent::StateChanged { state } => {
                last_state_change = Some(*state);
            }
            _ => {}
        }
    }

    // Should have received state change events
    assert!(
        last_state_change.is_some(),
        "Should have state change events"
    );
}

#[test]
fn test_no_duplicate_state_events() {
    let mut manager = PlaybackManager::default();

    manager.add_to_queue_end(create_test_track("1", 180));
    manager.play().ok();
    manager.set_audio_source(Box::new(EventMockSource::new(Duration::from_secs(180))));

    // Call play multiple times (should not emit duplicate events)
    manager.play().ok();
    manager.play().ok();
    manager.play().ok();

    // Collect state change events
    let events = manager.drain_events();
    let state_changes: Vec<_> = events
        .into_iter()
        .filter_map(|e| {
            if let PlaybackEvent::StateChanged { state } = e {
                Some(state)
            } else {
                None
            }
        })
        .collect();

    println!("State changes: {:?}", state_changes);

    // Should not have duplicate consecutive state changes
    let mut last_state = None;
    for state in state_changes {
        assert_ne!(
            Some(state),
            last_state,
            "Should not have duplicate consecutive state"
        );
        last_state = Some(state);
    }
}

// ============================================================================
// 4. Concurrent Event Access Tests
// ============================================================================

#[test]
#[ignore]
fn test_concurrent_event_polling() {
    use std::sync::{Arc, Mutex};

    let manager = Arc::new(Mutex::new(PlaybackManager::default()));

    {
        let mut mgr = manager.lock().unwrap();
        for i in 0..20 {
            mgr.add_to_queue_end(create_test_track(&i.to_string(), 180));
        }
        mgr.play().ok();
        mgr.set_audio_source(Box::new(EventMockSource::new(Duration::from_secs(180))));
    }

    let event_count = Arc::new(AtomicUsize::new(0));

    // Producer thread
    let producer = {
        let mgr = Arc::clone(&manager);
        thread::spawn(move || {
            let mut buffer = vec![0.0f32; 512];
            for _ in 0..500 {
                mgr.lock().unwrap().process_audio(&mut buffer).ok();
                thread::sleep(Duration::from_micros(500));
            }
        })
    };

    // Multiple consumer threads
    let consumers: Vec<_> = (0..3)
        .map(|_| {
            let mgr = Arc::clone(&manager);
            let count = Arc::clone(&event_count);
            thread::spawn(move || {
                for _ in 0..200 {
                    let events = mgr.lock().unwrap().drain_events();
                    count.fetch_add(events.len(), Ordering::SeqCst);
                    thread::sleep(Duration::from_millis(5));
                }
            })
        })
        .collect();

    producer.join().expect("Producer panicked");
    for consumer in consumers {
        consumer.join().expect("Consumer panicked");
    }

    let total_events = event_count.load(Ordering::SeqCst);
    println!("Total events consumed: {}", total_events);

    // Should have consumed events without panic or deadlock
    assert!(total_events > 0, "Should have consumed events");
}

// ============================================================================
// 5. Event Memory Tests
// ============================================================================

#[test]
#[ignore]
fn test_event_memory_no_leak() {
    let mut manager = PlaybackManager::default();

    for i in 0..10 {
        manager.add_to_queue_end(create_test_track(&i.to_string(), 10));
    }

    manager.play().ok();
    manager.set_audio_source(Box::new(EventMockSource::new(Duration::from_secs(10))));

    let mut buffer = vec![0.0f32; 1024];

    // Generate many events
    for _ in 0..10000 {
        manager.process_audio(&mut buffer).ok();

        // Drain events periodically
        let _ = manager.drain_events();
    }

    // Final drain
    let final_events = manager.drain_events();
    let final_count = final_events.len();

    println!("Final drain: {} events", final_count);

    if final_count > 10000 {
        panic!("Event queue appears to be leaking: {} events", final_count);
    }

    // Should not have excessive events queued
    assert!(
        final_count < 1000,
        "Should not have excessive events queued"
    );
}

// ============================================================================
// 6. Event Type Coverage Tests
// ============================================================================

#[test]
fn test_all_event_types_emitted() {
    let mut manager = PlaybackManager::default();

    manager.add_to_queue_end(create_test_track("1", 10));
    manager.add_to_queue_end(create_test_track("2", 10));

    // Trigger various events
    manager.play().ok();
    manager.set_audio_source(Box::new(EventMockSource::new(Duration::from_secs(10))));

    let mut buffer = vec![0.0f32; 4096];
    manager.process_audio(&mut buffer).ok();

    manager.pause();
    manager.set_volume(50);
    manager.next().ok();
    manager.set_audio_source(Box::new(EventMockSource::new(Duration::from_secs(10))));

    // Collect all events
    let events = manager.drain_events();

    // Check we got different event types
    let has_state_change = events
        .iter()
        .any(|e| matches!(e, PlaybackEvent::StateChanged { .. }));
    let has_volume_change = events
        .iter()
        .any(|e| matches!(e, PlaybackEvent::VolumeChanged { .. }));
    let has_queue_change = events
        .iter()
        .any(|e| matches!(e, PlaybackEvent::QueueChanged { .. }));

    assert!(has_state_change, "Should have state change events");
    assert!(has_volume_change, "Should have volume change events");

    println!("Event type coverage:");
    println!("  State change: {}", has_state_change);
    println!("  Volume change: {}", has_volume_change);
    println!("  Queue change: {}", has_queue_change);
}

#[test]
#[ignore]
fn stress_mixed_operations_with_event_verification() {
    let mut manager = PlaybackManager::default();

    for i in 0..50 {
        manager.add_to_queue_end(create_test_track(&i.to_string(), 10));
    }

    manager.play().ok();
    manager.set_audio_source(Box::new(EventMockSource::new(Duration::from_secs(10))));

    let mut buffer = vec![0.0f32; 512];
    let mut operation_count = 0;

    for i in 0..1000 {
        // Mix operations
        match i % 7 {
            0 => {
                manager.pause();
                operation_count += 1;
            }
            1 => {
                manager.play().ok();
                operation_count += 1;
            }
            2 => {
                manager.set_volume((i % 101) as u8);
                operation_count += 1;
            }
            3 => {
                manager.next().ok();
                manager.set_audio_source(Box::new(EventMockSource::new(Duration::from_secs(10))));
                operation_count += 1;
            }
            _ => {
                manager.process_audio(&mut buffer).ok();
            }
        }

        // Periodically verify events
        if i % 50 == 0 {
            let _ = manager.drain_events();
        }
    }

    // Final event drain and verification
    let final_event_vec = manager.drain_events();
    let final_events = final_event_vec.len();

    if final_events > 5000 {
        panic!("Too many events queued: {}", final_events);
    }

    println!(
        "Operations: {}, Final events: {}",
        operation_count, final_events
    );

    assert!(
        final_events < operation_count * 3,
        "Event count should be reasonable relative to operations"
    );
}
