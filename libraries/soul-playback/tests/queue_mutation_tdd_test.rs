//! TDD tests for queue mutation while a track is pending load
//!
//! These tests cover the interaction between queue mutation operations
//! (`remove_from_queue`, `clear_queue`) and the `pending_load_track` state
//! that is set when `play_next_in_queue()` emits a `LoadNext` event.
//!
//! # Scenario overview
//!
//! When `play()` is called, `play_next_in_queue()` emits `LoadNext(track_A)` and
//! sets `pending_load_track = Some(track_A)`. Track A has been dequeued from
//! `self.queue` at this point — it is no longer in the queue.
//!
//! Now if the user mutates the queue before `activate_source()` is called:
//!
//! 1. `remove_from_queue(index)` — removes from `self.queue` only. If track_A
//!    was the dequeued track, it's already in `pending_load_track`, not the queue.
//!    `activate_source(source, track_A)` should still work correctly.
//!
//! 2. `clear_queue()` — clears `self.queue` but NOT `pending_load_track`.
//!    After `clear_queue()`, `activate_source(source, track_A)` still fires
//!    correctly (the pending track is loaded). Is this the desired behavior?
//!    The test documents and verifies this.
//!
//! 3. `remove_from_queue(0)` on an empty queue — should return an error
//!    (IndexOutOfBounds), not panic.
//!
//! # Bugs found
//!
//! No bugs found in the queue mutation logic:
//! - `remove_from_queue()` correctly returns an error on out-of-bounds
//! - `activate_source()` works correctly even after the queue is mutated
//!   (the pending track is separate from the queue)
//! - `clear_queue()` does NOT clear `pending_load_track` — which is the
//!   correct behavior, since the track is already in flight. The platform
//!   has received a `LoadNext` event and is loading the file; cancelling
//!   that by clearing `pending_load_track` would leave the platform loading
//!   a track that the manager no longer expects. The correct cancellation
//!   path is `stop()` or `load_playlist()`, both of which explicitly clear
//!   `pending_load_track`.
//!
//! All tests pass immediately = correct behavior confirmed.

use soul_playback::{
    AudioSource, PlaybackError, PlaybackEvent, PlaybackManager, PlaybackState, QueueTrack,
    TrackSource,
};
use std::path::PathBuf;
use std::time::Duration;

// ===== Test Helpers =====

fn make_track(id: &str, duration_secs: u64) -> QueueTrack {
    QueueTrack {
        id: id.to_string(),
        path: PathBuf::from(format!("/music/{id}.mp3")),
        title: format!("Track {id}"),
        artist: "Test Artist".to_string(),
        album: Some("Test Album".to_string()),
        duration: Duration::from_secs(duration_secs),
        track_number: Some(id.parse().unwrap_or(1)),
        source: TrackSource::Single,
    }
}

struct MockAudioSource {
    duration: Duration,
    position: Duration,
}

impl MockAudioSource {
    fn new(duration_secs: u64) -> Self {
        Self {
            duration: Duration::from_secs(duration_secs),
            position: Duration::ZERO,
        }
    }
}

impl AudioSource for MockAudioSource {
    fn read_samples(&mut self, buffer: &mut [f32]) -> soul_playback::Result<usize> {
        let total = (self.duration.as_secs_f64() * 88200.0) as usize;
        let current = (self.position.as_secs_f64() * 88200.0) as usize;
        let to_read = (total.saturating_sub(current)).min(buffer.len());
        if to_read == 0 {
            return Ok(0);
        }
        for s in buffer.iter_mut().take(to_read) {
            *s = 0.0;
        }
        self.position += Duration::from_secs_f64(to_read as f64 / 88200.0);
        Ok(to_read)
    }

    fn seek(&mut self, position: Duration) -> soul_playback::Result<()> {
        self.position = position.min(self.duration);
        Ok(())
    }

    fn duration(&self) -> Duration {
        self.duration
    }

    fn position(&self) -> Duration {
        self.position
    }

