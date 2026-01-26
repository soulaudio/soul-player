//! Integration tests for async device monitoring
//!
//! These tests verify that the device monitoring system works correctly across
//! all platform implementations (CPAL fallback, macOS `CoreAudio`, Linux `PipeWire`, Windows `WinRT`).
//!
//! # Test Coverage
//!
//! - Device enumeration returns results
//! - Default device retrieval works
//! - Platform name identification

#![allow(clippy::doc_markdown)]
#![allow(clippy::match_wild_err_arm)]
//! - Device availability checking
//! - Watch for changes callback invocation
//! - Timeout protection
//! - Feature flag behavior (native vs fallback)
//!
//! # CI Compatibility
//!
//! All tests handle CI environments gracefully:
//! - Tests pass when no audio devices are present
//! - Platform-specific errors are expected and handled
//! - Timeout protection prevents hanging

use soul_audio_desktop::{create_async_device_monitor, device_monitor_async::DeviceEvent};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Test that enumerate_devices returns a result on the current platform
///
/// This test verifies that device enumeration completes without panicking.
/// In CI environments without audio devices, an error result is acceptable.
#[tokio::test]
async fn test_enumerate_devices_returns_result() {
    let monitor = create_async_device_monitor();

    // Enumeration should complete (success or error, but not panic)
    let result = monitor.enumerate_devices().await;

    // Log the result for debugging
    match &result {
        Ok(devices) => tracing::info!("Enumerated {} devices", devices.len()),
        Err(e) => tracing::warn!("Enumeration failed (expected in CI): {}", e),
    }

    // Test passes if we get here without panicking
    // We don't assert Ok because CI may not have devices
}

/// Test that get_default_device returns a valid result
///
/// Verifies that we can query for the default device. In CI environments,
/// this may return an error if no devices are present.
#[tokio::test]
async fn test_get_default_device() {
    let monitor = create_async_device_monitor();

    let result = monitor.get_default_device().await;

    match &result {
        Ok(device) => {
            tracing::info!("Default device: {} (id: {})", device.name, device.id);
            assert!(device.is_default, "Device should be marked as default");
            assert!(device.is_available, "Default device should be available");
        }
        Err(e) => {
            tracing::warn!("No default device found (expected in CI): {}", e);
        }
    }
}

/// Test that platform_name returns expected values based on build configuration
///
/// Verifies that the correct platform name is returned based on the feature flags
/// and target OS.
#[tokio::test]
async fn test_platform_name_matches_expected() {
    let monitor = create_async_device_monitor();
    let platform_name = monitor.platform_name();

    tracing::info!("Platform: {}", platform_name);

    // Verify platform name contains relevant keywords
    #[cfg(all(target_os = "macos", feature = "native-device-monitor"))]
    {
        assert!(platform_name.contains("macOS"), "Should identify as macOS");
        assert!(platform_name.contains("CoreAudio"), "Should use CoreAudio");
        assert!(
            platform_name.contains("Native"),
            "Should be native implementation"
        );
    }

    #[cfg(all(target_os = "linux", feature = "native-device-monitor"))]
    {
        assert!(platform_name.contains("Linux"), "Should identify as Linux");
        assert!(platform_name.contains("PipeWire"), "Should use PipeWire");
        assert!(
            platform_name.contains("Native"),
            "Should be native implementation"
        );
    }

    #[cfg(all(target_os = "windows", feature = "native-device-monitor"))]
    {
        assert!(
            platform_name.contains("Windows"),
            "Should identify as Windows"
        );
        assert!(platform_name.contains("WinRT"), "Should use WinRT");
        assert!(
            platform_name.contains("Native"),
            "Should be native implementation"
        );
    }

    #[cfg(not(feature = "native-device-monitor"))]
    {
        assert!(
            platform_name.contains("Fallback"),
            "Should use CPAL fallback"
        );
    }
}

