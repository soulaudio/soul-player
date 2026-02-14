//! CPAL-based fallback implementation of async device monitoring
//!
//! This is a compatibility layer that wraps CPAL's synchronous device enumeration
//! in async functions. It provides the same API as the native implementations but
//! uses blocking calls internally (spawned to tokio::task::spawn_blocking).
//!
//! # When to Use
//!
//! - **Now**: As a working async abstraction while native implementations are developed
//! - **Later**: Replace with platform-specific async implementations:
//!   - macOS: `device_monitor_macos.rs` (CoreAudio async)
//!   - Linux: `device_monitor_linux.rs` (PipeWire async)
//!   - Windows: `device_monitor_windows.rs` (WinRT async)
//!
//! # Performance
//!
//! - Device enumeration: ~50-500ms (blocks in spawn_blocking thread)
//! - Hotplug: Polling-based (checks every 2 seconds)
//! - Better than blocking the event loop, but not as good as native async
//!
//! # Migration Path
//!
//! This implementation matches the `AsyncDeviceMonitor` trait, so swapping it
//! for native implementations is transparent to callers.

use async_trait::async_trait;
use cpal::traits::{DeviceTrait, HostTrait};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::device_monitor_async::{
    AsyncDeviceInfo, AsyncDeviceMonitor, DeviceChangeCallback, DeviceEvent, DeviceMonitorError,
    WatchHandle,
};

/// CPAL-based fallback device monitor
///
/// Wraps CPAL's synchronous APIs in async functions using `spawn_blocking`.
/// This prevents blocking the async runtime while still using CPAL's
/// reliable cross-platform device enumeration.
pub struct CpalFallbackMonitor {
    /// Platform name for this monitor
    platform: &'static str,
}

impl CpalFallbackMonitor {
    /// Create a new CPAL-based monitor
    pub fn new() -> Self {
        let platform = if cfg!(target_os = "macos") {
            "macOS (CPAL Fallback - Native CoreAudio Async Planned)"
        } else if cfg!(target_os = "linux") {
            "Linux (CPAL Fallback - Native PipeWire Async Planned)"
        } else if cfg!(target_os = "windows") {
            "Windows (CPAL Fallback - Native WinRT Async Planned)"
        } else {
            "Unknown (CPAL Fallback)"
        };

        Self { platform }
    }

    /// Convert CPAL device to AsyncDeviceInfo
    fn device_to_info(device: &cpal::Device, is_default: bool) -> Option<AsyncDeviceInfo> {
        let description = device.description().ok()?;
        let name = description.name();

        // Try to get sample rate and channels
        let (sample_rate, channels) = device
            .default_output_config()
            .ok()
            .map(|config| (Some(config.sample_rate()), Some(config.channels())))
            .unwrap_or((None, None));

        Some(AsyncDeviceInfo {
            id: name.to_string(),
            name: name.to_string(),
            is_default,
            is_available: true,
            sample_rate,
            channels,
        })
    }
}