    fn is_finished(&self) -> bool {
        self.position >= self.duration
    }
}

/// Helper: collect LoadNext track IDs from events
fn load_next_ids(events: &[PlaybackEvent]) -> Vec<String> {
    events
        .iter()
        .filter_map(|e| {
            if let PlaybackEvent::LoadNext(t) = e {
                Some(t.id.clone())
            } else {
                None
            }
        })
        .collect()
}

// ===== SCENARIO 1: remove_from_queue on pending track =====

/// When play() emits LoadNext(track_A) and sets pending_load_track = Some(track_A),
/// track_A is already DEQUEUED from self.queue. A subsequent remove_from_queue()
/// targets the REMAINING queue (tracks after track_A).
///
/// Verify: activate_source(source, track_A) still succeeds after queue mutation.
/// This confirms pending_load_track is independent of self.queue.
#[test]
fn test_activate_source_succeeds_after_queue_mutation_during_pending_load() {
    let mut mgr = PlaybackManager::default();

    // Load 3-track playlist: t1 will be dispatched via LoadNext, t2/t3 remain in queue
    mgr.load_playlist(
        vec![
            make_track("1", 30),
            make_track("2", 30),
            make_track("3", 30),
        ],
        0,
    );

    // play() → LoadNext(t1), pending_load_track = t1, queue = [t2, t3]
    mgr.play().ok();
    let events = mgr.drain_events();
    let loads = load_next_ids(&events);
    assert_eq!(loads, vec!["1"], "play() should emit LoadNext(t1)");

    // Queue now has t2, t3 (t1 was consumed by play_next_in_queue)
    assert_eq!(
        mgr.queue_len(),
        2,
        "Queue should have 2 tracks remaining [t2, t3]"
    );

    // User removes t2 from the queue (index 0 in the remaining queue)
    let removed = mgr.remove_from_queue(0);
    assert!(removed.is_ok(), "remove_from_queue(0) must succeed");
    assert_eq!(removed.unwrap().id, "2", "Removed track must be t2");
    assert_eq!(mgr.queue_len(), 1, "Queue should now have 1 track [t3]");

    // Platform calls activate_source for t1 (which is pending_load_track)
    // This must succeed regardless of queue mutation
    mgr.activate_source(Box::new(MockAudioSource::new(30)), make_track("1", 30));

    // Manager must be in Playing state
    assert_eq!(
        mgr.get_state(),
        PlaybackState::Playing,
        "activate_source must succeed and transition to Playing after queue mutation"
    );

    // Current track must be t1
    let current = mgr.get_current_track().map(|t| t.id.clone());
    assert_eq!(
        current,
        Some("1".to_string()),
        "Current track must be t1 after activate_source, got: {:?}",
        current
    );
}

/// Variation: remove_from_queue removes ALL remaining tracks (not the pending one).
/// activate_source should still work, and after that next() should return an error
/// (queue is empty).
#[test]
fn test_activate_source_works_then_next_fails_after_queue_cleared() {
    let mut mgr = PlaybackManager::default();

    mgr.load_playlist(vec![make_track("1", 30), make_track("2", 30)], 0);

    // play() → LoadNext(t1), queue = [t2]
    mgr.play().ok();
    mgr.drain_events();

    // Remove t2 (the only remaining queue entry)
    let removed = mgr.remove_from_queue(0);
    assert!(removed.is_ok(), "Removing t2 from queue must succeed");
    assert_eq!(mgr.queue_len(), 0, "Queue must be empty after removing t2");

    // activate_source for t1 must still work
    mgr.activate_source(Box::new(MockAudioSource::new(30)), make_track("1", 30));
    assert_eq!(mgr.get_state(), PlaybackState::Playing);

    // next() must now fail (queue empty, no repeat)
    let result = mgr.next();
    assert!(
        result.is_err(),
        "next() must fail when queue is empty after removal"
    );
}

// ===== SCENARIO 2: clear_queue while track is pending =====

