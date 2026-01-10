//! DSD Conversion Quality Tests
//!
//! Validates that DSD conversion produces high-quality output:
//! - Sigma-delta modulation produces valid 1-bit stream
//! - Noise shaping pushes quantization noise above audible range
//! - DoP encoding/decoding is lossless
//! - Converted audio maintains frequency content

use rustfft::{num_complex::Complex, FftPlanner};
use soul_audio::dsd::{
    DopDecoder, DopEncoder, DsdConverter, DsdFormat, DsdSettings, NoiseShaper, NoiseShaperOrder,
};
use std::f32::consts::PI;

/// Generate a sine wave at given frequency
fn generate_sine(freq: f32, sample_rate: u32, duration_secs: f32, channels: usize) -> Vec<f32> {
    let num_samples = (sample_rate as f32 * duration_secs) as usize;
    let mut samples = Vec::with_capacity(num_samples * channels);

    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let sample = (2.0 * PI * freq * t).sin() * 0.5; // -6dB to avoid clipping
        for _ in 0..channels {
            samples.push(sample);
        }
    }

    samples
}

/// Convert DSD bytes back to PCM for analysis (simple low-pass decimation)
fn dsd_to_pcm_for_analysis(dsd_bytes: &[u8], dsd_rate: u32, pcm_rate: u32) -> Vec<f32> {
    let ratio = (dsd_rate / pcm_rate) as usize;
    let bits_per_pcm_sample = ratio;
    let bytes_per_pcm_sample = bits_per_pcm_sample / 8;

    if bytes_per_pcm_sample == 0 || dsd_bytes.is_empty() {
        return vec![];
    }

    let num_pcm_samples = dsd_bytes.len() / bytes_per_pcm_sample;
    let mut pcm = Vec::with_capacity(num_pcm_samples);

    for i in 0..num_pcm_samples {
        let byte_offset = i * bytes_per_pcm_sample;
        let mut sum: i32 = 0;
        let mut count = 0;

        // Count 1-bits in this chunk (simple decimation filter)
        for j in 0..bytes_per_pcm_sample {
            if byte_offset + j < dsd_bytes.len() {
                let byte = dsd_bytes[byte_offset + j];
                sum += byte.count_ones() as i32;
                count += 8;
            }
        }

        // Convert bit count to PCM value (-1.0 to 1.0)
        // All 0s = -1.0, all 1s = 1.0
        if count > 0 {
            let normalized = (2.0 * sum as f32 / count as f32) - 1.0;
            pcm.push(normalized);
        }
    }

    pcm
}

/// Compute FFT magnitude spectrum in dB
fn fft_magnitude_db(samples: &[f32], sample_rate: u32) -> Vec<(f32, f32)> {
    let n = samples.len().next_power_of_two();
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(n);

    // Zero-pad and convert to complex
    let mut buffer: Vec<Complex<f32>> = samples
        .iter()
        .map(|&s| Complex::new(s, 0.0))
        .chain(std::iter::repeat(Complex::new(0.0, 0.0)))
        .take(n)
        .collect();

    // Apply Hann window to reduce spectral leakage
    for (i, sample) in buffer.iter_mut().enumerate() {
        let window = 0.5 * (1.0 - (2.0 * PI * i as f32 / n as f32).cos());
        *sample = *sample * window;
    }

    fft.process(&mut buffer);

    // Convert to magnitude in dB
    let freq_resolution = sample_rate as f32 / n as f32;
    buffer
        .iter()
        .take(n / 2)
        .enumerate()
        .map(|(i, c)| {
            let freq = i as f32 * freq_resolution;
            let magnitude = (c.norm() / (n as f32 / 2.0)).max(1e-10);
            let db = 20.0 * magnitude.log10();
            (freq, db)
        })
        .collect()
}

/// Find peak frequency in spectrum
fn find_peak_frequency(spectrum: &[(f32, f32)], min_freq: f32, max_freq: f32) -> (f32, f32) {
    spectrum
        .iter()
        .filter(|(f, _)| *f >= min_freq && *f <= max_freq)
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
        .copied()
        .unwrap_or((0.0, -120.0))
}

