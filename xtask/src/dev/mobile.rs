//! Mobile development server
//!
//! Runs the Tauri mobile application in development mode.
//! Supports iOS and Android platforms.

use anyhow::Result;

use crate::util::exec::{require_command, run_command_inherit};
use crate::util::output::{print_header, print_info};

/// Run mobile dev server
///
/// # Arguments
/// * `platform` - Target platform (ios, android). If None, uses default Tauri behavior.
pub fn run(platform: Option<String>) -> Result<()> {
    print_header("Mobile Dev Server");

    // Check for yarn
    require_command("yarn", "npm install -g yarn")?;

    // Build command args
    let mut args = vec!["workspace", "soul-player-mobile", "tauri:dev"];

    // Add platform flag if specified
    let platform_arg;
    if let Some(p) = platform.as_ref() {
        // Validate platform
        if p != "ios" && p != "android" {
            anyhow::bail!("Invalid platform: {}. Must be 'ios' or 'android'", p);
        }
        platform_arg = format!("--platform={}", p);
        args.push(&platform_arg);
        print_info(&format!("Target platform: {}", p));
    }

    print_info("Starting mobile dev server...");
    print_info("Press Ctrl+C to stop");

    let success = run_command_inherit("yarn", &args, "Mobile dev server")?;

    if !success {
        anyhow::bail!("Mobile dev server failed");
    }

    Ok(())
}
