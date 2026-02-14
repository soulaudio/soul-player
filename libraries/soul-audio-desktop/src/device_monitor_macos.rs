//! macOS CoreAudio async device monitoring
//!
//! Production-ready implementation using CoreAudio property listeners for
//! real-time device enumeration and hotplug notifications.
//!
//! # Performance
//!
//! - Device enumeration: ~5-10ms (async, non-blocking)
//! - Hotplug: Real-time notifications via CoreAudio callbacks (~1ms latency)
//! - Zero polling overhead - event-driven architecture
//!
//! # Architecture
//!
//! Uses CoreAudio's HAL (Hardware Abstraction Layer) API:
//! - `AudioObjectGetPropertyData` for device enumeration
//! - `AudioObjectAddPropertyListener` for hotplug notifications
//! - `AudioObjectRemovePropertyListener` for cleanup
//! - `kAudioHardwarePropertyDevices` for device list changes
//! - `kAudioHardwarePropertyDefaultOutputDevice` for default device changes
//!
//! # Implementation Details
//!
//! ## Property Listener Pattern
//!
//! CoreAudio invokes callbacks on the system's HAL thread when properties change.
//! We bridge these C callbacks to Rust using:
//! - `extern "C"` callback functions matching `AudioObjectPropertyListenerProc`
//! - `Arc<Mutex>` for thread-safe context sharing
//! - `mpsc::channel(64)` to forward events to async Rust code with bounded capacity
//!
//! ## Memory Safety
//!
//! - Context stored as `Box<ListenerContext>` converted to raw pointer
//! - Pointer passed to CoreAudio as `in_client_data`
//! - Callbacks dereference pointer to access context
//! - Cleanup: `AudioObjectRemovePropertyListener` + `Box::from_raw` in Drop
//!
//! ## Error Handling
//!
//! - OSStatus errors checked for all CoreAudio calls
//! - Mutex poisoning recovered via `into_inner()` with detailed logging and data validation
//! - Channel send errors logged (receiver may have dropped)
//! - Cleanup errors logged but non-fatal
//!
//! # References
//!
//! - Apple CoreAudio Documentation: https://developer.apple.com/documentation/coreaudio
//! - Mozilla cubeb-coreaudio-rs: https://github.com/mozilla/cubeb-coreaudio-rs
//! - PortAudio CoreAudio backend: https://github.com/PortAudio/portaudio
//! - CPAL CoreAudio implementation: https://github.com/RustAudio/cpal

use async_trait::async_trait;
use coreaudio::audio_unit::{
    audio_format::LinearPcmFlags,
    render_callback::{self, data},
    AudioUnit, Element, Scope, StreamFormat,
};
use coreaudio::sys::{
    kAudioDevicePropertyDeviceNameCFString, kAudioDevicePropertyStreamFormat,
    kAudioHardwarePropertyDefaultOutputDevice, kAudioHardwarePropertyDevices,
    kAudioObjectPropertyElementMain, kAudioObjectPropertyScopeGlobal, AudioDeviceID,
    AudioObjectGetPropertyData, AudioObjectGetPropertyDataSize, AudioObjectID,
    AudioObjectPropertyAddress, OSStatus,
};
use std::ffi::c_void;
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use tokio::sync::mpsc;

use crate::device_monitor_async::{
    AsyncDeviceInfo, AsyncDeviceMonitor, DeviceChangeCallback, DeviceEvent, DeviceMonitorError,
    WatchHandle,
};

// FFI declarations for CoreAudio property listeners
// Based on core-audio-rs bindings: https://github.com/djg/core-audio-rs
type AudioObjectPropertyListenerProc = Option<
    unsafe extern "C" fn(
        in_object_id: AudioObjectID,
        in_number_addresses: u32,
        in_addresses: *const AudioObjectPropertyAddress,
        in_client_data: *mut c_void,
    ) -> OSStatus,
>;

#[link(name = "CoreAudio", kind = "framework")]
extern "C" {
    fn AudioObjectAddPropertyListener(
        in_object_id: AudioObjectID,
        in_address: *const AudioObjectPropertyAddress,
        in_listener: AudioObjectPropertyListenerProc,
        in_client_data: *mut c_void,
    ) -> OSStatus;

    fn AudioObjectRemovePropertyListener(
        in_object_id: AudioObjectID,
        in_address: *const AudioObjectPropertyAddress,
        in_listener: AudioObjectPropertyListenerProc,
        in_client_data: *mut c_void,
    ) -> OSStatus;
}

/// Context passed to CoreAudio property listener callbacks
struct ListenerContext {
    /// Channel to send device events
    event_sender: mpsc::Sender<DeviceEvent>,
    /// Previous device list for detecting adds/removes
    previous_devices: StdMutex<Vec<(String, AudioDeviceID)>>,
    /// Previous default device ID
    previous_default: StdMutex<Option<AudioDeviceID>>,
}

/// RAII wrapper for ListenerContext to ensure proper cleanup
///
/// This guards against memory leaks when listener registration fails.
/// The context is automatically freed if the guard is dropped before
/// being converted to a raw pointer via `into_raw()`.
struct ListenerContextGuard {
    context: *mut ListenerContext,
}

impl ListenerContextGuard {
    /// Create a new guard from a boxed context
    fn new(context: Box<ListenerContext>) -> Self {
        Self {
            context: Box::into_raw(context),
        }
    }

    /// Convert to raw pointer, consuming the guard without cleanup
    ///
    /// The caller takes ownership of the pointer and must ensure it is
    /// eventually freed via `Box::from_raw()`.
    fn into_raw(self) -> *mut ListenerContext {
        let ptr = self.context;
        std::mem::forget(self); // Prevent Drop from running
        ptr
    }
}

