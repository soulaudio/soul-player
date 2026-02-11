//! Environment file setup (.env from .env.example)

use anyhow::Result;
use std::path::Path;

use crate::util::{fs, output, platform};

pub fn run(force: bool) -> Result<()> {
    output::print_header("Environment Setup");

    let workspace = platform::workspace_root()?;
    let env_example = workspace.join(".env.example");
    let env_file = workspace.join(".env");

    // Check if .env.example exists
    if !env_example.exists() {
        anyhow::bail!(".env.example not found in workspace root");
    }

    // Copy .env.example to .env if missing (or if force=true)
    if !env_file.exists() {
        output::print_step("Creating .env from .env.example");
        fs::copy_file(&env_example, &env_file)?;
        output::print_success(".env created successfully");
    } else if force {
        output::print_step("Overwriting .env with .env.example (force mode)");
        fs::copy_file(&env_example, &env_file)?;
        output::print_success(".env overwritten successfully");
    } else {
        output::print_success(".env already exists");
    }

    // Validate required environment variables
    output::print_step("Validating environment variables");
    validate_env_file(&env_file)?;

    output::print_complete("Environment setup complete!");

    // Print next steps
    println!();
    output::print_info("Next steps:");
    println!("  1. cargo xtask setup sqlx   # Setup database");
    println!("  2. yarn install              # Install dependencies");
    println!("  3. cargo xtask dev desktop   # Run desktop app");

    Ok(())
}

fn validate_env_file(env_path: &Path) -> Result<()> {
    let content = fs::read_file(env_path)?;

    // Check for DATABASE_URL
    let has_database_url = content
        .lines()
        .any(|line| !line.trim().starts_with('#') && line.contains("DATABASE_URL="));

    if has_database_url {
        output::print_success("DATABASE_URL found in .env");
    } else {
        output::print_warning("DATABASE_URL not found in .env");
        println!("    Expected: DATABASE_URL=sqlite:libraries/soul-storage/.tmp/dev.db");
    }

    Ok(())
}
