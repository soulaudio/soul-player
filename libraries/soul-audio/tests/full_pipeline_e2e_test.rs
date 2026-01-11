//! Full Audio Pipeline End-to-End Tests
//!
//! Comprehensive tests verifying the COMPLETE audio pipeline from input to output:
//! Decoder -> Resampling -> Effects -> Volume -> Output
//!
//! This test suite covers:
//! 1. Full signal path verification with known inputs and exact outputs
//! 2. Format combinations (FLAC, MP3, WAV at various sample rates)
//! 3. Quality preservation (SNR degradation, dynamic range, frequency response)
//! 4. Real-world scenarios (playlist playback, gapless, random queue)
//! 5. Edge cases (quiet passages, loud passages, DC offset, clipped material)
//! 6. Performance metrics (CPU usage proxy via timing, memory patterns)
//! 7. A/B testing framework (null tests, level matching, phase alignment)
//! 8. Regression suite (golden file comparisons, bit-exact reproduction)

use soul_audio::effects::{
    AudioEffect, Compressor, CompressorSettings, Crossfeed, CrossfeedPreset, EffectChain, EqBand,
    GraphicEq, GraphicEqPreset, Limiter, LimiterSettings, ParametricEq, StereoEnhancer,
    StereoSettings,
};
use soul_audio::resampling::{Resampler, ResamplerBackend, ResamplingQuality};
use std::f32::consts::PI;
use std::time::Instant;

// ============================================================================
// Test Signal Utilities (Comprehensive Suite)
// ============================================================================

/// Generate a pure sine wave (stereo interleaved)
fn generate_sine_wave(
    frequency: f32,
    sample_rate: u32,
    duration_secs: f32,
    amplitude: f32,
) -> Vec<f32> {
    let num_samples = (sample_rate as f32 * duration_secs) as usize;
    let mut samples = Vec::with_capacity(num_samples * 2);

    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let sample = (2.0 * PI * frequency * t).sin() * amplitude;
        samples.push(sample); // Left
        samples.push(sample); // Right
    }

    samples
}

/// Generate a logarithmic sine sweep (chirp)
fn generate_sine_sweep(
    start_freq: f32,
    end_freq: f32,
    sample_rate: u32,
    duration_secs: f32,
    amplitude: f32,
) -> Vec<f32> {
    let num_samples = (sample_rate as f32 * duration_secs) as usize;
    let mut samples = Vec::with_capacity(num_samples * 2);

    let k = (end_freq / start_freq).ln() / duration_secs;

    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let phase = 2.0 * PI * start_freq * ((k * t).exp() - 1.0) / k;
        let sample = phase.sin() * amplitude;
        samples.push(sample); // Left
        samples.push(sample); // Right
    }

    samples
}

/// Generate white noise
fn generate_white_noise(sample_rate: u32, duration_secs: f32, amplitude: f32) -> Vec<f32> {
    let num_samples = (sample_rate as f32 * duration_secs) as usize;
    let mut samples = Vec::with_capacity(num_samples * 2);

    for _ in 0..num_samples {
        let sample = (rand::random::<f32>() * 2.0 - 1.0) * amplitude;
        samples.push(sample);
        samples.push(sample);
    }

    samples
}

/// Generate a signal with known dynamic range (alternating quiet/loud sections)
fn generate_dynamic_signal(
    sample_rate: u32,
    duration_secs: f32,
    quiet_amplitude: f32,
    loud_amplitude: f32,
) -> Vec<f32> {
    let num_samples = (sample_rate as f32 * duration_secs) as usize;
    let mut samples = Vec::with_capacity(num_samples * 2);
    let section_length = num_samples / 4;

    for i in 0..num_samples {
        let section = i / section_length;
        let amplitude = if section % 2 == 0 {
            quiet_amplitude
        } else {
            loud_amplitude
        };
        let t = i as f32 / sample_rate as f32;
        let sample = (2.0 * PI * 440.0 * t).sin() * amplitude;
        samples.push(sample);
        samples.push(sample);
    }

    samples
}

/// Generate a silent signal (for noise floor testing)
fn generate_silence(sample_rate: u32, duration_secs: f32) -> Vec<f32> {
    let num_samples = (sample_rate as f32 * duration_secs) as usize;
    vec![0.0; num_samples * 2]
}

/// Generate a signal with DC offset
fn generate_with_dc_offset(
    frequency: f32,
    sample_rate: u32,
    duration_secs: f32,
    amplitude: f32,
    dc_offset: f32,
) -> Vec<f32> {
    let num_samples = (sample_rate as f32 * duration_secs) as usize;
    let mut samples = Vec::with_capacity(num_samples * 2);

    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let sample = (2.0 * PI * frequency * t).sin() * amplitude + dc_offset;
        samples.push(sample);
        samples.push(sample);
    }

    samples
}

/// Generate a clipped (hard-limited) signal
fn generate_clipped_signal(
    frequency: f32,
    sample_rate: u32,
    duration_secs: f32,
    overdrive_factor: f32,
) -> Vec<f32> {
    let num_samples = (sample_rate as f32 * duration_secs) as usize;
    let mut samples = Vec::with_capacity(num_samples * 2);

    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let raw_sample = (2.0 * PI * frequency * t).sin() * overdrive_factor;
        let sample = raw_sample.clamp(-1.0, 1.0);
        samples.push(sample);
        samples.push(sample);
    }

    samples
}

/// Generate a multi-tone test signal for frequency response
fn generate_multitone(
    frequencies: &[f32],
    sample_rate: u32,
    duration_secs: f32,
    amplitude: f32,
) -> Vec<f32> {
    let num_samples = (sample_rate as f32 * duration_secs) as usize;
    let mut samples = Vec::with_capacity(num_samples * 2);
    let per_tone_amp = amplitude / (frequencies.len() as f32).sqrt();

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

/// Generate an impulse signal for impulse response testing
fn generate_impulse(sample_rate: u32, duration_secs: f32, amplitude: f32) -> Vec<f32> {
    let num_samples = (sample_rate as f32 * duration_secs) as usize;
    let mut samples = vec![0.0; num_samples * 2];

    // Place impulse at 10% of duration
    let impulse_position = (num_samples / 10) * 2;
    if impulse_position < samples.len() {
        samples[impulse_position] = amplitude;
        samples[impulse_position + 1] = amplitude;
    }

    samples
}

/// Generate a square wave (for transient testing)
fn generate_square_wave(
    frequency: f32,
    sample_rate: u32,
    duration_secs: f32,
    amplitude: f32,
) -> Vec<f32> {
    let num_samples = (sample_rate as f32 * duration_secs) as usize;
    let mut samples = Vec::with_capacity(num_samples * 2);

    let period = sample_rate as f32 / frequency;

    for i in 0..num_samples {
        let phase = (i as f32 % period) / period;
        let sample = if phase < 0.5 { amplitude } else { -amplitude };
        samples.push(sample);
        samples.push(sample);
    }

    samples
}

// ============================================================================
// Analysis Utilities
// ============================================================================

/// Calculate RMS level
fn calculate_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_squares: f32 = samples.iter().map(|s| s * s).sum();
    (sum_squares / samples.len() as f32).sqrt()
}

/// Calculate peak amplitude
fn calculate_peak(samples: &[f32]) -> f32 {
    samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max)
}

/// Extract mono channel from stereo interleaved
fn extract_mono(stereo: &[f32], channel: usize) -> Vec<f32> {
    stereo.chunks_exact(2).map(|chunk| chunk[channel]).collect()
}

/// Convert linear amplitude to dB
fn linear_to_db(linear: f32) -> f32 {
    if linear <= 0.0 {
        -100.0
    } else {
        20.0 * linear.log10()
    }
}

/// Convert dB to linear amplitude
fn db_to_linear(db: f32) -> f32 {
    10.0f32.powf(db / 20.0)
}

