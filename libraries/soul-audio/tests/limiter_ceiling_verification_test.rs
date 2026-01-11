//! Comprehensive limiter ceiling verification tests
//!
//! These tests verify that the limiter properly limits audio output to the specified
//! ceiling (threshold) value. The tests use various input amplitudes and threshold
//! settings to ensure the limiter works correctly in all scenarios.
//!
//! Test Categories:
//! 1. Basic ceiling verification with sine waves
//! 2. Various threshold values from -0.1dB to -12dB
//! 3. Various input levels from just above threshold to extreme (24dB hot)
//! 4. Impulse/transient limiting
//! 5. Square wave limiting
//! 6. Continuous signal limiting over time

use soul_audio::effects::{AudioEffect, Limiter, LimiterSettings};
use std::f32::consts::PI;

const SAMPLE_RATE: u32 = 44100;

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Convert dB to linear amplitude
fn db_to_linear(db: f32) -> f32 {
    10.0f32.powf(db / 20.0)
}

/// Convert linear amplitude to dB
fn linear_to_db(linear: f32) -> f32 {
    if linear <= 0.0 {
        f32::NEG_INFINITY
    } else {
        20.0 * linear.log10()
    }
}

/// Find the maximum absolute sample value in a buffer
fn find_peak(buffer: &[f32]) -> f32 {
    buffer.iter().map(|s| s.abs()).fold(0.0_f32, f32::max)
}

/// Generate a stereo sine wave at the given frequency and amplitude
fn generate_sine_wave(frequency: f32, sample_rate: u32, num_samples: usize, amplitude: f32) -> Vec<f32> {
    let mut buffer = Vec::with_capacity(num_samples * 2);
    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let sample = amplitude * (2.0 * PI * frequency * t).sin();
        buffer.push(sample); // Left
        buffer.push(sample); // Right
    }
    buffer
}

/// Generate a single-sample impulse at specified position
fn generate_impulse(num_samples: usize, impulse_position: usize, amplitude: f32) -> Vec<f32> {
    let mut buffer = vec![0.0_f32; num_samples * 2];
    if impulse_position < num_samples {
        buffer[impulse_position * 2] = amplitude;     // Left
        buffer[impulse_position * 2 + 1] = amplitude; // Right
    }
    buffer
}

/// Generate a square wave signal
fn generate_square_wave(frequency: f32, sample_rate: u32, num_samples: usize, amplitude: f32) -> Vec<f32> {
    let mut buffer = Vec::with_capacity(num_samples * 2);
    let period = sample_rate as f32 / frequency;

    for i in 0..num_samples {
        let phase = (i as f32 % period) / period;
        let sample = if phase < 0.5 { amplitude } else { -amplitude };
        buffer.push(sample);
        buffer.push(sample);
    }
    buffer
}

/// Generate a step response test signal (silence then loud)
fn generate_step_signal(num_samples: usize, step_position: usize, amplitude: f32) -> Vec<f32> {
    let mut buffer = vec![0.0_f32; num_samples * 2];
    for i in step_position..num_samples {
        buffer[i * 2] = amplitude;
        buffer[i * 2 + 1] = amplitude;
    }
    buffer
}

// =============================================================================
// TEST MODULE: BASIC CEILING VERIFICATION
// =============================================================================

mod basic_ceiling_tests {
    use super::*;

    /// Test that a 0dB (amplitude 1.0) sine wave is limited to the threshold
    #[test]
    fn test_0db_sine_limited_to_threshold() {
        let threshold_db = -1.0;
        let threshold_linear = db_to_linear(threshold_db);

        let mut limiter = Limiter::with_settings(LimiterSettings {
            threshold_db,
            release_ms: 100.0,
        });

        // 0dB sine wave (amplitude 1.0)
        let mut buffer = generate_sine_wave(1000.0, SAMPLE_RATE, 4096, 1.0);

        limiter.process(&mut buffer, SAMPLE_RATE);

        let output_peak = find_peak(&buffer);
        let output_peak_db = linear_to_db(output_peak);

        assert!(
            output_peak <= threshold_linear + 0.01,
            "Output peak {:.4} ({:.2} dB) exceeded threshold {:.4} ({:.2} dB)",
            output_peak, output_peak_db, threshold_linear, threshold_db
        );
    }

