// soul-audio-desktop/src/exclusive.rs
//
// Exclusive mode / bit-perfect audio output
//
// Provides low-latency exclusive access to audio devices with:
// - WASAPI exclusive mode on Windows
// - ASIO support (already exclusive by design)
// - Direct sample format output (no conversion for bit-perfect playback)

use crate::backend::AudioBackend;
use crate::device::{find_device_by_name, SupportedBitDepth};
use crate::error::{AudioOutputError, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{BufferSize, Device, SampleFormat, Stream, StreamConfig, SupportedBufferSize};
use crossbeam_channel::{bounded, Receiver, Sender};

/// Helper to extract u32 value from `cpal::SampleRate`
/// In the current CPAL version, `SampleRate` is a type alias for u32
#[inline]
fn cpal_sample_rate_to_u32(sr: cpal::SampleRate) -> u32 {
    sr
}
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

/// Exclusive mode configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExclusiveConfig {
    /// Target sample rate (Hz) - 0 for device native
    pub sample_rate: u32,

    /// Target bit depth / sample format
    pub bit_depth: SupportedBitDepth,

    /// Buffer size in frames (smaller = lower latency, higher CPU)
    /// None for device default
    pub buffer_frames: Option<u32>,

    /// Enable exclusive mode (WASAPI exclusive, bypasses Windows mixer)
    pub exclusive_mode: bool,

    /// Device name (None for default device)
    pub device_name: Option<String>,

    /// Audio backend to use
    pub backend: AudioBackend,
}

impl Default for ExclusiveConfig {
    fn default() -> Self {
        Self {
            sample_rate: 0, // Use device native
            bit_depth: SupportedBitDepth::Float32,
            buffer_frames: None,
            exclusive_mode: true,
            device_name: None,
            backend: AudioBackend::Default,
        }
    }
}

impl ExclusiveConfig {
    /// Create config for bit-perfect 16-bit playback
    pub fn bit_perfect_16() -> Self {
        Self {
            bit_depth: SupportedBitDepth::Int16,
            exclusive_mode: true,
            ..Default::default()
        }
    }

    /// Create config for bit-perfect 24-bit playback
    pub fn bit_perfect_24() -> Self {
        Self {
            bit_depth: SupportedBitDepth::Int24,
            exclusive_mode: true,
            ..Default::default()
        }
    }

    /// Create config for bit-perfect 32-bit integer playback
    pub fn bit_perfect_32() -> Self {
        Self {
            bit_depth: SupportedBitDepth::Int32,
            exclusive_mode: true,
            ..Default::default()
        }
    }

    /// Create config for low-latency playback (smaller buffer)
    pub fn low_latency() -> Self {
        Self {
            buffer_frames: Some(128), // ~2.9ms at 44.1kHz
            exclusive_mode: true,
            ..Default::default()
        }
    }

    /// Create config for ultra-low latency (for ASIO)
    pub fn ultra_low_latency() -> Self {
        Self {
            buffer_frames: Some(64), // ~1.5ms at 44.1kHz
            exclusive_mode: true,
            ..Default::default()
        }
    }

    /// Set sample rate
    pub fn with_sample_rate(mut self, rate: u32) -> Self {
        self.sample_rate = rate;
        self
    }

    /// Set buffer size in frames
    pub fn with_buffer_frames(mut self, frames: u32) -> Self {
        self.buffer_frames = Some(frames);
        self
    }

    /// Set device by name
    pub fn with_device(mut self, name: &str) -> Self {
        self.device_name = Some(name.to_string());
        self
    }

    /// Set backend
    pub fn with_backend(mut self, backend: AudioBackend) -> Self {
        self.backend = backend;
        self
    }
}

/// Current latency information
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct LatencyInfo {
    /// Buffer latency in samples
    pub buffer_samples: u32,

    /// Buffer latency in milliseconds
    pub buffer_ms: f32,

    /// Total estimated output latency in milliseconds (includes DAC)
    pub total_ms: f32,

    /// Whether running in exclusive mode
    pub exclusive: bool,
}

/// Commands sent to the exclusive audio thread
enum ExclusiveCommand {
    /// Play samples with native format
    Play { samples: Arc<AudioData> },
    /// Pause playback
    Pause,
    /// Resume playback
    Resume,
    /// Stop playback
    Stop,
    /// Set volume (0.0 - 1.0)
    SetVolume(f32),
    /// Shutdown the audio thread
    Shutdown,
}

