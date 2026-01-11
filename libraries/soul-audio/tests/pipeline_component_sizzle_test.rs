//! Pipeline Component Sizzle Detection Tests
//!
//! These tests verify that ALL pipeline components handle parameter updates
//! without producing audio artifacts (sizzle, clicks, pops, zipper noise).
//!
//! IMPORTANT: These tests run at the backend level to enforce artifact-free
//! parameter updates regardless of how the UI or frontend changes parameters.
//!
//! Test methodology:
//! 1. Generate a continuous, phase-coherent sine wave
//! 2. Process through the effect while updating parameters
//! 3. Analyze output for discontinuities using derivative analysis
//! 4. Fail if artifacts exceed threshold
//!
//! The key insight is that a clean parameter change should NOT introduce
//! high-frequency transients (sudden jumps in the signal derivative).

use soul_audio::effects::{
    AudioEffect, Compressor, CompressorSettings, Crossfeed, CrossfeedPreset, EqBand, GraphicEq,
    Limiter, ParametricEq, StereoEnhancer,
};
use std::f32::consts::PI;

const SAMPLE_RATE: u32 = 44100;
const TEST_FREQ: f32 = 440.0; // A4 - common test tone

/// Generate a phase-continuous sine wave starting at a specific sample offset
fn generate_sine_wave(num_samples: usize, freq: f32, sample_rate: u32, start_sample: usize) -> Vec<f32> {
    let mut buffer = Vec::with_capacity(num_samples * 2);
    for i in 0..num_samples {
        let t = (start_sample + i) as f32 / sample_rate as f32;
        let sample = (2.0 * PI * freq * t).sin() * 0.5; // -6dB to leave headroom
        buffer.push(sample); // Left
        buffer.push(sample); // Right (mono for simplicity)
    }
    buffer
}

/// Analyze audio buffer for discontinuities (sizzle/clicks)
///
/// Returns (max_derivative, avg_derivative, discontinuity_count)
/// A discontinuity is detected when the derivative exceeds the threshold
fn analyze_for_discontinuities(buffer: &[f32], threshold: f32) -> (f32, f32, usize) {
    if buffer.len() < 4 {
        return (0.0, 0.0, 0);
    }

    let mut max_derivative: f32 = 0.0;
    let mut total_derivative: f32 = 0.0;
    let mut count = 0usize;
    let mut discontinuity_count = 0usize;

    // Analyze left channel only (every other sample)
    for i in 1..(buffer.len() / 2) {
        let prev = buffer[(i - 1) * 2];
        let curr = buffer[i * 2];
        let derivative = (curr - prev).abs();

        max_derivative = max_derivative.max(derivative);
        total_derivative += derivative;
        count += 1;

        if derivative > threshold {
            discontinuity_count += 1;
        }
    }

    let avg_derivative = if count > 0 {
        total_derivative / count as f32
    } else {
        0.0
    };

    (max_derivative, avg_derivative, discontinuity_count)
}

/// Calculate the expected maximum derivative for a sine wave
/// For a sine wave: max derivative = 2 * PI * freq / sample_rate * amplitude
fn expected_sine_derivative(freq: f32, sample_rate: u32, amplitude: f32) -> f32 {
    2.0 * PI * freq / sample_rate as f32 * amplitude
}

/// Test helper that processes audio through an effect while updating parameters
/// Returns the output buffer for analysis
fn process_with_parameter_updates<F, U>(
    effect: &mut F,
    update_fn: U,
    num_updates: usize,
    samples_per_update: usize,
) -> Vec<f32>
where
    F: AudioEffect,
    U: Fn(&mut F, usize), // update_fn(effect, update_index)
{
    let total_samples = num_updates * samples_per_update;
    let mut output = Vec::with_capacity(total_samples * 2);
    let mut sample_offset = 0;

    for update_idx in 0..num_updates {
        // Generate phase-continuous audio
        let mut buffer = generate_sine_wave(samples_per_update, TEST_FREQ, SAMPLE_RATE, sample_offset);

        // Update parameters before processing this chunk
        update_fn(effect, update_idx);

        // Process
        effect.process(&mut buffer, SAMPLE_RATE);

        output.extend_from_slice(&buffer);
        sample_offset += samples_per_update;
    }

    output
}