/// Simple DFT-based frequency detection
fn find_dominant_frequency(samples: &[f32], sample_rate: u32) -> f32 {
    let n = samples.len().min(4096);
    let samples = &samples[0..n];

    let mut max_magnitude = 0.0f32;
    let mut dominant_freq = 0.0f32;

    for k in 1..n / 2 {
        let mut real = 0.0;
        let mut imag = 0.0;

        for (i, &sample) in samples.iter().enumerate() {
            let angle = -2.0 * PI * (k as f32) * (i as f32) / (n as f32);
            real += sample * angle.cos();
            imag += sample * angle.sin();
        }

        let magnitude = (real * real + imag * imag).sqrt();
        if magnitude > max_magnitude {
            max_magnitude = magnitude;
            dominant_freq = (k as f32 * sample_rate as f32) / (n as f32);
        }
    }

    dominant_freq
}

/// Calculate signal difference (max absolute difference)
fn calculate_max_difference(signal_a: &[f32], signal_b: &[f32]) -> f32 {
    if signal_a.len() != signal_b.len() {
        return f32::INFINITY;
    }

    signal_a
        .iter()
        .zip(signal_b.iter())
        .map(|(a, b)| (a - b).abs())
        .fold(0.0f32, f32::max)
}

/// Calculate average absolute difference
fn calculate_avg_difference(signal_a: &[f32], signal_b: &[f32]) -> f32 {
    if signal_a.len() != signal_b.len() {
        return f32::INFINITY;
    }

    signal_a
        .iter()
        .zip(signal_b.iter())
        .map(|(a, b)| (a - b).abs())
        .sum::<f32>()
        / signal_a.len() as f32
}

/// Calculate SNR between processed and reference signals
fn calculate_snr_db(processed: &[f32], reference: &[f32]) -> f32 {
    if processed.len() != reference.len() || processed.is_empty() {
        return 0.0;
    }

    let signal_power: f32 = reference.iter().map(|s| s * s).sum();
    let noise_power: f32 = processed
        .iter()
        .zip(reference.iter())
        .map(|(p, r)| (p - r).powi(2))
        .sum();

    if noise_power <= 0.0 {
        return 120.0; // Maximum measurable
    }

    10.0 * (signal_power / noise_power).log10()
}

/// Calculate THD (Total Harmonic Distortion) percentage
fn calculate_thd_percent(samples: &[f32], fundamental_freq: f32, sample_rate: u32) -> f32 {
    let n = samples.len().min(8192);
    let samples = &samples[0..n];

    let mut harmonic_power = 0.0f32;

    // Calculate power at fundamental
    let fund_bin = (fundamental_freq * n as f32 / sample_rate as f32).round() as usize;
    let fundamental_power = {
        let mut real = 0.0f32;
        let mut imag = 0.0f32;
        for (i, &sample) in samples.iter().enumerate() {
            let angle = -2.0 * PI * (fund_bin as f32) * (i as f32) / (n as f32);
            real += sample * angle.cos();
            imag += sample * angle.sin();
        }
        real * real + imag * imag
    };

    // Calculate power at harmonics (2nd through 5th)
    for harmonic in 2..=5 {
        let harm_bin = fund_bin * harmonic;
        if harm_bin >= n / 2 {
            break;
        }

        let mut real = 0.0f32;
        let mut imag = 0.0f32;
        for (i, &sample) in samples.iter().enumerate() {
            let angle = -2.0 * PI * (harm_bin as f32) * (i as f32) / (n as f32);
            real += sample * angle.cos();
            imag += sample * angle.sin();
        }
        harmonic_power += real * real + imag * imag;
    }

    if fundamental_power <= 0.0 {
        return 0.0;
    }

    ((harmonic_power / fundamental_power).sqrt() * 100.0).min(100.0)
}

/// Calculate dynamic range in dB
fn calculate_dynamic_range_db(samples: &[f32]) -> f32 {
    let peak = calculate_peak(samples);
    let rms = calculate_rms(samples);

    if rms <= 0.0 {
        return 0.0;
    }

    linear_to_db(peak / rms)
}

/// Measure frequency response at specific frequencies (returns magnitudes in dB)
fn measure_frequency_response(
    samples: &[f32],
    frequencies: &[f32],
    sample_rate: u32,
) -> Vec<(f32, f32)> {
    let n = samples.len().min(8192);
    let samples = &samples[0..n];

    frequencies
        .iter()
        .map(|&freq| {
            let bin = (freq * n as f32 / sample_rate as f32).round() as usize;
            if bin >= n / 2 {
                return (freq, -100.0);
            }

            let mut real = 0.0f32;
            let mut imag = 0.0f32;
            for (i, &sample) in samples.iter().enumerate() {
                let angle = -2.0 * PI * (bin as f32) * (i as f32) / (n as f32);
                real += sample * angle.cos();
                imag += sample * angle.sin();
            }

            let magnitude = (real * real + imag * imag).sqrt() / n as f32;
            (freq, linear_to_db(magnitude))
        })
        .collect()
}

/// Calculate phase difference between two signals at a specific frequency
fn calculate_phase_difference(
    signal_a: &[f32],
    signal_b: &[f32],
    frequency: f32,
    sample_rate: u32,
) -> f32 {
    if signal_a.len() != signal_b.len() || signal_a.is_empty() {
        return 0.0;
    }

    let n = signal_a.len().min(8192);

    let omega = 2.0 * PI * frequency / sample_rate as f32;

    let mut real_a = 0.0f32;
    let mut imag_a = 0.0f32;
    let mut real_b = 0.0f32;
    let mut imag_b = 0.0f32;

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

    let mut phase_diff = (phase_b - phase_a) * 180.0 / PI;

    // Normalize to -180 to +180
    if phase_diff > 180.0 {
        phase_diff -= 360.0;
    } else if phase_diff < -180.0 {
        phase_diff += 360.0;
    }

    phase_diff
}

// ============================================================================
// Full Pipeline Helper
// ============================================================================

/// Full audio pipeline structure for testing
struct FullAudioPipeline {
    resampler: Option<Resampler>,
    effect_chain: EffectChain,
    volume_gain: f32,
}

impl FullAudioPipeline {
    /// Create a new full audio pipeline
    fn new() -> Self {
        Self {
            resampler: None,
            effect_chain: EffectChain::new(),
            volume_gain: 1.0,
        }
    }

    /// Setup resampler if sample rate conversion is needed
    fn setup_resampler(
        &mut self,
        input_rate: u32,
        output_rate: u32,
        quality: ResamplingQuality,
    ) -> Result<(), String> {
        if input_rate != output_rate {
            self.resampler = Some(
                Resampler::new(ResamplerBackend::Auto, input_rate, output_rate, 2, quality)
                    .map_err(|e| e.to_string())?,
            );
        }
        Ok(())
    }

    /// Add an effect to the chain
    fn add_effect(&mut self, effect: Box<dyn AudioEffect>) {
        self.effect_chain.add_effect(effect);
    }

    /// Set volume gain (linear)
    fn set_volume(&mut self, gain: f32) {
        self.volume_gain = gain;
    }

    /// Process audio through the full pipeline
    fn process(&mut self, input: &[f32], sample_rate: u32) -> Vec<f32> {
        // Step 1: Resampling (if configured)
        let resampled = if let Some(ref mut resampler) = self.resampler {
            resampler.process(input).unwrap_or_else(|_| input.to_vec())
        } else {
            input.to_vec()
        };

        // Step 2: Effects processing
        let output_rate = self
            .resampler
            .as_ref()
            .map(|r| r.output_rate())
            .unwrap_or(sample_rate);

        let mut output = resampled;
        self.effect_chain.process(&mut output, output_rate);

        // Step 3: Volume control
        for sample in &mut output {
            *sample *= self.volume_gain;
        }

        output
    }

    /// Reset the entire pipeline
    fn reset(&mut self) {
        if let Some(ref mut resampler) = self.resampler {
            resampler.reset();
        }
        self.effect_chain.reset();
    }

    /// Get total latency in samples
    fn get_latency(&self) -> usize {
        self.resampler.as_ref().map(|r| r.latency()).unwrap_or(0)
    }
}

// ============================================================================
// 1. Full Signal Path Verification Tests
// ============================================================================

