//! Async device monitoring abstraction layer
//!
//! Provides industry-standard async device monitoring using platform-native APIs:
//! - macOS: CoreAudio async property listeners
//! - Linux: PipeWire async device notifications
//! - Windows: WinRT async device watchers
//!
//! This is separate from CPAL (which is used for playback) to get truly non-blocking
//! device enumeration and hotplug notifications.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────┐
//! │           AsyncDeviceMonitor (Trait)                │
//! ├─────────────────────────────────────────────────────┤
//! │  - async enumerate_devices()                        │
//! │  - async watch_for_changes(callback)                │
//! │  - async get_default_device()                       │
//! └─────────────────────────────────────────────────────┘
//!          │                 │                 │
//!     ┌────┴────┐       ┌────┴────┐       ┌────┴────┐
//!     │ macOS   │       │ Linux   │       │Windows  │
//!     │CoreAudio│       │PipeWire │       │  WinRT  │
//!     └─────────┘       └─────────┘       └─────────┘
//! ```
//!
//! # Why Separate From CPAL?
//!
//! - **CPAL**: Excellent for playback, but device enumeration is synchronous
//! - **Native APIs**: Provide async device notifications and faster enumeration
//! - **Best of Both**: Use CPAL for reliable playback, native APIs for monitoring
//!
//! # Example
//!
//! ```no_run
//! use soul_audio_desktop::create_async_device_monitor;
//!
//! #[tokio::main]
//! async fn main() {
//!     let monitor = create_async_device_monitor();
//!
//!     // Enumerate devices asynchronously (non-blocking)
//!     let devices = monitor.enumerate_devices().await.unwrap();
//!     println!("Found {} devices", devices.len());
//!
//!     // Watch for device changes (hotplug notifications)
//!     monitor.watch_for_changes(|event| {
//!         println!("Device event: {:?}", event);
//!     }).await;
//! }
//! ```

use async_trait::async_trait;
use std::fmt;

/// Device change event types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceEvent {
    /// A new device was connected
    DeviceAdded { id: String, name: String },
    /// A device was disconnected
    DeviceRemoved { id: String },
    /// Default device changed
    DefaultDeviceChanged { id: String, name: String },
    /// Device property changed (sample rate, format, etc.)
    DevicePropertyChanged { id: String, property: String },
}

/// Information about an audio device
#[derive(Debug, Clone)]
pub struct AsyncDeviceInfo {
    /// Unique device identifier
    pub id: String,
    /// Human-readable device name
    pub name: String,
    /// Whether this is the default device
    pub is_default: bool,
    /// Whether the device is currently available
    pub is_available: bool,
    /// Sample rate (if known)
    pub sample_rate: Option<u32>,
    /// Channel count (if known)
    pub channels: Option<u16>,
}

/// Callback type for device change notifications
pub type DeviceChangeCallback = Box<dyn Fn(DeviceEvent) + Send + Sync>;

/// Async device monitoring trait
///
/// Platform-specific implementations provide truly async device enumeration
/// and hotplug notifications without blocking.
#[async_trait]
pub trait AsyncDeviceMonitor: Send + Sync {
    /// Enumerate all available audio output devices
    ///
    /// # Returns
    /// List of audio devices, or error if enumeration fails
    ///
    /// # Performance
    /// This is async and non-blocking on all platforms:
    /// - macOS: Uses CoreAudio async property queries
    /// - Linux: Uses PipeWire async device listing
    /// - Windows: Uses WinRT async device enumeration
    async fn enumerate_devices(&self) -> Result<Vec<AsyncDeviceInfo>, DeviceMonitorError>;

    /// Get the default output device
    ///
    /// # Returns
    /// Default device info, or error if no default exists
    async fn get_default_device(&self) -> Result<AsyncDeviceInfo, DeviceMonitorError>;

    /// Watch for device changes (hotplug notifications)
    ///
    /// Registers a callback that will be invoked whenever:
    /// - A device is added or removed
    /// - The default device changes
    /// - A device property changes
    ///
    /// # Arguments
    /// - `callback`: Function to call when device changes occur
    ///
    /// # Returns
    /// Handle that can be used to stop watching (dropped when done)
    async fn watch_for_changes(
        &self,
        callback: DeviceChangeCallback,
    ) -> Result<Box<dyn WatchHandle>, DeviceMonitorError>;

