//! TDD tests for three-tier queue edge cases in PlaybackManager
//!
//! Covers interactions between play_next (LIFO), queued_later (FIFO), and source
//! queues, with emphasis on skip_to_queue_index indexing, clear operations,
//! peek_next_track consistency, and RepeatAll behaviour with explicit queue items.

use soul_playback::{
    AudioSource, PlaybackEvent, PlaybackManager, QueueTrack, RepeatMode, TrackSource,
};
use std::path::PathBuf;
use std::time::Duration;

// ===== Helpers =====

struct MockAudioSource {
    duration: Duration,
    position: Duration,
    finished: bool,
}

impl MockAudioSource {
    fn new(duration_secs: u64) -> Self {
        Self {
            duration: Duration::from_secs(duration_secs),
            position: Duration::ZERO,
            finished: false,
        }
    }
}

impl AudioSource for MockAudioSource {
    fn read_samples(&mut self, buffer: &mut [f32]) -> soul_playback::Result<usize> {
        if self.finished {
            return Ok(0);
        }
        let total = (self.duration.as_secs_f64() * 88200.0) as u64; // stereo 44100
        let current = (self.position.as_secs_f64() * 88200.0) as u64;
        let remaining = (total.saturating_sub(current)) as usize;
        let to_read = remaining.min(buffer.len());
        if to_read == 0 {
            self.finished = true;
            return Ok(0);
        }
        for s in buffer.iter_mut().take(to_read) {
            *s = 0.0;
        }
        self.position += Duration::from_secs_f64(to_read as f64 / 88200.0);
        Ok(to_read)
    }