    /// Test that a +6dB signal is limited to the threshold
    #[test]
    fn test_6db_hot_sine_limited_to_threshold() {
        let threshold_db = -1.0;
        let threshold_linear = db_to_linear(threshold_db);

        let mut limiter = Limiter::with_settings(LimiterSettings {
            threshold_db,
            release_ms: 100.0,
        });

        // +6dB sine wave (amplitude 2.0)
        let amplitude = db_to_linear(6.0);
        let mut buffer = generate_sine_wave(1000.0, SAMPLE_RATE, 4096, amplitude);

        limiter.process(&mut buffer, SAMPLE_RATE);

        let output_peak = find_peak(&buffer);
        let output_peak_db = linear_to_db(output_peak);

        assert!(
            output_peak <= threshold_linear + 0.01,
            "+6dB input: Output peak {:.4} ({:.2} dB) exceeded threshold {:.4} ({:.2} dB)",
            output_peak, output_peak_db, threshold_linear, threshold_db
        );
    }

    /// Test that the limiter passes through signals below threshold unchanged
    #[test]
    fn test_signal_below_threshold_passes_unchanged() {
        let threshold_db = -1.0;

        let mut limiter = Limiter::with_settings(LimiterSettings {
            threshold_db,
            release_ms: 100.0,
        });

        // Signal at -6dB (well below -1dB threshold)
        let amplitude = db_to_linear(-6.0);
        let original = generate_sine_wave(1000.0, SAMPLE_RATE, 4096, amplitude);
        let mut buffer = original.clone();

        limiter.process(&mut buffer, SAMPLE_RATE);

        // Signal should be essentially unchanged
        let max_diff: f32 = buffer.iter()
            .zip(original.iter())
            .map(|(a, b)| (a - b).abs())
            .fold(0.0, f32::max);

        assert!(
            max_diff < 0.001,
            "Signal below threshold was modified: max_diff = {:.6}",
            max_diff
        );
    }
}

// =============================================================================
// TEST MODULE: VARIOUS THRESHOLD VALUES
// =============================================================================

mod threshold_value_tests {
    use super::*;

    /// Test ceiling at -0.1dB (brickwall setting)
    #[test]
    fn test_ceiling_at_minus_0_1db() {
        verify_ceiling_at_threshold(-0.1);
    }

    /// Test ceiling at -0.3dB (default setting)
    #[test]
    fn test_ceiling_at_minus_0_3db() {
        verify_ceiling_at_threshold(-0.3);
    }

    /// Test ceiling at -1dB (soft setting)
    #[test]
    fn test_ceiling_at_minus_1db() {
        verify_ceiling_at_threshold(-1.0);
    }

    /// Test ceiling at -3dB
    #[test]
    fn test_ceiling_at_minus_3db() {
        verify_ceiling_at_threshold(-3.0);
    }

    /// Test ceiling at -6dB
    #[test]
    fn test_ceiling_at_minus_6db() {
        verify_ceiling_at_threshold(-6.0);
    }

    /// Test ceiling at -12dB (extreme limiting)
    #[test]
    fn test_ceiling_at_minus_12db() {
        verify_ceiling_at_threshold(-12.0);
    }

    fn verify_ceiling_at_threshold(threshold_db: f32) {
        let threshold_linear = db_to_linear(threshold_db);

        let mut limiter = Limiter::with_settings(LimiterSettings {
            threshold_db,
            release_ms: 100.0,
        });

        // Test with +12dB hot signal (always above any threshold)
        let amplitude = db_to_linear(12.0);
        let mut buffer = generate_sine_wave(1000.0, SAMPLE_RATE, 4096, amplitude);

        limiter.process(&mut buffer, SAMPLE_RATE);

        let output_peak = find_peak(&buffer);
        let output_peak_db = linear_to_db(output_peak);

        assert!(
            output_peak <= threshold_linear + 0.01,
            "Threshold {:.1} dB: Output peak {:.4} ({:.2} dB) exceeded threshold {:.4}",
            threshold_db, output_peak, output_peak_db, threshold_linear
        );
    }
}

// =============================================================================
// TEST MODULE: VARIOUS INPUT LEVELS
// =============================================================================

mod input_level_tests {
    use super::*;

    const THRESHOLD_DB: f32 = -1.0;

    /// Test with input just above threshold
    #[test]
    fn test_input_just_above_threshold() {
        verify_limiting_at_input_level(0.0); // 0dB is 1dB above -1dB threshold
    }

    /// Test with input 3dB above threshold
    #[test]
    fn test_input_3db_above_threshold() {
        verify_limiting_at_input_level(2.0); // +2dB is 3dB above -1dB threshold
    }

    /// Test with input 6dB above threshold
    #[test]
    fn test_input_6db_above_threshold() {
        verify_limiting_at_input_level(5.0);
    }

    /// Test with input 12dB above threshold
    #[test]
    fn test_input_12db_above_threshold() {
        verify_limiting_at_input_level(11.0);
    }