    /// Check if a specific device is available
    ///
    /// # Arguments
    /// - `device_id`: Device identifier to check
    ///
    /// # Returns
    /// true if device is available, false otherwise
    async fn is_device_available(&self, device_id: &str) -> bool;

    /// Get platform name
    fn platform_name(&self) -> &'static str;
}

/// Handle for stopping device change watching
pub trait WatchHandle: Send + Sync {
    /// Stop watching for device changes
    fn stop(&mut self);
}

/// Errors that can occur during async device monitoring
#[derive(Debug)]
pub enum DeviceMonitorError {
    /// Platform API not available
    PlatformUnavailable(String),
    /// Device enumeration failed
    EnumerationFailed(String),
    /// Device not found
    DeviceNotFound(String),
    /// Permission denied (e.g., mic access on macOS)
    PermissionDenied(String),
    /// Internal error
    Internal(String),
}

impl fmt::Display for DeviceMonitorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PlatformUnavailable(msg) => write!(f, "Platform unavailable: {}", msg),
            Self::EnumerationFailed(msg) => write!(f, "Enumeration failed: {}", msg),
            Self::DeviceNotFound(msg) => write!(f, "Device not found: {}", msg),
            Self::PermissionDenied(msg) => write!(f, "Permission denied: {}", msg),
            Self::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for DeviceMonitorError {}

