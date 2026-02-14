//! End-to-end tests for automatic track advancement
//!
//! Tests verify that when a track finishes playing naturally (reaches the end),
//! the playback manager automatically advances to the next track in the queue.
//!
//! Critical behaviors tested:
//! - Auto-advance when track finishes with more tracks in queue
//! - Auto-advance stops when queue is empty (repeat OFF)
//! - Auto-advance loops when repeat ALL is enabled
//! - Auto-advance restarts track when repeat ONE is enabled
//! - LoadNext event is emitted for desktop layer to load source

use soul_playback::{
    AudioSource, PlaybackError, PlaybackEvent, PlaybackManager, PlaybackState, QueueTrack,
    RepeatMode, Result, ShuffleMode, TrackSource,
};
use std::path::PathBuf;
use std::time::Duration;

// ============================================================================
// Test Infrastructure
// ============================================================================

/// Mock audio source that can simulate track completion
struct MockAudioSource {
    duration: Duration,
    position: Duration,
    sample_rate: u32,
    channels: u16,
    finished: bool,
}

impl MockAudioSource {
    fn new(duration_secs: u64) -> Self {
        Self {
            duration: Duration::from_secs(duration_secs),
            position: Duration::ZERO,
            sample_rate: 44100,
            channels: 2,
            finished: false,
        }
    }

    /// Jump to near the end (useful for testing)
    fn seek_near_end(&mut self) {
        self.position = self.duration.saturating_sub(Duration::from_millis(100));
        self.finished = false;
    }
}

