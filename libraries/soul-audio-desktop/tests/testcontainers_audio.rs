//! Testcontainers module for audio testing with PulseAudio virtual devices.
//!
//! This module provides an `AudioTestContainer` that creates an isolated Linux
//! environment with PulseAudio and virtual audio devices for comprehensive
//! audio testing without requiring physical hardware.
//!
//! # Features
//!
//! - Multiple virtual audio sinks (outputs) at different sample rates
//! - Virtual audio sources (inputs) for recording
//! - Methods to play audio, record, and detect glitches
//! - Underrun detection for performance testing
//!
//! # Usage
//!
//! ```rust,no_run
//! use testcontainers_audio::AudioTestContainer;
//!
//! #[tokio::test]
//! async fn test_audio_playback() {
//!     let container = AudioTestContainer::start().await.unwrap();
//!
//!     // List available virtual devices
//!     let devices = container.list_devices().await.unwrap();
//!     assert!(devices.len() >= 4);
//!
//!     // Play a test tone
//!     container.play_test_tone("virtual_output_1", 440, 1.0).await.unwrap();
//!
//!     // Check for underruns
//!     let underruns = container.detect_underruns("virtual_output_1").await.unwrap();
//!     assert_eq!(underruns, 0);
//! }
//! ```
//!
//! # Requirements
//!
//! - Docker must be installed and running
//! - Tests must be run with `--features testcontainers`
//!
//! Run tests with:
//! ```bash
//! cargo test --features testcontainers --test testcontainers_audio_test
//! ```

#![cfg(feature = "testcontainers")]

use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;
use testcontainers::{
    core::{ContainerPort, WaitFor},
    runners::AsyncRunner,
    ContainerAsync, Image,
};

/// Represents a virtual audio device in the container.
#[derive(Debug, Clone)]
pub struct VirtualAudioDevice {
    /// Device name (e.g., "virtual_output_1")
    pub name: String,
    /// Device description
    pub description: String,
    /// Sample rate in Hz
    pub sample_rate: u32,
    /// Number of channels
    pub channels: u8,
    /// Whether this is the default device
    pub is_default: bool,
    /// Device type
    pub device_type: AudioDeviceType,
}

/// Type of audio device.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioDeviceType {
    /// Output device (sink)
    Sink,
    /// Input device (source)
    Source,
}

/// Audio recording result.
#[derive(Debug)]
pub struct AudioRecording {
    /// Path to the recorded WAV file in the container
    pub path: String,
    /// Duration of the recording in seconds
    pub duration_secs: f32,
    /// Sample rate
    pub sample_rate: u32,
    /// Number of channels
    pub channels: u8,
}

/// Glitch detection result.
#[derive(Debug, Default)]
pub struct GlitchReport {
    /// Number of detected underruns
    pub underrun_count: u32,
    /// Number of clipping events
    pub clipping_count: u32,
    /// Number of silence gaps detected
    pub silence_gaps: u32,
    /// Peak level in dB
    pub peak_level_db: f32,
    /// Whether any glitches were detected
    pub has_glitches: bool,
}

/// Docker image for audio testing.
#[derive(Debug, Clone)]
pub struct AudioTestImage {
    /// Path to the Dockerfile directory
    dockerfile_path: PathBuf,
    /// Whether to rebuild the image
    force_rebuild: bool,
}

impl Default for AudioTestImage {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioTestImage {
    /// Create a new audio test image configuration.
    pub fn new() -> Self {
        // Use the docker/audio-test directory
        let dockerfile_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("docker")
            .join("audio-test");

        Self {
            dockerfile_path,
            force_rebuild: false,
        }
    }

    /// Force rebuild the Docker image.
    pub fn with_force_rebuild(mut self) -> Self {
        self.force_rebuild = true;
        self
    }

    /// Build the Docker image if needed.
    pub fn build_if_needed(&self) -> Result<(), String> {
        // Check if image exists
        if !self.force_rebuild {
            let output = Command::new("docker")
                .args(["images", "-q", "soul-audio-test:latest"])
                .output()
                .map_err(|e| format!("Failed to check for image: {}", e))?;

            if output.status.success() && !output.stdout.is_empty() {
                eprintln!("Audio test image already exists, skipping build");
                return Ok(());
            }
        }

        eprintln!(
            "Building audio test image from: {}",
            self.dockerfile_path.display()
        );

        let output = Command::new("docker")
            .args([
                "build",
                "-t",
                "soul-audio-test:latest",
                "-f",
                "Dockerfile",
                ".",
            ])
            .current_dir(&self.dockerfile_path)
            .output()
            .map_err(|e| format!("Failed to run docker build: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Docker build failed: {}", stderr));
        }

        eprintln!("Audio test image built successfully");
        Ok(())
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
        vec![
            // Wait for the startup message
            WaitFor::message_on_stdout("Audio Container Ready"),
            // Or timeout after 30 seconds
            WaitFor::Duration {
                length: Duration::from_secs(30),
            },
        ]
    }

