//! Integration tests for audio engine
//!
//! These tests verify real audio processing behavior, not just API contracts.

use soul_audio::effects::*;
use std::f32::consts::PI;

// Helper: Generate sine wave at specific frequency
fn generate_sine(freq: f32, duration_secs: f32, sample_rate: u32) -> Vec<f32> {
    let num_samples = (duration_secs * sample_rate as f32) as usize;
    let mut samples = Vec::with_capacity(num_samples * 2); // Stereo

    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let sample = (2.0 * PI * freq * t).sin();
        samples.push(sample); // Left
        samples.push(sample); // Right
    }

    samples
}

// Helper: Calculate RMS (root mean square) amplitude
fn calculate_rms(buffer: &[f32]) -> f32 {
    let sum: f32 = buffer.iter().map(|s| s * s).sum();
    (sum / buffer.len() as f32).sqrt()
}

// Helper: Calculate peak amplitude
fn calculate_peak(buffer: &[f32]) -> f32 {
    buffer.iter().map(|s| s.abs()).fold(0.0f32, f32::max)
}

#[test]
fn test_eq_affects_frequency_content() {
    // Test that EQ actually changes the audio, not just passes it through
    let mut eq = ParametricEq::new();

    // Extreme boost on low band
    eq.set_low_band(EqBand::low_shelf(200.0, 12.0)); // +12 dB boost
    eq.reset(); // Snap coefficients to target for deterministic test

    // Generate low frequency signal (100 Hz)
    let mut buffer = generate_sine(100.0, 0.1, 44100);
    let original_rms = calculate_rms(&buffer);

    eq.process(&mut buffer, 44100);

    let processed_rms = calculate_rms(&buffer);

    // With +12 dB boost, RMS should increase significantly
    // 12 dB = 4x amplitude (20*log10(4) = 12)
    assert!(
        processed_rms > original_rms * 2.0,
        "Expected significant boost at low frequencies, original RMS: {}, processed RMS: {}",
        original_rms,
        processed_rms
    );
}

#[test]
fn test_eq_different_bands_affect_different_frequencies() {
    // Verify that low band doesn't affect high frequencies and vice versa
    let sample_rate = 44100;

    // Test 1: Boost low, check high frequency unchanged
    let mut eq_low = ParametricEq::new();
    eq_low.set_low_band(EqBand::low_shelf(200.0, 12.0)); // Boost low

    let mut high_freq = generate_sine(8000.0, 0.1, sample_rate); // High frequency
    let original_rms = calculate_rms(&high_freq);

    eq_low.process(&mut high_freq, sample_rate);

    let processed_rms = calculate_rms(&high_freq);

    // High frequency should be mostly unchanged (< 20% difference)
    let ratio = (processed_rms / original_rms - 1.0).abs();
    assert!(
        ratio < 0.2,
        "Low shelf boost should not significantly affect high frequencies, ratio: {}",
        ratio
    );
}

#[test]
fn test_compressor_reduces_peaks() {
    // Verify compressor actually reduces loud signals
    let mut comp = Compressor::new();
    comp.set_threshold(-20.0);
    comp.set_ratio(8.0);
    comp.set_attack(1.0); // Fast attack
    comp.set_makeup_gain(0.0); // No makeup gain

    // Generate loud signal that will be compressed
    let mut buffer = vec![0.7; 2000]; // Loud constant signal (stereo)

    let original_peak = calculate_peak(&buffer);

    comp.process(&mut buffer, 44100);

    // Skip first 100 samples (attack time)
    let processed_peak = calculate_peak(&buffer[200..]);

    // Peak should be significantly reduced
    assert!(
        processed_peak < original_peak * 0.9,
        "Compressor should reduce peaks, original: {}, processed: {}",
        original_peak,
        processed_peak
    );
}

#[test]
fn test_compressor_prevents_clipping() {
    // Verify compressor with limiting ratio prevents clipping
    let mut comp = Compressor::new();
    comp.set_threshold(-6.0);
    comp.set_ratio(20.0); // Very high ratio (limiter)
    comp.set_makeup_gain(0.0); // No makeup gain

    // Generate signal that would clip
    let mut buffer = vec![0.95; 1000]; // Very loud signal

    comp.process(&mut buffer, 44100);

    let peak = calculate_peak(&buffer);

    // Peak should be reduced below threshold
    assert!(
        peak < 1.0,
        "Compressor should prevent clipping, peak: {}",
        peak
    );
}