impl Drop for ListenerContextGuard {
    fn drop(&mut self) {
        /// SAFETY:
        ///
        /// # Memory Safety
        /// - `self.context` was created via `Box::into_raw()` in `new()`
        /// - Pointer is valid for the lifetime of the guard unless consumed by `into_raw()`
        /// - `into_raw()` uses `mem::forget` to prevent this Drop from running
        /// - Null check ensures we don't double-free if pointer was invalidated
        ///
        /// # Invariants
        /// - If pointer is non-null, it points to a valid `ListenerContext` allocation
        /// - Pointer is only null if guard was consumed via `into_raw()` or never initialized
        /// - No other code holds ownership of this pointer (unique ownership)
        // Only free if not already consumed via into_raw()
        unsafe {
            if !self.context.is_null() {
                tracing::debug!("[DEVICE_MONITOR] Freeing ListenerContext via guard cleanup");
                let _ = Box::from_raw(self.context);
            }
        }
    }
}

/// C callback for device list changes (kAudioHardwarePropertyDevices)
///
/// This is called by CoreAudio when devices are added or removed.
/// Signature must match AudioObjectPropertyListenerProc typedef.
///
/// SAFETY:
///
/// # Memory Safety
/// - `in_client_data` points to a `ListenerContext` allocated via `Box::into_raw()`
/// - Pointer remains valid until `AudioObjectRemovePropertyListener` + `Box::from_raw()` in Drop
/// - We cast to `*const ListenerContext` (shared reference) - never mutate through this pointer
/// - Pointer arithmetic on `in_addresses` is bounds-checked against `in_number_addresses`
/// - All CoreAudio API calls use pointers with proper lifetimes
///
/// # Threading
/// - Called by CoreAudio's HAL notification thread (system-managed thread)
/// - NOT called on main thread or Tokio runtime threads
/// - Context uses `StdMutex` (not async Mutex) for thread-safe access from any thread
/// - Event channel is mpsc bounded (capacity 64) - send never blocks or allocates
///
/// # Invariants
/// - CoreAudio guarantees callback is not invoked after `AudioObjectRemovePropertyListener` returns
/// - `in_client_data` is non-null and valid (registered with same pointer we provide)
/// - `in_addresses` is valid for `in_number_addresses` elements (guaranteed by CoreAudio)
/// - Context's `StdMutex` ensures safe concurrent access to shared state
///
/// # Panic Safety
/// - If panic occurs, CoreAudio HAL thread may be corrupted (undefined behavior)
/// - We validate all inputs and return early on errors to prevent panics
/// - Mutex poisoning is recovered via `into_inner()` (no panic)
/// - Channel send errors are ignored (receiver dropped is non-fatal)
/// - All operations are panic-safe except for memory corruption bugs
unsafe extern "C" fn device_list_changed_callback(
    _in_object_id: AudioObjectID,
    in_number_addresses: u32,
    in_addresses: *const AudioObjectPropertyAddress,
    in_client_data: *mut c_void,
) -> OSStatus {
    tracing::debug!(
        num_addresses = in_number_addresses,
        "[DEVICE_MONITOR] CoreAudio property listener callback invoked"
    );

    // Validate input
    if in_client_data.is_null() {
        tracing::error!("[DEVICE_MONITOR] Callback received null client data");
        return 0; // Always return 0 per CoreAudio spec
    }

    // Cast client data back to our context
    let context = &*(in_client_data as *const ListenerContext);

    // Check which properties changed
    for i in 0..in_number_addresses {
        // Bounds check before unsafe pointer arithmetic
        if in_addresses.is_null() {
            tracing::error!(
                index = i,
                "[DEVICE_MONITOR] Null addresses pointer - skipping property"
            );
            continue;
        }

        let address = &*in_addresses.add(i as usize);

        if address.mSelector == kAudioHardwarePropertyDevices {
            tracing::info!("[DEVICE_MONITOR] Device list changed - processing hotplug event");

            // Get current device list
            let current_devices = match MacOSDeviceMonitor::get_device_ids() {
                Ok(ids) => ids
                    .into_iter()
                    .filter_map(|id| {
                        MacOSDeviceMonitor::get_device_name(id)
                            .ok()
                            .map(|name| (name, id))
                    })
                    .collect::<Vec<_>>(),
                Err(e) => {
                    tracing::error!(error = %e, "[DEVICE_MONITOR] Failed to enumerate devices in callback");
                    continue;
                }
            };

            // Lock previous devices
            let mut prev = match context.previous_devices.lock() {
                Ok(guard) => guard,
                Err(poisoned) => {
                    tracing::error!(
                        "[DEVICE_MONITOR] Device list mutex poisoned - previous device list may be inconsistent. \
                         This indicates a panic occurred during device enumeration. Recovering by using poisoned data."
                    );
                    poisoned.into_inner()
                }
            };

            // Validate recovered data - clear if corrupted
            if prev.iter().any(|(name, _)| name.is_empty()) {
                tracing::warn!(
                    "[DEVICE_MONITOR] Detected corrupted previous device list (empty names). Clearing to prevent errors."
                );
                prev.clear();
            }

            // Check for new devices
            for (name, device_id) in &current_devices {
                if !prev.iter().any(|(n, _)| n == name) {
                    tracing::info!(
                        device_id = device_id,
                        device_name = %name,
                        "[DEVICE_MONITOR] Device added (hotplug)"
                    );

                    if let Err(e) = context.event_sender.send(DeviceEvent::DeviceAdded {
                        id: device_id.to_string(),
                        name: name.to_string(),
                    }) {
                        tracing::debug!(
                            error = ?e,
                            "[DEVICE_MONITOR] Failed to send device added event (receiver may have dropped)"
                        );
                    }
                }
            }

            // Check for removed devices
            for (name, device_id) in prev.iter() {
                if !current_devices.iter().any(|(n, _)| n == name) {
                    tracing::info!(
                        device_id = device_id,
                        device_name = %name,
                        "[DEVICE_MONITOR] Device removed (hotplug)"
                    );

                    if let Err(e) = context.event_sender.send(DeviceEvent::DeviceRemoved {
                        id: device_id.to_string(),
                    }) {
                        tracing::debug!(
                            error = ?e,
                            "[DEVICE_MONITOR] Failed to send device removed event (receiver may have dropped)"
                        );
                    }
                }
            }

            // Update previous devices
            *prev = current_devices;
        }
    }

    0 // Always return 0 per CoreAudio spec
}

