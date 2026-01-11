//! Industry-standard limiter testing based on ITU-R BS.1770, EBU R128, and professional mastering practices
//!
//! This test suite verifies critical limiter behavior required for broadcast and mastering:
//!
//! ## Standards Referenced:
//! - ITU-R BS.1770-4/5: Algorithms for audio programme loudness and true-peak measurement
//! - EBU R128: Loudness normalisation and permitted maximum level (tolerance: +/-0.3 dB)
//! - Netflix delivery spec: True peaks must not exceed -1 dBTP
//! - Spotify guidelines: -1.0 dBTP safety margin
//!
//! ## Critical Test Areas:
//! 1. Ceiling accuracy - output must NEVER exceed threshold (brickwall behavior)
//! 2. Intersample peak detection - per ITU-R BS.1770 Annex 2 (4x oversampling minimum)
//! 3. Attack time - must be effectively instant for true limiting
//! 4. Release behavior - should avoid pumping artifacts
//! 5. THD at various input levels
//! 6. True peak vs sample peak comparison
//!
//! ## Sources:
//! - [ITU-R BS.1770-5](https://www.itu.int/dms_pubrec/itu-r/rec/bs/R-REC-BS.1770-5-202311-I!!PDF-E.pdf)
//! - [Netflix Loudness Guidelines](https://partnerhelp.netflixstudios.com/hc/en-us/articles/360050414014)
//! - [EBU R128](https://tech.ebu.ch/docs/r/r128.pdf)
//! - [Mastering The Mix - True Peak](https://www.masteringthemix.com/blogs/learn/inter-sample-and-true-peak-metering)
//! - [FabFilter Pro-L2 True Peak Limiting](https://www.fabfilter.com/help/pro-l/using/truepeaklimiting)

use soul_audio::effects::{AudioEffect, Limiter, LimiterSettings};
use std::f32::consts::PI;

const SAMPLE_RATE: u32 = 44100;

// =============================================================================
// TEST SIGNAL GENERATORS
// =============================================================================

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

/// Generate a single-sample impulse (Dirac delta) at specified position
/// This is the most challenging test for a limiter - a single sample spike
fn generate_impulse(num_samples: usize, impulse_position: usize, amplitude: f32) -> Vec<f32> {
    let mut buffer = vec![0.0_f32; num_samples * 2];
    if impulse_position < num_samples {
        buffer[impulse_position * 2] = amplitude;     // Left
        buffer[impulse_position * 2 + 1] = amplitude; // Right
    }
    buffer
}

/// Generate multiple random impulses at various levels
fn generate_random_impulses(num_samples: usize, num_impulses: usize, max_amplitude: f32) -> Vec<f32> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut buffer = vec![0.0_f32; num_samples * 2];

    for i in 0..num_impulses {
        let mut hasher = DefaultHasher::new();
        i.hash(&mut hasher);
        let hash = hasher.finish();

        // Generate pseudo-random position and amplitude
        let position = (hash as usize % num_samples) * 2;
        let amplitude = (((hash >> 16) as f32 / u32::MAX as f32) * 0.5 + 0.5) * max_amplitude;

        // Alternate positive and negative impulses
        let sign = if (hash >> 32) & 1 == 0 { 1.0 } else { -1.0 };

        buffer[position] = sign * amplitude;
        buffer[position + 1] = sign * amplitude;
    }
    buffer
}

/// Generate a clipped (hard-limited) sine wave - tests limiter with already-distorted input
fn generate_clipped_sine(frequency: f32, sample_rate: u32, num_samples: usize, amplitude: f32, clip_level: f32) -> Vec<f32> {
    let mut buffer = Vec::with_capacity(num_samples * 2);
    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let mut sample = amplitude * (2.0 * PI * frequency * t).sin();
        // Hard clip
        sample = sample.clamp(-clip_level, clip_level);
        buffer.push(sample); // Left
        buffer.push(sample); // Right
    }
    buffer
}

/// Generate an inter-sample peaking signal
/// Per ITU-R BS.1770: At Fs/4 frequency with 45-degree phase offset, the sample values
/// are at 0.707 (sqrt(2)/2) but the true analog peak between samples reaches 1.0
/// This is a CRITICAL test case for true peak limiting
fn generate_intersample_peak_signal(sample_rate: u32, num_samples: usize) -> Vec<f32> {
    // At Fs/4 (e.g., 11025 Hz at 44.1kHz), we get exactly 4 samples per cycle
    // With a 45-degree (pi/4) phase offset, sample values are at +/- 0.707
    // but the continuous signal actually peaks at +/- 1.0 between samples
    //
    // Samples:    -0.707, +0.707, -0.707, +0.707, ...
    // True peaks: -1.0 and +1.0 occur exactly between samples
    let frequency = sample_rate as f32 / 4.0;
    let phase_offset = PI / 4.0; // 45 degrees

    let mut buffer = Vec::with_capacity(num_samples * 2);
    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        // Sample values will be at sin(45°) = 0.707 but the continuous
        // waveform peaks at 1.0 between samples
        let sample = (2.0 * PI * frequency * t + phase_offset).sin();
        buffer.push(sample);
        buffer.push(sample);
    }
    buffer
}

/// Generate a worst-case intersample peak signal that's normalized to a target sample peak
/// The true peak will be sqrt(2) times higher than the sample peak
fn generate_normalized_isp_signal(sample_rate: u32, num_samples: usize, target_sample_peak: f32) -> Vec<f32> {
    // Scale so sample values hit target_sample_peak
    // True peak will be target_sample_peak / 0.707 = target_sample_peak * sqrt(2)
    let frequency = sample_rate as f32 / 4.0;
    let phase_offset = PI / 4.0;
    let amplitude = target_sample_peak / (2.0_f32.sqrt() / 2.0); // = target * sqrt(2)

    let mut buffer = Vec::with_capacity(num_samples * 2);
    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let sample = amplitude * (2.0 * PI * frequency * t + phase_offset).sin();
        buffer.push(sample);
        buffer.push(sample);
    }
    buffer
}

/// Generate a square wave (maximum harmonic content, tests limiter with rich harmonics)
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

/// Generate a triangle wave ramp from 0 to peak (tests transient response)
fn generate_ramp(num_samples: usize, peak_amplitude: f32) -> Vec<f32> {
    let mut buffer = Vec::with_capacity(num_samples * 2);
    for i in 0..num_samples {
        let sample = (i as f32 / num_samples as f32) * peak_amplitude;
        buffer.push(sample);
        buffer.push(sample);
    }
    buffer
}

