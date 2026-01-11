//! Windows-specific audio tests for WASAPI
//!
//! These tests cover Windows-specific audio functionality:
//! - WASAPI exclusive mode acquisition and release
//! - WASAPI shared mode fallback
//! - Buffer size negotiation
//! - Format negotiation (bit depth, channels)
//!
//! ## Running Hardware-Dependent Tests
//!
//! Hardware-dependent tests are marked with `#[ignore]` by default.
//! To run them, use:
//!
//! ```bash
//! cargo test -p soul-audio-desktop platform_windows_test -- --ignored
//! ```
//!
//! Or to run all tests including ignored:
//!
//! ```bash
//! cargo test -p soul-audio-desktop platform_windows_test -- --include-ignored
//! ```
//!
//! ## Requirements for Hardware Tests
//!
//! - A physical audio output device must be connected
//! - For exclusive mode tests, no other application should be using the device
//! - Administrator privileges may be required for some exclusive mode operations
//!
//! ## Test Categories
//!
//! 1. **Unit Tests** - Run without hardware, test data structures and logic
//! 2. **Integration Tests** - Require audio device, gracefully skip if unavailable
//! 3. **Hardware Tests** - Require specific hardware setup, marked with `#[ignore]`

#![cfg(target_os = "windows")]

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
// Section 1: WASAPI Backend Identification Tests
// ============================================================================

mod wasapi_backend_tests {
    use super::*;
    use soul_audio_desktop::backend::{get_backend_info, list_available_backends};

    #[test]
    fn test_default_backend_is_wasapi() {
        let backend = AudioBackend::Default;
        assert_eq!(
            backend.name(),
            "WASAPI",
            "Default backend on Windows should be WASAPI"
        );
    }

    #[test]
    fn test_wasapi_description_mentions_windows() {
        let backend = AudioBackend::Default;
        let desc = backend.description();
        assert!(
            desc.contains("Windows") || desc.contains("WASAPI") || desc.contains("shared"),
            "WASAPI description should mention Windows or WASAPI: {}",
            desc
        );
    }

    #[test]
    fn test_wasapi_always_available() {
        let backend = AudioBackend::Default;
        assert!(
            backend.is_available(),
            "WASAPI should always be available on Windows"
        );
    }

    #[test]
    fn test_wasapi_in_backend_list() {
        let backends = list_available_backends();
        assert!(
            backends.contains(&AudioBackend::Default),
            "WASAPI (Default) should be in available backends"
        );
    }

    #[test]
    fn test_wasapi_backend_info() {
        let info = get_backend_info();
        let wasapi_info = info
            .iter()
            .find(|b| b.name == "WASAPI" || b.backend == AudioBackend::Default);
        assert!(wasapi_info.is_some(), "Should find WASAPI in backend info");

        let wasapi = wasapi_info.unwrap();
        assert!(wasapi.available, "WASAPI should be available");
        assert!(wasapi.is_default, "WASAPI should be the default on Windows");
    }

    #[test]
    fn test_wasapi_cpal_host() {
        let backend = AudioBackend::Default;
        let host = backend.to_cpal_host();
        assert!(
            host.is_ok(),
            "Should be able to get CPAL host for WASAPI: {:?}",
            host.err()
        );
    }

    #[cfg(feature = "asio")]
    #[test]
    fn test_asio_backend_available_if_feature_enabled() {
        // ASIO backend should exist when feature is enabled
        let backend = AudioBackend::Asio;
        assert_eq!(backend.name(), "ASIO");
        // Note: ASIO availability depends on installed drivers
        let _available = backend.is_available();
    }
}

// ============================================================================
// Section 2: Exclusive Mode Configuration Tests
// ============================================================================

mod exclusive_config_tests {
    use super::*;

    #[test]
    fn test_exclusive_mode_enabled_by_default() {
        let config = ExclusiveConfig::default();
        assert!(
            config.exclusive_mode,
            "Exclusive mode should be enabled by default for bit-perfect playback"
        );
    }

