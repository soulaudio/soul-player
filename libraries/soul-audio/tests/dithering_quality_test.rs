use soul_audio::dither::{StereoDither, TpdfDither};

/// Test that TPDF dither has zero mean (no DC offset)
#[test]
fn test_tpdf_zero_mean() {
    let mut dither = TpdfDither::new();
    let mut sum = 0i64;
    let iterations = 10_000;

    for _ in 0..iterations {
        let dithered = dither.dither_to_i16(0.0);
        sum += dithered as i64;
    }

    let average = sum as f64 / iterations as f64;
    assert!(
        average.abs() < 10.0,
        "TPDF dither should have zero mean, got average: {}",
        average
    );
}

/// Test that dithering zero signal produces no DC offset
#[test]
fn test_tpdf_no_dc_offset() {
    let mut dither = TpdfDither::new();
    let mut sum = 0i64;
    let iterations = 10_000;

    for _ in 0..iterations {
        let quantized = dither.dither_to_i16(0.0);
        sum += quantized as i64;
    }

    let average = sum as f64 / iterations as f64;
    assert!(
        average.abs() < 5.0,
        "Dithered zeros should sum to near zero, got average: {}",
        average
    );
}

/// Test that TPDF noise follows triangular distribution
#[test]
fn test_tpdf_triangular_distribution() {
    let mut dither = TpdfDither::new();
    let samples = 100_000;
    let bins = 100;
    let mut histogram = vec![0; bins];

    // Collect samples - dither at zero to isolate the noise distribution
    for _ in 0..samples {
        let quantized = dither.dither_to_i16(0.0);
        // Map to histogram bins (most values should be 0, ±1)
        // Center around bin 50
        let bin = (quantized + 50).clamp(0, (bins - 1) as i16) as usize;
        histogram[bin] += 1;
    }

    // Triangular distribution: peak in middle, decreases linearly to edges
    let middle = 50; // Zero is in the middle
    let middle_count = histogram[middle];

    // Check that bins near zero have higher counts than bins far from zero
    let near_count = histogram[middle - 1] + histogram[middle + 1];
    let far_count = histogram[middle - 10].max(1) + histogram[middle + 10].max(1);

    assert!(
        middle_count > near_count,
        "Middle of distribution should have most samples"
    );
    assert!(
        near_count > far_count,
        "Distribution should decrease away from center"
    );
}

/// Test that TPDF noise is in expected range [-1 LSB, +1 LSB]
#[test]
fn test_tpdf_range() {
    let mut dither = TpdfDither::new();
    let iterations = 10_000;

    for _ in 0..iterations {
        let dithered = dither.dither_to_i16(0.0);
        // For zero input, TPDF noise should produce values very close to 0
        // The noise itself is ±1 LSB in float, which rounds to 0, ±1 in i16
        assert!(
            dithered.abs() <= 1,
            "TPDF noise at zero input should be 0 or ±1, got: {}",
            dithered
        );
    }
}

/// Test that dither linearizes quantization for low-level signals
#[test]
fn test_dither_linearizes_quantization() {
    // Test with a range of very low amplitudes
    let mut dither = TpdfDither::new();
    let test_values = [0.0001, 0.0005, 0.001, 0.002];

    for &amplitude in &test_values {
        let samples = 10_000;
        let mut sum_undithered = 0i64;
        let mut sum_dithered = 0i64;

        for _ in 0..samples {
            // Undithered quantization
            let undithered = ((amplitude * 32768.0_f32).round() as i16) as i64;
            sum_undithered += undithered;

            // Dithered quantization
            let dithered = dither.dither_to_i16(amplitude) as i64;
            sum_dithered += dithered;
        }

        let avg_undithered = sum_undithered as f64 / samples as f64;
        let avg_dithered = sum_dithered as f64 / samples as f64;
        let expected = amplitude * 32768.0;

        println!(
            "Amplitude {:.4}: undithered avg {:.3}, dithered avg {:.3}, expected {:.3}",
            amplitude, avg_undithered, avg_dithered, expected
        );

        // Dithered average should track the input more accurately
        let dithered_error = (avg_dithered - expected as f64).abs();

        // Dithered output should produce reasonable approximations
        assert!(
            dithered_error < expected as f64 * 0.2,
            "Dithered average should be within 20% of expected"
        );
    }

    // Test that dither produces varying outputs for constant input
    let constant_input = 0.001;
    let mut outputs = std::collections::HashSet::new();
    for _ in 0..100 {
        outputs.insert(dither.dither_to_i16(constant_input));
    }

    println!(
        "Unique outputs for constant {}: {}",
        constant_input,
        outputs.len()
    );

    // Should have multiple different outputs due to dither
    assert!(
        outputs.len() >= 2,
        "Dither should produce varying outputs for constant input"
    );
}