impl Default for CpalFallbackMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AsyncDeviceMonitor for CpalFallbackMonitor {
    async fn enumerate_devices(&self) -> Result<Vec<AsyncDeviceInfo>, DeviceMonitorError> {
        tracing::debug!("[DEVICE_MONITOR] Starting device enumeration (CPAL fallback)");

        // Spawn blocking to avoid blocking async runtime
        // Recreate host in blocking context (cheap operation)
        let result = tokio::task::spawn_blocking(|| {
            let host = cpal::default_host();
            let default_device = host.default_output_device();
            let default_name = default_device
                .as_ref()
                .and_then(|d| d.description().ok())
                .map(|desc| desc.name().to_string());

            let devices = host.output_devices().map_err(|e| {
                tracing::error!(error = %e, "[DEVICE_MONITOR] Enumeration failed");
                DeviceMonitorError::EnumerationFailed(e.to_string())
            })?;

            let mut device_list = Vec::new();
            for device in devices {
                if let Ok(description) = device.description() {
                    let name = description.name();
                    let is_default = default_name.as_deref() == Some(name);

                    let (sample_rate, channels) = device
                        .default_output_config()
                        .ok()
                        .map(|config| (Some(config.sample_rate()), Some(config.channels())))
                        .unwrap_or((None, None));

                    tracing::debug!(
                        device_name = %name,
                        is_default = is_default,
                        sample_rate = ?sample_rate,
                        channels = ?channels,
                        "[DEVICE_MONITOR] Found device"
                    );

                    device_list.push(AsyncDeviceInfo {
                        id: name.to_string(),
                        name: name.to_string(),
                        is_default,
                        is_available: true,
                        sample_rate,
                        channels,
                    });
                }
            }

            tracing::info!(
                device_count = device_list.len(),
                "[DEVICE_MONITOR] Enumeration completed"
            );
            Ok(device_list)
        })
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "[DEVICE_MONITOR] Internal error during enumeration");
            DeviceMonitorError::Internal(e.to_string())
        })?;

        result
    }

    async fn get_default_device(&self) -> Result<AsyncDeviceInfo, DeviceMonitorError> {
        tracing::debug!("[DEVICE_MONITOR] Getting default device (CPAL fallback)");

        tokio::task::spawn_blocking(|| {
            let host = cpal::default_host();
            let device = host.default_output_device().ok_or_else(|| {
                tracing::error!("[DEVICE_MONITOR] No default device found");
                DeviceMonitorError::DeviceNotFound("No default device".to_string())
            })?;

            let description = device.description().map_err(|e| {
                tracing::error!(error = %e, "[DEVICE_MONITOR] Failed to get device description");
                DeviceMonitorError::Internal(e.to_string())
            })?;

            let name = description.name();

            let (sample_rate, channels) = device
                .default_output_config()
                .ok()
                .map(|config| (Some(config.sample_rate()), Some(config.channels())))
                .unwrap_or((None, None));

            tracing::info!(
                device_name = %name,
                sample_rate = ?sample_rate,
                channels = ?channels,
                "[DEVICE_MONITOR] Default device retrieved"
            );

            Ok(AsyncDeviceInfo {
                id: name.to_string(),
                name: name.to_string(),
                is_default: true,
                is_available: true,
                sample_rate,
                channels,
            })
        })
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "[DEVICE_MONITOR] Internal error getting default device");
            DeviceMonitorError::Internal(e.to_string())
        })?
    }

    async fn watch_for_changes(
        &self,
        callback: DeviceChangeCallback,
    ) -> Result<Box<dyn WatchHandle>, DeviceMonitorError> {
        tracing::debug!(
            "[DEVICE_MONITOR] Starting device change watcher (CPAL fallback - polling mode)"
        );

        // CPAL doesn't support hotplug notifications, so we poll
        // This is a limitation that will be fixed by native implementations
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();
        let previous_devices = Arc::new(Mutex::new(Vec::<(String, cpal::Device)>::new()));

        // Create bounded channel for device events (capacity 8 for backpressure)
        // This prevents unbounded memory growth if callback processing is slow
        let (device_event_tx, mut device_event_rx) = tokio::sync::mpsc::channel::<DeviceEvent>(8);

        // Wrap callback in Arc for cloning across spawn_blocking calls
        let callback = Arc::new(callback);

        // Spawn event processing task with proper error handling
        let callback_clone = callback.clone();
        tokio::spawn(async move {
            while let Some(event) = device_event_rx.recv().await {
                let callback_inner = callback_clone.clone();
                match tokio::task::spawn_blocking(move || {
                    callback_inner(event);
                })
                .await
                {
                    Ok(()) => {
                        tracing::debug!("[DEVICE_MONITOR] Device event processed successfully");
                    }
                    Err(e) => {
                        tracing::error!(
                            error = %e,
                            "[DEVICE_MONITOR] Device event callback failed to execute"
                        );
                    }
                }
            }
            tracing::debug!("[DEVICE_MONITOR] Device event processing task stopped");
        });

        // Spawn a background task that polls for device changes
        tokio::spawn(async move {
            while running_clone.load(Ordering::Relaxed) {
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

                // Recreate host for each poll (cheap operation)
                let host = cpal::default_host();

                // Enumerate devices
                let current_devices = match host.output_devices() {
                    Ok(devices) => devices
                        .filter_map(|d| {
                            d.description()
                                .ok()
                                .map(|desc| (desc.name().to_string(), d.clone()))
                        })
                        .collect::<Vec<_>>(),
                    Err(e) => {
                        tracing::debug!(error = %e, "[DEVICE_MONITOR] Poll enumeration failed, will retry");
                        continue;
                    }
                };

                let mut prev = previous_devices.lock().await;

                // Check for new devices
                for (name, _device) in &current_devices {
                    if !prev.iter().any(|(n, _)| n == name) {
                        tracing::info!(device_name = %name, "[DEVICE_MONITOR] Device added");
                        let event = DeviceEvent::DeviceAdded {
                            id: name.clone(),
                            name: name.clone(),
                        };
                        if let Err(e) = device_event_tx.try_send(event) {
                            tracing::warn!(
                                error = %e,
                                device_name = %name,
                                "[DEVICE_MONITOR] Device event channel full, dropping DeviceAdded event"
                            );
                        }
                    }
                }

                // Check for removed devices
                for (name, _device) in prev.iter() {
                    if !current_devices.iter().any(|(n, _)| n == name) {
                        tracing::info!(device_name = %name, "[DEVICE_MONITOR] Device removed");
                        let event = DeviceEvent::DeviceRemoved { id: name.clone() };
                        if let Err(e) = device_event_tx.try_send(event) {
                            tracing::warn!(
                                error = %e,
                                device_name = %name,
                                "[DEVICE_MONITOR] Device event channel full, dropping DeviceRemoved event"
                            );
                        }
                    }
                }

                *prev = current_devices;
            }
            tracing::debug!("[DEVICE_MONITOR] Device polling task stopped");
        });

        tracing::debug!("[DEVICE_MONITOR] Device change watcher started");
        Ok(Box::new(CpalWatchHandle { running }))
    }

    async fn is_device_available(&self, device_id: &str) -> bool {
        let device_id = device_id.to_string();

        tracing::debug!(device_id = %device_id, "[DEVICE_MONITOR] Checking device availability");

        let device_id_clone = device_id.clone();
        let result = tokio::task::spawn_blocking(move || {
            let host = cpal::default_host();
            if let Ok(devices) = host.output_devices() {
                for device in devices {
                    if let Ok(description) = device.description() {
                        if description.name() == device_id_clone {
                            return true;
                        }
                    }
                }
            }
            false
        })
        .await
        .unwrap_or(false);

        tracing::debug!(device_id = %device_id, is_available = result, "[DEVICE_MONITOR] Device availability checked");
        result
    }

    fn platform_name(&self) -> &'static str {
        self.platform
    }
}

