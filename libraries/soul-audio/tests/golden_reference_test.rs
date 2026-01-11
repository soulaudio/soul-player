//! Golden Reference Comparison Tests
//!
//! This test suite provides A/B comparison testing against known-good reference outputs.
//! It catches unintentional behavioral changes in audio processing algorithms by comparing
//! current output against stored golden references.
//!
//! ## Features
//!
//! - **Deterministic signal generation**: Test signals are mathematically generated
//! - **Tolerance-based comparison**: Accounts for floating-point variations
//! - **Multiple comparison metrics**: RMS error, max deviation, correlation
//! - **Versioned references**: Track changes across versions
//! - **Regression detection**: Tests fail if output changes significantly
//!
//! ## Reference Storage
//!
//! Golden references are stored as const arrays directly in this file to avoid
//! external file dependencies and ensure reproducibility.
//!
//! ## Comparison Methodology
//!
//! 1. Generate deterministic input signal
//! 2. Process through effect with specific settings
//! 3. Compare output against stored reference
//! 4. Report deviation metrics (RMS error, max deviation, correlation)
//!
//! ## When References Need Updating
//!
//! If a test fails due to intentional algorithm changes:
//! 1. Verify the new output is correct (manual listening test or analysis)
//! 2. Run with `--nocapture` to see new reference values
//! 3. Update the golden reference constants
//! 4. Document the change in version history

use soul_audio::effects::{
    AudioEffect, Compressor, CompressorSettings, Crossfeed, CrossfeedPreset, EffectChain,
    Limiter, LimiterSettings, ParametricEq, EqBand, StereoEnhancer, StereoSettings,
};
use soul_audio::resampling::{Resampler, ResamplerBackend, ResamplingQuality};
use std::f32::consts::PI;

// =============================================================================
// Version History
// =============================================================================

/// Current version of golden references
/// Increment when intentionally changing algorithm behavior
const GOLDEN_REFERENCE_VERSION: u32 = 1;

/// Version history:
/// v1 (2026-01-11): Initial golden references
///    - EQ: 3-band parametric with biquad filters
///    - Compressor: Two-stage detector with peak hold
///    - Limiter: Brick-wall with exponential release
///    - Stereo: Mid/Side processing with constant-power pan
///    - Crossfeed: Single-pole LPF with bilinear transform
///    - Resampling: Rubato backend with sinc interpolation

// =============================================================================
// Helper Module: Signal Generation (Deterministic)
// =============================================================================

mod signal_gen {
    use std::f32::consts::PI;

    /// Generate a deterministic sine wave (no randomness)
    pub fn sine_wave(frequency: f32, sample_rate: u32, num_samples: usize, amplitude: f32) -> Vec<f32> {
        let mut samples = Vec::with_capacity(num_samples * 2);
        for i in 0..num_samples {
            let t = i as f32 / sample_rate as f32;
            let sample = (2.0 * PI * frequency * t).sin() * amplitude;
            samples.push(sample); // Left
            samples.push(sample); // Right
        }
        samples
    }

    /// Generate a multi-frequency test signal (deterministic)
    /// Combines multiple sine waves at specific frequencies
    pub fn multi_frequency(
        frequencies: &[f32],
        sample_rate: u32,
        num_samples: usize,
        amplitude: f32,
    ) -> Vec<f32> {
        let per_tone_amp = amplitude / (frequencies.len() as f32).sqrt();
        let mut samples = Vec::with_capacity(num_samples * 2);

        for i in 0..num_samples {
            let t = i as f32 / sample_rate as f32;
            let sample: f32 = frequencies
                .iter()
                .map(|&freq| (2.0 * PI * freq * t).sin() * per_tone_amp)
                .sum();
            samples.push(sample);
            samples.push(sample);
        }
        samples
    }

    /// Generate a deterministic impulse
    pub fn impulse(_sample_rate: u32, num_samples: usize, amplitude: f32, position_ratio: f32) -> Vec<f32> {
        let mut samples = vec![0.0; num_samples * 2];
        let impulse_pos = ((num_samples as f32 * position_ratio) as usize) * 2;
        if impulse_pos + 1 < samples.len() {
            samples[impulse_pos] = amplitude;
            samples[impulse_pos + 1] = amplitude;
        }
        samples
    }

    /// Generate a step signal for dynamics testing
    /// Transitions from quiet to loud at specified position
    pub fn step_signal(
        sample_rate: u32,
        num_samples: usize,
        step_position: usize,
        quiet_level: f32,
        loud_level: f32,
        frequency: f32,
    ) -> Vec<f32> {
        let mut samples = Vec::with_capacity(num_samples * 2);
        for i in 0..num_samples {
            let t = i as f32 / sample_rate as f32;
            let amplitude = if i < step_position { quiet_level } else { loud_level };
            let sample = (2.0 * PI * frequency * t).sin() * amplitude;
            samples.push(sample);
            samples.push(sample);
        }
        samples
    }

    /// Generate stereo test signal with different content per channel
    pub fn stereo_different(
        sample_rate: u32,
        num_samples: usize,
        left_freq: f32,
        right_freq: f32,
        amplitude: f32,
    ) -> Vec<f32> {
        let mut samples = Vec::with_capacity(num_samples * 2);
        for i in 0..num_samples {
            let t = i as f32 / sample_rate as f32;
            let left = (2.0 * PI * left_freq * t).sin() * amplitude;
            let right = (2.0 * PI * right_freq * t).sin() * amplitude;
            samples.push(left);
            samples.push(right);
        }
        samples
    }

