//! macOS-specific audio tests for CoreAudio
//!
//! These tests cover macOS-specific audio functionality:
//! - Audio unit configuration
//! - Aggregate device handling
//! - Sample rate switching
//!
//! ## Running Hardware-Dependent Tests
//!
//! Hardware-dependent tests are marked with `#[ignore]` by default.
//! To run them, use:
//!
//! ```bash
//! cargo test -p soul-audio-desktop platform_macos_test -- --ignored
//! ```
//!
//! Or to run all tests including ignored:
//!
//! ```bash
//! cargo test -p soul-audio-desktop platform_macos_test -- --include-ignored
//! ```
//!
//! ## Requirements for Hardware Tests
//!
//! - A physical audio output device must be connected (built-in speakers work)
//! - For aggregate device tests, you need Audio MIDI Setup configured
//! - Some tests may require adjusting System Preferences > Sound
//!
//! ## CoreAudio Architecture Notes
//!
//! CoreAudio on macOS operates differently from WASAPI on Windows:
//! - All audio goes through the Core Audio HAL (Hardware Abstraction Layer)
//! - "Exclusive mode" is achieved via kAudioDevicePropertyHogMode
//! - Sample rate changes affect the entire system (not per-stream)
//! - Aggregate devices combine multiple interfaces into one
//!
//! ## Test Categories
//!
//! 1. **Unit Tests** - Run without hardware, test data structures and logic
//! 2. **Integration Tests** - Require audio device, gracefully skip if unavailable
//! 3. **Hardware Tests** - Require specific hardware setup, marked with `#[ignore]`

#![cfg(target_os = "macos")]

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

// ============================================================================
// Section 1: CoreAudio Backend Identification Tests
// ============================================================================

mod coreaudio_backend_tests {
    use super::*;
    use soul_audio_desktop::backend::{get_backend_info, list_available_backends};

    #[test]
    fn test_default_backend_is_coreaudio() {
        let backend = AudioBackend::Default;
        assert_eq!(
            backend.name(),
            "CoreAudio",
            "Default backend on macOS should be CoreAudio"
        );
    }

    #[test]
    fn test_coreaudio_description() {
        let backend = AudioBackend::Default;
        let desc = backend.description();
        assert!(
            desc.contains("macOS") || desc.contains("Core Audio") || desc.contains("native"),
            "CoreAudio description should mention macOS or Core Audio: {}",
            desc
        );
    }

    #[test]
    fn test_coreaudio_always_available() {
        let backend = AudioBackend::Default;
        assert!(
            backend.is_available(),
            "CoreAudio should always be available on macOS"
        );
    }

    #[test]
    fn test_coreaudio_in_backend_list() {
        let backends = list_available_backends();
        assert!(
            backends.contains(&AudioBackend::Default),
            "CoreAudio (Default) should be in available backends"
        );
    }

    #[test]
    fn test_coreaudio_backend_info() {
        let info = get_backend_info();
        let coreaudio_info = info
            .iter()
            .find(|b| b.name == "CoreAudio" || b.backend == AudioBackend::Default);
        assert!(
            coreaudio_info.is_some(),
            "Should find CoreAudio in backend info"
        );

        let coreaudio = coreaudio_info.unwrap();
        assert!(coreaudio.available, "CoreAudio should be available");
        assert!(coreaudio.is_default, "CoreAudio should be the default on macOS");
    }

    #[test]
    fn test_coreaudio_cpal_host() {
        let backend = AudioBackend::Default;
        let host = backend.to_cpal_host();
        assert!(
            host.is_ok(),
            "Should be able to get CPAL host for CoreAudio: {:?}",
            host.err()
        );
    }

    #[cfg(feature = "jack")]
    #[test]
    fn test_jack_backend_on_macos() {
        // JACK can be installed on macOS for pro audio routing
        let backend = AudioBackend::Jack;
        assert_eq!(backend.name(), "JACK");
        // Availability depends on JACK server being installed/running
        let _available = backend.is_available();
    }
}

