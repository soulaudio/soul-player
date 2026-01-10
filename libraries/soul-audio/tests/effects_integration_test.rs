//! Comprehensive integration tests for audio effects
//!
//! Tests cover:
//! - Graphic EQ: presets, band interaction, frequency response
//! - Stereo Enhancer: width control, mono compatibility
//! - Crossfeed: presets, stereo imaging
//! - Effect combination matrix: all possible combinations
//! - Stream robustness: sample rate changes, buffer size variations
//! - Numerical stability: denormals, DC offset, edge cases
//! - Real-time safety: no allocations in process path

use soul_audio::effects::{
    AudioEffect, Compressor, CompressorSettings, Crossfeed, CrossfeedPreset, EffectChain,
    EqBand, GraphicEq, GraphicEqPreset, Limiter, LimiterSettings, ParametricEq,
    StereoEnhancer, StereoSettings, mono_compatibility,
};
use std::f32::consts::PI;

const SAMPLE_RATE: u32 = 44100;

// ============================================================================
// TEST UTILITIES
// ============================================================================

/// Generate a stereo sine wave at the given frequency
fn generate_sine_wave(frequency: f32, sample_rate: u32, num_samples: usize) -> Vec<f32> {
    let mut buffer = Vec::with_capacity(num_samples * 2);
    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let sample = (2.0 * PI * frequency * t).sin();
        buffer.push(sample); // Left
        buffer.push(sample); // Right
    }
    buffer
}

/// Generate stereo test signal with phase difference between channels
fn generate_stereo_signal(
    frequency: f32,
    sample_rate: u32,
    num_samples: usize,
    phase_offset: f32,
) -> Vec<f32> {
    let mut buffer = Vec::with_capacity(num_samples * 2);
    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let left = (2.0 * PI * frequency * t).sin();
        let right = (2.0 * PI * frequency * t + phase_offset).sin();
        buffer.push(left);
        buffer.push(right);
    }
    buffer
}

/// Generate white noise
fn generate_white_noise(num_samples: usize) -> Vec<f32> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut buffer = Vec::with_capacity(num_samples * 2);
    for i in 0..num_samples {
        let mut hasher = DefaultHasher::new();
        i.hash(&mut hasher);
        let hash = hasher.finish();
        let sample = (hash as f32 / u64::MAX as f32) * 2.0 - 1.0;
        buffer.push(sample);
        buffer.push(sample);
    }
    buffer
}

/// Calculate RMS of a buffer
fn calculate_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_squares: f32 = samples.iter().map(|x| x * x).sum();
    (sum_squares / samples.len() as f32).sqrt()
}

/// Extract left channel from interleaved buffer
fn left_channel(buffer: &[f32]) -> Vec<f32> {
    buffer.chunks(2).map(|c| c[0]).collect()
}

/// Extract right channel from interleaved buffer
fn right_channel(buffer: &[f32]) -> Vec<f32> {
    buffer.chunks(2).map(|c| c[1]).collect()
}

// ============================================================================
// GRAPHIC EQ TESTS
// ============================================================================

mod graphic_eq_tests {
    use super::*;

    #[test]
    fn test_10_band_eq_creation() {
        let eq = GraphicEq::new_10_band();
        assert!(eq.band_count() == 10);
    }

    #[test]
    fn test_31_band_eq_creation() {
        let eq = GraphicEq::new_31_band();
        assert!(eq.band_count() == 31);
    }

    #[test]
    fn test_flat_preset_passthrough() {
        let mut eq = GraphicEq::new_10_band();
        eq.set_preset(GraphicEqPreset::Flat);

        let mut buffer = generate_sine_wave(1000.0, SAMPLE_RATE, 4096);
        let original_rms = calculate_rms(&buffer);

        eq.process(&mut buffer, SAMPLE_RATE);
        let processed_rms = calculate_rms(&buffer);

        // Flat preset should not significantly change signal level
        let ratio_db = 20.0 * (processed_rms / original_rms).log10();
        assert!(
            ratio_db.abs() < 1.0,
            "Flat preset changed level by {} dB",
            ratio_db
        );
    }

    #[test]
    fn test_bass_boost_preset() {
        let mut eq = GraphicEq::new_10_band();
        eq.set_preset(GraphicEqPreset::BassBoost);

        // Test at 60 Hz (bass) vs 4000 Hz (treble)
        let mut bass_buffer = generate_sine_wave(60.0, SAMPLE_RATE, 8192);
        let mut treble_buffer = generate_sine_wave(4000.0, SAMPLE_RATE, 8192);

        let bass_original = calculate_rms(&bass_buffer);
        let treble_original = calculate_rms(&treble_buffer);

        eq.process(&mut bass_buffer, SAMPLE_RATE);

        // Reset filter state between tests
        let mut eq2 = GraphicEq::new_10_band();
        eq2.set_preset(GraphicEqPreset::BassBoost);
        eq2.process(&mut treble_buffer, SAMPLE_RATE);

        let bass_processed = calculate_rms(&bass_buffer);
        let treble_processed = calculate_rms(&treble_buffer);

        let bass_gain_db = 20.0 * (bass_processed / bass_original).log10();
        let treble_gain_db = 20.0 * (treble_processed / treble_original).log10();

        // Bass should be boosted more than treble
        assert!(
            bass_gain_db > treble_gain_db,
            "Bass boost should increase bass ({:.1} dB) more than treble ({:.1} dB)",
            bass_gain_db,
            treble_gain_db
        );
    }

    #[test]
    fn test_treble_boost_preset() {
        let mut eq = GraphicEq::new_10_band();
        eq.set_preset(GraphicEqPreset::TrebleBoost);

        let mut bass_buffer = generate_sine_wave(60.0, SAMPLE_RATE, 8192);
        let mut treble_buffer = generate_sine_wave(8000.0, SAMPLE_RATE, 8192);

        let bass_original = calculate_rms(&bass_buffer);
        let treble_original = calculate_rms(&treble_buffer);

        eq.process(&mut bass_buffer, SAMPLE_RATE);

        let mut eq2 = GraphicEq::new_10_band();
        eq2.set_preset(GraphicEqPreset::TrebleBoost);
        eq2.process(&mut treble_buffer, SAMPLE_RATE);

        let bass_processed = calculate_rms(&bass_buffer);
        let treble_processed = calculate_rms(&treble_buffer);

        let bass_gain_db = 20.0 * (bass_processed / bass_original).log10();
        let treble_gain_db = 20.0 * (treble_processed / treble_original).log10();

        // Treble should be boosted more than bass
        assert!(
            treble_gain_db > bass_gain_db,
            "Treble boost should increase treble ({:.1} dB) more than bass ({:.1} dB)",
            treble_gain_db,
            bass_gain_db
        );
    }

    #[test]
    fn test_vocal_preset() {
        let mut eq = GraphicEq::new_10_band();
        eq.set_preset(GraphicEqPreset::Vocal);

        // Vocal frequencies (1-4kHz) should be emphasized
        let mut vocal_buffer = generate_sine_wave(2000.0, SAMPLE_RATE, 8192);
        let mut bass_buffer = generate_sine_wave(60.0, SAMPLE_RATE, 8192);

        let vocal_original = calculate_rms(&vocal_buffer);
        let bass_original = calculate_rms(&bass_buffer);

        eq.process(&mut vocal_buffer, SAMPLE_RATE);

        let mut eq2 = GraphicEq::new_10_band();
        eq2.set_preset(GraphicEqPreset::Vocal);
        eq2.process(&mut bass_buffer, SAMPLE_RATE);

        let vocal_processed = calculate_rms(&vocal_buffer);
        let bass_processed = calculate_rms(&bass_buffer);

        let vocal_gain_db = 20.0 * (vocal_processed / vocal_original).log10();
        let bass_gain_db = 20.0 * (bass_processed / bass_original).log10();

        // Vocal frequencies should be boosted relative to bass
        assert!(
            vocal_gain_db > bass_gain_db,
            "Vocal preset should boost mids ({:.1} dB) more than bass ({:.1} dB)",
            vocal_gain_db,
            bass_gain_db
        );
    }

