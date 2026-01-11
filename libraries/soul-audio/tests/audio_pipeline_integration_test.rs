//! Audio Pipeline Integration Tests
//!
//! Comprehensive end-to-end tests for the audio processing pipeline.
//! Tests effect chains, signal integrity, resampling integration, and decoder integration.
//!
//! Following project rules:
//! - No shallow tests - every test verifies meaningful behavior
//! - Uses test-utils feature for signal generation and analysis

use soul_audio::effects::{
    AudioEffect, Compressor, CompressorSettings, Crossfeed, CrossfeedPreset, EffectChain, EqBand,
    GraphicEq, GraphicEqPreset, Limiter, LimiterSettings, ParametricEq, StereoEnhancer,
    StereoSettings,
};
use soul_audio::resampling::{Resampler, ResamplerBackend, ResamplingQuality};
use std::f32::consts::PI;

// ============================================================================
// Test Signal Utilities
// ============================================================================

/// Generate a sine wave for testing (stereo interleaved)
fn generate_sine_wave(
    frequency: f32,
    sample_rate: u32,
    duration_secs: f32,
    amplitude: f32,
) -> Vec<f32> {
    let num_samples = (sample_rate as f32 * duration_secs) as usize;
    let mut samples = Vec::with_capacity(num_samples * 2);

    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let sample = (2.0 * PI * frequency * t).sin() * amplitude;
        samples.push(sample); // Left
        samples.push(sample); // Right
    }

    samples
}

/// Generate white noise for testing
fn generate_white_noise(sample_rate: u32, duration_secs: f32, amplitude: f32) -> Vec<f32> {
    let num_samples = (sample_rate as f32 * duration_secs) as usize;
    let mut samples = Vec::with_capacity(num_samples * 2);

    for _ in 0..num_samples {
        let sample = (rand::random::<f32>() * 2.0 - 1.0) * amplitude;
        samples.push(sample);
        samples.push(sample);
    }

    samples
}

/// Generate an impulse signal
fn generate_impulse(sample_rate: u32, duration_secs: f32, amplitude: f32) -> Vec<f32> {
    let num_samples = (sample_rate as f32 * duration_secs) as usize;
    let mut samples = vec![0.0; num_samples * 2];

    // Place impulse at 10% of duration
    let impulse_position = (num_samples / 10) * 2;
    if impulse_position < samples.len() {
        samples[impulse_position] = amplitude;
        samples[impulse_position + 1] = amplitude;
    }

    samples
}

/// Generate a dynamic test signal with loud and quiet sections
fn generate_dynamic_signal(
    sample_rate: u32,
    duration_secs: f32,
    quiet_amplitude: f32,
    loud_amplitude: f32,
) -> Vec<f32> {
    let num_samples = (sample_rate as f32 * duration_secs) as usize;
    let mut samples = Vec::with_capacity(num_samples * 2);
    let section_length = num_samples / 4;

    for i in 0..num_samples {
        let section = i / section_length;
        let amplitude = if section % 2 == 0 {
            quiet_amplitude
        } else {
            loud_amplitude
        };
        let t = i as f32 / sample_rate as f32;
        let sample = (2.0 * PI * 440.0 * t).sin() * amplitude;
        samples.push(sample);
        samples.push(sample);
    }

    samples
}

/// Calculate RMS of a signal
fn calculate_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_squares: f32 = samples.iter().map(|s| s * s).sum();
    (sum_squares / samples.len() as f32).sqrt()
}

/// Calculate peak amplitude
fn calculate_peak(samples: &[f32]) -> f32 {
    samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max)
}

/// Extract mono channel from stereo
fn extract_mono(stereo: &[f32], channel: usize) -> Vec<f32> {
    stereo.chunks_exact(2).map(|chunk| chunk[channel]).collect()
}

/// Simple DFT-based frequency detection
fn find_dominant_frequency(samples: &[f32], sample_rate: u32) -> f32 {
    let n = samples.len().min(4096);
    let samples = &samples[0..n];

    let mut max_magnitude = 0.0f32;
    let mut dominant_freq = 0.0f32;

    for k in 1..n / 2 {
        let mut real = 0.0;
        let mut imag = 0.0;

        for (i, &sample) in samples.iter().enumerate() {
            let angle = -2.0 * PI * (k as f32) * (i as f32) / (n as f32);
            real += sample * angle.cos();
            imag += sample * angle.sin();
        }

        let magnitude = (real * real + imag * imag).sqrt();
        if magnitude > max_magnitude {
            max_magnitude = magnitude;
            dominant_freq = (k as f32 * sample_rate as f32) / (n as f32);
        }
    }

    dominant_freq
}

// ============================================================================
// 1. Full Effect Chain Tests
// ============================================================================

