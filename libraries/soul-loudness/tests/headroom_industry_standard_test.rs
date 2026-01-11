//! Industry Standard Headroom Management Tests
//!
//! Comprehensive tests for headroom management based on industry standards:
//!
//! # ReplayGain 2.0 Specification
//! Reference: https://wiki.hydrogenaudio.org/index.php?title=ReplayGain_2.0_specification
//! - Target loudness: -18 LUFS (equivalent to 89 dB SPL reference)
//! - Peak-based clipping prevention using stored peak values
//! - Gain calculation: Gain = Reference (-18 LUFS) - Track Loudness
//!
//! # EBU R128 Recommendations
//! Reference: https://tech.ebu.ch/docs/r/r128.pdf
//! - Broadcast target: -23 LUFS (+/- 1 LU tolerance)
//! - Streaming target: -14 LUFS (common for Spotify, YouTube)
//! - Maximum true peak: -1 dBTP (or -2 dBTP for lossy codec distribution)
//! - Headroom requirement: 1 dB below 0 dBFS minimum
//!
//! # ReplayGain Clipping Prevention Methods
//! Reference: https://replaygain.hydrogenaudio.org/player_clipping.html
//! 1. Hard limiting: Clip peaks at just below full scale
//! 2. Gain reduction: Reduce gain based on peak values to prevent clipping
//!    Formula: safe_gain = min(desired_gain, -peak_dBFS)
//!
//! # Professional Mastering Headroom Practices
//! Reference: https://www.izotope.com/en/learn/headroom-how-to-set-levels-mixing-and-mastering
//! - Pre-mastering headroom: -3 dB to -6 dB recommended
//! - True peak ceiling: -1.0 dBTP (EBU R128), -1.5 dBTP (music production)
//! - 24-bit recording: 15-20 dB headroom common practice

use soul_loudness::{
    headroom::{calculate_auto_headroom, calculate_eq_headroom, HeadroomManager, HeadroomMode},
    LoudnessNormalizer, NormalizationMode, ReplayGainCalculator, TruePeakLimiter,
    EBU_R128_BROADCAST_LUFS, EBU_R128_STREAMING_LUFS, REPLAYGAIN_REFERENCE_LUFS,
};

// ============================================================================
// Constants from Industry Standards
// ============================================================================

/// ReplayGain 2.0 reference level (-18 LUFS)
/// This is equivalent to 89 dB SPL playback level on SMPTE calibrated systems
const RG2_REFERENCE_LUFS: f64 = -18.0;

/// ReplayGain original reference (89 dB SPL = -14 dBFS headroom)
const RG_REFERENCE_HEADROOM_DB: f64 = -14.0;

/// EBU R128 recommended maximum true peak (-1 dBTP)
const EBU_MAX_TRUE_PEAK_DBTP: f64 = -1.0;

/// EBU R128 recommended true peak for lossy codec distribution (-2 dBTP)
const EBU_LOSSY_TRUE_PEAK_DBTP: f64 = -2.0;

/// Typical streaming service target (Spotify, YouTube)
const STREAMING_TARGET_LUFS: f64 = -14.0;

/// Professional mastering recommended pre-headroom
const MASTERING_HEADROOM_DB: f64 = -6.0;

// ============================================================================
// Test Scenarios from ReplayGain Specification
// ============================================================================

/// ReplayGain scenario: Loud, dynamically compressed pop/rock track
/// - Integrated loudness: -10 LUFS (loud modern master)
/// - True peak: -0.5 dBTP (heavily limited)
/// - Expected gain: -18 - (-10) = -8 dB (attenuate)
/// - Clipping risk: None (gain is negative)
#[test]
fn test_rg_scenario_loud_compressed_track() {
    let track_lufs = -10.0;
    let track_peak_dbtp = -0.5;

    // Calculate ReplayGain
    let expected_gain = RG2_REFERENCE_LUFS - track_lufs; // -18 - (-10) = -8 dB
    assert!(
        (expected_gain - (-8.0)).abs() < 0.01,
        "Loud track gain calculation error"
    );

    // Verify no clipping (peak + gain < 0)
    let post_gain_peak = track_peak_dbtp + expected_gain; // -0.5 + (-8) = -8.5 dBTP
    assert!(
        post_gain_peak < 0.0,
        "Loud track should not clip after RG: post-gain peak = {:.2} dBTP",
        post_gain_peak
    );

    // Test with HeadroomManager
    let mut headroom = HeadroomManager::new();
    headroom.set_mode(HeadroomMode::Auto);
    headroom.set_replaygain_db(expected_gain);

    // With negative RG, no headroom attenuation needed
    let attenuation = headroom.attenuation_db();
    assert!(
        attenuation.abs() < 0.1,
        "Negative RG should not need headroom: got {:.2} dB",
        attenuation
    );

    println!(
        "Loud compressed track test PASSED:\n\
         Input: {:.1} LUFS, peak {:.1} dBTP\n\
         RG gain: {:.2} dB\n\
         Post-gain peak: {:.2} dBTP\n\
         Headroom attenuation: {:.2} dB",
        track_lufs, track_peak_dbtp, expected_gain, post_gain_peak, attenuation
    );
}

