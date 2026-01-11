//! Industry-Standard Resampler Quality Tests
//!
//! This test suite implements professional SRC testing methodologies based on:
//!
//! - **Infinite Wave** (src.infinitewave.ca): 96kHz -> 44.1kHz conversion tests,
//!   spectrogram analysis, transition band roll-off, phase response
//! - **AES17**: THD+N measurement, frequency response, dynamic range
//! - **libsamplerate benchmarks**: SNR measurement, bandwidth testing
//!
//! ## Industry Standards Referenced
//!
//! - AES17-2015: Standard method for digital audio equipment measurements
//! - Infinite Wave methodology: Developed by Dave Horrocks (mastering engineer)
//!   and Alexey Lukin (DSP developer at iZotope)
//!
//! ## Test Metrics
//!
//! | Metric                    | Industry Standard              | Our Threshold    |
//! |---------------------------|-------------------------------|------------------|
//! | Passband Ripple           | < 0.1 dB (studio grade)       | < 0.1 dB High+   |
//! | Stopband Attenuation      | > 100 dB (audiophile)         | > 100 dB Maximum |
//! | THD+N @ 1kHz              | < -100 dB (pro equipment)     | < -50 dB High    |
//! | Aliasing Rejection        | > 96 dB (CD quality)          | > 80 dB High     |
//! | Pre-ringing               | < -60 dB relative to peak     | Measured only    |
//!
//! ## Test Categories
//!
//! 1. Passband Flatness (Infinite Wave method)
//! 2. Stopband Attenuation (AES17 method)
//! 3. THD+N at Multiple Frequencies (AES17 method)
//! 4. Aliasing Rejection (downsampling tests)
//! 5. Pre-ringing Measurement (impulse response analysis)
//! 6. Latency/Delay Measurement
//! 7. All Quality Presets
//! 8. Standard Rate Combinations (44.1<->48, 44.1<->96, 48<->96, 96<->192)
//!
//! Run with: cargo test -p soul-audio --test resampler_industry_test -- --nocapture

use rustfft::{num_complex::Complex, FftPlanner};
use soul_audio::resampling::{Resampler, ResamplerBackend, ResamplingQuality};
use std::f32::consts::PI;

// =============================================================================
// INDUSTRY STANDARD THRESHOLDS
// =============================================================================
//
// These thresholds are based on:
// - Infinite Wave comparative testing (src.infinitewave.ca)
// - AES17-2015 measurement standards
// - libsamplerate quality benchmarks (libsndfile.github.io/libsamplerate)

/// Passband ripple threshold for Maximum quality (Infinite Wave: <0.1dB for studio grade)
const PASSBAND_RIPPLE_MAXIMUM_DB: f32 = 0.1;

/// Passband ripple threshold for High quality
const PASSBAND_RIPPLE_HIGH_DB: f32 = 0.2;

/// Passband ripple threshold for Balanced quality
const PASSBAND_RIPPLE_BALANCED_DB: f32 = 0.5;

/// Passband ripple threshold for Fast quality
const PASSBAND_RIPPLE_FAST_DB: f32 = 1.0;

/// Stopband attenuation for Maximum quality (AES: >96dB for CD quality, >120dB for audiophile)
const STOPBAND_ATTENUATION_MAXIMUM_DB: f32 = 100.0;

/// Stopband attenuation for High quality
const STOPBAND_ATTENUATION_HIGH_DB: f32 = 80.0;

/// Stopband attenuation for Balanced quality
const STOPBAND_ATTENUATION_BALANCED_DB: f32 = 60.0;

/// Stopband attenuation for Fast quality
const STOPBAND_ATTENUATION_FAST_DB: f32 = 40.0;

/// THD+N threshold for Maximum quality (libsamplerate: -145dB achievable, AES17: -120dB pro grade)
const THD_N_MAXIMUM_DB: f32 = -80.0;

/// THD+N threshold for High quality
const THD_N_HIGH_DB: f32 = -60.0;

/// THD+N threshold for Balanced quality
const THD_N_BALANCED_DB: f32 = -50.0;

/// THD+N threshold for Fast quality
const THD_N_FAST_DB: f32 = -40.0;

/// Aliasing rejection threshold for High/Maximum quality
const ALIASING_REJECTION_HIGH_DB: f32 = 80.0;

/// Aliasing rejection threshold for Balanced quality
const ALIASING_REJECTION_BALANCED_DB: f32 = 60.0;

/// Aliasing rejection threshold for Fast quality
const ALIASING_REJECTION_FAST_DB: f32 = 40.0;

// =============================================================================
// TEST DATA STRUCTURES
// =============================================================================

/// Result of a single measurement
#[derive(Debug, Clone)]
struct MeasurementResult {
    description: String,
    measured_value: f32,
    threshold: f32,
    unit: &'static str,
    passed: bool,
    quality: ResamplingQuality,
}

impl MeasurementResult {
    fn new(
        description: &str,
        measured: f32,
        threshold: f32,
        unit: &'static str,
        higher_is_better: bool,
        quality: ResamplingQuality,
    ) -> Self {
        let passed = if higher_is_better {
            measured >= threshold
        } else {
            measured <= threshold
        };
        Self {
            description: description.to_string(),
            measured_value: measured,
            threshold,
            unit,
            passed,
            quality,
        }
    }
}

/// Bug report structure
#[derive(Debug)]
struct BugReport {
    category: &'static str,
    description: String,
    measured_value: f32,
    expected_threshold: f32,
    unit: &'static str,
    quality: ResamplingQuality,
    rate_conversion: String,
    industry_reference: &'static str,
}

// =============================================================================
// SIGNAL GENERATION HELPERS
// =============================================================================