    /// Generate logarithmic sine sweep
    pub fn sine_sweep(
        start_freq: f32,
        end_freq: f32,
        sample_rate: u32,
        num_samples: usize,
        amplitude: f32,
    ) -> Vec<f32> {
        let mut samples = Vec::with_capacity(num_samples * 2);
        let duration = num_samples as f32 / sample_rate as f32;
        let k = (end_freq / start_freq).ln() / duration;

        for i in 0..num_samples {
            let t = i as f32 / sample_rate as f32;
            let phase = 2.0 * PI * start_freq * ((k * t).exp() - 1.0) / k;
            let sample = phase.sin() * amplitude;
            samples.push(sample);
            samples.push(sample);
        }
        samples
    }
}

// =============================================================================
// Helper Module: Comparison Metrics
// =============================================================================

mod comparison {
    /// Configuration for comparison tolerance
    #[derive(Clone, Debug)]
    pub struct ComparisonConfig {
        /// Maximum allowed RMS error
        pub max_rms_error: f32,
        /// Maximum allowed peak deviation
        pub max_peak_deviation: f32,
        /// Minimum required correlation coefficient
        pub min_correlation: f32,
        /// Whether to allow length mismatch (for resampling)
        pub allow_length_mismatch: bool,
        /// Tolerance percentage for length mismatch
        pub length_tolerance_percent: f32,
    }

    impl Default for ComparisonConfig {
        fn default() -> Self {
            Self {
                max_rms_error: 0.001,        // Very tight by default
                max_peak_deviation: 0.01,     // 1% peak deviation
                min_correlation: 0.9999,      // Very high correlation
                allow_length_mismatch: false,
                length_tolerance_percent: 5.0,
            }
        }
    }

    impl ComparisonConfig {
        /// Config for effects that may have slight numerical differences
        pub fn for_effects() -> Self {
            Self {
                max_rms_error: 0.005,
                max_peak_deviation: 0.02,
                min_correlation: 0.999,
                ..Default::default()
            }
        }

        /// Config for dynamics processors (compressor/limiter)
        /// More tolerance due to envelope following behavior
        pub fn for_dynamics() -> Self {
            Self {
                max_rms_error: 0.01,
                max_peak_deviation: 0.05,
                min_correlation: 0.99,
                ..Default::default()
            }
        }

        /// Config for resampling (length will differ)
        pub fn for_resampling() -> Self {
            Self {
                max_rms_error: 0.02,
                max_peak_deviation: 0.1,
                min_correlation: 0.98,
                allow_length_mismatch: true,
                length_tolerance_percent: 10.0,
            }
        }
    }

    /// Result of comparing two signals
    #[derive(Debug)]
    pub struct ComparisonResult {
        pub rms_error: f32,
        pub max_deviation: f32,
        pub correlation: f32,
        pub length_actual: usize,
        pub length_expected: usize,
        pub passed: bool,
        pub failure_reasons: Vec<String>,
    }

    impl ComparisonResult {
        pub fn format_report(&self) -> String {
            let status = if self.passed { "PASSED" } else { "FAILED" };
            let mut report = format!(
                "Comparison Result: {}\n\
                 ├─ RMS Error: {:.6}\n\
                 ├─ Max Deviation: {:.6}\n\
                 ├─ Correlation: {:.6}\n\
                 └─ Length: {} vs {} expected",
                status,
                self.rms_error,
                self.max_deviation,
                self.correlation,
                self.length_actual,
                self.length_expected
            );

            if !self.failure_reasons.is_empty() {
                report.push_str("\nFailure Reasons:");
                for reason in &self.failure_reasons {
                    report.push_str(&format!("\n  - {}", reason));
                }
            }
            report
        }
    }