// ============================================================================
// Section 2: Audio Unit Configuration Tests
// ============================================================================

mod audio_unit_config_tests {
    use super::*;

    #[test]
    fn test_default_audio_unit_config() {
        // On macOS, the "exclusive" config creates an audio unit with specific settings
        let config = ExclusiveConfig::default();

        // Default should work with CoreAudio
        assert_eq!(config.backend, AudioBackend::Default);
    }

    #[test]
    fn test_bit_depth_configs() {
        // CoreAudio typically works with 32-bit float internally
        // but can output various formats to hardware

        let float32 = ExclusiveConfig {
            bit_depth: SupportedBitDepth::Float32,
            ..ExclusiveConfig::default()
        };
        assert_eq!(float32.bit_depth, SupportedBitDepth::Float32);

        let int16 = ExclusiveConfig::bit_perfect_16();
        assert_eq!(int16.bit_depth, SupportedBitDepth::Int16);

        let int24 = ExclusiveConfig::bit_perfect_24();
        assert_eq!(int24.bit_depth, SupportedBitDepth::Int24);
    }

    #[test]
    fn test_buffer_size_configs() {
        // CoreAudio supports various buffer sizes
        // Common sizes: 128, 256, 512, 1024, 2048

        let low_latency = ExclusiveConfig::low_latency();
        assert_eq!(
            low_latency.buffer_frames,
            Some(128),
            "Low latency should use 128 frames"
        );

        let ultra_low = ExclusiveConfig::ultra_low_latency();
        assert_eq!(
            ultra_low.buffer_frames,
            Some(64),
            "Ultra-low latency should use 64 frames"
        );
    }

    #[test]
    fn test_sample_rate_configs() {
        // Common macOS sample rates: 44100, 48000, 88200, 96000
        let rates = [44100, 48000, 88200, 96000, 176400, 192000];

        for &rate in &rates {
            let config = ExclusiveConfig::default().with_sample_rate(rate);
            assert_eq!(config.sample_rate, rate);
        }
    }
}

// ============================================================================
// Section 3: Audio Unit Integration Tests
// ============================================================================

mod audio_unit_integration {
    use super::*;

    #[test]
    fn test_create_audio_output() {
        match ExclusiveOutput::new(ExclusiveConfig::default()) {
            Ok(output) => {
                assert!(output.sample_rate() > 0, "Sample rate should be positive");
                assert!(
                    output.latency().buffer_samples > 0,
                    "Buffer samples should be positive"
                );
                eprintln!(
                    "CoreAudio output: {} Hz, {} samples buffer",
                    output.sample_rate(),
                    output.latency().buffer_samples
                );
            }
            Err(e) => {
                eprintln!(
                    "Could not create CoreAudio output (may be expected in CI): {}",
                    e
                );
            }
        }
    }