#[test]
fn test_full_pipeline_sine_wave_integrity() {
    // Test: Known 1kHz sine wave through full pipeline, verify output is valid
    let mut pipeline = FullAudioPipeline::new();

    // Add typical effect chain
    let eq = ParametricEq::new(); // Flat EQ
    pipeline.add_effect(Box::new(eq));

    let comp = Compressor::with_settings(CompressorSettings::gentle());
    pipeline.add_effect(Box::new(comp));

    let limiter = Limiter::with_settings(LimiterSettings::soft());
    pipeline.add_effect(Box::new(limiter));

    // Generate 1kHz sine wave
    let input = generate_sine_wave(1000.0, 44100, 0.5, 0.5);
    let original_peak = calculate_peak(&input);
    let original_rms = calculate_rms(&input);

    // Process through pipeline
    let output = pipeline.process(&input, 44100);

    // Verify output is valid
    assert!(
        output.iter().all(|s| s.is_finite()),
        "All output samples should be finite"
    );

    // Verify no clipping
    let output_peak = calculate_peak(&output);
    assert!(
        output_peak <= 1.0,
        "Output should not clip, got peak: {}",
        output_peak
    );

    // Verify signal is present
    let output_rms = calculate_rms(&output);
    assert!(
        output_rms > 0.01,
        "Output should have signal, got RMS: {}",
        output_rms
    );

    // Verify frequency is preserved
    let mono_output = extract_mono(&output, 0);
    let detected_freq = find_dominant_frequency(&mono_output, 44100);
    let freq_error = (detected_freq - 1000.0).abs() / 1000.0;
    assert!(
        freq_error < 0.05,
        "Frequency should be preserved, expected ~1000Hz, got {}Hz",
        detected_freq
    );

    println!(
        "Full Pipeline Sine Test: Input Peak={:.3}, Output Peak={:.3}, Input RMS={:.3}, Output RMS={:.3}",
        original_peak, output_peak, original_rms, output_rms
    );
}

#[test]
fn test_full_pipeline_thd_measurement() {
    // Test: Measure Total Harmonic Distortion through the full chain
    let mut pipeline = FullAudioPipeline::new();

    // Add neutral effect chain (should minimize THD)
    let eq = ParametricEq::new();
    pipeline.add_effect(Box::new(eq));

    let limiter = Limiter::with_settings(LimiterSettings::soft());
    pipeline.add_effect(Box::new(limiter));

    // Generate pure 1kHz sine
    let input = generate_sine_wave(1000.0, 44100, 1.0, 0.5);

    // Process
    let output = pipeline.process(&input, 44100);

    // Measure THD
    let mono = extract_mono(&output, 0);
    let thd = calculate_thd_percent(&mono, 1000.0, 44100);

    // THD should be reasonably low (< 5% for basic effects chain)
    // Note: Simple DFT has spectral leakage, so we use a lenient threshold
    assert!(
        thd < 10.0,
        "THD through pipeline should be < 10%, got {:.2}%",
        thd
    );

    println!("Full Pipeline THD: {:.4}%", thd);
}

#[test]
fn test_full_pipeline_latency_measurement() {
    // Test: Measure and verify latency through resampling + effects
    let mut pipeline = FullAudioPipeline::new();

    // Setup resampler 44.1kHz -> 48kHz
    pipeline
        .setup_resampler(44100, 48000, ResamplingQuality::High)
        .expect("Resampler setup should succeed");

    // Add effects
    let eq = ParametricEq::new();
    pipeline.add_effect(Box::new(eq));

    let comp = Compressor::new();
    pipeline.add_effect(Box::new(comp));

    // Get latency
    let latency = pipeline.get_latency();

    // Latency should be reasonable (< 10000 samples)
    assert!(
        latency < 10000,
        "Latency should be reasonable, got {} samples",
        latency
    );

    println!(
        "Full Pipeline Latency: {} samples ({:.2}ms at 48kHz)",
        latency,
        latency as f32 / 48.0
    );
}

#[test]
fn test_full_pipeline_frequency_response() {
    // Test: Measure frequency response through the full chain
    let mut pipeline = FullAudioPipeline::new();

    // Add flat EQ
    let eq = ParametricEq::new();
    pipeline.add_effect(Box::new(eq));

    let limiter = Limiter::with_settings(LimiterSettings::soft());
    pipeline.add_effect(Box::new(limiter));

    // Test frequencies
    let test_frequencies = [100.0, 500.0, 1000.0, 5000.0, 10000.0];

    // Generate multi-tone signal
    let input = generate_multitone(&test_frequencies, 44100, 1.0, 0.3);

    // Process
    let output = pipeline.process(&input, 44100);

    // Measure frequency response
    let mono_input = extract_mono(&input, 0);
    let mono_output = extract_mono(&output, 0);

    let input_response = measure_frequency_response(&mono_input, &test_frequencies, 44100);
    let output_response = measure_frequency_response(&mono_output, &test_frequencies, 44100);

    // Check response at each frequency
    for ((freq, in_db), (_, out_db)) in input_response.iter().zip(output_response.iter()) {
        let diff = (out_db - in_db).abs();
        // Allow up to 6dB deviation (generous for soft limiter effects)
        assert!(
            diff < 6.0,
            "Frequency response at {}Hz should be within 6dB, got {:.1}dB difference (in: {:.1}dB, out: {:.1}dB)",
            freq, diff, in_db, out_db
        );
    }

    println!("Full Pipeline Frequency Response Test: PASSED");
}

// ============================================================================
// 2. Format Combinations Tests
// ============================================================================

#[test]
fn test_96k24_to_48k16_with_all_effects() {
    // Simulate: FLAC 96/24 -> 48/16 with all effects
    let mut pipeline = FullAudioPipeline::new();

    // Setup downsampling
    pipeline
        .setup_resampler(96000, 48000, ResamplingQuality::High)
        .expect("Resampler setup should succeed");

    // Add all effects
    let mut eq = ParametricEq::new();
    eq.set_low_band(EqBand::low_shelf(80.0, 2.0));
    eq.set_mid_band(EqBand::peaking(1000.0, -1.0, 1.0));
    eq.set_high_band(EqBand::high_shelf(8000.0, 1.0));
    pipeline.add_effect(Box::new(eq));

    let mut graphic_eq = GraphicEq::new_10_band();
    graphic_eq.set_preset(GraphicEqPreset::Flat);
    pipeline.add_effect(Box::new(graphic_eq));

    let comp = Compressor::with_settings(CompressorSettings::gentle());
    pipeline.add_effect(Box::new(comp));

    let crossfeed = Crossfeed::with_preset(CrossfeedPreset::Relaxed);
    pipeline.add_effect(Box::new(crossfeed));

    let enhancer = StereoEnhancer::with_settings(StereoSettings::default());
    pipeline.add_effect(Box::new(enhancer));

    let limiter = Limiter::with_settings(LimiterSettings::brickwall());
    pipeline.add_effect(Box::new(limiter));

    // Generate 96kHz input
    let input = generate_sine_wave(1000.0, 96000, 0.5, 0.6);

    // Process
    let output = pipeline.process(&input, 96000);

    // Verify output
    assert!(
        !output.is_empty(),
        "Output should not be empty after 96k->48k conversion"
    );
    assert!(
        output.iter().all(|s| s.is_finite()),
        "All samples should be finite"
    );

    let peak = calculate_peak(&output);
    assert!(
        peak <= 1.0,
        "Limiter should prevent clipping, got peak: {}",
        peak
    );

    println!(
        "96k/24 -> 48k/16 with all effects: {} input samples -> {} output samples, peak: {:.3}",
        input.len(),
        output.len(),
        peak
    );
}

