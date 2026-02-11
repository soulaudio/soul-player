//! Audio E2E test orchestration
//!
//! This module coordinates the execution of audio E2E tests, including:
//! - Virtual device checking
//! - Test asset generation
//! - Test execution with proper timeouts
//! - Metrics collection and export

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Test execution metrics
#[derive(Debug, Serialize, Deserialize)]
pub struct TestMetrics {
    pub total_duration_secs: f64,
    pub tests_run: usize,
    pub tests_passed: usize,
    pub tests_failed: usize,
    pub virtual_device_available: bool,
    pub test_assets_present: bool,
    pub timestamp: String,
}

/// Run complete audio E2E test suite
pub fn run_e2e_tests(
    ci: bool,
    skip_device_check: bool,
    init_only: bool,
    stutter_only: bool,
    export_metrics: Option<String>,
) -> Result<()> {
    let start_time = Instant::now();

    println!("=== Soul Player Audio E2E Tests ===");
    println!();

    // Step 1: Check for virtual audio device
    let has_virtual_device = if skip_device_check {
        println!("⚠️  Skipping virtual device check");
        false
    } else {
        println!("Step 1: Checking for virtual audio device...");
        match check_virtual_device() {
            Ok(device_name) => {
                println!("  ✓ Virtual device found: {}", device_name);
                true
            }
            Err(e) => {
                println!("  ✗ {}", e);
                print_virtual_device_setup();
                if !ci {
                    bail!("Virtual audio device required for E2E tests");
                }
                println!("  ⚠️ Continuing in CI mode...");
                false
            }
        }
    };

    // Step 2: Check/generate test assets
    println!();
    println!("Step 2: Checking test assets...");
    let assets_present = check_test_assets()?;
    if !assets_present {
        println!("  → Generating test assets...");
        generate_assets("tests/assets", false)?;
        println!("  ✓ Test assets generated");
    } else {
        println!("  ✓ Test assets present");
    }

    // Step 3: Run tests
    println!();
    println!("Step 3: Running audio E2E tests...");

    let mut test_args = vec!["test", "--package", "soul-audio-desktop"];

    if init_only {
        test_args.push("--test");
        test_args.push("pause_during_startup_e2e_test");
    } else if stutter_only {
        test_args.push("--test");
        test_args.push("device_hotplug_e2e");
    } else {
        test_args.push("--test");
        test_args.push("pause_during_startup_e2e_test");
        test_args.push("--test");
        test_args.push("device_hotplug_e2e");
    }

    test_args.extend_from_slice(&["--", "--nocapture"]);

    if !ci {
        test_args.push("--show-output");
    }

    let test_result = Command::new("cargo")
        .args(&test_args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .context("Failed to run cargo test")?;

    // Step 4: Report results
    println!();
    let elapsed = start_time.elapsed();

    let metrics = TestMetrics {
        total_duration_secs: elapsed.as_secs_f64(),
        tests_run: if init_only || stutter_only { 1 } else { 2 },
        tests_passed: if test_result.success() {
            if init_only || stutter_only {
                1
            } else {
                2
            }
        } else {
            0
        },
        tests_failed: if test_result.success() { 0 } else { 1 },
        virtual_device_available: has_virtual_device,
        test_assets_present: assets_present,
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    print_test_summary(&metrics);

    // Export metrics if requested
    if let Some(export_path) = export_metrics {
        export_test_metrics(&metrics, &export_path)?;
        println!();
        println!("  ✓ Metrics exported to {}", export_path);
    }

    if !test_result.success() {
        bail!("Audio E2E tests failed");
    }

    Ok(())
}

/// Run CI-friendly E2E tests with timeouts
pub fn run_ci_tests(timeout_secs: u64, export_metrics: Option<String>) -> Result<()> {
    println!("=== Soul Player Audio E2E Tests (CI Mode) ===");
    println!("  Timeout: {} seconds", timeout_secs);
    println!();

    let start_time = Instant::now();

    // Quick asset check/generation
    println!("Preparing test environment...");
    let assets_present = check_test_assets()?;
    if !assets_present {
        generate_assets("tests/assets", false)?;
        println!("  ✓ Test assets generated");
    }

    // Run tests with timeout
    println!();
    println!("Running tests with timeout...");

    let test_process = Command::new("cargo")
        .args(&[
            "test",
            "--package",
            "soul-audio-desktop",
            "--test",
            "pause_during_startup_e2e_test",
            "--",
            "--nocapture",
            "--test-threads=1",
        ])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .context("Failed to spawn cargo test")?;

    // Wait with timeout
    let timeout = Duration::from_secs(timeout_secs);
    let test_result = wait_with_timeout(test_process, timeout)?;

    let elapsed = start_time.elapsed();

    let metrics = TestMetrics {
        total_duration_secs: elapsed.as_secs_f64(),
        tests_run: 1,
        tests_passed: if test_result { 1 } else { 0 },
        tests_failed: if test_result { 0 } else { 1 },
        virtual_device_available: false,
        test_assets_present: assets_present,
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    println!();
    print_test_summary(&metrics);

    if let Some(path) = export_metrics {
        export_test_metrics(&metrics, &path)?;
        println!("  ✓ Metrics exported to {}", path);
    }

    if !test_result {
        bail!("CI tests failed");
    }

    Ok(())
}

/// List available audio devices
pub fn list_devices(verbose: bool, filter: Option<String>) -> Result<()> {
    super::devices::list_devices(verbose, filter.as_deref())
}

/// Generate test audio assets
pub fn generate_assets(_output: &str, _force: bool) -> Result<()> {
    println!("Generating test audio assets...");

    // Use the generate-test-audio script
    let script_path = if cfg!(windows) {
        "scripts/generate-test-audio.ps1"
    } else {
        "scripts/generate-test-audio.sh"
    };

    let status = if cfg!(windows) {
        Command::new("powershell")
            .arg("-ExecutionPolicy")
            .arg("Bypass")
            .arg("-File")
            .arg(script_path)
            .status()
    } else {
        Command::new("bash").arg(script_path).status()
    };

    match status {
        Ok(s) if s.success() => {
            println!("✓ Test assets generated successfully");
            Ok(())
        }
        Ok(s) => {
            bail!("Asset generation failed with status: {}", s)
        }
        Err(e) => {
            bail!("Failed to run asset generation script: {}", e)
        }
    }
}

/// Check for virtual audio device
fn check_virtual_device() -> Result<String> {
    // Run device listing test to check for virtual devices
    let output = Command::new("cargo")
        .args(&[
            "test",
            "--package",
            "soul-audio-desktop",
            "--test",
            "device_handling_test",
            "test_real_enumerate_devices",
            "--",
            "--nocapture",
            "--show-output",
        ])
        .output()
        .context("Failed to run device enumeration")?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Look for common virtual device names
    let virtual_devices = [
        "BlackHole",
        "VB-Cable",
        "CABLE Input",
        "Virtual Audio Cable",
        "Loopback",
        "snd-aloop",
    ];

    for device in &virtual_devices {
        if stdout.contains(device) {
            return Ok(device.to_string());
        }
    }

    bail!("No virtual audio device found")
}

/// Check if test assets exist
fn check_test_assets() -> Result<bool> {
    let asset_dir = std::path::Path::new("tests/assets");

    if !asset_dir.exists() {
        return Ok(false);
    }

    // Check for key test files
    let required_files = ["1khz-sine-10s.wav", "1khz-sine-30s.wav", "silence-1s.wav"];

    for file in &required_files {
        if !asset_dir.join(file).exists() {
            return Ok(false);
        }
    }

    Ok(true)
}

/// Print virtual device setup instructions
fn print_virtual_device_setup() {
    println!();
    println!("Virtual Audio Device Setup:");
    println!();
    println!("  macOS:");
    println!("    brew install blackhole-2ch");
    println!();
    println!("  Linux:");
    println!("    sudo modprobe snd-aloop");
    println!();
    println!("  Windows:");
    println!("    choco install vb-cable");
    println!("    # Or download from: https://vb-audio.com/Cable/");
    println!();
}

/// Print test execution summary
fn print_test_summary(metrics: &TestMetrics) {
    println!("=== Test Summary ===");
    println!("  Duration: {:.2}s", metrics.total_duration_secs);
    println!("  Tests run: {}", metrics.tests_run);
    println!("  Passed: {}", metrics.tests_passed);

    if metrics.tests_failed > 0 {
        println!("  Failed: {}", metrics.tests_failed);
    }

    println!();
    println!("Environment:");
    println!(
        "  Virtual device: {}",
        if metrics.virtual_device_available {
            "Yes"
        } else {
            "No"
        }
    );
    println!(
        "  Test assets: {}",
        if metrics.test_assets_present {
            "Yes"
        } else {
            "No"
        }
    );
}

/// Export metrics to JSON file
fn export_test_metrics(metrics: &TestMetrics, path: &str) -> Result<()> {
    let json = serde_json::to_string_pretty(metrics).context("Failed to serialize metrics")?;

    std::fs::write(path, json).context("Failed to write metrics file")?;

    Ok(())
}

/// Wait for process with timeout
fn wait_with_timeout(mut process: std::process::Child, timeout: Duration) -> Result<bool> {
    let start = Instant::now();

    loop {
        match process.try_wait()? {
            Some(status) => return Ok(status.success()),
            None => {
                if start.elapsed() > timeout {
                    process.kill()?;
                    bail!("Test execution timed out after {:?}", timeout);
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        }
    }
}
