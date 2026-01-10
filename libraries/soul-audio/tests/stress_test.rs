//! Stress tests for audio processing
//!
//! Tests for:
//! - Long-running processing (simulating hours of playback)
//! - Rapid parameter changes during processing
//! - Rapid enable/disable toggling
//! - Memory stability over long runs
//! - Effect chain stability

use soul_audio::effects::{
    AudioEffect, Compressor, CompressorSettings, Crossfeed, CrossfeedPreset, EqBand, GraphicEq,
    GraphicEqPreset, Limiter, LimiterSettings, ParametricEq, StereoEnhancer, StereoSettings,
};
use std::f32::consts::PI;
use std::time::{Duration, Instant};

const SAMPLE_RATE: u32 = 44100;

/// Generate a stereo sine wave buffer
fn generate_stereo_sine(frequency: f32, sample_rate: u32, num_samples: usize) -> Vec<f32> {
    let mut buffer = Vec::with_capacity(num_samples * 2);
    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let sample = (2.0 * PI * frequency * t).sin();
        buffer.push(sample);
        buffer.push(sample);
    }
    buffer
}

/// Generate varied audio content (mix of frequencies)
fn generate_varied_audio(sample_rate: u32, num_samples: usize) -> Vec<f32> {
    let mut buffer = Vec::with_capacity(num_samples * 2);
    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let sample = 0.3 * (2.0 * PI * 440.0 * t).sin()
            + 0.2 * (2.0 * PI * 880.0 * t).sin()
            + 0.15 * (2.0 * PI * 220.0 * t).sin()
            + 0.1 * (2.0 * PI * 1760.0 * t).sin();
        buffer.push(sample);
        buffer.push(sample * 0.9); // Slight stereo difference
    }
    buffer
}

/// Calculate RMS of a buffer
fn calculate_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_squares: f32 = samples.iter().map(|x| x * x).sum();
    (sum_squares / samples.len() as f32).sqrt()
}

// ============================================================================
// LONG-RUNNING PROCESSING TESTS
// ============================================================================

#[test]
fn test_eq_long_run_1_minute_simulated() {
    let mut eq = ParametricEq::new();
    eq.set_low_band(EqBand::low_shelf(100.0, 3.0));
    eq.set_mid_band(EqBand::peaking(1000.0, -2.0, 1.0));
    eq.set_high_band(EqBand::high_shelf(10000.0, 1.0));

    // Simulate 1 minute at 44.1kHz with 512-sample buffers
    // 44100 * 60 / 512 ≈ 5168 buffers
    let num_buffers = 5168;
    let buffer_size = 512;

    for i in 0..num_buffers {
        let mut buffer = generate_stereo_sine(1000.0, SAMPLE_RATE, buffer_size);
        eq.process(&mut buffer, SAMPLE_RATE);

        // Check every 1000 buffers for stability
        if i % 1000 == 0 {
            for sample in &buffer {
                assert!(sample.is_finite(), "Sample became non-finite at buffer {}", i);
                assert!(
                    sample.abs() < 10.0,
                    "Sample amplitude grew unbounded at buffer {}",
                    i
                );
            }
        }
    }
}

#[test]
fn test_effect_chain_long_run_5_minutes_simulated() {
    let mut eq = ParametricEq::new();
    eq.set_low_band(EqBand::low_shelf(100.0, 3.0));
    eq.set_mid_band(EqBand::peaking(1000.0, -2.0, 1.0));
    eq.set_high_band(EqBand::high_shelf(10000.0, 1.0));

    let mut geq = GraphicEq::new_10_band();
    geq.set_preset(GraphicEqPreset::Rock);

    let mut stereo = StereoEnhancer::with_settings(StereoSettings::wide());

    let mut crossfeed = Crossfeed::new();
    crossfeed.set_preset(CrossfeedPreset::Natural);
    crossfeed.set_enabled(true);

    let mut limiter = Limiter::with_settings(LimiterSettings::default());

    // 5 minutes simulation
    let num_buffers = 5168 * 5;
    let buffer_size = 512;

    let start = Instant::now();

    for i in 0..num_buffers {
        let mut buffer = generate_varied_audio(SAMPLE_RATE, buffer_size);

        eq.process(&mut buffer, SAMPLE_RATE);
        geq.process(&mut buffer, SAMPLE_RATE);
        stereo.process(&mut buffer, SAMPLE_RATE);
        crossfeed.process(&mut buffer, SAMPLE_RATE);
        limiter.process(&mut buffer, SAMPLE_RATE);

        // Periodic stability check
        if i % 5000 == 0 {
            for sample in &buffer {
                assert!(sample.is_finite(), "Non-finite at buffer {}", i);
            }
        }
    }

    let elapsed = start.elapsed();
    println!(
        "Processed {} buffers ({} simulated minutes) in {:?}",
        num_buffers,
        num_buffers * buffer_size / SAMPLE_RATE as usize / 60,
        elapsed
    );
}

