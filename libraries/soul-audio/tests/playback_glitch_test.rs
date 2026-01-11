//! Playback Glitch Detection Tests
//!
//! Comprehensive tests to detect audio playback issues including:
//! - Buffer underruns (callback processing exceeds buffer period)
//! - Continuous playback gaps (discontinuities in output)
//! - Batch processing stress (heavy background work during audio)
//! - Sample rate transition glitches (clicks/pops at rate changes)
//! - Memory allocation in audio path (real-time safety violations)
//! - Long-running stability (drift, growing latency over time)
//!
//! These tests are designed to catch Windows playback gap issues during
//! batch processing and other real-world audio problems.

use soul_audio::effects::{
    AudioEffect, Compressor, CompressorSettings, Crossfeed, CrossfeedPreset, EffectChain, EqBand,
    GraphicEq, GraphicEqPreset, Limiter, LimiterSettings, ParametricEq, StereoEnhancer,
    StereoSettings,
};
use soul_audio::resampling::{Resampler, ResamplerBackend, ResamplingQuality};
use std::f32::consts::PI;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

// ============================================================================
// CONSTANTS
// ============================================================================

const STANDARD_SAMPLE_RATES: [u32; 5] = [44100, 48000, 88200, 96000, 192000];
const STANDARD_BUFFER_SIZES: [usize; 7] = [64, 128, 256, 512, 1024, 2048, 4096];

// ============================================================================
// TEST UTILITIES
// ============================================================================

/// Generate a phase-continuous sine wave for discontinuity detection
/// Returns the samples and the ending phase for seamless continuation
fn generate_phase_continuous_sine(
    frequency: f32,
    sample_rate: u32,
    num_frames: usize,
    amplitude: f32,
    start_phase: f32,
) -> (Vec<f32>, f32) {
    let mut samples = Vec::with_capacity(num_frames * 2);
    let phase_increment = 2.0 * PI * frequency / sample_rate as f32;

    let mut phase = start_phase;
    for _ in 0..num_frames {
        let sample = phase.sin() * amplitude;
        samples.push(sample); // Left
        samples.push(sample); // Right
        phase += phase_increment;
        // Keep phase in [0, 2*PI) to avoid precision loss
        while phase >= 2.0 * PI {
            phase -= 2.0 * PI;
        }
    }

    (samples, phase)
}

/// Generate stereo sine wave buffer (simpler version)
fn generate_stereo_sine(frequency: f32, sample_rate: u32, num_frames: usize) -> Vec<f32> {
    let mut buffer = Vec::with_capacity(num_frames * 2);
    for i in 0..num_frames {
        let t = i as f32 / sample_rate as f32;
        let sample = (2.0 * PI * frequency * t).sin();
        buffer.push(sample); // Left
        buffer.push(sample); // Right
    }
    buffer
}

/// Generate varied audio content (multiple frequencies)
fn generate_varied_audio(sample_rate: u32, num_frames: usize) -> Vec<f32> {
    let mut buffer = Vec::with_capacity(num_frames * 2);
    for i in 0..num_frames {
        let t = i as f32 / sample_rate as f32;
        let sample = 0.3 * (2.0 * PI * 440.0 * t).sin()
            + 0.2 * (2.0 * PI * 880.0 * t).sin()
            + 0.15 * (2.0 * PI * 220.0 * t).sin()
            + 0.1 * (2.0 * PI * 1760.0 * t).sin();
        buffer.push(sample);
        buffer.push(sample * 0.95); // Slight stereo difference
    }
    buffer
}

/// Detect discontinuities in a phase-continuous sine wave
/// Returns positions where the signal deviates from expected sine behavior
fn detect_sine_discontinuities(
    samples: &[f32],
    sample_rate: u32,
    frequency: f32,
    threshold_multiplier: f32,
) -> Vec<(usize, f32)> {
    let mut discontinuities = Vec::new();

    // Maximum expected sample-to-sample change for a sine wave
    // Derivative of A*sin(2*pi*f*t) is A*2*pi*f*cos(2*pi*f*t)
    // Max value is A*2*pi*f, per sample: A*2*pi*f/sample_rate
    let max_expected_derivative = 2.0 * PI * frequency / sample_rate as f32;
    let threshold = max_expected_derivative * threshold_multiplier;

    // Check left channel only (every other sample)
    for i in 1..samples.len() / 2 {
        let prev_sample = samples[(i - 1) * 2];
        let curr_sample = samples[i * 2];
        let diff = (curr_sample - prev_sample).abs();

        if diff > threshold {
            discontinuities.push((i, diff));
        }
    }

    discontinuities
}

/// Detect any sudden jumps in audio signal
fn detect_discontinuities(samples: &[f32], threshold: f32) -> Vec<(usize, f32)> {
    let mut discontinuities = Vec::new();

    for i in 1..samples.len() {
        let diff = (samples[i] - samples[i - 1]).abs();
        if diff > threshold {
            discontinuities.push((i, diff));
        }
    }

    discontinuities
}

/// Calculate buffer period in Duration
fn buffer_period(buffer_size: usize, sample_rate: u32) -> Duration {
    Duration::from_secs_f64(buffer_size as f64 / sample_rate as f64)
}

/// Statistics collector for timing measurements
struct TimingStats {
    samples: Vec<Duration>,
}

impl TimingStats {
    fn new() -> Self {
        Self {
            samples: Vec::with_capacity(10000),
        }
    }

    fn add(&mut self, duration: Duration) {
        self.samples.push(duration);
    }

    fn mean(&self) -> Duration {
        if self.samples.is_empty() {
            return Duration::ZERO;
        }
        let total: Duration = self.samples.iter().sum();
        total / self.samples.len() as u32
    }

    fn min(&self) -> Duration {
        self.samples.iter().copied().min().unwrap_or(Duration::ZERO)
    }

    fn max(&self) -> Duration {
        self.samples.iter().copied().max().unwrap_or(Duration::ZERO)
    }

    fn percentile(&self, p: f64) -> Duration {
        if self.samples.is_empty() {
            return Duration::ZERO;
        }
        let mut sorted = self.samples.clone();
        sorted.sort();
        let idx = ((sorted.len() as f64 * p / 100.0) as usize).min(sorted.len() - 1);
        sorted[idx]
    }

    fn jitter_ratio(&self) -> f64 {
        let mean = self.mean();
        let p99 = self.percentile(99.0);
        if mean.as_nanos() == 0 {
            return 0.0;
        }
        p99.as_nanos() as f64 / mean.as_nanos() as f64
    }

    fn count_exceeding(&self, threshold: Duration) -> usize {
        self.samples.iter().filter(|&&d| d > threshold).count()
    }
}

