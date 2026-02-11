//! Mobile build automation
//!
//! Builds the Tauri mobile application for iOS and Android.

use anyhow::Result;

use crate::util::{exec, output, platform};

/// Build mobile application
pub fn run(_release: bool, platform_arg: Option<String>) -> Result<()> {
    output::print_header("Mobile Build");

    let workspace_root = platform::workspace_root()?;
    let mobile_dir = workspace_root.join("applications").join("mobile");

    // Verify directory exists
    if !mobile_dir.exists() {
        anyhow::bail!("Mobile directory not found: {}", mobile_dir.display());
    }

    // Check for yarn
    exec::require_command("yarn", "npm install -g yarn")?;

    // Build command
    output::print_step("Building mobile application...");
    output::print_info(&format!("Directory: {}", mobile_dir.display()));

    let mut base_args = vec!["workspace", "soul-player-mobile", "run", "tauri:build"];

    // Add platform argument if specified
    let platform_owned;
    if let Some(platform) = platform_arg {
        output::print_info(&format!("Platform: {}", platform));
        platform_owned = platform;
        base_args.push("--");
        base_args.push("--platform");
        base_args.push(&platform_owned);
    }

    let success = exec::run_command_in_dir(
        "yarn",
        &base_args,
        &workspace_root,
        "yarn workspace soul-player-mobile tauri:build",
    )?;

    if success {
        output::print_success("Mobile build complete!");
        Ok(())
    } else {
        anyhow::bail!("Mobile build failed")
    }
}