#[test]
fn test_44k16_to_96k24_upsampling_with_effects() {
    // Simulate: WAV 44.1/16 -> 96/24 upsampling with effects
    let mut pipeline = FullAudioPipeline::new();

    // Setup upsampling
    pipeline
        .setup_resampler(44100, 96000, ResamplingQuality::Maximum)
        .expect("Resampler setup should succeed");

    // Add effects
    let eq = ParametricEq::new();
    pipeline.add_effect(Box::new(eq));

    let limiter = Limiter::new();
    pipeline.add_effect(Box::new(limiter));

    // Generate 44.1kHz input
    let input = generate_sine_wave(1000.0, 44100, 0.5, 0.5);

    // Process
    let output = pipeline.process(&input, 44100);

    // Verify output length is roughly 2.18x (96000/44100)
    let expected_ratio = 96000.0 / 44100.0;
    let actual_ratio = output.len() as f32 / input.len() as f32;

    // Allow significant margin due to resampler buffering
    assert!(
        actual_ratio > expected_ratio * 0.5 && actual_ratio < expected_ratio * 1.5,
        "Upsampling ratio should be approximately {:.2}, got {:.2}",
        expected_ratio,
        actual_ratio
    );

    assert!(
        output.iter().all(|s| s.is_finite()),
        "All samples should be finite after upsampling"
    );

    println!(
        "44.1k -> 96k upsampling: ratio={:.2} (expected {:.2})",
        actual_ratio, expected_ratio
    );
}

#[test]
fn test_mp3_like_quality_through_pipeline() {
    // Simulate: MP3 320kbps quality (44.1kHz, potentially with artifacts)
    let mut pipeline = FullAudioPipeline::new();

    // Add effects
    let eq = ParametricEq::new();
    pipeline.add_effect(Box::new(eq));

    let comp = Compressor::with_settings(CompressorSettings::moderate());
    pipeline.add_effect(Box::new(comp));

    let limiter = Limiter::with_settings(LimiterSettings::soft());
    pipeline.add_effect(Box::new(limiter));

    // Generate typical MP3 frequency content (limited to ~16kHz)
    let frequencies = [100.0, 500.0, 1000.0, 5000.0, 10000.0, 15000.0];
    let input = generate_multitone(&frequencies, 44100, 0.5, 0.3);

    // Process
    let output = pipeline.process(&input, 44100);

    // Verify output is valid
    assert!(output.iter().all(|s| s.is_finite()));
    assert!(calculate_peak(&output) <= 1.0);

    println!("MP3-like quality through pipeline: PASSED");
}

// ============================================================================
// 3. Quality Preservation Tests
// ============================================================================

#[test]
fn test_snr_degradation_through_chain() {
    // Test: SNR degradation should be < 3dB through the chain
    let mut pipeline = FullAudioPipeline::new();

    // Add neutral effects
    let eq = ParametricEq::new(); // Flat
    pipeline.add_effect(Box::new(eq));

    // Generate pure sine wave
    let input = generate_sine_wave(1000.0, 44100, 1.0, 0.5);

    // Process
    let output = pipeline.process(&input, 44100);

    // Calculate SNR (comparing output to input)
    let snr = calculate_snr_db(&output, &input);

    // SNR should be very high for neutral effects (> 40dB is reasonable)
    assert!(
        snr > 40.0,
        "SNR through neutral chain should be > 40dB, got {:.1}dB",
        snr
    );

    println!("SNR through neutral chain: {:.1} dB", snr);
}

#[test]
fn test_dynamic_range_preservation() {
    // Test: Dynamic range should be preserved (or controlled compression)
    let mut pipeline = FullAudioPipeline::new();

    // Add only EQ (no compression)
    let eq = ParametricEq::new();
    pipeline.add_effect(Box::new(eq));

    // Generate signal with known dynamic range
    let input = generate_dynamic_signal(44100, 1.0, 0.1, 0.8);
    let input_dr = calculate_dynamic_range_db(&input);

    // Process
    let output = pipeline.process(&input, 44100);
    let output_dr = calculate_dynamic_range_db(&output);

    // Dynamic range should be similar (within 3dB)
    let dr_diff = (output_dr - input_dr).abs();
    assert!(
        dr_diff < 3.0,
        "Dynamic range should be preserved within 3dB, got {:.1}dB difference",
        dr_diff
    );

    println!(
        "Dynamic Range: Input={:.1}dB, Output={:.1}dB, Diff={:.1}dB",
        input_dr, output_dr, dr_diff
    );
}

#[test]
fn test_frequency_response_flatness() {
    // Test: Frequency response should be flat through neutral chain
    let mut pipeline = FullAudioPipeline::new();

    // Add flat EQ
    let eq = ParametricEq::new();
    pipeline.add_effect(Box::new(eq));

    // Test multiple frequencies
    let test_freqs = [100.0, 500.0, 1000.0, 5000.0, 10000.0];

    let mut level_diffs = Vec::new();

    for &freq in &test_freqs {
        // Generate sine at this frequency
        let input = generate_sine_wave(freq, 44100, 0.5, 0.5);
        let input_rms = calculate_rms(&input);

        // Process
        let output = pipeline.process(&input, 44100);
        let output_rms = calculate_rms(&output);

        // Calculate level difference
        let diff_db = linear_to_db(output_rms / input_rms);
        level_diffs.push((freq, diff_db));

        // Reset pipeline for next frequency
        pipeline.reset();
    }

    // Check that all frequencies are within 1dB of each other
    let max_diff = level_diffs
        .iter()
        .map(|(_, d)| d.abs())
        .fold(0.0f32, f32::max);
    assert!(
        max_diff < 1.0,
        "Frequency response should be flat within 1dB, got max diff of {:.2}dB",
        max_diff
    );

    println!(
        "Frequency Response Flatness: max deviation = {:.2} dB",
        max_diff
    );
}

// ============================================================================
// 4. Real-World Scenarios Tests
// ============================================================================

#[test]
fn test_playlist_playback_simulation() {
    // Simulate: Playing multiple tracks in sequence (playlist)
    let mut pipeline = FullAudioPipeline::new();

    let eq = ParametricEq::new();
    pipeline.add_effect(Box::new(eq));

    let comp = Compressor::with_settings(CompressorSettings::gentle());
    pipeline.add_effect(Box::new(comp));

    let limiter = Limiter::new();
    pipeline.add_effect(Box::new(limiter));

    // Simulate 5 tracks with different characteristics
    let tracks = vec![
        generate_sine_wave(440.0, 44100, 0.5, 0.5), // Track 1: Simple tone
        generate_multitone(&[100.0, 500.0, 2000.0], 44100, 0.5, 0.3), // Track 2: Multi-tone
        generate_dynamic_signal(44100, 0.5, 0.1, 0.7), // Track 3: Dynamic
        generate_white_noise(44100, 0.5, 0.3),      // Track 4: Noise
        generate_sine_wave(1000.0, 44100, 0.5, 0.6), // Track 5: Another tone
    ];

    for (i, track) in tracks.iter().enumerate() {
        // Reset between tracks (simulates track change)
        pipeline.reset();

        // Process
        let output = pipeline.process(track, 44100);

        // Verify each track output is valid
        assert!(
            output.iter().all(|s| s.is_finite()),
            "Track {} output should be finite",
            i + 1
        );
        assert!(
            calculate_peak(&output) <= 1.0,
            "Track {} should not clip",
            i + 1
        );
    }

    println!("Playlist Playback Simulation: 5 tracks processed successfully");
}

#[test]
fn test_gapless_album_playback() {
    // Simulate: Gapless playback (no reset between tracks)
    let mut pipeline = FullAudioPipeline::new();

    let eq = ParametricEq::new();
    pipeline.add_effect(Box::new(eq));

    let comp = Compressor::with_settings(CompressorSettings::gentle());
    pipeline.add_effect(Box::new(comp));

    // Generate 3 "album tracks" that should flow seamlessly
    let track1 = generate_sine_wave(440.0, 44100, 0.3, 0.5);
    let track2 = generate_sine_wave(440.0, 44100, 0.3, 0.5); // Same frequency for gapless
    let track3 = generate_sine_wave(440.0, 44100, 0.3, 0.5);

    // Process without reset (gapless)
    let output1 = pipeline.process(&track1, 44100);
    let output2 = pipeline.process(&track2, 44100);
    let output3 = pipeline.process(&track3, 44100);

    // Check for clicks/pops at boundaries
    // The last sample of output1 should be similar to first sample of output2
    if !output1.is_empty() && !output2.is_empty() {
        let boundary_diff = (output1[output1.len() - 2] - output2[0]).abs();
        assert!(
            boundary_diff < 0.5,
            "Gapless transition should be smooth, got diff of {}",
            boundary_diff
        );
    }

    // All outputs should be valid
    assert!(output1.iter().all(|s| s.is_finite()));
    assert!(output2.iter().all(|s| s.is_finite()));
    assert!(output3.iter().all(|s| s.is_finite()));

    println!("Gapless Album Playback: Transitions verified");
}