#[test]
fn test_compressor_long_run_stability() {
    let mut compressor = Compressor::with_settings(CompressorSettings {
        threshold_db: -20.0,
        ratio: 4.0,
        attack_ms: 10.0,
        release_ms: 100.0,
        knee_db: 3.0,
        makeup_gain_db: 0.0,
    });

    // 2 minutes simulation
    let num_buffers = 5168 * 2;

    for i in 0..num_buffers {
        // Vary the input level to exercise the compressor
        let amplitude = if i % 100 < 50 { 0.9 } else { 0.1 };
        let mut buffer: Vec<f32> = generate_stereo_sine(1000.0, SAMPLE_RATE, 512)
            .iter()
            .map(|s| s * amplitude)
            .collect();

        compressor.process(&mut buffer, SAMPLE_RATE);

        if i % 2000 == 0 {
            for sample in &buffer {
                assert!(sample.is_finite());
                assert!(sample.abs() <= 1.5); // Compressor might have some overshoot
            }
        }
    }
}

// ============================================================================
// RAPID PARAMETER CHANGE TESTS
// ============================================================================

#[test]
fn test_eq_rapid_band_changes() {
    let mut eq = ParametricEq::new();

    for i in 0..10000 {
        // Rapidly change EQ bands every buffer
        let gain = (i as f32 * 0.01).sin() * 12.0; // Oscillate -12 to +12 dB
        let freq = 100.0 + (i as f32 * 0.1).sin().abs() * 10000.0; // 100-10100 Hz
        let q = 0.5 + (i as f32 * 0.02).sin().abs() * 9.5; // 0.5-10 Q

        eq.set_mid_band(EqBand::peaking(freq, gain, q));

        let mut buffer = generate_stereo_sine(1000.0, SAMPLE_RATE, 256);
        eq.process(&mut buffer, SAMPLE_RATE);

        for sample in &buffer {
            assert!(sample.is_finite(), "Non-finite after rapid change at {}", i);
        }
    }
}

#[test]
fn test_graphic_eq_rapid_preset_changes() {
    let mut geq = GraphicEq::new_10_band();

    let presets = [
        GraphicEqPreset::Flat,
        GraphicEqPreset::BassBoost,
        GraphicEqPreset::TrebleBoost,
        GraphicEqPreset::VShape,
        GraphicEqPreset::Vocal,
    ];

    for i in 0..5000 {
        // Change preset every iteration
        geq.set_preset(presets[i % presets.len()]);

        let mut buffer = generate_varied_audio(SAMPLE_RATE, 256);
        geq.process(&mut buffer, SAMPLE_RATE);

        for sample in &buffer {
            assert!(sample.is_finite());
        }
    }
}

#[test]
fn test_graphic_eq_rapid_band_changes() {
    let mut geq = GraphicEq::new_10_band();

    for i in 0..5000 {
        // Rapidly change individual bands
        for band in 0..10 {
            let gain = ((i + band) as f32 * 0.1).sin() * 12.0;
            geq.set_band_gain(band, gain);
        }

        let mut buffer = generate_varied_audio(SAMPLE_RATE, 256);
        geq.process(&mut buffer, SAMPLE_RATE);

        for sample in &buffer {
            assert!(sample.is_finite());
        }
    }
}

#[test]
fn test_stereo_enhancer_rapid_width_changes() {
    let mut enhancer = StereoEnhancer::new();

    for i in 0..5000 {
        // Rapidly change width
        let width = (i as f32 * 0.01).sin().abs() * 3.0; // 0 to 3
        enhancer.set_width(width);

        let mut buffer = generate_varied_audio(SAMPLE_RATE, 256);
        enhancer.process(&mut buffer, SAMPLE_RATE);

        for sample in &buffer {
            assert!(sample.is_finite());
        }
    }
}

#[test]
fn test_compressor_rapid_threshold_changes() {
    let mut compressor = Compressor::new();

    for i in 0..5000 {
        // Rapidly change threshold
        let threshold = -40.0 + (i as f32 * 0.02).sin() * 30.0; // -40 to -10 dB
        compressor.set_threshold(threshold);

        let mut buffer = generate_varied_audio(SAMPLE_RATE, 256);
        compressor.process(&mut buffer, SAMPLE_RATE);

        for sample in &buffer {
            assert!(sample.is_finite());
        }
    }
}

