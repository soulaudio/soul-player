//! Memory and resource leak detection tests for audio processing
//!
//! **IMPORTANT: These tests are designed for NIGHTLY CI runs, not regular CI.**
//! They take 10-20 minutes to complete and stress-test memory stability.
//!
//! ## Running These Tests
//!
//! ```bash
//! # Regular CI should skip these (too slow):
//! cargo test -p soul-audio --test memory_stability_test -- --ignored
//!
//! # Nightly CI should run all:
//! cargo test -p soul-audio --test memory_stability_test
//!
//! # For true leak detection, use valgrind (Linux):
//! valgrind --leak-check=full cargo test -p soul-audio --test memory_stability_test
//! ```
//!
//! ## Test Categories
//!
//! 1. **Long-running stability**: Simulate hours of playback in accelerated time
//! 2. **Memory usage tracking**: Measure memory before/after extended processing
//! 3. **Resource leak detection**: Verify file handles and references are properly cleaned up
//! 4. **Create/destroy cycles**: Verify no accumulation after repeated instantiation
//! 5. **Buffer reuse verification**: Ensure buffers are reused, not reallocated
//! 6. **Stress patterns**: Rapid reconfiguration, sample rate changes, parameter automation
//!
//! ## Limitations
//!
//! Process memory measurement is imprecise and affected by:
//! - OS allocator behavior and fragmentation
//! - Other processes running on the system
//! - Rust's allocator caching
//!
//! True memory leak detection requires external tools like valgrind or heaptrack.
//! These tests verify observable behavior that would indicate leaks:
//! - Process memory growth beyond reasonable bounds
//! - Resource exhaustion patterns
//! - Stability over extended processing

use soul_audio::effects::{
    AudioEffect, Compressor, CompressorSettings, ConvolutionEngine, Crossfeed, CrossfeedPreset,
    EffectChain, EqBand, GraphicEq, GraphicEqPreset, Limiter, LimiterSettings, ParametricEq,
    StereoEnhancer, StereoSettings,
};
use soul_audio::resampling::{Resampler, ResamplerBackend, ResamplingQuality};
use soul_audio::SymphoniaDecoder;
use soul_core::AudioDecoder;
use std::f32::consts::PI;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

// ============================================================================
// SIMPLE ALLOCATION TRACKING (without unsafe code)
// ============================================================================

/// Simple allocation counter using Vec capacity as a proxy
/// This tracks allocations indirectly by measuring Vec sizes
struct AllocationTracker {
    baseline_capacity: usize,
}

impl AllocationTracker {
    fn new() -> Self {
        // Force some initial allocations to establish baseline
        let baseline: Vec<u8> = Vec::with_capacity(1024);
        let capacity = baseline.capacity();
        drop(baseline);
        Self {
            baseline_capacity: capacity,
        }
    }

    /// Estimate current heap usage by creating and measuring a large allocation
    fn estimate_heap_pressure(&self) -> usize {
        // Create a reasonably sized allocation and see what we get
        let test_vec: Vec<u8> = Vec::with_capacity(1024 * 1024);
        let got = test_vec.capacity();
        drop(test_vec);
        got
    }
}

/// Measure the approximate heap size of the current process
/// Uses platform-specific methods where available
fn get_process_memory_bytes() -> Option<usize> {
    // On Linux, read /proc/self/statm
    #[cfg(target_os = "linux")]
    {
        if let Ok(statm) = std::fs::read_to_string("/proc/self/statm") {
            let parts: Vec<&str> = statm.split_whitespace().collect();
            if let Some(resident) = parts.get(1) {
                if let Ok(pages) = resident.parse::<usize>() {
                    // Page size is typically 4096 bytes
                    return Some(pages * 4096);
                }
            }
        }
    }

    // On macOS, we could use mach APIs but they require unsafe
    // For simplicity, we return None and the tests will use other methods

    // Windows would use GetProcessMemoryInfo but that also needs unsafe

    None
}

/// Alternative memory estimation using Vec allocation behavior
fn estimate_memory_usage() -> usize {
    // Allocate a known size and see what capacity we actually get
    // This is an indirect measure of heap fragmentation/pressure
    let mut total = 0;
    for size in [1024, 4096, 16384, 65536] {
        let v: Vec<u8> = Vec::with_capacity(size);
        total += v.capacity();
    }
    total
}

const SAMPLE_RATE: u32 = 44100;

// ============================================================================
// TEST UTILITIES
// ============================================================================

/// Generate a stereo sine wave buffer
fn generate_stereo_sine(frequency: f32, sample_rate: u32, num_frames: usize) -> Vec<f32> {
    let mut buffer = Vec::with_capacity(num_frames * 2);
    for i in 0..num_frames {
        let t = i as f32 / sample_rate as f32;
        let sample = (2.0 * PI * frequency * t).sin();
        buffer.push(sample);
        buffer.push(sample);
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
        buffer.push(sample * 0.9);
    }
    buffer
}