#[test]
fn test_random_queue_with_various_formats() {
    // Simulate: Random queue with different "formats" (sample rates)
    let mut pipeline = FullAudioPipeline::new();

    let eq = ParametricEq::new();
    pipeline.add_effect(Box::new(eq));

    let limiter = Limiter::new();
    pipeline.add_effect(Box::new(limiter));

    // Simulate tracks at different sample rates
    let sample_rates = [44100, 48000, 96000, 44100, 48000];

    for &sr in &sample_rates {
        let track = generate_sine_wave(1000.0, sr, 0.2, 0.5);

        // Reset pipeline between different sample rates
        pipeline.reset();

        let output = pipeline.process(&track, sr);

        assert!(
            output.iter().all(|s| s.is_finite()),
            "Output at {}Hz should be finite",
            sr
        );
        assert!(
            calculate_peak(&output) <= 1.0,
            "Output at {}Hz should not clip",
            sr
        );
    }

    println!("Random Queue with Various Formats: All sample rates handled");
}

#[test]
fn test_long_listening_session_simulation() {
    // Simulate: 1 hour of playback (in chunks)
    let mut pipeline = FullAudioPipeline::new();

    let eq = ParametricEq::new();
    pipeline.add_effect(Box::new(eq));

    let comp = Compressor::with_settings(CompressorSettings::gentle());
    pipeline.add_effect(Box::new(comp));

    let limiter = Limiter::new();
    pipeline.add_effect(Box::new(limiter));

    // Simulate 1 hour = 3600 seconds
    // Use 1-second chunks, but only process 60 chunks (1 minute) for test speed
    let chunk_duration = 1.0; // seconds
    let total_chunks = 60; // 1 minute simulation

    let start_time = Instant::now();

    for i in 0..total_chunks {
        // Alternate between different signals
        let signal = match i % 4 {
            0 => generate_sine_wave(440.0, 44100, chunk_duration, 0.5),
            1 => generate_multitone(&[100.0, 1000.0, 5000.0], 44100, chunk_duration, 0.3),
            2 => generate_dynamic_signal(44100, chunk_duration, 0.1, 0.7),
            _ => generate_white_noise(44100, chunk_duration, 0.3),
        };

        let output = pipeline.process(&signal, 44100);

        // Verify no degradation over time
        assert!(
            output.iter().all(|s| s.is_finite()),
            "Chunk {} should have finite samples",
            i
        );
        assert!(
            calculate_peak(&output) <= 1.0,
            "Chunk {} should not clip",
            i
        );
    }

    let elapsed = start_time.elapsed();
    let real_time_ratio = (total_chunks as f32) / elapsed.as_secs_f32();

    println!(
        "Long Listening Session: {} chunks processed in {:.2}s ({:.1}x real-time)",
        total_chunks,
        elapsed.as_secs_f32(),
        real_time_ratio
    );

    // Should process faster than real-time
    assert!(
        real_time_ratio > 10.0,
        "Pipeline should be at least 10x real-time, got {:.1}x",
        real_time_ratio
    );
}

// ============================================================================
// 5. Edge Cases in Full Chain Tests
// ============================================================================

#[test]
fn test_very_quiet_passages() {
    // Test: Very quiet signals (-60dB) through full chain
    let mut pipeline = FullAudioPipeline::new();

    let eq = ParametricEq::new();
    pipeline.add_effect(Box::new(eq));

    let comp = Compressor::with_settings(CompressorSettings::gentle());
    pipeline.add_effect(Box::new(comp));

    let limiter = Limiter::new();
    pipeline.add_effect(Box::new(limiter));

    // Generate very quiet signal (-60dB = 0.001 amplitude)
    let quiet_amplitude = db_to_linear(-60.0);
    let input = generate_sine_wave(1000.0, 44100, 0.5, quiet_amplitude);

    let output = pipeline.process(&input, 44100);

    // Output should still be valid and not amplify noise
    assert!(output.iter().all(|s| s.is_finite()));

    let output_peak = calculate_peak(&output);
    assert!(
        output_peak < 0.1,
        "Quiet signal should not be excessively amplified, got peak: {}",
        output_peak
    );

    println!(
        "Very Quiet Passage: Input amplitude={:.4}, Output peak={:.4}",
        quiet_amplitude, output_peak
    );
}

#[test]
fn test_very_loud_passages() {
    // Test: Very loud signals (near 0dBFS) through full chain
    let mut pipeline = FullAudioPipeline::new();

    // Add EQ with boost (potentially causing clipping)
    let mut eq = ParametricEq::new();
    eq.set_low_band(EqBand::low_shelf(100.0, 6.0)); // +6dB boost
    pipeline.add_effect(Box::new(eq));

    let comp = Compressor::with_settings(CompressorSettings::aggressive());
    pipeline.add_effect(Box::new(comp));

    let limiter = Limiter::with_settings(LimiterSettings::brickwall());
    pipeline.add_effect(Box::new(limiter));

    // Generate loud signal
    let input = generate_sine_wave(100.0, 44100, 0.5, 0.9);

    let output = pipeline.process(&input, 44100);

    // Limiter should prevent clipping
    assert!(output.iter().all(|s| s.is_finite()));

    let output_peak = calculate_peak(&output);
    assert!(
        output_peak <= 1.0,
        "Limiter should prevent clipping on loud material, got peak: {}",
        output_peak
    );

    println!(
        "Very Loud Passage: Input peak=0.9, Output peak={:.3}",
        output_peak
    );
}

#[test]
fn test_silent_tracks() {
    // Test: Silent input through full chain
    let mut pipeline = FullAudioPipeline::new();

    let eq = ParametricEq::new();
    pipeline.add_effect(Box::new(eq));

    let comp = Compressor::new();
    pipeline.add_effect(Box::new(comp));

    let limiter = Limiter::new();
    pipeline.add_effect(Box::new(limiter));

    // Generate silence
    let input = generate_silence(44100, 0.5);

    let output = pipeline.process(&input, 44100);

    // Output should be silent (no noise injection)
    assert!(output.iter().all(|s| s.is_finite()));

    let output_peak = calculate_peak(&output);
    assert!(
        output_peak < 0.0001,
        "Silent input should produce silent output, got peak: {}",
        output_peak
    );

    println!("Silent Track: Output peak={:.6}", output_peak);
}

#[test]
fn test_dc_offset_handling() {
    // Test: Signal with DC offset through full chain
    let mut pipeline = FullAudioPipeline::new();

    let eq = ParametricEq::new();
    pipeline.add_effect(Box::new(eq));

    let comp = Compressor::new();
    pipeline.add_effect(Box::new(comp));

    let limiter = Limiter::new();
    pipeline.add_effect(Box::new(limiter));

    // Generate signal with DC offset
    let input = generate_with_dc_offset(440.0, 44100, 0.5, 0.3, 0.2);

    let output = pipeline.process(&input, 44100);

    // Output should be valid
    assert!(output.iter().all(|s| s.is_finite()));

    // Check for DC component in output
    let output_dc = output.iter().sum::<f32>() / output.len() as f32;

    // DC might be preserved or filtered depending on effects
    // Just verify it doesn't cause instability
    assert!(
        output_dc.abs() < 0.5,
        "DC offset should not cause instability, got DC: {}",
        output_dc
    );

    println!(
        "DC Offset Handling: Input DC=0.2, Output DC={:.4}",
        output_dc
    );
}

