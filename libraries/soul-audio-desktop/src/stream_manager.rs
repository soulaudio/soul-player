//! Audio stream management helpers
//!
//! This module provides reusable components for CPAL stream management:
//! - StreamStartEnvelope: 30ms fade to prevent DAC pops at stream start
//! - CallbackDropGuard: Diagnostics for ASIO stream lifecycle
//! - Helper functions for stream configuration

use cpal::{Device, StreamConfig};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::error::Result;

/// Global counter for I32 (ASIO) callbacks - used for diagnostics
/// This is updated by audio callbacks and can be read for debugging
pub static GLOBAL_I32_CALLBACK_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Stream-level fade envelope to prevent clicks/pops at audio stream start
///
/// When a CPAL audio stream first starts, the DAC may be in an undefined state.
/// Jumping directly to audio output can cause a hardware-level pop.
/// This envelope applies a 30ms fade at stream start to let the DAC settle.
///
/// See: <https://www.kernel.org/doc/html/v4.13/sound/soc/pops-clicks.html>
pub struct StreamStartEnvelope {
    /// Current position in the fade (in stereo samples)
    position: usize,
    /// Total duration of fade (in stereo samples)
    duration: usize,
    /// Whether fade has completed
    completed: bool,
}

/// Stream start fade duration in milliseconds (30ms recommended by Linux kernel docs)
const STREAM_START_FADE_MS: u32 = 30;

impl StreamStartEnvelope {
    /// Create a new stream start envelope for the given sample rate
    pub fn new(sample_rate: u32, channels: u16) -> Self {
        // Calculate duration in samples: sample_rate * duration_ms / 1000 * channels
        let duration =
            ((sample_rate as u64 * STREAM_START_FADE_MS as u64 * channels as u64) / 1000) as usize;
        Self {
            position: 0,
            duration,
            completed: false,
        }
    }

    /// Apply the stream start envelope to an audio buffer
    ///
    /// Uses a smooth S-curve for natural-sounding fade.
    /// Returns true if the fade is still active, false if completed.
    #[inline]
    pub fn process(&mut self, buffer: &mut [f32]) -> bool {
        if self.completed {
            return false;
        }

        // Debug: log first buffer
        if self.position == 0 {
            tracing::debug!(
                "[StreamEnvelope] Processing FIRST buffer: {} samples, duration: {} samples",
                buffer.len(),
                self.duration
            );
            if buffer.len() >= 4 {
                tracing::debug!(
                    "[StreamEnvelope] Input samples: [{:.6}, {:.6}, {:.6}, {:.6}]",
                    buffer[0],
                    buffer[1],
                    buffer[2],
                    buffer[3]
                );
            }
        }

        let remaining = self.duration.saturating_sub(self.position);
        let samples_to_process = buffer.len().min(remaining);

        // Apply S-curve fade (smoother than linear)
        // S-curve: (1 - cos(π * t)) / 2
        for i in 0..samples_to_process {
            let progress = (self.position + i) as f32 / self.duration as f32;
            // S-curve formula for smooth start and end
            let gain = (1.0 - (std::f32::consts::PI * progress).cos()) * 0.5;
            buffer[i] *= gain;
        }

        self.position += samples_to_process;

        if self.position >= self.duration {
            self.completed = true;
            tracing::debug!(
                "[StreamEnvelope] Fade COMPLETED after {} samples",
                self.position
            );
        }

        !self.completed
    }

    /// Process i32 buffer (for ASIO)
    #[inline]
    pub fn process_i32(&mut self, buffer: &mut [i32]) -> bool {
        if self.completed {
            return false;
        }

        let remaining = self.duration.saturating_sub(self.position);
        let samples_to_process = buffer.len().min(remaining);

        for i in 0..samples_to_process {
            let progress = (self.position + i) as f32 / self.duration as f32;
            let gain = (1.0 - (std::f32::consts::PI * progress).cos()) * 0.5;
            buffer[i] = (buffer[i] as f32 * gain) as i32;
        }

        self.position += samples_to_process;

        if self.position >= self.duration {
            self.completed = true;
        }

        !self.completed
    }

    /// Process i16 buffer
    #[inline]
    pub fn process_i16(&mut self, buffer: &mut [i16]) -> bool {
        if self.completed {
            return false;
        }

        let remaining = self.duration.saturating_sub(self.position);
        let samples_to_process = buffer.len().min(remaining);

        for i in 0..samples_to_process {
            let progress = (self.position + i) as f32 / self.duration as f32;
            let gain = (1.0 - (std::f32::consts::PI * progress).cos()) * 0.5;
            buffer[i] = (buffer[i] as f32 * gain) as i16;
        }

        self.position += samples_to_process;

        if self.position >= self.duration {
            self.completed = true;
        }

        !self.completed
    }
}

/// Drop guard for detecting when callback closures are dropped
///
/// This helps diagnose ASIO stream issues where the callback is silently dropped.
/// When a callback closure is dropped, this guard will log an error to help debug
/// stream lifecycle issues.
pub struct CallbackDropGuard {
    /// Unique identifier for the stream (creation timestamp)
    pub stream_id: std::time::Instant,
    /// Sample format of the stream (for diagnostics)
    pub sample_format: &'static str,
}

impl Drop for CallbackDropGuard {
    fn drop(&mut self) {
        tracing::error!(
            "[CallbackDropGuard] !!! {} stream {:?} callback closure is being DROPPED !!!",
            self.sample_format,
            self.stream_id
        );
        tracing::error!(
            "[CallbackDropGuard] This means the ASIO/audio callback will no longer be called."
        );
        tracing::error!("[CallbackDropGuard] The command_rx receiver will be dropped, causing channel disconnect.");
    }
}

