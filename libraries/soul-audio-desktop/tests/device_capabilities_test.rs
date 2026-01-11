//! Extensive tests for DAC capability detection
//!
//! This test module validates device capability detection for audio hardware,
//! including sample rates, bit depths, DSD support, and backend-specific features.
//!
//! ## Test Strategy
//!
//! Tests are designed to be robust in CI environments without audio hardware:
//! - Core unit tests for data structures work without hardware
//! - Integration tests use graceful fallbacks when no devices exist
//! - Property tests verify invariants on any data returned
//!
//! Run with: `cargo test -p soul-audio-desktop device_capabilities_test`

use soul_audio_desktop::{
    backend::{get_backend_info, list_available_backends, BackendError},
    detect_device_capabilities,
    device::{find_device_by_name, get_default_device, list_devices},
    get_default_device_with_capabilities, get_device_capabilities, list_devices_with_capabilities,
    AudioBackend, AudioDeviceInfo, DeviceCapabilities, DeviceError, SupportedBitDepth, DSD_RATES,
    STANDARD_SAMPLE_RATES,
};

// ============================================================================
// 1. DeviceCapabilities Struct Tests
// ============================================================================

mod device_capabilities_struct {
    use super::*;

    #[test]
    fn test_default_values() {
        let caps = DeviceCapabilities::default();

        // Default sample rates should include CD and DAT quality
        assert!(
            caps.sample_rates.contains(&44100),
            "Default should include 44.1kHz (CD quality)"
        );
        assert!(
            caps.sample_rates.contains(&48000),
            "Default should include 48kHz (DAT quality)"
        );

        // Default bit depth should be Float32
        assert_eq!(
            caps.bit_depths.len(),
            1,
            "Default should have one bit depth"
        );
        assert_eq!(
            caps.bit_depths[0],
            SupportedBitDepth::Float32,
            "Default bit depth should be Float32"
        );

        // Default channels should be stereo
        assert_eq!(caps.max_channels, 2, "Default should be stereo");

        // Default should not support exclusive mode or DSD
        assert!(
            !caps.supports_exclusive,
            "Default should not support exclusive mode"
        );
        assert!(!caps.supports_dsd, "Default should not support DSD");
        assert!(
            caps.dsd_rates.is_empty(),
            "Default should have no DSD rates"
        );

        // Buffer sizes should be unset by default
        assert!(
            caps.min_buffer_frames.is_none(),
            "Default min buffer should be None"
        );
        assert!(
            caps.max_buffer_frames.is_none(),
            "Default max buffer should be None"
        );

        // Hardware volume should be off by default
        assert!(
            !caps.has_hardware_volume,
            "Default should not have hardware volume"
        );
    }

    #[test]
    fn test_default_is_valid() {
        let caps = DeviceCapabilities::default();

        // Validate invariants
        assert!(
            !caps.sample_rates.is_empty(),
            "Must have at least one sample rate"
        );
        assert!(
            !caps.bit_depths.is_empty(),
            "Must have at least one bit depth"
        );
        assert!(caps.max_channels > 0, "Must have at least one channel");
    }

    #[test]
    fn test_custom_capabilities() {
        let caps = DeviceCapabilities {
            sample_rates: vec![44100, 48000, 96000, 192000],
            bit_depths: vec![
                SupportedBitDepth::Int16,
                SupportedBitDepth::Int24,
                SupportedBitDepth::Float32,
            ],
            max_channels: 8,
            supports_exclusive: true,
            supports_dsd: true,
            dsd_rates: vec![2822400, 5644800],
            min_buffer_frames: Some(64),
            max_buffer_frames: Some(8192),
            has_hardware_volume: true,
        };

        assert_eq!(caps.sample_rates.len(), 4);
        assert_eq!(caps.bit_depths.len(), 3);
        assert_eq!(caps.max_channels, 8);
        assert!(caps.supports_exclusive);
        assert!(caps.supports_dsd);
        assert_eq!(caps.dsd_rates.len(), 2);
        assert_eq!(caps.min_buffer_frames, Some(64));
        assert_eq!(caps.max_buffer_frames, Some(8192));
        assert!(caps.has_hardware_volume);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let original = DeviceCapabilities {
            sample_rates: vec![44100, 96000, 192000],
            bit_depths: vec![SupportedBitDepth::Int24, SupportedBitDepth::Float32],
            max_channels: 2,
            supports_exclusive: true,
            supports_dsd: false,
            dsd_rates: vec![],
            min_buffer_frames: Some(256),
            max_buffer_frames: Some(4096),
            has_hardware_volume: false,
        };

        // Serialize to JSON
        let json = serde_json::to_string(&original).expect("Failed to serialize");

        // Deserialize back
        let deserialized: DeviceCapabilities =
            serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(original.sample_rates, deserialized.sample_rates);
        assert_eq!(original.bit_depths, deserialized.bit_depths);
        assert_eq!(original.max_channels, deserialized.max_channels);
        assert_eq!(original.supports_exclusive, deserialized.supports_exclusive);
        assert_eq!(original.supports_dsd, deserialized.supports_dsd);
        assert_eq!(original.dsd_rates, deserialized.dsd_rates);
        assert_eq!(original.min_buffer_frames, deserialized.min_buffer_frames);
        assert_eq!(original.max_buffer_frames, deserialized.max_buffer_frames);
        assert_eq!(
            original.has_hardware_volume,
            deserialized.has_hardware_volume
        );
    }
}

// ============================================================================
// 2. SupportedBitDepth Enum Tests
// ============================================================================

mod supported_bit_depth {
    use super::*;

    #[test]
    fn test_all_variants_exist() {
        // Ensure all expected variants are accessible
        let _int16 = SupportedBitDepth::Int16;
        let _int24 = SupportedBitDepth::Int24;
        let _int32 = SupportedBitDepth::Int32;
        let _float32 = SupportedBitDepth::Float32;
        let _float64 = SupportedBitDepth::Float64;
    }

    #[test]
    fn test_bits_method() {
        assert_eq!(SupportedBitDepth::Int16.bits(), 16);
        assert_eq!(SupportedBitDepth::Int24.bits(), 24);
        assert_eq!(SupportedBitDepth::Int32.bits(), 32);
        assert_eq!(SupportedBitDepth::Float32.bits(), 32);
        assert_eq!(SupportedBitDepth::Float64.bits(), 64);
    }

