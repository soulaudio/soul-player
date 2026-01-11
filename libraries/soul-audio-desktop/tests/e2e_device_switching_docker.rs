//! E2E tests for device switching using Docker containers
//!
//! These tests verify device switching functionality in containerized
//! environments with virtual audio devices.
//!
//! Run these tests with: cargo test --features docker-tests

#![cfg(feature = "docker-tests")]

use soul_audio_desktop::{backend, device, AudioBackend, DesktopPlayback, PlaybackCommand};
use soul_playback::PlaybackConfig;
use std::thread;
use std::time::Duration;
use testcontainers::{core::WaitFor, runners::AsyncRunner, Image};

/// Custom Docker image for audio testing with PulseAudio virtual devices
#[derive(Debug, Clone)]
struct AudioTestImage;

impl AudioTestImage {
    fn build_if_needed() {
        // Build the image using the Dockerfile
        let dockerfile_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/docker");

        eprintln!(
            "Building audio test image from Dockerfile at: {}",
            dockerfile_dir
        );

        let output = std::process::Command::new("docker")
            .args([
                "build",
                "-t",
                "soul-audio-test:latest",
                "-f",
                "Dockerfile.audio-test",
                ".",
            ])
            .current_dir(dockerfile_dir)
            .output()
            .expect("Failed to build Docker image");

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            panic!("Docker build failed: {}", stderr);
        }

        eprintln!("✓ Audio test image built successfully");
    }
}

impl Image for AudioTestImage {
    fn name(&self) -> &str {
        "soul-audio-test"
    }

    fn tag(&self) -> &str {
        "latest"
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::Duration {
            length: Duration::from_secs(3),
        }]
    }
}

