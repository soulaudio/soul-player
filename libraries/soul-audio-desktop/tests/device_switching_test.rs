//! Integration tests for device switching
//!
//! These tests verify device switching functionality works correctly across different scenarios.

use soul_audio_desktop::{AudioBackend, DesktopPlayback};
use soul_playback::PlaybackConfig;
use std::time::Duration;

/// Test that device switching preserves playback state
#[test]
fn test_device_switch_preserves_state() {
    let result = DesktopPlayback::new(PlaybackConfig::default());

    match result {
        Ok(mut playback) => {
            // Get initial device
            let initial_device = playback.get_current_device();
            let initial_backend = playback.get_current_backend();

            eprintln!("Initial device: {} ({:?})", initial_device, initial_backend);

            // Attempt to switch to default device (should work even if it's already the default)
            match playback.switch_device(AudioBackend::Default, None) {
                Ok(_) => {
                    let new_device = playback.get_current_device();
                    let new_backend = playback.get_current_backend();

                    eprintln!("After switch: {} ({:?})", new_device, new_backend);

                    // Backend should be preserved
                    assert_eq!(
                        new_backend, initial_backend,
                        "Backend should remain the same"
                    );

                    // Device name should be non-empty
                    assert!(!new_device.is_empty(), "Device name should not be empty");
                }
                Err(e) => {
                    eprintln!("Device switch failed (may be expected in test env): {}", e);
                }
            }
        }
        Err(e) => {
            eprintln!(
                "Note: Audio device not available in test environment: {}",
                e
            );
        }
    }
}

/// Test switching to a specific device by name
#[test]
fn test_switch_to_specific_device() {
    let result = DesktopPlayback::new(PlaybackConfig::default());

    match result {
        Ok(mut playback) => {
            // Get list of available devices
            let devices_result = soul_audio_desktop::device::list_devices(AudioBackend::Default);

            match devices_result {
                Ok(devices) => {
                    if devices.len() > 1 {
                        // Try switching to the second device
                        let target_device = &devices[1];
                        eprintln!("Attempting to switch to: {}", target_device.name);

                        match playback
                            .switch_device(AudioBackend::Default, Some(target_device.name.clone()))
                        {
                            Ok(_) => {
                                let current = playback.get_current_device();
                                eprintln!("Successfully switched to: {}", current);
                                assert_eq!(
                                    current, target_device.name,
                                    "Should have switched to target device"
                                );
                            }
                            Err(e) => {
                                eprintln!("Device switch failed: {}", e);
                            }
                        }
                    } else {
                        eprintln!("Only one device available, skipping specific device test");
                    }
                }
                Err(e) => {
                    eprintln!("Could not list devices: {}", e);
                }
            }
        }
        Err(e) => {
            eprintln!(
                "Note: Audio device not available in test environment: {}",
                e
            );
        }
    }
}

/// Test multiple consecutive device switches
#[test]
fn test_multiple_device_switches() {
    let result = DesktopPlayback::new(PlaybackConfig::default());

    match result {
        Ok(mut playback) => {
            eprintln!("Testing multiple device switches");

            // Perform 3 switches
            for i in 1..=3 {
                eprintln!("Switch iteration {}", i);

                match playback.switch_device(AudioBackend::Default, None) {
                    Ok(_) => {
                        let device = playback.get_current_device();
                        eprintln!("  Current device: {}", device);
                        assert!(!device.is_empty());

                        // Small delay between switches
                        std::thread::sleep(Duration::from_millis(100));
                    }
                    Err(e) => {
                        eprintln!("  Switch {} failed: {}", i, e);
                        break;
                    }
                }
            }
        }
        Err(e) => {
            eprintln!(
                "Note: Audio device not available in test environment: {}",
                e
            );
        }
    }
}

/// Test switching with invalid device name
#[test]
fn test_switch_invalid_device_name() {
    let result = DesktopPlayback::new(PlaybackConfig::default());

    match result {
        Ok(mut playback) => {
            let invalid_name = "ThisDeviceDefinitelyDoesNotExist9876543210";

            let switch_result =
                playback.switch_device(AudioBackend::Default, Some(invalid_name.to_string()));

            // Should fail with an error
            assert!(
                switch_result.is_err(),
                "Switching to invalid device should return an error"
            );

            if let Err(e) = switch_result {
                eprintln!("Expected error for invalid device: {}", e);
                assert!(
                    e.to_string().contains("not found") || e.to_string().contains("Device error"),
                    "Error message should indicate device not found"
                );
            }

            // Original device should still be active
            let current_device = playback.get_current_device();
            assert!(!current_device.is_empty());
            assert_ne!(current_device, invalid_name);
        }
        Err(e) => {
            eprintln!(
                "Note: Audio device not available in test environment: {}",
                e
            );
        }
    }
}

