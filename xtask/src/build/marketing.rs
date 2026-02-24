//! Marketing site build automation
//!
//! Builds the Next.js marketing site, ensuring WASM is built first.

use anyhow::Result;

use crate::build;
use crate::util::{exec, output, platform};

/// Build marketing site
pub fn run(release: bool) -> Result<()> {
    output::print_header("Marketing Site Build");

    let workspace_root = platform::workspace_root()?;
    let marketing_dir = workspace_root.join("applications").join("marketing");

    // Verify directory exists
    if !marketing_dir.exists() {
        anyhow::bail!("Marketing directory not found: {}", marketing_dir.display());
    }

    // Check for yarn
    exec::require_command("yarn", "npm install -g yarn")?;

    // Step 1: Ensure WASM is built first
    output::print_step("Ensuring WASM is built...");
    build::wasm::run(false, release)?;

    // Step 2: Build marketing site
    output::print_step("Building marketing site...");
    output::print_info(&format!("Directory: {}", marketing_dir.display()));

    let args = ["workspace", "@soul-player/marketing", "run", "build"];

    let success = exec::run_command_in_dir(
        "yarn",
        &args,
        &workspace_root,
        "yarn workspace @soul-player/marketing build",
    )?;

    if success {
        output::print_success("Marketing site build complete!");
        Ok(())
    } else {
        anyhow::bail!("Marketing site build failed")
    }
}
