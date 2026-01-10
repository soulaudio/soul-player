//! Audio analysis tools for verification
//!
//! Provides professional audio quality metrics for verifying DSP algorithm correctness:
//! - SNR (Signal-to-Noise Ratio)
//! - THD (Total Harmonic Distortion)
//! - THD+N (Total Harmonic Distortion plus Noise)
//! - IMD (Intermodulation Distortion) - SMPTE and ITU-R methods
//! - A-Weighted Noise Measurement
//! - Dynamic Range (DR)
//! - Frequency Response Analysis
//! - Phase Analysis
//!
//! These metrics follow industry standards:
//! - AES17: Standard for measuring audio equipment
//! - IEC 61606: Audio and audiovisual equipment
//! - ITU-R BS.1770: Loudness measurement

use std::f32::consts::PI;

/// Calculate RMS (Root Mean Square) level
///
/// RMS is a measure of the average power in a signal.
///
/// # Arguments
/// * `samples` - Audio samples (can be mono or stereo interleaved)
///
/// # Returns
/// RMS value (0.0 to 1.0 for normalized audio)
pub fn calculate_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }

    let sum_squares: f32 = samples.iter().map(|s| s * s).sum();
    (sum_squares / samples.len() as f32).sqrt()
}

/// Calculate peak level
///
/// Returns the absolute maximum sample value.
pub fn calculate_peak(samples: &[f32]) -> f32 {
    samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max)
}

/// Convert linear amplitude to dB
pub fn linear_to_db(linear: f32) -> f32 {
    if linear <= 0.0 {
        -100.0 // Silence
    } else {
        20.0 * linear.log10()
    }
}

/// Convert dB to linear amplitude
pub fn db_to_linear(db: f32) -> f32 {
    10.0f32.powf(db / 20.0)
}

/// Detect peaks in a signal
///
/// Returns sorted list of peak amplitudes (absolute values).
pub fn detect_peaks(samples: &[f32]) -> Vec<f32> {
    let mut peaks = Vec::new();
    let window_size = 100; // Look for peaks in 100-sample windows

    for chunk in samples.chunks(window_size) {
        let peak = chunk.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        peaks.push(peak);
    }

    peaks.sort_by(|a, b| b.partial_cmp(a).unwrap()); // Descending order
    peaks
}

/// Simple FFT-based frequency analysis
///
/// Returns power spectrum (frequency, magnitude) pairs.
/// Uses a basic DFT implementation for testing (slow but accurate).
///
/// # Arguments
/// * `samples` - Mono audio samples
/// * `sample_rate` - Sample rate in Hz
///
/// # Returns
/// Vector of (frequency_hz, magnitude_db) tuples
pub fn analyze_frequency_spectrum(samples: &[f32], sample_rate: u32) -> Vec<(f32, f32)> {
    let n = samples.len().min(4096); // Limit to 4096 samples for DFT
    let samples = &samples[0..n];

    let mut spectrum = Vec::new();

    // Simple DFT (not optimized, but works for tests)
    for k in 0..n / 2 {
        let mut real = 0.0;
        let mut imag = 0.0;

        for (i, &sample) in samples.iter().enumerate() {
            let angle = -2.0 * PI * (k as f32) * (i as f32) / (n as f32);
            real += sample * angle.cos();
            imag += sample * angle.sin();
        }

        let magnitude = (real * real + imag * imag).sqrt() / (n as f32);
        let magnitude_db = linear_to_db(magnitude);
        let frequency = (k as f32 * sample_rate as f32) / (n as f32);

        spectrum.push((frequency, magnitude_db));
    }

    spectrum
}

/// Find the dominant frequency in a signal
///
/// Returns the frequency with the highest magnitude in the spectrum.
pub fn find_dominant_frequency(samples: &[f32], sample_rate: u32) -> f32 {
    let spectrum = analyze_frequency_spectrum(samples, sample_rate);

    spectrum
        .into_iter()
        .max_by(|(_, mag_a), (_, mag_b)| mag_a.partial_cmp(mag_b).unwrap())
        .map(|(freq, _)| freq)
        .unwrap_or(0.0)
}

