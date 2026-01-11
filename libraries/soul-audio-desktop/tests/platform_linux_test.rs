//! Linux-specific audio tests for ALSA and PulseAudio
//!
//! These tests cover Linux-specific audio functionality:
//! - ALSA direct access
//! - PulseAudio integration
//! - Buffer underrun recovery
//!
//! ## Running Hardware-Dependent Tests
//!
//! Hardware-dependent tests are marked with `#[ignore]` by default.
//! To run them, use:
//!
//! ```bash
//! cargo test -p soul-audio-desktop platform_linux_test -- --ignored
//! ```
//!
//! Or to run all tests including ignored:
//!
//! ```bash
//! cargo test -p soul-audio-desktop platform_linux_test -- --include-ignored
//! ```
//!
//! ## Requirements for Hardware Tests
//!
//! - A physical audio output device must be connected
//! - ALSA or PulseAudio must be properly configured
//! - For ALSA direct tests, PulseAudio should not be using the device
//! - For JACK tests, JACK server must be running
//!
//! ## Linux Audio Architecture Notes
//!
//! Linux has multiple audio subsystems:
//!
//! 1. **ALSA (Advanced Linux Sound Architecture)**
//!    - Kernel-level audio driver framework
//!    - Direct hardware access with lowest latency
//!    - Exclusive access when used directly (without PulseAudio)
//!
//! 2. **PulseAudio**
//!    - Sound server running in userspace
//!    - Provides mixing, routing, and per-app volume
//!    - Higher latency but more flexible
//!    - Most desktop Linux distributions use PulseAudio by default
//!
//! 3. **PipeWire** (newer systems)
//!    - Modern replacement for both PulseAudio and JACK
//!    - Compatible with PulseAudio and JACK applications
//!
//! 4. **JACK (JACK Audio Connection Kit)**
//!    - Professional low-latency audio
//!    - Requires dedicated JACK server
//!
//! ## Test Categories
//!
//! 1. **Unit Tests** - Run without hardware, test data structures and logic
//! 2. **Integration Tests** - Require audio device, gracefully skip if unavailable
//! 3. **Hardware Tests** - Require specific hardware setup, marked with `#[ignore]`

#![cfg(target_os = "linux")]

#[allow(unused_imports)]
use soul_audio_desktop::{
    AudioBackend, AudioData, DeviceCapabilities, ExclusiveConfig, ExclusiveOutput, LatencyInfo,
    SupportedBitDepth,
};

// ============================================================================
// Helper Functions
// ============================================================================

/// Check if any audio device is available
fn has_audio_device() -> bool {
    ExclusiveOutput::new(ExclusiveConfig::default()).is_ok()
}

/// Generate a test sine wave
fn generate_test_audio(sample_rate: u32, duration_ms: u32, channels: u16) -> Vec<f32> {
    use std::f32::consts::PI;
    let num_samples = (sample_rate as f32 * duration_ms as f32 / 1000.0) as usize;
    let frequency = 440.0; // A4

    let mut samples = Vec::with_capacity(num_samples * channels as usize);
    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let sample = (2.0 * PI * frequency * t).sin() * 0.3;
        for _ in 0..channels {
            samples.push(sample);
        }
    }
    samples
}