    #[test]
    fn test_is_integer_method() {
        assert!(SupportedBitDepth::Int16.is_integer());
        assert!(SupportedBitDepth::Int24.is_integer());
        assert!(SupportedBitDepth::Int32.is_integer());
        assert!(!SupportedBitDepth::Float32.is_integer());
        assert!(!SupportedBitDepth::Float64.is_integer());
    }

    #[test]
    fn test_is_float_method() {
        assert!(!SupportedBitDepth::Int16.is_float());
        assert!(!SupportedBitDepth::Int24.is_float());
        assert!(!SupportedBitDepth::Int32.is_float());
        assert!(SupportedBitDepth::Float32.is_float());
        assert!(SupportedBitDepth::Float64.is_float());
    }

    #[test]
    fn test_integer_float_mutually_exclusive() {
        let all_variants = [
            SupportedBitDepth::Int16,
            SupportedBitDepth::Int24,
            SupportedBitDepth::Int32,
            SupportedBitDepth::Float32,
            SupportedBitDepth::Float64,
        ];

        for variant in all_variants {
            assert_ne!(
                variant.is_integer(),
                variant.is_float(),
                "A bit depth cannot be both integer and float: {:?}",
                variant
            );
        }
    }

    #[test]
    fn test_display_name_method() {
        assert_eq!(SupportedBitDepth::Int16.display_name(), "16-bit");
        assert_eq!(SupportedBitDepth::Int24.display_name(), "24-bit");
        assert_eq!(SupportedBitDepth::Int32.display_name(), "32-bit");
        assert_eq!(SupportedBitDepth::Float32.display_name(), "32-bit float");
        assert_eq!(SupportedBitDepth::Float64.display_name(), "64-bit float");
    }

    #[test]
    fn test_display_names_are_unique() {
        let names: Vec<&str> = [
            SupportedBitDepth::Int16,
            SupportedBitDepth::Int24,
            SupportedBitDepth::Int32,
            SupportedBitDepth::Float32,
            SupportedBitDepth::Float64,
        ]
        .iter()
        .map(|d| d.display_name())
        .collect();

        for (i, name1) in names.iter().enumerate() {
            for (j, name2) in names.iter().enumerate() {
                if i != j {
                    assert_ne!(name1, name2, "Display names should be unique");
                }
            }
        }
    }

    #[test]
    fn test_serialization() {
        let depths = [
            SupportedBitDepth::Int16,
            SupportedBitDepth::Int24,
            SupportedBitDepth::Int32,
            SupportedBitDepth::Float32,
            SupportedBitDepth::Float64,
        ];

        for depth in depths {
            let json = serde_json::to_string(&depth).expect("Failed to serialize");
            let restored: SupportedBitDepth =
                serde_json::from_str(&json).expect("Failed to deserialize");
            assert_eq!(
                depth, restored,
                "Serialization roundtrip failed for {:?}",
                depth
            );
        }
    }

    #[test]
    fn test_equality_and_hash() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(SupportedBitDepth::Int16);
        set.insert(SupportedBitDepth::Int24);
        set.insert(SupportedBitDepth::Int32);
        set.insert(SupportedBitDepth::Float32);
        set.insert(SupportedBitDepth::Float64);

        // All should be inserted (5 unique values)
        assert_eq!(set.len(), 5);

        // Duplicates should not increase size
        set.insert(SupportedBitDepth::Int16);
        assert_eq!(set.len(), 5);

        // Contains checks
        assert!(set.contains(&SupportedBitDepth::Int16));
        assert!(set.contains(&SupportedBitDepth::Float32));
    }

    #[test]
    fn test_clone_and_copy() {
        let depth = SupportedBitDepth::Int24;
        let cloned = depth.clone();
        let copied = depth;

        assert_eq!(depth, cloned);
        assert_eq!(depth, copied);
    }
}

// ============================================================================
// 3. Sample Rate Constants Tests
// ============================================================================

mod sample_rate_constants {
    use super::*;

    #[test]
    fn test_standard_sample_rates_defined() {
        assert!(
            !STANDARD_SAMPLE_RATES.is_empty(),
            "Standard rates must be defined"
        );
    }

    #[test]
    fn test_standard_rates_include_cd_quality() {
        assert!(
            STANDARD_SAMPLE_RATES.contains(&44100),
            "Must include 44.1kHz (CD quality)"
        );
    }

    #[test]
    fn test_standard_rates_include_dat_quality() {
        assert!(
            STANDARD_SAMPLE_RATES.contains(&48000),
            "Must include 48kHz (DAT/DVD quality)"
        );
    }

    #[test]
    fn test_high_resolution_rates_882k() {
        assert!(
            STANDARD_SAMPLE_RATES.contains(&88200),
            "Must include 88.2kHz (2x CD)"
        );
    }

    #[test]
    fn test_high_resolution_rates_96k() {
        assert!(
            STANDARD_SAMPLE_RATES.contains(&96000),
            "Must include 96kHz (high-res)"
        );
    }

    #[test]
    fn test_high_resolution_rates_1764k() {
        assert!(
            STANDARD_SAMPLE_RATES.contains(&176400),
            "Must include 176.4kHz (4x CD)"
        );
    }

    #[test]
    fn test_high_resolution_rates_192k() {
        assert!(
            STANDARD_SAMPLE_RATES.contains(&192000),
            "Must include 192kHz (high-res)"
        );
    }

    #[test]
    fn test_ultra_high_resolution_rates() {
        // These are used for DSD over PCM (DoP)
        assert!(
            STANDARD_SAMPLE_RATES.contains(&352800),
            "Must include 352.8kHz (DSD64 DoP)"
        );
        assert!(
            STANDARD_SAMPLE_RATES.contains(&384000),
            "Must include 384kHz (8x 48kHz)"
        );
    }

    #[test]
    fn test_standard_rates_are_sorted() {
        for i in 1..STANDARD_SAMPLE_RATES.len() {
            assert!(
                STANDARD_SAMPLE_RATES[i - 1] < STANDARD_SAMPLE_RATES[i],
                "Standard rates must be sorted in ascending order"
            );
        }
    }

    #[test]
    fn test_standard_rates_are_positive() {
        for &rate in STANDARD_SAMPLE_RATES {
            assert!(rate > 0, "All sample rates must be positive");
        }
    }