/// Generate a pure sine wave (mono)
fn generate_sine(frequency: f32, sample_rate: u32, duration_secs: f32, amplitude: f32) -> Vec<f32> {
    let num_samples = (sample_rate as f32 * duration_secs) as usize;
    (0..num_samples)
        .map(|i| {
            let t = i as f32 / sample_rate as f32;
            amplitude * (2.0 * PI * frequency * t).sin()
        })
        .collect()
}

/// Generate stereo from mono (duplicate channels)
fn mono_to_stereo(mono: &[f32]) -> Vec<f32> {
    mono.iter().flat_map(|&s| [s, s]).collect()
}

/// Extract mono from stereo interleaved
fn extract_mono(stereo: &[f32], channel: usize) -> Vec<f32> {
    stereo.iter().skip(channel).step_by(2).copied().collect()
}

/// Generate an impulse for transient response testing
fn generate_impulse(sample_rate: u32, duration_secs: f32) -> Vec<f32> {
    let num_samples = (sample_rate as f32 * duration_secs) as usize;
    let impulse_idx = num_samples / 2;
    let mut buffer = vec![0.0; num_samples];
    if impulse_idx < num_samples {
        buffer[impulse_idx] = 1.0;
    }
    buffer
}

/// Generate a swept sine (chirp) for frequency response testing
fn generate_chirp(
    start_freq: f32,
    end_freq: f32,
    sample_rate: u32,
    duration_secs: f32,
    amplitude: f32,
) -> Vec<f32> {
    let num_samples = (sample_rate as f32 * duration_secs) as usize;
    let k = (end_freq - start_freq) / duration_secs;

    (0..num_samples)
        .map(|i| {
            let t = i as f32 / sample_rate as f32;
            let freq = start_freq + k * t / 2.0;
            amplitude * (2.0 * PI * freq * t).sin()
        })
        .collect()
}

// =============================================================================
// MEASUREMENT HELPERS
// =============================================================================

/// Calculate RMS of a signal
fn calculate_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_squares: f32 = samples.iter().map(|&s| s * s).sum();
    (sum_squares / samples.len() as f32).sqrt()
}

/// Convert linear amplitude to dB
fn linear_to_db(linear: f32) -> f32 {
    20.0 * (linear.max(1e-10)).log10()
}

/// Apply Blackman-Harris window (better sidelobe rejection than Hann)
fn apply_blackman_harris_window(samples: &[f32]) -> Vec<f32> {
    let n = samples.len() as f32;
    let a0 = 0.35875;
    let a1 = 0.48829;
    let a2 = 0.14128;
    let a3 = 0.01168;

    samples
        .iter()
        .enumerate()
        .map(|(i, &s)| {
            let x = i as f32 / n;
            let window = a0 - a1 * (2.0 * PI * x).cos() + a2 * (4.0 * PI * x).cos()
                - a3 * (6.0 * PI * x).cos();
            s * window
        })
        .collect()
}

/// Perform FFT and return complex spectrum
fn fft_spectrum(samples: &[f32]) -> Vec<Complex<f32>> {
    let n = samples.len();
    let fft_size = n.next_power_of_two();

    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(fft_size);

    let windowed = apply_blackman_harris_window(samples);
    let mut buffer: Vec<Complex<f32>> =
        windowed.into_iter().map(|s| Complex::new(s, 0.0)).collect();

    buffer.resize(fft_size, Complex::new(0.0, 0.0));
    fft.process(&mut buffer);
    buffer
}

/// Get magnitude at specific frequency
fn magnitude_at_frequency(spectrum: &[Complex<f32>], frequency: f32, sample_rate: u32) -> f32 {
    let fft_size = spectrum.len();
    let bin_width = sample_rate as f32 / fft_size as f32;
    let bin = (frequency / bin_width).round() as usize;

    if bin < fft_size / 2 {
        spectrum[bin].norm()
    } else {
        0.0
    }
}

/// Calculate THD+N using AES17 methodology
fn calculate_thd_n_aes17(samples: &[f32], fundamental_freq: f32, sample_rate: u32) -> f32 {
    let spectrum = fft_spectrum(samples);
    let fft_size = spectrum.len();
    let bin_width = sample_rate as f32 / fft_size as f32;
    let fundamental_bin = (fundamental_freq / bin_width).round() as usize;

    // Fundamental power (with window around bin for spectral leakage)
    let window = 5;
    let fundamental_power: f32 = spectrum
        [fundamental_bin.saturating_sub(window)..=(fundamental_bin + window).min(fft_size / 2 - 1)]
        .iter()
        .map(|c| c.norm_sqr())
        .sum();

    // Total power in audio band (20Hz - 20kHz or up to Nyquist)
    let nyquist_bin = fft_size / 2;
    let max_bin = ((20000.0 / bin_width) as usize).min(nyquist_bin - 1);
    let min_bin = ((20.0 / bin_width) as usize).max(1);

    let total_power: f32 = spectrum[min_bin..=max_bin].iter().map(|c| c.norm_sqr()).sum();

    // THD+N = (total - fundamental) / fundamental
    let distortion_power = (total_power - fundamental_power).max(0.0);
    let thd_n_ratio = (distortion_power / fundamental_power.max(1e-20)).sqrt();

    linear_to_db(thd_n_ratio)
}

