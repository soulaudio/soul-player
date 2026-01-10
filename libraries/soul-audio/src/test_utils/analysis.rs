//! Audio analysis tools for verification
//!
//! Provides FFT-based frequency analysis, THD calculation, peak detection,
//! and other metrics for verifying DSP algorithm correctness.

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