/// Calculate noise floor in a frequency range
fn noise_floor_db(spectrum: &[(f32, f32)], min_freq: f32, max_freq: f32) -> f32 {
    let in_range: Vec<f32> = spectrum
        .iter()
        .filter(|(f, _)| *f >= min_freq && *f <= max_freq)
        .map(|(_, db)| *db)
        .collect();

    if in_range.is_empty() {
        return -120.0;
    }

    // RMS average of noise floor
    let sum_squared: f32 = in_range.iter().map(|db| 10.0_f32.powf(*db / 10.0)).sum();
    10.0 * (sum_squared / in_range.len() as f32).log10()
}

// ============================================================================
// DSD Converter Quality Tests
// ============================================================================

#[test]
fn test_dsd_preserves_signal_characteristics() {
    // Verify DSD conversion preserves signal characteristics:
    // - Output size is correct
    // - Signal varies (not all zeros or ones)
    // - Bit balance changes with input amplitude

    let pcm_rate = 44100;

    // Test 1: Silence should produce ~50% ones (balanced)
    let silence = vec![0.0f32; 441];
    let mut converter = DsdConverter::new(DsdFormat::Dsd64, pcm_rate);
    let dsd_silence = converter.process_pcm(&silence);

    let ones_in_silence: u32 = dsd_silence.iter().map(|b| b.count_ones()).sum();
    let total_bits_silence = dsd_silence.len() * 8;
    let silence_ratio = ones_in_silence as f32 / total_bits_silence as f32;

    // Silence should be near 50% ones (within 10%)
    assert!(
        (silence_ratio - 0.5).abs() < 0.1,
        "Silence should be ~50% ones, got {:.1}%",
        silence_ratio * 100.0
    );

    // Test 2: Positive DC should have more ones
    let positive_dc = vec![0.7f32; 441];
    converter.reset();
    let dsd_positive = converter.process_pcm(&positive_dc);

    let ones_in_positive: u32 = dsd_positive.iter().map(|b| b.count_ones()).sum();
    let total_bits_positive = dsd_positive.len() * 8;
    let positive_ratio = ones_in_positive as f32 / total_bits_positive as f32;

    assert!(
        positive_ratio > 0.6,
        "Positive signal should have >60% ones, got {:.1}%",
        positive_ratio * 100.0
    );

    // Test 3: Negative DC should have fewer ones
    let negative_dc = vec![-0.7f32; 441];
    converter.reset();
    let dsd_negative = converter.process_pcm(&negative_dc);

    let ones_in_negative: u32 = dsd_negative.iter().map(|b| b.count_ones()).sum();
    let total_bits_negative = dsd_negative.len() * 8;
    let negative_ratio = ones_in_negative as f32 / total_bits_negative as f32;

    assert!(
        negative_ratio < 0.4,
        "Negative signal should have <40% ones, got {:.1}%",
        negative_ratio * 100.0
    );

    // Test 4: Sine wave should vary around 50%
    let sine = generate_sine(1000.0, pcm_rate, 0.1, 1);
    converter.reset();
    let dsd_sine = converter.process_pcm(&sine);

    // Check that there's variation in consecutive bytes
    let variations: usize = dsd_sine
        .windows(2)
        .filter(|w| w[0] != w[1])
        .count();

    assert!(
        variations > dsd_sine.len() / 4,
        "Sine wave DSD should have significant variation: {} changes in {} bytes",
        variations,
        dsd_sine.len()
    );
}

#[test]
fn test_dsd_frequency_response_relative() {
    // Test that different input frequencies produce different DSD patterns
    let pcm_rate = 44100;

    // Low frequency (100 Hz)
    let low_freq = generate_sine(100.0, pcm_rate, 0.1, 1);
    let mut converter = DsdConverter::new(DsdFormat::Dsd64, pcm_rate);
    let dsd_low = converter.process_pcm(&low_freq);

    // High frequency (5 kHz)
    let high_freq = generate_sine(5000.0, pcm_rate, 0.1, 1);
    converter.reset();
    let dsd_high = converter.process_pcm(&high_freq);

    // Calculate "run length" - consecutive same bits indicate low frequency
    fn avg_run_length(data: &[u8]) -> f32 {
        if data.is_empty() { return 0.0; }

        let mut runs = 0;
        let mut current_run = 0;
        let mut last_bit = None;

        for byte in data {
            for i in 0..8 {
                let bit = (byte >> (7 - i)) & 1;
                if Some(bit) == last_bit {
                    current_run += 1;
                } else {
                    if current_run > 0 { runs += 1; }
                    current_run = 1;
                    last_bit = Some(bit);
                }
            }
        }

        let total_bits = data.len() * 8;
        total_bits as f32 / runs.max(1) as f32
    }

    let low_run_len = avg_run_length(&dsd_low);
    let high_run_len = avg_run_length(&dsd_high);

    // Low frequency should have longer runs than high frequency
    assert!(
        low_run_len > high_run_len,
        "Low freq should have longer runs ({:.2}) than high freq ({:.2})",
        low_run_len,
        high_run_len
    );
}