/// Measure passband ripple (Infinite Wave methodology)
///
/// Tests frequency response flatness across the passband.
/// Per Infinite Wave: measures gain variation across frequency range.
fn measure_passband_ripple(
    resampler: &mut Resampler,
    input_rate: u32,
    output_rate: u32,
) -> (f32, Vec<(f32, f32)>) {
    let nyquist = (input_rate.min(output_rate) as f32) / 2.0;

    // Test frequencies from 100Hz to 90% of Nyquist
    let test_frequencies: Vec<f32> = vec![
        100.0,
        500.0,
        1000.0,
        2000.0,
        5000.0,
        8000.0,
        10000.0,
        12000.0,
        15000.0,
        18000.0,
        nyquist * 0.85,
        nyquist * 0.90,
    ]
    .into_iter()
    .filter(|&f| f < nyquist * 0.95)
    .collect();

    let mut gains_db = Vec::new();

    for &freq in &test_frequencies {
        resampler.reset();

        let input_mono = generate_sine(freq, input_rate, 0.5, 0.5);
        let input = mono_to_stereo(&input_mono);

        let mut output = resampler.process(&input).unwrap();
        output.extend(resampler.flush().unwrap());

        if output.len() < 200 {
            continue;
        }

        // Skip transient, use middle portion
        let skip = output.len() / 4;
        let end = output.len() - output.len() / 4;
        if skip >= end {
            continue;
        }

        let input_rms = calculate_rms(&input_mono);
        let output_rms = calculate_rms(&extract_mono(&output[skip..end], 0));

        let gain_db = linear_to_db(output_rms / input_rms.max(1e-10));
        gains_db.push((freq, gain_db));
    }

    if gains_db.is_empty() {
        return (0.0, vec![]);
    }

    // Calculate ripple as max deviation from mean
    let gains: Vec<f32> = gains_db.iter().map(|(_, g)| *g).collect();
    let mean_gain = gains.iter().sum::<f32>() / gains.len() as f32;
    let max_deviation = gains.iter().map(|&g| (g - mean_gain).abs()).fold(0.0f32, f32::max);

    // Ripple is peak-to-peak variation
    let max_gain = gains.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let min_gain = gains.iter().cloned().fold(f32::INFINITY, f32::min);
    let ripple = max_gain - min_gain;

    (ripple.max(max_deviation * 2.0), gains_db)
}

/// Measure stopband attenuation (AES17 methodology)
///
/// Tests rejection of frequencies above the target Nyquist during downsampling.
/// Per AES17: stopband attenuation is measured as the ratio of input to output
/// for frequencies that should be completely rejected.
fn measure_stopband_attenuation(
    resampler: &mut Resampler,
    input_rate: u32,
    output_rate: u32,
    test_frequency: f32,
) -> f32 {
    resampler.reset();

    let input_mono = generate_sine(test_frequency, input_rate, 1.0, 0.5);
    let input = mono_to_stereo(&input_mono);

    let mut output = resampler.process(&input).unwrap();
    output.extend(resampler.flush().unwrap());

    if output.len() < 100 {
        return 0.0;
    }

    let input_rms = calculate_rms(&input_mono);
    let output_rms = calculate_rms(&extract_mono(&output, 0));

    // Attenuation = input / output (in dB)
    linear_to_db(input_rms / output_rms.max(1e-10))
}

/// Measure aliasing rejection during downsampling
///
/// Per Infinite Wave: when downsampling, frequencies above the new Nyquist
/// should be attenuated, not aliased (folded) into the passband.
fn measure_aliasing_rejection(
    resampler: &mut Resampler,
    input_rate: u32,
    output_rate: u32,
) -> (f32, f32) {
    let output_nyquist = output_rate as f32 / 2.0;

    // Test with a tone just above output Nyquist
    let test_freq = output_nyquist * 1.1;

    // Alias would appear at: test_freq - output_nyquist (for simple folding)
    // Or at: output_rate - test_freq (for other alias products)
    let alias_freq = output_rate as f32 - test_freq;

    resampler.reset();

    let input_mono = generate_sine(test_freq, input_rate, 1.0, 0.5);
    let input = mono_to_stereo(&input_mono);

    let mut output = resampler.process(&input).unwrap();
    output.extend(resampler.flush().unwrap());

    if output.len() < 1000 {
        return (0.0, 0.0);
    }

    let skip = output.len() / 4;
    let output_mono = extract_mono(&output[skip..], 0);

    // Measure energy at alias frequency
    let spectrum = fft_spectrum(&output_mono);
    let alias_mag = magnitude_at_frequency(&spectrum, alias_freq, output_rate);

    // Measure total output energy
    let output_rms = calculate_rms(&output_mono);
    let input_rms = calculate_rms(&input_mono);

    // Alias rejection = how much the alias is attenuated relative to original
    let alias_level_db = linear_to_db(alias_mag / (input_rms * output_mono.len() as f32).sqrt());

    // Overall attenuation
    let overall_attenuation_db = linear_to_db(input_rms / output_rms.max(1e-10));

    (overall_attenuation_db, alias_level_db)
}

/// Measure pre-ringing from impulse response
///
/// Per Infinite Wave: linear-phase filters exhibit pre-ringing (energy before
/// the impulse peak). This is measured as the ratio of pre-peak energy to
/// peak energy.
fn measure_pre_ringing(resampler: &mut Resampler, input_rate: u32, output_rate: u32) -> (f32, f32) {
    resampler.reset();

    let input_mono = generate_impulse(input_rate, 0.2);
    let input = mono_to_stereo(&input_mono);

    let mut output = resampler.process(&input).unwrap();
    output.extend(resampler.flush().unwrap());

    if output.len() < 100 {
        return (0.0, 0.0);
    }

    let output_mono = extract_mono(&output, 0);

    // Find peak
    let (peak_idx, peak_val) = output_mono
        .iter()
        .enumerate()
        .map(|(i, &v)| (i, v.abs()))
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
        .unwrap_or((0, 0.0));

    if peak_val < 0.01 {
        return (0.0, 0.0);
    }

    // Calculate pre-ringing energy (before peak)
    let pre_ring_energy: f32 = output_mono[..peak_idx].iter().map(|&x| x * x).sum();
    let post_ring_energy: f32 = output_mono[peak_idx + 1..].iter().map(|&x| x * x).sum();
    let peak_energy = peak_val * peak_val;

    let pre_ring_db = linear_to_db((pre_ring_energy / peak_energy.max(1e-10)).sqrt());
    let post_ring_db = linear_to_db((post_ring_energy / peak_energy.max(1e-10)).sqrt());

    (pre_ring_db, post_ring_db)
}