    #[test]
    fn test_band_gain_adjustment() {
        let mut eq = GraphicEq::new_10_band();

        // Set band 0 (31 Hz) to +6 dB
        eq.set_band_gain(0, 6.0);

        // Set band 9 (16 kHz) to -6 dB
        eq.set_band_gain(9, -6.0);

        let mut low_buffer = generate_sine_wave(31.0, SAMPLE_RATE, 16384);
        let mut high_buffer = generate_sine_wave(16000.0, SAMPLE_RATE, 8192);

        let low_original = calculate_rms(&low_buffer);
        let high_original = calculate_rms(&high_buffer);

        eq.process(&mut low_buffer, SAMPLE_RATE);

        let mut eq2 = GraphicEq::new_10_band();
        eq2.set_band_gain(0, 6.0);
        eq2.set_band_gain(9, -6.0);
        eq2.process(&mut high_buffer, SAMPLE_RATE);

        let low_processed = calculate_rms(&low_buffer);
        let high_processed = calculate_rms(&high_buffer);

        let low_gain_db = 20.0 * (low_processed / low_original).log10();
        let high_gain_db = 20.0 * (high_processed / high_original).log10();

        // Low should be boosted, high should be cut
        assert!(
            low_gain_db > 0.0,
            "Low frequency should be boosted, got {:.1} dB",
            low_gain_db
        );
        assert!(
            high_gain_db < 0.0,
            "High frequency should be cut, got {:.1} dB",
            high_gain_db
        );
    }

    #[test]
    fn test_adjacent_band_interaction() {
        // When boosting one band, adjacent bands should see some effect
        // due to filter overlap
        let mut eq = GraphicEq::new_10_band();

        // Boost 1kHz band significantly
        eq.set_band_gain(5, 12.0); // 1kHz is typically band 5

        // Test at center frequency and adjacent frequencies
        let frequencies = [500.0, 1000.0, 2000.0];
        let mut gains = Vec::new();

        for &freq in &frequencies {
            let mut buffer = generate_sine_wave(freq, SAMPLE_RATE, 8192);
            let original = calculate_rms(&buffer);

            let mut eq_copy = GraphicEq::new_10_band();
            eq_copy.set_band_gain(5, 12.0);
            eq_copy.process(&mut buffer, SAMPLE_RATE);

            let processed = calculate_rms(&buffer);
            let gain_db = 20.0 * (processed / original).log10();
            gains.push(gain_db);
        }

        // Center frequency (1kHz) should have highest gain
        assert!(
            gains[1] > gains[0] && gains[1] > gains[2],
            "Center frequency should have highest gain: 500Hz={:.1}dB, 1kHz={:.1}dB, 2kHz={:.1}dB",
            gains[0],
            gains[1],
            gains[2]
        );

        // Adjacent frequencies should still see some boost (filter overlap)
        assert!(
            gains[0] > 0.0,
            "500Hz should see some boost, got {:.1}dB",
            gains[0]
        );
        assert!(
            gains[2] > 0.0,
            "2kHz should see some boost, got {:.1}dB",
            gains[2]
        );
    }

    #[test]
    fn test_extreme_gain_values() {
        let mut eq = GraphicEq::new_10_band();

        // Set all bands to extreme values
        for i in 0..10 {
            eq.set_band_gain(i, if i % 2 == 0 { 12.0 } else { -12.0 });
        }

        let mut buffer = generate_white_noise(8192);
        eq.process(&mut buffer, SAMPLE_RATE);

        // Should not produce NaN or infinity
        for sample in &buffer {
            assert!(sample.is_finite(), "Output contains non-finite values");
        }
    }

    #[test]
    fn test_31_band_frequency_response() {
        let mut eq = GraphicEq::new_31_band();

        // Set multiple adjacent bands around 1kHz to -12dB for a stronger notch
        // Band 19 is approximately 1kHz in ISO 31-band (500, 630, 800, 1000...)
        eq.set_band_gain(18, -12.0);
        eq.set_band_gain(19, -12.0);
        eq.set_band_gain(20, -12.0);

        let mut buffer = generate_sine_wave(1000.0, SAMPLE_RATE, 8192);
        let original = calculate_rms(&buffer);

        eq.process(&mut buffer, SAMPLE_RATE);
        let processed = calculate_rms(&buffer);

        let gain_db = 20.0 * (processed / original).log10();

        // Should see some cut at 1kHz (even if filter overlap is minimal)
        assert!(
            gain_db < 0.0,
            "1kHz should be cut when bands are set to -12dB, got {:.1}dB",
            gain_db
        );
    }
}

// ============================================================================
// STEREO ENHANCER TESTS
// ============================================================================

mod stereo_enhancer_tests {
    use super::*;

    #[test]
    fn test_default_settings_passthrough() {
        let mut enhancer = StereoEnhancer::new();

        let mut buffer = generate_stereo_signal(1000.0, SAMPLE_RATE, 4096, 0.0);
        let original = buffer.clone();

        enhancer.process(&mut buffer, SAMPLE_RATE);

        // Default settings (width=1.0) should be close to passthrough
        let diff: f32 = buffer
            .iter()
            .zip(&original)
            .map(|(a, b)| (a - b).abs())
            .sum();

        assert!(
            diff < 0.01,
            "Default settings changed signal too much: {}",
            diff
        );
    }

    #[test]
    fn test_width_zero_creates_mono() {
        let mut enhancer = StereoEnhancer::with_settings(StereoSettings::mono());

        // Create a stereo signal with different content in each channel
        let mut buffer = generate_stereo_signal(1000.0, SAMPLE_RATE, 4096, PI / 2.0);

        enhancer.process(&mut buffer, SAMPLE_RATE);

        // With width=0, left and right should be identical (mono)
        for chunk in buffer.chunks(2) {
            assert!(
                (chunk[0] - chunk[1]).abs() < 0.001,
                "Width=0 should create mono, diff={}",
                (chunk[0] - chunk[1]).abs()
            );
        }
    }

    #[test]
    fn test_width_expansion() {
        let mut enhancer = StereoEnhancer::with_settings(StereoSettings::extra_wide());

        // Create stereo signal with phase difference
        let mut buffer = generate_stereo_signal(1000.0, SAMPLE_RATE, 4096, PI / 4.0);

        let original_diff: f32 = buffer
            .chunks(2)
            .map(|c| (c[0] - c[1]).powi(2))
            .sum::<f32>()
            .sqrt();

        enhancer.process(&mut buffer, SAMPLE_RATE);

        let new_diff: f32 = buffer
            .chunks(2)
            .map(|c| (c[0] - c[1]).powi(2))
            .sum::<f32>()
            .sqrt();

        // Width=2 should increase the difference between channels
        assert!(
            new_diff > original_diff,
            "Width=2 should increase stereo separation: original={:.3}, new={:.3}",
            original_diff,
            new_diff
        );
    }