    #[test]
    fn test_standard_rates_minimum() {
        let min_rate = *STANDARD_SAMPLE_RATES.iter().min().unwrap();
        assert!(
            min_rate >= 8000,
            "Minimum rate should be at least 8kHz (telephony)"
        );
    }

    #[test]
    fn test_standard_rates_maximum() {
        let max_rate = *STANDARD_SAMPLE_RATES.iter().max().unwrap();
        assert!(
            max_rate <= 1_000_000,
            "Maximum rate should be reasonable (<= 1MHz)"
        );
    }

    #[test]
    fn test_standard_rates_count() {
        // Should have a reasonable number of standard rates
        assert!(
            STANDARD_SAMPLE_RATES.len() >= 6,
            "Should have at least 6 standard rates"
        );
        assert!(
            STANDARD_SAMPLE_RATES.len() <= 20,
            "Should not have too many standard rates"
        );
    }
}

// ============================================================================
// 4. DSD Rate Constants Tests
// ============================================================================

mod dsd_rate_constants {
    use super::*;

    #[test]
    fn test_dsd_rates_defined() {
        assert!(!DSD_RATES.is_empty(), "DSD rates must be defined");
    }

    #[test]
    fn test_dsd64_rate() {
        let dsd64 = DSD_RATES.iter().find(|(_, name)| *name == "DSD64");
        assert!(dsd64.is_some(), "DSD64 must be defined");
        assert_eq!(
            dsd64.unwrap().0,
            2_822_400,
            "DSD64 = 64 x 44.1kHz = 2.8224 MHz"
        );
    }

    #[test]
    fn test_dsd128_rate() {
        let dsd128 = DSD_RATES.iter().find(|(_, name)| *name == "DSD128");
        assert!(dsd128.is_some(), "DSD128 must be defined");
        assert_eq!(
            dsd128.unwrap().0,
            5_644_800,
            "DSD128 = 128 x 44.1kHz = 5.6448 MHz"
        );
    }

    #[test]
    fn test_dsd256_rate() {
        let dsd256 = DSD_RATES.iter().find(|(_, name)| *name == "DSD256");
        assert!(dsd256.is_some(), "DSD256 must be defined");
        assert_eq!(
            dsd256.unwrap().0,
            11_289_600,
            "DSD256 = 256 x 44.1kHz = 11.2896 MHz"
        );
    }

    #[test]
    fn test_dsd512_rate() {
        let dsd512 = DSD_RATES.iter().find(|(_, name)| *name == "DSD512");
        assert!(dsd512.is_some(), "DSD512 must be defined");
        assert_eq!(
            dsd512.unwrap().0,
            22_579_200,
            "DSD512 = 512 x 44.1kHz = 22.5792 MHz"
        );
    }

    #[test]
    fn test_dsd_rates_are_multiples_of_44100() {
        for &(rate, name) in DSD_RATES {
            assert!(
                rate % 44100 == 0,
                "{} rate ({}) should be a multiple of 44.1kHz",
                name,
                rate
            );
        }
    }

    #[test]
    fn test_dsd_rates_are_powers_of_two_multiples() {
        let expected_multipliers = [64u32, 128, 256, 512];
        let base = 44100u32;

        for &multiplier in &expected_multipliers {
            let expected_rate = base * multiplier;
            let found = DSD_RATES.iter().any(|&(rate, _)| rate == expected_rate);
            assert!(
                found,
                "DSD rates should include {}x 44.1kHz = {}",
                multiplier, expected_rate
            );
        }
    }

    #[test]
    fn test_dsd_rates_are_sorted() {
        for i in 1..DSD_RATES.len() {
            assert!(
                DSD_RATES[i - 1].0 < DSD_RATES[i].0,
                "DSD rates must be sorted in ascending order"
            );
        }
    }

    #[test]
    fn test_dsd_rate_names_are_unique() {
        let names: Vec<&str> = DSD_RATES.iter().map(|(_, name)| *name).collect();
        for (i, name1) in names.iter().enumerate() {
            for (j, name2) in names.iter().enumerate() {
                if i != j {
                    assert_ne!(name1, name2, "DSD rate names must be unique");
                }
            }
        }
    }

    #[test]
    fn test_dsd_rates_count() {
        assert_eq!(
            DSD_RATES.len(),
            4,
            "Should have exactly 4 DSD rate levels (64, 128, 256, 512)"
        );
    }
}

// ============================================================================
// 5. Device Enumeration Tests (Integration)
// ============================================================================

mod device_enumeration {
    use super::*;

    /// Helper to check if we have audio devices available
    fn has_audio_devices() -> bool {
        match list_devices(AudioBackend::Default) {
            Ok(devices) => !devices.is_empty(),
            Err(_) => false,
        }
    }

    #[test]
    fn test_list_devices_returns_result() {
        let result = list_devices(AudioBackend::Default);
        // Should return Ok or specific error, never panic
        match result {
            Ok(devices) => {
                eprintln!("Found {} audio devices", devices.len());
            }
            Err(e) => {
                eprintln!("Note: Could not list devices (expected in CI): {}", e);
            }
        }
    }

    #[test]
    fn test_list_devices_with_capabilities_enabled() {
        let result = list_devices_with_capabilities(AudioBackend::Default, true);

        match result {
            Ok(devices) => {
                for device in &devices {
                    assert!(
                        device.capabilities.is_some(),
                        "When capabilities requested, all devices should have them"
                    );
                }
            }
            Err(e) => {
                eprintln!("Note: Could not list devices with capabilities: {}", e);
            }
        }
    }

    #[test]
    fn test_list_devices_with_capabilities_disabled() {
        let result = list_devices_with_capabilities(AudioBackend::Default, false);

        match result {
            Ok(devices) => {
                for device in &devices {
                    assert!(
                        device.capabilities.is_none(),
                        "When capabilities not requested, devices should not have them"
                    );
                }
            }
            Err(e) => {
                eprintln!("Note: Could not list devices: {}", e);
            }
        }
    }