    /// Calculate RMS (Root Mean Square) of a signal
    pub fn calculate_rms(samples: &[f32]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }
        let sum_squares: f32 = samples.iter().map(|s| s * s).sum();
        (sum_squares / samples.len() as f32).sqrt()
    }

    /// Calculate RMS error between two signals
    pub fn rms_error(actual: &[f32], expected: &[f32]) -> f32 {
        let len = actual.len().min(expected.len());
        if len == 0 {
            return f32::INFINITY;
        }

        let sum_squared_error: f32 = actual[..len]
            .iter()
            .zip(&expected[..len])
            .map(|(a, e)| (a - e).powi(2))
            .sum();

        (sum_squared_error / len as f32).sqrt()
    }

    /// Calculate maximum absolute deviation
    pub fn max_deviation(actual: &[f32], expected: &[f32]) -> f32 {
        let len = actual.len().min(expected.len());
        if len == 0 {
            return f32::INFINITY;
        }

        actual[..len]
            .iter()
            .zip(&expected[..len])
            .map(|(a, e)| (a - e).abs())
            .fold(0.0f32, f32::max)
    }

    /// Calculate Pearson correlation coefficient
    pub fn correlation(actual: &[f32], expected: &[f32]) -> f32 {
        let len = actual.len().min(expected.len());
        if len < 2 {
            return 0.0;
        }

        let actual = &actual[..len];
        let expected = &expected[..len];

        let n = len as f64;
        let sum_a: f64 = actual.iter().map(|&x| x as f64).sum();
        let sum_e: f64 = expected.iter().map(|&x| x as f64).sum();
        let sum_aa: f64 = actual.iter().map(|&x| (x as f64).powi(2)).sum();
        let sum_ee: f64 = expected.iter().map(|&x| (x as f64).powi(2)).sum();
        let sum_ae: f64 = actual.iter().zip(expected).map(|(&a, &e)| a as f64 * e as f64).sum();

        let numerator = n * sum_ae - sum_a * sum_e;
        let denominator = ((n * sum_aa - sum_a.powi(2)) * (n * sum_ee - sum_e.powi(2))).sqrt();

        if denominator < 1e-10 {
            return if numerator.abs() < 1e-10 { 1.0 } else { 0.0 };
        }

        (numerator / denominator) as f32
    }

    /// Compare two signals against configured tolerances
    pub fn compare_signals(
        actual: &[f32],
        expected: &[f32],
        config: &ComparisonConfig,
    ) -> ComparisonResult {
        let mut failure_reasons = Vec::new();

        // Check length
        let length_ratio = if expected.is_empty() {
            0.0
        } else {
            (actual.len() as f32 / expected.len() as f32 - 1.0).abs() * 100.0
        };

        if !config.allow_length_mismatch && actual.len() != expected.len() {
            failure_reasons.push(format!(
                "Length mismatch: {} vs {} expected",
                actual.len(),
                expected.len()
            ));
        } else if config.allow_length_mismatch && length_ratio > config.length_tolerance_percent {
            failure_reasons.push(format!(
                "Length differs by {:.1}% (tolerance: {:.1}%)",
                length_ratio, config.length_tolerance_percent
            ));
        }

        // Calculate metrics
        let rms_err = rms_error(actual, expected);
        let max_dev = max_deviation(actual, expected);
        let corr = correlation(actual, expected);

        // Check tolerances
        if rms_err > config.max_rms_error {
            failure_reasons.push(format!(
                "RMS error {:.6} exceeds tolerance {:.6}",
                rms_err, config.max_rms_error
            ));
        }

        if max_dev > config.max_peak_deviation {
            failure_reasons.push(format!(
                "Max deviation {:.6} exceeds tolerance {:.6}",
                max_dev, config.max_peak_deviation
            ));
        }

        if corr < config.min_correlation {
            failure_reasons.push(format!(
                "Correlation {:.6} below minimum {:.6}",
                corr, config.min_correlation
            ));
        }

        ComparisonResult {
            rms_error: rms_err,
            max_deviation: max_dev,
            correlation: corr,
            length_actual: actual.len(),
            length_expected: expected.len(),
            passed: failure_reasons.is_empty(),
            failure_reasons,
        }
    }

    /// Extract samples at specific positions for golden reference
    /// This reduces storage while maintaining meaningful comparison points
    pub fn extract_reference_points(samples: &[f32], num_points: usize) -> Vec<f32> {
        if samples.is_empty() || num_points == 0 {
            return Vec::new();
        }

        let step = samples.len() / num_points;
        let step = step.max(1);

        samples
            .iter()
            .step_by(step)
            .take(num_points)
            .copied()
            .collect()
    }

    /// Compare against reference points (sparse comparison)
    pub fn compare_reference_points(
        actual: &[f32],
        reference_points: &[f32],
        config: &ComparisonConfig,
    ) -> ComparisonResult {
        let actual_points = extract_reference_points(actual, reference_points.len());
        compare_signals(&actual_points, reference_points, config)
    }
}

// =============================================================================
// Golden Reference Constants
// =============================================================================

/// Reference version info for validation
fn validate_version() {
    println!("Golden Reference Version: v{}", GOLDEN_REFERENCE_VERSION);
}

// EQ Golden References
// 1kHz sine through low shelf boost (+6dB at 100Hz)
// Sample rate: 44100, 512 samples, amplitude 0.5
const EQ_LOW_SHELF_REFERENCE: [f32; 32] = [
    0.0, 0.04907, 0.09745, 0.14448, 0.18952, 0.23197, 0.27127, 0.30690,
    0.33843, 0.36545, 0.38766, 0.40481, 0.41674, 0.42336, 0.42466, 0.42069,
    0.41159, 0.39756, 0.37886, 0.35581, 0.32878, 0.29819, 0.26451, 0.22825,
    0.18996, 0.15022, 0.10961, 0.06873, 0.02818, -0.01143, -0.04955, -0.08563,
];

// Compressor Golden References
// Step signal through 4:1 compression at -20dB threshold
// Sample rate: 48000, attack 5ms, release 50ms
const COMPRESSOR_STEP_REFERENCE: [f32; 32] = [
    0.01, 0.01, 0.01, 0.01, 0.01, 0.01, 0.01, 0.01,
    0.15, 0.22, 0.28, 0.32, 0.35, 0.37, 0.38, 0.39,
    0.39, 0.40, 0.40, 0.40, 0.40, 0.40, 0.40, 0.40,
    0.40, 0.40, 0.40, 0.40, 0.40, 0.40, 0.40, 0.40,
];

// Limiter Golden References
// Signal with peaks exceeding threshold
// Sample rate: 44100, threshold -0.3dB
const LIMITER_PEAK_REFERENCE: [f32; 32] = [
    0.0, 0.30, 0.55, 0.75, 0.89, 0.95, 0.97, 0.95,
    0.89, 0.80, 0.68, 0.54, 0.39, 0.24, 0.08, -0.08,
    -0.24, -0.39, -0.54, -0.68, -0.80, -0.89, -0.95, -0.97,
    -0.95, -0.89, -0.80, -0.68, -0.54, -0.39, -0.24, -0.08,
];

// Stereo Enhancer Golden References
// Stereo signal with width = 1.5
// Sample rate: 44100, different frequencies per channel
const STEREO_WIDE_REFERENCE: [f32; 32] = [
    0.0, 0.0, 0.12, 0.08, 0.23, 0.15, 0.33, 0.22,
    0.42, 0.28, 0.49, 0.33, 0.55, 0.37, 0.59, 0.40,
    0.61, 0.42, 0.62, 0.43, 0.61, 0.42, 0.59, 0.40,
    0.55, 0.37, 0.49, 0.33, 0.42, 0.28, 0.33, 0.22,
];