/// ReplayGain scenario: Quiet classical music with high dynamic range
/// - Integrated loudness: -25 LUFS (quiet average)
/// - True peak: -3 dBTP (dynamic peaks)
/// - Expected gain: -18 - (-25) = +7 dB (amplify)
/// - Clipping risk: HIGH (+7 dB gain + -3 dBTP peak = +4 dBTP would clip)
#[test]
fn test_rg_scenario_quiet_dynamic_classical() {
    let track_lufs = -25.0;
    let track_peak_dbtp = -3.0;

    // Calculate ReplayGain
    let expected_gain = RG2_REFERENCE_LUFS - track_lufs; // -18 - (-25) = +7 dB
    assert!(
        (expected_gain - 7.0).abs() < 0.01,
        "Classical track gain calculation error"
    );

    // Check clipping (peak + gain > 0 means clipping)
    let post_gain_peak = track_peak_dbtp + expected_gain; // -3 + 7 = +4 dBTP
    let would_clip = post_gain_peak > 0.0;
    assert!(
        would_clip,
        "Classical track SHOULD clip without prevention: post-gain peak = {:.2} dBTP",
        post_gain_peak
    );

    // Calculate safe gain (clipping prevention via gain reduction)
    // Safe gain = -peak_dBTP = -(-3) = +3 dB
    let safe_gain = (-track_peak_dbtp).min(expected_gain);
    let post_safe_peak = track_peak_dbtp + safe_gain; // -3 + 3 = 0 dBTP
    assert!(
        post_safe_peak <= 0.0,
        "Safe gain should prevent clipping: post-gain peak = {:.2} dBTP",
        post_safe_peak
    );

    // Test with HeadroomManager
    let mut headroom = HeadroomManager::new();
    headroom.set_mode(HeadroomMode::Auto);
    headroom.set_replaygain_db(expected_gain);

    // HeadroomManager should attenuate by positive gain amount
    let attenuation = headroom.attenuation_db();
    assert!(
        (attenuation - (-7.0)).abs() < 0.1,
        "Auto headroom should attenuate by +7 dB gain: got {:.2} dB",
        attenuation
    );

    println!(
        "Quiet classical track test PASSED:\n\
         Input: {:.1} LUFS, peak {:.1} dBTP\n\
         RG gain: {:.2} dB (would clip!)\n\
         Safe gain: {:.2} dB\n\
         Headroom attenuation: {:.2} dB",
        track_lufs, track_peak_dbtp, expected_gain, safe_gain, attenuation
    );
}

/// ReplayGain scenario: Very quiet ambient/drone music
/// - Integrated loudness: -30 LUFS (very quiet)
/// - True peak: -10 dBTP (low peaks)
/// - Expected gain: -18 - (-30) = +12 dB (high amplification)
/// - Clipping risk: YES (+12 dB + -10 dBTP = +2 dBTP)
#[test]
fn test_rg_scenario_very_quiet_ambient() {
    let track_lufs = -30.0;
    let track_peak_dbtp = -10.0;

    let expected_gain = RG2_REFERENCE_LUFS - track_lufs; // +12 dB
    let post_gain_peak = track_peak_dbtp + expected_gain; // +2 dBTP

    assert!(
        post_gain_peak > 0.0,
        "Very quiet track should clip without prevention"
    );

    // With safe gain reduction
    let safe_gain = (-track_peak_dbtp).min(expected_gain); // min(10, 12) = 10 dB
    assert!(
        (safe_gain - 10.0).abs() < 0.01,
        "Safe gain should be limited to +10 dB"
    );

    let mut headroom = HeadroomManager::new();
    headroom.set_mode(HeadroomMode::Auto);
    headroom.set_replaygain_db(expected_gain);

    let attenuation = headroom.attenuation_db();
    assert!(
        (attenuation - (-12.0)).abs() < 0.1,
        "Auto headroom should attenuate by full +12 dB gain"
    );

    println!(
        "Very quiet ambient track test PASSED:\n\
         Input: {:.1} LUFS, peak {:.1} dBTP\n\
         RG gain: {:.2} dB\n\
         Safe gain: {:.2} dB\n\
         Headroom attenuation: {:.2} dB",
        track_lufs, track_peak_dbtp, expected_gain, safe_gain, attenuation
    );
}

/// ReplayGain scenario: Reference level track (no adjustment needed)
/// - Integrated loudness: -18 LUFS (already at reference)
/// - Expected gain: 0 dB
#[test]
fn test_rg_scenario_reference_level_track() {
    let track_lufs = RG2_REFERENCE_LUFS;
    let _track_peak_dbtp = -6.0;

    let expected_gain = RG2_REFERENCE_LUFS - track_lufs; // 0 dB
    assert!(
        expected_gain.abs() < 0.01,
        "Reference level track should need 0 dB adjustment"
    );

    let mut headroom = HeadroomManager::new();
    headroom.set_mode(HeadroomMode::Auto);
    headroom.set_replaygain_db(expected_gain);

    let attenuation_linear = headroom.attenuation_linear();
    assert!(
        (attenuation_linear - 1.0).abs() < 0.001,
        "No attenuation needed for reference level track"
    );

    println!(
        "Reference level track test PASSED:\n\
         Input: {:.1} LUFS (= reference)\n\
         RG gain: {:.2} dB\n\
         Headroom attenuation: unity (1.0)",
        track_lufs, expected_gain
    );
}

// ============================================================================
// Cumulative Gain Tracking Tests
// ============================================================================

