//! Regression and TDD tests for PlaybackManager
//!
//! Two groups of tests:
//!
//! ## Group 1: Regression — add_playlist_to_queue() state reset
//!
//! The bug was that `add_playlist_to_queue()` did not reset `loading`,
//! `pending_load_track`, or `sources`. The fix added those three resets.
//! These tests lock in the fix so the bug can never silently re-appear.
//!
//! ## Group 2: TDD — activate_source() state-machine contracts
//!
//! An audit found potential edge cases in `activate_source()`:
//! - Might emit StateChanged(Playing) without a prior play() call
//! - Might emit StateChanged(Playing) twice if called twice
//! - Might auto-play after stop() or pause() during loading
//!
//! Each test pins exactly one behaviour contract.

use soul_playback::{
    AudioSource, PlaybackEvent, PlaybackManager, PlaybackState, PlaybackStateEvent, QueueTrack,
    TrackSource,
};
use std::path::PathBuf;
use std::time::Duration;

// ===== Shared test helpers =====

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

/// Minimal mock audio source. Returns silent samples until `duration` is reached.
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
        // Stereo 44100 Hz
        let total = (self.duration.as_secs_f64() * 88_200.0) as usize;
        let current = (self.position.as_secs_f64() * 88_200.0) as usize;
        let to_read = (total.saturating_sub(current)).min(buffer.len());
        if to_read == 0 {
            return Ok(0);
        }
        for s in buffer.iter_mut().take(to_read) {
            *s = 0.0;
        }
        self.position += Duration::from_secs_f64(to_read as f64 / 88_200.0);
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

// ===== Event helpers =====

fn has_state_playing(events: &[PlaybackEvent]) -> bool {
    events.iter().any(|e| {
        matches!(
            e,
            PlaybackEvent::StateChanged {
                state: PlaybackStateEvent::Playing
            }
        )
    })
}

fn count_state_playing(events: &[PlaybackEvent]) -> usize {
    events
        .iter()
        .filter(|e| {
            matches!(
                e,
                PlaybackEvent::StateChanged {
                    state: PlaybackStateEvent::Playing
                }
            )
        })
        .count()
}

fn has_load_next(events: &[PlaybackEvent]) -> bool {
    events
        .iter()
        .any(|e| matches!(e, PlaybackEvent::LoadNext(_)))
}

// =============================================================================
// GROUP 1 — Regression tests for add_playlist_to_queue() state reset
// =============================================================================

/// Regression test 1: add_playlist_to_queue() must clear loading=true
/// so that a subsequent play() emits LoadNext instead of silently ignoring it.
///
/// Scenario that caused the original bug:
///   1. Start playback → play_next_in_queue() fires → loading=true
///   2. add_playlist_to_queue(new_tracks) is called
///   3. play() — with the bug: silently ignored because loading=true
///              — with the fix: emits LoadNext because loading was cleared to false
#[test]
fn test_add_playlist_to_queue_clears_loading_so_play_works() {
    let mut mgr = PlaybackManager::default();

    // Step 1: trigger loading=true via play_next_in_queue
    mgr.add_playlist_to_queue(vec![make_track("1", 180), make_track("2", 180)]);
    mgr.play().ok(); // loading=true, state=Stopped, emits LoadNext("1")

    // Confirm we are in the loading state (drain events to clear them)
    mgr.drain_events();

    // Step 2: replace queue while loading is still in flight
    mgr.add_playlist_to_queue(vec![make_track("a", 240), make_track("b", 240)]);

    // Step 3: play() — must emit LoadNext for the new queue, NOT be silently ignored
    mgr.play().ok();

    let events = mgr.drain_events();

    assert!(
        has_load_next(&events),
        "play() after add_playlist_to_queue() must emit LoadNext; \
         bug was that loading=true caused play() to silently do nothing"
    );
}