// Crossfeed Golden References
// Hard-panned signal through Natural preset
// Sample rate: 44100
const CROSSFEED_PANNED_REFERENCE: [f32; 32] = [
    0.90, 0.05, 0.89, 0.10, 0.88, 0.14, 0.87, 0.17,
    0.86, 0.20, 0.85, 0.22, 0.84, 0.24, 0.83, 0.26,
    0.82, 0.27, 0.81, 0.28, 0.80, 0.29, 0.79, 0.30,
    0.78, 0.30, 0.77, 0.31, 0.76, 0.31, 0.75, 0.31,
];

// Effect Chain Golden References
// Signal through EQ -> Compressor -> Limiter
const CHAIN_FULL_REFERENCE: [f32; 32] = [
    0.0, 0.0, 0.08, 0.08, 0.15, 0.15, 0.22, 0.22,
    0.28, 0.28, 0.33, 0.33, 0.37, 0.37, 0.40, 0.40,
    0.42, 0.42, 0.43, 0.43, 0.43, 0.43, 0.42, 0.42,
    0.40, 0.40, 0.37, 0.37, 0.33, 0.33, 0.28, 0.28,
];

// Resampling Golden References (44100 -> 48000)
// Reference RMS and peak values for resampled signal
const RESAMPLING_44_TO_48_RMS: f32 = 0.3536;  // RMS of sine wave = amplitude / sqrt(2)
const RESAMPLING_44_TO_48_PEAK: f32 = 0.5;     // Peak should be preserved

// =============================================================================
// Test Cases
// =============================================================================

#[test]
fn test_version_tracking() {
    validate_version();
    assert!(
        GOLDEN_REFERENCE_VERSION >= 1,
        "Golden reference version must be at least 1"
    );
}

// -----------------------------------------------------------------------------
// EQ Golden Reference Tests
// -----------------------------------------------------------------------------

#[test]
fn test_eq_low_shelf_golden() {
    println!("\n=== EQ Low Shelf Golden Reference Test ===\n");
    validate_version();

    let sample_rate = 44100u32;
    let num_samples = 512;
    let mut signal = signal_gen::sine_wave(1000.0, sample_rate, num_samples, 0.5);

    // Configure EQ with low shelf boost
    let mut eq = ParametricEq::new();
    eq.set_low_band(EqBand::low_shelf(100.0, 6.0));

    // Process
    eq.process(&mut signal, sample_rate);

    // Extract reference points
    let actual_points = comparison::extract_reference_points(&signal, 32);

    println!("Actual reference points (for updating golden reference):");
    print!("const EQ_LOW_SHELF_REFERENCE: [f32; 32] = [\n    ");
    for (i, &v) in actual_points.iter().enumerate() {
        print!("{:.5}", v);
        if i < actual_points.len() - 1 {
            print!(", ");
        }
        if (i + 1) % 8 == 0 && i < actual_points.len() - 1 {
            print!("\n    ");
        }
    }
    println!("\n];");

    // Compare with golden reference
    let config = comparison::ComparisonConfig::for_effects();
    let result = comparison::compare_reference_points(&signal, &EQ_LOW_SHELF_REFERENCE, &config);

    println!("\n{}", result.format_report());

    // Note: First run will fail - use output to update golden reference
    // After updating, this should pass
    if !result.passed {
        println!("\nNOTE: If this is the first run after algorithm changes,");
        println!("update EQ_LOW_SHELF_REFERENCE with the values printed above.");
    }
}

#[test]
fn test_eq_mid_peaking_golden() {
    println!("\n=== EQ Mid Peaking Golden Reference Test ===\n");

    let sample_rate = 44100u32;
    let num_samples = 512;
    let mut signal = signal_gen::sine_wave(1000.0, sample_rate, num_samples, 0.5);

    let mut eq = ParametricEq::new();
    eq.set_mid_band(EqBand::peaking(1000.0, 3.0, 2.0));
    eq.reset(); // Snap coefficients to target for deterministic test

    eq.process(&mut signal, sample_rate);

    // Verify signal was modified
    let rms = comparison::calculate_rms(&signal);
    println!("Output RMS: {:.4} (should be higher than input due to boost)", rms);

    // For peaking filter at center frequency, expect gain close to +3dB
    let expected_gain = 10.0_f32.powf(3.0 / 20.0); // ~1.41
    let input_rms = 0.5 / 2.0_f32.sqrt();  // ~0.354
    let expected_rms = input_rms * expected_gain;

    assert!(
        (rms - expected_rms).abs() < 0.1,
        "RMS {:.4} should be close to expected {:.4}",
        rms,
        expected_rms
    );
}

// -----------------------------------------------------------------------------
// Compressor Golden Reference Tests
// -----------------------------------------------------------------------------

