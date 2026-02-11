//! Web app development server
//!
//! Runs the web application in development mode with hot-reload.

use anyhow::Result;

use crate::util::exec::{require_command, run_command_inherit};
use crate::util::output::{print_header, print_info};

/// Run web app dev server
///
/// This command starts the Vite dev server for the web application
/// with hot module replacement enabled.
pub fn run() -> Result<()> {
    print_header("Web App Dev Server");

    // Check for yarn
    require_command("yarn", "npm install -g yarn")?;

    print_info("Starting web app dev server with hot-reload...");
    print_info("Press Ctrl+C to stop");

    let success = run_command_inherit(
        "yarn",
        &["workspace", "@soul-player/web", "dev"],
        "Web dev server",
    )?;

    if !success {
        anyhow::bail!("Web dev server failed");
    }

    Ok(())
}
