//! E2E Audio Initialization Latency Tests
//!
//! These tests measure real audio output latency using Tauri WebDriver.
//! Requires virtual audio device setup (see tests/e2e/README.md).
//!
//! Run with:
//! ```bash
//! # Linux (requires snd-aloop)
//! AUDIO_TEST_DEVICE="hw:Loopback,0,0" cargo test --test audio_initialization_latency
//!
//! # macOS (requires BlackHole)
//! AUDIO_TEST_DEVICE="BlackHole 2ch" cargo test --test audio_initialization_latency
//!
//! # Windows (requires VB-Cable)
//! $env:AUDIO_TEST_DEVICE="CABLE Input"; cargo test --test audio_initialization_latency
//! ```

mod common;

use common::TauriDriver;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tracing_subscriber;

// ============================================================================
// Test Configuration
// ============================================================================

#[derive(Debug, Clone)]
struct AudioTestConfig {
    /// Use real audio device (virtual loopback)
    use_real_device: bool,
    /// Device name to use for testing
    device_name: Option<String>,
    /// Test timeout
    timeout: Duration,
    /// Silence detection threshold
    silence_threshold: f32,
    /// Minimum consecutive samples to detect audio start
    min_samples_for_detection: usize,
}

impl AudioTestConfig {
    fn from_env() -> Self {
        Self {
            use_real_device: std::env::var("AUDIO_E2E_REAL_DEVICE").is_ok()
                || std::env::var("AUDIO_TEST_DEVICE").is_ok(),
            device_name: std::env::var("AUDIO_TEST_DEVICE").ok(),
            timeout: Duration::from_secs(
                std::env::var("AUDIO_TEST_TIMEOUT")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(10),
            ),
            silence_threshold: 0.01,
            min_samples_for_detection: 100,
        }
    }
}

// ============================================================================
// Audio Monitoring Infrastructure
// ============================================================================

struct AudioMonitor {
    started: Arc<AtomicBool>,
    first_audio_detected: Arc<Mutex<Option<Instant>>>,
    total_samples: Arc<AtomicU64>,
    config: AudioTestConfig,
    stream: Option<cpal::Stream>,
}

impl AudioMonitor {
    fn new(config: AudioTestConfig) -> Result<Self, String> {
        Ok(Self {
            started: Arc::new(AtomicBool::new(false)),
            first_audio_detected: Arc::new(Mutex::new(None)),
            total_samples: Arc::new(AtomicU64::new(0)),
            config,
            stream: None,
        })
    }

    fn start_monitoring(&mut self) -> Result<(), String> {
        if !self.config.use_real_device {
            return Ok(()); // Skip for mock tests
        }

        let host = cpal::default_host();

        // Find the loopback/virtual device
        let device = if let Some(ref name) = self.config.device_name {
            host.input_devices()
                .map_err(|e| format!("Failed to enumerate devices: {}", e))?
                .find(|d| d.name().map(|n| n.contains(name)).unwrap_or(false))
                .ok_or_else(|| format!("Device '{}' not found", name))?
        } else {
            host.default_input_device()
                .ok_or_else(|| "No default input device available".to_string())?
        };

        let config = device
            .default_input_config()
            .map_err(|e| format!("Failed to get device config: {}", e))?;

        tracing::info!(
            "[AudioMonitor] Starting monitoring on device: {:?}, config: {:?}",
            device.name(),
            config
        );

        let threshold = self.config.silence_threshold;
        let min_samples = self.config.min_samples_for_detection;
        let started = Arc::clone(&self.started);
        let first_audio = Arc::clone(&self.first_audio_detected);
        let total_samples = Arc::clone(&self.total_samples);

        let mut consecutive_audio_samples = 0;

        let stream = device
            .build_input_stream(
                &config.into(),
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    if !started.load(Ordering::Relaxed) {
                        return;
                    }

                    let samples_before = total_samples.fetch_add(data.len() as u64, Ordering::Relaxed);
                    let max_magnitude = data.iter().map(|s| s.abs()).fold(0.0f32, f32::max);

                    if max_magnitude > threshold {
                        consecutive_audio_samples += data.len();

                        if consecutive_audio_samples >= min_samples {
                            let mut first = first_audio.lock().unwrap();
                            if first.is_none() {
                                let now = Instant::now();
                                *first = Some(now);
                                tracing::info!(
                                    "[AudioMonitor] First audio detected at sample {} (magnitude: {:.4})",
                                    samples_before,
                                    max_magnitude
                                );
                            }
                        }
                    } else {
                        consecutive_audio_samples = 0;
                    }
                },
                |err| {
                    tracing::error!("[AudioMonitor] Stream error: {}", err);
                },
                None,
            )
            .map_err(|e| format!("Failed to build input stream: {}", e))?;