/// Get stream configuration from device
///
/// Returns (`StreamConfig`, `SampleFormat`)
///
/// IMPORTANT: Always uses the device's ACTUAL configured sample rate from
/// `default_output_config()`. We don't try to request a different rate because:
/// - ASIO: Sample rate is fixed by the driver control panel
/// - WASAPI Shared: Sample rate is fixed by Windows sound settings
/// - WASAPI Exclusive: Can change rate, but `default_output_config` gives us the current one
///
/// If we request a different rate than what the device is actually running at,
/// the audio will play at the wrong speed (e.g., requesting 96kHz when device
/// is at 48kHz will play audio at 2x speed).
pub fn get_stream_config(device: &Device) -> Result<(StreamConfig, cpal::SampleFormat)> {
    use cpal::traits::DeviceTrait;

    // Get the device's ACTUAL current configuration
    // This is the sample rate the device is really running at
    let default_config = device.default_output_config()?;
    let actual_sample_rate = default_config.sample_rate();

    tracing::debug!(
        "[CPAL] Device's actual sample rate: {:?}",
        actual_sample_rate
    );
    tracing::debug!(
        "[CPAL] Device's default config: channels={}, format={:?}",
        default_config.channels(),
        default_config.sample_format()
    );

    // Also log supported configs for debugging
    tracing::debug!("[CPAL] Checking supported output configurations...");
    let supported_configs: Vec<_> = device
        .supported_output_configs()
        .map(|configs| configs.collect())
        .unwrap_or_default();

    for cfg in &supported_configs {
        tracing::debug!(
            "[CPAL]   Supported: channels={}, sample_rate={:?}-{:?}, format={:?}",
            cfg.channels(),
            cfg.min_sample_rate(),
            cfg.max_sample_rate(),
            cfg.sample_format()
        );
    }

    // Find a config that matches the device's actual sample rate
    // Prefer stereo, then prefer f32 > i32 > i16
    let matching_config = supported_configs
        .iter()
        .filter(|c| {
            // Config must support the device's actual sample rate
            c.min_sample_rate() <= actual_sample_rate && c.max_sample_rate() >= actual_sample_rate
        })
        .filter(|c| c.channels() == 2) // Prefer stereo
        .max_by_key(|c| {
            // Prefer f32 > i32 > i16
            match c.sample_format() {
                cpal::SampleFormat::F32 => 3,
                cpal::SampleFormat::I32 => 2,
                cpal::SampleFormat::I16 => 1,
                _ => 0,
            }
        })
        .or_else(|| {
            // Fallback: any config that supports the actual sample rate
            supported_configs
                .iter()
                .filter(|c| {
                    c.min_sample_rate() <= actual_sample_rate
                        && c.max_sample_rate() >= actual_sample_rate
                })
                .next()
        });

    let config = if let Some(cfg) = matching_config {
        // Use the config with the device's ACTUAL sample rate
        (*cfg).with_sample_rate(actual_sample_rate)
    } else {
        // Fall back to default config (which already has the actual sample rate)
        tracing::debug!("[CPAL] No matching config found, using default");
        default_config
    };

    let sample_format = config.sample_format();

    tracing::debug!("[CPAL] Selected config:");
    tracing::debug!(
        "  - Sample rate: {:?} (device's actual rate)",
        config.sample_rate()
    );
    tracing::debug!("  - Channels: {}", config.channels());
    tracing::debug!("  - Sample format: {:?}", sample_format);
    tracing::debug!("  - Buffer size: {:?}", config.buffer_size());

    // Convert to StreamConfig
    let mut stream_config: StreamConfig = config.clone().into();

    // ASIO and some other drivers require an explicit buffer size
    // Handle different buffer size configurations
    match config.buffer_size() {
        cpal::SupportedBufferSize::Range { min, max } => {
            // Use a buffer size that's a power of 2 and within range
            // Common ASIO buffer sizes: 64, 128, 256, 512, 1024
            let preferred_sizes = [256u32, 512, 128, 1024, 64, 2048];
            let buffer_size = preferred_sizes
                .iter()
                .find(|&&size| size >= *min && size <= *max)
                .copied()
                .unwrap_or(*min.max(&16));

            stream_config.buffer_size = cpal::BufferSize::Fixed(buffer_size);
            tracing::debug!(
                "[CPAL] Using fixed buffer size: {} frames (range: {}-{})",
                buffer_size,
                min,
                max
            );
        }
        cpal::SupportedBufferSize::Unknown => {
            // For unknown buffer size, try a common default
            // Many ASIO drivers work well with 256 or 512
            tracing::debug!("[CPAL] Buffer size unknown, trying default of 512 frames");
            stream_config.buffer_size = cpal::BufferSize::Fixed(512);
        }
    }

    Ok((stream_config, sample_format))
}

/// Increment the global I32 callback counter
///
/// Used for diagnostics to track ASIO callback invocations
#[inline]
pub fn increment_i32_callback_counter() -> u64 {
    GLOBAL_I32_CALLBACK_COUNTER.fetch_add(1, Ordering::Relaxed)
}

/// Get the current I32 callback counter value
///
/// Used for diagnostics
#[inline]
pub fn get_i32_callback_counter() -> u64 {
    GLOBAL_I32_CALLBACK_COUNTER.load(Ordering::Relaxed)
}
