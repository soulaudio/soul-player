//! Property-based tests for audio engine
//!
//! These tests use proptest to verify invariants across many random inputs.

use proptest::prelude::*;
use soul_audio::effects::*;

// Helper: Check if buffer contains only finite values
fn all_finite(buffer: &[f32]) -> bool {
    buffer.iter().all(|s| s.is_finite())
}

// Helper: Calculate peak
fn peak(buffer: &[f32]) -> f32 {
    buffer.iter().map(|s| s.abs()).fold(0.0f32, f32::max)
}

proptest! {
    /// Property: EQ should never produce NaN or Inf, regardless of input
    #[test]
    fn eq_never_produces_nan_or_inf(
        freq in 20.0f32..20000.0,
        gain_db in -12.0f32..12.0,
        q in 0.1f32..10.0,
        samples in prop::collection::vec(-1.0f32..1.0, 100..1000)
    ) {
        let mut eq = ParametricEq::new();
        eq.set_mid_band(EqBand::peaking(freq, gain_db, q));

        let mut buffer = samples;
        eq.process(&mut buffer, 44100);

        prop_assert!(all_finite(&buffer), "EQ produced NaN or Inf");
    }

    /// Property: Compressor should never produce NaN or Inf
    #[test]
    fn compressor_never_produces_nan_or_inf(
        threshold_db in -60.0f32..0.0,
        ratio in 1.0f32..20.0,
        attack_ms in 0.1f32..100.0,
        release_ms in 10.0f32..1000.0,
        samples in prop::collection::vec(-1.0f32..1.0, 100..1000)
    ) {
        let mut comp = Compressor::new();
        comp.set_threshold(threshold_db);
        comp.set_ratio(ratio);
        comp.set_attack(attack_ms);
        comp.set_release(release_ms);

        let mut buffer = samples;
        comp.process(&mut buffer, 44100);

        prop_assert!(all_finite(&buffer), "Compressor produced NaN or Inf");
    }

    /// Property: Disabled effects should not modify audio at all
    #[test]
    fn disabled_effects_are_true_bypass(
        effect_type in 0u8..2, // 0 = EQ, 1 = Compressor
        samples in prop::collection::vec(-1.0f32..1.0, 100..1000)
    ) {
        let mut buffer = samples.clone();
        let original = samples;

        match effect_type {
            0 => {
                let mut eq = ParametricEq::new();
                eq.set_enabled(false);
                eq.process(&mut buffer, 44100);
            }
            1 => {
                let mut comp = Compressor::new();
                comp.set_enabled(false);
                comp.process(&mut buffer, 44100);
            }
            _ => {}
        }

        prop_assert_eq!(buffer, original, "Disabled effect modified audio");
    }

    /// Property: EQ with 0 dB gain should not significantly change audio
    #[test]
    fn eq_with_zero_gain_is_nearly_transparent(
        freq in 100.0f32..10000.0,
        samples in prop::collection::vec(-0.5f32..0.5, 1000..2000)
    ) {
        let mut eq = ParametricEq::new();
        eq.set_low_band(EqBand::low_shelf(freq, 0.0));  // 0 dB
        eq.set_mid_band(EqBand::peaking(freq, 0.0, 1.0)); // 0 dB
        eq.set_high_band(EqBand::high_shelf(freq, 0.0)); // 0 dB

        let original = samples.clone();
        let mut buffer = samples;

        eq.process(&mut buffer, 44100);

        // Calculate difference
        let max_diff = original.iter()
            .zip(buffer.iter())
            .map(|(o, p)| (o - p).abs())
            .fold(0.0f32, f32::max);

        // Should be very close (allow small filter artifacts)
        prop_assert!(max_diff < 0.1, "0 dB EQ changed audio too much: {}", max_diff);
    }

    /// Property: Compressor with ratio 1.0 should not compress
    #[test]
    fn compressor_with_ratio_one_is_transparent(
        threshold in -40.0f32..-10.0,
        samples in prop::collection::vec(-0.5f32..0.5, 1000..2000)
    ) {
        let mut comp = Compressor::new();
        comp.set_threshold(threshold);
        comp.set_ratio(1.0); // No compression
        comp.set_makeup_gain(0.0);

        let original_peak = peak(&samples);
        let mut buffer = samples;

        comp.process(&mut buffer, 44100);

        let processed_peak = peak(&buffer);

        // Peak should not be significantly reduced (allow envelope follower smoothing)
        let ratio = processed_peak / original_peak;
        prop_assert!(ratio > 0.8, "Ratio 1.0 compressor reduced peaks: {}", ratio);
    }

    /// Property: Effect chain should preserve audio length
    #[test]
    fn effect_chain_preserves_length(
        num_effects in 1usize..10,
        samples in prop::collection::vec(-1.0f32..1.0, 100..1000)
    ) {
        let mut chain = EffectChain::new();

        for _ in 0..num_effects {
            chain.add_effect(Box::new(ParametricEq::new()));
        }

        let original_len = samples.len();
        let mut buffer = samples;

        chain.process(&mut buffer, 44100);

        prop_assert_eq!(buffer.len(), original_len, "Chain changed buffer length");
    }

    /// Property: Extreme EQ boost should increase signal level
    #[test]
    fn extreme_eq_boost_increases_level(
        freq in 100.0f32..5000.0,
        samples in prop::collection::vec(0.01f32..0.1, 1000..2000) // Non-zero signal
    ) {
        let mut eq = ParametricEq::new();
        eq.set_mid_band(EqBand::peaking(freq, 12.0, 2.0)); // Max boost

        let original_peak = peak(&samples);
        let mut buffer = samples;

        eq.process(&mut buffer, 44100);

        let processed_peak = peak(&buffer);

        // Should increase (or at least not decrease significantly)
        prop_assert!(
            processed_peak >= original_peak * 0.9,
            "Positive EQ gain decreased signal: {} -> {}",
            original_peak,
            processed_peak
        );
    }

    /// Property: Compressor should not increase peaks (without makeup gain)
    #[test]
    fn compressor_reduces_or_maintains_peaks(
        threshold in -30.0f32..-10.0,
        ratio in 2.0f32..10.0,
        samples in prop::collection::vec(0.3f32..0.9, 1000..2000) // Loud signals
    ) {
        let mut comp = Compressor::new();
        comp.set_threshold(threshold);
        comp.set_ratio(ratio);
        comp.set_makeup_gain(0.0); // No makeup gain

        let original_peak = peak(&samples);
        let mut buffer = samples;

        comp.process(&mut buffer, 44100);

        let processed_peak = peak(&buffer);

        // Peaks should not increase (allow small overshoot during attack)
        prop_assert!(
            processed_peak <= original_peak * 1.1,
            "Compressor increased peaks: {} -> {}",
            original_peak,
            processed_peak
        );
    }

    /// Property: Reset should clear effect state
    #[test]
    fn reset_clears_state_deterministically(
        samples1 in prop::collection::vec(-0.5f32..0.5, 500..1000),
        samples2 in prop::collection::vec(-0.5f32..0.5, 500..1000),
    ) {
        let mut eq = ParametricEq::new();
        eq.set_low_band(EqBand::low_shelf(100.0, 6.0));

        // Process first buffer
        let mut buffer1 = samples1;
        eq.process(&mut buffer1, 44100);

        // Reset
        eq.reset();

        // Process second buffer (different samples)
        let mut buffer2 = samples2.clone();
        eq.process(&mut buffer2, 44100);

        // Reset again
        eq.reset();

        // Process second buffer again - should get same result
        let mut buffer3 = samples2;
        eq.process(&mut buffer3, 44100);

        prop_assert_eq!(buffer2, buffer3, "Reset not deterministic");
    }

    /// Property: Multiple sample rates should produce finite output
    #[test]
    fn multiple_sample_rates_produce_finite_output(
        sample_rate in prop::sample::select(vec![22050u32, 44100, 48000, 88200, 96000]),
        samples in prop::collection::vec(-1.0f32..1.0, 100..1000)
    ) {
        let mut chain = EffectChain::new();
        chain.add_effect(Box::new(ParametricEq::new()));
        chain.add_effect(Box::new(Compressor::new()));

        let mut buffer = samples;
        chain.process(&mut buffer, sample_rate);

        prop_assert!(all_finite(&buffer), "Non-finite output at {} Hz", sample_rate);
    }

    /// Property: EQ cut should not significantly increase signal level
    #[test]
    fn eq_cut_does_not_boost(
        freq in 100.0f32..5000.0,
        samples in prop::collection::vec(0.3f32..0.7, 1000..2000) // Moderate signal
    ) {
        let mut eq = ParametricEq::new();
        eq.set_mid_band(EqBand::peaking(freq, -12.0, 2.0)); // Max cut

        let original_peak = peak(&samples);
        let mut buffer = samples;

        eq.process(&mut buffer, 44100);

        let processed_peak = peak(&buffer);

        // Cut should not increase signal (allow small overshoot due to filter transients)
        prop_assert!(
            processed_peak <= original_peak * 1.3,
            "Negative EQ gain significantly increased signal: {} -> {}",
            original_peak,
            processed_peak
        );
    }

    /// Property: Chain with all disabled effects equals bypass
    #[test]
    fn chain_all_disabled_is_bypass(
        num_effects in 1usize..5,
        samples in prop::collection::vec(-1.0f32..1.0, 100..1000)
    ) {
        let mut chain = EffectChain::new();

        for _ in 0..num_effects {
            let mut eq = ParametricEq::new();
            eq.set_enabled(false);
            chain.add_effect(Box::new(eq));
        }

        let original = samples.clone();
        let mut buffer = samples;

        chain.process(&mut buffer, 44100);

        prop_assert_eq!(buffer, original, "All-disabled chain modified audio");
    }

    /// Property: Effect processing is consistent across multiple calls
    #[test]
    fn processing_is_consistent(
        samples in prop::collection::vec(-0.5f32..0.5, 500..1000)
    ) {
        let mut eq = ParametricEq::new();
        eq.set_low_band(EqBand::low_shelf(100.0, 3.0));

        // Process same input twice
        let mut buffer1 = samples.clone();
        eq.process(&mut buffer1, 44100);

        eq.reset(); // Reset state

        let mut buffer2 = samples;
        eq.process(&mut buffer2, 44100);

        prop_assert_eq!(buffer1, buffer2, "Processing not consistent");
    }

    /// Property: Empty buffer doesn't cause panic or corruption
    #[test]
    fn empty_buffer_handled_safely(
        effect_type in 0u8..2
    ) {
        let mut buffer: Vec<f32> = Vec::new();

        match effect_type {
            0 => {
                let mut eq = ParametricEq::new();
                eq.process(&mut buffer, 44100);
            }
            1 => {
                let mut comp = Compressor::new();
                comp.process(&mut buffer, 44100);
            }
            _ => {}
        }

        prop_assert!(buffer.is_empty(), "Empty buffer was modified");
    }
}
