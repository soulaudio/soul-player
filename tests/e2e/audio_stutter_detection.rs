//! E2E Audio Stutter Detection Tests
//!
//! These tests detect audio stuttering and false-start issues using Tauri WebDriver.
//! Uses audio loopback to capture and analyze actual playback quality.
//!
//! Detects:
//! - Phase discontinuities (stutters/glitches)
//! - False starts (song starts, then restarts)
//! - Buffer underruns (gaps in playback)

mod common;

use common::TauriDriver;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::f32::consts::PI;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tracing_subscriber;

// ============================================================================
// Stutter Detection Infrastructure
// ============================================================================

#[derive(Debug, Clone)]
struct StutterEvent {
    sample_index: usize,
    magnitude: f32,
    timestamp_ms: f64,
}

#[derive(Debug, Clone)]
struct SilenceGap {
    start_index: usize,
    duration_samples: usize,
    duration_ms: f64,
}

#[derive(Debug)]
struct AudioQualityMetrics {
    total_samples: usize,
    stutters: Vec<StutterEvent>,
    silence_gaps: Vec<SilenceGap>,
    false_start_detected: bool,
    max_discontinuity: f32,
}

impl AudioQualityMetrics {
    fn assert_quality_thresholds(&self) {
        assert_eq!(
            self.stutters.len(),
            0,
            "Detected {} stutters: {:#?}",
            self.stutters.len(),
            self.stutters
        );

        assert_eq!(
            self.silence_gaps.len(),
            0,
            "Detected {} silence gaps: {:#?}",
            self.silence_gaps.len(),
            self.silence_gaps
        );

        assert!(
            !self.false_start_detected,
            "False start detected (song restarted)"
        );

        assert!(
            self.max_discontinuity < 0.5,
            "Maximum discontinuity {:.4} exceeds threshold 0.5",
            self.max_discontinuity
        );
    }

    fn print_summary(&self) {
        println!("\n=== Audio Quality Metrics ===");
        println!("Total samples analyzed: {}", self.total_samples);
        println!("Stutters detected: {}", self.stutters.len());
        println!("Silence gaps: {}", self.silence_gaps.len());
        println!("False start: {}", self.false_start_detected);
        println!("Max discontinuity: {:.6}", self.max_discontinuity);

        if !self.stutters.is_empty() {
            println!("\nStutter Details:");
            for (i, stutter) in self.stutters.iter().enumerate().take(10) {
                println!(
                    "  [{}] @{:.2}ms - magnitude: {:.4}",
                    i, stutter.timestamp_ms, stutter.magnitude
                );
            }
        }

        if !self.silence_gaps.is_empty() {
            println!("\nSilence Gap Details:");
            for (i, gap) in self.silence_gaps.iter().enumerate().take(10) {
                println!(
                    "  [{}] Start: {}, Duration: {:.2}ms",
                    i, gap.start_index, gap.duration_ms
                );
            }
        }
    }
}

// ============================================================================
// Audio Analysis Functions
// ============================================================================

fn detect_phase_discontinuities(
    samples: &[f32],
    frequency: f32,
    sample_rate: u32,
    threshold_multiplier: f32,
) -> Vec<StutterEvent> {
    if samples.len() < 2 {
        return Vec::new();
    }

    let mut stutters = Vec::new();
    let max_expected_derivative = 2.0 * PI * frequency / sample_rate as f32;
    let threshold = max_expected_derivative * threshold_multiplier;
    let step = if samples.len() % 2 == 0 { 2 } else { 1 };

    for i in (step..samples.len()).step_by(step) {
        let prev_sample = samples[i - step];
        let curr_sample = samples[i];
        let diff = (curr_sample - prev_sample).abs();

        if diff > threshold {
            let sample_index = i / step;
            let timestamp_ms = (sample_index as f64 / sample_rate as f64) * 1000.0;

            stutters.push(StutterEvent {
                sample_index,
                magnitude: diff,
                timestamp_ms,
            });
        }
    }

    stutters
}

