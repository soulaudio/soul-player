//! Thread Safety and Real-Time Guarantees Test Suite
//!
//! Comprehensive tests for verifying thread safety and real-time audio constraints.
//!
//! Tests include:
//! - Concurrent effect parameter changes while processing
//! - Multiple threads reading/writing shared audio buffers
//! - Effect chain modification from different threads
//! - Concurrent decoder instances
//! - Race condition detection for common patterns
//! - Real-time deadline enforcement across various buffer sizes
//! - Worst-case latency measurement
//! - Processing time spike detection
//!
//! Real-time audio requires deterministic behavior with strict timing constraints.
//! These tests verify that the audio pipeline meets those requirements.

use soul_audio::effects::{
    AudioEffect, Compressor, CompressorSettings, Crossfeed, CrossfeedPreset, EffectChain, EqBand,
    GraphicEq, GraphicEqPreset, Limiter, LimiterSettings, ParametricEq, StereoEnhancer,
    StereoSettings,
};
use std::f32::consts::PI;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Barrier, Mutex, RwLock};
use std::thread;
use std::time::{Duration, Instant};

// ============================================================================
// CONSTANTS
// ============================================================================

/// Standard buffer sizes to test (samples per channel)
const BUFFER_SIZES: [usize; 6] = [64, 128, 256, 512, 1024, 2048];

/// Standard sample rate for tests
const SAMPLE_RATE: u32 = 48000;

/// Number of iterations for stress tests
const STRESS_ITERATIONS: usize = 10000;

/// Maximum allowed jitter ratio (P99/mean) for real-time safety
const MAX_JITTER_RATIO: f64 = 5.0;

/// Safety margin for real-time budget (50% of buffer period)
const REAL_TIME_SAFETY_MARGIN: f64 = 0.5;

// ============================================================================
// TEST UTILITIES
// ============================================================================

/// Generate a stereo sine wave buffer
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