    #[test]
    fn test_bit_perfect_presets_enable_exclusive() {
        let bp16 = ExclusiveConfig::bit_perfect_16();
        let bp24 = ExclusiveConfig::bit_perfect_24();
        let bp32 = ExclusiveConfig::bit_perfect_32();

        assert!(bp16.exclusive_mode, "bit_perfect_16 should enable exclusive mode");
        assert!(bp24.exclusive_mode, "bit_perfect_24 should enable exclusive mode");
        assert!(bp32.exclusive_mode, "bit_perfect_32 should enable exclusive mode");
    }

    #[test]
    fn test_low_latency_presets_enable_exclusive() {
        let low_lat = ExclusiveConfig::low_latency();
        let ultra_low = ExclusiveConfig::ultra_low_latency();

        assert!(
            low_lat.exclusive_mode,
            "low_latency should enable exclusive mode"
        );
        assert!(
            ultra_low.exclusive_mode,
            "ultra_low_latency should enable exclusive mode"
        );
    }

    #[test]
    fn test_exclusive_config_with_wasapi_backend() {
        let config = ExclusiveConfig::default().with_backend(AudioBackend::Default);

        assert_eq!(
            config.backend,
            AudioBackend::Default,
            "Should use Default (WASAPI) backend"
        );
        assert!(
            config.exclusive_mode,
            "Should have exclusive mode enabled for WASAPI"
        );
    }

    #[test]
    fn test_buffer_frames_for_low_latency_wasapi() {
        // WASAPI typically supports buffer sizes from 10ms down to ~3ms
        // For low-latency, we want 128 frames (~2.9ms at 44.1kHz)
        let config = ExclusiveConfig::low_latency();
        assert_eq!(config.buffer_frames, Some(128));

        // Ultra-low is 64 frames (~1.5ms at 44.1kHz)
        let ultra = ExclusiveConfig::ultra_low_latency();
        assert_eq!(ultra.buffer_frames, Some(64));
    }

    #[test]
    fn test_exclusive_config_serialization() {
        let config = ExclusiveConfig::bit_perfect_24()
            .with_sample_rate(96000)
            .with_buffer_frames(256);

        let json = serde_json::to_string(&config).expect("Failed to serialize");
        assert!(json.contains("exclusive_mode"));
        assert!(json.contains("96000"));
        assert!(json.contains("256"));

        let restored: ExclusiveConfig = serde_json::from_str(&json).expect("Failed to deserialize");
        assert!(restored.exclusive_mode);
        assert_eq!(restored.sample_rate, 96000);
        assert_eq!(restored.buffer_frames, Some(256));
    }
}

// ============================================================================
// Section 3: Exclusive Mode Acquisition Tests (Integration)
// ============================================================================

mod exclusive_mode_acquisition {
    use super::*;

    #[test]
    fn test_create_exclusive_output() {
        match ExclusiveOutput::new(ExclusiveConfig::default()) {
            Ok(output) => {
                assert!(output.sample_rate() > 0, "Sample rate should be positive");
                assert!(
                    output.latency().buffer_samples > 0,
                    "Buffer samples should be positive"
                );
                // WASAPI exclusive should be indicated in latency info
                // Note: This depends on whether exclusive was actually acquired
            }
            Err(e) => {
                eprintln!(
                    "Could not create exclusive output (expected in CI without audio): {}",
                    e
                );
            }
        }
    }

    #[test]
    fn test_exclusive_mode_with_specific_format() {
        let config = ExclusiveConfig::bit_perfect_24().with_sample_rate(48000);

        match ExclusiveOutput::new(config) {
            Ok(output) => {
                // Should have acquired exclusive mode with requested format
                assert!(output.config().exclusive_mode);
                // Note: actual sample rate may differ if 48kHz not supported
            }
            Err(e) => {
                eprintln!("Could not create 24-bit exclusive output: {}", e);
            }
        }
    }

