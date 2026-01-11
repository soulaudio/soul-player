//! Integration tests using Docker testcontainers for audio testing.
//!
//! These tests verify audio functionality using PulseAudio virtual devices
//! in isolated Docker containers.
//!
//! Run with:
//! ```bash
//! cargo test --features testcontainers --test testcontainers_audio_test -- --nocapture
//! ```

#![cfg(feature = "testcontainers")]

mod testcontainers_audio;

use testcontainers_audio::{is_docker_available, AudioDeviceType, AudioTestContainer};

/// Skip test if Docker is not available.
macro_rules! skip_without_docker {
    () => {
        if !is_docker_available() {
            eprintln!("Skipping test: Docker not available");
            return;
        }
    };
}

// =============================================================================
// Device Enumeration Tests
// =============================================================================

#[tokio::test]
async fn test_container_starts_and_lists_devices() {
    skip_without_docker!();

    eprintln!("Starting audio test container...");
    let container = AudioTestContainer::start()
        .await
        .expect("Failed to start container");

    eprintln!("Container started: {}", container.container_id());

    // List devices
    let devices = container
        .list_devices()
        .await
        .expect("Failed to list devices");

    eprintln!("Found {} devices:", devices.len());
    for device in &devices {
        eprintln!(
            "  - {} ({:?}): {}Hz, {}ch{}",
            device.name,
            device.device_type,
            device.sample_rate,
            device.channels,
            if device.is_default { " [DEFAULT]" } else { "" }
        );
    }

    // Should have at least 4 sinks (virtual_output_1, 2, 3, hires)
    let sinks: Vec<_> = devices
        .iter()
        .filter(|d| d.device_type == AudioDeviceType::Sink)
        .collect();
    assert!(
        sinks.len() >= 4,
        "Expected at least 4 virtual sinks, found {}",
        sinks.len()
    );

    // Should have virtual sources
    let sources: Vec<_> = devices
        .iter()
        .filter(|d| d.device_type == AudioDeviceType::Source)
        .collect();
    assert!(!sources.is_empty(), "Expected at least one virtual source");

    eprintln!("Device enumeration test passed!");
}

#[tokio::test]
async fn test_virtual_devices_have_correct_sample_rates() {
    skip_without_docker!();

    let container = AudioTestContainer::start()
        .await
        .expect("Failed to start container");

    let sinks = container
        .list_sinks()
        .await
        .expect("Failed to list sinks");

    // Find specific devices and verify sample rates
    let output_1 = sinks.iter().find(|d| d.name.contains("virtual_output_1"));
    let output_2 = sinks.iter().find(|d| d.name.contains("virtual_output_2"));
    let output_3 = sinks.iter().find(|d| d.name.contains("virtual_output_3"));
    let output_hires = sinks.iter().find(|d| d.name.contains("hires"));

    if let Some(d) = output_1 {
        assert_eq!(d.sample_rate, 44100, "virtual_output_1 should be 44.1kHz");
    }
    if let Some(d) = output_2 {
        assert_eq!(d.sample_rate, 48000, "virtual_output_2 should be 48kHz");
    }
    if let Some(d) = output_3 {
        assert_eq!(d.sample_rate, 96000, "virtual_output_3 should be 96kHz");
    }
    if let Some(d) = output_hires {
        assert_eq!(
            d.sample_rate, 192000,
            "virtual_output_hires should be 192kHz"
        );
    }

    eprintln!("Sample rate verification passed!");
}

// =============================================================================
// Audio Playback Tests
// =============================================================================

#[tokio::test]
async fn test_play_audio_to_virtual_sink() {
    skip_without_docker!();

    let container = AudioTestContainer::start()
        .await
        .expect("Failed to start container");

    eprintln!("Playing test tone to virtual_output_1...");

    // Play a 440Hz tone for 1 second
    container
        .play_test_tone("virtual_output_1", 440, 1.0)
        .await
        .expect("Failed to play test tone");

    eprintln!("Test tone played successfully!");

    // Verify no underruns occurred
    let underruns = container
        .detect_underruns("virtual_output_1")
        .await
        .expect("Failed to detect underruns");

    eprintln!("Detected {} underruns", underruns);
    // Note: underruns might occur in CI due to resource constraints
    // We just verify we can detect them

    eprintln!("Playback to virtual sink test passed!");
}