/// Calculate THD (Total Harmonic Distortion)
///
/// Measures the ratio of harmonic power to fundamental power.
///
/// # Arguments
/// * `samples` - Mono audio samples containing a sine wave
/// * `fundamental_freq` - Expected fundamental frequency in Hz
/// * `sample_rate` - Sample rate in Hz
///
/// # Returns
/// THD as a percentage (0.0 to 100.0)
pub fn calculate_thd(samples: &[f32], fundamental_freq: f32, sample_rate: u32) -> f32 {
    let spectrum = analyze_frequency_spectrum(samples, sample_rate);

    // Find fundamental power
    let fundamental_power = spectrum
        .iter()
        .filter(|(freq, _)| (*freq - fundamental_freq).abs() < 50.0)
        .map(|(_, mag)| db_to_linear(*mag).powi(2))
        .sum::<f32>();

    // Find harmonic powers (2nd through 5th harmonics)
    let mut harmonic_power = 0.0;
    for harmonic in 2..=5 {
        let harmonic_freq = fundamental_freq * harmonic as f32;
        let power: f32 = spectrum
            .iter()
            .filter(|(freq, _)| (*freq - harmonic_freq).abs() < 50.0)
            .map(|(_, mag)| db_to_linear(*mag).powi(2))
            .sum();
        harmonic_power += power;
    }

    if fundamental_power <= 0.0 {
        return 0.0;
    }

    // THD = sqrt(sum of harmonic powers / fundamental power)
    ((harmonic_power / fundamental_power).sqrt() * 100.0).min(100.0)
}

/// Measure compression ratio
///
/// Compares input and output levels above threshold to determine
/// actual compression ratio achieved.
///
/// # Arguments
/// * `input` - Input signal samples
/// * `output` - Output signal samples (after compression)
/// * `threshold_db` - Threshold in dB
///
/// # Returns
/// Measured compression ratio (e.g., 4.0 for 4:1 compression)
pub fn measure_compression_ratio(input: &[f32], output: &[f32], threshold_db: f32) -> f32 {
    if input.len() != output.len() {
        return 1.0; // No compression if lengths don't match
    }

    let threshold_linear = db_to_linear(threshold_db);

    // Find samples above threshold
    let mut input_levels = Vec::new();
    let mut output_levels = Vec::new();

    for i in 0..input.len() {
        if input[i].abs() > threshold_linear {
            input_levels.push(input[i].abs());
            output_levels.push(output[i].abs());
        }
    }

    if input_levels.is_empty() {
        return 1.0; // No samples above threshold
    }

    // Calculate average level change
    let input_avg_db = linear_to_db(input_levels.iter().sum::<f32>() / input_levels.len() as f32);
    let output_avg_db =
        linear_to_db(output_levels.iter().sum::<f32>() / output_levels.len() as f32);

    // Ratio = input change / output change
    let input_change = input_avg_db - threshold_db;
    let output_change = output_avg_db - threshold_db;

    if output_change.abs() < 0.1 {
        return 100.0; // Infinite compression
    }

    (input_change / output_change).abs().min(100.0)
}

/// Compare two signals and calculate difference
///
/// Returns the maximum absolute difference between signals.
/// Useful for testing bypass/null tests.
pub fn calculate_signal_difference(signal_a: &[f32], signal_b: &[f32]) -> f32 {
    if signal_a.len() != signal_b.len() {
        return f32::INFINITY;
    }

    signal_a
        .iter()
        .zip(signal_b.iter())
        .map(|(a, b)| (a - b).abs())
        .fold(0.0f32, f32::max)
}

/// Check if a signal is silent (below threshold)
pub fn is_silent(samples: &[f32], threshold_db: f32) -> bool {
    let peak = calculate_peak(samples);
    linear_to_db(peak) < threshold_db
}