/// Regression test 2: add_playlist_to_queue() must clear pending_load_track.
///
/// If a LoadNext was dispatched (pending_load_track = Some(track1)) and
/// add_playlist_to_queue() is then called, the pending track from the old
/// playback session must be discarded. Otherwise it could be written to
/// history (via play_next_in_queue's pending.take() branch) and confuse
/// navigation in the new session.
///
/// We verify this indirectly: after add_playlist_to_queue + play(), the new
/// LoadNext must be for a track from the new playlist, not the stale pending track.
#[test]
fn test_add_playlist_to_queue_clears_pending_track() {
    let mut mgr = PlaybackManager::default();

    // Start loading track "old-1"
    mgr.add_playlist_to_queue(vec![make_track("old-1", 180), make_track("old-2", 180)]);
    mgr.play().ok(); // pending_load_track = Some("old-1"), loading=true

    // Drain to clear old events
    mgr.drain_events();

    // Replace with a completely different playlist
    mgr.add_playlist_to_queue(vec![make_track("new-1", 240), make_track("new-2", 240)]);

    // Now play() should load "new-1", not "old-1"
    mgr.play().ok();

    let events = mgr.drain_events();

    let load_next_ids: Vec<String> = events
        .iter()
        .filter_map(|e| {
            if let PlaybackEvent::LoadNext(t) = e {
                Some(t.id.clone())
            } else {
                None
            }
        })
        .collect();

    assert!(
        load_next_ids.iter().any(|id| id == "new-1"),
        "LoadNext after add_playlist_to_queue must be for the new playlist's first track ('new-1'), \
         but got: {:?}",
        load_next_ids
    );

    assert!(
        !load_next_ids.iter().any(|id| id == "old-1"),
        "LoadNext must NOT reference the stale pending track ('old-1'), got: {:?}",
        load_next_ids
    );
}

/// Regression test 3: add_playlist_to_queue() must clear sources (SourceState::Empty).
///
/// If there was an active audio source when add_playlist_to_queue() is called
/// (e.g., user clicks a new album while a track is playing), the old source
/// must be dropped. This prevents:
/// - Crossfade into a source from a different album
/// - "previous track ID" leaking from the old session into TrackChanged events
///
/// We verify this by checking the state after add_playlist_to_queue: when
/// activate_source() is called for the new track, the TrackChanged event must
/// report `previous_track_id = None` (no previous source was active).
#[test]
fn test_add_playlist_to_queue_clears_sources() {
    let mut mgr = PlaybackManager::default();

    // Fully activate a source from the first playlist
    let t1 = make_track("first-album-track", 180);
    mgr.add_playlist_to_queue(vec![t1.clone()]);
    mgr.play().ok();
    mgr.activate_source(Box::new(MockAudioSource::new(180)), t1);

    // Confirm we are in Playing state with an active source
    assert_eq!(mgr.get_state(), PlaybackState::Playing);

    // Drain all setup events
    mgr.drain_events();

    // Load a completely new playlist (simulates user clicking a new album)
    mgr.add_playlist_to_queue(vec![make_track("second-album-track", 240)]);

    // At this point sources must be cleared (SourceState::Empty).
    // We verify by calling activate_source for the new track and checking
    // that the TrackChanged event reports no previous track.
    mgr.play().ok();
    mgr.activate_source(
        Box::new(MockAudioSource::new(240)),
        make_track("second-album-track", 240),
    );

    let events = mgr.drain_events();

    // Find the TrackChanged event and check previous_track_id is None
    let track_changed = events.iter().find_map(|e| {
        if let PlaybackEvent::TrackChanged {
            track_id,
            previous_track_id,
        } = e
        {
            Some((track_id.clone(), previous_track_id.clone()))
        } else {
            None
        }
    });

    assert!(
        track_changed.is_some(),
        "activate_source() must emit a TrackChanged event"
    );

    let (new_id, prev_id) = track_changed.unwrap();
    assert_eq!(
        new_id, "second-album-track",
        "TrackChanged must be for the new track"
    );
    assert_eq!(
        prev_id, None,
        "previous_track_id must be None because add_playlist_to_queue cleared sources; \
         got: {:?}",
        prev_id
    );
}

// =============================================================================
// GROUP 2 — TDD tests for activate_source() state-machine contracts
// =============================================================================