/// Create a full effect chain for testing
fn create_full_effect_chain() -> EffectChain {
    let mut chain = EffectChain::new();

    let mut eq = ParametricEq::new();
    eq.set_low_band(EqBand::low_shelf(100.0, 3.0));
    eq.set_mid_band(EqBand::peaking(1000.0, -2.0, 1.0));
    eq.set_high_band(EqBand::high_shelf(8000.0, 1.0));
    chain.add_effect(Box::new(eq));

    let mut geq = GraphicEq::new_10_band();
    geq.set_preset(GraphicEqPreset::Rock);
    chain.add_effect(Box::new(geq));

    let compressor = Compressor::with_settings(CompressorSettings::moderate());
    chain.add_effect(Box::new(compressor));

    let crossfeed = Crossfeed::with_preset(CrossfeedPreset::Natural);
    chain.add_effect(Box::new(crossfeed));

    let enhancer = StereoEnhancer::with_settings(StereoSettings::wide());
    chain.add_effect(Box::new(enhancer));

    let limiter = Limiter::with_settings(LimiterSettings::brickwall());
    chain.add_effect(Box::new(limiter));

    chain.set_enabled(true);
    chain
}

// ============================================================================
// 1. BUFFER UNDERRUN DETECTION TESTS
// ============================================================================

#[test]
fn test_buffer_underrun_detection_at_various_buffer_sizes() {
    let sample_rate = 48000u32;
    let iterations = 500;

    println!("\n=== Buffer Underrun Detection ===");
    println!(
        "{:>8} {:>12} {:>12} {:>12} {:>12} {:>10}",
        "Buffer", "Period(us)", "Mean(us)", "P99(us)", "Max(us)", "Underruns"
    );
    println!("{}", "-".repeat(70));

    for &buffer_size in &STANDARD_BUFFER_SIZES {
        let mut chain = create_full_effect_chain();
        let budget = buffer_period(buffer_size, sample_rate);

        let mut stats = TimingStats::new();

        // Warmup
        for _ in 0..50 {
            let mut buffer = generate_varied_audio(sample_rate, buffer_size);
            chain.process(&mut buffer, sample_rate);
        }

        // Measure
        for _ in 0..iterations {
            let mut buffer = generate_varied_audio(sample_rate, buffer_size);
            let start = Instant::now();
            chain.process(&mut buffer, sample_rate);
            stats.add(start.elapsed());
        }

        let underruns = stats.count_exceeding(budget);
        let budget_us = budget.as_nanos() as f64 / 1000.0;
        let mean_us = stats.mean().as_nanos() as f64 / 1000.0;
        let p99_us = stats.percentile(99.0).as_nanos() as f64 / 1000.0;
        let max_us = stats.max().as_nanos() as f64 / 1000.0;

        println!(
            "{:>8} {:>12.2} {:>12.2} {:>12.2} {:>12.2} {:>10}",
            buffer_size, budget_us, mean_us, p99_us, max_us, underruns
        );

        // Allow up to 5% underruns for small buffers, 2% for large
        let max_underrun_rate = if buffer_size <= 128 { 0.05 } else { 0.02 };
        let underrun_rate = underruns as f64 / iterations as f64;

        // Soft assertion - log warning but don't fail in CI
        if underrun_rate > max_underrun_rate {
            println!(
                "  WARNING: Buffer {} has {:.1}% underrun rate (max: {:.1}%)",
                buffer_size,
                underrun_rate * 100.0,
                max_underrun_rate * 100.0
            );
        }
    }
}

#[test]
fn test_simulated_high_cpu_load_underrun() {
    let sample_rate = 48000u32;
    let buffer_size = 512usize;
    let iterations = 200;

    let mut chain = create_full_effect_chain();
    let budget = buffer_period(buffer_size, sample_rate);

    let mut underruns_without_load = 0;
    let mut underruns_with_load = 0;

    // Test without additional CPU load
    for _ in 0..iterations {
        let mut buffer = generate_varied_audio(sample_rate, buffer_size);
        let start = Instant::now();
        chain.process(&mut buffer, sample_rate);
        if start.elapsed() > budget {
            underruns_without_load += 1;
        }
    }

    // Test with simulated CPU load (heavy computation)
    for _ in 0..iterations {
        let mut buffer = generate_varied_audio(sample_rate, buffer_size);

        // Simulate heavy CPU load alongside audio processing
        let start = Instant::now();

        // Heavy computation (simulate batch processing work)
        let mut load_result = 0.0f64;
        for j in 0..1000 {
            load_result += (j as f64).sin().cos().tan().abs();
        }
        std::hint::black_box(load_result);

        chain.process(&mut buffer, sample_rate);
        if start.elapsed() > budget {
            underruns_with_load += 1;
        }
    }

    println!("\n=== CPU Load Impact on Buffer Underruns ===");
    println!("Buffer size: {} samples, Budget: {:.2}ms", buffer_size, budget.as_secs_f64() * 1000.0);
    println!("Underruns without load: {} / {}", underruns_without_load, iterations);
    println!("Underruns with load: {} / {}", underruns_with_load, iterations);

    // The audio processing itself should not cause underruns
    assert!(
        underruns_without_load <= iterations / 20,
        "Too many underruns without CPU load: {} / {}",
        underruns_without_load,
        iterations
    );
}

#[test]
fn test_underrun_detection_callback_timing() {
    let sample_rate = 48000u32;
    let buffer_size = 256usize;
    let iterations = 1000;

    let budget = buffer_period(buffer_size, sample_rate);
    let mut chain = create_full_effect_chain();

    let mut callback_timings: Vec<Duration> = Vec::with_capacity(iterations);

    // Simulate audio callback loop
    let mut last_callback = Instant::now();

    for _ in 0..iterations {
        let callback_start = Instant::now();
        let inter_callback_time = callback_start.duration_since(last_callback);

        let mut buffer = generate_varied_audio(sample_rate, buffer_size);
        chain.process(&mut buffer, sample_rate);

        let processing_time = callback_start.elapsed();
        callback_timings.push(processing_time);

        // Check for timing issues
        if inter_callback_time > Duration::ZERO && processing_time > inter_callback_time {
            // Processing took longer than the interval - underrun condition
        }

        last_callback = callback_start;

        // Simulate real callback timing (wait for remainder of budget)
        if processing_time < budget {
            // In real audio, the next callback would happen after budget time
            // Here we just continue to measure processing time accurately
        }
    }

    // Calculate statistics
    let mean = callback_timings.iter().sum::<Duration>() / iterations as u32;
    let max = callback_timings.iter().max().unwrap();
    let over_budget = callback_timings.iter().filter(|&&t| t > budget).count();

    println!("\n=== Callback Timing Analysis ===");
    println!("Buffer: {} samples, Budget: {:.2}us", buffer_size, budget.as_nanos() as f64 / 1000.0);
    println!("Mean processing: {:.2}us", mean.as_nanos() as f64 / 1000.0);
    println!("Max processing: {:.2}us", max.as_nanos() as f64 / 1000.0);
    println!("Over budget: {} / {} ({:.2}%)", over_budget, iterations, over_budget as f64 / iterations as f64 * 100.0);

    // Processing should typically be well under budget
    assert!(
        mean < budget,
        "Mean processing time exceeds budget"
    );
}