/// Strict threshold for detecting sizzle
/// This is based on the expected derivative of a 440Hz sine wave at 44.1kHz
/// with some margin for filter processing (filters can increase derivative slightly)
fn get_sizzle_threshold() -> f32 {
    // Expected max derivative for 440Hz sine at 0.5 amplitude
    let expected = expected_sine_derivative(TEST_FREQ, SAMPLE_RATE, 0.5);
    // Allow 3x the expected derivative - anything higher indicates a discontinuity
    // This is strict but reasonable for smooth parameter changes
    expected * 3.0
}

// ============================================================================
// PARAMETRIC EQ TESTS
// ============================================================================

#[test]
fn test_parametric_eq_band_gain_update_no_sizzle() {
    let mut eq = ParametricEq::new();
    eq.set_bands(vec![EqBand::peaking(1000.0, 0.0, 1.0)]);

    let output = process_with_parameter_updates(
        &mut eq,
        |eq, idx| {
            // Gradually increase gain from -6dB to +6dB
            let gain = -6.0 + (idx as f32 / 10.0) * 12.0;
            eq.set_bands(vec![EqBand::peaking(1000.0, gain.min(6.0), 1.0)]);
        },
        20,   // 20 updates
        1024, // 1024 samples per update (~23ms at 44.1kHz)
    );

    let threshold = get_sizzle_threshold();
    let (max_deriv, avg_deriv, discontinuities) = analyze_for_discontinuities(&output, threshold);

    println!(
        "ParametricEq band gain: max_deriv={:.6}, avg_deriv={:.6}, discontinuities={}, threshold={:.6}",
        max_deriv, avg_deriv, discontinuities, threshold
    );

    assert!(
        discontinuities == 0,
        "ParametricEq band gain change produced {} discontinuities (sizzle). Max derivative: {:.6}, threshold: {:.6}",
        discontinuities, max_deriv, threshold
    );
}

#[test]
fn test_parametric_eq_frequency_update_no_sizzle() {
    let mut eq = ParametricEq::new();
    eq.set_bands(vec![EqBand::peaking(500.0, 6.0, 1.0)]);

    let output = process_with_parameter_updates(
        &mut eq,
        |eq, idx| {
            // Sweep frequency from 500Hz to 2000Hz
            let freq = 500.0 + (idx as f32 / 20.0) * 1500.0;
            eq.set_bands(vec![EqBand::peaking(freq.min(2000.0), 6.0, 1.0)]);
        },
        20,
        1024,
    );

    let threshold = get_sizzle_threshold();
    let (max_deriv, _avg_deriv, discontinuities) = analyze_for_discontinuities(&output, threshold);

    assert!(
        discontinuities == 0,
        "ParametricEq frequency sweep produced {} discontinuities. Max derivative: {:.6}",
        discontinuities, max_deriv
    );
}

#[test]
fn test_parametric_eq_add_remove_bands_no_sizzle() {
    let mut eq = ParametricEq::new();
    eq.set_bands(vec![]);

    let output = process_with_parameter_updates(
        &mut eq,
        |eq, idx| {
            // Alternate between 0, 1, 2, 3 bands
            let num_bands = idx % 4;
            let bands: Vec<EqBand> = (0..num_bands)
                .map(|i| EqBand::peaking(500.0 * (i + 1) as f32, 3.0, 1.0))
                .collect();
            eq.set_bands(bands);
        },
        20,
        1024,
    );

    let threshold = get_sizzle_threshold();
    let (max_deriv, _avg_deriv, discontinuities) = analyze_for_discontinuities(&output, threshold);

    assert!(
        discontinuities == 0,
        "ParametricEq add/remove bands produced {} discontinuities. Max derivative: {:.6}",
        discontinuities, max_deriv
    );
}

// ============================================================================
// GRAPHIC EQ TESTS
// ============================================================================