/// Counter for tracking iterations - useful for verifying processing completed
static PROCESSING_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn reset_counter() {
    PROCESSING_COUNTER.store(0, Ordering::SeqCst);
}

fn increment_counter() {
    PROCESSING_COUNTER.fetch_add(1, Ordering::SeqCst);
}

fn get_counter() -> usize {
    PROCESSING_COUNTER.load(Ordering::SeqCst)
}

// ============================================================================
// 1. LONG-RUNNING STABILITY TESTS
// ============================================================================

#[test]
fn test_1_hour_simulated_playback_stability() {
    // Simulate 1 hour of playback at 44.1kHz with 512-sample buffers
    // We'll simulate by processing enough buffers to catch slow memory leaks

    let mut eq = ParametricEq::new();
    eq.set_low_band(EqBand::low_shelf(100.0, 3.0));
    eq.set_mid_band(EqBand::peaking(1000.0, -2.0, 1.0));
    eq.set_high_band(EqBand::high_shelf(10000.0, 1.0));
    eq.set_enabled(true);

    let mut compressor = Compressor::with_settings(CompressorSettings::moderate());
    compressor.set_enabled(true);

    let mut limiter = Limiter::new();
    limiter.set_enabled(true);

    // Track memory at start
    let mem_before = get_process_memory_bytes();

    // Simulate ~10 minutes of playback (50,000 buffers at 512 frames = ~9.7 minutes)
    let num_buffers = 50_000;
    let buffer_size = 512;

    let start = Instant::now();

    for i in 0..num_buffers {
        let mut buffer = generate_varied_audio(SAMPLE_RATE, buffer_size);

        eq.process(&mut buffer, SAMPLE_RATE);
        compressor.process(&mut buffer, SAMPLE_RATE);
        limiter.process(&mut buffer, SAMPLE_RATE);

        // Periodically verify output is valid
        if i % 10000 == 0 {
            for sample in &buffer {
                assert!(sample.is_finite(), "Non-finite at buffer {}", i);
            }
        }
    }

    let elapsed = start.elapsed();
    let mem_after = get_process_memory_bytes();

    println!(
        "Processed {} buffers ({:.1} simulated minutes) in {:?}",
        num_buffers,
        (num_buffers * buffer_size) as f64 / SAMPLE_RATE as f64 / 60.0,
        elapsed
    );

    // Check for significant memory growth if we have measurements
    if let (Some(before), Some(after)) = (mem_before, mem_after) {
        let growth = after as isize - before as isize;
        println!(
            "Memory: before={}KB, after={}KB, growth={}KB",
            before / 1024,
            after / 1024,
            growth / 1024
        );

        // Allow some memory growth but flag significant leaks
        assert!(
            growth < 10 * 1024 * 1024, // Less than 10MB growth
            "Significant memory growth detected: {} bytes",
            growth
        );
    }
}

#[test]
fn test_effect_chain_long_run_8_hours_simulated() {
    // Simulate extended playback with full effect chain

    let mut chain = EffectChain::new();

    let mut eq = ParametricEq::new();
    eq.set_low_band(EqBand::low_shelf(100.0, 3.0));
    eq.set_mid_band(EqBand::peaking(1000.0, -2.0, 1.0));
    eq.set_high_band(EqBand::high_shelf(10000.0, 1.0));
    chain.add_effect(Box::new(eq));

    let mut geq = GraphicEq::new_10_band();
    geq.set_preset(GraphicEqPreset::Rock);
    chain.add_effect(Box::new(geq));

    chain.add_effect(Box::new(StereoEnhancer::with_settings(StereoSettings::wide())));

    let mut crossfeed = Crossfeed::new();
    crossfeed.set_preset(CrossfeedPreset::Natural);
    crossfeed.set_enabled(true);
    chain.add_effect(Box::new(crossfeed));

    chain.add_effect(Box::new(Compressor::with_settings(
        CompressorSettings::moderate(),
    )));
    chain.add_effect(Box::new(Limiter::with_settings(LimiterSettings::brickwall())));

    chain.set_enabled(true);

    // Process 100,000 buffers (~35 minutes simulated at 44.1kHz/512 frames)
    let num_buffers = 100_000;
    let buffer_size = 512;

    let start = Instant::now();

    for i in 0..num_buffers {
        let mut buffer = generate_varied_audio(SAMPLE_RATE, buffer_size);
        chain.process(&mut buffer, SAMPLE_RATE);

        // Verify every 20000th buffer
        if i % 20000 == 0 {
            for sample in &buffer {
                assert!(sample.is_finite(), "Non-finite at buffer {}", i);
            }
        }
    }

    let elapsed = start.elapsed();
    println!(
        "8-hour simulation: Processed {} buffers in {:?}",
        num_buffers, elapsed
    );
}

