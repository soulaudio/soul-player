//! Tests for previous track navigation — edge cases and regressions
//!
//! These tests document and verify the correct contract for `previous()`:
//!
//! 1. When history is available and position < 3s:
//!    - Must emit `LoadNext(prev_track)` so the platform layer loads the track
//!    - Must emit `StateChanged(Stopped)` so the UI stops the progress timer
//!
//! 2. Second `previous()` with empty history and no source must be a graceful no-op.
//!
//! 3. After `previous()` leaves `loading=true`, calling `load_playlist + play` for
//!    a new album must emit `LoadNext` (the "new album doesn't start" regression).
//!
//! 4. When > 3s into a track, `previous()` must restart the current track — no LoadNext.
//!
//! 5. When < 3s into a track (with real audio source), `previous()` navigates backwards.

use soul_playback::{
    AudioSource, PlaybackEvent, PlaybackManager, PlaybackState, PlaybackStateEvent, QueueTrack,
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

/// Mock audio source — supports setting initial position for threshold tests.
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

    /// Create a source that is already `position_secs` into the track.
    fn at_secs(duration_secs: u64, position_secs: u64) -> Self {
        Self {
            duration: Duration::from_secs(duration_secs),
            position: Duration::from_secs(position_secs),
        }
    }
}

impl AudioSource for MockAudioSource {
    fn read_samples(&mut self, buffer: &mut [f32]) -> soul_playback::Result<usize> {
        let total = (self.duration.as_secs_f64() * 88200.0) as usize; // stereo 44100
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

// ===== Setup helper =====

/// Returns a manager that has been loaded with 3 tracks and advanced once via
/// `play() + next()`, simulating: "play album, skip one song".
///
/// State after setup:
/// - history = [track "1"]     ← track 1 was played and skipped
/// - pending  = Some(track "2") ← track 2 is the next-to-load
/// - state    = Stopped, loading = true
/// - Events cleared (drain called) — only events from the test action are visible
fn setup_advanced_one_track() -> PlaybackManager {
    let mut mgr = PlaybackManager::default();
    mgr.load_playlist(
        vec![
            make_track("1", 180),
            make_track("2", 180),
            make_track("3", 180),
        ],
        0,
    );
    // play() → LoadNext("1"), pending=t1
    mgr.play().ok();
    // next() → t1 pushed to history from pending slot, LoadNext("2"), pending=t2
    mgr.next().ok();
    // Clear accumulated setup events so tests start with a clean slate
    mgr.drain_events();
    mgr
}

// ===== Tests =====

/// CRITICAL: previous() must emit LoadNext so the platform can load the track.
///
/// Without this the audio is never loaded — the old FIXME code left loading=true
/// but never emitted the event, so the desktop layer never started loading.
#[test]
fn test_previous_emits_load_next_for_history_track() {
    let mut mgr = setup_advanced_one_track();

    mgr.previous().ok();

    let events = mgr.drain_events();
    let load_next = events.iter().find_map(|e| {
        if let PlaybackEvent::LoadNext(t) = e {
            Some(t)
        } else {
            None
        }
    });

    assert!(
        load_next.is_some(),
        "previous() must emit LoadNext — without it the platform never loads the track"
    );
    assert_eq!(
        load_next.unwrap().id,
        "1",
        "LoadNext must target track '1' (the history track)"
    );
}

/// CRITICAL: previous() must emit StateChanged(Stopped) so the UI stops the timer.
///
/// Without this the frontend never receives a state update, isPlaying stays true
/// in the Zustand store, and the progress bar keeps ticking even though audio
/// has stopped — exactly the bug the user reported.
#[test]
fn test_previous_emits_state_changed_stopped() {
    let mut mgr = setup_advanced_one_track();

    mgr.previous().ok();

    let events = mgr.drain_events();
    let stopped = events.iter().any(|e| {
        matches!(
            e,
            PlaybackEvent::StateChanged {
                state: PlaybackStateEvent::Stopped
            }
        )
    });

    assert!(
        stopped,
        "previous() must emit StateChanged(Stopped) so the UI knows playback is transitioning"
    );
}

/// State must be Stopped after previous() (loading the previous track).
#[test]
fn test_previous_state_is_stopped_after_history_navigation() {
    let mut mgr = setup_advanced_one_track();

    mgr.previous().ok();

    assert_eq!(
        mgr.get_state(),
        PlaybackState::Stopped,
        "state must be Stopped after previous() triggers a track load"
    );
}

/// Second previous() when history is empty and source is empty must be a
/// graceful no-op — no crash, no spurious LoadNext event.
#[test]
fn test_previous_twice_second_press_is_graceful_no_op() {
    let mut mgr = setup_advanced_one_track();

    mgr.previous().ok(); // first: pops history, emits LoadNext("1")
    mgr.drain_events();

    // Second press: history is now empty, sources is Empty
    let result = mgr.previous();

    assert!(result.is_ok(), "second previous() must not return an error");

    let events = mgr.drain_events();
    let has_load_next = events
        .iter()
        .any(|e| matches!(e, PlaybackEvent::LoadNext(_)));

    assert!(
        !has_load_next,
        "second previous() with empty history and no source must not emit LoadNext"
    );
}

/// State must remain Stopped after second previous() — not transition to a
/// broken intermediate state.
#[test]
fn test_previous_twice_state_remains_stopped() {
    let mut mgr = setup_advanced_one_track();

    mgr.previous().ok();
    mgr.drain_events();
    mgr.previous().ok();

    assert_eq!(
        mgr.get_state(),
        PlaybackState::Stopped,
        "state must remain Stopped after second previous() with empty history"
    );
}

/// REGRESSION: After previous() leaves loading=true, playing a new album via
/// load_playlist + play must successfully emit LoadNext.
///
/// Bug: play_queue (Tauri command) checks state==Stopped and skips stop().
/// previous() sets state=Stopped AND loading=true but never emits LoadNext.
/// Then play() sees loading=true and ignores the call → audio never starts.
/// User had to click a track in the queue to begin playback.
#[test]
fn test_play_queue_after_previous_emits_load_next_for_new_album() {
    let mut mgr = setup_advanced_one_track();

    // Simulate the broken state left by previous()
    mgr.previous().ok();
    mgr.drain_events();

    // Simulate play_queue for a new album: load_playlist then play
    mgr.load_playlist(vec![make_track("10", 240), make_track("11", 240)], 0);
    mgr.play().ok();

    let events = mgr.drain_events();
    let load_next = events.iter().find_map(|e| {
        if let PlaybackEvent::LoadNext(t) = e {
            Some(t)
        } else {
            None
        }
    });

    assert!(
        load_next.is_some(),
        "play_queue after previous() must emit LoadNext — loading state must be reset by load_playlist()"
    );
    assert_eq!(
        load_next.unwrap().id,
        "10",
        "LoadNext must target the first track of the new album"
    );
}

/// previous() with no history and no active source must be a graceful no-op.
/// It must not emit a duplicate LoadNext for the pending track.
#[test]
fn test_previous_at_start_with_no_history_no_source_is_graceful() {
    let mut mgr = PlaybackManager::default();
    mgr.load_playlist(vec![make_track("1", 180), make_track("2", 180)], 0);
    mgr.play().ok(); // LoadNext("1"), pending=t1, no activate_source yet
    mgr.drain_events();

    // No history, no active source — previous must be a no-op
    let result = mgr.previous();

    assert!(
        result.is_ok(),
        "previous() at start with no history must not error"
    );

    let events = mgr.drain_events();
    let has_load_next = events
        .iter()
        .any(|e| matches!(e, PlaybackEvent::LoadNext(_)));

    assert!(
        !has_load_next,
        "previous() with no history and no source must not emit spurious LoadNext"
    );
}

/// When > 3 seconds into a track, previous() must restart the current track,
/// NOT navigate backwards. No LoadNext event should be emitted.
#[test]
fn test_previous_beyond_3s_threshold_restarts_current_track() {
    let mut mgr = PlaybackManager::default();
    let t1 = make_track("1", 180);
    let t2 = make_track("2", 180);

    mgr.load_playlist(vec![t1.clone(), t2.clone()], 0);
    mgr.play().ok();
    mgr.activate_source(Box::new(MockAudioSource::new(180)), t1.clone());
    mgr.next().ok(); // t1 → history

    // Activate t2 at 10 seconds (well beyond the 3-second threshold)
    mgr.activate_source(Box::new(MockAudioSource::at_secs(180, 10)), t2.clone());
    mgr.drain_events();

    mgr.previous().ok();

    let events = mgr.drain_events();
    let has_load_next = events
        .iter()
        .any(|e| matches!(e, PlaybackEvent::LoadNext(_)));

    assert!(
        !has_load_next,
        "previous() at 10s must restart current track, not navigate to history (no LoadNext)"
    );

    assert_eq!(
        mgr.get_position(),
        Duration::ZERO,
        "track position must be reset to 0 after restart-via-previous()"
    );
}

/// When < 3 seconds into a track (position = 0), previous() must navigate
/// backwards to the history track and emit LoadNext.
#[test]
fn test_previous_within_3s_navigates_to_history_with_real_source() {
    let mut mgr = PlaybackManager::default();
    let t1 = make_track("1", 180);
    let t2 = make_track("2", 180);

    mgr.load_playlist(vec![t1.clone(), t2.clone()], 0);
    mgr.play().ok();
    mgr.activate_source(Box::new(MockAudioSource::new(180)), t1.clone());
    mgr.next().ok(); // t1 → history

    // Activate t2 at position 0 (< 3 seconds)
    mgr.activate_source(Box::new(MockAudioSource::new(180)), t2.clone());
    mgr.drain_events();

    mgr.previous().ok();

    let events = mgr.drain_events();
    let load_next = events.iter().find_map(|e| {
        if let PlaybackEvent::LoadNext(t) = e {
            Some(t)
        } else {
            None
        }
    });

    assert!(
        load_next.is_some(),
        "previous() at 0s must navigate backwards (LoadNext expected)"
    );
    assert_eq!(
        load_next.unwrap().id,
        "1",
        "must navigate to track '1' from history"
    );
}

/// Full user-reported scenario:
///   1. Play album
///   2. Skip one song (next)
///   3. Press previous → audio stops, but UI timer must also stop (StateChanged emitted)
///   4. Press previous again → graceful, state stays Stopped
///   5. Start a new album → playback must start (LoadNext emitted)
#[test]
fn test_full_scenario_play_skip_prev_prev_new_album() {
    let mut mgr = PlaybackManager::default();

    let tracks: Vec<_> = (1u32..=5)
        .map(|i| make_track(&i.to_string(), 180))
        .collect();
    mgr.load_playlist(tracks, 0);
    mgr.play().ok(); // LoadNext("1")
    mgr.next().ok(); // push "1" to history, LoadNext("2")
    mgr.drain_events();

    // --- First previous ---
    mgr.previous().ok();
    let events_1 = mgr.drain_events();

    assert!(
        events_1
            .iter()
            .any(|e| matches!(e, PlaybackEvent::LoadNext(_))),
        "first previous() must emit LoadNext"
    );
    assert!(
        events_1.iter().any(|e| matches!(
            e,
            PlaybackEvent::StateChanged {
                state: PlaybackStateEvent::Stopped
            }
        )),
        "first previous() must emit StateChanged(Stopped) — UI timer must stop"
    );
    assert_eq!(mgr.get_state(), PlaybackState::Stopped);

    // --- Second previous (history now empty) ---
    let result = mgr.previous();
    let events_2 = mgr.drain_events();
    assert!(result.is_ok(), "second previous() must not error");
    assert_eq!(
        mgr.get_state(),
        PlaybackState::Stopped,
        "state must remain Stopped after second previous()"
    );
    // Second press should not introduce a spurious LoadNext for a wrong track
    let second_load_next_ids: Vec<_> = events_2
        .iter()
        .filter_map(|e| {
            if let PlaybackEvent::LoadNext(t) = e {
                Some(t.id.clone())
            } else {
                None
            }
        })
        .collect();
    // If a LoadNext is emitted it must be for a track in the album, not some garbage value
    for id in &second_load_next_ids {
        let valid = ["1", "2", "3", "4", "5"].contains(&id.as_str());
        assert!(
            valid,
            "second previous() emitted LoadNext for unknown track id: {id}"
        );
    }

    // --- Play new album (the regression case: must not silently ignore play()) ---
    mgr.load_playlist(vec![make_track("99", 240)], 0);
    mgr.play().ok();
    let events_3 = mgr.drain_events();

    let new_album_load = events_3.iter().find_map(|e| {
        if let PlaybackEvent::LoadNext(t) = e {
            Some(t)
        } else {
            None
        }
    });

    assert!(
        new_album_load.is_some(),
        "play_queue for new album after previous() regression: LoadNext must be emitted"
    );
    assert_eq!(
        new_album_load.unwrap().id,
        "99",
        "LoadNext must be for the new album's first track"
    );
}
