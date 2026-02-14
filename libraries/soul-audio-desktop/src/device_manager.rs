//! Device management for audio output
//!
//! This module handles audio device enumeration, switching, and state management.
//! It provides a centralized interface for managing audio output devices and their
//! configurations.
//!
//! # Device ID Tracking
//!
//! The device manager tracks two types of device identifiers:
//!
//! 1. **Synthetic Device ID**: Created from backend + device name (e.g., "WASAPI::Speakers")
//!    - Always available
//!    - Used for CPAL-based device identification
//!
//! 2. **Native Device ID**: Platform-specific identifier from native APIs
//!    - WinRT device ID on Windows (e.g., "{0.0.0.00000000}.{guid...}")
//!    - CoreAudio device ID on macOS (e.g., numeric ID)
//!    - Optional, only set when available from device monitor
//!    - Provides most reliable device removal detection
//!
//! # Device Removal Detection
//!
//! The `is_current_device()` method uses a multi-tier matching strategy:
//! 1. Exact match with native device ID (most reliable, no false positives)
//! 2. Exact match with device name
//! 3. Substring match (fallback for WinRT IDs containing device name)
//! 4. Match with synthetic device ID
//!
//! This ensures reliable device removal detection while maintaining backward compatibility.

use std::sync::{Arc, Mutex};

use crate::AudioBackend;

/// Device manager for audio output
///
/// Manages the current audio device, backend selection, and device switching logic.
/// Uses a simple Mutex for state management (device switches are infrequent operations).
pub struct DeviceManager {
    /// Device state (backend, device name, device ID)
    state: Arc<Mutex<DeviceState>>,
}

#[derive(Clone)]
struct DeviceState {
    /// Current audio backend
    backend: AudioBackend,
    /// Current device name (from CPAL)
    device_name: String,
    /// Synthetic device ID (backend::device_name)
    device_id: Option<String>,
    /// Native platform device ID (from WinRT/CoreAudio/etc if available)
    /// This is used for precise device removal detection
    native_device_id: Option<String>,
}