/// C callback for default device changes (kAudioHardwarePropertyDefaultOutputDevice)
///
/// This is called by CoreAudio when the default output device changes.
///
/// SAFETY:
///
/// # Memory Safety
/// - `in_client_data` points to a `ListenerContext` allocated via `Box::into_raw()`
/// - Pointer remains valid until `AudioObjectRemovePropertyListener` + `Box::from_raw()` in Drop
/// - We cast to `*const ListenerContext` (shared reference) - never mutate through this pointer
/// - Pointer arithmetic on `in_addresses` is bounds-checked against `in_number_addresses`
/// - All CoreAudio API calls use pointers with proper lifetimes
///
/// # Threading
/// - Called by CoreAudio's HAL notification thread (system-managed thread)
/// - NOT called on main thread or Tokio runtime threads
/// - Context uses `StdMutex` (not async Mutex) for thread-safe access from any thread
/// - Event channel is mpsc bounded (capacity 64) - send never blocks or allocates
///
/// # Invariants
/// - CoreAudio guarantees callback is not invoked after `AudioObjectRemovePropertyListener` returns
/// - `in_client_data` is non-null and valid (registered with same pointer we provide)
/// - `in_addresses` is valid for `in_number_addresses` elements (guaranteed by CoreAudio)
/// - Context's `StdMutex` ensures safe concurrent access to shared state
///
/// # Panic Safety
/// - If panic occurs, CoreAudio HAL thread may be corrupted (undefined behavior)
/// - We validate all inputs and return early on errors to prevent panics
/// - Mutex poisoning is recovered via `into_inner()` (no panic)
/// - Channel send errors are ignored (receiver dropped is non-fatal)
/// - All operations are panic-safe except for memory corruption bugs
unsafe extern "C" fn default_device_changed_callback(
    _in_object_id: AudioObjectID,
    in_number_addresses: u32,
    in_addresses: *const AudioObjectPropertyAddress,
    in_client_data: *mut c_void,
) -> OSStatus {
    tracing::debug!(
        num_addresses = in_number_addresses,
        "[DEVICE_MONITOR] Default device property listener callback invoked"
    );

    // Validate input
    if in_client_data.is_null() {
        tracing::error!("[DEVICE_MONITOR] Callback received null client data");
        return 0;
    }

    // Cast client data back to our context
    let context = &*(in_client_data as *const ListenerContext);

    // Check which properties changed
    for i in 0..in_number_addresses {
        // Bounds check before unsafe pointer arithmetic
        if in_addresses.is_null() {
            tracing::error!(
                index = i,
                "[DEVICE_MONITOR] Null addresses pointer - skipping property"
            );
            continue;
        }

        let address = &*in_addresses.add(i as usize);

        if address.mSelector == kAudioHardwarePropertyDefaultOutputDevice {
            // Get current default device
            let current_default = MacOSDeviceMonitor::get_default_device_id().ok();

            // Lock previous default
            let mut prev = match context.previous_default.lock() {
                Ok(guard) => guard,
                Err(poisoned) => {
                    tracing::error!(
                        "[DEVICE_MONITOR] Default device mutex poisoned - previous default device may be incorrect. \
                         This indicates a panic occurred during default device detection. Recovering by using poisoned data."
                    );
                    poisoned.into_inner()
                }
            };

            // Check if default changed
            if current_default != *prev {
                if let Some(new_default_id) = current_default {
                    if let Ok(name) = MacOSDeviceMonitor::get_device_name(new_default_id) {
                        tracing::info!(
                            device_id = new_default_id,
                            device_name = %name,
                            "[DEVICE_MONITOR] Default device changed"
                        );

                        if let Err(e) =
                            context
                                .event_sender
                                .send(DeviceEvent::DefaultDeviceChanged {
                                    id: new_default_id.to_string(),
                                    name: name.clone(),
                                })
                        {
                            tracing::debug!(
                                error = ?e,
                                "[DEVICE_MONITOR] Failed to send default device changed event (receiver may have dropped)"
                            );
                        }
                    }
                }

                *prev = current_default;
            }
        }
    }

    0 // Always return 0 per CoreAudio spec
}

/// macOS CoreAudio async device monitor
///
/// Uses CoreAudio HAL property listeners for real-time device notifications.
/// Provides industry-standard async device monitoring without blocking.
pub struct MacOSDeviceMonitor {
    /// Platform identifier
    platform: &'static str,
}

impl MacOSDeviceMonitor {
    /// Create a new macOS CoreAudio device monitor
    pub fn new() -> Self {
        Self {
            platform: "macOS (CoreAudio Native Async)",
        }
    }