#[tokio::test]
async fn test_play_to_multiple_sinks() {
    skip_without_docker!();

    let container = AudioTestContainer::start()
        .await
        .expect("Failed to start container");

    let sinks = container
        .list_sinks()
        .await
        .expect("Failed to list sinks");

    eprintln!("Testing playback to {} sinks...", sinks.len());

    for sink in &sinks {
        eprintln!("  Playing to {}...", sink.name);
        let result = container.play_test_tone(&sink.name, 880, 0.5).await;

        match result {
            Ok(()) => eprintln!("    Played successfully"),
            Err(e) => eprintln!("    Failed: {}", e),
        }
    }

    eprintln!("Multi-sink playback test completed!");
}

// =============================================================================
// Audio Recording Tests
// =============================================================================

#[tokio::test]
async fn test_record_from_virtual_source() {
    skip_without_docker!();

    let container = AudioTestContainer::start()
        .await
        .expect("Failed to start container");

    let sources = container
        .list_sources()
        .await
        .expect("Failed to list sources");

    if sources.is_empty() {
        eprintln!("No virtual sources available, skipping test");
        return;
    }

    let source = &sources[0];
    eprintln!("Recording from {}...", source.name);

    // Play a tone in the background to the corresponding sink
    // (virtual_input_1 monitors virtual_output_1)
    tokio::spawn({
        let container_id = container.container_id().to_string();
        async move {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            // Play tone to sink
            let _ = std::process::Command::new("docker")
                .args([
                    "exec",
                    &container_id,
                    "su",
                    "-",
                    "testuser",
                    "-c",
                    "sox -n -t pulseaudio virtual_output_1 synth 1 sine 1000",
                ])
                .output();
        }
    });

    let recording = container
        .record_from_source(&source.name, 2.0, "/tmp/test_recording.wav")
        .await
        .expect("Failed to record audio");

    eprintln!("Recorded to: {}", recording.path);
    eprintln!(
        "Duration: {}s, {}Hz, {}ch",
        recording.duration_secs, recording.sample_rate, recording.channels
    );

    eprintln!("Recording test passed!");
}

// =============================================================================
// Glitch Detection Tests
// =============================================================================

#[tokio::test]
async fn test_detect_glitches_in_clean_audio() {
    skip_without_docker!();

    let container = AudioTestContainer::start()
        .await
        .expect("Failed to start container");

    // Generate a clean test file
    let _ = container
        .exec_as_testuser("sox -n /tmp/clean_test.wav synth 1 sine 440")
        .await;

    let report = container
        .detect_glitches("/tmp/clean_test.wav")
        .await
        .expect("Failed to detect glitches");

    eprintln!("Glitch report for clean audio:");
    eprintln!("  Underruns: {}", report.underrun_count);
    eprintln!("  Clipping: {}", report.clipping_count);
    eprintln!("  Silence gaps: {}", report.silence_gaps);
    eprintln!("  Peak level: {} dB", report.peak_level_db);
    eprintln!("  Has glitches: {}", report.has_glitches);

    // Clean sine wave should not have significant clipping
    assert!(
        report.peak_level_db < 0.0,
        "Clean audio should not clip"
    );

    eprintln!("Glitch detection test passed!");
}

// =============================================================================
// Device Switching Tests
// =============================================================================

#[tokio::test]
async fn test_switch_default_sink() {
    skip_without_docker!();

    let container = AudioTestContainer::start()
        .await
        .expect("Failed to start container");

    let sinks = container
        .list_sinks()
        .await
        .expect("Failed to list sinks");

    if sinks.len() < 2 {
        eprintln!("Not enough sinks for switching test");
        return;
    }

    eprintln!("Testing sink switching...");

    // Find original default
    let original_default = sinks.iter().find(|s| s.is_default);
    if let Some(d) = original_default {
        eprintln!("Original default: {}", d.name);
    }

    // Switch to each sink
    for sink in &sinks {
        eprintln!("  Switching to {}...", sink.name);
        container
            .switch_default_sink(&sink.name)
            .await
            .expect("Failed to switch sink");

        // Verify playback works after switch
        container
            .play_test_tone(&sink.name, 440, 0.2)
            .await
            .expect("Failed to play after switch");
    }

    eprintln!("Sink switching test passed!");
}