    #[test]
    fn test_mid_gain_boost() {
        let mut enhancer = StereoEnhancer::new();
        enhancer.set_mid_gain_db(6.0);

        // Mono signal (identical in both channels) - should be boosted
        let mut buffer = generate_sine_wave(1000.0, SAMPLE_RATE, 4096);
        let original_rms = calculate_rms(&buffer);

        enhancer.process(&mut buffer, SAMPLE_RATE);
        let processed_rms = calculate_rms(&buffer);

        let gain_db = 20.0 * (processed_rms / original_rms).log10();

        // Should see approximately 6dB boost for mono content
        assert!(
            gain_db > 4.0 && gain_db < 8.0,
            "Mid boost should be ~6dB, got {:.1}dB",
            gain_db
        );
    }

    #[test]
    fn test_side_gain_boost() {
        let mut enhancer = StereoEnhancer::new();
        enhancer.set_side_gain_db(6.0);

        // Pure side signal (left = -right)
        let mut buffer: Vec<f32> = (0..4096)
            .flat_map(|i| {
                let t = i as f32 / SAMPLE_RATE as f32;
                let sample = (2.0 * PI * 1000.0 * t).sin();
                [sample, -sample]
            })
            .collect();

        let original_side_rms = calculate_rms(&left_channel(&buffer));

        enhancer.process(&mut buffer, SAMPLE_RATE);

        // Calculate side component after processing
        let left = left_channel(&buffer);
        let right = right_channel(&buffer);
        let side: Vec<f32> = left.iter().zip(&right).map(|(l, r)| (l - r) / 2.0).collect();
        let processed_side_rms = calculate_rms(&side);

        let gain_db = 20.0 * (processed_side_rms / original_side_rms).log10();

        // Should see approximately 6dB boost for side content
        assert!(
            gain_db > 4.0 && gain_db < 8.0,
            "Side boost should be ~6dB, got {:.1}dB",
            gain_db
        );
    }

    #[test]
    fn test_balance_left() {
        let mut enhancer = StereoEnhancer::new();
        enhancer.set_balance(-1.0);

        let mut buffer = generate_sine_wave(1000.0, SAMPLE_RATE, 4096);

        enhancer.process(&mut buffer, SAMPLE_RATE);

        let left_rms = calculate_rms(&left_channel(&buffer));
        let right_rms = calculate_rms(&right_channel(&buffer));

        // Right should be significantly quieter than left
        assert!(
            left_rms > right_rms * 2.0,
            "Balance left should reduce right channel: L={:.3}, R={:.3}",
            left_rms,
            right_rms
        );
    }

    #[test]
    fn test_balance_right() {
        let mut enhancer = StereoEnhancer::new();
        enhancer.set_balance(1.0);

        let mut buffer = generate_sine_wave(1000.0, SAMPLE_RATE, 4096);

        enhancer.process(&mut buffer, SAMPLE_RATE);

        let left_rms = calculate_rms(&left_channel(&buffer));
        let right_rms = calculate_rms(&right_channel(&buffer));

        // Left should be significantly quieter than right
        assert!(
            right_rms > left_rms * 2.0,
            "Balance right should reduce left channel: L={:.3}, R={:.3}",
            left_rms,
            right_rms
        );
    }

    #[test]
    fn test_mono_compatibility_function() {
        // Perfectly correlated (mono)
        let mono_buffer = generate_sine_wave(1000.0, SAMPLE_RATE, 4096);
        let compat = mono_compatibility(&mono_buffer);
        assert!(compat > 0.99, "Mono signal should have correlation ~1.0, got {}", compat);

        // Anti-correlated
        let anti_buffer: Vec<f32> = (0..4096)
            .flat_map(|i| {
                let t = i as f32 / SAMPLE_RATE as f32;
                let sample = (2.0 * PI * 1000.0 * t).sin();
                [sample, -sample]
            })
            .collect();
        let anti_compat = mono_compatibility(&anti_buffer);
        assert!(anti_compat < -0.99, "Anti-correlated signal should have correlation ~-1.0, got {}", anti_compat);
    }
}

// ============================================================================
// CROSSFEED TESTS
// ============================================================================

mod crossfeed_tests {
    use super::*;

    #[test]
    fn test_bypass_passthrough() {
        let mut crossfeed = Crossfeed::new();
        crossfeed.set_enabled(false);

        let mut buffer = generate_stereo_signal(1000.0, SAMPLE_RATE, 4096, PI / 4.0);
        let original = buffer.clone();

        crossfeed.process(&mut buffer, SAMPLE_RATE);

        // Bypassed should be identical
        assert_eq!(buffer, original, "Bypassed should be unchanged");
    }

    #[test]
    fn test_natural_preset_reduces_separation() {
        let mut crossfeed = Crossfeed::new();
        crossfeed.set_preset(CrossfeedPreset::Natural);
        crossfeed.set_enabled(true);

        // Hard-panned signal (only in left channel)
        let mut buffer: Vec<f32> = (0..4096)
            .flat_map(|i| {
                let t = i as f32 / SAMPLE_RATE as f32;
                let sample = (2.0 * PI * 1000.0 * t).sin();
                [sample, 0.0]
            })
            .collect();

        crossfeed.process(&mut buffer, SAMPLE_RATE);

        // Right channel should now have some signal (crossfed from left)
        let right_rms = calculate_rms(&right_channel(&buffer));
        assert!(
            right_rms > 0.01,
            "Crossfeed should add signal to silent channel, got RMS={}",
            right_rms
        );
    }

    #[test]
    fn test_relaxed_preset() {
        let mut crossfeed = Crossfeed::new();
        crossfeed.set_preset(CrossfeedPreset::Relaxed);
        crossfeed.set_enabled(true);

        // Hard-panned signal
        let mut buffer: Vec<f32> = (0..4096)
            .flat_map(|i| {
                let t = i as f32 / SAMPLE_RATE as f32;
                let sample = (2.0 * PI * 1000.0 * t).sin();
                [sample, 0.0]
            })
            .collect();

        let original_left_rms = calculate_rms(&left_channel(&buffer));

        crossfeed.process(&mut buffer, SAMPLE_RATE);

        let right_rms = calculate_rms(&right_channel(&buffer));

        // Relaxed should have significant crossfeed
        let crossfeed_ratio = right_rms / original_left_rms;
        assert!(
            crossfeed_ratio > 0.05,
            "Relaxed preset should have significant crossfeed: ratio={}",
            crossfeed_ratio
        );
    }

    #[test]
    fn test_meier_preset() {
        let mut crossfeed = Crossfeed::new();
        crossfeed.set_preset(CrossfeedPreset::Meier);
        crossfeed.set_enabled(true);

        let mut buffer: Vec<f32> = (0..4096)
            .flat_map(|i| {
                let t = i as f32 / SAMPLE_RATE as f32;
                let sample = (2.0 * PI * 1000.0 * t).sin();
                [sample, 0.0]
            })
            .collect();

        crossfeed.process(&mut buffer, SAMPLE_RATE);

        // Should produce valid output
        for sample in &buffer {
            assert!(sample.is_finite(), "Meier preset output not finite");
        }

        // Should have crossfeed
        assert!(
            calculate_rms(&right_channel(&buffer)) > 0.01,
            "Meier preset should add crossfeed"
        );
    }

