//! Git hooks setup

use anyhow::{Context, Result};

use crate::util::{fs, output, platform};

pub fn run() -> Result<()> {
    output::print_header("Git Hooks Setup");

    let workspace = platform::workspace_root()?;
    let husky_dir = workspace.join(".husky");
    let pre_commit_hook = husky_dir.join("pre-commit");

    // Create .husky directory if it doesn't exist
    if !husky_dir.exists() {
        output::print_step("Creating .husky directory");
        std::fs::create_dir_all(&husky_dir)
            .with_context(|| format!("Failed to create directory: {:?}", husky_dir))?;
    }

    // Write pre-commit hook that calls cargo xtask check precommit
    output::print_step("Writing pre-commit hook");

    let hook_content = if cfg!(target_os = "windows") {
        // Windows version (Git Bash)
        r#"#!/bin/sh
. "$(dirname "$0")/_/husky.sh"

# Run pre-commit checks via xtask
cargo xtask check precommit
"#
    } else {
        // Unix version
        r#"#!/bin/sh
. "$(dirname "$0")/_/husky.sh"

# Run pre-commit checks via xtask
cargo xtask check precommit
"#
    };

    fs::write_file(&pre_commit_hook, hook_content)?;

    // Make it executable (Unix only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&pre_commit_hook)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&pre_commit_hook, perms)?;
        output::print_success("Made pre-commit hook executable");
    }

    #[cfg(not(unix))]
    {
        output::print_success("Pre-commit hook created (Git Bash will handle permissions)");
    }

    output::print_complete("Git hooks setup complete!");

    println!();
    output::print_info("Pre-commit hook installed:");
    println!("  → Runs: cargo xtask check precommit");
    println!("  → To bypass: git commit --no-verify");

    Ok(())
}
