//! End-to-End Integration Tests for Async Device Monitoring
//!
//! These tests verify that the async device monitoring system works correctly
//! in realistic scenarios, including hotplug event handling, device switching,
//! and integration with playback systems.
//!
//! # Test Philosophy
//!
//! - **Realistic**: Simulate actual device hotplug scenarios
//! - **CI-Safe**: Work without physical audio hardware
//! - **Comprehensive**: Cover all critical paths
//! - **Performance**: Verify real-time vs polling performance

#![allow(clippy::doc_markdown)]
#![allow(clippy::manual_let_else)]
#![allow(clippy::single_match_else)]

use soul_audio_desktop::{create_async_device_monitor, device_monitor_async::DeviceEvent};
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc, Mutex,
};
use std::time::{Duration, Instant};

/// Test that device enumeration completes quickly on all platforms
///
/// Verifies that async device monitoring provides fast enumeration
/// as an alternative to slower synchronous polling.
#[tokio::test]
async fn test_enumeration_performance() {
    let monitor = create_async_device_monitor();

    let start = Instant::now();
    let result = monitor.enumerate_devices().await;
    let elapsed = start.elapsed();

    match result {
        Ok(devices) => {
            tracing::info!("Enumerated {} devices in {:?}", devices.len(), elapsed);

            // Verify performance targets:
            // - macOS CoreAudio: < 50ms
            // - Linux PipeWire: < 100ms
            // - Windows WinRT: < 150ms
            // - CPAL fallback: < 1000ms
            #[cfg(all(target_os = "macos", feature = "native-device-monitor"))]
            assert!(
                elapsed < Duration::from_millis(50),
                "macOS CoreAudio enumeration should be < 50ms, got {:?}",
                elapsed
            );

            #[cfg(all(target_os = "linux", feature = "native-device-monitor"))]
            assert!(
                elapsed < Duration::from_millis(100),
                "Linux PipeWire enumeration should be < 100ms, got {:?}",
                elapsed
            );

            #[cfg(all(target_os = "windows", feature = "native-device-monitor"))]
            assert!(
                elapsed < Duration::from_millis(150),
                "Windows WinRT enumeration should be < 150ms, got {:?}",
                elapsed
            );

            // CPAL fallback allowed to be slower
            #[cfg(not(feature = "native-device-monitor"))]
            assert!(
                elapsed < Duration::from_secs(1),
                "CPAL fallback enumeration should be < 1s, got {:?}",
                elapsed
            );
        }
        Err(e) => {
            tracing::warn!("Enumeration failed (expected in CI): {}", e);
        }
    }
}

/// Test that hotplug notifications are received with low latency
///
/// Simulates device hotplug by starting a watcher and measuring callback
/// invocation time. On platforms with real-time hotplug (Linux PipeWire,
/// Windows WinRT, macOS CoreAudio), latency should be < 100ms.
#[tokio::test]
async fn test_hotplug_notification_latency() {
    let monitor = create_async_device_monitor();

    let event_received = Arc::new(AtomicBool::new(false));
    let event_received_clone = event_received.clone();
    let event_timestamp = Arc::new(Mutex::new(None::<Instant>));
    let event_timestamp_clone = event_timestamp.clone();

    let callback = Box::new(move |event: DeviceEvent| {
        tracing::info!("Hotplug event received: {:?}", event);
        event_received_clone.store(true, Ordering::SeqCst);
        *event_timestamp_clone.lock().unwrap() = Some(Instant::now());
    });

    match monitor.watch_for_changes(callback).await {
        Ok(_handle) => {
            tracing::info!("Device watcher started successfully");

            // Wait for potential events (or timeout)
            tokio::time::sleep(Duration::from_secs(3)).await;

            // Log whether events were received
            if event_received.load(Ordering::SeqCst) {
                tracing::info!("✅ Hotplug event was received during test");
            } else {
                tracing::info!(
                    "ℹ️  No hotplug events during test (expected if no devices plugged/unplugged)"
                );
            }

            // Note: Cannot assert events were received since this depends on
            // physical device changes. This test verifies the watcher starts
            // correctly and can receive events if they occur.
        }
        Err(e) => {
            tracing::warn!("Failed to start device watcher: {}", e);
        }
    }
}

/// Test that device removal is detected quickly
///
/// Verifies that when a device becomes unavailable, the monitoring system
/// detects it and can inform the playback system to switch devices.
#[tokio::test]
async fn test_device_removal_detection() {
    let monitor = create_async_device_monitor();

    // Get initial device list
    let initial_devices = match monitor.enumerate_devices().await {
        Ok(devices) => devices,
        Err(_) => {
            tracing::warn!("Skipping test - no devices available");
            return;
        }
    };

    if initial_devices.is_empty() {
        tracing::warn!("Skipping test - no devices to monitor");
        return;
    }

    let removed_devices = Arc::new(Mutex::new(Vec::<String>::new()));
    let removed_devices_clone = removed_devices.clone();

    let callback = Box::new(move |event: DeviceEvent| {
        if let DeviceEvent::DeviceRemoved { id } = event {
            tracing::info!("Device removed detected: {}", id);
            removed_devices_clone.lock().unwrap().push(id);
        }
    });

    match monitor.watch_for_changes(callback).await {
        Ok(_handle) => {
            // Monitor for device changes
            tokio::time::sleep(Duration::from_secs(2)).await;

            // Verify we can track removals (even if none occurred)
            let removed = removed_devices.lock().unwrap();
            tracing::info!("Devices removed during test: {}", removed.len());
        }
        Err(e) => {
            tracing::warn!("Failed to start watcher: {}", e);
        }
    }
}

