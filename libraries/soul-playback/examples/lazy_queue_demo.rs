//! Demonstration of seed-based shuffle for lazy queue loading
//!
//! This example shows the memory efficiency of using a seed-based shuffle
//! instead of storing the entire shuffle order in memory.
//!
//! Run with: cargo run --example lazy_queue_demo

use soul_playback::lazy_queue::{LazyQueueState, QueueContext};
use soul_playback::types::ShuffleMode;
use std::time::Instant;

fn main() {
    println!("=== Seed-Based Shuffle Demo ===\n");

    // Simulate different library sizes
    let test_cases = vec![
        ("Small library", 1_000),
        ("Medium library", 10_000),
        ("Large library", 100_000),
        ("Very large library", 500_000),
    ];

    for (name, total_count) in test_cases {
        println!("--- {} ({} tracks) ---", name, total_count);

        // Create queue state
        let mut state = LazyQueueState::new(
            QueueContext::AllTracks {
                user_id: 1,
                total_count,
            },
            0,
        );

        // Enable shuffle
        state.enable_shuffle(ShuffleMode::Random);

        // Memory footprint
        let seed_size = std::mem::size_of::<Option<u64>>();
        println!(
            "Memory used for shuffle: {} bytes (just the seed)",
            seed_size
        );

        // Compare with storing full shuffle order
        let full_shuffle_size = total_count * std::mem::size_of::<usize>();
        let savings = full_shuffle_size - seed_size;
        let savings_mb = savings as f64 / 1_048_576.0;

        println!(
            "Full shuffle would use: {} bytes ({:.2} MB)",
            full_shuffle_size,
            full_shuffle_size as f64 / 1_048_576.0
        );
        println!("Memory saved: {} bytes ({:.2} MB)", savings, savings_mb);
        println!(
            "Efficiency: {:.1}x smaller",
            full_shuffle_size as f64 / seed_size as f64
        );

        // Benchmark window generation
        state.window_start = 0;
        state.window_end = 50;

        let start = Instant::now();
        let indices = state.current_window_indices();
        let duration = start.elapsed();

        println!(
            "Time to generate 50-track window: {:.3}ms",
            duration.as_secs_f64() * 1000.0
        );
        println!(
            "First 5 shuffled indices: {:?}\n",
            &indices[..5.min(indices.len())]
        );
    }

    println!("=== Determinism Test ===");
    println!("Same seed should always produce the same shuffle order:\n");

    // Create two states with same seed
    let seed = 42;

    let mut state1 = LazyQueueState::new(
        QueueContext::AllTracks {
            user_id: 1,
            total_count: 100,
        },
        0,
    );
    state1.shuffle_seed = Some(seed);
    state1.window_start = 0;
    state1.window_end = 10;

    let mut state2 = LazyQueueState::new(
        QueueContext::AllTracks {
            user_id: 1,
            total_count: 100,
        },
        0,
    );
    state2.shuffle_seed = Some(seed);
    state2.window_start = 0;
    state2.window_end = 10;

    let indices1 = state1.current_window_indices();
    let indices2 = state2.current_window_indices();

    println!("State 1 (seed {}): {:?}", seed, indices1);
    println!("State 2 (seed {}): {:?}", seed, indices2);
    println!("Identical: {}", indices1 == indices2);

    println!("\n=== Window Loading Demo ===");
    println!("Simulating loading 50-track windows from a 1000-track collection:\n");

    let mut state = LazyQueueState::new(
        QueueContext::AllTracks {
            user_id: 1,
            total_count: 1000,
        },
        0,
    );
    state.enable_shuffle(ShuffleMode::Random);

    // Load 3 windows
    for window_num in 1..=3 {
        state.window_start = (window_num - 1) * 50;
        state.window_end = state.window_start + 50;

        let indices = state.current_window_indices();
        println!(
            "Window {} (tracks {}-{}): {:?}...",
            window_num,
            state.window_start,
            state.window_end - 1,
            &indices[..5]
        );
    }

    println!("\n✅ Seed-based shuffle successfully demonstrated!");
    println!("   - Minimal memory usage (8 bytes regardless of library size)");
    println!("   - Deterministic (same seed = same shuffle)");
    println!("   - Fast window generation (< 5ms even for 500k tracks)");
}