/// Test cumulative gain: ReplayGain + Preamp
/// Per ReplayGain spec: "Pre-amp gain simply gets added to each song's gain value"
#[test]
fn test_cumulative_gain_rg_plus_preamp() {
    let rg_gain = 5.0; // +5 dB RG (quiet track)
    let preamp = 3.0; // +3 dB user preamp boost

    let mut headroom = HeadroomManager::new();
    headroom.set_mode(HeadroomMode::Auto);
    headroom.set_replaygain_db(rg_gain);
    headroom.set_preamp_db(preamp);

    let total_gain = headroom.total_potential_gain_db();
    assert!(
        (total_gain - 8.0).abs() < 0.01,
        "Total gain should be RG + preamp: {:.2} dB",
        total_gain
    );

    let attenuation = headroom.attenuation_db();
    assert!(
        (attenuation - (-8.0)).abs() < 0.1,
        "Should attenuate by cumulative gain: {:.2} dB",
        attenuation
    );

    println!(
        "Cumulative gain (RG + preamp) test PASSED:\n\
         RG gain: +{:.1} dB\n\
         Preamp: +{:.1} dB\n\
         Total: +{:.1} dB\n\
         Headroom: {:.1} dB",
        rg_gain, preamp, total_gain, attenuation
    );
}

/// Test cumulative gain: ReplayGain + Preamp + EQ boost
/// This is the full signal chain scenario
#[test]
fn test_cumulative_gain_full_chain() {
    let rg_gain = 4.0; // +4 dB RG
    let preamp = 2.0; // +2 dB preamp
    let eq_boost = 6.0; // +6 dB max EQ band boost

    let mut headroom = HeadroomManager::new();
    headroom.set_mode(HeadroomMode::Auto);
    headroom.set_replaygain_db(rg_gain);
    headroom.set_preamp_db(preamp);
    headroom.set_eq_max_boost_db(eq_boost);

    let total_gain = headroom.total_potential_gain_db();
    assert!(
        (total_gain - 12.0).abs() < 0.01,
        "Total gain should be RG + preamp + EQ: {:.2} dB",
        total_gain
    );

    let attenuation = headroom.attenuation_db();
    assert!(
        (attenuation - (-12.0)).abs() < 0.1,
        "Should attenuate by full cumulative gain: {:.2} dB",
        attenuation
    );

    println!(
        "Full chain cumulative gain test PASSED:\n\
         RG: +{:.1} dB, Preamp: +{:.1} dB, EQ: +{:.1} dB\n\
         Total: +{:.1} dB, Headroom: {:.1} dB",
        rg_gain, preamp, eq_boost, total_gain, attenuation
    );
}

/// Test cumulative gain with negative RG (loud track)
/// Negative RG should reduce total, potentially eliminating need for headroom
#[test]
fn test_cumulative_gain_negative_rg() {
    let rg_gain = -8.0; // -8 dB RG (loud track)
    let preamp = 3.0; // +3 dB preamp
    let eq_boost = 4.0; // +4 dB EQ

    let mut headroom = HeadroomManager::new();
    headroom.set_mode(HeadroomMode::Auto);
    headroom.set_replaygain_db(rg_gain);
    headroom.set_preamp_db(preamp);
    headroom.set_eq_max_boost_db(eq_boost);

    let total_gain = headroom.total_potential_gain_db();
    // -8 + 3 + 4 = -1 dB (net attenuation)
    assert!(
        (total_gain - (-1.0)).abs() < 0.01,
        "Total gain should account for negative RG: {:.2} dB",
        total_gain
    );

    // No headroom attenuation needed (total is negative)
    let attenuation = headroom.attenuation_db();
    assert!(
        attenuation.abs() < 0.1,
        "No headroom needed when total gain is negative: {:.2} dB",
        attenuation
    );

    println!(
        "Negative RG cumulative gain test PASSED:\n\
         RG: {:.1} dB, Preamp: +{:.1} dB, EQ: +{:.1} dB\n\
         Total: {:.1} dB (no headroom needed)",
        rg_gain, preamp, eq_boost, total_gain
    );
}

// ============================================================================
// EQ Boost Headroom Calculation Tests
// ============================================================================

/// Test EQ headroom calculation: various band configurations
/// Only positive boosts contribute to headroom requirements
#[test]
fn test_eq_headroom_various_bands() {
    // Typical parametric EQ with cuts and boosts
    let eq_bands_1 = [3.0, -2.0, 6.0, 0.0, -1.0, 4.0];
    let max_boost_1 = calculate_eq_headroom(&eq_bands_1);
    assert!(
        (max_boost_1 - 6.0).abs() < 0.001,
        "Max boost should be 6.0 dB"
    );

    // All cuts (negative)
    let eq_bands_2 = [-3.0, -2.0, -6.0, -1.0];
    let max_boost_2 = calculate_eq_headroom(&eq_bands_2);
    assert!(
        max_boost_2.abs() < 0.001,
        "All cuts should yield 0 dB headroom requirement"
    );

    // Single high boost
    let eq_bands_3 = [0.0, 0.0, 12.0, 0.0];
    let max_boost_3 = calculate_eq_headroom(&eq_bands_3);
    assert!(
        (max_boost_3 - 12.0).abs() < 0.001,
        "Single +12 dB boost should require 12 dB headroom"
    );

    // Empty (flat)
    let eq_bands_4: [f64; 0] = [];
    let max_boost_4 = calculate_eq_headroom(&eq_bands_4);
    assert!(
        max_boost_4.abs() < 0.001,
        "Empty EQ should require 0 dB headroom"
    );

    println!(
        "EQ headroom calculation tests PASSED:\n\
         Mixed bands [3, -2, 6, 0, -1, 4]: {:.1} dB\n\
         All cuts: {:.1} dB\n\
         Single +12: {:.1} dB\n\
         Flat: {:.1} dB",
        max_boost_1, max_boost_2, max_boost_3, max_boost_4
    );
}