#[test]
fn test_limiter_rapid_ceiling_changes() {
    let mut limiter = Limiter::new();

    for i in 0..5000 {
        // Rapidly change threshold (ceiling)
        let threshold = -6.0 + (i as f32 * 0.02).sin() * 5.0; // -6 to -1 dB
        limiter.set_threshold(threshold);

        let mut buffer = generate_varied_audio(SAMPLE_RATE, 256);
        limiter.process(&mut buffer, SAMPLE_RATE);

        for sample in &buffer {
            assert!(sample.is_finite());
            assert!(sample.abs() <= 1.0, "Limiter failed to limit");
        }
    }
}

// ============================================================================
// RAPID ENABLE/DISABLE TOGGLING
// ============================================================================

#[test]
fn test_eq_rapid_enable_disable() {
    let mut eq = ParametricEq::new();
    eq.set_low_band(EqBand::low_shelf(100.0, 6.0));

    for i in 0..10000 {
        eq.set_enabled(i % 2 == 0);

        let mut buffer = generate_stereo_sine(1000.0, SAMPLE_RATE, 256);
        eq.process(&mut buffer, SAMPLE_RATE);

        for sample in &buffer {
            assert!(sample.is_finite());
        }
    }
}

#[test]
fn test_all_effects_rapid_toggling() {
    let mut eq = ParametricEq::new();
    eq.set_low_band(EqBand::low_shelf(100.0, 3.0));

    let mut geq = GraphicEq::new_10_band();
    geq.set_preset(GraphicEqPreset::Rock);

    let mut stereo = StereoEnhancer::with_settings(StereoSettings::wide());

    let mut crossfeed = Crossfeed::new();
    crossfeed.set_preset(CrossfeedPreset::Natural);

    let mut limiter = Limiter::with_settings(LimiterSettings::brickwall());

    for i in 0..5000 {
        // Toggle each effect in a pattern
        eq.set_enabled(i % 2 == 0);
        geq.set_enabled(i % 3 == 0);
        stereo.set_enabled(i % 5 == 0);
        crossfeed.set_enabled(i % 7 == 0);
        limiter.set_enabled(i % 11 == 0);

        let mut buffer = generate_varied_audio(SAMPLE_RATE, 256);

        eq.process(&mut buffer, SAMPLE_RATE);
        geq.process(&mut buffer, SAMPLE_RATE);
        stereo.process(&mut buffer, SAMPLE_RATE);
        crossfeed.process(&mut buffer, SAMPLE_RATE);
        limiter.process(&mut buffer, SAMPLE_RATE);

        for sample in &buffer {
            assert!(sample.is_finite());
        }
    }
}

// ============================================================================
// MEMORY STABILITY
// ============================================================================

#[test]
fn test_no_memory_growth_long_run() {
    // This test verifies that processing many buffers doesn't cause
    // unbounded memory growth (within the effect's internal state)

    let mut eq = ParametricEq::new();
    eq.set_low_band(EqBand::low_shelf(100.0, 3.0));
    eq.set_mid_band(EqBand::peaking(1000.0, 6.0, 1.0));
    eq.set_high_band(EqBand::high_shelf(10000.0, -3.0));

    // Process many buffers
    for _ in 0..100000 {
        let mut buffer = generate_stereo_sine(1000.0, SAMPLE_RATE, 512);
        eq.process(&mut buffer, SAMPLE_RATE);
    }

    // If we got here without OOM, the test passes
    // The internal filter state should be fixed-size
}

#[test]
fn test_reset_clears_accumulated_state() {
    let mut eq = ParametricEq::new();
    eq.set_mid_band(EqBand::peaking(1000.0, 12.0, 10.0)); // High Q, high gain

    // Process to build up filter state
    for _ in 0..1000 {
        let mut buffer = generate_stereo_sine(1000.0, SAMPLE_RATE, 512);
        eq.process(&mut buffer, SAMPLE_RATE);
    }

    // Reset
    eq.reset();

    // Process a new buffer - should start fresh
    let mut buffer = generate_stereo_sine(1000.0, SAMPLE_RATE, 512);
    eq.process(&mut buffer, SAMPLE_RATE);

    for sample in &buffer {
        assert!(sample.is_finite());
        assert!(sample.abs() < 10.0);
    }
}

// ============================================================================
// EXTREME INPUT TESTS
// ============================================================================

#[test]
fn test_alternating_extreme_amplitudes() {
    let mut limiter = Limiter::with_settings(LimiterSettings::brickwall());

    for i in 0..5000 {
        let amplitude = if i % 2 == 0 { 10.0 } else { 0.001 };
        let mut buffer: Vec<f32> = generate_stereo_sine(1000.0, SAMPLE_RATE, 256)
            .iter()
            .map(|s| s * amplitude)
            .collect();

        limiter.process(&mut buffer, SAMPLE_RATE);

        for sample in &buffer {
            assert!(sample.is_finite());
            assert!(sample.abs() <= 1.0);
        }
    }
}