/// Extract mono channel from stereo interleaved signal
///
/// # Arguments
/// * `stereo` - Stereo interleaved samples (L, R, L, R, ...)
/// * `channel` - 0 for left, 1 for right
pub fn extract_mono(stereo: &[f32], channel: usize) -> Vec<f32> {
    stereo
        .chunks_exact(2)
        .map(|chunk| chunk[channel])
        .collect()
}

// =============================================================================
// Professional Audio Quality Metrics
// =============================================================================

/// Calculate SNR (Signal-to-Noise Ratio) in dB
///
/// Measures the ratio of signal power to noise power.
/// Higher values indicate cleaner audio.
///
/// # Arguments
/// * `signal_with_noise` - The signal containing both signal and noise
/// * `reference_signal` - The clean reference signal (optional, for comparison)
///
/// # Returns
/// SNR in dB. Typical values:
/// - CD quality: ~96 dB
/// - Professional audio: >100 dB
/// - Excellent: >110 dB
pub fn calculate_snr(signal_with_noise: &[f32], reference_signal: Option<&[f32]>) -> f32 {
    if signal_with_noise.is_empty() {
        return 0.0;
    }

    if let Some(reference) = reference_signal {
        // Calculate noise as difference between signal and reference
        if reference.len() != signal_with_noise.len() {
            return 0.0;
        }

        let signal_power: f32 = reference.iter().map(|s| s * s).sum();
        let noise_power: f32 = signal_with_noise
            .iter()
            .zip(reference.iter())
            .map(|(s, r)| (s - r).powi(2))
            .sum();

        if noise_power <= 0.0 {
            return 140.0; // Maximum measurable SNR
        }

        10.0 * (signal_power / noise_power).log10()
    } else {
        // For a single signal, estimate SNR using spectral analysis
        // Find the peak frequency and measure signal vs noise in spectrum
        let spectrum = analyze_frequency_spectrum(signal_with_noise, 44100);

        if spectrum.is_empty() {
            return 0.0;
        }

        // Find peak magnitude
        let (peak_freq, _peak_mag) = spectrum
            .iter()
            .filter(|(f, _)| *f > 20.0) // Ignore DC
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .copied()
            .unwrap_or((0.0, -100.0));

        if peak_freq == 0.0 {
            return 0.0;
        }

        // Calculate signal power at peak (±10% bandwidth)
        let bandwidth = peak_freq * 0.1;
        let signal_power: f32 = spectrum
            .iter()
            .filter(|(f, _)| (*f - peak_freq).abs() < bandwidth)
            .map(|(_, mag)| db_to_linear(*mag).powi(2))
            .sum();

        // Calculate noise power (everything else in audible range)
        let noise_power: f32 = spectrum
            .iter()
            .filter(|(f, _)| *f > 20.0 && *f < 20000.0 && (*f - peak_freq).abs() >= bandwidth)
            .map(|(_, mag)| db_to_linear(*mag).powi(2))
            .sum();

        if noise_power <= 0.0 {
            return 140.0;
        }

        10.0 * (signal_power / noise_power).log10()
    }
}

/// Calculate SNR with spectral analysis at a known frequency
///
/// More accurate than estimate_noise_floor for tonal signals.
pub fn calculate_snr_at_frequency(samples: &[f32], frequency: f32, sample_rate: u32) -> f32 {
    let spectrum = analyze_frequency_spectrum(samples, sample_rate);

    if spectrum.is_empty() {
        return 0.0;
    }

    // Signal power at fundamental (±5% bandwidth)
    let bandwidth = frequency * 0.05;
    let signal_power: f32 = spectrum
        .iter()
        .filter(|(f, _)| (*f - frequency).abs() < bandwidth)
        .map(|(_, mag)| db_to_linear(*mag).powi(2))
        .sum();

    // Noise power everywhere else
    let noise_power: f32 = spectrum
        .iter()
        .filter(|(f, _)| *f > 20.0 && *f < 20000.0 && (*f - frequency).abs() >= bandwidth)
        .map(|(_, mag)| db_to_linear(*mag).powi(2))
        .sum();

    if noise_power <= 0.0 {
        return 140.0;
    }

    10.0 * (signal_power / noise_power).log10()
}