    /// Test with input 18dB above threshold (very hot)
    #[test]
    fn test_input_18db_above_threshold() {
        verify_limiting_at_input_level(17.0);
    }

    /// Test with input 24dB above threshold (extreme)
    #[test]
    fn test_input_24db_above_threshold() {
        verify_limiting_at_input_level(23.0);
    }

    fn verify_limiting_at_input_level(input_db: f32) {
        let threshold_linear = db_to_linear(THRESHOLD_DB);

        let mut limiter = Limiter::with_settings(LimiterSettings {
            threshold_db: THRESHOLD_DB,
            release_ms: 100.0,
        });

        let amplitude = db_to_linear(input_db);
        let mut buffer = generate_sine_wave(1000.0, SAMPLE_RATE, 4096, amplitude);

        limiter.process(&mut buffer, SAMPLE_RATE);

        let output_peak = find_peak(&buffer);
        let output_peak_db = linear_to_db(output_peak);

        assert!(
            output_peak <= threshold_linear + 0.01,
            "Input {:.1} dB: Output peak {:.4} ({:.2} dB) exceeded threshold {:.4} ({:.1} dB)",
            input_db, output_peak, output_peak_db, threshold_linear, THRESHOLD_DB
        );
    }
}

// =============================================================================
// TEST MODULE: TRANSIENT/IMPULSE LIMITING
// =============================================================================

mod impulse_tests {
    use super::*;

    /// Test that single impulses are limited
    #[test]
    fn test_single_impulse_limited() {
        let threshold_db = -1.0;
        let threshold_linear = db_to_linear(threshold_db);

        let mut limiter = Limiter::with_settings(LimiterSettings {
            threshold_db,
            release_ms: 100.0,
        });

        // Impulse at amplitude 2.0 (+6dB)
        let mut buffer = generate_impulse(1024, 512, 2.0);

        limiter.process(&mut buffer, SAMPLE_RATE);

        let output_peak = find_peak(&buffer);

        assert!(
            output_peak <= threshold_linear + 0.01,
            "Impulse: Output peak {:.4} exceeded threshold {:.4}",
            output_peak, threshold_linear
        );
    }

    /// Test that multiple impulses are limited
    #[test]
    fn test_multiple_impulses_limited() {
        let threshold_db = -1.0;
        let threshold_linear = db_to_linear(threshold_db);

        let mut limiter = Limiter::with_settings(LimiterSettings {
            threshold_db,
            release_ms: 100.0,
        });

        // Create buffer with multiple impulses at different amplitudes
        let mut buffer = vec![0.0_f32; 4096 * 2];
        let impulse_data = [
            (100, 1.5),
            (500, 2.0),
            (1000, 3.0),
            (2000, 1.8),
            (3000, 4.0),
        ];

        for (pos, amp) in impulse_data {
            buffer[pos * 2] = amp;
            buffer[pos * 2 + 1] = amp;
        }

        limiter.process(&mut buffer, SAMPLE_RATE);

        let output_peak = find_peak(&buffer);

        assert!(
            output_peak <= threshold_linear + 0.01,
            "Multiple impulses: Output peak {:.4} exceeded threshold {:.4}",
            output_peak, threshold_linear
        );
    }

    /// Test step response (instant transition from silence to loud)
    #[test]
    fn test_step_response_limited() {
        let threshold_db = -1.0;
        let threshold_linear = db_to_linear(threshold_db);

        let mut limiter = Limiter::with_settings(LimiterSettings {
            threshold_db,
            release_ms: 100.0,
        });

        // Step from 0 to 2.0 at sample 512
        let mut buffer = generate_step_signal(2048, 512, 2.0);

        limiter.process(&mut buffer, SAMPLE_RATE);

        // Check all samples after the step
        let mut max_after_step = 0.0_f32;
        for i in 512..2048 {
            let sample_peak = buffer[i * 2].abs().max(buffer[i * 2 + 1].abs());
            max_after_step = max_after_step.max(sample_peak);
        }

        assert!(
            max_after_step <= threshold_linear + 0.01,
            "Step response: Max output after step {:.4} exceeded threshold {:.4}",
            max_after_step, threshold_linear
        );
    }
}

// =============================================================================
// TEST MODULE: SQUARE WAVE LIMITING
// =============================================================================

mod square_wave_tests {
    use super::*;