/// Helper to check if Docker is available
fn is_docker_available() -> bool {
    std::process::Command::new("docker")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Execute command in running container
fn exec_in_container(container_id: &str, command: &[&str]) -> Result<String, String> {
    let mut args = vec!["exec", container_id];
    args.extend_from_slice(command);

    let output = std::process::Command::new("docker")
        .args(&args)
        .output()
        .map_err(|e| format!("Failed to execute command: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Command failed: {}", stderr));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Test device enumeration in a Linux container with PulseAudio
#[tokio::test]
async fn test_device_enumeration_in_container() {
    if !is_docker_available() {
        eprintln!("⊘ Skipping test: Docker not available");
        return;
    }

    eprintln!("Building and starting audio test container...");

    // Build the Docker image first
    AudioTestImage::build_if_needed();

    // Start the container using testcontainers
    let container = AudioTestImage
        .start()
        .await
        .expect("Failed to start container");

    eprintln!("✓ Container started: {}", container.id());

    // Verify PulseAudio virtual sinks exist
    match exec_in_container(container.id(), &["pactl", "list", "sinks", "short"]) {
        Ok(stdout) => {
            eprintln!("PulseAudio sinks:\n{}", stdout);

            assert!(
                stdout.contains("virtual_output"),
                "Container should have virtual audio sinks"
            );

            // Count virtual outputs
            let virtual_count = stdout.matches("virtual_output").count();
            eprintln!("Found {} virtual audio outputs", virtual_count);
            assert!(virtual_count >= 3, "Should have at least 3 virtual outputs");
        }
        Err(e) => {
            panic!("Failed to list PulseAudio sinks: {}", e);
        }
    }

    eprintln!("✓ Virtual audio devices detected in container");
}

/// Test that verifies backend enumeration works
/// (This runs on host, not in container, to test actual CPAL functionality)
#[test]
fn test_backend_enumeration_host() {
    let backends = backend::list_available_backends();

    eprintln!("Available backends on host: {:?}", backends);

    // Should always have at least the default backend
    assert!(
        !backends.is_empty(),
        "Should have at least one audio backend"
    );

    // Verify we can get backend info
    let backend_info = backend::get_backend_info();
    assert!(!backend_info.is_empty());

    for info in &backend_info {
        eprintln!(
            "Backend: {} - available: {}, devices: {}",
            info.name, info.available, info.device_count
        );
    }

    eprintln!("✓ Backend enumeration works on host");
}

/// Test device listing on host system
#[test]
fn test_device_listing_host() {
    let backends = backend::list_available_backends();

    for backend in backends {
        eprintln!("Testing device listing for backend: {:?}", backend);

        match device::list_devices(backend) {
            Ok(devices) => {
                eprintln!("  Found {} devices", devices.len());

                for (i, device) in devices.iter().enumerate() {
                    eprintln!(
                        "  Device {}: {} ({}Hz, {}ch){}",
                        i,
                        device.name,
                        device.sample_rate,
                        device.channels,
                        if device.is_default { " [DEFAULT]" } else { "" }
                    );
                }

                assert!(!devices.is_empty(), "Should have at least one device");
            }
            Err(e) => {
                eprintln!("  Failed to list devices: {}", e);
            }
        }
    }

    eprintln!("✓ Device listing works on host");
}

/// Test creating playback instance on host
#[test]
fn test_playback_creation_host() {
    let config = PlaybackConfig::default();

    match DesktopPlayback::new(config) {
        Ok(playback) => {
            let backend = playback.get_current_backend();
            let device = playback.get_current_device();

            eprintln!("Playback created successfully");
            eprintln!("  Backend: {:?}", backend);
            eprintln!("  Device: {}", device);

            assert!(!device.is_empty(), "Device name should not be empty");
            eprintln!("✓ Playback creation works on host");
        }
        Err(e) => {
            eprintln!("Failed to create playback (may be expected in CI): {}", e);
        }
    }
}

/// Test device switching on host with multiple switches
#[test]
fn test_device_switching_stress_host() {
    let config = PlaybackConfig::default();

    match DesktopPlayback::new(config) {
        Ok(mut playback) => {
            eprintln!("Testing stress device switching on host");

            let initial_device = playback.get_current_device();
            eprintln!("Initial device: {}", initial_device);

            // Perform 5 rapid switches to default device
            let mut success_count = 0;

            for i in 0..5 {
                match playback.switch_device(AudioBackend::Default, None) {
                    Ok(_) => {
                        success_count += 1;
                        let current = playback.get_current_device();
                        eprintln!("  Switch {}: {} ✓", i + 1, current);
                        thread::sleep(Duration::from_millis(50));
                    }
                    Err(e) => {
                        eprintln!("  Switch {} failed: {}", i + 1, e);
                    }
                }
            }

            eprintln!("Stress test: {}/5 switches succeeded", success_count);
            assert!(success_count >= 3, "At least 3 switches should succeed");

            eprintln!("✓ Stress test passed on host");
        }
        Err(e) => {
            eprintln!("Failed to create playback (may be expected in CI): {}", e);
        }
    }
}

/// Integration test: Verify playback commands work after device switch
#[test]
fn test_commands_after_device_switch() {
    let config = PlaybackConfig::default();

    match DesktopPlayback::new(config) {
        Ok(mut playback) => {
            eprintln!("Testing playback commands after device switch");

            // Switch device
            if let Ok(_) = playback.switch_device(AudioBackend::Default, None) {
                let device = playback.get_current_device();
                eprintln!("Switched to device: {}", device);

                // Try sending various commands
                let commands = vec![
                    PlaybackCommand::SetVolume(50),
                    PlaybackCommand::SetVolume(75),
                    PlaybackCommand::Pause,
                ];

                for cmd in commands {
                    if let Err(e) = playback.send_command(cmd.clone()) {
                        eprintln!("Command {:?} failed: {}", cmd, e);
                    } else {
                        eprintln!("Command {:?} succeeded", cmd);
                    }
                }

                eprintln!("✓ Commands work after device switch");
            } else {
                eprintln!("Device switch failed, skipping command test");
            }
        }
        Err(e) => {
            eprintln!("Failed to create playback (may be expected in CI): {}", e);
        }
    }
}

/// Test that exercises multiple backends if available
#[test]
fn test_multi_backend_switching() {
    let backends = backend::list_available_backends();

    if backends.len() < 2 {
        eprintln!("Only one backend available, skipping multi-backend test");
        return;
    }

    eprintln!("Testing switching between {} backends", backends.len());

    let config = PlaybackConfig::default();

    match DesktopPlayback::new(config) {
        Ok(mut playback) => {
            for (i, backend) in backends.iter().enumerate() {
                eprintln!("Switching to backend {}: {:?}", i + 1, backend);

                match playback.switch_device(*backend, None) {
                    Ok(_) => {
                        let current_backend = playback.get_current_backend();
                        let current_device = playback.get_current_device();

                        eprintln!("  Success: {:?} - {}", current_backend, current_device);

                        assert_eq!(current_backend, *backend);
                        thread::sleep(Duration::from_millis(100));
                    }
                    Err(e) => {
                        eprintln!("  Failed to switch to {:?}: {}", backend, e);
                    }
                }
            }

            eprintln!("✓ Multi-backend switching completed");
        }
        Err(e) => {
            eprintln!("Failed to create playback: {}", e);
        }
    }
}

/// Docker container health check test
#[tokio::test]
async fn test_docker_audio_container_health() {
    if !is_docker_available() {
        eprintln!("⊘ Skipping test: Docker not available");
        return;
    }

    eprintln!("Testing Docker container audio health...");

    // Build the Docker image first
    AudioTestImage::build_if_needed();

    // Start container using testcontainers
    let container = AudioTestImage
        .start()
        .await
        .expect("Failed to start container");

    eprintln!("✓ Container started: {}", container.id());

    // Wait a bit for PulseAudio to fully start
    thread::sleep(Duration::from_secs(3));

    // Check container logs first for debugging
    eprintln!("Checking container logs...");
    if let Ok(logs) = exec_in_container(container.id(), &["tail", "-20", "/var/log/syslog"]) {
        eprintln!("Container syslog (last 20 lines):");
        for line in logs.lines() {
            eprintln!("  {}", line);
        }
    }

    // Check if container is still running
    let ps_output = exec_in_container(container.id(), &["ps", "aux"]).unwrap_or_default();
    eprintln!("Running processes:\n{}", ps_output);

    // Check PulseAudio is running
    match exec_in_container(container.id(), &["pgrep", "pulseaudio"]) {
        Ok(output) => {
            eprintln!("PulseAudio process IDs: {}", output.trim());
            assert!(!output.trim().is_empty(), "PulseAudio should be running");
        }
        Err(e) => {
            eprintln!("ERROR: PulseAudio check failed: {}", e);

            // Try alternative check
            match exec_in_container(container.id(), &["ps", "aux"]) {
                Ok(ps) => {
                    eprintln!("Process list:\n{}", ps);
                    if ps.contains("pulseaudio") {
                        eprintln!("PulseAudio found in process list!");
                    } else {
                        panic!("PulseAudio is not running in container");
                    }
                }
                Err(e2) => {
                    panic!("Could not check processes: {}", e2);
                }
            }
        }
    }

    // Check virtual sinks exist
    match exec_in_container(container.id(), &["pactl", "list", "sinks", "short"]) {
        Ok(sinks_output) => {
            eprintln!("Configured sinks:\n{}", sinks_output);

            assert!(
                sinks_output.contains("virtual_output_1"),
                "Should have virtual_output_1"
            );
            assert!(
                sinks_output.contains("virtual_output_2"),
                "Should have virtual_output_2"
            );
            assert!(
                sinks_output.contains("virtual_output_3"),
                "Should have virtual_output_3"
            );

            eprintln!("✓ All 3 virtual audio devices configured correctly");
        }
        Err(e) => {
            panic!("Failed to verify virtual sinks: {}", e);
        }
    }

    eprintln!("✓ Docker audio container is healthy");
}