#[test]
fn test_graphic_eq_single_band_update_no_sizzle() {
    let mut eq = GraphicEq::new_10_band();

    let output = process_with_parameter_updates(
        &mut eq,
        |eq, idx| {
            // Update band 5 (1kHz) gradually from -6dB to +6dB
            let gain = -6.0 + (idx as f32 / 10.0) * 12.0;
            eq.set_band_gain(5, gain.min(6.0));
        },
        20,
        1024,
    );

    let threshold = get_sizzle_threshold();
    let (max_deriv, _avg_deriv, discontinuities) = analyze_for_discontinuities(&output, threshold);

    println!(
        "GraphicEq single band: max_deriv={:.6}, discontinuities={}, threshold={:.6}",
        max_deriv, discontinuities, threshold
    );

    assert!(
        discontinuities == 0,
        "GraphicEq single band update produced {} discontinuities (sizzle). Max derivative: {:.6}",
        discontinuities, max_deriv
    );
}

#[test]
fn test_graphic_eq_preset_change_no_sizzle() {
    let mut eq = GraphicEq::new_10_band();
    eq.set_preset(soul_audio::effects::GraphicEqPreset::Flat);

    let presets = [
        soul_audio::effects::GraphicEqPreset::Flat,
        soul_audio::effects::GraphicEqPreset::BassBoost,
        soul_audio::effects::GraphicEqPreset::VShape,
        soul_audio::effects::GraphicEqPreset::Rock,
        soul_audio::effects::GraphicEqPreset::Flat,
    ];

    let output = process_with_parameter_updates(
        &mut eq,
        |eq, idx| {
            eq.set_preset(presets[idx % presets.len()]);
        },
        20,
        1024,
    );

    let threshold = get_sizzle_threshold();
    let (max_deriv, _avg_deriv, discontinuities) = analyze_for_discontinuities(&output, threshold);

    println!(
        "GraphicEq preset change: max_deriv={:.6}, discontinuities={}, threshold={:.6}",
        max_deriv, discontinuities, threshold
    );

    assert!(
        discontinuities == 0,
        "GraphicEq preset change produced {} discontinuities (sizzle). Max derivative: {:.6}",
        discontinuities, max_deriv
    );
}

#[test]
fn test_graphic_eq_all_bands_update_no_sizzle() {
    let mut eq = GraphicEq::new_10_band();

    let output = process_with_parameter_updates(
        &mut eq,
        |eq, idx| {
            // Update all bands simultaneously
            let gains: [f32; 10] = std::array::from_fn(|i| {
                let base = (idx as f32 / 20.0) * 6.0; // 0 to 6dB
                let variation = (i as f32 - 5.0) * 0.5; // Band-dependent variation
                (base + variation).clamp(-12.0, 12.0)
            });
            eq.set_gains_10(gains);
        },
        20,
        1024,
    );

    let threshold = get_sizzle_threshold();
    let (max_deriv, _avg_deriv, discontinuities) = analyze_for_discontinuities(&output, threshold);

    println!(
        "GraphicEq all bands: max_deriv={:.6}, discontinuities={}, threshold={:.6}",
        max_deriv, discontinuities, threshold
    );

    assert!(
        discontinuities == 0,
        "GraphicEq all bands update produced {} discontinuities (sizzle). Max derivative: {:.6}",
        discontinuities, max_deriv
    );
}

// ============================================================================
// COMPRESSOR TESTS
// ============================================================================

#[test]
fn test_compressor_threshold_update_no_sizzle() {
    let mut comp = Compressor::new();

    let output = process_with_parameter_updates(
        &mut comp,
        |comp, idx| {
            // Sweep threshold from -30dB to -10dB
            let threshold = -30.0 + (idx as f32 / 20.0) * 20.0;
            comp.set_threshold(threshold.min(-10.0));
        },
        20,
        1024,
    );

    let threshold = get_sizzle_threshold();
    let (max_deriv, _avg_deriv, discontinuities) = analyze_for_discontinuities(&output, threshold);

    println!(
        "Compressor threshold: max_deriv={:.6}, discontinuities={}, threshold={:.6}",
        max_deriv, discontinuities, threshold
    );

    assert!(
        discontinuities == 0,
        "Compressor threshold update produced {} discontinuities (sizzle). Max derivative: {:.6}",
        discontinuities, max_deriv
    );
}