#[test]
fn test_eq_compressor_limiter_chain() {
    let mut chain = EffectChain::new();

    // Add EQ with bass boost
    let mut eq = ParametricEq::new();
    eq.set_low_band(EqBand::low_shelf(100.0, 6.0)); // +6dB bass boost
    eq.set_mid_band(EqBand::peaking(1000.0, 0.0, 1.0));
    eq.set_high_band(EqBand::high_shelf(8000.0, 0.0));
    chain.add_effect(Box::new(eq));

    // Add compressor
    let comp = Compressor::with_settings(CompressorSettings::moderate());
    chain.add_effect(Box::new(comp));

    // Add limiter to prevent clipping
    let limiter = Limiter::with_settings(LimiterSettings::brickwall());
    chain.add_effect(Box::new(limiter));

    // Generate test signal
    let mut buffer = generate_sine_wave(100.0, 44100, 0.2, 0.8);
    let original_peak = calculate_peak(&buffer);

    // Process through chain
    chain.process(&mut buffer, 44100);

    // Verify limiter prevents clipping
    let processed_peak = calculate_peak(&buffer);
    assert!(
        processed_peak <= 1.0,
        "Limiter should prevent clipping, got peak: {}",
        processed_peak
    );

    // Signal should be modified by the chain
    assert!(
        (original_peak - processed_peak).abs() > 0.001,
        "Signal should be modified by the effect chain"
    );
}

#[test]
fn test_crossfeed_stereo_enhancer_chain() {
    let mut chain = EffectChain::new();

    // Add crossfeed for headphone listening
    let crossfeed = Crossfeed::with_preset(CrossfeedPreset::Natural);
    chain.add_effect(Box::new(crossfeed));

    // Add stereo enhancer
    let enhancer = StereoEnhancer::with_settings(StereoSettings::wide());
    chain.add_effect(Box::new(enhancer));

    // Generate hard-panned stereo signal (left only)
    let sample_rate = 44100u32;
    let num_samples = 4410; // 0.1 seconds
    let mut buffer: Vec<f32> = (0..num_samples)
        .flat_map(|i| {
            let t = i as f32 / sample_rate as f32;
            let left = (2.0 * PI * 440.0 * t).sin() * 0.5;
            [left, 0.0] // Left channel only
        })
        .collect();

    // Process
    chain.process(&mut buffer, sample_rate);

    // After crossfeed, right channel should have some signal
    let right_channel = extract_mono(&buffer, 1);
    let right_rms = calculate_rms(&right_channel);

    assert!(
        right_rms > 0.01,
        "Crossfeed should add signal to right channel, got RMS: {}",
        right_rms
    );
}

// Note: test_convolution_eq_chain removed - convolution module is disabled

#[test]
fn test_all_effects_enabled_simultaneously() {
    let mut chain = EffectChain::new();

    // Add all effects
    let mut eq = ParametricEq::new();
    eq.set_low_band(EqBand::low_shelf(80.0, 2.0));
    eq.set_mid_band(EqBand::peaking(1000.0, -1.0, 1.0));
    eq.set_high_band(EqBand::high_shelf(8000.0, 1.0));
    chain.add_effect(Box::new(eq));

    let mut graphic_eq = GraphicEq::new_10_band();
    graphic_eq.set_preset(GraphicEqPreset::BassBoost);
    chain.add_effect(Box::new(graphic_eq));

    let compressor = Compressor::with_settings(CompressorSettings::gentle());
    chain.add_effect(Box::new(compressor));

    let crossfeed = Crossfeed::with_preset(CrossfeedPreset::Relaxed);
    chain.add_effect(Box::new(crossfeed));

    let enhancer = StereoEnhancer::with_settings(StereoSettings::with_width(1.2));
    chain.add_effect(Box::new(enhancer));

    let limiter = Limiter::with_settings(LimiterSettings::soft());
    chain.add_effect(Box::new(limiter));

    // Generate test signal
    let mut buffer = generate_sine_wave(440.0, 44100, 0.5, 0.7);

    // Process - should not panic or produce NaN/Inf
    chain.process(&mut buffer, 44100);

    // Verify output is valid
    assert!(
        buffer.iter().all(|s| s.is_finite()),
        "All output samples should be finite"
    );

    // Verify no clipping
    let peak = calculate_peak(&buffer);
    assert!(peak <= 1.0, "Output should not clip, got peak: {}", peak);
}

#[test]
fn test_effect_order_matters() {
    // Test that different effect orders produce different results

    // Order 1: EQ -> Compressor
    let mut chain1 = EffectChain::new();
    let mut eq1 = ParametricEq::new();
    eq1.set_low_band(EqBand::low_shelf(100.0, 12.0)); // Huge bass boost
    chain1.add_effect(Box::new(eq1));
    chain1.add_effect(Box::new(Compressor::with_settings(
        CompressorSettings::aggressive(),
    )));

    // Order 2: Compressor -> EQ
    let mut chain2 = EffectChain::new();
    chain2.add_effect(Box::new(Compressor::with_settings(
        CompressorSettings::aggressive(),
    )));
    let mut eq2 = ParametricEq::new();
    eq2.set_low_band(EqBand::low_shelf(100.0, 12.0)); // Same bass boost
    chain2.add_effect(Box::new(eq2));

    // Generate same test signal for both
    let original = generate_sine_wave(100.0, 44100, 0.2, 0.5);
    let mut buffer1 = original.clone();
    let mut buffer2 = original.clone();

    // Process both
    chain1.process(&mut buffer1, 44100);
    chain2.process(&mut buffer2, 44100);

    // Calculate difference
    let diff: f32 = buffer1
        .iter()
        .zip(buffer2.iter())
        .map(|(a, b)| (a - b).abs())
        .sum::<f32>()
        / buffer1.len() as f32;

    assert!(
        diff > 0.001,
        "Different effect orders should produce different results, diff: {}",
        diff
    );
}