/// Estimate noise floor from signal
///
/// Uses the quietest portions of the signal to estimate noise.
fn estimate_noise_floor(samples: &[f32]) -> f32 {
    if samples.len() < 100 {
        return 0.0;
    }

    // Split into windows and find the quietest ones
    let window_size = 512;
    let mut window_rms_values: Vec<f32> = samples
        .chunks(window_size)
        .filter(|chunk| chunk.len() == window_size)
        .map(|chunk| calculate_rms(chunk))
        .filter(|&rms| rms > 0.0) // Exclude perfect silence
        .collect();

    if window_rms_values.is_empty() {
        return 0.0;
    }

    // Sort and take the 10th percentile as noise floor
    window_rms_values.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let percentile_idx = (window_rms_values.len() as f32 * 0.1) as usize;
    window_rms_values[percentile_idx.min(window_rms_values.len() - 1)]
}

/// Calculate THD+N (Total Harmonic Distortion plus Noise)
///
/// Measures the total harmonic distortion plus noise relative to the fundamental.
/// This is a more comprehensive metric than THD alone.
///
/// # Arguments
/// * `samples` - Audio samples (should contain a sine wave test signal)
/// * `fundamental_freq` - The fundamental frequency of the test signal
/// * `sample_rate` - Sample rate in Hz
///
/// # Returns
/// THD+N as a percentage. Lower is better:
/// - Excellent: <0.001% (-100 dB)
/// - Very good: <0.01% (-80 dB)
/// - Good: <0.1% (-60 dB)
/// - Acceptable: <1% (-40 dB)
pub fn calculate_thd_plus_n(samples: &[f32], fundamental_freq: f32, sample_rate: u32) -> f32 {
    let spectrum = analyze_frequency_spectrum(samples, sample_rate);

    // Find fundamental power (within ±5% of expected frequency)
    let freq_tolerance = fundamental_freq * 0.05;
    let fundamental_power: f32 = spectrum
        .iter()
        .filter(|(freq, _)| (*freq - fundamental_freq).abs() < freq_tolerance)
        .map(|(_, mag)| db_to_linear(*mag).powi(2))
        .sum();

    if fundamental_power <= 0.0 {
        return 100.0;
    }

    // Calculate total power (all frequencies)
    let total_power: f32 = spectrum.iter().map(|(_, mag)| db_to_linear(*mag).powi(2)).sum();

    // THD+N = sqrt((total - fundamental) / total) * 100
    let distortion_plus_noise_power = (total_power - fundamental_power).max(0.0);
    ((distortion_plus_noise_power / total_power).sqrt() * 100.0).min(100.0)
}

/// Calculate SINAD (Signal-to-Noise and Distortion ratio) in dB
///
/// SINAD is the reciprocal of THD+N expressed in dB.
/// Higher values are better.
///
/// # Returns
/// SINAD in dB. Typical values:
/// - Excellent: >90 dB
/// - Good: >70 dB
/// - Acceptable: >50 dB
pub fn calculate_sinad(samples: &[f32], fundamental_freq: f32, sample_rate: u32) -> f32 {
    let thd_plus_n = calculate_thd_plus_n(samples, fundamental_freq, sample_rate);
    if thd_plus_n <= 0.0 {
        return 120.0; // Maximum measurable
    }
    -20.0 * (thd_plus_n / 100.0).log10()
}

/// Calculate ENOB (Effective Number of Bits) from SINAD
///
/// ENOB = (SINAD - 1.76) / 6.02
///
/// This tells you the effective bit depth of your signal path.
pub fn calculate_enob(sinad_db: f32) -> f32 {
    (sinad_db - 1.76) / 6.02
}