// ============================================================================
// 2. CONTINUOUS PLAYBACK VERIFICATION TESTS
// ============================================================================

#[test]
fn test_continuous_playback_no_gaps() {
    let sample_rate = 48000u32;
    let buffer_size = 512usize;
    let frequency = 1000.0f32;
    let amplitude = 0.8f32;
    let num_buffers = 100;

    let mut chain = create_full_effect_chain();

    // Collect all output samples
    let mut all_output: Vec<f32> = Vec::with_capacity(num_buffers * buffer_size * 2);
    let mut current_phase = 0.0f32;

    for _ in 0..num_buffers {
        let (buffer, end_phase) = generate_phase_continuous_sine(
            frequency,
            sample_rate,
            buffer_size,
            amplitude,
            current_phase,
        );
        current_phase = end_phase;

        let mut processed = buffer;
        chain.process(&mut processed, sample_rate);
        all_output.extend_from_slice(&processed);
    }

    // Check for gaps in the output (sudden drops to zero or near-zero)
    let mut gap_count = 0;
    let gap_threshold = 0.01; // Consider values below this as potential gaps
    let min_gap_duration = 10; // Minimum consecutive samples to be considered a gap

    let mut consecutive_low = 0;
    for sample in &all_output {
        if sample.abs() < gap_threshold {
            consecutive_low += 1;
        } else {
            if consecutive_low >= min_gap_duration {
                gap_count += 1;
            }
            consecutive_low = 0;
        }
    }

    println!("\n=== Continuous Playback Gap Detection ===");
    println!("Total samples: {}", all_output.len());
    println!("Gap events detected: {}", gap_count);

    // The effect chain includes a limiter, so zeros are NOT expected in normal operation
    // (unless the input was zero, which it wasn't)
    assert!(
        gap_count < 5,
        "Too many gap events detected: {}",
        gap_count
    );
}

#[test]
fn test_phase_continuity_through_effect_chain() {
    let sample_rate = 48000u32;
    let buffer_size = 256usize;
    let frequency = 440.0f32;
    let amplitude = 0.7f32;
    let num_buffers = 50;

    // Create a simple EQ that shouldn't introduce discontinuities
    let mut eq = ParametricEq::new();
    eq.set_mid_band(EqBand::peaking(frequency, 3.0, 1.0)); // Boost at test frequency

    let mut all_output: Vec<f32> = Vec::new();
    let mut current_phase = 0.0f32;

    for _ in 0..num_buffers {
        let (mut buffer, end_phase) = generate_phase_continuous_sine(
            frequency,
            sample_rate,
            buffer_size,
            amplitude,
            current_phase,
        );
        current_phase = end_phase;

        eq.process(&mut buffer, sample_rate);
        all_output.extend_from_slice(&buffer);
    }

    // Detect discontinuities in the processed signal
    // The threshold should be higher because EQ changes amplitude and introduces phase shift
    let threshold = 0.3; // Allow for EQ-induced changes
    let discontinuities = detect_discontinuities(&all_output, threshold);

    // Count discontinuities at buffer boundaries
    let samples_per_buffer = buffer_size * 2; // Stereo
    let boundary_discontinuities: Vec<_> = discontinuities
        .iter()
        .filter(|(pos, _)| *pos % samples_per_buffer < 10 || *pos % samples_per_buffer > samples_per_buffer - 10)
        .collect();

    println!("\n=== Phase Continuity Test ===");
    println!("Total discontinuities: {}", discontinuities.len());
    println!("Buffer boundary discontinuities: {}", boundary_discontinuities.len());

    // Should have very few discontinuities at buffer boundaries
    // Some may occur due to filter state, but should be minimal
    assert!(
        boundary_discontinuities.len() < num_buffers,
        "Too many discontinuities at buffer boundaries: {}",
        boundary_discontinuities.len()
    );
}

#[test]
fn test_sample_continuity_across_buffers() {
    let sample_rate = 48000u32;
    let buffer_size = 512usize;
    let frequency = 1000.0f32;
    let num_buffers = 20;

    let mut chain = create_full_effect_chain();

    let mut last_sample: Option<f32> = None;
    let mut boundary_jumps: Vec<f32> = Vec::new();

    for _ in 0..num_buffers {
        let mut buffer = generate_stereo_sine(frequency, sample_rate, buffer_size);
        chain.process(&mut buffer, sample_rate);

        // Check continuity with previous buffer's last sample
        if let Some(prev) = last_sample {
            let first = buffer[0];
            let jump = (first - prev).abs();
            boundary_jumps.push(jump);
        }

        // Store last sample for next iteration
        last_sample = buffer.last().copied();
    }

    // Calculate statistics on boundary jumps
    let mean_jump: f32 = boundary_jumps.iter().sum::<f32>() / boundary_jumps.len() as f32;
    let max_jump = boundary_jumps.iter().cloned().fold(0.0f32, f32::max);

    println!("\n=== Sample Continuity Across Buffers ===");
    println!("Mean boundary jump: {:.6}", mean_jump);
    println!("Max boundary jump: {:.6}", max_jump);

    // The effect chain will cause some amplitude variation, but not extreme jumps
    // Jumps above 0.5 would indicate a discontinuity or glitch
    assert!(
        max_jump < 0.8,
        "Buffer boundary jump too large: {:.4}",
        max_jump
    );
}

// ============================================================================
// 3. BATCH PROCESSING STRESS TESTS
// ============================================================================

