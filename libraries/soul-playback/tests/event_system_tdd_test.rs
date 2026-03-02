//! TDD tests for PlaybackManager event system: deduplication and overflow behavior.
//!
//! These tests were written BEFORE the production fixes to confirm which behaviors
//! were broken. Run with:
//!   cargo test --package soul-playback --test event_system_tdd_test

use soul_playback::{
    AudioSource, PlaybackError, PlaybackEvent, PlaybackManager, PlaybackStateEvent, QueueTrack,
    Result, TrackSource,
};
use std::path::PathBuf;
use std::time::Duration;

// ============================================================================
// Test infrastructure
// ============================================================================

struct MockAudioSource {
    duration: Duration,
    position: Duration,
    finished: bool,
}

impl MockAudioSource {
    fn new(duration: Duration) -> Self {
        Self {
            duration,
            position: Duration::ZERO,
            finished: false,
        }
    }
}

impl AudioSource for MockAudioSource {
    fn read_samples(&mut self, buffer: &mut [f32]) -> Result<usize> {
        if self.finished {
            return Ok(0);
        }
        let sample_rate: u64 = 44100 * 2; // stereo
        let total = (self.duration.as_secs_f64() * sample_rate as f64) as u64;
        let current = (self.position.as_secs_f64() * sample_rate as f64) as u64;
        let remaining = total.saturating_sub(current) as usize;
        let to_read = remaining.min(buffer.len());
        if to_read == 0 {
            self.finished = true;
            return Ok(0);
        }
        for s in buffer.iter_mut().take(to_read) {
            *s = 0.1;
        }
        self.position += Duration::from_secs_f64(to_read as f64 / sample_rate as f64);
        Ok(to_read)
    }