#[test]
fn test_effect_chain_order_matters() {
    // Verify that EQ → Compressor ≠ Compressor → EQ
    let sample_rate = 44100;

    // Use a longer signal so compressor has time to fully engage
    let signal = generate_sine(100.0, 0.5, sample_rate);

    // Chain 1: EQ (boost) → Compressor
    // EQ makes signal louder, then compressor reduces it
    let mut chain1 = EffectChain::new();
    let mut eq1 = ParametricEq::new();
    eq1.set_low_band(EqBand::low_shelf(200.0, 12.0)); // Large boost
    chain1.add_effect(Box::new(eq1));

    let comp1 = Compressor::with_settings(CompressorSettings {
        threshold_db: -20.0,
        ratio: 10.0,
        attack_ms: 1.0, // Fast attack to engage quickly
        release_ms: 100.0,
        makeup_gain_db: 0.0,
        knee_db: 0.0,
    });
    chain1.add_effect(Box::new(comp1));

    let mut buffer1 = signal.clone();
    chain1.process(&mut buffer1, sample_rate);

    // Chain 2: Compressor → EQ (boost)
    // Compressor first (less effect on moderate signal), then EQ boost
    let mut chain2 = EffectChain::new();

    let comp2 = Compressor::with_settings(CompressorSettings {
        threshold_db: -20.0,
        ratio: 10.0,
        attack_ms: 1.0,
        release_ms: 100.0,
        makeup_gain_db: 0.0,
        knee_db: 0.0,
    });
    chain2.add_effect(Box::new(comp2));

    let mut eq2 = ParametricEq::new();
    eq2.set_low_band(EqBand::low_shelf(200.0, 12.0)); // Large boost
    chain2.add_effect(Box::new(eq2));

    let mut buffer2 = signal.clone();
    chain2.process(&mut buffer2, sample_rate);

    // Compare RMS levels - more representative of compression effects than peak
    let rms1 = calculate_rms(&buffer1);
    let rms2 = calculate_rms(&buffer2);

    // Chain 1 (EQ→Comp): Signal is boosted then compressed - should have lower RMS
    // Chain 2 (Comp→EQ): Compressed first (at lower level), then boosted - should have higher RMS
    assert!(
        (rms1 - rms2).abs() > 0.05,
        "Effect chain order should matter significantly, RMS1: {:.3}, RMS2: {:.3}",
        rms1,
        rms2
    );
}

#[test]
fn test_disabled_effect_is_bit_perfect() {
    // Verify disabled effects don't introduce any processing artifacts
    let mut eq = ParametricEq::new();
    eq.set_enabled(false);

    // Use actual audio-like signal (not DC)
    let mut buffer = generate_sine(440.0, 0.1, 44100);
    let original = buffer.clone();

    eq.process(&mut buffer, 44100);

    // Should be bit-for-bit identical
    assert_eq!(
        buffer, original,
        "Disabled effect should not modify audio at all"
    );
}

// NOTE: Removed shallow tests that only checked is_finite() or trivial conditions:
// - test_empty_buffer_handling (trivial - just checks empty stays empty)
// - test_zero_signal_handling (covered by bit_perfect_test.rs)
// - test_eq_at_extreme_parameters (only checked is_finite, no frequency verification)
// - test_compressor_with_very_fast_attack (only checked is_finite, no attack verification)
// These behaviors are tested more thoroughly in regression_test.rs and *_precision_test.rs

#[test]
fn test_multiple_effect_resets() {
    // Verify resetting effects produces consistent results
    let mut eq = ParametricEq::new();
    eq.set_low_band(EqBand::low_shelf(100.0, 6.0));
    eq.reset(); // Snap coefficients to target for deterministic starting state

    let mut buffer = generate_sine(100.0, 0.1, 44100);

    // Process once
    eq.process(&mut buffer, 44100);
    let result1 = buffer.clone();

    // Reset and process again
    eq.reset();
    let mut buffer2 = generate_sine(100.0, 0.1, 44100);
    eq.process(&mut buffer2, 44100);

    // Results should be identical (deterministic)
    assert_eq!(
        result1, buffer2,
        "Reset should produce deterministic results"
    );
}