    fn expose_ports(&self) -> &[ContainerPort] {
        // No ports exposed - we use docker exec for communication
        &[]
    }
}

/// Audio test container providing virtual audio devices.
pub struct AudioTestContainer {
    container: ContainerAsync<AudioTestImage>,
}

impl AudioTestContainer {
    /// Start a new audio test container.
    ///
    /// This will build the Docker image if needed and start the container
    /// with PulseAudio and virtual audio devices.
    pub async fn start() -> Result<Self, String> {
        let image = AudioTestImage::new();

        // Build image if needed
        image.build_if_needed()?;

        // Start container
        let container = image
            .start()
            .await
            .map_err(|e| format!("Failed to start container: {}", e))?;

        // Wait a bit for PulseAudio to fully initialize
        tokio::time::sleep(Duration::from_secs(3)).await;

        Ok(Self { container })
    }

    /// Start with custom image configuration.
    pub async fn start_with_image(image: AudioTestImage) -> Result<Self, String> {
        image.build_if_needed()?;

        let container = image
            .start()
            .await
            .map_err(|e| format!("Failed to start container: {}", e))?;

        tokio::time::sleep(Duration::from_secs(3)).await;

        Ok(Self { container })
    }

    /// Get the container ID.
    pub fn container_id(&self) -> &str {
        self.container.id()
    }

    /// Execute a command in the container.
    async fn exec(&self, args: &[&str]) -> Result<String, String> {
        let mut cmd_args = vec!["exec", self.container.id()];
        cmd_args.extend_from_slice(args);

        let output = Command::new("docker")
            .args(&cmd_args)
            .output()
            .map_err(|e| format!("Failed to execute command: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Command failed: {}", stderr));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Execute a command as testuser in the container.
    pub async fn exec_as_testuser(&self, command: &str) -> Result<String, String> {
        self.exec(&["su", "-", "testuser", "-c", command]).await
    }

    /// List all available virtual audio devices.
    pub async fn list_devices(&self) -> Result<Vec<VirtualAudioDevice>, String> {
        let mut devices = Vec::new();

        // Get sinks (outputs)
        let sinks_output = self.exec_as_testuser("pactl list sinks short").await?;
        let default_sink = self
            .exec_as_testuser("pactl get-default-sink")
            .await
            .unwrap_or_default()
            .trim()
            .to_string();

        for line in sinks_output.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let name = parts[1].to_string();
                let is_default = name == default_sink;

                // Determine sample rate from device name
                let sample_rate = if name.contains("hires") {
                    192000
                } else if name.contains("3") {
                    96000
                } else if name.contains("2") {
                    48000
                } else {
                    44100
                };

                devices.push(VirtualAudioDevice {
                    name: name.clone(),
                    description: format!("Virtual Output: {}", name),
                    sample_rate,
                    channels: 2,
                    is_default,
                    device_type: AudioDeviceType::Sink,
                });
            }
        }

        // Get sources (inputs)
        let sources_output = self.exec_as_testuser("pactl list sources short").await?;
        let default_source = self
            .exec_as_testuser("pactl get-default-source")
            .await
            .unwrap_or_default()
            .trim()
            .to_string();

        for line in sources_output.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let name = parts[1].to_string();
                // Skip monitor sources - we only want actual virtual inputs
                if name.ends_with(".monitor") {
                    continue;
                }

                let is_default = name == default_source;