#[test]
fn test_compressor_makeup_gain_update_no_sizzle() {
    let mut comp = Compressor::new();
    comp.set_threshold(-20.0);

    let output = process_with_parameter_updates(
        &mut comp,
        |comp, idx| {
            // Sweep makeup gain from 0dB to 12dB
            let gain = (idx as f32 / 20.0) * 12.0;
            comp.set_makeup_gain(gain.min(12.0));
        },
        20,
        1024,
    );

    let threshold = get_sizzle_threshold();
    let (max_deriv, _avg_deriv, discontinuities) = analyze_for_discontinuities(&output, threshold);

    println!(
        "Compressor makeup gain: max_deriv={:.6}, discontinuities={}, threshold={:.6}",
        max_deriv, discontinuities, threshold
    );

    assert!(
        discontinuities == 0,
        "Compressor makeup gain update produced {} discontinuities (sizzle). Max derivative: {:.6}",
        discontinuities, max_deriv
    );
}

#[test]
fn test_compressor_ratio_update_no_sizzle() {
    let mut comp = Compressor::new();
    comp.set_threshold(-20.0);

    let output = process_with_parameter_updates(
        &mut comp,
        |comp, idx| {
            // Sweep ratio from 2:1 to 10:1
            let ratio = 2.0 + (idx as f32 / 20.0) * 8.0;
            comp.set_ratio(ratio.min(10.0));
        },
        20,
        1024,
    );

    let threshold = get_sizzle_threshold();
    let (max_deriv, _avg_deriv, discontinuities) = analyze_for_discontinuities(&output, threshold);

    println!(
        "Compressor ratio: max_deriv={:.6}, discontinuities={}, threshold={:.6}",
        max_deriv, discontinuities, threshold
    );

    assert!(
        discontinuities == 0,
        "Compressor ratio update produced {} discontinuities (sizzle). Max derivative: {:.6}",
        discontinuities, max_deriv
    );
}

#[test]
fn test_compressor_settings_update_no_sizzle() {
    let mut comp = Compressor::new();

    let output = process_with_parameter_updates(
        &mut comp,
        |comp, idx| {
            // Update full settings object
            let settings = CompressorSettings {
                threshold_db: -20.0 - (idx as f32 % 10.0),
                ratio: 2.0 + (idx as f32 % 8.0),
                attack_ms: 5.0,
                release_ms: 50.0,
                knee_db: 6.0,
                makeup_gain_db: (idx as f32 / 20.0) * 6.0,
            };
            comp.set_settings(settings);
        },
        20,
        1024,
    );

    let threshold = get_sizzle_threshold();
    let (max_deriv, _avg_deriv, discontinuities) = analyze_for_discontinuities(&output, threshold);

    println!(
        "Compressor settings: max_deriv={:.6}, discontinuities={}, threshold={:.6}",
        max_deriv, discontinuities, threshold
    );

    assert!(
        discontinuities == 0,
        "Compressor settings update produced {} discontinuities (sizzle). Max derivative: {:.6}",
        discontinuities, max_deriv
    );
}

// ============================================================================
// LIMITER TESTS
// ============================================================================

#[test]
fn test_limiter_threshold_update_no_sizzle() {
    let mut limiter = Limiter::new();

    let output = process_with_parameter_updates(
        &mut limiter,
        |limiter, idx| {
            // Sweep threshold from -6dB to 0dB
            let threshold = -6.0 + (idx as f32 / 20.0) * 6.0;
            limiter.set_threshold(threshold.min(0.0));
        },
        20,
        1024,
    );

    let threshold = get_sizzle_threshold();
    let (max_deriv, _avg_deriv, discontinuities) = analyze_for_discontinuities(&output, threshold);

    println!(
        "Limiter threshold: max_deriv={:.6}, discontinuities={}, threshold={:.6}",
        max_deriv, discontinuities, threshold
    );

    assert!(
        discontinuities == 0,
        "Limiter threshold update produced {} discontinuities (sizzle). Max derivative: {:.6}",
        discontinuities, max_deriv
    );
}