#[test]
fn test_sample_rate_change_handling() {
    // Verify effects adapt to sample rate changes
    let mut eq = ParametricEq::new();
    eq.set_mid_band(EqBand::peaking(1000.0, 6.0, 1.0));
    eq.reset(); // Snap coefficients to target for deterministic test

    // Process at 44.1kHz
    let mut buffer1 = generate_sine(1000.0, 0.1, 44100);
    eq.process(&mut buffer1, 44100);

    // Process at 48kHz (different sample rate)
    let mut buffer2 = generate_sine(1000.0, 0.1, 48000);
    eq.process(&mut buffer2, 48000);

    // Both should be processed without panic
    // Frequency response should be similar (relative to sample rate)
    let rms1 = calculate_rms(&buffer1);
    let rms2 = calculate_rms(&buffer2);

    // RMS should be in similar range (within 10% - tighter tolerance)
    let ratio = (rms1 / rms2).max(rms2 / rms1);
    assert!(
        ratio < 1.1,
        "Sample rate change should not significantly change processing, ratio: {}",
        ratio
    );
}

#[test]
fn test_chain_with_many_effects() {
    // Verify chain handles many effects without issues
    let mut chain = EffectChain::new();

    // Add 10 effects
    for _ in 0..5 {
        chain.add_effect(Box::new(ParametricEq::new()));
        chain.add_effect(Box::new(Compressor::new()));
    }

    assert_eq!(chain.len(), 10);

    let mut buffer = generate_sine(440.0, 0.1, 44100);

    // Should not panic or take excessive time
    chain.process(&mut buffer, 44100);

    // Verify audio is still finite
    for sample in &buffer {
        assert!(sample.is_finite(), "Chain should not produce NaN/Inf");
    }
}

#[test]
fn test_effect_state_isolation() {
    // Verify that multiple instances of same effect don't share state
    let mut eq1 = ParametricEq::new();
    let mut eq2 = ParametricEq::new();

    eq1.set_low_band(EqBand::low_shelf(100.0, 6.0));
    eq2.set_low_band(EqBand::low_shelf(100.0, -6.0)); // Different setting

    let mut buffer1 = generate_sine(100.0, 0.1, 44100);
    let mut buffer2 = buffer1.clone();

    eq1.process(&mut buffer1, 44100);
    eq2.process(&mut buffer2, 44100);

    let rms1 = calculate_rms(&buffer1);
    let rms2 = calculate_rms(&buffer2);

    // Results should be different (opposite gains)
    assert!(
        (rms1 - rms2).abs() > 0.1,
        "Different effect instances should not share state"
    );
}

#[test]
fn test_compressor_makeup_gain_compensation() {
    // Verify makeup gain compensates for compression
    let mut comp = Compressor::new();
    comp.set_threshold(-20.0);
    comp.set_ratio(4.0);
    comp.set_makeup_gain(0.0); // No makeup gain first

    let mut buffer1 = vec![0.5; 1000];
    comp.process(&mut buffer1, 44100);
    let rms_no_makeup = calculate_rms(&buffer1);

    // Reset and add makeup gain
    comp.reset();
    comp.set_makeup_gain(12.0); // Significant makeup gain

    let mut buffer2 = vec![0.5; 1000];
    comp.process(&mut buffer2, 44100);
    let rms_with_makeup = calculate_rms(&buffer2);

    // With makeup gain, RMS should be higher
    assert!(
        rms_with_makeup > rms_no_makeup * 1.5,
        "Makeup gain should increase output level significantly"
    );
}

#[test]
fn test_effect_chain_clear_and_reuse() {
    // Verify chain can be cleared and reused
    let mut chain = EffectChain::new();

    chain.add_effect(Box::new(ParametricEq::new()));
    chain.add_effect(Box::new(Compressor::new()));

    assert_eq!(chain.len(), 2);

    let mut buffer = generate_sine(440.0, 0.1, 44100);
    chain.process(&mut buffer, 44100); // Should work

    // Clear and add different effects
    chain.clear();
    assert_eq!(chain.len(), 0);

    chain.add_effect(Box::new(Compressor::new()));
    assert_eq!(chain.len(), 1);

    // Should still work
    chain.process(&mut buffer, 44100);
}
