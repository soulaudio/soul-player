//! Cache invalidation E2E test automation
//!
//! Runs cache invalidation tests for React Query and component caches.

use anyhow::{Context, Result};
use std::process::Command;

pub fn run_e2e_tests(ci: bool, cache_type: Option<String>) -> Result<()> {
    println!("💾 Running cache invalidation E2E tests...");

    // For now, run the TypeScript tests via yarn
    let mut yarn_args = vec!["workspace", "@soul-player/shared", "test"];

    if !ci {
        yarn_args.push("--watch=false");
    }

    if let Some(ref cache_filter) = cache_type {
        yarn_args.push("--testNamePattern");
        yarn_args.push(cache_filter);
    }

    let status = Command::new("yarn")
        .args(&yarn_args)
        .status()
        .context("Failed to run cache E2E tests")?;

    if !status.success() {
        anyhow::bail!("Cache E2E tests failed");
    }

    println!("✅ Cache E2E tests passed");
    Ok(())
}

pub fn run_integration_tests(filter: Option<String>) -> Result<()> {
    println!("🔗 Running cache integration tests...");

    let mut yarn_args = vec![
        "workspace",
        "@soul-player/shared",
        "test",
        "--testPathPattern",
        "cache",
    ];

    if let Some(ref filter_str) = filter {
        yarn_args.push("--testNamePattern");
        yarn_args.push(filter_str);
    }

    let status = Command::new("yarn")
        .args(&yarn_args)
        .status()
        .context("Failed to run cache integration tests")?;

    if !status.success() {
        anyhow::bail!("Cache integration tests failed");
    }

    println!("✅ Cache integration tests passed");
    Ok(())
}