#[test]
fn test_limiter_release_update_no_sizzle() {
    let mut limiter = Limiter::new();
    limiter.set_threshold(-3.0);

    let output = process_with_parameter_updates(
        &mut limiter,
        |limiter, idx| {
            // Sweep release from 50ms to 200ms
            let release = 50.0 + (idx as f32 / 20.0) * 150.0;
            limiter.set_release(release.min(200.0));
        },
        20,
        1024,
    );

    let threshold = get_sizzle_threshold();
    let (max_deriv, _avg_deriv, discontinuities) = analyze_for_discontinuities(&output, threshold);

    println!(
        "Limiter release: max_deriv={:.6}, discontinuities={}, threshold={:.6}",
        max_deriv, discontinuities, threshold
    );

    assert!(
        discontinuities == 0,
        "Limiter release update produced {} discontinuities (sizzle). Max derivative: {:.6}",
        discontinuities, max_deriv
    );
}

// ============================================================================
// STEREO ENHANCER TESTS
// ============================================================================

#[test]
fn test_stereo_enhancer_width_update_no_sizzle() {
    let mut enhancer = StereoEnhancer::new();

    let output = process_with_parameter_updates(
        &mut enhancer,
        |enhancer, idx| {
            // Sweep width from 0.5 to 2.0
            let width = 0.5 + (idx as f32 / 20.0) * 1.5;
            enhancer.set_width(width.min(2.0));
        },
        20,
        1024,
    );

    let threshold = get_sizzle_threshold();
    let (max_deriv, _avg_deriv, discontinuities) = analyze_for_discontinuities(&output, threshold);

    println!(
        "StereoEnhancer width: max_deriv={:.6}, discontinuities={}, threshold={:.6}",
        max_deriv, discontinuities, threshold
    );

    assert!(
        discontinuities == 0,
        "StereoEnhancer width update produced {} discontinuities (sizzle). Max derivative: {:.6}",
        discontinuities, max_deriv
    );
}

#[test]
fn test_stereo_enhancer_mid_side_update_no_sizzle() {
    let mut enhancer = StereoEnhancer::new();

    let output = process_with_parameter_updates(
        &mut enhancer,
        |enhancer, idx| {
            // Sweep mid gain from -6dB to +6dB, side gain inverse
            let mid_gain = -6.0 + (idx as f32 / 20.0) * 12.0;
            let side_gain = 6.0 - (idx as f32 / 20.0) * 12.0;
            enhancer.set_mid_gain_db(mid_gain.clamp(-12.0, 12.0));
            enhancer.set_side_gain_db(side_gain.clamp(-12.0, 12.0));
        },
        20,
        1024,
    );

    let threshold = get_sizzle_threshold();
    let (max_deriv, _avg_deriv, discontinuities) = analyze_for_discontinuities(&output, threshold);

    println!(
        "StereoEnhancer mid/side: max_deriv={:.6}, discontinuities={}, threshold={:.6}",
        max_deriv, discontinuities, threshold
    );

    assert!(
        discontinuities == 0,
        "StereoEnhancer mid/side update produced {} discontinuities (sizzle). Max derivative: {:.6}",
        discontinuities, max_deriv
    );
}

#[test]
fn test_stereo_enhancer_balance_update_no_sizzle() {
    let mut enhancer = StereoEnhancer::new();

    let output = process_with_parameter_updates(
        &mut enhancer,
        |enhancer, idx| {
            // Sweep balance from -1.0 (left) to +1.0 (right)
            let balance = -1.0 + (idx as f32 / 10.0);
            enhancer.set_balance(balance.clamp(-1.0, 1.0));
        },
        20,
        1024,
    );

    let threshold = get_sizzle_threshold();
    let (max_deriv, _avg_deriv, discontinuities) = analyze_for_discontinuities(&output, threshold);

    println!(
        "StereoEnhancer balance: max_deriv={:.6}, discontinuities={}, threshold={:.6}",
        max_deriv, discontinuities, threshold
    );

    assert!(
        discontinuities == 0,
        "StereoEnhancer balance update produced {} discontinuities (sizzle). Max derivative: {:.6}",
        discontinuities, max_deriv
    );
}