        stream
            .play()
            .map_err(|e| format!("Failed to start stream: {}", e))?;

        self.stream = Some(stream);
        self.started.store(true, Ordering::Relaxed);

        Ok(())
    }

    fn wait_for_audio(&self, timeout: Duration) -> Result<Instant, String> {
        let start = Instant::now();

        loop {
            {
                let first = self.first_audio_detected.lock().unwrap();
                if let Some(detected_time) = *first {
                    return Ok(detected_time);
                }
            }

            if start.elapsed() > timeout {
                return Err(format!(
                    "No audio detected after {:?} (samples processed: {})",
                    timeout,
                    self.total_samples.load(Ordering::Relaxed)
                ));
            }

            std::thread::sleep(Duration::from_millis(10));
        }
    }

    fn stop(&mut self) {
        self.started.store(false, Ordering::Relaxed);
        self.stream = None;
    }
}

// ============================================================================
// Test Metrics Collection
// ============================================================================

#[derive(Debug, Clone)]
struct AudioPlaybackMetrics {
    play_to_audio_latency: Duration,
    total_samples_processed: u64,
    test_duration: Duration,
}

impl AudioPlaybackMetrics {
    fn assert_thresholds(&self, max_latency: Duration) {
        assert!(
            self.play_to_audio_latency <= max_latency,
            "Play-to-audio latency {:?} exceeds threshold {:?}",
            self.play_to_audio_latency,
            max_latency
        );
    }

    fn print_summary(&self) {
        println!("\n=== Audio Playback Metrics ===");
        println!(
            "Play-to-audio latency: {:.2}ms",
            self.play_to_audio_latency.as_secs_f64() * 1000.0
        );
        println!("Total samples: {}", self.total_samples_processed);
        println!("Test duration: {:.2}s", self.test_duration.as_secs_f64());
    }
}

// ============================================================================
// Tests
// ============================================================================

#[tokio::test]
#[serial_test::serial]
async fn test_cold_start_latency_with_tauri() {
    // Initialize logging
    let _ = tracing_subscriber::fmt::try_init();

    let config = AudioTestConfig::from_env();

    if !config.use_real_device {
        println!("Skipping real device test (no AUDIO_TEST_DEVICE set)");
        println!("Set AUDIO_TEST_DEVICE environment variable to run this test");
        return;
    }

    // Start audio monitoring
    let mut monitor = AudioMonitor::new(config.clone()).expect("Failed to create audio monitor");
    monitor
        .start_monitoring()
        .expect("Failed to start monitoring");

    // Launch Tauri app
    let driver = TauriDriver::new()
        .await
        .expect("Failed to launch Soul Player");

    // Wait for app ready
    driver
        .wait_for_window(Duration::from_secs(15))
        .await
        .expect("App window not ready");

    // Load test track
    let test_track = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("tests/assets/1khz-sine-10s.wav");

    if !test_track.exists() {
        panic!(
            "Test track not found: {}\nRun: ./scripts/generate-test-audio.sh",
            test_track.display()
        );
    }

    driver
        .load_test_track(&test_track)
        .await
        .expect("Failed to load test track");

    // Click play and measure latency
    let play_start = Instant::now();
    driver.click_play().await.expect("Failed to click play");

    // Wait for first audio
    let first_audio_time = monitor
        .wait_for_audio(Duration::from_secs(5))
        .expect("No audio detected");

    let latency = first_audio_time.duration_since(play_start);

    monitor.stop();

    // Report metrics
    let metrics = AudioPlaybackMetrics {
        play_to_audio_latency: latency,
        total_samples_processed: monitor.total_samples.load(Ordering::Relaxed),
        test_duration: play_start.elapsed(),
    };

    metrics.print_summary();
    metrics.assert_thresholds(Duration::from_millis(1000)); // 1s for cold start

    // Export metrics for CI
    if let Ok(output_path) = std::env::var("AUDIO_METRICS_OUTPUT") {
        export_metrics_to_json(&metrics, &output_path);
    }
}