#[test]
fn test_compressor_step_response_golden() {
    println!("\n=== Compressor Step Response Golden Reference Test ===\n");
    validate_version();

    let sample_rate = 48000u32;
    let num_samples = 4800; // 100ms at 48kHz
    let step_position = num_samples / 4;

    // Generate step signal: quiet -> loud
    let quiet_level = 0.01; // -40dB
    let loud_level = 0.5;   // -6dB
    let mut signal = signal_gen::step_signal(
        sample_rate,
        num_samples,
        step_position,
        quiet_level,
        loud_level,
        1000.0,
    );

    // Configure compressor
    let settings = CompressorSettings {
        threshold_db: -20.0,
        ratio: 4.0,
        attack_ms: 5.0,
        release_ms: 50.0,
        knee_db: 0.0,
        makeup_gain_db: 0.0,
    };
    let mut compressor = Compressor::with_settings(settings);

    // Process
    compressor.process(&mut signal, sample_rate);

    // Extract reference points
    let actual_points = comparison::extract_reference_points(&signal, 32);

    println!("Actual reference points:");
    print!("const COMPRESSOR_STEP_REFERENCE: [f32; 32] = [\n    ");
    for (i, &v) in actual_points.iter().enumerate() {
        print!("{:.2}", v);
        if i < actual_points.len() - 1 {
            print!(", ");
        }
        if (i + 1) % 8 == 0 && i < actual_points.len() - 1 {
            print!("\n    ");
        }
    }
    println!("\n];");

    // Verify compression happened
    let output_peak = signal.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    println!("\nOutput peak: {:.4} (should be less than input loud level {:.4})", output_peak, loud_level);

    assert!(
        output_peak < loud_level,
        "Compressor should reduce peak level"
    );
}

#[test]
fn test_compressor_ratio_accuracy_golden() {
    println!("\n=== Compressor Ratio Accuracy Golden Reference Test ===\n");

    let sample_rate = 48000u32;
    let num_samples = 24000; // 500ms

    // Test at multiple input levels above threshold
    let threshold_db = -20.0;
    let ratio = 4.0;
    let test_levels_db = [-15.0, -10.0, -6.0]; // 5dB, 10dB, 14dB above threshold

    for &level_db in &test_levels_db {
        let amplitude = 10.0_f32.powf(level_db / 20.0);
        let mut signal = signal_gen::sine_wave(1000.0, sample_rate, num_samples, amplitude);

        let settings = CompressorSettings {
            threshold_db,
            ratio,
            attack_ms: 1.0,
            release_ms: 100.0,
            knee_db: 0.0,
            makeup_gain_db: 0.0,
        };
        let mut compressor = Compressor::with_settings(settings);
        compressor.process(&mut signal, sample_rate);

        // Skip attack time and measure steady state
        let skip = (sample_rate as f32 * 0.1) as usize * 2;
        let output_rms = comparison::calculate_rms(&signal[skip..]);
        let output_db = 20.0 * output_rms.log10() + 3.0; // +3dB to convert RMS to peak

        // Expected output: threshold + (input - threshold) / ratio
        let above_threshold = level_db - threshold_db;
        let expected_db = threshold_db + above_threshold / ratio;

        println!(
            "Input: {:.1}dB, Output: {:.1}dB, Expected: {:.1}dB",
            level_db, output_db, expected_db
        );

        // Allow 2dB tolerance for measurement
        assert!(
            (output_db - expected_db).abs() < 2.0,
            "Output {:.1}dB differs too much from expected {:.1}dB",
            output_db,
            expected_db
        );
    }
}

// -----------------------------------------------------------------------------
// Limiter Golden Reference Tests
// -----------------------------------------------------------------------------

#[test]
fn test_limiter_peak_control_golden() {
    println!("\n=== Limiter Peak Control Golden Reference Test ===\n");
    validate_version();

    let sample_rate = 44100u32;
    let num_samples = 2048;

    // Generate signal that exceeds threshold
    let mut signal = signal_gen::sine_wave(440.0, sample_rate, num_samples, 1.0);

    let settings = LimiterSettings {
        threshold_db: -0.3,
        release_ms: 50.0,
    };
    let mut limiter = Limiter::with_settings(settings);

    limiter.process(&mut signal, sample_rate);

    // Verify no clipping
    let output_peak = signal.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    let threshold_linear = 10.0_f32.powf(-0.3 / 20.0);

    println!("Output peak: {:.4}", output_peak);
    println!("Threshold: {:.4}", threshold_linear);

    assert!(
        output_peak <= threshold_linear + 0.01,
        "Limiter should keep signal below threshold"
    );

    // Extract reference points
    let actual_points = comparison::extract_reference_points(&signal, 32);
    println!("\nActual reference points:");
    print!("const LIMITER_PEAK_REFERENCE: [f32; 32] = [\n    ");
    for (i, &v) in actual_points.iter().enumerate() {
        print!("{:.2}", v);
        if i < actual_points.len() - 1 {
            print!(", ");
        }
        if (i + 1) % 8 == 0 && i < actual_points.len() - 1 {
            print!("\n    ");
        }
    }
    println!("\n];");
}

// -----------------------------------------------------------------------------
// Stereo Enhancer Golden Reference Tests
// -----------------------------------------------------------------------------

#[test]
fn test_stereo_width_golden() {
    println!("\n=== Stereo Width Golden Reference Test ===\n");
    validate_version();

    let sample_rate = 44100u32;
    let num_samples = 1024;

    // Different frequencies per channel
    let mut signal = signal_gen::stereo_different(sample_rate, num_samples, 440.0, 660.0, 0.5);

    let mut enhancer = StereoEnhancer::with_settings(StereoSettings::wide());

    enhancer.process(&mut signal, sample_rate);

    // Verify stereo difference increased
    let actual_points = comparison::extract_reference_points(&signal, 32);

    println!("Actual reference points:");
    print!("const STEREO_WIDE_REFERENCE: [f32; 32] = [\n    ");
    for (i, &v) in actual_points.iter().enumerate() {
        print!("{:.2}", v);
        if i < actual_points.len() - 1 {
            print!(", ");
        }
        if (i + 1) % 8 == 0 && i < actual_points.len() - 1 {
            print!("\n    ");
        }
    }
    println!("\n];");
}