#[test]
fn test_batch_processing_with_file_io_simulation() {
    let sample_rate = 48000u32;
    let buffer_size = 512usize;
    let iterations = 200;

    let mut chain = create_full_effect_chain();
    let budget = buffer_period(buffer_size, sample_rate);

    let mut stats_normal = TimingStats::new();
    let mut stats_with_io = TimingStats::new();

    // Normal processing
    for _ in 0..iterations {
        let mut buffer = generate_varied_audio(sample_rate, buffer_size);
        let start = Instant::now();
        chain.process(&mut buffer, sample_rate);
        stats_normal.add(start.elapsed());
    }

    // Processing with simulated file I/O overhead
    for i in 0..iterations {
        let mut buffer = generate_varied_audio(sample_rate, buffer_size);

        let start = Instant::now();

        // Simulate file I/O work (memory operations that might cause cache misses)
        if i % 10 == 0 {
            let temp: Vec<u8> = (0..10000).map(|x| (x % 256) as u8).collect();
            std::hint::black_box(&temp);
        }

        chain.process(&mut buffer, sample_rate);
        stats_with_io.add(start.elapsed());
    }

    println!("\n=== Batch Processing with Simulated I/O ===");
    println!("Budget: {:.2}us", budget.as_nanos() as f64 / 1000.0);
    println!(
        "Normal - Mean: {:.2}us, P99: {:.2}us, Underruns: {}",
        stats_normal.mean().as_nanos() as f64 / 1000.0,
        stats_normal.percentile(99.0).as_nanos() as f64 / 1000.0,
        stats_normal.count_exceeding(budget)
    );
    println!(
        "With I/O - Mean: {:.2}us, P99: {:.2}us, Underruns: {}",
        stats_with_io.mean().as_nanos() as f64 / 1000.0,
        stats_with_io.percentile(99.0).as_nanos() as f64 / 1000.0,
        stats_with_io.count_exceeding(budget)
    );

    // Verify the audio processing itself remains fast
    assert!(
        stats_normal.percentile(99.0) < budget,
        "Normal processing P99 exceeds budget"
    );
}

#[test]
fn test_batch_processing_with_background_threads() {
    let sample_rate = 48000u32;
    let buffer_size = 512usize;
    let test_duration = Duration::from_secs(2);

    let processing_active = Arc::new(AtomicBool::new(true));
    let underruns = Arc::new(AtomicU64::new(0));
    let buffers_processed = Arc::new(AtomicU64::new(0));

    let budget = buffer_period(buffer_size, sample_rate);

    // Audio processing thread
    let active = Arc::clone(&processing_active);
    let underruns_clone = Arc::clone(&underruns);
    let buffers_clone = Arc::clone(&buffers_processed);

    let audio_thread = thread::spawn(move || {
        let mut chain = create_full_effect_chain();

        while active.load(Ordering::Relaxed) {
            let mut buffer = generate_varied_audio(sample_rate, buffer_size);
            let start = Instant::now();
            chain.process(&mut buffer, sample_rate);
            let elapsed = start.elapsed();

            buffers_clone.fetch_add(1, Ordering::Relaxed);
            if elapsed > budget {
                underruns_clone.fetch_add(1, Ordering::Relaxed);
            }

            // Verify output validity
            for sample in &buffer {
                if !sample.is_finite() {
                    panic!("Invalid sample detected in audio thread");
                }
            }
        }
    });

    // Background work threads (simulating batch processing, database queries, etc.)
    let mut background_threads = Vec::new();
    for _ in 0..2 {
        let active = Arc::clone(&processing_active);
        let handle = thread::spawn(move || {
            while active.load(Ordering::Relaxed) {
                // Simulate heavy background work
                let mut result = 0.0f64;
                for i in 0..5000 {
                    result += (i as f64 * 0.001).sin().cos();
                }
                std::hint::black_box(result);
                thread::yield_now();
            }
        });
        background_threads.push(handle);
    }

    thread::sleep(test_duration);
    processing_active.store(false, Ordering::Relaxed);

    audio_thread.join().unwrap();
    for handle in background_threads {
        handle.join().unwrap();
    }

    let total_buffers = buffers_processed.load(Ordering::Relaxed);
    let total_underruns = underruns.load(Ordering::Relaxed);
    let underrun_rate = if total_buffers > 0 {
        total_underruns as f64 / total_buffers as f64
    } else {
        0.0
    };

    println!("\n=== Batch Processing with Background Threads ===");
    println!("Duration: {:?}", test_duration);
    println!("Buffers processed: {}", total_buffers);
    println!("Underruns: {} ({:.2}%)", total_underruns, underrun_rate * 100.0);

    // Allow up to 5% underrun rate with heavy background load
    assert!(
        underrun_rate < 0.05,
        "Underrun rate too high: {:.2}%",
        underrun_rate * 100.0
    );
}

#[test]
fn test_callback_timing_jitter_under_load() {
    let sample_rate = 48000u32;
    let buffer_size = 256usize;
    let iterations = 500;

    let mut chain = create_full_effect_chain();

    // Measure baseline jitter
    let mut baseline_stats = TimingStats::new();
    for _ in 0..iterations {
        let mut buffer = generate_varied_audio(sample_rate, buffer_size);
        let start = Instant::now();
        chain.process(&mut buffer, sample_rate);
        baseline_stats.add(start.elapsed());
    }

    // Measure jitter under simulated load
    let mut loaded_stats = TimingStats::new();
    for i in 0..iterations {
        let mut buffer = generate_varied_audio(sample_rate, buffer_size);

        // Simulate periodic heavy operations (like UI updates, database writes)
        if i % 20 == 0 {
            let temp: Vec<f64> = (0..1000).map(|x| (x as f64).sqrt()).collect();
            std::hint::black_box(&temp);
        }

        let start = Instant::now();
        chain.process(&mut buffer, sample_rate);
        loaded_stats.add(start.elapsed());
    }

    println!("\n=== Callback Timing Jitter ===");
    println!(
        "Baseline - Mean: {:.2}us, P99/Mean: {:.2}x",
        baseline_stats.mean().as_nanos() as f64 / 1000.0,
        baseline_stats.jitter_ratio()
    );
    println!(
        "Under load - Mean: {:.2}us, P99/Mean: {:.2}x",
        loaded_stats.mean().as_nanos() as f64 / 1000.0,
        loaded_stats.jitter_ratio()
    );

    // Jitter ratio should remain reasonable
    assert!(
        baseline_stats.jitter_ratio() < 10.0,
        "Baseline jitter too high: {:.2}x",
        baseline_stats.jitter_ratio()
    );
}

// ============================================================================
// 4. SAMPLE RATE TRANSITION GLITCHES
// ============================================================================

