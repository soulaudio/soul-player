//! Audio device enumeration for E2E testing

use anyhow::{Context, Result};
use std::process::Command;

pub fn list() -> Result<()> {
    list_devices(false, None)
}

pub fn list_devices(verbose: bool, _filter: Option<&str>) -> Result<()> {
    println!("=== Available Audio Devices ===\n");

    // Run the actual E2E test that lists devices
    let args = vec![
        "test",
        "--package",
        "soul-player-e2e-tests",
        "--test",
        "audio_initialization_latency",
        "list_available_audio_devices",
        "--",
        "--nocapture",
        "--show-output",
    ];

    let output = Command::new("cargo")
        .args(&args)
        .output()
        .context("Failed to run device enumeration test")?;

    // Print the output
    println!("{}", String::from_utf8_lossy(&output.stdout));

    if verbose {
        println!("\n{}", String::from_utf8_lossy(&output.stderr));
    }

    if !output.status.success() {
        println!("\nNote: If no devices are shown, make sure:");
        println!("  1. Virtual audio device is installed");
        println!("  2. cpal dependencies are properly installed");
    }

    Ok(())
}