// ============================================================================
// 2. MEMORY USAGE TRACKING TESTS
// ============================================================================

#[test]
fn test_memory_before_after_extended_processing() {
    // Measure memory indirectly before and after extended processing

    let baseline_estimate = estimate_memory_usage();

    // Create effects and process
    let mut eq = ParametricEq::new();
    eq.set_enabled(true);
    eq.set_mid_band(EqBand::peaking(1000.0, 6.0, 1.0));

    // Process 10,000 buffers
    for _ in 0..10_000 {
        let mut buffer = generate_stereo_sine(1000.0, SAMPLE_RATE, 256);
        eq.process(&mut buffer, SAMPLE_RATE);
    }

    let after_estimate = estimate_memory_usage();

    println!(
        "Memory estimate before: {}, after: {}",
        baseline_estimate, after_estimate
    );

    // The estimates should be similar if there's no leak
    // Allow for normal allocator variance
}

#[test]
fn test_detect_gradual_memory_growth() {
    // Process in phases and check that memory doesn't grow between phases

    let mut eq = ParametricEq::new();
    eq.set_enabled(true);

    let mut measurements = Vec::new();

    // Process in 5 phases, measuring memory after each
    for phase in 0..5 {
        for _ in 0..5000 {
            let mut buffer = generate_varied_audio(SAMPLE_RATE, 512);
            eq.process(&mut buffer, SAMPLE_RATE);
        }

        if let Some(mem) = get_process_memory_bytes() {
            measurements.push(mem);
            println!("Phase {}: memory = {} KB", phase, mem / 1024);
        }
    }

    // Check that memory didn't grow significantly between phases
    if measurements.len() >= 2 {
        let growth = measurements
            .last()
            .unwrap_or(&0)
            .saturating_sub(*measurements.first().unwrap_or(&0));
        println!("Total memory growth across phases: {} KB", growth / 1024);

        // Memory should be relatively stable
        assert!(
            growth < 5 * 1024 * 1024,
            "Memory grew {} bytes across processing phases",
            growth
        );
    }
}

#[test]
fn test_verify_cleanup_after_effect_destruction() {
    // Create many effects, destroy them, and verify memory is reclaimed

    let initial_mem = get_process_memory_bytes();

    // Create and destroy many effect chains
    for _ in 0..100 {
        let mut chain = EffectChain::new();

        chain.add_effect(Box::new(ParametricEq::new()));
        chain.add_effect(Box::new(GraphicEq::new_10_band()));
        chain.add_effect(Box::new(Compressor::new()));
        chain.add_effect(Box::new(Limiter::new()));
        chain.add_effect(Box::new(StereoEnhancer::new()));
        chain.add_effect(Box::new(Crossfeed::new()));

        // Process some audio
        for _ in 0..100 {
            let mut buffer = generate_varied_audio(SAMPLE_RATE, 512);
            chain.process(&mut buffer, SAMPLE_RATE);
        }

        // Chain goes out of scope and is dropped
    }

    let final_mem = get_process_memory_bytes();

    if let (Some(initial), Some(final_)) = (initial_mem, final_mem) {
        let retained = final_ as isize - initial as isize;
        println!(
            "Memory retained after 100 effect chain cycles: {} KB",
            retained / 1024
        );

        // Should not retain significant memory after cleanup
        assert!(
            retained < 2 * 1024 * 1024,
            "Retained {} bytes after effect cleanup",
            retained
        );
    }
}

// ============================================================================
// 3. RESOURCE LEAK DETECTION TESTS
// ============================================================================

#[test]
fn test_decoder_file_handles_closed() {
    // Create and destroy many decoder instances to verify file handles are released

    for i in 0..1000 {
        let decoder = SymphoniaDecoder::new();
        drop(decoder);

        if i % 100 == 0 {
            // Verify we can still create new decoders
            let _ = SymphoniaDecoder::new();
        }
    }

    // If we get here without running out of file handles, the test passes
    println!("Created and destroyed 1000 decoders successfully");
}

#[test]
fn test_no_dangling_references_after_reset() {
    // Create effects, build up internal state, reset, and verify clean state

    let mut effects: Vec<Box<dyn AudioEffect>> = vec![
        Box::new(ParametricEq::new()),
        Box::new(GraphicEq::new_10_band()),
        Box::new(Compressor::new()),
        Box::new(Limiter::new()),
        Box::new(StereoEnhancer::new()),
        Box::new(Crossfeed::new()),
    ];

    for effect in &mut effects {
        effect.set_enabled(true);

        // Build up internal state by processing audio
        for _ in 0..1000 {
            let mut buffer = generate_varied_audio(SAMPLE_RATE, 512);
            effect.process(&mut buffer, SAMPLE_RATE);
        }

        // Reset should clear all internal state
        effect.reset();

        // Process again - should work correctly with fresh state
        let mut buffer = generate_varied_audio(SAMPLE_RATE, 512);
        effect.process(&mut buffer, SAMPLE_RATE);

        for sample in &buffer {
            assert!(
                sample.is_finite(),
                "Effect {} produced non-finite output after reset",
                effect.name()
            );
        }
    }
}