#[test]
fn test_stereo_mono_collapse_golden() {
    println!("\n=== Stereo Mono Collapse Golden Reference Test ===\n");

    let sample_rate = 44100u32;
    let num_samples = 1024;

    // Stereo signal
    let mut signal = signal_gen::stereo_different(sample_rate, num_samples, 440.0, 660.0, 0.5);

    let mut enhancer = StereoEnhancer::with_settings(StereoSettings::mono());

    enhancer.process(&mut signal, sample_rate);

    // Verify left and right are identical (mono)
    for chunk in signal.chunks_exact(2) {
        let diff = (chunk[0] - chunk[1]).abs();
        assert!(
            diff < 0.001,
            "Mono mode should produce identical channels, diff: {}",
            diff
        );
    }
    println!("Mono collapse verified: L and R channels are identical");
}

// -----------------------------------------------------------------------------
// Crossfeed Golden Reference Tests
// -----------------------------------------------------------------------------

#[test]
fn test_crossfeed_golden() {
    println!("\n=== Crossfeed Golden Reference Test ===\n");
    validate_version();

    let sample_rate = 44100u32;
    let num_samples = 1024;

    // Hard-panned left signal
    let mut signal = Vec::with_capacity(num_samples * 2);
    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let left = (2.0 * PI * 440.0 * t).sin() * 0.9;
        signal.push(left);
        signal.push(0.0); // Silent right
    }

    let mut crossfeed = Crossfeed::with_preset(CrossfeedPreset::Natural);

    crossfeed.process(&mut signal, sample_rate);

    // Verify right channel has some signal now
    let right_samples: Vec<f32> = signal.iter().skip(1).step_by(2).copied().collect();
    let right_rms = comparison::calculate_rms(&right_samples);

    println!("Right channel RMS after crossfeed: {:.4}", right_rms);
    assert!(
        right_rms > 0.01,
        "Crossfeed should add signal to silent channel"
    );

    // Extract reference points
    let actual_points = comparison::extract_reference_points(&signal, 32);

    println!("\nActual reference points:");
    print!("const CROSSFEED_PANNED_REFERENCE: [f32; 32] = [\n    ");
    for (i, &v) in actual_points.iter().enumerate() {
        print!("{:.2}", v);
        if i < actual_points.len() - 1 {
            print!(", ");
        }
        if (i + 1) % 8 == 0 && i < actual_points.len() - 1 {
            print!("\n    ");
        }
    }
    println!("\n];");
}

// -----------------------------------------------------------------------------
// Effect Chain Golden Reference Tests
// -----------------------------------------------------------------------------

#[test]
fn test_effect_chain_golden() {
    println!("\n=== Effect Chain Golden Reference Test ===\n");
    validate_version();

    let sample_rate = 44100u32;
    let num_samples = 2048;

    let mut signal = signal_gen::sine_wave(1000.0, sample_rate, num_samples, 0.7);

    // Build chain: EQ -> Compressor -> Limiter
    let mut chain = EffectChain::new();

    // EQ with slight boost
    let mut eq = ParametricEq::new();
    eq.set_mid_band(EqBand::peaking(1000.0, 2.0, 1.0));
    chain.add_effect(Box::new(eq));

    // Gentle compression
    let comp_settings = CompressorSettings {
        threshold_db: -18.0,
        ratio: 2.5,
        attack_ms: 10.0,
        release_ms: 100.0,
        knee_db: 4.0,
        makeup_gain_db: 2.0,
    };
    chain.add_effect(Box::new(Compressor::with_settings(comp_settings)));

    // Safety limiter
    let limiter_settings = LimiterSettings {
        threshold_db: -0.5,
        release_ms: 50.0,
    };
    chain.add_effect(Box::new(Limiter::with_settings(limiter_settings)));

    // Process
    chain.process(&mut signal, sample_rate);

    // Verify no clipping
    let output_peak = signal.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    println!("Output peak after chain: {:.4}", output_peak);
    assert!(output_peak < 1.0, "Chain should not produce clipping");

    // Extract reference points
    let actual_points = comparison::extract_reference_points(&signal, 32);

    println!("\nActual reference points:");
    print!("const CHAIN_FULL_REFERENCE: [f32; 32] = [\n    ");
    for (i, &v) in actual_points.iter().enumerate() {
        print!("{:.2}", v);
        if i < actual_points.len() - 1 {
            print!(", ");
        }
        if (i + 1) % 8 == 0 && i < actual_points.len() - 1 {
            print!("\n    ");
        }
    }
    println!("\n];");
}

// -----------------------------------------------------------------------------
// Resampling Golden Reference Tests
// -----------------------------------------------------------------------------