/// Test 4: activate_source() called WITHOUT a prior play() must NOT emit
/// StateChanged(Playing).
///
/// The contract: if `loading` was never set to true, activate_source() is
/// being called speculatively (e.g., a background preload). Auto-starting
/// playback in that case would be unexpected behaviour.
///
/// In activate_source(): `was_loading = self.loading` which is false here,
/// so the `if was_loading && state==Stopped` branch is skipped.
#[test]
fn test_activate_source_when_not_loading_does_not_emit_state_changed_playing() {
    let mut mgr = PlaybackManager::default();
    let t1 = make_track("1", 180);

    // Load queue but do NOT call play() — loading stays false
    mgr.add_playlist_to_queue(vec![t1.clone()]);

    mgr.drain_events();

    // Call activate_source directly without a prior play()
    mgr.activate_source(Box::new(MockAudioSource::new(180)), t1);

    let events = mgr.drain_events();

    assert!(
        !has_state_playing(&events),
        "activate_source() without a prior play() must NOT emit StateChanged(Playing); \
         auto-play without user intent is incorrect"
    );

    // State must remain Stopped (no play was requested)
    assert_eq!(
        mgr.get_state(),
        PlaybackState::Stopped,
        "state must remain Stopped when activate_source() is called without play()"
    );
}

/// Test 5: activate_source() after play() must emit exactly one StateChanged(Playing).
///
/// Normal happy-path: play() sets loading=true, state=Stopped → activate_source()
/// detects was_loading=true && state==Stopped → transitions to Playing.
#[test]
fn test_activate_source_when_loading_emits_state_changed_playing() {
    let mut mgr = PlaybackManager::default();
    let t1 = make_track("1", 180);

    mgr.add_playlist_to_queue(vec![t1.clone()]);
    mgr.play().ok(); // loading=true, state=Stopped

    mgr.drain_events(); // clear LoadNext

    // Platform finishes loading and calls activate_source
    mgr.activate_source(Box::new(MockAudioSource::new(180)), t1);

    let events = mgr.drain_events();

    assert!(
        has_state_playing(&events),
        "activate_source() after play() must emit StateChanged(Playing)"
    );

    assert_eq!(
        mgr.get_state(),
        PlaybackState::Playing,
        "manager state must be Playing after normal activate_source() flow"
    );
}

/// Test 6: activate_source() after stop() must NOT auto-play.
///
/// Flow: play() → loading=true → stop() → loading=false → activate_source()
///
/// stop() resets `loading=false`. When activate_source() is called, `was_loading`
/// will be false, so the transition to Playing must be skipped.
///
/// This models a race: platform starts loading, user presses stop, loading
/// finishes anyway — platform should be able to call activate_source safely
/// without accidentally restarting playback.
#[test]
fn test_activate_source_after_stop_does_not_auto_play() {
    let mut mgr = PlaybackManager::default();
    let t1 = make_track("1", 180);

    mgr.add_playlist_to_queue(vec![t1.clone()]);
    mgr.play().ok(); // loading=true

    // User presses stop before the source arrives
    mgr.stop(); // loading=false, state=Stopped

    mgr.drain_events(); // clear stop events

    // Platform loading finishes — calls activate_source despite stop
    mgr.activate_source(Box::new(MockAudioSource::new(180)), t1);

    let events = mgr.drain_events();

    assert!(
        !has_state_playing(&events),
        "activate_source() after stop() must NOT emit StateChanged(Playing); \
         stop() clears loading=false so auto-play must be suppressed"
    );

    assert_eq!(
        mgr.get_state(),
        PlaybackState::Stopped,
        "state must remain Stopped after activate_source() when stop() was called first"
    );
}

/// Test 7: activate_source() after pause() during loading must NOT auto-play.
///
/// Flow: play() → loading=true, state=Stopped →
///       pause() → state=Paused (loading still true) →
///       activate_source() → was_loading=true BUT state==Paused, not Stopped
///
/// The condition in activate_source() is:
///   if was_loading && self.state == PlaybackState::Stopped { ... }
/// Because state is Paused (not Stopped), the branch is skipped — correct.
#[test]
fn test_activate_source_after_pause_during_loading_does_not_auto_play() {
    let mut mgr = PlaybackManager::default();
    let t1 = make_track("1", 180);

    mgr.add_playlist_to_queue(vec![t1.clone()]);
    mgr.play().ok(); // loading=true, state=Stopped

    // User pauses before source arrives
    mgr.pause(); // state → Paused, loading still true

    assert_eq!(
        mgr.get_state(),
        PlaybackState::Paused,
        "pause() during loading must set state to Paused"
    );

    mgr.drain_events();

    // Platform finishes loading; calls activate_source
    mgr.activate_source(Box::new(MockAudioSource::new(180)), t1);

    let events = mgr.drain_events();

    assert!(
        !has_state_playing(&events),
        "activate_source() after pause-during-loading must NOT emit StateChanged(Playing); \
         user intent was to pause before the track arrived"
    );

    assert_eq!(
        mgr.get_state(),
        PlaybackState::Paused,
        "state must remain Paused after activate_source() when user paused during loading"
    );
}