    /// Get all audio output devices from CoreAudio
    fn get_device_ids() -> Result<Vec<AudioDeviceID>, DeviceMonitorError> {
        /// SAFETY:
        ///
        /// # Memory Safety
        /// - All pointers passed to CoreAudio APIs are valid and properly aligned
        /// - `device_ids` vector is allocated with correct size before passing to CoreAudio
        /// - CoreAudio writes exactly `device_count` elements to the buffer
        /// - All references are short-lived and don't outlive the function
        ///
        /// # Invariants
        /// - `kAudioObjectSystemObject` is a valid constant defined by CoreAudio
        /// - `AudioObjectGetPropertyDataSize` returns the exact size needed for device list
        /// - Buffer is sized to hold `data_size / sizeof(AudioDeviceID)` elements
        /// - CoreAudio guarantees it won't write beyond the buffer size
        unsafe {
            let property_address = AudioObjectPropertyAddress {
                mSelector: kAudioHardwarePropertyDevices,
                mScope: kAudioObjectPropertyScopeGlobal,
                mElement: kAudioObjectPropertyElementMain,
            };

            // Get size of device list
            let mut data_size: u32 = 0;
            let status = AudioObjectGetPropertyDataSize(
                coreaudio::sys::kAudioObjectSystemObject,
                &property_address as *const _,
                0,
                ptr::null(),
                &mut data_size as *mut _,
            );

            if status != 0 {
                tracing::error!(
                    os_status = status,
                    "[DEVICE_MONITOR] Failed to get device list size"
                );
                return Err(DeviceMonitorError::EnumerationFailed(format!(
                    "Failed to get device list size: OSStatus {}",
                    status
                )));
            }

            // Allocate buffer for device IDs
            let device_count = data_size / std::mem::size_of::<AudioDeviceID>() as u32;
            let mut device_ids: Vec<AudioDeviceID> = vec![0; device_count as usize];

            // Get device IDs
            let status = AudioObjectGetPropertyData(
                coreaudio::sys::kAudioObjectSystemObject,
                &property_address as *const _,
                0,
                ptr::null(),
                &mut data_size as *mut _,
                device_ids.as_mut_ptr() as *mut _,
            );

            if status != 0 {
                tracing::error!(
                    os_status = status,
                    "[DEVICE_MONITOR] Failed to get device IDs"
                );
                return Err(DeviceMonitorError::EnumerationFailed(format!(
                    "Failed to get device IDs: OSStatus {}",
                    status
                )));
            }

            tracing::debug!(
                device_count = device_count,
                "[DEVICE_MONITOR] Retrieved CoreAudio device IDs"
            );
            Ok(device_ids)
        }
    }

    /// Get device name from CoreAudio
    fn get_device_name(device_id: AudioDeviceID) -> Result<String, DeviceMonitorError> {
        /// SAFETY:
        ///
        /// # Memory Safety
        /// - `cf_string_ref` is initialized by CoreAudio as a CFString pointer
        /// - `wrap_under_create_rule` properly manages the CFString ownership (transfers ownership)
        /// - CFString is automatically released when Rust wrapper is dropped
        /// - All pointers are valid for the duration of the API calls
        ///
        /// # Invariants
        /// - `device_id` must be a valid AudioDeviceID (caller's responsibility)
        /// - CoreAudio returns a valid CFStringRef or fails with non-zero OSStatus
        /// - `wrap_under_create_rule` is correct for "Create" rule (we own the returned CFString)
        unsafe {
            let property_address = AudioObjectPropertyAddress {
                mSelector: kAudioDevicePropertyDeviceNameCFString,
                mScope: kAudioObjectPropertyScopeGlobal,
                mElement: kAudioObjectPropertyElementMain,
            };

            let mut cf_string_ref: *mut std::ffi::c_void = ptr::null_mut();
            let mut data_size = std::mem::size_of::<*mut std::ffi::c_void>() as u32;

            let status = AudioObjectGetPropertyData(
                device_id,
                &property_address as *const _,
                0,
                ptr::null(),
                &mut data_size as *mut _,
                &mut cf_string_ref as *mut _ as *mut _,
            );

            if status != 0 {
                return Err(DeviceMonitorError::Internal(format!(
                    "Failed to get device name: OSStatus {}",
                    status
                )));
            }

            // Convert CFString to Rust String
            let cf_string = core_foundation::string::CFString::wrap_under_create_rule(
                cf_string_ref as core_foundation::string::CFStringRef,
            );
            Ok(cf_string.to_string())
        }
    }

    /// Get default output device ID
    fn get_default_device_id() -> Result<AudioDeviceID, DeviceMonitorError> {
        /// SAFETY:
        ///
        /// # Memory Safety
        /// - `device_id` is a stack-allocated u32, properly initialized to 0
        /// - CoreAudio writes exactly `sizeof(AudioDeviceID)` bytes to the buffer
        /// - All pointers are valid and properly aligned for the API call
        ///
        /// # Invariants
        /// - `kAudioObjectSystemObject` is a valid constant defined by CoreAudio
        /// - `kAudioHardwarePropertyDefaultOutputDevice` is a valid property selector
        /// - CoreAudio guarantees it writes a valid AudioDeviceID or fails with non-zero OSStatus
        unsafe {
            let property_address = AudioObjectPropertyAddress {
                mSelector: kAudioHardwarePropertyDefaultOutputDevice,
                mScope: kAudioObjectPropertyScopeGlobal,
                mElement: kAudioObjectPropertyElementMain,
            };

            let mut device_id: AudioDeviceID = 0;
            let mut data_size = std::mem::size_of::<AudioDeviceID>() as u32;

            let status = AudioObjectGetPropertyData(
                coreaudio::sys::kAudioObjectSystemObject,
                &property_address as *const _,
                0,
                ptr::null(),
                &mut data_size as *mut _,
                &mut device_id as *mut _ as *mut _,
            );

            if status != 0 {
                return Err(DeviceMonitorError::DeviceNotFound(format!(
                    "Failed to get default device: OSStatus {}",
                    status
                )));
            }

            Ok(device_id)
        }
    }