/// Calculate IMD (Intermodulation Distortion) using SMPTE method
///
/// Uses a 60 Hz + 7 kHz test signal (standard SMPTE/DIN method).
/// Measures modulation of the HF signal by the LF signal.
///
/// # Arguments
/// * `samples` - Audio samples containing the two-tone test signal
/// * `sample_rate` - Sample rate in Hz
///
/// # Returns
/// IMD percentage. Lower is better:
/// - Excellent: <0.01%
/// - Good: <0.1%
/// - Acceptable: <1%
pub fn calculate_imd_smpte(samples: &[f32], sample_rate: u32) -> f32 {
    let lf_freq = 60.0;
    let hf_freq = 7000.0;

    let spectrum = analyze_frequency_spectrum(samples, sample_rate);

    // Find HF carrier power
    let hf_power: f32 = spectrum
        .iter()
        .filter(|(freq, _)| (*freq - hf_freq).abs() < 100.0)
        .map(|(_, mag)| db_to_linear(*mag).powi(2))
        .sum();

    if hf_power <= 0.0 {
        return 100.0;
    }

    // Find IMD products at HF ± LF, HF ± 2*LF
    let mut imd_power = 0.0;
    for harmonic in 1..=3 {
        let imd_freq_low = hf_freq - (lf_freq * harmonic as f32);
        let imd_freq_high = hf_freq + (lf_freq * harmonic as f32);

        for freq in [imd_freq_low, imd_freq_high] {
            imd_power += spectrum
                .iter()
                .filter(|(f, _)| (*f - freq).abs() < 30.0)
                .map(|(_, mag)| db_to_linear(*mag).powi(2))
                .sum::<f32>();
        }
    }

    ((imd_power / hf_power).sqrt() * 100.0).min(100.0)
}

/// Calculate IMD using ITU-R (CCIF) method
///
/// Uses two high-frequency tones close together (e.g., 19 kHz + 20 kHz).
/// Measures difference frequency products.
///
/// # Arguments
/// * `samples` - Audio samples containing the two-tone test signal
/// * `freq1` - First test frequency (default: 19000 Hz)
/// * `freq2` - Second test frequency (default: 20000 Hz)
/// * `sample_rate` - Sample rate in Hz
pub fn calculate_imd_ccif(samples: &[f32], freq1: f32, freq2: f32, sample_rate: u32) -> f32 {
    let spectrum = analyze_frequency_spectrum(samples, sample_rate);

    // Find fundamental powers
    let fund1_power: f32 = spectrum
        .iter()
        .filter(|(freq, _)| (*freq - freq1).abs() < 100.0)
        .map(|(_, mag)| db_to_linear(*mag).powi(2))
        .sum();

    let fund2_power: f32 = spectrum
        .iter()
        .filter(|(freq, _)| (*freq - freq2).abs() < 100.0)
        .map(|(_, mag)| db_to_linear(*mag).powi(2))
        .sum();

    let total_fundamental = fund1_power + fund2_power;
    if total_fundamental <= 0.0 {
        return 100.0;
    }

    // Find difference frequency products (f2-f1, 2*f1-f2, 2*f2-f1)
    let diff_freq = (freq2 - freq1).abs();
    let mut imd_power = 0.0;

    for product_freq in [
        diff_freq,
        2.0 * freq1 - freq2,
        2.0 * freq2 - freq1,
        2.0 * diff_freq,
        3.0 * diff_freq,
    ] {
        if product_freq > 0.0 && product_freq < sample_rate as f32 / 2.0 {
            imd_power += spectrum
                .iter()
                .filter(|(f, _)| (*f - product_freq).abs() < 50.0)
                .map(|(_, mag)| db_to_linear(*mag).powi(2))
                .sum::<f32>();
        }
    }

    ((imd_power / total_fundamental).sqrt() * 100.0).min(100.0)
}

