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
pub mod device_monitor_cpal_fallback;
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

// ===== Sample Trait for Generic Audio Callback =====

/// Trait for audio sample formats
///
/// Enables a single generic audio callback to work with multiple sample formats
/// (f32, i32, i16) instead of duplicating callback code three times.
pub trait Sample: Copy + Send + 'static {
    /// Convert a single f32 sample to this format
    fn from_f32(sample: f32) -> Self;

    /// Convert a slice of f32 samples to this format
    fn from_f32_slice(input: &[f32], output: &mut [Self]);

    /// Fill a buffer with silence (zeros)
    fn fill_silence(buffer: &mut [Self]);
}

impl Sample for f32 {
    #[inline]
    fn from_f32(sample: f32) -> Self {
        sample
    }

    #[inline]
    fn from_f32_slice(input: &[f32], output: &mut [Self]) {
        output.copy_from_slice(input);
    }

    #[inline]
    fn fill_silence(buffer: &mut [Self]) {
        buffer.fill(0.0);
    }
}

impl Sample for i32 {
    #[inline]
    fn from_f32(sample: f32) -> Self {
        // Apply TPDF dithering for high-quality conversion
        // TPDF (Triangular Probability Density Function) dithering
        // eliminates quantization distortion and correlation artifacts
        let dither = tpdf_dither_i32();
        let scaled = sample * 2147483648.0; // Scale to i32 range
        (scaled + dither).clamp(-2147483648.0, 2147483647.0) as i32
    }

    #[inline]
    fn from_f32_slice(input: &[f32], output: &mut [Self]) {
        for (i, &sample) in input.iter().enumerate() {
            output[i] = Self::from_f32(sample);
        }
    }

    #[inline]
    fn fill_silence(buffer: &mut [Self]) {
        buffer.fill(0);
    }
}

impl Sample for i16 {
    #[inline]
    fn from_f32(sample: f32) -> Self {
        // Apply TPDF dithering for high-quality conversion
        let dither = tpdf_dither_i16();
        let scaled = sample * 32768.0; // Scale to i16 range
        (scaled + dither).clamp(-32768.0, 32767.0) as i16
    }

    #[inline]
    fn from_f32_slice(input: &[f32], output: &mut [Self]) {
        for (i, &sample) in input.iter().enumerate() {
            output[i] = Self::from_f32(sample);
        }
    }

    #[inline]
    fn fill_silence(buffer: &mut [Self]) {
        buffer.fill(0);
    }
}

/// TPDF dithering for i32 conversion
///
/// Uses thread-local LFSR for fast pseudo-random generation
#[inline]
fn tpdf_dither_i32() -> f32 {
    use std::cell::Cell;
    thread_local! {
        static STATE: Cell<u32> = const { Cell::new(0xDEADBEEF) };
    }
    STATE.with(|state| {
        let mut s = state.get();
        // Simple LFSR for fast pseudo-random generation
        s ^= s << 13;
        s ^= s >> 17;
        s ^= s << 5;
        state.set(s);
        let r1 = (s & 0xFFFF) as f32 / 65536.0;

        s ^= s << 13;
        s ^= s >> 17;
        s ^= s << 5;
        state.set(s);
        let r2 = (s & 0xFFFF) as f32 / 65536.0;

        (r1 + r2 - 1.0) * 0.5 // TPDF: sum of two uniform random variables
    })
}

/// TPDF dithering for i16 conversion
#[inline]
fn tpdf_dither_i16() -> f32 {
    use std::cell::Cell;
    thread_local! {
        static STATE: Cell<u32> = const { Cell::new(0xCAFEBABE) };
    }
    STATE.with(|state| {
        let mut s = state.get();
        // Simple LFSR for fast pseudo-random generation
        s ^= s << 13;
        s ^= s >> 17;
        s ^= s << 5;
        state.set(s);
        let r1 = (s & 0xFFFF) as f32 / 65536.0;

        s ^= s << 13;
        s ^= s >> 17;
        s ^= s << 5;
        state.set(s);
        let r2 = (s & 0xFFFF) as f32 / 65536.0;

        (r1 + r2 - 1.0) * 0.5
    })
}

// ===== Public Exports =====

pub use backend::{
    get_backend_info_async, get_backend_info_with_timeout, AudioBackend, BackendError, BackendInfo,
    BACKEND_ENUM_TIMEOUT_SECS,
};
// Device module exports
// NOTE: Some functions are deprecated in favor of AsyncDeviceMonitor
#[allow(deprecated)]
pub use device::{
    // Still useful: Capability detection, device lookup, and types
    detect_device_capabilities,
    find_device_by_name,
    find_device_by_name_async,
    find_device_by_name_with_timeout,
    // DEPRECATED: Use AsyncDeviceMonitor instead
    get_default_device,
    get_default_device_async,
    get_default_device_with_capabilities,
    get_default_device_with_capabilities_async,
    get_default_device_with_capabilities_with_timeout,
    get_default_device_with_timeout,
    get_device_capabilities,
    get_device_capabilities_async,
    get_device_capabilities_with_timeout,
    list_devices,
    list_devices_async,
    list_devices_with_capabilities,
    list_devices_with_capabilities_async,
    list_devices_with_capabilities_with_timeout,
    list_devices_with_timeout,
    AudioDeviceInfo,
    DeviceCapabilities,
    DeviceError,
    SupportedBitDepth,
    DEVICE_ENUM_TIMEOUT_SECS,
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
pub use device_monitor_cpal_fallback::detect_device_changes;
pub use error::{AudioError, AudioOutputError, Result};
pub use exclusive::{AudioData, ExclusiveConfig, ExclusiveOutput, LatencyInfo};
pub use output::{CpalOutput, ResamplingQuality};
pub use playback::{
    DesktopPlayback, DeviceSwitchConfig, DeviceSwitchReason, DeviceSwitchState, PlaybackCommand,
    PlaybackEvent, ResamplingSettings, SampleRateMode,
};
// Re-export Receiver for event loops to use without mutex contention
pub use crossbeam_channel::Receiver;
pub use sources::{DsdAudioSource, LocalAudioSource, StreamingAudioSource};