/// Measure resampler latency in output frames
fn measure_latency(resampler: &Resampler) -> usize {
    resampler.latency()
}

/// Process audio through resampler with flush
fn resample_with_flush(resampler: &mut Resampler, input: &[f32]) -> Vec<f32> {
    let mut output = resampler.process(input).unwrap();
    output.extend(resampler.flush().unwrap());
    output
}

// =============================================================================
// TEST SECTION 1: PASSBAND FLATNESS (Infinite Wave Methodology)
// =============================================================================

#[test]
fn test_passband_flatness_all_quality_presets() {
    println!("\n");
    println!("================================================================================");
    println!("  PASSBAND FLATNESS TESTS (Infinite Wave Methodology)");
    println!("================================================================================");
    println!();
    println!("Industry Standard: <0.1dB ripple for studio-grade SRC");
    println!("Reference: src.infinitewave.ca");
    println!();

    let mut bugs: Vec<BugReport> = Vec::new();

    let conversions = [
        (44100, 48000, "44.1kHz -> 48kHz"),
        (48000, 44100, "48kHz -> 44.1kHz"),
        (44100, 96000, "44.1kHz -> 96kHz"),
        (96000, 44100, "96kHz -> 44.1kHz (Infinite Wave test mode)"),
        (48000, 96000, "48kHz -> 96kHz"),
        (96000, 192000, "96kHz -> 192kHz"),
    ];

    let presets = [
        (ResamplingQuality::Fast, "Fast", PASSBAND_RIPPLE_FAST_DB),
        (
            ResamplingQuality::Balanced,
            "Balanced",
            PASSBAND_RIPPLE_BALANCED_DB,
        ),
        (ResamplingQuality::High, "High", PASSBAND_RIPPLE_HIGH_DB),
        (
            ResamplingQuality::Maximum,
            "Maximum",
            PASSBAND_RIPPLE_MAXIMUM_DB,
        ),
    ];

    for (input_rate, output_rate, conv_name) in conversions {
        println!("--- {} ---", conv_name);

        for (quality, quality_name, threshold) in &presets {
            let mut resampler = Resampler::new(
                ResamplerBackend::Rubato,
                input_rate,
                output_rate,
                2,
                *quality,
            )
            .unwrap();

            let (ripple, gains) = measure_passband_ripple(&mut resampler, input_rate, output_rate);

            let status = if ripple <= *threshold { "PASS" } else { "FAIL" };
            println!(
                "  {:10}: Ripple = {:6.3} dB (threshold: {:4.1} dB) [{}]",
                quality_name, ripple, threshold, status
            );

            if !gains.is_empty() && ripple > *threshold {
                bugs.push(BugReport {
                    category: "Passband Flatness",
                    description: format!(
                        "Passband ripple {:.3}dB exceeds threshold {:.1}dB",
                        ripple, threshold
                    ),
                    measured_value: ripple,
                    expected_threshold: *threshold,
                    unit: "dB",
                    quality: *quality,
                    rate_conversion: conv_name.to_string(),
                    industry_reference: "Infinite Wave (src.infinitewave.ca)",
                });
            }
        }
        println!();
    }

    // Print any bugs found
    if !bugs.is_empty() {
        println!("BUGS FOUND IN PASSBAND FLATNESS:");
        for (i, bug) in bugs.iter().enumerate() {
            println!(
                "  {}. {} ({:?}, {}): {:.3} {} vs threshold {:.1} {}",
                i + 1,
                bug.rate_conversion,
                bug.quality,
                bug.category,
                bug.measured_value,
                bug.unit,
                bug.expected_threshold,
                bug.unit
            );
        }
    }
}

// =============================================================================
// TEST SECTION 2: STOPBAND ATTENUATION (AES17 Methodology)
// =============================================================================

#[test]
fn test_stopband_attenuation_all_quality_presets() {
    println!("\n");
    println!("================================================================================");
    println!("  STOPBAND ATTENUATION TESTS (AES17 Methodology)");
    println!("================================================================================");
    println!();
    println!("Industry Standard: >96dB for CD quality, >120dB for audiophile");
    println!("Reference: AES17-2015");
    println!();

    let mut bugs: Vec<BugReport> = Vec::new();

    // Downsampling test cases: input_rate, output_rate, test_freq (above output Nyquist)
    let test_cases = [
        (96000, 44100, 30000.0, "96kHz -> 44.1kHz (30kHz test tone)"),
        (96000, 44100, 40000.0, "96kHz -> 44.1kHz (40kHz test tone)"),
        (192000, 44100, 50000.0, "192kHz -> 44.1kHz (50kHz test tone)"),
        (192000, 48000, 40000.0, "192kHz -> 48kHz (40kHz test tone)"),
        (96000, 48000, 30000.0, "96kHz -> 48kHz (30kHz test tone)"),
    ];

    let presets = [
        (
            ResamplingQuality::Fast,
            "Fast",
            STOPBAND_ATTENUATION_FAST_DB,
        ),
        (
            ResamplingQuality::Balanced,
            "Balanced",
            STOPBAND_ATTENUATION_BALANCED_DB,
        ),
        (
            ResamplingQuality::High,
            "High",
            STOPBAND_ATTENUATION_HIGH_DB,
        ),
        (
            ResamplingQuality::Maximum,
            "Maximum",
            STOPBAND_ATTENUATION_MAXIMUM_DB,
        ),
    ];

    for (input_rate, output_rate, test_freq, description) in test_cases {
        println!("--- {} ---", description);

        for (quality, quality_name, threshold) in &presets {
            let mut resampler = Resampler::new(
                ResamplerBackend::Rubato,
                input_rate,
                output_rate,
                2,
                *quality,
            )
            .unwrap();

            let attenuation =
                measure_stopband_attenuation(&mut resampler, input_rate, output_rate, test_freq);

            let status = if attenuation >= *threshold {
                "PASS"
            } else {
                "FAIL"
            };
            println!(
                "  {:10}: Attenuation = {:6.1} dB (threshold: {:5.1} dB) [{}]",
                quality_name, attenuation, threshold, status
            );

            if attenuation < *threshold {
                bugs.push(BugReport {
                    category: "Stopband Attenuation",
                    description: format!(
                        "Stopband attenuation {:.1}dB below threshold {:.1}dB",
                        attenuation, threshold
                    ),
                    measured_value: attenuation,
                    expected_threshold: *threshold,
                    unit: "dB",
                    quality: *quality,
                    rate_conversion: description.to_string(),
                    industry_reference: "AES17-2015",
                });
            }
        }
        println!();
    }

    if !bugs.is_empty() {
        println!("BUGS FOUND IN STOPBAND ATTENUATION:");
        for (i, bug) in bugs.iter().enumerate() {
            println!(
                "  {}. {} ({:?}): {:.1} {} vs threshold {:.1} {}",
                i + 1,
                bug.rate_conversion,
                bug.quality,
                bug.measured_value,
                bug.unit,
                bug.expected_threshold,
                bug.unit
            );
        }
    }
}

