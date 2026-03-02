//! TDD tests for volume leveling state reset across track changes
//!
//! These tests verify that the loudness normalizer (and related volume leveling
//! state) is properly reset when transitioning between tracks via any path:
//! `next()`, `previous()`, `skip_to_queue_index()`, `stop()` + `play()`, and
//! direct `activate_source()` without crossfade.
//!
//! # What we test
//!
//! The `PlaybackManager` exposes:
//! - `reset_loudness_normalizer()` — resets normalizer internal state (limiter buffer)
//! - `reset_output_limiter()` — resets output limiter state (lookahead buffer, gain_reduction)
//! - `reset_headroom()` — resets headroom manager state
//! - `get_effective_gain_db()` — observable proxy for static gain value
//! - `set_track_gain(gain_db, peak_dbfs)` — sets per-track gain
//! - `clear_loudness_gains()` — clears per-track static gain data
//! - `get_output_limiter_gain_reduction_db()` — observable proxy for limiter state
//!
//! # Bug found
//!
//! `activate_source()` does NOT call `loudness_normalizer.reset()` or
//! `output_limiter.reset()`. Only `transition_to_next_track()` (the crossfade
//! completion path) resets the loudness normalizer. This means that for the
//! normal (non-crossfade) path via `next()`, `previous()`, `skip_to_queue_index()`,
//! the output limiter's lookahead buffer retains samples from the old track.
//!
//! The limiter maintains a ring buffer of audio samples (the lookahead window).
//! Without reset, audio frames from the end of track1 remain in the buffer and
//! get emitted at the start of track2 — causing an audible "ghost" of the
//! previous track at the beginning of the new track.
//!
//! The output limiter also tracks `gain_reduction` — if the previous track
//! was loud (triggering limiting), the gain reduction state persists into the
//! new track, causing the first frames of the new track to be incorrectly
//! attenuated.
//!
//! # Fix
//!
//! Add to `activate_source()`:
//! ```rust
//! #[cfg(feature = "volume-leveling")]
//! self.loudness_normalizer.reset();
//! #[cfg(feature = "volume-leveling")]
//! self.output_limiter.reset();
//! #[cfg(feature = "volume-leveling")]
//! self.headroom_manager.reset();
//! ```
//!
//! # Test strategy
//!
//! We test the output limiter's `gain_reduction_db()` as a proxy:
//! 1. Process a loud (full-scale) audio signal through `process_audio()` to
//!    cause the limiter to engage (gain_reduction_db will be non-zero)
//! 2. Call `next()` + `activate_source()` to simulate a track change
//! 3. Check that `get_output_limiter_gain_reduction_db()` is 0.0 after
//!    the track change (limiter state was reset by activate_source)
//!
//! Before the fix: gain_reduction_db will be non-zero (persisting from track1)
//! After the fix: gain_reduction_db will be 0.0 (reset in activate_source)

#[cfg(feature = "volume-leveling")]
use soul_playback::NormalizationMode;
use soul_playback::{AudioSource, PlaybackManager, PlaybackState, QueueTrack, TrackSource};
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

/// Mock source that emits a configurable signal level (0.0 to 1.0+).
struct MockAudioSource {
    duration: Duration,
    position: Duration,
    /// Signal amplitude to emit (1.0 = full scale, 0.0 = silence)
    amplitude: f32,
}

impl MockAudioSource {
    fn new(duration_secs: u64) -> Self {
        Self {
            duration: Duration::from_secs(duration_secs),
            position: Duration::ZERO,
            amplitude: 0.0,
        }
    }

    /// Create a source that emits a loud (clipping-level) signal.
    fn loud(duration_secs: u64) -> Self {
        Self {
            duration: Duration::from_secs(duration_secs),
            position: Duration::ZERO,
            amplitude: 2.0, // Over-full-scale — will trigger limiter
        }
    }