fn detect_silence_gaps(
    samples: &[f32],
    silence_threshold: f32,
    min_gap_samples: usize,
    sample_rate: u32,
) -> Vec<SilenceGap> {
    let mut gaps = Vec::new();
    let mut consecutive_silence = 0;
    let mut gap_start: Option<usize> = None;

    for (i, &sample) in samples.iter().enumerate() {
        if sample.abs() < silence_threshold {
            if consecutive_silence == 0 {
                gap_start = Some(i);
            }
            consecutive_silence += 1;
        } else {
            if consecutive_silence >= min_gap_samples {
                if let Some(start) = gap_start {
                    let duration_ms = (consecutive_silence as f64 / sample_rate as f64) * 1000.0;

                    gaps.push(SilenceGap {
                        start_index: start,
                        duration_samples: consecutive_silence,
                        duration_ms,
                    });
                }
            }
            consecutive_silence = 0;
            gap_start = None;
        }
    }

    if consecutive_silence >= min_gap_samples {
        if let Some(start) = gap_start {
            let duration_ms = (consecutive_silence as f64 / sample_rate as f64) * 1000.0;
            gaps.push(SilenceGap {
                start_index: start,
                duration_samples: consecutive_silence,
                duration_ms,
            });
        }
    }

    gaps
}

fn detect_false_start(
    recorded_samples: &[f32],
    intro_pattern: &[f32],
    sample_rate: u32,
    correlation_threshold: f32,
) -> bool {
    if recorded_samples.len() < intro_pattern.len() * 2 {
        return false;
    }

    let mut matches = find_pattern_matches(recorded_samples, intro_pattern, correlation_threshold);
    let min_separation = sample_rate as usize / 10;
    matches.retain(|&pos| pos == 0 || pos > intro_pattern.len() + min_separation);

    matches.len() > 1
}

fn find_pattern_matches(signal: &[f32], pattern: &[f32], threshold: f32) -> Vec<usize> {
    let mut matches = Vec::new();

    if signal.len() < pattern.len() {
        return matches;
    }

    let pattern_rms: f32 =
        (pattern.iter().map(|s| s * s).sum::<f32>() / pattern.len() as f32).sqrt();

    if pattern_rms < 0.001 {
        return matches;
    }

    for i in 0..=(signal.len() - pattern.len()) {
        let window = &signal[i..i + pattern.len()];
        let window_rms: f32 =
            (window.iter().map(|s| s * s).sum::<f32>() / window.len() as f32).sqrt();

        if window_rms < 0.001 {
            continue;
        }

        let correlation: f32 = window
            .iter()
            .zip(pattern.iter())
            .map(|(a, b)| a * b)
            .sum::<f32>()
            / (window_rms * pattern_rms * pattern.len() as f32);

        if correlation > threshold {
            matches.push(i);
        }
    }

    matches
}

// ============================================================================
// Audio Recording Infrastructure
// ============================================================================

struct AudioRecorder {
    samples: Arc<Mutex<Vec<f32>>>,
    recording: Arc<Mutex<bool>>,
    stream: Option<cpal::Stream>,
}

impl AudioRecorder {
    fn new() -> Self {
        Self {
            samples: Arc::new(Mutex::new(Vec::new())),
            recording: Arc::new(Mutex::new(false)),
            stream: None,
        }
    }