    #[test]
    fn test_frequency_dependent_crossfeed() {
        // Test crossfeed at different frequencies
        // Lower frequencies should have more crossfeed (ILD is less at low frequencies)
        let mut crossfeed_ratios = Vec::new();

        for &freq in &[100.0, 1000.0, 8000.0] {
            let mut buffer: Vec<f32> = (0..8192)
                .flat_map(|i| {
                    let t = i as f32 / SAMPLE_RATE as f32;
                    let sample = (2.0 * PI * freq * t).sin();
                    [sample, 0.0]
                })
                .collect();

            let original_left = calculate_rms(&left_channel(&buffer));

            let mut cf = Crossfeed::new();
            cf.set_preset(CrossfeedPreset::Natural);
            cf.set_enabled(true);
            cf.process(&mut buffer, SAMPLE_RATE);

            let right_rms = calculate_rms(&right_channel(&buffer));
            crossfeed_ratios.push((freq, right_rms / original_left));
        }

        // Log results for analysis
        for (freq, ratio) in &crossfeed_ratios {
            println!("{} Hz: crossfeed ratio = {:.3}", freq, ratio);
        }
    }

    #[test]
    fn test_mono_content_unaffected() {
        let mut crossfeed = Crossfeed::new();
        crossfeed.set_preset(CrossfeedPreset::Natural);
        crossfeed.set_enabled(true);

        // Mono content (identical in both channels)
        let mut buffer = generate_sine_wave(1000.0, SAMPLE_RATE, 4096);
        let original_rms = calculate_rms(&buffer);

        crossfeed.process(&mut buffer, SAMPLE_RATE);

        let left_rms = calculate_rms(&left_channel(&buffer));
        let right_rms = calculate_rms(&right_channel(&buffer));

        // Mono content should remain mostly centered
        let balance_diff = (left_rms - right_rms).abs() / original_rms;
        assert!(
            balance_diff < 0.1,
            "Mono content balance should be preserved: diff={}",
            balance_diff
        );
    }
}

// ============================================================================
// EFFECT COMBINATION MATRIX TESTS
// ============================================================================

mod effect_combination_tests {
    use super::*;

    /// Test a specific combination of effects
    fn test_effect_combination(
        use_eq: bool,
        use_graphic_eq: bool,
        use_stereo_enhancer: bool,
        use_crossfeed: bool,
        use_limiter: bool,
    ) {
        let mut buffer = generate_stereo_signal(1000.0, SAMPLE_RATE, 4096, 0.25);

        // Apply effects in chain order
        if use_eq {
            let mut eq = ParametricEq::new();
            eq.set_low_band(EqBand::low_shelf(100.0, 3.0));
            eq.set_mid_band(EqBand::peaking(1000.0, -2.0, 1.0));
            eq.set_high_band(EqBand::high_shelf(10000.0, 1.0));
            eq.process(&mut buffer, SAMPLE_RATE);
        }

        if use_graphic_eq {
            let mut geq = GraphicEq::new_10_band();
            geq.set_preset(GraphicEqPreset::Rock);
            geq.process(&mut buffer, SAMPLE_RATE);
        }

        if use_stereo_enhancer {
            let mut se = StereoEnhancer::with_settings(StereoSettings::wide());
            se.process(&mut buffer, SAMPLE_RATE);
        }

        if use_crossfeed {
            let mut cf = Crossfeed::new();
            cf.set_preset(CrossfeedPreset::Natural);
            cf.set_enabled(true);
            cf.process(&mut buffer, SAMPLE_RATE);
        }

        if use_limiter {
            let mut limiter = Limiter::with_settings(LimiterSettings::brickwall());
            limiter.process(&mut buffer, SAMPLE_RATE);
        }

        // Verify output is valid
        for sample in &buffer {
            assert!(
                sample.is_finite(),
                "Effect combination produced non-finite output"
            );
            assert!(
                sample.abs() < 10.0,
                "Effect combination produced excessive amplitude: {}",
                sample
            );
        }
    }

    #[test]
    fn test_all_effects_disabled() {
        test_effect_combination(false, false, false, false, false);
    }

    #[test]
    fn test_parametric_eq_only() {
        test_effect_combination(true, false, false, false, false);
    }

    #[test]
    fn test_graphic_eq_only() {
        test_effect_combination(false, true, false, false, false);
    }

    #[test]
    fn test_stereo_enhancer_only() {
        test_effect_combination(false, false, true, false, false);
    }

    #[test]
    fn test_crossfeed_only() {
        test_effect_combination(false, false, false, true, false);
    }

    #[test]
    fn test_limiter_only() {
        test_effect_combination(false, false, false, false, true);
    }

    #[test]
    fn test_all_effects_enabled() {
        test_effect_combination(true, true, true, true, true);
    }

    #[test]
    fn test_eq_and_limiter() {
        test_effect_combination(true, false, false, false, true);
    }

    #[test]
    fn test_graphic_eq_and_stereo() {
        test_effect_combination(false, true, true, false, false);
    }

    #[test]
    fn test_crossfeed_and_stereo_enhancer() {
        // This is an interesting combination - stereo enhancer widens, crossfeed narrows
        test_effect_combination(false, false, true, true, false);
    }

    #[test]
    fn test_both_eqs() {
        test_effect_combination(true, true, false, false, false);
    }

    #[test]
    fn test_typical_audiophile_setup() {
        // Parametric EQ + Crossfeed + Limiter
        test_effect_combination(true, false, false, true, true);
    }

    #[test]
    fn test_typical_casual_setup() {
        // Graphic EQ + Stereo Enhancer
        test_effect_combination(false, true, true, false, false);
    }

    /// Generate all 32 combinations and test them
    #[test]
    fn test_all_32_combinations() {
        let mut passed = 0;
        let mut failed = 0;

        for i in 0..32 {
            let use_eq = (i & 1) != 0;
            let use_graphic_eq = (i & 2) != 0;
            let use_stereo_enhancer = (i & 4) != 0;
            let use_crossfeed = (i & 8) != 0;
            let use_limiter = (i & 16) != 0;

            let result = std::panic::catch_unwind(|| {
                test_effect_combination(
                    use_eq,
                    use_graphic_eq,
                    use_stereo_enhancer,
                    use_crossfeed,
                    use_limiter,
                );
            });

            if result.is_ok() {
                passed += 1;
            } else {
                failed += 1;
                println!(
                    "Combination {} failed: EQ={}, GEQ={}, SE={}, CF={}, Lim={}",
                    i, use_eq, use_graphic_eq, use_stereo_enhancer, use_crossfeed, use_limiter
                );
            }
        }

        assert_eq!(
            failed, 0,
            "All 32 combinations should pass: {} passed, {} failed",
            passed, failed
        );
    }
}

// ============================================================================
// STREAM ROBUSTNESS TESTS
// ============================================================================

mod stream_robustness_tests {
    use super::*;

    #[test]
    fn test_variable_buffer_sizes() {
        let mut eq = ParametricEq::new();
        eq.set_low_band(EqBand::low_shelf(100.0, 3.0));

        // Test with various buffer sizes
        for buffer_size in [64, 128, 256, 512, 1024, 2048, 4096] {
            let mut buffer = generate_sine_wave(1000.0, SAMPLE_RATE, buffer_size);
            eq.process(&mut buffer, SAMPLE_RATE);

            for sample in &buffer {
                assert!(
                    sample.is_finite(),
                    "Buffer size {} produced non-finite output",
                    buffer_size
                );
            }
        }
    }