    #[test]
    fn test_audio_output_with_specific_format() {
        let config = ExclusiveConfig::bit_perfect_24().with_sample_rate(48000);

        match ExclusiveOutput::new(config) {
            Ok(output) => {
                eprintln!(
                    "24-bit output: {} Hz, {} samples buffer",
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
    fn test_output_releases_on_drop() {
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
    fn test_latency_info() {
        if let Ok(output) = ExclusiveOutput::new(ExclusiveConfig::default()) {
            let latency = output.latency();

            assert!(latency.buffer_samples > 0, "Should have buffer samples");
            assert!(latency.buffer_ms > 0.0, "Should have buffer latency");
            assert!(
                latency.total_ms >= latency.buffer_ms,
                "Total should include buffer"
            );

            eprintln!(
                "CoreAudio latency: {} samples = {:.2} ms (total: {:.2} ms)",
                latency.buffer_samples, latency.buffer_ms, latency.total_ms
            );
        }
    }
}

// ============================================================================
// Section 4: Sample Rate Switching Tests
// ============================================================================

mod sample_rate_switching {
    use super::*;

    #[test]
    fn test_sample_rate_44100() {
        let config = ExclusiveConfig::default().with_sample_rate(44100);

        match ExclusiveOutput::new(config) {
            Ok(output) => {
                eprintln!("Requested 44100, got {}", output.sample_rate());
                // macOS might round to nearest supported rate
            }
            Err(e) => {
                eprintln!("44100 Hz not available: {}", e);
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
                eprintln!("48000 Hz not available: {}", e);
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
                // High sample rates may not be supported on all devices
                eprintln!("96000 Hz not available: {}", e);
            }
        }
    }

    #[test]
    fn test_sample_rate_sequence() {
        // Test switching through different sample rates
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
            // Small delay between rate changes
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }

    #[test]
    fn test_native_sample_rate() {
        // With sample_rate = 0, should use device native rate
        let config = ExclusiveConfig::default();
        assert_eq!(config.sample_rate, 0, "Default should use native rate");

        match ExclusiveOutput::new(config) {
            Ok(output) => {
                let native_rate = output.sample_rate();
                assert!(native_rate > 0);
                eprintln!("Device native rate: {} Hz", native_rate);
            }
            Err(e) => {
                eprintln!("Could not get native rate: {}", e);
            }
        }
    }
}

// ============================================================================
// Section 5: Aggregate Device Tests
// ============================================================================

mod aggregate_device_tests {
    use super::*;
    use soul_audio_desktop::device::{list_devices, list_devices_with_capabilities};

    /// Note: Aggregate devices are created in Audio MIDI Setup.app
    /// These tests verify we can enumerate and use them if present.

    #[test]
    fn test_enumerate_all_devices() {
        match list_devices(AudioBackend::Default) {
            Ok(devices) => {
                eprintln!("Found {} CoreAudio devices:", devices.len());
                for device in &devices {
                    eprintln!(
                        "  - {} (default: {}, {} Hz, {} ch)",
                        device.name, device.is_default, device.sample_rate, device.channels
                    );

                    // Aggregate devices often have "Aggregate" in the name
                    if device.name.contains("Aggregate") {
                        eprintln!("    ^ This is an aggregate device");
                    }
                }
            }
            Err(e) => {
                eprintln!("Could not enumerate devices: {}", e);
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
                        eprintln!(
                            "  Bit depths: {:?}",
                            caps.bit_depths.iter().map(|d| d.display_name()).collect::<Vec<_>>()
                        );
                    }
                }
            }
            Err(e) => {
                eprintln!("Could not get device capabilities: {}", e);
            }
        }
    }

    #[test]
    fn test_multi_channel_device() {
        // Aggregate devices or pro interfaces may have many channels
        match list_devices_with_capabilities(AudioBackend::Default, true) {
            Ok(devices) => {
                for device in &devices {
                    if let Some(caps) = &device.capabilities {
                        if caps.max_channels > 2 {
                            eprintln!(
                                "Multi-channel device: {} ({} channels)",
                                device.name, caps.max_channels
                            );
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Could not check multi-channel: {}", e);
            }
        }
    }
}

// ============================================================================
// Section 6: Buffer Size Negotiation Tests
// ============================================================================

mod buffer_size_tests {
    use super::*;

    #[test]
    fn test_default_buffer_size() {
        if let Ok(output) = ExclusiveOutput::new(ExclusiveConfig::default()) {
            let buffer_samples = output.latency().buffer_samples;
            eprintln!("Default buffer size: {} samples", buffer_samples);
            // CoreAudio default is typically 512 or 1024 samples
            assert!(buffer_samples >= 64 && buffer_samples <= 8192);
        }
    }

    #[test]
    fn test_small_buffer() {
        let config = ExclusiveConfig::default().with_buffer_frames(64);

        match ExclusiveOutput::new(config) {
            Ok(output) => {
                let actual = output.latency().buffer_samples;
                eprintln!("Requested 64, got {} samples", actual);
                // CoreAudio may have minimum buffer size
            }
            Err(e) => {
                eprintln!("Small buffer error: {}", e);
            }
        }
    }

    #[test]
    fn test_large_buffer() {
        let config = ExclusiveConfig::default().with_buffer_frames(4096);

        match ExclusiveOutput::new(config) {
            Ok(output) => {
                let actual = output.latency().buffer_samples;
                eprintln!("Requested 4096, got {} samples", actual);
            }
            Err(e) => {
                eprintln!("Large buffer error: {}", e);
            }
        }
    }

    #[test]
    fn test_various_buffer_sizes() {
        // Common CoreAudio buffer sizes
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
    fn test_buffer_latency_relationship() {
        if let Ok(output) = ExclusiveOutput::new(ExclusiveConfig::default()) {
            let latency = output.latency();
            let sample_rate = output.sample_rate();

            // Verify: buffer_ms = buffer_samples / sample_rate * 1000
            let expected_ms = latency.buffer_samples as f32 / sample_rate as f32 * 1000.0;
            let diff = (latency.buffer_ms - expected_ms).abs();

            assert!(
                diff < 0.1,
                "Latency calculation mismatch: expected {}, got {}",
                expected_ms,
                latency.buffer_ms
            );
        }
    }
}

// ============================================================================
// Section 7: Hardware-Dependent Tests (marked with #[ignore])
// ============================================================================

mod hardware_tests {
    use super::*;
    use std::time::Duration;

    /// Test actual audio playback
    ///
    /// Requirements:
    /// - Audio output device connected
    /// - Speakers or headphones to verify
    #[test]
    #[ignore = "Requires audio hardware - run with --ignored"]
    fn test_coreaudio_playback() {
        let output =
            ExclusiveOutput::new(ExclusiveConfig::default()).expect("Should create output");

        eprintln!("CoreAudio playback test");
        eprintln!("  Sample rate: {} Hz", output.sample_rate());
        eprintln!("  Latency: {:.2} ms", output.latency().buffer_ms);

        let samples = generate_test_audio(output.sample_rate(), 1000, 2);
        output
            .play(AudioData::Float32(samples))
            .expect("Should play");

        std::thread::sleep(Duration::from_secs(1));
        output.stop().expect("Should stop");
    }

    /// Test sample rate switching with actual audio
    #[test]
    #[ignore = "Requires audio hardware - run with --ignored"]
    fn test_sample_rate_switch_with_playback() {
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
                    eprintln!("{} Hz not available: {}", rate, e);
                }
            }

            std::thread::sleep(Duration::from_millis(100));
        }
    }

    /// Test different bit depths
    #[test]
    #[ignore = "Requires audio hardware - run with --ignored"]
    fn test_bit_depths_with_playback() {
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

    /// Test aggregate device (requires setup in Audio MIDI Setup.app)
    #[test]
    #[ignore = "Requires aggregate device configured - run with --ignored"]
    fn test_aggregate_device_playback() {
        use soul_audio_desktop::device::list_devices;

        // Find an aggregate device
        let devices = list_devices(AudioBackend::Default).expect("List devices");
        let aggregate = devices.iter().find(|d| d.name.contains("Aggregate"));

        if let Some(device) = aggregate {
            eprintln!("Found aggregate device: {}", device.name);

            let config = ExclusiveConfig::default().with_device(&device.name);

            match ExclusiveOutput::new(config) {
                Ok(output) => {
                    eprintln!(
                        "Aggregate output: {} Hz, {} channels",
                        output.sample_rate(),
                        // Note: actual channel count from capabilities
                        2
                    );

                    let samples = generate_test_audio(output.sample_rate(), 500, 2);
                    output.play(AudioData::Float32(samples)).expect("Play");
                    std::thread::sleep(Duration::from_millis(500));
                    output.stop().expect("Stop");
                }
                Err(e) => {
                    eprintln!("Aggregate device error: {}", e);
                }
            }
        } else {
            eprintln!("No aggregate device found. Create one in Audio MIDI Setup.app");
        }
    }

    /// Test low-latency configuration
    #[test]
    #[ignore = "Requires audio hardware - run with --ignored"]
    fn test_low_latency_playback() {
        let config = ExclusiveConfig::low_latency();

        match ExclusiveOutput::new(config) {
            Ok(output) => {
                let latency = output.latency();
                eprintln!(
                    "Low-latency mode: {} samples = {:.2} ms",
                    latency.buffer_samples, latency.buffer_ms
                );

                // Low latency should be under 10ms
                assert!(
                    latency.buffer_ms < 10.0,
                    "Low latency should be < 10ms, got {}",
                    latency.buffer_ms
                );

                let samples = generate_test_audio(output.sample_rate(), 500, 2);
                output.play(AudioData::Float32(samples)).expect("Play");
                std::thread::sleep(Duration::from_millis(500));
                output.stop().expect("Stop");
            }
            Err(e) => {
                eprintln!("Low latency mode error: {}", e);
            }
        }
    }
}

// ============================================================================
// Section 8: CoreAudio-Specific Features
// ============================================================================

mod coreaudio_specific {
    use super::*;
    use soul_audio_desktop::device::get_default_device_with_capabilities;

    #[test]
    fn test_default_device_info() {
        match get_default_device_with_capabilities(AudioBackend::Default, true) {
            Ok(device) => {
                eprintln!("Default CoreAudio device: {}", device.name);
                eprintln!("  Native rate: {} Hz", device.sample_rate);
                eprintln!("  Channels: {}", device.channels);

                if let Some(caps) = &device.capabilities {
                    eprintln!("  Sample rates: {:?}", caps.sample_rates);
                    eprintln!("  Max channels: {}", caps.max_channels);
                    eprintln!(
                        "  Bit depths: {:?}",
                        caps.bit_depths.iter().map(|d| d.display_name()).collect::<Vec<_>>()
                    );
                    eprintln!("  DSD support: {}", caps.supports_dsd);
                }
            }
            Err(e) => {
                eprintln!("Could not get default device: {}", e);
            }
        }
    }

    #[test]
    fn test_device_sample_rate_support() {
        // macOS devices typically support multiple sample rates
        match get_default_device_with_capabilities(AudioBackend::Default, true) {
            Ok(device) => {
                if let Some(caps) = &device.capabilities {
                    // Most Mac audio devices support at least 44.1 and 48 kHz
                    let supports_44100 = caps.sample_rates.contains(&44100);
                    let supports_48000 = caps.sample_rates.contains(&48000);

                    eprintln!("44.1 kHz support: {}", supports_44100);
                    eprintln!("48 kHz support: {}", supports_48000);

                    // At least one standard rate should be supported
                    assert!(
                        supports_44100 || supports_48000,
                        "Device should support standard sample rates"
                    );
                }
            }
            Err(e) => {
                eprintln!("Could not check sample rate support: {}", e);
            }
        }
    }

    #[test]
    fn test_built_in_speakers() {
        // On MacBooks, there should be built-in speakers
        use soul_audio_desktop::device::list_devices;

        match list_devices(AudioBackend::Default) {
            Ok(devices) => {
                let builtin = devices.iter().find(|d| {
                    d.name.to_lowercase().contains("built-in")
                        || d.name.to_lowercase().contains("speaker")
                        || d.name.to_lowercase().contains("macbook")
                });

                if let Some(device) = builtin {
                    eprintln!("Found built-in output: {}", device.name);
                } else {
                    eprintln!("No built-in speakers found (external display or headphones only?)");
                }
            }
            Err(e) => {
                eprintln!("Could not enumerate devices: {}", e);
            }
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
        eprintln!("JACK available on macOS: {}", available);
        // JACK availability depends on JackServer being installed and running
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
            eprintln!("JACK not available - install JackServer for macOS");
        }
    }

    /// Test JACK output (requires JACK server running)
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
            }
            Err(e) => {
                eprintln!("JACK output error: {}", e);
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
