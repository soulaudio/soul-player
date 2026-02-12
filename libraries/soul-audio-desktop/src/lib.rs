#![allow(clippy::match_same_arms)]
#![allow(clippy::return_self_not_must_use)]
#![allow(clippy::match_wildcard_for_single_variants)]
#![allow(clippy::needless_range_loop)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_lossless)]
#![allow(clippy::filter_next)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::assigning_clones)]
#![allow(clippy::manual_let_else)]
#![allow(clippy::comparison_to_empty)]

//! Desktop audio output implementation using CPAL
//!
//! This crate provides the `CpalOutput` implementation of the `AudioOutput` trait
//! for cross-platform desktop audio playback.
//!
//! # Features
//!
//! - Cross-platform audio output using CPAL
//! - Automatic sample rate conversion
//! - Volume control
//! - Playback controls (play, pause, resume, stop)
//!
//! # Example
//!
//! ```no_run
//! use soul_audio_desktop::CpalOutput;
//! use soul_core::{AudioOutput, AudioBuffer, AudioFormat, SampleRate};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Create audio output
//! let mut output = CpalOutput::new()?;
//!
//! // Create a test buffer
//! let format = AudioFormat::new(SampleRate::CD_QUALITY, 2, 32);
//! let buffer = AudioBuffer::new(vec![0.0; 44100 * 2], format);
//!
//! // Play the buffer
//! output.play(&buffer)?;
//!
//! // Control playback
//! output.set_volume(0.5)?;
//! output.pause()?;
//! output.resume()?;
//! output.stop()?;
//! # Ok(())
//! # }
//! ```

// Allow unsafe code for platform-specific device monitoring (CoreAudio FFI, WinRT, PipeWire)
// The unsafe code is isolated to device_monitor_* modules and wraps low-level platform APIs
#![cfg_attr(
    any(target_os = "macos", target_os = "linux", target_os = "windows"),
    allow(unsafe_code)
)]
#![cfg_attr(
    not(any(target_os = "macos", target_os = "linux", target_os = "windows")),
    deny(unsafe_code)
)]

pub mod backend;
pub mod device;
pub mod device_check_timeout;
pub mod device_manager;
pub mod device_monitor_async;
mod device_monitor_cpal_fallback;
#[cfg(all(target_os = "linux", feature = "native-device-monitor"))]
mod device_monitor_linux;
#[cfg(all(target_os = "macos", feature = "native-device-monitor"))]
mod device_monitor_macos;
#[cfg(all(target_os = "windows", feature = "native-device-monitor"))]
mod device_monitor_windows;
mod error;
pub mod exclusive;
mod output;
pub mod playback;
pub mod sources;
pub mod track_loader;

pub use backend::{AudioBackend, BackendError, BackendInfo};
// Device module exports
// NOTE: Some functions are deprecated in favor of AsyncDeviceMonitor
#[allow(deprecated)]
pub use device::{
    // Still useful: Capability detection, device lookup, and types
    detect_device_capabilities,
    find_device_by_name,
    // DEPRECATED: Use AsyncDeviceMonitor instead
    get_default_device,
    get_default_device_with_capabilities,
    get_device_capabilities,
    list_devices,
    list_devices_with_capabilities,
    AudioDeviceInfo,
    DeviceCapabilities,
    DeviceError,
    SupportedBitDepth,
    DSD_RATES,
    STANDARD_SAMPLE_RATES,
};
pub use device_check_timeout::{
    device_check_with_timeout_sync, device_check_with_timeout_sync_custom, TimeoutConfig,
    TimeoutTracker, DEFAULT_DEVICE_CHECK_TIMEOUT,
};
pub use device_monitor_async::{
    create_async_device_monitor, AsyncDeviceInfo, AsyncDeviceMonitor, DeviceChangeCallback,
    DeviceEvent, DeviceMonitorError, WatchHandle,
};
pub use error::{AudioError, AudioOutputError, Result};
pub use exclusive::{AudioData, ExclusiveConfig, ExclusiveOutput, LatencyInfo};
pub use output::{CpalOutput, ResamplingQuality};
pub use playback::{
    DesktopPlayback, DeviceSwitchConfig, DeviceSwitchReason, DeviceSwitchState, PlaybackCommand,
    PlaybackEvent, ResamplingSettings, SampleRateMode,
};
pub use sources::{LocalAudioSource, StreamingAudioSource};
pub use track_loader::{LoadRequest, LoadResult, TrackLoader};