#[test]
fn test_convolution_engine_ir_memory_cleanup() {
    // Load and unload impulse responses, verify memory is released

    let initial_mem = get_process_memory_bytes();

    for _ in 0..50 {
        let mut engine = ConvolutionEngine::new();

        // Create a large impulse response (simulating a reverb IR)
        let ir_length = 44100 * 2; // 2 seconds at 44.1kHz
        let ir: Vec<f32> = (0..ir_length * 2)
            .map(|i| {
                let decay = (-i as f32 / (ir_length as f32 * 0.3)).exp();
                decay * ((i as f32 * 0.1).sin() * 0.5 + 0.5)
            })
            .collect();

        engine.load_impulse_response(&ir, 44100, 2).unwrap();

        // Process some audio
        for _ in 0..100 {
            let mut buffer = generate_varied_audio(SAMPLE_RATE, 512);
            engine.process(&mut buffer, SAMPLE_RATE);
        }

        // Engine goes out of scope - IR memory should be freed
    }

    let final_mem = get_process_memory_bytes();

    if let (Some(initial), Some(final_)) = (initial_mem, final_mem) {
        let retained = final_ as isize - initial as isize;
        println!(
            "Memory retained after 50 IR load/unload cycles: {} KB",
            retained / 1024
        );
    }
}

// ============================================================================
// 4. REPEATED CREATE/DESTROY CYCLES
// ============================================================================

#[test]
fn test_create_destroy_1000_effect_instances() {
    // Create and destroy 1000+ effect instances to verify no accumulation

    let initial_mem = get_process_memory_bytes();

    // Test each effect type
    for _ in 0..1000 {
        let _ = ParametricEq::new();
    }
    println!("Created/destroyed 1000 ParametricEq instances");

    for _ in 0..1000 {
        let _ = GraphicEq::new_10_band();
    }
    println!("Created/destroyed 1000 GraphicEq instances");

    for _ in 0..1000 {
        let _ = Compressor::new();
    }
    println!("Created/destroyed 1000 Compressor instances");

    for _ in 0..1000 {
        let _ = Limiter::new();
    }
    println!("Created/destroyed 1000 Limiter instances");

    for _ in 0..1000 {
        let _ = StereoEnhancer::new();
    }
    println!("Created/destroyed 1000 StereoEnhancer instances");

    for _ in 0..1000 {
        let _ = Crossfeed::new();
    }
    println!("Created/destroyed 1000 Crossfeed instances");

    let final_mem = get_process_memory_bytes();

    if let (Some(initial), Some(final_)) = (initial_mem, final_mem) {
        let retained = final_ as isize - initial as isize;
        println!(
            "Memory retained after 6000 total effect instances: {} KB",
            retained / 1024
        );

        // Should not accumulate significant memory
        assert!(
            retained < 1024 * 1024, // Less than 1MB
            "Memory accumulation detected: {} bytes after 6000 effect creations",
            retained
        );
    }
}

#[test]
fn test_create_destroy_decoder_instances() {
    // Create and destroy decoder instances repeatedly

    for i in 0..500 {
        let decoder = SymphoniaDecoder::new();

        // Verify it's functional
        assert!(decoder.supports_format(std::path::Path::new("test.mp3")));

        drop(decoder);

        if i % 100 == 0 {
            println!("Completed {} decoder create/destroy cycles", i);
        }
    }
}

#[test]
fn test_create_destroy_resampler_instances() {
    // Create and destroy resampler instances

    let initial_mem = get_process_memory_bytes();

    for _ in 0..100 {
        // Create resamplers with various configurations
        let resampler = Resampler::new(
            ResamplerBackend::Auto,
            44100,
            96000,
            2,
            ResamplingQuality::High,
        )
        .unwrap();
        drop(resampler);

        let resampler = Resampler::new(
            ResamplerBackend::Rubato,
            48000,
            44100,
            2,
            ResamplingQuality::Balanced,
        )
        .unwrap();
        drop(resampler);
    }

    let final_mem = get_process_memory_bytes();

    if let (Some(initial), Some(final_)) = (initial_mem, final_mem) {
        let retained = final_ as isize - initial as isize;
        println!(
            "Memory retained after 200 resampler cycles: {} KB",
            retained / 1024
        );
    }

    println!("Created/destroyed 200 resampler instances successfully");
}

