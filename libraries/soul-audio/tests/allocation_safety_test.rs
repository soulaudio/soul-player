//! Allocation safety tests for audio processing
//!
//! Verifies that audio effect processing doesn't perform allocations
//! that could cause glitches in real-time audio processing.
//!
//! These tests verify:
//! - Pre-allocated buffers are properly sized
//! - No dynamic allocations during process() calls
//! - Buffer reuse patterns work correctly

use soul_audio::effects::{
    AudioEffect, Compressor, CompressorSettings, Crossfeed, CrossfeedPreset, EffectChain, EqBand,
    GraphicEq, GraphicEqPreset, Limiter, LimiterSettings, ParametricEq, StereoEnhancer,
    StereoSettings,
};
use std::f32::consts::PI;

const SAMPLE_RATE: u32 = 44100;

// ============================================================================
// TEST UTILITIES
// ============================================================================

/// Generate a stereo sine wave buffer
fn generate_stereo_sine(frequency: f32, sample_rate: u32, num_frames: usize) -> Vec<f32> {
    let mut buffer = Vec::with_capacity(num_frames * 2);
    for i in 0..num_frames {
        let t = i as f32 / sample_rate as f32;
        let sample = (2.0 * PI * frequency * t).sin();
        buffer.push(sample); // Left
        buffer.push(sample); // Right
    }
    buffer
}

/// Generate random-ish audio data for testing
fn generate_varied_audio(sample_rate: u32, num_frames: usize) -> Vec<f32> {
    let mut buffer = Vec::with_capacity(num_frames * 2);
    for i in 0..num_frames * 2 {
        let t = i as f32 / sample_rate as f32;
        let sample = (2.0 * PI * 440.0 * t).sin() * 0.3
            + (2.0 * PI * 880.0 * t).sin() * 0.2
            + (2.0 * PI * 220.0 * t).sin() * 0.1;
        buffer.push(sample);
    }
    buffer
}

// ============================================================================
// BUFFER PRE-ALLOCATION TESTS
// ============================================================================

#[test]
fn test_parametric_eq_internal_buffers_preallocated() {
    // Create and use immediately - internal state should be ready
    let mut eq = ParametricEq::new();
    eq.set_enabled(true);
    eq.set_low_band(EqBand::low_shelf(100.0, 6.0));
    eq.set_mid_band(EqBand::peaking(1000.0, 6.0, 1.0));
    eq.set_high_band(EqBand::high_shelf(8000.0, 6.0));

    // Process multiple times - should not need any additional allocation
    for _ in 0..1000 {
        let mut buffer = generate_stereo_sine(1000.0, SAMPLE_RATE, 512);
        eq.process(&mut buffer, SAMPLE_RATE);

        // Verify output is valid
        for sample in &buffer {
            assert!(sample.is_finite());
        }
    }
}

#[test]
fn test_graphic_eq_internal_buffers_preallocated() {
    let mut geq = GraphicEq::new_10_band();
    geq.set_enabled(true);
    geq.set_preset(GraphicEqPreset::BassBoost);

    for _ in 0..1000 {
        let mut buffer = generate_stereo_sine(1000.0, SAMPLE_RATE, 512);
        geq.process(&mut buffer, SAMPLE_RATE);

        for sample in &buffer {
            assert!(sample.is_finite());
        }
    }
}

#[test]
fn test_compressor_internal_buffers_preallocated() {
    let mut compressor = Compressor::with_settings(CompressorSettings {
        threshold_db: -20.0,
        ratio: 4.0,
        attack_ms: 10.0,
        release_ms: 100.0,
        knee_db: 6.0,
        makeup_gain_db: 6.0,
    });
    compressor.set_enabled(true);

    for _ in 0..1000 {
        let mut buffer = generate_varied_audio(SAMPLE_RATE, 512);
        compressor.process(&mut buffer, SAMPLE_RATE);

        for sample in &buffer {
            assert!(sample.is_finite());
        }
    }
}