    #[test]
    fn test_list_devices_valid_data() {
        let result = list_devices_with_capabilities(AudioBackend::Default, true);

        match result {
            Ok(devices) => {
                for device in &devices {
                    // Basic validation
                    assert!(!device.name.is_empty(), "Device name should not be empty");
                    assert!(device.sample_rate > 0, "Sample rate should be positive");
                    assert!(device.channels > 0, "Channel count should be positive");

                    // Capability validation
                    if let Some(caps) = &device.capabilities {
                        assert!(!caps.sample_rates.is_empty(), "Should have sample rates");
                        assert!(!caps.bit_depths.is_empty(), "Should have bit depths");
                        assert!(caps.max_channels > 0, "Should have at least one channel");
                    }
                }
            }
            Err(e) => {
                eprintln!("Note: Could not list devices: {}", e);
            }
        }
    }

    #[test]
    fn test_get_default_device() {
        let result = get_default_device(AudioBackend::Default);

        match result {
            Ok(device) => {
                assert!(
                    device.is_default,
                    "Default device should be marked as default"
                );
                assert!(!device.name.is_empty(), "Device should have a name");
                assert!(device.sample_rate > 0, "Sample rate should be positive");
            }
            Err(DeviceError::NoDeviceFound) => {
                eprintln!("Note: No default device found (expected in CI)");
            }
            Err(e) => {
                eprintln!("Note: Could not get default device: {}", e);
            }
        }
    }

    #[test]
    fn test_get_default_device_with_capabilities() {
        let result = get_default_device_with_capabilities(AudioBackend::Default, true);

        match result {
            Ok(device) => {
                assert!(
                    device.capabilities.is_some(),
                    "Should have capabilities when requested"
                );

                let caps = device.capabilities.as_ref().unwrap();
                assert!(!caps.sample_rates.is_empty());
                assert!(!caps.bit_depths.is_empty());

                // Sample rates should be sorted
                for i in 1..caps.sample_rates.len() {
                    assert!(
                        caps.sample_rates[i - 1] <= caps.sample_rates[i],
                        "Sample rates should be sorted"
                    );
                }
            }
            Err(e) => {
                eprintln!("Note: Could not get default device: {}", e);
            }
        }
    }

    #[test]
    fn test_find_device_by_name_valid() {
        if !has_audio_devices() {
            eprintln!("Skipping: No audio devices available");
            return;
        }

        let devices = list_devices(AudioBackend::Default).unwrap();
        if let Some(first) = devices.first() {
            let result = find_device_by_name(AudioBackend::Default, &first.name);
            assert!(result.is_ok(), "Should find device by exact name");
        }
    }

    #[test]
    fn test_find_device_by_name_invalid() {
        let invalid_name = "This Device Does Not Exist XYZ123!@#";
        let result = find_device_by_name(AudioBackend::Default, invalid_name);

        assert!(result.is_err(), "Should fail for invalid device name");
        assert!(
            matches!(result, Err(DeviceError::DeviceNotFound(_))),
            "Should return DeviceNotFound error"
        );
    }

    #[test]
    fn test_find_device_by_name_empty_string() {
        let result = find_device_by_name(AudioBackend::Default, "");

        // Empty string should return DeviceNotFound
        assert!(result.is_err(), "Empty device name should fail");
    }

    #[test]
    fn test_device_sorting_default_first() {
        let result = list_devices(AudioBackend::Default);

        match result {
            Ok(devices) if !devices.is_empty() => {
                // If there's a default device, it should be first
                if devices.iter().any(|d| d.is_default) {
                    assert!(
                        devices[0].is_default,
                        "Default device should be sorted first"
                    );
                }
            }
            _ => {
                eprintln!("Note: Cannot test sorting without devices");
            }
        }
    }

    #[test]
    fn test_device_sorting_alphabetical_after_default() {
        let result = list_devices(AudioBackend::Default);

        match result {
            Ok(devices) if devices.len() > 2 => {
                // Skip the default device, remaining should be alphabetical
                let non_default: Vec<&AudioDeviceInfo> =
                    devices.iter().filter(|d| !d.is_default).collect();

                for i in 1..non_default.len() {
                    assert!(
                        non_default[i - 1].name <= non_default[i].name,
                        "Non-default devices should be sorted alphabetically"
                    );
                }
            }
            _ => {
                eprintln!("Note: Need at least 3 devices to test alphabetical sorting");
            }
        }
    }
}

// ============================================================================
// 6. Backend Tests
// ============================================================================

mod backend_tests {
    use super::*;

    #[test]
    fn test_default_backend_always_available() {
        let backend = AudioBackend::Default;
        assert!(
            backend.is_available(),
            "Default backend should always be available"
        );
    }

    #[test]
    fn test_default_backend_name_not_empty() {
        let backend = AudioBackend::Default;
        let name = backend.name();
        assert!(!name.is_empty(), "Backend name should not be empty");
    }

    #[test]
    fn test_default_backend_description_not_empty() {
        let backend = AudioBackend::Default;
        let desc = backend.description();
        assert!(!desc.is_empty(), "Backend description should not be empty");
    }

    #[test]
    fn test_default_backend_platform_name() {
        let backend = AudioBackend::Default;
        let name = backend.name();

        #[cfg(target_os = "windows")]
        assert_eq!(name, "WASAPI");

        #[cfg(target_os = "macos")]
        assert_eq!(name, "CoreAudio");

        #[cfg(target_os = "linux")]
        assert_eq!(name, "ALSA");
    }

    #[test]
    fn test_to_cpal_host_succeeds() {
        let backend = AudioBackend::Default;
        let result = backend.to_cpal_host();
        assert!(result.is_ok(), "Should get CPAL host for default backend");
    }

    #[test]
    fn test_list_available_backends_not_empty() {
        let backends = list_available_backends();
        assert!(
            !backends.is_empty(),
            "At least one backend should be available"
        );
    }

    #[test]
    fn test_list_available_backends_includes_default() {
        let backends = list_available_backends();
        assert!(
            backends.contains(&AudioBackend::Default),
            "Default backend should always be in the list"
        );
    }

    #[test]
    fn test_get_backend_info_not_empty() {
        let info = get_backend_info();
        assert!(!info.is_empty(), "Backend info should not be empty");
    }

    #[test]
    fn test_get_backend_info_has_available() {
        let info = get_backend_info();
        assert!(
            info.iter().any(|b| b.available),
            "At least one backend should be available"
        );
    }

    #[test]
    fn test_backend_info_fields_populated() {
        let info = get_backend_info();

        for backend_info in &info {
            assert!(!backend_info.name.is_empty(), "Backend name should be set");
            assert!(
                !backend_info.description.is_empty(),
                "Backend description should be set"
            );
        }
    }