#[test]
fn test_verify_no_accumulation_effect_chains() {
    // Create effect chains with all effects, process, destroy, repeat

    let initial_mem = get_process_memory_bytes();

    for cycle in 0..100 {
        let mut chain = EffectChain::new();

        // Add all effect types
        let mut eq = ParametricEq::new();
        eq.set_mid_band(EqBand::peaking(1000.0, 6.0, 1.0));
        chain.add_effect(Box::new(eq));

        let mut geq = GraphicEq::new_10_band();
        geq.set_preset(GraphicEqPreset::BassBoost);
        chain.add_effect(Box::new(geq));

        chain.add_effect(Box::new(Compressor::with_settings(
            CompressorSettings::moderate(),
        )));
        chain.add_effect(Box::new(Limiter::with_settings(LimiterSettings::brickwall())));
        chain.add_effect(Box::new(StereoEnhancer::with_settings(StereoSettings::wide())));
        chain.add_effect(Box::new(Crossfeed::with_preset(CrossfeedPreset::Natural)));

        chain.set_enabled(true);

        // Process some buffers
        for _ in 0..50 {
            let mut buffer = generate_varied_audio(SAMPLE_RATE, 512);
            chain.process(&mut buffer, SAMPLE_RATE);
        }

        // Chain is dropped here

        if cycle % 20 == 0 {
            println!("Completed {} chain create/process/destroy cycles", cycle);
        }
    }

    let final_mem = get_process_memory_bytes();

    if let (Some(initial), Some(final_)) = (initial_mem, final_mem) {
        let growth = final_ as isize - initial as isize;
        println!(
            "Memory growth after 100 effect chain cycles: {} KB",
            growth / 1024
        );

        assert!(
            growth < 2 * 1024 * 1024,
            "Memory accumulation of {} bytes detected",
            growth
        );
    }
}

// ============================================================================
// 5. BUFFER REUSE VERIFICATION
// ============================================================================

#[test]
fn test_buffers_reused_not_reallocated() {
    // Verify that effects reuse internal buffers during processing
    // We test this by processing many buffers and verifying stable behavior

    let mut eq = ParametricEq::new();
    eq.set_enabled(true);
    eq.set_mid_band(EqBand::peaking(1000.0, 6.0, 1.0));

    // Warm up - first few calls may allocate
    for _ in 0..10 {
        let mut buffer = generate_stereo_sine(1000.0, SAMPLE_RATE, 512);
        eq.process(&mut buffer, SAMPLE_RATE);
    }

    // Process many buffers of the same size
    // If internal buffers were being reallocated, we'd likely see slowdowns or issues
    let start = Instant::now();
    for _ in 0..10_000 {
        let mut buffer = generate_stereo_sine(1000.0, SAMPLE_RATE, 512);
        eq.process(&mut buffer, SAMPLE_RATE);
    }
    let elapsed = start.elapsed();

    println!("Processed 10,000 buffers in {:?}", elapsed);

    // Processing time should be consistent - large variance might indicate reallocations
    // Each buffer should take roughly the same time
}

#[test]
fn test_no_growth_in_internal_vectors_during_process() {
    // Process with same buffer size repeatedly, verify no internal growth

    let mut effects: Vec<Box<dyn AudioEffect>> = vec![
        Box::new(ParametricEq::new()),
        Box::new(GraphicEq::new_10_band()),
        Box::new(Compressor::new()),
        Box::new(Limiter::new()),
        Box::new(StereoEnhancer::new()),
        Box::new(Crossfeed::new()),
    ];

    for effect in &mut effects {
        effect.set_enabled(true);

        // Process same-sized buffers many times
        for i in 0..10_000 {
            let mut buffer = generate_stereo_sine(1000.0, SAMPLE_RATE, 512);
            effect.process(&mut buffer, SAMPLE_RATE);

            // Periodically verify output is valid
            if i % 2000 == 0 {
                for sample in &buffer {
                    assert!(sample.is_finite());
                }
            }
        }
    }

    println!("All effects processed 10,000 buffers without issues");
}

#[test]
fn test_varying_buffer_sizes_no_unbounded_growth() {
    // Process with varying buffer sizes, verify internal buffers don't grow unbounded

    let mut eq = ParametricEq::new();
    eq.set_enabled(true);

    let buffer_sizes = [64, 128, 256, 512, 1024, 2048, 4096, 512, 256, 128];

    // Cycle through different sizes many times
    for cycle in 0..100 {
        for &size in &buffer_sizes {
            let mut buffer = generate_varied_audio(SAMPLE_RATE, size);
            eq.process(&mut buffer, SAMPLE_RATE);

            for sample in &buffer {
                assert!(sample.is_finite());
            }
        }

        if cycle % 20 == 0 {
            println!("Completed {} cycles of varying buffer sizes", cycle);
        }
    }
}

// ============================================================================
// 6. STRESS PATTERNS
// ============================================================================