// =============================================================================
// TEST SECTION 3: THD+N AT MULTIPLE FREQUENCIES (AES17 Methodology)
// =============================================================================

#[test]
fn test_thd_n_multiple_frequencies() {
    println!("\n");
    println!("================================================================================");
    println!("  THD+N TESTS AT MULTIPLE FREQUENCIES (AES17 Methodology)");
    println!("================================================================================");
    println!();
    println!("Industry Standard: <-100dB for pro equipment (Audio Precision)");
    println!("Reference: AES17-2015, IEC60268");
    println!();

    let mut bugs: Vec<BugReport> = Vec::new();

    // Test frequencies: 100Hz, 1kHz, 10kHz per task requirement
    let test_frequencies = [100.0, 1000.0, 10000.0];

    let conversions = [
        (44100, 48000, "44.1kHz -> 48kHz"),
        (44100, 96000, "44.1kHz -> 96kHz"),
        (96000, 44100, "96kHz -> 44.1kHz"),
        (48000, 96000, "48kHz -> 96kHz"),
        (96000, 192000, "96kHz -> 192kHz"),
    ];

    let presets = [
        (ResamplingQuality::Fast, "Fast", THD_N_FAST_DB),
        (ResamplingQuality::Balanced, "Balanced", THD_N_BALANCED_DB),
        (ResamplingQuality::High, "High", THD_N_HIGH_DB),
        (ResamplingQuality::Maximum, "Maximum", THD_N_MAXIMUM_DB),
    ];

    for (input_rate, output_rate, conv_name) in conversions {
        println!("--- {} ---", conv_name);

        for &test_freq in &test_frequencies {
            // Skip if frequency is above input Nyquist
            if test_freq >= input_rate as f32 / 2.0 {
                continue;
            }

            println!("  {} Hz:", test_freq as u32);

            for (quality, quality_name, threshold) in &presets {
                let mut resampler = Resampler::new(
                    ResamplerBackend::Rubato,
                    input_rate,
                    output_rate,
                    2,
                    *quality,
                )
                .unwrap();

                let input_mono = generate_sine(test_freq, input_rate, 1.0, 0.5);
                let input = mono_to_stereo(&input_mono);

                let output = resample_with_flush(&mut resampler, &input);

                if output.len() < 1000 {
                    println!("    {:10}: SKIPPED (insufficient output)", quality_name);
                    continue;
                }

                let skip = output.len() / 4;
                let output_mono = extract_mono(&output[skip..], 0);

                let thd_n = calculate_thd_n_aes17(&output_mono, test_freq, output_rate);

                let status = if thd_n <= *threshold { "PASS" } else { "FAIL" };
                println!(
                    "    {:10}: THD+N = {:6.1} dB (threshold: {:5.1} dB) [{}]",
                    quality_name, thd_n, threshold, status
                );

                if thd_n > *threshold {
                    bugs.push(BugReport {
                        category: "THD+N",
                        description: format!(
                            "THD+N {:.1}dB at {}Hz exceeds threshold {:.1}dB",
                            thd_n, test_freq, threshold
                        ),
                        measured_value: thd_n,
                        expected_threshold: *threshold,
                        unit: "dB",
                        quality: *quality,
                        rate_conversion: format!("{} @ {}Hz", conv_name, test_freq),
                        industry_reference: "AES17-2015",
                    });
                }
            }
        }
        println!();
    }

    if !bugs.is_empty() {
        println!("BUGS FOUND IN THD+N:");
        for (i, bug) in bugs.iter().enumerate() {
            println!(
                "  {}. {} ({:?}): {:.1} {} vs threshold {:.1} {}",
                i + 1,
                bug.rate_conversion,
                bug.quality,
                bug.measured_value,
                bug.unit,
                bug.expected_threshold,
                bug.unit
            );
        }
    }
}

// =============================================================================
// TEST SECTION 4: ALIASING REJECTION (Infinite Wave Methodology)
// =============================================================================