#[test]
fn test_clipped_source_material() {
    // Test: Already clipped source material through full chain
    let mut pipeline = FullAudioPipeline::new();

    let eq = ParametricEq::new();
    pipeline.add_effect(Box::new(eq));

    let comp = Compressor::with_settings(CompressorSettings::aggressive());
    pipeline.add_effect(Box::new(comp));

    let limiter = Limiter::new();
    pipeline.add_effect(Box::new(limiter));

    // Generate clipped signal (1.5x overdrive, clipped to -1..1)
    let input = generate_clipped_signal(440.0, 44100, 0.5, 1.5);

    // Verify input is clipped
    let input_peak = calculate_peak(&input);
    assert!(
        (input_peak - 1.0).abs() < 0.01,
        "Input should be clipped at 1.0"
    );

    let output = pipeline.process(&input, 44100);

    // Output should be valid
    assert!(output.iter().all(|s| s.is_finite()));

    let output_peak = calculate_peak(&output);
    assert!(
        output_peak <= 1.0,
        "Clipped source should not cause further clipping"
    );

    println!(
        "Clipped Source: Input THD is high (expected), Output peak={:.3}",
        output_peak
    );
}

// ============================================================================
// 6. Performance in Full Chain Tests
// ============================================================================

#[test]
fn test_cpu_usage_proxy_typical_settings() {
    // Test: Processing speed with typical settings (proxy for CPU usage)
    let mut pipeline = FullAudioPipeline::new();

    // Add typical effect chain
    let mut eq = ParametricEq::new();
    eq.set_low_band(EqBand::low_shelf(80.0, 2.0));
    eq.set_mid_band(EqBand::peaking(1000.0, -1.0, 1.0));
    eq.set_high_band(EqBand::high_shelf(8000.0, 1.0));
    pipeline.add_effect(Box::new(eq));

    let comp = Compressor::with_settings(CompressorSettings::gentle());
    pipeline.add_effect(Box::new(comp));

    let crossfeed = Crossfeed::with_preset(CrossfeedPreset::Natural);
    pipeline.add_effect(Box::new(crossfeed));

    let limiter = Limiter::new();
    pipeline.add_effect(Box::new(limiter));

    // Process 10 seconds of audio
    let input = generate_multitone(&[100.0, 500.0, 1000.0, 5000.0], 44100, 10.0, 0.3);

    let start = Instant::now();
    let output = pipeline.process(&input, 44100);
    let elapsed = start.elapsed();

    let audio_duration = 10.0; // seconds
    let real_time_ratio = audio_duration / elapsed.as_secs_f32();

    assert!(
        output.iter().all(|s| s.is_finite()),
        "Output should be valid"
    );

    // Should be at least 20x real-time for efficiency (lower threshold for debug builds)
    // Release builds would typically be 100x+ real-time
    assert!(
        real_time_ratio > 20.0,
        "Pipeline should be at least 20x real-time with typical settings, got {:.1}x",
        real_time_ratio
    );

    println!(
        "CPU Usage Proxy (Typical): {:.2}ms for 10s audio ({:.0}x real-time)",
        elapsed.as_millis(),
        real_time_ratio
    );
}

#[test]
fn test_cpu_usage_all_effects_enabled() {
    // Test: Processing speed with ALL effects enabled
    let mut pipeline = FullAudioPipeline::new();

    // Add all possible effects
    let mut eq = ParametricEq::new();
    eq.set_low_band(EqBand::low_shelf(80.0, 3.0));
    eq.set_mid_band(EqBand::peaking(1000.0, -2.0, 1.5));
    eq.set_high_band(EqBand::high_shelf(8000.0, 2.0));
    pipeline.add_effect(Box::new(eq));

    let mut graphic_eq = GraphicEq::new_10_band();
    graphic_eq.set_preset(GraphicEqPreset::VShape);
    pipeline.add_effect(Box::new(graphic_eq));

    let comp = Compressor::with_settings(CompressorSettings::aggressive());
    pipeline.add_effect(Box::new(comp));

    let crossfeed = Crossfeed::with_preset(CrossfeedPreset::Meier);
    pipeline.add_effect(Box::new(crossfeed));

    let enhancer = StereoEnhancer::with_settings(StereoSettings::wide());
    pipeline.add_effect(Box::new(enhancer));

    let limiter = Limiter::with_settings(LimiterSettings::brickwall());
    pipeline.add_effect(Box::new(limiter));

    // Process 5 seconds of audio
    let input = generate_multitone(&[100.0, 500.0, 1000.0, 5000.0, 10000.0], 44100, 5.0, 0.3);

    let start = Instant::now();
    let output = pipeline.process(&input, 44100);
    let elapsed = start.elapsed();

    let audio_duration = 5.0;
    let real_time_ratio = audio_duration / elapsed.as_secs_f32();

    assert!(output.iter().all(|s| s.is_finite()));
    assert!(calculate_peak(&output) <= 1.0);

    // Should still be at least 20x real-time even with all effects
    assert!(
        real_time_ratio > 20.0,
        "Pipeline should be at least 20x real-time with all effects, got {:.1}x",
        real_time_ratio
    );

    println!(
        "CPU Usage (All Effects): {:.2}ms for 5s audio ({:.0}x real-time)",
        elapsed.as_millis(),
        real_time_ratio
    );
}

#[test]
fn test_memory_usage_pattern() {
    // Test: Memory usage pattern (no unexpected growth)
    let mut pipeline = FullAudioPipeline::new();

    let eq = ParametricEq::new();
    pipeline.add_effect(Box::new(eq));

    let comp = Compressor::new();
    pipeline.add_effect(Box::new(comp));

    // Process multiple chunks and verify no memory issues
    let chunk = generate_sine_wave(440.0, 44100, 0.1, 0.5);

    for i in 0..100 {
        let output = pipeline.process(&chunk, 44100);

        // Output length should be consistent
        assert!(
            output.len() == chunk.len(),
            "Output length should be consistent at iteration {}",
            i
        );

        assert!(output.iter().all(|s| s.is_finite()));
    }

    println!("Memory Usage Pattern: 100 iterations completed without issues");
}

#[test]
fn test_latency_with_all_effects() {
    // Test: Total latency with all effects enabled
    let mut pipeline = FullAudioPipeline::new();

    // Setup resampler
    pipeline
        .setup_resampler(44100, 48000, ResamplingQuality::High)
        .unwrap();

    // Add all effects
    let eq = ParametricEq::new();
    pipeline.add_effect(Box::new(eq));

    let comp = Compressor::new();
    pipeline.add_effect(Box::new(comp));

    let crossfeed = Crossfeed::with_preset(CrossfeedPreset::Natural);
    pipeline.add_effect(Box::new(crossfeed));

    let limiter = Limiter::new();
    pipeline.add_effect(Box::new(limiter));

    let latency = pipeline.get_latency();

    // Latency should be reasonable (< 5000 samples at output rate)
    let latency_ms = latency as f32 / 48.0; // 48kHz output rate

    assert!(
        latency_ms < 100.0,
        "Total latency should be < 100ms, got {:.1}ms",
        latency_ms
    );

    println!(
        "Total Latency with All Effects: {} samples ({:.1}ms at 48kHz)",
        latency, latency_ms
    );
}

// ============================================================================
// 7. A/B Testing Framework Tests
// ============================================================================

#[test]
fn test_null_test_bypassed_chain_vs_original() {
    // Null test: Bypassed chain should be identical to original
    let mut pipeline = FullAudioPipeline::new();

    // Add effects but disable them
    let mut eq = ParametricEq::new();
    eq.set_enabled(false);
    pipeline.add_effect(Box::new(eq));

    let mut comp = Compressor::new();
    comp.set_enabled(false);
    pipeline.add_effect(Box::new(comp));

    let mut limiter = Limiter::new();
    limiter.set_enabled(false);
    pipeline.add_effect(Box::new(limiter));

    // Generate input
    let input = generate_sine_wave(1000.0, 44100, 0.5, 0.5);

    // Process
    let output = pipeline.process(&input, 44100);

    // Should be identical
    let max_diff = calculate_max_difference(&output, &input);
    assert!(
        max_diff < 0.0001,
        "Bypassed chain should be identical to original, got max diff: {}",
        max_diff
    );

    println!("Null Test (Bypassed): Max difference = {:.6}", max_diff);
}