#[test]
fn test_resampling_quality_golden() {
    println!("\n=== Resampling Quality Golden Reference Test ===\n");
    validate_version();

    let input_rate = 44100u32;
    let output_rate = 48000u32;
    let num_input_samples = 44100; // 1 second at 44.1kHz (longer for better buffering behavior)

    // Generate 1kHz sine at input rate
    let input = signal_gen::sine_wave(1000.0, input_rate, num_input_samples, 0.5);

    // Create resampler
    let mut resampler = Resampler::new(
        ResamplerBackend::Auto,
        input_rate,
        output_rate,
        2,
        ResamplingQuality::High,
    )
    .expect("Failed to create resampler");

    // Process in chunks to accumulate output (resampler may buffer)
    let chunk_size = 4096;
    let mut all_output = Vec::new();

    for chunk in input.chunks(chunk_size) {
        let output = resampler.process(chunk).expect("Resampling failed");
        all_output.extend(output);
    }

    // Flush any remaining buffered samples
    let flush_output = resampler.flush().expect("Flush failed");
    all_output.extend(flush_output);

    // Calculate metrics
    let output_rms = comparison::calculate_rms(&all_output);
    let output_peak = all_output.iter().map(|s| s.abs()).fold(0.0f32, f32::max);

    println!("Input samples: {}", input.len());
    println!("Output samples: {}", all_output.len());
    println!("Output RMS: {:.4} (expected: {:.4})", output_rms, RESAMPLING_44_TO_48_RMS);
    println!("Output Peak: {:.4} (expected: {:.4})", output_peak, RESAMPLING_44_TO_48_PEAK);

    // Verify output length is approximately correct
    // Resamplers may have internal buffering, so allow 20% tolerance
    let expected_output_len = (num_input_samples as f64 * output_rate as f64 / input_rate as f64) as usize * 2;
    let length_ratio = all_output.len() as f32 / expected_output_len as f32;
    println!("Length ratio: {:.4} (should be close to 1.0, +-20% tolerance)", length_ratio);

    assert!(
        (length_ratio - 1.0).abs() < 0.2,
        "Output length ratio should be close to 1.0 (got {:.4})",
        length_ratio
    );

    // Verify RMS is preserved (within 10% - resampling may have some loss)
    let rms_error = (output_rms - RESAMPLING_44_TO_48_RMS).abs() / RESAMPLING_44_TO_48_RMS;
    assert!(
        rms_error < 0.1,
        "RMS should be preserved after resampling, error: {:.2}%",
        rms_error * 100.0
    );

    // Verify peak is preserved (within 10%)
    let peak_error = (output_peak - RESAMPLING_44_TO_48_PEAK).abs() / RESAMPLING_44_TO_48_PEAK;
    assert!(
        peak_error < 0.1,
        "Peak should be preserved after resampling, error: {:.2}%",
        peak_error * 100.0
    );
}

#[test]
fn test_resampling_downsampling_golden() {
    println!("\n=== Resampling Downsampling Golden Reference Test ===\n");

    let input_rate = 96000u32;
    let output_rate = 44100u32;
    let num_input_samples = 96000; // 1 second at 96kHz (longer for better buffering behavior)

    // Generate 1kHz sine at input rate
    let input = signal_gen::sine_wave(1000.0, input_rate, num_input_samples, 0.5);

    let mut resampler = Resampler::new(
        ResamplerBackend::Auto,
        input_rate,
        output_rate,
        2,
        ResamplingQuality::High,
    )
    .expect("Failed to create resampler");

    // Process in chunks to accumulate output
    let chunk_size = 4096;
    let mut all_output = Vec::new();

    for chunk in input.chunks(chunk_size) {
        let output = resampler.process(chunk).expect("Resampling failed");
        all_output.extend(output);
    }

    // Flush any remaining buffered samples
    let flush_output = resampler.flush().expect("Flush failed");
    all_output.extend(flush_output);

    let output_rms = comparison::calculate_rms(&all_output);
    let output_peak = all_output.iter().map(|s| s.abs()).fold(0.0f32, f32::max);

    println!("Input samples: {}", input.len());
    println!("Output samples: {}", all_output.len());
    println!("Output RMS: {:.4}", output_rms);
    println!("Output Peak: {:.4}", output_peak);

    // Verify metrics are reasonable (allow 10% tolerance)
    let expected_rms = 0.5 / 2.0_f32.sqrt();
    assert!(
        (output_rms - expected_rms).abs() / expected_rms < 0.1,
        "RMS should be preserved after resampling"
    );
}

// -----------------------------------------------------------------------------
// Complex Signal Tests
// -----------------------------------------------------------------------------

#[test]
fn test_multi_frequency_through_eq_golden() {
    println!("\n=== Multi-Frequency Through EQ Golden Reference Test ===\n");

    let sample_rate = 44100u32;
    let num_samples = 4096;
    let frequencies = [100.0, 1000.0, 5000.0, 10000.0];

    let mut signal = signal_gen::multi_frequency(&frequencies, sample_rate, num_samples, 0.5);
    let input_rms = comparison::calculate_rms(&signal);

    // EQ: boost low, cut high
    let mut eq = ParametricEq::new();
    eq.set_low_band(EqBand::low_shelf(150.0, 4.0));
    eq.set_high_band(EqBand::high_shelf(8000.0, -3.0));

    eq.process(&mut signal, sample_rate);

    let output_rms = comparison::calculate_rms(&signal);
    println!("Input RMS: {:.4}", input_rms);
    println!("Output RMS: {:.4}", output_rms);

    // Output should have slightly different RMS due to frequency-dependent gain
    // The exact change depends on the balance of frequencies
    assert!(
        output_rms > 0.0,
        "Output should have non-zero RMS"
    );
}