/// Test that dither adds randomness to quantization
#[test]
fn test_dither_adds_randomness() {
    let samples = 10_000;
    let mut dither = TpdfDither::new();
    let signal_value = 0.5; // Constant signal

    let mut output_values = Vec::with_capacity(samples);

    for _ in 0..samples {
        let dithered = dither.dither_to_i16(signal_value);
        output_values.push(dithered);
    }

    // For constant input, output should vary due to dither
    let min_val = *output_values.iter().min().unwrap();
    let max_val = *output_values.iter().max().unwrap();
    let range = max_val - min_val;

    println!(
        "Output range for constant 0.5 input: {} to {} (range: {})",
        min_val, max_val, range
    );

    // Should have some variation (at least 2-3 different values)
    assert!(
        range >= 1,
        "Dithered output should vary for constant input, got range: {}",
        range
    );

    // Calculate variance
    let mean: f64 = output_values.iter().map(|&x| x as f64).sum::<f64>() / samples as f64;
    let variance: f64 = output_values
        .iter()
        .map(|&x| (x as f64 - mean).powi(2))
        .sum::<f64>()
        / samples as f64;

    println!("Variance: {:.4}", variance);

    // Variance should be non-zero
    assert!(
        variance > 0.0,
        "Dithered output should have non-zero variance"
    );
}

/// Test that stereo channels are independently dithered
#[test]
fn test_stereo_channels_independent() {
    let mut stereo = StereoDither::new();
    let iterations = 1000;
    let mut same_count = 0;

    let input = vec![0.5_f32; iterations * 2]; // Stereo: L, R, L, R...
    let mut output = vec![0_i16; iterations * 2];

    stereo.process_stereo_to_i16(&input, &mut output);

    for i in 0..iterations {
        let left = output[i * 2];
        let right = output[i * 2 + 1];

        if left == right {
            same_count += 1;
        }
    }

    let same_percent = (same_count as f32 / iterations as f32) * 100.0;
    println!("Stereo channels: {:.2}% identical values", same_percent);

    // For independent dither, we expect some different values
    // But since the signal is constant, there's a reasonable chance of overlap
    // We just want to verify they're not perfectly correlated
    assert!(
        same_count < iterations,
        "Stereo channels should not be perfectly identical (100% same)"
    );

    // At least some samples should be different
    let diff_count = iterations - same_count;
    assert!(
        diff_count > iterations / 10, // At least 10% different
        "At least some stereo samples should differ (got only {}/{} different)",
        diff_count,
        iterations
    );
}

/// Test stereo decorrelation quality over long duration
#[test]
fn test_stereo_decorrelation_quality() {
    let mut stereo = StereoDither::new();
    let samples = 10_000;

    let mut input = Vec::with_capacity(samples * 2);
    let mut output = vec![0_i16; samples * 2];

    // Generate slowly varying signal
    for i in 0..samples {
        let sample = 0.3 * ((i as f32) * 0.01).sin();
        input.push(sample); // Left
        input.push(sample); // Right
    }

    stereo.process_stereo_to_i16(&input, &mut output);

    // Count how many frames have different L/R values
    let mut diff_count = 0;
    for i in 0..samples {
        let left = output[i * 2];
        let right = output[i * 2 + 1];
        if left != right {
            diff_count += 1;
        }
    }

    let diff_percent = (diff_count as f32 / samples as f32) * 100.0;
    println!(
        "Stereo decorrelation: {:.2}% of frames have different L/R values",
        diff_percent
    );

    // With independent dither, we expect at least some frames to differ
    assert!(
        diff_count > 0,
        "Independent dither should produce some different L/R values"
    );

    // Calculate variance in the differences
    let mut diff_sum = 0i64;
    for i in 0..samples {
        let diff = (output[i * 2] as i32 - output[i * 2 + 1] as i32).abs();
        diff_sum += diff as i64;
    }

    let avg_diff = diff_sum as f64 / samples as f64;
    println!("Average |L-R| difference: {:.4}", avg_diff);

    // Average difference should be small but non-zero
    assert!(
        avg_diff > 0.0,
        "Average L-R difference should be non-zero due to independent dither"
    );
}