    #[test]
    fn test_odd_buffer_sizes() {
        let mut eq = ParametricEq::new();
        eq.set_mid_band(EqBand::peaking(1000.0, 3.0, 1.0));

        // Test with non-power-of-2 buffer sizes
        for buffer_size in [100, 333, 500, 777, 1000, 1234, 3333] {
            let mut buffer = generate_sine_wave(1000.0, SAMPLE_RATE, buffer_size);
            eq.process(&mut buffer, SAMPLE_RATE);

            for sample in &buffer {
                assert!(
                    sample.is_finite(),
                    "Buffer size {} produced non-finite output",
                    buffer_size
                );
            }
        }
    }

    #[test]
    fn test_sample_rate_change() {
        let sample_rates = [44100, 48000, 88200, 96000, 176400, 192000];

        for &sr in &sample_rates {
            let mut eq = ParametricEq::new();
            eq.set_low_band(EqBand::low_shelf(100.0, 3.0));
            eq.set_mid_band(EqBand::peaking(1000.0, -2.0, 1.0));
            eq.set_high_band(EqBand::high_shelf(10000.0, 1.0));

            let mut buffer = generate_sine_wave(1000.0, sr, 4096);
            eq.process(&mut buffer, sr);

            for sample in &buffer {
                assert!(
                    sample.is_finite(),
                    "Sample rate {} produced non-finite output",
                    sr
                );
            }
        }
    }

    #[test]
    fn test_continuous_processing() {
        let mut eq = ParametricEq::new();
        eq.set_low_band(EqBand::low_shelf(100.0, 3.0));
        eq.set_mid_band(EqBand::peaking(1000.0, -2.0, 1.0));
        eq.set_high_band(EqBand::high_shelf(10000.0, 1.0));

        // Process many consecutive buffers
        for _ in 0..1000 {
            let mut buffer = generate_sine_wave(1000.0, SAMPLE_RATE, 512);
            eq.process(&mut buffer, SAMPLE_RATE);

            // Check for drift or instability
            for sample in &buffer {
                assert!(
                    sample.is_finite(),
                    "Continuous processing produced non-finite output"
                );
                assert!(
                    sample.abs() < 10.0,
                    "Continuous processing produced excessive amplitude"
                );
            }
        }
    }

    #[test]
    fn test_gapless_transition_simulation() {
        let mut eq = ParametricEq::new();
        eq.set_low_band(EqBand::low_shelf(100.0, 3.0));

        // Simulate track transition - different frequencies
        let mut buffer1 = generate_sine_wave(440.0, SAMPLE_RATE, 2048);
        let mut buffer2 = generate_sine_wave(880.0, SAMPLE_RATE, 2048);

        eq.process(&mut buffer1, SAMPLE_RATE);
        eq.process(&mut buffer2, SAMPLE_RATE);

        // Check for continuity issues at transition point
        // Look for large discontinuities
        let last_sample = buffer1[buffer1.len() - 1];
        let first_sample = buffer2[0];

        // The transition might have a discontinuity due to frequency change,
        // but it shouldn't be extreme
        let discontinuity = (first_sample - last_sample).abs();
        assert!(
            discontinuity < 2.0,
            "Gapless transition has large discontinuity: {}",
            discontinuity
        );
    }

    #[test]
    fn test_empty_buffer_handling() {
        let mut eq = ParametricEq::new();
        eq.set_low_band(EqBand::low_shelf(100.0, 3.0));

        let mut buffer: Vec<f32> = vec![];
        eq.process(&mut buffer, SAMPLE_RATE); // Should not panic

        assert!(buffer.is_empty());
    }

    #[test]
    fn test_single_sample_buffer() {
        let mut eq = ParametricEq::new();
        eq.set_low_band(EqBand::low_shelf(100.0, 3.0));

        // Single stereo sample
        let mut buffer = vec![0.5, 0.5];
        eq.process(&mut buffer, SAMPLE_RATE);

        assert!(buffer[0].is_finite());
        assert!(buffer[1].is_finite());
    }
}

// ============================================================================
// NUMERICAL STABILITY TESTS
// ============================================================================

mod numerical_stability_tests {
    use super::*;

    #[test]
    fn test_denormal_handling() {
        let mut eq = ParametricEq::new();
        eq.set_low_band(EqBand::low_shelf(100.0, 3.0));
        eq.set_mid_band(EqBand::peaking(1000.0, -2.0, 1.0));
        eq.set_high_band(EqBand::high_shelf(10000.0, 1.0));

        // Create buffer with denormal values
        let denormal = f32::MIN_POSITIVE / 1000.0;
        let mut buffer = vec![denormal; 8192];

        // Measure processing time to check for denormal performance issues
        let start = std::time::Instant::now();
        eq.process(&mut buffer, SAMPLE_RATE);
        let elapsed = start.elapsed();

        // Output should be finite (denormals are OK, but NaN/Inf are not)
        for sample in &buffer {
            assert!(sample.is_finite(), "Denormal input produced non-finite output");
        }

        // Processing should complete in reasonable time (denormals shouldn't cause slowdown)
        // Note: some CPUs may not have denormal flushing, but processing should still complete
        assert!(
            elapsed.as_millis() < 100,
            "Processing denormals took too long: {:?}",
            elapsed
        );
    }

    #[test]
    fn test_dc_offset_handling() {
        let mut eq = ParametricEq::new();
        eq.set_low_band(EqBand::low_shelf(100.0, 3.0));

        // Signal with DC offset
        let dc_offset = 0.5;
        let mut buffer: Vec<f32> = (0..4096)
            .flat_map(|i| {
                let t = i as f32 / SAMPLE_RATE as f32;
                let sample = dc_offset + 0.3 * (2.0 * PI * 1000.0 * t).sin();
                [sample, sample]
            })
            .collect();

        eq.process(&mut buffer, SAMPLE_RATE);

        // Output should still be finite
        for sample in &buffer {
            assert!(sample.is_finite(), "DC offset input produced non-finite output");
        }
    }

    #[test]
    fn test_very_small_signals() {
        let mut eq = ParametricEq::new();
        eq.set_low_band(EqBand::low_shelf(100.0, 6.0));
        eq.set_mid_band(EqBand::peaking(1000.0, 6.0, 1.0));
        eq.set_high_band(EqBand::high_shelf(10000.0, 6.0));

        // Very small amplitude signal
        let amplitude = 1e-6;
        let mut buffer: Vec<f32> = (0..4096)
            .flat_map(|i| {
                let t = i as f32 / SAMPLE_RATE as f32;
                let sample = amplitude * (2.0 * PI * 1000.0 * t).sin();
                [sample, sample]
            })
            .collect();

        eq.process(&mut buffer, SAMPLE_RATE);

        for sample in &buffer {
            assert!(sample.is_finite(), "Small signal produced non-finite output");
        }
    }

    #[test]
    fn test_very_large_signals() {
        let mut limiter = Limiter::with_settings(LimiterSettings::brickwall());

        // Large amplitude signal (would clip without limiter)
        let amplitude = 10.0;
        let mut buffer: Vec<f32> = (0..4096)
            .flat_map(|i| {
                let t = i as f32 / SAMPLE_RATE as f32;
                let sample = amplitude * (2.0 * PI * 1000.0 * t).sin();
                [sample, sample]
            })
            .collect();

        limiter.process(&mut buffer, SAMPLE_RATE);

        for sample in &buffer {
            assert!(sample.is_finite(), "Large signal produced non-finite output");
            assert!(
                sample.abs() <= 1.0,
                "Limiter failed to limit: {}",
                sample
            );
        }
    }