/// Test auto headroom calculation with EQ
#[test]
fn test_auto_headroom_with_eq() {
    let rg = 3.0;
    let preamp = 2.0;
    let eq_bands = [1.0, -3.0, 5.0, 2.0, -1.0];
    let other_gains: [f64; 0] = [];

    let headroom = calculate_auto_headroom(rg, preamp, &eq_bands, &other_gains);

    // Total positive: RG(3) + preamp(2) + max_eq(5) = 10 dB
    // Headroom should be -10 dB
    assert!(
        (headroom - (-10.0)).abs() < 0.001,
        "Auto headroom should be -10 dB: got {:.2} dB",
        headroom
    );

    println!(
        "Auto headroom with EQ test PASSED:\n\
         RG: +{:.1} dB, Preamp: +{:.1} dB, Max EQ: +5.0 dB\n\
         Calculated headroom: {:.1} dB",
        rg, preamp, headroom
    );
}

// ============================================================================
// Mode Switching Tests
// ============================================================================

/// Test switching between Auto, Manual, and Disabled modes
#[test]
fn test_headroom_mode_switching() {
    let mut headroom = HeadroomManager::new();
    headroom.set_replaygain_db(10.0);
    headroom.set_preamp_db(2.0);
    headroom.set_eq_max_boost_db(4.0);

    // Auto mode: attenuate by cumulative gain (16 dB)
    headroom.set_mode(HeadroomMode::Auto);
    let auto_atten = headroom.attenuation_db();
    assert!(
        (auto_atten - (-16.0)).abs() < 0.1,
        "Auto mode: {:.2} dB (expected -16)",
        auto_atten
    );

    // Manual mode: fixed attenuation (-6 dB typical)
    headroom.set_mode(HeadroomMode::Manual(-6.0));
    let manual_atten = headroom.attenuation_db();
    assert!(
        (manual_atten - (-6.0)).abs() < 0.1,
        "Manual mode: {:.2} dB (expected -6)",
        manual_atten
    );

    // Disabled mode: no attenuation
    headroom.set_mode(HeadroomMode::Disabled);
    let disabled_atten = headroom.attenuation_linear();
    assert!(
        (disabled_atten - 1.0).abs() < 0.001,
        "Disabled mode: {:.4} (expected 1.0)",
        disabled_atten
    );

    println!(
        "Mode switching test PASSED:\n\
         Auto: {:.1} dB, Manual(-6): {:.1} dB, Disabled: {:.3} (linear)",
        auto_atten, manual_atten, disabled_atten
    );
}

/// Test mode string parsing and roundtrip
#[test]
fn test_headroom_mode_string_parsing() {
    // Test various string formats
    let test_cases = [
        ("auto", HeadroomMode::Auto),
        ("automatic", HeadroomMode::Auto),
        ("disabled", HeadroomMode::Disabled),
        ("off", HeadroomMode::Disabled),
        ("none", HeadroomMode::Disabled),
        ("manual:-6", HeadroomMode::Manual(-6.0)),
        ("-3.5", HeadroomMode::Manual(-3.5)),
    ];

    for (input, expected) in test_cases {
        let parsed = HeadroomMode::from_str(input);
        assert_eq!(
            parsed,
            Some(expected),
            "Failed to parse '{}': got {:?}",
            input,
            parsed
        );
    }

    // Test roundtrip
    let modes = [
        HeadroomMode::Auto,
        HeadroomMode::Disabled,
        HeadroomMode::Manual(-6.0),
        HeadroomMode::Manual(-12.0),
    ];

    for mode in modes {
        let s = mode.as_str();
        let parsed = HeadroomMode::from_str(&s);
        assert_eq!(
            parsed,
            Some(mode),
            "Roundtrip failed for {:?}: {} -> {:?}",
            mode,
            s,
            parsed
        );
    }

    println!("Mode string parsing test PASSED");
}

// ============================================================================
// Clipping Prevention Verification Tests
// ============================================================================

/// Verify that headroom attenuation prevents clipping
/// Test signal: 0 dBFS peak with +10 dB gain (would clip to +10 dBFS)
#[test]
fn test_clipping_prevention_with_headroom() {
    let mut headroom = HeadroomManager::new();
    headroom.set_mode(HeadroomMode::Auto);
    headroom.set_replaygain_db(10.0); // +10 dB gain

    // Generate near-full-scale samples
    let mut samples: Vec<f32> = vec![0.95, -0.95, 0.9, -0.9, 0.85, -0.85];

    // Apply headroom attenuation
    headroom.process(&mut samples);

    // Verify no sample exceeds 1.0 (0 dBFS)
    for &sample in &samples {
        assert!(
            sample.abs() <= 1.0,
            "Sample exceeded 0 dBFS after headroom: {:.4}",
            sample
        );
    }

    // With -10 dB headroom on 0.95 input: 0.95 * 10^(-10/20) = 0.95 * 0.316 = 0.30
    let expected_max = 0.95 * 10.0_f32.powf(-10.0 / 20.0);
    let actual_max = samples.iter().map(|s| s.abs()).fold(0.0_f32, f32::max);
    assert!(
        (actual_max - expected_max).abs() < 0.01,
        "Headroom attenuation incorrect: expected {:.3}, got {:.3}",
        expected_max,
        actual_max
    );

    println!(
        "Clipping prevention test PASSED:\n\
         Input peak: 0.95 (before +10 dB gain)\n\
         After headroom: {:.3} (prevented clipping)",
        actual_max
    );
}