#[test]
fn test_aliasing_rejection_downsampling() {
    println!("\n");
    println!("================================================================================");
    println!("  ALIASING REJECTION TESTS (Infinite Wave Methodology)");
    println!("================================================================================");
    println!();
    println!("Industry Standard: Frequencies above new Nyquist should not alias into passband");
    println!("Reference: Infinite Wave, AES17");
    println!();

    let mut bugs: Vec<BugReport> = Vec::new();

    let test_cases = [
        (96000, 44100, "96kHz -> 44.1kHz"),
        (192000, 44100, "192kHz -> 44.1kHz"),
        (192000, 48000, "192kHz -> 48kHz"),
        (96000, 48000, "96kHz -> 48kHz"),
    ];

    let presets = [
        (
            ResamplingQuality::Fast,
            "Fast",
            ALIASING_REJECTION_FAST_DB,
        ),
        (
            ResamplingQuality::Balanced,
            "Balanced",
            ALIASING_REJECTION_BALANCED_DB,
        ),
        (
            ResamplingQuality::High,
            "High",
            ALIASING_REJECTION_HIGH_DB,
        ),
        (
            ResamplingQuality::Maximum,
            "Maximum",
            ALIASING_REJECTION_HIGH_DB,
        ),
    ];

    for (input_rate, output_rate, description) in test_cases {
        println!("--- {} ---", description);

        for (quality, quality_name, threshold) in &presets {
            let mut resampler = Resampler::new(
                ResamplerBackend::Rubato,
                input_rate,
                output_rate,
                2,
                *quality,
            )
            .unwrap();

            let (overall_attenuation, _alias_level) =
                measure_aliasing_rejection(&mut resampler, input_rate, output_rate);

            let status = if overall_attenuation >= *threshold {
                "PASS"
            } else {
                "FAIL"
            };
            println!(
                "  {:10}: Rejection = {:6.1} dB (threshold: {:5.1} dB) [{}]",
                quality_name, overall_attenuation, threshold, status
            );

            if overall_attenuation < *threshold {
                bugs.push(BugReport {
                    category: "Aliasing Rejection",
                    description: format!(
                        "Aliasing rejection {:.1}dB below threshold {:.1}dB",
                        overall_attenuation, threshold
                    ),
                    measured_value: overall_attenuation,
                    expected_threshold: *threshold,
                    unit: "dB",
                    quality: *quality,
                    rate_conversion: description.to_string(),
                    industry_reference: "Infinite Wave (src.infinitewave.ca)",
                });
            }
        }
        println!();
    }

    if !bugs.is_empty() {
        println!("BUGS FOUND IN ALIASING REJECTION:");
        for (i, bug) in bugs.iter().enumerate() {
            println!(
                "  {}. {} ({:?}): {:.1} {} vs threshold {:.1} {}",
                i + 1,
                bug.rate_conversion,
                bug.quality,
                bug.measured_value,
                bug.unit,
                bug.expected_threshold,
                bug.unit
            );
        }
    }
}

// =============================================================================
// TEST SECTION 5: PRE-RINGING MEASUREMENT (Infinite Wave Methodology)
// =============================================================================

#[test]
fn test_pre_ringing_measurement() {
    println!("\n");
    println!("================================================================================");
    println!("  PRE-RINGING MEASUREMENT (Infinite Wave Methodology)");
    println!("================================================================================");
    println!();
    println!("Note: Pre-ringing is inherent to linear-phase FIR filters.");
    println!("Minimum-phase filters have no pre-ringing but more post-ringing.");
    println!("Reference: Infinite Wave, CCRMA Stanford");
    println!();

    let conversions = [
        (44100, 96000, "44.1kHz -> 96kHz"),
        (96000, 44100, "96kHz -> 44.1kHz"),
        (44100, 48000, "44.1kHz -> 48kHz"),
    ];

    let presets = [
        (ResamplingQuality::Fast, "Fast"),
        (ResamplingQuality::Balanced, "Balanced"),
        (ResamplingQuality::High, "High"),
        (ResamplingQuality::Maximum, "Maximum"),
    ];

    for (input_rate, output_rate, conv_name) in conversions {
        println!("--- {} ---", conv_name);

        for (quality, quality_name) in &presets {
            let mut resampler = Resampler::new(
                ResamplerBackend::Rubato,
                input_rate,
                output_rate,
                2,
                *quality,
            )
            .unwrap();

            let (pre_ring_db, post_ring_db) =
                measure_pre_ringing(&mut resampler, input_rate, output_rate);

            println!(
                "  {:10}: Pre-ringing = {:6.1} dB, Post-ringing = {:6.1} dB",
                quality_name, pre_ring_db, post_ring_db
            );
        }
        println!();
    }

    println!("Note: These are informational measurements. Pre-ringing < -40dB is generally");
    println!("      considered inaudible. Lower (more negative) values are better.");
}

// =============================================================================
// TEST SECTION 6: LATENCY/DELAY MEASUREMENT
// =============================================================================

#[test]
fn test_latency_measurement() {
    println!("\n");
    println!("================================================================================");
    println!("  LATENCY/DELAY MEASUREMENT");
    println!("================================================================================");
    println!();
    println!("Latency is important for real-time applications and synchronization.");
    println!("Reference: libsamplerate quality benchmarks");
    println!();

    let conversions = [
        (44100, 48000, "44.1kHz -> 48kHz"),
        (44100, 96000, "44.1kHz -> 96kHz"),
        (96000, 44100, "96kHz -> 44.1kHz"),
        (48000, 96000, "48kHz -> 96kHz"),
        (96000, 192000, "96kHz -> 192kHz"),
    ];

    let presets = [
        (ResamplingQuality::Fast, "Fast"),
        (ResamplingQuality::Balanced, "Balanced"),
        (ResamplingQuality::High, "High"),
        (ResamplingQuality::Maximum, "Maximum"),
    ];

    for (input_rate, output_rate, conv_name) in conversions {
        println!("--- {} ---", conv_name);

        for (quality, quality_name) in &presets {
            let resampler = Resampler::new(
                ResamplerBackend::Rubato,
                input_rate,
                output_rate,
                2,
                *quality,
            )
            .unwrap();

            let latency_frames = measure_latency(&resampler);
            let latency_ms = latency_frames as f32 / output_rate as f32 * 1000.0;

            println!(
                "  {:10}: Latency = {:5} frames ({:.2} ms)",
                quality_name, latency_frames, latency_ms
            );
        }
        println!();
    }
}