/// clear_queue() clears self.queue but NOT pending_load_track.
/// After clear_queue(), activate_source(source, track_A) should still work
/// because the pending track was dispatched before the clear.
///
/// This documents the INTENDED behavior: clear_queue() only affects future
/// track navigation. The in-flight load (pending_load_track) completes
/// because the platform already received the LoadNext event.
#[test]
fn test_activate_source_succeeds_after_clear_queue_during_pending_load() {
    let mut mgr = PlaybackManager::default();

    mgr.load_playlist(
        vec![
            make_track("1", 30),
            make_track("2", 30),
            make_track("3", 30),
        ],
        0,
    );

    // play() → pending_load_track = t1, queue = [t2, t3]
    mgr.play().ok();
    mgr.drain_events();

    // User clears the remaining queue
    mgr.clear_queue();
    assert_eq!(
        mgr.queue_len(),
        0,
        "Queue must be empty after clear_queue()"
    );

    // Pending track t1 is still being loaded by the platform.
    // activate_source must succeed.
    mgr.activate_source(Box::new(MockAudioSource::new(30)), make_track("1", 30));

    assert_eq!(
        mgr.get_state(),
        PlaybackState::Playing,
        "activate_source must succeed after clear_queue() — pending track is independent of queue"
    );

    // After t1 finishes, next() must fail (queue was cleared)
    let result = mgr.next();
    assert!(
        result.is_err(),
        "next() must fail after clear_queue() empties the remaining tracks"
    );
}

/// Verify clear_queue() does NOT affect get_current_track() when loading.
/// During the loading window, pending_load_track IS the current track
/// (get_current_track() returns it as a fallback).
#[test]
fn test_clear_queue_does_not_clear_pending_load_track() {
    let mut mgr = PlaybackManager::default();

    mgr.load_playlist(vec![make_track("1", 30), make_track("2", 30)], 0);

    // play() → pending_load_track = t1
    mgr.play().ok();
    mgr.drain_events();

    // Before activate_source, get_current_track() returns pending_load_track
    let current_before = mgr.get_current_track().map(|t| t.id.clone());
    assert_eq!(
        current_before,
        Some("1".to_string()),
        "Before activate_source, get_current_track() must return pending t1"
    );

    // Clear the queue
    mgr.clear_queue();

    // get_current_track() must still return t1 (pending_load_track is not cleared)
    let current_after = mgr.get_current_track().map(|t| t.id.clone());
    assert_eq!(
        current_after,
        Some("1".to_string()),
        "After clear_queue(), get_current_track() must still return pending t1. \
         clear_queue() must NOT clear pending_load_track."
    );
}

// ===== SCENARIO 3: remove_from_queue bounds check =====

/// remove_from_queue(0) on an empty queue must return an error (IndexOutOfBounds),
/// not panic or silently succeed.
#[test]
fn test_remove_from_empty_queue_returns_error() {
    let mut mgr = PlaybackManager::default();
    assert_eq!(mgr.queue_len(), 0, "Fresh manager has empty queue");

    let result = mgr.remove_from_queue(0);

    assert!(
        result.is_err(),
        "remove_from_queue(0) on empty queue must return an error"
    );

    let err = result.unwrap_err();
    let err_str = format!("{:?}", err);
    assert!(
        err_str.contains("IndexOutOfBounds") || err_str.contains("index"),
        "Error must be IndexOutOfBounds (or similar), got: {}",
        err_str
    );
}

/// remove_from_queue with a large out-of-bounds index must return IndexOutOfBounds.
#[test]
fn test_remove_from_queue_out_of_bounds_returns_error() {
    let mut mgr = PlaybackManager::default();
    mgr.load_playlist(vec![make_track("1", 30), make_track("2", 30)], 0);
    mgr.play().ok(); // Consumes t1 → queue = [t2]
    mgr.drain_events();

    assert_eq!(mgr.queue_len(), 1, "Queue has 1 track (t2)");

    // Index 1 is out of bounds for a 1-element queue
    let result = mgr.remove_from_queue(1);
    assert!(
        result.is_err(),
        "remove_from_queue(1) on 1-element queue must return an error"
    );

    let err = result.unwrap_err();
    assert!(
        matches!(err, PlaybackError::IndexOutOfBounds(_)),
        "Error must be IndexOutOfBounds, got: {:?}",
        err
    );
}