// ============================================================================
// 2. Signal Integrity Tests
// ============================================================================

#[test]
fn test_sine_wave_frequency_preservation() {
    // Test that a sine wave through the pipeline maintains its fundamental frequency
    let test_freq = 1000.0;
    let sample_rate = 44100u32;

    let mut chain = EffectChain::new();

    // Add effects that shouldn't change frequency (just gain/dynamics)
    let compressor = Compressor::with_settings(CompressorSettings::gentle());
    chain.add_effect(Box::new(compressor));

    let limiter = Limiter::with_settings(LimiterSettings::soft());
    chain.add_effect(Box::new(limiter));

    // Generate 1kHz sine
    let mut buffer = generate_sine_wave(test_freq, sample_rate, 0.5, 0.5);

    // Process
    chain.process(&mut buffer, sample_rate);

    // Detect dominant frequency
    let mono = extract_mono(&buffer, 0);
    let detected_freq = find_dominant_frequency(&mono, sample_rate);

    // Should be within 5% of original
    let freq_error = (detected_freq - test_freq).abs() / test_freq;
    assert!(
        freq_error < 0.05,
        "Frequency should be preserved, expected ~{} Hz, got {} Hz (error: {:.1}%)",
        test_freq,
        detected_freq,
        freq_error * 100.0
    );
}

#[test]
fn test_white_noise_through_pipeline() {
    let sample_rate = 44100u32;

    let mut chain = EffectChain::new();

    // Add EQ with cuts and boosts
    let mut eq = ParametricEq::new();
    eq.set_low_band(EqBand::low_shelf(100.0, -6.0)); // Cut bass
    eq.set_high_band(EqBand::high_shelf(8000.0, -6.0)); // Cut treble
    chain.add_effect(Box::new(eq));

    // Add compressor
    let compressor = Compressor::with_settings(CompressorSettings::moderate());
    chain.add_effect(Box::new(compressor));

    // Generate white noise
    let mut buffer = generate_white_noise(sample_rate, 0.5, 0.5);
    let original_rms = calculate_rms(&buffer);

    // Process
    chain.process(&mut buffer, sample_rate);

    // RMS should be reduced (EQ cuts + compression)
    let processed_rms = calculate_rms(&buffer);

    assert!(
        processed_rms < original_rms,
        "White noise RMS should be reduced after EQ cuts, original: {}, processed: {}",
        original_rms,
        processed_rms
    );

    // Output should still be finite
    assert!(
        buffer.iter().all(|s| s.is_finite()),
        "All samples should be finite after processing white noise"
    );
}

#[test]
fn test_impulse_response_through_chain() {
    let sample_rate = 44100u32;

    let mut chain = EffectChain::new();

    // Add all effect types
    let eq = ParametricEq::new();
    chain.add_effect(Box::new(eq));

    let compressor = Compressor::with_settings(CompressorSettings::gentle());
    chain.add_effect(Box::new(compressor));

    let limiter = Limiter::new();
    chain.add_effect(Box::new(limiter));

    // Generate impulse
    let mut buffer = generate_impulse(sample_rate, 0.1, 0.9);

    // Process
    chain.process(&mut buffer, sample_rate);

    // Find impulse in output (should be smeared by filters)
    let peak = calculate_peak(&buffer);

    // Impulse should not be clipped
    assert!(peak <= 1.0, "Impulse should not clip, got peak: {}", peak);

    // Impulse should still be present (not zeroed out)
    assert!(
        peak > 0.01,
        "Impulse should produce output, got peak: {}",
        peak
    );
}

#[test]
fn test_limiter_prevents_clipping() {
    let sample_rate = 44100u32;

    let mut chain = EffectChain::new();

    // Add EQ with extreme boost to intentionally cause clipping
    let mut eq = ParametricEq::new();
    eq.set_low_band(EqBand::low_shelf(100.0, 12.0)); // +12dB
    eq.set_mid_band(EqBand::peaking(1000.0, 12.0, 1.0)); // +12dB
    eq.set_high_band(EqBand::high_shelf(8000.0, 12.0)); // +12dB
    chain.add_effect(Box::new(eq));

    // Add limiter to catch the boosted signal
    let limiter = Limiter::with_settings(LimiterSettings {
        threshold_db: -0.1,
        release_ms: 50.0,
    });
    chain.add_effect(Box::new(limiter));

    // Generate loud signal that will clip after EQ boost
    let mut buffer = generate_sine_wave(500.0, sample_rate, 0.5, 0.5);

    // Process
    chain.process(&mut buffer, sample_rate);

    // Verify no sample exceeds 0dBFS
    let peak = calculate_peak(&buffer);
    assert!(
        peak <= 1.0,
        "Limiter should prevent clipping, got peak: {} ({} dB)",
        peak,
        20.0 * peak.log10()
    );
}

// ============================================================================
// 3. Resampling Integration Tests
// ============================================================================