#[test]
fn test_stereo_enhancer_all_params_update_no_sizzle() {
    let mut enhancer = StereoEnhancer::new();

    let output = process_with_parameter_updates(
        &mut enhancer,
        |enhancer, idx| {
            // Update all parameters at once
            enhancer.set_width(1.0 + (idx as f32 / 20.0) * 0.5);
            enhancer.set_mid_gain_db((idx as f32 % 6.0) - 3.0);
            enhancer.set_side_gain_db(3.0 - (idx as f32 % 6.0));
            enhancer.set_balance((idx as f32 / 20.0) - 0.5);
        },
        20,
        1024,
    );

    let threshold = get_sizzle_threshold();
    let (max_deriv, _avg_deriv, discontinuities) = analyze_for_discontinuities(&output, threshold);

    println!(
        "StereoEnhancer all params: max_deriv={:.6}, discontinuities={}, threshold={:.6}",
        max_deriv, discontinuities, threshold
    );

    assert!(
        discontinuities == 0,
        "StereoEnhancer all params update produced {} discontinuities (sizzle). Max derivative: {:.6}",
        discontinuities, max_deriv
    );
}

// ============================================================================
// CROSSFEED TESTS
// ============================================================================

#[test]
fn test_crossfeed_level_update_no_sizzle() {
    let mut crossfeed = Crossfeed::new();
    crossfeed.set_preset(CrossfeedPreset::Natural);

    let output = process_with_parameter_updates(
        &mut crossfeed,
        |crossfeed, idx| {
            // Sweep level from -6dB to -1dB
            let level = -6.0 + (idx as f32 / 20.0) * 5.0;
            crossfeed.set_level_db(level.clamp(-6.0, -1.0));
        },
        20,
        1024,
    );

    let threshold = get_sizzle_threshold();
    let (max_deriv, _avg_deriv, discontinuities) = analyze_for_discontinuities(&output, threshold);

    println!(
        "Crossfeed level: max_deriv={:.6}, discontinuities={}, threshold={:.6}",
        max_deriv, discontinuities, threshold
    );

    assert!(
        discontinuities == 0,
        "Crossfeed level update produced {} discontinuities (sizzle). Max derivative: {:.6}",
        discontinuities, max_deriv
    );
}

#[test]
fn test_crossfeed_cutoff_update_no_sizzle() {
    let mut crossfeed = Crossfeed::new();
    crossfeed.set_preset(CrossfeedPreset::Natural);

    let output = process_with_parameter_updates(
        &mut crossfeed,
        |crossfeed, idx| {
            // Sweep cutoff from 500Hz to 1500Hz
            let cutoff = 500.0 + (idx as f32 / 20.0) * 1000.0;
            crossfeed.set_cutoff_hz(cutoff.clamp(500.0, 1500.0));
        },
        20,
        1024,
    );

    let threshold = get_sizzle_threshold();
    let (max_deriv, _avg_deriv, discontinuities) = analyze_for_discontinuities(&output, threshold);

    println!(
        "Crossfeed cutoff: max_deriv={:.6}, discontinuities={}, threshold={:.6}",
        max_deriv, discontinuities, threshold
    );

    assert!(
        discontinuities == 0,
        "Crossfeed cutoff update produced {} discontinuities (sizzle). Max derivative: {:.6}",
        discontinuities, max_deriv
    );
}

#[test]
fn test_crossfeed_preset_change_no_sizzle() {
    let mut crossfeed = Crossfeed::new();

    let presets = [
        CrossfeedPreset::Relaxed,
        CrossfeedPreset::Natural,
        CrossfeedPreset::Meier,
        CrossfeedPreset::Relaxed,
    ];

    let output = process_with_parameter_updates(
        &mut crossfeed,
        |crossfeed, idx| {
            crossfeed.set_preset(presets[idx % presets.len()]);
        },
        20,
        1024,
    );

    let threshold = get_sizzle_threshold();
    let (max_deriv, _avg_deriv, discontinuities) = analyze_for_discontinuities(&output, threshold);

    println!(
        "Crossfeed preset: max_deriv={:.6}, discontinuities={}, threshold={:.6}",
        max_deriv, discontinuities, threshold
    );

    assert!(
        discontinuities == 0,
        "Crossfeed preset change produced {} discontinuities (sizzle). Max derivative: {:.6}",
        discontinuities, max_deriv
    );
}

// ============================================================================
// RAPID UPDATE STRESS TESTS
// ============================================================================

/// These tests simulate aggressive UI slider movement (rapid parameter changes)