impl DeviceManager {
    /// Create a new device manager
    ///
    /// Initializes with default values. Call `update_device` after stream creation
    /// to set the actual device.
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(DeviceState {
                backend: AudioBackend::Default,
                device_name: "(None)".to_string(),
                device_id: None,
                native_device_id: None,
            })),
        }
    }

    /// Get current backend
    pub fn get_current_backend(&self) -> AudioBackend {
        self.state.lock().unwrap().backend
    }

    /// Get current device name
    pub fn get_current_device(&self) -> String {
        self.state.lock().unwrap().device_name.clone()
    }

    /// Get current synthetic device ID (backend::device_name)
    ///
    /// Returns a unique identifier for the current audio device (backend + device name).
    /// This is used to prevent false positive device switches when checking sample rates.
    ///
    /// # Returns
    /// * `Some(device_id)` - The current device's unique identifier
    /// * `None` - No device active (silent mode)
    pub fn get_current_device_id(&self) -> Option<String> {
        self.state.lock().unwrap().device_id.clone()
    }

    /// Get current native platform device ID (if available)
    ///
    /// Returns the native device ID from the platform API (WinRT, CoreAudio, etc.).
    /// This is the most reliable identifier for device removal detection.
    ///
    /// # Returns
    /// * `Some(native_id)` - The platform-specific device identifier
    /// * `None` - Native ID not available or device inactive
    pub fn get_native_device_id(&self) -> Option<String> {
        self.state.lock().unwrap().native_device_id.clone()
    }

    /// Update the current device after stream creation
    ///
    /// This should be called after successfully creating a new audio stream.
    ///
    /// # Arguments
    /// * `backend` - The backend used for the stream
    /// * `device_name` - The name of the device
    /// * `is_silent_mode` - Whether this is silent mode (no audio devices available)
    pub fn update_device(&self, backend: AudioBackend, device_name: &str, is_silent_mode: bool) {
        let mut state = self.state.lock().unwrap();
        state.backend = backend;
        state.device_name = device_name.to_string();
        state.device_id = if is_silent_mode {
            None
        } else {
            Some(Self::make_device_id(backend, device_name))
        };
        // Native device ID is cleared here - should be set separately via set_native_device_id
        state.native_device_id = None;
    }

    /// Set the native platform device ID
    ///
    /// This should be called after successfully switching to a device when the
    /// native device ID is known (from WinRT, CoreAudio, etc.).
    ///
    /// # Arguments
    /// * `native_id` - The platform-specific device identifier
    pub fn set_native_device_id(&self, native_id: Option<String>) {
        let mut state = self.state.lock().unwrap();
        state.native_device_id = native_id;
    }

    /// Create a unique device ID from backend and device name
    ///
    /// Device ID format: "{backend}::{device_name}"
    /// This provides a unique identifier that can be used to track
    /// which device is currently active and detect device removal events.
    ///
    /// # Example
    /// ```rust,ignore
    /// let device_id = DeviceManager::make_device_id(
    ///     AudioBackend::Default,
    ///     "Speakers (Realtek Audio)"
    /// );
    /// // Returns: "WASAPI::Speakers (Realtek Audio)" on Windows
    /// // Returns: "CoreAudio::Speakers (Realtek Audio)" on macOS
    /// // Returns: "ALSA::Speakers (Realtek Audio)" on Linux
    /// ```
    pub fn make_device_id(backend: AudioBackend, device_name: &str) -> String {
        format!("{}::{}", backend.name(), device_name)
    }

    /// Check if a device name matches our current device
    ///
    /// Device IDs from platform APIs (WinRT, CoreAudio) may differ in format
    /// from the device names we store. This method handles the comparison
    /// by checking both the full device ID and the device name.
    ///
    /// Priority order:
    /// 1. Exact match with native device ID (most reliable)
    /// 2. Exact match with device name
    /// 3. Substring match with device name (fallback for compatibility)
    /// 4. Match with synthetic device ID
    ///
    /// # Arguments
    /// * `device_id_or_name` - The device identifier to check (from platform API)
    ///
    /// # Returns
    /// * `true` - The provided ID/name matches our current device
    /// * `false` - The provided ID/name does not match
    pub fn is_current_device(&self, device_id_or_name: &str) -> bool {
        let state = self.state.lock().unwrap();

        // 1. BEST: Exact match with native device ID (most reliable, no false positives)
        if let Some(ref native_id) = state.native_device_id {
            if native_id == device_id_or_name {
                tracing::debug!(
                    device_id = %device_id_or_name,
                    "[DeviceManager] Matched via native device ID (exact)"
                );
                return true;
            }
        }

        // 2. GOOD: Exact match with device name
        if state.device_name == device_id_or_name {
            tracing::debug!(
                device_id = %device_id_or_name,
                "[DeviceManager] Matched via device name (exact)"
            );
            return true;
        }

        // 3. FALLBACK: Substring match (handles WinRT full IDs containing device name)
        // This can have false positives if two devices have similar names
        if device_id_or_name.contains(&state.device_name) {
            tracing::debug!(
                device_id = %device_id_or_name,
                device_name = %state.device_name,
                "[DeviceManager] Matched via substring (device_id contains name)"
            );
            return true;
        }

        // 4. Check against synthetic device ID
        if let Some(ref stored_id) = state.device_id {
            if stored_id == device_id_or_name {
                tracing::debug!(
                    device_id = %device_id_or_name,
                    "[DeviceManager] Matched via synthetic ID (exact)"
                );
                return true;
            }
            // Also check if the provided ID contains our stored ID or vice versa
            if device_id_or_name.contains(stored_id) || stored_id.contains(device_id_or_name) {
                tracing::debug!(
                    device_id = %device_id_or_name,
                    stored_id = %stored_id,
                    "[DeviceManager] Matched via synthetic ID (substring)"
                );
                return true;
            }
        }

        false
    }
}