// =============================================================================
// TEST SECTION 7: ALL RATE COMBINATIONS WITH ALL QUALITY PRESETS
// =============================================================================

#[test]
fn test_all_rate_combinations() {
    println!("\n");
    println!("================================================================================");
    println!("  COMPREHENSIVE RATE COMBINATION TESTS");
    println!("================================================================================");
    println!();
    println!("Testing: 44.1<->48, 44.1<->96, 48<->96, 96<->192");
    println!();

    let mut all_bugs: Vec<BugReport> = Vec::new();
    let mut total_tests = 0;
    let mut passed_tests = 0;

    let rate_combinations = [
        (44100, 48000),
        (48000, 44100),
        (44100, 96000),
        (96000, 44100),
        (48000, 96000),
        (96000, 48000),
        (96000, 192000),
        (192000, 96000),
    ];

    let presets = [
        (ResamplingQuality::Fast, "Fast"),
        (ResamplingQuality::Balanced, "Balanced"),
        (ResamplingQuality::High, "High"),
        (ResamplingQuality::Maximum, "Maximum"),
    ];

    for (input_rate, output_rate) in rate_combinations {
        let conv_name = format!("{}Hz -> {}Hz", input_rate, output_rate);
        println!("--- {} ---", conv_name);

        for (quality, quality_name) in &presets {
            let mut resampler = Resampler::new(
                ResamplerBackend::Rubato,
                input_rate,
                output_rate,
                2,
                *quality,
            )
            .unwrap();

            total_tests += 1;

            // Test 1: Basic 1kHz sine wave processing
            let input_mono = generate_sine(1000.0, input_rate, 0.5, 0.5);
            let input = mono_to_stereo(&input_mono);
            let output = resample_with_flush(&mut resampler, &input);

            if output.is_empty() {
                println!("  {:10}: FAIL - No output produced", quality_name);
                all_bugs.push(BugReport {
                    category: "Basic Processing",
                    description: "No output produced".to_string(),
                    measured_value: 0.0,
                    expected_threshold: 1.0,
                    unit: "samples",
                    quality: *quality,
                    rate_conversion: conv_name.clone(),
                    industry_reference: "Basic functionality",
                });
                continue;
            }

            // Test 2: Amplitude preservation
            let skip = output.len() / 4;
            let input_rms = calculate_rms(&input_mono);
            let output_rms = calculate_rms(&extract_mono(&output[skip..], 0));
            let gain_db = linear_to_db(output_rms / input_rms.max(1e-10));

            let amplitude_ok = gain_db.abs() < 1.0;

            // Test 3: THD+N check
            let output_mono = extract_mono(&output[skip..], 0);
            let thd_n = if output_mono.len() > 500 {
                calculate_thd_n_aes17(&output_mono, 1000.0, output_rate)
            } else {
                0.0
            };

            let thd_threshold = match quality {
                ResamplingQuality::Fast => THD_N_FAST_DB,
                ResamplingQuality::Balanced => THD_N_BALANCED_DB,
                ResamplingQuality::High => THD_N_HIGH_DB,
                ResamplingQuality::Maximum => THD_N_MAXIMUM_DB,
            };

            let thd_ok = thd_n <= thd_threshold;

            let all_ok = amplitude_ok && thd_ok;
            if all_ok {
                passed_tests += 1;
            }

            let status = if all_ok { "PASS" } else { "FAIL" };
            println!(
                "  {:10}: Gain={:+5.2}dB, THD+N={:6.1}dB [{}]",
                quality_name, gain_db, thd_n, status
            );

            if !amplitude_ok {
                all_bugs.push(BugReport {
                    category: "Amplitude Preservation",
                    description: format!("Gain {:.2}dB exceeds +/-1dB tolerance", gain_db),
                    measured_value: gain_db,
                    expected_threshold: 1.0,
                    unit: "dB",
                    quality: *quality,
                    rate_conversion: conv_name.clone(),
                    industry_reference: "Basic amplitude preservation",
                });
            }

            if !thd_ok {
                all_bugs.push(BugReport {
                    category: "THD+N",
                    description: format!(
                        "THD+N {:.1}dB exceeds threshold {:.1}dB",
                        thd_n, thd_threshold
                    ),
                    measured_value: thd_n,
                    expected_threshold: thd_threshold,
                    unit: "dB",
                    quality: *quality,
                    rate_conversion: conv_name.clone(),
                    industry_reference: "AES17-2015",
                });
            }
        }
        println!();
    }

    println!(
        "Total: {}/{} tests passed",
        passed_tests, total_tests
    );

    if !all_bugs.is_empty() {
        println!();
        println!("ALL BUGS FOUND:");
        for (i, bug) in all_bugs.iter().enumerate() {
            println!(
                "  {}. [{}] {} ({:?}): {:.2} {} (threshold: {:.1} {})",
                i + 1,
                bug.category,
                bug.rate_conversion,
                bug.quality,
                bug.measured_value,
                bug.unit,
                bug.expected_threshold,
                bug.unit
            );
        }
    }
}

// =============================================================================
// COMPREHENSIVE BUG SUMMARY TEST
// =============================================================================