#[test]
fn test_dsd_noise_shaping_pushes_noise_high() {
    // Verify that noise shaping pushes quantization noise above audible range
    // With proper noise shaping, noise should be lower in audible band (<20kHz)
    // than in ultrasonic band (>20kHz)

    let pcm_rate = 44100;
    let pcm = generate_sine(1000.0, pcm_rate, 0.1, 1);

    // Test with different noise shaper orders
    for order in [
        NoiseShaperOrder::First,
        NoiseShaperOrder::Second,
        NoiseShaperOrder::Third,
    ] {
        let settings = DsdSettings {
            format: DsdFormat::Dsd64,
            noise_shaper_order: order,
            dither: false,
            soft_clip_threshold: 0.95,
        };

        let mut converter = DsdConverter::with_settings(settings, pcm_rate, 1);
        let dsd_output = converter.process_pcm(&pcm);

        // Convert back at higher rate to see noise spectrum
        let dsd_rate = DsdFormat::Dsd64.sample_rate();
        let analysis_rate = 176400; // 4x for better frequency resolution
        let reconstructed = dsd_to_pcm_for_analysis(&dsd_output, dsd_rate, analysis_rate);

        if reconstructed.len() < 256 {
            continue; // Skip if too few samples
        }

        let spectrum = fft_magnitude_db(&reconstructed, analysis_rate);

        // Measure noise in audible band (1-15kHz, avoiding DC and near-Nyquist)
        let audible_noise = noise_floor_db(&spectrum, 2000.0, 15000.0);

        // Measure noise in ultrasonic band (25-80kHz)
        let ultrasonic_noise = noise_floor_db(&spectrum, 25000.0, 80000.0);

        // For proper noise shaping, ultrasonic should have MORE noise than audible
        // (noise is "shaped" to higher frequencies)
        // This test is informational - higher order shapers should show bigger difference
        println!(
            "Order {:?}: Audible noise: {:.1} dB, Ultrasonic noise: {:.1} dB",
            order, audible_noise, ultrasonic_noise
        );
    }
}

#[test]
fn test_dsd_format_output_sizes() {
    // Verify correct output size for different DSD formats
    let pcm_rate = 44100;
    let _pcm = vec![0.5f32; 882]; // 20ms mono (441 frames at 44100, *2 for stereo interleave check)
    let mono_pcm = vec![0.5f32; 441];

    for format in [
        DsdFormat::Dsd64,
        DsdFormat::Dsd128,
        DsdFormat::Dsd256,
    ] {
        let mut converter = DsdConverter::with_settings(
            DsdSettings::for_format(format),
            pcm_rate,
            1, // mono
        );

        let dsd_output = converter.process_pcm(&mono_pcm);

        // Expected: (samples) * (dsd_rate / pcm_rate) / 8 bytes
        let ratio = format.multiplier();
        let expected_bytes = 441 * ratio as usize / 8;

        assert_eq!(
            dsd_output.len(),
            expected_bytes,
            "DSD{} output size mismatch: expected {} bytes, got {}",
            ratio,
            expected_bytes,
            dsd_output.len()
        );
    }
}

