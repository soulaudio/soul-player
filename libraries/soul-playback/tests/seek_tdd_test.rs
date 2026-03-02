//! TDD tests for `seek_to()` and `seek_to_percent()` edge cases in PlaybackManager.
//!
//! Covers:
//! 1. seek_to() while Stopped — no source loaded
//! 2. seek_to() while loading (LoadNext emitted, activate_source not yet called)
//! 3. seek_to() while Paused (regression guard)
//! 4. seek_to() while Playing (regression guard)
//! 5. seek_to_percent() below 0.0 — clamp
//! 6. seek_to_percent() above 1.0 — clamp
//! 7. seek_to() beyond track duration — clamp to near-end
//! 8. get_position() reflects new position after seek
//! 9. 10 rapid seeks in a row — state remains valid

use soul_playback::{
    AudioSource, PlaybackError, PlaybackManager, PlaybackState, QueueTrack, TrackSource,
};
use std::path::PathBuf;
use std::time::Duration;

// ===== Test Helpers =====

/// Minimal mock audio source that supports seek.
///
/// Seek is permissive: clamps to [0, duration] rather than erroring, so that
/// the manager's own clamping logic is exercised separately from the source.
struct MockSource {
    duration: Duration,
    position: Duration,
    finished: bool,
}

impl MockSource {
    fn new(duration_secs: u64) -> Self {
        Self {
            duration: Duration::from_secs(duration_secs),
            position: Duration::ZERO,
            finished: false,
        }
    }
}

impl AudioSource for MockSource {
    fn read_samples(&mut self, buffer: &mut [f32]) -> soul_playback::Result<usize> {
        if self.finished {
            return Ok(0);
        }
        let samples_per_sec = 44100u64 * 2; // stereo
        let total = (self.duration.as_secs_f64() * samples_per_sec as f64) as u64;
        let current = (self.position.as_secs_f64() * samples_per_sec as f64) as u64;
        let remaining = (total.saturating_sub(current)) as usize;
        let to_read = remaining.min(buffer.len());
        if to_read == 0 {
            self.finished = true;
            return Ok(0);
        }
        for s in buffer.iter_mut().take(to_read) {
            *s = 0.0;
        }
        self.position += Duration::from_secs_f64(to_read as f64 / samples_per_sec as f64);
        Ok(to_read)
    }

