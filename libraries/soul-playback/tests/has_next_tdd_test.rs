//! TDD tests for `has_next()` correctness across repeat modes
//!
//! Covers the following cases:
//!
//! 1. RepeatAll at end of source queue → `has_next()` true (can always cycle)
//! 2. RepeatOne active with a loaded track → `has_next()` true (same track repeats)
//! 3. RepeatOne with NO current track and empty queue → `has_next()` false (nothing to repeat)
//! 4. RepeatAll with a single-track source queue → `has_next()` true
//! 5. RepeatOff with empty queue → `has_next()` false
//! 6. Track consumption sequence: RepeatOff vs RepeatAll vs RepeatOne
//! 7. "Play Next" queue items + RepeatAll on source: explicit queue takes priority,
//!    and has_next() still returns true for RepeatAll after source exhaustion

use soul_playback::{AudioSource, PlaybackManager, QueueTrack, RepeatMode, TrackSource};
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

/// Minimal mock audio source used to activate a track in the manager
/// (simulates the platform loading and providing an audio stream).
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

// ===== Test 1: RepeatAll at end of source queue =====

#[test]
fn test_has_next_repeat_all_at_end_of_source_queue() {
    // RepeatAll: even after all source tracks are consumed (source_index >= source.len()),
    // has_next() must return true because the queue will reload and cycle.
    let mut manager = PlaybackManager::default();
    manager.set_repeat(RepeatMode::All);

    manager.load_playlist(
        vec![
            make_track("1", 180),
            make_track("2", 180),
            make_track("3", 180),
        ],
        0,
    );

    // Play through all three tracks via next(), exhausting the source queue
    manager.play().ok();
    manager.next().ok(); // 1 → history, 2 loading
    manager.next().ok(); // 2 → history, 3 loading
    manager.next().ok(); // 3 → history, queue exhausted → RepeatAll reloads

    // After exhausting the queue with RepeatAll, has_next must still be true
    assert!(
        manager.has_next(),
        "RepeatAll: has_next() must be true even after exhausting the source queue, \
         because the queue will loop from the beginning"
    );
}

// ===== Test 2: RepeatOne with a current track loaded =====

#[test]
fn test_has_next_repeat_one_with_current_track() {
    // RepeatOne: when a track is actively loaded, has_next() must be true
    // because the same track will play again.
    let mut manager = PlaybackManager::default();
    manager.set_repeat(RepeatMode::One);

    let track = make_track("1", 180);
    manager.load_playlist(vec![track.clone()], 0);

    // Trigger play so a LoadNext event is emitted, then activate the source
    // so the manager has a current track in SourceState
    manager.play().ok();
    manager.activate_source(Box::new(MockAudioSource::new(180)), track);

    // A current track is loaded — RepeatOne guarantees has_next() = true
    assert!(
        manager.has_next(),
        "RepeatOne: has_next() must be true when a track is currently loaded"
    );
}

// ===== Test 3: RepeatOne with NO current track and empty queue =====

#[test]
fn test_has_next_repeat_one_no_current_track_empty_queue() {
    // RepeatOne with NO active track and NO queued tracks: there is nothing to repeat.
    // has_next() must return false, not true.
    let mut manager = PlaybackManager::default();
    manager.set_repeat(RepeatMode::One);

    // No tracks added — queue is empty and no track is loaded
    assert!(
        !manager.has_next(),
        "RepeatOne: has_next() must be false when no track is loaded and the queue is empty; \
         there is nothing to repeat"
    );
}

// ===== Test 4: RepeatAll with single-track source queue =====

#[test]
fn test_has_next_repeat_all_single_track_queue() {
    // A single-track source queue with RepeatAll must always have a next track
    // because the one track loops indefinitely.
    let mut manager = PlaybackManager::default();
    manager.set_repeat(RepeatMode::All);

    manager.load_playlist(vec![make_track("only", 180)], 0);

    // Before playing
    assert!(
        manager.has_next(),
        "RepeatAll single-track: has_next() must be true before playing"
    );

    // After exhausting (next() drives past the only track)
    manager.play().ok();
    manager.next().ok(); // single track → history, RepeatAll reloads

    assert!(
        manager.has_next(),
        "RepeatAll single-track: has_next() must be true after the only track is consumed \
         because RepeatAll loops it"
    );
}