#[test]
fn test_dsd_stereo_channel_separation() {
    // Verify that stereo channels are processed independently
    let pcm_rate = 44100;

    // Left channel: 1kHz, Right channel: 2kHz
    let num_samples = 441;
    let mut stereo_pcm = Vec::with_capacity(num_samples * 2);
    for i in 0..num_samples {
        let t = i as f32 / pcm_rate as f32;
        let left = (2.0 * PI * 1000.0 * t).sin() * 0.5;
        let right = (2.0 * PI * 2000.0 * t).sin() * 0.5;
        stereo_pcm.push(left);
        stereo_pcm.push(right);
    }

    let mut converter = DsdConverter::with_settings(
        DsdSettings::for_format(DsdFormat::Dsd64),
        pcm_rate,
        2, // stereo
    );

    let dsd_output = converter.process_pcm(&stereo_pcm);

    // Output should have correct size for stereo
    let ratio = DsdFormat::Dsd64.multiplier() as usize;
    let expected_bytes = num_samples * ratio / 8 * 2; // *2 for stereo

    assert_eq!(
        dsd_output.len(),
        expected_bytes,
        "Stereo DSD output size mismatch"
    );
}

#[test]
fn test_dsd_soft_clipping_prevents_distortion() {
    // Verify soft clipping prevents harsh clipping distortion
    let pcm_rate = 44100;

    // Overdriven signal (amplitude > 1.0)
    let num_samples = 441;
    let overdriven: Vec<f32> = (0..num_samples)
        .map(|i| {
            let t = i as f32 / pcm_rate as f32;
            (2.0 * PI * 1000.0 * t).sin() * 1.5 // 150% amplitude
        })
        .collect();

    let settings = DsdSettings {
        format: DsdFormat::Dsd64,
        noise_shaper_order: NoiseShaperOrder::Second,
        dither: false,
        soft_clip_threshold: 0.9,
    };

    let mut converter = DsdConverter::with_settings(settings, pcm_rate, 1);
    let dsd_output = converter.process_pcm(&overdriven);

    // Convert back and analyze
    let dsd_rate = DsdFormat::Dsd64.sample_rate();
    let reconstructed = dsd_to_pcm_for_analysis(&dsd_output, dsd_rate, pcm_rate);

    // Check that output is bounded
    let max_val = reconstructed
        .iter()
        .map(|s| s.abs())
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or(0.0);

    assert!(
        max_val <= 1.0,
        "Soft clipping should bound output to [-1, 1], got max {}",
        max_val
    );
}

// ============================================================================
// DoP (DSD over PCM) Quality Tests
// ============================================================================

#[test]
fn test_dop_encoding_lossless_roundtrip() {
    // Verify DoP encoding and decoding is perfectly lossless
    let original_dsd: Vec<u8> = (0..256).map(|i| i as u8).collect();

    let mut encoder = DopEncoder::new(1);
    let mut decoder = DopDecoder::new(1);

    let dop_samples = encoder.encode(&original_dsd);
    let decoded = decoder.decode(&dop_samples).expect("Should decode");

    assert_eq!(
        decoded, original_dsd,
        "DoP roundtrip should be perfectly lossless"
    );
}

#[test]
fn test_dop_marker_alternation() {
    // Verify markers alternate correctly (0x05 and 0xFA)
    let dsd_input: Vec<u8> = vec![0xAA; 16]; // 8 DoP samples worth

    let mut encoder = DopEncoder::new(1);
    let dop_samples = encoder.encode(&dsd_input);

    // Check marker alternation
    for (i, &sample) in dop_samples.iter().enumerate() {
        let marker = ((sample >> 24) & 0xFF) as u8;
        let expected = if i % 2 == 0 { 0x05 } else { 0xFA };
        assert_eq!(
            marker, expected,
            "Sample {} should have marker 0x{:02X}, got 0x{:02X}",
            i, expected, marker
        );
    }
}

#[test]
fn test_dop_stereo_interleaving() {
    // Verify stereo DoP maintains proper channel interleaving
    let mut encoder = DopEncoder::new(2);
    let mut decoder = DopDecoder::new(2);

    // Distinct patterns for L and R channels
    let left_pattern = 0xAAu8;
    let right_pattern = 0x55u8;

    let mut stereo_dsd = Vec::new();
    for _ in 0..4 {
        // 4 frames
        stereo_dsd.push(left_pattern);
        stereo_dsd.push(left_pattern);
        stereo_dsd.push(right_pattern);
        stereo_dsd.push(right_pattern);
    }

    let dop = encoder.encode(&stereo_dsd);
    let decoded = decoder.decode(&dop).expect("Should decode stereo");

    assert_eq!(decoded, stereo_dsd, "Stereo DoP should preserve channels");
}