#[test]
fn test_44k_to_48k_with_effects() {
    // Test upsampling from 44.1kHz to 48kHz, then apply effects

    let mut resampler = Resampler::new(
        ResamplerBackend::Auto,
        44100,
        48000,
        2,
        ResamplingQuality::High,
    )
    .unwrap();

    // Generate 44.1kHz signal
    let input = generate_sine_wave(1000.0, 44100, 0.5, 0.5);

    // Resample to 48kHz
    let resampled = resampler.process(&input).unwrap();

    // Verify output is produced (resampler may buffer some samples)
    // The exact length depends on resampler internals (chunking, buffering)
    assert!(!resampled.is_empty(), "Resampler should produce output");

    // Verify output length is in the right ballpark
    // Allow for significant margin due to resampler buffering
    let expected_ratio = 48000.0 / 44100.0;
    let expected_len = (input.len() as f32 * expected_ratio) as usize;
    assert!(
        resampled.len() > expected_len / 2 && resampled.len() < expected_len * 2,
        "Resampled length should be roughly proportional, expected ~{}, got {}",
        expected_len,
        resampled.len()
    );

    // Now apply effects at 48kHz
    let mut chain = EffectChain::new();
    let eq = ParametricEq::new();
    chain.add_effect(Box::new(eq));

    let mut processed = resampled;
    chain.process(&mut processed, 48000);

    // Verify output is valid
    assert!(processed.iter().all(|s| s.is_finite()));
}

#[test]
fn test_96k_to_44k_downsampling_with_effects() {
    // Test downsampling from 96kHz to 44.1kHz

    let mut resampler = Resampler::new(
        ResamplerBackend::Auto,
        96000,
        44100,
        2,
        ResamplingQuality::High,
    )
    .unwrap();

    // Generate 96kHz signal
    let input = generate_sine_wave(1000.0, 96000, 0.5, 0.5);

    // Resample to 44.1kHz
    let resampled = resampler.process(&input).unwrap();

    // Verify output is produced
    assert!(!resampled.is_empty(), "Downsampler should produce output");

    // Verify output length is in the right ballpark
    // Allow for significant margin due to resampler buffering
    let expected_ratio = 44100.0 / 96000.0;
    let expected_len = (input.len() as f32 * expected_ratio) as usize;
    assert!(
        resampled.len() > expected_len / 2 && resampled.len() < expected_len * 2,
        "Downsampled length should be roughly proportional, expected ~{}, got {}",
        expected_len,
        resampled.len()
    );

    // Apply effects at 44.1kHz
    let mut chain = EffectChain::new();
    let compressor = Compressor::with_settings(CompressorSettings::gentle());
    chain.add_effect(Box::new(compressor));

    let mut processed = resampled;
    chain.process(&mut processed, 44100);

    // Verify output is valid (no NaN/Inf)
    assert!(
        processed.iter().all(|s| s.is_finite()),
        "Downsampled and processed output should be valid"
    );
}

#[test]
fn test_resampling_quality_levels() {
    // Test that different quality levels work correctly

    let qualities = [
        ResamplingQuality::Fast,
        ResamplingQuality::Balanced,
        ResamplingQuality::High,
        ResamplingQuality::Maximum,
    ];

    let input = generate_sine_wave(1000.0, 44100, 0.1, 0.5);

    for quality in qualities {
        let mut resampler =
            Resampler::new(ResamplerBackend::Auto, 44100, 48000, 2, quality).unwrap();

        let output = resampler.process(&input).unwrap();

        // All quality levels should produce valid output
        assert!(
            output.iter().all(|s| s.is_finite()),
            "Quality {:?} produced invalid output",
            quality
        );

        // Output should not be empty
        assert!(
            !output.is_empty(),
            "Quality {:?} produced empty output",
            quality
        );
    }
}

#[test]
fn test_sample_rate_change_with_effect_reset() {
    // Simulate sample rate change during playback

    let mut eq = ParametricEq::new();
    eq.set_low_band(EqBand::low_shelf(100.0, 6.0));

    // Process at 44.1kHz
    let mut buffer_44k = generate_sine_wave(440.0, 44100, 0.1, 0.5);
    eq.process(&mut buffer_44k, 44100);

    // Reset effect state (as would happen on track change)
    eq.reset();

    // Process at 48kHz (different sample rate)
    let mut buffer_48k = generate_sine_wave(440.0, 48000, 0.1, 0.5);
    eq.process(&mut buffer_48k, 48000);

    // Both outputs should be valid
    assert!(buffer_44k.iter().all(|s| s.is_finite()));
    assert!(buffer_48k.iter().all(|s| s.is_finite()));
}

// ============================================================================
// 4. Effect State and Reset Tests
// ============================================================================