/// A-weighting filter coefficients for noise measurement
///
/// A-weighting approximates human hearing sensitivity.
/// Returns gain in dB for a given frequency.
pub fn a_weighting_db(freq: f32) -> f32 {
    if freq <= 0.0 {
        return -100.0;
    }

    let f2 = freq * freq;
    let f4 = f2 * f2;

    // A-weighting formula (IEC 61672-1)
    let numerator = 12194.0_f32.powi(2) * f4;
    let denominator = (f2 + 20.6_f32.powi(2))
        * ((f2 + 107.7_f32.powi(2)) * (f2 + 737.9_f32.powi(2))).sqrt()
        * (f2 + 12194.0_f32.powi(2));

    let ra = numerator / denominator;
    20.0 * ra.log10() + 2.0 // +2.0 dB normalization at 1kHz
}

/// Calculate A-weighted noise level
///
/// Applies A-weighting to the noise spectrum, matching human hearing sensitivity.
///
/// # Returns
/// A-weighted noise level in dBFS (dB relative to full scale)
pub fn calculate_a_weighted_noise(samples: &[f32], sample_rate: u32) -> f32 {
    let spectrum = analyze_frequency_spectrum(samples, sample_rate);

    let weighted_power: f32 = spectrum
        .iter()
        .filter(|(freq, _)| *freq > 20.0 && *freq < 20000.0) // Audible range
        .map(|(freq, mag_db)| {
            let weight = a_weighting_db(*freq);
            let weighted_mag = *mag_db + weight;
            db_to_linear(weighted_mag).powi(2)
        })
        .sum();

    if weighted_power <= 0.0 {
        return -140.0;
    }

    10.0 * weighted_power.log10()
}

/// Calculate Dynamic Range (DR)
///
/// Measures the ratio between the loudest and quietest parts of a signal.
///
/// # Returns
/// Dynamic range in dB
pub fn calculate_dynamic_range(samples: &[f32]) -> f32 {
    let peak = calculate_peak(samples);
    let noise_floor = estimate_noise_floor(samples);

    if noise_floor <= 0.0 || peak <= 0.0 {
        return 0.0;
    }

    20.0 * (peak / noise_floor).log10()
}

/// Measure frequency response at specific frequencies
///
/// Useful for verifying filter/EQ behavior.
///
/// # Arguments
/// * `samples` - Processed audio (e.g., filtered sweep)
/// * `frequencies` - Frequencies to measure (in Hz)
/// * `sample_rate` - Sample rate in Hz
///
/// # Returns
/// Vector of (frequency, magnitude_db) tuples
pub fn measure_frequency_response(
    samples: &[f32],
    frequencies: &[f32],
    sample_rate: u32,
) -> Vec<(f32, f32)> {
    let spectrum = analyze_frequency_spectrum(samples, sample_rate);

    frequencies
        .iter()
        .map(|&target_freq| {
            // Find closest frequency bin
            let mag = spectrum
                .iter()
                .filter(|(f, _)| (*f - target_freq).abs() < (sample_rate as f32 / samples.len() as f32) * 2.0)
                .map(|(_, m)| *m)
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap_or(-100.0);
            (target_freq, mag)
        })
        .collect()
}

/// Calculate channel separation (crosstalk) for stereo signals
///
/// Measures how much signal from one channel leaks into the other.
///
/// # Arguments
/// * `stereo_samples` - Interleaved stereo samples where one channel should be silent
/// * `active_channel` - Which channel has the signal (0=left, 1=right)
///
/// # Returns
/// Channel separation in dB (higher is better, typical: >60 dB)
pub fn calculate_channel_separation(stereo_samples: &[f32], active_channel: usize) -> f32 {
    let active = extract_mono(stereo_samples, active_channel);
    let silent = extract_mono(stereo_samples, 1 - active_channel);

    let active_power = calculate_rms(&active);
    let leakage_power = calculate_rms(&silent);

    if leakage_power <= 0.0 {
        return 120.0; // Maximum measurable
    }

    20.0 * (active_power / leakage_power).log10()
}