/// Test that subsequent gain stages don't cause clipping
/// Signal flow: [Headroom] -> [RG gain] -> [Preamp] -> [EQ] -> should not clip
#[test]
fn test_full_signal_chain_no_clipping() {
    let rg_db = 5.0;
    let preamp_db = 2.0;
    let eq_max_db = 4.0;
    let total_gain_db = rg_db + preamp_db + eq_max_db; // +11 dB

    // Setup headroom
    let mut headroom = HeadroomManager::new();
    headroom.set_mode(HeadroomMode::Auto);
    headroom.set_replaygain_db(rg_db);
    headroom.set_preamp_db(preamp_db);
    headroom.set_eq_max_boost_db(eq_max_db);

    // Start with full scale
    let mut samples: Vec<f32> = vec![1.0, -1.0, 0.99, -0.99];

    // Apply headroom first (attenuates)
    headroom.process(&mut samples);

    // Simulate subsequent gain stages
    let rg_linear = 10.0_f32.powf(rg_db as f32 / 20.0);
    let preamp_linear = 10.0_f32.powf(preamp_db as f32 / 20.0);
    let eq_linear = 10.0_f32.powf(eq_max_db as f32 / 20.0);

    for sample in &mut samples {
        *sample *= rg_linear; // Apply RG
        *sample *= preamp_linear; // Apply preamp
        *sample *= eq_linear; // Apply worst-case EQ boost
    }

    // Verify no clipping occurred
    let max_sample = samples.iter().map(|s| s.abs()).fold(0.0_f32, f32::max);
    assert!(
        max_sample <= 1.01, // Allow tiny tolerance for FP precision
        "Signal chain clipping: max sample = {:.4} after +{:.1} dB gain",
        max_sample,
        total_gain_db
    );

    println!(
        "Full signal chain test PASSED:\n\
         Total gain: +{:.1} dB\n\
         Final max sample: {:.4} (no clipping)",
        total_gain_db, max_sample
    );
}

// ============================================================================
// Integration with LoudnessNormalizer Tests
// ============================================================================

/// Test headroom manager integration with normalizer
#[test]
fn test_headroom_normalizer_integration() {
    let mut normalizer = LoudnessNormalizer::new(48000, 2);
    normalizer.set_mode(NormalizationMode::ReplayGainTrack);
    normalizer.set_prevent_clipping(true);
    normalizer.set_use_internal_limiter(true);

    // Simulate quiet track that would need +10 dB gain
    // Peak at -2 dBTP means +10 dB gain would clip to +8 dBTP
    normalizer.set_track_gain(10.0, -2.0);

    // The normalizer should limit gain to prevent clipping
    // Safe gain = min(10, -(-2)) = min(10, 2) = +2 dB
    let effective_gain = normalizer.effective_gain_db();

    // With clipping prevention, gain should be limited
    assert!(
        effective_gain <= 2.1,
        "Normalizer should limit gain to prevent clipping: got {:.2} dB",
        effective_gain
    );

    println!(
        "Normalizer integration test PASSED:\n\
         Requested gain: +10 dB, Peak: -2 dBTP\n\
         Effective gain: {:.2} dB (limited to prevent clipping)",
        effective_gain
    );
}

/// Test headroom with external limiter (normalizer internal limiter disabled)
#[test]
fn test_headroom_with_external_limiter() {
    let mut normalizer = LoudnessNormalizer::new(48000, 2);
    normalizer.set_mode(NormalizationMode::ReplayGainTrack);
    normalizer.set_use_internal_limiter(false); // External limiter will handle peaks

    // When external limiter is used, we might allow more aggressive gain
    // The headroom manager provides pre-emptive attenuation
    let mut headroom = HeadroomManager::new();
    headroom.set_mode(HeadroomMode::Auto);
    headroom.set_replaygain_db(8.0);

    // Headroom attenuation happens first
    let attenuation = headroom.attenuation_linear();
    assert!(
        attenuation < 1.0,
        "Auto headroom should attenuate for +8 dB gain"
    );

    // Check limiter usage flag
    assert!(
        !normalizer.uses_internal_limiter(),
        "Internal limiter should be disabled"
    );

    println!(
        "External limiter test PASSED:\n\
         Internal limiter: disabled\n\
         Headroom attenuation: {:.4} linear ({:.2} dB)",
        attenuation,
        20.0 * attenuation.log10() as f64
    );
}

// ============================================================================
// EBU R128 Headroom Compliance Tests
// ============================================================================

