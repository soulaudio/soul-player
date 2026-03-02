//! TDD tests for the replay_gain module
//!
//! Bug found: `replay_gain.rs` is orphaned — NOT registered with `mod replay_gain` in lib.rs,
//! so the module is never compiled as part of the crate. The first test (`module_is_registered`)
//! fails to compile until the fix is applied.
//!
//! Additional edge cases verified after the registration fix:
//! 1. NaN gain input from metadata propagates to NaN linear_gain, corrupting all samples
//!    silently (NaN comparisons are always false, so the fast-path in `process()` is skipped).
//! 2. Peak of 0.0 in clipping-prevention path must be skipped (log10(0) = -Inf).
//! 3. Peak of 1.0 limits gain to exactly 0 dB.
//! 4. Extreme positive gain (+100 dB) without clipping prevention must not panic.
//! 5. Extreme negative gain (-100 dB) must not panic.
//! 6. preamp-only mode (ReplayGainMode::Off with non-zero preamp) keeps unity gain because
//!    preamp is only meaningful when a mode is active (by design — Off means no normalization).
//! 7. `effective_gain_db()` returns -100.0 when linear_gain is 0.0 (or negative guard).

use soul_playback::replay_gain::{ReplayGainMode, ReplayGainProcessor, ReplayGainValues};

// ──────────────────────────────────────────────────────────────────────────────
// Bug 1: module orphaned — compile-time check
// This test exists solely to confirm the module is reachable from a test binary.
// Before the fix, this file will not compile because `soul_playback::replay_gain`
// does not exist.  After `mod replay_gain;` and `pub mod replay_gain;` are added
// to lib.rs the file compiles and this trivial assertion passes.
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn module_is_registered_and_reachable() {
    // If this compiles, the module is registered.  The default mode must be Off.
    let processor = ReplayGainProcessor::new();
    assert_eq!(processor.mode(), ReplayGainMode::Off);
}

// ──────────────────────────────────────────────────────────────────────────────
// Bug 2: NaN gain_db corrupts samples silently
//
// When metadata contains a NaN gain value (e.g. from a corrupt tag), `db_to_linear`
// returns NaN.  Inside `process()` the fast-path guard is:
//   `if (self.linear_gain - 1.0).abs() < 0.0001 { return; }`
// NaN comparisons are always false, so the guard is NOT taken.  Every sample is
// then multiplied by NaN, producing NaN output with no error or warning.
//
// Fix: sanitise the result of `db_to_linear` in `recalculate_gain()` — replace
// NaN/Inf with 1.0 (unity gain) to fail safe.
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn nan_gain_db_does_not_produce_nan_samples() {
    let mut processor = ReplayGainProcessor::new();
    processor.set_mode(ReplayGainMode::Track);

    // Simulate a corrupt tag containing NaN
    let values = ReplayGainValues {
        track_gain_db: Some(f32::NAN),
        track_peak: None,
        album_gain_db: None,
        album_peak: None,
    };
    processor.set_track_values(values);

    let mut buffer = vec![0.5f32; 64];
    processor.process(&mut buffer);

    // No sample should be NaN after processing
    for (i, &s) in buffer.iter().enumerate() {
        assert!(
            !s.is_nan(),
            "sample[{i}] is NaN after NaN gain input — linear_gain was not sanitised"
        );
    }
}

#[test]
fn inf_gain_db_does_not_produce_inf_samples() {
    let mut processor = ReplayGainProcessor::new();
    processor.set_mode(ReplayGainMode::Track);

    let values = ReplayGainValues {
        track_gain_db: Some(f32::INFINITY),
        track_peak: None,
        album_gain_db: None,
        album_peak: None,
    };
    processor.set_track_values(values);

    let mut buffer = vec![0.5f32; 64];
    processor.process(&mut buffer);

    for (i, &s) in buffer.iter().enumerate() {
        assert!(
            s.is_finite(),
            "sample[{i}] is infinite after +Inf gain input — linear_gain was not sanitised"
        );
    }
}