    fn seek(&mut self, position: Duration) -> soul_playback::Result<()> {
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

fn track(id: &str) -> QueueTrack {
    QueueTrack {
        id: id.to_string(),
        path: PathBuf::from(format!("/music/{id}.mp3")),
        title: format!("Track {id}"),
        artist: "Artist".to_string(),
        album: Some("Album".to_string()),
        duration: Duration::from_secs(180),
        track_number: Some(1),
        source: TrackSource::Single,
    }
}

/// Load source, activate first track (simulates the platform responding to LoadNext),
/// and drain all pending events. Returns the manager with one track playing.
fn setup_playing(manager: &mut PlaybackManager, tracks: Vec<QueueTrack>) {
    let first = tracks[0].clone();
    manager.load_playlist(tracks, 0);
    manager.play().unwrap();
    // Drain the LoadNext event
    let _events = manager.drain_events();
    manager.activate_source(Box::new(MockAudioSource::new(180)), first);
    let _events = manager.drain_events();
}

/// Drain events and return all LoadNext track IDs in order.
fn drain_load_next_ids(manager: &mut PlaybackManager) -> Vec<String> {
    manager
        .drain_events()
        .into_iter()
        .filter_map(|e| {
            if let PlaybackEvent::LoadNext(t) = e {
                Some(t.id)
            } else {
                None
            }
        })
        .collect()
}

// ===== skip_to_queue_index with play_next items =====

/// skip_to_queue_index() counts play_next items as part of the visible queue.
/// Visible queue after play() starts s1: [p2, p1, s2, s3]
///   index 0 = p2, index 1 = p1, index 2 = s2, index 3 = s3
/// skip_to_queue_index(2) must dispatch LoadNext for s2.
#[test]
fn test_skip_to_index_counts_play_next_items_in_total() {
    let mut manager = PlaybackManager::default();

    // Load source [s1, s2, s3] and activate s1
    setup_playing(&mut manager, vec![track("s1"), track("s2"), track("s3")]);

    // Add play_next items (LIFO: p2 inserts before p1 → p2 is at index 0)
    manager.add_to_queue_next(track("p1"));
    manager.add_to_queue_next(track("p2"));

    // Visible queue is now: [p2, p1, s2, s3]
    // index 2 → s2
    manager.skip_to_queue_index(2).unwrap();
    let ids = drain_load_next_ids(&mut manager);

    assert_eq!(
        ids,
        vec!["s2"],
        "skip_to_queue_index(2) with 2 play_next items should dispatch LoadNext for s2"
    );
}

/// skip_to_queue_index(0) targets the topmost play_next item (p1, since only one was added).
#[test]
fn test_skip_to_index_zero_with_play_next_items() {
    let mut manager = PlaybackManager::default();

    // Load source [s1, s2] and activate s1
    setup_playing(&mut manager, vec![track("s1"), track("s2")]);

    // Add one play_next item
    manager.add_to_queue_next(track("p1"));

    // Visible queue: [p1, s2] — index 0 = p1
    manager.skip_to_queue_index(0).unwrap();
    let ids = drain_load_next_ids(&mut manager);

    assert_eq!(
        ids,
        vec!["p1"],
        "skip_to_queue_index(0) with a play_next item should dispatch LoadNext for p1"
    );
}

// ===== clear_play_next / clear_add_to_queue during playback =====

/// After clear_play_next(), calling next() must skip play_next items and advance
/// to the next source track instead.
#[test]
fn test_clear_play_next_during_playback() {
    let mut manager = PlaybackManager::default();

    // Load source [s1, s2] and activate s1
    setup_playing(&mut manager, vec![track("s1"), track("s2")]);

    // Add play_next items
    manager.add_to_queue_next(track("p1"));
    manager.add_to_queue_next(track("p2"));

    // Clear the play_next queue
    manager.clear_play_next();

    // Calling next() must now advance to s2 (source), not p1 or p2
    manager.next().unwrap();
    let ids = drain_load_next_ids(&mut manager);

    assert_eq!(
        ids,
        vec!["s2"],
        "After clear_play_next(), next() should dispatch LoadNext for s2 (source track)"
    );
    assert!(
        !ids.contains(&"p1".to_string()) && !ids.contains(&"p2".to_string()),
        "Cleared play_next tracks must not appear in LoadNext events"
    );
}

/// After clear_add_to_queue(), calling next() past the source must stop (or wrap),
/// but must never dispatch the cleared queued_later tracks.
#[test]
fn test_clear_add_to_queue_during_playback() {
    let mut manager = PlaybackManager::default();

    // Load source [s1, s2] and activate s1
    setup_playing(&mut manager, vec![track("s1"), track("s2")]);

    // Add to the queue_later tier
    manager.add_to_queue_end(track("q1"));
    manager.add_to_queue_end(track("q2"));

    // Clear the queued_later tier
    manager.clear_add_to_queue();

    // next() must advance to s2 (source), not q1 or q2
    manager.next().unwrap();
    let ids = drain_load_next_ids(&mut manager);

    assert_eq!(
        ids,
        vec!["s2"],
        "After clear_add_to_queue(), next() should dispatch LoadNext for s2"
    );
    assert!(
        !ids.contains(&"q1".to_string()) && !ids.contains(&"q2".to_string()),
        "Cleared queued_later tracks must not appear in LoadNext events"
    );
}

// ===== peek_next_track() vs actual next track consistency =====

/// peek_next_track() must return the same track that next() will dispatch via LoadNext.
/// With play_next items and an active track (non-empty history):
///   peek should show the topmost play_next item.
#[test]
fn test_peek_next_track_matches_actual_next_with_play_next() {
    let mut manager = PlaybackManager::default();

    // Load source [s1, s2] and activate s1 (builds history)
    setup_playing(&mut manager, vec![track("s1"), track("s2")]);

    // Add play_next item — history is non-empty so peek sees it
    manager.add_to_queue_next(track("p1"));

    let peeked = manager.peek_next_track().unwrap();
    assert_eq!(
        peeked.id, "p1",
        "peek_next_track() must return p1 when history is non-empty and p1 is in play_next"
    );

    // next() must dispatch LoadNext for that exact track
    manager.next().unwrap();
    let ids = drain_load_next_ids(&mut manager);

    assert_eq!(
        ids,
        vec!["p1"],
        "next() must dispatch LoadNext for the same track returned by peek_next_track()"
    );
}

/// With no play_next items and an active track, peek_next_track() must return
/// the next source track, consistent with what next() will dispatch.
#[test]
fn test_peek_next_track_matches_actual_next_source_only() {
    let mut manager = PlaybackManager::default();

    setup_playing(&mut manager, vec![track("s1"), track("s2"), track("s3")]);
    // History is non-empty now; no play_next items

    let peeked = manager.peek_next_track().unwrap();
    assert_eq!(
        peeked.id, "s2",
        "peek_next_track() must return s2 (next source track)"
    );

    manager.next().unwrap();
    let ids = drain_load_next_ids(&mut manager);
    assert_eq!(
        ids,
        vec!["s2"],
        "next() must dispatch LoadNext for s2, matching peek"
    );
}

/// When queue is exhausted and RepeatAll is active, peek_next_track() should
/// return the first source track (the one that will wrap around), not an error.
///
/// This tests whether peek_next_track() handles RepeatAll correctly.
#[test]
fn test_peek_next_track_with_repeat_all_at_end() {
    let mut manager = PlaybackManager::default();
    manager.set_repeat(RepeatMode::All);

    // Load [s1, s2], play through both
    setup_playing(&mut manager, vec![track("s1"), track("s2")]);

    // Advance through s2 so source queue is exhausted
    manager.next().unwrap(); // dispatches LoadNext for s2
    let _ = drain_load_next_ids(&mut manager);
    // Activate s2 to advance history
    manager.activate_source(Box::new(MockAudioSource::new(180)), track("s2"));
    let _ = manager.drain_events();

    // Source queue is now exhausted. With RepeatAll, next() would wrap to s1.
    // peek_next_track() should return s1 (not Err).
    let result = manager.peek_next_track();
    assert!(
        result.is_ok(),
        "peek_next_track() must not return Err when RepeatAll is active and source exists: {:?}",
        result
    );
    assert_eq!(
        result.unwrap().id,
        "s1",
        "peek_next_track() with RepeatAll at end of queue must return s1 (first source track)"
    );
}

/// Empty queue (all tiers empty): peek_next_track() must return Err, not panic.
#[test]
fn test_peek_next_track_with_empty_queue() {
    let manager = PlaybackManager::default();
    let result = manager.peek_next_track();
    assert!(
        result.is_err(),
        "peek_next_track() on an empty queue must return Err, not panic"
    );
}

// ===== RepeatAll with explicit queue items =====

/// RepeatAll must NOT cycle queued_later (Add to Queue) items.
/// After playing s1 → s2 → q1 with RepeatAll, the next track must be s1
/// (only the source queue loops), never q1 again.
#[test]
fn test_repeat_all_does_not_cycle_explicit_queue_items() {
    let mut manager = PlaybackManager::default();
    manager.set_repeat(RepeatMode::All);

    // Load source [s1, s2] and activate s1
    setup_playing(&mut manager, vec![track("s1"), track("s2")]);

    // Add q1 to queued_later
    manager.add_to_queue_end(track("q1"));

    // Advance to s2
    manager.next().unwrap();
    let _ = drain_load_next_ids(&mut manager);
    manager.activate_source(Box::new(MockAudioSource::new(180)), track("s2"));
    let _ = manager.drain_events();

    // Advance to q1 (source exhausted, queued_later plays)
    manager.next().unwrap();
    let ids = drain_load_next_ids(&mut manager);
    assert_eq!(ids, vec!["q1"], "should dispatch q1 after source exhausted");

    manager.activate_source(Box::new(MockAudioSource::new(180)), track("q1"));
    let _ = manager.drain_events();

    // Now both source and queued_later are exhausted.
    // RepeatAll must reload only source → next should be s1.
    manager.next().unwrap();
    let ids = drain_load_next_ids(&mut manager);

    assert_eq!(
        ids,
        vec!["s1"],
        "RepeatAll must cycle back to s1 (source only), not q1"
    );
    assert!(
        !ids.contains(&"q1".to_string()),
        "q1 (queued_later) must NOT repeat under RepeatAll"
    );
}

/// play_next items are consumed with priority before the source queue continues,
/// even in RepeatAll mode.
#[test]
fn test_play_next_consumed_before_source_in_repeat_all() {
    let mut manager = PlaybackManager::default();
    manager.set_repeat(RepeatMode::All);

    // Load source [s1, s2] and activate s1
    setup_playing(&mut manager, vec![track("s1"), track("s2")]);

    // Add p1 to play_next while s1 is playing
    manager.add_to_queue_next(track("p1"));

    // peek must show p1 (history is non-empty)
    let peeked = manager.peek_next_track().unwrap();
    assert_eq!(
        peeked.id, "p1",
        "peek_next_track() must show p1 when it is in play_next and history is non-empty"
    );

    // next() must play p1 first
    manager.next().unwrap();
    let ids = drain_load_next_ids(&mut manager);
    assert_eq!(
        ids,
        vec!["p1"],
        "play_next item p1 must be dispatched before continuing source in RepeatAll mode"
    );

    manager.activate_source(Box::new(MockAudioSource::new(180)), track("p1"));
    let _ = manager.drain_events();

    // After p1, source continues with s2
    manager.next().unwrap();
    let ids = drain_load_next_ids(&mut manager);
    assert_eq!(
        ids,
        vec!["s2"],
        "After consuming play_next item p1, source must continue with s2"
    );
}