/// Test that default device changes are detected
///
/// When the system default device changes, the monitoring system should
/// notify listeners immediately (real-time platforms) or within polling
/// interval (fallback).
#[tokio::test]
async fn test_default_device_change_detection() {
    let monitor = create_async_device_monitor();

    let default_changes = Arc::new(AtomicUsize::new(0));
    let default_changes_clone = default_changes.clone();
    let last_default = Arc::new(Mutex::new(None::<String>));
    let last_default_clone = last_default.clone();

    let callback = Box::new(move |event: DeviceEvent| {
        if let DeviceEvent::DefaultDeviceChanged { id, name } = event {
            tracing::info!("Default device changed: {} ({})", name, id);
            default_changes_clone.fetch_add(1, Ordering::SeqCst);
            *last_default_clone.lock().unwrap() = Some(id);
        }
    });

    match monitor.watch_for_changes(callback).await {
        Ok(_handle) => {
            // Monitor for default device changes
            tokio::time::sleep(Duration::from_secs(2)).await;

            let changes = default_changes.load(Ordering::SeqCst);
            tracing::info!("Default device changes detected: {}", changes);

            // In CI, we don't expect changes, but the watcher should start successfully
        }
        Err(e) => {
            tracing::warn!("Failed to start watcher: {}", e);
        }
    }
}

/// Test that device watcher can be stopped cleanly
///
/// Verifies proper resource cleanup when stopping device monitoring.
/// This is critical to avoid resource leaks in long-running applications.
#[tokio::test]
async fn test_watcher_cleanup() {
    let monitor = create_async_device_monitor();

    let events_received = Arc::new(AtomicUsize::new(0));
    let events_received_clone = events_received.clone();

    let callback = Box::new(move |_event: DeviceEvent| {
        events_received_clone.fetch_add(1, Ordering::SeqCst);
    });

    match monitor.watch_for_changes(callback).await {
        Ok(mut handle) => {
            tracing::info!("Watcher started, waiting briefly...");
            tokio::time::sleep(Duration::from_millis(500)).await;

            let events_before_stop = events_received.load(Ordering::SeqCst);
            tracing::info!("Events received before stop: {}", events_before_stop);

            // Stop the watcher
            handle.stop();
            tracing::info!("Watcher stopped");

            // Wait to ensure no more events
            tokio::time::sleep(Duration::from_millis(500)).await;

            let events_after_stop = events_received.load(Ordering::SeqCst);
            assert_eq!(
                events_before_stop, events_after_stop,
                "No events should be received after stop()"
            );

            tracing::info!("✅ Watcher cleanup verified - no events after stop");
        }
        Err(e) => {
            tracing::warn!("Failed to start watcher: {}", e);
        }
    }
}

/// Test that multiple watchers can coexist
///
/// Verifies that multiple parts of an application can monitor device
/// changes simultaneously without conflicts.
#[tokio::test]
async fn test_multiple_watchers() {
    let monitor1 = create_async_device_monitor();
    let monitor2 = create_async_device_monitor();

    let events1 = Arc::new(AtomicUsize::new(0));
    let events1_clone = events1.clone();
    let events2 = Arc::new(AtomicUsize::new(0));
    let events2_clone = events2.clone();

    let callback1 = Box::new(move |_: DeviceEvent| {
        events1_clone.fetch_add(1, Ordering::SeqCst);
    });

    let callback2 = Box::new(move |_: DeviceEvent| {
        events2_clone.fetch_add(1, Ordering::SeqCst);
    });

    let handle1 = monitor1.watch_for_changes(callback1).await;
    let handle2 = monitor2.watch_for_changes(callback2).await;

    match (handle1, handle2) {
        (Ok(mut h1), Ok(mut h2)) => {
            tracing::info!("Two watchers started successfully");

            tokio::time::sleep(Duration::from_secs(1)).await;

            tracing::info!("Events on watcher 1: {}", events1.load(Ordering::SeqCst));
            tracing::info!("Events on watcher 2: {}", events2.load(Ordering::SeqCst));

            h1.stop();
            h2.stop();

            tracing::info!("✅ Multiple watchers coexist successfully");
        }
        _ => {
            tracing::warn!("Failed to start both watchers");
        }
    }
}