#[tokio::test]
#[serial_test::serial]
async fn test_warm_start_latency_with_tauri() {
    let _ = tracing_subscriber::fmt::try_init();

    let config = AudioTestConfig::from_env();

    if !config.use_real_device {
        println!("Skipping real device test");
        return;
    }

    let driver = TauriDriver::new()
        .await
        .expect("Failed to launch Soul Player");

    driver
        .wait_for_window(Duration::from_secs(15))
        .await
        .expect("App not ready");

    let test_track = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("tests/assets/1khz-sine-10s.wav");

    driver
        .load_test_track(&test_track)
        .await
        .expect("Failed to load test track");

    // First play (warm up)
    driver.click_play().await.expect("Failed to click play");
    tokio::time::sleep(Duration::from_secs(1)).await;
    driver.click_pause().await.expect("Failed to pause");
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Start monitoring for second play
    let mut monitor = AudioMonitor::new(config.clone()).expect("Failed to create monitor");
    monitor
        .start_monitoring()
        .expect("Failed to start monitoring");

    // Second play (warm start)
    let play_start = Instant::now();
    driver.click_play().await.expect("Failed to click play");

    let first_audio_time = monitor
        .wait_for_audio(Duration::from_secs(2))
        .expect("No audio detected");

    let latency = first_audio_time.duration_since(play_start);
    monitor.stop();

    let metrics = AudioPlaybackMetrics {
        play_to_audio_latency: latency,
        total_samples_processed: monitor.total_samples.load(Ordering::Relaxed),
        test_duration: play_start.elapsed(),
    };

    metrics.print_summary();
    metrics.assert_thresholds(Duration::from_millis(100)); // 100ms for warm start
}

// ============================================================================
// Helper Functions
// ============================================================================

fn export_metrics_to_json(metrics: &AudioPlaybackMetrics, path: &str) {
    let json = serde_json::json!({
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "commit": std::env::var("GITHUB_SHA").ok(),
        "metrics": {
            "play_to_audio_latency_ms": metrics.play_to_audio_latency.as_millis(),
            "total_samples": metrics.total_samples_processed,
            "test_duration_ms": metrics.test_duration.as_millis(),
        }
    });

    std::fs::write(path, serde_json::to_string_pretty(&json).unwrap())
        .expect("Failed to write metrics");
}

/// Helper test to verify audio monitoring infrastructure works
#[test]
fn list_available_audio_devices() {
    let host = cpal::default_host();

    println!("\n=== Available Audio Devices ===");

    if let Ok(devices) = host.input_devices() {
        println!("\nInput Devices:");
        for (i, device) in devices.enumerate() {
            if let Ok(name) = device.name() {
                println!("  [{}] {}", i, name);

                if name.contains("Loopback")
                    || name.contains("BlackHole")
                    || name.contains("CABLE")
                    || name.contains("VB-Audio")
                {
                    println!("      ^ Virtual device detected - suitable for testing");
                }
            }
        }
    }

    if let Ok(devices) = host.output_devices() {
        println!("\nOutput Devices:");
        for (i, device) in devices.enumerate() {
            if let Ok(name) = device.name() {
                println!("  [{}] {}", i, name);
            }
        }
    }

    println!("\nTo use a virtual device for testing, set:");
    println!("  export AUDIO_TEST_DEVICE=\"device_name\"");
}