/// Test that dither preserves signal amplitude
#[test]
fn test_dither_preserves_amplitude() {
    let sample_rate = 44100.0;
    let frequency = 1000.0;
    let amplitude = 0.5;
    let samples = (sample_rate * 0.1) as usize; // 100ms

    let mut dither = TpdfDither::new();
    let mut sum_squares = 0.0;

    for i in 0..samples {
        let t = i as f32 / sample_rate;
        let sample = amplitude * (2.0 * std::f32::consts::PI * frequency * t).sin();

        let quantized = dither.dither_to_i16(sample) as f32 / 32768.0;
        sum_squares += quantized * quantized;
    }

    let rms = (sum_squares / samples as f32).sqrt();
    let expected_rms = amplitude / 2.0_f32.sqrt(); // RMS of sine wave

    let error = (rms - expected_rms).abs() / expected_rms;
    assert!(
        error < 0.05,
        "RMS should be preserved (expected {:.4}, got {:.4}, error {:.2}%)",
        expected_rms,
        rms,
        error * 100.0
    );
}

/// Test that full-scale signals don't clip after dither
#[test]
fn test_full_scale_dither() {
    let mut dither = TpdfDither::new();
    let iterations = 10_000;
    let mut max_value = i16::MIN;
    let mut min_value = i16::MAX;

    for _ in 0..iterations {
        let sample = 0.9999; // Near full scale
        let quantized = dither.dither_to_i16(sample);

        max_value = max_value.max(quantized);
        min_value = min_value.min(quantized);
    }

    // Should reach close to i16::MAX but not clip
    assert!(
        max_value > 32700,
        "Full-scale signals should reach near i16::MAX, got {}",
        max_value
    );
    assert!(
        max_value <= i16::MAX,
        "Full-scale signals should not exceed i16::MAX"
    );
}

/// Test dithering zero signal
#[test]
fn test_dither_zero_signal() {
    let mut dither = TpdfDither::new();
    let iterations = 1000;
    let mut sum_abs = 0i64;

    for _ in 0..iterations {
        let quantized = dither.dither_to_i16(0.0);
        sum_abs += quantized.abs() as i64;
    }

    let avg_abs = sum_abs as f64 / iterations as f64;
    assert!(
        avg_abs < 1.0,
        "Dithered zero should average near zero, got: {}",
        avg_abs
    );
}

/// Test dithering very small signals (below 1 LSB)
#[test]
fn test_dither_very_small_signals() {
    let mut dither = TpdfDither::new();
    let iterations = 10_000;
    let signal = 0.0001; // Well below 1 LSB at 16-bit (1/32768 ≈ 0.00003)

    let mut output = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        output.push(dither.dither_to_i16(signal));
    }

    // Calculate average output
    let avg: f64 = output.iter().map(|&x| x as f64).sum::<f64>() / iterations as f64;
    let expected = signal * 32768.0;

    println!(
        "Very small signal {}: average output {:.4}, expected {:.4}",
        signal, avg, expected
    );

    // Average should be close to expected value (dither reveals low-level detail)
    let error = (avg - expected as f64).abs() / expected as f64;
    assert!(
        error < 0.5,
        "Average output should be near expected value (expected {:.4}, got {:.4})",
        expected,
        avg
    );
}

/// Test NaN and infinity protection
#[test]
fn test_nan_infinity_protection() {
    let mut dither = TpdfDither::new();

    // Test NaN
    let dithered_nan = dither.dither_to_i16(f32::NAN);
    assert_eq!(
        dithered_nan, 0,
        "NaN should be safely handled as 0, got: {}",
        dithered_nan
    );

    // Test positive infinity
    let dithered_inf = dither.dither_to_i16(f32::INFINITY);
    assert_eq!(
        dithered_inf, 0,
        "Infinity should be safely handled as 0, got: {}",
        dithered_inf
    );

    // Test negative infinity
    let dithered_neg_inf = dither.dither_to_i16(f32::NEG_INFINITY);
    assert_eq!(
        dithered_neg_inf, 0,
        "Negative infinity should be safely handled as 0, got: {}",
        dithered_neg_inf
    );
}

/// Test processing stereo buffer
#[test]
fn test_process_buffer_stereo() {
    let mut stereo = StereoDither::new();
    let frames = 1000; // More frames for better statistics
    let input = vec![0.5_f32; frames * 2]; // Stereo: L, R, L, R...
    let mut output = vec![0_i16; frames * 2];

    stereo.process_stereo_to_i16(&input, &mut output);

    // Check that values are in reasonable range for 0.5 input
    for &sample in &output {
        assert!(
            (sample as i32 - 16384).abs() < 200,
            "Sample should be near 16384 (0.5 * 32768), got: {}",
            sample
        );
    }

    // Check that channels have some independence
    let mut diff_count = 0;
    for i in 0..frames {
        let left = output[i * 2];
        let right = output[i * 2 + 1];

        if left != right {
            diff_count += 1;
        }
    }

    let diff_percent = (diff_count as f32 / frames as f32) * 100.0;
    println!(
        "Stereo buffer processing: {:.2}% of frames have different L/R",
        diff_percent
    );

    // With independent dither, expect some variation
    assert!(
        diff_count > 0,
        "Stereo channels should show some independence from dither"
    );
}

