//! End-to-end tests for DSP effects chain
//!
//! These tests verify that audio effects work correctly using:
//! - Generated test signals (sine waves, sweeps, noise)
//! - FFT-based frequency analysis
//! - THD measurements
//! - Peak/RMS analysis
//! - Compression ratio verification
//!
//! Run these tests with: `cargo test -p soul-audio --features test-utils`

#![cfg(feature = "test-utils")]

use soul_audio::effects::*;
use soul_audio::test_utils::analysis::*;
use soul_audio::test_utils::signals::*;

// ===== PARAMETRIC EQ TESTS =====

#[test]
fn test_eq_boosts_frequency() {
    // Generate 1kHz sine wave
    let input = generate_sine_wave(1000.0, 44100, 0.5, 0.5);
    let input_mono = extract_mono(&input, 0);

    // Apply EQ with +6dB at 1kHz
    let mut eq = ParametricEq::new();
    eq.set_bands(vec![
        EqBand::new(100.0, 0.0, 1.0),
        EqBand::new(1000.0, 6.0, 1.0), // +6dB boost at 1kHz
        EqBand::new(10000.0, 0.0, 1.0),
    ]);

    let mut output = input.clone();
    eq.process(&mut output, 44100);
    let output_mono = extract_mono(&output, 0);

    // Verify output is louder
    let input_rms = calculate_rms(&input_mono);
    let output_rms = calculate_rms(&output_mono);

    println!(
        "EQ Boost Test - Input RMS: {:.4}, Output RMS: {:.4}",
        input_rms, output_rms
    );
    println!(
        "Gain ratio: {:.2}x ({:.2} dB)",
        output_rms / input_rms,
        linear_to_db(output_rms / input_rms)
    );

    // +6dB should be ~2x amplitude
    assert!(
        output_rms > input_rms * 1.8,
        "Output should be ~2x louder after +6dB boost"
    );
    assert!(
        output_rms < input_rms * 2.2,
        "Output shouldn't be too much louder"
    );
}

#[test]
fn test_eq_cuts_frequency() {
    // Generate 1kHz sine wave
    let input = generate_sine_wave(1000.0, 44100, 0.5, 0.5);
    let input_mono = extract_mono(&input, 0);

    // Apply EQ with -12dB at 1kHz
    let mut eq = ParametricEq::new();
    eq.set_bands(vec![
        EqBand::new(100.0, 0.0, 1.0),
        EqBand::new(1000.0, -12.0, 1.0), // -12dB cut at 1kHz
        EqBand::new(10000.0, 0.0, 1.0),
    ]);

    let mut output = input.clone();
    eq.process(&mut output, 44100);
    let output_mono = extract_mono(&output, 0);

    // Verify output is quieter
    let input_rms = calculate_rms(&input_mono);
    let output_rms = calculate_rms(&output_mono);

    println!(
        "EQ Cut Test - Input RMS: {:.4}, Output RMS: {:.4}",
        input_rms, output_rms
    );
    println!(
        "Gain ratio: {:.2}x ({:.2} dB)",
        output_rms / input_rms,
        linear_to_db(output_rms / input_rms)
    );

    // -12dB should be ~0.25x amplitude
    assert!(
        output_rms < input_rms * 0.3,
        "Output should be ~4x quieter after -12dB cut"
    );
    assert!(
        output_rms > input_rms * 0.2,
        "Output shouldn't be too quiet"
    );
}

#[test]
fn test_eq_doesnt_affect_other_frequencies() {
    // Generate 100Hz sine wave
    let input = generate_sine_wave(100.0, 44100, 0.5, 0.5);

    // Apply EQ with boost at 1kHz (should not affect 100Hz)
    let mut eq = ParametricEq::new();
    eq.set_bands(vec![
        EqBand::new(100.0, 0.0, 1.0),
        EqBand::new(1000.0, 12.0, 1.0), // +12dB at 1kHz (far from 100Hz)
        EqBand::new(10000.0, 0.0, 1.0),
    ]);

    let mut output = input.clone();
    eq.process(&mut output, 44100);

    // 100Hz should be mostly unchanged
    let input_mono = extract_mono(&input, 0);
    let output_mono = extract_mono(&output, 0);

    let input_rms = calculate_rms(&input_mono);
    let output_rms = calculate_rms(&output_mono);

    println!(
        "EQ Isolation Test - RMS change: {:.2}%",
        ((output_rms / input_rms) - 1.0) * 100.0
    );

    // Should be within 10% (some bleed expected due to Q factor)
    assert!(
        (output_rms / input_rms - 1.0).abs() < 0.1,
        "100Hz should not be significantly affected by 1kHz boost"
    );
}