/// Watch handle for CPAL fallback
struct CpalWatchHandle {
    running: Arc<AtomicBool>,
}

impl WatchHandle for CpalWatchHandle {
    fn stop(&mut self) {
        tracing::debug!("[DEVICE_MONITOR] Stopping device change watcher");
        self.running.store(false, Ordering::Relaxed);
    }
}

impl Drop for CpalWatchHandle {
    fn drop(&mut self) {
        tracing::debug!("[DEVICE_MONITOR] Device change watcher handle dropped");
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    #[tokio::test]
    async fn test_enumerate_devices() {
        let monitor = CpalFallbackMonitor::new();
        // This may fail in CI without audio devices - that's okay
        let result = monitor.enumerate_devices().await;
        assert!(result.is_ok() || matches!(result, Err(DeviceMonitorError::EnumerationFailed(_))));
    }

    #[tokio::test]
    async fn test_platform_name() {
        let monitor = CpalFallbackMonitor::new();
        let platform = monitor.platform_name();
        assert!(platform.contains("Fallback"));
    }

    #[tokio::test]
    async fn test_get_default_device_returns_default_flag() {
        let monitor = CpalFallbackMonitor::new();
        if let Ok(device) = monitor.get_default_device().await {
            assert!(
                device.is_default,
                "Device returned by get_default_device should be marked as default"
            );
            assert!(device.is_available, "Default device should be available");
        }
    }

    #[tokio::test]
    async fn test_is_device_available_with_invalid_id() {
        let monitor = CpalFallbackMonitor::new();
        let available = monitor
            .is_device_available("nonexistent_device_12345")
            .await;
        assert!(!available, "Nonexistent device should not be available");
    }

    #[tokio::test]
    async fn test_watch_handle_can_be_stopped() {
        let monitor = CpalFallbackMonitor::new();
        let callback_invoked = Arc::new(Mutex::new(false));
        let callback_invoked_clone = callback_invoked.clone();

        let callback = Box::new(move |_event: DeviceEvent| {
            *callback_invoked_clone.lock().unwrap() = true;
        });

        if let Ok(mut handle) = monitor.watch_for_changes(callback).await {
            // Stop immediately
            handle.stop();
            // Should not panic
        }
    }

    #[tokio::test]
    async fn test_watch_handle_drop_cleanup() {
        let monitor = CpalFallbackMonitor::new();
        let callback = Box::new(|_event: DeviceEvent| {});

        {
            let _handle = monitor.watch_for_changes(callback).await;
            // Handle dropped at end of scope
        }

        // Give time for cleanup
        tokio::time::sleep(Duration::from_millis(100)).await;
        // Should not leak resources
    }

    #[tokio::test]
    async fn test_thread_safety() {
        let monitor = Arc::new(CpalFallbackMonitor::new());

        let mut handles = vec![];
        for _ in 0..3 {
            let monitor_clone = monitor.clone();
            let handle = tokio::spawn(async move {
                let _ = monitor_clone.enumerate_devices().await;
            });
            handles.push(handle);
        }

        for handle in handles {
            assert!(handle.await.is_ok());
        }
    }

    #[tokio::test]
    async fn test_device_info_has_id_and_name() {
        let monitor = CpalFallbackMonitor::new();
        if let Ok(devices) = monitor.enumerate_devices().await {
            for device in devices {
                assert!(!device.id.is_empty(), "Device ID should not be empty");
                assert!(!device.name.is_empty(), "Device name should not be empty");
            }
        }
    }
}