    #[test]
    fn test_exclusive_output_releases_on_drop() {
        // Create exclusive output in a block
        {
            if let Ok(output) = ExclusiveOutput::new(ExclusiveConfig::default()) {
                let sample_rate = output.sample_rate();
                let samples = generate_test_audio(sample_rate, 100, 2);
                let _ = output.play(AudioData::Float32(samples));
                std::thread::sleep(std::time::Duration::from_millis(50));
                // Output will be dropped here
            }
        }

        // Small delay to allow release
        std::thread::sleep(std::time::Duration::from_millis(50));

        // Should be able to create another exclusive output
        match ExclusiveOutput::new(ExclusiveConfig::default()) {
            Ok(_) => {
                // Successfully re-acquired exclusive mode
            }
            Err(e) => {
                eprintln!("Note: Could not re-acquire exclusive mode: {}", e);
            }
        }
    }

    #[test]
    fn test_exclusive_mode_latency_info() {
        if let Ok(output) = ExclusiveOutput::new(ExclusiveConfig::default()) {
            let latency = output.latency();

            assert!(latency.buffer_samples > 0, "Should have buffer samples");
            assert!(latency.buffer_ms > 0.0, "Should have buffer latency");
            assert!(
                latency.total_ms >= latency.buffer_ms,
                "Total should include buffer"
            );

            // In exclusive mode, latency should typically be low
            // Shared mode might have higher latency
            if latency.exclusive {
                assert!(
                    latency.buffer_ms < 50.0,
                    "Exclusive mode buffer latency should be < 50ms, got {}",
                    latency.buffer_ms
                );
            }
        }
    }
}

// ============================================================================
// Section 4: Shared Mode Fallback Tests (Integration)
// ============================================================================

mod shared_mode_fallback {
    use super::*;

    #[test]
    fn test_shared_mode_config() {
        // Create a config that explicitly doesn't request exclusive mode
        // Note: The current API always sets exclusive_mode = true by default
        // This test documents that behavior and tests graceful fallback
        let config = ExclusiveConfig {
            exclusive_mode: false, // Request shared mode
            ..ExclusiveConfig::default()
        };

        // When exclusive mode is not requested, output should still work
        match ExclusiveOutput::new(config) {
            Ok(output) => {
                assert!(output.sample_rate() > 0);
                // In shared mode, Windows mixer is involved
            }
            Err(e) => {
                eprintln!("Could not create shared mode output: {}", e);
            }
        }
    }

    #[test]
    fn test_fallback_when_device_busy() {
        // This test verifies graceful behavior when exclusive mode cannot be acquired
        // In a real scenario, another app might have exclusive access

        // First output (might get exclusive)
        let first_result = ExclusiveOutput::new(ExclusiveConfig::default());

        if first_result.is_ok() {
            // Try to create a second exclusive output (might fail or fallback)
            let second_config = ExclusiveConfig::default();
            let second_result = ExclusiveOutput::new(second_config);

            match second_result {
                Ok(output2) => {
                    // Both outputs work (possibly in shared mode)
                    eprintln!(
                        "Note: Second output created, latency: {} ms",
                        output2.latency().buffer_ms
                    );
                }
                Err(e) => {
                    // Expected if exclusive mode is truly exclusive
                    eprintln!("Second exclusive output failed (expected): {}", e);
                }
            }
        }
    }
}

// ============================================================================
// Section 5: Buffer Size Negotiation Tests
// ============================================================================

mod buffer_size_negotiation {
    use super::*;

    #[test]
    fn test_default_buffer_size() {
        if let Ok(output) = ExclusiveOutput::new(ExclusiveConfig::default()) {
            let buffer_samples = output.latency().buffer_samples;
            // WASAPI default buffer is typically 10ms, so at 44.1kHz that's ~441 samples
            // At 48kHz that's ~480 samples
            // We'll accept a reasonable range
            assert!(
                buffer_samples >= 64 && buffer_samples <= 8192,
                "Default buffer should be reasonable: {} samples",
                buffer_samples
            );
        }
    }