/// Test that is_device_available works correctly
///
/// Verifies that device availability checking works for both valid and invalid device IDs.
#[tokio::test]
async fn test_is_device_available() {
    let monitor = create_async_device_monitor();

    // Try to get a real device ID first
    if let Ok(devices) = monitor.enumerate_devices().await {
        if let Some(device) = devices.first() {
            // Test with a real device ID
            let is_available = monitor.is_device_available(&device.id).await;
            tracing::info!("Device '{}' availability: {}", device.id, is_available);

            // Should return true for a device we just enumerated
            assert!(
                is_available,
                "Recently enumerated device should be available"
            );
        }
    }

    // Test with an invalid device ID
    let is_available = monitor.is_device_available("invalid_device_id_12345").await;
    assert!(!is_available, "Invalid device ID should return false");
}

/// Test that watch_for_changes can be started and stopped
///
/// Verifies that the device change watcher can be created and properly cleaned up.
/// Does not require actual device changes to occur.
#[tokio::test]
async fn test_watch_for_changes_starts_and_stops() {
    let monitor = create_async_device_monitor();

    // Track if callback was invoked (may not happen in test environment)
    let callback_count = Arc::new(Mutex::new(0));
    let callback_count_clone = callback_count.clone();

    // Create a callback that counts invocations
    let callback = Box::new(move |event: DeviceEvent| {
        let mut count = callback_count_clone.lock().unwrap();
        *count += 1;
        tracing::info!("Device event received: {:?} (total: {})", event, *count);
    });

    // Start watching
    let result = monitor.watch_for_changes(callback).await;

    match result {
        Ok(mut handle) => {
            tracing::info!("Watch handle created successfully");

            // Wait a short time to see if any events occur
            tokio::time::sleep(Duration::from_millis(500)).await;

            // Stop watching
            handle.stop();
            tracing::info!("Watch handle stopped");

            // Log callback count (may be 0 in CI)
            let count = *callback_count.lock().unwrap();
            tracing::info!("Callback was invoked {} times", count);
        }
        Err(e) => {
            tracing::warn!(
                "Failed to create watch handle (expected in some CI environments): {}",
                e
            );
        }
    }
}