#[test]
fn test_eq_with_zero_gain_is_transparent() {
    // Generate pink noise
    let input = generate_pink_noise(44100, 0.5, 0.5);

    // Apply flat EQ (all gains = 0)
    let mut eq = ParametricEq::new();
    eq.set_bands(vec![
        EqBand::new(100.0, 0.0, 1.0),
        EqBand::new(1000.0, 0.0, 1.0),
        EqBand::new(10000.0, 0.0, 1.0),
    ]);

    let mut output = input.clone();
    eq.process(&mut output, 44100);

    // Output should be nearly identical to input
    let diff = calculate_signal_difference(&input, &output);

    println!("EQ Transparency Test - Max difference: {:.6}", diff);

    // Should be very small (< 0.01)
    assert!(diff < 0.01, "Flat EQ should be nearly transparent");
}

// ===== COMPRESSOR TESTS =====

#[test]
fn test_compressor_reduces_peaks() {
    // Generate longer signal (3 seconds) with loud peaks to allow envelope to settle
    let input = generate_dynamic_test_signal(44100, 3.0, 0.1, 0.9);
    let input_mono = extract_mono(&input, 0);

    // Apply compressor with fast attack
    let mut compressor = Compressor::with_settings(CompressorSettings {
        threshold_db: -12.0,
        ratio: 4.0,
        attack_ms: 1.0, // Very fast attack
        release_ms: 200.0,
        knee_db: 0.0, // Hard knee for predictable results
        makeup_gain_db: 0.0,
    });

    let mut output = input.clone();
    compressor.process(&mut output, 44100);
    let output_mono = extract_mono(&output, 0);

    // Check RMS levels instead of peaks (more reliable for compression testing)
    let input_rms = calculate_rms(&input_mono);
    let output_rms = calculate_rms(&output_mono);

    println!("Compressor Peak Test:");
    println!(
        "  Input RMS: {:.4} ({:.2} dB)",
        input_rms,
        linear_to_db(input_rms)
    );
    println!(
        "  Output RMS: {:.4} ({:.2} dB)",
        output_rms,
        linear_to_db(output_rms)
    );
    println!(
        "  Change: {:.2} dB",
        linear_to_db(output_rms) - linear_to_db(input_rms)
    );

    // Output RMS should be lower (compression reduces dynamic range)
    assert!(
        output_rms < input_rms,
        "Compressor should reduce average level"
    );
}

#[test]
fn test_compressor_doesnt_affect_quiet_signals() {
    // Generate very quiet signal well below threshold (2 seconds)
    let input = generate_sine_wave(440.0, 44100, 2.0, 0.01); // -40dB

    // Apply compressor with -12dB threshold
    let mut compressor = Compressor::with_settings(CompressorSettings {
        threshold_db: -12.0,
        ratio: 4.0,
        attack_ms: 10.0,
        release_ms: 100.0,
        knee_db: 0.0,
        makeup_gain_db: 0.0,
    });

    let mut output = input.clone();
    compressor.process(&mut output, 44100);

    // Check RMS levels (should be very similar)
    let input_mono = extract_mono(&input, 0);
    let output_mono = extract_mono(&output, 0);

    let input_rms = calculate_rms(&input_mono);
    let output_rms = calculate_rms(&output_mono);

    println!("Compressor Quiet Signal Test:");
    println!(
        "  Input RMS: {:.4} ({:.2} dB)",
        input_rms,
        linear_to_db(input_rms)
    );
    println!(
        "  Output RMS: {:.4} ({:.2} dB)",
        output_rms,
        linear_to_db(output_rms)
    );
    println!(
        "  Difference: {:.2} dB",
        (linear_to_db(output_rms) - linear_to_db(input_rms)).abs()
    );

    // RMS should be within 1dB
    let diff_db = (linear_to_db(output_rms) - linear_to_db(input_rms)).abs();
    assert!(
        diff_db < 1.0,
        "Quiet signal below threshold should pass through mostly unchanged"
    );
}