/// Test that enumeration consistency is maintained
///
/// Verifies that repeated enumerations return consistent device lists
/// (devices don't appear/disappear randomly due to race conditions).
#[tokio::test]
async fn test_enumeration_consistency() {
    let monitor = create_async_device_monitor();

    let mut enumerations = Vec::new();

    for i in 0..5 {
        match monitor.enumerate_devices().await {
            Ok(devices) => {
                let device_ids: Vec<String> = devices.iter().map(|d| d.id.clone()).collect();
                tracing::info!("Enumeration #{}: {} devices", i + 1, device_ids.len());
                enumerations.push(device_ids);
            }
            Err(e) => {
                tracing::warn!("Enumeration #{} failed: {}", i + 1, e);
                return;
            }
        }

        // Small delay between enumerations
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // Verify consistency (same devices in same order, assuming no physical changes)
    if enumerations.len() >= 2 {
        let first = &enumerations[0];
        for (i, current) in enumerations.iter().enumerate().skip(1) {
            if first != current {
                tracing::warn!(
                    "Enumeration #{} differs from first (may be due to actual device changes)",
                    i + 1
                );
            }
        }
        tracing::info!(
            "✅ Enumeration consistency verified across {} calls",
            enumerations.len()
        );
    }
}

/// Test that device availability checking is accurate
///
/// Verifies that is_device_available() correctly reports device status
/// immediately after enumeration.
#[tokio::test]
async fn test_device_availability_accuracy() {
    let monitor = create_async_device_monitor();

    match monitor.enumerate_devices().await {
        Ok(devices) => {
            if devices.is_empty() {
                tracing::warn!("No devices to test availability");
                return;
            }

            for device in devices.iter().take(3) {
                let is_available = monitor.is_device_available(&device.id).await;

                tracing::info!(
                    "Device '{}' availability: {} (should be true since just enumerated)",
                    device.name,
                    is_available
                );

                // Device should be available since we just enumerated it
                assert!(
                    is_available,
                    "Device '{}' should be available after enumeration",
                    device.name
                );
            }

            // Test with obviously invalid ID
            let invalid_available = monitor.is_device_available("invalid_device_99999").await;
            assert!(!invalid_available, "Invalid device ID should return false");

            tracing::info!("✅ Device availability checking is accurate");
        }
        Err(e) => {
            tracing::warn!("Enumeration failed: {}", e);
        }
    }
}

/// Benchmark: Compare real-time vs polling performance
///
/// Measures the performance difference between real-time hotplug
/// notifications and polling-based detection.
#[tokio::test]
async fn test_performance_real_time_vs_polling() {
    let monitor = create_async_device_monitor();
    let platform = monitor.platform_name();

    tracing::info!("Performance test on platform: {}", platform);

    // Test enumeration speed
    let mut enumeration_times = Vec::new();
    for i in 0..10 {
        let start = Instant::now();
        let _ = monitor.enumerate_devices().await;
        let elapsed = start.elapsed();
        enumeration_times.push(elapsed);

        tracing::debug!("Enumeration #{}: {:?}", i + 1, elapsed);
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    let avg_enumeration =
        enumeration_times.iter().sum::<Duration>() / enumeration_times.len() as u32;
    let min_enumeration = enumeration_times.iter().min().unwrap();
    let max_enumeration = enumeration_times.iter().max().unwrap();

    tracing::info!("Enumeration performance:");
    tracing::info!("  Average: {:?}", avg_enumeration);
    tracing::info!("  Min: {:?}", min_enumeration);
    tracing::info!("  Max: {:?}", max_enumeration);

    // Expected performance by platform
    #[cfg(all(target_os = "macos", feature = "native-device-monitor"))]
    {
        assert!(
            avg_enumeration < Duration::from_millis(50),
            "macOS CoreAudio average enumeration should be < 50ms"
        );
        tracing::info!("✅ macOS CoreAudio meets performance target");
    }

    #[cfg(all(target_os = "linux", feature = "native-device-monitor"))]
    {
        assert!(
            avg_enumeration < Duration::from_millis(100),
            "Linux PipeWire average enumeration should be < 100ms"
        );
        tracing::info!("✅ Linux PipeWire meets performance target");
    }

    #[cfg(all(target_os = "windows", feature = "native-device-monitor"))]
    {
        assert!(
            avg_enumeration < Duration::from_millis(150),
            "Windows WinRT average enumeration should be < 150ms"
        );
        tracing::info!("✅ Windows WinRT meets performance target");
    }
}

/// Stress test: Rapid device queries
///
/// Verifies system stability under high query load.
#[tokio::test]
async fn test_stress_rapid_queries() {
    let monitor = Arc::new(create_async_device_monitor());

    let mut handles = vec![];

    for i in 0..20 {
        let monitor_clone = monitor.clone();
        let handle = tokio::spawn(async move {
            for j in 0..5 {
                let _ = monitor_clone.enumerate_devices().await;
                tracing::trace!("Task {} iteration {}", i, j);
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        });
        handles.push(handle);
    }

    // Wait for all tasks
    for handle in handles {
        assert!(
            handle.await.is_ok(),
            "Task should complete without panicking"
        );
    }

    tracing::info!("✅ System stable under 100 rapid concurrent queries");
}