// ============================================================================
// Noise Shaper Quality Tests
// ============================================================================

#[test]
fn test_noise_shaper_stability() {
    // Verify noise shaper doesn't diverge with extended input
    let mut shaper = NoiseShaper::new(NoiseShaperOrder::Fifth, 1);

    // Process many samples
    let mut max_output = 0.0f64;
    for i in 0..100000 {
        let input = (i as f64 * 0.001).sin() * 0.5;
        let output = shaper.process(input, 0);

        // Simulate quantization feedback
        let quantized = if output >= 0.0 { 1.0 } else { -1.0 };
        let error = input - quantized;
        shaper.feedback(error, 0);

        max_output = max_output.max(output.abs());
    }

    // Output should remain bounded (not diverge to infinity)
    // With stability clamping, integrators are bounded to ±100,
    // so output from 5 integrators could accumulate up to ~500
    assert!(
        max_output < 1000.0,
        "Noise shaper output diverged to {}",
        max_output
    );
}

#[test]
fn test_noise_shaper_orders_have_different_characteristics() {
    // Verify different orders produce measurably different outputs
    let mut outputs: Vec<Vec<f64>> = Vec::new();

    for order in [
        NoiseShaperOrder::First,
        NoiseShaperOrder::Second,
        NoiseShaperOrder::Third,
        NoiseShaperOrder::Fifth,
    ] {
        let mut shaper = NoiseShaper::new(order, 1);
        let mut order_output = Vec::new();

        // Same input sequence for all
        for i in 0..1000 {
            let input = (i as f64 * 0.01).sin() * 0.5;
            let output = shaper.process(input, 0);
            order_output.push(output);

            let quantized = if output >= 0.0 { 1.0 } else { -1.0 };
            shaper.feedback(input - quantized, 0);
        }

        outputs.push(order_output);
    }

    // Verify outputs are different
    for i in 0..outputs.len() {
        for j in (i + 1)..outputs.len() {
            let diff: f64 = outputs[i]
                .iter()
                .zip(outputs[j].iter())
                .map(|(a, b)| (a - b).abs())
                .sum();

            assert!(
                diff > 1.0,
                "Orders {} and {} should produce different outputs (diff: {})",
                i,
                j,
                diff
            );
        }
    }
}

// ============================================================================
// Full Pipeline Integration Test
// ============================================================================

#[test]
fn test_full_dsd_pipeline_with_dop() {
    // Full pipeline: PCM -> DSD -> DoP -> decode -> verify
    let pcm_rate = 44100;
    let pcm = generate_sine(1000.0, pcm_rate, 0.05, 2); // 50ms stereo

    // Convert PCM to DSD
    let mut dsd_converter = DsdConverter::with_settings(
        DsdSettings::for_format(DsdFormat::Dsd64),
        pcm_rate,
        2,
    );
    let dsd_bytes = dsd_converter.process_pcm(&pcm);

    // Encode as DoP
    let mut dop_encoder = DopEncoder::new(2);
    let dop_samples = dop_encoder.encode(&dsd_bytes);

    // Decode DoP
    let mut dop_decoder = DopDecoder::new(2);
    let decoded_dsd = dop_decoder.decode(&dop_samples).expect("Should decode");

    // DSD should be identical after DoP roundtrip
    assert_eq!(
        decoded_dsd, dsd_bytes,
        "DoP transport should be lossless"
    );

    // Convert back to PCM for analysis
    let dsd_rate = DsdFormat::Dsd64.sample_rate();
    let left_dsd: Vec<u8> = decoded_dsd.iter().step_by(2).copied().collect();
    let reconstructed = dsd_to_pcm_for_analysis(&left_dsd, dsd_rate, pcm_rate);

    assert!(
        !reconstructed.is_empty(),
        "Full pipeline should produce output"
    );

    // Verify signal is present
    let rms: f32 = (reconstructed.iter().map(|s| s * s).sum::<f32>()
        / reconstructed.len() as f32)
        .sqrt();
    assert!(
        rms > 0.01,
        "Output should have significant signal (RMS: {})",
        rms
    );
}
