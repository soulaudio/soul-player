//! Marketing site development server
//!
//! Runs the marketing website in development mode with hot-reload.
//! Automatically builds WASM modules via npm hooks.

use anyhow::Result;

use crate::util::exec::{require_command, run_command_inherit};
use crate::util::output::{print_header, print_info};

/// Run marketing site dev server
///
/// This command:
/// - Starts the Vite dev server for the marketing site
/// - Auto-builds WASM modules via npm hooks
/// - Enables hot module replacement
pub fn run() -> Result<()> {
    print_header("Marketing Site Dev Server");

    // Check for yarn
    require_command("yarn", "npm install -g yarn")?;

    print_info("Starting marketing site dev server with hot-reload...");
    print_info("WASM modules will be built automatically via npm hooks");
    print_info("Press Ctrl+C to stop");

    let success = run_command_inherit(
        "yarn",
        &["workspace", "@soul-player/marketing", "dev"],
        "Marketing dev server",
    )?;

    if !success {
        anyhow::bail!("Marketing dev server failed");
    }

    Ok(())
}
