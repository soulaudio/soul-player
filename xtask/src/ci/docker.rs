//! CI Docker build
//!
//! Builds Docker images and reports build size.
//! Primarily used for testing CI environment locally.

use anyhow::Result;

use crate::util::{exec, output, platform};

/// Build Docker image
pub fn run(show_size: bool) -> Result<()> {
    output::print_header("Docker Build");

    let workspace_root = platform::workspace_root()?;
    let dockerfile = workspace_root.join("Dockerfile.ci");

    // Verify Dockerfile exists
    if !dockerfile.exists() {
        anyhow::bail!("Dockerfile.ci not found: {}", dockerfile.display());
    }

    // Check for docker
    exec::require_command(
        "docker",
        "Install Docker from https://docs.docker.com/get-docker/",
    )?;

    // Build Docker image
    output::print_step("Building Docker image...");
    output::print_info(&format!("Dockerfile: {}", dockerfile.display()));
    output::print_info("Image: soul-player:ci-test");

    let args = vec![
        "build",
        "-t",
        "soul-player:ci-test",
        "-f",
        "Dockerfile.ci",
        ".",
    ];

    let success = exec::run_command_in_dir(
        "docker",
        &args,
        &workspace_root,
        "docker build -t soul-player:ci-test",
    )?;

    if !success {
        anyhow::bail!("Docker build failed");
    }

    output::print_success("Docker image built successfully!");

    // Show image size if requested
    if show_size {
        output::print_step("Checking image size...");

        let size_output = exec::run_command_capture(
            "docker",
            &["images", "soul-player:ci-test", "--format", "{{.Size}}"],
        )?;

        output::print_info(&format!("Image size: {}", size_output));
    }

    output::print_complete("Docker build complete!");
    output::print_info("Run with: docker run --rm -v \"$(pwd):/workspace\" soul-player:ci-test");

    Ok(())
}