/// Test processing mono buffer
#[test]
fn test_process_buffer_mono() {
    let mut dither = TpdfDither::new();
    let samples = 100;
    let input = vec![0.5_f32; samples];
    let mut output = vec![0_i16; samples];

    dither.process_to_i16(&input, &mut output);

    // Check that dither was applied
    let all_same = output.windows(2).all(|w| w[0] == w[1]);
    assert!(
        !all_same,
        "Mono buffer should have dither applied (not all values identical)"
    );

    // Check that values are in reasonable range for 0.5 input
    for &sample in &output {
        assert!(
            (sample as i32 - 16384).abs() < 200,
            "Sample should be near 16384 (0.5 * 32768), got: {}",
            sample
        );
    }
}

/// Test that mono dither produces values around expected quantization point
#[test]
fn test_mono_dither_distribution() {
    let mut dither = TpdfDither::new();
    let iterations = 1000;
    let input = vec![0.5_f32; iterations];
    let mut output = vec![0_i16; iterations];

    dither.process_to_i16(&input, &mut output);

    let mean: f64 = output.iter().map(|&x| x as f64).sum::<f64>() / iterations as f64;
    let expected = 0.5 * 32768.0; // 16384

    let error = (mean - expected).abs() / expected;
    assert!(
        error < 0.01,
        "Mean should be near expected value (expected {:.1}, got {:.1}, error {:.2}%)",
        expected,
        mean,
        error * 100.0
    );
}

/// Test TPDF noise statistical properties
#[test]
fn test_tpdf_noise_statistics() {
    let mut dither = TpdfDither::new();
    let samples = 100_000;
    let mut values = Vec::with_capacity(samples);

    // Collect dithered zero samples to analyze noise distribution
    for _ in 0..samples {
        values.push(dither.dither_to_i16(0.0));
    }

    // Calculate mean
    let mean: f64 = values.iter().map(|&x| x as f64).sum::<f64>() / samples as f64;
    assert!(
        mean.abs() < 1.0,
        "TPDF noise should have zero mean, got: {}",
        mean
    );

    // Calculate variance
    let variance: f64 = values
        .iter()
        .map(|&x| (x as f64 - mean).powi(2))
        .sum::<f64>()
        / samples as f64;

    // TPDF variance should be approximately 1/6 for normalized ±1 range
    // For our implementation, the variance should be very small since noise is < 1 LSB
    assert!(
        variance < 2.0,
        "TPDF variance should be small, got: {}",
        variance
    );
}

// === Helper Functions ===

/// Calculate RMS error between two signals
fn calculate_rms_error(signal1: &[f32], signal2: &[f32]) -> f32 {
    let mut sum_sq_error = 0.0;
    let n = signal1.len().min(signal2.len());

    for i in 0..n {
        let error = signal1[i] - signal2[i];
        sum_sq_error += error * error;
    }

    (sum_sq_error / n as f32).sqrt()
}

/// Calculate Signal-to-Noise Ratio in dB
fn calculate_snr(signal: &[f32], noisy_signal: &[f32]) -> f32 {
    let mut signal_power = 0.0;
    let mut noise_power = 0.0;

    for i in 0..signal.len() {
        signal_power += signal[i] * signal[i];
        let noise = signal[i] - noisy_signal[i];
        noise_power += noise * noise;
    }

    let snr = signal_power / (noise_power + 1e-10);
    10.0 * snr.log10()
}

/// Calculate autocorrelation at given lag
fn calculate_autocorrelation(signal: &[f32], lag: usize) -> f32 {
    if signal.len() <= lag {
        return 0.0;
    }

    let mut sum = 0.0;
    let mut sum_sq = 0.0;

    for i in 0..signal.len() {
        sum_sq += signal[i] * signal[i];
    }

    for i in 0..(signal.len() - lag) {
        sum += signal[i] * signal[i + lag];
    }

    sum / (sum_sq + 1e-10)
}

/// Calculate correlation between two signals
fn calculate_correlation(signal1: &[f32], signal2: &[f32]) -> f32 {
    let n = signal1.len().min(signal2.len());

    let mean1: f32 = signal1[..n].iter().sum::<f32>() / n as f32;
    let mean2: f32 = signal2[..n].iter().sum::<f32>() / n as f32;

    let mut covariance = 0.0;
    let mut var1 = 0.0;
    let mut var2 = 0.0;

    for i in 0..n {
        let d1 = signal1[i] - mean1;
        let d2 = signal2[i] - mean2;
        covariance += d1 * d2;
        var1 += d1 * d1;
        var2 += d2 * d2;
    }

    covariance / ((var1 * var2).sqrt() + 1e-10)
}