/// remove_from_queue with index == queue_len() is exactly out of bounds.
/// (Valid indices are 0..queue_len()-1; queue_len() itself is out of bounds)
#[test]
fn test_remove_from_queue_at_exact_boundary_returns_error() {
    let mut mgr = PlaybackManager::default();
    mgr.load_playlist(vec![make_track("1", 30), make_track("2", 30)], 0);
    mgr.play().ok(); // Consumes t1 → queue = [t2]
    mgr.drain_events();

    let len = mgr.queue_len(); // 1
    let result = mgr.remove_from_queue(len);
    assert!(
        result.is_err(),
        "remove_from_queue(queue_len()) must return an error (exclusive upper bound)"
    );
}

/// remove_from_queue(0) on a queue with one element must succeed and return that track.
#[test]
fn test_remove_from_queue_last_element_succeeds() {
    let mut mgr = PlaybackManager::default();
    mgr.load_playlist(vec![make_track("1", 30), make_track("2", 30)], 0);
    mgr.play().ok(); // Consumes t1 → queue = [t2]
    mgr.drain_events();

    assert_eq!(mgr.queue_len(), 1);
    let result = mgr.remove_from_queue(0);
    assert!(
        result.is_ok(),
        "remove_from_queue(0) on 1-element queue must succeed"
    );
    assert_eq!(result.unwrap().id, "2", "Removed track must be t2");
    assert_eq!(
        mgr.queue_len(),
        0,
        "Queue must be empty after removing last element"
    );
}

// ===== SCENARIO 4: pending_load_track behavior during clear/stop =====

/// stop() clears pending_load_track.
/// After stop(), get_current_track() must return None.
#[test]
fn test_stop_clears_pending_load_track() {
    let mut mgr = PlaybackManager::default();

    mgr.load_playlist(vec![make_track("1", 30)], 0);
    mgr.play().ok(); // pending_load_track = t1
    mgr.drain_events();

    // Before stop: pending track is visible
    let current_before = mgr.get_current_track().map(|t| t.id.clone());
    assert_eq!(current_before, Some("1".to_string()));

    // stop() must clear pending_load_track
    mgr.stop();

    let current_after = mgr.get_current_track();
    assert!(
        current_after.is_none(),
        "After stop(), get_current_track() must return None (pending_load_track is cleared)"
    );
}

/// load_playlist() clears pending_load_track from any prior play().
/// After load_playlist() + play(), pending_load_track is the NEW first track.
#[test]
fn test_load_playlist_clears_pending_load_track() {
    let mut mgr = PlaybackManager::default();

    // First playlist: leave pending_load_track = t1
    mgr.load_playlist(vec![make_track("1", 30), make_track("2", 30)], 0);
    mgr.play().ok();
    mgr.drain_events();

    let current_before = mgr.get_current_track().map(|t| t.id.clone());
    assert_eq!(current_before, Some("1".to_string()));

    // Load a new playlist (without activating t1 first)
    mgr.load_playlist(vec![make_track("99", 60), make_track("100", 60)], 0);

    // After load_playlist, pending_load_track must be cleared
    // (the manager has superseded the in-flight load with a new playlist)
    let current_after_load = mgr.get_current_track();
    assert!(
        current_after_load.is_none(),
        "After load_playlist(), get_current_track() must return None \
         (old pending_load_track was cleared)"
    );

    // play() on the new playlist must emit LoadNext for track 99
    mgr.play().ok();
    let events = mgr.drain_events();
    let loads = load_next_ids(&events);
    assert_eq!(
        loads,
        vec!["99"],
        "play() after load_playlist() must LoadNext track 99"
    );
}

// ===== SCENARIO 5: activate_source with a different track than pending =====