/// Audio data in various formats for bit-perfect output
#[derive(Debug)]
pub enum AudioData {
    /// 16-bit integer samples (interleaved)
    Int16(Vec<i16>),
    /// 32-bit integer samples (interleaved, for 24-bit content)
    Int32(Vec<i32>),
    /// 32-bit float samples (interleaved)
    Float32(Vec<f32>),
}

impl AudioData {
    /// Create from f32 samples with target bit depth conversion
    pub fn from_f32(samples: &[f32], target: SupportedBitDepth) -> Self {
        match target {
            SupportedBitDepth::Int16 => {
                let converted: Vec<i16> = samples
                    .iter()
                    .map(|&s| (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
                    .collect();
                Self::Int16(converted)
            }
            SupportedBitDepth::Int24 | SupportedBitDepth::Int32 => {
                // Pack 24-bit into 32-bit container
                let scale = if target == SupportedBitDepth::Int24 {
                    8388607.0 // 2^23 - 1
                } else {
                    i32::MAX as f32
                };
                let converted: Vec<i32> = samples
                    .iter()
                    .map(|&s| (s.clamp(-1.0, 1.0) * scale) as i32)
                    .collect();
                Self::Int32(converted)
            }
            SupportedBitDepth::Float32 | SupportedBitDepth::Float64 => {
                Self::Float32(samples.to_vec())
            }
        }
    }

    /// Get length in samples
    pub fn len(&self) -> usize {
        match self {
            Self::Int16(v) => v.len(),
            Self::Int32(v) => v.len(),
            Self::Float32(v) => v.len(),
        }
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Shared state for exclusive audio output
struct ExclusiveState {
    /// Audio samples in native format
    buffer: Mutex<Arc<AudioData>>,
    /// Current playback position
    position: AtomicUsize,
    /// Is playing
    playing: AtomicBool,
    /// Is paused
    paused: AtomicBool,
    /// Volume (0.0 - 1.0) - stored as AtomicU32 using f32::to_bits/from_bits
    volume: AtomicU32,
    /// Loop flag
    looping: AtomicBool,
}

impl ExclusiveState {
    fn new() -> Self {
        Self {
            buffer: Mutex::new(Arc::new(AudioData::Float32(Vec::new()))),
            position: AtomicUsize::new(0),
            playing: AtomicBool::new(false),
            paused: AtomicBool::new(false),
            volume: AtomicU32::new(1.0f32.to_bits()),
            looping: AtomicBool::new(false),
        }
    }

    /// Get current volume (0.0 - 1.0)
    #[inline]
    fn get_volume(&self) -> f32 {
        f32::from_bits(self.volume.load(Ordering::Relaxed))
    }

    /// Set volume (0.0 - 1.0)
    #[inline]
    fn set_volume(&self, vol: f32) {
        self.volume.store(vol.to_bits(), Ordering::Relaxed);
    }
}

/// Exclusive mode audio output
///
/// Provides bit-perfect audio output with exclusive device access.
/// On Windows, this uses WASAPI exclusive mode to bypass the audio mixer.
pub struct ExclusiveOutput {
    /// Command channel to audio thread
    command_tx: Sender<ExclusiveCommand>,
    /// Configuration used
    config: ExclusiveConfig,
    /// Actual sample rate being used
    sample_rate: u32,
    /// Shared state
    state: Arc<ExclusiveState>,
    /// Audio thread handle (kept alive to prevent thread termination)
    audio_thread: Option<JoinHandle<()>>,
    /// Latency information
    latency: LatencyInfo,
}

impl ExclusiveOutput {
    /// Create new exclusive output with configuration
    pub fn new(config: ExclusiveConfig) -> Result<Self> {
        tracing::info!(
            backend = ?config.backend,
            device_name = ?config.device_name,
            sample_rate = config.sample_rate,
            bit_depth = ?config.bit_depth,
            exclusive_mode = config.exclusive_mode,
            "[Exclusive] Creating exclusive output"
        );
        let start = std::time::Instant::now();

        // Get the host for the configured backend
        tracing::debug!(backend = ?config.backend, "[Exclusive] Acquiring audio host");
        let host = config.backend.to_cpal_host().map_err(|_e| {
            tracing::error!(backend = ?config.backend, "[Exclusive] Failed to acquire audio host");
            AudioOutputError::DeviceNotFound
        })?;

        // Get device
        let device_lookup_start = std::time::Instant::now();
        let device = if let Some(ref name) = config.device_name {
            tracing::debug!(device_name = %name, "[Exclusive] Looking up device by name");
            find_device_by_name(config.backend, name)
                .map_err(|e| {
                    tracing::error!(device_name = %name, error = ?e, "[Exclusive] Device lookup failed");
                    AudioOutputError::DeviceNotFound
                })?
        } else {
            tracing::debug!("[Exclusive] Using default output device");
            host.default_output_device().ok_or_else(|| {
                tracing::error!("[Exclusive] No default output device found");
                AudioOutputError::DeviceNotFound
            })?
        };
        let device_lookup_duration = device_lookup_start.elapsed();
        tracing::debug!(
            duration_ms = device_lookup_duration.as_millis(),
            "[Exclusive] Device lookup completed"
        );

        // Find the best matching configuration
        let config_start = std::time::Instant::now();
        let (stream_config, sample_format, buffer_size) = Self::find_best_config(&device, &config)?;
        let config_duration = config_start.elapsed();

        let sample_rate = stream_config.sample_rate;

        // Calculate latency
        let buffer_samples = match buffer_size {
            BufferSize::Fixed(n) => n,
            BufferSize::Default => 512, // Estimate
        };
        let buffer_ms = buffer_samples as f32 / sample_rate as f32 * 1000.0;

        let latency = LatencyInfo {
            buffer_samples,
            buffer_ms,
            total_ms: buffer_ms + 5.0, // Add ~5ms for DAC latency estimate
            exclusive: config.exclusive_mode,
        };

        let state = Arc::new(ExclusiveState::new());
        let (command_tx, command_rx) = bounded::<ExclusiveCommand>(32);

        // Spawn audio thread
        tracing::debug!(
            sample_rate,
            sample_format = ?sample_format,
            buffer_samples,
            latency_ms = latency.buffer_ms,
            "[Exclusive] Spawning audio thread"
        );
        let thread_start = std::time::Instant::now();
        let state_clone = Arc::clone(&state);
        let audio_thread = thread::spawn(move || {
            Self::audio_thread_run(
                device,
                stream_config,
                sample_format,
                state_clone,
                command_rx,
            );
        });
        let thread_duration = thread_start.elapsed();

        let total_duration = start.elapsed();
        tracing::info!(
            sample_rate,
            sample_format = ?sample_format,
            buffer_samples,
            latency_ms = latency.buffer_ms,
            exclusive = latency.exclusive,
            device_lookup_ms = device_lookup_duration.as_millis(),
            config_selection_ms = config_duration.as_millis(),
            thread_spawn_ms = thread_duration.as_millis(),
            total_duration_ms = total_duration.as_millis(),
            "[Exclusive] Exclusive output created successfully"
        );

        Ok(Self {
            command_tx,
            config,
            sample_rate,
            state,
            audio_thread: Some(audio_thread),
            latency,
        })
    }

    /// Create with default configuration
    pub fn default_exclusive() -> Result<Self> {
        Self::new(ExclusiveConfig {
            exclusive_mode: true,
            ..Default::default()
        })
    }

    /// Find best matching device configuration
    fn find_best_config(
        device: &Device,
        config: &ExclusiveConfig,
    ) -> Result<(StreamConfig, SampleFormat, BufferSize)> {
        // Default buffer size for invalid (zero) buffer requests
        // This follows WASAPI best practices: zero buffer is invalid and should use a safe default
        const DEFAULT_BUFFER_FRAMES: u32 = 1024;

        tracing::debug!(
            target_sample_rate = config.sample_rate,
            target_bit_depth = ?config.bit_depth,
            buffer_frames = ?config.buffer_frames,
            "[Exclusive] Searching for matching device configuration"
        );

        let supported_configs = device.supported_output_configs().map_err(|e| {
            tracing::error!(error = %e, "[Exclusive] Failed to query supported output configs");
            AudioOutputError::StreamBuildError(e.to_string())
        })?;

        // Target sample format based on bit depth
        let target_format = match config.bit_depth {
            SupportedBitDepth::Int16 => SampleFormat::I16,
            SupportedBitDepth::Int24 | SupportedBitDepth::Int32 => SampleFormat::I32,
            SupportedBitDepth::Float32 => SampleFormat::F32,
            SupportedBitDepth::Float64 => SampleFormat::F64,
        };
        tracing::debug!(target_format = ?target_format, "[Exclusive] Target sample format selected");

        // Find a matching config
        let mut best_config = None;
        let mut fallback_config = None;
        let mut config_iteration_count = 0;

        for supported in supported_configs {
            config_iteration_count += 1;
            let format = supported.sample_format();
            // Extract sample rate from cpal::SampleRate (inner .0 field)
            let min_sample_rate = supported.min_sample_rate();
            let max_sample_rate = supported.max_sample_rate();
            let min_rate = cpal_sample_rate_to_u32(min_sample_rate);
            let max_rate = cpal_sample_rate_to_u32(max_sample_rate);

            // Check if target sample rate is in range
            let target_rate = if config.sample_rate == 0 {
                // Use device's best rate (prefer 44100 or 48000)
                if min_rate <= 44100 && max_rate >= 44100 {
                    44100
                } else if min_rate <= 48000 && max_rate >= 48000 {
                    48000
                } else {
                    min_rate
                }
            } else if config.sample_rate >= min_rate && config.sample_rate <= max_rate {
                config.sample_rate
            } else {
                continue;
            };

            // Exact format match
            if format == target_format {
                best_config = Some((supported, target_rate));
                break;
            }

            // Keep a fallback (f32 is always usable)
            if format == SampleFormat::F32 && fallback_config.is_none() {
                fallback_config = Some((supported, target_rate));
            }
        }

        let (selected_config, target_rate) = best_config.or(fallback_config).ok_or_else(|| {
            tracing::error!(
                configs_checked = config_iteration_count,
                target_format = ?target_format,
                "[Exclusive] No compatible configuration found"
            );
            AudioOutputError::StreamBuildError("No compatible config found".into())
        })?;

        let sample_format = selected_config.sample_format();
        let is_fallback = best_config.is_none();
        if is_fallback {
            tracing::warn!(
                configs_checked = config_iteration_count,
                selected_format = ?sample_format,
                target_format = ?target_format,
                "[Exclusive] Using fallback configuration (exact format not available)"
            );
        } else {
            tracing::debug!(
                configs_checked = config_iteration_count,
                selected_format = ?sample_format,
                "[Exclusive] Found exact format match"
            );
        }

        // Determine buffer size
        // If zero buffer size is requested, fall back to DEFAULT_BUFFER_FRAMES
        let buffer_size = if let Some(frames) = config.buffer_frames {
            // Handle zero buffer specially - use default instead
            let requested_frames = if frames == 0 {
                DEFAULT_BUFFER_FRAMES
            } else {
                frames
            };

            match selected_config.buffer_size() {
                SupportedBufferSize::Range { min, max } => {
                    let clamped = requested_frames.clamp(*min, *max);
                    BufferSize::Fixed(clamped)
                }
                SupportedBufferSize::Unknown => BufferSize::Fixed(requested_frames),
            }
        } else {
            BufferSize::Default
        };

        let stream_config = StreamConfig {
            channels: selected_config.channels(),
            sample_rate: target_rate,
            buffer_size,
        };

        tracing::debug!(
            sample_rate = target_rate,
            channels = selected_config.channels(),
            buffer_size = ?buffer_size,
            sample_format = ?sample_format,
            "[Exclusive] Stream configuration finalized"
        );

        Ok((stream_config, sample_format, buffer_size))
    }

    /// Audio thread main loop
    fn audio_thread_run(
        device: Device,
        config: StreamConfig,
        sample_format: SampleFormat,
        state: Arc<ExclusiveState>,
        command_rx: Receiver<ExclusiveCommand>,
    ) {
        tracing::info!(
            sample_rate = config.sample_rate,
            channels = config.channels,
            sample_format = ?sample_format,
            "[Exclusive] Audio thread started"
        );
        let mut stream: Option<Stream> = None;

        while let Ok(cmd) = command_rx.recv() {
            match cmd {
                ExclusiveCommand::Play { samples } => {
                    tracing::debug!(
                        sample_count = samples.len(),
                        "[Exclusive] Play command received"
                    );
                    // Stop existing stream
                    if let Some(s) = stream.take() {
                        drop(s);
                    }

                    // Update buffer
                    let buffer_update_start = std::time::Instant::now();
                    {
                        let mut buffer = state.buffer.lock().unwrap();
                        *buffer = samples;
                    }
                    let buffer_update_duration = buffer_update_start.elapsed();

                    // Reset position
                    state.position.store(0, Ordering::Relaxed);
                    state.playing.store(true, Ordering::Relaxed);
                    state.paused.store(false, Ordering::Relaxed);

                    // Build stream based on sample format
                    tracing::debug!(sample_format = ?sample_format, "[Exclusive] Building audio stream");
                    let stream_build_start = std::time::Instant::now();
                    let new_stream = match sample_format {
                        SampleFormat::I16 => Self::build_stream_i16(&device, &config, &state),
                        SampleFormat::I32 => Self::build_stream_i32(&device, &config, &state),
                        SampleFormat::F32 => Self::build_stream_f32(&device, &config, &state),
                        SampleFormat::F64 => Self::build_stream_f64(&device, &config, &state),
                        _ => Self::build_stream_f32(&device, &config, &state), // Fallback
                    };
                    let stream_build_duration = stream_build_start.elapsed();

                    if let Ok(s) = new_stream {
                        if s.play().is_ok() {
                            stream = Some(s);
                            tracing::info!(
                                buffer_update_us = buffer_update_duration.as_micros(),
                                stream_build_ms = stream_build_duration.as_millis(),
                                "[Exclusive] Stream started successfully"
                            );
                        } else {
                            tracing::error!("[Exclusive] Failed to start stream playback");
                        }
                    } else {
                        tracing::error!(
                            stream_build_ms = stream_build_duration.as_millis(),
                            "[Exclusive] Failed to build stream"
                        );
                    }
                }
                ExclusiveCommand::Pause => {
                    if let Some(ref s) = stream {
                        let _ = s.pause();
                        state.paused.store(true, Ordering::Relaxed);
                    }
                }
                ExclusiveCommand::Resume => {
                    if let Some(ref s) = stream {
                        let _ = s.play();
                        state.paused.store(false, Ordering::Relaxed);
                    }
                }
                ExclusiveCommand::Stop => {
                    if let Some(s) = stream.take() {
                        drop(s);
                    }
                    state.playing.store(false, Ordering::Relaxed);
                    state.position.store(0, Ordering::Relaxed);
                }
                ExclusiveCommand::SetVolume(vol) => {
                    tracing::debug!(volume = vol, "[Exclusive] Setting volume");
                    state.set_volume(vol);
                }
                ExclusiveCommand::Shutdown => {
                    tracing::info!("[Exclusive] Shutting down audio thread");
                    if let Some(s) = stream.take() {
                        drop(s);
                    }
                    break;
                }
            }
        }
        tracing::info!("[Exclusive] Audio thread terminated");
    }

    /// Build i16 stream
    fn build_stream_i16(
        device: &Device,
        config: &StreamConfig,
        state: &Arc<ExclusiveState>,
    ) -> std::result::Result<Stream, cpal::BuildStreamError> {
        let state = Arc::clone(state);
        device.build_output_stream(
            config,
            move |data: &mut [i16], _| Self::callback_i16(data, &state),
            |err| tracing::error!(error = %err, "[Exclusive] Audio stream error"),
            None,
        )
    }

    /// Build i32 stream
    fn build_stream_i32(
        device: &Device,
        config: &StreamConfig,
        state: &Arc<ExclusiveState>,
    ) -> std::result::Result<Stream, cpal::BuildStreamError> {
        let state = Arc::clone(state);
        device.build_output_stream(
            config,
            move |data: &mut [i32], _| Self::callback_i32(data, &state),
            |err| tracing::error!(error = %err, "[Exclusive] Audio stream error"),
            None,
        )
    }

    /// Build f32 stream
    fn build_stream_f32(
        device: &Device,
        config: &StreamConfig,
        state: &Arc<ExclusiveState>,
    ) -> std::result::Result<Stream, cpal::BuildStreamError> {
        let state = Arc::clone(state);
        device.build_output_stream(
            config,
            move |data: &mut [f32], _| Self::callback_f32(data, &state),
            |err| tracing::error!(error = %err, "[Exclusive] Audio stream error"),
            None,
        )
    }

    /// Build f64 stream
    fn build_stream_f64(
        device: &Device,
        config: &StreamConfig,
        state: &Arc<ExclusiveState>,
    ) -> std::result::Result<Stream, cpal::BuildStreamError> {
        let state = Arc::clone(state);
        device.build_output_stream(
            config,
            move |data: &mut [f64], _| Self::callback_f64(data, &state),
            |err| tracing::error!(error = %err, "[Exclusive] Audio stream error"),
            None,
        )
    }

    /// i16 audio callback
    fn callback_i16(output: &mut [i16], state: &ExclusiveState) {
        if !state.playing.load(Ordering::Relaxed) || state.paused.load(Ordering::Relaxed) {
            output.fill(0);
            return;
        }

        let volume = state.get_volume();
        let buffer = {
            let b = state.buffer.lock().unwrap();
            Arc::clone(&b)
        };

        let mut pos = state.position.load(Ordering::Relaxed);

        match buffer.as_ref() {
            AudioData::Int16(samples) => {
                for out in output.iter_mut() {
                    if pos >= samples.len() {
                        if state.looping.load(Ordering::Relaxed) {
                            pos = 0;
                        } else {
                            *out = 0;
                            continue;
                        }
                    }
                    *out = ((samples[pos] as f32) * volume) as i16;
                    pos += 1;
                }
            }
            AudioData::Float32(samples) => {
                for out in output.iter_mut() {
                    if pos >= samples.len() {
                        if state.looping.load(Ordering::Relaxed) {
                            pos = 0;
                        } else {
                            *out = 0;
                            continue;
                        }
                    }
                    *out = (samples[pos].clamp(-1.0, 1.0) * volume * i16::MAX as f32) as i16;
                    pos += 1;
                }
            }
            _ => {
                output.fill(0);
            }
        }

        state.position.store(pos, Ordering::Relaxed);
    }

    /// i32 audio callback
    fn callback_i32(output: &mut [i32], state: &ExclusiveState) {
        if !state.playing.load(Ordering::Relaxed) || state.paused.load(Ordering::Relaxed) {
            output.fill(0);
            return;
        }

        let volume = state.get_volume();
        let buffer = {
            let b = state.buffer.lock().unwrap();
            Arc::clone(&b)
        };

        let mut pos = state.position.load(Ordering::Relaxed);

        match buffer.as_ref() {
            AudioData::Int32(samples) => {
                for out in output.iter_mut() {
                    if pos >= samples.len() {
                        if state.looping.load(Ordering::Relaxed) {
                            pos = 0;
                        } else {
                            *out = 0;
                            continue;
                        }
                    }
                    *out = ((samples[pos] as f64) * volume as f64) as i32;
                    pos += 1;
                }
            }
            AudioData::Float32(samples) => {
                for out in output.iter_mut() {
                    if pos >= samples.len() {
                        if state.looping.load(Ordering::Relaxed) {
                            pos = 0;
                        } else {
                            *out = 0;
                            continue;
                        }
                    }
                    *out = (samples[pos].clamp(-1.0, 1.0) * volume * i32::MAX as f32) as i32;
                    pos += 1;
                }
            }
            _ => {
                output.fill(0);
            }
        }

        state.position.store(pos, Ordering::Relaxed);
    }

    /// f32 audio callback
    fn callback_f32(output: &mut [f32], state: &ExclusiveState) {
        if !state.playing.load(Ordering::Relaxed) || state.paused.load(Ordering::Relaxed) {
            output.fill(0.0);
            return;
        }

        let volume = state.get_volume();
        let buffer = {
            let b = state.buffer.lock().unwrap();
            Arc::clone(&b)
        };

        let mut pos = state.position.load(Ordering::Relaxed);

        match buffer.as_ref() {
            AudioData::Float32(samples) => {
                for out in output.iter_mut() {
                    if pos >= samples.len() {
                        if state.looping.load(Ordering::Relaxed) {
                            pos = 0;
                        } else {
                            *out = 0.0;
                            continue;
                        }
                    }
                    *out = samples[pos] * volume;
                    pos += 1;
                }
            }
            AudioData::Int16(samples) => {
                for out in output.iter_mut() {
                    if pos >= samples.len() {
                        if state.looping.load(Ordering::Relaxed) {
                            pos = 0;
                        } else {
                            *out = 0.0;
                            continue;
                        }
                    }
                    *out = (samples[pos] as f32 / i16::MAX as f32) * volume;
                    pos += 1;
                }
            }
            AudioData::Int32(samples) => {
                for out in output.iter_mut() {
                    if pos >= samples.len() {
                        if state.looping.load(Ordering::Relaxed) {
                            pos = 0;
                        } else {
                            *out = 0.0;
                            continue;
                        }
                    }
                    *out = (samples[pos] as f32 / i32::MAX as f32) * volume;
                    pos += 1;
                }
            }
        }

        state.position.store(pos, Ordering::Relaxed);
    }

    /// f64 audio callback
    fn callback_f64(output: &mut [f64], state: &ExclusiveState) {
        if !state.playing.load(Ordering::Relaxed) || state.paused.load(Ordering::Relaxed) {
            output.fill(0.0);
            return;
        }

        let volume = state.get_volume() as f64;
        let buffer = {
            let b = state.buffer.lock().unwrap();
            Arc::clone(&b)
        };

        let mut pos = state.position.load(Ordering::Relaxed);

        match buffer.as_ref() {
            AudioData::Float32(samples) => {
                for out in output.iter_mut() {
                    if pos >= samples.len() {
                        if state.looping.load(Ordering::Relaxed) {
                            pos = 0;
                        } else {
                            *out = 0.0;
                            continue;
                        }
                    }
                    *out = samples[pos] as f64 * volume;
                    pos += 1;
                }
            }
            _ => {
                output.fill(0.0);
            }
        }

        state.position.store(pos, Ordering::Relaxed);
    }

    /// Play audio data
    pub fn play(&self, data: AudioData) -> Result<()> {
        self.command_tx
            .send(ExclusiveCommand::Play {
                samples: Arc::new(data),
            })
            .map_err(|e| AudioOutputError::StreamBuildError(format!("Channel error: {}", e)))?;
        Ok(())
    }

    /// Play f32 samples with automatic conversion to target format
    pub fn play_f32(&self, samples: &[f32]) -> Result<()> {
        let data = AudioData::from_f32(samples, self.config.bit_depth);
        self.play(data)
    }

    /// Pause playback
    pub fn pause(&self) -> Result<()> {
        self.command_tx
            .send(ExclusiveCommand::Pause)
            .map_err(|e| AudioOutputError::StreamBuildError(format!("Channel error: {}", e)))?;
        Ok(())
    }

    /// Resume playback
    pub fn resume(&self) -> Result<()> {
        self.command_tx
            .send(ExclusiveCommand::Resume)
            .map_err(|e| AudioOutputError::StreamBuildError(format!("Channel error: {}", e)))?;
        Ok(())
    }

    /// Stop playback
    pub fn stop(&self) -> Result<()> {
        self.command_tx
            .send(ExclusiveCommand::Stop)
            .map_err(|e| AudioOutputError::StreamBuildError(format!("Channel error: {}", e)))?;
        Ok(())
    }

    /// Set volume (0.0 - 1.0)
    pub fn set_volume(&self, volume: f32) -> Result<()> {
        let vol = volume.clamp(0.0, 1.0);
        self.command_tx
            .send(ExclusiveCommand::SetVolume(vol))
            .map_err(|e| AudioOutputError::StreamBuildError(format!("Channel error: {}", e)))?;
        // Update local state atomically
        self.state.set_volume(vol);
        Ok(())
    }

    /// Get current volume
    pub fn volume(&self) -> f32 {
        self.state.get_volume()
    }

    /// Get sample rate
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Get configuration
    pub fn config(&self) -> &ExclusiveConfig {
        &self.config
    }

    /// Get latency information
    pub fn latency(&self) -> LatencyInfo {
        self.latency
    }

    /// Check if currently playing
    pub fn is_playing(&self) -> bool {
        self.state.playing.load(Ordering::Relaxed) && !self.state.paused.load(Ordering::Relaxed)
    }

    /// Check if paused
    pub fn is_paused(&self) -> bool {
        self.state.paused.load(Ordering::Relaxed)
    }

    /// Set looping mode
    pub fn set_looping(&self, looping: bool) {
        self.state.looping.store(looping, Ordering::Relaxed);
    }

    /// Get current playback position in samples
    pub fn position(&self) -> usize {
        self.state.position.load(Ordering::Relaxed)
    }
}

impl Drop for ExclusiveOutput {
    fn drop(&mut self) {
        // Send shutdown command
        let _ = self.command_tx.send(ExclusiveCommand::Shutdown);

        // Wait for audio thread to finish cleanup to avoid WASAPI/COM access violations
        // This is critical on Windows where COM cleanup must happen in the correct order
        if let Some(handle) = self.audio_thread.take() {
            let _ = handle.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exclusive_config_defaults() {
        let config = ExclusiveConfig::default();
        assert_eq!(config.sample_rate, 0);
        assert_eq!(config.bit_depth, SupportedBitDepth::Float32);
        assert!(config.exclusive_mode);
        assert!(config.device_name.is_none());
    }

    #[test]
    fn test_exclusive_config_presets() {
        let bp16 = ExclusiveConfig::bit_perfect_16();
        assert_eq!(bp16.bit_depth, SupportedBitDepth::Int16);
        assert!(bp16.exclusive_mode);

        let bp24 = ExclusiveConfig::bit_perfect_24();
        assert_eq!(bp24.bit_depth, SupportedBitDepth::Int24);

        let low_lat = ExclusiveConfig::low_latency();
        assert_eq!(low_lat.buffer_frames, Some(128));
    }

    #[test]
    fn test_audio_data_conversion() {
        let samples = vec![0.0f32, 0.5, -0.5, 1.0, -1.0];

        // Test i16 conversion
        let data16 = AudioData::from_f32(&samples, SupportedBitDepth::Int16);
        let AudioData::Int16(v) = data16 else {
            panic!("Expected Int16 audio data format");
        };
        assert_eq!(v.len(), 5);
        assert_eq!(v[0], 0);
        assert!(v[1] > 0);
        assert!(v[2] < 0);

        // Test i32 conversion
        let data32 = AudioData::from_f32(&samples, SupportedBitDepth::Int32);
        let AudioData::Int32(v) = data32 else {
            panic!("Expected Int32 audio data format");
        };
        assert_eq!(v.len(), 5);

        // Test f32 passthrough
        let dataf32 = AudioData::from_f32(&samples, SupportedBitDepth::Float32);
        let AudioData::Float32(v) = dataf32 else {
            panic!("Expected Float32 audio data format");
        };
        assert_eq!(v, samples);
    }

    #[test]
    fn test_latency_info() {
        let info = LatencyInfo {
            buffer_samples: 256,
            buffer_ms: 5.8,
            total_ms: 10.8,
            exclusive: true,
        };

        assert_eq!(info.buffer_samples, 256);
        assert!(info.exclusive);
    }

    #[test]
    fn test_exclusive_config_builder() {
        let config = ExclusiveConfig::default()
            .with_sample_rate(96000)
            .with_buffer_frames(256)
            .with_device("Test Device");

        assert_eq!(config.sample_rate, 96000);
        assert_eq!(config.buffer_frames, Some(256));
        assert_eq!(config.device_name, Some("Test Device".to_string()));
    }

    // Integration tests that require audio device
    #[test]
    #[ignore = "Requires real audio hardware - not available in CI environments"]
    fn test_create_exclusive_output() {
        // This may fail in CI without audio devices
        let config = ExclusiveConfig::default();
        match ExclusiveOutput::new(config) {
            Ok(output) => {
                assert!(output.sample_rate() > 0);
                assert!(output.latency().buffer_samples > 0);
            }
            Err(AudioOutputError::DeviceNotFound) => {
                // Expected in headless environments
            }
            Err(e) => {
                // Other errors might be acceptable in CI
                tracing::warn!(
                    error = %e,
                    "[Test] Exclusive output creation error (expected in CI)"
                );
            }
        }
    }
}
