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

#![deny(unsafe_code)]

pub mod backend;
pub mod device;
mod error;
pub mod exclusive;
mod output;
pub mod playback;
pub mod sources;
pub mod track_loader;

pub use backend::{AudioBackend, BackendError, BackendInfo};
pub use device::{
    detect_device_capabilities, get_default_device_with_capabilities, get_device_capabilities,
    list_devices_with_capabilities, AudioDeviceInfo, DeviceCapabilities, DeviceError,
    SupportedBitDepth, DSD_RATES, STANDARD_SAMPLE_RATES,
};
pub use error::{AudioError, AudioOutputError, Result};
pub use exclusive::{AudioData, ExclusiveConfig, ExclusiveOutput, LatencyInfo};
pub use output::{CpalOutput, ResamplingQuality};
pub use playback::{DesktopPlayback, PlaybackCommand, PlaybackEvent, ResamplingSettings, SampleRateMode};
pub use sources::{LocalAudioSource, StreamingAudioSource};
pub use track_loader::{LoadRequest, LoadResult, TrackLoader};