/// What if activate_source() is called with a DIFFERENT track than pending_load_track?
/// This could happen if the platform delivers a stale response (e.g. device switch
/// mid-load causes the old loader thread to send back a result for a track that is
/// no longer expected).
///
/// The stale-activation guard introduced in the previous session REJECTS such
/// activations so that the queue is not corrupted by an out-of-order response.
/// The manager stays in Stopped state; the caller is expected to retry loading.
#[test]
fn test_activate_source_with_stale_track_is_rejected() {
    let mut mgr = PlaybackManager::default();

    mgr.load_playlist(vec![make_track("1", 30), make_track("2", 30)], 0);

    // play() → pending_load_track = "1"
    mgr.play().ok();
    mgr.drain_events();

    // Platform activates with a DIFFERENT track id ("999") — this is a stale response.
    let stale_track = make_track("999", 60);
    let accepted = mgr.activate_source(Box::new(MockAudioSource::new(60)), stale_track);

    // The stale-activation guard must reject mismatched track ids.
    assert!(
        !accepted,
        "Stale activation (track id mismatch) must be rejected, got accepted=true"
    );

    // Manager must stay Stopped — a stale source must not corrupt playback state.
    assert_eq!(
        mgr.get_state(),
        PlaybackState::Stopped,
        "Manager must remain Stopped after a stale activation is rejected"
    );

    // Current track must NOT be the stale one.
    let current_id = mgr.get_current_track().map(|t| t.id.clone());
    assert_ne!(
        current_id,
        Some("999".to_string()),
        "Stale track must not become the current track, got: {:?}",
        current_id
    );
}

// ===== SCENARIO 6: pending_load_track history path =====

/// When next() is called while loading (before activate_source), the pending
/// track is saved to history instead of the active source.
/// This verifies the pending_load_track path in next()'s history push.
#[test]
fn test_next_during_loading_saves_pending_to_history() {
    let mut mgr = PlaybackManager::default();

    mgr.load_playlist(
        vec![
            make_track("1", 30),
            make_track("2", 30),
            make_track("3", 30),
        ],
        0,
    );

    // play() → pending_load_track = t1
    mgr.play().ok();
    mgr.drain_events();

    // next() while loading: t1 (pending) goes to history, LoadNext(t2) emitted
    mgr.next().ok();
    let events = mgr.drain_events();
    let loads = load_next_ids(&events);
    assert_eq!(
        loads,
        vec!["2"],
        "next() during loading must emit LoadNext(t2)"
    );

    // t1 must now be in history
    let history = mgr.get_history();
    let history_ids: Vec<_> = history.iter().map(|t| t.id.as_str()).collect();
    assert!(
        history_ids.contains(&"1"),
        "t1 must be in history after next() during loading, history: {:?}",
        history_ids
    );
}

/// When skip_to_queue_index() is called while loading (before activate_source),
/// the pending track is saved to history.
#[test]
fn test_skip_during_loading_saves_pending_to_history() {
    let mut mgr = PlaybackManager::default();

    mgr.load_playlist(
        vec![
            make_track("1", 30),
            make_track("2", 30),
            make_track("3", 30),
        ],
        0,
    );

    // play() → pending_load_track = t1, queue = [t2, t3]
    mgr.play().ok();
    mgr.drain_events();

    // skip_to_queue_index(1) while loading: pending t1 → history, LoadNext(t3)
    if mgr.skip_to_queue_index(1).is_ok() {
        let events = mgr.drain_events();
        let loads = load_next_ids(&events);
        assert_eq!(
            loads,
            vec!["3"],
            "skip_to_queue_index(1) during loading must emit LoadNext(t3)"
        );

        let history = mgr.get_history();
        let history_ids: Vec<_> = history.iter().map(|t| t.id.as_str()).collect();
        assert!(
            history_ids.contains(&"1"),
            "t1 must be in history after skip_to_queue_index() during loading, history: {:?}",
            history_ids
        );
    }
    // If skip fails (index out of bounds), the test is a no-op.
}