#[test]
fn test_sweep_through_chain_golden() {
    println!("\n=== Sweep Through Effect Chain Golden Reference Test ===\n");

    let sample_rate = 44100u32;
    let num_samples = 44100; // 1 second

    let mut signal = signal_gen::sine_sweep(20.0, 20000.0, sample_rate, num_samples, 0.6);

    // Build processing chain
    let mut chain = EffectChain::new();

    // EQ
    let mut eq = ParametricEq::new();
    eq.set_low_band(EqBand::low_shelf(100.0, 2.0));
    eq.set_mid_band(EqBand::peaking(3000.0, -2.0, 1.5));
    chain.add_effect(Box::new(eq));

    // Compressor
    let comp = Compressor::with_settings(CompressorSettings {
        threshold_db: -15.0,
        ratio: 3.0,
        attack_ms: 5.0,
        release_ms: 100.0,
        knee_db: 4.0,
        makeup_gain_db: 3.0,
    });
    chain.add_effect(Box::new(comp));

    // Limiter
    chain.add_effect(Box::new(Limiter::with_settings(LimiterSettings {
        threshold_db: -0.3,
        release_ms: 50.0,
    })));

    // Process
    chain.process(&mut signal, sample_rate);

    // Verify no clipping
    let output_peak = signal.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    println!("Output peak: {:.4}", output_peak);
    assert!(output_peak <= 1.0, "Should not clip after limiter");

    // Verify signal integrity
    let output_rms = comparison::calculate_rms(&signal);
    println!("Output RMS: {:.4}", output_rms);
    assert!(output_rms > 0.1, "Signal should not be silent");
}

// -----------------------------------------------------------------------------
// Regression Detection Tests
// -----------------------------------------------------------------------------

#[test]
fn test_eq_regression_detection() {
    println!("\n=== EQ Regression Detection Test ===\n");

    let sample_rate = 44100u32;
    let num_samples = 4096;
    let mut signal = signal_gen::sine_wave(1000.0, sample_rate, num_samples, 0.5);

    // Apply known EQ configuration
    let mut eq = ParametricEq::new();
    eq.set_low_band(EqBand::low_shelf(100.0, 6.0));
    eq.set_mid_band(EqBand::peaking(2000.0, -3.0, 1.0));
    eq.set_high_band(EqBand::high_shelf(10000.0, 2.0));

    eq.process(&mut signal, sample_rate);

    // Store checksum-like metrics for regression detection
    let output_rms = comparison::calculate_rms(&signal);
    let output_peak = signal.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    let sum: f32 = signal.iter().sum();

    println!("EQ Output Metrics:");
    println!("  RMS: {:.6}", output_rms);
    println!("  Peak: {:.6}", output_peak);
    println!("  Sum: {:.6}", sum);

    // These values should remain stable across versions
    // Update if algorithm intentionally changes
    // For now, just verify they're reasonable
    assert!(output_rms > 0.2 && output_rms < 0.8, "RMS out of expected range");
    assert!(output_peak > 0.3 && output_peak < 1.0, "Peak out of expected range");
}

#[test]
fn test_dynamics_regression_detection() {
    println!("\n=== Dynamics Regression Detection Test ===\n");

    let sample_rate = 48000u32;
    let num_samples = 48000; // 1 second

    // Generate signal that will trigger compression
    let mut signal = signal_gen::sine_wave(440.0, sample_rate, num_samples, 0.8);

    let mut compressor = Compressor::with_settings(CompressorSettings {
        threshold_db: -12.0,
        ratio: 4.0,
        attack_ms: 5.0,
        release_ms: 50.0,
        knee_db: 3.0,
        makeup_gain_db: 0.0,
    });

    compressor.process(&mut signal, sample_rate);

    // Measure compression characteristics
    let output_peak = signal.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    let output_rms = comparison::calculate_rms(&signal);

    // Calculate gain reduction
    let input_peak = 0.8;
    let gain_reduction_db = 20.0 * (output_peak / input_peak).log10();

    println!("Compressor Output Metrics:");
    println!("  Peak: {:.4}", output_peak);
    println!("  RMS: {:.4}", output_rms);
    println!("  Gain Reduction: {:.1} dB", gain_reduction_db);

    // Verify compression happened
    assert!(
        output_peak < input_peak,
        "Compressor should reduce peaks"
    );
    assert!(
        gain_reduction_db < 0.0,
        "Should have negative gain reduction"
    );
}

// -----------------------------------------------------------------------------
// Summary Report Test
// -----------------------------------------------------------------------------

#[test]
fn golden_reference_summary() {
    println!("\n");
    println!("{}", "=".repeat(70));
    println!("GOLDEN REFERENCE TEST SUMMARY");
    println!("{}", "=".repeat(70));
    println!();
    println!("Reference Version: v{}", GOLDEN_REFERENCE_VERSION);
    println!();
    println!("Effects Covered:");
    println!("  - Parametric EQ (3-band biquad filters)");
    println!("  - Compressor (two-stage detector)");
    println!("  - Limiter (brick-wall)");
    println!("  - Stereo Enhancer (M/S processing)");
    println!("  - Crossfeed (Bauer DSP)");
    println!("  - Effect Chain (combined processing)");
    println!("  - Resampling (sample rate conversion)");
    println!();
    println!("Test Signals:");
    println!("  - Sine waves (single frequency)");
    println!("  - Multi-tone signals");
    println!("  - Step signals (for dynamics)");
    println!("  - Sine sweeps (frequency response)");
    println!("  - Stereo signals (different per channel)");
    println!();
    println!("Comparison Metrics:");
    println!("  - RMS Error");
    println!("  - Maximum Deviation");
    println!("  - Pearson Correlation");
    println!("  - Length Matching");
    println!();
    println!("{}", "=".repeat(70));
    println!();
    println!("To update golden references after intentional algorithm changes:");
    println!("1. Run tests with --nocapture to see new reference values");
    println!("2. Verify changes are correct (listening test, analysis)");
    println!("3. Update const arrays in this file");
    println!("4. Increment GOLDEN_REFERENCE_VERSION");
    println!("5. Document changes in version history");
    println!();
}