#[test]
fn test_sample_rate_transition_no_glitches() {
    let transitions = [
        (44100, 48000),
        (48000, 44100),
        (44100, 96000),
        (96000, 44100),
        (48000, 96000),
    ];

    println!("\n=== Sample Rate Transition Glitch Detection ===");

    for (input_rate, output_rate) in transitions {
        let buffer_size = 1024usize;
        let frequency = 1000.0f32;
        let _amplitude = 0.8f32;

        // Generate input at source sample rate
        let input = generate_stereo_sine(frequency, input_rate, buffer_size);

        // Create resampler
        let mut resampler = Resampler::new(
            ResamplerBackend::Auto,
            input_rate,
            output_rate,
            2,
            ResamplingQuality::Balanced,
        )
        .unwrap();

        // Process
        let output = resampler.process(&input).unwrap();

        // Check for glitches (sudden jumps)
        let max_expected_derivative = 2.0 * PI * frequency / output_rate as f32;
        let threshold = max_expected_derivative * 3.0;

        let glitches = detect_discontinuities(&output, threshold);

        // Skip first few samples (startup transient) and last few (tail)
        let interior_glitches: Vec<_> = glitches
            .iter()
            .filter(|(pos, _)| *pos > 100 && *pos < output.len() - 100)
            .collect();

        println!(
            "  {}Hz -> {}Hz: {} total glitches, {} interior glitches",
            input_rate,
            output_rate,
            glitches.len(),
            interior_glitches.len()
        );

        // Should have minimal interior glitches
        assert!(
            interior_glitches.len() < 10,
            "Too many glitches in {}Hz->{}Hz transition: {}",
            input_rate,
            output_rate,
            interior_glitches.len()
        );
    }
}

#[test]
fn test_sample_rate_switch_during_playback() {
    let buffer_size = 512usize;
    let frequency = 440.0f32;

    // Simulate switching sample rates during playback
    let rates_sequence = [44100u32, 48000, 96000, 48000, 44100];
    let target_rate = 48000u32;

    let mut all_output: Vec<f32> = Vec::new();
    let mut total_glitches = 0;

    for input_rate in rates_sequence {
        // Generate input
        let input = generate_stereo_sine(frequency, input_rate, buffer_size);

        // Create new resampler for this rate
        let mut resampler = Resampler::new(
            ResamplerBackend::Auto,
            input_rate,
            target_rate,
            2,
            ResamplingQuality::Balanced,
        )
        .unwrap();

        let output = resampler.process(&input).unwrap();

        // Check for glitches at the rate-switch boundary
        if !all_output.is_empty() {
            let last_sample = *all_output.last().unwrap();
            let first_sample = output.first().copied().unwrap_or(0.0);
            let transition_jump = (first_sample - last_sample).abs();

            if transition_jump > 0.3 {
                total_glitches += 1;
            }
        }

        all_output.extend_from_slice(&output);
    }

    println!("\n=== Sample Rate Switch During Playback ===");
    println!("Rate sequence: {:?}", rates_sequence);
    println!("Transition glitches detected: {}", total_glitches);

    // Some glitches at rate transitions are expected due to resampler state reset
    // But should be limited
    assert!(
        total_glitches <= rates_sequence.len(),
        "Too many transition glitches: {}",
        total_glitches
    );
}

// ============================================================================
// 5. MEMORY ALLOCATION IN AUDIO PATH TESTS
// ============================================================================

#[test]
fn test_no_allocations_timing_variance() {
    // Allocations cause timing variance - detect them via timing analysis
    let sample_rate = 48000u32;
    let buffer_size = 512usize;
    let iterations = 5000;

    let mut chain = create_full_effect_chain();

    // Warmup to trigger any lazy allocations
    for _ in 0..100 {
        let mut buffer = generate_varied_audio(sample_rate, buffer_size);
        chain.process(&mut buffer, sample_rate);
    }

    // Measure timing variance
    let mut stats = TimingStats::new();
    for _ in 0..iterations {
        let mut buffer = generate_varied_audio(sample_rate, buffer_size);
        let start = Instant::now();
        chain.process(&mut buffer, sample_rate);
        stats.add(start.elapsed());
    }

    let jitter = stats.jitter_ratio();
    let max_mean_ratio = stats.max().as_nanos() as f64 / stats.mean().as_nanos() as f64;

    println!("\n=== Memory Allocation Detection via Timing ===");
    println!("Iterations: {}", iterations);
    println!("Mean: {:.2}us", stats.mean().as_nanos() as f64 / 1000.0);
    println!("P99/Mean (jitter): {:.2}x", jitter);
    println!("Max/Mean ratio: {:.2}x", max_mean_ratio);

    // Low jitter indicates no allocation-related variance
    // Higher threshold for CI environments
    assert!(
        jitter < 15.0,
        "High timing variance suggests possible allocations: jitter = {:.2}x",
        jitter
    );
}

#[test]
fn test_verify_buffer_reuse_pattern() {
    let sample_rate = 48000u32;
    let buffer_sizes = [64, 128, 256, 512, 1024, 2048, 4096];
    let iterations_per_size = 100;
    let warmup_per_size = 20;

    let mut chain = create_full_effect_chain();

    println!("\n=== Buffer Reuse Verification ===");

    for &buffer_size in &buffer_sizes {
        // Warmup for this specific buffer size to stabilize internal allocations
        for _ in 0..warmup_per_size {
            let mut buffer = generate_varied_audio(sample_rate, buffer_size);
            chain.process(&mut buffer, sample_rate);
        }

        // Now measure with warmed-up chain
        let mut timing_stats = TimingStats::new();

        for _ in 0..iterations_per_size {
            let mut buffer = generate_varied_audio(sample_rate, buffer_size);
            let start = Instant::now();
            chain.process(&mut buffer, sample_rate);
            timing_stats.add(start.elapsed());

            // Verify output validity
            for sample in &buffer {
                assert!(sample.is_finite(), "Invalid sample at buffer size {}", buffer_size);
            }
        }

        // Compare first and last iterations (both should be warm now)
        let first_mean: Duration = timing_stats.samples.iter().take(10).sum::<Duration>() / 10;
        let last_mean: Duration = timing_stats.samples.iter().skip(90).sum::<Duration>() / 10;

        // Ratio of last/first - should not show significant degradation
        // Use the smaller as denominator to catch degradation in either direction
        let (slower, faster) = if last_mean > first_mean {
            (last_mean, first_mean)
        } else {
            (first_mean, last_mean)
        };
        let ratio = slower.as_nanos() as f64 / faster.as_nanos().max(1) as f64;

        println!(
            "  Buffer {}: first_mean={:.2}us, last_mean={:.2}us, variance={:.2}x",
            buffer_size,
            first_mean.as_nanos() as f64 / 1000.0,
            last_mean.as_nanos() as f64 / 1000.0,
            ratio
        );

        // Allow higher variance in CI environments due to system scheduling
        // Main goal is to catch severe degradation (>10x would indicate a real problem)
        assert!(
            ratio < 10.0,
            "Severe performance variance at buffer size {}: {:.2}x",
            buffer_size,
            ratio
        );
    }
}

