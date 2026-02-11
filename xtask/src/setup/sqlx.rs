//! SQLx setup (database, migrations, offline mode)

use anyhow::{Context, Result};
use std::path::Path;

use crate::util::{exec, fs, output, platform};

pub fn run(skip_create: bool, skip_migrate: bool) -> Result<()> {
    output::print_header("SQLx Setup");

    let workspace = platform::workspace_root()?;
    let env_file = workspace.join(".env");

    // Check/create .env from .env.example if needed
    if !env_file.exists() {
        output::print_step("Creating .env from .env.example");
        crate::setup::env::run(false)?;
    } else {
        output::print_success(".env exists");
    }

    // Parse DATABASE_URL from .env
    let database_url = parse_database_url(&env_file)?;
    output::print_info(&format!("Using DATABASE_URL: {}", database_url));

    // Create .tmp/ directory
    let tmp_dir = workspace.join("libraries/soul-storage/.tmp");
    if !tmp_dir.exists() {
        output::print_step("Creating .tmp directory");
        std::fs::create_dir_all(&tmp_dir)
            .with_context(|| format!("Failed to create directory: {:?}", tmp_dir))?;
        output::print_success("Created .tmp directory");
    }

    // Check sqlx-cli installed
    output::print_step("Checking sqlx-cli installation");
    if !exec::command_exists("sqlx") {
        output::print_warning("sqlx-cli not found, installing...");
        let success = exec::run_command_inherit(
            "cargo",
            &[
                "install",
                "sqlx-cli",
                "--no-default-features",
                "--features",
                "sqlite",
                "--locked",
            ],
            "Installing sqlx-cli",
        )?;

        if !success {
            anyhow::bail!("Failed to install sqlx-cli");
        }
        output::print_success("sqlx-cli installed");
    } else {
        output::print_success("sqlx-cli is installed");
    }

    // Run sqlx database create
    if !skip_create {
        output::print_step("Creating database");
        let success =
            exec::run_command_inherit("sqlx", &["database", "create"], "Creating database")?;

        if success {
            output::print_success("Database ready");
        } else {
            output::print_warning("Database creation failed (may already exist)");
        }
    }

    // Run sqlx migrate run
    if !skip_migrate {
        output::print_step("Running migrations");
        let success = exec::run_command_inherit(
            "sqlx",
            &[
                "migrate",
                "run",
                "--source",
                "libraries/soul-storage/migrations",
            ],
            "Running migrations",
        )?;

        if success {
            output::print_success("Migrations applied");
        } else {
            anyhow::bail!("Failed to apply migrations");
        }
    }

    // Run cargo sqlx prepare (in libraries/soul-storage)
    output::print_step("Preparing SQLx offline mode");
    let storage_dir = workspace.join("libraries/soul-storage");

    // Get absolute database path for prepare command
    let abs_db_path = workspace.join("libraries/soul-storage/.tmp/dev.db");
    let abs_db_url = format!("sqlite://{}", abs_db_path.display());

    // Set DATABASE_URL environment variable for the command
    let success = std::process::Command::new("cargo")
        .args(["sqlx", "prepare", "--", "--lib"])
        .current_dir(&storage_dir)
        .env("DATABASE_URL", &abs_db_url)
        .status()
        .with_context(|| "Failed to run cargo sqlx prepare")?
        .success();

    if success {
        output::print_success("SQLx offline data prepared");
    } else {
        output::print_warning("SQLx offline mode preparation skipped (compilation errors)");
        output::print_info("This is optional - SQLx will work fine using the database");
    }

    // Verify with cargo check
    output::print_step("Verifying setup");
    let success = exec::run_command(
        "cargo",
        &["check", "-p", "soul-storage"],
        "Verifying soul-storage compiles",
    )?;

    if success {
        output::print_success("Verification successful");
    } else {
        anyhow::bail!("Verification failed - check errors above");
    }

    output::print_complete("SQLx setup complete!");

    println!();
    output::print_info(&format!("Database location: {}", database_url));
    println!();
    println!("You can now:");
    println!("  • Run cargo build - SQLx will verify queries at compile time");
    println!("  • Run cargo test - Tests will use testcontainers");
    println!("  • Use SQLX_OFFLINE=true for CI/offline builds");

    Ok(())
}

fn parse_database_url(env_path: &Path) -> Result<String> {
    let content = fs::read_file(env_path)?;

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        if line.starts_with("DATABASE_URL=") {
            return Ok(line
                .strip_prefix("DATABASE_URL=")
                .unwrap_or("")
                .trim()
                .to_string());
        }
    }

    anyhow::bail!("DATABASE_URL not found in .env file")
}