#[test]
fn test_graphic_eq_rapid_updates_no_sizzle() {
    let mut eq = GraphicEq::new_10_band();

    // 100 updates with only 256 samples between each (~5.8ms at 44.1kHz)
    // This simulates dragging a slider quickly
    let output = process_with_parameter_updates(
        &mut eq,
        |eq, idx| {
            let gain = (idx as f32 * 0.5).sin() * 6.0; // Oscillating gain
            eq.set_band_gain(5, gain);
        },
        100,
        256,
    );

    let threshold = get_sizzle_threshold();
    let (max_deriv, _avg_deriv, discontinuities) = analyze_for_discontinuities(&output, threshold);

    println!(
        "GraphicEq rapid: max_deriv={:.6}, discontinuities={}, threshold={:.6}",
        max_deriv, discontinuities, threshold
    );

    assert!(
        discontinuities == 0,
        "GraphicEq rapid updates produced {} discontinuities (sizzle). Max derivative: {:.6}",
        discontinuities, max_deriv
    );
}

#[test]
fn test_compressor_rapid_makeup_gain_no_sizzle() {
    let mut comp = Compressor::new();
    comp.set_threshold(-20.0);

    let output = process_with_parameter_updates(
        &mut comp,
        |comp, idx| {
            let gain = (idx as f32 * 0.3).sin() * 6.0 + 6.0; // 0 to 12dB oscillating
            comp.set_makeup_gain(gain.clamp(0.0, 12.0));
        },
        100,
        256,
    );

    let threshold = get_sizzle_threshold();
    let (max_deriv, _avg_deriv, discontinuities) = analyze_for_discontinuities(&output, threshold);

    println!(
        "Compressor rapid makeup: max_deriv={:.6}, discontinuities={}, threshold={:.6}",
        max_deriv, discontinuities, threshold
    );

    assert!(
        discontinuities == 0,
        "Compressor rapid makeup gain produced {} discontinuities (sizzle). Max derivative: {:.6}",
        discontinuities, max_deriv
    );
}

#[test]
fn test_stereo_enhancer_rapid_width_no_sizzle() {
    let mut enhancer = StereoEnhancer::new();

    let output = process_with_parameter_updates(
        &mut enhancer,
        |enhancer, idx| {
            let width = 1.0 + (idx as f32 * 0.2).sin() * 0.5; // 0.5 to 1.5 oscillating
            enhancer.set_width(width);
        },
        100,
        256,
    );

    let threshold = get_sizzle_threshold();
    let (max_deriv, _avg_deriv, discontinuities) = analyze_for_discontinuities(&output, threshold);

    println!(
        "StereoEnhancer rapid width: max_deriv={:.6}, discontinuities={}, threshold={:.6}",
        max_deriv, discontinuities, threshold
    );

    assert!(
        discontinuities == 0,
        "StereoEnhancer rapid width produced {} discontinuities (sizzle). Max derivative: {:.6}",
        discontinuities, max_deriv
    );
}

#[test]
fn test_crossfeed_rapid_level_no_sizzle() {
    let mut crossfeed = Crossfeed::new();
    crossfeed.set_preset(CrossfeedPreset::Natural);

    let output = process_with_parameter_updates(
        &mut crossfeed,
        |crossfeed, idx| {
            let level = -4.5 + (idx as f32 * 0.2).sin() * 1.5; // -6 to -3 dB
            crossfeed.set_level_db(level.clamp(-6.0, -1.0));
        },
        100,
        256,
    );

    let threshold = get_sizzle_threshold();
    let (max_deriv, _avg_deriv, discontinuities) = analyze_for_discontinuities(&output, threshold);

    println!(
        "Crossfeed rapid level: max_deriv={:.6}, discontinuities={}, threshold={:.6}",
        max_deriv, discontinuities, threshold
    );

    assert!(
        discontinuities == 0,
        "Crossfeed rapid level produced {} discontinuities (sizzle). Max derivative: {:.6}",
        discontinuities, max_deriv
    );
}

// ============================================================================
// SUMMARY TEST - Runs all components and reports which ones fail
// ============================================================================