    #[test]
    fn test_small_buffer_request() {
        let config = ExclusiveConfig::default().with_buffer_frames(64);

        match ExclusiveOutput::new(config) {
            Ok(output) => {
                let actual = output.latency().buffer_samples;
                // Device may clamp to minimum supported
                eprintln!("Requested 64 frames, got {} frames", actual);
                assert!(actual >= 32, "Buffer should be at least 32 samples");
            }
            Err(e) => {
                eprintln!("Small buffer not supported: {}", e);
            }
        }
    }

    #[test]
    fn test_large_buffer_request() {
        let config = ExclusiveConfig::default().with_buffer_frames(4096);

        match ExclusiveOutput::new(config) {
            Ok(output) => {
                let actual = output.latency().buffer_samples;
                // Device may clamp to maximum supported or accept
                eprintln!("Requested 4096 frames, got {} frames", actual);
                assert!(actual <= 65536, "Buffer should not exceed reasonable max");
            }
            Err(e) => {
                eprintln!("Large buffer request error: {}", e);
            }
        }
    }

    #[test]
    fn test_buffer_latency_calculation() {
        if let Ok(output) = ExclusiveOutput::new(ExclusiveConfig::default()) {
            let latency = output.latency();
            let sample_rate = output.sample_rate();

            // Verify latency calculation: buffer_ms = buffer_samples / sample_rate * 1000
            let expected_ms = latency.buffer_samples as f32 / sample_rate as f32 * 1000.0;
            let diff = (latency.buffer_ms - expected_ms).abs();

            assert!(
                diff < 0.1,
                "Buffer latency calculation should match: expected {}, got {}",
                expected_ms,
                latency.buffer_ms
            );
        }
    }

    #[test]
    fn test_various_buffer_sizes() {
        let buffer_sizes = [128, 256, 512, 1024, 2048];

        for &size in &buffer_sizes {
            let config = ExclusiveConfig::default().with_buffer_frames(size);
            match ExclusiveOutput::new(config) {
                Ok(output) => {
                    let actual = output.latency().buffer_samples;
                    eprintln!("Buffer {} frames -> {} actual", size, actual);
                }
                Err(e) => {
                    eprintln!("Buffer {} frames: {}", size, e);
                }
            }
        }
    }
}

// ============================================================================
// Section 6: Format Negotiation Tests
// ============================================================================

mod format_negotiation {
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
            }
            Err(e) => {
                eprintln!("Float32 format not available: {}", e);
            }
        }
    }

    #[test]
    fn test_int16_format() {
        let config = ExclusiveConfig::bit_perfect_16();

        match ExclusiveOutput::new(config) {
            Ok(output) => {
                assert_eq!(output.config().bit_depth, SupportedBitDepth::Int16);
            }
            Err(e) => {
                eprintln!("Int16 format not available: {}", e);
            }
        }
    }

    #[test]
    fn test_int24_format() {
        let config = ExclusiveConfig::bit_perfect_24();

        match ExclusiveOutput::new(config) {
            Ok(output) => {
                // Note: 24-bit is packed in 32-bit container (Int32 variant)
                let depth = output.config().bit_depth;
                assert!(
                    depth == SupportedBitDepth::Int24 || depth == SupportedBitDepth::Int32,
                    "Should be Int24 or Int32"
                );
            }
            Err(e) => {
                eprintln!("Int24 format not available: {}", e);
            }
        }
    }

    #[test]
    fn test_int32_format() {
        let config = ExclusiveConfig::bit_perfect_32();

        match ExclusiveOutput::new(config) {
            Ok(output) => {
                assert_eq!(output.config().bit_depth, SupportedBitDepth::Int32);
            }
            Err(e) => {
                eprintln!("Int32 format not available: {}", e);
            }
        }
    }

    #[test]
    fn test_sample_rate_44100() {
        let config = ExclusiveConfig::default().with_sample_rate(44100);

        match ExclusiveOutput::new(config) {
            Ok(output) => {
                // Might get 44100 or fallback to device native
                eprintln!("Requested 44100, got {}", output.sample_rate());
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
                eprintln!("96000 Hz not available (high-res may not be supported): {}", e);
            }
        }
    }

    #[test]
    fn test_sample_rate_192000() {
        let config = ExclusiveConfig::default().with_sample_rate(192000);

        match ExclusiveOutput::new(config) {
            Ok(output) => {
                eprintln!("Requested 192000, got {}", output.sample_rate());
            }
            Err(e) => {
                eprintln!("192000 Hz not available: {}", e);
            }
        }
    }

    #[test]
    fn test_channel_count_stereo() {
        // Most devices support stereo (2 channels)
        if let Ok(output) = ExclusiveOutput::new(ExclusiveConfig::default()) {
            let sample_rate = output.sample_rate();
            // Generate stereo audio
            let samples = generate_test_audio(sample_rate, 100, 2);
            assert!(output.play(AudioData::Float32(samples)).is_ok());
        }
    }
}