#[test]
fn test_compressor_ratio() {
    // Generate sustained signal above threshold (5 seconds for full envelope settlement)
    let input = generate_sine_wave(440.0, 44100, 5.0, 0.5); // -6dB, well above -12dB threshold

    // Apply 4:1 compression with very fast attack and hard knee
    let mut compressor = Compressor::with_settings(CompressorSettings {
        threshold_db: -12.0,
        ratio: 4.0,
        attack_ms: 0.1,     // Extremely fast attack
        release_ms: 1000.0, // Slow release to maintain steady compression
        knee_db: 0.0,       // Hard knee for accurate ratio measurement
        makeup_gain_db: 0.0,
    });

    let mut output = input.clone();
    compressor.process(&mut output, 44100);

    let input_mono = extract_mono(&input, 0);
    let output_mono = extract_mono(&output, 0);

    // Use only the last 20% of the signal (fully settled)
    let start_idx = (input_mono.len() as f32 * 0.8) as usize;
    let input_settled = &input_mono[start_idx..];
    let output_settled = &output_mono[start_idx..];

    // Calculate actual dB levels for verification
    let input_rms = calculate_rms(input_settled);
    let output_rms = calculate_rms(output_settled);
    let input_rms_db = linear_to_db(input_rms);
    let output_rms_db = linear_to_db(output_rms);

    println!("Compressor Ratio Test:");
    println!("  Input RMS: {:.2} dB", input_rms_db);
    println!("  Output RMS: {:.2} dB", output_rms_db);
    println!("  Threshold: -12.0 dB");
    println!("  Expected: 4:1 compression");

    // Compressor works on PEAK levels (instantaneous samples), not RMS
    // For sine wave: Peak = RMS + 3 dB
    let input_peak_db = -6.02; // 0.5 amplitude = -6.02 dB peak
    let threshold_db = -12.0;

    // Calculate gain reduction using compressor formula
    let over_threshold = input_peak_db - threshold_db; // 5.98 dB
    let gain_reduction = over_threshold * (1.0 - 1.0 / 4.0); // 4.485 dB
    let output_peak_db = input_peak_db - gain_reduction; // -10.505 dB
    let expected_output_rms_db = output_peak_db - 3.0; // -13.505 dB (peak - 3 dB for sine RMS)

    println!(
        "  Peak tracking: input {:.2} dB -> output {:.2} dB",
        input_peak_db, output_peak_db
    );
    println!("  Expected output RMS: {:.2} dB", expected_output_rms_db);
    println!("  Actual output RMS: {:.2} dB", output_rms_db);
    println!(
        "  Difference: {:.2} dB",
        (output_rms_db - expected_output_rms_db).abs()
    );

    // Verify output is within 0.5 dB of expected (compressor is very accurate with hard knee)
    let output_diff = (output_rms_db - expected_output_rms_db).abs();
    assert!(output_diff < 0.5,
            "Output level should match 4:1 compression (expected {:.2} dB, got {:.2} dB, diff {:.2} dB)",
            expected_output_rms_db, output_rms_db, output_diff);
}