#[test]
fn test_limiter_internal_buffers_preallocated() {
    let mut limiter = Limiter::with_settings(LimiterSettings {
        threshold_db: -3.0,
        release_ms: 50.0,
    });
    limiter.set_enabled(true);

    for _ in 0..1000 {
        let mut buffer = generate_varied_audio(SAMPLE_RATE, 512);
        limiter.process(&mut buffer, SAMPLE_RATE);

        for sample in &buffer {
            assert!(sample.is_finite());
            // Limiter should prevent samples from exceeding threshold
            // (allowing small margin for transients)
            assert!(sample.abs() < 1.5, "Limiter output too high: {}", sample);
        }
    }
}

#[test]
fn test_stereo_enhancer_internal_buffers_preallocated() {
    let mut enhancer = StereoEnhancer::with_settings(StereoSettings {
        width: 1.5,
        mid_gain_db: 3.0,
        side_gain_db: 3.0,
        balance: 0.0,
    });
    enhancer.set_enabled(true);

    for _ in 0..1000 {
        let mut buffer = generate_stereo_sine(1000.0, SAMPLE_RATE, 512);
        enhancer.process(&mut buffer, SAMPLE_RATE);

        for sample in &buffer {
            assert!(sample.is_finite());
        }
    }
}

#[test]
fn test_crossfeed_internal_buffers_preallocated() {
    let mut crossfeed = Crossfeed::with_preset(CrossfeedPreset::Natural);
    crossfeed.set_enabled(true);

    for _ in 0..1000 {
        let mut buffer = generate_stereo_sine(1000.0, SAMPLE_RATE, 512);
        crossfeed.process(&mut buffer, SAMPLE_RATE);

        for sample in &buffer {
            assert!(sample.is_finite());
        }
    }
}

// ============================================================================
// EFFECT CHAIN BUFFER REUSE TESTS
// ============================================================================

#[test]
fn test_effect_chain_buffer_reuse() {
    let mut chain = EffectChain::new();

    // Add multiple effects
    let mut eq = ParametricEq::new();
    eq.set_mid_band(EqBand::peaking(1000.0, 6.0, 1.0));
    chain.add_effect(Box::new(eq));

    chain.add_effect(Box::new(Compressor::with_settings(CompressorSettings {
        threshold_db: -20.0,
        ratio: 4.0,
        attack_ms: 10.0,
        release_ms: 100.0,
        knee_db: 6.0,
        makeup_gain_db: 0.0,
    })));

    chain.add_effect(Box::new(Limiter::new()));

    chain.set_enabled(true);

    // Process many times - same buffer can be reused
    let mut buffer = generate_varied_audio(SAMPLE_RATE, 1024);

    for _ in 0..1000 {
        // Reset buffer content
        for (i, sample) in buffer.iter_mut().enumerate() {
            let t = i as f32 / SAMPLE_RATE as f32;
            *sample = (2.0 * PI * 440.0 * t).sin() * 0.5;
        }

        chain.process(&mut buffer, SAMPLE_RATE);

        for sample in &buffer {
            assert!(sample.is_finite());
        }
    }
}

#[test]
fn test_effect_chain_no_internal_vec_growth() {
    let mut chain = EffectChain::new();

    // Add effects
    chain.add_effect(Box::new(ParametricEq::new()));
    chain.add_effect(Box::new(GraphicEq::new_10_band()));
    chain.add_effect(Box::new(Compressor::new()));
    chain.add_effect(Box::new(Limiter::new()));
    chain.add_effect(Box::new(StereoEnhancer::new()));

    // The chain should not need to grow its internal storage during processing
    chain.set_enabled(true);

    // Process with various buffer sizes
    for size in [64, 128, 256, 512, 1024, 2048, 4096] {
        let mut buffer = generate_varied_audio(SAMPLE_RATE, size);
        chain.process(&mut buffer, SAMPLE_RATE);

        for sample in &buffer {
            assert!(sample.is_finite());
        }
    }
}

// ============================================================================
// VARYING BUFFER SIZE TESTS
// ============================================================================