                devices.push(VirtualAudioDevice {
                    name: name.clone(),
                    description: format!("Virtual Input: {}", name),
                    sample_rate: 44100,
                    channels: 2,
                    is_default,
                    device_type: AudioDeviceType::Source,
                });
            }
        }

        Ok(devices)
    }

    /// List only sink (output) devices.
    pub async fn list_sinks(&self) -> Result<Vec<VirtualAudioDevice>, String> {
        let devices = self.list_devices().await?;
        Ok(devices
            .into_iter()
            .filter(|d| d.device_type == AudioDeviceType::Sink)
            .collect())
    }

    /// List only source (input) devices.
    pub async fn list_sources(&self) -> Result<Vec<VirtualAudioDevice>, String> {
        let devices = self.list_devices().await?;
        Ok(devices
            .into_iter()
            .filter(|d| d.device_type == AudioDeviceType::Source)
            .collect())
    }

    /// Play a test tone to a virtual sink.
    ///
    /// # Arguments
    /// * `sink` - Name of the sink (e.g., "virtual_output_1")
    /// * `frequency` - Frequency in Hz
    /// * `duration_secs` - Duration in seconds
    pub async fn play_test_tone(
        &self,
        sink: &str,
        frequency: u32,
        duration_secs: f32,
    ) -> Result<(), String> {
        let cmd = format!(
            "sox -n -t pulseaudio {} synth {} sine {}",
            sink, duration_secs, frequency
        );

        self.exec_as_testuser(&cmd).await?;
        Ok(())
    }

    /// Play audio from a WAV file to a virtual sink.
    pub async fn play_audio_file(&self, sink: &str, file_path: &str) -> Result<(), String> {
        let cmd = format!("paplay -d {} {}", sink, file_path);
        self.exec_as_testuser(&cmd).await?;
        Ok(())
    }

    /// Record audio from a virtual source.
    ///
    /// # Arguments
    /// * `source` - Name of the source (e.g., "virtual_input_1")
    /// * `duration_secs` - Recording duration in seconds
    /// * `output_path` - Path in container to save the recording
    pub async fn record_from_source(
        &self,
        source: &str,
        duration_secs: f32,
        output_path: &str,
    ) -> Result<AudioRecording, String> {
        // Start recording in background
        let cmd = format!(
            "timeout {} parecord -d {} --file-format=wav {} &",
            duration_secs + 0.5,
            source,
            output_path
        );
        self.exec_as_testuser(&cmd).await?;

        // Wait for recording to complete
        tokio::time::sleep(Duration::from_secs_f32(duration_secs + 1.0)).await;

        Ok(AudioRecording {
            path: output_path.to_string(),
            duration_secs,
            sample_rate: 44100,
            channels: 2,
        })
    }

    /// Detect underruns on a sink.
    ///
    /// Returns the number of underruns detected.
    pub async fn detect_underruns(&self, sink: &str) -> Result<u32, String> {
        let output = self
            .exec_as_testuser(&format!("pactl list sinks | grep -A 20 '{}'", sink))
            .await?;

        // Look for underrun count in output
        // PulseAudio reports this in sink statistics
        if let Some(line) = output.lines().find(|l| l.contains("underrun")) {
            // Try to parse the number
            for word in line.split_whitespace() {
                if let Ok(count) = word.parse::<u32>() {
                    return Ok(count);
                }
            }
        }

        Ok(0) // No underruns detected
    }

    /// Detect audio glitches in a recorded file.
    pub async fn detect_glitches(&self, wav_path: &str) -> Result<GlitchReport, String> {
        let output = self
            .exec_as_testuser(&format!("sox {} -n stats 2>&1", wav_path))
            .await?;

        let mut report = GlitchReport::default();

        for line in output.lines() {
            if line.contains("Pk lev dB") {
                if let Some(value) = line.split_whitespace().last() {
                    report.peak_level_db = value.parse().unwrap_or(0.0);
                    if report.peak_level_db >= 0.0 {
                        report.clipping_count = 1;
                        report.has_glitches = true;
                    }
                }
            }
            if line.contains("Flat factor") {
                if let Some(value) = line.split_whitespace().last() {
                    if let Ok(flat) = value.parse::<f32>() {
                        if flat > 100.0 {
                            // High flat factor indicates long silence
                            report.silence_gaps = 1;
                            report.has_glitches = true;
                        }
                    }
                }
            }
        }

        Ok(report)
    }

    /// Switch the default sink.
    pub async fn switch_default_sink(&self, sink: &str) -> Result<(), String> {
        self.exec_as_testuser(&format!("pactl set-default-sink {}", sink))
            .await?;
        Ok(())
    }

    /// Switch the default source.
    pub async fn switch_default_source(&self, source: &str) -> Result<(), String> {
        self.exec_as_testuser(&format!("pactl set-default-source {}", source))
            .await?;
        Ok(())
    }

    /// Set volume on a sink (0-100).
    pub async fn set_volume(&self, sink: &str, volume_percent: u8) -> Result<(), String> {
        let volume = volume_percent.min(100);
        self.exec_as_testuser(&format!("pactl set-sink-volume {} {}%", sink, volume))
            .await?;
        Ok(())
    }

    /// Mute or unmute a sink.
    pub async fn set_mute(&self, sink: &str, mute: bool) -> Result<(), String> {
        let mute_arg = if mute { "1" } else { "0" };
        self.exec_as_testuser(&format!("pactl set-sink-mute {} {}", sink, mute_arg))
            .await?;
        Ok(())
    }

    /// Verify the audio pipeline is working.
    pub async fn verify_pipeline(&self, sink: &str) -> Result<bool, String> {
        let result = self
            .exec_as_testuser(&format!(
                "sox -n -t pulseaudio {} synth 0.5 sine 1000 2>&1",
                sink
            ))
            .await;

        match result {
            Ok(_) => Ok(true),
            Err(e) if e.contains("cannot open") => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// Get PulseAudio server info.
    pub async fn get_server_info(&self) -> Result<String, String> {
        self.exec_as_testuser("pactl info").await
    }

    /// Check if PulseAudio is running.
    pub async fn is_pulseaudio_running(&self) -> bool {
        self.exec_as_testuser("pgrep pulseaudio")
            .await
            .map(|out| !out.trim().is_empty())
            .unwrap_or(false)
    }
}

/// Check if Docker is available on the system.
pub fn is_docker_available() -> bool {
    Command::new("docker")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_docker_availability() {
        let available = is_docker_available();
        println!("Docker available: {}", available);
    }
}