impl AudioSource for MockAudioSource {
    fn read_samples(&mut self, buffer: &mut [f32]) -> Result<usize> {
        if self.finished || self.position >= self.duration {
            self.finished = true;
            return Ok(0); // Signal track finished
        }

        let samples_per_second = self.sample_rate as u64 * self.channels as u64;
        let total_samples = (self.duration.as_secs_f64() * samples_per_second as f64) as u64;
        let current_sample = (self.position.as_secs_f64() * samples_per_second as f64) as u64;

        let remaining = (total_samples.saturating_sub(current_sample)) as usize;
        let to_read = remaining.min(buffer.len());

        if to_read == 0 {
            self.finished = true;
            return Ok(0);
        }

        // Fill buffer with test pattern
        for sample in buffer.iter_mut().take(to_read) {
            *sample = 0.5;
        }

        // Advance position
        let duration_read = Duration::from_secs_f64(to_read as f64 / samples_per_second as f64);
        self.position += duration_read;

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

    fn reset(&mut self) -> Result<()> {
        self.position = Duration::ZERO;
        self.finished = false;
        Ok(())
    }

    fn sample_rate(&self) -> Option<u32> {
        Some(self.sample_rate)
    }

    fn is_ready(&self) -> bool {
        true
    }
}

fn create_test_track(id: &str, title: &str, duration_secs: u64) -> QueueTrack {
    QueueTrack {
        id: id.to_string(),
        path: PathBuf::from(format!("/test/{}.mp3", id)),
        title: title.to_string(),
        artist: "Test Artist".to_string(),
        album: Some("Test Album".to_string()),
        duration: Duration::from_secs(duration_secs),
        track_number: Some(id.parse().unwrap_or(1)),
        source: TrackSource::Album {
            id: "album1".to_string(),
            name: "Test Album".to_string(),
        },
    }
}

/// Process audio until track finishes or max iterations reached
fn process_until_finished(manager: &mut PlaybackManager, max_iterations: usize) -> usize {
    let mut buffer = vec![0.0f32; 2048];
    let mut iterations = 0;

    while iterations < max_iterations {
        match manager.process_audio(&mut buffer) {
            Ok(0) => break, // Track finished
            Ok(_) => {
                iterations += 1;
            }
            Err(e) => {
                eprintln!("Error processing audio: {:?}", e);
                break;
            }
        }
    }

    iterations
}

// ============================================================================
// Auto-Advance Tests
// ============================================================================

#[test]
fn auto_advance_to_next_track_in_queue() {
    let mut manager = PlaybackManager::default();

    // Load a queue with 3 tracks
    let tracks = vec![
        create_test_track("1", "Track 1", 1), // 1 second track
        create_test_track("2", "Track 2", 1),
        create_test_track("3", "Track 3", 1),
    ];
    manager.load_playlist(tracks, 0);

    // Start playing first track
    manager.play().expect("Failed to start playback");

    // Get the LoadNext event to simulate loading the source
    let events = manager.drain_events();
    assert!(
        events
            .iter()
            .any(|e| matches!(e, PlaybackEvent::LoadNext(_))),
        "Expected LoadNext event for first track"
    );

    // Simulate loading the first track's source
    let mut source1 = MockAudioSource::new(1);
    source1.seek_near_end(); // Jump near end for faster test
    manager.activate_source(Box::new(source1), create_test_track("1", "Track 1", 1));

    assert_eq!(manager.get_state(), PlaybackState::Playing);

    // Process audio until track finishes
    process_until_finished(&mut manager, 1000);

    // Check that LoadNext event was emitted for track 2
    let events: Vec<_> = manager.drain_events();
    let has_load_next = events.iter().any(|e| {
        if let PlaybackEvent::LoadNext(track) = e {
            track.id == "2"
        } else {
            false
        }
    });

    assert!(
        has_load_next,
        "Expected LoadNext event for track 2 after track 1 finished. Events: {:?}",
        events
    );
}

#[test]
fn auto_advance_stops_when_queue_empty_repeat_off() {
    let mut manager = PlaybackManager::default();
    manager.set_repeat(RepeatMode::Off);

    // Load queue with single track
    manager.load_playlist(vec![create_test_track("1", "Only Track", 1)], 0);

    // Start playback
    manager.play().expect("Failed to start playback");

    // Clear LoadNext event
    let _events: Vec<_> = manager.drain_events();

    // Activate source
    let mut source = MockAudioSource::new(1);
    source.seek_near_end();
    manager
        .activate_source(Box::new(source), create_test_track("1", "Only Track", 1))
        .expect("Failed to activate source");

    // Process until finished
    process_until_finished(&mut manager, 1000);

    // Should NOT emit LoadNext (no more tracks)
    let events: Vec<_> = manager.drain_events();
    let has_load_next = events
        .iter()
        .any(|e| matches!(e, PlaybackEvent::LoadNext(_)));

    assert!(
        !has_load_next,
        "Should NOT emit LoadNext when queue is empty and repeat is OFF. Events: {:?}",
        events
    );

    // State should be Stopped
    assert_eq!(
        manager.get_state(),
        PlaybackState::Stopped,
        "Playback should stop when queue is empty"
    );
}

#[test]
fn auto_advance_loops_with_repeat_all() {
    let mut manager = PlaybackManager::default();
    manager.set_repeat(RepeatMode::All);

    // Load queue with 2 tracks
    let tracks = vec![
        create_test_track("1", "Track 1", 1),
        create_test_track("2", "Track 2", 1),
    ];
    manager.load_playlist(tracks, 0);

    // Start playing and activate first track
    manager.play().expect("Failed to start playback");
    let _events: Vec<_> = manager.drain_events();

    let mut source1 = MockAudioSource::new(1);
    source1.seek_near_end();
    manager
        .activate_source(Box::new(source1), create_test_track("1", "Track 1", 1))
        .expect("Failed to activate source");

    // Process track 1 until finished
    process_until_finished(&mut manager, 1000);

    // Should load track 2
    let events: Vec<_> = manager.drain_events();
    assert!(
        events
            .iter()
            .any(|e| matches!(e, PlaybackEvent::LoadNext(t) if t.id == "2")),
        "Expected LoadNext for track 2"
    );

    // Activate track 2
    let mut source2 = MockAudioSource::new(1);
    source2.seek_near_end();
    manager
        .activate_source(Box::new(source2), create_test_track("2", "Track 2", 1))
        .expect("Failed to activate source");

    // Process track 2 until finished
    process_until_finished(&mut manager, 1000);

    // With repeat ALL, should loop back to track 1
    let events: Vec<_> = manager.drain_events();
    let has_loop_back = events.iter().any(|e| {
        if let PlaybackEvent::LoadNext(track) = e {
            track.id == "1"
        } else {
            false
        }
    });

    assert!(
        has_loop_back,
        "Expected LoadNext for track 1 (loop back) with repeat ALL. Events: {:?}",
        events
    );
}

#[test]
fn auto_advance_restarts_track_with_repeat_one() {
    let mut manager = PlaybackManager::default();
    manager.set_repeat(RepeatMode::One);

    // Load queue
    manager.load_playlist(vec![create_test_track("1", "Track 1", 1)], 0);

    // Start playback
    manager.play().expect("Failed to start playback");
    let _events: Vec<_> = manager.drain_events();

    // Activate source
    let mut source = MockAudioSource::new(1);
    source.seek_near_end();
    manager
        .activate_source(Box::new(source), create_test_track("1", "Track 1", 1))
        .expect("Failed to activate source");

    let initial_position = manager.get_position();

    // Process until finished
    process_until_finished(&mut manager, 1000);

    // With repeat ONE, track should restart (position reset to 0)
    let new_position = manager.get_position();

    assert!(
        new_position < initial_position,
        "Track should restart with repeat ONE. Initial: {:?}, New: {:?}",
        initial_position,
        new_position
    );

    // Should still be playing
    assert_eq!(
        manager.get_state(),
        PlaybackState::Playing,
        "Should still be playing with repeat ONE"
    );
}

#[test]
fn auto_advance_emits_track_finished_event() {
    let mut manager = PlaybackManager::default();

    // Load queue with 2 tracks
    let tracks = vec![
        create_test_track("1", "Track 1", 1),
        create_test_track("2", "Track 2", 1),
    ];
    manager.load_playlist(tracks, 0);

    // Start playback
    manager.play().expect("Failed to start playback");
    let _events: Vec<_> = manager.drain_events();

    // Activate source
    let mut source = MockAudioSource::new(1);
    source.seek_near_end();
    manager
        .activate_source(Box::new(source), create_test_track("1", "Track 1", 1))
        .expect("Failed to activate source");

    // Process until finished
    process_until_finished(&mut manager, 1000);

    // Check events
    let events: Vec<_> = manager.drain_events();

    // Should have both TrackFinished and LoadNext events
    let has_track_finished = events
        .iter()
        .any(|e| matches!(e, PlaybackEvent::TrackFinished { track_id } if track_id == "1"));
    let has_load_next = events
        .iter()
        .any(|e| matches!(e, PlaybackEvent::LoadNext(t) if t.id == "2"));

    assert!(
        has_track_finished,
        "Expected TrackFinished event. Events: {:?}",
        events
    );
    assert!(
        has_load_next,
        "Expected LoadNext event. Events: {:?}",
        events
    );
}

#[test]
fn auto_advance_preserves_queue_order() {
    let mut manager = PlaybackManager::default();
    manager.set_shuffle(ShuffleMode::Off);

    // Load queue with 4 tracks in specific order
    let tracks = vec![
        create_test_track("1", "First", 1),
        create_test_track("2", "Second", 1),
        create_test_track("3", "Third", 1),
        create_test_track("4", "Fourth", 1),
    ];
    manager.load_playlist(tracks, 0);

    // Verify queue order by playing through tracks
    let expected_order = vec!["1", "2", "3", "4"];
    let mut actual_order = Vec::new();

    for expected_id in &expected_order {
        // Start playback (or continue)
        if manager.get_state() == PlaybackState::Stopped {
            manager.play().expect("Failed to start playback");
        }

        // Get LoadNext event
        let events: Vec<_> = manager.drain_events();
        if let Some(PlaybackEvent::LoadNext(track)) = events
            .iter()
            .find(|e| matches!(e, PlaybackEvent::LoadNext(_)))
        {
            actual_order.push(track.id.clone());

            // Activate source
            let mut source = MockAudioSource::new(1);
            source.seek_near_end();
            manager
                .activate_source(Box::new(source), track.clone())
                .expect("Failed to activate source");

            // Process until finished
            if expected_id != &"4" {
                process_until_finished(&mut manager, 1000);
            }
        }
    }

    assert_eq!(
        actual_order, expected_order,
        "Tracks should play in original queue order"
    );
}