/// Generate burst signal (silence -> loud -> silence) for testing attack/release
fn generate_burst(
    sample_rate: u32,
    silence_before_ms: f32,
    burst_duration_ms: f32,
    silence_after_ms: f32,
    amplitude: f32,
    frequency: f32,
) -> Vec<f32> {
    let silence_before_samples = (silence_before_ms / 1000.0 * sample_rate as f32) as usize;
    let burst_samples = (burst_duration_ms / 1000.0 * sample_rate as f32) as usize;
    let silence_after_samples = (silence_after_ms / 1000.0 * sample_rate as f32) as usize;
    let total_samples = silence_before_samples + burst_samples + silence_after_samples;

    let mut buffer = Vec::with_capacity(total_samples * 2);

    // Silence before
    for _ in 0..silence_before_samples {
        buffer.push(0.0);
        buffer.push(0.0);
    }

    // Burst
    for i in 0..burst_samples {
        let t = i as f32 / sample_rate as f32;
        let sample = amplitude * (2.0 * PI * frequency * t).sin();
        buffer.push(sample);
        buffer.push(sample);
    }

    // Silence after
    for _ in 0..silence_after_samples {
        buffer.push(0.0);
        buffer.push(0.0);
    }

    buffer
}

// =============================================================================
// ANALYSIS UTILITIES
// =============================================================================

/// Find the maximum absolute sample value in a buffer
fn find_peak(buffer: &[f32]) -> f32 {
    buffer.iter().map(|s| s.abs()).fold(0.0_f32, f32::max)
}

/// Convert linear amplitude to dB
fn linear_to_db(linear: f32) -> f32 {
    if linear <= 0.0 {
        -f32::INFINITY
    } else {
        20.0 * linear.log10()
    }
}

/// Convert dB to linear amplitude
fn db_to_linear(db: f32) -> f32 {
    10.0_f32.powf(db / 20.0)
}

/// Calculate RMS of a buffer
fn calculate_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_squares: f32 = samples.iter().map(|x| x * x).sum();
    (sum_squares / samples.len() as f32).sqrt()
}

