//! Stress test orchestration
//!
//! This module coordinates the execution of stress tests:
//! - Lock contention tests
//! - Endurance tests (5 min, 1 hour)
//! - Corrupted file recovery tests
//! - Performance benchmarks

use anyhow::{Context, Result};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Run lock contention stress tests
pub fn run_lock_contention_tests(verbose: bool) -> Result<()> {
    println!("\n=== Lock Contention Stress Tests ===");
    println!("Testing concurrent command flooding and lock contention...\n");

    let start = Instant::now();

    let mut args = vec![
        "test",
        "--package",
        "soul-audio-desktop",
        "--test",
        "lock_contention_stress_test",
        "--",
        "--include-ignored",
        "--nocapture",
    ];

    if verbose {
        args.push("--show-output");
    }

    let status = Command::new("cargo")
        .args(&args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .context("Failed to run lock contention tests")?;

    let duration = start.elapsed();

    println!("\n=== Results ===");
    println!("Duration: {:.2}s", duration.as_secs_f64());

    if !status.success() {
        anyhow::bail!("Lock contention tests failed");
    }

    println!("✓ All lock contention tests passed");
    Ok(())
}

/// Run endurance stress tests
pub fn run_endurance_tests(duration_override: Option<Duration>, verbose: bool) -> Result<()> {
    println!("\n=== Endurance Stress Tests ===");

    let test_name = if let Some(duration) = duration_override {
        let minutes = duration.as_secs() / 60;
        if minutes >= 60 {
            println!("Running {}-hour endurance test...\n", minutes / 60);
            "test_continuous_playback_1_hour"
        } else if minutes >= 5 {
            println!("Running {}-minute endurance test...\n", minutes);
            "test_continuous_playback_5_min"
        } else {
            println!("Running rapid cycling test...\n");
            "test_rapid_track_cycling"
        }
    } else {
        println!("Running 5-minute endurance test...\n");
        "test_continuous_playback_5_min"
    };

    let start = Instant::now();

    let mut args = vec![
        "test",
        "--package",
        "soul-audio-desktop",
        "--test",
        "endurance_stress_test",
        test_name,
        "--",
        "--include-ignored",
        "--nocapture",
    ];

    if verbose {
        args.push("--show-output");
    }

    let status = Command::new("cargo")
        .args(&args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .context("Failed to run endurance tests")?;

    let actual_duration = start.elapsed();

    println!("\n=== Results ===");
    println!(
        "Duration: {:.2}s ({:.1} min)",
        actual_duration.as_secs_f64(),
        actual_duration.as_secs_f64() / 60.0
    );

    if !status.success() {
        anyhow::bail!("Endurance tests failed");
    }

    println!("✓ Endurance tests passed");
    Ok(())
}

/// Run corrupted file recovery tests
pub fn run_corrupted_file_tests(verbose: bool) -> Result<()> {
    println!("\n=== Corrupted File Recovery Tests ===");
    println!("Testing error handling for invalid/corrupted audio files...\n");

    let start = Instant::now();

    let mut args = vec![
        "test",
        "--package",
        "soul-audio-desktop",
        "--test",
        "corrupted_file_recovery_test",
        "--",
        "--include-ignored",
        "--nocapture",
    ];

    if verbose {
        args.push("--show-output");
    }

    let status = Command::new("cargo")
        .args(&args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .context("Failed to run corrupted file tests")?;

    let duration = start.elapsed();

    println!("\n=== Results ===");
    println!("Duration: {:.2}s", duration.as_secs_f64());

    if !status.success() {
        anyhow::bail!("Corrupted file tests failed");
    }

    println!("✓ All corrupted file tests passed");
    Ok(())
}

/// Run performance benchmarks
pub fn run_benchmarks(verbose: bool) -> Result<()> {
    println!("\n=== Performance Benchmarks ===");
    println!("Running Criterion benchmarks...\n");

    let start = Instant::now();

    let mut args = vec![
        "bench",
        "--package",
        "soul-audio-desktop",
        "--bench",
        "playback_latency_benchmark",
    ];

    if verbose {
        args.push("--");
        args.push("--verbose");
    }

    let status = Command::new("cargo")
        .args(&args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .context("Failed to run benchmarks")?;

    let duration = start.elapsed();

    println!("\n=== Results ===");
    println!("Duration: {:.2}s", duration.as_secs_f64());

    if !status.success() {
        anyhow::bail!("Benchmarks failed");
    }

    println!("✓ Benchmarks completed");
    println!("\nResults saved to target/criterion/");
    Ok(())
}

/// Run all stress tests (quick suite - excludes 1-hour endurance test)
pub fn run_all_stress_tests(verbose: bool) -> Result<()> {
    println!("\n=== Running All Stress Tests (Quick Suite) ===\n");

    let overall_start = Instant::now();

    // 1. Lock contention tests (~5 seconds)
    run_lock_contention_tests(verbose)?;

    // 2. Short endurance test (5 minutes)
    run_endurance_tests(Some(Duration::from_secs(300)), verbose)?;

    // 3. Corrupted file recovery tests (~10 seconds)
    run_corrupted_file_tests(verbose)?;

    let overall_duration = overall_start.elapsed();

    println!("\n=== Overall Results ===");
    println!(
        "Total duration: {:.2}s ({:.1} min)",
        overall_duration.as_secs_f64(),
        overall_duration.as_secs_f64() / 60.0
    );
    println!("✓ All stress tests passed");

    Ok(())
}