/// Create platform-appropriate async device monitor
///
/// Returns the best async device monitor for the current platform:
/// - macOS: CoreAudio-based monitor (native async)
/// - Linux: PipeWire-based monitor (native async)
/// - Windows: WinRT-based monitor (native async)
/// - Other: CPAL fallback (async via spawn_blocking)
///
/// # Returns
/// Platform-specific async device monitor instance
///
/// # Example
///
/// ```no_run
/// use soul_audio_desktop::create_async_device_monitor;
///
/// #[tokio::main]
/// async fn main() {
///     let monitor = create_async_device_monitor();
///     let devices = monitor.enumerate_devices().await.unwrap();
///     println!("Platform: {}", monitor.platform_name());
/// }
/// ```
pub fn create_async_device_monitor() -> Box<dyn AsyncDeviceMonitor> {
    // Phase 3-5: Native implementations for major platforms (when feature enabled)
    // Phase 2: CPAL fallback (works everywhere, async via spawn_blocking)

    #[cfg(all(target_os = "macos", feature = "native-device-monitor"))]
    {
        Box::new(crate::device_monitor_macos::MacOSDeviceMonitor::new())
    }

    #[cfg(all(target_os = "linux", feature = "native-device-monitor"))]
    {
        Box::new(crate::device_monitor_linux::LinuxDeviceMonitor::new())
    }

    #[cfg(all(target_os = "windows", feature = "native-device-monitor"))]
    {
        Box::new(crate::device_monitor_windows::WindowsDeviceMonitor::new())
    }

    #[cfg(not(all(
        feature = "native-device-monitor",
        any(target_os = "macos", target_os = "linux", target_os = "windows")
    )))]
    {
        // CPAL fallback for:
        // - When native-device-monitor feature is disabled
        // - Unsupported platforms (FreeBSD, OpenBSD, etc.)
        Box::new(crate::device_monitor_cpal_fallback::CpalFallbackMonitor::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_device_event_equality() {
        let event1 = DeviceEvent::DeviceAdded {
            id: "dev1".to_string(),
            name: "Speaker".to_string(),
        };
        let event2 = DeviceEvent::DeviceAdded {
            id: "dev1".to_string(),
            name: "Speaker".to_string(),
        };
        assert_eq!(event1, event2);
    }

    #[test]
    fn test_device_info_creation() {
        let info = AsyncDeviceInfo {
            id: "test".to_string(),
            name: "Test Device".to_string(),
            is_default: true,
            is_available: true,
            sample_rate: Some(48000),
            channels: Some(2),
        };
        assert_eq!(info.id, "test");
        assert!(info.is_default);
    }

    #[test]
    fn test_device_event_variants() {
        let added = DeviceEvent::DeviceAdded {
            id: "1".to_string(),
            name: "Speaker".to_string(),
        };
        let removed = DeviceEvent::DeviceRemoved {
            id: "1".to_string(),
        };
        let changed = DeviceEvent::DefaultDeviceChanged {
            id: "2".to_string(),
            name: "Headphones".to_string(),
        };
        let property = DeviceEvent::DevicePropertyChanged {
            id: "3".to_string(),
            property: "sample_rate".to_string(),
        };

        // All variants should be creatable
        assert!(matches!(added, DeviceEvent::DeviceAdded { .. }));
        assert!(matches!(removed, DeviceEvent::DeviceRemoved { .. }));
        assert!(matches!(changed, DeviceEvent::DefaultDeviceChanged { .. }));
        assert!(matches!(
            property,
            DeviceEvent::DevicePropertyChanged { .. }
        ));
    }

    #[test]
    fn test_device_info_clone() {
        let info1 = AsyncDeviceInfo {
            id: "test".to_string(),
            name: "Test Device".to_string(),
            is_default: true,
            is_available: true,
            sample_rate: Some(48000),
            channels: Some(2),
        };

        let info2 = info1.clone();
        assert_eq!(info1.id, info2.id);
        assert_eq!(info1.name, info2.name);
        assert_eq!(info1.is_default, info2.is_default);
    }

    #[test]
    fn test_device_monitor_error_display() {
        let errors = vec![
            DeviceMonitorError::PlatformUnavailable("test".to_string()),
            DeviceMonitorError::EnumerationFailed("test".to_string()),
            DeviceMonitorError::DeviceNotFound("test".to_string()),
            DeviceMonitorError::PermissionDenied("test".to_string()),
            DeviceMonitorError::Internal("test".to_string()),
        ];

        for error in errors {
            let msg = format!("{}", error);
            assert!(!msg.is_empty(), "Error should have a display message");
        }
    }

    #[tokio::test]
    async fn test_create_async_device_monitor_returns_implementation() {
        let monitor = create_async_device_monitor();
        let platform = monitor.platform_name();

        // Should return a valid platform name
        assert!(!platform.is_empty(), "Platform name should not be empty");
    }

    #[tokio::test]
    async fn test_monitor_trait_methods_work() {
        let monitor = create_async_device_monitor();

        // All trait methods should be callable without panic
        let _result = monitor.enumerate_devices().await;
        let _result = monitor.get_default_device().await;
        let _result = monitor.is_device_available("test").await;
        let _platform = monitor.platform_name();
    }

    #[tokio::test]
    async fn test_watch_handle_trait_object() {
        let monitor = create_async_device_monitor();
        let callback = Box::new(|_: DeviceEvent| {});

        if let Ok(mut handle) = monitor.watch_for_changes(callback).await {
            // WatchHandle trait should be usable as trait object
            handle.stop();
        }
    }

    #[tokio::test]
    async fn test_concurrent_monitor_creation() {
        let mut handles = vec![];

        for _ in 0..5 {
            let handle = tokio::spawn(async {
                let monitor = create_async_device_monitor();
                let _platform = monitor.platform_name();
            });
            handles.push(handle);
        }

        for handle in handles {
            assert!(handle.await.is_ok());
        }
    }

    #[tokio::test]
    async fn test_monitor_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Arc<dyn AsyncDeviceMonitor>>();
    }

    #[test]
    fn test_feature_flag_creates_correct_implementation() {
        let monitor = create_async_device_monitor();
        let platform = monitor.platform_name();

        #[cfg(all(target_os = "macos", feature = "native-device-monitor"))]
        {
            assert!(
                platform.contains("CoreAudio"),
                "Should use CoreAudio on macOS with feature"
            );
        }

        #[cfg(all(target_os = "linux", feature = "native-device-monitor"))]
        {
            assert!(
                platform.contains("PipeWire"),
                "Should use PipeWire on Linux with feature"
            );
        }

        #[cfg(all(target_os = "windows", feature = "native-device-monitor"))]
        {
            assert!(
                platform.contains("WinRT"),
                "Should use WinRT on Windows with feature"
            );
        }

        #[cfg(not(all(
            feature = "native-device-monitor",
            any(target_os = "macos", target_os = "linux", target_os = "windows")
        )))]
        {
            assert!(
                platform.contains("Fallback"),
                "Should use CPAL fallback without feature or on unsupported platform"
            );
        }
    }
}