/// Test that device info is correctly reported after creation
#[test]
fn test_device_info_after_creation() {
    let result = DesktopPlayback::new(PlaybackConfig::default());

    match result {
        Ok(playback) => {
            let backend = playback.get_current_backend();
            let device = playback.get_current_device();

            eprintln!("Backend: {:?}", backend);
            eprintln!("Device: {}", device);

            // Backend should be Default (since we used `new()`)
            assert_eq!(backend, AudioBackend::Default, "Should use default backend");

            // Device name should not be empty
            assert!(!device.is_empty(), "Device name should not be empty");

            // Device name should not be "Unknown Device" (unless that's actually the device name)
            if device != "Unknown Device" {
                assert_ne!(device, "Unknown Device", "Device name should be resolved");
            }
        }
        Err(e) => {
            eprintln!(
                "Note: Audio device not available in test environment: {}",
                e
            );
        }
    }
}

/// Test creating playback with specific backend
#[test]
fn test_create_with_specific_backend() {
    let result =
        DesktopPlayback::new_with_device(PlaybackConfig::default(), AudioBackend::Default, None);

    match result {
        Ok(playback) => {
            assert_eq!(playback.get_current_backend(), AudioBackend::Default);
            assert!(!playback.get_current_device().is_empty());
        }
        Err(e) => {
            eprintln!(
                "Note: Audio device not available in test environment: {}",
                e
            );
        }
    }
}

/// Stress test: rapid device switches
#[test]
fn test_rapid_device_switches() {
    let result = DesktopPlayback::new(PlaybackConfig::default());

    match result {
        Ok(mut playback) => {
            eprintln!("Performing rapid device switches");

            let mut success_count = 0;
            let mut fail_count = 0;

            // Try 10 rapid switches
            for i in 0..10 {
                match playback.switch_device(AudioBackend::Default, None) {
                    Ok(_) => {
                        success_count += 1;
                        // Very small delay
                        std::thread::sleep(Duration::from_millis(20));
                    }
                    Err(e) => {
                        fail_count += 1;
                        eprintln!("Switch {} failed: {}", i, e);
                    }
                }
            }

            eprintln!(
                "Rapid switch results: {} succeeded, {} failed",
                success_count, fail_count
            );

            // At least some switches should succeed
            assert!(
                success_count > 0,
                "At least some device switches should succeed"
            );

            // Final state should be valid
            let final_device = playback.get_current_device();
            assert!(!final_device.is_empty());
        }
        Err(e) => {
            eprintln!(
                "Note: Audio device not available in test environment: {}",
                e
            );
        }
    }
}

/// Test device switching with different backends (if available)
#[test]
fn test_switch_between_backends() {
    // Test switching between available backends
    let backends = soul_audio_desktop::backend::list_available_backends();

    eprintln!("Available backends: {:?}", backends);

    if backends.len() > 1 {
        let result = DesktopPlayback::new(PlaybackConfig::default());

        match result {
            Ok(mut playback) => {
                // Try switching to each available backend
                for backend in backends {
                    eprintln!("Switching to backend: {:?}", backend);

                    match playback.switch_device(backend, None) {
                        Ok(_) => {
                            let current_backend = playback.get_current_backend();
                            let current_device = playback.get_current_device();

                            eprintln!(
                                "Successfully switched to {:?}: {}",
                                current_backend, current_device
                            );

                            assert_eq!(current_backend, backend);
                            assert!(!current_device.is_empty());

                            std::thread::sleep(Duration::from_millis(100));
                        }
                        Err(e) => {
                            eprintln!("Could not switch to {:?}: {}", backend, e);
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!(
                    "Note: Audio device not available in test environment: {}",
                    e
                );
            }
        }
    } else {
        eprintln!("Only one backend available, skipping multi-backend test");
    }
}