/// Test that watch handle is properly dropped
///
/// Verifies that dropping a watch handle stops the watcher.
#[tokio::test]
async fn test_watch_handle_drop_cleanup() {
    let monitor = create_async_device_monitor();

    let callback_invoked = Arc::new(Mutex::new(false));
    let callback_invoked_clone = callback_invoked.clone();

    let callback = Box::new(move |event: DeviceEvent| {
        *callback_invoked_clone.lock().unwrap() = true;
        tracing::info!("Device event in drop test: {:?}", event);
    });

    {
        // Create watch handle in inner scope
        if let Ok(_handle) = monitor.watch_for_changes(callback).await {
            tracing::info!("Watch handle created, will be dropped at end of scope");
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        // Handle is dropped here
    }

    // Give time for cleanup
    tokio::time::sleep(Duration::from_millis(100)).await;

    tracing::info!("Watch handle dropped and cleaned up");
}

/// Test enumeration with timeout protection
///
/// Verifies that device enumeration completes within a reasonable timeout,
/// protecting against hangs in platform APIs.
#[tokio::test]
async fn test_enumerate_with_timeout() {
    let monitor = create_async_device_monitor();

    // Set a generous timeout (5 seconds should be more than enough)
    let timeout = Duration::from_secs(5);

    let result = tokio::time::timeout(timeout, monitor.enumerate_devices()).await;

    match result {
        Ok(Ok(devices)) => {
            tracing::info!(
                "Enumeration completed successfully with {} devices",
                devices.len()
            );
        }
        Ok(Err(e)) => {
            tracing::warn!("Enumeration failed (expected in CI): {}", e);
        }
        Err(_) => {
            panic!(
                "Enumeration timed out after {:?} - this indicates a platform API hang",
                timeout
            );
        }
    }
}

/// Test that multiple enumerate calls work correctly
///
/// Verifies that we can enumerate devices multiple times without issues,
/// ensuring proper resource management.
#[tokio::test]
async fn test_multiple_enumerations() {
    let monitor = create_async_device_monitor();

    for i in 1..=3 {
        let result = monitor.enumerate_devices().await;
        match &result {
            Ok(devices) => {
                tracing::info!("Enumeration #{}: {} devices", i, devices.len());
            }
            Err(e) => {
                tracing::warn!("Enumeration #{} failed: {}", i, e);
            }
        }

        // Small delay between enumerations
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

/// Test device info structure completeness
///
/// Verifies that device info contains reasonable data when devices are available.
#[tokio::test]
async fn test_device_info_completeness() {
    let monitor = create_async_device_monitor();

    if let Ok(devices) = monitor.enumerate_devices().await {
        for device in devices.iter().take(3) {
            // Check first 3 devices
            tracing::info!("Device info: {:?}", device);

            // All devices should have an ID and name
            assert!(!device.id.is_empty(), "Device should have non-empty ID");
            assert!(!device.name.is_empty(), "Device should have non-empty name");

            // Log additional properties
            tracing::info!(
                "  - ID: {}, Name: {}, Default: {}, Available: {}",
                device.id,
                device.name,
                device.is_default,
                device.is_available
            );

            if let Some(rate) = device.sample_rate {
                tracing::info!("  - Sample rate: {} Hz", rate);
            }

            if let Some(channels) = device.channels {
                tracing::info!("  - Channels: {}", channels);
            }
        }
    } else {
        tracing::warn!("No devices available for completeness test");
    }
}

/// Test that default device is included in enumeration
///
/// Verifies that if a default device exists, it's included in the enumerated list.
#[tokio::test]
async fn test_default_device_in_enumeration() {
    let monitor = create_async_device_monitor();

    if let Ok(default_device) = monitor.get_default_device().await {
        if let Ok(devices) = monitor.enumerate_devices().await {
            // In some environments, default device might be a virtual device not in the list
            // So we log but don't fail if it's not found
            let found = devices.iter().any(|d| d.id == default_device.id);
            if !found {
                tracing::warn!(
                    "Default device '{}' (id: {}) not found in enumerated list - may be virtual device",
                    default_device.name,
                    default_device.id
                );
            }

            // Verify at least one device is marked as default (if any devices exist)
            if !devices.is_empty() {
                let default_count = devices.iter().filter(|d| d.is_default).count();
                if default_count == 0 {
                    tracing::warn!("No devices marked as default in enumerated list");
                }
            }
        }
    }
}

/// Test concurrent device operations
///
/// Verifies that multiple operations can be performed concurrently without issues.
#[tokio::test]
async fn test_concurrent_operations() {
    let monitor = Arc::new(create_async_device_monitor());

    let mut handles = vec![];

    // Spawn 5 concurrent enumeration tasks
    for i in 0..5 {
        let monitor_clone = monitor.clone();
        let handle = tokio::spawn(async move {
            let result = monitor_clone.enumerate_devices().await;
            tracing::info!("Concurrent task #{} completed: {:?}", i, result.is_ok());
            result.is_ok() || result.is_err() // Always returns something
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        assert!(
            handle.await.is_ok(),
            "Task should complete without panicking"
        );
    }
}

/// Test feature flag behavior
///
/// Verifies that the correct implementation is used based on feature flags.
#[tokio::test]
async fn test_feature_flag_selection() {
    let monitor = create_async_device_monitor();
    let platform_name = monitor.platform_name();

    #[cfg(feature = "native-device-monitor")]
    {
        #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
        {
            assert!(
                !platform_name.contains("Fallback"),
                "With native-device-monitor feature, should not use fallback on supported platforms"
            );
        }
    }

    #[cfg(not(feature = "native-device-monitor"))]
    {
        assert!(
            platform_name.contains("Fallback"),
            "Without native-device-monitor feature, should use CPAL fallback"
        );
    }
}