    fn seek(&mut self, position: Duration) -> soul_playback::Result<()> {
        // Permissive seek — clamps rather than errors.
        // The manager's own clamping logic is tested separately.
        self.position = position.min(self.duration);
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

fn make_track(id: &str, duration_secs: u64) -> QueueTrack {
    QueueTrack {
        id: id.to_string(),
        path: PathBuf::from(format!("/music/{id}.mp3")),
        title: format!("Track {id}"),
        artist: "Test Artist".to_string(),
        album: Some("Test Album".to_string()),
        duration: Duration::from_secs(duration_secs),
        track_number: Some(1),
        source: TrackSource::Single,
    }
}

/// Returns a manager in Playing state with a 60-second track loaded.
fn manager_in_playing_state() -> PlaybackManager {
    let mut mgr = PlaybackManager::default();
    let track = make_track("1", 60);
    mgr.add_to_queue_end(track.clone());
    mgr.play().expect("play() should succeed");
    // activate_source transitions from Stopped/loading -> Playing
    mgr.activate_source(Box::new(MockSource::new(60)), track);
    assert_eq!(mgr.get_state(), PlaybackState::Playing);
    mgr
}

/// Returns a manager in Paused state with a 60-second track loaded.
fn manager_in_paused_state() -> PlaybackManager {
    let mut mgr = manager_in_playing_state();
    mgr.pause();
    // Pause uses a fade-out, so the state may still be Playing until the fade
    // completes. For seek tests we only need the source loaded; the seek_to()
    // contract is "works when not Stopped", so we verify seek succeeds.
    mgr
}

// ===== Tests =====

/// seek_to() on a brand-new, Stopped manager with no track loaded must return
/// Err(NoTrackLoaded) rather than panicking.
#[test]
fn test_seek_to_while_stopped_returns_error() {
    let mut mgr = PlaybackManager::default();
    assert_eq!(mgr.get_state(), PlaybackState::Stopped);

    let result = mgr.seek_to(Duration::from_secs(1));

    assert!(
        result.is_err(),
        "seek_to() on a Stopped manager with no source should return Err"
    );
    match result.unwrap_err() {
        PlaybackError::NoTrackLoaded => {} // expected
        other => panic!("Expected NoTrackLoaded, got: {:?}", other),
    }
}

/// seek_to() while loading (play() emitted LoadNext but activate_source() has
/// not been called yet) must fail gracefully — not panic.
///
/// During loading the state is Stopped and sources is Empty, so there is
/// nothing to seek; returning Err(NoTrackLoaded) is the correct, safe behaviour.
#[test]
fn test_seek_to_while_loading_is_graceful() {
    let mut mgr = PlaybackManager::default();
    let track = make_track("1", 60);
    mgr.add_to_queue_end(track);
    mgr.play().expect("play() should succeed");

    // State is now Stopped with loading=true (LoadNext event emitted,
    // waiting for the platform layer to call activate_source()).
    assert_eq!(
        mgr.get_state(),
        PlaybackState::Stopped,
        "Manager should be in Stopped/loading state before activate_source"
    );

    // Seek while loading: must not panic, must return Err.
    let result = mgr.seek_to(Duration::from_secs(5));
    assert!(
        result.is_err(),
        "seek_to() while loading (no source yet) should return Err, not panic"
    );
}

/// seek_to() while Paused must succeed — regression guard for known-good path.
///
/// Note: pause() uses an async fade-out; the state may still be Playing
/// immediately after pause() returns. We therefore accept Ok from seek_to()
/// in either Playing or Paused state as long as the source is loaded.
#[test]
fn test_seek_to_while_paused_succeeds() {
    let mut mgr = manager_in_paused_state();
    // Source is loaded; seek must succeed regardless of Playing/Paused state.
    let result = mgr.seek_to(Duration::from_secs(10));
    assert!(
        result.is_ok(),
        "seek_to() on a loaded track (paused or playing) should succeed, got: {:?}",
        result.err()
    );
}

/// seek_to() while Playing must succeed — regression guard for known-good path.
#[test]
fn test_seek_to_while_playing_succeeds() {
    let mut mgr = manager_in_playing_state();
    let result = mgr.seek_to(Duration::from_secs(30));
    assert!(
        result.is_ok(),
        "seek_to() while Playing should succeed, got: {:?}",
        result.err()
    );
}

/// seek_to_percent(-0.5) must be treated exactly as seek_to_percent(0.0).
///
/// A negative percent is clamped to 0.0, so the resulting position is 0s —
/// the beginning of the track.
#[test]
fn test_seek_to_percent_clamps_below_zero() {
    let mut mgr = manager_in_playing_state();

    // Seek to a known position first so we can detect the change.
    mgr.seek_to(Duration::from_secs(30))
        .expect("initial seek should succeed");

    // Now seek to a negative percent — should clamp to 0.0.
    let result = mgr.seek_to_percent(-0.5);
    assert!(
        result.is_ok(),
        "seek_to_percent(-0.5) should not error, got: {:?}",
        result.err()
    );

    let pos = mgr.get_position();
    assert_eq!(
        pos,
        Duration::ZERO,
        "After seek_to_percent(-0.5), position should be 0s (clamped to start), got: {:?}",
        pos
    );
}

/// seek_to_percent(1.5) must be treated as seek_to_percent(1.0) — near the end.
///
/// The manager clamps the percent to [0.0, 1.0] and then calls seek_to(duration),
/// which in turn clamps to (duration - 1ms) to avoid immediately triggering EOF.
#[test]
fn test_seek_to_percent_clamps_above_one() {
    let mut mgr = manager_in_playing_state();

    // Should not panic and should not error.
    let result = mgr.seek_to_percent(1.5);
    assert!(
        result.is_ok(),
        "seek_to_percent(1.5) should not panic or error, got: {:?}",
        result.err()
    );

    // Position must be near end (within 10ms of the 60s track), not beyond it.
    let pos = mgr.get_position();
    assert!(
        pos <= Duration::from_secs(60),
        "Position must not exceed track duration, got: {:?}",
        pos
    );
    assert!(
        pos >= Duration::from_millis(59_990), // 60s - 10ms tolerance
        "Position should be near end of 60s track after clamped seek, got: {:?}",
        pos
    );
}

/// seek_to(Duration::from_secs(999)) on a 60-second track must not panic and
/// must clamp the position to near the end of the track.
#[test]
fn test_seek_to_beyond_duration_clamps() {
    let mut mgr = manager_in_playing_state();

    let result = mgr.seek_to(Duration::from_secs(999));
    assert!(
        result.is_ok(),
        "seek_to() far beyond duration should clamp, not error. Got: {:?}",
        result.err()
    );

    let pos = mgr.get_position();
    assert!(
        pos <= Duration::from_secs(60),
        "Position must not exceed track duration (60s), got: {:?}",
        pos
    );
    assert!(
        pos > Duration::from_secs(59),
        "Position should be near end of 60s track after clamped seek, got: {:?}",
        pos
    );
}

/// After seek_to(5s), get_position() must return approximately 5 seconds.
#[test]
fn test_seek_then_position_reflects_new_position() {
    let mut mgr = manager_in_playing_state();

    mgr.seek_to(Duration::from_secs(5))
        .expect("seek should succeed");

    let pos = mgr.get_position();
    // Allow 10ms tolerance for floating-point arithmetic inside the source.
    let diff = if pos >= Duration::from_secs(5) {
        pos - Duration::from_secs(5)
    } else {
        Duration::from_secs(5) - pos
    };
    assert!(
        diff <= Duration::from_millis(10),
        "get_position() should be ~5s after seek_to(5s), got: {:?}",
        pos
    );
}

/// Ten rapid seeks to different positions must leave the player in a valid,
/// non-panicking state with a position that is consistent with the last seek.
#[test]
fn test_rapid_seeks_do_not_corrupt_state() {
    let mut mgr = manager_in_playing_state();

    let seek_positions_secs: [u64; 10] = [5, 10, 3, 50, 20, 59, 1, 40, 15, 30];

    for &secs in &seek_positions_secs {
        mgr.seek_to(Duration::from_secs(secs))
            .unwrap_or_else(|e| panic!("seek_to({}s) failed: {:?}", secs, e));
    }

    // After all seeks the manager must still report a sensible state.
    let state = mgr.get_state();
    assert!(
        state == PlaybackState::Playing || state == PlaybackState::Paused,
        "After rapid seeks, manager should still be Playing or Paused, got: {:?}",
        state
    );

    // Position must be within track duration.
    let pos = mgr.get_position();
    assert!(
        pos <= Duration::from_secs(60),
        "Position must not exceed track duration after rapid seeks, got: {:?}",
        pos
    );

    // The position should be close to the last seek target (30s).
    let last_target = Duration::from_secs(30);
    let diff = if pos >= last_target {
        pos - last_target
    } else {
        last_target - pos
    };
    assert!(
        diff <= Duration::from_millis(10),
        "Position should be ~30s after last seek, got: {:?}",
        pos
    );
}