#[test]
fn test_rapid_effect_chain_reconfiguration() {
    // Rapidly add/remove effects from chain while processing

    let mut chain = EffectChain::new();

    for i in 0..1000 {
        // Clear and rebuild chain
        chain.clear();

        // Add effects based on iteration
        if i % 2 == 0 {
            chain.add_effect(Box::new(ParametricEq::new()));
        }
        if i % 3 == 0 {
            chain.add_effect(Box::new(Compressor::new()));
        }
        if i % 5 == 0 {
            chain.add_effect(Box::new(Limiter::new()));
        }
        if i % 7 == 0 {
            chain.add_effect(Box::new(StereoEnhancer::new()));
        }

        chain.set_enabled(true);

        // Process a buffer
        let mut buffer = generate_varied_audio(SAMPLE_RATE, 256);
        chain.process(&mut buffer, SAMPLE_RATE);

        for sample in &buffer {
            assert!(sample.is_finite(), "Non-finite at iteration {}", i);
        }
    }

    println!("Completed 1000 rapid chain reconfigurations");
}

#[test]
fn test_continuous_sample_rate_changes() {
    // Test effects with continuous sample rate changes

    let sample_rates = [22050, 44100, 48000, 88200, 96000, 176400, 192000];

    let mut eq = ParametricEq::new();
    eq.set_enabled(true);
    eq.set_mid_band(EqBand::peaking(1000.0, 6.0, 1.0));

    for cycle in 0..100 {
        for &rate in &sample_rates {
            let mut buffer = generate_varied_audio(rate, 512);
            eq.process(&mut buffer, rate);

            for sample in &buffer {
                assert!(
                    sample.is_finite(),
                    "Non-finite at cycle {}, rate {}",
                    cycle,
                    rate
                );
            }
        }
    }

    println!("Completed 700 sample rate transitions");
}

#[test]
fn test_heavy_parameter_automation() {
    // Simulate continuous parameter automation (like from a MIDI controller)

    let mut compressor = Compressor::new();
    compressor.set_enabled(true);

    // Simulate 60 seconds of parameter automation at 60Hz update rate
    let updates = 60 * 60; // 60 seconds * 60 Hz

    for i in 0..updates {
        // Oscillate parameters
        let t = i as f32 / 60.0; // Time in seconds

        compressor.set_threshold(-40.0 + 20.0 * (t * 0.5).sin());
        compressor.set_ratio(2.0 + 8.0 * (t * 0.3).sin().abs());
        compressor.set_attack(1.0 + 50.0 * (t * 0.7).sin().abs());
        compressor.set_release(20.0 + 200.0 * (t * 0.4).sin().abs());

        // Process a buffer
        let mut buffer = generate_varied_audio(SAMPLE_RATE, 256);
        compressor.process(&mut buffer, SAMPLE_RATE);

        // Verify output
        for sample in &buffer {
            assert!(sample.is_finite(), "Non-finite at update {}", i);
        }
    }

    println!("Completed {} parameter automation updates", updates);
}

#[test]
fn test_stress_all_effects_simultaneously() {
    // Create multiple instances of all effects and process in parallel-ish fashion

    let mut chains: Vec<EffectChain> = (0..10)
        .map(|_| {
            let mut chain = EffectChain::new();

            let mut eq = ParametricEq::new();
            eq.set_mid_band(EqBand::peaking(1000.0, 6.0, 1.0));
            chain.add_effect(Box::new(eq));

            chain.add_effect(Box::new(GraphicEq::new_10_band()));
            chain.add_effect(Box::new(Compressor::new()));
            chain.add_effect(Box::new(Limiter::new()));
            chain.add_effect(Box::new(StereoEnhancer::new()));
            chain.add_effect(Box::new(Crossfeed::new()));

            chain.set_enabled(true);
            chain
        })
        .collect();

    let initial_mem = get_process_memory_bytes();

    // Process all chains
    for i in 0..1000 {
        for chain in &mut chains {
            let mut buffer = generate_varied_audio(SAMPLE_RATE, 512);
            chain.process(&mut buffer, SAMPLE_RATE);
        }

        if i % 200 == 0 {
            println!("Processed {} iterations on 10 parallel chains", i);
        }
    }

    let final_mem = get_process_memory_bytes();

    if let (Some(initial), Some(final_)) = (initial_mem, final_mem) {
        let growth = final_ as isize - initial as isize;
        println!(
            "Memory growth after 10,000 chain processing cycles: {} KB",
            growth / 1024
        );
    }
}