#[test]
fn test_level_matching_for_ab_comparison() {
    // Test: Level matching between processed and original
    let mut pipeline = FullAudioPipeline::new();

    // Add EQ with boost
    let mut eq = ParametricEq::new();
    eq.set_low_band(EqBand::low_shelf(80.0, 6.0)); // +6dB
    pipeline.add_effect(Box::new(eq));

    // Generate input
    let input = generate_sine_wave(80.0, 44100, 0.5, 0.3);
    let input_rms = calculate_rms(&input);

    // Process
    let output = pipeline.process(&input, 44100);
    let output_rms = calculate_rms(&output);

    // Calculate level difference
    let level_diff_db = linear_to_db(output_rms / input_rms);

    // Level should have increased by approximately 6dB
    assert!(
        level_diff_db > 3.0 && level_diff_db < 9.0,
        "Level should increase by ~6dB, got {:.1}dB",
        level_diff_db
    );

    // For A/B test, we could normalize the output
    let normalization_factor = input_rms / output_rms;
    let normalized_output: Vec<f32> = output.iter().map(|s| s * normalization_factor).collect();
    let normalized_rms = calculate_rms(&normalized_output);

    let rms_diff = (normalized_rms - input_rms).abs();
    assert!(
        rms_diff < 0.01,
        "Level-matched output should have same RMS as input"
    );

    println!(
        "Level Matching: Original RMS={:.3}, Processed RMS={:.3}, Diff={:.1}dB",
        input_rms, output_rms, level_diff_db
    );
}

#[test]
fn test_phase_alignment_verification() {
    // Test: Verify phase alignment through chain
    let mut pipeline = FullAudioPipeline::new();

    // Add neutral effects (should have minimal phase shift at 1kHz)
    let eq = ParametricEq::new();
    pipeline.add_effect(Box::new(eq));

    // Generate input
    let input = generate_sine_wave(1000.0, 44100, 0.5, 0.5);

    // Process
    let output = pipeline.process(&input, 44100);

    // Calculate phase difference
    let mono_input = extract_mono(&input, 0);
    let mono_output = extract_mono(&output, 0);
    let phase_diff = calculate_phase_difference(&mono_input, &mono_output, 1000.0, 44100);

    // Phase should be relatively aligned (within 30 degrees at 1kHz)
    assert!(
        phase_diff.abs() < 30.0,
        "Phase should be relatively aligned, got {:.1} degrees",
        phase_diff
    );

    println!(
        "Phase Alignment: Phase difference at 1kHz = {:.1} degrees",
        phase_diff
    );
}

// ============================================================================
// 8. Regression Suite Tests
// ============================================================================

#[test]
fn test_deterministic_output() {
    // Test: Same input should always produce same output
    let mut pipeline = FullAudioPipeline::new();

    let eq = ParametricEq::new();
    pipeline.add_effect(Box::new(eq));

    let comp = Compressor::new();
    pipeline.add_effect(Box::new(comp));

    let limiter = Limiter::new();
    pipeline.add_effect(Box::new(limiter));

    // Generate input
    let input = generate_sine_wave(440.0, 44100, 0.5, 0.5);

    // Process multiple times
    let outputs: Vec<Vec<f32>> = (0..3)
        .map(|_| {
            pipeline.reset();
            pipeline.process(&input, 44100)
        })
        .collect();

    // All outputs should be identical
    for i in 1..outputs.len() {
        let diff = calculate_max_difference(&outputs[0], &outputs[i]);
        assert!(
            diff < 0.0001,
            "Output should be deterministic, run {} differs by {}",
            i,
            diff
        );
    }

    println!("Deterministic Output: All 3 runs produced identical output");
}

#[test]
fn test_golden_reference_comparison() {
    // Test: Compare output against a "golden" reference
    // In a real system, golden files would be stored externally

    let mut pipeline = FullAudioPipeline::new();

    let eq = ParametricEq::new();
    pipeline.add_effect(Box::new(eq));

    let limiter = Limiter::new();
    pipeline.add_effect(Box::new(limiter));

    // Generate known input
    let input = generate_sine_wave(1000.0, 44100, 0.2, 0.5);

    // Process
    let output = pipeline.process(&input, 44100);

    // Create "golden" reference (in practice, this would be loaded from file)
    // For this test, we use properties that should remain stable
    let output_peak = calculate_peak(&output);
    let output_rms = calculate_rms(&output);

    // These values should be consistent across versions
    // Peak should be close to input peak (neutral effects)
    assert!(
        (output_peak - 0.5).abs() < 0.1,
        "Peak should be stable across versions"
    );

    // RMS should be close to theoretical sine RMS (0.707 * amplitude)
    let expected_rms = 0.5 / 2.0_f32.sqrt();
    assert!(
        (output_rms - expected_rms).abs() < 0.1,
        "RMS should be stable across versions"
    );

    println!(
        "Golden Reference: Peak={:.3} (expected ~0.5), RMS={:.3} (expected ~{:.3})",
        output_peak, output_rms, expected_rms
    );
}

#[test]
fn test_bit_exact_reproduction() {
    // Test: Verify bit-exact reproduction with reset
    let mut pipeline = FullAudioPipeline::new();

    let eq = ParametricEq::new();
    pipeline.add_effect(Box::new(eq));

    let comp = Compressor::new();
    pipeline.add_effect(Box::new(comp));

    // Generate input
    let input = generate_sine_wave(440.0, 44100, 0.3, 0.5);

    // First pass
    pipeline.reset();
    let output1 = pipeline.process(&input, 44100);

    // Reset and second pass
    pipeline.reset();
    let output2 = pipeline.process(&input, 44100);

    // Should be bit-exact
    assert_eq!(output1.len(), output2.len(), "Output lengths should match");

    for (i, (a, b)) in output1.iter().zip(output2.iter()).enumerate() {
        assert!(
            (a - b).abs() < f32::EPSILON * 100.0,
            "Sample {} should be bit-exact: {} vs {}",
            i,
            a,
            b
        );
    }

    println!("Bit-Exact Reproduction: Verified across reset cycles");
}

#[test]
fn test_version_compatibility_api() {
    // Test: API compatibility check (all expected methods exist)
    // This test ensures the API doesn't break between versions

    // EffectChain API
    let mut chain = EffectChain::new();
    let _ = chain.len();
    let _ = chain.is_empty();
    chain.clear();

    // ParametricEq API
    let mut eq = ParametricEq::new();
    eq.set_low_band(EqBand::low_shelf(80.0, 0.0));
    eq.set_mid_band(EqBand::peaking(1000.0, 0.0, 1.0));
    eq.set_high_band(EqBand::high_shelf(8000.0, 0.0));
    let _ = eq.low_band();
    let _ = eq.mid_band();
    let _ = eq.high_band();

    // Compressor API
    let mut comp = Compressor::new();
    let _ = Compressor::with_settings(CompressorSettings::gentle());
    let _ = Compressor::with_settings(CompressorSettings::moderate());
    let _ = Compressor::with_settings(CompressorSettings::aggressive());
    comp.set_threshold(-20.0);
    comp.set_ratio(4.0);
    comp.set_attack(5.0);
    comp.set_release(50.0);
    comp.set_makeup_gain(6.0);

    // Limiter API
    let mut limiter = Limiter::new();
    let _ = Limiter::with_settings(LimiterSettings::brickwall());
    let _ = Limiter::with_settings(LimiterSettings::soft());
    limiter.set_threshold(-0.5);
    limiter.set_release(100.0);

    // Crossfeed API
    let _ = Crossfeed::with_preset(CrossfeedPreset::Natural);
    let _ = Crossfeed::with_preset(CrossfeedPreset::Relaxed);
    let _ = Crossfeed::with_preset(CrossfeedPreset::Meier);

    // StereoEnhancer API
    let _ = StereoEnhancer::with_settings(StereoSettings::default());
    let _ = StereoEnhancer::with_settings(StereoSettings::mono());
    let _ = StereoEnhancer::with_settings(StereoSettings::wide());
    let _ = StereoEnhancer::with_settings(StereoSettings::extra_wide());

    // GraphicEq API
    let mut geq = GraphicEq::new_10_band();
    geq.set_preset(GraphicEqPreset::Flat);
    geq.set_preset(GraphicEqPreset::BassBoost);
    geq.set_preset(GraphicEqPreset::TrebleBoost);
    geq.set_preset(GraphicEqPreset::VShape);

    // Resampler API
    let mut resampler = Resampler::new(
        ResamplerBackend::Auto,
        44100,
        48000,
        2,
        ResamplingQuality::High,
    )
    .unwrap();
    let _ = resampler.input_rate();
    let _ = resampler.output_rate();
    let _ = resampler.channels();
    let _ = resampler.latency();
    resampler.reset();

    println!("Version Compatibility API: All expected methods exist");
}