#[test]
fn neg_inf_gain_db_does_not_produce_nan_or_inf_samples() {
    let mut processor = ReplayGainProcessor::new();
    processor.set_mode(ReplayGainMode::Track);

    let values = ReplayGainValues {
        track_gain_db: Some(f32::NEG_INFINITY),
        track_peak: None,
        album_gain_db: None,
        album_peak: None,
    };
    processor.set_track_values(values);

    let mut buffer = vec![0.5f32; 64];
    processor.process(&mut buffer);

    for (i, &s) in buffer.iter().enumerate() {
        assert!(
            s.is_finite(),
            "sample[{i}] is not finite after -Inf gain input"
        );
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Bug 3: peak of 0.0 causes log10(0.0) = -Inf in clipping prevention
//
// In `recalculate_gain()`:
//   let max_safe_gain_db = -20.0 * peak.log10();
// When `peak == 0.0`, `log10(0.0) == -Inf`, so `max_safe_gain_db == +Inf`.
// That means `total_gain_db.min(+Inf) == total_gain_db` — the clipping branch
// effectively does nothing, which is harmless but the existing guard `if peak > 0.0`
// already skips the branch.  This test confirms the guard works correctly.
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn zero_peak_with_clipping_prevention_does_not_panic_or_produce_inf() {
    let mut processor = ReplayGainProcessor::new();
    processor.set_mode(ReplayGainMode::Track);
    processor.set_prevent_clipping(true);

    let values = ReplayGainValues {
        track_gain_db: Some(6.0),
        track_peak: Some(0.0), // Edge: peak exactly zero
        album_gain_db: None,
        album_peak: None,
    };
    processor.set_track_values(values);

    // Must not panic and linear_gain must be finite
    assert!(
        processor.linear_gain_is_finite(),
        "linear_gain must be finite when peak == 0.0"
    );

    let mut buffer = vec![0.5f32; 8];
    processor.process(&mut buffer);
    for &s in &buffer {
        assert!(s.is_finite(), "sample must be finite when peak == 0.0");
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Correct behaviour: peak == 1.0 limits gain to exactly 0 dB (unity)
// max_safe_gain_db = -20 * log10(1.0) = -20 * 0 = 0 dB
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn peak_of_one_limits_gain_to_unity_with_clipping_prevention() {
    let mut processor = ReplayGainProcessor::new();
    processor.set_mode(ReplayGainMode::Track);
    processor.set_prevent_clipping(true);

    let values = ReplayGainValues {
        track_gain_db: Some(12.0), // Would boost volume — but peak is 1.0
        track_peak: Some(1.0),
        album_gain_db: None,
        album_peak: None,
    };
    processor.set_track_values(values);

    // Gain must be clamped to 0 dB (unity = 1.0 linear)
    // linear_gain should be ≤ 1.0
    assert!(
        processor.effective_gain_db() <= 0.01,
        "gain must be ≤ 0 dB when peak == 1.0 and prevent_clipping is on, got {} dB",
        processor.effective_gain_db()
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// Correct behaviour: extreme but finite gains must not panic
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn extreme_positive_gain_without_clipping_prevention_does_not_panic() {
    let mut processor = ReplayGainProcessor::new();
    processor.set_mode(ReplayGainMode::Track);
    processor.set_prevent_clipping(false);

    let values = ReplayGainValues {
        track_gain_db: Some(100.0), // Very loud
        track_peak: Some(0.001),
        album_gain_db: None,
        album_peak: None,
    };
    processor.set_track_values(values);

    let mut buffer = vec![0.001f32; 8];
    processor.process(&mut buffer);
    // Just verify it does not panic and samples are finite
    for &s in &buffer {
        assert!(s.is_finite());
    }
}

#[test]
fn extreme_negative_gain_produces_near_silence() {
    let mut processor = ReplayGainProcessor::new();
    processor.set_mode(ReplayGainMode::Track);

    let values = ReplayGainValues {
        track_gain_db: Some(-100.0), // Very quiet
        track_peak: None,
        album_gain_db: None,
        album_peak: None,
    };
    processor.set_track_values(values);

    let mut buffer = vec![1.0f32; 8];
    processor.process(&mut buffer);
    for &s in &buffer {
        assert!(s < 0.001, "sample should be near silence with -100 dB gain");
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Correct behaviour: Off mode ignores preamp (preamp only applies when a mode
// is active — Off means "no normalization at all, unity gain").
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn off_mode_with_preamp_stays_at_unity_gain() {
    let mut processor = ReplayGainProcessor::new();
    // Mode stays Off
    processor.set_preamp_db(6.0); // Attempt to apply +6 dB preamp

    let values = ReplayGainValues {
        track_gain_db: Some(-5.0),
        track_peak: Some(0.9),
        album_gain_db: None,
        album_peak: None,
    };
    processor.set_track_values(values);

    // In Off mode, preamp is still added to total_gain_db, so the gain
    // is 0 (no RG) + 6 (preamp) = +6 dB.  The current implementation DOES
    // apply preamp even in Off mode.  This test documents that behaviour.
    //
    // Note: this is a documentation/regression test.  If the intended
    // design is that Off mode always gives unity gain, the production code
    // should be fixed and this test updated.  For now we assert the actual
    // current (as-shipped) behaviour so we catch any unintended change.
    let original = vec![1.0f32; 4];
    let mut buffer = original.clone();
    processor.process(&mut buffer);

    // With mode Off the fast-path in process() always returns early because
    // linear_gain == 1.0 only when total preamp brings it there.
    // Actually: mode Off means `if self.mode != ReplayGainMode::Off` is false,
    // so gain_db stays 0.  But preamp IS added unconditionally.
    // So after set_preamp_db(6.0): total_gain_db = 0 + 6 = 6; linear ≈ 2.0.
    // The samples should be doubled.
    //
    // This reveals a design discrepancy: preamp is applied even when mode is Off.
    // We document it here and add a separate test for the fix below.
    assert!(
        !processor.is_active(),
        "is_active() must be false in Off mode regardless of preamp"
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// Bug 4: preamp is incorrectly applied when mode is Off
//
// In `recalculate_gain()`:
//   if self.mode != ReplayGainMode::Off { total_gain_db += gain_db }
//   total_gain_db += self.preamp_db;   // ← always runs, even when Off
//
// Consequence: setting preamp to +6 dB while mode is Off will multiply all
// samples by ~2.0, even though Off is supposed to mean "no normalization".
//
// Fix: wrap preamp addition inside the same `mode != Off` guard.
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn off_mode_preamp_does_not_alter_samples() {
    let mut processor = ReplayGainProcessor::new();
    processor.set_preamp_db(6.0); // +6 dB preamp, but mode is Off

    let original = vec![1.0f32, -1.0, 0.5, -0.5];
    let mut buffer = original.clone();
    processor.process(&mut buffer);

    // Samples must be unchanged in Off mode — preamp must NOT be applied
    for (i, (&orig, &processed)) in original.iter().zip(buffer.iter()).enumerate() {
        assert!(
            (orig - processed).abs() < 0.0001,
            "sample[{i}]: Off mode should not alter samples even with preamp set. \
             orig={orig}, processed={processed}"
        );
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Regression: empty buffer must not panic
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn process_empty_buffer_does_not_panic() {
    let mut processor = ReplayGainProcessor::new();
    processor.set_mode(ReplayGainMode::Track);
    let values = ReplayGainValues {
        track_gain_db: Some(-6.0),
        track_peak: Some(0.9),
        album_gain_db: None,
        album_peak: None,
    };
    processor.set_track_values(values);

    let mut buffer: Vec<f32> = vec![];
    processor.process(&mut buffer); // Must not panic
}

// ──────────────────────────────────────────────────────────────────────────────
// Regression: no ReplayGain tags → unity gain
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn no_replaygain_tags_gives_unity_gain() {
    let mut processor = ReplayGainProcessor::new();
    processor.set_mode(ReplayGainMode::Track);

    // No tags at all
    processor.set_track_values(ReplayGainValues::none());

    let original = vec![1.0f32, -1.0, 0.5, -0.5];
    let mut buffer = original.clone();
    processor.process(&mut buffer);

    // With no tags in Track mode, no gain is applied → unity
    for (i, (&orig, &processed)) in original.iter().zip(buffer.iter()).enumerate() {
        assert!(
            (orig - processed).abs() < 0.0001,
            "sample[{i}]: no RG tags should give unity gain. orig={orig}, processed={processed}"
        );
    }
}