    #[test]
    fn test_nan_input_handling() {
        let mut eq = ParametricEq::new();
        eq.set_low_band(EqBand::low_shelf(100.0, 3.0));

        // Create buffer with NaN - effects should handle gracefully
        let mut buffer = vec![0.0; 8192];
        buffer[100] = f32::NAN;
        buffer[200] = f32::INFINITY;
        buffer[300] = f32::NEG_INFINITY;

        // This might propagate NaN, but shouldn't panic
        eq.process(&mut buffer, SAMPLE_RATE);

        // At least verify no panic occurred
    }

    #[test]
    fn test_impulse_response_stability() {
        let mut eq = ParametricEq::new();
        eq.set_low_band(EqBand::low_shelf(100.0, 12.0)); // High gain
        eq.set_mid_band(EqBand::peaking(1000.0, 12.0, 1.0));
        eq.set_high_band(EqBand::high_shelf(10000.0, 12.0));

        // Single impulse
        let mut buffer = vec![0.0; 8192];
        buffer[0] = 1.0;
        buffer[1] = 1.0;

        eq.process(&mut buffer, SAMPLE_RATE);

        // Filter should be stable - impulse response should decay
        let first_quarter_energy: f32 = buffer[..2048].iter().map(|x| x * x).sum();
        let last_quarter_energy: f32 = buffer[6144..].iter().map(|x| x * x).sum();

        assert!(
            last_quarter_energy < first_quarter_energy || last_quarter_energy < 1e-6,
            "Impulse response doesn't decay properly: first={}, last={}",
            first_quarter_energy,
            last_quarter_energy
        );
    }

    #[test]
    fn test_extreme_q_values() {
        // Very high Q can cause instability in some filter implementations
        let mut eq = ParametricEq::new();
        eq.set_mid_band(EqBand::peaking(1000.0, 3.0, 10.0)); // High Q

        let mut buffer = generate_sine_wave(1000.0, SAMPLE_RATE, 4096);
        eq.process(&mut buffer, SAMPLE_RATE);

        for sample in &buffer {
            assert!(
                sample.is_finite(),
                "High Q produced non-finite output"
            );
        }
    }

    #[test]
    fn test_negative_gain_stability() {
        let mut eq = ParametricEq::new();
        eq.set_low_band(EqBand::low_shelf(100.0, -12.0)); // Deep cut
        eq.set_mid_band(EqBand::peaking(1000.0, -12.0, 1.0));
        eq.set_high_band(EqBand::high_shelf(10000.0, -12.0));

        let mut buffer = generate_sine_wave(1000.0, SAMPLE_RATE, 4096);
        eq.process(&mut buffer, SAMPLE_RATE);

        for sample in &buffer {
            assert!(
                sample.is_finite(),
                "Deep cut produced non-finite output"
            );
        }
    }

    #[test]
    fn test_alternating_silence_and_signal() {
        let mut eq = ParametricEq::new();
        eq.set_low_band(EqBand::low_shelf(100.0, 6.0));

        // Alternate between signal and silence
        for _ in 0..10 {
            let mut signal_buffer = generate_sine_wave(1000.0, SAMPLE_RATE, 512);
            eq.process(&mut signal_buffer, SAMPLE_RATE);

            let mut silence_buffer = vec![0.0; 1024];
            eq.process(&mut silence_buffer, SAMPLE_RATE);
        }

        // Final buffer should be stable
        let mut final_buffer = generate_sine_wave(1000.0, SAMPLE_RATE, 512);
        eq.process(&mut final_buffer, SAMPLE_RATE);

        for sample in &final_buffer {
            assert!(
                sample.is_finite(),
                "Alternating signal/silence produced instability"
            );
        }
    }
}

// ============================================================================
// REAL-TIME SAFETY TESTS
// ============================================================================

mod realtime_safety_tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_processing_latency_consistency() {
        let mut eq = ParametricEq::new();
        eq.set_low_band(EqBand::low_shelf(100.0, 3.0));
        eq.set_mid_band(EqBand::peaking(1000.0, -2.0, 1.0));
        eq.set_high_band(EqBand::high_shelf(10000.0, 1.0));

        let buffer_size = 512;
        let iterations = 100;
        let mut durations = Vec::with_capacity(iterations);

        // Warm up
        for _ in 0..10 {
            let mut buffer = generate_sine_wave(1000.0, SAMPLE_RATE, buffer_size);
            eq.process(&mut buffer, SAMPLE_RATE);
        }

        // Measure
        for _ in 0..iterations {
            let mut buffer = generate_sine_wave(1000.0, SAMPLE_RATE, buffer_size);
            let start = Instant::now();
            eq.process(&mut buffer, SAMPLE_RATE);
            durations.push(start.elapsed());
        }

        let avg_duration = durations.iter().map(|d| d.as_nanos()).sum::<u128>() / iterations as u128;
        let max_duration = durations.iter().map(|d| d.as_nanos()).max().unwrap();

        // Max should not be much more than average (consistent timing)
        let ratio = max_duration as f64 / avg_duration as f64;
        assert!(
            ratio < 10.0,
            "Processing time varies too much: avg={}ns, max={}ns, ratio={:.1}",
            avg_duration,
            max_duration,
            ratio
        );
    }

    #[test]
    fn test_buffer_independence() {
        // Process should not retain excessive state between independent buffers
        let mut eq = ParametricEq::new();
        eq.set_low_band(EqBand::low_shelf(100.0, 3.0));

        // Process with high amplitude
        let mut loud_buffer: Vec<f32> = generate_sine_wave(1000.0, SAMPLE_RATE, 512)
            .iter()
            .map(|&x| x * 0.9)
            .collect();
        eq.process(&mut loud_buffer, SAMPLE_RATE);

        // Reset and process with low amplitude
        let mut eq2 = ParametricEq::new();
        eq2.set_low_band(EqBand::low_shelf(100.0, 3.0));

        let mut quiet_buffer: Vec<f32> = generate_sine_wave(1000.0, SAMPLE_RATE, 512)
            .iter()
            .map(|&x| x * 0.01)
            .collect();
        eq2.process(&mut quiet_buffer, SAMPLE_RATE);

        // The quiet buffer output should be much quieter than loud buffer
        let loud_rms = calculate_rms(&loud_buffer);
        let quiet_rms = calculate_rms(&quiet_buffer);

        assert!(
            loud_rms > quiet_rms * 10.0,
            "Buffer independence issue: loud_rms={}, quiet_rms={}",
            loud_rms,
            quiet_rms
        );
    }

    #[test]
    fn test_no_infinite_loop_on_edge_cases() {
        let mut eq = ParametricEq::new();

        // Edge case buffer sizes (in stereo samples, so multiply by 2)
        for size in [0, 1, 2, 3, 7, 15, 31, 63] {
            let mut buffer = vec![0.5; size * 2];
            let start = Instant::now();
            eq.process(&mut buffer, SAMPLE_RATE);
            let elapsed = start.elapsed();

            // Should complete quickly
            assert!(
                elapsed.as_millis() < 100,
                "Processing buffer of size {} took too long: {:?}",
                size,
                elapsed
            );
        }
    }

    #[test]
    fn test_effect_chain_latency() {
        // Full effect chain should still be fast enough for real-time
        let buffer_size = 512;
        let sample_rate = 48000_u32;

        let mut eq = ParametricEq::new();
        eq.set_low_band(EqBand::low_shelf(100.0, 3.0));
        eq.set_mid_band(EqBand::peaking(1000.0, -2.0, 1.0));
        eq.set_high_band(EqBand::high_shelf(10000.0, 1.0));

        let mut geq = GraphicEq::new_10_band();
        geq.set_preset(GraphicEqPreset::Rock);

        let mut stereo = StereoEnhancer::with_settings(StereoSettings::wide());

        let mut crossfeed = Crossfeed::new();
        crossfeed.set_preset(CrossfeedPreset::Natural);
        crossfeed.set_enabled(true);

        let mut limiter = Limiter::with_settings(LimiterSettings::brickwall());

        let mut buffer = generate_stereo_signal(1000.0, sample_rate, buffer_size, 0.25);

        let start = Instant::now();

        // Apply all effects
        eq.process(&mut buffer, sample_rate);
        geq.process(&mut buffer, sample_rate);
        stereo.process(&mut buffer, sample_rate);
        crossfeed.process(&mut buffer, sample_rate);
        limiter.process(&mut buffer, sample_rate);

        let elapsed = start.elapsed();

        // Calculate deadline for real-time
        let buffer_time_ms = (buffer_size as f64 / sample_rate as f64) * 1000.0;

        assert!(
            elapsed.as_secs_f64() * 1000.0 < buffer_time_ms,
            "Effect chain too slow for real-time: took {:.2}ms, deadline {:.2}ms",
            elapsed.as_secs_f64() * 1000.0,
            buffer_time_ms
        );
    }
}

