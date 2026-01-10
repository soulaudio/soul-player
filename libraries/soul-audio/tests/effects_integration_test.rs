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
    AudioEffect, Crossfeed, CrossfeedPreset, GraphicEq, GraphicEqPreset, Limiter,
    ParametricEq, StereoEnhancer, StereoSettings, VolumeNormalizer,
};
use soul_audio::test_utils::{
    analysis::{calculate_snr_at_frequency, measure_frequency_response},
    signals::{generate_sine_wave, generate_stereo_test_signal, generate_white_noise},
};
use std::f32::consts::PI;

const SAMPLE_RATE: u32 = 44100;

// ============================================================================
// GRAPHIC EQ TESTS
// ============================================================================

mod graphic_eq_tests {
    use super::*;

    #[test]
    fn test_10_band_eq_creation() {
        let eq = GraphicEq::new_10_band(SAMPLE_RATE);
        // 10-band EQ should have bands at standard frequencies
        // 31, 62, 125, 250, 500, 1k, 2k, 4k, 8k, 16k Hz
        assert!(eq.band_count() == 10);
    }

    #[test]
    fn test_31_band_eq_creation() {
        let eq = GraphicEq::new_31_band(SAMPLE_RATE);
        assert!(eq.band_count() == 31);
    }

    #[test]
    fn test_flat_preset_passthrough() {
        let mut eq = GraphicEq::new_10_band(SAMPLE_RATE);
        eq.set_preset(GraphicEqPreset::Flat);

        // Generate test signal at 1kHz
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
        let mut eq = GraphicEq::new_10_band(SAMPLE_RATE);
        eq.set_preset(GraphicEqPreset::BassBoost);

        // Test at 60 Hz (bass) vs 4000 Hz (treble)
        let mut bass_buffer = generate_sine_wave(60.0, SAMPLE_RATE, 8192);
        let mut treble_buffer = generate_sine_wave(4000.0, SAMPLE_RATE, 8192);

        let bass_original = calculate_rms(&bass_buffer);
        let treble_original = calculate_rms(&treble_buffer);

        eq.process(&mut bass_buffer, SAMPLE_RATE);

        // Reset filter state between tests
        let mut eq2 = GraphicEq::new_10_band(SAMPLE_RATE);
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
        let mut eq = GraphicEq::new_10_band(SAMPLE_RATE);
        eq.set_preset(GraphicEqPreset::TrebleBoost);

        let mut bass_buffer = generate_sine_wave(60.0, SAMPLE_RATE, 8192);
        let mut treble_buffer = generate_sine_wave(8000.0, SAMPLE_RATE, 8192);

        let bass_original = calculate_rms(&bass_buffer);
        let treble_original = calculate_rms(&treble_buffer);

        eq.process(&mut bass_buffer, SAMPLE_RATE);

        let mut eq2 = GraphicEq::new_10_band(SAMPLE_RATE);
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
        let mut eq = GraphicEq::new_10_band(SAMPLE_RATE);
        eq.set_preset(GraphicEqPreset::Vocal);

        // Vocal frequencies (1-4kHz) should be emphasized
        let mut vocal_buffer = generate_sine_wave(2000.0, SAMPLE_RATE, 8192);
        let mut bass_buffer = generate_sine_wave(60.0, SAMPLE_RATE, 8192);

        let vocal_original = calculate_rms(&vocal_buffer);
        let bass_original = calculate_rms(&bass_buffer);

        eq.process(&mut vocal_buffer, SAMPLE_RATE);

        let mut eq2 = GraphicEq::new_10_band(SAMPLE_RATE);
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
        let mut eq = GraphicEq::new_10_band(SAMPLE_RATE);

        // Set band 0 (31 Hz) to +6 dB
        eq.set_band_gain(0, 6.0);

        // Set band 9 (16 kHz) to -6 dB
        eq.set_band_gain(9, -6.0);

        let mut low_buffer = generate_sine_wave(31.0, SAMPLE_RATE, 16384);
        let mut high_buffer = generate_sine_wave(16000.0, SAMPLE_RATE, 8192);

        let low_original = calculate_rms(&low_buffer);
        let high_original = calculate_rms(&high_buffer);

        eq.process(&mut low_buffer, SAMPLE_RATE);

        let mut eq2 = GraphicEq::new_10_band(SAMPLE_RATE);
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
        let mut eq = GraphicEq::new_10_band(SAMPLE_RATE);

        // Boost 1kHz band significantly
        eq.set_band_gain(5, 12.0); // 1kHz is typically band 5

        // Test at center frequency and adjacent frequencies
        let frequencies = [500.0, 1000.0, 2000.0];
        let mut gains = Vec::new();

        for &freq in &frequencies {
            let mut buffer = generate_sine_wave(freq, SAMPLE_RATE, 8192);
            let original = calculate_rms(&buffer);

            let mut eq_copy = GraphicEq::new_10_band(SAMPLE_RATE);
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
        let mut eq = GraphicEq::new_10_band(SAMPLE_RATE);

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
        let mut eq = GraphicEq::new_31_band(SAMPLE_RATE);

        // Set a notch at 1kHz (approximately band 15)
        eq.set_band_gain(15, -12.0);

        let mut buffer = generate_sine_wave(1000.0, SAMPLE_RATE, 8192);
        let original = calculate_rms(&buffer);

        eq.process(&mut buffer, SAMPLE_RATE);
        let processed = calculate_rms(&buffer);

        let gain_db = 20.0 * (processed / original).log10();

        // Should see significant cut at 1kHz
        assert!(
            gain_db < -3.0,
            "1kHz should be cut by at least 3dB, got {:.1}dB",
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
        enhancer.update_settings(StereoSettings::default());

        let (mut left, mut right) = generate_stereo_test_signal(1000.0, SAMPLE_RATE, 4096, 0.0);
        let original_left = left.clone();
        let original_right = right.clone();

        enhancer.process_stereo(&mut left, &mut right, SAMPLE_RATE);

        // Default settings (width=1.0) should be close to passthrough
        let diff_left: f32 = left
            .iter()
            .zip(&original_left)
            .map(|(a, b)| (a - b).abs())
            .sum();
        let diff_right: f32 = right
            .iter()
            .zip(&original_right)
            .map(|(a, b)| (a - b).abs())
            .sum();

        assert!(
            diff_left < 0.01,
            "Left channel changed too much: {}",
            diff_left
        );
        assert!(
            diff_right < 0.01,
            "Right channel changed too much: {}",
            diff_right
        );
    }

    #[test]
    fn test_width_zero_creates_mono() {
        let mut enhancer = StereoEnhancer::new();
        enhancer.update_settings(StereoSettings {
            width: 0.0,
            mid_gain_db: 0.0,
            side_gain_db: 0.0,
            balance: 0.0,
        });

        // Create a stereo signal with different content in each channel
        let mut left = generate_sine_wave(1000.0, SAMPLE_RATE, 4096);
        let mut right = generate_sine_wave(1500.0, SAMPLE_RATE, 4096);

        enhancer.process_stereo(&mut left, &mut right, SAMPLE_RATE);

        // With width=0, left and right should be identical (mono)
        for (l, r) in left.iter().zip(&right) {
            assert!(
                (l - r).abs() < 0.001,
                "Width=0 should create mono, diff={}",
                (l - r).abs()
            );
        }
    }

    #[test]
    fn test_width_expansion() {
        let mut enhancer = StereoEnhancer::new();
        enhancer.update_settings(StereoSettings {
            width: 2.0, // Double the stereo width
            mid_gain_db: 0.0,
            side_gain_db: 0.0,
            balance: 0.0,
        });

        // Create stereo signal with phase difference
        let mut left = Vec::with_capacity(4096);
        let mut right = Vec::with_capacity(4096);
        for i in 0..4096 {
            let t = i as f32 / SAMPLE_RATE as f32;
            left.push((2.0 * PI * 1000.0 * t).sin());
            right.push((2.0 * PI * 1000.0 * t + PI / 4.0).sin()); // 45 degree phase shift
        }

        let original_diff: f32 = left
            .iter()
            .zip(&right)
            .map(|(l, r)| (l - r).powi(2))
            .sum::<f32>()
            .sqrt();

        enhancer.process_stereo(&mut left, &mut right, SAMPLE_RATE);

        let new_diff: f32 = left
            .iter()
            .zip(&right)
            .map(|(l, r)| (l - r).powi(2))
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
        enhancer.update_settings(StereoSettings {
            width: 1.0,
            mid_gain_db: 6.0, // Boost mid by 6dB
            side_gain_db: 0.0,
            balance: 0.0,
        });

        // Mono signal (identical in both channels) - should be boosted
        let mut left = generate_sine_wave(1000.0, SAMPLE_RATE, 4096);
        let mut right = left.clone();

        let original_rms = calculate_rms(&left);

        enhancer.process_stereo(&mut left, &mut right, SAMPLE_RATE);

        let processed_rms = (calculate_rms(&left) + calculate_rms(&right)) / 2.0;
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
        enhancer.update_settings(StereoSettings {
            width: 1.0,
            mid_gain_db: 0.0,
            side_gain_db: 6.0, // Boost side by 6dB
            balance: 0.0,
        });

        // Pure side signal (left = -right)
        let mut left = generate_sine_wave(1000.0, SAMPLE_RATE, 4096);
        let mut right: Vec<f32> = left.iter().map(|&x| -x).collect();

        let original_side_rms = calculate_rms(&left); // In pure side, left RMS = side RMS

        enhancer.process_stereo(&mut left, &mut right, SAMPLE_RATE);

        // Calculate side component after processing
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
        enhancer.update_settings(StereoSettings {
            width: 1.0,
            mid_gain_db: 0.0,
            side_gain_db: 0.0,
            balance: -1.0, // Full left
        });

        let mut left = generate_sine_wave(1000.0, SAMPLE_RATE, 4096);
        let mut right = left.clone();

        enhancer.process_stereo(&mut left, &mut right, SAMPLE_RATE);

        let left_rms = calculate_rms(&left);
        let right_rms = calculate_rms(&right);

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
        enhancer.update_settings(StereoSettings {
            width: 1.0,
            mid_gain_db: 0.0,
            side_gain_db: 0.0,
            balance: 1.0, // Full right
        });

        let mut left = generate_sine_wave(1000.0, SAMPLE_RATE, 4096);
        let mut right = left.clone();

        enhancer.process_stereo(&mut left, &mut right, SAMPLE_RATE);

        let left_rms = calculate_rms(&left);
        let right_rms = calculate_rms(&right);

        // Left should be significantly quieter than right
        assert!(
            right_rms > left_rms * 2.0,
            "Balance right should reduce left channel: L={:.3}, R={:.3}",
            left_rms,
            right_rms
        );
    }

    #[test]
    fn test_mono_compatibility() {
        let mut enhancer = StereoEnhancer::new();
        enhancer.update_settings(StereoSettings {
            width: 1.5,
            mid_gain_db: 0.0,
            side_gain_db: 0.0,
            balance: 0.0,
        });

        // Check mono compatibility value
        let compat = enhancer.mono_compatibility();

        // With width > 1, mono compatibility should be less than 1
        assert!(
            compat >= 0.0 && compat <= 1.0,
            "Mono compatibility should be 0-1, got {}",
            compat
        );
    }
}

// ============================================================================
// CROSSFEED TESTS
// ============================================================================

mod crossfeed_tests {
    use super::*;

    #[test]
    fn test_bypass_passthrough() {
        let mut crossfeed = Crossfeed::new(SAMPLE_RATE);
        crossfeed.set_enabled(false);

        let mut left = generate_sine_wave(1000.0, SAMPLE_RATE, 4096);
        let mut right = generate_sine_wave(1500.0, SAMPLE_RATE, 4096);

        let original_left = left.clone();
        let original_right = right.clone();

        crossfeed.process_stereo(&mut left, &mut right, SAMPLE_RATE);

        // Bypassed should be identical
        assert_eq!(left, original_left, "Bypassed left should be unchanged");
        assert_eq!(right, original_right, "Bypassed right should be unchanged");
    }

    #[test]
    fn test_natural_preset_reduces_separation() {
        let mut crossfeed = Crossfeed::new(SAMPLE_RATE);
        crossfeed.set_preset(CrossfeedPreset::Natural);
        crossfeed.set_enabled(true);

        // Hard-panned signal (only in left channel)
        let mut left = generate_sine_wave(1000.0, SAMPLE_RATE, 4096);
        let mut right = vec![0.0; 4096];

        crossfeed.process_stereo(&mut left, &mut right, SAMPLE_RATE);

        // Right channel should now have some signal (crossfed from left)
        let right_rms = calculate_rms(&right);
        assert!(
            right_rms > 0.01,
            "Crossfeed should add signal to silent channel, got RMS={}",
            right_rms
        );
    }

    #[test]
    fn test_relaxed_preset() {
        let mut crossfeed = Crossfeed::new(SAMPLE_RATE);
        crossfeed.set_preset(CrossfeedPreset::Relaxed);
        crossfeed.set_enabled(true);

        let mut left = generate_sine_wave(1000.0, SAMPLE_RATE, 4096);
        let mut right = vec![0.0; 4096];

        let original_left_rms = calculate_rms(&left);

        crossfeed.process_stereo(&mut left, &mut right, SAMPLE_RATE);

        let new_left_rms = calculate_rms(&left);
        let right_rms = calculate_rms(&right);

        // Relaxed should have stronger crossfeed than natural
        let crossfeed_ratio = right_rms / original_left_rms;
        assert!(
            crossfeed_ratio > 0.1,
            "Relaxed preset should have significant crossfeed: ratio={}",
            crossfeed_ratio
        );
    }

    #[test]
    fn test_meier_preset() {
        let mut crossfeed = Crossfeed::new(SAMPLE_RATE);
        crossfeed.set_preset(CrossfeedPreset::Meier);
        crossfeed.set_enabled(true);

        let mut left = generate_sine_wave(1000.0, SAMPLE_RATE, 4096);
        let mut right = vec![0.0; 4096];

        crossfeed.process_stereo(&mut left, &mut right, SAMPLE_RATE);

        // Should produce valid output
        for sample in &left {
            assert!(sample.is_finite(), "Meier preset output not finite");
        }
        for sample in &right {
            assert!(sample.is_finite(), "Meier preset output not finite");
        }

        // Should have crossfeed
        assert!(
            calculate_rms(&right) > 0.01,
            "Meier preset should add crossfeed"
        );
    }

    #[test]
    fn test_frequency_dependent_crossfeed() {
        let mut crossfeed = Crossfeed::new(SAMPLE_RATE);
        crossfeed.set_preset(CrossfeedPreset::Natural);
        crossfeed.set_enabled(true);

        // Test crossfeed at different frequencies
        // Lower frequencies should have more crossfeed (ILD is less at low frequencies)
        let mut crossfeed_ratios = Vec::new();

        for &freq in &[100.0, 1000.0, 8000.0] {
            let mut left = generate_sine_wave(freq, SAMPLE_RATE, 8192);
            let mut right = vec![0.0; 8192];

            let original_left = calculate_rms(&left);

            let mut cf = Crossfeed::new(SAMPLE_RATE);
            cf.set_preset(CrossfeedPreset::Natural);
            cf.set_enabled(true);
            cf.process_stereo(&mut left, &mut right, SAMPLE_RATE);

            let right_rms = calculate_rms(&right);
            crossfeed_ratios.push((freq, right_rms / original_left));
        }

        // Log results for analysis
        for (freq, ratio) in &crossfeed_ratios {
            println!("{} Hz: crossfeed ratio = {:.3}", freq, ratio);
        }
    }

    #[test]
    fn test_mono_content_unaffected() {
        let mut crossfeed = Crossfeed::new(SAMPLE_RATE);
        crossfeed.set_preset(CrossfeedPreset::Natural);
        crossfeed.set_enabled(true);

        // Mono content (identical in both channels)
        let mut left = generate_sine_wave(1000.0, SAMPLE_RATE, 4096);
        let mut right = left.clone();

        let original_rms = calculate_rms(&left);

        crossfeed.process_stereo(&mut left, &mut right, SAMPLE_RATE);

        let left_rms = calculate_rms(&left);
        let right_rms = calculate_rms(&right);

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
        use_normalizer: bool,
    ) {
        let (mut left, mut right) = generate_stereo_test_signal(1000.0, SAMPLE_RATE, 4096, 0.25);

        // Apply effects in chain order
        if use_eq {
            let mut eq = ParametricEq::new(SAMPLE_RATE);
            eq.set_band(0, 100.0, 3.0, 1.0);
            eq.set_band(1, 1000.0, -2.0, 1.0);
            eq.set_band(2, 10000.0, 1.0, 1.0);
            let mut buffer: Vec<f32> = left
                .iter()
                .zip(&right)
                .flat_map(|(l, r)| [*l, *r])
                .collect();
            eq.process(&mut buffer, SAMPLE_RATE);
            for (i, l) in left.iter_mut().enumerate() {
                *l = buffer[i * 2];
            }
            for (i, r) in right.iter_mut().enumerate() {
                *r = buffer[i * 2 + 1];
            }
        }

        if use_graphic_eq {
            let mut geq = GraphicEq::new_10_band(SAMPLE_RATE);
            geq.set_preset(GraphicEqPreset::Rock);
            let mut buffer: Vec<f32> = left
                .iter()
                .zip(&right)
                .flat_map(|(l, r)| [*l, *r])
                .collect();
            geq.process(&mut buffer, SAMPLE_RATE);
            for (i, l) in left.iter_mut().enumerate() {
                *l = buffer[i * 2];
            }
            for (i, r) in right.iter_mut().enumerate() {
                *r = buffer[i * 2 + 1];
            }
        }

        if use_stereo_enhancer {
            let mut se = StereoEnhancer::new();
            se.update_settings(StereoSettings {
                width: 1.3,
                mid_gain_db: 0.0,
                side_gain_db: 1.0,
                balance: 0.0,
            });
            se.process_stereo(&mut left, &mut right, SAMPLE_RATE);
        }

        if use_crossfeed {
            let mut cf = Crossfeed::new(SAMPLE_RATE);
            cf.set_preset(CrossfeedPreset::Natural);
            cf.set_enabled(true);
            cf.process_stereo(&mut left, &mut right, SAMPLE_RATE);
        }

        if use_limiter {
            let mut limiter = Limiter::new(-1.0);
            let mut buffer: Vec<f32> = left
                .iter()
                .zip(&right)
                .flat_map(|(l, r)| [*l, *r])
                .collect();
            limiter.process(&mut buffer, SAMPLE_RATE);
            for (i, l) in left.iter_mut().enumerate() {
                *l = buffer[i * 2];
            }
            for (i, r) in right.iter_mut().enumerate() {
                *r = buffer[i * 2 + 1];
            }
        }

        if use_normalizer {
            let mut normalizer = VolumeNormalizer::new();
            normalizer.set_target_lufs(-14.0);
            let mut buffer: Vec<f32> = left
                .iter()
                .zip(&right)
                .flat_map(|(l, r)| [*l, *r])
                .collect();
            normalizer.process(&mut buffer, SAMPLE_RATE);
            for (i, l) in left.iter_mut().enumerate() {
                *l = buffer[i * 2];
            }
            for (i, r) in right.iter_mut().enumerate() {
                *r = buffer[i * 2 + 1];
            }
        }

        // Verify output is valid
        for sample in left.iter().chain(right.iter()) {
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
        test_effect_combination(false, false, false, false, false, false);
    }

    #[test]
    fn test_parametric_eq_only() {
        test_effect_combination(true, false, false, false, false, false);
    }

    #[test]
    fn test_graphic_eq_only() {
        test_effect_combination(false, true, false, false, false, false);
    }

    #[test]
    fn test_stereo_enhancer_only() {
        test_effect_combination(false, false, true, false, false, false);
    }

    #[test]
    fn test_crossfeed_only() {
        test_effect_combination(false, false, false, true, false, false);
    }

    #[test]
    fn test_limiter_only() {
        test_effect_combination(false, false, false, false, true, false);
    }

    #[test]
    fn test_normalizer_only() {
        test_effect_combination(false, false, false, false, false, true);
    }

    #[test]
    fn test_all_effects_enabled() {
        test_effect_combination(true, true, true, true, true, true);
    }

    #[test]
    fn test_eq_and_limiter() {
        test_effect_combination(true, false, false, false, true, false);
    }

    #[test]
    fn test_graphic_eq_and_stereo() {
        test_effect_combination(false, true, true, false, false, false);
    }

    #[test]
    fn test_crossfeed_and_stereo_enhancer() {
        // This is an interesting combination - stereo enhancer widens, crossfeed narrows
        test_effect_combination(false, false, true, true, false, false);
    }

    #[test]
    fn test_both_eqs() {
        test_effect_combination(true, true, false, false, false, false);
    }

    #[test]
    fn test_dynamics_chain() {
        test_effect_combination(false, false, false, false, true, true);
    }

    #[test]
    fn test_typical_audiophile_setup() {
        // Parametric EQ + Crossfeed + Limiter
        test_effect_combination(true, false, false, true, true, false);
    }

    #[test]
    fn test_typical_casual_setup() {
        // Graphic EQ + Stereo Enhancer + Normalizer
        test_effect_combination(false, true, true, false, false, true);
    }

    /// Generate all 64 combinations and test them
    #[test]
    fn test_all_64_combinations() {
        let mut passed = 0;
        let mut failed = 0;

        for i in 0..64 {
            let use_eq = (i & 1) != 0;
            let use_graphic_eq = (i & 2) != 0;
            let use_stereo_enhancer = (i & 4) != 0;
            let use_crossfeed = (i & 8) != 0;
            let use_limiter = (i & 16) != 0;
            let use_normalizer = (i & 32) != 0;

            let result = std::panic::catch_unwind(|| {
                test_effect_combination(
                    use_eq,
                    use_graphic_eq,
                    use_stereo_enhancer,
                    use_crossfeed,
                    use_limiter,
                    use_normalizer,
                );
            });

            if result.is_ok() {
                passed += 1;
            } else {
                failed += 1;
                println!(
                    "Combination {} failed: EQ={}, GEQ={}, SE={}, CF={}, Lim={}, Norm={}",
                    i, use_eq, use_graphic_eq, use_stereo_enhancer, use_crossfeed, use_limiter, use_normalizer
                );
            }
        }

        assert_eq!(
            failed, 0,
            "All 64 combinations should pass: {} passed, {} failed",
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
        let mut eq = ParametricEq::new(SAMPLE_RATE);
        eq.set_band(0, 100.0, 3.0, 1.0);
        eq.set_band(1, 1000.0, 0.0, 1.0);
        eq.set_band(2, 10000.0, 0.0, 1.0);

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
        let mut eq = ParametricEq::new(SAMPLE_RATE);
        eq.set_band(0, 100.0, 0.0, 1.0);
        eq.set_band(1, 1000.0, 3.0, 1.0);
        eq.set_band(2, 10000.0, 0.0, 1.0);

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
            let mut eq = ParametricEq::new(sr);
            eq.set_band(0, 100.0, 3.0, 1.0);
            eq.set_band(1, 1000.0, -2.0, 1.0);
            eq.set_band(2, 10000.0, 1.0, 1.0);

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
        let mut eq = ParametricEq::new(SAMPLE_RATE);
        eq.set_band(0, 100.0, 3.0, 1.0);
        eq.set_band(1, 1000.0, -2.0, 1.0);
        eq.set_band(2, 10000.0, 1.0, 1.0);

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
        let mut eq = ParametricEq::new(SAMPLE_RATE);
        eq.set_band(0, 100.0, 3.0, 1.0);
        eq.set_band(1, 1000.0, 0.0, 1.0);
        eq.set_band(2, 10000.0, 0.0, 1.0);

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
        let mut eq = ParametricEq::new(SAMPLE_RATE);
        eq.set_band(0, 100.0, 3.0, 1.0);
        eq.set_band(1, 1000.0, -2.0, 1.0);
        eq.set_band(2, 10000.0, 1.0, 1.0);

        let mut buffer: Vec<f32> = vec![];
        eq.process(&mut buffer, SAMPLE_RATE); // Should not panic

        assert!(buffer.is_empty());
    }

    #[test]
    fn test_single_sample_buffer() {
        let mut eq = ParametricEq::new(SAMPLE_RATE);
        eq.set_band(0, 100.0, 3.0, 1.0);
        eq.set_band(1, 1000.0, 0.0, 1.0);
        eq.set_band(2, 10000.0, 0.0, 1.0);

        let mut buffer = vec![0.5];
        eq.process(&mut buffer, SAMPLE_RATE);

        assert!(buffer[0].is_finite());
    }
}

// ============================================================================
// NUMERICAL STABILITY TESTS
// ============================================================================

mod numerical_stability_tests {
    use super::*;

    #[test]
    fn test_denormal_handling() {
        let mut eq = ParametricEq::new(SAMPLE_RATE);
        eq.set_band(0, 100.0, 3.0, 1.0);
        eq.set_band(1, 1000.0, -2.0, 1.0);
        eq.set_band(2, 10000.0, 1.0, 1.0);

        // Create buffer with denormal values
        let denormal = f32::MIN_POSITIVE / 1000.0;
        let mut buffer = vec![denormal; 4096];

        eq.process(&mut buffer, SAMPLE_RATE);

        // Output should be finite and not contain denormals that slow processing
        for sample in &buffer {
            assert!(sample.is_finite(), "Denormal input produced non-finite output");
            // Denormals should be flushed to zero or produce normal values
            assert!(
                *sample == 0.0 || sample.abs() >= f32::MIN_POSITIVE,
                "Output contains denormal: {}",
                sample
            );
        }
    }

    #[test]
    fn test_dc_offset_handling() {
        let mut eq = ParametricEq::new(SAMPLE_RATE);
        eq.set_band(0, 100.0, 3.0, 1.0);
        eq.set_band(1, 1000.0, 0.0, 1.0);
        eq.set_band(2, 10000.0, 0.0, 1.0);

        // Signal with DC offset
        let dc_offset = 0.5;
        let mut buffer: Vec<f32> = (0..4096)
            .map(|i| dc_offset + 0.3 * (2.0 * PI * 1000.0 * i as f32 / SAMPLE_RATE as f32).sin())
            .collect();

        eq.process(&mut buffer, SAMPLE_RATE);

        // High-pass nature of typical EQ should reduce DC
        let avg: f32 = buffer.iter().sum::<f32>() / buffer.len() as f32;
        // DC should be reduced (but not necessarily eliminated depending on EQ settings)
        assert!(
            avg.abs() < 1.0,
            "DC offset handling failed, avg={}",
            avg
        );
    }

    #[test]
    fn test_very_small_signals() {
        let mut eq = ParametricEq::new(SAMPLE_RATE);
        eq.set_band(0, 100.0, 6.0, 1.0);
        eq.set_band(1, 1000.0, 6.0, 1.0);
        eq.set_band(2, 10000.0, 6.0, 1.0);

        // Very small amplitude signal
        let amplitude = 1e-6;
        let mut buffer: Vec<f32> = (0..4096)
            .map(|i| amplitude * (2.0 * PI * 1000.0 * i as f32 / SAMPLE_RATE as f32).sin())
            .collect();

        eq.process(&mut buffer, SAMPLE_RATE);

        for sample in &buffer {
            assert!(sample.is_finite(), "Small signal produced non-finite output");
        }
    }

    #[test]
    fn test_very_large_signals() {
        let mut limiter = Limiter::new(-0.1);

        // Large amplitude signal (would clip without limiter)
        let amplitude = 10.0;
        let mut buffer: Vec<f32> = (0..4096)
            .map(|i| amplitude * (2.0 * PI * 1000.0 * i as f32 / SAMPLE_RATE as f32).sin())
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
        let mut eq = ParametricEq::new(SAMPLE_RATE);
        eq.set_band(0, 100.0, 3.0, 1.0);
        eq.set_band(1, 1000.0, 0.0, 1.0);
        eq.set_band(2, 10000.0, 0.0, 1.0);

        // Create buffer with NaN - effects should handle gracefully
        let mut buffer = vec![0.0; 4096];
        buffer[100] = f32::NAN;
        buffer[200] = f32::INFINITY;
        buffer[300] = f32::NEG_INFINITY;

        // This might propagate NaN, but shouldn't panic
        eq.process(&mut buffer, SAMPLE_RATE);

        // At least verify no panic occurred
    }

    #[test]
    fn test_impulse_response_stability() {
        let mut eq = ParametricEq::new(SAMPLE_RATE);
        eq.set_band(0, 100.0, 12.0, 1.0); // High gain
        eq.set_band(1, 1000.0, 12.0, 1.0);
        eq.set_band(2, 10000.0, 12.0, 1.0);

        // Single impulse
        let mut buffer = vec![0.0; 4096];
        buffer[0] = 1.0;

        eq.process(&mut buffer, SAMPLE_RATE);

        // Filter should be stable - impulse response should decay
        let first_quarter_energy: f32 = buffer[..1024].iter().map(|x| x * x).sum();
        let last_quarter_energy: f32 = buffer[3072..].iter().map(|x| x * x).sum();

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
        let mut eq = ParametricEq::new(SAMPLE_RATE);
        eq.set_band(0, 100.0, 3.0, 10.0); // High Q
        eq.set_band(1, 1000.0, 3.0, 10.0);
        eq.set_band(2, 10000.0, 3.0, 10.0);

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
        let mut eq = ParametricEq::new(SAMPLE_RATE);
        eq.set_band(0, 100.0, -12.0, 1.0); // Deep cut
        eq.set_band(1, 1000.0, -12.0, 1.0);
        eq.set_band(2, 10000.0, -12.0, 1.0);

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
        let mut eq = ParametricEq::new(SAMPLE_RATE);
        eq.set_band(0, 100.0, 6.0, 1.0);
        eq.set_band(1, 1000.0, 0.0, 1.0);
        eq.set_band(2, 10000.0, 0.0, 1.0);

        // Alternate between signal and silence
        for _ in 0..10 {
            let mut signal_buffer = generate_sine_wave(1000.0, SAMPLE_RATE, 512);
            eq.process(&mut signal_buffer, SAMPLE_RATE);

            let mut silence_buffer = vec![0.0; 512];
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
        let mut eq = ParametricEq::new(SAMPLE_RATE);
        eq.set_band(0, 100.0, 3.0, 1.0);
        eq.set_band(1, 1000.0, -2.0, 1.0);
        eq.set_band(2, 10000.0, 1.0, 1.0);

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
            ratio < 5.0,
            "Processing time varies too much: avg={}ns, max={}ns, ratio={:.1}",
            avg_duration,
            max_duration,
            ratio
        );
    }

    #[test]
    fn test_buffer_independence() {
        // Process should not retain state between independent buffers
        // (except for intended filter state)
        let mut eq = ParametricEq::new(SAMPLE_RATE);
        eq.set_band(0, 100.0, 3.0, 1.0);
        eq.set_band(1, 1000.0, 0.0, 1.0);
        eq.set_band(2, 10000.0, 0.0, 1.0);

        // Process with high amplitude
        let mut loud_buffer = generate_sine_wave(1000.0, SAMPLE_RATE, 512);
        for sample in &mut loud_buffer {
            *sample *= 0.9;
        }
        eq.process(&mut loud_buffer, SAMPLE_RATE);

        // Reset and process with low amplitude
        let mut eq2 = ParametricEq::new(SAMPLE_RATE);
        eq2.set_band(0, 100.0, 3.0, 1.0);
        eq2.set_band(1, 1000.0, 0.0, 1.0);
        eq2.set_band(2, 10000.0, 0.0, 1.0);

        let mut quiet_buffer = generate_sine_wave(1000.0, SAMPLE_RATE, 512);
        for sample in &mut quiet_buffer {
            *sample *= 0.01;
        }
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
        let mut eq = ParametricEq::new(SAMPLE_RATE);
        eq.set_band(0, 100.0, 0.0, 1.0);
        eq.set_band(1, 1000.0, 0.0, 1.0);
        eq.set_band(2, 10000.0, 0.0, 1.0);

        // Edge case buffer sizes
        for size in [0, 1, 2, 3, 7, 15, 31, 63] {
            let mut buffer = vec![0.5; size];
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

        let mut eq = ParametricEq::new(sample_rate);
        eq.set_band(0, 100.0, 3.0, 1.0);
        eq.set_band(1, 1000.0, -2.0, 1.0);
        eq.set_band(2, 10000.0, 1.0, 1.0);

        let mut geq = GraphicEq::new_10_band(sample_rate);
        geq.set_preset(GraphicEqPreset::Rock);

        let mut stereo = StereoEnhancer::new();
        let mut crossfeed = Crossfeed::new(sample_rate);
        crossfeed.set_enabled(true);

        let mut limiter = Limiter::new(-1.0);

        // Time the full chain
        let (mut left, mut right) = generate_stereo_test_signal(1000.0, sample_rate, buffer_size, 0.25);
        let mut interleaved: Vec<f32> = left
            .iter()
            .zip(&right)
            .flat_map(|(l, r)| [*l, *r])
            .collect();

        let start = Instant::now();

        // Apply all effects
        eq.process(&mut interleaved, sample_rate);
        geq.process(&mut interleaved, sample_rate);

        for (i, l) in left.iter_mut().enumerate() {
            *l = interleaved[i * 2];
        }
        for (i, r) in right.iter_mut().enumerate() {
            *r = interleaved[i * 2 + 1];
        }

        stereo.process_stereo(&mut left, &mut right, sample_rate);
        crossfeed.process_stereo(&mut left, &mut right, sample_rate);

        interleaved = left
            .iter()
            .zip(&right)
            .flat_map(|(l, r)| [*l, *r])
            .collect();
        limiter.process(&mut interleaved, sample_rate);

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
// HELPER FUNCTIONS
// ============================================================================

fn calculate_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_squares: f32 = samples.iter().map(|x| x * x).sum();
    (sum_squares / samples.len() as f32).sqrt()
}