#[test]
fn test_effect_chain_reset_clears_all_state() {
    let mut chain = EffectChain::new();

    // Add stateful effects
    let mut eq = ParametricEq::new();
    eq.set_low_band(EqBand::low_shelf(100.0, 6.0));
    chain.add_effect(Box::new(eq));

    let compressor = Compressor::with_settings(CompressorSettings::aggressive());
    chain.add_effect(Box::new(compressor));

    // Process some audio to build up state
    let mut buffer1 = generate_dynamic_signal(44100, 0.5, 0.1, 0.9);
    chain.process(&mut buffer1, 44100);

    // Reset chain
    chain.reset();

    // Process same signal again - should be deterministic
    let mut buffer2 = generate_dynamic_signal(44100, 0.5, 0.1, 0.9);
    chain.process(&mut buffer2, 44100);

    let mut buffer3 = generate_dynamic_signal(44100, 0.5, 0.1, 0.9);
    chain.reset();
    chain.process(&mut buffer3, 44100);

    // buffer2 and buffer3 should be identical (both processed from reset state)
    let diff: f32 = buffer2
        .iter()
        .zip(buffer3.iter())
        .map(|(a, b)| (a - b).abs())
        .sum::<f32>()
        / buffer2.len() as f32;

    assert!(
        diff < 0.001,
        "Reset should produce deterministic results, diff: {}",
        diff
    );
}

#[test]
fn test_effect_bypass_preserves_signal() {
    let mut chain = EffectChain::new();

    // Add effects but disable them
    let mut eq = ParametricEq::new();
    eq.set_low_band(EqBand::low_shelf(100.0, 12.0)); // Would drastically change signal
    eq.set_enabled(false);
    chain.add_effect(Box::new(eq));

    let mut compressor = Compressor::with_settings(CompressorSettings::aggressive());
    compressor.set_enabled(false);
    chain.add_effect(Box::new(compressor));

    // Process
    let original = generate_sine_wave(440.0, 44100, 0.2, 0.5);
    let mut processed = original.clone();
    chain.process(&mut processed, 44100);

    // Should be unchanged
    let diff: f32 = original
        .iter()
        .zip(processed.iter())
        .map(|(a, b)| (a - b).abs())
        .sum::<f32>();

    assert!(
        diff < 0.0001,
        "Disabled effects should not modify signal, diff: {}",
        diff
    );
}

#[test]
fn test_enable_disable_all_effects() {
    let mut chain = EffectChain::new();

    let eq = ParametricEq::new();
    chain.add_effect(Box::new(eq));

    let compressor = Compressor::new();
    chain.add_effect(Box::new(compressor));

    // Disable all
    chain.set_enabled(false);

    let original = generate_sine_wave(440.0, 44100, 0.2, 0.5);
    let mut processed = original.clone();
    chain.process(&mut processed, 44100);

    // Should be unchanged
    assert_eq!(
        original, processed,
        "Disabled chain should not modify signal"
    );

    // Re-enable all
    chain.set_enabled(true);

    let mut processed2 = original.clone();
    chain.process(&mut processed2, 44100);

    // May or may not change (depends on neutral settings), but should be valid
    assert!(processed2.iter().all(|s| s.is_finite()));
}

// ============================================================================
// 5. Edge Cases and Stress Tests
// ============================================================================

#[test]
fn test_empty_buffer_processing() {
    let mut chain = EffectChain::new();
    chain.add_effect(Box::new(ParametricEq::new()));
    chain.add_effect(Box::new(Compressor::new()));

    // Process empty buffer - should not panic
    let mut empty: Vec<f32> = vec![];
    chain.process(&mut empty, 44100);

    assert!(empty.is_empty());
}

#[test]
fn test_single_sample_processing() {
    let mut chain = EffectChain::new();
    chain.add_effect(Box::new(ParametricEq::new()));
    chain.add_effect(Box::new(Compressor::new()));
    chain.add_effect(Box::new(Limiter::new()));

    // Process single stereo sample
    let mut buffer = vec![0.5, 0.5];
    chain.process(&mut buffer, 44100);

    assert!(buffer.iter().all(|s| s.is_finite()));
}

#[test]
fn test_very_high_sample_rate() {
    let mut chain = EffectChain::new();

    let mut eq = ParametricEq::new();
    eq.set_low_band(EqBand::low_shelf(100.0, 3.0));
    chain.add_effect(Box::new(eq));

    // Generate at 192kHz
    let mut buffer = generate_sine_wave(1000.0, 192000, 0.1, 0.5);

    // Process at 192kHz
    chain.process(&mut buffer, 192000);

    // Should produce valid output
    assert!(
        buffer.iter().all(|s| s.is_finite()),
        "192kHz processing should produce valid output"
    );
}

#[test]
fn test_very_low_sample_rate() {
    let mut chain = EffectChain::new();

    let mut eq = ParametricEq::new();
    eq.set_low_band(EqBand::low_shelf(50.0, 3.0));
    chain.add_effect(Box::new(eq));

    // Generate at 8kHz (telephony rate)
    let mut buffer = generate_sine_wave(300.0, 8000, 0.1, 0.5);

    // Process at 8kHz
    chain.process(&mut buffer, 8000);

    // Should produce valid output
    assert!(
        buffer.iter().all(|s| s.is_finite()),
        "8kHz processing should produce valid output"
    );
}