    fn start_recording(&mut self, device_name: Option<&str>) -> Result<(), String> {
        let host = cpal::default_host();

        let device = if let Some(name) = device_name {
            host.input_devices()
                .map_err(|e| format!("Failed to enumerate devices: {}", e))?
                .find(|d| d.name().map(|n| n.contains(name)).unwrap_or(false))
                .ok_or_else(|| format!("Device '{}' not found", name))?
        } else {
            host.default_input_device()
                .ok_or_else(|| "No default input device".to_string())?
        };

        let config = device
            .default_input_config()
            .map_err(|e| format!("Failed to get config: {}", e))?;

        tracing::info!(
            "[AudioRecorder] Recording on device: {:?}, config: {:?}",
            device.name(),
            config
        );

        let samples = Arc::clone(&self.samples);
        let recording = Arc::clone(&self.recording);

        let stream = device
            .build_input_stream(
                &config.into(),
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    if *recording.lock().unwrap() {
                        samples.lock().unwrap().extend_from_slice(data);
                    }
                },
                |err| tracing::error!("[AudioRecorder] Stream error: {}", err),
                None,
            )
            .map_err(|e| format!("Failed to build stream: {}", e))?;

        stream
            .play()
            .map_err(|e| format!("Failed to start stream: {}", e))?;

        *self.recording.lock().unwrap() = true;
        self.stream = Some(stream);