// ============================================================================
// Section 7: Hardware-Dependent Tests (marked with #[ignore])
// ============================================================================

mod hardware_tests {
    use super::*;
    use std::time::Duration;

    /// Test actual exclusive mode playback with audio output
    ///
    /// This test requires:
    /// - A physical audio device connected
    /// - No other application using the device exclusively
    /// - Speakers or headphones to verify audio
    #[test]
    #[ignore = "Requires audio hardware - run with --ignored"]
    fn test_exclusive_mode_playback() {
        let config = ExclusiveConfig::bit_perfect_24().with_sample_rate(48000);

        let output = ExclusiveOutput::new(config).expect("Should create exclusive output");

        assert!(output.config().exclusive_mode);
        eprintln!("Exclusive mode acquired at {} Hz", output.sample_rate());
        eprintln!("Latency: {} ms", output.latency().buffer_ms);

        // Play test tone
        let samples = generate_test_audio(output.sample_rate(), 1000, 2);
        output
            .play(AudioData::Float32(samples))
            .expect("Should play");

        std::thread::sleep(Duration::from_secs(1));

        output.stop().expect("Should stop");
    }

    /// Test exclusive mode with different bit depths
    #[test]
    #[ignore = "Requires audio hardware - run with --ignored"]
    fn test_exclusive_mode_bit_depths() {
        let bit_depths = [
            SupportedBitDepth::Int16,
            SupportedBitDepth::Int24,
            SupportedBitDepth::Int32,
            SupportedBitDepth::Float32,
        ];

        for depth in bit_depths {
            let config = ExclusiveConfig {
                bit_depth: depth,
                exclusive_mode: true,
                ..ExclusiveConfig::default()
            };

            match ExclusiveOutput::new(config) {
                Ok(output) => {
                    eprintln!("{:?}: {} Hz, {} ms latency", depth, output.sample_rate(), output.latency().buffer_ms);
                    let samples = generate_test_audio(output.sample_rate(), 500, 2);
                    output.play_f32(&samples).expect("Should play");
                    std::thread::sleep(Duration::from_millis(500));
                    output.stop().expect("Should stop");
                }
                Err(e) => {
                    eprintln!("{:?}: Not supported - {}", depth, e);
                }
            }

            std::thread::sleep(Duration::from_millis(100));
        }
    }

    /// Test buffer size impact on latency
    #[test]
    #[ignore = "Requires audio hardware - run with --ignored"]
    fn test_buffer_sizes_latency_impact() {
        let buffer_sizes = [64, 128, 256, 512, 1024, 2048, 4096];

        for &size in &buffer_sizes {
            let config = ExclusiveConfig::default().with_buffer_frames(size);

            match ExclusiveOutput::new(config) {
                Ok(output) => {
                    let latency = output.latency();
                    eprintln!(
                        "Buffer {} frames: actual {} frames, {:.2} ms latency",
                        size, latency.buffer_samples, latency.buffer_ms
                    );
                }
                Err(e) => {
                    eprintln!("Buffer {} frames: {}", size, e);
                }
            }

            std::thread::sleep(Duration::from_millis(50));
        }
    }