    /// Test that square waves are limited (instantaneous transitions)
    #[test]
    fn test_square_wave_limited() {
        let threshold_db = -1.0;
        let threshold_linear = db_to_linear(threshold_db);

        let mut limiter = Limiter::with_settings(LimiterSettings {
            threshold_db,
            release_ms: 50.0,
        });

        // Square wave at amplitude 2.0 (+6dB)
        let mut buffer = generate_square_wave(100.0, SAMPLE_RATE, 4096, 2.0);

        limiter.process(&mut buffer, SAMPLE_RATE);

        let output_peak = find_peak(&buffer);

        assert!(
            output_peak <= threshold_linear + 0.01,
            "Square wave: Output peak {:.4} exceeded threshold {:.4}",
            output_peak, threshold_linear
        );
    }

    /// Test square wave at various amplitudes
    #[test]
    fn test_square_wave_various_amplitudes() {
        let threshold_db = -1.0;
        let threshold_linear = db_to_linear(threshold_db);

        for amplitude in [1.0, 1.5, 2.0, 3.0, 5.0, 10.0] {
            let mut limiter = Limiter::with_settings(LimiterSettings {
                threshold_db,
                release_ms: 50.0,
            });

            let mut buffer = generate_square_wave(100.0, SAMPLE_RATE, 4096, amplitude);

            limiter.process(&mut buffer, SAMPLE_RATE);

            let output_peak = find_peak(&buffer);

            assert!(
                output_peak <= threshold_linear + 0.01,
                "Square wave amplitude {}: Output peak {:.4} exceeded threshold {:.4}",
                amplitude, output_peak, threshold_linear
            );
        }
    }
}

// =============================================================================
// TEST MODULE: CONTINUOUS PROCESSING
// =============================================================================

mod continuous_processing_tests {
    use super::*;

    /// Test continuous processing over time (many buffers)
    #[test]
    fn test_continuous_processing_maintains_ceiling() {
        let threshold_db = -1.0;
        let threshold_linear = db_to_linear(threshold_db);

        let mut limiter = Limiter::with_settings(LimiterSettings {
            threshold_db,
            release_ms: 100.0,
        });

        // Process 100 buffers of 1024 samples each (about 2.3 seconds at 44.1kHz)
        for i in 0..100 {
            let amplitude = db_to_linear(6.0); // +6dB
            let mut buffer = generate_sine_wave(1000.0, SAMPLE_RATE, 1024, amplitude);

            limiter.process(&mut buffer, SAMPLE_RATE);

            let output_peak = find_peak(&buffer);

            assert!(
                output_peak <= threshold_linear + 0.01,
                "Buffer {}: Output peak {:.4} exceeded threshold {:.4}",
                i, output_peak, threshold_linear
            );
        }
    }

    /// Test that limiter maintains ceiling across varying input levels
    #[test]
    fn test_varying_input_levels_maintains_ceiling() {
        let threshold_db = -1.0;
        let threshold_linear = db_to_linear(threshold_db);

        let mut limiter = Limiter::with_settings(LimiterSettings {
            threshold_db,
            release_ms: 100.0,
        });

        // Alternate between quiet and loud signals
        let input_levels_db = [-10.0, 6.0, -3.0, 12.0, 0.0, 18.0, -6.0, 3.0];

        for (i, &level_db) in input_levels_db.iter().enumerate() {
            let amplitude = db_to_linear(level_db);
            let mut buffer = generate_sine_wave(1000.0, SAMPLE_RATE, 2048, amplitude);

            limiter.process(&mut buffer, SAMPLE_RATE);

            let output_peak = find_peak(&buffer);

            // Only check for signals above threshold
            if level_db > threshold_db {
                assert!(
                    output_peak <= threshold_linear + 0.01,
                    "Buffer {} (input {:.1} dB): Output peak {:.4} exceeded threshold {:.4}",
                    i, level_db, output_peak, threshold_linear
                );
            }
        }
    }
}

// =============================================================================
// TEST MODULE: OUTPUT VERIFICATION (EXACT VALUES)
// =============================================================================

mod exact_output_tests {
    use super::*;

    /// Verify that output peak exactly matches threshold (not just below it)
    #[test]
    fn test_output_approaches_threshold_for_hot_input() {
        let threshold_db = -1.0;
        let threshold_linear = db_to_linear(threshold_db);

        let mut limiter = Limiter::with_settings(LimiterSettings {
            threshold_db,
            release_ms: 100.0,
        });

        // Very hot input (+12dB)
        let amplitude = db_to_linear(12.0);
        let mut buffer = generate_sine_wave(1000.0, SAMPLE_RATE, 8192, amplitude);

        limiter.process(&mut buffer, SAMPLE_RATE);

        // Find peak in the second half (after limiter has settled)
        let second_half = &buffer[buffer.len()/2..];
        let output_peak = find_peak(second_half);

        // Output should be very close to threshold (within 0.5dB)
        let output_db = linear_to_db(output_peak);
        let difference_db = (output_db - threshold_db).abs();

        assert!(
            difference_db < 0.5,
            "Output peak {:.4} ({:.2} dB) should be close to threshold {:.4} ({:.1} dB), diff = {:.2} dB",
            output_peak, output_db, threshold_linear, threshold_db, difference_db
        );
    }