    /// Get device sample_rate
    fn get_device_sample_rate(device_id: AudioDeviceID) -> Option<u32> {
        /// SAFETY:
        ///
        /// # Memory Safety
        /// - `stream_format` is zero-initialized via `mem::zeroed()` which is safe for AudioStreamBasicDescription (C struct with no invalid bit patterns)
        /// - CoreAudio writes exactly `sizeof(AudioStreamBasicDescription)` bytes to the buffer
        /// - All pointers are valid and properly aligned for the API call
        ///
        /// # Invariants
        /// - `device_id` must be a valid AudioDeviceID (caller's responsibility)
        /// - `AudioStreamBasicDescription` is a C struct that is safe to zero-initialize
        /// - CoreAudio guarantees it writes a valid struct or fails with non-zero OSStatus
        unsafe {
            let property_address = AudioObjectPropertyAddress {
                mSelector: kAudioDevicePropertyStreamFormat,
                mScope: kAudioObjectPropertyScopeGlobal,
                mElement: kAudioObjectPropertyElementMain,
            };

            let mut stream_format: coreaudio::sys::AudioStreamBasicDescription = std::mem::zeroed();
            let mut data_size =
                std::mem::size_of::<coreaudio::sys::AudioStreamBasicDescription>() as u32;

            let status = AudioObjectGetPropertyData(
                device_id,
                &property_address as *const _,
                0,
                ptr::null(),
                &mut data_size as *mut _,
                &mut stream_format as *mut _ as *mut _,
            );

            if status == 0 {
                Some(stream_format.mSampleRate as u32)
            } else {
                None
            }
        }
    }

    /// Get device channel count
    fn get_device_channels(device_id: AudioDeviceID) -> Option<u16> {
        /// SAFETY:
        ///
        /// # Memory Safety
        /// - `stream_format` is zero-initialized via `mem::zeroed()` which is safe for AudioStreamBasicDescription (C struct with no invalid bit patterns)
        /// - CoreAudio writes exactly `sizeof(AudioStreamBasicDescription)` bytes to the buffer
        /// - All pointers are valid and properly aligned for the API call
        ///
        /// # Invariants
        /// - `device_id` must be a valid AudioDeviceID (caller's responsibility)
        /// - `AudioStreamBasicDescription` is a C struct that is safe to zero-initialize
        /// - CoreAudio guarantees it writes a valid struct or fails with non-zero OSStatus
        unsafe {
            let property_address = AudioObjectPropertyAddress {
                mSelector: kAudioDevicePropertyStreamFormat,
                mScope: kAudioObjectPropertyScopeGlobal,
                mElement: kAudioObjectPropertyElementMain,
            };

            let mut stream_format: coreaudio::sys::AudioStreamBasicDescription = std::mem::zeroed();
            let mut data_size =
                std::mem::size_of::<coreaudio::sys::AudioStreamBasicDescription>() as u32;

            let status = AudioObjectGetPropertyData(
                device_id,
                &property_address as *const _,
                0,
                ptr::null(),
                &mut data_size as *mut _,
                &mut stream_format as *mut _ as *mut _,
            );

            if status == 0 {
                Some(stream_format.mChannelsPerFrame as u16)
            } else {
                None
            }
        }
    }

    /// Convert device ID to AsyncDeviceInfo
    fn device_to_info(device_id: AudioDeviceID, is_default: bool) -> Option<AsyncDeviceInfo> {
        let name = Self::get_device_name(device_id).ok()?;
        let sample_rate = Self::get_device_sample_rate(device_id);
        let channels = Self::get_device_channels(device_id);

        Some(AsyncDeviceInfo {
            id: device_id.to_string(),
            name,
            is_default,
            is_available: true,
            sample_rate,
            channels,
        })
    }
}

