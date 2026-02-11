//! Import/Re-import E2E test automation
//!
//! Runs import, re-import, and duplicate detection tests.

use anyhow::{Context, Result};
use std::process::Command;

pub fn run_e2e_tests(ci: bool, filter: Option<String>, threads: usize) -> Result<()> {
    println!("📦 Running import E2E tests...");

    let threads_arg = format!("--test-threads={}", threads);
    let mut cargo_args = vec![
        "test",
        "--package",
        "soul-importer",
        "--test",
        "e2e_reimport_tests",
        "--",
        &threads_arg,
    ];

    if !ci {
        cargo_args.push("--nocapture");
    }

    if let Some(ref filter_str) = filter {
        cargo_args.push(filter_str);
    }

    let status = Command::new("cargo")
        .args(&cargo_args)
        .status()
        .context("Failed to run import E2E tests")?;

    if !status.success() {
        anyhow::bail!("Import E2E tests failed");
    }

    println!("✅ Import E2E tests passed");
    Ok(())
}

pub fn run_unit_tests(filter: Option<String>) -> Result<()> {
    println!("🧪 Running import unit tests...");

    let mut cargo_args = vec!["test", "--package", "soul-importer", "--lib"];

    if let Some(ref filter_str) = filter {
        cargo_args.push(filter_str);
    }

    let status = Command::new("cargo")
        .args(&cargo_args)
        .status()
        .context("Failed to run import unit tests")?;

    if !status.success() {
        anyhow::bail!("Import unit tests failed");
    }

    println!("✅ Import unit tests passed");
    Ok(())
}