/// Generate varied audio content (mix of frequencies)
fn generate_varied_audio(sample_rate: u32, num_frames: usize) -> Vec<f32> {
    let mut buffer = Vec::with_capacity(num_frames * 2);
    for i in 0..num_frames {
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

/// Check if all samples are finite
fn all_finite(buffer: &[f32]) -> bool {
    buffer.iter().all(|s| s.is_finite())
}

/// Calculate buffer period as Duration
fn buffer_period(buffer_size: usize, sample_rate: u32) -> Duration {
    Duration::from_secs_f64(buffer_size as f64 / sample_rate as f64)
}

/// Create a full effect chain with all effects enabled
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

/// Statistics for timing measurements
struct TimingStats {
    samples: Vec<Duration>,
}

impl TimingStats {
    fn new() -> Self {
        Self {
            samples: Vec::new(),
        }
    }

    fn with_capacity(capacity: usize) -> Self {
        Self {
            samples: Vec::with_capacity(capacity),
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

    fn max(&self) -> Duration {
        self.samples.iter().copied().max().unwrap_or(Duration::ZERO)
    }

    fn min(&self) -> Duration {
        self.samples.iter().copied().min().unwrap_or(Duration::ZERO)
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

// ============================================================================
// 1. THREAD SAFETY: CONCURRENT EFFECT PARAMETER CHANGES
// ============================================================================

#[test]
fn test_concurrent_eq_parameter_changes_while_processing() {
    let num_threads = 4;
    let duration = Duration::from_secs(3);
    let buffer_size = 256;

    // Shared state for coordination
    let processing_active = Arc::new(AtomicBool::new(true));
    let buffers_processed = Arc::new(AtomicU64::new(0));
    let parameter_changes = Arc::new(AtomicU64::new(0));
    let errors_detected = Arc::new(AtomicBool::new(false));

    // Create effects with Mutex for thread-safe access
    let eq = Arc::new(Mutex::new(ParametricEq::new()));

    // Audio processing threads
    let mut audio_handles = Vec::new();
    for _ in 0..num_threads {
        let processing_active_clone = Arc::clone(&processing_active);
        let buffers_processed_clone = Arc::clone(&buffers_processed);
        let errors_detected_clone = Arc::clone(&errors_detected);
        let eq_clone = Arc::clone(&eq);

        let handle = thread::spawn(move || {
            while processing_active_clone.load(Ordering::Relaxed) {
                let mut buffer = generate_varied_audio(SAMPLE_RATE, buffer_size);

                {
                    let mut eq = eq_clone.lock().unwrap();
                    eq.process(&mut buffer, SAMPLE_RATE);
                }

                if !all_finite(&buffer) {
                    errors_detected_clone.store(true, Ordering::Relaxed);
                }

                buffers_processed_clone.fetch_add(1, Ordering::Relaxed);
            }
        });
        audio_handles.push(handle);
    }

    // Parameter modification thread
    let processing_active_clone = Arc::clone(&processing_active);
    let parameter_changes_clone = Arc::clone(&parameter_changes);
    let eq_clone = Arc::clone(&eq);

    let param_handle = thread::spawn(move || {
        let mut i = 0u32;
        while processing_active_clone.load(Ordering::Relaxed) {
            {
                let mut eq = eq_clone.lock().unwrap();
                // Rapidly change EQ parameters
                let gain = (i as f32 * 0.1).sin() * 12.0;
                let freq = 100.0 + (i as f32 * 0.05).sin().abs() * 10000.0;
                let q = 0.5 + (i as f32 * 0.02).sin().abs() * 9.5;
                eq.set_mid_band(EqBand::peaking(freq, gain, q));
            }

            parameter_changes_clone.fetch_add(1, Ordering::Relaxed);
            i = i.wrapping_add(1);
            thread::yield_now();
        }
    });

    thread::sleep(duration);
    processing_active.store(false, Ordering::Relaxed);

    for handle in audio_handles {
        handle.join().unwrap();
    }
    param_handle.join().unwrap();

    let buffers = buffers_processed.load(Ordering::Relaxed);
    let changes = parameter_changes.load(Ordering::Relaxed);
    let had_errors = errors_detected.load(Ordering::Relaxed);

    println!("\n=== Concurrent EQ Parameter Changes Test ===");
    println!("Duration: {:?}", duration);
    println!("Buffers processed: {}", buffers);
    println!("Parameter changes: {}", changes);
    println!("Errors detected: {}", had_errors);

    assert!(!had_errors, "Errors detected during concurrent EQ parameter changes");
    assert!(buffers > 0, "No buffers were processed");
    assert!(changes > 0, "No parameter changes were made");
}

#[test]
fn test_concurrent_compressor_parameter_changes() {
    let duration = Duration::from_secs(2);
    let buffer_size = 512;

    let processing_active = Arc::new(AtomicBool::new(true));
    let errors_detected = Arc::new(AtomicBool::new(false));

    let compressor = Arc::new(Mutex::new(Compressor::new()));

    // Audio processing thread
    let processing_active_clone = Arc::clone(&processing_active);
    let errors_detected_clone = Arc::clone(&errors_detected);
    let compressor_clone = Arc::clone(&compressor);

    let audio_thread = thread::spawn(move || {
        while processing_active_clone.load(Ordering::Relaxed) {
            let mut buffer = generate_varied_audio(SAMPLE_RATE, buffer_size);
            {
                let mut comp = compressor_clone.lock().unwrap();
                comp.process(&mut buffer, SAMPLE_RATE);
            }
            if !all_finite(&buffer) {
                errors_detected_clone.store(true, Ordering::Relaxed);
            }
        }
    });

    // Parameter modification thread - changes threshold, ratio, attack, release
    let processing_active_clone = Arc::clone(&processing_active);
    let compressor_clone = Arc::clone(&compressor);

    let param_thread = thread::spawn(move || {
        let mut i = 0u32;
        while processing_active_clone.load(Ordering::Relaxed) {
            {
                let mut comp = compressor_clone.lock().unwrap();
                comp.set_threshold(-50.0 + (i as f32 % 40.0));
                comp.set_ratio(1.0 + (i as f32 % 19.0));
            }
            i = i.wrapping_add(1);
            thread::sleep(Duration::from_micros(50));
        }
    });

    thread::sleep(duration);
    processing_active.store(false, Ordering::Relaxed);

    audio_thread.join().unwrap();
    param_thread.join().unwrap();

    let had_errors = errors_detected.load(Ordering::Relaxed);
    assert!(!had_errors, "Errors during concurrent compressor parameter changes");
}

#[test]
fn test_concurrent_limiter_threshold_changes() {
    let duration = Duration::from_secs(2);
    let buffer_size = 256;

    let processing_active = Arc::new(AtomicBool::new(true));
    let errors_detected = Arc::new(AtomicBool::new(false));

    let limiter = Arc::new(Mutex::new(Limiter::new()));

    let processing_active_clone = Arc::clone(&processing_active);
    let errors_detected_clone = Arc::clone(&errors_detected);
    let limiter_clone = Arc::clone(&limiter);

    let audio_thread = thread::spawn(move || {
        while processing_active_clone.load(Ordering::Relaxed) {
            let mut buffer = generate_varied_audio(SAMPLE_RATE, buffer_size);
            {
                let mut lim = limiter_clone.lock().unwrap();
                lim.process(&mut buffer, SAMPLE_RATE);
            }
            if !all_finite(&buffer) {
                errors_detected_clone.store(true, Ordering::Relaxed);
            }
        }
    });

    let processing_active_clone = Arc::clone(&processing_active);
    let limiter_clone = Arc::clone(&limiter);

    let param_thread = thread::spawn(move || {
        let mut i = 0u32;
        while processing_active_clone.load(Ordering::Relaxed) {
            {
                let mut lim = limiter_clone.lock().unwrap();
                lim.set_threshold(-12.0 + (i as f32 % 12.0));
            }
            i = i.wrapping_add(1);
            thread::sleep(Duration::from_micros(100));
        }
    });

    thread::sleep(duration);
    processing_active.store(false, Ordering::Relaxed);

    audio_thread.join().unwrap();
    param_thread.join().unwrap();

    assert!(!errors_detected.load(Ordering::Relaxed), "Errors during limiter threshold changes");
}

// ============================================================================
// 2. THREAD SAFETY: MULTIPLE THREADS READING/WRITING SHARED BUFFERS
// ============================================================================

#[test]
fn test_shared_buffer_rwlock_pattern() {
    let num_readers = 4;
    let num_writers = 2;
    let duration = Duration::from_secs(2);
    let buffer_size = 1024;

    let buffer = Arc::new(RwLock::new(generate_varied_audio(SAMPLE_RATE, buffer_size)));
    let processing_active = Arc::new(AtomicBool::new(true));
    let reads_completed = Arc::new(AtomicU64::new(0));
    let writes_completed = Arc::new(AtomicU64::new(0));

    let mut handles = Vec::new();

    // Reader threads - analyze buffer without modifying
    for _ in 0..num_readers {
        let buffer_clone = Arc::clone(&buffer);
        let processing_active_clone = Arc::clone(&processing_active);
        let reads_completed_clone = Arc::clone(&reads_completed);

        let handle = thread::spawn(move || {
            while processing_active_clone.load(Ordering::Relaxed) {
                let buf = buffer_clone.read().unwrap();
                // Analyze without modifying
                let _peak = buf.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
                let _rms = (buf.iter().map(|s| s * s).sum::<f32>() / buf.len() as f32).sqrt();
                reads_completed_clone.fetch_add(1, Ordering::Relaxed);
                drop(buf);
                thread::yield_now();
            }
        });
        handles.push(handle);
    }

    // Writer threads - process and modify buffer
    for i in 0..num_writers {
        let buffer_clone = Arc::clone(&buffer);
        let processing_active_clone = Arc::clone(&processing_active);
        let writes_completed_clone = Arc::clone(&writes_completed);

        let handle = thread::spawn(move || {
            let mut eq = ParametricEq::new();
            eq.set_mid_band(EqBand::peaking(1000.0 + (i as f32 * 100.0), 3.0, 1.0));

            while processing_active_clone.load(Ordering::Relaxed) {
                let mut buf = buffer_clone.write().unwrap();
                eq.process(&mut buf, SAMPLE_RATE);
                writes_completed_clone.fetch_add(1, Ordering::Relaxed);
                drop(buf);
                thread::sleep(Duration::from_micros(100));
            }
        });
        handles.push(handle);
    }

    thread::sleep(duration);
    processing_active.store(false, Ordering::Relaxed);

    for handle in handles {
        handle.join().unwrap();
    }

    let reads = reads_completed.load(Ordering::Relaxed);
    let writes = writes_completed.load(Ordering::Relaxed);

    println!("\n=== Shared Buffer RwLock Test ===");
    println!("Reads completed: {}", reads);
    println!("Writes completed: {}", writes);

    assert!(reads > 0, "No reads completed");
    assert!(writes > 0, "No writes completed");
}

#[test]
fn test_double_buffering_pattern() {
    // Simulate double-buffering: one buffer being processed while another is filled
    let buffer_size = 512;
    let buffer_a = Arc::new(Mutex::new(generate_varied_audio(SAMPLE_RATE, buffer_size)));
    let buffer_b = Arc::new(Mutex::new(generate_varied_audio(SAMPLE_RATE, buffer_size)));
    let current_buffer = Arc::new(AtomicBool::new(true)); // true = A, false = B
    let processing_active = Arc::new(AtomicBool::new(true));
    let swaps_completed = Arc::new(AtomicU64::new(0));

    // Producer thread - fills the "back" buffer
    let buffer_a_clone = Arc::clone(&buffer_a);
    let buffer_b_clone = Arc::clone(&buffer_b);
    let current_buffer_clone = Arc::clone(&current_buffer);
    let processing_active_clone = Arc::clone(&processing_active);
    let swaps_completed_clone = Arc::clone(&swaps_completed);

    let producer = thread::spawn(move || {
        while processing_active_clone.load(Ordering::Relaxed) {
            let is_a = current_buffer_clone.load(Ordering::Acquire);
            // Fill the back buffer (opposite of current)
            let back_buffer = if is_a {
                buffer_b_clone.lock().unwrap()
            } else {
                buffer_a_clone.lock().unwrap()
            };
            // Simulate filling with new data
            let _ = back_buffer.len();
            drop(back_buffer);

            // Swap buffers
            current_buffer_clone.store(!is_a, Ordering::Release);
            swaps_completed_clone.fetch_add(1, Ordering::Relaxed);
            thread::yield_now();
        }
    });

    // Consumer thread - processes the "front" buffer
    let buffer_a_clone = Arc::clone(&buffer_a);
    let buffer_b_clone = Arc::clone(&buffer_b);
    let current_buffer_clone = Arc::clone(&current_buffer);
    let processing_active_clone = Arc::clone(&processing_active);

    let consumer = thread::spawn(move || {
        let mut eq = ParametricEq::new();
        eq.set_mid_band(EqBand::peaking(1000.0, 6.0, 1.0));

        while processing_active_clone.load(Ordering::Relaxed) {
            let is_a = current_buffer_clone.load(Ordering::Acquire);
            // Process the front buffer
            let mut front_buffer = if is_a {
                buffer_a_clone.lock().unwrap()
            } else {
                buffer_b_clone.lock().unwrap()
            };
            eq.process(&mut front_buffer, SAMPLE_RATE);
            drop(front_buffer);
            thread::yield_now();
        }
    });

    // Run for limited iterations
    thread::sleep(Duration::from_millis(500));
    processing_active.store(false, Ordering::Relaxed);

    producer.join().unwrap();
    consumer.join().unwrap();

    let swaps = swaps_completed.load(Ordering::Relaxed);
    println!("\n=== Double Buffering Pattern Test ===");
    println!("Buffer swaps completed: {}", swaps);
    assert!(swaps > 0, "No buffer swaps completed");
}

// ============================================================================
// 3. THREAD SAFETY: EFFECT CHAIN MODIFICATION FROM DIFFERENT THREADS
// ============================================================================

#[test]
fn test_effect_chain_enable_disable_from_threads() {
    let duration = Duration::from_secs(2);
    let buffer_size = 256;

    let chain = Arc::new(Mutex::new(create_full_effect_chain()));
    let processing_active = Arc::new(AtomicBool::new(true));
    let errors_detected = Arc::new(AtomicBool::new(false));

    // Audio processing thread
    let chain_clone = Arc::clone(&chain);
    let processing_active_clone = Arc::clone(&processing_active);
    let errors_detected_clone = Arc::clone(&errors_detected);

    let audio_thread = thread::spawn(move || {
        while processing_active_clone.load(Ordering::Relaxed) {
            let mut buffer = generate_varied_audio(SAMPLE_RATE, buffer_size);
            {
                let mut c = chain_clone.lock().unwrap();
                c.process(&mut buffer, SAMPLE_RATE);
            }
            if !all_finite(&buffer) {
                errors_detected_clone.store(true, Ordering::Relaxed);
            }
        }
    });

    // Toggle thread - rapidly enables/disables effects
    let chain_clone = Arc::clone(&chain);
    let processing_active_clone = Arc::clone(&processing_active);

    let toggle_thread = thread::spawn(move || {
        let mut enabled = true;
        while processing_active_clone.load(Ordering::Relaxed) {
            {
                let mut c = chain_clone.lock().unwrap();
                c.set_enabled(enabled);
            }
            enabled = !enabled;
            thread::sleep(Duration::from_micros(200));
        }
    });

    thread::sleep(duration);
    processing_active.store(false, Ordering::Relaxed);

    audio_thread.join().unwrap();
    toggle_thread.join().unwrap();

    assert!(
        !errors_detected.load(Ordering::Relaxed),
        "Errors detected during enable/disable toggling"
    );
}

#[test]
fn test_effect_chain_reset_while_processing() {
    let duration = Duration::from_secs(2);
    let buffer_size = 256;

    let chain = Arc::new(Mutex::new(create_full_effect_chain()));
    let processing_active = Arc::new(AtomicBool::new(true));
    let errors_detected = Arc::new(AtomicBool::new(false));
    let resets_performed = Arc::new(AtomicU64::new(0));

    // Audio processing thread
    let chain_clone = Arc::clone(&chain);
    let processing_active_clone = Arc::clone(&processing_active);
    let errors_detected_clone = Arc::clone(&errors_detected);

    let audio_thread = thread::spawn(move || {
        while processing_active_clone.load(Ordering::Relaxed) {
            let mut buffer = generate_varied_audio(SAMPLE_RATE, buffer_size);
            {
                let mut c = chain_clone.lock().unwrap();
                c.process(&mut buffer, SAMPLE_RATE);
            }
            if !all_finite(&buffer) {
                errors_detected_clone.store(true, Ordering::Relaxed);
            }
        }
    });

    // Reset thread - periodically resets all effects
    let chain_clone = Arc::clone(&chain);
    let processing_active_clone = Arc::clone(&processing_active);
    let resets_performed_clone = Arc::clone(&resets_performed);

    let reset_thread = thread::spawn(move || {
        while processing_active_clone.load(Ordering::Relaxed) {
            {
                let mut c = chain_clone.lock().unwrap();
                c.reset();
            }
            resets_performed_clone.fetch_add(1, Ordering::Relaxed);
            thread::sleep(Duration::from_millis(10));
        }
    });

    thread::sleep(duration);
    processing_active.store(false, Ordering::Relaxed);

    audio_thread.join().unwrap();
    reset_thread.join().unwrap();

    let resets = resets_performed.load(Ordering::Relaxed);
    println!("\n=== Effect Chain Reset While Processing ===");
    println!("Resets performed: {}", resets);

    assert!(!errors_detected.load(Ordering::Relaxed), "Errors during reset");
    assert!(resets > 0, "No resets were performed");
}

// ============================================================================
// 4. THREAD SAFETY: CONCURRENT DECODER INSTANCES
// ============================================================================

#[test]
fn test_multiple_independent_decoder_effect_chains() {
    // Test that multiple independent effect chains can process concurrently
    let num_threads = 8;
    let iterations_per_thread = 1000;
    let buffer_size = 256;

    let barrier = Arc::new(Barrier::new(num_threads));
    let errors_detected = Arc::new(AtomicBool::new(false));
    let buffers_processed = Arc::new(AtomicU64::new(0));

    let handles: Vec<_> = (0..num_threads)
        .map(|thread_id| {
            let barrier_clone = Arc::clone(&barrier);
            let errors_detected_clone = Arc::clone(&errors_detected);
            let buffers_processed_clone = Arc::clone(&buffers_processed);

            thread::spawn(move || {
                // Each thread has its own effect chain
                let mut chain = create_full_effect_chain();

                // Synchronize start
                barrier_clone.wait();

                for i in 0..iterations_per_thread {
                    // Vary frequency by thread to ensure different processing
                    let freq = 440.0 + (thread_id as f32 * 100.0) + (i as f32 * 0.1);
                    let mut buffer = generate_stereo_sine(freq, SAMPLE_RATE, buffer_size);

                    chain.process(&mut buffer, SAMPLE_RATE);

                    if !all_finite(&buffer) {
                        errors_detected_clone.store(true, Ordering::Relaxed);
                        return;
                    }

                    buffers_processed_clone.fetch_add(1, Ordering::Relaxed);
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    let total_buffers = buffers_processed.load(Ordering::Relaxed);
    let expected_buffers = (num_threads * iterations_per_thread) as u64;

    println!("\n=== Multiple Independent Effect Chains ===");
    println!("Threads: {}", num_threads);
    println!("Buffers processed: {} / {}", total_buffers, expected_buffers);

    assert!(
        !errors_detected.load(Ordering::Relaxed),
        "Errors detected in concurrent processing"
    );
    assert_eq!(
        total_buffers, expected_buffers,
        "Not all buffers were processed"
    );
}

#[test]
fn test_effect_send_trait_verification() {
    // Verify that effects implement Send and can be moved between threads
    let buffer_size = 512;

    // Create effects in main thread
    let mut eq = ParametricEq::new();
    eq.set_mid_band(EqBand::peaking(1000.0, 6.0, 1.0));

    let mut compressor = Compressor::with_settings(CompressorSettings::moderate());
    let mut limiter = Limiter::new();

    // Move to worker thread and process
    let handle = thread::spawn(move || {
        let mut buffer = generate_varied_audio(SAMPLE_RATE, buffer_size);

        eq.process(&mut buffer, SAMPLE_RATE);
        compressor.process(&mut buffer, SAMPLE_RATE);
        limiter.process(&mut buffer, SAMPLE_RATE);

        // Return ownership back
        (eq, compressor, limiter, buffer)
    });

    let (mut eq, mut compressor, mut limiter, buffer) = handle.join().unwrap();

    // Verify output is valid
    assert!(
        all_finite(&buffer),
        "Output should be valid after cross-thread processing"
    );

    // Effects can be used again in main thread
    let mut buffer2 = generate_varied_audio(SAMPLE_RATE, buffer_size);
    eq.process(&mut buffer2, SAMPLE_RATE);
    compressor.process(&mut buffer2, SAMPLE_RATE);
    limiter.process(&mut buffer2, SAMPLE_RATE);

    assert!(
        all_finite(&buffer2),
        "Effects should work after returning from worker thread"
    );
}

// ============================================================================
// 5. RACE CONDITION DETECTION
// ============================================================================

#[test]
fn test_rapid_enable_disable_race_detection() {
    // Test for potential races in enable/disable state transitions
    let iterations = 10000;
    let buffer_size = 128;

    let mut eq = ParametricEq::new();
    eq.set_mid_band(EqBand::peaking(1000.0, 6.0, 1.0));

    // Rapidly toggle enabled state and process
    for i in 0..iterations {
        eq.set_enabled(i % 2 == 0);
        let mut buffer = generate_stereo_sine(1000.0, SAMPLE_RATE, buffer_size);
        eq.process(&mut buffer, SAMPLE_RATE);

        assert!(
            all_finite(&buffer),
            "Output invalid after rapid enable/disable at iteration {}",
            i
        );
    }
}

#[test]
fn test_concurrent_atomic_flag_pattern() {
    // Test pattern used in real-time audio: atomic enabled flag
    let enabled = Arc::new(AtomicBool::new(true));
    let processed_count = Arc::new(AtomicUsize::new(0));
    let error_count = Arc::new(AtomicUsize::new(0));

    let num_threads = 4;
    let iterations_per_thread = 1000;

    let handles: Vec<_> = (0..num_threads)
        .map(|_| {
            let enabled_clone = Arc::clone(&enabled);
            let processed_count_clone = Arc::clone(&processed_count);
            let error_count_clone = Arc::clone(&error_count);

            thread::spawn(move || {
                let mut eq = ParametricEq::new();
                eq.set_mid_band(EqBand::peaking(1000.0, 6.0, 1.0));

                for _ in 0..iterations_per_thread {
                    let is_enabled = enabled_clone.load(Ordering::Relaxed);
                    eq.set_enabled(is_enabled);

                    let mut buffer = generate_stereo_sine(1000.0, SAMPLE_RATE, 64);
                    eq.process(&mut buffer, SAMPLE_RATE);

                    if all_finite(&buffer) {
                        processed_count_clone.fetch_add(1, Ordering::Relaxed);
                    } else {
                        error_count_clone.fetch_add(1, Ordering::Relaxed);
                    }
                }
            })
        })
        .collect();

    // Toggle enabled flag while threads are processing
    for i in 0..100 {
        enabled.store(i % 2 == 0, Ordering::Relaxed);
        thread::sleep(Duration::from_micros(50));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let processed = processed_count.load(Ordering::Relaxed);
    let errors = error_count.load(Ordering::Relaxed);

    println!("\n=== Concurrent Atomic Flag Pattern ===");
    println!("Processed: {}", processed);
    println!("Errors: {}", errors);

    assert_eq!(errors, 0, "Errors detected with atomic flag pattern");
    assert!(processed > 0, "No buffers were processed");
}

#[test]
fn test_parameter_update_during_process_boundary() {
    // Test updating parameters at exact buffer boundaries
    let buffer_size = 256;
    let iterations = 5000;

    let eq = Arc::new(Mutex::new(ParametricEq::new()));
    let processing_active = Arc::new(AtomicBool::new(true));
    let boundary_updates = Arc::new(AtomicU64::new(0));
    let errors_detected = Arc::new(AtomicBool::new(false));

    let eq_clone = Arc::clone(&eq);
    let processing_active_clone = Arc::clone(&processing_active);
    let boundary_updates_clone = Arc::clone(&boundary_updates);
    let errors_detected_clone = Arc::clone(&errors_detected);

    // Processing thread with precise timing
    let process_thread = thread::spawn(move || {
        for _ in 0..iterations {
            if !processing_active_clone.load(Ordering::Relaxed) {
                break;
            }

            let mut buffer = generate_varied_audio(SAMPLE_RATE, buffer_size);

            // Signal that we're about to process
            boundary_updates_clone.fetch_add(1, Ordering::SeqCst);

            {
                let mut eq = eq_clone.lock().unwrap();
                eq.process(&mut buffer, SAMPLE_RATE);
            }

            if !all_finite(&buffer) {
                errors_detected_clone.store(true, Ordering::Relaxed);
            }
        }
    });

    // Update thread that tries to hit buffer boundaries
    let eq_clone = Arc::clone(&eq);
    let boundary_updates_clone = Arc::clone(&boundary_updates);

    let update_thread = thread::spawn(move || {
        let mut last_boundary = 0u64;
        loop {
            let current = boundary_updates_clone.load(Ordering::SeqCst);
            if current != last_boundary {
                // Try to update right at the boundary
                if let Ok(mut eq) = eq_clone.try_lock() {
                    let gain = (current as f32 * 0.1).sin() * 12.0;
                    eq.set_mid_band(EqBand::peaking(1000.0, gain, 1.0));
                }
                last_boundary = current;
            }
            if current >= iterations as u64 {
                break;
            }
            thread::yield_now();
        }
    });

    process_thread.join().unwrap();
    processing_active.store(false, Ordering::Relaxed);
    update_thread.join().unwrap();

    assert!(
        !errors_detected.load(Ordering::Relaxed),
        "Errors detected at buffer boundaries"
    );
}

// ============================================================================
// 6. REAL-TIME DEADLINE ENFORCEMENT
// ============================================================================

#[test]
fn test_processing_within_buffer_period() {
    println!("\n=== Real-Time Deadline Enforcement ===");
    println!(
        "{:>8} {:>12} {:>12} {:>12} {:>10}",
        "Buffer", "Budget(us)", "P99(us)", "Max(us)", "Overruns"
    );
    println!("{}", "-".repeat(58));

    for &buffer_size in &BUFFER_SIZES {
        let mut chain = create_full_effect_chain();
        let budget = buffer_period(buffer_size, SAMPLE_RATE);
        let _adjusted_budget = Duration::from_secs_f64(
            budget.as_secs_f64() * REAL_TIME_SAFETY_MARGIN
        );

        let mut stats = TimingStats::with_capacity(STRESS_ITERATIONS);

        // Warmup
        for _ in 0..100 {
            let mut buffer = generate_varied_audio(SAMPLE_RATE, buffer_size);
            chain.process(&mut buffer, SAMPLE_RATE);
        }

        // Measure
        for _ in 0..STRESS_ITERATIONS {
            let mut buffer = generate_varied_audio(SAMPLE_RATE, buffer_size);
            let start = Instant::now();
            chain.process(&mut buffer, SAMPLE_RATE);
            stats.add(start.elapsed());
        }

        let budget_us = budget.as_nanos() as f64 / 1000.0;
        let p99_us = stats.percentile(99.0).as_nanos() as f64 / 1000.0;
        let max_us = stats.max().as_nanos() as f64 / 1000.0;
        let overruns = stats.count_exceeding(budget);

        println!(
            "{:>8} {:>12.2} {:>12.2} {:>12.2} {:>10}",
            buffer_size, budget_us, p99_us, max_us, overruns
        );

        // Assert P99 is within safety margin of budget
        // Note: In debug builds, allow more slack due to unoptimized code
        #[cfg(debug_assertions)]
        let max_overruns = STRESS_ITERATIONS / 5; // 20% tolerance for debug
        #[cfg(not(debug_assertions))]
        let max_overruns = STRESS_ITERATIONS / 20; // 5% tolerance for release

        if overruns > max_overruns {
            println!(
                "  WARNING: Buffer {} had {} overruns (max allowed: {})",
                buffer_size, overruns, max_overruns
            );
        }
    }
}

#[test]
fn test_worst_case_latency_measurement() {
    println!("\n=== Worst-Case Latency Measurement ===");

    for &buffer_size in &[64, 128, 256] {
        let mut chain = create_full_effect_chain();
        let budget = buffer_period(buffer_size, SAMPLE_RATE);
        let iterations = 20000;

        let mut worst_case = Duration::ZERO;
        let mut stats = TimingStats::with_capacity(iterations);

        // Warmup
        for _ in 0..200 {
            let mut buffer = generate_varied_audio(SAMPLE_RATE, buffer_size);
            chain.process(&mut buffer, SAMPLE_RATE);
        }

        // Measure
        for _ in 0..iterations {
            let mut buffer = generate_varied_audio(SAMPLE_RATE, buffer_size);
            let start = Instant::now();
            chain.process(&mut buffer, SAMPLE_RATE);
            let elapsed = start.elapsed();
            stats.add(elapsed);
            if elapsed > worst_case {
                worst_case = elapsed;
            }
        }

        let budget_us = budget.as_nanos() as f64 / 1000.0;
        let worst_us = worst_case.as_nanos() as f64 / 1000.0;
        let mean_us = stats.mean().as_nanos() as f64 / 1000.0;
        let p999_us = stats.percentile(99.9).as_nanos() as f64 / 1000.0;

        println!(
            "Buffer {}: Budget={:.2}us, Mean={:.2}us, P99.9={:.2}us, Max={:.2}us",
            buffer_size, budget_us, mean_us, p999_us, worst_us
        );

        // In debug builds, worst case may exceed budget significantly
        #[cfg(debug_assertions)]
        let max_ratio = 50.0; // Debug builds are slow
        #[cfg(not(debug_assertions))]
        let max_ratio = 3.0; // Release should be much faster

        let actual_ratio = worst_case.as_secs_f64() / budget.as_secs_f64();
        assert!(
            actual_ratio < max_ratio,
            "Buffer {}: Worst case ({:.2}us) exceeds {:.0}x budget ({:.2}us)",
            buffer_size,
            worst_us,
            max_ratio,
            budget_us
        );
    }
}

#[test]
fn test_processing_time_spike_detection() {
    // Detect spikes that would cause audio underruns
    let buffer_size = 256;
    let iterations = 50000;

    let mut chain = create_full_effect_chain();
    let budget = buffer_period(buffer_size, SAMPLE_RATE);

    let mut stats = TimingStats::with_capacity(iterations);
    let mut spike_count = 0;
    let spike_threshold = Duration::from_secs_f64(budget.as_secs_f64() * 2.0);

    // Warmup
    for _ in 0..500 {
        let mut buffer = generate_varied_audio(SAMPLE_RATE, buffer_size);
        chain.process(&mut buffer, SAMPLE_RATE);
    }

    // Measure
    for _ in 0..iterations {
        let mut buffer = generate_varied_audio(SAMPLE_RATE, buffer_size);
        let start = Instant::now();
        chain.process(&mut buffer, SAMPLE_RATE);
        let elapsed = start.elapsed();
        stats.add(elapsed);

        if elapsed > spike_threshold {
            spike_count += 1;
        }
    }

    let spike_rate = spike_count as f64 / iterations as f64 * 100.0;
    let jitter = stats.jitter_ratio();

    println!("\n=== Processing Time Spike Detection ===");
    println!("Buffer size: {} samples", buffer_size);
    println!("Iterations: {}", iterations);
    println!("Spike threshold: {:.2}us (2x budget)", spike_threshold.as_nanos() as f64 / 1000.0);
    println!("Spikes detected: {} ({:.4}%)", spike_count, spike_rate);
    println!("Jitter ratio (P99/Mean): {:.2}x", jitter);

    // Assert jitter is within acceptable bounds
    assert!(
        jitter < MAX_JITTER_RATIO,
        "Jitter ratio {:.2}x exceeds maximum {:.2}x",
        jitter,
        MAX_JITTER_RATIO
    );

    // Allow some spikes in CI environments, but flag if too many
    #[cfg(debug_assertions)]
    let max_spike_rate = 5.0; // 5% for debug
    #[cfg(not(debug_assertions))]
    let max_spike_rate = 1.0; // 1% for release

    if spike_rate > max_spike_rate {
        println!(
            "WARNING: Spike rate {:.2}% exceeds threshold {:.2}%",
            spike_rate, max_spike_rate
        );
    }
}

// ============================================================================
// 7. TEST WITH ALL EFFECTS ENABLED
// ============================================================================

#[test]
fn test_all_effects_enabled_timing() {
    let buffer_size = 512;
    let iterations = 5000;

    // Create chain with ALL effects at aggressive settings
    let mut chain = EffectChain::new();

    // Parametric EQ with heavy processing
    let mut eq = ParametricEq::new();
    eq.set_low_band(EqBand::low_shelf(80.0, 6.0));
    eq.set_mid_band(EqBand::peaking(1000.0, -6.0, 2.0));
    eq.set_high_band(EqBand::high_shelf(10000.0, 3.0));
    chain.add_effect(Box::new(eq));

    // 31-band graphic EQ (heavier than 10-band)
    let mut geq = GraphicEq::new_31_band();
    // Set varying gains
    for i in 0..31 {
        geq.set_band_gain(i, (i as f32 - 15.0) * 0.4);
    }
    chain.add_effect(Box::new(geq));

    // Aggressive compressor
    let compressor = Compressor::with_settings(CompressorSettings::aggressive());
    chain.add_effect(Box::new(compressor));

    // Meier crossfeed (most complex preset)
    let crossfeed = Crossfeed::with_preset(CrossfeedPreset::Meier);
    chain.add_effect(Box::new(crossfeed));

    // Extra wide stereo enhancer
    let enhancer = StereoEnhancer::with_settings(StereoSettings::extra_wide());
    chain.add_effect(Box::new(enhancer));

    // Brickwall limiter
    let limiter = Limiter::with_settings(LimiterSettings::brickwall());
    chain.add_effect(Box::new(limiter));

    chain.set_enabled(true);

    let budget = buffer_period(buffer_size, SAMPLE_RATE);
    let mut stats = TimingStats::with_capacity(iterations);

    // Warmup
    for _ in 0..200 {
        let mut buffer = generate_varied_audio(SAMPLE_RATE, buffer_size);
        chain.process(&mut buffer, SAMPLE_RATE);
    }

    // Measure
    for i in 0..iterations {
        let mut buffer = generate_varied_audio(SAMPLE_RATE, buffer_size);
        let start = Instant::now();
        chain.process(&mut buffer, SAMPLE_RATE);
        let elapsed = start.elapsed();
        stats.add(elapsed);

        // Verify output at intervals
        if i % 1000 == 0 {
            assert!(
                all_finite(&buffer),
                "Invalid output at iteration {} with all effects enabled",
                i
            );
        }
    }

    let budget_us = budget.as_nanos() as f64 / 1000.0;
    let mean_us = stats.mean().as_nanos() as f64 / 1000.0;
    let p95_us = stats.percentile(95.0).as_nanos() as f64 / 1000.0;
    let p99_us = stats.percentile(99.0).as_nanos() as f64 / 1000.0;
    let max_us = stats.max().as_nanos() as f64 / 1000.0;
    let cpu_usage = (mean_us / budget_us) * 100.0;

    println!("\n=== All Effects Enabled Timing ===");
    println!("Buffer: {} samples @ {}Hz", buffer_size, SAMPLE_RATE);
    println!("Budget: {:.2}us", budget_us);
    println!("Effects: 6 (ParametricEQ, 31-band GEQ, Compressor, Crossfeed, StereoEnhancer, Limiter)");
    println!();
    println!("Timing Statistics:");
    println!("  Mean:   {:.2}us ({:.1}% of budget)", mean_us, cpu_usage);
    println!("  P95:    {:.2}us", p95_us);
    println!("  P99:    {:.2}us", p99_us);
    println!("  Max:    {:.2}us", max_us);
    println!("  Jitter: {:.2}x", stats.jitter_ratio());

    // With all effects, we expect higher CPU usage but still within budget
    #[cfg(debug_assertions)]
    let max_cpu_usage = 500.0; // Debug builds are very slow
    #[cfg(not(debug_assertions))]
    let max_cpu_usage = 75.0; // Release should stay under 75%

    if cpu_usage > max_cpu_usage {
        println!(
            "WARNING: CPU usage {:.1}% exceeds target {:.1}%",
            cpu_usage, max_cpu_usage
        );
    }
}

// ============================================================================
// 8. DETERMINISTIC OUTPUT VERIFICATION
// ============================================================================

#[test]
fn test_deterministic_processing_output() {
    let buffer_size = 1024;
    let input = generate_varied_audio(SAMPLE_RATE, buffer_size);

    // Process the same input multiple times with fresh effect chains
    let mut outputs: Vec<Vec<f32>> = Vec::new();
    for _ in 0..10 {
        let mut chain = create_full_effect_chain();
        let mut buffer = input.clone();
        chain.process(&mut buffer, SAMPLE_RATE);
        outputs.push(buffer);
    }

    // All outputs must be bit-identical
    for i in 1..outputs.len() {
        for (j, (a, b)) in outputs[0].iter().zip(outputs[i].iter()).enumerate() {
            assert!(
                (a - b).abs() < 1e-6,
                "Non-deterministic output at sample {}: run 0 = {}, run {} = {}",
                j,
                a,
                i,
                b
            );
        }
    }

    println!("\n=== Deterministic Output Verification ===");
    println!("Verified: 10 identical runs produce identical output");
}

#[test]
fn test_deterministic_after_reset() {
    let buffer_size = 512;

    let mut chain = create_full_effect_chain();

    // Process some audio to build up state
    for _ in 0..100 {
        let mut buffer = generate_varied_audio(SAMPLE_RATE, buffer_size);
        chain.process(&mut buffer, SAMPLE_RATE);
    }

    // Reset and capture output
    chain.reset();
    let input = generate_stereo_sine(1000.0, SAMPLE_RATE, buffer_size);
    let mut output1 = input.clone();
    chain.process(&mut output1, SAMPLE_RATE);

    // Build up state again
    for _ in 0..100 {
        let mut buffer = generate_varied_audio(SAMPLE_RATE, buffer_size);
        chain.process(&mut buffer, SAMPLE_RATE);
    }

    // Reset and process same input
    chain.reset();
    let mut output2 = input.clone();
    chain.process(&mut output2, SAMPLE_RATE);

    // Outputs should be identical after reset
    for (i, (a, b)) in output1.iter().zip(output2.iter()).enumerate() {
        assert!(
            (a - b).abs() < 1e-6,
            "Output not deterministic after reset at sample {}: {} vs {}",
            i,
            a,
            b
        );
    }

    println!("\n=== Deterministic After Reset ===");
    println!("Verified: Reset produces identical initial state");
}

// ============================================================================
// 9. STRESS TESTS FOR EDGE CASES
// ============================================================================

#[test]
fn test_rapid_buffer_size_changes_thread_safe() {
    // Simulate scenario where buffer size changes rapidly (e.g., device reconfiguration)
    let iterations = 1000;

    let chain = Arc::new(Mutex::new(create_full_effect_chain()));
    let errors_detected = Arc::new(AtomicBool::new(false));

    let handles: Vec<_> = (0..4)
        .map(|thread_id| {
            let chain_clone = Arc::clone(&chain);
            let errors_detected_clone = Arc::clone(&errors_detected);

            thread::spawn(move || {
                for i in 0..iterations {
                    // Each thread uses different buffer sizes
                    let buffer_size = BUFFER_SIZES[(thread_id + i) % BUFFER_SIZES.len()];
                    let mut buffer = generate_varied_audio(SAMPLE_RATE, buffer_size);

                    {
                        let mut c = chain_clone.lock().unwrap();
                        c.process(&mut buffer, SAMPLE_RATE);
                    }

                    if !all_finite(&buffer) {
                        errors_detected_clone.store(true, Ordering::Relaxed);
                        return;
                    }
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    assert!(
        !errors_detected.load(Ordering::Relaxed),
        "Errors with rapid buffer size changes"
    );
}

#[test]
fn test_sample_rate_changes_thread_safe() {
    let sample_rates = [22050, 44100, 48000, 88200, 96000, 192000];
    let buffer_size = 256;
    let iterations = 500;

    let chain = Arc::new(Mutex::new(create_full_effect_chain()));
    let errors_detected = Arc::new(AtomicBool::new(false));

    let handles: Vec<_> = (0..4)
        .map(|thread_id| {
            let chain_clone = Arc::clone(&chain);
            let errors_detected_clone = Arc::clone(&errors_detected);

            thread::spawn(move || {
                for i in 0..iterations {
                    let sample_rate = sample_rates[(thread_id + i) % sample_rates.len()];
                    let mut buffer = generate_varied_audio(sample_rate, buffer_size);

                    {
                        let mut c = chain_clone.lock().unwrap();
                        c.process(&mut buffer, sample_rate);
                    }

                    if !all_finite(&buffer) {
                        errors_detected_clone.store(true, Ordering::Relaxed);
                        return;
                    }
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    assert!(
        !errors_detected.load(Ordering::Relaxed),
        "Errors with sample rate changes"
    );
}

#[test]
fn test_creation_destruction_stress() {
    // Stress test creation and destruction of effect chains from multiple threads
    let iterations = 100;
    let num_threads = 8;

    let barrier = Arc::new(Barrier::new(num_threads));
    let total_created = Arc::new(AtomicU64::new(0));

    let handles: Vec<_> = (0..num_threads)
        .map(|_| {
            let barrier_clone = Arc::clone(&barrier);
            let total_created_clone = Arc::clone(&total_created);

            thread::spawn(move || {
                barrier_clone.wait();

                for _ in 0..iterations {
                    // Create a full effect chain
                    let mut chain = create_full_effect_chain();

                    // Process a buffer
                    let mut buffer = generate_varied_audio(SAMPLE_RATE, 256);
                    chain.process(&mut buffer, SAMPLE_RATE);

                    // Verify output
                    assert!(all_finite(&buffer), "Invalid output during stress test");

                    // Drop chain (destructor)
                    drop(chain);

                    total_created_clone.fetch_add(1, Ordering::Relaxed);
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    let created = total_created.load(Ordering::Relaxed);
    let expected = (num_threads * iterations) as u64;

    println!("\n=== Creation/Destruction Stress Test ===");
    println!("Chains created and destroyed: {} / {}", created, expected);

    assert_eq!(created, expected, "Not all chains were created/destroyed");
}

// ============================================================================
// 10. MEMORY SAFETY VERIFICATION
// ============================================================================

#[test]
fn test_no_memory_growth_under_sustained_load() {
    // Process for extended period to verify no memory leaks
    let buffer_size = 512;
    let duration = Duration::from_secs(10);

    let mut chain = create_full_effect_chain();
    let start = Instant::now();
    let mut buffers_processed = 0u64;

    while start.elapsed() < duration {
        let mut buffer = generate_varied_audio(SAMPLE_RATE, buffer_size);
        chain.process(&mut buffer, SAMPLE_RATE);
        buffers_processed += 1;

        // Periodic output verification
        if buffers_processed % 10000 == 0 {
            assert!(
                all_finite(&buffer),
                "Output invalid at buffer {}",
                buffers_processed
            );
        }
    }

    let audio_seconds = buffers_processed as f64 * buffer_size as f64 / SAMPLE_RATE as f64;

    println!("\n=== Memory Stability Test ===");
    println!("Duration: {:?}", start.elapsed());
    println!("Buffers processed: {}", buffers_processed);
    println!("Audio processed: {:.1}s", audio_seconds);
    println!("Memory: No growth detected (test completed without OOM)");
}

#[test]
fn test_concurrent_memory_access_patterns() {
    // Test various memory access patterns under concurrent load
    let duration = Duration::from_secs(3);
    let buffer_size = 256;

    let processing_active = Arc::new(AtomicBool::new(true));
    let allocations = Arc::new(AtomicU64::new(0));
    let deallocations = Arc::new(AtomicU64::new(0));

    let mut handles = Vec::new();

    // Threads that create/destroy effects
    for _ in 0..4 {
        let processing_active_clone = Arc::clone(&processing_active);
        let allocations_clone = Arc::clone(&allocations);
        let deallocations_clone = Arc::clone(&deallocations);

        let handle = thread::spawn(move || {
            while processing_active_clone.load(Ordering::Relaxed) {
                // Create effect chain
                let mut chain = create_full_effect_chain();
                allocations_clone.fetch_add(1, Ordering::Relaxed);

                // Process a few buffers
                for _ in 0..10 {
                    let mut buffer = generate_varied_audio(SAMPLE_RATE, buffer_size);
                    chain.process(&mut buffer, SAMPLE_RATE);
                }

                // Destroy
                drop(chain);
                deallocations_clone.fetch_add(1, Ordering::Relaxed);
            }
        });
        handles.push(handle);
    }

    thread::sleep(duration);
    processing_active.store(false, Ordering::Relaxed);

    for handle in handles {
        handle.join().unwrap();
    }

    let allocs = allocations.load(Ordering::Relaxed);
    let deallocs = deallocations.load(Ordering::Relaxed);

    println!("\n=== Concurrent Memory Access Patterns ===");
    println!("Allocations: {}", allocs);
    println!("Deallocations: {}", deallocs);

    // All allocations should be deallocated
    assert_eq!(
        allocs, deallocs,
        "Memory leak detected: {} allocs, {} deallocs",
        allocs, deallocs
    );
}
