//! Backend server development
//!
//! Runs the Soul Player backend server using Docker Compose.
//! Supports both foreground and detached modes.

use anyhow::Result;

use crate::util::exec::{command_exists, require_command, run_command_inherit};
use crate::util::output::{print_error, print_header, print_info};

/// Run backend server with Docker Compose
///
/// # Arguments
/// * `detached` - If true, run in detached mode (-d flag)
///
/// # Requirements
/// - Docker must be installed and running
/// - docker-compose (or docker compose) must be available
pub fn run(detached: bool) -> Result<()> {
    print_header("Backend Server (Docker Compose)");

    // Check for Docker
    require_command("docker", "https://docs.docker.com/get-docker/")?;
    print_info("Docker found");

    // Check for docker-compose (standalone) or docker compose (plugin)
    let use_compose_plugin = if command_exists("docker-compose") {
        print_info("Using standalone docker-compose");
        false
    } else {
        // Check if docker compose plugin is available
        let output = std::process::Command::new("docker")
            .args(["compose", "version"])
            .output();

        if output.is_ok() && output.unwrap().status.success() {
            print_info("Using docker compose plugin");
            true
        } else {
            print_error("Neither 'docker-compose' nor 'docker compose' found");
            anyhow::bail!(
                "Docker Compose not found. Install with: https://docs.docker.com/compose/install/"
            );
        }
    };

    // Build command args
    let mut args = vec!["up", "--build"];
    if detached {
        args.push("-d");
        print_info("Running in detached mode");
    } else {
        print_info("Running in foreground mode");
        print_info("Press Ctrl+C to stop");
    }

    // Run docker-compose or docker compose
    let success = if use_compose_plugin {
        let mut compose_args = vec!["compose"];
        compose_args.extend_from_slice(&args);
        run_command_inherit("docker", &compose_args, "Docker Compose")?
    } else {
        run_command_inherit("docker-compose", &args, "Docker Compose")?
    };

    if !success {
        anyhow::bail!("Docker Compose failed");
    }

    if detached {
        print_info("Server running in background");
        print_info("View logs: docker compose logs -f");
        print_info("Stop server: docker compose down");
    }

    Ok(())
}