/// Test 8: Calling activate_source() twice must not emit StateChanged(Playing) twice.
///
/// The first call clears `loading=false`. The second call sees `was_loading=false`
/// so the Playing transition is skipped — no duplicate event.
///
/// This protects against a race where the platform layer calls activate_source
/// twice (e.g., a bug in the loading machinery that calls back twice for the
/// same track). The manager must be idempotent.
#[test]
fn test_double_activate_source_no_duplicate_state_events() {
    let mut mgr = PlaybackManager::default();
    let t1 = make_track("1", 180);
    let t2 = make_track("1-dup", 180); // same logical track, second activation

    mgr.add_playlist_to_queue(vec![t1.clone()]);
    mgr.play().ok(); // loading=true

    mgr.drain_events();

    // First activation — should emit Playing
    mgr.activate_source(Box::new(MockAudioSource::new(180)), t1);

    let events_first = mgr.drain_events();
    let playing_count_first = count_state_playing(&events_first);

    assert_eq!(
        playing_count_first, 1,
        "first activate_source() after play() must emit exactly one StateChanged(Playing)"
    );

    // Second activation — must NOT emit another StateChanged(Playing)
    mgr.activate_source(Box::new(MockAudioSource::new(180)), t2);

    let events_second = mgr.drain_events();
    let playing_count_second = count_state_playing(&events_second);

    assert_eq!(
        playing_count_second, 0,
        "second activate_source() must NOT emit StateChanged(Playing) again; \
         loading was already cleared by the first call, so was_loading=false"
    );
}

/// Test 9: Stale activation from a background loader from a prior play() cycle is rejected.
///
/// Scenario (mirrors the E2E test race that caused queue-operations test 4 to fail):
///   1. play() with track T1 → loading=true, pending=Some(T1)
///   2. stop() → loading=false, pending=None
///   3. play() with track T2 → loading=true, pending=Some(T2)
///   4. Stale ActivateSource(T1) arrives (background thread from step 1 finishes late)
///   5. T1 must be rejected — T2 must still be pending (not replaced by T1)
///   6. Real ActivateSource(T2) arrives → accepted, state transitions to Playing
#[test]
fn test_stale_activate_source_from_prior_play_cycle_is_rejected() {
    let mut mgr = PlaybackManager::default();
    let t1 = make_track("t1", 180);
    let t2 = make_track("t2", 180);

    // First play cycle: load T1
    mgr.add_playlist_to_queue(vec![t1.clone()]);
    mgr.play().ok(); // pending=Some(T1), loading=true

    // User stops before T1 arrives
    mgr.stop(); // loading=false, pending=None
    mgr.drain_events();

    // Second play cycle: new queue with T2
    mgr.add_playlist_to_queue(vec![t2.clone()]);
    mgr.play().ok(); // pending=Some(T2), loading=true
    mgr.drain_events();

    // Stale T1 arrives (background thread from first cycle finishes late)
    let accepted = mgr.activate_source(Box::new(MockAudioSource::new(180)), t1);
    assert!(
        !accepted,
        "stale T1 activation must be rejected (pending is T2)"
    );

    // State must still be Stopped (T1 was not allowed to start Playing)
    assert_eq!(
        mgr.get_state(),
        PlaybackState::Stopped,
        "state must remain Stopped after stale T1 activation is rejected"
    );

    // Stale activation must not emit any events
    let stale_events = mgr.drain_events();
    assert!(
        stale_events.is_empty(),
        "stale activation must not emit any events; got: {:?}",
        stale_events
    );

    // Real T2 arrives — must be accepted and start Playing
    let accepted = mgr.activate_source(Box::new(MockAudioSource::new(180)), t2);
    assert!(accepted, "real T2 activation must be accepted");

    assert_eq!(
        mgr.get_state(),
        PlaybackState::Playing,
        "state must be Playing after real T2 activation"
    );

    let events = mgr.drain_events();
    assert!(
        has_state_playing(&events),
        "real T2 activation must emit StateChanged(Playing)"
    );
}