    #[test]
    fn test_backend_info_default_marked() {
        let info = get_backend_info();

        let default_count = info.iter().filter(|b| b.is_default).count();
        assert!(
            default_count >= 1,
            "At least one backend should be marked as default"
        );
    }

    #[cfg(all(target_os = "windows", feature = "asio"))]
    mod asio_tests {
        use super::*;

        #[test]
        fn test_asio_backend_exists() {
            let _asio = AudioBackend::Asio;
        }

        #[test]
        fn test_asio_backend_name() {
            let backend = AudioBackend::Asio;
            assert_eq!(backend.name(), "ASIO");
        }

        #[test]
        fn test_asio_description_not_empty() {
            let backend = AudioBackend::Asio;
            assert!(!backend.description().is_empty());
        }

        #[test]
        fn test_asio_to_cpal_host_graceful() {
            let backend = AudioBackend::Asio;
            // May succeed or fail depending on ASIO drivers, but should not panic
            let _result = backend.to_cpal_host();
        }

        #[test]
        fn test_asio_is_available_no_panic() {
            let backend = AudioBackend::Asio;
            // Should not panic even if ASIO is unavailable
            let _available = backend.is_available();
        }

        #[test]
        fn test_asio_in_available_backends_if_available() {
            let backend = AudioBackend::Asio;
            if backend.is_available() {
                let backends = list_available_backends();
                assert!(
                    backends.contains(&AudioBackend::Asio),
                    "ASIO should be in available backends when available"
                );
            }
        }
    }

    #[cfg(feature = "jack")]
    mod jack_tests {
        use super::*;

        #[test]
        fn test_jack_backend_exists() {
            let _jack = AudioBackend::Jack;
        }

        #[test]
        fn test_jack_backend_name() {
            let backend = AudioBackend::Jack;
            assert_eq!(backend.name(), "JACK");
        }

        #[test]
        fn test_jack_description_not_empty() {
            let backend = AudioBackend::Jack;
            assert!(!backend.description().is_empty());
        }

        #[test]
        fn test_jack_to_cpal_host_graceful() {
            let backend = AudioBackend::Jack;
            // May succeed or fail depending on JACK server, but should not panic
            let _result = backend.to_cpal_host();
        }

        #[test]
        fn test_jack_is_available_no_panic() {
            let backend = AudioBackend::Jack;
            // Should not panic even if JACK is not running
            let _available = backend.is_available();
        }

        #[test]
        fn test_jack_in_available_backends_if_available() {
            let backend = AudioBackend::Jack;
            if backend.is_available() {
                let backends = list_available_backends();
                assert!(
                    backends.contains(&AudioBackend::Jack),
                    "JACK should be in available backends when available"
                );
            }
        }
    }

    #[test]
    fn test_backend_serialization() {
        let backend = AudioBackend::Default;
        let json = serde_json::to_string(&backend).expect("Failed to serialize");
        let restored: AudioBackend = serde_json::from_str(&json).expect("Failed to deserialize");
        assert_eq!(backend, restored);
    }

    #[test]
    fn test_backend_equality() {
        assert_eq!(AudioBackend::Default, AudioBackend::Default);
    }

    #[test]
    fn test_backend_clone() {
        let backend = AudioBackend::Default;
        let cloned = backend.clone();
        assert_eq!(backend, cloned);
    }

    #[test]
    fn test_backend_copy() {
        let backend = AudioBackend::Default;
        let copied = backend;
        assert_eq!(backend, copied);
    }

    #[test]
    fn test_backend_hash() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(AudioBackend::Default);
        assert!(set.contains(&AudioBackend::Default));
    }
}

// ============================================================================
// 7. Capability Detection Tests (Integration)
// ============================================================================

mod capability_detection {
    use super::*;

    /// Helper to get a device for testing, returns None in CI without audio
    fn get_test_device() -> Option<cpal::Device> {
        find_device_by_name(AudioBackend::Default, &get_any_device_name()?).ok()
    }

    fn get_any_device_name() -> Option<String> {
        list_devices(AudioBackend::Default)
            .ok()
            .and_then(|d| d.first().map(|dev| dev.name.clone()))
    }

    #[test]
    fn test_detect_device_capabilities_returns_valid_data() {
        if let Some(device) = get_test_device() {
            let caps = detect_device_capabilities(&device, AudioBackend::Default);

            // Basic invariants
            assert!(!caps.sample_rates.is_empty(), "Should detect sample rates");
            assert!(!caps.bit_depths.is_empty(), "Should detect bit depths");
            assert!(caps.max_channels > 0, "Should detect at least one channel");
        } else {
            eprintln!("Note: No audio device available for capability detection test");
        }
    }

    #[test]
    fn test_detected_sample_rates_are_sorted() {
        if let Some(device) = get_test_device() {
            let caps = detect_device_capabilities(&device, AudioBackend::Default);

            for i in 1..caps.sample_rates.len() {
                assert!(
                    caps.sample_rates[i - 1] <= caps.sample_rates[i],
                    "Detected sample rates should be sorted"
                );
            }
        }
    }

    #[test]
    fn test_detected_bit_depths_are_sorted() {
        if let Some(device) = get_test_device() {
            let caps = detect_device_capabilities(&device, AudioBackend::Default);

            for i in 1..caps.bit_depths.len() {
                assert!(
                    caps.bit_depths[i - 1].bits() <= caps.bit_depths[i].bits(),
                    "Detected bit depths should be sorted by bits"
                );
            }
        }
    }

    #[test]
    fn test_get_device_capabilities_by_name() {
        if let Some(device_name) = get_any_device_name() {
            let result = get_device_capabilities(AudioBackend::Default, &device_name);
            assert!(
                result.is_ok(),
                "Should get capabilities for existing device"
            );
        }
    }

    #[test]
    fn test_get_device_capabilities_invalid_name() {
        let result = get_device_capabilities(AudioBackend::Default, "NonExistentDevice12345");
        assert!(result.is_err(), "Should fail for nonexistent device");
    }

