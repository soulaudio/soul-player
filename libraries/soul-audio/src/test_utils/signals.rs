//! Test signal generation for audio testing
//!
//! Provides standard test signals used in audio DSP verification:
//! - Sine waves (single frequency)
//! - Sine sweeps (chirp signals)
//! - White noise
//! - Pink noise
//! - Square waves
//! - Impulses

use std::f32::consts::PI;

/// Generate a sine wave
///
/// # Arguments
/// * `frequency` - Frequency in Hz
/// * `sample_rate` - Sample rate in Hz
/// * `duration` - Duration in seconds
/// * `amplitude` - Peak amplitude (0.0 to 1.0)
///
/// # Returns
/// Stereo interleaved samples (L, R, L, R, ...)
pub fn generate_sine_wave(
    frequency: f32,
    sample_rate: u32,
    duration: f32,
    amplitude: f32,
) -> Vec<f32> {
    let num_samples = (sample_rate as f32 * duration) as usize;
    let mut samples = Vec::with_capacity(num_samples * 2); // Stereo

    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let sample = (2.0 * PI * frequency * t).sin() * amplitude;
        samples.push(sample); // Left
        samples.push(sample); // Right
    }

    samples
}

/// Generate a logarithmic sine sweep (chirp)
///
/// Useful for measuring frequency response across the audible spectrum.
///
/// # Arguments
/// * `start_freq` - Starting frequency in Hz
/// * `end_freq` - Ending frequency in Hz
/// * `sample_rate` - Sample rate in Hz
/// * `duration` - Duration in seconds
/// * `amplitude` - Peak amplitude (0.0 to 1.0)
pub fn generate_sine_sweep(
    start_freq: f32,
    end_freq: f32,
    sample_rate: u32,
    duration: f32,
    amplitude: f32,
) -> Vec<f32> {
    let num_samples = (sample_rate as f32 * duration) as usize;
    let mut samples = Vec::with_capacity(num_samples * 2);

    let k = (end_freq / start_freq).ln() / duration;

    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let _freq = start_freq * (k * t).exp(); // Frequency at time t (not used directly, but part of chirp calculation)
        let phase = 2.0 * PI * start_freq * ((k * t).exp() - 1.0) / k;
        let sample = phase.sin() * amplitude;
        samples.push(sample); // Left
        samples.push(sample); // Right
    }

    samples
}

/// Generate white noise
///
/// All frequencies have equal power.
///
/// # Arguments
/// * `sample_rate` - Sample rate in Hz
/// * `duration` - Duration in seconds
/// * `amplitude` - Peak amplitude (0.0 to 1.0)
pub fn generate_white_noise(sample_rate: u32, duration: f32, amplitude: f32) -> Vec<f32> {
    let num_samples = (sample_rate as f32 * duration) as usize;
    let mut samples = Vec::with_capacity(num_samples * 2);

    for _ in 0..num_samples {
        let sample = (rand::random::<f32>() * 2.0 - 1.0) * amplitude;
        samples.push(sample); // Left
        samples.push(sample); // Right
    }

    samples
}

/// Generate pink noise (1/f noise)
///
/// Power decreases by 3dB per octave. More natural sounding than white noise.
/// Uses Paul Kellett's refined method.
pub fn generate_pink_noise(sample_rate: u32, duration: f32, amplitude: f32) -> Vec<f32> {
    let num_samples = (sample_rate as f32 * duration) as usize;
    let mut samples = Vec::with_capacity(num_samples * 2);

    // State for pink noise generation (Paul Kellett's method)
    let mut b0 = 0.0;
    let mut b1 = 0.0;
    let mut b2 = 0.0;
    let mut b3 = 0.0;
    let mut b4 = 0.0;
    let mut b5 = 0.0;
    let mut b6 = 0.0;

    for _ in 0..num_samples {
        let white = rand::random::<f32>() * 2.0 - 1.0;

        b0 = 0.99886 * b0 + white * 0.0555179;
        b1 = 0.99332 * b1 + white * 0.0750759;
        b2 = 0.96900 * b2 + white * 0.1538520;
        b3 = 0.86650 * b3 + white * 0.3104856;
        b4 = 0.55000 * b4 + white * 0.5329522;
        b5 = -0.7616 * b5 - white * 0.0168980;

        let pink = b0 + b1 + b2 + b3 + b4 + b5 + b6 + white * 0.5362;
        b6 = white * 0.115926;

        let sample = (pink * 0.11) * amplitude; // Scale down
        samples.push(sample); // Left
        samples.push(sample); // Right
    }

    samples
}

/// Generate a square wave
///
/// Useful for testing transient response and slew rate.
///
/// # Arguments
/// * `frequency` - Frequency in Hz
/// * `sample_rate` - Sample rate in Hz
/// * `duration` - Duration in seconds
/// * `amplitude` - Peak amplitude (0.0 to 1.0)
pub fn generate_square_wave(
    frequency: f32,
    sample_rate: u32,
    duration: f32,
    amplitude: f32,
) -> Vec<f32> {
    let num_samples = (sample_rate as f32 * duration) as usize;
    let mut samples = Vec::with_capacity(num_samples * 2);

    let period = sample_rate as f32 / frequency;

    for i in 0..num_samples {
        let phase = (i as f32 % period) / period;
        let sample = if phase < 0.5 { amplitude } else { -amplitude };
        samples.push(sample); // Left
        samples.push(sample); // Right
    }

    samples
}