// ===== Test 5: RepeatOff with empty queue =====

#[test]
fn test_has_next_repeat_off_empty_queue() {
    // The baseline case: no repeat, no tracks → has_next() is false.
    let manager = PlaybackManager::default();

    assert_eq!(manager.get_repeat(), RepeatMode::Off);
    assert!(
        !manager.has_next(),
        "RepeatOff: has_next() must be false when the queue is empty"
    );
}

// ===== Test 6: Track consumption sequence for all three repeat modes =====

#[test]
fn test_has_next_consumption_sequence_repeat_off() {
    // RepeatOff: has_next() should flip from true to false as tracks are consumed.
    let mut manager = PlaybackManager::default();

    manager.add_to_queue_end(make_track("1", 180));
    manager.add_to_queue_end(make_track("2", 180));

    assert!(manager.has_next(), "RepeatOff: has_next true with 2 tracks");

    manager.next().ok(); // consume track 1

    assert!(
        manager.has_next(),
        "RepeatOff: has_next true with 1 track left"
    );

    manager.next().ok(); // consume track 2 — queue now empty

    assert!(
        !manager.has_next(),
        "RepeatOff: has_next must be false after all tracks are consumed"
    );
}

#[test]
fn test_has_next_consumption_sequence_repeat_all() {
    // RepeatAll: has_next() must remain true throughout, including after all source
    // tracks are consumed (source reloads from beginning).
    let mut manager = PlaybackManager::default();
    manager.set_repeat(RepeatMode::All);

    manager.load_playlist(vec![make_track("1", 180), make_track("2", 180)], 0);

    assert!(manager.has_next(), "RepeatAll: true before any next()");

    manager.play().ok();
    manager.next().ok(); // consume track 1

    assert!(manager.has_next(), "RepeatAll: true with 1 track left");

    manager.next().ok(); // consume track 2 → RepeatAll reloads source

    assert!(
        manager.has_next(),
        "RepeatAll: must remain true even after all source tracks are consumed \
         because the source reloads and cycles"
    );
}

#[test]
fn test_has_next_consumption_sequence_repeat_one() {
    // RepeatOne: has_next() stays true as long as a track is currently loaded;
    // consuming via next() under RepeatOne should restart the same track.
    let mut manager = PlaybackManager::default();
    manager.set_repeat(RepeatMode::One);

    let track = make_track("1", 180);
    manager.load_playlist(vec![track.clone()], 0);

    assert!(manager.has_next(), "RepeatOne: true when track in queue");

    // Activate source so the manager has a current_track()
    manager.play().ok();
    manager.activate_source(Box::new(MockAudioSource::new(180)), track);

    assert!(
        manager.has_next(),
        "RepeatOne: must remain true while current track is active"
    );
}

// ===== Test 7: "Play Next" queue items + RepeatAll on source queue =====

#[test]
fn test_has_next_play_next_queue_plus_repeat_all() {
    // When explicit "play next" user-queued tracks exist, has_next() returns true
    // because those tracks are in the immediate queue regardless of RepeatAll.
    // After those are consumed, RepeatAll on the source queue keeps has_next() true.
    let mut manager = PlaybackManager::default();
    manager.set_repeat(RepeatMode::All);

    // Source queue with 2 tracks (RepeatAll applies to this)
    manager.load_playlist(vec![make_track("src1", 180), make_track("src2", 180)], 0);

    // Add an explicit "play next" track (highest priority)
    manager.add_to_queue_next(make_track("next1", 180));

    // has_next must be true — explicit "play next" track is queued
    assert!(
        manager.has_next(),
        "has_next() must be true when a 'Play Next' track is queued"
    );

    // Consume the play-next track and one source track
    manager.play().ok(); // emits LoadNext for src1 (initial — skips play_next per logic)
    manager.next().ok(); // src1 → history, next1 now in play

    // Source still has src2, play_next consumed its item — but RepeatAll keeps has_next true
    assert!(
        manager.has_next(),
        "has_next() must be true: RepeatAll source still has src2"
    );

    // Exhaust source queue
    manager.next().ok(); // → src2
    manager.next().ok(); // → source exhausted → RepeatAll reloads

    assert!(
        manager.has_next(),
        "has_next() must remain true after source exhausted with RepeatAll active"
    );
}