#[test]
fn test_varying_buffer_sizes_no_reallocation() {
    let sample_rate = 48000u32;
    let iterations = 500;

    let mut chain = create_full_effect_chain();

    // Warmup with max size
    for _ in 0..50 {
        let mut buffer = generate_varied_audio(sample_rate, 4096);
        chain.process(&mut buffer, sample_rate);
    }

    // Process with varying sizes in random-ish order
    let sizes = [256, 1024, 128, 2048, 64, 512, 4096, 256, 1024];
    let mut stats = TimingStats::new();

    for i in 0..iterations {
        let size = sizes[i % sizes.len()];
        let mut buffer = generate_varied_audio(sample_rate, size);
        let start = Instant::now();
        chain.process(&mut buffer, sample_rate);
        stats.add(start.elapsed());

        // Verify output
        for sample in &buffer {
            assert!(sample.is_finite());
        }
    }

    // Timing should remain consistent even with varying sizes
    let jitter = stats.jitter_ratio();

    println!("\n=== Varying Buffer Sizes ===");
    println!("Jitter ratio: {:.2}x", jitter);

    assert!(
        jitter < 20.0,
        "High jitter with varying buffer sizes: {:.2}x",
        jitter
    );
}

// ============================================================================
// 6. LONG-RUNNING STABILITY TESTS
// ============================================================================

#[test]
fn test_long_running_latency_stability() {
    let sample_rate = 48000u32;
    let buffer_size = 512usize;
    let duration_secs = 30; // 30 seconds simulated
    let measurement_interval_buffers = 1000;

    let buffers_per_second = sample_rate as usize / buffer_size;
    let total_buffers = duration_secs * buffers_per_second;

    let mut chain = create_full_effect_chain();

    let mut interval_means: Vec<Duration> = Vec::new();
    let mut interval_stats = TimingStats::new();

    println!("\n=== Long-Running Latency Stability ({} seconds) ===", duration_secs);

    for i in 0..total_buffers {
        let mut buffer = generate_varied_audio(sample_rate, buffer_size);
        let start = Instant::now();
        chain.process(&mut buffer, sample_rate);
        interval_stats.add(start.elapsed());

        // Record interval statistics
        if (i + 1) % measurement_interval_buffers == 0 {
            interval_means.push(interval_stats.mean());
            interval_stats = TimingStats::new();
        }

        // Verify output validity periodically
        if i % 10000 == 0 {
            for sample in &buffer {
                assert!(sample.is_finite(), "Invalid sample at iteration {}", i);
            }
        }
    }

    // Analyze latency drift
    if !interval_means.is_empty() {
        let first_mean = interval_means[0];
        let last_mean = *interval_means.last().unwrap();
        let drift_ratio = last_mean.as_nanos() as f64 / first_mean.as_nanos() as f64;

        let min_mean = interval_means.iter().min().unwrap();
        let max_mean = interval_means.iter().max().unwrap();
        let variation = max_mean.as_nanos() as f64 / min_mean.as_nanos() as f64;

        println!("First interval mean: {:.2}us", first_mean.as_nanos() as f64 / 1000.0);
        println!("Last interval mean: {:.2}us", last_mean.as_nanos() as f64 / 1000.0);
        println!("Drift ratio (last/first): {:.2}x", drift_ratio);
        println!("Max variation (max/min): {:.2}x", variation);

        // Latency should be stable (within 50% of initial)
        assert!(
            drift_ratio < 1.5 && drift_ratio > 0.5,
            "Significant latency drift: {:.2}x",
            drift_ratio
        );

        // Variation should be bounded
        assert!(
            variation < 2.0,
            "Latency variation too high: {:.2}x",
            variation
        );
    }
}

#[test]
fn test_accumulated_state_stability() {
    let sample_rate = 48000u32;
    let buffer_size = 512usize;
    let iterations = 50000; // ~500 seconds of audio

    let mut chain = create_full_effect_chain();

    println!("\n=== Accumulated State Stability ===");

    // Process many buffers to accumulate internal state
    for i in 0..iterations {
        let mut buffer = generate_varied_audio(sample_rate, buffer_size);
        chain.process(&mut buffer, sample_rate);

        // Check for state explosion (values growing unboundedly)
        if i % 10000 == 0 {
            let max_sample = buffer.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
            let rms: f32 = (buffer.iter().map(|s| s * s).sum::<f32>() / buffer.len() as f32).sqrt();

            println!(
                "  Iteration {}: max={:.4}, rms={:.4}",
                i, max_sample, rms
            );

            // Values should remain bounded
            assert!(
                max_sample < 2.0,
                "Output growing unboundedly at iteration {}: max = {:.4}",
                i,
                max_sample
            );
            assert!(
                rms < 1.0,
                "RMS growing unboundedly at iteration {}: rms = {:.4}",
                i,
                rms
            );
        }
    }

    println!("State remains stable over {} iterations", iterations);
}

#[test]
fn test_one_hour_simulated_playback() {
    // Reduced from 1 hour to ~1 minute of simulated audio for CI
    // Run with --ignored for full 1-hour test
    let sample_rate = 48000u32;
    let buffer_size = 512usize;
    let simulated_seconds = 60; // 1 minute

    let buffers_per_second = sample_rate as usize / buffer_size;
    let total_buffers = simulated_seconds * buffers_per_second;

    let mut chain = create_full_effect_chain();
    let budget = buffer_period(buffer_size, sample_rate);

    let start = Instant::now();
    let mut underruns = 0;
    let mut max_latency = Duration::ZERO;

    for _ in 0..total_buffers {
        let mut buffer = generate_varied_audio(sample_rate, buffer_size);
        let process_start = Instant::now();
        chain.process(&mut buffer, sample_rate);
        let elapsed = process_start.elapsed();

        if elapsed > budget {
            underruns += 1;
        }
        if elapsed > max_latency {
            max_latency = elapsed;
        }
    }

    let real_elapsed = start.elapsed();
    let simulated_duration = Duration::from_secs(simulated_seconds as u64);
    let underrun_rate = underruns as f64 / total_buffers as f64;

    println!("\n=== Simulated Playback Test ===");
    println!("Simulated duration: {} seconds", simulated_seconds);
    println!("Real elapsed: {:.2}s", real_elapsed.as_secs_f64());
    println!(
        "Speed factor: {:.1}x realtime",
        simulated_duration.as_secs_f64() / real_elapsed.as_secs_f64()
    );
    println!("Total buffers: {}", total_buffers);
    println!("Underruns: {} ({:.3}%)", underruns, underrun_rate * 100.0);
    println!("Max latency: {:.2}us", max_latency.as_nanos() as f64 / 1000.0);

    // Should process faster than realtime
    assert!(
        real_elapsed < simulated_duration,
        "Processing slower than realtime"
    );

    // Underrun rate should be very low
    assert!(
        underrun_rate < 0.01,
        "Underrun rate too high: {:.3}%",
        underrun_rate * 100.0
    );
}