/// Generate an impulse (single sample spike)
///
/// Useful for measuring impulse response.
///
/// # Arguments
/// * `sample_rate` - Sample rate in Hz
/// * `duration` - Duration in seconds
/// * `amplitude` - Peak amplitude (0.0 to 1.0)
pub fn generate_impulse(sample_rate: u32, duration: f32, amplitude: f32) -> Vec<f32> {
    let num_samples = (sample_rate as f32 * duration) as usize;
    let mut samples = vec![0.0; num_samples * 2];

    // Place impulse at 10% of duration
    let impulse_position = (num_samples / 10) * 2; // Stereo
    if impulse_position < samples.len() {
        samples[impulse_position] = amplitude;
        samples[impulse_position + 1] = amplitude;
    }

    samples
}

/// Generate a signal with known dynamic range
///
/// Alternates between quiet and loud sections for testing compression/limiting.
///
/// # Arguments
/// * `sample_rate` - Sample rate in Hz
/// * `duration` - Duration in seconds
/// * `quiet_amplitude` - Amplitude during quiet sections
/// * `loud_amplitude` - Amplitude during loud sections
pub fn generate_dynamic_test_signal(
    sample_rate: u32,
    duration: f32,
    quiet_amplitude: f32,
    loud_amplitude: f32,
) -> Vec<f32> {
    let num_samples = (sample_rate as f32 * duration) as usize;
    let mut samples = Vec::with_capacity(num_samples * 2);

    let section_length = num_samples / 4; // 4 sections

    for i in 0..num_samples {
        let section = i / section_length;
        let amplitude = if section % 2 == 0 {
            quiet_amplitude
        } else {
            loud_amplitude
        };

        let t = i as f32 / sample_rate as f32;
        let sample = (2.0 * PI * 440.0 * t).sin() * amplitude; // 440 Hz sine
        samples.push(sample); // Left
        samples.push(sample); // Right
    }

    samples
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sine_wave_generation() {
        let signal = generate_sine_wave(440.0, 44100, 1.0, 1.0);

        // Should be stereo
        assert_eq!(signal.len(), 44100 * 2);

        // Check peak amplitude
        let max_amplitude = signal.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        assert!(max_amplitude > 0.99 && max_amplitude <= 1.0);
    }

    #[test]
    fn test_sine_sweep_generation() {
        let signal = generate_sine_sweep(20.0, 20000.0, 44100, 2.0, 0.5);

        // Should be stereo
        assert_eq!(signal.len(), 44100 * 2 * 2);

        // Check amplitude is scaled
        let max_amplitude = signal.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        assert!(max_amplitude > 0.45 && max_amplitude <= 0.5);
    }

    #[test]
    fn test_white_noise_generation() {
        let signal = generate_white_noise(44100, 0.1, 0.5);

        // Should have random values
        let first = signal[0];
        let last = signal[signal.len() - 1];
        assert_ne!(first, last);

        // Check amplitude bounds
        let max_amplitude = signal.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        assert!(max_amplitude <= 0.5);
    }

    #[test]
    fn test_square_wave_generation() {
        let signal = generate_square_wave(100.0, 44100, 0.1, 1.0);

        // Should alternate between +1 and -1
        let unique_values: Vec<_> = signal.iter().copied().collect();
        assert!(unique_values.contains(&1.0) || unique_values.contains(&-1.0));
    }

    #[test]
    fn test_impulse_generation() {
        let signal = generate_impulse(44100, 0.1, 1.0);

        // Most samples should be zero
        let non_zero_count = signal.iter().filter(|&&s| s.abs() > 0.0001).count();
        assert!(non_zero_count <= 4); // Should be just the impulse (stereo)

        // Should contain one impulse
        let max_amplitude = signal.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        assert!((max_amplitude - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_dynamic_signal_generation() {
        let signal = generate_dynamic_test_signal(44100, 1.0, 0.1, 0.9);

        // Should be stereo (2 channels)
        assert_eq!(signal.len(), 44100 * 2);

        // Extract mono for analysis (stereo is interleaved: L,R,L,R,...)
        let mono: Vec<f32> = signal.chunks_exact(2).map(|chunk| chunk[0]).collect();

        // Check first quarter (quiet: 0.1) vs second quarter (loud: 0.9)
        let quarter_len = mono.len() / 4;
        let first_quarter = &mono[0..quarter_len];
        let second_quarter = &mono[quarter_len..quarter_len * 2];

        let first_rms: f32 = (first_quarter.iter().map(|s| s * s).sum::<f32>() / first_quarter.len() as f32).sqrt();
        let second_rms: f32 = (second_quarter.iter().map(|s| s * s).sum::<f32>() / second_quarter.len() as f32).sqrt();

        // Second quarter should be much louder (0.9 vs 0.1 amplitude = 9x in RMS)
        assert!(second_rms > first_rms * 5.0);
    }
}