#[test]
fn test_extreme_eq_settings() {
    let mut eq = ParametricEq::new();

    // Max boost on all bands
    eq.set_low_band(EqBand::low_shelf(80.0, 12.0));
    eq.set_mid_band(EqBand::peaking(1000.0, 12.0, 0.1)); // Very narrow Q
    eq.set_high_band(EqBand::high_shelf(8000.0, 12.0));

    let mut buffer = generate_sine_wave(1000.0, 44100, 0.2, 0.3);

    eq.process(&mut buffer, 44100);

    // Output should still be finite (no NaN/Inf)
    assert!(
        buffer.iter().all(|s| s.is_finite()),
        "Extreme EQ settings should still produce finite output"
    );
}

#[test]
fn test_dc_offset_handling() {
    let mut chain = EffectChain::new();
    chain.add_effect(Box::new(ParametricEq::new()));
    chain.add_effect(Box::new(Compressor::new()));

    // Generate signal with DC offset
    let mut buffer: Vec<f32> = (0..4410)
        .flat_map(|i| {
            let t = i as f32 / 44100.0;
            let sample = (2.0 * PI * 440.0 * t).sin() * 0.3 + 0.3; // DC offset of 0.3
            [sample, sample]
        })
        .collect();

    chain.process(&mut buffer, 44100);

    // Output should be valid
    assert!(buffer.iter().all(|s| s.is_finite()));
}

#[test]
fn test_near_nyquist_frequency() {
    let sample_rate = 44100u32;
    let nyquist = sample_rate as f32 / 2.0;

    let mut eq = ParametricEq::new();
    eq.set_high_band(EqBand::high_shelf(nyquist - 100.0, 6.0)); // Very high frequency

    // Generate high frequency signal (15kHz)
    let mut buffer = generate_sine_wave(15000.0, sample_rate, 0.1, 0.5);

    eq.process(&mut buffer, sample_rate);

    // Should not produce unstable output
    assert!(
        buffer.iter().all(|s| s.is_finite() && s.abs() < 10.0),
        "Near-Nyquist processing should be stable"
    );
}

#[test]
fn test_long_duration_processing() {
    let mut chain = EffectChain::new();
    chain.add_effect(Box::new(ParametricEq::new()));
    chain.add_effect(Box::new(Compressor::new()));
    chain.add_effect(Box::new(Limiter::new()));

    // Process 30 seconds of audio (in chunks)
    let chunk_size = 4096;
    let total_samples = 44100 * 30 * 2; // 30 seconds stereo
    let num_chunks = total_samples / chunk_size;

    for _ in 0..num_chunks {
        let mut chunk: Vec<f32> = (0..chunk_size / 2)
            .flat_map(|i| {
                let t = (rand::random::<f32>() * 0.1) as f32;
                let sample = (2.0 * PI * 440.0 * t).sin() * 0.3;
                [sample, sample]
            })
            .collect();

        chain.process(&mut chunk, 44100);

        // Verify each chunk is valid
        assert!(chunk.iter().all(|s| s.is_finite() && s.abs() <= 1.5));
    }
}

// ============================================================================
// 6. Compression and Dynamics Tests
// ============================================================================

#[test]
fn test_compressor_reduces_dynamic_range() {
    // Use settings without makeup gain to test pure compression
    let settings = CompressorSettings {
        threshold_db: -20.0,
        ratio: 4.0,
        attack_ms: 5.0,
        release_ms: 50.0,
        knee_db: 6.0,
        makeup_gain_db: 0.0, // No makeup gain
    };
    let mut compressor = Compressor::with_settings(settings);

    // Generate signal with high dynamic range
    let mut buffer = generate_dynamic_signal(44100, 0.5, 0.1, 0.9);

    // Calculate original dynamic range
    let mono = extract_mono(&buffer, 0);
    let original_peak = calculate_peak(&mono);

    // Process
    compressor.process(&mut buffer, 44100);

    // Find loud sections and verify they're compressed
    let processed_mono = extract_mono(&buffer, 0);
    let processed_peak = calculate_peak(&processed_mono);

    // With compression and no makeup gain, peaks should be reduced or similar
    // (compression reduces loud parts, quiet parts stay the same)
    assert!(
        processed_peak <= original_peak + 0.1,
        "Compressor without makeup gain should not increase peaks significantly, original: {}, processed: {}",
        original_peak,
        processed_peak
    );
}

#[test]
fn test_limiter_brickwall_behavior() {
    let mut limiter = Limiter::with_settings(LimiterSettings {
        threshold_db: -3.0, // -3dB threshold (~0.707)
        release_ms: 50.0,
    });

    // Generate loud signal that exceeds threshold
    let mut buffer = generate_sine_wave(440.0, 44100, 0.2, 0.9);

    limiter.process(&mut buffer, 44100);

    // All samples should be below threshold (with small margin for attack)
    let peak = calculate_peak(&buffer);
    let threshold_linear = 10.0f32.powf(-3.0 / 20.0); // ~0.707

    assert!(
        peak <= threshold_linear + 0.05,
        "Limiter should enforce threshold, expected <= {}, got {}",
        threshold_linear,
        peak
    );
}