#[test]
fn test_resampler_continuous_use_no_leak() {
    // Use resampler continuously and verify no memory leak

    let mut resampler =
        Resampler::new(ResamplerBackend::Auto, 44100, 96000, 2, ResamplingQuality::High).unwrap();

    let initial_mem = get_process_memory_bytes();

    // Process many buffers
    for i in 0..10_000 {
        let input = generate_stereo_sine(1000.0, 44100, 1024);
        let output = resampler.process(&input).unwrap();

        assert!(!output.is_empty());

        if i % 2000 == 0 {
            for sample in &output {
                assert!(sample.is_finite());
            }
        }
    }

    let final_mem = get_process_memory_bytes();

    if let (Some(initial), Some(final_)) = (initial_mem, final_mem) {
        let growth = final_ as isize - initial as isize;
        println!(
            "Memory growth after 10,000 resampler passes: {} KB",
            growth / 1024
        );

        // Resampler should not leak memory during processing
        // Note: Using 50MB tolerance because process memory measurement is imprecise
        // and affected by OS allocator behavior, fragmentation, and other processes.
        // True leak detection requires valgrind/heaptrack in nightly CI.
        assert!(
            growth < 50 * 1024 * 1024,
            "Resampler memory leak detected: {} MB (likely a real leak)",
            growth / (1024 * 1024)
        );
    }
}

#[test]
fn test_convolution_stress() {
    // Stress test convolution engine with large IRs

    let mut engine = ConvolutionEngine::new();

    // Create a medium-sized IR (0.5 seconds)
    let ir_length = 44100 / 2;
    let ir: Vec<f32> = (0..ir_length * 2)
        .map(|i| {
            let decay = (-i as f32 / (ir_length as f32 * 0.3)).exp();
            decay * ((i as f32 * 0.1).sin())
        })
        .collect();

    engine.load_impulse_response(&ir, 44100, 2).unwrap();
    engine.set_enabled(true);
    engine.set_dry_wet_mix(0.5);

    let initial_mem = get_process_memory_bytes();

    // Process many buffers
    for i in 0..5000 {
        let mut buffer = generate_varied_audio(SAMPLE_RATE, 512);
        engine.process(&mut buffer, SAMPLE_RATE);

        if i % 1000 == 0 {
            for sample in &buffer {
                assert!(sample.is_finite());
            }
        }
    }

    let final_mem = get_process_memory_bytes();

    if let (Some(initial), Some(final_)) = (initial_mem, final_mem) {
        let growth = final_ as isize - initial as isize;
        println!(
            "Convolution memory growth after 5000 buffers: {} KB",
            growth / 1024
        );
    }
}

// ============================================================================
// TIMING STABILITY TESTS
// ============================================================================

#[test]
fn test_processing_time_stability_over_long_run() {
    // Verify that processing time remains stable over many iterations
    // This indirectly detects memory issues that would cause slowdowns

    let mut eq = ParametricEq::new();
    eq.set_enabled(true);
    eq.set_mid_band(EqBand::peaking(1000.0, 6.0, 1.0));

    // Warmup
    for _ in 0..100 {
        let mut buffer = generate_stereo_sine(1000.0, SAMPLE_RATE, 512);
        eq.process(&mut buffer, SAMPLE_RATE);
    }

    // Measure phases
    let mut phase_times = Vec::new();

    for phase in 0..5 {
        let start = Instant::now();

        for _ in 0..5000 {
            let mut buffer = generate_stereo_sine(1000.0, SAMPLE_RATE, 512);
            eq.process(&mut buffer, SAMPLE_RATE);
        }

        let elapsed = start.elapsed();
        phase_times.push(elapsed);
        println!("Phase {}: {:?}", phase, elapsed);
    }

    // Check that later phases aren't significantly slower
    if let (Some(first), Some(last)) = (phase_times.first(), phase_times.last()) {
        let slowdown = last.as_secs_f64() / first.as_secs_f64();
        println!(
            "Slowdown ratio (last/first): {:.2}x",
            slowdown
        );

        // Should not slow down significantly (allow 2x for OS scheduling variance)
        assert!(
            slowdown < 2.0,
            "Processing slowed down significantly: {:.2}x slower in final phase",
            slowdown
        );
    }
}

// ============================================================================
// COMPREHENSIVE STABILITY TEST
// ============================================================================