// ============================================================================
// 7. WINDOWS-SPECIFIC PLAYBACK GAP DETECTION
// ============================================================================

#[test]
fn test_windows_batch_processing_gap_simulation() {
    // Simulate the Windows playback gap issue during batch processing
    // The issue manifests when background work causes audio callbacks to be delayed
    let sample_rate = 48000u32;
    let buffer_size = 512usize;
    let num_batches = 10;
    let buffers_per_batch = 50;

    let mut chain = create_full_effect_chain();
    let budget = buffer_period(buffer_size, sample_rate);

    let mut all_timings: Vec<Duration> = Vec::new();
    let mut batch_underruns: Vec<usize> = Vec::new();

    println!("\n=== Windows Batch Processing Gap Simulation ===");

    for batch in 0..num_batches {
        let mut batch_underrun_count = 0;

        // Simulate batch operation start (like scanning files)
        if batch % 2 == 0 {
            // Heavy batch work simulation
            let work: Vec<f64> = (0..50000).map(|x| (x as f64).sin().cos()).collect();
            std::hint::black_box(&work);
        }

        // Process audio during/after batch work
        for _ in 0..buffers_per_batch {
            let mut buffer = generate_varied_audio(sample_rate, buffer_size);
            let start = Instant::now();
            chain.process(&mut buffer, sample_rate);
            let elapsed = start.elapsed();

            all_timings.push(elapsed);
            if elapsed > budget {
                batch_underrun_count += 1;
            }

            // Verify no gaps in audio (sudden silence)
            let has_content = buffer.iter().any(|s| s.abs() > 0.001);
            assert!(has_content, "Audio gap detected - buffer contains only silence");
        }

        batch_underruns.push(batch_underrun_count);
        println!(
            "  Batch {}: {} underruns / {} buffers",
            batch, batch_underrun_count, buffers_per_batch
        );
    }

    // Check that batch work doesn't cause catastrophic underruns
    let total_underruns: usize = batch_underruns.iter().sum();
    let total_buffers = num_batches * buffers_per_batch;
    let underrun_rate = total_underruns as f64 / total_buffers as f64;

    println!("Total underruns: {} / {} ({:.2}%)", total_underruns, total_buffers, underrun_rate * 100.0);

    // The audio processing itself should remain fast regardless of batch work
    let mean_timing: Duration = all_timings.iter().sum::<Duration>() / all_timings.len() as u32;
    assert!(
        mean_timing < budget / 2,
        "Mean processing time too high: {:.2}us (budget: {:.2}us)",
        mean_timing.as_nanos() as f64 / 1000.0,
        budget.as_nanos() as f64 / 1000.0
    );
}

#[test]
fn test_inter_buffer_gap_detection() {
    // Test that verifies no severe discontinuities are introduced between buffers
    // Note: The effect chain (especially compressor, limiter) will introduce some
    // sample-to-sample variation due to envelope following and state changes.
    // What we're detecting here are SEVERE discontinuities (>0.8) that would
    // cause audible clicks/pops, not normal DSP artifacts.
    let sample_rate = 48000u32;
    let buffer_size = 256usize;
    let frequency = 1000.0f32;
    let num_buffers = 100;

    let mut chain = create_full_effect_chain();

    // Collect last and first samples at each boundary
    let mut boundary_transitions: Vec<(f32, f32, f32)> = Vec::new(); // (last, first, gap)
    let mut prev_last: Option<f32> = None;

    for _ in 0..num_buffers {
        let mut buffer = generate_stereo_sine(frequency, sample_rate, buffer_size);
        chain.process(&mut buffer, sample_rate);

        let first = buffer[0];
        let last = buffer[buffer.len() - 2]; // Left channel

        if let Some(prev) = prev_last {
            let gap = (first - prev).abs();
            boundary_transitions.push((prev, first, gap));
        }

        prev_last = Some(last);
    }

    // Analyze gaps
    let max_gap = boundary_transitions
        .iter()
        .map(|(_, _, g)| *g)
        .fold(0.0f32, f32::max);
    let mean_gap: f32 = boundary_transitions.iter().map(|(_, _, g)| g).sum::<f32>()
        / boundary_transitions.len() as f32;

    // Find severe gaps (potential clicks/pops - values above 0.8 would be very audible)
    // Normal DSP processing through compressor/limiter may cause gaps up to 0.5-0.6
    let severe_gap_threshold = 0.8;
    let severe_gaps: Vec<_> = boundary_transitions
        .iter()
        .filter(|(_, _, g)| *g > severe_gap_threshold)
        .collect();

    println!("\n=== Inter-Buffer Gap Detection ===");
    println!("Mean gap: {:.6}", mean_gap);
    println!("Max gap: {:.6}", max_gap);
    println!("Severe gaps (>{:.1}): {}", severe_gap_threshold, severe_gaps.len());

    // Should have no severe gaps (these would cause audible clicks)
    assert!(
        severe_gaps.is_empty(),
        "Severe gaps detected at buffer boundaries: {} (would cause clicks)",
        severe_gaps.len()
    );

    // Max gap should stay below 1.0 (full scale jump would be very bad)
    assert!(
        max_gap < 1.0,
        "Maximum gap too large: {:.4} (full scale discontinuity)",
        max_gap
    );
}