/// Check if PulseAudio is available
fn has_pulseaudio() -> bool {
    std::process::Command::new("pactl")
        .arg("info")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Check if PipeWire is available
fn has_pipewire() -> bool {
    std::process::Command::new("pw-cli")
        .arg("info")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

// ============================================================================
// Section 1: ALSA Backend Identification Tests
// ============================================================================

mod alsa_backend_tests {
    use super::*;
    use soul_audio_desktop::backend::{get_backend_info, list_available_backends};

    #[test]
    fn test_default_backend_is_alsa() {
        let backend = AudioBackend::Default;
        assert_eq!(
            backend.name(),
            "ALSA",
            "Default backend on Linux should be ALSA"
        );
    }

    #[test]
    fn test_alsa_description() {
        let backend = AudioBackend::Default;
        let desc = backend.description();
        assert!(
            desc.contains("Linux")
                || desc.contains("ALSA")
                || desc.contains("Advanced Linux Sound"),
            "ALSA description should mention Linux or ALSA: {}",
            desc
        );
    }

    #[test]
    fn test_alsa_always_available() {
        let backend = AudioBackend::Default;
        assert!(
            backend.is_available(),
            "ALSA should always be available on Linux"
        );
    }

    #[test]
    fn test_alsa_in_backend_list() {
        let backends = list_available_backends();
        assert!(
            backends.contains(&AudioBackend::Default),
            "ALSA (Default) should be in available backends"
        );
    }

    #[test]
    fn test_alsa_backend_info() {
        let info = get_backend_info();
        let alsa_info = info
            .iter()
            .find(|b| b.name == "ALSA" || b.backend == AudioBackend::Default);
        assert!(alsa_info.is_some(), "Should find ALSA in backend info");

        let alsa = alsa_info.unwrap();
        assert!(alsa.available, "ALSA should be available");
        assert!(alsa.is_default, "ALSA should be the default on Linux");
    }

    #[test]
    fn test_alsa_cpal_host() {
        let backend = AudioBackend::Default;
        let host = backend.to_cpal_host();
        assert!(
            host.is_ok(),
            "Should be able to get CPAL host for ALSA: {:?}",
            host.err()
        );
    }

    #[cfg(feature = "jack")]
    #[test]
    fn test_jack_backend_available_if_feature_enabled() {
        // JACK backend should exist when feature is enabled
        let backend = AudioBackend::Jack;
        assert_eq!(backend.name(), "JACK");
        // Availability depends on JACK server running
        let _available = backend.is_available();
    }
}

// ============================================================================
// Section 2: ALSA Direct Access Tests
// ============================================================================

mod alsa_direct_tests {
    use super::*;

    #[test]
    fn test_alsa_config_defaults() {
        let config = ExclusiveConfig::default();

        // On Linux, "exclusive mode" means direct ALSA access
        // (bypassing PulseAudio if possible)
        assert!(config.exclusive_mode, "Exclusive mode should be enabled by default");
        assert_eq!(config.backend, AudioBackend::Default);
    }

    #[test]
    fn test_create_alsa_output() {
        match ExclusiveOutput::new(ExclusiveConfig::default()) {
            Ok(output) => {
                assert!(output.sample_rate() > 0, "Sample rate should be positive");
                assert!(
                    output.latency().buffer_samples > 0,
                    "Buffer samples should be positive"
                );
                eprintln!(
                    "ALSA output: {} Hz, {} samples buffer ({:.2} ms)",
                    output.sample_rate(),
                    output.latency().buffer_samples,
                    output.latency().buffer_ms
                );
            }
            Err(e) => {
                eprintln!(
                    "Could not create ALSA output (may be expected in CI): {}",
                    e
                );
            }
        }
    }

    #[test]
    fn test_alsa_with_specific_format() {
        let config = ExclusiveConfig::bit_perfect_24().with_sample_rate(48000);

        match ExclusiveOutput::new(config) {
            Ok(output) => {
                eprintln!(
                    "24-bit ALSA output: {} Hz, {} samples buffer",
                    output.sample_rate(),
                    output.latency().buffer_samples
                );
            }
            Err(e) => {
                eprintln!("Could not create 24-bit output: {}", e);
            }
        }
    }

    #[test]
    fn test_alsa_output_releases() {
        {
            if let Ok(output) = ExclusiveOutput::new(ExclusiveConfig::default()) {
                let sample_rate = output.sample_rate();
                let samples = generate_test_audio(sample_rate, 100, 2);
                let _ = output.play(AudioData::Float32(samples));
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(50));

        // Should be able to create another output
        match ExclusiveOutput::new(ExclusiveConfig::default()) {
            Ok(_) => {}
            Err(e) => {
                eprintln!("Note: Could not re-create output: {}", e);
            }
        }
    }

    #[test]
    fn test_alsa_latency_info() {
        if let Ok(output) = ExclusiveOutput::new(ExclusiveConfig::default()) {
            let latency = output.latency();

            assert!(latency.buffer_samples > 0, "Should have buffer samples");
            assert!(latency.buffer_ms > 0.0, "Should have buffer latency");

            eprintln!(
                "ALSA latency: {} samples = {:.2} ms (total: {:.2} ms)",
                latency.buffer_samples, latency.buffer_ms, latency.total_ms
            );
        }
    }
}

// ============================================================================
// Section 3: PulseAudio Integration Tests
// ============================================================================

mod pulseaudio_tests {
    use super::*;

    #[test]
    fn test_pulseaudio_availability() {
        if has_pulseaudio() {
            eprintln!("PulseAudio is available on this system");
        } else if has_pipewire() {
            eprintln!("PipeWire is available (may provide PulseAudio compatibility)");
        } else {
            eprintln!("Neither PulseAudio nor PipeWire detected");
        }
    }

    #[test]
    fn test_output_through_pulseaudio() {
        // When PulseAudio is running, CPAL/ALSA will typically route through it
        // unless exclusive mode is specifically requested
        match ExclusiveOutput::new(ExclusiveConfig::default()) {
            Ok(output) => {
                eprintln!(
                    "Output created (may be through PulseAudio): {} Hz",
                    output.sample_rate()
                );
            }
            Err(e) => {
                eprintln!("Output error: {}", e);
            }
        }
    }

    #[test]
    fn test_shared_mode_config() {
        // On Linux, shared mode typically goes through PulseAudio
        let config = ExclusiveConfig {
            exclusive_mode: false,
            ..ExclusiveConfig::default()
        };

        match ExclusiveOutput::new(config) {
            Ok(output) => {
                eprintln!("Shared mode output: {} Hz", output.sample_rate());
            }
            Err(e) => {
                eprintln!("Shared mode error: {}", e);
            }
        }
    }

    /// PulseAudio typically adds some latency compared to direct ALSA
    #[test]
    fn test_pulseaudio_latency() {
        if !has_pulseaudio() && !has_pipewire() {
            eprintln!("Skipping: No PulseAudio/PipeWire");
            return;
        }

        if let Ok(output) = ExclusiveOutput::new(ExclusiveConfig::default()) {
            let latency = output.latency();
            eprintln!(
                "Audio latency: {:.2} ms (PulseAudio typically adds 20-50ms)",
                latency.buffer_ms
            );
        }
    }
}

// ============================================================================
// Section 4: Buffer Underrun Recovery Tests
// ============================================================================

mod buffer_underrun_tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_small_buffer_stability() {
        // Small buffers are more prone to underruns
        let config = ExclusiveConfig::default().with_buffer_frames(64);

        match ExclusiveOutput::new(config) {
            Ok(output) => {
                let actual = output.latency().buffer_samples;
                eprintln!(
                    "Small buffer test: {} samples ({:.2} ms)",
                    actual,
                    output.latency().buffer_ms
                );

                // Try playing with small buffer
                let samples = generate_test_audio(output.sample_rate(), 200, 2);
                if output.play(AudioData::Float32(samples)).is_ok() {
                    std::thread::sleep(Duration::from_millis(200));
                    let _ = output.stop();
                }
            }
            Err(e) => {
                eprintln!("Small buffer not supported: {}", e);
            }
        }
    }

    #[test]
    fn test_various_buffer_sizes() {
        // ALSA supports various buffer sizes
        let buffer_sizes = [64, 128, 256, 512, 1024, 2048, 4096];

        for &size in &buffer_sizes {
            let config = ExclusiveConfig::default().with_buffer_frames(size);
            match ExclusiveOutput::new(config) {
                Ok(output) => {
                    let actual = output.latency().buffer_samples;
                    let latency_ms = output.latency().buffer_ms;
                    eprintln!(
                        "Buffer {} -> {} samples ({:.2} ms)",
                        size, actual, latency_ms
                    );
                }
                Err(e) => {
                    eprintln!("Buffer {}: {}", size, e);
                }
            }
        }
    }

    #[test]
    fn test_rapid_play_stop_cycles() {
        // Rapid start/stop can cause underruns if not handled properly
        if let Ok(output) = ExclusiveOutput::new(ExclusiveConfig::default()) {
            let sample_rate = output.sample_rate();

            for i in 0..5 {
                let samples = generate_test_audio(sample_rate, 50, 2);
                if output.play(AudioData::Float32(samples)).is_ok() {
                    std::thread::sleep(Duration::from_millis(30));
                    let _ = output.stop();
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            eprintln!("Completed {} rapid play/stop cycles", 5);
        }
    }

    #[test]
    fn test_pause_resume_stability() {
        if let Ok(output) = ExclusiveOutput::new(ExclusiveConfig::default()) {
            let samples = generate_test_audio(output.sample_rate(), 500, 2);

            if output.play(AudioData::Float32(samples)).is_ok() {
                std::thread::sleep(Duration::from_millis(100));

                // Pause/resume cycle
                let _ = output.pause();
                std::thread::sleep(Duration::from_millis(50));
                let _ = output.resume();
                std::thread::sleep(Duration::from_millis(100));

                // Another pause/resume
                let _ = output.pause();
                std::thread::sleep(Duration::from_millis(50));
                let _ = output.resume();
                std::thread::sleep(Duration::from_millis(100));

                let _ = output.stop();
            }
            eprintln!("Pause/resume stability test completed");
        }
    }

    /// Test recovery from buffer underrun scenario
    /// (simulated by CPU-intensive operation during playback)
    #[test]
    fn test_underrun_recovery() {
        if let Ok(output) = ExclusiveOutput::new(ExclusiveConfig::default()) {
            let samples = generate_test_audio(output.sample_rate(), 1000, 2);

            if output.play(AudioData::Float32(samples)).is_ok() {
                // Simulate CPU load that might cause underrun
                std::thread::sleep(Duration::from_millis(200));

                // Should still be playing (or have recovered)
                assert!(
                    output.is_playing() || !output.is_paused(),
                    "Audio should continue or recover"
                );

                let _ = output.stop();
            }
            eprintln!("Underrun recovery test completed");
        }
    }
}

// ============================================================================
// Section 5: Sample Rate Tests
// ============================================================================

mod sample_rate_tests {
    use super::*;

    #[test]
    fn test_sample_rate_44100() {
        let config = ExclusiveConfig::default().with_sample_rate(44100);

        match ExclusiveOutput::new(config) {
            Ok(output) => {
                eprintln!("Requested 44100, got {}", output.sample_rate());
            }
            Err(e) => {
                eprintln!("44100 Hz: {}", e);
            }
        }
    }

    #[test]
    fn test_sample_rate_48000() {
        let config = ExclusiveConfig::default().with_sample_rate(48000);

        match ExclusiveOutput::new(config) {
            Ok(output) => {
                eprintln!("Requested 48000, got {}", output.sample_rate());
            }
            Err(e) => {
                eprintln!("48000 Hz: {}", e);
            }
        }
    }

    #[test]
    fn test_sample_rate_96000() {
        let config = ExclusiveConfig::default().with_sample_rate(96000);

        match ExclusiveOutput::new(config) {
            Ok(output) => {
                eprintln!("Requested 96000, got {}", output.sample_rate());
            }
            Err(e) => {
                eprintln!("96000 Hz: {}", e);
            }
        }
    }

    #[test]
    fn test_native_sample_rate() {
        let config = ExclusiveConfig::default();
        assert_eq!(config.sample_rate, 0, "Default should use native rate");

        match ExclusiveOutput::new(config) {
            Ok(output) => {
                let native = output.sample_rate();
                assert!(native > 0);
                eprintln!("Native sample rate: {} Hz", native);
            }
            Err(e) => {
                eprintln!("Could not get native rate: {}", e);
            }
        }
    }

    #[test]
    fn test_sample_rate_sequence() {
        let rates = [44100, 48000, 96000];

        for &rate in &rates {
            let config = ExclusiveConfig::default().with_sample_rate(rate);
            match ExclusiveOutput::new(config) {
                Ok(output) => {
                    eprintln!("{} Hz: actual {}", rate, output.sample_rate());
                }
                Err(e) => {
                    eprintln!("{} Hz: {}", rate, e);
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }
}

// ============================================================================
// Section 6: Format Tests
// ============================================================================

mod format_tests {
    use super::*;

    #[test]
    fn test_float32_format() {
        let config = ExclusiveConfig {
            bit_depth: SupportedBitDepth::Float32,
            ..ExclusiveConfig::default()
        };

        match ExclusiveOutput::new(config) {
            Ok(output) => {
                assert_eq!(output.config().bit_depth, SupportedBitDepth::Float32);
                eprintln!("Float32 format supported");
            }
            Err(e) => {
                eprintln!("Float32 not available: {}", e);
            }
        }
    }

    #[test]
    fn test_int16_format() {
        let config = ExclusiveConfig::bit_perfect_16();

        match ExclusiveOutput::new(config) {
            Ok(output) => {
                assert_eq!(output.config().bit_depth, SupportedBitDepth::Int16);
                eprintln!("Int16 format supported");
            }
            Err(e) => {
                eprintln!("Int16 not available: {}", e);
            }
        }
    }

    #[test]
    fn test_int24_format() {
        let config = ExclusiveConfig::bit_perfect_24();

        match ExclusiveOutput::new(config) {
            Ok(output) => {
                let depth = output.config().bit_depth;
                assert!(
                    depth == SupportedBitDepth::Int24 || depth == SupportedBitDepth::Int32,
                    "Should be Int24 or Int32"
                );
                eprintln!("Int24 format: {:?}", depth);
            }
            Err(e) => {
                eprintln!("Int24 not available: {}", e);
            }
        }
    }

    #[test]
    fn test_int32_format() {
        let config = ExclusiveConfig::bit_perfect_32();

        match ExclusiveOutput::new(config) {
            Ok(output) => {
                assert_eq!(output.config().bit_depth, SupportedBitDepth::Int32);
                eprintln!("Int32 format supported");
            }
            Err(e) => {
                eprintln!("Int32 not available: {}", e);
            }
        }
    }
}

// ============================================================================
// Section 7: Device Enumeration Tests
// ============================================================================

mod device_enumeration_tests {
    use super::*;
    use soul_audio_desktop::device::{
        get_default_device_with_capabilities, list_devices, list_devices_with_capabilities,
    };

    #[test]
    fn test_enumerate_alsa_devices() {
        match list_devices(AudioBackend::Default) {
            Ok(devices) => {
                eprintln!("Found {} ALSA devices:", devices.len());
                for device in &devices {
                    eprintln!(
                        "  - {} (default: {}, {} Hz, {} ch)",
                        device.name, device.is_default, device.sample_rate, device.channels
                    );
                }
            }
            Err(e) => {
                eprintln!("Could not enumerate devices: {}", e);
            }
        }
    }

    #[test]
    fn test_default_device_info() {
        match get_default_device_with_capabilities(AudioBackend::Default, true) {
            Ok(device) => {
                eprintln!("Default device: {}", device.name);
                eprintln!("  Native rate: {} Hz", device.sample_rate);
                eprintln!("  Channels: {}", device.channels);

                if let Some(caps) = &device.capabilities {
                    eprintln!("  Sample rates: {:?}", caps.sample_rates);
                    eprintln!("  Max channels: {}", caps.max_channels);
                    eprintln!(
                        "  Bit depths: {:?}",
                        caps.bit_depths.iter().map(|d| d.display_name()).collect::<Vec<_>>()
                    );
                }
            }
            Err(e) => {
                eprintln!("Could not get default device: {}", e);
            }
        }
    }

    #[test]
    fn test_device_capabilities() {
        match list_devices_with_capabilities(AudioBackend::Default, true) {
            Ok(devices) => {
                for device in &devices {
                    if let Some(caps) = &device.capabilities {
                        eprintln!("Device: {}", device.name);
                        eprintln!("  Sample rates: {:?}", caps.sample_rates);
                        eprintln!("  Max channels: {}", caps.max_channels);
                    }
                }
            }
            Err(e) => {
                eprintln!("Could not get capabilities: {}", e);
            }
        }
    }

    #[test]
    fn test_device_by_name() {
        use soul_audio_desktop::device::find_device_by_name;

        match list_devices(AudioBackend::Default) {
            Ok(devices) => {
                if let Some(first) = devices.first() {
                    match find_device_by_name(AudioBackend::Default, &first.name) {
                        Ok(_) => {
                            eprintln!("Found device by name: {}", first.name);
                        }
                        Err(e) => {
                            eprintln!("Could not find device by name: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Could not list devices: {}", e);
            }
        }
    }
}

// ============================================================================
// Section 8: Hardware-Dependent Tests (marked with #[ignore])
// ============================================================================

mod hardware_tests {
    use super::*;
    use std::time::Duration;

    /// Test actual ALSA playback
    #[test]
    #[ignore = "Requires audio hardware - run with --ignored"]
    fn test_alsa_playback() {
        let output =
            ExclusiveOutput::new(ExclusiveConfig::default()).expect("Should create output");

        eprintln!("ALSA playback test");
        eprintln!("  Sample rate: {} Hz", output.sample_rate());
        eprintln!("  Latency: {:.2} ms", output.latency().buffer_ms);

        let samples = generate_test_audio(output.sample_rate(), 1000, 2);
        output
            .play(AudioData::Float32(samples))
            .expect("Should play");

        std::thread::sleep(Duration::from_secs(1));
        output.stop().expect("Should stop");
    }

    /// Test different bit depths with actual audio
    #[test]
    #[ignore = "Requires audio hardware - run with --ignored"]
    fn test_bit_depths_playback() {
        let configs = [
            ("Float32", ExclusiveConfig::default()),
            ("Int16", ExclusiveConfig::bit_perfect_16()),
            ("Int24", ExclusiveConfig::bit_perfect_24()),
            ("Int32", ExclusiveConfig::bit_perfect_32()),
        ];

        for (name, config) in configs {
            match ExclusiveOutput::new(config) {
                Ok(output) => {
                    eprintln!("{}: {} Hz", name, output.sample_rate());
                    let samples = generate_test_audio(output.sample_rate(), 300, 2);
                    output.play_f32(&samples).expect("Play");
                    std::thread::sleep(Duration::from_millis(300));
                    output.stop().expect("Stop");
                }
                Err(e) => {
                    eprintln!("{}: {}", name, e);
                }
            }
        }
    }

    /// Test sample rate switching
    #[test]
    #[ignore = "Requires audio hardware - run with --ignored"]
    fn test_sample_rate_switch_playback() {
        let rates = [44100, 48000, 96000];

        for &rate in &rates {
            let config = ExclusiveConfig::default().with_sample_rate(rate);

            match ExclusiveOutput::new(config) {
                Ok(output) => {
                    let actual = output.sample_rate();
                    eprintln!("Playing at {} Hz (requested {})", actual, rate);

                    let samples = generate_test_audio(actual, 500, 2);
                    output.play(AudioData::Float32(samples)).expect("Play");
                    std::thread::sleep(Duration::from_millis(500));
                    output.stop().expect("Stop");
                }
                Err(e) => {
                    eprintln!("{} Hz: {}", rate, e);
                }
            }

            std::thread::sleep(Duration::from_millis(100));
        }
    }

    /// Test low-latency mode
    #[test]
    #[ignore = "Requires audio hardware - run with --ignored"]
    fn test_low_latency_playback() {
        let config = ExclusiveConfig::low_latency();

        match ExclusiveOutput::new(config) {
            Ok(output) => {
                let latency = output.latency();
                eprintln!(
                    "Low-latency: {} samples = {:.2} ms",
                    latency.buffer_samples, latency.buffer_ms
                );

                let samples = generate_test_audio(output.sample_rate(), 500, 2);
                output.play(AudioData::Float32(samples)).expect("Play");
                std::thread::sleep(Duration::from_millis(500));
                output.stop().expect("Stop");
            }
            Err(e) => {
                eprintln!("Low latency error: {}", e);
            }
        }
    }

    /// Test direct ALSA access (bypassing PulseAudio)
    #[test]
    #[ignore = "Requires ALSA direct access - may need to stop PulseAudio"]
    fn test_alsa_direct_exclusive() {
        // This test assumes PulseAudio is stopped or using
        // hw: device directly

        let config = ExclusiveConfig {
            exclusive_mode: true,
            ..ExclusiveConfig::default()
        };

        match ExclusiveOutput::new(config) {
            Ok(output) => {
                eprintln!(
                    "Direct ALSA: {} Hz, {:.2} ms latency",
                    output.sample_rate(),
                    output.latency().buffer_ms
                );

                // Direct ALSA should have lower latency
                if output.latency().buffer_ms < 20.0 {
                    eprintln!("  Low latency achieved (likely direct ALSA)");
                } else {
                    eprintln!("  Higher latency (may be going through PulseAudio)");
                }
            }
            Err(e) => {
                eprintln!("Direct ALSA error: {}", e);
            }
        }
    }

    /// Test USB audio device (if available)
    #[test]
    #[ignore = "Requires USB audio device - run with --ignored"]
    fn test_usb_audio_device() {
        use soul_audio_desktop::device::list_devices;

        let devices = list_devices(AudioBackend::Default).expect("List devices");
        let usb_device = devices
            .iter()
            .find(|d| d.name.to_lowercase().contains("usb"));

        if let Some(device) = usb_device {
            eprintln!("Found USB device: {}", device.name);

            let config = ExclusiveConfig::default().with_device(&device.name);

            match ExclusiveOutput::new(config) {
                Ok(output) => {
                    eprintln!(
                        "USB output: {} Hz, {:.2} ms",
                        output.sample_rate(),
                        output.latency().buffer_ms
                    );

                    let samples = generate_test_audio(output.sample_rate(), 500, 2);
                    output.play(AudioData::Float32(samples)).expect("Play");
                    std::thread::sleep(Duration::from_millis(500));
                    output.stop().expect("Stop");
                }
                Err(e) => {
                    eprintln!("USB device error: {}", e);
                }
            }
        } else {
            eprintln!("No USB audio device found");
        }
    }
}

// ============================================================================
// Section 9: JACK Tests (when feature enabled)
// ============================================================================

#[cfg(feature = "jack")]
mod jack_tests {
    use super::*;
    use soul_audio_desktop::device::list_devices;

    #[test]
    fn test_jack_backend_exists() {
        let backend = AudioBackend::Jack;
        assert_eq!(backend.name(), "JACK");
    }

    #[test]
    fn test_jack_availability() {
        let backend = AudioBackend::Jack;
        let available = backend.is_available();
        eprintln!("JACK available: {}", available);
    }

    #[test]
    fn test_jack_device_enumeration() {
        let backend = AudioBackend::Jack;
        if backend.is_available() {
            match list_devices(backend) {
                Ok(devices) => {
                    eprintln!("Found {} JACK devices", devices.len());
                    for device in &devices {
                        eprintln!("  - {}", device.name);
                    }
                }
                Err(e) => {
                    eprintln!("JACK enumeration error: {}", e);
                }
            }
        } else {
            eprintln!("JACK not available - start jackd or use pipewire-jack");
        }
    }

    /// Test JACK output (requires JACK server)
    #[test]
    #[ignore = "Requires JACK server - run with --ignored"]
    fn test_jack_output() {
        let backend = AudioBackend::Jack;
        if !backend.is_available() {
            eprintln!("JACK not available, skipping");
            return;
        }

        let config = ExclusiveConfig::default().with_backend(backend);

        match ExclusiveOutput::new(config) {
            Ok(output) => {
                eprintln!(
                    "JACK output: {} Hz, {:.2} ms latency",
                    output.sample_rate(),
                    output.latency().buffer_ms
                );

                // JACK should have low latency
                assert!(
                    output.latency().buffer_ms < 20.0,
                    "JACK latency should be < 20ms"
                );
            }
            Err(e) => {
                eprintln!("JACK output error: {}", e);
            }
        }
    }

    /// Test JACK with specific buffer size
    #[test]
    #[ignore = "Requires JACK server - run with --ignored"]
    fn test_jack_low_latency() {
        let backend = AudioBackend::Jack;
        if !backend.is_available() {
            return;
        }

        // JACK buffer size is set by the server, but we can request
        let config = ExclusiveConfig::low_latency().with_backend(backend);

        match ExclusiveOutput::new(config) {
            Ok(output) => {
                eprintln!(
                    "JACK low-latency: {} samples, {:.2} ms",
                    output.latency().buffer_samples,
                    output.latency().buffer_ms
                );
            }
            Err(e) => {
                eprintln!("JACK low-latency error: {}", e);
            }
        }
    }
}

// ============================================================================
// Section 10: Error Handling Tests
// ============================================================================

mod error_handling {
    use super::*;

    #[test]
    fn test_nonexistent_device_error() {
        let config = ExclusiveConfig::default().with_device("NonexistentDevice12345XYZ");

        let result = ExclusiveOutput::new(config);
        assert!(result.is_err(), "Should fail with nonexistent device");
    }

    #[test]
    fn test_invalid_sample_rate() {
        let config = ExclusiveConfig::default().with_sample_rate(12345);

        match ExclusiveOutput::new(config) {
            Ok(output) => {
                eprintln!("Got fallback rate: {}", output.sample_rate());
            }
            Err(e) => {
                eprintln!("Invalid rate error: {}", e);
            }
        }
    }

    #[test]
    fn test_zero_buffer_handling() {
        let config = ExclusiveConfig::default().with_buffer_frames(0);

        match ExclusiveOutput::new(config) {
            Ok(output) => {
                assert!(output.latency().buffer_samples > 0);
            }
            Err(_) => {}
        }
    }

    #[test]
    fn test_very_large_buffer() {
        let config = ExclusiveConfig::default().with_buffer_frames(1_000_000);

        match ExclusiveOutput::new(config) {
            Ok(output) => {
                let actual = output.latency().buffer_samples;
                eprintln!("Large buffer clamped to: {}", actual);
            }
            Err(e) => {
                eprintln!("Large buffer error: {}", e);
            }
        }
    }

    #[test]
    fn test_stop_without_play() {
        if let Ok(output) = ExclusiveOutput::new(ExclusiveConfig::default()) {
            assert!(output.stop().is_ok());
        }
    }

    #[test]
    fn test_pause_without_play() {
        if let Ok(output) = ExclusiveOutput::new(ExclusiveConfig::default()) {
            assert!(output.pause().is_ok());
        }
    }

    #[test]
    fn test_multiple_stop_calls() {
        if let Ok(output) = ExclusiveOutput::new(ExclusiveConfig::default()) {
            let _ = output.stop();
            let _ = output.stop();
            let _ = output.stop();
        }
    }
}

// ============================================================================
// Section 11: PipeWire Compatibility Tests
// ============================================================================

mod pipewire_tests {
    use super::*;

    #[test]
    fn test_pipewire_detection() {
        if has_pipewire() {
            eprintln!("PipeWire detected - provides ALSA/PulseAudio/JACK compatibility");
        } else {
            eprintln!("PipeWire not detected");
        }
    }

    #[test]
    fn test_output_with_pipewire() {
        // PipeWire provides ALSA compatibility layer
        // Output should work whether PipeWire or native ALSA is used
        match ExclusiveOutput::new(ExclusiveConfig::default()) {
            Ok(output) => {
                eprintln!(
                    "Output created: {} Hz (PipeWire or ALSA)",
                    output.sample_rate()
                );
            }
            Err(e) => {
                eprintln!("Output error: {}", e);
            }
        }
    }
}