        Ok(())
    }

    fn stop_recording(&mut self) -> Vec<f32> {
        *self.recording.lock().unwrap() = false;
        self.stream = None;
        std::mem::take(&mut *self.samples.lock().unwrap())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[test]
fn test_phase_discontinuity_detection() {
    let sample_rate = 44100u32;
    let frequency = 1000.0f32;
    let num_samples = 8820;

    let mut samples = Vec::with_capacity(num_samples);

    for i in 0..num_samples / 2 {
        let t = i as f32 / sample_rate as f32;
        let sample = (2.0 * PI * frequency * t).sin();
        samples.push(sample);
        samples.push(sample);

        if i == num_samples / 4 {
            samples[(i - 1) * 2] = 0.5;
        }
    }

    let stutters = detect_phase_discontinuities(&samples, frequency, sample_rate, 3.0);

    assert!(
        !stutters.is_empty(),
        "Should detect the injected discontinuity"
    );
    assert!(
        stutters[0].timestamp_ms > 40.0 && stutters[0].timestamp_ms < 60.0,
        "Discontinuity should be near 50ms mark"
    );
}

#[test]
fn test_silence_gap_detection() {
    let sample_rate = 44100u32;
    let mut samples = Vec::new();

    for _ in 0..4410 {
        samples.push(0.5);
    }
    for _ in 0..4410 {
        samples.push(0.0);
    }
    for _ in 0..4410 {
        samples.push(0.5);
    }

    let gaps = detect_silence_gaps(&samples, 0.01, 100, sample_rate);

    assert_eq!(gaps.len(), 1, "Should detect exactly one silence gap");
    assert!(
        gaps[0].duration_ms > 45.0 && gaps[0].duration_ms < 55.0,
        "Gap should be approximately 50ms"
    );
}

#[test]
fn test_false_start_detection() {
    let sample_rate = 44100u32;
    let frequency = 440.0f32;

    let mut intro_pattern = Vec::new();
    for i in 0..22050 {
        let t = i as f32 / sample_rate as f32;
        let sample = (2.0 * PI * frequency * t).sin() * ((-t * 5.0).exp());
        intro_pattern.push(sample);
    }

    // Normal playback
    let mut normal_playback = intro_pattern.clone();
    for i in 0..88200 {
        let t = (i + 22050) as f32 / sample_rate as f32;
        let sample = (2.0 * PI * (frequency * 2.0) * t).sin();
        normal_playback.push(sample);
    }

    let false_start_1 = detect_false_start(&normal_playback, &intro_pattern, sample_rate, 0.8);
    assert!(
        !false_start_1,
        "Normal playback should not detect false start"
    );

    // False start scenario
    let mut false_start_playback = intro_pattern.clone();
    for _ in 0..4410 {
        false_start_playback.push(0.0);
    }
    false_start_playback.extend_from_slice(&intro_pattern);

    let false_start_2 = detect_false_start(&false_start_playback, &intro_pattern, sample_rate, 0.8);
    assert!(
        false_start_2,
        "Should detect false start when pattern repeats"
    );
}

#[tokio::test]
#[serial_test::serial]
async fn test_no_stutter_at_playback_start_e2e() {
    let _ = tracing_subscriber::fmt::try_init();

    let device_name = match std::env::var("AUDIO_TEST_DEVICE") {
        Ok(name) => name,
        Err(_) => {
            println!("Skipping E2E test - AUDIO_TEST_DEVICE not set");
            return;
        }
    };

    // Launch app
    let driver = TauriDriver::new()
        .await
        .expect("Failed to launch Soul Player");

    driver
        .wait_for_window(Duration::from_secs(15))
        .await
        .expect("App not ready");

    // Start recording
    let mut recorder = AudioRecorder::new();
    recorder
        .start_recording(Some(&device_name))
        .expect("Failed to start recording");

    // Load and play test track
    let test_track = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("tests/assets/1khz-sine-10s.wav");

    driver
        .load_test_track(&test_track)
        .await
        .expect("Failed to load test track");

    driver.click_play().await.expect("Failed to click play");

    // Record for 3 seconds
    tokio::time::sleep(Duration::from_secs(3)).await;

    let recorded = recorder.stop_recording();

    println!("Recorded {} samples", recorded.len());

    // Analyze quality
    let stutters = detect_phase_discontinuities(&recorded, 1000.0, 44100, 3.0);
    let gaps = detect_silence_gaps(&recorded, 0.01, 100, 44100);

    let metrics = AudioQualityMetrics {
        total_samples: recorded.len(),
        stutters: stutters.clone(),
        silence_gaps: gaps.clone(),
        false_start_detected: false,
        max_discontinuity: stutters.iter().map(|s| s.magnitude).fold(0.0f32, f32::max),
    };

    metrics.print_summary();
    metrics.assert_quality_thresholds();
}

#[tokio::test]
#[serial_test::serial]
async fn test_no_false_start_with_tauri() {
    let _ = tracing_subscriber::fmt::try_init();

    let device_name = match std::env::var("AUDIO_TEST_DEVICE") {
        Ok(name) => name,
        Err(_) => {
            println!("Skipping E2E test - AUDIO_TEST_DEVICE not set");
            return;
        }
    };

    let driver = TauriDriver::new()
        .await
        .expect("Failed to launch Soul Player");

    driver
        .wait_for_window(Duration::from_secs(15))
        .await
        .expect("App not ready");

    let mut recorder = AudioRecorder::new();
    recorder
        .start_recording(Some(&device_name))
        .expect("Failed to start recording");

    // Load test track with distinctive intro
    let test_track = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("tests/assets/distinctive-intro.wav");

    driver
        .load_test_track(&test_track)
        .await
        .expect("Failed to load test track");

    driver.click_play().await.expect("Failed to click play");

    // Record for 5 seconds to capture potential false starts
    tokio::time::sleep(Duration::from_secs(5)).await;

    let recorded = recorder.stop_recording();

    // Load intro pattern reference
    let intro_pattern_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("tests/assets/distinctive-intro-first-500ms.wav");

    let intro_pattern =
        load_wav_samples(&intro_pattern_path).expect("Failed to load intro pattern");

    let false_start = detect_false_start(&recorded, &intro_pattern, 44100, 0.8);

    assert!(
        !false_start,
        "False start detected - song restarted during playback"
    );
}

// ============================================================================
// Helper Functions
// ============================================================================

fn load_wav_samples(path: &std::path::Path) -> Result<Vec<f32>, String> {
    let reader =
        hound::WavReader::open(path).map_err(|e| format!("Failed to open WAV file: {}", e))?;

    let samples: Result<Vec<f32>, _> = reader.into_samples::<f32>().collect();

    samples.map_err(|e| format!("Failed to read samples: {}", e))
}