/// Test EBU R128 recommended headroom (-1 dBTP ceiling)
/// Per EBU Tech 3343: "leave a headroom of 1 dB below 0 dBFS"
#[test]
fn test_ebu_r128_headroom_compliance() {
    let mut limiter = TruePeakLimiter::new(48000, 2);
    limiter.set_threshold_db(EBU_MAX_TRUE_PEAK_DBTP as f32); // -1 dBTP

    // Generate signal that will exceed threshold
    let mut samples: Vec<f32> = (0..48000)
        .map(|i| {
            let t = i as f32 / 48000.0;
            (2.0 * std::f32::consts::PI * 997.0 * t).sin() * 1.5 // +3.5 dB over FS
        })
        .collect();

    // Process through limiter (multiple passes for lookahead)
    for _ in 0..10 {
        limiter.process(&mut samples);
    }

    // Verify compliance with -1 dBTP ceiling
    let threshold_linear = 10.0_f32.powf(EBU_MAX_TRUE_PEAK_DBTP as f32 / 20.0);
    let max_sample = samples.iter().map(|s| s.abs()).fold(0.0_f32, f32::max);

    // Allow small tolerance (1%) for limiter implementation
    let tolerance_factor = 1.01;
    assert!(
        max_sample <= threshold_linear * tolerance_factor,
        "EBU R128 compliance failed: max {:.4} exceeds -1 dBTP ({:.4})",
        max_sample,
        threshold_linear
    );

    println!(
        "EBU R128 headroom compliance test PASSED:\n\
         Threshold: -1 dBTP ({:.4} linear)\n\
         Max sample: {:.4} ({:.2} dB)",
        threshold_linear,
        max_sample,
        20.0 * max_sample.log10()
    );
}

/// Test EBU R128 streaming headroom (-2 dBTP for lossy codecs)
/// Per EBU R128: "-2 dBTP if we know that later on lossy encoding will happen"
#[test]
fn test_ebu_r128_streaming_headroom() {
    let mut limiter = TruePeakLimiter::new(48000, 2);
    limiter.set_threshold_db(EBU_LOSSY_TRUE_PEAK_DBTP as f32); // -2 dBTP

    let mut samples: Vec<f32> = (0..48000)
        .map(|i| {
            let t = i as f32 / 48000.0;
            (2.0 * std::f32::consts::PI * 997.0 * t).sin() * 1.2
        })
        .collect();

    for _ in 0..10 {
        limiter.process(&mut samples);
    }

    let threshold_linear = 10.0_f32.powf(EBU_LOSSY_TRUE_PEAK_DBTP as f32 / 20.0);
    let max_sample = samples.iter().map(|s| s.abs()).fold(0.0_f32, f32::max);

    assert!(
        max_sample <= threshold_linear * 1.01,
        "Streaming headroom compliance failed: max {:.4} exceeds -2 dBTP ({:.4})",
        max_sample,
        threshold_linear
    );

    println!(
        "EBU R128 streaming headroom test PASSED:\n\
         Threshold: -2 dBTP ({:.4} linear)\n\
         Max sample: {:.4} ({:.2} dB)",
        threshold_linear,
        max_sample,
        20.0 * max_sample.log10()
    );
}

// ============================================================================
// Real-World Scenario Tests
// ============================================================================

/// Scenario: Playlist with mixed loudness levels
/// Tests that headroom management handles track-to-track transitions
#[test]
fn test_scenario_mixed_playlist() {
    let tracks = [
        ("Pop (loud)", -10.0_f64, -0.5_f64), // Loud with limited peaks
        ("Classical", -25.0, -3.0),          // Quiet with dynamic range
        ("Jazz", -20.0, -4.0),               // Medium with some dynamics
        ("Electronic", -8.0, -0.1),          // Very loud, heavily limited
    ];

    let _calc = ReplayGainCalculator::new();
    let mut headroom = HeadroomManager::new();
    headroom.set_mode(HeadroomMode::Auto);

    println!("Mixed playlist scenario test:");
    println!("{:-<60}", "");

    for (name, lufs, peak) in tracks {
        // Calculate RG gain
        let gain = REPLAYGAIN_REFERENCE_LUFS - lufs;

        // Update headroom manager
        headroom.clear_track_gains();
        headroom.set_replaygain_db(gain);

        let attenuation = headroom.attenuation_db();
        let would_clip = gain + peak > 0.0;
        let safe_gain = gain.min(-peak);

        println!(
            "{:12} | LUFS: {:5.1} | Peak: {:5.1} dBTP | RG: {:+6.2} dB | \
             Clip: {:5} | Safe: {:+6.2} dB | Headroom: {:6.2} dB",
            name, lufs, peak, gain, would_clip, safe_gain, attenuation
        );

        // Verify headroom is appropriate
        if gain > 0.0 {
            assert!(
                attenuation < -0.1,
                "{}: Positive gain should have headroom attenuation",
                name
            );
        }
    }

    println!("{:-<60}", "");
    println!("Mixed playlist scenario PASSED");
}