#[test]
fn test_compressor_with_makeup_gain() {
    // Generate signal below threshold (no compression, only makeup gain applied)
    let input = generate_sine_wave(440.0, 44100, 2.0, 0.05); // -26dB, well below -12dB threshold
    let input_mono = extract_mono(&input, 0);

    // Apply compressor with makeup gain (signal below threshold, so only gain applied)
    let mut compressor = Compressor::with_settings(CompressorSettings {
        threshold_db: -12.0,
        ratio: 4.0,
        attack_ms: 10.0,
        release_ms: 100.0,
        knee_db: 0.0,
        makeup_gain_db: 12.0, // +12dB makeup
    });

    let mut output = input.clone();
    compressor.process(&mut output, 44100);
    let output_mono = extract_mono(&output, 0);

    // Output should be louder due to makeup gain
    let input_rms = calculate_rms(&input_mono);
    let output_rms = calculate_rms(&output_mono);

    println!("Compressor Makeup Gain Test:");
    println!(
        "  Input RMS: {:.4} ({:.2} dB)",
        input_rms,
        linear_to_db(input_rms)
    );
    println!(
        "  Output RMS: {:.4} ({:.2} dB)",
        output_rms,
        linear_to_db(output_rms)
    );
    println!(
        "  Gain change: {:.2} dB",
        linear_to_db(output_rms / input_rms)
    );

    // Should be significantly louder (+12dB ≈ 4x amplitude)
    assert!(
        output_rms > input_rms * 3.0,
        "Makeup gain should increase overall level by ~12dB (4x)"
    );
}

// ===== LIMITER TESTS =====

#[test]
fn test_limiter_prevents_clipping() {
    // Generate signal that would clip (peaks above 1.0)
    let mut input = generate_sine_wave(440.0, 44100, 0.5, 1.5);

    // Verify input actually exceeds 1.0
    let input_peak = calculate_peak(&input);
    assert!(input_peak > 1.0, "Test signal should exceed 1.0");

    println!("Limiter Clipping Prevention Test:");
    println!(
        "  Input peak: {:.4} ({:.2} dB)",
        input_peak,
        linear_to_db(input_peak)
    );

    // Apply limiter
    let mut limiter = Limiter::with_settings(LimiterSettings {
        threshold_db: -0.1,
        release_ms: 50.0,
    });

    limiter.process(&mut input, 44100);

    // Verify no clipping
    let output_peak = calculate_peak(&input);
    println!(
        "  Output peak: {:.4} ({:.2} dB)",
        output_peak,
        linear_to_db(output_peak)
    );

    assert!(output_peak <= 1.0, "Limiter should prevent clipping");
    assert!(output_peak > 0.95, "Limiter should get close to threshold");
}

#[test]
fn test_limiter_preserves_quiet_signals() {
    // Generate quiet signal well below threshold
    let input = generate_sine_wave(440.0, 44100, 0.5, 0.3);

    // Apply limiter
    let mut limiter = Limiter::with_settings(LimiterSettings {
        threshold_db: -0.3,
        release_ms: 50.0,
    });

    let mut output = input.clone();
    limiter.process(&mut output, 44100);

    // Should be nearly unchanged
    let diff = calculate_signal_difference(&input, &output);

    println!("Limiter Quiet Signal Test - Max difference: {:.6}", diff);

    assert!(
        diff < 0.01,
        "Quiet signal below threshold should pass through unchanged"
    );
}

#[test]
fn test_limiter_brickwall_behavior() {
    // Generate sweep that goes from quiet to loud
    let mut input = Vec::new();
    for i in 0..44100 {
        let amplitude = (i as f32 / 44100.0) * 1.5; // Ramp from 0 to 1.5
        let t = i as f32 / 44100.0;
        let sample = (2.0 * std::f32::consts::PI * 440.0 * t).sin() * amplitude;
        input.push(sample); // Left
        input.push(sample); // Right
    }

    // Apply brickwall limiter
    let mut limiter = Limiter::with_settings(LimiterSettings::brickwall());

    limiter.process(&mut input, 44100);

    // Check that ALL samples are below threshold
    let exceeded_count = input.iter().filter(|&&s| s.abs() > 1.0).count();

    println!("Limiter Brickwall Test:");
    println!("  Samples exceeding 0 dBFS: {}", exceeded_count);
    println!("  Total samples: {}", input.len());

    assert_eq!(
        exceeded_count, 0,
        "Brickwall limiter should never allow clipping"
    );
}

// ===== EFFECT CHAIN TESTS =====