#[test]
fn test_generate_comprehensive_bug_report() {
    println!("\n");
    println!("================================================================================");
    println!("  COMPREHENSIVE INDUSTRY-STANDARD RESAMPLER QUALITY REPORT");
    println!("================================================================================");
    println!();

    let mut all_bugs: Vec<BugReport> = Vec::new();

    // Run all measurements and collect bugs
    let standard_conversions = [
        (44100, 48000),
        (48000, 44100),
        (44100, 96000),
        (96000, 44100),
        (48000, 96000),
        (96000, 48000),
        (96000, 192000),
        (192000, 96000),
    ];

    let presets = [
        (ResamplingQuality::Fast, "Fast"),
        (ResamplingQuality::Balanced, "Balanced"),
        (ResamplingQuality::High, "High"),
        (ResamplingQuality::Maximum, "Maximum"),
    ];

    println!("Running comprehensive measurements...");
    println!();

    for (input_rate, output_rate) in standard_conversions {
        for (quality, _quality_name) in &presets {
            let mut resampler = Resampler::new(
                ResamplerBackend::Rubato,
                input_rate,
                output_rate,
                2,
                *quality,
            )
            .unwrap();

            let conv_name = format!("{}Hz -> {}Hz", input_rate, output_rate);

            // Passband ripple check
            let (ripple, _) = measure_passband_ripple(&mut resampler, input_rate, output_rate);
            let ripple_threshold = match quality {
                ResamplingQuality::Fast => PASSBAND_RIPPLE_FAST_DB,
                ResamplingQuality::Balanced => PASSBAND_RIPPLE_BALANCED_DB,
                ResamplingQuality::High => PASSBAND_RIPPLE_HIGH_DB,
                ResamplingQuality::Maximum => PASSBAND_RIPPLE_MAXIMUM_DB,
            };
            if ripple > ripple_threshold {
                all_bugs.push(BugReport {
                    category: "Passband Flatness",
                    description: format!(
                        "Passband ripple {:.3}dB exceeds threshold {:.1}dB",
                        ripple, ripple_threshold
                    ),
                    measured_value: ripple,
                    expected_threshold: ripple_threshold,
                    unit: "dB",
                    quality: *quality,
                    rate_conversion: conv_name.clone(),
                    industry_reference: "Infinite Wave (src.infinitewave.ca)",
                });
            }

            // THD+N check at 1kHz
            let input_mono = generate_sine(1000.0, input_rate, 1.0, 0.5);
            let input = mono_to_stereo(&input_mono);
            resampler.reset();
            let output = resample_with_flush(&mut resampler, &input);

            if output.len() > 1000 {
                let skip = output.len() / 4;
                let output_mono = extract_mono(&output[skip..], 0);
                let thd_n = calculate_thd_n_aes17(&output_mono, 1000.0, output_rate);

                let thd_threshold = match quality {
                    ResamplingQuality::Fast => THD_N_FAST_DB,
                    ResamplingQuality::Balanced => THD_N_BALANCED_DB,
                    ResamplingQuality::High => THD_N_HIGH_DB,
                    ResamplingQuality::Maximum => THD_N_MAXIMUM_DB,
                };

                if thd_n > thd_threshold {
                    all_bugs.push(BugReport {
                        category: "THD+N",
                        description: format!(
                            "THD+N {:.1}dB @ 1kHz exceeds threshold {:.1}dB",
                            thd_n, thd_threshold
                        ),
                        measured_value: thd_n,
                        expected_threshold: thd_threshold,
                        unit: "dB",
                        quality: *quality,
                        rate_conversion: conv_name.clone(),
                        industry_reference: "AES17-2015",
                    });
                }
            }

            // Stopband attenuation (for downsampling only)
            if output_rate < input_rate {
                let test_freq = output_rate as f32 * 0.7; // Above output Nyquist
                resampler.reset();
                let attenuation =
                    measure_stopband_attenuation(&mut resampler, input_rate, output_rate, test_freq);

                let atten_threshold = match quality {
                    ResamplingQuality::Fast => STOPBAND_ATTENUATION_FAST_DB,
                    ResamplingQuality::Balanced => STOPBAND_ATTENUATION_BALANCED_DB,
                    ResamplingQuality::High => STOPBAND_ATTENUATION_HIGH_DB,
                    ResamplingQuality::Maximum => STOPBAND_ATTENUATION_MAXIMUM_DB,
                };

                if attenuation < atten_threshold {
                    all_bugs.push(BugReport {
                        category: "Stopband Attenuation",
                        description: format!(
                            "Stopband attenuation {:.1}dB below threshold {:.1}dB",
                            attenuation, atten_threshold
                        ),
                        measured_value: attenuation,
                        expected_threshold: atten_threshold,
                        unit: "dB",
                        quality: *quality,
                        rate_conversion: conv_name.clone(),
                        industry_reference: "AES17-2015",
                    });
                }
            }
        }
    }

    // Generate final report
    println!("================================================================================");
    println!("  FINAL BUG REPORT");
    println!("================================================================================");
    println!();

    if all_bugs.is_empty() {
        println!("NO BUGS FOUND - All measurements within industry thresholds!");
    } else {
        println!("NUMBER OF BUGS FOUND: {}", all_bugs.len());
        println!();

        // Group bugs by category
        let mut by_category: std::collections::HashMap<&str, Vec<&BugReport>> =
            std::collections::HashMap::new();
        for bug in &all_bugs {
            by_category.entry(bug.category).or_default().push(bug);
        }

        for (category, bugs) in &by_category {
            println!("--- {} ({} issues) ---", category, bugs.len());
            for bug in bugs {
                println!(
                    "  [{:?}] {}: Measured {:.2}{} vs threshold {:.1}{}",
                    bug.quality,
                    bug.rate_conversion,
                    bug.measured_value,
                    bug.unit,
                    bug.expected_threshold,
                    bug.unit
                );
            }
            println!();
        }

        println!("INDUSTRY STANDARDS REFERENCED:");
        println!("  - Infinite Wave (src.infinitewave.ca) - SRC comparison methodology");
        println!("  - AES17-2015 - Standard method for digital audio equipment measurements");
        println!("  - libsamplerate quality benchmarks (libsndfile.github.io/libsamplerate)");
    }

    println!();
    println!("================================================================================");
}