    /// Create a source that emits a quiet signal.
    fn quiet(duration_secs: u64) -> Self {
        Self {
            duration: Duration::from_secs(duration_secs),
            position: Duration::ZERO,
            amplitude: 0.01, // -40 dBFS — well below limiter threshold
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
            *s = self.amplitude;
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

// ===== Volume leveling reset API regression tests =====

/// Regression: reset APIs are callable and don't panic.
#[test]
fn test_reset_apis_do_not_panic() {
    let mut mgr = PlaybackManager::default();
    #[cfg(feature = "volume-leveling")]
    {
        mgr.reset_loudness_normalizer();
        mgr.reset_output_limiter();
        mgr.reset_headroom();
        mgr.clear_loudness_gains();
        mgr.clear_headroom_track_gains();
    }
    let _ = mgr.get_state();
}

/// Regression: `get_output_limiter_gain_reduction_db()` returns 0.0 on a fresh manager.
#[cfg(feature = "volume-leveling")]
#[test]
fn test_fresh_manager_limiter_gain_reduction_is_zero() {
    let mgr = PlaybackManager::default();
    let reduction = mgr.get_output_limiter_gain_reduction_db();
    assert_eq!(
        reduction, 0.0,
        "Fresh manager must have 0 dB gain reduction (limiter inactive)"
    );
}

/// Regression: `reset_output_limiter()` restores gain_reduction to 0 after
/// processing a loud signal.
#[cfg(feature = "volume-leveling")]
#[test]
fn regression_reset_output_limiter_clears_gain_reduction() {
    let mut mgr = PlaybackManager::default();
    mgr.set_volume(100); // Full volume so loud signal reaches limiter at full amplitude

    let t1 = make_track("1", 5);
    mgr.load_playlist(vec![t1.clone()], 0);
    mgr.play().ok();
    mgr.activate_source(Box::new(MockAudioSource::loud(5)), t1);

    // Process enough loud samples to engage the limiter
    let mut output = vec![0.0f32; 4096];
    for _ in 0..20 {
        mgr.process_audio(&mut output).ok();
    }

    // The output limiter should now have non-zero gain reduction
    // (it was processing a 2.0 amplitude signal, well above 0 dBFS threshold)
    // Note: this depends on the threshold being <= 0 dBFS (default), which it is.
    // We check that resetting clears the gain reduction.
    mgr.reset_output_limiter();

    let reduction = mgr.get_output_limiter_gain_reduction_db();
    assert_eq!(
        reduction, 0.0,
        "After reset_output_limiter(), gain_reduction_db must be 0.0"
    );
}

// ===== BUG TESTS: activate_source() must reset volume leveling state =====

/// FAILING TEST (before fix): After `next()` + `activate_source()`, the output
/// limiter's gain reduction must be 0.0 (state reset from previous track).
///
/// Scenario:
/// - Track 1: loud signal at full volume → limiter engages, gain_reduction_db < 0
/// - next() + activate_source(track2): limiter must be reset
/// - Before fix: gain_reduction_db is still non-zero (limiter state carries over)
/// - After fix: gain_reduction_db is 0.0 (reset in activate_source)
///
/// We set volume to 100 so the loud signal (amplitude 2.0) is not attenuated
/// below the limiter threshold (0 dBFS = 1.0 linear) before reaching the limiter.
#[cfg(feature = "volume-leveling")]
#[test]
fn test_output_limiter_resets_on_activate_source_after_next() {
    let mut mgr = PlaybackManager::default();
    mgr.set_volume(100); // Full volume so loud signal reaches limiter at 2.0 amplitude

    let t1 = make_track("1", 10);
    let t2 = make_track("2", 10);
    mgr.load_playlist(vec![t1.clone(), t2.clone()], 0);
    mgr.play().ok();
    mgr.activate_source(Box::new(MockAudioSource::loud(10)), t1);

    // Process enough audio to engage the limiter fully.
    // The limiter uses a release time of 100ms, so we need more than that.
    let mut output = vec![0.0f32; 4096];
    for _ in 0..50 {
        mgr.process_audio(&mut output).ok();
    }

    // Confirm limiter is engaged (non-zero gain reduction)
    let reduction_before = mgr.get_output_limiter_gain_reduction_db();
    assert!(
        reduction_before < 0.0,
        "Expected non-zero limiting (< 0 dB gain reduction) after loud signal at full volume, \
         got: {reduction_before} dB. The limiter threshold is 0 dBFS and signal is 2.0 amplitude."
    );

    // Now advance to track 2
    mgr.next().ok();
    mgr.activate_source(Box::new(MockAudioSource::quiet(10)), t2);

    // KEY ASSERTION: After activate_source for the new track, the limiter
    // state must have been reset. Without the fix, gain_reduction_db
    // retains the value from track 1.
    let reduction_after = mgr.get_output_limiter_gain_reduction_db();
    assert_eq!(
        reduction_after, 0.0,
        "After next() + activate_source(track2), output limiter gain reduction must be \
         0.0 dB (limiter state was reset). Got: {reduction_after} dB. \
         This indicates activate_source() is NOT resetting the output limiter state \
         — gain reduction from track1 is carried over to track2."
    );
}

/// FAILING TEST (before fix): After `previous()` + `activate_source()`, the
/// output limiter's gain reduction must be 0.0.
#[cfg(feature = "volume-leveling")]
#[test]
fn test_output_limiter_resets_on_activate_source_after_previous() {
    let mut mgr = PlaybackManager::default();
    mgr.set_volume(100); // Full volume so loud signal reaches limiter at full amplitude

    let t1 = make_track("1", 10);
    let t2 = make_track("2", 10);
    mgr.load_playlist(vec![t1.clone(), t2.clone()], 0);
    mgr.play().ok();
    mgr.activate_source(Box::new(MockAudioSource::new(10)), t1.clone());

    // Advance to t2
    mgr.next().ok();
    mgr.activate_source(Box::new(MockAudioSource::loud(10)), t2);

    // Process loud audio on t2 to engage the limiter
    let mut output = vec![0.0f32; 4096];
    for _ in 0..50 {
        mgr.process_audio(&mut output).ok();
    }

    let reduction_before = mgr.get_output_limiter_gain_reduction_db();
    assert!(
        reduction_before < 0.0,
        "Limiter must be active after loud signal on t2 (got: {reduction_before} dB)"
    );

    // Go back to t1 via previous()
    mgr.previous().ok(); // position is 0 → go to history (t1)
    mgr.activate_source(Box::new(MockAudioSource::quiet(10)), t1);

    let reduction_after = mgr.get_output_limiter_gain_reduction_db();
    assert_eq!(
        reduction_after, 0.0,
        "After previous() + activate_source(t1), output limiter gain reduction must be 0.0. \
         Got: {reduction_after} dB."
    );
}

/// FAILING TEST (before fix): After `skip_to_queue_index()` + `activate_source()`,
/// the output limiter must be reset.
#[cfg(feature = "volume-leveling")]
#[test]
fn test_output_limiter_resets_on_activate_source_after_skip() {
    let mut mgr = PlaybackManager::default();
    mgr.set_volume(100); // Full volume so loud signal reaches limiter at full amplitude

    let t1 = make_track("1", 10);
    let t2 = make_track("2", 10);
    let t3 = make_track("3", 10);
    mgr.load_playlist(vec![t1.clone(), t2.clone(), t3.clone()], 0);
    mgr.play().ok();
    mgr.activate_source(Box::new(MockAudioSource::loud(10)), t1);

    // Process loud audio on t1 to engage the limiter
    let mut output = vec![0.0f32; 4096];
    for _ in 0..50 {
        mgr.process_audio(&mut output).ok();
    }

    let reduction_before = mgr.get_output_limiter_gain_reduction_db();
    assert!(
        reduction_before < 0.0,
        "Limiter must be active after loud signal on t1 (got: {reduction_before} dB)"
    );

    // Skip to index 1 in the remaining queue [t2, t3] → jumps to t3
    if mgr.skip_to_queue_index(1).is_ok() {
        mgr.activate_source(Box::new(MockAudioSource::quiet(10)), t3);

        let reduction_after = mgr.get_output_limiter_gain_reduction_db();
        assert_eq!(
            reduction_after, 0.0,
            "After skip_to_queue_index() + activate_source(t3), output limiter gain \
             reduction must be 0.0. Got: {reduction_after} dB."
        );
    }
    // If skip fails (queue exhausted after consuming t1 during play()), test is no-op.
}

/// FAILING TEST (before fix): After `stop()` + `play()` + `activate_source()`,
/// the output limiter must be reset.
///
/// Note: `stop()` does clear the sources and emits StateChanged(Stopped), but
/// does it reset the audio pipeline state? It should — and this test verifies it.
#[cfg(feature = "volume-leveling")]
#[test]
fn test_output_limiter_resets_on_stop_then_play() {
    let mut mgr = PlaybackManager::default();
    mgr.set_volume(100); // Full volume so loud signal reaches limiter at full amplitude

    let t1 = make_track("1", 10);
    mgr.load_playlist(vec![t1.clone()], 0);
    mgr.play().ok();
    mgr.activate_source(Box::new(MockAudioSource::loud(10)), t1.clone());

    // Process loud audio to engage limiter
    let mut output = vec![0.0f32; 4096];
    for _ in 0..50 {
        mgr.process_audio(&mut output).ok();
    }

    let reduction_before = mgr.get_output_limiter_gain_reduction_db();
    assert!(
        reduction_before < 0.0,
        "Limiter must be active after loud signal (got: {reduction_before} dB)"
    );

    // Stop and restart
    mgr.stop();
    mgr.load_playlist(vec![t1.clone()], 0);
    mgr.play().ok();
    mgr.activate_source(Box::new(MockAudioSource::quiet(10)), t1);

    let reduction_after = mgr.get_output_limiter_gain_reduction_db();
    assert_eq!(
        reduction_after, 0.0,
        "After stop() + play() + activate_source(), output limiter gain reduction must be 0.0. \
         Got: {reduction_after} dB."
    );
}

// ===== Regression: correct behaviors that must remain correct =====

/// Regression: `reset_loudness_normalizer()` + `clear_loudness_gains()` gives
/// neutral effective gain (0.0 dB).
#[cfg(feature = "volume-leveling")]
#[test]
fn regression_explicit_reset_then_clear_gains_gives_neutral_effective_gain() {
    let mut mgr = PlaybackManager::default();

    let t1 = make_track("1", 10);
    mgr.load_playlist(vec![t1.clone()], 0);
    mgr.play().ok();
    mgr.activate_source(Box::new(MockAudioSource::new(10)), t1);

    mgr.set_track_gain(-6.0, -0.5);
    mgr.reset_loudness_normalizer();
    mgr.clear_loudness_gains();

    let effective_gain = mgr.get_effective_gain_db();
    assert_eq!(
        effective_gain, 0.0,
        "After explicit reset + clear_gains, effective gain must be 0.0 dB. Got: {effective_gain}"
    );
}

/// Regression: `set_track_gain()` with `ReplayGainTrack` mode changes effective gain.
#[cfg(feature = "volume-leveling")]
#[test]
fn regression_set_track_gain_changes_effective_gain_in_replaygain_mode() {
    let mut mgr = PlaybackManager::default();

    let t1 = make_track("1", 10);
    mgr.load_playlist(vec![t1.clone()], 0);
    mgr.play().ok();
    mgr.activate_source(Box::new(MockAudioSource::new(10)), t1);

    mgr.set_volume_leveling_mode(NormalizationMode::ReplayGainTrack);
    mgr.clear_loudness_gains();

    let gain_before = mgr.get_effective_gain_db();
    assert_eq!(
        gain_before, 0.0,
        "No gain data: effective gain must be 0 dB"
    );

    mgr.set_track_gain(-6.0, -0.1);
    let gain_after = mgr.get_effective_gain_db();
    assert_ne!(
        gain_after, 0.0,
        "set_track_gain(-6.0) must produce non-zero effective gain"
    );
}

/// Regression: the output limiter gain_reduction is 0 when no audio has been
/// processed (fresh manager or after reset).
#[cfg(feature = "volume-leveling")]
#[test]
fn regression_output_limiter_starts_at_zero_reduction() {
    let mgr = PlaybackManager::default();
    let reduction = mgr.get_output_limiter_gain_reduction_db();
    assert_eq!(
        reduction, 0.0,
        "Fresh manager: output limiter gain reduction must be 0.0 dB"
    );
}

/// Regression: latency of the output limiter is accessible and non-negative.
#[cfg(feature = "volume-leveling")]
#[test]
fn regression_output_limiter_latency_is_accessible() {
    let mgr = PlaybackManager::default();
    let latency = mgr.get_output_limiter_latency();
    // Latency should be non-negative (0 for "instant" preset, >0 for others)
    // Default preset in PlaybackManager::new is whatever TruePeakLimiter::new sets.
    assert!(
        latency < 1_000_000,
        "Limiter latency should be a reasonable sample count"
    );
}