#[test]
fn test_random_amplitude_variations() {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut eq = ParametricEq::new();
    eq.set_mid_band(EqBand::peaking(1000.0, 6.0, 1.0));

    for i in 0..10000 {
        // Pseudo-random amplitude
        let mut hasher = DefaultHasher::new();
        i.hash(&mut hasher);
        let amplitude = (hasher.finish() % 1000) as f32 / 1000.0 * 2.0; // 0-2

        let mut buffer: Vec<f32> = generate_stereo_sine(1000.0, SAMPLE_RATE, 256)
            .iter()
            .map(|s| s * amplitude)
            .collect();

        eq.process(&mut buffer, SAMPLE_RATE);

        for sample in &buffer {
            assert!(sample.is_finite());
        }
    }
}

// ============================================================================
// TIMING AND PERFORMANCE
// ============================================================================

#[test]
fn test_consistent_processing_time() {
    let mut eq = ParametricEq::new();
    eq.set_low_band(EqBand::low_shelf(100.0, 3.0));
    eq.set_mid_band(EqBand::peaking(1000.0, -2.0, 1.0));
    eq.set_high_band(EqBand::high_shelf(10000.0, 1.0));

    let buffer_size = 512;
    let mut times = Vec::with_capacity(1000);

    // Warmup
    for _ in 0..100 {
        let mut buffer = generate_stereo_sine(1000.0, SAMPLE_RATE, buffer_size);
        eq.process(&mut buffer, SAMPLE_RATE);
    }

    // Measure
    for _ in 0..1000 {
        let mut buffer = generate_stereo_sine(1000.0, SAMPLE_RATE, buffer_size);
        let start = Instant::now();
        eq.process(&mut buffer, SAMPLE_RATE);
        times.push(start.elapsed());
    }

    let avg_ns = times.iter().map(|t| t.as_nanos()).sum::<u128>() / times.len() as u128;
    let max_ns = times.iter().map(|t| t.as_nanos()).max().unwrap();
    let min_ns = times.iter().map(|t| t.as_nanos()).min().unwrap();

    // Use 99th percentile instead of max to avoid OS scheduling outliers
    let mut sorted_times: Vec<u128> = times.iter().map(|t| t.as_nanos()).collect();
    sorted_times.sort();
    let p99_ns = sorted_times[(sorted_times.len() * 99) / 100];

    println!(
        "EQ Processing: avg={}ns, min={}ns, max={}ns, p99={}ns, max_ratio={:.1}x, p99_ratio={:.1}x",
        avg_ns,
        min_ns,
        max_ns,
        p99_ns,
        max_ns as f64 / avg_ns as f64,
        p99_ns as f64 / avg_ns as f64
    );

    // 99th percentile should not be too much more than average (consistent timing)
    // Max can have OS scheduling outliers, so we use p99 instead
    assert!(
        p99_ns < avg_ns * 10,
        "Processing time too variable (p99 = {}ns, avg = {}ns)",
        p99_ns,
        avg_ns
    );
}

#[test]
fn test_real_time_budget_compliance() {
    // Verify that a full effect chain can process within real-time budget
    let buffer_size = 512;
    let sample_rate = 48000;

    let mut eq = ParametricEq::new();
    eq.set_low_band(EqBand::low_shelf(100.0, 3.0));

    let mut geq = GraphicEq::new_10_band();
    geq.set_preset(GraphicEqPreset::Rock);

    let mut stereo = StereoEnhancer::with_settings(StereoSettings::wide());

    let mut crossfeed = Crossfeed::new();
    crossfeed.set_enabled(true);

    let mut compressor = Compressor::with_settings(CompressorSettings::default());

    let mut limiter = Limiter::with_settings(LimiterSettings::brickwall());

    // Real-time budget for this buffer
    let budget = Duration::from_secs_f64(buffer_size as f64 / sample_rate as f64);

    let mut over_budget_count = 0;

    for _ in 0..1000 {
        let mut buffer = generate_varied_audio(sample_rate, buffer_size);

        let start = Instant::now();
        eq.process(&mut buffer, sample_rate);
        geq.process(&mut buffer, sample_rate);
        stereo.process(&mut buffer, sample_rate);
        crossfeed.process(&mut buffer, sample_rate);
        compressor.process(&mut buffer, sample_rate);
        limiter.process(&mut buffer, sample_rate);
        let elapsed = start.elapsed();

        if elapsed > budget {
            over_budget_count += 1;
        }
    }

    // Allow some variance but most should be within budget
    assert!(
        over_budget_count < 10,
        "Too many buffers over real-time budget: {}/1000",
        over_budget_count
    );
}