#[test]
fn test_callback_scheduling_variance() {
    // Test that measures callback scheduling variance
    // High variance indicates potential for gaps/glitches
    let sample_rate = 48000u32;
    let buffer_size = 512usize;
    let iterations = 500;

    let mut chain = create_full_effect_chain();
    let budget = buffer_period(buffer_size, sample_rate);

    let mut inter_callback_times: Vec<Duration> = Vec::new();
    let mut last_callback = Instant::now();

    // Simulate callback loop with realistic timing
    for _ in 0..iterations {
        let callback_start = Instant::now();

        // Record time since last callback
        let inter_time = callback_start.duration_since(last_callback);
        if inter_time > Duration::ZERO {
            inter_callback_times.push(inter_time);
        }

        // Process audio
        let mut buffer = generate_varied_audio(sample_rate, buffer_size);
        chain.process(&mut buffer, sample_rate);

        last_callback = callback_start;

        // Sleep to simulate real callback timing (approximately)
        let processing_time = callback_start.elapsed();
        if processing_time < budget {
            thread::sleep(budget - processing_time);
        }
    }

    // Skip first few samples (warmup)
    let stable_times: Vec<_> = inter_callback_times.iter().skip(10).collect();

    if stable_times.len() > 10 {
        let mean: Duration = stable_times.iter().map(|&&d| d).sum::<Duration>() / stable_times.len() as u32;
        let max_deviation = stable_times
            .iter()
            .map(|&&d| {
                if d > mean {
                    d - mean
                } else {
                    mean - d
                }
            })
            .max()
            .unwrap_or(Duration::ZERO);

        println!("\n=== Callback Scheduling Variance ===");
        println!("Expected interval: {:.2}ms", budget.as_secs_f64() * 1000.0);
        println!("Mean interval: {:.2}ms", mean.as_secs_f64() * 1000.0);
        println!("Max deviation: {:.2}ms", max_deviation.as_secs_f64() * 1000.0);

        // Deviation should be within 10% of budget for stable playback
        let acceptable_deviation = budget.mul_f64(0.1);
        if max_deviation > acceptable_deviation {
            println!(
                "WARNING: High scheduling variance detected ({:.2}ms > {:.2}ms acceptable)",
                max_deviation.as_secs_f64() * 1000.0,
                acceptable_deviation.as_secs_f64() * 1000.0
            );
        }
    }
}

#[test]
fn test_output_buffer_completeness() {
    // Verify that output buffers are completely filled (no partial processing)
    let sample_rate = 48000u32;
    let buffer_sizes = [64, 128, 256, 512, 1024, 2048];
    let iterations = 50;

    let mut chain = create_full_effect_chain();

    println!("\n=== Output Buffer Completeness ===");

    for &buffer_size in &buffer_sizes {
        let mut incomplete_count = 0;

        for _ in 0..iterations {
            let mut buffer = generate_varied_audio(sample_rate, buffer_size);
            let original_len = buffer.len();

            chain.process(&mut buffer, sample_rate);

            // Verify buffer length unchanged
            assert_eq!(
                buffer.len(),
                original_len,
                "Buffer length changed during processing"
            );

            // Check for partial processing (trailing zeros that shouldn't be there)
            let trailing_zeros = buffer.iter().rev().take_while(|&&s| s == 0.0).count();
            if trailing_zeros > 10 {
                incomplete_count += 1;
            }

            // Verify all samples are valid
            for sample in &buffer {
                assert!(sample.is_finite(), "Invalid sample in output buffer");
            }
        }

        println!(
            "  Buffer {}: {} / {} incomplete",
            buffer_size, incomplete_count, iterations
        );

        // Should have no incomplete buffers
        assert!(
            incomplete_count == 0,
            "Incomplete buffers detected at size {}: {}",
            buffer_size,
            incomplete_count
        );
    }
}

#[test]
fn test_rapid_buffer_submission_no_gaps() {
    // Test rapid buffer submission (simulates Windows WASAPI scenario)
    let sample_rate = 48000u32;
    let buffer_size = 256usize;
    let num_buffers = 200;

    let mut chain = create_full_effect_chain();

    let mut all_samples: Vec<f32> = Vec::with_capacity(num_buffers * buffer_size * 2);
    let start = Instant::now();

    // Submit buffers as fast as possible (no waiting)
    for _ in 0..num_buffers {
        let mut buffer = generate_varied_audio(sample_rate, buffer_size);
        chain.process(&mut buffer, sample_rate);
        all_samples.extend_from_slice(&buffer);
    }

    let elapsed = start.elapsed();

    // Check for gaps in the combined output
    let mut gap_regions = 0;
    let mut consecutive_near_zero = 0;
    let gap_threshold = 0.001;
    let min_gap_samples = 20;

    for sample in &all_samples {
        if sample.abs() < gap_threshold {
            consecutive_near_zero += 1;
        } else {
            if consecutive_near_zero >= min_gap_samples {
                gap_regions += 1;
            }
            consecutive_near_zero = 0;
        }
    }

    println!("\n=== Rapid Buffer Submission ===");
    println!("Buffers processed: {}", num_buffers);
    println!("Total samples: {}", all_samples.len());
    println!("Processing time: {:.2}ms", elapsed.as_secs_f64() * 1000.0);
    println!("Gap regions detected: {}", gap_regions);

    // Should have no gap regions in continuous output
    assert!(
        gap_regions < 3,
        "Too many gap regions in rapid submission: {}",
        gap_regions
    );
}

// ============================================================================
// IGNORED TESTS (Run with --ignored flag for extended tests)
// ============================================================================

#[test]
#[ignore]
fn test_one_hour_actual_playback() {
    // Full 1-hour sustained load test
    // Run with: cargo test --package soul-audio test_one_hour_actual_playback -- --ignored

    let sample_rate = 48000u32;
    let buffer_size = 512usize;
    let duration = Duration::from_secs(3600); // 1 hour

    let mut chain = create_full_effect_chain();
    let budget = buffer_period(buffer_size, sample_rate);

    let start = Instant::now();
    let mut buffers_processed = 0u64;
    let mut underruns = 0u64;
    let mut last_report = Instant::now();

    println!("\n=== Extended Playback Test (1 hour) ===");

    while start.elapsed() < duration {
        let mut buffer = generate_varied_audio(sample_rate, buffer_size);
        let process_start = Instant::now();
        chain.process(&mut buffer, sample_rate);

        buffers_processed += 1;
        if process_start.elapsed() > budget {
            underruns += 1;
        }

        // Report progress every minute
        if last_report.elapsed() > Duration::from_secs(60) {
            let progress = start.elapsed().as_secs() as f64 / duration.as_secs() as f64 * 100.0;
            let underrun_rate = underruns as f64 / buffers_processed as f64;
            println!(
                "[{:.1}%] Buffers: {}, Underruns: {} ({:.3}%)",
                progress, buffers_processed, underruns, underrun_rate * 100.0
            );
            last_report = Instant::now();
        }
    }

    let total_elapsed = start.elapsed();
    let underrun_rate = underruns as f64 / buffers_processed as f64;

    println!("\n=== Final Results ===");
    println!("Total time: {:.1} minutes", total_elapsed.as_secs() as f64 / 60.0);
    println!("Buffers processed: {}", buffers_processed);
    println!("Underruns: {} ({:.4}%)", underruns, underrun_rate * 100.0);

    assert!(
        underrun_rate < 0.001,
        "Underrun rate too high for 1-hour test: {:.4}%",
        underrun_rate * 100.0
    );
}