    /// Test sample rate switching in exclusive mode
    #[test]
    #[ignore = "Requires audio hardware - run with --ignored"]
    fn test_sample_rate_switching() {
        let sample_rates = [44100, 48000, 88200, 96000, 176400, 192000];

        for &rate in &sample_rates {
            let config = ExclusiveConfig::default().with_sample_rate(rate);

            match ExclusiveOutput::new(config) {
                Ok(output) => {
                    let actual = output.sample_rate();
                    if actual == rate {
                        eprintln!("{} Hz: Supported (exact match)", rate);
                    } else {
                        eprintln!("{} Hz: Fallback to {} Hz", rate, actual);
                    }
                }
                Err(e) => {
                    eprintln!("{} Hz: Not supported - {}", rate, e);
                }
            }

            std::thread::sleep(Duration::from_millis(50));
        }
    }

    /// Test concurrent exclusive mode requests (should fail gracefully)
    #[test]
    #[ignore = "Requires audio hardware - run with --ignored"]
    fn test_concurrent_exclusive_requests() {
        let output1 = ExclusiveOutput::new(ExclusiveConfig::default());

        if output1.is_err() {
            eprintln!("First output failed, skipping concurrent test");
            return;
        }

        let _output1 = output1.unwrap();
        eprintln!("First exclusive output acquired");

        // Try to acquire second exclusive output
        let output2 = ExclusiveOutput::new(ExclusiveConfig::default());

        match output2 {
            Ok(_) => {
                eprintln!("Warning: Second exclusive output succeeded (may be shared mode fallback)");
            }
            Err(e) => {
                eprintln!("Second exclusive output failed (expected): {}", e);
            }
        }
    }
}

// ============================================================================
// Section 8: WASAPI-Specific Feature Tests
// ============================================================================

mod wasapi_specific {
    use super::*;
    use soul_audio_desktop::device::{get_default_device_with_capabilities, list_devices_with_capabilities};

    #[test]
    fn test_wasapi_device_capabilities() {
        match get_default_device_with_capabilities(AudioBackend::Default, true) {
            Ok(device) => {
                eprintln!("WASAPI device: {}", device.name);
                eprintln!("Native rate: {} Hz", device.sample_rate);
                eprintln!("Channels: {}", device.channels);

                if let Some(caps) = &device.capabilities {
                    eprintln!("Sample rates: {:?}", caps.sample_rates);
                    eprintln!(
                        "Bit depths: {:?}",
                        caps.bit_depths.iter().map(|d| d.display_name()).collect::<Vec<_>>()
                    );
                    eprintln!("Exclusive mode support: {}", caps.supports_exclusive);
                }
            }
            Err(e) => {
                eprintln!("Could not get WASAPI device: {}", e);
            }
        }
    }

    #[test]
    fn test_wasapi_multiple_devices() {
        match list_devices_with_capabilities(AudioBackend::Default, true) {
            Ok(devices) => {
                eprintln!("Found {} WASAPI devices:", devices.len());
                for device in &devices {
                    eprintln!(
                        "  - {} (default: {}, {} Hz, {} ch)",
                        device.name, device.is_default, device.sample_rate, device.channels
                    );
                }
            }
            Err(e) => {
                eprintln!("Could not enumerate WASAPI devices: {}", e);
            }
        }
    }

    #[test]
    fn test_wasapi_exclusive_mode_support_flag() {
        match get_default_device_with_capabilities(AudioBackend::Default, true) {
            Ok(device) => {
                if let Some(caps) = &device.capabilities {
                    // WASAPI should indicate exclusive mode support
                    eprintln!(
                        "Device {} exclusive mode support: {}",
                        device.name, caps.supports_exclusive
                    );
                }
            }
            Err(e) => {
                eprintln!("Could not check exclusive mode support: {}", e);
            }
        }
    }
}

// ============================================================================
// Section 9: ASIO Tests (when feature enabled)
// ============================================================================

#[cfg(feature = "asio")]
mod asio_tests {
    use super::*;
    use soul_audio_desktop::device::list_devices;

    #[test]
    fn test_asio_backend_exists() {
        let backend = AudioBackend::Asio;
        assert_eq!(backend.name(), "ASIO");
    }

    #[test]
    fn test_asio_availability() {
        let backend = AudioBackend::Asio;
        let available = backend.is_available();
        eprintln!("ASIO available: {}", available);
        // ASIO depends on installed drivers
    }