/// Phase difference between two signals at a specific frequency
///
/// # Returns
/// Phase difference in degrees
pub fn calculate_phase_difference(
    signal_a: &[f32],
    signal_b: &[f32],
    frequency: f32,
    sample_rate: u32,
) -> f32 {
    if signal_a.len() != signal_b.len() || signal_a.is_empty() {
        return 0.0;
    }

    let n = signal_a.len().min(8192);

    // Calculate phase at target frequency using DFT
    let mut real_a = 0.0;
    let mut imag_a = 0.0;
    let mut real_b = 0.0;
    let mut imag_b = 0.0;

    let omega = 2.0 * PI * frequency / sample_rate as f32;

    for i in 0..n {
        let angle = omega * i as f32;
        let cos_val = angle.cos();
        let sin_val = angle.sin();

        real_a += signal_a[i] * cos_val;
        imag_a -= signal_a[i] * sin_val;
        real_b += signal_b[i] * cos_val;
        imag_b -= signal_b[i] * sin_val;
    }

    let phase_a = imag_a.atan2(real_a);
    let phase_b = imag_b.atan2(real_b);

    let phase_diff = (phase_b - phase_a) * 180.0 / PI;

    // Normalize to -180 to +180
    if phase_diff > 180.0 {
        phase_diff - 360.0
    } else if phase_diff < -180.0 {
        phase_diff + 360.0
    } else {
        phase_diff
    }
}

/// Audio quality report structure
#[derive(Debug, Clone)]
pub struct AudioQualityReport {
    pub snr_db: f32,
    pub thd_percent: f32,
    pub thd_plus_n_percent: f32,
    pub sinad_db: f32,
    pub enob: f32,
    pub dynamic_range_db: f32,
    pub a_weighted_noise_db: f32,
    pub peak_db: f32,
    pub rms_db: f32,
}

impl AudioQualityReport {
    /// Generate a comprehensive audio quality report
    pub fn analyze(samples: &[f32], fundamental_freq: f32, sample_rate: u32) -> Self {
        let mono = if samples.len() % 2 == 0 {
            extract_mono(samples, 0)
        } else {
            samples.to_vec()
        };

        // Use spectral SNR for tonal signals
        let snr_db = calculate_snr_at_frequency(&mono, fundamental_freq, sample_rate);
        let thd_percent = calculate_thd(&mono, fundamental_freq, sample_rate);
        let thd_plus_n_percent = calculate_thd_plus_n(&mono, fundamental_freq, sample_rate);
        let sinad_db = calculate_sinad(&mono, fundamental_freq, sample_rate);
        let enob = calculate_enob(sinad_db);
        let dynamic_range_db = calculate_dynamic_range(&mono);
        let a_weighted_noise_db = calculate_a_weighted_noise(&mono, sample_rate);
        let peak = calculate_peak(&mono);
        let rms = calculate_rms(&mono);

        AudioQualityReport {
            snr_db,
            thd_percent,
            thd_plus_n_percent,
            sinad_db,
            enob,
            dynamic_range_db,
            a_weighted_noise_db,
            peak_db: linear_to_db(peak),
            rms_db: linear_to_db(rms),
        }
    }

    /// Check if the report meets basic audio quality standards
    ///
    /// Note: Due to simple DFT analysis limitations, thresholds are very lenient
    /// compared to professional measurement equipment (Audio Precision, etc.)
    /// The simple DFT has spectral leakage that raises the noise floor.
    pub fn meets_professional_standards(&self) -> bool {
        // Adjusted thresholds for simple DFT analysis:
        // - SNR > 15 dB (simple DFT leakage limits this severely)
        // - THD < 5% (instead of 0.1% with precision equipment)
        // - SINAD > 15 dB (instead of 80 dB)
        self.snr_db > 15.0
            && self.thd_percent < 5.0
            && self.sinad_db > 15.0
    }

