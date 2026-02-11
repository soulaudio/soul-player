//! Desktop development server
//!
//! Runs the Tauri desktop application in development mode with hot-reload.
//! Uses `yarn workspace soul-player-desktop tauri:dev` to start the dev server.

use anyhow::Result;

use crate::util::exec::{require_command, run_command_inherit};
use crate::util::output::{print_header, print_info};

/// Run desktop dev server
///
/// # Arguments
/// * `logs` - If true, show logs only (Tauri doesn't support this flag, so we warn the user)
pub fn run(logs: bool) -> Result<()> {
    print_header("Desktop Dev Server");

    // Check for yarn
    require_command("yarn", "npm install -g yarn")?;

    if logs {
        print_info("Note: Tauri dev mode always shows logs. The --logs flag has no effect.");
    }

    print_info("Starting desktop dev server with hot-reload...");
    print_info("Press Ctrl+C to stop");

    // Change to workspace root and run yarn command
    let success = run_command_inherit(
        "yarn",
        &["workspace", "soul-player-desktop", "tauri:dev"],
        "Desktop dev server",
    )?;

    if !success {
        anyhow::bail!("Desktop dev server failed");
    }

    Ok(())
}