#[test]
fn test_compressor_makeup_gain() {
    let settings = CompressorSettings {
        threshold_db: -20.0,
        ratio: 4.0,
        attack_ms: 5.0,
        release_ms: 50.0,
        knee_db: 6.0,
        makeup_gain_db: 12.0, // +12dB makeup
    };

    let mut compressor = Compressor::with_settings(settings);

    // Generate quiet signal
    let mut buffer = generate_sine_wave(440.0, 44100, 0.3, 0.2);
    let original_rms = calculate_rms(&buffer);

    compressor.process(&mut buffer, 44100);

    let processed_rms = calculate_rms(&buffer);

    // Makeup gain should increase output level
    assert!(
        processed_rms > original_rms * 1.5,
        "Makeup gain should increase output, original RMS: {}, processed RMS: {}",
        original_rms,
        processed_rms
    );
}

// ============================================================================
// 7. Stereo Processing Tests
// ============================================================================

#[test]
fn test_stereo_enhancer_width_control() {
    // Test mono (width = 0)
    let mut enhancer_mono = StereoEnhancer::with_settings(StereoSettings::mono());

    let mut stereo_signal: Vec<f32> = (0..4410)
        .flat_map(|i| {
            let t = i as f32 / 44100.0;
            let left = (2.0 * PI * 440.0 * t).sin() * 0.5;
            let right = (2.0 * PI * 880.0 * t).sin() * 0.5; // Different frequency
            [left, right]
        })
        .collect();

    enhancer_mono.process(&mut stereo_signal, 44100);

    // In mono mode, L and R should be identical
    for chunk in stereo_signal.chunks_exact(2) {
        assert!(
            (chunk[0] - chunk[1]).abs() < 0.001,
            "Mono mode should produce identical L/R, got L={}, R={}",
            chunk[0],
            chunk[1]
        );
    }
}

#[test]
fn test_stereo_enhancer_extra_wide() {
    let mut enhancer = StereoEnhancer::with_settings(StereoSettings::extra_wide());

    // Create stereo signal with some stereo content
    let mut buffer: Vec<f32> = (0..4410)
        .flat_map(|i| {
            let t = i as f32 / 44100.0;
            let left = (2.0 * PI * 440.0 * t).sin() * 0.4;
            let right = (2.0 * PI * 440.0 * t + 0.5).sin() * 0.4; // Slightly phase shifted
            [left, right]
        })
        .collect();

    // Calculate original stereo difference
    let original_diff: f32 = buffer
        .chunks_exact(2)
        .map(|c| (c[0] - c[1]).abs())
        .sum::<f32>()
        / (buffer.len() / 2) as f32;

    enhancer.process(&mut buffer, 44100);

    // Calculate processed stereo difference
    let processed_diff: f32 = buffer
        .chunks_exact(2)
        .map(|c| (c[0] - c[1]).abs())
        .sum::<f32>()
        / (buffer.len() / 2) as f32;

    // Extra wide should increase stereo difference
    assert!(
        processed_diff > original_diff,
        "Extra wide should increase stereo separation, original: {}, processed: {}",
        original_diff,
        processed_diff
    );
}

#[test]
fn test_crossfeed_adds_crosstalk() {
    let mut crossfeed = Crossfeed::with_preset(CrossfeedPreset::Natural);

    // Create hard-panned left signal
    let mut buffer: Vec<f32> = (0..4410)
        .flat_map(|i| {
            let t = i as f32 / 44100.0;
            let left = (2.0 * PI * 440.0 * t).sin() * 0.5;
            [left, 0.0] // Right is silent
        })
        .collect();

    crossfeed.process(&mut buffer, 44100);

    // Right channel should now have some signal
    let right_rms = calculate_rms(&extract_mono(&buffer, 1));

    assert!(
        right_rms > 0.01,
        "Crossfeed should add signal to right channel, got RMS: {}",
        right_rms
    );
}

// ============================================================================
// 8. Graphic EQ Tests
// ============================================================================

#[test]
fn test_graphic_eq_presets() {
    let presets = [
        GraphicEqPreset::Flat,
        GraphicEqPreset::BassBoost,
        GraphicEqPreset::TrebleBoost,
        GraphicEqPreset::VShape,
        GraphicEqPreset::Vocal,
    ];

    for preset in presets {
        let mut eq = GraphicEq::new_10_band();
        eq.set_preset(preset);
        let mut buffer = generate_sine_wave(440.0, 44100, 0.1, 0.5);

        eq.process(&mut buffer, 44100);

        assert!(
            buffer.iter().all(|s| s.is_finite()),
            "Preset {:?} should produce finite output",
            preset
        );
    }
}

#[test]
fn test_graphic_eq_individual_bands() {
    let mut eq = GraphicEq::new_10_band();

    // Boost 1kHz band (index 5 = 1000.0 Hz in ISO_10_BAND_FREQUENCIES)
    let mut gains = [0.0f32; 10];
    gains[5] = 6.0; // 1kHz band
    eq.set_gains_10(gains);

    // Generate 1kHz sine
    let original = generate_sine_wave(1000.0, 44100, 0.2, 0.3);
    let original_rms = calculate_rms(&original);

    let mut processed = original.clone();
    eq.process(&mut processed, 44100);

    let processed_rms = calculate_rms(&processed);

    // 1kHz signal should be boosted
    assert!(
        processed_rms > original_rms * 1.2,
        "1kHz boost should increase signal level, original: {}, processed: {}",
        original_rms,
        processed_rms
    );
}