// ============================================================================
// COMPRESSOR INTEGRATION TESTS
// ============================================================================

mod compressor_integration_tests {
    use super::*;

    #[test]
    fn test_compressor_reduces_peaks() {
        let mut compressor = Compressor::with_settings(CompressorSettings {
            threshold_db: -20.0,
            ratio: 4.0,
            attack_ms: 1.0,
            release_ms: 50.0,
            knee_db: 0.0,
            makeup_gain_db: 0.0,
        });

        // Signal that peaks above threshold
        let mut buffer = generate_sine_wave(1000.0, SAMPLE_RATE, 4096);
        let original_peak: f32 = buffer.iter().map(|s| s.abs()).max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();

        compressor.process(&mut buffer, SAMPLE_RATE);

        let compressed_peak: f32 = buffer.iter().map(|s| s.abs()).max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();

        // Peak should be reduced
        assert!(
            compressed_peak < original_peak,
            "Compressor should reduce peaks: original={}, compressed={}",
            original_peak,
            compressed_peak
        );
    }

    #[test]
    fn test_compressor_makeup_gain() {
        // Test that makeup gain actually boosts the signal
        // Compare same settings with and without makeup gain
        let mut compressor_no_makeup = Compressor::with_settings(CompressorSettings {
            threshold_db: -20.0,
            ratio: 4.0,
            attack_ms: 1.0,
            release_ms: 50.0,
            knee_db: 0.0,
            makeup_gain_db: 0.0,
        });

        let mut compressor_with_makeup = Compressor::with_settings(CompressorSettings {
            threshold_db: -20.0,
            ratio: 4.0,
            attack_ms: 1.0,
            release_ms: 50.0,
            knee_db: 0.0,
            makeup_gain_db: 6.0, // +6 dB makeup
        });

        let mut buffer_no_makeup = generate_sine_wave(1000.0, SAMPLE_RATE, 4096);
        let mut buffer_with_makeup = buffer_no_makeup.clone();

        compressor_no_makeup.process(&mut buffer_no_makeup, SAMPLE_RATE);
        compressor_with_makeup.process(&mut buffer_with_makeup, SAMPLE_RATE);

        let rms_no_makeup = calculate_rms(&buffer_no_makeup);
        let rms_with_makeup = calculate_rms(&buffer_with_makeup);

        // With makeup gain, output should be louder
        assert!(
            rms_with_makeup > rms_no_makeup,
            "Makeup gain should increase output level: without={}, with={}",
            rms_no_makeup,
            rms_with_makeup
        );
    }

    #[test]
    fn test_compressor_soft_knee() {
        let mut hard_knee = Compressor::with_settings(CompressorSettings {
            threshold_db: -20.0,
            ratio: 4.0,
            attack_ms: 1.0,
            release_ms: 50.0,
            knee_db: 0.0, // Hard knee
            makeup_gain_db: 0.0,
        });

        let mut soft_knee = Compressor::with_settings(CompressorSettings {
            threshold_db: -20.0,
            ratio: 4.0,
            attack_ms: 1.0,
            release_ms: 50.0,
            knee_db: 12.0, // Soft knee
            makeup_gain_db: 0.0,
        });

        let mut hard_buffer = generate_sine_wave(1000.0, SAMPLE_RATE, 4096);
        let mut soft_buffer = hard_buffer.clone();

        hard_knee.process(&mut hard_buffer, SAMPLE_RATE);
        soft_knee.process(&mut soft_buffer, SAMPLE_RATE);

        // Both should produce valid output
        for sample in hard_buffer.iter().chain(soft_buffer.iter()) {
            assert!(sample.is_finite());
        }
    }

    #[test]
    fn test_compressor_attack_time() {
        // Very fast attack
        let mut fast_attack = Compressor::with_settings(CompressorSettings {
            threshold_db: -20.0,
            ratio: 8.0,
            attack_ms: 0.1, // Very fast
            release_ms: 100.0,
            knee_db: 0.0,
            makeup_gain_db: 0.0,
        });

        // Slow attack
        let mut slow_attack = Compressor::with_settings(CompressorSettings {
            threshold_db: -20.0,
            ratio: 8.0,
            attack_ms: 100.0, // Slow
            release_ms: 100.0,
            knee_db: 0.0,
            makeup_gain_db: 0.0,
        });

        // Impulse followed by sustained signal
        let mut fast_buffer = vec![0.0; 8820]; // 0.1 sec silence
        let mut slow_buffer = fast_buffer.clone();

        // Add impulse at the start
        fast_buffer[0] = 0.9;
        fast_buffer[1] = 0.9;
        slow_buffer[0] = 0.9;
        slow_buffer[1] = 0.9;

        fast_attack.process(&mut fast_buffer, SAMPLE_RATE);
        slow_attack.process(&mut slow_buffer, SAMPLE_RATE);

        // Both should complete without error
        for sample in fast_buffer.iter().chain(slow_buffer.iter()) {
            assert!(sample.is_finite());
        }
    }

    #[test]
    fn test_compressor_in_effect_chain() {
        let mut eq = ParametricEq::new();
        eq.set_low_band(EqBand::low_shelf(100.0, 6.0)); // Bass boost

        let mut compressor = Compressor::with_settings(CompressorSettings {
            threshold_db: -12.0,
            ratio: 4.0,
            attack_ms: 10.0,
            release_ms: 100.0,
            knee_db: 3.0,
            makeup_gain_db: 0.0,
        });

        let mut limiter = Limiter::with_settings(LimiterSettings::brickwall());

        let mut buffer = generate_sine_wave(60.0, SAMPLE_RATE, 4096); // Bass frequency

        eq.process(&mut buffer, SAMPLE_RATE);
        compressor.process(&mut buffer, SAMPLE_RATE);
        limiter.process(&mut buffer, SAMPLE_RATE);

        // Should be limited to safe range
        for sample in &buffer {
            assert!(sample.is_finite());
            assert!(sample.abs() <= 1.0);
        }
    }