/// Scenario: High-gain EQ settings (bass boost, presence boost)
#[test]
fn test_scenario_aggressive_eq() {
    let eq_scenarios = [
        ("Bass boost", [8.0, 4.0, 0.0, 0.0, 0.0]),
        ("V-shape", [6.0, 2.0, -2.0, 2.0, 6.0]),
        ("Presence boost", [0.0, 0.0, 2.0, 8.0, 4.0]),
        ("Flat", [0.0, 0.0, 0.0, 0.0, 0.0]),
        ("All cut", [-3.0, -4.0, -5.0, -4.0, -3.0]),
    ];

    let rg_gain = 3.0; // +3 dB ReplayGain

    println!("Aggressive EQ scenario test:");
    println!("{:-<70}", "");

    for (name, eq_bands) in eq_scenarios {
        let max_eq = calculate_eq_headroom(&eq_bands);
        let headroom = calculate_auto_headroom(rg_gain, 0.0, &eq_bands, &[]);
        let total_gain = rg_gain + max_eq;

        println!(
            "{:15} | EQ max: {:+5.1} dB | Total gain: {:+6.1} dB | Headroom: {:6.1} dB",
            name, max_eq, total_gain, headroom
        );

        // Verify headroom calculation
        if total_gain > 0.0 {
            assert!(
                (headroom - (-total_gain)).abs() < 0.01,
                "{}: Headroom should match total positive gain",
                name
            );
        } else {
            assert!(
                headroom.abs() < 0.01,
                "{}: No headroom needed for net negative gain",
                name
            );
        }
    }

    println!("{:-<70}", "");
    println!("Aggressive EQ scenario PASSED");
}

/// Scenario: User with high preamp setting (audiophile volume preference)
#[test]
fn test_scenario_high_preamp_user() {
    // User prefers +6 dB preamp for their system calibration
    let preamp = 6.0;

    let tracks = [
        ("Quiet track", 8.0),  // Would be +14 dB total
        ("Medium track", 0.0), // Would be +6 dB total
        ("Loud track", -6.0),  // Would be 0 dB total
        ("Very loud", -10.0),  // Would be -4 dB total (no headroom needed)
    ];

    println!("High preamp user scenario test:");
    println!("{:-<55}", "");

    let mut headroom = HeadroomManager::new();
    headroom.set_mode(HeadroomMode::Auto);
    headroom.set_preamp_db(preamp);

    for (name, rg_gain) in tracks {
        headroom.set_replaygain_db(rg_gain);
        let total = rg_gain + preamp;
        let attenuation = headroom.attenuation_db();

        println!(
            "{:12} | RG: {:+5.1} dB | Total: {:+5.1} dB | Headroom: {:6.1} dB",
            name, rg_gain, total, attenuation
        );

        if total > 0.0 {
            assert!(
                (attenuation - (-total)).abs() < 0.1,
                "{}: Headroom should compensate for total gain",
                name
            );
        } else {
            assert!(attenuation.abs() < 0.1, "{}: No headroom needed", name);
        }
    }

    println!("{:-<55}", "");
    println!("High preamp user scenario PASSED");
}

// ============================================================================
// Performance Under Various ReplayGain Values Tests
// ============================================================================

/// Test headroom calculation performance across RG range
/// Per ReplayGain spec: typical values range from -12 dB to +12 dB
#[test]
fn test_performance_across_rg_range() {
    let rg_values: Vec<f64> = (-12..=12).map(|i| i as f64).collect();
    let mut headroom = HeadroomManager::new();
    headroom.set_mode(HeadroomMode::Auto);

    println!("RG range performance test:");
    println!("{:-<45}", "");

    for rg in rg_values {
        headroom.set_replaygain_db(rg);
        let attenuation_db = headroom.attenuation_db();
        let attenuation_linear = headroom.attenuation_linear();

        // Verify correct behavior
        if rg > 0.0 {
            assert!(
                (attenuation_db - (-rg)).abs() < 0.1,
                "RG +{:.0}: Attenuation should be {:.0} dB",
                rg,
                -rg
            );
        } else {
            assert!(
                attenuation_linear >= 0.999,
                "RG {:.0}: No attenuation needed",
                rg
            );
        }

        // Only print every 4th value for brevity
        if rg as i32 % 4 == 0 {
            println!(
                "RG: {:+5.0} dB | Attenuation: {:6.2} dB ({:.4} linear)",
                rg, attenuation_db, attenuation_linear
            );
        }
    }

    println!("{:-<45}", "");
    println!("RG range performance test PASSED");
}

/// Test headroom with extreme values (stress test)
#[test]
fn test_extreme_values_stress() {
    let mut headroom = HeadroomManager::new();
    headroom.set_mode(HeadroomMode::Auto);

    // Extreme positive gain
    headroom.set_replaygain_db(24.0);
    headroom.set_preamp_db(12.0); // Max preamp
    headroom.set_eq_max_boost_db(18.0);

    let total = headroom.total_potential_gain_db();
    assert!(
        (total - 54.0).abs() < 0.01,
        "Extreme total gain: {:.2}",
        total
    );

    let attenuation = headroom.attenuation_linear();
    assert!(
        attenuation > 0.0 && attenuation < 1.0,
        "Attenuation should be valid: {:.6}",
        attenuation
    );

    // Extreme negative gain (no attenuation needed)
    headroom.reset();
    headroom.set_replaygain_db(-24.0);
    headroom.set_preamp_db(-12.0);

    let attenuation_neg = headroom.attenuation_linear();
    assert!(
        (attenuation_neg - 1.0).abs() < 0.001,
        "Extreme negative gain should need no attenuation: {:.4}",
        attenuation_neg
    );

    println!(
        "Extreme values stress test PASSED:\n\
         Max total gain: +54 dB, Attenuation: {:.6}\n\
         Min total gain: -36 dB, Attenuation: 1.0 (none)",
        attenuation
    );
}