    #[test]
    fn test_asio_device_enumeration() {
        let backend = AudioBackend::Asio;
        if backend.is_available() {
            match list_devices(backend) {
                Ok(devices) => {
                    eprintln!("Found {} ASIO devices", devices.len());
                    for device in &devices {
                        eprintln!("  - {}", device.name);
                    }
                }
                Err(e) => {
                    eprintln!("ASIO enumeration error: {}", e);
                }
            }
        }
    }

    /// Test ASIO exclusive output (ASIO is always exclusive by design)
    #[test]
    #[ignore = "Requires ASIO driver - run with --ignored"]
    fn test_asio_exclusive_output() {
        let backend = AudioBackend::Asio;
        if !backend.is_available() {
            eprintln!("ASIO not available, skipping");
            return;
        }

        let config = ExclusiveConfig::default().with_backend(backend);

        match ExclusiveOutput::new(config) {
            Ok(output) => {
                eprintln!(
                    "ASIO output: {} Hz, {} ms latency",
                    output.sample_rate(),
                    output.latency().buffer_ms
                );
                // ASIO should have very low latency
                assert!(
                    output.latency().buffer_ms < 20.0,
                    "ASIO latency should be < 20ms"
                );
            }
            Err(e) => {
                eprintln!("ASIO output error: {}", e);
            }
        }
    }
}

// ============================================================================
// Section 10: Edge Cases and Error Handling
// ============================================================================

mod error_handling {
    use super::*;
    #[allow(unused_imports)]
    use soul_audio_desktop::AudioOutputError;

    #[test]
    fn test_nonexistent_device_error() {
        let config = ExclusiveConfig::default().with_device("NonexistentDevice12345XYZ");

        let result = ExclusiveOutput::new(config);
        assert!(result.is_err(), "Should fail with nonexistent device");
    }

    #[test]
    fn test_invalid_sample_rate_handling() {
        // Try an unusual sample rate that likely isn't supported
        let config = ExclusiveConfig::default().with_sample_rate(12345);

        match ExclusiveOutput::new(config) {
            Ok(output) => {
                // May succeed with fallback rate
                eprintln!("Got fallback rate: {}", output.sample_rate());
            }
            Err(e) => {
                // Error is expected
                eprintln!("Invalid rate error (expected): {}", e);
            }
        }
    }

    #[test]
    fn test_zero_buffer_handling() {
        let config = ExclusiveConfig::default().with_buffer_frames(0);

        match ExclusiveOutput::new(config) {
            Ok(output) => {
                // May succeed with default buffer
                assert!(output.latency().buffer_samples > 0);
            }
            Err(_) => {
                // Error is also acceptable
            }
        }
    }

    #[test]
    fn test_very_large_buffer_handling() {
        let config = ExclusiveConfig::default().with_buffer_frames(1_000_000);

        match ExclusiveOutput::new(config) {
            Ok(output) => {
                // Should clamp or use reasonable max
                let actual = output.latency().buffer_samples;
                eprintln!("Very large buffer clamped to: {}", actual);
            }
            Err(e) => {
                eprintln!("Very large buffer error: {}", e);
            }
        }
    }

    #[test]
    fn test_stop_without_play() {
        if let Ok(output) = ExclusiveOutput::new(ExclusiveConfig::default()) {
            // Stop without playing should not panic
            let result = output.stop();
            assert!(result.is_ok(), "Stop without play should succeed");
        }
    }

    #[test]
    fn test_pause_without_play() {
        if let Ok(output) = ExclusiveOutput::new(ExclusiveConfig::default()) {
            // Pause without playing should not panic
            let result = output.pause();
            assert!(result.is_ok(), "Pause without play should succeed");
        }
    }

    #[test]
    fn test_multiple_stop_calls() {
        if let Ok(output) = ExclusiveOutput::new(ExclusiveConfig::default()) {
            let _ = output.stop();
            let _ = output.stop();
            let _ = output.stop();
            // Should not panic
        }
    }
}