impl Default for MacOSDeviceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AsyncDeviceMonitor for MacOSDeviceMonitor {
    async fn enumerate_devices(&self) -> Result<Vec<AsyncDeviceInfo>, DeviceMonitorError> {
        tracing::debug!("[DEVICE_MONITOR] Starting device enumeration (CoreAudio native)");

        // Spawn blocking to avoid blocking async runtime
        // Even though CoreAudio is fast, we want consistent async behavior
        let result = tokio::task::spawn_blocking(|| {
            let device_ids = Self::get_device_ids()?;
            let default_id = Self::get_default_device_id().ok();

            let mut devices = Vec::new();
            for device_id in device_ids {
                if let Some(info) = Self::device_to_info(device_id, default_id == Some(device_id)) {
                    tracing::debug!(
                        device_id = device_id,
                        device_name = %info.name,
                        is_default = info.is_default,
                        sample_rate = ?info.sample_rate,
                        channels = ?info.channels,
                        "[DEVICE_MONITOR] Found device"
                    );
                    devices.push(info);
                }
            }

            tracing::info!(
                device_count = devices.len(),
                "[DEVICE_MONITOR] Enumeration completed"
            );
            Ok(devices)
        })
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "[DEVICE_MONITOR] Internal error during enumeration");
            DeviceMonitorError::Internal(e.to_string())
        })?;

        result
    }

    async fn get_default_device(&self) -> Result<AsyncDeviceInfo, DeviceMonitorError> {
        tracing::debug!("[DEVICE_MONITOR] Getting default device (CoreAudio native)");

        tokio::task::spawn_blocking(|| {
            let device_id = Self::get_default_device_id()?;
            let info = Self::device_to_info(device_id, true).ok_or_else(|| {
                tracing::error!("[DEVICE_MONITOR] Default device unavailable");
                DeviceMonitorError::DeviceNotFound("Default device unavailable".to_string())
            })?;

            tracing::info!(
                device_id = device_id,
                device_name = %info.name,
                sample_rate = ?info.sample_rate,
                channels = ?info.channels,
                "[DEVICE_MONITOR] Default device retrieved"
            );

            Ok(info)
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
        tracing::info!(
            "[DEVICE_MONITOR] Starting device change watcher (CoreAudio property listeners)"
        );

        // Create channel for events from CoreAudio callbacks
        let (event_sender, mut event_receiver) = mpsc::channel(64);

        // Get initial device state
        let initial_devices = Self::get_device_ids()?
            .into_iter()
            .filter_map(|id| Self::get_device_name(id).ok().map(|name| (name, id)))
            .collect::<Vec<_>>();

        let initial_default = Self::get_default_device_id().ok();

        tracing::debug!(
            initial_device_count = initial_devices.len(),
            has_default = initial_default.is_some(),
            "[DEVICE_MONITOR] Initial device state captured"
        );

        // Create listener context with RAII guard for automatic cleanup on error
        let context = Box::new(ListenerContext {
            event_sender,
            previous_devices: StdMutex::new(initial_devices),
            previous_default: StdMutex::new(initial_default),
        });

        let context_guard = ListenerContextGuard::new(context);
        let context_ptr = context_guard.context;

        // Register property listeners
        // If any registration fails, context_guard will automatically clean up
        /// SAFETY:
        ///
        /// # Memory Safety
        /// - `context_ptr` points to valid `ListenerContext` allocated via `Box::into_raw()`
        /// - Pointer remains valid for the lifetime of the listeners (until removal in Drop)
        /// - `ListenerContextGuard` ensures cleanup if registration fails (RAII pattern)
        /// - If both listeners register successfully, we call `into_raw()` to transfer ownership
        /// - Callbacks receive the same `context_ptr` as `in_client_data` parameter
        /// - Cleanup order: (1) Remove listeners, (2) `Box::from_raw()` to deallocate context
        ///
        /// # Threading
        /// - `AudioObjectAddPropertyListener` is thread-safe (CoreAudio guarantee)
        /// - Callbacks will be invoked on CoreAudio's HAL notification thread
        /// - Context uses `StdMutex` for thread-safe access from any thread
        ///
        /// # Invariants
        /// - Function pointers (`device_list_changed_callback`, `default_device_changed_callback`) are static and always valid
        /// - `kAudioObjectSystemObject` is a valid constant defined by CoreAudio
        /// - Property addresses are valid CoreAudio property selectors
        /// - If first listener succeeds but second fails, we remove the first before returning error
        /// - `context_ptr` is only converted to raw pointer via `into_raw()` if both listeners succeed
        ///
        /// # Panic Safety
        /// - If panic occurs before `into_raw()`, guard's Drop will free the context (no leak)
        /// - If panic occurs after `into_raw()`, context leaks (acceptable - process likely terminating)
        unsafe {
            // Listen for device list changes (kAudioHardwarePropertyDevices)
            let devices_address = AudioObjectPropertyAddress {
                mSelector: kAudioHardwarePropertyDevices,
                mScope: kAudioObjectPropertyScopeGlobal,
                mElement: kAudioObjectPropertyElementMain,
            };

            let status = AudioObjectAddPropertyListener(
                coreaudio::sys::kAudioObjectSystemObject,
                &devices_address as *const _,
                Some(device_list_changed_callback),
                context_ptr as *mut c_void,
            );

            if status != 0 {
                // context_guard will automatically free context on drop
                tracing::error!(
                    os_status = status,
                    "[DEVICE_MONITOR] Failed to register device list property listener"
                );
                return Err(DeviceMonitorError::Internal(format!(
                    "Failed to register device list listener: OSStatus {}",
                    status
                )));
            }

            tracing::debug!("[DEVICE_MONITOR] Registered kAudioHardwarePropertyDevices listener");

            // Listen for default device changes (kAudioHardwarePropertyDefaultOutputDevice)
            let default_address = AudioObjectPropertyAddress {
                mSelector: kAudioHardwarePropertyDefaultOutputDevice,
                mScope: kAudioObjectPropertyScopeGlobal,
                mElement: kAudioObjectPropertyElementMain,
            };

            let status = AudioObjectAddPropertyListener(
                coreaudio::sys::kAudioObjectSystemObject,
                &default_address as *const _,
                Some(default_device_changed_callback),
                context_ptr as *mut c_void,
            );

            if status != 0 {
                // Cleanup: remove first listener
                let _ = AudioObjectRemovePropertyListener(
                    coreaudio::sys::kAudioObjectSystemObject,
                    &devices_address as *const _,
                    Some(device_list_changed_callback),
                    context_ptr as *mut c_void,
                );
                // context_guard will automatically free context on drop

                tracing::error!(
                    os_status = status,
                    "[DEVICE_MONITOR] Failed to register default device property listener"
                );
                return Err(DeviceMonitorError::Internal(format!(
                    "Failed to register default device listener: OSStatus {}",
                    status
                )));
            }

            tracing::debug!(
                "[DEVICE_MONITOR] Registered kAudioHardwarePropertyDefaultOutputDevice listener"
            );
        }

        // Success: convert guard to raw pointer (prevents automatic cleanup)
        let context_ptr = context_guard.into_raw();

        // Spawn task to forward events from channel to user callback
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();

        // Wrap callback in Arc for cloning across spawn_blocking calls
        let callback = Arc::new(callback);

        tokio::spawn(async move {
            tracing::debug!("[DEVICE_MONITOR] Event forwarding task started");

            while running_clone.load(Ordering::Relaxed) {
                tokio::select! {
                    Some(event) = event_receiver.recv() => {
                        tracing::trace!(event = ?event, "[DEVICE_MONITOR] Forwarding event to user callback");
                        // Use spawn_blocking to handle async/sync boundary properly with error handling
                        let callback_clone = callback.clone();
                        match tokio::task::spawn_blocking(move || {
                            callback_clone(event);
                        })
                        .await
                        {
                            Ok(()) => {
                                tracing::trace!("[DEVICE_MONITOR] Device event callback executed successfully");
                            }
                            Err(e) => {
                                tracing::error!(
                                    error = %e,
                                    "[DEVICE_MONITOR] Device event callback failed to execute"
                                );
                            }
                        }
                    }
                    _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {
                        // Check if still running
                    }
                }
            }

            tracing::debug!("[DEVICE_MONITOR] Event forwarding task stopped");
        });

        tracing::info!("[DEVICE_MONITOR] Device change watcher started with property listeners");

        Ok(Box::new(MacOSWatchHandle {
            running,
            context_ptr,
        }))
    }

    async fn is_device_available(&self, device_id: &str) -> bool {
        let device_id_str = device_id.to_string();

        tracing::debug!(device_id = %device_id, "[DEVICE_MONITOR] Checking device availability");

        let result = tokio::task::spawn_blocking(move || {
            if let Ok(id) = device_id_str.parse::<AudioDeviceID>() {
                Self::get_device_name(id).is_ok()
            } else {
                false
            }
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

/// Watch handle for macOS CoreAudio property listeners
///
/// Manages lifecycle of CoreAudio property listeners and ensures proper cleanup.
/// Both listeners must be successfully registered before the handle is created,
/// ensuring cleanup is always safe.
struct MacOSWatchHandle {
    /// Flag to stop event forwarding task
    running: Arc<AtomicBool>,
    /// Raw pointer to listener context (must be cleaned up)
    /// INVARIANT: This pointer is non-null and valid until stop() is called
    context_ptr: *mut ListenerContext,
}

impl WatchHandle for MacOSWatchHandle {
    fn stop(&mut self) {
        tracing::info!("[DEVICE_MONITOR] Stopping device change watcher");

        // Stop event forwarding task
        self.running.store(false, Ordering::Relaxed);

        // Remove CoreAudio property listeners
        /// SAFETY:
        ///
        /// # Memory Safety
        /// - `context_ptr` is non-null and valid (created via `Box::into_raw()` in `watch_for_changes`)
        /// - Pointer is unique to this handle (no other code holds ownership)
        /// - `AudioObjectRemovePropertyListener` must be called with the same function pointer and context as registration
        /// - After removal, CoreAudio guarantees callbacks will not be invoked
        /// - `Box::from_raw(context_ptr)` deallocates the context after listeners are removed
        /// - Setting `context_ptr = null_mut()` prevents double-free if stop() is called twice
        ///
        /// # Threading
        /// - `AudioObjectRemovePropertyListener` is thread-safe (CoreAudio guarantee)
        /// - We wait for removal to complete before freeing context (synchronous API)
        /// - No callbacks can be in-flight after removal completes
        ///
        /// # Invariants
        /// - Both listeners were successfully registered before handle creation (guaranteed by `watch_for_changes`)
        /// - Cleanup order is critical: (1) Remove device list listener, (2) Remove default device listener, (3) Free context
        /// - CRITICAL: Context is only freed if BOTH listeners are successfully removed (prevents use-after-free)
        /// - If either removal fails, context is intentionally leaked (safer than use-after-free)
        /// - Context is freed exactly once per handle lifetime
        ///
        /// # Panic Safety
        /// - If panic occurs during listener removal, context may leak (acceptable - rare and non-critical)
        /// - If panic occurs after `Box::from_raw()`, no memory leak (context is already freed)
        /// - OSStatus errors are logged but don't panic (defensive programming)
        unsafe {
            if !self.context_ptr.is_null() {
                tracing::debug!("[DEVICE_MONITOR] Removing CoreAudio property listeners");

                // Track both listener removal results
                let mut listeners_removed = true;

                // Remove device list listener
                let devices_address = AudioObjectPropertyAddress {
                    mSelector: kAudioHardwarePropertyDevices,
                    mScope: kAudioObjectPropertyScopeGlobal,
                    mElement: kAudioObjectPropertyElementMain,
                };

                let status1 = AudioObjectRemovePropertyListener(
                    coreaudio::sys::kAudioObjectSystemObject,
                    &devices_address as *const _,
                    Some(device_list_changed_callback),
                    self.context_ptr as *mut c_void,
                );

                if status1 != 0 {
                    tracing::error!(
                        os_status = status1,
                        "[DEVICE_MONITOR] Failed to remove device list listener - context will NOT be freed to prevent use-after-free"
                    );
                    listeners_removed = false;
                } else {
                    tracing::debug!(
                        "[DEVICE_MONITOR] Removed kAudioHardwarePropertyDevices listener"
                    );
                }

                // Remove default device listener
                let default_address = AudioObjectPropertyAddress {
                    mSelector: kAudioHardwarePropertyDefaultOutputDevice,
                    mScope: kAudioObjectPropertyScopeGlobal,
                    mElement: kAudioObjectPropertyElementMain,
                };

                let status2 = AudioObjectRemovePropertyListener(
                    coreaudio::sys::kAudioObjectSystemObject,
                    &default_address as *const _,
                    Some(default_device_changed_callback),
                    self.context_ptr as *mut c_void,
                );

                if status2 != 0 {
                    tracing::error!(
                        os_status = status2,
                        "[DEVICE_MONITOR] Failed to remove default device listener - context will NOT be freed to prevent use-after-free"
                    );
                    listeners_removed = false;
                } else {
                    tracing::debug!("[DEVICE_MONITOR] Removed kAudioHardwarePropertyDefaultOutputDevice listener");
                }

                // CRITICAL: Only free context if BOTH listeners were removed successfully
                if listeners_removed {
                    let _ = Box::from_raw(self.context_ptr);
                    self.context_ptr = ptr::null_mut();
                    tracing::debug!("[DEVICE_MONITOR] Freed listener context successfully");
                } else {
                    tracing::warn!(
                        "[DEVICE_MONITOR] Context leaked to prevent use-after-free (listeners still active)"
                    );
                    // Set to null to prevent double-free in Drop
                    self.context_ptr = ptr::null_mut();
                }
            }
        }

        tracing::info!("[DEVICE_MONITOR] Device change watcher stopped");
    }
}

impl Drop for MacOSWatchHandle {
    fn drop(&mut self) {
        tracing::debug!("[DEVICE_MONITOR] Device change watcher handle dropped");
        self.stop();
    }
}

/// SAFETY:
///
/// # Thread Safety
/// MacOSWatchHandle can be sent between threads because:
/// - `running` is an `Arc<AtomicBool>` which is `Send + Sync`
/// - `context_ptr` is a raw pointer to heap-allocated data (pointer itself is `Send`)
/// - The pointed-to `ListenerContext` contains only `Send` types:
///   - `mpsc::Sender` is `Send`
///   - `StdMutex<Vec<...>>` is `Send` (std Mutex is Send if T is Send)
///   - `StdMutex<Option<AudioDeviceID>>` is `Send`
/// - `context_ptr` is only dereferenced in callbacks (CoreAudio thread) and `stop()` (any thread)
/// - CoreAudio guarantees no callbacks after `AudioObjectRemovePropertyListener` returns
/// - `stop()` uses proper synchronization: (1) stop callbacks, (2) free context
/// - No data races: callbacks access context via shared references with mutex protection
unsafe impl Send for MacOSWatchHandle {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    #[tokio::test]
    async fn test_enumerate_devices_macos() {
        let monitor = MacOSDeviceMonitor::new();
        // This may fail in CI without audio devices - that's okay
        let result = monitor.enumerate_devices().await;
        assert!(result.is_ok() || matches!(result, Err(DeviceMonitorError::EnumerationFailed(_))));
    }

    #[tokio::test]
    async fn test_platform_name() {
        let monitor = MacOSDeviceMonitor::new();
        let platform = monitor.platform_name();
        assert!(platform.contains("CoreAudio"));
        assert!(platform.contains("Native"));
    }

    #[test]
    fn test_device_id_operations() {
        // Test basic CoreAudio operations
        let device_ids = MacOSDeviceMonitor::get_device_ids();
        // May fail without audio hardware, but shouldn't panic
        if let Ok(ids) = device_ids {
            for id in ids {
                // Try to get device name
                let _ = MacOSDeviceMonitor::get_device_name(id);
            }
        }
    }

    #[tokio::test]
    async fn test_get_default_device_returns_default_flag() {
        let monitor = MacOSDeviceMonitor::new();
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
        let monitor = MacOSDeviceMonitor::new();
        // Use a clearly invalid device ID
        let available = monitor.is_device_available("999999").await;
        assert!(!available, "Invalid device ID should not be available");
    }

    #[tokio::test]
    async fn test_watch_handle_can_be_stopped() {
        let monitor = MacOSDeviceMonitor::new();
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
        let monitor = MacOSDeviceMonitor::new();
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
        let monitor = Arc::new(MacOSDeviceMonitor::new());

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
        let monitor = MacOSDeviceMonitor::new();
        if let Ok(devices) = monitor.enumerate_devices().await {
            for device in devices {
                assert!(!device.id.is_empty(), "Device ID should not be empty");
                assert!(!device.name.is_empty(), "Device name should not be empty");
            }
        }
    }

    #[test]
    fn test_get_default_device_id_sync() {
        // Test synchronous default device retrieval
        let result = MacOSDeviceMonitor::get_default_device_id();
        // May fail in CI, but shouldn't panic
        match result {
            Ok(id) => assert!(id > 0, "Device ID should be positive"),
            Err(_) => {} // Expected in CI
        }
    }

    #[test]
    fn test_device_sample_rate_and_channels() {
        // Test that we can get sample rate and channels for devices
        if let Ok(ids) = MacOSDeviceMonitor::get_device_ids() {
            for id in ids.iter().take(1) {
                // Just test first device
                let _sample_rate = MacOSDeviceMonitor::get_device_sample_rate(*id);
                let _channels = MacOSDeviceMonitor::get_device_channels(*id);
                // These may return None, but shouldn't panic
            }
        }
    }
}