    #[test]
    fn test_dsd_detection_consistency() {
        if let Some(device) = get_test_device() {
            let caps = detect_device_capabilities(&device, AudioBackend::Default);

            // If DSD is supported, there should be DSD rates
            if caps.supports_dsd {
                assert!(
                    !caps.dsd_rates.is_empty(),
                    "If supports_dsd is true, dsd_rates should not be empty"
                );
            }

            // If there are DSD rates, supports_dsd should be true
            if !caps.dsd_rates.is_empty() {
                assert!(
                    caps.supports_dsd,
                    "If dsd_rates is not empty, supports_dsd should be true"
                );
            }
        }
    }

    #[test]
    fn test_common_sample_rate_detection() {
        if let Some(device) = get_test_device() {
            let caps = detect_device_capabilities(&device, AudioBackend::Default);

            // Most devices should support at least 44.1kHz or 48kHz
            let has_standard_rate =
                caps.sample_rates.contains(&44100) || caps.sample_rates.contains(&48000);

            if !has_standard_rate {
                eprintln!(
                    "Warning: Device does not support 44.1 or 48kHz. Rates: {:?}",
                    caps.sample_rates
                );
            }
        }
    }
}

// ============================================================================
// 8. Edge Cases and Error Handling
// ============================================================================

mod edge_cases {
    use super::*;

    #[test]
    fn test_graceful_handling_no_devices() {
        // This test verifies the code handles environments without audio gracefully
        // The actual behavior depends on the environment, but it should not panic

        let result = list_devices(AudioBackend::Default);
        match result {
            Ok(devices) => {
                eprintln!("System has {} devices", devices.len());
                // If we got here, enumeration succeeded
            }
            Err(DeviceError::NoDeviceFound) => {
                eprintln!("No devices found (expected in some CI environments)");
            }
            Err(DeviceError::EnumerationFailed(msg)) => {
                eprintln!("Enumeration failed: {} (may be expected in CI)", msg);
            }
            Err(e) => {
                eprintln!("Other error: {} (gracefully handled)", e);
            }
        }
        // Test passes if we didn't panic
    }

    #[test]
    fn test_backend_unavailable_error() {
        // The default backend should always be available, but let's test the error type
        let error = DeviceError::BackendUnavailable("TestBackend");
        assert!(
            error.to_string().contains("TestBackend"),
            "Error message should contain backend name"
        );
    }

    #[test]
    fn test_device_not_found_error() {
        let error = DeviceError::DeviceNotFound("TestDevice".to_string());
        assert!(
            error.to_string().contains("TestDevice"),
            "Error message should contain device name"
        );
    }

    #[test]
    fn test_enumeration_failed_error() {
        let error = DeviceError::EnumerationFailed("Test failure".to_string());
        assert!(
            error.to_string().contains("Test failure"),
            "Error message should contain failure reason"
        );
    }

    #[test]
    fn test_device_info_failed_error() {
        let error = DeviceError::DeviceInfoFailed("Info error".to_string());
        assert!(
            error.to_string().contains("Info error"),
            "Error message should contain info error"
        );
    }

    #[test]
    fn test_backend_error_conversion() {
        let backend_err = BackendError::BackendUnavailable("Test");
        let device_err: DeviceError = backend_err.into();

        assert!(matches!(device_err, DeviceError::BackendUnavailable(_)));
    }

    #[test]
    fn test_special_characters_in_device_name_search() {
        // Test that special characters don't cause issues
        let special_names = [
            "Device with spaces",
            "Device-with-dashes",
            "Device_with_underscores",
            "Device (with parens)",
            "Device [with brackets]",
            "Device/with/slashes",
            "Device\\with\\backslashes",
            "Device:with:colons",
            "Device'with'quotes",
            "Device\"with\"doublequotes",
        ];

        for name in &special_names {
            let result = find_device_by_name(AudioBackend::Default, name);
            // Should return DeviceNotFound, not panic or crash
            assert!(
                result.is_err(),
                "Should handle special characters in device name: {}",
                name
            );
        }
    }