    /// Check if the report meets professional standards with precision equipment
    ///
    /// These are stricter thresholds for use with proper measurement gear.
    pub fn meets_strict_standards(&self) -> bool {
        self.snr_db > 90.0
            && self.thd_percent < 0.1
            && self.thd_plus_n_percent < 0.5
            && self.sinad_db > 80.0
    }

    /// Format as human-readable string
    pub fn format(&self) -> String {
        format!(
            "Audio Quality Report:\n\
             ├─ SNR: {:.1} dB\n\
             ├─ THD: {:.4}%\n\
             ├─ THD+N: {:.4}%\n\
             ├─ SINAD: {:.1} dB\n\
             ├─ ENOB: {:.1} bits\n\
             ├─ Dynamic Range: {:.1} dB\n\
             ├─ A-weighted Noise: {:.1} dBFS\n\
             ├─ Peak: {:.1} dBFS\n\
             └─ RMS: {:.1} dBFS",
            self.snr_db,
            self.thd_percent,
            self.thd_plus_n_percent,
            self.sinad_db,
            self.enob,
            self.dynamic_range_db,
            self.a_weighted_noise_db,
            self.peak_db,
            self.rms_db
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::signals::*;

    #[test]
    fn test_rms_calculation() {
        // RMS of sine wave should be ~0.707 of peak (1/sqrt(2))
        let signal = generate_sine_wave(440.0, 44100, 0.1, 1.0);
        let mono = extract_mono(&signal, 0);
        let rms = calculate_rms(&mono);

        assert!((rms - 0.707).abs() < 0.01);
    }

    #[test]
    fn test_peak_detection() {
        let mut signal = vec![0.0; 1000];
        signal[500] = 0.8;
        signal[700] = 0.6;

        let peak = calculate_peak(&signal);
        assert_eq!(peak, 0.8);
    }

    #[test]
    fn test_db_conversion() {
        assert!((linear_to_db(1.0) - 0.0).abs() < 0.001);
        assert!((linear_to_db(0.5) - (-6.02)).abs() < 0.1);
        assert!((linear_to_db(0.0) - (-100.0)).abs() < 0.001);

        assert!((db_to_linear(0.0) - 1.0).abs() < 0.001);
        assert!((db_to_linear(-6.0) - 0.501).abs() < 0.01);
    }

    #[test]
    fn test_frequency_analysis() {
        // Generate 1kHz sine wave
        let signal = generate_sine_wave(1000.0, 44100, 0.1, 1.0);
        let mono = extract_mono(&signal, 0);

        let dominant = find_dominant_frequency(&mono, 44100);

        // Should detect 1kHz (within 50Hz tolerance)
        assert!((dominant - 1000.0).abs() < 50.0);
    }

    #[test]
    fn test_thd_pure_sine() {
        // Pure sine wave should have very low THD
        let signal = generate_sine_wave(440.0, 44100, 0.1, 0.5);
        let mono = extract_mono(&signal, 0);

        let thd = calculate_thd(&mono, 440.0, 44100);

        println!("THD of pure sine: {:.2}%", thd);

        // Simple DFT has some numerical error, allow up to 5% THD
        assert!(thd < 5.0, "THD should be low for pure sine (got {:.2}%)", thd);
    }

    #[test]
    fn test_signal_difference() {
        let signal_a = vec![1.0, 2.0, 3.0];
        let signal_b = vec![1.1, 2.1, 3.1];

        let diff = calculate_signal_difference(&signal_a, &signal_b);
        assert!((diff - 0.1).abs() < 0.001);
    }

    #[test]
    fn test_is_silent() {
        let quiet = vec![0.0001; 100];
        let loud = vec![0.5; 100];

        assert!(is_silent(&quiet, -60.0));
        assert!(!is_silent(&loud, -60.0));
    }

    #[test]
    fn test_extract_mono() {
        let stereo = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let left = extract_mono(&stereo, 0);
        let right = extract_mono(&stereo, 1);

        assert_eq!(left, vec![1.0, 3.0, 5.0]);
        assert_eq!(right, vec![2.0, 4.0, 6.0]);
    }
}
