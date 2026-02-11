use anyhow::{Context, Result};
use colored::Colorize;
use std::process::{Command, Stdio};

/// Execute a command and return success/failure
pub fn run_command(program: &str, args: &[&str], description: &str) -> Result<bool> {
    println!("  {} {}", "Running:".cyan(), description);

    let status = Command::new(program)
        .args(args)
        .status()
        .with_context(|| format!("Failed to execute: {} {}", program, args.join(" ")))?;

    Ok(status.success())
}

/// Execute a command with inherited stdout/stderr
pub fn run_command_inherit(program: &str, args: &[&str], description: &str) -> Result<bool> {
    println!("  {} {}", "Running:".cyan(), description);

    // Try to find the full path to the program
    let program_path = which::which(program)
        .with_context(|| {
            format!(
                "{} not found in PATH.\n  \
                 Try running the command directly: {} {}",
                program,
                program,
                args.join(" ")
            )
        })?;

    let status = Command::new(program_path)
        .args(args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .with_context(|| format!("Failed to execute: {} {}", program, args.join(" ")))?;

    Ok(status.success())
}

/// Execute a command and capture output
pub fn run_command_capture(program: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .with_context(|| format!("Failed to execute: {} {}", program, args.join(" ")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Command failed: {}", stderr);
    }

    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

/// Execute a command in a specific directory
pub fn run_command_in_dir(
    program: &str,
    args: &[&str],
    dir: &std::path::Path,
    description: &str,
) -> Result<bool> {
    println!("  {} {} (in {:?})", "Running:".cyan(), description, dir);

    // Try to find the full path to the program
    let program_path = which::which(program)
        .with_context(|| {
            format!(
                "{} not found in PATH.\n  \
                 Try running the command directly: {} {}",
                program,
                program,
                args.join(" ")
            )
        })?;

    let status = Command::new(program_path)
        .args(args)
        .current_dir(dir)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .with_context(|| {
            format!(
                "Failed to execute: {} {} in {:?}",
                program,
                args.join(" "),
                dir
            )
        })?;

    Ok(status.success())
}

/// Check if a command exists in PATH
pub fn command_exists(command: &str) -> bool {
    which::which(command).is_ok()
}

/// Ensure a command exists, or return error with installation hint
pub fn require_command(command: &str, install_hint: &str) -> Result<()> {
    if !command_exists(command) {
        anyhow::bail!("{} not found. Install with: {}", command, install_hint);
    }
    Ok(())
}