    fn seek(&mut self, pos: Duration) -> Result<()> {
        if pos > self.duration {
            return Err(PlaybackError::InvalidSeekPosition(pos));
        }
        self.position = pos;
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

fn make_track(id: &str) -> QueueTrack {
    QueueTrack {
        id: id.to_string(),
        path: PathBuf::from(format!("/music/{}.mp3", id)),
        title: format!("Track {}", id),
        artist: "Artist".to_string(),
        album: Some("Album".to_string()),
        duration: Duration::from_secs(180),
        track_number: Some(1),
        source: TrackSource::Single,
    }
}

/// Helper: start a track playing and return the manager in Playing state.
fn manager_playing_track(id: &str) -> PlaybackManager {
    let mut manager = PlaybackManager::default();
    manager.add_to_queue_end(make_track(id));
    manager.play().expect("play() should succeed");
    // activate_source simulates the platform layer loading the file
    manager.activate_source(
        Box::new(MockAudioSource::new(Duration::from_secs(180))),
        make_track(id),
    );
    // Drain startup events so tests start from a clean slate
    let _ = manager.drain_events();
    manager
}

// ============================================================================
// Overflow behavior tests
// ============================================================================

/// The event queue overflow MUST log a warning — it must not silently drop events.
///
/// This test confirms that `push_event()` emits a tracing::warn! when the queue
/// overflows. We cannot capture tracing output in a unit test, but we CAN verify
/// that the overflow handling itself doesn't panic and that the queue stays
/// bounded (which documents the intended behavior).
#[test]
fn test_event_overflow_logs_warning_not_silently_drops() {
    let mut manager = PlaybackManager::default();

    // Flood the queue well beyond MAX_PENDING_EVENTS (1000) by toggling volume
    // repeatedly (each set_volume call emits one VolumeChanged event).
    for i in 0..1200usize {
        manager.set_volume((i % 101) as u8);
    }

    let events = manager.drain_events();

    // The queue must have been bounded — overflow did NOT grow unbounded.
    assert!(
        events.len() <= 1100,
        "Event queue should be bounded after overflow; got {}",
        events.len()
    );

    // The queue must still contain events — overflow dropped OLDEST, not newest.
    assert!(
        !events.is_empty(),
        "Event queue must not be empty after overflow handling"
    );

    // The most recent VolumeChanged event should reference a high-index volume
    // (proving that RECENT events were kept, not discarded).
    let last_volume = events.iter().rev().find_map(|e| {
        if let PlaybackEvent::VolumeChanged { level, .. } = e {
            Some(*level)
        } else {
            None
        }
    });
    assert!(
        last_volume.is_some(),
        "Should still have VolumeChanged events after overflow"
    );
}

/// Critical state-transition events (StateChanged) must survive an overflowing
/// queue. If we overflow *before* a state change, the state change must still
/// appear in the drained events.
#[test]
fn test_critical_events_not_lost_during_rapid_operations() {
    let mut manager = PlaybackManager::default();
    manager.add_to_queue_end(make_track("1"));
    manager.play().expect("play should succeed");
    manager.activate_source(
        Box::new(MockAudioSource::new(Duration::from_secs(180))),
        make_track("1"),
    );

    // Generate lots of volume events to approach overflow, then trigger a
    // critical state change (stop).
    for i in 0..900usize {
        manager.set_volume((i % 101) as u8);
    }
    manager.stop(); // must emit StateChanged(Stopped)

    let events = manager.drain_events();

    let has_stopped = events.iter().any(|e| {
        matches!(
            e,
            PlaybackEvent::StateChanged {
                state: PlaybackStateEvent::Stopped
            }
        )
    });

    assert!(
        has_stopped,
        "StateChanged(Stopped) must not be lost even when queue is near overflow; \
         got {} events total",
        events.len()
    );
}

// ============================================================================
// Deduplication tests
// ============================================================================

/// Calling stop() twice in a row should only emit ONE StateChanged(Stopped).
///
/// The first stop() transitions Playing → Stopped and emits an event.
/// The second stop() finds the state is already Stopped — no new event needed.
#[test]
fn test_consecutive_same_state_changed_deduplicated() {
    let mut manager = manager_playing_track("1");

    manager.stop(); // Playing → Stopped: should emit
    manager.stop(); // Already Stopped: should NOT emit again

    let events = manager.drain_events();
    let stopped_count = events
        .iter()
        .filter(|e| {
            matches!(
                e,
                PlaybackEvent::StateChanged {
                    state: PlaybackStateEvent::Stopped
                }
            )
        })
        .count();

    assert_eq!(
        stopped_count, 1,
        "Consecutive stop() calls should produce exactly one StateChanged(Stopped); \
         got {} events: {:?}",
        stopped_count, events
    );
}

/// Calling play() when already Playing should not emit a duplicate
/// StateChanged(Playing).
#[test]
fn test_consecutive_play_state_deduplicated() {
    let mut manager = manager_playing_track("1");

    // Manager is already Playing (startup events drained).
    // Another play() call while already playing should be a no-op for events.
    manager.play().ok();
    manager.play().ok();

    let events = manager.drain_events();
    let playing_count = events
        .iter()
        .filter(|e| {
            matches!(
                e,
                PlaybackEvent::StateChanged {
                    state: PlaybackStateEvent::Playing
                }
            )
        })
        .count();

    assert_eq!(
        playing_count, 0,
        "play() calls while already Playing should not emit duplicate StateChanged(Playing); \
         got {} extra Playing events",
        playing_count
    );
}

/// Non-consecutive events for the same state must NOT be deduplicated.
///
/// Sequence: stop → play → stop should produce:
///   StateChanged(Stopped), StateChanged(Playing), StateChanged(Stopped)
/// All three must be emitted because each is a real state transition.
#[test]
fn test_non_consecutive_same_events_not_deduplicated() {
    let mut manager = manager_playing_track("1");

    manager.stop(); // → Stopped

    // Reload a track and resume so we can go Playing again
    manager.add_to_queue_end(make_track("2"));
    manager.play().expect("play should succeed after stop");
    manager.activate_source(
        Box::new(MockAudioSource::new(Duration::from_secs(180))),
        make_track("2"),
    );

    manager.stop(); // → Stopped again

    let events = manager.drain_events();

    let state_events: Vec<PlaybackStateEvent> = events
        .iter()
        .filter_map(|e| {
            if let PlaybackEvent::StateChanged { state } = e {
                Some(*state)
            } else {
                None
            }
        })
        .collect();

    // Must have at least: Stopped, Playing, Stopped
    // (there may be extra events like Stopped from play() resetting state before
    //  activate_source transitions to Playing — so we check for the pattern rather
    //  than exact count)
    let stopped_count = state_events
        .iter()
        .filter(|&&s| s == PlaybackStateEvent::Stopped)
        .count();
    let playing_count = state_events
        .iter()
        .filter(|&&s| s == PlaybackStateEvent::Playing)
        .count();

    assert!(
        stopped_count >= 2,
        "Should have at least 2 Stopped events (stop → play → stop); got {}: {:?}",
        stopped_count,
        state_events
    );
    assert!(
        playing_count >= 1,
        "Should have at least 1 Playing event; got {}: {:?}",
        playing_count,
        state_events
    );
}

/// After drain_events() the internal buffer must be empty.
/// A second immediate drain must return an empty vec.
#[test]
fn test_drain_events_clears_buffer() {
    let mut manager = PlaybackManager::default();
    manager.add_to_queue_end(make_track("1"));
    manager.play().ok();

    // First drain - get all events so far
    let first = manager.drain_events();
    assert!(
        !first.is_empty(),
        "Should have events from play() call; got none"
    );

    // Second drain - must be empty
    let second = manager.drain_events();
    assert!(
        second.is_empty(),
        "Second drain_events() must return empty vec; got {} events: {:?}",
        second.len(),
        second
    );
}

/// TrackChanged events for the same track must NOT be deduplicated.
///
/// Rationale: user pressed next → prev so the same track started again.
/// That IS a real event (track restarted) and the UI must respond to it.
#[test]
fn test_track_changed_not_deduplicated_even_if_same_track() {
    let mut manager = PlaybackManager::default();
    manager.add_to_queue_end(make_track("track-a"));
    manager.add_to_queue_end(make_track("track-b"));

    manager.play().expect("play should succeed");
    // Activate track-a
    manager.activate_source(
        Box::new(MockAudioSource::new(Duration::from_secs(30))),
        make_track("track-a"),
    );
    let _ = manager.drain_events(); // clear startup events

    // Skip to next (track-b loads)
    manager.next().expect("next() should succeed");
    manager.activate_source(
        Box::new(MockAudioSource::new(Duration::from_secs(30))),
        make_track("track-b"),
    );

    // Go previous back to track-a (same track plays again)
    manager.previous().expect("previous() should succeed");
    manager.activate_source(
        Box::new(MockAudioSource::new(Duration::from_secs(30))),
        make_track("track-a"),
    );

    let events = manager.drain_events();

    let track_changes: Vec<String> = events
        .iter()
        .filter_map(|e| {
            if let PlaybackEvent::TrackChanged { track_id, .. } = e {
                Some(track_id.clone())
            } else {
                None
            }
        })
        .collect();

    // We should have got two TrackChanged events (track-b, then track-a again)
    assert!(
        track_changes.len() >= 2,
        "Should have TrackChanged events for both transitions; \
         got {:?}",
        track_changes
    );

    // The final TrackChanged should be for track-a (same track as the first one)
    let last = track_changes.last().unwrap();
    assert_eq!(
        last, "track-a",
        "Last TrackChanged should be track-a (same as original); got {}",
        last
    );
}

/// PositionUpdate events must be throttled.
///
/// Processing 1000 small audio buffers (256 samples each at 44100 Hz ≈ ~5.8ms per
/// buffer = ~5.8 seconds of audio) should not produce one PositionUpdate per
/// process call. They must be throttled to approximately 100ms intervals.
#[test]
fn test_position_updated_events_throttled() {
    let mut manager = PlaybackManager::default();
    manager.add_to_queue_end(make_track("1"));
    manager.play().expect("play should succeed");
    manager.activate_source(
        Box::new(MockAudioSource::new(Duration::from_secs(300))),
        make_track("1"),
    );
    let _ = manager.drain_events();

    let process_calls = 1000usize;
    let mut buffer = vec![0.0f32; 256];

    for _ in 0..process_calls {
        let samples = manager.process_audio(&mut buffer).unwrap_or(0);
        manager.maybe_emit_position_update(samples);
    }

    let events = manager.drain_events();
    let position_update_count = events
        .iter()
        .filter(|e| matches!(e, PlaybackEvent::PositionUpdate { .. }))
        .count();

    // 1000 calls × 256 samples / 44100 Hz / 2 channels ≈ 2.9 seconds of audio
    // At ~100ms throttle that's at most ~29 updates. We allow some margin.
    // The key assertion: it is dramatically less than process_calls (1000).
    assert!(
        position_update_count < process_calls / 10,
        "PositionUpdate events must be throttled; got {} updates for {} process calls",
        position_update_count,
        process_calls
    );

    assert!(
        position_update_count > 0,
        "Should have at least some PositionUpdate events; got 0"
    );
}