// ============================================================================
// Reference Level Verification Tests
// ============================================================================

/// Verify ReplayGain 2.0 uses -18 LUFS reference
#[test]
fn test_replaygain_2_reference_level() {
    assert_eq!(
        REPLAYGAIN_REFERENCE_LUFS, RG2_REFERENCE_LUFS,
        "ReplayGain 2.0 reference should be -18 LUFS"
    );

    let calc = ReplayGainCalculator::new();

    // Track at -18 LUFS should have 0 dB gain
    let info = soul_loudness::LoudnessInfo {
        integrated_lufs: -18.0,
        loudness_range_lu: 5.0,
        true_peak_dbfs: -6.0,
        sample_peak_dbfs: -6.5,
        duration_seconds: 180.0,
        sample_rate: 44100,
        channels: 2,
    };

    let gain = calc.track_gain(&info);
    assert!(
        gain.gain_db.abs() < 0.001,
        "Track at -18 LUFS should have 0 dB gain: got {:.4} dB",
        gain.gain_db
    );

    println!("ReplayGain 2.0 reference level verification PASSED");
}

/// Verify EBU R128 reference levels
#[test]
fn test_ebu_r128_reference_levels() {
    assert_eq!(
        EBU_R128_BROADCAST_LUFS, -23.0,
        "EBU R128 broadcast should be -23 LUFS"
    );

    assert_eq!(
        EBU_R128_STREAMING_LUFS, -14.0,
        "EBU R128 streaming should be -14 LUFS"
    );

    // Verify mode reference levels
    assert_eq!(
        NormalizationMode::EbuR128Broadcast.reference_lufs(),
        Some(-23.0)
    );
    assert_eq!(
        NormalizationMode::EbuR128Streaming.reference_lufs(),
        Some(-14.0)
    );
    assert_eq!(
        NormalizationMode::ReplayGainTrack.reference_lufs(),
        Some(-18.0)
    );
    assert_eq!(NormalizationMode::Disabled.reference_lufs(), None);

    println!("EBU R128 reference level verification PASSED");
}

// ============================================================================
// Summary Test
// ============================================================================

/// Comprehensive summary of industry standard compliance
#[test]
fn test_industry_standard_compliance_summary() {
    let mut passed = 0;
    let mut total = 0;

    // Test 1: ReplayGain 2.0 reference
    total += 1;
    if (REPLAYGAIN_REFERENCE_LUFS - (-18.0)).abs() < 0.001 {
        passed += 1;
    }

    // Test 2: EBU R128 broadcast reference
    total += 1;
    if (EBU_R128_BROADCAST_LUFS - (-23.0)).abs() < 0.001 {
        passed += 1;
    }

    // Test 3: Auto headroom for +10 dB gain
    total += 1;
    let mut h = HeadroomManager::new();
    h.set_mode(HeadroomMode::Auto);
    h.set_replaygain_db(10.0);
    if (h.attenuation_db() - (-10.0)).abs() < 0.1 {
        passed += 1;
    }

    // Test 4: Cumulative gain calculation
    total += 1;
    h.set_preamp_db(2.0);
    h.set_eq_max_boost_db(4.0);
    if (h.total_potential_gain_db() - 16.0).abs() < 0.01 {
        passed += 1;
    }

    // Test 5: Disabled mode passthrough
    total += 1;
    h.set_mode(HeadroomMode::Disabled);
    if (h.attenuation_linear() - 1.0).abs() < 0.001 {
        passed += 1;
    }

    // Test 6: Manual mode fixed attenuation
    total += 1;
    h.set_mode(HeadroomMode::Manual(-6.0));
    if (h.attenuation_db() - (-6.0)).abs() < 0.1 {
        passed += 1;
    }

    // Test 7: Clipping prevention (safe gain calculation)
    total += 1;
    let track_gain: f64 = 10.0;
    let track_peak: f64 = -3.0;
    let safe = track_gain.min(-track_peak); // min(10, 3) = 3
    if (safe - 3.0).abs() < 0.001 {
        passed += 1;
    }

    println!("\n============================================================");
    println!("HEADROOM INDUSTRY STANDARD COMPLIANCE SUMMARY");
    println!("============================================================");
    println!();
    println!("Standards referenced:");
    println!("  - ReplayGain 2.0 (Hydrogenaudio)");
    println!("  - EBU R128 v5.0 (November 2023)");
    println!("  - EBU Tech 3343 (Practical Guidelines)");
    println!("  - ITU-R BS.1770-5 (Loudness Measurement)");
    println!();
    println!("Key compliance points:");
    println!("  [x] ReplayGain 2.0 reference: -18 LUFS");
    println!("  [x] EBU R128 broadcast target: -23 LUFS");
    println!("  [x] EBU R128 streaming target: -14 LUFS");
    println!("  [x] Maximum true peak: -1 dBTP (broadcast)");
    println!("  [x] Streaming true peak: -2 dBTP (lossy codec)");
    println!();
    println!(
        "Test results: {}/{} passed ({:.0}%)",
        passed,
        total,
        100.0 * passed as f64 / total as f64
    );
    println!("============================================================\n");

    assert_eq!(
        passed, total,
        "Industry standard compliance: {} of {} tests passed",
        passed, total
    );
}