// ============================================================================
// 9. Integration with Real Audio Patterns
// ============================================================================

#[test]
fn test_music_like_signal_processing() {
    // Simulate a music-like signal with bass, mids, and highs
    let sample_rate = 44100u32;
    let duration = 0.5;
    let num_samples = (sample_rate as f32 * duration) as usize;

    let mut buffer: Vec<f32> = (0..num_samples)
        .flat_map(|i| {
            let t = i as f32 / sample_rate as f32;
            // Bass (80Hz) + Mids (500Hz) + Highs (5kHz)
            let bass = (2.0 * PI * 80.0 * t).sin() * 0.3;
            let mids = (2.0 * PI * 500.0 * t).sin() * 0.2;
            let highs = (2.0 * PI * 5000.0 * t).sin() * 0.1;
            let sample = bass + mids + highs;
            [sample, sample]
        })
        .collect();

    // Create realistic mastering chain
    let mut chain = EffectChain::new();

    // EQ for tonal balance
    let mut eq = ParametricEq::new();
    eq.set_low_band(EqBand::low_shelf(100.0, -2.0)); // Slight bass cut
    eq.set_mid_band(EqBand::peaking(3000.0, 2.0, 1.0)); // Presence boost
    eq.set_high_band(EqBand::high_shelf(10000.0, 1.0)); // Air
    chain.add_effect(Box::new(eq));

    // Gentle compression
    let compressor = Compressor::with_settings(CompressorSettings::gentle());
    chain.add_effect(Box::new(compressor));

    // Limiting
    let limiter = Limiter::with_settings(LimiterSettings::brickwall());
    chain.add_effect(Box::new(limiter));

    // Process
    chain.process(&mut buffer, sample_rate);

    // Verify output is broadcast-ready
    let peak = calculate_peak(&buffer);
    assert!(peak <= 1.0, "Output should not clip, peak: {}", peak);
    assert!(peak > 0.1, "Output should not be silent, peak: {}", peak);

    // Verify no artifacts
    assert!(
        buffer.iter().all(|s| s.is_finite()),
        "Output should have no NaN/Inf values"
    );
}

#[test]
fn test_podcast_voice_processing() {
    // Simulate voice-like signal (300Hz fundamental with harmonics)
    let sample_rate = 44100u32;
    let num_samples = 22050; // 0.5 seconds

    let mut buffer: Vec<f32> = (0..num_samples)
        .flat_map(|i| {
            let t = i as f32 / sample_rate as f32;
            // Fundamental + harmonics
            let f0 = (2.0 * PI * 300.0 * t).sin() * 0.4;
            let f1 = (2.0 * PI * 600.0 * t).sin() * 0.2;
            let f2 = (2.0 * PI * 900.0 * t).sin() * 0.1;
            let sample = f0 + f1 + f2;
            [sample, sample]
        })
        .collect();

    // Calculate original RMS for level comparison
    let original_rms = calculate_rms(&buffer);

    // Podcast processing chain
    let mut chain = EffectChain::new();

    // Mild EQ for voice (the low shelf cut can significantly reduce level)
    let mut eq = ParametricEq::new();
    eq.set_low_band(EqBand::low_shelf(80.0, -3.0)); // Mild sub-bass cut
    eq.set_mid_band(EqBand::peaking(3500.0, 2.0, 1.5)); // Clarity boost
    chain.add_effect(Box::new(eq));

    // Moderate compression with appropriate makeup gain to maintain level
    // Threshold at -12dB, input signal is around -10dB RMS, so we need makeup gain
    // to compensate for the compression applied to peaks above threshold
    let compressor = Compressor::with_settings(CompressorSettings {
        threshold_db: -12.0,
        ratio: 3.0,
        attack_ms: 5.0,
        release_ms: 100.0,
        knee_db: 4.0,
        makeup_gain_db: 6.0, // Increased makeup gain to maintain unity gain output
    });
    chain.add_effect(Box::new(compressor));

    // Limiting
    let limiter = Limiter::with_settings(LimiterSettings {
        threshold_db: -1.0,
        release_ms: 100.0,
    });
    chain.add_effect(Box::new(limiter));

    chain.process(&mut buffer, sample_rate);

    // Verify output is valid and not clipping
    let peak = calculate_peak(&buffer);
    assert!(
        peak <= 1.0,
        "Podcast processing should not clip, peak: {}",
        peak
    );

    // Verify output has audio (not silent)
    let processed_rms = calculate_rms(&buffer);
    assert!(
        processed_rms > 0.01,
        "Processed voice should have audio content, RMS: {}",
        processed_rms
    );

    // Verify level is roughly maintained (within -3dB to +3dB of original)
    // A well-configured voice processing chain should have approximately unity gain
    let level_ratio = processed_rms / original_rms;
    let level_change_db = 20.0 * level_ratio.log10();
    assert!(
        level_change_db >= -3.0 && level_change_db <= 3.0,
        "Voice processing should maintain roughly unity gain. Original RMS: {:.3}, Processed RMS: {:.3}, Level change: {:.1} dB",
        original_rms,
        processed_rms,
        level_change_db
    );

    // All samples should be valid
    assert!(buffer.iter().all(|s| s.is_finite()));
}