#[test]
fn test_all_components_summary() {
    let threshold = get_sizzle_threshold();
    let mut failures = Vec::new();

    // ParametricEq
    {
        let mut eq = ParametricEq::new();
        eq.set_bands(vec![EqBand::peaking(1000.0, 0.0, 1.0)]);
        let output = process_with_parameter_updates(
            &mut eq,
            |eq, idx| {
                let gain = -6.0 + (idx as f32 / 10.0) * 12.0;
                eq.set_bands(vec![EqBand::peaking(1000.0, gain.min(6.0), 1.0)]);
            },
            20, 1024,
        );
        let (max_deriv, _, discontinuities) = analyze_for_discontinuities(&output, threshold);
        if discontinuities > 0 {
            failures.push(format!("ParametricEq: {} discontinuities, max_deriv={:.6}", discontinuities, max_deriv));
        }
    }

    // GraphicEq
    {
        let mut eq = GraphicEq::new_10_band();
        let output = process_with_parameter_updates(
            &mut eq,
            |eq, idx| {
                let gain = -6.0 + (idx as f32 / 10.0) * 12.0;
                eq.set_band_gain(5, gain.min(6.0));
            },
            20, 1024,
        );
        let (max_deriv, _, discontinuities) = analyze_for_discontinuities(&output, threshold);
        if discontinuities > 0 {
            failures.push(format!("GraphicEq: {} discontinuities, max_deriv={:.6}", discontinuities, max_deriv));
        }
    }

    // Compressor
    {
        let mut comp = Compressor::new();
        comp.set_threshold(-20.0);
        let output = process_with_parameter_updates(
            &mut comp,
            |comp, idx| {
                let gain = (idx as f32 / 20.0) * 12.0;
                comp.set_makeup_gain(gain.min(12.0));
            },
            20, 1024,
        );
        let (max_deriv, _, discontinuities) = analyze_for_discontinuities(&output, threshold);
        if discontinuities > 0 {
            failures.push(format!("Compressor: {} discontinuities, max_deriv={:.6}", discontinuities, max_deriv));
        }
    }

    // Limiter
    {
        let mut limiter = Limiter::new();
        let output = process_with_parameter_updates(
            &mut limiter,
            |limiter, idx| {
                let threshold = -6.0 + (idx as f32 / 20.0) * 6.0;
                limiter.set_threshold(threshold.min(0.0));
            },
            20, 1024,
        );
        let (max_deriv, _, discontinuities) = analyze_for_discontinuities(&output, threshold);
        if discontinuities > 0 {
            failures.push(format!("Limiter: {} discontinuities, max_deriv={:.6}", discontinuities, max_deriv));
        }
    }

    // StereoEnhancer
    {
        let mut enhancer = StereoEnhancer::new();
        let output = process_with_parameter_updates(
            &mut enhancer,
            |enhancer, idx| {
                let width = 0.5 + (idx as f32 / 20.0) * 1.5;
                enhancer.set_width(width.min(2.0));
            },
            20, 1024,
        );
        let (max_deriv, _, discontinuities) = analyze_for_discontinuities(&output, threshold);
        if discontinuities > 0 {
            failures.push(format!("StereoEnhancer: {} discontinuities, max_deriv={:.6}", discontinuities, max_deriv));
        }
    }

    // Crossfeed
    {
        let mut crossfeed = Crossfeed::new();
        crossfeed.set_preset(CrossfeedPreset::Natural);
        let output = process_with_parameter_updates(
            &mut crossfeed,
            |crossfeed, idx| {
                let level = -6.0 + (idx as f32 / 20.0) * 5.0;
                crossfeed.set_level_db(level.clamp(-6.0, -1.0));
            },
            20, 1024,
        );
        let (max_deriv, _, discontinuities) = analyze_for_discontinuities(&output, threshold);
        if discontinuities > 0 {
            failures.push(format!("Crossfeed: {} discontinuities, max_deriv={:.6}", discontinuities, max_deriv));
        }
    }

    if !failures.is_empty() {
        println!("\n=== SIZZLE TEST FAILURES ===");
        for failure in &failures {
            println!("  FAIL: {}", failure);
        }
        println!("============================\n");
        panic!(
            "{} out of 6 components failed sizzle tests:\n{}",
            failures.len(),
            failures.join("\n")
        );
    } else {
        println!("\n=== ALL COMPONENTS PASSED SIZZLE TESTS ===\n");
    }
}