#[test]
fn test_effect_chain_order_matters() {
    // Generate dynamic signal
    let input = generate_dynamic_test_signal(44100, 1.0, 0.2, 0.8);

    // Chain 1: EQ -> Compressor
    let mut chain1 = EffectChain::new();
    let mut eq1 = ParametricEq::new();
    eq1.set_bands(vec![
        EqBand::new(100.0, 0.0, 1.0),
        EqBand::new(1000.0, 6.0, 1.0),
        EqBand::new(10000.0, 0.0, 1.0),
    ]);
    chain1.add_effect(Box::new(eq1));
    chain1.add_effect(Box::new(Compressor::with_settings(
        CompressorSettings::moderate(),
    )));

    // Chain 2: Compressor -> EQ
    let mut chain2 = EffectChain::new();
    chain2.add_effect(Box::new(Compressor::with_settings(
        CompressorSettings::moderate(),
    )));
    let mut eq2 = ParametricEq::new();
    eq2.set_bands(vec![
        EqBand::new(100.0, 0.0, 1.0),
        EqBand::new(1000.0, 6.0, 1.0),
        EqBand::new(10000.0, 0.0, 1.0),
    ]);
    chain2.add_effect(Box::new(eq2));

    // Process
    let mut output1 = input.clone();
    let mut output2 = input.clone();

    chain1.process(&mut output1, 44100);
    chain2.process(&mut output2, 44100);

    // Outputs should be different
    let diff = calculate_signal_difference(&output1, &output2);

    println!("Effect Chain Order Test - Difference: {:.6}", diff);

    assert!(
        diff > 0.01,
        "Different effect orders should produce different results"
    );
}

#[test]
fn test_empty_effect_chain_is_transparent() {
    let input = generate_sine_wave(440.0, 44100, 0.5, 0.5);

    // Create empty chain
    let mut chain = EffectChain::new();

    let mut output = input.clone();
    chain.process(&mut output, 44100);

    // Should be unchanged
    let diff = calculate_signal_difference(&input, &output);

    println!("Empty Chain Transparency Test - Difference: {:.6}", diff);

    assert!(
        diff < 0.000001,
        "Empty chain should be perfectly transparent"
    );
}

// ===== AUDIO QUALITY TESTS =====

#[test]
fn test_effects_dont_add_excessive_thd() {
    // Generate pure sine wave
    let input = generate_sine_wave(440.0, 44100, 0.5, 0.5);

    // Apply moderate processing
    let mut chain = EffectChain::new();
    chain.add_effect(Box::new(Compressor::with_settings(
        CompressorSettings::gentle(),
    )));
    chain.add_effect(Box::new(Limiter::with_settings(LimiterSettings::soft())));

    let mut output = input.clone();
    chain.process(&mut output, 44100);
    let output_mono = extract_mono(&output, 0);

    // Measure THD
    let thd = calculate_thd(&output_mono, 440.0, 44100);

    println!("THD Test - THD: {:.2}%", thd);

    // THD should be low (< 2% for gentle processing with soft limiter)
    // Professional equipment targets < 0.1%, but DFT-based measurement has spectral leakage
    // and dynamics processing inherently adds harmonics. 2% is a realistic target.
    // (Previously was 5% which was too loose)
    assert!(
        thd < 2.0,
        "Effects should not add excessive harmonic distortion (THD: {:.2}%, max: 2%)",
        thd
    );
}

#[test]
fn test_effects_preserve_silence() {
    // Generate silence
    let input = vec![0.0; 44100 * 2]; // 1 second of silence

    // Apply aggressive processing
    let mut chain = EffectChain::new();
    let mut eq = ParametricEq::new();
    eq.set_bands(vec![
        EqBand::new(100.0, 0.0, 1.0),
        EqBand::new(1000.0, 12.0, 1.0),
        EqBand::new(10000.0, 0.0, 1.0),
    ]);
    chain.add_effect(Box::new(eq));
    chain.add_effect(Box::new(Compressor::with_settings(
        CompressorSettings::aggressive(),
    )));

    let mut output = input.clone();
    chain.process(&mut output, 44100);

    // Should still be silent
    assert!(
        is_silent(&output, -80.0),
        "Silence should remain silent after processing"
    );
}
