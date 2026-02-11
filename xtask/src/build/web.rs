//! Web app build automation
//!
//! Builds the web application.

use anyhow::Result;

use crate::util::{exec, output, platform};

/// Build web application
pub fn run(_release: bool) -> Result<()> {
    output::print_header("Web App Build");

    let workspace_root = platform::workspace_root()?;
    let web_dir = workspace_root.join("applications").join("web");

    // Verify directory exists
    if !web_dir.exists() {
        anyhow::bail!("Web directory not found: {}", web_dir.display());
    }

    // Check for yarn
    exec::require_command("yarn", "npm install -g yarn")?;

    // Build command
    output::print_step("Building web application...");
    output::print_info(&format!("Directory: {}", web_dir.display()));

    let args = vec!["workspace", "@soul-player/web", "run", "build"];

    let success = exec::run_command_in_dir(
        "yarn",
        &args.iter().map(|s| *s).collect::<Vec<_>>(),
        &workspace_root,
        "yarn workspace @soul-player/web build",
    )?;

    if success {
        output::print_success("Web app build complete!");
        Ok(())
    } else {
        anyhow::bail!("Web app build failed")
    }
}