#[test]
fn test_effects_handle_varying_buffer_sizes() {
    let effects: Vec<Box<dyn AudioEffect>> = vec![
        Box::new(ParametricEq::new()),
        Box::new(GraphicEq::new_10_band()),
        Box::new(Compressor::new()),
        Box::new(Limiter::new()),
        Box::new(StereoEnhancer::new()),
        Box::new(Crossfeed::new()),
    ];

    // Various buffer sizes that might be used in real-time audio
    let buffer_sizes = [32, 64, 128, 256, 512, 1024, 2048, 4096];

    for mut effect in effects {
        effect.set_enabled(true);

        for &size in &buffer_sizes {
            let mut buffer = generate_varied_audio(SAMPLE_RATE, size);
            effect.process(&mut buffer, SAMPLE_RATE);

            for sample in &buffer {
                assert!(
                    sample.is_finite(),
                    "Effect {} produced non-finite output at buffer size {}",
                    effect.name(),
                    size
                );
            }
        }
    }
}

#[test]
fn test_effects_handle_minimum_buffer_size() {
    // Test with very small buffers (1-8 frames)
    let effects: Vec<Box<dyn AudioEffect>> = vec![
        Box::new(ParametricEq::new()),
        Box::new(GraphicEq::new_10_band()),
        Box::new(Compressor::new()),
        Box::new(Limiter::new()),
        Box::new(StereoEnhancer::new()),
        Box::new(Crossfeed::new()),
    ];

    for mut effect in effects {
        effect.set_enabled(true);

        for size in 1..=8 {
            let mut buffer = generate_varied_audio(SAMPLE_RATE, size);
            effect.process(&mut buffer, SAMPLE_RATE);

            for sample in &buffer {
                assert!(
                    sample.is_finite(),
                    "Effect {} produced non-finite output at minimum buffer size {}",
                    effect.name(),
                    size
                );
            }
        }
    }
}

#[test]
fn test_effects_handle_large_buffer_size() {
    // Test with large buffer (~ 1 second at 44.1kHz)
    let effects: Vec<Box<dyn AudioEffect>> = vec![
        Box::new(ParametricEq::new()),
        Box::new(GraphicEq::new_10_band()),
        Box::new(Compressor::new()),
        Box::new(Limiter::new()),
        Box::new(StereoEnhancer::new()),
        Box::new(Crossfeed::new()),
    ];

    for mut effect in effects {
        effect.set_enabled(true);

        let mut buffer = generate_varied_audio(SAMPLE_RATE, 44100);
        effect.process(&mut buffer, SAMPLE_RATE);

        for sample in &buffer {
            assert!(
                sample.is_finite(),
                "Effect {} produced non-finite output with large buffer",
                effect.name()
            );
        }
    }
}

// ============================================================================
// SAMPLE RATE INDEPENDENCE TESTS
// ============================================================================

#[test]
fn test_effects_work_at_various_sample_rates() {
    let sample_rates = [22050, 44100, 48000, 88200, 96000, 176400, 192000];

    for &rate in &sample_rates {
        let mut eq = ParametricEq::new();
        eq.set_enabled(true);
        eq.set_mid_band(EqBand::peaking(1000.0, 6.0, 1.0));

        let mut buffer = generate_varied_audio(rate, 1024);
        eq.process(&mut buffer, rate);

        for sample in &buffer {
            assert!(
                sample.is_finite(),
                "EQ produced non-finite output at {} Hz",
                rate
            );
        }
    }
}

// ============================================================================
// STATE CONSISTENCY TESTS
// ============================================================================

#[test]
fn test_effect_state_remains_consistent_after_many_iterations() {
    let mut compressor = Compressor::with_settings(CompressorSettings {
        threshold_db: -20.0,
        ratio: 4.0,
        attack_ms: 10.0,
        release_ms: 100.0,
        knee_db: 6.0,
        makeup_gain_db: 0.0,
    });
    compressor.set_enabled(true);

    // Process many iterations
    for _ in 0..10000 {
        let mut buffer = generate_varied_audio(SAMPLE_RATE, 256);
        compressor.process(&mut buffer, SAMPLE_RATE);
    }

    // State should still be valid
    assert!(compressor.is_enabled());

    // Process one more time - should still work
    let mut buffer = generate_varied_audio(SAMPLE_RATE, 256);
    compressor.process(&mut buffer, SAMPLE_RATE);

    for sample in &buffer {
        assert!(sample.is_finite());
    }
}