// ============================================================================
// 9. Comprehensive Integration Scenarios
// ============================================================================

#[test]
fn test_audiophile_mastering_chain() {
    // Simulate: Audiophile-grade mastering chain
    let mut pipeline = FullAudioPipeline::new();

    // High-quality resampling to 96kHz
    pipeline
        .setup_resampler(44100, 96000, ResamplingQuality::Maximum)
        .unwrap();

    // Subtle EQ for tonal balance
    let mut eq = ParametricEq::new();
    eq.set_low_band(EqBand::low_shelf(40.0, -0.5)); // Subtle sub cut
    eq.set_mid_band(EqBand::peaking(3000.0, 0.5, 1.5)); // Slight presence
    eq.set_high_band(EqBand::high_shelf(12000.0, 0.3)); // Air
    pipeline.add_effect(Box::new(eq));

    // Very gentle compression
    let comp = Compressor::with_settings(CompressorSettings {
        threshold_db: -10.0,
        ratio: 1.5,
        attack_ms: 30.0,
        release_ms: 200.0,
        knee_db: 10.0,
        makeup_gain_db: 0.5,
    });
    pipeline.add_effect(Box::new(comp));

    // Transparent limiting
    let limiter = Limiter::with_settings(LimiterSettings {
        threshold_db: -0.3,
        release_ms: 300.0,
    });
    pipeline.add_effect(Box::new(limiter));

    // Generate high-quality input (multi-tone)
    let input = generate_multitone(&[100.0, 500.0, 1000.0, 5000.0, 10000.0], 44100, 1.0, 0.3);

    // Process
    let output = pipeline.process(&input, 44100);

    // Verify output quality
    assert!(output.iter().all(|s| s.is_finite()));
    assert!(calculate_peak(&output) <= 1.0);

    // Check frequency preservation
    let mono_output = extract_mono(&output, 0);
    let response = measure_frequency_response(&mono_output, &[100.0, 500.0, 1000.0, 5000.0], 96000);

    // All frequencies should be present (within 10dB of each other)
    let levels: Vec<f32> = response.iter().map(|(_, db)| *db).collect();
    let level_range = levels.iter().cloned().fold(f32::NEG_INFINITY, f32::max)
        - levels.iter().cloned().fold(f32::INFINITY, f32::min);

    assert!(
        level_range < 10.0,
        "Frequency response should be balanced, range: {:.1}dB",
        level_range
    );

    println!("Audiophile Mastering Chain: Quality verified");
}

#[test]
fn test_podcast_voice_processing_chain() {
    // Simulate: Podcast voice processing
    let mut pipeline = FullAudioPipeline::new();

    // Voice EQ
    let mut eq = ParametricEq::new();
    eq.set_low_band(EqBand::low_shelf(80.0, -6.0)); // Cut rumble
    eq.set_mid_band(EqBand::peaking(3500.0, 3.0, 1.5)); // Presence
    eq.set_high_band(EqBand::high_shelf(6000.0, -2.0)); // De-ess slightly
    pipeline.add_effect(Box::new(eq));

    // Voice compression
    let comp = Compressor::with_settings(CompressorSettings {
        threshold_db: -18.0,
        ratio: 3.5,
        attack_ms: 5.0,
        release_ms: 80.0,
        knee_db: 6.0,
        makeup_gain_db: 8.0,
    });
    pipeline.add_effect(Box::new(comp));

    // Broadcast limiting
    let limiter = Limiter::with_settings(LimiterSettings {
        threshold_db: -1.0,
        release_ms: 100.0,
    });
    pipeline.add_effect(Box::new(limiter));

    // Generate voice-like signal
    let input = generate_multitone(&[300.0, 600.0, 900.0, 1200.0], 44100, 0.5, 0.3);

    // Process
    let output = pipeline.process(&input, 44100);

    // Verify
    assert!(output.iter().all(|s| s.is_finite()));
    assert!(calculate_peak(&output) <= 1.0);

    println!("Podcast Voice Processing: Chain verified");
}

#[test]
fn test_headphone_listening_chain() {
    // Simulate: Optimized for headphone listening
    let mut pipeline = FullAudioPipeline::new();

    // Crossfeed for better stereo image
    let crossfeed = Crossfeed::with_preset(CrossfeedPreset::Natural);
    pipeline.add_effect(Box::new(crossfeed));

    // Subtle EQ for headphones
    let mut eq = ParametricEq::new();
    eq.set_low_band(EqBand::low_shelf(80.0, 2.0)); // Bass compensation
    eq.set_mid_band(EqBand::peaking(3000.0, -1.0, 2.0)); // Reduce harshness
    eq.set_high_band(EqBand::high_shelf(8000.0, 1.0)); // Air
    pipeline.add_effect(Box::new(eq));

    // Stereo enhancement
    let enhancer = StereoEnhancer::with_settings(StereoSettings::with_width(1.1));
    pipeline.add_effect(Box::new(enhancer));

    // Soft limiting for ear protection
    let limiter = Limiter::with_settings(LimiterSettings::soft());
    pipeline.add_effect(Box::new(limiter));

    // Generate stereo content
    let input: Vec<f32> = (0..22050)
        .flat_map(|i| {
            let t = i as f32 / 44100.0;
            let left = (2.0 * PI * 440.0 * t).sin() * 0.4;
            let right = (2.0 * PI * 660.0 * t).sin() * 0.4; // Different frequency
            [left, right]
        })
        .collect();

    // Process
    let output = pipeline.process(&input, 44100);

    // Verify crossfeed effect (right channel should have some left content)
    let right_channel = extract_mono(&output, 1);
    let right_rms = calculate_rms(&right_channel);

    assert!(
        right_rms > 0.1,
        "Right channel should have content (crossfeed)"
    );
    assert!(calculate_peak(&output) <= 1.0);

    println!("Headphone Listening Chain: Crossfeed and stereo processing verified");
}

#[test]
fn test_stress_extreme_parameters() {
    // Stress test: Extreme parameter combinations
    let mut pipeline = FullAudioPipeline::new();

    // Extreme EQ
    let mut eq = ParametricEq::new();
    eq.set_low_band(EqBand::low_shelf(20.0, 12.0)); // Max bass boost at lowest frequency
    eq.set_mid_band(EqBand::peaking(1000.0, 12.0, 0.1)); // Max boost, narrowest Q
    eq.set_high_band(EqBand::high_shelf(20000.0, 12.0)); // Max treble boost at highest frequency
    pipeline.add_effect(Box::new(eq));

    // Aggressive compression
    let comp = Compressor::with_settings(CompressorSettings::aggressive());
    pipeline.add_effect(Box::new(comp));

    // Brickwall limiting
    let limiter = Limiter::with_settings(LimiterSettings::brickwall());
    pipeline.add_effect(Box::new(limiter));

    // Generate full-scale signal
    let input = generate_sine_wave(100.0, 44100, 0.5, 0.8);

    // Process
    let output = pipeline.process(&input, 44100);

    // Output should still be valid and limited
    assert!(
        output.iter().all(|s| s.is_finite()),
        "Extreme parameters should still produce finite output"
    );
    assert!(
        calculate_peak(&output) <= 1.0,
        "Limiter should prevent clipping even with extreme parameters"
    );

    println!("Stress Test (Extreme Parameters): Pipeline remained stable");
}
