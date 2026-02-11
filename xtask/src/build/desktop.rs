//! Desktop build automation
//!
//! Builds the Tauri desktop application and provides "build all" command.

use anyhow::Result;

use crate::build;
use crate::util::{exec, output, platform};

/// Build desktop application
pub fn run(release: bool) -> Result<()> {
    output::print_header("Desktop Build");

    let workspace_root = platform::workspace_root()?;
    let desktop_dir = workspace_root.join("applications").join("desktop");

    // Verify directory exists
    if !desktop_dir.exists() {
        anyhow::bail!("Desktop directory not found: {}", desktop_dir.display());
    }

    // Check for yarn
    exec::require_command("yarn", "npm install -g yarn")?;

    // Build command
    output::print_step("Building desktop application...");
    output::print_info(&format!("Directory: {}", desktop_dir.display()));

    let build_cmd = if release { "tauri:build" } else { "build" };
    let args = vec!["workspace", "soul-player-desktop", "run", build_cmd];

    let success = exec::run_command_in_dir(
        "yarn",
        &args.iter().map(|s| *s).collect::<Vec<_>>(),
        &workspace_root,
        &format!("yarn workspace soul-player-desktop {}", build_cmd),
    )?;

    if success {
        output::print_success("Desktop build complete!");
        Ok(())
    } else {
        anyhow::bail!("Desktop build failed")
    }
}

/// Build all targets (WASM, desktop, marketing)
pub fn run_all(release: bool) -> Result<()> {
    output::print_header("Build All Targets");

    // Step 1: Build WASM
    output::print_header("Step 1/3: Building WASM");
    build::wasm::run(false, release)?;

    // Step 2: Build desktop
    output::print_header("Step 2/3: Building Desktop");
    run(release)?;

    // Step 3: Build marketing
    output::print_header("Step 3/3: Building Marketing");
    build::marketing::run(release)?;

    output::print_complete("All builds complete!");

    Ok(())
}