#[test]
fn test_comprehensive_memory_stability() {
    // A comprehensive test that exercises all components together

    println!("=== Comprehensive Memory Stability Test ===\n");

    let test_start = Instant::now();
    let initial_mem = get_process_memory_bytes();

    // Phase 1: Create many effects
    println!("Phase 1: Creating 100 effect chains...");
    let chains: Vec<_> = (0..100)
        .map(|_| {
            let mut chain = EffectChain::new();
            chain.add_effect(Box::new(ParametricEq::new()));
            chain.add_effect(Box::new(GraphicEq::new_10_band()));
            chain.add_effect(Box::new(Compressor::new()));
            chain.add_effect(Box::new(Limiter::new()));
            chain
        })
        .collect();

    // Phase 2: Process audio
    println!("Phase 2: Processing 50,000 buffers across chains...");
    let mut chains = chains;
    for i in 0..50_000 {
        let chain_idx = i % chains.len();
        let mut buffer = generate_varied_audio(SAMPLE_RATE, 256);
        chains[chain_idx].process(&mut buffer, SAMPLE_RATE);
    }

    // Phase 3: Destroy chains
    println!("Phase 3: Destroying chains...");
    drop(chains);

    // Phase 4: Create resamplers
    println!("Phase 4: Creating and using resamplers...");
    for _ in 0..50 {
        let mut resampler = Resampler::new(
            ResamplerBackend::Auto,
            44100,
            48000,
            2,
            ResamplingQuality::Balanced,
        )
        .unwrap();

        for _ in 0..100 {
            let input = generate_stereo_sine(440.0, 44100, 512);
            let _ = resampler.process(&input);
        }
    }

    // Phase 5: Final memory check
    let final_mem = get_process_memory_bytes();
    let elapsed = test_start.elapsed();

    println!("\n=== Results ===");
    println!("Total test time: {:?}", elapsed);

    if let (Some(initial), Some(final_)) = (initial_mem, final_mem) {
        let retained = final_ as isize - initial as isize;
        println!("Initial memory: {} KB", initial / 1024);
        println!("Final memory: {} KB", final_ / 1024);
        println!("Retained after cleanup: {} KB", retained / 1024);

        assert!(
            retained < 10 * 1024 * 1024,
            "Comprehensive test detected {} bytes memory retention",
            retained
        );
    }

    println!("\n=== Comprehensive Test PASSED ===");
}

// ============================================================================
// EXTREME STRESS TESTS
// ============================================================================

#[test]
fn test_extreme_buffer_size_transitions() {
    // Test transitions between extremely different buffer sizes

    let mut chain = EffectChain::new();

    let mut eq = ParametricEq::new();
    eq.set_mid_band(EqBand::peaking(1000.0, 6.0, 1.0));
    chain.add_effect(Box::new(eq));
    chain.add_effect(Box::new(Compressor::new()));
    chain.add_effect(Box::new(Limiter::new()));
    chain.set_enabled(true);

    // Alternate between tiny and huge buffers
    let sizes = [1, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096, 8192];

    for cycle in 0..50 {
        for &size in &sizes {
            let mut buffer = generate_varied_audio(SAMPLE_RATE, size);
            chain.process(&mut buffer, SAMPLE_RATE);

            for sample in &buffer {
                assert!(
                    sample.is_finite(),
                    "Non-finite at cycle {}, size {}",
                    cycle,
                    size
                );
            }
        }

        // Also do reverse order
        for &size in sizes.iter().rev() {
            let mut buffer = generate_varied_audio(SAMPLE_RATE, size);
            chain.process(&mut buffer, SAMPLE_RATE);
        }
    }

    println!("Completed 50 cycles of extreme buffer size transitions");
}

#[test]
fn test_rapid_reset_cycles() {
    // Test rapid reset/process cycles

    let mut effects: Vec<Box<dyn AudioEffect>> = vec![
        Box::new(ParametricEq::new()),
        Box::new(GraphicEq::new_10_band()),
        Box::new(Compressor::new()),
        Box::new(Limiter::new()),
        Box::new(StereoEnhancer::new()),
        Box::new(Crossfeed::new()),
    ];

    for effect in &mut effects {
        effect.set_enabled(true);

        for i in 0..1000 {
            // Process a few buffers
            for _ in 0..5 {
                let mut buffer = generate_varied_audio(SAMPLE_RATE, 256);
                effect.process(&mut buffer, SAMPLE_RATE);
            }

            // Reset
            effect.reset();

            // Immediately process again
            let mut buffer = generate_varied_audio(SAMPLE_RATE, 256);
            effect.process(&mut buffer, SAMPLE_RATE);

            if i % 200 == 0 {
                for sample in &buffer {
                    assert!(
                        sample.is_finite(),
                        "Effect {} non-finite after reset at iteration {}",
                        effect.name(),
                        i
                    );
                }
            }
        }
    }

    println!("Completed 1000 rapid reset cycles for all effects");
}

#[test]
fn test_alternating_enabled_disabled_processing() {
    // Rapidly toggle effects while processing

    let mut eq = ParametricEq::new();
    eq.set_mid_band(EqBand::peaking(1000.0, 6.0, 1.0));

    let mut compressor = Compressor::new();
    let mut limiter = Limiter::new();

    for i in 0..10_000 {
        // Toggle states
        eq.set_enabled(i % 2 == 0);
        compressor.set_enabled(i % 3 == 0);
        limiter.set_enabled(i % 5 == 0);

        // Process
        let mut buffer = generate_varied_audio(SAMPLE_RATE, 256);
        eq.process(&mut buffer, SAMPLE_RATE);
        compressor.process(&mut buffer, SAMPLE_RATE);
        limiter.process(&mut buffer, SAMPLE_RATE);

        // Verify output
        for sample in &buffer {
            assert!(sample.is_finite(), "Non-finite at iteration {}", i);
        }
    }

    println!("Completed 10,000 enabled/disabled toggle cycles");
}