    /// Verify that the limiter doesn't boost signals (gain <= 1.0)
    #[test]
    fn test_limiter_never_boosts_signal() {
        let threshold_db = -3.0;

        let mut limiter = Limiter::with_settings(LimiterSettings {
            threshold_db,
            release_ms: 100.0,
        });

        // Signal below threshold should not be boosted
        let amplitude = db_to_linear(-6.0);
        let original = generate_sine_wave(1000.0, SAMPLE_RATE, 4096, amplitude);
        let mut buffer = original.clone();

        limiter.process(&mut buffer, SAMPLE_RATE);

        // No sample should be larger than original
        for (out, orig) in buffer.iter().zip(original.iter()) {
            assert!(
                out.abs() <= orig.abs() + 0.001,
                "Limiter boosted signal: output {:.4} > original {:.4}",
                out.abs(), orig.abs()
            );
        }
    }
}

// =============================================================================
// TEST MODULE: SAMPLE RATE INDEPENDENCE
// =============================================================================

mod sample_rate_tests {
    use super::*;

    /// Test that ceiling is maintained at different sample rates
    #[test]
    fn test_ceiling_at_various_sample_rates() {
        let sample_rates = [44100, 48000, 88200, 96000, 176400, 192000];
        let threshold_db = -1.0;
        let threshold_linear = db_to_linear(threshold_db);

        for &sr in &sample_rates {
            let mut limiter = Limiter::with_settings(LimiterSettings {
                threshold_db,
                release_ms: 100.0,
            });

            let amplitude = db_to_linear(6.0);
            let num_samples = sr as usize / 10; // 100ms of audio
            let mut buffer = generate_sine_wave(1000.0, sr, num_samples, amplitude);

            limiter.process(&mut buffer, sr);

            let output_peak = find_peak(&buffer);

            assert!(
                output_peak <= threshold_linear + 0.01,
                "Sample rate {} Hz: Output peak {:.4} exceeded threshold {:.4}",
                sr, output_peak, threshold_linear
            );
        }
    }
}

// =============================================================================
// SUMMARY TEST
// =============================================================================

#[test]
fn comprehensive_ceiling_verification() {
    let mut tests_passed = 0;
    let mut tests_failed = 0;
    let mut failures: Vec<String> = Vec::new();

    let threshold_db = -1.0;
    let threshold_linear = db_to_linear(threshold_db);

    // Test matrix: various input levels and signal types
    let input_levels_db = [0.0, 3.0, 6.0, 12.0, 18.0];
    let frequencies = [100.0, 1000.0, 10000.0];

    println!("\n========================================");
    println!("LIMITER CEILING VERIFICATION SUMMARY");
    println!("========================================");
    println!("Threshold: {} dB ({:.4} linear)", threshold_db, threshold_linear);
    println!("");

    for &input_db in &input_levels_db {
        for &freq in &frequencies {
            let mut limiter = Limiter::with_settings(LimiterSettings {
                threshold_db,
                release_ms: 100.0,
            });

            let amplitude = db_to_linear(input_db);
            let mut buffer = generate_sine_wave(freq, SAMPLE_RATE, 4096, amplitude);

            limiter.process(&mut buffer, SAMPLE_RATE);

            let output_peak = find_peak(&buffer);
            let output_db = linear_to_db(output_peak);

            if output_peak <= threshold_linear + 0.01 {
                tests_passed += 1;
                println!("PASS: Input {:.0} dB @ {} Hz -> Output {:.2} dB", input_db, freq, output_db);
            } else {
                tests_failed += 1;
                let msg = format!("FAIL: Input {:.0} dB @ {} Hz -> Output {:.2} dB (exceeded by {:.2} dB)",
                    input_db, freq, output_db, output_db - threshold_db);
                println!("{}", msg);
                failures.push(msg);
            }
        }
    }

    println!("");
    println!("========================================");
    println!("Results: {} passed, {} failed", tests_passed, tests_failed);
    println!("========================================");

    if !failures.is_empty() {
        println!("\nFailures:");
        for failure in &failures {
            println!("  - {}", failure);
        }
    }

    assert_eq!(tests_failed, 0, "Ceiling verification tests failed");
}