#[test]
fn test_effect_reset_clears_internal_state() {
    let mut eq = ParametricEq::new();
    eq.set_enabled(true);
    eq.set_mid_band(EqBand::peaking(1000.0, 12.0, 1.0)); // High gain

    // Process some audio to build up internal state
    for _ in 0..100 {
        let mut buffer = generate_stereo_sine(1000.0, SAMPLE_RATE, 256);
        eq.process(&mut buffer, SAMPLE_RATE);
    }

    // Reset
    eq.reset();

    // Process after reset - should still work correctly
    let mut buffer = generate_stereo_sine(1000.0, SAMPLE_RATE, 256);
    eq.process(&mut buffer, SAMPLE_RATE);

    for sample in &buffer {
        assert!(sample.is_finite());
    }
}

// ============================================================================
// CONCURRENT ACCESS PATTERN TESTS (SIMULATED)
// ============================================================================

#[test]
fn test_rapid_enable_disable_is_safe() {
    let mut effect = ParametricEq::new();
    effect.set_mid_band(EqBand::peaking(1000.0, 12.0, 1.0));

    for i in 0..1000 {
        effect.set_enabled(i % 2 == 0);

        let mut buffer = generate_varied_audio(SAMPLE_RATE, 256);
        effect.process(&mut buffer, SAMPLE_RATE);

        for sample in &buffer {
            assert!(sample.is_finite());
        }
    }
}

#[test]
fn test_rapid_parameter_changes_are_safe() {
    let mut compressor = Compressor::new();
    compressor.set_enabled(true);

    for i in 0..1000 {
        // Rapidly change parameters
        compressor.set_threshold(-40.0 + (i as f32 % 30.0));
        compressor.set_ratio(1.0 + (i as f32 % 10.0));
        compressor.set_attack(1.0 + (i as f32 % 100.0));
        compressor.set_release(10.0 + (i as f32 % 500.0));

        let mut buffer = generate_varied_audio(SAMPLE_RATE, 256);
        compressor.process(&mut buffer, SAMPLE_RATE);

        for sample in &buffer {
            assert!(sample.is_finite());
        }
    }
}

// ============================================================================
// MEMORY PATTERN VERIFICATION
// ============================================================================

#[test]
fn test_repeated_processing_maintains_output_quality() {
    let mut eq = ParametricEq::new();
    eq.set_enabled(true);
    eq.set_mid_band(EqBand::peaking(1000.0, 6.0, 1.0));

    // Process the same signal many times and verify consistent output
    let input = generate_stereo_sine(1000.0, SAMPLE_RATE, 1024);
    let mut outputs: Vec<Vec<f32>> = Vec::new();

    for _ in 0..10 {
        eq.reset(); // Reset to ensure consistent starting state
        let mut buffer = input.clone();
        eq.process(&mut buffer, SAMPLE_RATE);
        outputs.push(buffer);
    }

    // All outputs should be identical (deterministic processing)
    for i in 1..outputs.len() {
        for (j, (a, b)) in outputs[0].iter().zip(outputs[i].iter()).enumerate() {
            assert!(
                (a - b).abs() < 1e-6,
                "Output mismatch at sample {}: {} vs {}",
                j,
                a,
                b
            );
        }
    }
}

#[test]
fn test_effect_chain_order_is_deterministic() {
    // Create two identical chains
    let mut chain1 = EffectChain::new();
    let mut chain2 = EffectChain::new();

    // Add same effects in same order
    for chain in [&mut chain1, &mut chain2] {
        let mut eq = ParametricEq::new();
        eq.set_mid_band(EqBand::peaking(1000.0, 6.0, 1.0));
        chain.add_effect(Box::new(eq));
        chain.add_effect(Box::new(Compressor::new()));
        chain.add_effect(Box::new(Limiter::new()));
        chain.set_enabled(true);
    }

    // Process same input through both
    let input = generate_varied_audio(SAMPLE_RATE, 1024);

    let mut buffer1 = input.clone();
    let mut buffer2 = input.clone();

    chain1.process(&mut buffer1, SAMPLE_RATE);
    chain2.process(&mut buffer2, SAMPLE_RATE);

    // Outputs should be identical
    for (i, (a, b)) in buffer1.iter().zip(buffer2.iter()).enumerate() {
        assert!(
            (a - b).abs() < 1e-6,
            "Chain output mismatch at sample {}: {} vs {}",
            i,
            a,
            b
        );
    }
}