    #[test]
    fn test_unicode_device_name_search() {
        let unicode_names = [
            "Dispositivo de audio",
            "Audiogeraet",
            "Appareil audio",
            "Zvukove zariadenie",
        ];

        for name in &unicode_names {
            let result = find_device_by_name(AudioBackend::Default, name);
            // Should handle gracefully
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_very_long_device_name_search() {
        let long_name = "A".repeat(10000);
        let result = find_device_by_name(AudioBackend::Default, &long_name);
        // Should handle gracefully without panic or memory issues
        assert!(result.is_err());
    }

    #[test]
    fn test_repeated_enumeration() {
        // Enumerate devices multiple times to check for resource leaks or race conditions
        for i in 0..5 {
            let result = list_devices(AudioBackend::Default);
            match result {
                Ok(devices) => {
                    eprintln!("Enumeration {}: {} devices", i, devices.len());
                }
                Err(e) => {
                    eprintln!("Enumeration {} failed: {}", i, e);
                }
            }
        }
    }

    #[test]
    fn test_repeated_capability_detection() {
        let result = list_devices(AudioBackend::Default);

        if let Ok(devices) = result {
            if let Some(first) = devices.first() {
                // Get capabilities multiple times for the same device
                for i in 0..3 {
                    let caps = get_device_capabilities(AudioBackend::Default, &first.name);
                    if let Ok(caps) = caps {
                        eprintln!(
                            "Capability detection {}: {} sample rates",
                            i,
                            caps.sample_rates.len()
                        );
                    }
                }
            }
        }
    }
}

// ============================================================================
// 9. Property Tests
// ============================================================================

mod property_tests {
    use super::*;

    #[test]
    fn test_all_sample_rates_positive() {
        let result = list_devices_with_capabilities(AudioBackend::Default, true);

        if let Ok(devices) = result {
            for device in &devices {
                // Native sample rate should be positive
                assert!(
                    device.sample_rate > 0,
                    "Device {} has invalid sample rate: {}",
                    device.name,
                    device.sample_rate
                );

                if let Some(caps) = &device.capabilities {
                    for &rate in &caps.sample_rates {
                        assert!(rate > 0, "All sample rates should be positive");
                    }
                }
            }
        }
    }

    #[test]
    fn test_sample_rates_reasonable_range() {
        let result = list_devices_with_capabilities(AudioBackend::Default, true);

        if let Ok(devices) = result {
            for device in &devices {
                if let Some(caps) = &device.capabilities {
                    for &rate in &caps.sample_rates {
                        assert!(rate >= 8000, "Sample rate {} is below minimum (8kHz)", rate);
                        assert!(
                            rate <= 1_000_000,
                            "Sample rate {} exceeds maximum (1MHz)",
                            rate
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_max_channels_reasonable_range() {
        let result = list_devices_with_capabilities(AudioBackend::Default, true);

        if let Ok(devices) = result {
            for device in &devices {
                // Native channel count should be reasonable
                assert!(
                    device.channels >= 1,
                    "Device should have at least 1 channel"
                );
                assert!(
                    device.channels <= 64,
                    "Device channel count {} is unreasonably high",
                    device.channels
                );

                if let Some(caps) = &device.capabilities {
                    assert!(caps.max_channels >= 1, "Max channels should be at least 1");
                    assert!(
                        caps.max_channels <= 64,
                        "Max channels {} is unreasonably high",
                        caps.max_channels
                    );
                }
            }
        }
    }

    #[test]
    fn test_buffer_size_ranges_valid() {
        let result = list_devices_with_capabilities(AudioBackend::Default, true);

        if let Ok(devices) = result {
            for device in &devices {
                if let Some(caps) = &device.capabilities {
                    // If both min and max are set, min should be <= max
                    if let (Some(min), Some(max)) = (caps.min_buffer_frames, caps.max_buffer_frames)
                    {
                        assert!(
                            min <= max,
                            "Min buffer {} should be <= max buffer {}",
                            min,
                            max
                        );
                    }

                    // If set, buffer sizes should be reasonable
                    if let Some(min) = caps.min_buffer_frames {
                        assert!(min > 0, "Min buffer should be positive");
                        assert!(min <= 65536, "Min buffer {} is unreasonably large", min);
                    }

                    if let Some(max) = caps.max_buffer_frames {
                        assert!(max > 0, "Max buffer should be positive");
                        assert!(max <= 1048576, "Max buffer {} is unreasonably large", max);
                    }
                }
            }
        }
    }

    #[test]
    fn test_sample_rate_range_valid() {
        let result = list_devices(AudioBackend::Default);

        if let Ok(devices) = result {
            for device in &devices {
                if let Some((min, max)) = device.sample_rate_range {
                    assert!(min > 0, "Min sample rate should be positive");
                    assert!(max > 0, "Max sample rate should be positive");
                    assert!(
                        min <= max,
                        "Min sample rate {} should be <= max {}",
                        min,
                        max
                    );
                }
            }
        }
    }

    #[test]
    fn test_dsd_rates_are_valid() {
        let result = list_devices_with_capabilities(AudioBackend::Default, true);

        if let Ok(devices) = result {
            for device in &devices {
                if let Some(caps) = &device.capabilities {
                    for &dsd_rate in &caps.dsd_rates {
                        // DSD rates should be multiples of 44.1kHz
                        assert!(
                            dsd_rate % 44100 == 0,
                            "DSD rate {} should be multiple of 44.1kHz",
                            dsd_rate
                        );

                        // DSD rates should be at least DSD64 (2.8224 MHz)
                        assert!(
                            dsd_rate >= 2_822_400,
                            "DSD rate {} is below DSD64",
                            dsd_rate
                        );

                        // DSD rates should be at most DSD512 or similar
                        assert!(
                            dsd_rate <= 50_000_000,
                            "DSD rate {} is unreasonably high",
                            dsd_rate
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_bit_depths_not_empty() {
        let result = list_devices_with_capabilities(AudioBackend::Default, true);

        if let Ok(devices) = result {
            for device in &devices {
                if let Some(caps) = &device.capabilities {
                    assert!(
                        !caps.bit_depths.is_empty(),
                        "Device {} should support at least one bit depth",
                        device.name
                    );
                }
            }
        }
    }

    #[test]
    fn test_sample_rates_not_empty() {
        let result = list_devices_with_capabilities(AudioBackend::Default, true);

        if let Ok(devices) = result {
            for device in &devices {
                if let Some(caps) = &device.capabilities {
                    assert!(
                        !caps.sample_rates.is_empty(),
                        "Device {} should support at least one sample rate",
                        device.name
                    );
                }
            }
        }
    }

    #[test]
    fn test_device_backend_matches_request() {
        let result = list_devices(AudioBackend::Default);

        if let Ok(devices) = result {
            for device in &devices {
                assert_eq!(
                    device.backend,
                    AudioBackend::Default,
                    "Device backend should match requested backend"
                );
            }
        }
    }

    #[test]
    fn test_exactly_one_default_device() {
        let result = list_devices(AudioBackend::Default);

        if let Ok(devices) = result {
            if !devices.is_empty() {
                let default_count = devices.iter().filter(|d| d.is_default).count();
                assert!(
                    default_count <= 1,
                    "Should have at most one default device, found {}",
                    default_count
                );
            }
        }
    }
}

// ============================================================================
// 10. AudioDeviceInfo Tests
// ============================================================================

mod audio_device_info {
    use super::*;

    #[test]
    fn test_audio_device_info_serialization() {
        let device_info = AudioDeviceInfo {
            name: "Test Device".to_string(),
            backend: AudioBackend::Default,
            is_default: true,
            sample_rate: 48000,
            channels: 2,
            sample_rate_range: Some((44100, 192000)),
            capabilities: None,
        };

        let json = serde_json::to_string(&device_info).expect("Failed to serialize");
        let restored: AudioDeviceInfo = serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(device_info.name, restored.name);
        assert_eq!(device_info.backend, restored.backend);
        assert_eq!(device_info.is_default, restored.is_default);
        assert_eq!(device_info.sample_rate, restored.sample_rate);
        assert_eq!(device_info.channels, restored.channels);
        assert_eq!(device_info.sample_rate_range, restored.sample_rate_range);
    }

    #[test]
    fn test_audio_device_info_with_capabilities_serialization() {
        let device_info = AudioDeviceInfo {
            name: "Pro Audio Interface".to_string(),
            backend: AudioBackend::Default,
            is_default: false,
            sample_rate: 96000,
            channels: 8,
            sample_rate_range: Some((44100, 384000)),
            capabilities: Some(DeviceCapabilities {
                sample_rates: vec![44100, 48000, 88200, 96000, 176400, 192000],
                bit_depths: vec![SupportedBitDepth::Int24, SupportedBitDepth::Float32],
                max_channels: 8,
                supports_exclusive: true,
                supports_dsd: false,
                dsd_rates: vec![],
                min_buffer_frames: Some(64),
                max_buffer_frames: Some(4096),
                has_hardware_volume: true,
            }),
        };

        let json = serde_json::to_string(&device_info).expect("Failed to serialize");
        let restored: AudioDeviceInfo = serde_json::from_str(&json).expect("Failed to deserialize");

        assert!(restored.capabilities.is_some());
        let caps = restored.capabilities.unwrap();
        assert_eq!(caps.sample_rates.len(), 6);
        assert_eq!(caps.max_channels, 8);
        assert!(caps.supports_exclusive);
    }

    #[test]
    fn test_audio_device_info_clone() {
        let result = list_devices(AudioBackend::Default);

        if let Ok(devices) = result {
            if let Some(device) = devices.first() {
                let cloned = device.clone();
                assert_eq!(device.name, cloned.name);
                assert_eq!(device.backend, cloned.backend);
                assert_eq!(device.is_default, cloned.is_default);
            }
        }
    }
}

// ============================================================================
// 11. Concurrent Access Tests
// ============================================================================

mod concurrent_tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_concurrent_device_enumeration() {
        let handles: Vec<_> = (0..4)
            .map(|i| {
                thread::spawn(move || {
                    let result = list_devices(AudioBackend::Default);
                    eprintln!("Thread {}: {:?}", i, result.as_ref().map(|d| d.len()));
                    result
                })
            })
            .collect();

        for handle in handles {
            let result = handle.join().expect("Thread panicked");
            // All threads should either succeed or fail gracefully
            match result {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("Thread enumeration error (expected in CI): {}", e);
                }
            }
        }
    }

    #[test]
    fn test_concurrent_capability_detection() {
        let result = list_devices(AudioBackend::Default);
        let device_name = result
            .ok()
            .and_then(|d| d.first().map(|dev| dev.name.clone()));

        if let Some(name) = device_name {
            let handles: Vec<_> = (0..4)
                .map(|i| {
                    let name_clone = name.clone();
                    thread::spawn(move || {
                        let result = get_device_capabilities(AudioBackend::Default, &name_clone);
                        eprintln!(
                            "Thread {} capability result: {:?}",
                            i,
                            result.as_ref().map(|c| c.sample_rates.len())
                        );
                        result
                    })
                })
                .collect();

            for handle in handles {
                let result = handle.join().expect("Thread panicked");
                // Should succeed or fail gracefully
                match result {
                    Ok(caps) => {
                        assert!(!caps.sample_rates.is_empty());
                    }
                    Err(e) => {
                        eprintln!("Capability detection error: {}", e);
                    }
                }
            }
        }
    }

    #[test]
    fn test_concurrent_backend_enumeration() {
        let handles: Vec<_> = (0..4)
            .map(|i| {
                thread::spawn(move || {
                    let backends = list_available_backends();
                    eprintln!("Thread {}: found {} backends", i, backends.len());
                    backends
                })
            })
            .collect();

        for handle in handles {
            let backends = handle.join().expect("Thread panicked");
            assert!(!backends.is_empty());
            assert!(backends.contains(&AudioBackend::Default));
        }
    }
}

// ============================================================================
// 12. Integration Smoke Tests
// ============================================================================

mod smoke_tests {
    use super::*;

    /// Complete workflow: list backends, enumerate devices, get capabilities
    #[test]
    fn test_full_discovery_workflow() {
        eprintln!("=== Full Device Discovery Workflow ===");

        // Step 1: List available backends
        let backends = list_available_backends();
        eprintln!("Step 1: Found {} backends", backends.len());
        assert!(!backends.is_empty());

        for backend in &backends {
            eprintln!("  - {} ({})", backend.name(), backend.description());
        }

        // Step 2: For each backend, list devices
        for backend in backends {
            eprintln!("\nStep 2: Enumerating devices for {:?}", backend);

            match list_devices_with_capabilities(backend, true) {
                Ok(devices) => {
                    eprintln!("  Found {} devices", devices.len());

                    for device in &devices {
                        eprintln!(
                            "  - {} (default: {}, {}Hz, {} ch)",
                            device.name, device.is_default, device.sample_rate, device.channels
                        );

                        if let Some(caps) = &device.capabilities {
                            eprintln!(
                                "    Capabilities: {} sample rates, {} bit depths, {} max ch",
                                caps.sample_rates.len(),
                                caps.bit_depths.len(),
                                caps.max_channels
                            );
                            eprintln!(
                                "    DSD: {} (rates: {:?})",
                                caps.supports_dsd, caps.dsd_rates
                            );
                        }
                    }
                }
                Err(e) => {
                    eprintln!("  Error: {}", e);
                }
            }
        }

        eprintln!("\n=== Workflow Complete ===");
    }

    #[test]
    fn test_default_device_workflow() {
        eprintln!("=== Default Device Workflow ===");

        match get_default_device_with_capabilities(AudioBackend::Default, true) {
            Ok(device) => {
                eprintln!("Default device: {}", device.name);
                eprintln!("  Sample rate: {} Hz", device.sample_rate);
                eprintln!("  Channels: {}", device.channels);

                if let Some(caps) = &device.capabilities {
                    eprintln!("  Supported sample rates: {:?}", caps.sample_rates);
                    eprintln!(
                        "  Supported bit depths: {:?}",
                        caps.bit_depths
                            .iter()
                            .map(|d| d.display_name())
                            .collect::<Vec<_>>()
                    );
                    eprintln!("  Max channels: {}", caps.max_channels);
                    eprintln!("  Exclusive mode: {}", caps.supports_exclusive);
                    eprintln!("  DSD support: {}", caps.supports_dsd);
                }
            }
            Err(e) => {
                eprintln!("Could not get default device: {}", e);
            }
        }
    }

    #[test]
    fn test_backend_info_workflow() {
        eprintln!("=== Backend Info Workflow ===");

        let backend_info = get_backend_info();
        eprintln!("Found {} backends", backend_info.len());

        for info in &backend_info {
            eprintln!("Backend: {}", info.name);
            eprintln!("  Description: {}", info.description);
            eprintln!("  Available: {}", info.available);
            eprintln!("  Is default: {}", info.is_default);
            eprintln!("  Device count: {}", info.device_count);
        }

        // At least one should be available
        assert!(backend_info.iter().any(|b| b.available));
    }
}