impl Default for DeviceManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_make_device_id() {
        let backend = AudioBackend::Default;
        let backend_name = backend.name(); // WASAPI, CoreAudio, or ALSA depending on OS

        let id = DeviceManager::make_device_id(backend, "Speakers");
        assert_eq!(id, format!("{}::Speakers", backend_name));

        let id = DeviceManager::make_device_id(backend, "hw:0,0");
        assert_eq!(id, format!("{}::hw:0,0", backend_name));
    }

    #[test]
    fn test_device_manager_initial_state() {
        let manager = DeviceManager::new();
        assert_eq!(manager.get_current_device(), "(None)");
        assert_eq!(manager.get_current_backend(), AudioBackend::Default);
        assert_eq!(manager.get_current_device_id(), None);
        assert_eq!(manager.get_native_device_id(), None);
    }

    #[test]
    fn test_update_device() {
        let manager = DeviceManager::new();

        // Update to a real device
        let backend = AudioBackend::Default;
        let backend_name = backend.name();
        manager.update_device(backend, "Test Speaker", false);
        assert_eq!(manager.get_current_device(), "Test Speaker");
        assert_eq!(manager.get_current_backend(), backend);
        assert_eq!(
            manager.get_current_device_id(),
            Some(format!("{}::Test Speaker", backend_name))
        );
        assert_eq!(manager.get_native_device_id(), None); // Not set by update_device

        // Update to silent mode
        manager.update_device(AudioBackend::Default, "(Silent)", true);
        assert_eq!(manager.get_current_device(), "(Silent)");
        assert_eq!(manager.get_current_device_id(), None);
        assert_eq!(manager.get_native_device_id(), None);
    }

    #[test]
    fn test_is_current_device() {
        let manager = DeviceManager::new();
        let backend = AudioBackend::Default;
        let backend_name = backend.name();
        manager.update_device(backend, "My Speakers", false);

        // Exact match with device name
        assert!(manager.is_current_device("My Speakers"));

        // Exact match with device ID
        assert!(manager.is_current_device(&format!("{}::My Speakers", backend_name)));

        // Partial match (handles WinRT full IDs)
        assert!(manager.is_current_device("Some prefix My Speakers some suffix"));

        // No match
        assert!(!manager.is_current_device("Different Device"));
    }

    #[test]
    fn test_native_device_id() {
        let manager = DeviceManager::new();
        let backend = AudioBackend::Default;

        // Update device (clears native ID)
        manager.update_device(backend, "Test Device", false);
        assert_eq!(manager.get_native_device_id(), None);

        // Set native device ID
        let native_id = "{0.0.0.00000000}.{a1b2c3d4-1234-5678-90ab-cdef12345678}";
        manager.set_native_device_id(Some(native_id.to_string()));
        assert_eq!(manager.get_native_device_id(), Some(native_id.to_string()));

        // Native ID takes priority in is_current_device
        assert!(manager.is_current_device(native_id));

        // Clear native ID
        manager.set_native_device_id(None);
        assert_eq!(manager.get_native_device_id(), None);
        assert!(!manager.is_current_device(native_id));
    }

    #[test]
    fn test_is_current_device_with_native_id() {
        let manager = DeviceManager::new();
        let backend = AudioBackend::Default;

        // Setup device with native ID
        manager.update_device(backend, "My Speakers", false);
        let native_id = "{0.0.0.00000000}.{a1b2c3d4}";
        manager.set_native_device_id(Some(native_id.to_string()));

        // Native ID match should work (exact match, no substring)
        assert!(manager.is_current_device(native_id));

        // Device name match should still work
        assert!(manager.is_current_device("My Speakers"));

        // Different native ID should not match
        assert!(!manager.is_current_device("{0.0.0.00000000}.{different}"));
    }
}