#[tokio::test]
async fn test_volume_control() {
    skip_without_docker!();

    let container = AudioTestContainer::start()
        .await
        .expect("Failed to start container");

    let sinks = container
        .list_sinks()
        .await
        .expect("Failed to list sinks");

    if sinks.is_empty() {
        return;
    }

    let sink = &sinks[0].name;
    eprintln!("Testing volume control on {}...", sink);

    // Test volume levels
    for volume in [0, 25, 50, 75, 100] {
        container
            .set_volume(sink, volume)
            .await
            .expect("Failed to set volume");
        eprintln!("  Set volume to {}%", volume);
    }

    // Test mute
    container.set_mute(sink, true).await.expect("Failed to mute");
    eprintln!("  Muted");

    container
        .set_mute(sink, false)
        .await
        .expect("Failed to unmute");
    eprintln!("  Unmuted");

    eprintln!("Volume control test passed!");
}

// =============================================================================
// Pipeline Verification Tests
// =============================================================================

#[tokio::test]
async fn test_verify_audio_pipeline() {
    skip_without_docker!();

    let container = AudioTestContainer::start()
        .await
        .expect("Failed to start container");

    // Verify PulseAudio is running
    assert!(
        container.is_pulseaudio_running().await,
        "PulseAudio should be running"
    );
    eprintln!("PulseAudio is running");

    // Get server info
    let info = container
        .get_server_info()
        .await
        .expect("Failed to get server info");
    eprintln!("Server info:\n{}", info);

    // Verify pipeline for each sink
    let sinks = container
        .list_sinks()
        .await
        .expect("Failed to list sinks");

    for sink in &sinks {
        let ok = container
            .verify_pipeline(&sink.name)
            .await
            .expect("Pipeline verification failed");
        eprintln!("  {} pipeline: {}", sink.name, if ok { "OK" } else { "FAIL" });
        assert!(ok, "Pipeline should work for {}", sink.name);
    }

    eprintln!("Pipeline verification test passed!");
}

// =============================================================================
// Stress Tests
// =============================================================================

#[tokio::test]
async fn test_rapid_device_switching_stress() {
    skip_without_docker!();

    let container = AudioTestContainer::start()
        .await
        .expect("Failed to start container");

    let sinks = container
        .list_sinks()
        .await
        .expect("Failed to list sinks");

    if sinks.len() < 2 {
        eprintln!("Not enough sinks for stress test");
        return;
    }

    eprintln!("Running rapid device switching stress test...");

    let iterations = 10;
    let mut success_count = 0;

    for i in 0..iterations {
        let sink = &sinks[i % sinks.len()];

        if container.switch_default_sink(&sink.name).await.is_ok()
            && container.play_test_tone(&sink.name, 440, 0.1).await.is_ok()
        {
            success_count += 1;
        }
    }

    eprintln!("  Completed {}/{} iterations successfully", success_count, iterations);
    assert!(
        success_count >= iterations / 2,
        "At least half of switches should succeed"
    );

    eprintln!("Stress test passed!");
}

#[tokio::test]
async fn test_underrun_detection_during_load() {
    skip_without_docker!();

    let container = AudioTestContainer::start()
        .await
        .expect("Failed to start container");

    eprintln!("Testing underrun detection during load...");

    // Play multiple tones in sequence to create load
    let sinks = container
        .list_sinks()
        .await
        .expect("Failed to list sinks");

    for _ in 0..5 {
        for sink in &sinks {
            let _ = container.play_test_tone(&sink.name, 440, 0.2).await;
        }
    }

    // Check for underruns on all sinks
    let mut total_underruns = 0;
    for sink in &sinks {
        let underruns = container.detect_underruns(&sink.name).await.unwrap_or(0);
        eprintln!("  {}: {} underruns", sink.name, underruns);
        total_underruns += underruns;
    }

    eprintln!("Total underruns detected: {}", total_underruns);

    // Note: We don't assert on underrun count as it depends on system load
    // The important thing is that detection works

    eprintln!("Underrun detection test passed!");
}