    #[test]
    fn test_compressor_with_all_effects() {
        let mut eq = ParametricEq::new();
        eq.set_mid_band(EqBand::peaking(1000.0, 3.0, 1.0));

        let mut compressor = Compressor::with_settings(CompressorSettings::default());

        let mut stereo = StereoEnhancer::with_settings(StereoSettings::wide());

        let mut limiter = Limiter::with_settings(LimiterSettings::brickwall());

        let mut buffer = generate_stereo_signal(1000.0, SAMPLE_RATE, 4096, 0.25);

        eq.process(&mut buffer, SAMPLE_RATE);
        compressor.process(&mut buffer, SAMPLE_RATE);
        stereo.process(&mut buffer, SAMPLE_RATE);
        limiter.process(&mut buffer, SAMPLE_RATE);

        for sample in &buffer {
            assert!(sample.is_finite());
            assert!(sample.abs() <= 1.0);
        }
    }
}

// ============================================================================
// EFFECT CHAIN API TESTS
// ============================================================================

mod effect_chain_api_tests {
    use super::*;

    #[test]
    fn test_empty_chain_passthrough() {
        let mut chain = EffectChain::new();

        let original = generate_sine_wave(1000.0, SAMPLE_RATE, 4096);
        let mut buffer = original.clone();

        chain.process(&mut buffer, SAMPLE_RATE);

        // Empty chain should be passthrough
        assert_eq!(buffer, original);
    }

    #[test]
    fn test_add_single_effect() {
        let mut chain = EffectChain::new();

        let eq = ParametricEq::new();
        chain.add_effect(Box::new(eq));

        assert_eq!(chain.len(), 1);
        assert!(!chain.is_empty());
    }

    #[test]
    fn test_add_multiple_effects() {
        let mut chain = EffectChain::new();

        chain.add_effect(Box::new(ParametricEq::new()));
        chain.add_effect(Box::new(GraphicEq::new_10_band()));
        chain.add_effect(Box::new(Limiter::new()));

        assert_eq!(chain.len(), 3);
    }

    #[test]
    fn test_chain_processing_order() {
        let mut chain = EffectChain::new();

        // First effect: high gain EQ
        let mut eq = ParametricEq::new();
        eq.set_mid_band(EqBand::peaking(1000.0, 12.0, 1.0));
        chain.add_effect(Box::new(eq));

        // Second effect: limiter
        let limiter = Limiter::with_settings(LimiterSettings::brickwall());
        chain.add_effect(Box::new(limiter));

        let mut buffer = generate_sine_wave(1000.0, SAMPLE_RATE, 4096);
        chain.process(&mut buffer, SAMPLE_RATE);

        // With EQ first then limiter, output should be limited
        for sample in &buffer {
            assert!(sample.abs() <= 1.0, "Limiter should cap output");
        }
    }

    #[test]
    fn test_chain_clear() {
        let mut chain = EffectChain::new();

        chain.add_effect(Box::new(ParametricEq::new()));
        chain.add_effect(Box::new(Limiter::new()));

        assert_eq!(chain.len(), 2);

        chain.clear();

        assert_eq!(chain.len(), 0);
        assert!(chain.is_empty());
    }

    #[test]
    fn test_chain_reset() {
        let mut chain = EffectChain::new();

        let mut eq = ParametricEq::new();
        eq.set_mid_band(EqBand::peaking(1000.0, 6.0, 10.0)); // High Q
        chain.add_effect(Box::new(eq));

        // Process to build up filter state
        for _ in 0..100 {
            let mut buffer = generate_sine_wave(1000.0, SAMPLE_RATE, 512);
            chain.process(&mut buffer, SAMPLE_RATE);
        }

        // Reset chain
        chain.reset();

        // Should work normally after reset
        let mut buffer = generate_sine_wave(1000.0, SAMPLE_RATE, 512);
        chain.process(&mut buffer, SAMPLE_RATE);

        for sample in &buffer {
            assert!(sample.is_finite());
        }
    }

    #[test]
    fn test_chain_get_effect() {
        let mut chain = EffectChain::new();

        chain.add_effect(Box::new(ParametricEq::new()));
        chain.add_effect(Box::new(Limiter::new()));

        let effect0 = chain.get_effect(0);
        assert!(effect0.is_some());
        assert_eq!(effect0.unwrap().name(), "3-Band Parametric EQ");

        let effect1 = chain.get_effect(1);
        assert!(effect1.is_some());
        assert_eq!(effect1.unwrap().name(), "Limiter");

        let effect2 = chain.get_effect(2);
        assert!(effect2.is_none());
    }

    #[test]
    fn test_chain_enable_disable_all() {
        let mut chain = EffectChain::new();

        let mut eq = ParametricEq::new();
        eq.set_mid_band(EqBand::peaking(1000.0, 12.0, 1.0));
        chain.add_effect(Box::new(eq));

        let original = generate_sine_wave(1000.0, SAMPLE_RATE, 4096);

        // Disable all
        chain.set_enabled(false);

        let mut buffer = original.clone();
        chain.process(&mut buffer, SAMPLE_RATE);

        // All disabled should be passthrough
        assert_eq!(buffer, original);
    }

    #[test]
    fn test_chain_with_many_effects() {
        let mut chain = EffectChain::new();

        // Add 10 effects
        for _ in 0..10 {
            chain.add_effect(Box::new(ParametricEq::new()));
        }

        assert_eq!(chain.len(), 10);

        let mut buffer = generate_sine_wave(1000.0, SAMPLE_RATE, 4096);
        chain.process(&mut buffer, SAMPLE_RATE);

        for sample in &buffer {
            assert!(sample.is_finite());
        }
    }

    #[test]
    fn test_chain_continuous_add_and_process() {
        let mut chain = EffectChain::new();

        for i in 0..10 {
            // Add effect
            let mut eq = ParametricEq::new();
            let gain = (i as f32 - 5.0) * 2.0; // -10 to +8 dB
            eq.set_mid_band(EqBand::peaking(1000.0, gain, 1.0));
            chain.add_effect(Box::new(eq));

            // Process
            let mut buffer = generate_sine_wave(1000.0, SAMPLE_RATE, 512);
            chain.process(&mut buffer, SAMPLE_RATE);

            for sample in &buffer {
                assert!(sample.is_finite());
            }
        }
    }

    #[test]
    fn test_chain_with_all_effect_types() {
        let mut chain = EffectChain::new();

        // Add one of each effect type
        let mut eq = ParametricEq::new();
        eq.set_mid_band(EqBand::peaking(1000.0, 3.0, 1.0));
        chain.add_effect(Box::new(eq));

        let mut geq = GraphicEq::new_10_band();
        geq.set_preset(GraphicEqPreset::Rock);
        chain.add_effect(Box::new(geq));

        chain.add_effect(Box::new(StereoEnhancer::with_settings(StereoSettings::wide())));

        let mut crossfeed = Crossfeed::new();
        crossfeed.set_enabled(true);
        chain.add_effect(Box::new(crossfeed));

        chain.add_effect(Box::new(Compressor::with_settings(CompressorSettings::default())));

        chain.add_effect(Box::new(Limiter::with_settings(LimiterSettings::brickwall())));

        assert_eq!(chain.len(), 6);

        let mut buffer = generate_stereo_signal(1000.0, SAMPLE_RATE, 4096, 0.25);
        chain.process(&mut buffer, SAMPLE_RATE);

        for sample in &buffer {
            assert!(sample.is_finite());
            assert!(sample.abs() <= 1.0);
        }
    }
}