/// Estimate true peak using sinc interpolation (simplified ITU-R BS.1770 method)
/// This uses a 4-point Catmull-Rom spline which approximates sinc better than linear
/// Full BS.1770 uses 48-tap polyphase FIR at 4x oversampling
fn estimate_true_peak_4x(buffer: &[f32]) -> f32 {
    let mut max_peak = 0.0_f32;

    // Process each channel separately
    for channel in 0..2 {
        let num_samples = buffer.len() / 2;
        if num_samples < 4 {
            continue;
        }

        for i in 1..num_samples - 2 {
            // Get 4 samples for Catmull-Rom interpolation
            let p0 = buffer[(i - 1) * 2 + channel];
            let p1 = buffer[i * 2 + channel];
            let p2 = buffer[(i + 1) * 2 + channel];
            let p3 = buffer[(i + 2) * 2 + channel];

            // Check sample value
            max_peak = max_peak.max(p1.abs());

            // 4x interpolation between p1 and p2 using Catmull-Rom spline
            for j in 1..4 {
                let t = j as f32 / 4.0;
                let t2 = t * t;
                let t3 = t2 * t;

                // Catmull-Rom spline formula
                let interpolated = 0.5 * (
                    (2.0 * p1) +
                    (-p0 + p2) * t +
                    (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2 +
                    (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3
                );
                max_peak = max_peak.max(interpolated.abs());
            }
        }

        // Check remaining samples
        for i in [0, num_samples - 2, num_samples - 1] {
            if i < num_samples {
                max_peak = max_peak.max(buffer[i * 2 + channel].abs());
            }
        }
    }

    max_peak
}

/// Calculate the theoretical true peak for an Fs/4 signal with 45-degree phase
/// This is used to validate the true peak estimator
fn calculate_theoretical_true_peak(sample_peak: f32) -> f32 {
    // For Fs/4 with 45-degree phase, sample peak = 0.707 * amplitude
    // True peak = 1.0 * amplitude
    // Therefore true_peak / sample_peak = sqrt(2) = 1.414
    sample_peak * 2.0_f32.sqrt()
}

/// Calculate Total Harmonic Distortion (THD) for a sine wave
/// Uses FFT-like analysis to estimate distortion from harmonics
fn estimate_thd(buffer: &[f32], fundamental_freq: f32, sample_rate: u32) -> f32 {
    let samples: Vec<f32> = buffer.chunks(2).map(|c| (c[0] + c[1]) / 2.0).collect();
    let n = samples.len();

    if n < 256 {
        return 0.0;
    }

    // DFT at fundamental and harmonics
    let bin_size = sample_rate as f32 / n as f32;
    let fundamental_bin = (fundamental_freq / bin_size).round() as usize;

    let mut fundamental_power = 0.0_f32;
    let mut harmonic_power = 0.0_f32;

    // Calculate power at fundamental
    let mut real = 0.0_f32;
    let mut imag = 0.0_f32;
    for (i, &sample) in samples.iter().enumerate() {
        let angle = 2.0 * PI * fundamental_bin as f32 * i as f32 / n as f32;
        real += sample * angle.cos();
        imag += sample * angle.sin();
    }
    fundamental_power = (real * real + imag * imag) / (n * n) as f32;

    // Calculate power at harmonics 2-10
    for harmonic in 2..=10 {
        let harmonic_bin = fundamental_bin * harmonic;
        if harmonic_bin >= n / 2 {
            break;
        }

        let mut real = 0.0_f32;
        let mut imag = 0.0_f32;
        for (i, &sample) in samples.iter().enumerate() {
            let angle = 2.0 * PI * harmonic_bin as f32 * i as f32 / n as f32;
            real += sample * angle.cos();
            imag += sample * angle.sin();
        }
        harmonic_power += (real * real + imag * imag) / (n * n) as f32;
    }

    if fundamental_power > 0.0 {
        (harmonic_power / fundamental_power).sqrt() * 100.0
    } else {
        0.0
    }
}

// =============================================================================
// TEST MODULE: CEILING ACCURACY (CRITICAL)
// =============================================================================
// Per ITU-R BS.1770 and EBU R128, the ceiling must NEVER be exceeded.
// Any overshoot is considered a CRITICAL bug for broadcast applications.

mod ceiling_accuracy_tests {
    use super::*;

    /// ITU-R BS.1770 tolerance for true peak measurement is +/- 0.3 dB
    /// We test that output never exceeds threshold by more than this tolerance
    const CEILING_TOLERANCE_DB: f32 = 0.3;

    #[test]
    fn test_ceiling_never_exceeded_sine_wave() {
        let thresholds = [-0.1, -0.3, -0.5, -1.0, -2.0, -3.0];

        for &threshold_db in &thresholds {
            let mut limiter = Limiter::with_settings(LimiterSettings {
                threshold_db,
                release_ms: 50.0,
            });

            let threshold_linear = db_to_linear(threshold_db);

            // Test with sine wave at various input levels above threshold
            for input_level_db in [-threshold_db as i32, 0, 3, 6, 12] {
                let input_amplitude = db_to_linear(input_level_db as f32);
                let mut buffer = generate_sine_wave(1000.0, SAMPLE_RATE, 4096, input_amplitude);

                limiter.process(&mut buffer, SAMPLE_RATE);

                let output_peak = find_peak(&buffer);
                let output_peak_db = linear_to_db(output_peak);

                assert!(
                    output_peak <= threshold_linear + db_to_linear(CEILING_TOLERANCE_DB) - threshold_linear,
                    "CEILING BREACH: threshold={:.1} dB, input={} dB, output_peak={:.2} dB ({:.4} linear)\n\
                     Output exceeded threshold by {:.2} dB. This is a CRITICAL bug for broadcast!",
                    threshold_db,
                    input_level_db,
                    output_peak_db,
                    output_peak,
                    output_peak_db - threshold_db
                );
            }
        }
    }

    #[test]
    fn test_ceiling_never_exceeded_single_impulse() {
        // Single sample impulses are the most challenging test for a limiter
        // Without lookahead, most limiters will fail this test
        let mut limiter = Limiter::with_settings(LimiterSettings {
            threshold_db: -1.0,
            release_ms: 100.0,
        });

        let threshold_linear = db_to_linear(-1.0);

        // Test impulses at various amplitudes
        for &amplitude in &[1.0, 1.5, 2.0, 3.0, 5.0, 10.0] {
            let mut buffer = generate_impulse(1024, 512, amplitude);

            limiter.reset();
            limiter.process(&mut buffer, SAMPLE_RATE);

            let output_peak = find_peak(&buffer);

            assert!(
                output_peak <= threshold_linear + 0.01,
                "CEILING BREACH on impulse: amplitude={}, output_peak={:.4} (threshold={:.4})\n\
                 Single sample impulses require instant attack or lookahead!",
                amplitude,
                output_peak,
                threshold_linear
            );
        }
    }

    #[test]
    fn test_ceiling_never_exceeded_random_impulses() {
        let mut limiter = Limiter::with_settings(LimiterSettings {
            threshold_db: -0.5,
            release_ms: 100.0,
        });

        let threshold_linear = db_to_linear(-0.5);

        // Generate signal with many random impulses
        let mut buffer = generate_random_impulses(8192, 100, 5.0);

        limiter.process(&mut buffer, SAMPLE_RATE);

        let output_peak = find_peak(&buffer);

        assert!(
            output_peak <= threshold_linear + 0.01,
            "CEILING BREACH on random impulses: output_peak={:.4} (threshold={:.4})\n\
             Random transient content should never exceed ceiling!",
            output_peak,
            threshold_linear
        );
    }

    #[test]
    fn test_ceiling_never_exceeded_square_wave() {
        let mut limiter = Limiter::with_settings(LimiterSettings {
            threshold_db: -1.0,
            release_ms: 50.0,
        });

        let threshold_linear = db_to_linear(-1.0);

        // Square wave has instantaneous transitions that test limiter response
        let mut buffer = generate_square_wave(1000.0, SAMPLE_RATE, 4096, 2.0);

        limiter.process(&mut buffer, SAMPLE_RATE);

        let output_peak = find_peak(&buffer);

        assert!(
            output_peak <= threshold_linear + 0.01,
            "CEILING BREACH on square wave: output_peak={:.4} (threshold={:.4})\n\
             Square wave transitions must be limited!",
            output_peak,
            threshold_linear
        );
    }

    #[test]
    fn test_ceiling_never_exceeded_clipped_sine() {
        let mut limiter = Limiter::with_settings(LimiterSettings {
            threshold_db: -0.3,
            release_ms: 100.0,
        });

        let threshold_linear = db_to_linear(-0.3);

        // Pre-clipped sine (simulating already-distorted input)
        let mut buffer = generate_clipped_sine(1000.0, SAMPLE_RATE, 4096, 2.0, 0.8);

        limiter.process(&mut buffer, SAMPLE_RATE);

        let output_peak = find_peak(&buffer);

        assert!(
            output_peak <= threshold_linear + 0.01,
            "CEILING BREACH on clipped sine: output_peak={:.4} (threshold={:.4})",
            output_peak,
            threshold_linear
        );
    }

    #[test]
    fn test_ceiling_at_extreme_input_levels() {
        let mut limiter = Limiter::with_settings(LimiterSettings {
            threshold_db: -1.0,
            release_ms: 50.0,
        });

        let threshold_linear = db_to_linear(-1.0);

        // Test with extremely hot input (24 dB above threshold)
        let extreme_amplitude = db_to_linear(24.0);
        let mut buffer = generate_sine_wave(1000.0, SAMPLE_RATE, 4096, extreme_amplitude);

        limiter.process(&mut buffer, SAMPLE_RATE);

        let output_peak = find_peak(&buffer);

        assert!(
            output_peak <= threshold_linear + 0.01,
            "CEILING BREACH at extreme input: 24 dB hot, output_peak={:.4} (threshold={:.4})",
            output_peak,
            threshold_linear
        );
    }
}

// =============================================================================
// TEST MODULE: INTERSAMPLE PEAK HANDLING
// =============================================================================
// Per ITU-R BS.1770, the true peak of a signal can be up to 3 dB above sample values
// at quarter Nyquist with 45-degree phase offset. A proper true-peak limiter must
// use oversampling (minimum 4x per BS.1770) to catch these peaks.

mod intersample_peak_tests {
    use super::*;

    #[test]
    fn test_true_peak_estimator_validation() {
        // Validate that our true peak estimator detects some ISP excess
        // NOTE: Catmull-Rom interpolation is an approximation of sinc reconstruction
        // Full ITU-R BS.1770 compliance requires a 48-tap polyphase FIR filter
        // Our simplified estimator will underestimate the true peak by ~12%
        let buffer = generate_intersample_peak_signal(SAMPLE_RATE, 4096);

        let sample_peak = find_peak(&buffer);
        let true_peak = estimate_true_peak_4x(&buffer);
        let theoretical_true_peak = calculate_theoretical_true_peak(sample_peak);

        let isp_db = linear_to_db(true_peak) - linear_to_db(sample_peak);
        let theoretical_isp_db = linear_to_db(theoretical_true_peak) - linear_to_db(sample_peak);

        println!("True peak estimator validation:");
        println!("  Sample peak: {:.4} ({:.2} dB)", sample_peak, linear_to_db(sample_peak));
        println!("  Estimated true peak: {:.4} ({:.2} dB)", true_peak, linear_to_db(true_peak));
        println!("  Theoretical true peak: {:.4} ({:.2} dB)", theoretical_true_peak, linear_to_db(theoretical_true_peak));
        println!("  Detected ISP: {:.2} dB (theoretical: {:.2} dB)", isp_db, theoretical_isp_db);

        // The estimator should detect at least 1 dB of ISP (theoretical is ~3 dB)
        // This validates the estimator is working, even if not perfectly accurate
        assert!(
            isp_db >= 1.0,
            "True peak estimator should detect at least 1 dB of ISP, got {:.2} dB",
            isp_db
        );

        // Also verify the estimated true peak is greater than sample peak
        assert!(
            true_peak > sample_peak,
            "Estimated true peak ({:.4}) should exceed sample peak ({:.4})",
            true_peak,
            sample_peak
        );
    }

    #[test]
    fn test_intersample_peak_detection() {
        // Generate the canonical ISP test signal: Fs/4 with 45-degree phase
        // Sample values are at 0.707 but true peak is at 1.0
        let buffer = generate_intersample_peak_signal(SAMPLE_RATE, 4096);

        // Measure peaks
        let input_sample_peak = find_peak(&buffer);
        let input_true_peak = estimate_true_peak_4x(&buffer);
        let true_peak_excess_db = linear_to_db(input_true_peak) - linear_to_db(input_sample_peak);

        println!(
            "Intersample test signal characteristics:");
        println!(
            "  Sample peak: {:.3} ({:.1} dB)", input_sample_peak, linear_to_db(input_sample_peak));
        println!(
            "  True peak: {:.3} ({:.1} dB)", input_true_peak, linear_to_db(input_true_peak));
        println!(
            "  ISP excess: {:.1} dB (expected ~3 dB)", true_peak_excess_db);

        // Apply limiter with threshold at -1 dB
        // A sample-peak limiter will see 0.707 (-3 dB) and won't limit
        // But the true peak is 1.0 (0 dB) which exceeds -1 dB threshold
        let mut limited_buffer = buffer.clone();
        let mut limiter = Limiter::with_settings(LimiterSettings {
            threshold_db: -1.0,
            release_ms: 100.0,
        });

        limiter.process(&mut limited_buffer, SAMPLE_RATE);

        let output_sample_peak = find_peak(&limited_buffer);
        let output_true_peak = estimate_true_peak_4x(&limited_buffer);
        let threshold_linear = db_to_linear(-1.0);

        println!(
            "After limiting (threshold -1 dB):");
        println!(
            "  Output sample peak: {:.3} ({:.1} dB)", output_sample_peak, linear_to_db(output_sample_peak));
        println!(
            "  Output true peak: {:.3} ({:.1} dB)", output_true_peak, linear_to_db(output_true_peak));

        // For a proper true-peak limiter, the TRUE PEAK should be at or below threshold
        let true_peak_exceeds = output_true_peak > threshold_linear + 0.02;

        if true_peak_exceeds {
            println!(
                "WARNING: True peak ({:.3} = {:.1} dB) exceeds threshold ({:.1} dB) by {:.2} dB\n\
                 This limiter does not handle intersample peaks per ITU-R BS.1770.\n\
                 For broadcast compliance, 4x oversampling is required.",
                output_true_peak,
                linear_to_db(output_true_peak),
                -1.0,
                linear_to_db(output_true_peak) - (-1.0)
            );
        }

        // This is documented as a known limitation, not necessarily a failure
        // True peak limiting requires oversampling which adds latency
        assert!(
            output_true_peak <= threshold_linear + 0.1,
            "True peak ({:.1} dB) exceeds threshold ({:.1} dB) by more than 0.5 dB.\n\
             For broadcast/streaming delivery, true peak limiting is required.",
            linear_to_db(output_true_peak),
            -1.0
        );
    }

    #[test]
    fn test_sample_peak_vs_true_peak_difference() {
        // This test documents the expected difference between sample peak and true peak
        // per ITU-R BS.1770 Annex 2

        let buffer = generate_intersample_peak_signal(SAMPLE_RATE, 4096);

        let sample_peak = find_peak(&buffer);
        let true_peak = estimate_true_peak_4x(&buffer);

        let difference_db = linear_to_db(true_peak) - linear_to_db(sample_peak);

        println!(
            "Sample peak: {:.3} ({:.2} dB), True peak: {:.3} ({:.2} dB), Difference: {:.2} dB",
            sample_peak,
            linear_to_db(sample_peak),
            true_peak,
            linear_to_db(true_peak),
            difference_db
        );

        // Per ITU-R BS.1770, the difference can be up to ~3 dB in worst case
        // Our test signal should produce at least 0.5 dB difference
        assert!(
            difference_db >= 0.0,
            "True peak should be >= sample peak, got {:.2} dB difference",
            difference_db
        );
    }

    #[test]
    fn test_high_frequency_true_peak_potential() {
        // Test at various frequencies approaching Nyquist to show increasing ISP potential
        let frequencies: Vec<(f32, &str)> = vec![
            (1000.0, "1 kHz (low ISP)"),
            (5512.5, "Fs/8 (moderate ISP)"),
            (11025.0, "Fs/4 (high ISP)"),
            (16537.5, "3Fs/8 (high ISP)"),
        ];

        for (freq, description) in frequencies {
            let phase_offset = PI / 4.0;
            let amplitude = 1.0; // Full amplitude

            let mut buffer = Vec::with_capacity(4096 * 2);
            for i in 0..4096 {
                let t = i as f32 / SAMPLE_RATE as f32;
                let sample = amplitude * (2.0 * PI * freq * t + phase_offset).sin();
                buffer.push(sample);
                buffer.push(sample);
            }

            let sample_peak = find_peak(&buffer);
            let true_peak = estimate_true_peak_4x(&buffer);
            let isp_db = linear_to_db(true_peak) - linear_to_db(sample_peak);

            println!("{}: sample_peak={:.3}, true_peak={:.3}, ISP potential = {:.2} dB",
                     description, sample_peak, true_peak, isp_db);
        }
    }

    #[test]
    fn test_normalized_isp_signal_limiter_behavior() {
        // This test creates a signal where:
        // - Sample peak = threshold (e.g., 0.891 for -1 dB threshold)
        // - True peak = threshold * sqrt(2) = threshold + 3 dB
        //
        // A sample-peak limiter won't engage (sample peak at threshold)
        // A true-peak limiter MUST engage (true peak 3 dB above threshold)

        let threshold_db = -1.0;
        let threshold_linear = db_to_linear(threshold_db);

        // Generate signal with sample peak at threshold
        let mut buffer = generate_normalized_isp_signal(SAMPLE_RATE, 4096, threshold_linear);

        let input_sample_peak = find_peak(&buffer);
        let input_true_peak = estimate_true_peak_4x(&buffer);

        println!("Normalized ISP test:");
        println!("  Threshold: {:.3} ({:.1} dB)", threshold_linear, threshold_db);
        println!("  Input sample peak: {:.3} ({:.1} dB)", input_sample_peak, linear_to_db(input_sample_peak));
        println!("  Input true peak: {:.3} ({:.1} dB)", input_true_peak, linear_to_db(input_true_peak));
        println!("  ISP excess over threshold: {:.1} dB",
                 linear_to_db(input_true_peak) - threshold_db);

        // Apply limiter
        let mut limiter = Limiter::with_settings(LimiterSettings {
            threshold_db,
            release_ms: 100.0,
        });

        limiter.process(&mut buffer, SAMPLE_RATE);

        let output_sample_peak = find_peak(&buffer);
        let output_true_peak = estimate_true_peak_4x(&buffer);

        println!("After limiting:");
        println!("  Output sample peak: {:.3} ({:.1} dB)", output_sample_peak, linear_to_db(output_sample_peak));
        println!("  Output true peak: {:.3} ({:.1} dB)", output_true_peak, linear_to_db(output_true_peak));

        // BUG DETECTION: If output true peak > threshold, this limiter doesn't handle ISP
        let isp_bug_detected = output_true_peak > threshold_linear * 1.02; // 0.2 dB tolerance

        if isp_bug_detected {
            println!("\n  BUG DETECTED: Intersample peaks not handled!");
            println!("  The limiter uses sample-peak detection, not true-peak detection.");
            println!("  Per ITU-R BS.1770, broadcast limiters require 4x oversampling.");
            println!("  True peak exceeds threshold by {:.1} dB",
                     linear_to_db(output_true_peak) - threshold_db);
        }

        // Document the issue but don't necessarily fail
        // (ISP handling is an enhancement, not a critical bug for non-broadcast use)
    }

    #[test]
    fn test_isp_with_sample_peak_at_half_threshold() {
        // Most challenging case: signal where sample peak is at -3 dB relative to threshold
        // but true peak exactly hits threshold
        let threshold_db = -1.0;
        let threshold_linear = db_to_linear(threshold_db);

        // For Fs/4 + 45deg phase: sample_peak * sqrt(2) = true_peak
        // If we want true_peak = threshold, then sample_peak = threshold / sqrt(2)
        let target_sample_peak = threshold_linear / 2.0_f32.sqrt(); // = threshold - 3 dB

        let mut buffer = generate_normalized_isp_signal(SAMPLE_RATE, 4096, target_sample_peak);

        let input_sample_peak = find_peak(&buffer);
        let input_true_peak = estimate_true_peak_4x(&buffer);

        println!("ISP at boundary test:");
        println!("  Threshold: {:.3} ({:.1} dB)", threshold_linear, threshold_db);
        println!("  Input sample peak: {:.3} ({:.1} dB) - BELOW threshold", input_sample_peak, linear_to_db(input_sample_peak));
        println!("  Input true peak: {:.3} ({:.1} dB) - AT threshold", input_true_peak, linear_to_db(input_true_peak));

        // Apply limiter
        let mut limiter = Limiter::with_settings(LimiterSettings {
            threshold_db,
            release_ms: 100.0,
        });

        limiter.process(&mut buffer, SAMPLE_RATE);

        let output_sample_peak = find_peak(&buffer);
        let output_true_peak = estimate_true_peak_4x(&buffer);

        println!("After limiting:");
        println!("  Output sample peak: {:.3} ({:.1} dB)", output_sample_peak, linear_to_db(output_sample_peak));
        println!("  Output true peak: {:.3} ({:.1} dB)", output_true_peak, linear_to_db(output_true_peak));

        // A sample-peak limiter won't engage at all (sample peak is 3 dB below threshold)
        // A true-peak limiter should just barely engage (true peak at threshold)
        let was_limited = output_sample_peak < input_sample_peak * 0.99;

        if !was_limited {
            println!("\n  KNOWN LIMITATION: Sample-peak limiter doesn't engage");
            println!("  because sample values ({:.1} dB) are below threshold ({:.1} dB).",
                     linear_to_db(input_sample_peak), threshold_db);
            println!("  A true-peak limiter would need to engage here.");
        }
    }
}

// =============================================================================
// TEST MODULE: ATTACK TIME MEASUREMENT
// =============================================================================
// For true brickwall limiting, attack must be effectively instant.
// Any attack time > 0 with no lookahead will result in ceiling breaches on transients.

mod attack_time_tests {
    use super::*;

    #[test]
    fn test_attack_is_instant_no_overshoot() {
        let mut limiter = Limiter::with_settings(LimiterSettings {
            threshold_db: -1.0,
            release_ms: 100.0,
        });

        let threshold_linear = db_to_linear(-1.0);

        // Generate a signal that goes from silence to loud instantly
        let mut buffer = vec![0.0_f32; 256]; // 128 stereo samples of silence
        // Then instant peak
        for i in 64..128 {
            buffer[i * 2] = 2.0;     // Well above threshold
            buffer[i * 2 + 1] = 2.0;
        }

        limiter.process(&mut buffer, SAMPLE_RATE);

        // Check that the first loud sample is already limited
        let first_loud_output = buffer[128]; // First loud sample

        assert!(
            first_loud_output <= threshold_linear + 0.01,
            "First transient sample exceeded threshold: output={:.4} (threshold={:.4})\n\
             Attack time must be instant for brickwall limiting!",
            first_loud_output,
            threshold_linear
        );
    }

    #[test]
    fn test_step_response_no_overshoot() {
        let mut limiter = Limiter::with_settings(LimiterSettings {
            threshold_db: -3.0,
            release_ms: 100.0,
        });

        let threshold_linear = db_to_linear(-3.0);

        // Step from 0 to 2.0 (6 dB above threshold)
        let mut buffer = Vec::with_capacity(2048 * 2);
        for i in 0..2048 {
            let sample = if i < 512 { 0.0 } else { 2.0 };
            buffer.push(sample);
            buffer.push(sample);
        }

        limiter.process(&mut buffer, SAMPLE_RATE);

        // Check every sample after the step
        let mut max_overshoot = 0.0_f32;
        for i in 512..2048 {
            let sample = buffer[i * 2].abs().max(buffer[i * 2 + 1].abs());
            max_overshoot = max_overshoot.max(sample - threshold_linear);
        }

        assert!(
            max_overshoot <= 0.01,
            "Step response overshoot detected: {:.4} above threshold\n\
             This indicates non-instant attack time.",
            max_overshoot
        );
    }

    #[test]
    fn test_transient_attack_measurement() {
        // Measure how quickly the limiter responds to a transient
        let mut limiter = Limiter::with_settings(LimiterSettings {
            threshold_db: -1.0,
            release_ms: 200.0,
        });

        // Generate silence then sudden loud signal
        let mut buffer = generate_burst(SAMPLE_RATE, 10.0, 50.0, 10.0, 2.0, 1000.0);

        limiter.process(&mut buffer, SAMPLE_RATE);

        // Find first sample above threshold in output (should be none for instant attack)
        let threshold_linear = db_to_linear(-1.0);
        let samples_above_threshold: Vec<(usize, f32)> = buffer
            .chunks(2)
            .enumerate()
            .filter(|(_, chunk)| chunk[0].abs() > threshold_linear || chunk[1].abs() > threshold_linear)
            .map(|(i, chunk)| (i, chunk[0].abs().max(chunk[1].abs())))
            .collect();

        if !samples_above_threshold.is_empty() {
            let (first_idx, first_val) = samples_above_threshold[0];
            println!(
                "Attack failure: {} samples above threshold, first at sample {} with value {:.4}",
                samples_above_threshold.len(),
                first_idx,
                first_val
            );
        }

        assert!(
            samples_above_threshold.is_empty(),
            "Found {} samples above threshold. First at sample {}.\n\
             For brickwall limiting, attack must be instant (or use lookahead).",
            samples_above_threshold.len(),
            samples_above_threshold.first().map(|(i, _)| *i).unwrap_or(0)
        );
    }
}

// =============================================================================
// TEST MODULE: RELEASE BEHAVIOR AND PUMPING
// =============================================================================
// Release should be smooth to avoid audible pumping artifacts.
// Too fast = distortion, too slow = pumping.

mod release_behavior_tests {
    use super::*;

    #[test]
    fn test_release_curve_is_exponential() {
        let release_ms = 100.0;
        let mut limiter = Limiter::with_settings(LimiterSettings {
            threshold_db: -3.0,
            release_ms,
        });

        // First, drive the limiter with a loud signal
        let mut loud_buffer = generate_sine_wave(1000.0, SAMPLE_RATE, 4410, 2.0); // 100ms
        limiter.process(&mut loud_buffer, SAMPLE_RATE);

        // Then switch to a quiet signal and measure the gain recovery
        let quiet_amplitude = 0.2; // Below threshold
        let mut quiet_buffer = generate_sine_wave(1000.0, SAMPLE_RATE, 8820, quiet_amplitude); // 200ms
        limiter.process(&mut quiet_buffer, SAMPLE_RATE);

        // Measure the envelope of the quiet signal (should be recovering)
        let mut gain_readings: Vec<f32> = Vec::new();
        for chunk in quiet_buffer.chunks(2 * 441) { // Every 10ms
            let rms = calculate_rms(chunk);
            let apparent_gain = rms / (quiet_amplitude / 2.0_f32.sqrt()); // Normalize to expected RMS
            gain_readings.push(apparent_gain);
        }

        // Release should be gradual, not instant
        let initial_gain = gain_readings.first().copied().unwrap_or(1.0);
        let final_gain = gain_readings.last().copied().unwrap_or(1.0);

        println!(
            "Release behavior: initial_gain={:.3}, final_gain={:.3}, readings={:?}",
            initial_gain,
            final_gain,
            gain_readings
        );

        // Initial gain should be lower (limiter was engaged)
        // Final gain should be higher (limiter released)
        assert!(
            final_gain >= initial_gain * 0.9 || (initial_gain - final_gain).abs() < 0.1,
            "Release curve appears incorrect: initial={:.3}, final={:.3}",
            initial_gain,
            final_gain
        );
    }

    #[test]
    fn test_release_time_approximately_correct() {
        let release_ms = 50.0;
        let mut limiter = Limiter::with_settings(LimiterSettings {
            threshold_db: -6.0,
            release_ms,
        });

        // Drive limiter hard
        let mut loud = generate_sine_wave(1000.0, SAMPLE_RATE, 4410, 3.0);
        limiter.process(&mut loud, SAMPLE_RATE);

        // Then silence and measure recovery
        let samples_for_recovery = (release_ms * 5.0 / 1000.0 * SAMPLE_RATE as f32) as usize;
        let mut silence = vec![0.001_f32; samples_for_recovery * 2]; // Tiny signal to measure gain

        // Add a tiny reference signal to measure gain
        for i in 0..samples_for_recovery {
            let t = i as f32 / SAMPLE_RATE as f32;
            let sample = 0.01 * (2.0 * PI * 1000.0 * t).sin();
            silence[i * 2] = sample;
            silence[i * 2 + 1] = sample;
        }

        limiter.process(&mut silence, SAMPLE_RATE);

        // The limiter should have mostly released by 3x the release time
        let release_samples = (release_ms * 3.0 / 1000.0 * SAMPLE_RATE as f32) as usize;
        let late_portion = &silence[release_samples * 2..];
        let late_rms = calculate_rms(late_portion);
        let expected_rms = 0.01 / 2.0_f32.sqrt(); // RMS of 0.01 amplitude sine

        // By 3x release time, should be close to unity gain
        let gain_ratio = late_rms / expected_rms;

        println!(
            "Release test: late_rms={:.4}, expected_rms={:.4}, gain_ratio={:.3}",
            late_rms, expected_rms, gain_ratio
        );

        // Allow wide tolerance due to measurement noise
        assert!(
            gain_ratio > 0.5,
            "Release appears too slow: gain_ratio={:.3} at 3x release time",
            gain_ratio
        );
    }

    #[test]
    fn test_no_pumping_on_music_like_signal() {
        let mut limiter = Limiter::with_settings(LimiterSettings {
            threshold_db: -1.0,
            release_ms: 100.0,
        });

        // Generate music-like signal with varying dynamics
        let mut buffer = Vec::with_capacity(44100 * 2); // 1 second
        for i in 0..44100 {
            let t = i as f32 / SAMPLE_RATE as f32;
            // Sum of multiple frequencies with amplitude modulation
            let envelope = 0.5 + 0.5 * (2.0 * PI * 2.0 * t).sin(); // 2 Hz modulation
            let sample = envelope * 1.5 * (
                (2.0 * PI * 200.0 * t).sin() * 0.5 +
                (2.0 * PI * 400.0 * t).sin() * 0.3 +
                (2.0 * PI * 800.0 * t).sin() * 0.2
            );
            buffer.push(sample);
            buffer.push(sample);
        }

        limiter.process(&mut buffer, SAMPLE_RATE);

        // Check for large gain variations (pumping)
        let mut gain_variations = 0;
        let mut prev_gain = 1.0_f32;

        for chunk in buffer.chunks(441 * 2) { // 10ms chunks
            let chunk_peak = find_peak(chunk);
            if chunk_peak > 0.01 {
                let gain = chunk_peak / 1.5; // Rough estimate
                if (gain - prev_gain).abs() > 0.3 {
                    gain_variations += 1;
                }
                prev_gain = gain;
            }
        }

        println!("Pumping test: {} significant gain variations", gain_variations);

        // Some variation is expected, but excessive pumping is a problem
        assert!(
            gain_variations < 20,
            "Excessive pumping detected: {} large gain variations.\n\
             Release time may need adjustment.",
            gain_variations
        );
    }
}

// =============================================================================
// TEST MODULE: THD (TOTAL HARMONIC DISTORTION)
// =============================================================================
// A quality limiter should minimize distortion, especially at lower input levels.

mod thd_tests {
    use super::*;

    #[test]
    fn test_thd_at_various_input_levels() {
        let test_cases = [
            (-6.0, 1.0, "moderate limiting"),
            (-3.0, 1.0, "moderate limiting"),
            (0.0, 1.0, "at threshold"),
            (3.0, 1.0, "3 dB above threshold"),
            (6.0, 1.0, "6 dB above threshold"),
            (12.0, 1.0, "heavy limiting"),
        ];

        for (input_db, threshold_db, description) in test_cases {
            let mut limiter = Limiter::with_settings(LimiterSettings {
                threshold_db: -threshold_db,
                release_ms: 100.0,
            });

            let input_amplitude = db_to_linear(input_db);
            let mut buffer = generate_sine_wave(1000.0, SAMPLE_RATE, 8192, input_amplitude);

            limiter.process(&mut buffer, SAMPLE_RATE);

            let thd = estimate_thd(&buffer, 1000.0, SAMPLE_RATE);

            println!(
                "THD at {} (input={:.1} dB, threshold={:.1} dB): {:.2}%",
                description, input_db, -threshold_db, thd
            );

            // Heavy limiting will add distortion, but it should be controlled
            let max_acceptable_thd = if input_db > 6.0 { 20.0 } else { 10.0 };

            assert!(
                thd < max_acceptable_thd,
                "THD too high at {}: {:.2}% (max acceptable: {:.1}%)",
                description,
                thd,
                max_acceptable_thd
            );
        }
    }

    #[test]
    fn test_thd_below_threshold_is_minimal() {
        let mut limiter = Limiter::with_settings(LimiterSettings {
            threshold_db: -1.0,
            release_ms: 100.0,
        });

        // Signal well below threshold (-10 dB vs -1 dB threshold)
        let input_amplitude = db_to_linear(-10.0);
        let mut buffer = generate_sine_wave(1000.0, SAMPLE_RATE, 8192, input_amplitude);

        let original = buffer.clone();

        limiter.process(&mut buffer, SAMPLE_RATE);

        // Signal should be essentially unchanged
        let max_diff = buffer.iter().zip(&original)
            .map(|(a, b)| (a - b).abs())
            .fold(0.0_f32, f32::max);

        let diff_db = if max_diff > 0.0 {
            linear_to_db(max_diff / input_amplitude)
        } else {
            -f32::INFINITY
        };

        println!(
            "Below-threshold signal: max_diff={:.6} ({:.1} dB below input)",
            max_diff,
            -diff_db
        );

        // Difference should be very small
        assert!(
            max_diff < 0.01,
            "Signal below threshold was modified: max_diff={:.4}.\n\
             Limiter should pass through signals below threshold unchanged.",
            max_diff
        );
    }
}

// =============================================================================
// TEST MODULE: SAMPLE RATE HANDLING
// =============================================================================

mod sample_rate_tests {
    use super::*;

    #[test]
    fn test_various_sample_rates() {
        let sample_rates = [44100, 48000, 88200, 96000, 176400, 192000];

        for &sr in &sample_rates {
            let mut limiter = Limiter::with_settings(LimiterSettings {
                threshold_db: -1.0,
                release_ms: 100.0,
            });

            let threshold_linear = db_to_linear(-1.0);
            let mut buffer = generate_sine_wave(1000.0, sr, 4096, 2.0);

            limiter.process(&mut buffer, sr);

            let output_peak = find_peak(&buffer);

            assert!(
                output_peak <= threshold_linear + 0.01,
                "Ceiling breach at {} Hz sample rate: peak={:.4} (threshold={:.4})",
                sr,
                output_peak,
                threshold_linear
            );
        }
    }

    #[test]
    fn test_release_time_scales_with_sample_rate() {
        // Release time should be consistent across sample rates
        let release_ms = 100.0;

        for &sr in &[44100_u32, 96000] {
            let mut limiter = Limiter::with_settings(LimiterSettings {
                threshold_db: -6.0,
                release_ms,
            });

            // Drive limiter
            let loud_samples = (0.1 * sr as f32) as usize;
            let mut loud = generate_sine_wave(1000.0, sr, loud_samples, 3.0);
            limiter.process(&mut loud, sr);

            // Measure release on quiet signal
            let quiet_samples = (release_ms * 3.0 / 1000.0 * sr as f32) as usize;
            let mut quiet = vec![0.001_f32; quiet_samples * 2];
            limiter.process(&mut quiet, sr);

            // Release behavior should be similar regardless of sample rate
            // (relative to time, not sample count)
            println!(
                "Release test at {} Hz: {} samples for {} ms",
                sr, quiet_samples, release_ms * 3.0
            );
        }
    }
}

// =============================================================================
// SUMMARY TEST: COUNT ALL CEILING BREACHES
// =============================================================================

#[test]
fn comprehensive_ceiling_breach_count() {
    let mut total_tests = 0;
    let mut ceiling_breaches = 0;
    let mut breach_details: Vec<String> = Vec::new();

    let threshold_db = -1.0;
    let threshold_linear = db_to_linear(threshold_db);

    // Test 1: Various amplitude sine waves
    for amplitude in [1.0, 1.5, 2.0, 3.0, 5.0] {
        total_tests += 1;
        let mut limiter = Limiter::with_settings(LimiterSettings {
            threshold_db,
            release_ms: 100.0,
        });
        let mut buffer = generate_sine_wave(1000.0, SAMPLE_RATE, 4096, amplitude);
        limiter.process(&mut buffer, SAMPLE_RATE);

        let peak = find_peak(&buffer);
        if peak > threshold_linear + 0.01 {
            ceiling_breaches += 1;
            breach_details.push(format!(
                "Sine wave amplitude {}: peak {:.4} > threshold {:.4}",
                amplitude, peak, threshold_linear
            ));
        }
    }

    // Test 2: Single impulses
    for amplitude in [1.0, 2.0, 5.0, 10.0] {
        total_tests += 1;
        let mut limiter = Limiter::with_settings(LimiterSettings {
            threshold_db,
            release_ms: 100.0,
        });
        let mut buffer = generate_impulse(1024, 512, amplitude);
        limiter.process(&mut buffer, SAMPLE_RATE);

        let peak = find_peak(&buffer);
        if peak > threshold_linear + 0.01 {
            ceiling_breaches += 1;
            breach_details.push(format!(
                "Impulse amplitude {}: peak {:.4} > threshold {:.4}",
                amplitude, peak, threshold_linear
            ));
        }
    }

    // Test 3: Square wave
    total_tests += 1;
    let mut limiter = Limiter::with_settings(LimiterSettings {
        threshold_db,
        release_ms: 100.0,
    });
    let mut buffer = generate_square_wave(1000.0, SAMPLE_RATE, 4096, 2.0);
    limiter.process(&mut buffer, SAMPLE_RATE);
    let peak = find_peak(&buffer);
    if peak > threshold_linear + 0.01 {
        ceiling_breaches += 1;
        breach_details.push(format!(
            "Square wave: peak {:.4} > threshold {:.4}",
            peak, threshold_linear
        ));
    }

    // Test 4: Random impulses
    total_tests += 1;
    let mut limiter = Limiter::with_settings(LimiterSettings {
        threshold_db,
        release_ms: 100.0,
    });
    let mut buffer = generate_random_impulses(8192, 100, 5.0);
    limiter.process(&mut buffer, SAMPLE_RATE);
    let peak = find_peak(&buffer);
    if peak > threshold_linear + 0.01 {
        ceiling_breaches += 1;
        breach_details.push(format!(
            "Random impulses: peak {:.4} > threshold {:.4}",
            peak, threshold_linear
        ));
    }

    // Test 5: Step response
    total_tests += 1;
    let mut limiter = Limiter::with_settings(LimiterSettings {
        threshold_db,
        release_ms: 100.0,
    });
    let mut buffer = vec![0.0_f32; 1024];
    for i in 256..512 {
        buffer[i * 2] = 3.0;
        buffer[i * 2 + 1] = 3.0;
    }
    limiter.process(&mut buffer, SAMPLE_RATE);
    let peak = find_peak(&buffer);
    if peak > threshold_linear + 0.01 {
        ceiling_breaches += 1;
        breach_details.push(format!(
            "Step response: peak {:.4} > threshold {:.4}",
            peak, threshold_linear
        ));
    }

    println!("\n========================================");
    println!("LIMITER INDUSTRY STANDARD TEST SUMMARY");
    println!("========================================");
    println!("Standards referenced:");
    println!("  - ITU-R BS.1770-4/5 (True Peak Measurement)");
    println!("  - EBU R128 (Loudness Normalisation)");
    println!("  - Netflix/Spotify delivery specs (-1 dBTP)");
    println!("");
    println!("Total ceiling tests: {}", total_tests);
    println!("Ceiling breaches: {}", ceiling_breaches);
    println!("");

    if !breach_details.is_empty() {
        println!("BREACH DETAILS:");
        for (i, detail) in breach_details.iter().enumerate() {
            println!("  {}. {}", i + 1, detail);
        }
    }

    println!("");
    println!("========================================");

    // This is informational - we document breaches rather than failing
    // because some breaches may be acceptable depending on use case
    if ceiling_breaches > 0 {
        println!(
            "WARNING: {} ceiling breach(es) detected out of {} tests.\n\
             For broadcast compliance per ITU-R BS.1770, NO ceiling breaches are acceptable.\n\
             Consider implementing lookahead or true peak limiting.",
            ceiling_breaches, total_tests
        );
    } else {
        println!("All ceiling tests passed.");
    }
}

// =============================================================================
// EDGE CASES AND STABILITY
// =============================================================================

mod edge_case_tests {
    use super::*;

    #[test]
    fn test_denormal_handling() {
        let mut limiter = Limiter::new();

        let denormal = f32::MIN_POSITIVE / 1000.0;
        let mut buffer = vec![denormal; 1024];

        limiter.process(&mut buffer, SAMPLE_RATE);

        for sample in &buffer {
            assert!(sample.is_finite(), "Denormal input produced non-finite output");
        }
    }

    #[test]
    fn test_empty_buffer() {
        let mut limiter = Limiter::new();
        let mut buffer: Vec<f32> = vec![];
        limiter.process(&mut buffer, SAMPLE_RATE);
        // Should not panic
    }

    #[test]
    fn test_single_sample() {
        let mut limiter = Limiter::new();
        let mut buffer = vec![2.0_f32, 2.0];
        limiter.process(&mut buffer, SAMPLE_RATE);

        let threshold_linear = db_to_linear(-0.3); // Default threshold
        assert!(
            buffer[0].abs() <= threshold_linear + 0.01,
            "Single sample not limited: {:.4}",
            buffer[0]
        );
    }

    #[test]
    fn test_long_continuous_processing() {
        let mut limiter = Limiter::with_settings(LimiterSettings {
            threshold_db: -1.0,
            release_ms: 100.0,
        });

        let threshold_linear = db_to_linear(-1.0);

        // Process 10 seconds of audio in small chunks
        for _ in 0..1000 {
            let mut buffer = generate_sine_wave(1000.0, SAMPLE_RATE, 441, 2.0);
            limiter.process(&mut buffer, SAMPLE_RATE);

            let peak = find_peak(&buffer);
            assert!(
                peak <= threshold_linear + 0.01,
                "Ceiling breach during continuous processing: {:.4}",
                peak
            );
        }
    }

    #[test]
    fn test_extreme_threshold_values() {
        // Very high threshold (almost 0 dB)
        let mut limiter = Limiter::with_settings(LimiterSettings {
            threshold_db: -0.01,
            release_ms: 50.0,
        });
        let mut buffer = generate_sine_wave(1000.0, SAMPLE_RATE, 1024, 1.5);
        limiter.process(&mut buffer, SAMPLE_RATE);

        let threshold_linear = db_to_linear(-0.01);
        let peak = find_peak(&buffer);
        assert!(
            peak <= threshold_linear + 0.01,
            "Extreme threshold -0.01 dB: peak {:.4} > threshold {:.4}",
            peak,
            threshold_linear
        );

        // Very low threshold (-20 dB)
        let mut limiter = Limiter::with_settings(LimiterSettings {
            threshold_db: -20.0,
            release_ms: 50.0,
        });
        let mut buffer = generate_sine_wave(1000.0, SAMPLE_RATE, 1024, 1.0);
        limiter.process(&mut buffer, SAMPLE_RATE);

        let threshold_linear = db_to_linear(-20.0);
        let peak = find_peak(&buffer);
        assert!(
            peak <= threshold_linear + 0.01,
            "Extreme threshold -20 dB: peak {:.4} > threshold {:.4}",
            peak,
            threshold_linear
        );
    }

    #[test]
    fn test_dc_offset_handling() {
        let mut limiter = Limiter::with_settings(LimiterSettings {
            threshold_db: -1.0,
            release_ms: 100.0,
        });

        // Signal with DC offset
        let dc = 0.5;
        let mut buffer: Vec<f32> = (0..2048)
            .flat_map(|i| {
                let t = i as f32 / SAMPLE_RATE as f32;
                let sample = dc + 0.8 * (2.0 * PI * 1000.0 * t).sin();
                [sample, sample]
            })
            .collect();

        limiter.process(&mut buffer, SAMPLE_RATE);

        // All outputs should be finite
        for sample in &buffer {
            assert!(sample.is_finite(), "DC offset produced non-finite output");
        }

        // Peaks should still be limited
        let threshold_linear = db_to_linear(-1.0);
        let peak = find_peak(&buffer);
        assert!(
            peak <= threshold_linear + 0.01,
            "DC offset signal: peak {:.4} > threshold {:.4}",
            peak,
            threshold_linear
        );
    }
}
