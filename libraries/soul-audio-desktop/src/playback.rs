//! Desktop playback integration
//!
//! Combines `PlaybackManager` with CPAL audio output for desktop playback.

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Device, Stream, StreamConfig,
};
use crossbeam_channel::{bounded, Receiver, Sender};
use soul_playback::{PlaybackConfig, PlaybackManager, QueueTrack};
use std::sync::{Arc, Mutex};

use crate::error::Result;
use std::sync::atomic::{AtomicU64, Ordering};

/// Global counter for I32 (ASIO) callbacks - used for diagnostics
/// This is updated by audio_callback_i32 and read by send_command for debugging
static GLOBAL_I32_CALLBACK_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Drop guard for detecting when callback closures are dropped
/// This helps diagnose ASIO stream issues where the callback is silently dropped
struct CallbackDropGuard {
    stream_id: std::time::Instant,
    sample_format: &'static str,
}

impl Drop for CallbackDropGuard {
    fn drop(&mut self) {
        eprintln!("[CallbackDropGuard] !!! {} stream {:?} callback closure is being DROPPED !!!",
            self.sample_format, self.stream_id);
        eprintln!("[CallbackDropGuard] This means the ASIO/audio callback will no longer be called.");
        eprintln!("[CallbackDropGuard] The command_rx receiver will be dropped, causing channel disconnect.");
    }
}

/// Commands sent to playback thread
#[derive(Debug, Clone)]
pub enum PlaybackCommand {
    /// Start or resume playback
    Play,

    /// Pause playback
    Pause,

    /// Stop playback
    Stop,

    /// Skip to next track
    Next,

    /// Go to previous track
    Previous,

    /// Seek to position (in seconds)
    Seek(f64),

    /// Set volume (0-100)
    SetVolume(u8),

    /// Mute audio
    Mute,

    /// Unmute audio
    Unmute,

    /// Add track to queue
    AddToQueue(QueueTrack),

    /// Remove track from queue
    RemoveFromQueue(usize),

    /// Clear queue
    ClearQueue,

    /// Skip to track at queue index
    SkipToQueueIndex(usize),

    /// Load playlist/album as new source queue (replaces playback context)
    LoadPlaylist(Vec<QueueTrack>),

    /// Set shuffle mode
    SetShuffle(soul_playback::ShuffleMode),

    /// Set repeat mode
    SetRepeat(soul_playback::RepeatMode),

    /// Switch audio output device
    /// Arguments: (backend, device_name)
    SwitchDevice(crate::AudioBackend, String),
}

/// Playback events emitted by playback thread
#[derive(Debug, Clone)]
pub enum PlaybackEvent {
    /// Playback state changed
    StateChanged(soul_playback::PlaybackState),

    /// Track changed
    TrackChanged(Option<QueueTrack>),

    /// Position updated (in seconds)
    PositionUpdated(f64),

    /// Volume changed
    VolumeChanged(u8),

    /// Queue updated
    QueueUpdated,

    /// Device sample rate changed (old_rate, new_rate)
    SampleRateChanged(u32, u32),

    /// Error occurred
    Error(String),
}

/// Desktop playback integration
///
/// Manages `PlaybackManager` + CPAL audio output + event handling
pub struct DesktopPlayback {
    /// Command sender
    command_tx: Sender<PlaybackCommand>,

    /// Event receiver
    event_rx: Receiver<PlaybackEvent>,

    /// Event sender (for creating new streams)
    event_tx: Sender<PlaybackEvent>,

    /// CPAL audio stream
    stream: Arc<Mutex<Option<Stream>>>,

    /// Playback manager (shared with audio thread)
    manager: Arc<Mutex<PlaybackManager>>,

    /// Current audio backend
    current_backend: Arc<Mutex<crate::AudioBackend>>,

    /// Current device name
    current_device: Arc<Mutex<String>>,

    /// Current stream sample rate (what we're actually outputting at)
    current_sample_rate: Arc<std::sync::atomic::AtomicU32>,
}

// SAFETY: DesktopPlayback is safe to send between threads because:
// - command_tx and event_rx are both Send
// - manager is Arc<Mutex<>>, which is Send + Sync
// - _stream is CPAL's Stream, which internally uses thread-safe primitives
//   (the PhantomData<*mut ()> is just a marker, not actually unsafe)
#[allow(unsafe_code)]
unsafe impl Send for DesktopPlayback {}

#[allow(unsafe_code)]
unsafe impl Sync for DesktopPlayback {}

impl DesktopPlayback {
    /// Create new desktop playback system
    ///
    /// # Arguments
    /// * `config` - Playback configuration
    ///
    /// # Returns
    /// * `Ok(playback)` - Desktop playback ready
    /// * `Err(_)` - Failed to initialize audio output
    pub fn new(config: PlaybackConfig) -> Result<Self> {
        Self::new_with_device(config, crate::AudioBackend::Default, None)
    }

    /// Create new desktop playback system with specific device
    ///
    /// # Arguments
    /// * `config` - Playback configuration
    /// * `backend` - Audio backend to use
    /// * `device_name` - Optional device name (uses default if None)
    ///
    /// # Returns
    /// * `Ok(playback)` - Desktop playback ready
    /// * `Err(_)` - Failed to initialize audio output
    pub fn new_with_device(
        config: PlaybackConfig,
        backend: crate::AudioBackend,
        device_name: Option<String>,
    ) -> Result<Self> {
        let manager = Arc::new(Mutex::new(PlaybackManager::new(config)));

        let (command_tx, command_rx) = bounded(32);
        let (event_tx, event_rx) = bounded(32);

        // Create CPAL stream with specified device
        let (stream, actual_device_name, sample_rate) = Self::create_audio_stream(
            manager.clone(),
            command_rx.clone(),
            event_tx.clone(),
            backend,
            device_name,
        )?;

        let stream = Arc::new(Mutex::new(Some(stream)));
        let current_backend = Arc::new(Mutex::new(backend));
        let current_device = Arc::new(Mutex::new(actual_device_name));
        let current_sample_rate = Arc::new(std::sync::atomic::AtomicU32::new(sample_rate));

        Ok(Self {
            command_tx,
            event_rx,
            event_tx,
            stream,
            manager,
            current_backend,
            current_device,
            current_sample_rate,
        })
    }

    /// Create CPAL audio stream
    ///
    /// Returns (Stream, device_name, sample_rate)
    fn create_audio_stream(
        manager: Arc<Mutex<PlaybackManager>>,
        command_rx: Receiver<PlaybackCommand>,
        event_tx: Sender<PlaybackEvent>,
        backend: crate::AudioBackend,
        device_name: Option<String>,
    ) -> Result<(Stream, String, u32)> {
        let host = backend
            .to_cpal_host()
            .map_err(|_| crate::error::AudioError::DeviceNotFound)?;

        let device = if let Some(name) = device_name {
            // Find device by name
            crate::device::find_device_by_name(backend, &name)
                .map_err(|e| crate::error::AudioError::DeviceError(e.to_string()))?
        } else {
            // Use default device
            host.default_output_device()
                .ok_or(crate::error::AudioError::DeviceNotFound)?
        };

        let actual_device_name = device
            .name()
            .unwrap_or_else(|_| "Unknown Device".to_string());

        let (config, sample_format) = Self::get_stream_config(&device)?;
        let sample_rate = config.sample_rate;
        let channels = config.channels;

        // Set sample rate and channel count in manager
        {
            let mut mgr = manager.lock().unwrap();
            mgr.set_sample_rate(sample_rate);
            mgr.set_output_channels(channels);
        }

        eprintln!("[CPAL] Building output stream with config: sample_rate={}, channels={}, buffer_size={:?}, format={:?}",
            config.sample_rate, config.channels, config.buffer_size, sample_format);

        // Build stream with the appropriate sample format
        let stream = match sample_format {
            cpal::SampleFormat::F32 => {
                let manager_clone = manager.clone();
                // Per-stream callback counter for logging
                let mut callback_count: u32 = 0;
                let stream_id = std::time::Instant::now();
                eprintln!("[CPAL] Creating F32 stream callback (stream_id: {:?})", stream_id);
                device.build_output_stream(
                    &config,
                    move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                        callback_count += 1;
                        Self::audio_callback_f32(data, manager_clone.clone(), &command_rx, &event_tx, callback_count, stream_id);
                    },
                    |err| eprintln!("[CPAL] Audio stream error callback: {}", err),
                    None,
                )?
            }
            cpal::SampleFormat::I32 => {
                let manager_clone = manager.clone();
                // Pre-allocate conversion buffer to avoid allocation in audio callback
                // Use a reasonable default size that will be resized if needed
                let mut f32_buffer: Vec<f32> = Vec::with_capacity(4096);
                // Per-stream callback counter for logging
                let mut callback_count: u32 = 0;
                let stream_id = std::time::Instant::now();
                eprintln!("[CPAL] Creating I32 stream callback (stream_id: {:?})", stream_id);

                // Clone event_tx for the error callback
                let error_event_tx = event_tx.clone();

                // Create drop guard to detect when callback is dropped
                let drop_guard = CallbackDropGuard {
                    stream_id,
                    sample_format: "I32",
                };

                device.build_output_stream(
                    &config,
                    move |data: &mut [i32], _: &cpal::OutputCallbackInfo| {
                        // Keep drop guard alive - when this closure is dropped, drop_guard is dropped
                        let _ = &drop_guard;
                        callback_count += 1;
                        Self::audio_callback_i32(data, manager_clone.clone(), &command_rx, &event_tx, &mut f32_buffer, callback_count, stream_id);
                    },
                    move |err| {
                        eprintln!("[CPAL] !!! AUDIO STREAM ERROR CALLBACK !!!");
                        eprintln!("[CPAL]   Error: {}", err);
                        eprintln!("[CPAL]   This may cause the stream to be dropped!");
                        let _ = error_event_tx.try_send(PlaybackEvent::Error(format!("Stream error: {}", err)));
                    },
                    None,
                )?
            }
            cpal::SampleFormat::I16 => {
                let manager_clone = manager.clone();
                // Pre-allocate conversion buffer to avoid allocation in audio callback
                let mut f32_buffer: Vec<f32> = Vec::with_capacity(4096);
                // Per-stream callback counter for logging
                let mut callback_count: u32 = 0;
                let stream_id = std::time::Instant::now();
                eprintln!("[CPAL] Creating I16 stream callback (stream_id: {:?})", stream_id);
                device.build_output_stream(
                    &config,
                    move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                        callback_count += 1;
                        Self::audio_callback_i16(data, manager_clone.clone(), &command_rx, &event_tx, &mut f32_buffer, callback_count, stream_id);
                    },
                    |err| eprintln!("[CPAL] Audio stream error callback: {}", err),
                    None,
                )?
            }
            _ => {
                eprintln!("[CPAL] ERROR: Unsupported sample format: {:?}", sample_format);
                return Err(crate::error::AudioError::DeviceError(
                    format!("Unsupported sample format: {:?}", sample_format)
                ).into());
            }
        };

        eprintln!("[CPAL] Stream built successfully, calling play()...");

        match stream.play() {
            Ok(()) => {
                eprintln!("[CPAL] stream.play() returned Ok - stream should now be running");
            }
            Err(e) => {
                eprintln!("[CPAL] ERROR: Failed to start stream: {}", e);
                eprintln!("[CPAL] This may indicate:");
                eprintln!("  - Sample rate mismatch with driver settings");
                eprintln!("  - Buffer size not supported by driver");
                eprintln!("  - Another application has exclusive access to the device");
                eprintln!("  - Driver requires specific initialization");
                return Err(e.into());
            }
        }

        eprintln!("[CPAL] ==========================================");
        eprintln!("[CPAL] Stream created successfully!");
        eprintln!("[CPAL]   Device: {}", actual_device_name);
        eprintln!("[CPAL]   Sample rate: {} Hz", sample_rate);
        eprintln!("[CPAL]   Channels: {}", channels);
        eprintln!("[CPAL]   Sample format: {:?}", sample_format);
        eprintln!("[CPAL]   Buffer size: {:?}", config.buffer_size);
        eprintln!("[CPAL] ==========================================");
        eprintln!("[CPAL] Audio callbacks should start momentarily...");

        Ok((stream, actual_device_name, sample_rate))
    }

    /// Get stream configuration
    /// Returns (StreamConfig, SampleFormat)
    ///
    /// IMPORTANT: Always uses the device's ACTUAL configured sample rate from
    /// `default_output_config()`. We don't try to request a different rate because:
    /// - ASIO: Sample rate is fixed by the driver control panel
    /// - WASAPI Shared: Sample rate is fixed by Windows sound settings
    /// - WASAPI Exclusive: Can change rate, but default_output_config gives us the current one
    ///
    /// If we request a different rate than what the device is actually running at,
    /// the audio will play at the wrong speed (e.g., requesting 96kHz when device
    /// is at 48kHz will play audio at 2x speed).
    fn get_stream_config(device: &Device) -> Result<(StreamConfig, cpal::SampleFormat)> {
        // Get the device's ACTUAL current configuration
        // This is the sample rate the device is really running at
        let default_config = device.default_output_config()?;
        let actual_sample_rate = default_config.sample_rate();

        eprintln!("[CPAL] Device's actual sample rate: {:?}", actual_sample_rate);
        eprintln!("[CPAL] Device's default config: channels={}, format={:?}",
            default_config.channels(), default_config.sample_format());

        // Also log supported configs for debugging
        eprintln!("[CPAL] Checking supported output configurations...");
        let supported_configs: Vec<_> = device
            .supported_output_configs()
            .map(|configs| configs.collect())
            .unwrap_or_default();

        for cfg in &supported_configs {
            eprintln!("[CPAL]   Supported: channels={}, sample_rate={:?}-{:?}, format={:?}",
                cfg.channels(),
                cfg.min_sample_rate(),
                cfg.max_sample_rate(),
                cfg.sample_format());
        }

        // Find a config that matches the device's actual sample rate
        // Prefer stereo, then prefer f32 > i32 > i16
        let matching_config = supported_configs.iter()
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
                supported_configs.iter()
                    .filter(|c| c.min_sample_rate() <= actual_sample_rate && c.max_sample_rate() >= actual_sample_rate)
                    .next()
            });

        let config = if let Some(cfg) = matching_config {
            // Use the config with the device's ACTUAL sample rate
            cfg.clone().with_sample_rate(actual_sample_rate)
        } else {
            // Fall back to default config (which already has the actual sample rate)
            eprintln!("[CPAL] No matching config found, using default");
            default_config
        };

        let sample_format = config.sample_format();

        eprintln!("[CPAL] Selected config:");
        eprintln!("  - Sample rate: {:?} (device's actual rate)", config.sample_rate());
        eprintln!("  - Channels: {}", config.channels());
        eprintln!("  - Sample format: {:?}", sample_format);
        eprintln!("  - Buffer size: {:?}", config.buffer_size());

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
                eprintln!("[CPAL] Using fixed buffer size: {} frames (range: {}-{})", buffer_size, min, max);
            }
            cpal::SupportedBufferSize::Unknown => {
                // For unknown buffer size, try a common default
                // Many ASIO drivers work well with 256 or 512
                eprintln!("[CPAL] Buffer size unknown, trying default of 512 frames");
                stream_config.buffer_size = cpal::BufferSize::Fixed(512);
            }
        }

        Ok((stream_config, sample_format))
    }

    /// Load next track when track finishes (called from audio callbacks)
    ///
    /// This handles the case where `process_audio` detects track end and calls
    /// `handle_track_finished()` → `next()`, which sets state to Loading.
    /// We need to load the audio source for the new track and emit events.
    fn load_next_track(
        mgr: &mut PlaybackManager,
        event_tx: &Sender<PlaybackEvent>,
    ) {
        if let Some(track) = mgr.get_current_track().cloned() {
            let target_sample_rate = mgr.get_sample_rate();
            match crate::sources::local::LocalAudioSource::new(
                &track.path,
                target_sample_rate,
            ) {
                Ok(source) => {
                    mgr.set_audio_source(Box::new(source));
                    let _ = event_tx.try_send(PlaybackEvent::StateChanged(mgr.get_state()));
                    let _ = event_tx.try_send(PlaybackEvent::TrackChanged(Some(track)));
                    let _ = event_tx.try_send(PlaybackEvent::QueueUpdated);
                }
                Err(e) => {
                    eprintln!("[load_next_track] Failed to load next track: {}", e);
                    let _ = event_tx.try_send(PlaybackEvent::Error(format!(
                        "Failed to load next track: {}",
                        e
                    )));
                    mgr.stop();
                    let _ = event_tx.try_send(PlaybackEvent::StateChanged(mgr.get_state()));
                }
            }
        } else {
            // No more tracks - queue is empty, stop playback
            mgr.stop();
            let _ = event_tx.try_send(PlaybackEvent::StateChanged(mgr.get_state()));
            let _ = event_tx.try_send(PlaybackEvent::TrackChanged(None));
        }
    }

    /// Audio callback for f32 sample format (WASAPI, CoreAudio, etc.)
    fn audio_callback_f32(
        data: &mut [f32],
        manager: Arc<Mutex<PlaybackManager>>,
        command_rx: &Receiver<PlaybackCommand>,
        event_tx: &Sender<PlaybackEvent>,
        callback_count: u32,
        stream_id: std::time::Instant,
    ) {
        // Debug: log callback invocation with per-stream counter
        if callback_count == 1 {
            eprintln!("[audio_callback_f32] *** FIRST CALLBACK FOR STREAM {:?} ***", stream_id);
            eprintln!("[audio_callback_f32]   Buffer size: {} samples ({} frames stereo)",
                data.len(), data.len() / 2);
        } else if callback_count <= 5 {
            eprintln!("[audio_callback_f32] Stream {:?} call #{}, buffer: {} samples",
                stream_id, callback_count, data.len());
        } else if callback_count == 6 {
            eprintln!("[audio_callback_f32] Stream {:?}: further callback logs suppressed", stream_id);
        }

        // Process any pending commands
        while let Ok(command) = command_rx.try_recv() {
            if let Err(e) = Self::process_command(command, manager.clone(), event_tx) {
                let _ = event_tx.try_send(PlaybackEvent::Error(format!("Command error: {}", e)));
            }
        }

        // Get audio from playback manager
        let mut mgr = manager.lock().unwrap();

        match mgr.process_audio(data) {
            Ok(_) => {
                // Check if track finished and next track is ready to load
                if mgr.get_state() == soul_playback::PlaybackState::Loading {
                    Self::load_next_track(&mut mgr, event_tx);
                }
            }
            Err(e) => {
                // Error processing audio - fill with silence
                data.fill(0.0);
                let _ = event_tx.try_send(PlaybackEvent::Error(format!(
                    "Audio processing error: {}",
                    e
                )));
            }
        }
    }

    /// Audio callback for i32 sample format (ASIO)
    ///
    /// Uses a pre-allocated f32 buffer to avoid allocation in the real-time audio thread.
    fn audio_callback_i32(
        data: &mut [i32],
        manager: Arc<Mutex<PlaybackManager>>,
        command_rx: &Receiver<PlaybackCommand>,
        event_tx: &Sender<PlaybackEvent>,
        f32_buffer: &mut Vec<f32>,
        callback_count: u32,
        stream_id: std::time::Instant,
    ) {
        // Update global counter for diagnostics
        let global_count = GLOBAL_I32_CALLBACK_COUNTER.fetch_add(1, Ordering::Relaxed);

        // Debug: log callback invocation with per-stream counter
        // Log first 10 callbacks for each new stream
        if callback_count == 1 {
            eprintln!("[audio_callback_i32] *** FIRST CALLBACK FOR STREAM {:?} (global #{}) ***", stream_id, global_count + 1);
            eprintln!("[audio_callback_i32]   Thread ID: {:?}", std::thread::current().id());
            eprintln!("[audio_callback_i32]   Buffer size: {} samples ({} frames stereo)",
                data.len(), data.len() / 2);
        } else if callback_count <= 10 {
            eprintln!("[audio_callback_i32] Stream {:?} call #{} (global #{}), buffer: {} samples",
                stream_id, callback_count, global_count + 1, data.len());
        } else if callback_count == 11 {
            eprintln!("[audio_callback_i32] Stream {:?}: further callback logs suppressed (global #{})", stream_id, global_count + 1);
        } else if callback_count % 1000 == 0 {
            // Log every 1000 callbacks to show the stream is still alive
            eprintln!("[audio_callback_i32] Stream {:?} still alive: {} callbacks (global #{})", stream_id, callback_count, global_count + 1);
        }

        // Process any pending commands
        while let Ok(command) = command_rx.try_recv() {
            eprintln!("[audio_callback_i32] Received command: {:?}", command);
            if let Err(e) = Self::process_command(command, manager.clone(), event_tx) {
                let _ = event_tx.try_send(PlaybackEvent::Error(format!("Command error: {}", e)));
            }
        }

        // Ensure f32 buffer is large enough (only reallocates if needed, and rarely)
        if f32_buffer.len() < data.len() {
            f32_buffer.resize(data.len(), 0.0);
        }
        let f32_slice = &mut f32_buffer[..data.len()];
        f32_slice.fill(0.0);

        // Get audio from playback manager into f32 buffer, then convert to i32
        let mut mgr = manager.lock().unwrap();

        match mgr.process_audio(f32_slice) {
            Ok(_) => {
                // Check if track finished and next track is ready to load
                if mgr.get_state() == soul_playback::PlaybackState::Loading {
                    Self::load_next_track(&mut mgr, event_tx);
                }

                // Convert f32 [-1.0, 1.0] to i32 [-2147483648, 2147483647]
                for (out, &sample) in data.iter_mut().zip(f32_slice.iter()) {
                    // Clamp to valid range and scale to i32
                    let clamped = sample.clamp(-1.0, 1.0);
                    *out = (clamped * i32::MAX as f32) as i32;
                }
            }
            Err(e) => {
                // Error processing audio - fill with silence
                data.fill(0);
                let _ = event_tx.try_send(PlaybackEvent::Error(format!(
                    "Audio processing error: {}",
                    e
                )));
            }
        }
    }

    /// Audio callback for i16 sample format
    ///
    /// Uses a pre-allocated f32 buffer to avoid allocation in the real-time audio thread.
    fn audio_callback_i16(
        data: &mut [i16],
        manager: Arc<Mutex<PlaybackManager>>,
        command_rx: &Receiver<PlaybackCommand>,
        event_tx: &Sender<PlaybackEvent>,
        f32_buffer: &mut Vec<f32>,
        callback_count: u32,
        stream_id: std::time::Instant,
    ) {
        // Debug: log callback invocation with per-stream counter
        if callback_count == 1 {
            eprintln!("[audio_callback_i16] *** FIRST CALLBACK FOR STREAM {:?} ***", stream_id);
            eprintln!("[audio_callback_i16]   Buffer size: {} samples ({} frames stereo)",
                data.len(), data.len() / 2);
        } else if callback_count <= 5 {
            eprintln!("[audio_callback_i16] Stream {:?} call #{}, buffer: {} samples",
                stream_id, callback_count, data.len());
        } else if callback_count == 6 {
            eprintln!("[audio_callback_i16] Stream {:?}: further callback logs suppressed", stream_id);
        }

        // Process any pending commands
        while let Ok(command) = command_rx.try_recv() {
            if let Err(e) = Self::process_command(command, manager.clone(), event_tx) {
                let _ = event_tx.try_send(PlaybackEvent::Error(format!("Command error: {}", e)));
            }
        }

        // Ensure f32 buffer is large enough (only reallocates if needed, and rarely)
        if f32_buffer.len() < data.len() {
            f32_buffer.resize(data.len(), 0.0);
        }
        let f32_slice = &mut f32_buffer[..data.len()];
        f32_slice.fill(0.0);

        // Get audio from playback manager into f32 buffer, then convert to i16
        let mut mgr = manager.lock().unwrap();

        match mgr.process_audio(f32_slice) {
            Ok(_) => {
                // Check if track finished and next track is ready to load
                if mgr.get_state() == soul_playback::PlaybackState::Loading {
                    Self::load_next_track(&mut mgr, event_tx);
                }

                // Convert f32 [-1.0, 1.0] to i16 [-32768, 32767]
                for (out, &sample) in data.iter_mut().zip(f32_slice.iter()) {
                    // Clamp to valid range and scale to i16
                    let clamped = sample.clamp(-1.0, 1.0);
                    *out = (clamped * i16::MAX as f32) as i16;
                }
            }
            Err(e) => {
                // Error processing audio - fill with silence
                data.fill(0);
                let _ = event_tx.try_send(PlaybackEvent::Error(format!(
                    "Audio processing error: {}",
                    e
                )));
            }
        }
    }

    /// Process playback command
    fn process_command(
        command: PlaybackCommand,
        manager: Arc<Mutex<PlaybackManager>>,
        event_tx: &Sender<PlaybackEvent>,
    ) -> Result<()> {
        let mut mgr = manager.lock().unwrap();

        match command {
            PlaybackCommand::Play => {
                eprintln!("[PlaybackCommand::Play] Received");
                mgr.play()?;

                let state = mgr.get_state();
                eprintln!("[PlaybackCommand::Play] State after play(): {:?}", state);

                // If state is Loading, we need to load the audio source
                if state == soul_playback::PlaybackState::Loading {
                    if let Some(track) = mgr.get_current_track().cloned() {
                        eprintln!(
                            "[PlaybackCommand::Play] Loading track: {} from {}",
                            track.title,
                            track.path.display()
                        );
                        // Get target sample rate from manager
                        let target_sample_rate = mgr.get_sample_rate();
                        eprintln!(
                            "[PlaybackCommand::Play] Target sample rate: {}",
                            target_sample_rate
                        );
                        // Create audio source from file path
                        match crate::sources::local::LocalAudioSource::new(
                            &track.path,
                            target_sample_rate,
                        ) {
                            Ok(source) => {
                                eprintln!(
                                    "[PlaybackCommand::Play] Audio source loaded successfully (source rate: {}, target rate: {})",
                                    source.source_sample_rate(),
                                    source.sample_rate()
                                );
                                mgr.set_audio_source(Box::new(source));
                                event_tx
                                    .send(PlaybackEvent::StateChanged(mgr.get_state()))
                                    .ok();
                                event_tx
                                    .send(PlaybackEvent::TrackChanged(Some(track.clone())))
                                    .ok();
                            }
                            Err(e) => {
                                eprintln!("[PlaybackCommand::Play] Failed to load audio: {}", e);
                                event_tx
                                    .send(PlaybackEvent::Error(format!(
                                        "Failed to load audio: {}",
                                        e
                                    )))
                                    .ok();
                                mgr.stop();
                            }
                        }
                    } else {
                        eprintln!("[PlaybackCommand::Play] No current track to load");
                    }
                } else {
                    eprintln!(
                        "[PlaybackCommand::Play] State is {:?}, not loading audio",
                        state
                    );
                    event_tx
                        .send(PlaybackEvent::StateChanged(mgr.get_state()))
                        .ok();
                }
            }
            PlaybackCommand::Pause => {
                mgr.pause();
                event_tx
                    .send(PlaybackEvent::StateChanged(mgr.get_state()))
                    .ok();
            }
            PlaybackCommand::Stop => {
                mgr.stop();
                event_tx
                    .send(PlaybackEvent::StateChanged(mgr.get_state()))
                    .ok();
            }
            PlaybackCommand::Next => {
                mgr.next()?;

                // If state is Loading, we need to load the audio source
                if mgr.get_state() == soul_playback::PlaybackState::Loading {
                    if let Some(track) = mgr.get_current_track().cloned() {
                        let target_sample_rate = mgr.get_sample_rate();
                        match crate::sources::local::LocalAudioSource::new(
                            &track.path,
                            target_sample_rate,
                        ) {
                            Ok(source) => {
                                mgr.set_audio_source(Box::new(source));
                                event_tx
                                    .send(PlaybackEvent::StateChanged(mgr.get_state()))
                                    .ok();
                                event_tx
                                    .send(PlaybackEvent::TrackChanged(Some(track.clone())))
                                    .ok();
                            }
                            Err(e) => {
                                event_tx
                                    .send(PlaybackEvent::Error(format!(
                                        "Failed to load audio: {}",
                                        e
                                    )))
                                    .ok();
                                mgr.stop();
                            }
                        }
                    }
                } else {
                    event_tx
                        .send(PlaybackEvent::TrackChanged(
                            mgr.get_current_track().cloned(),
                        ))
                        .ok();
                }
                // Emit queue updated since position changed
                event_tx.send(PlaybackEvent::QueueUpdated).ok();
            }
            PlaybackCommand::Previous => {
                mgr.previous()?;

                // If state is Loading, we need to load the audio source
                if mgr.get_state() == soul_playback::PlaybackState::Loading {
                    if let Some(track) = mgr.get_current_track().cloned() {
                        let target_sample_rate = mgr.get_sample_rate();
                        match crate::sources::local::LocalAudioSource::new(
                            &track.path,
                            target_sample_rate,
                        ) {
                            Ok(source) => {
                                mgr.set_audio_source(Box::new(source));
                                event_tx
                                    .send(PlaybackEvent::StateChanged(mgr.get_state()))
                                    .ok();
                                event_tx
                                    .send(PlaybackEvent::TrackChanged(Some(track.clone())))
                                    .ok();
                            }
                            Err(e) => {
                                event_tx
                                    .send(PlaybackEvent::Error(format!(
                                        "Failed to load audio: {}",
                                        e
                                    )))
                                    .ok();
                                mgr.stop();
                            }
                        }
                    }
                } else {
                    event_tx
                        .send(PlaybackEvent::TrackChanged(
                            mgr.get_current_track().cloned(),
                        ))
                        .ok();
                }
                // Emit queue updated since position changed
                event_tx.send(PlaybackEvent::QueueUpdated).ok();
            }
            PlaybackCommand::Seek(seconds) => {
                mgr.seek_to(std::time::Duration::from_secs_f64(seconds))?;
                event_tx.send(PlaybackEvent::PositionUpdated(seconds)).ok();
            }
            PlaybackCommand::SetVolume(volume) => {
                mgr.set_volume(volume);
                event_tx.send(PlaybackEvent::VolumeChanged(volume)).ok();
            }
            PlaybackCommand::Mute => {
                mgr.mute();
                event_tx
                    .send(PlaybackEvent::VolumeChanged(mgr.get_volume()))
                    .ok();
            }
            PlaybackCommand::Unmute => {
                mgr.unmute();
                event_tx
                    .send(PlaybackEvent::VolumeChanged(mgr.get_volume()))
                    .ok();
            }
            PlaybackCommand::AddToQueue(track) => {
                mgr.add_to_queue_end(track);
                event_tx.send(PlaybackEvent::QueueUpdated).ok();
            }
            PlaybackCommand::RemoveFromQueue(index) => {
                mgr.remove_from_queue(index)?;
                event_tx.send(PlaybackEvent::QueueUpdated).ok();
            }
            PlaybackCommand::ClearQueue => {
                mgr.clear_queue();
                event_tx.send(PlaybackEvent::QueueUpdated).ok();
            }
            PlaybackCommand::SkipToQueueIndex(index) => {
                mgr.skip_to_queue_index(index)?;

                // If state is Loading, we need to load the audio source
                if mgr.get_state() == soul_playback::PlaybackState::Loading {
                    if let Some(track) = mgr.get_current_track().cloned() {
                        let target_sample_rate = mgr.get_sample_rate();
                        match crate::sources::local::LocalAudioSource::new(
                            &track.path,
                            target_sample_rate,
                        ) {
                            Ok(source) => {
                                mgr.set_audio_source(Box::new(source));
                                event_tx
                                    .send(PlaybackEvent::StateChanged(mgr.get_state()))
                                    .ok();
                                event_tx
                                    .send(PlaybackEvent::TrackChanged(Some(track.clone())))
                                    .ok();
                            }
                            Err(e) => {
                                event_tx
                                    .send(PlaybackEvent::Error(format!(
                                        "Failed to load audio: {}",
                                        e
                                    )))
                                    .ok();
                                mgr.stop();
                            }
                        }
                    }
                }
                event_tx.send(PlaybackEvent::QueueUpdated).ok();
            }
            PlaybackCommand::LoadPlaylist(tracks) => {
                // Load playlist/album as source queue (Spotify-style context)
                mgr.add_playlist_to_queue(tracks);
                event_tx.send(PlaybackEvent::QueueUpdated).ok();
            }
            PlaybackCommand::SetShuffle(mode) => {
                mgr.set_shuffle(mode);
                event_tx.send(PlaybackEvent::QueueUpdated).ok();
            }
            PlaybackCommand::SetRepeat(mode) => {
                mgr.set_repeat(mode);
            }
            PlaybackCommand::SwitchDevice(_, _) => {
                // Device switching is handled externally via switch_device() method
                // This command shouldn't reach here, but log if it does
                eprintln!("[WARN] SwitchDevice command received in audio callback - should be handled externally");
            }
        }

        Ok(())
    }

    // Public API

    /// Send command to playback thread
    ///
    /// Uses try_send to avoid blocking if the channel is full (e.g., when
    /// audio callbacks aren't running). Commands may be dropped if the
    /// channel is full - this prevents deadlocks when switching audio devices.
    pub fn send_command(&self, command: PlaybackCommand) -> Result<()> {
        eprintln!("[DesktopPlayback] Sending command: {:?}", command);

        // Debug: Check if stream is still alive and channel state
        let stream_alive = {
            let stream_guard = self.stream.lock().unwrap();
            stream_guard.is_some()
        };
        let channel_len = self.command_tx.len();
        let channel_capacity = self.command_tx.capacity().unwrap_or(0);
        let channel_is_empty = self.command_tx.is_empty();
        let channel_is_full = self.command_tx.is_full();

        eprintln!("[DesktopPlayback] Stream alive: {}, Channel: len={}, cap={}, empty={}, full={}",
            stream_alive, channel_len, channel_capacity, channel_is_empty, channel_is_full);

        // Get current backend for context
        let backend = *self.current_backend.lock().unwrap();
        let device = self.current_device.lock().unwrap().clone();
        eprintln!("[DesktopPlayback] Current backend: {:?}, device: {}", backend, device);

        match self.command_tx.try_send(command.clone()) {
            Ok(()) => {
                eprintln!("[DesktopPlayback] Command sent successfully");
                Ok(())
            }
            Err(crossbeam_channel::TrySendError::Full(_)) => {
                eprintln!("[DesktopPlayback] WARNING: Command channel FULL, dropping command: {:?}", command);
                // Return Ok to not fail the operation - the command is just dropped
                // This can happen when switching audio devices and callbacks aren't running yet
                Ok(())
            }
            Err(crossbeam_channel::TrySendError::Disconnected(_)) => {
                eprintln!("[DesktopPlayback] !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!");
                eprintln!("[DesktopPlayback] ERROR: Command channel disconnected!");
                eprintln!("[DesktopPlayback] !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!");
                eprintln!("[DesktopPlayback] Stream is_some: {}", stream_alive);
                eprintln!("[DesktopPlayback] Backend: {:?}", backend);
                eprintln!("[DesktopPlayback] Device: {}", device);
                eprintln!("[DesktopPlayback] This means the stream's command receiver (command_rx) was dropped.");
                eprintln!("[DesktopPlayback] Possible causes:");
                eprintln!("[DesktopPlayback]   1. ASIO driver silently terminated the stream");
                eprintln!("[DesktopPlayback]   2. Stream error callback was triggered");
                eprintln!("[DesktopPlayback]   3. Stream was dropped elsewhere");
                eprintln!("[DesktopPlayback]   4. command_tx was not updated after device switch");

                // Get the global callback counter for diagnostics
                let global_count = GLOBAL_I32_CALLBACK_COUNTER.load(Ordering::Relaxed);
                eprintln!("[DesktopPlayback] Global I32 callback count: {}", global_count);

                Err(crate::error::AudioError::PlaybackError(
                    "Command channel disconnected - stream may have been terminated".into(),
                )
                .into())
            }
        }
    }

    /// Try to receive next event (non-blocking)
    pub fn try_recv_event(&self) -> Option<PlaybackEvent> {
        self.event_rx.try_recv().ok()
    }

    /// Receive next event (blocking)
    pub fn recv_event(&self) -> Option<PlaybackEvent> {
        self.event_rx.recv().ok()
    }

    /// Get current playback state
    pub fn get_state(&self) -> soul_playback::PlaybackState {
        self.manager.lock().unwrap().get_state()
    }

    /// Get current track
    pub fn get_current_track(&self) -> Option<QueueTrack> {
        self.manager.lock().unwrap().get_current_track().cloned()
    }

    /// Get current position
    pub fn get_position(&self) -> std::time::Duration {
        self.manager.lock().unwrap().get_position()
    }

    /// Get queue
    pub fn get_queue(&self) -> Vec<soul_playback::QueueTrack> {
        self.manager
            .lock()
            .unwrap()
            .get_queue()
            .into_iter()
            .cloned()
            .collect()
    }

    /// Check if there is a next track
    pub fn has_next(&self) -> bool {
        self.manager.lock().unwrap().has_next()
    }

    /// Check if there is a previous track
    pub fn has_previous(&self) -> bool {
        self.manager.lock().unwrap().has_previous()
    }

    /// Get current volume
    pub fn get_volume(&self) -> u8 {
        self.manager.lock().unwrap().get_volume()
    }

    /// Switch to a different audio output device
    ///
    /// This will pause playback, switch to the new device, and resume if was playing.
    /// Playback position is preserved across the switch.
    ///
    /// # Arguments
    /// * `backend` - Audio backend to use
    /// * `device_name` - Device name to switch to (None for default device)
    ///
    /// # Returns
    /// * `Ok(())` - Device switched successfully
    /// * `Err(_)` - Failed to switch device
    pub fn switch_device(
        &mut self,
        backend: crate::AudioBackend,
        device_name: Option<String>,
    ) -> Result<()> {
        eprintln!("[DesktopPlayback] ==========================================");
        eprintln!("[DesktopPlayback] SWITCHING AUDIO DEVICE");
        eprintln!("[DesktopPlayback]   Thread ID: {:?}", std::thread::current().id());
        eprintln!("[DesktopPlayback]   Backend: {:?}", backend);
        eprintln!("[DesktopPlayback]   Device: {:?}", device_name);
        eprintln!("[DesktopPlayback] ==========================================");

        // Save current state
        let (was_playing, position) = {
            let mgr = self.manager.lock().unwrap();
            let state = mgr.get_state();
            let pos = mgr.get_position();
            (state == soul_playback::PlaybackState::Playing, pos)
        };

        eprintln!(
            "[DesktopPlayback] Current state: playing={}, position={:?}",
            was_playing, position
        );

        // Stop and drop the old stream
        // IMPORTANT: ASIO requires proper cleanup between stream creations
        {
            let mut stream_guard = self.stream.lock().unwrap();
            if let Some(stream) = stream_guard.take() {
                eprintln!("[DesktopPlayback] Pausing old stream before drop...");
                // Try to pause the stream first (some drivers need this)
                if let Err(e) = stream.pause() {
                    eprintln!("[DesktopPlayback] Warning: Failed to pause old stream: {}", e);
                }
                eprintln!("[DesktopPlayback] Dropping old stream...");
                drop(stream);
                eprintln!("[DesktopPlayback] Old stream dropped");
            }
        }

        // Longer delay for ASIO - drivers often need time to release resources
        eprintln!("[DesktopPlayback] Waiting for driver to release resources...");
        std::thread::sleep(std::time::Duration::from_millis(200));
        eprintln!("[DesktopPlayback] Resource release wait complete");

        // Create new command channel for the new stream
        eprintln!("[DesktopPlayback] Creating new command channel...");
        let (new_command_tx, new_command_rx) = bounded(32);

        // Update command_tx for this instance
        eprintln!("[DesktopPlayback] Updating command_tx to new channel...");
        self.command_tx = new_command_tx.clone();
        eprintln!("[DesktopPlayback] command_tx updated, channel capacity: {}", self.command_tx.capacity().unwrap_or(0));

        // Create new stream with new device, reusing the same event_tx
        let (new_stream, actual_device_name, new_sample_rate) = Self::create_audio_stream(
            self.manager.clone(),
            new_command_rx,
            self.event_tx.clone(),
            backend,
            device_name.clone(),
        )?;

        // Check if sample rate changed
        let old_sample_rate = self.current_sample_rate.load(Ordering::SeqCst);
        if old_sample_rate != new_sample_rate {
            eprintln!(
                "[DesktopPlayback] Sample rate changed: {} Hz -> {} Hz",
                old_sample_rate, new_sample_rate
            );
            self.current_sample_rate.store(new_sample_rate, Ordering::SeqCst);
            let _ = self.event_tx.try_send(PlaybackEvent::SampleRateChanged(old_sample_rate, new_sample_rate));
        }

        eprintln!(
            "[DesktopPlayback] New stream created for device: {} at {} Hz",
            actual_device_name, new_sample_rate
        );

        // Check callbacks before storing
        let callbacks_before_store = GLOBAL_I32_CALLBACK_COUNTER.load(Ordering::Relaxed);
        eprintln!("[DesktopPlayback] Callbacks before storing stream: {}", callbacks_before_store);

        // Store new stream
        eprintln!("[DesktopPlayback] Storing new stream...");
        {
            let mut stream_guard = self.stream.lock().unwrap();
            *stream_guard = Some(new_stream);
        }

        // Check callbacks immediately after storing
        let callbacks_after_store = GLOBAL_I32_CALLBACK_COUNTER.load(Ordering::Relaxed);
        eprintln!("[DesktopPlayback] Stream stored. Callbacks: {} (diff: {})",
            callbacks_after_store, callbacks_after_store - callbacks_before_store);
        eprintln!("[DesktopPlayback] Waiting 100ms for callbacks to start...");

        // Give the new stream a moment to start callbacks
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Check callbacks after sleep
        let callbacks_after_sleep = GLOBAL_I32_CALLBACK_COUNTER.load(Ordering::Relaxed);
        eprintln!("[DesktopPlayback] After 100ms sleep. Callbacks: {} (diff: {})",
            callbacks_after_sleep, callbacks_after_sleep - callbacks_after_store);

        // Verify channel status after stream creation
        let channel_len = self.command_tx.len();
        let channel_cap = self.command_tx.capacity().unwrap_or(0);
        let callbacks_so_far = GLOBAL_I32_CALLBACK_COUNTER.load(Ordering::Relaxed);
        eprintln!("[DesktopPlayback] Channel verification after stream creation:");
        eprintln!("[DesktopPlayback]   Queue length: {}", channel_len);
        eprintln!("[DesktopPlayback]   Capacity: {}", channel_cap);
        eprintln!("[DesktopPlayback]   Global I32 callbacks: {}", callbacks_so_far);

        // Check callbacks before updating backend
        let callbacks_before_backend = GLOBAL_I32_CALLBACK_COUNTER.load(Ordering::Relaxed);
        eprintln!("[DesktopPlayback] Callbacks before backend update: {}", callbacks_before_backend);

        // Update current backend and device
        {
            *self.current_backend.lock().unwrap() = backend;
            *self.current_device.lock().unwrap() = actual_device_name.clone();
        }

        // Check callbacks after updating backend
        let callbacks_after_backend = GLOBAL_I32_CALLBACK_COUNTER.load(Ordering::Relaxed);
        eprintln!("[DesktopPlayback] Backend and device name updated");
        eprintln!("[DesktopPlayback] Callbacks after backend update: {} (diff: {})",
            callbacks_after_backend, callbacks_after_backend - callbacks_before_backend);

        // Reload the audio source with the new sample rate
        // This is necessary because the old audio source was created with the old device's sample rate
        let current_track = {
            let mgr = self.manager.lock().unwrap();
            mgr.get_current_track().cloned()
        };

        if let Some(track) = current_track {
            eprintln!(
                "[DesktopPlayback] Reloading audio source for new sample rate"
            );

            let target_sample_rate = {
                let mgr = self.manager.lock().unwrap();
                mgr.get_sample_rate()
            };

            match crate::sources::local::LocalAudioSource::new(&track.path, target_sample_rate) {
                Ok(source) => {
                    let mut mgr = self.manager.lock().unwrap();
                    mgr.set_audio_source(Box::new(source));
                    eprintln!(
                        "[DesktopPlayback] Audio source reloaded with sample rate: {}",
                        target_sample_rate
                    );
                }
                Err(e) => {
                    eprintln!(
                        "[DesktopPlayback] Failed to reload audio source: {}",
                        e
                    );
                }
            }
        }

        // Restore position if we had one
        if position > std::time::Duration::ZERO {
            let mut mgr = self.manager.lock().unwrap();
            if let Err(e) = mgr.seek_to(position) {
                eprintln!("[DesktopPlayback] Failed to restore position: {}", e);
            } else {
                eprintln!(
                    "[DesktopPlayback] Position restored to {:?}",
                    position
                );
            }
        }

        // Resume playback if it was playing
        if was_playing {
            let mut mgr = self.manager.lock().unwrap();
            if let Err(e) = mgr.play() {
                eprintln!("[DesktopPlayback] Failed to resume playback: {}", e);
            } else {
                eprintln!("[DesktopPlayback] Playback resumed");
            }
        }

        // Always emit state changed event after device switch to ensure frontend sync
        // This is critical because the frontend's play/pause button must reflect the actual state
        let current_state = {
            let mgr = self.manager.lock().unwrap();
            mgr.get_state()
        };
        eprintln!("[DesktopPlayback] Emitting StateChanged after device switch: {:?}", current_state);
        // Use send() with timeout to ensure delivery of this critical event
        // Fall back to try_send if the blocking send times out
        match self.event_tx.send_timeout(
            PlaybackEvent::StateChanged(current_state),
            std::time::Duration::from_millis(100),
        ) {
            Ok(()) => {
                eprintln!("[DesktopPlayback] StateChanged event sent successfully");
            }
            Err(crossbeam_channel::SendTimeoutError::Timeout(_)) => {
                eprintln!("[DesktopPlayback] WARNING: StateChanged event timed out, frontend may be out of sync");
            }
            Err(crossbeam_channel::SendTimeoutError::Disconnected(_)) => {
                eprintln!("[DesktopPlayback] ERROR: Event channel disconnected");
            }
        }

        // Final callback check before returning
        let callbacks_at_end = GLOBAL_I32_CALLBACK_COUNTER.load(Ordering::Relaxed);
        eprintln!("[DesktopPlayback] Device switch complete. Final callback count: {}", callbacks_at_end);
        Ok(())
    }

    /// Get current backend
    pub fn get_current_backend(&self) -> crate::AudioBackend {
        *self.current_backend.lock().unwrap()
    }

    /// Get current device name
    pub fn get_current_device(&self) -> String {
        self.current_device.lock().unwrap().clone()
    }

    /// Get current stream sample rate
    pub fn get_current_sample_rate(&self) -> u32 {
        self.current_sample_rate.load(Ordering::SeqCst)
    }

    /// Query the device's current sample rate from the driver
    ///
    /// This queries the device directly to get its current configuration,
    /// which may differ from what we're outputting at if the user changed
    /// settings in the driver's control panel (e.g., ASIO settings).
    ///
    /// # Returns
    /// * `Ok(sample_rate)` - The device's current sample rate
    /// * `Err(_)` - Failed to query the device
    pub fn query_device_sample_rate(&self) -> Result<u32> {
        let backend = *self.current_backend.lock().unwrap();
        let device_name = self.current_device.lock().unwrap().clone();

        let device = crate::device::find_device_by_name(backend, &device_name)
            .map_err(|e| crate::error::AudioError::DeviceError(e.to_string()))?;

        let (config, _) = Self::get_stream_config(&device)?;
        Ok(config.sample_rate)
    }

    /// Check if the device's sample rate has changed and handle it
    ///
    /// This method should be called periodically (e.g., every few seconds)
    /// to detect if the user has changed the device's sample rate externally
    /// (e.g., via ASIO control panel, Windows sound settings, etc.).
    ///
    /// If a change is detected:
    /// 1. The audio stream is recreated with the new sample rate
    /// 2. The audio source is reloaded to resample correctly
    /// 3. Playback position is preserved
    /// 4. A `SampleRateChanged` event is emitted
    ///
    /// # Returns
    /// * `Ok(true)` - Sample rate changed and stream was recreated
    /// * `Ok(false)` - Sample rate unchanged, no action needed
    /// * `Err(_)` - Failed to check or update sample rate
    pub fn check_and_update_sample_rate(&mut self) -> Result<bool> {
        let device_rate = match self.query_device_sample_rate() {
            Ok(rate) => rate,
            Err(e) => {
                eprintln!("[DesktopPlayback] Failed to query device sample rate: {}", e);
                return Err(e);
            }
        };

        let current_rate = self.current_sample_rate.load(Ordering::SeqCst);

        if device_rate == current_rate {
            // No change
            return Ok(false);
        }

        eprintln!(
            "[DesktopPlayback] Device sample rate changed: {} Hz -> {} Hz",
            current_rate, device_rate
        );

        // Sample rate has changed - need to recreate the stream
        let backend = *self.current_backend.lock().unwrap();
        let device_name = self.current_device.lock().unwrap().clone();

        // switch_device will handle everything: stream recreation, source reload, position preservation
        self.switch_device(backend, Some(device_name))?;

        Ok(true)
    }

    /// Refresh the audio stream
    ///
    /// This is a convenience method that recreates the stream with the current device.
    /// Useful when you want to ensure the stream is using the device's current settings.
    ///
    /// # Returns
    /// * `Ok(())` - Stream refreshed successfully
    /// * `Err(_)` - Failed to refresh stream
    pub fn refresh_stream(&mut self) -> Result<()> {
        let backend = *self.current_backend.lock().unwrap();
        let device_name = self.current_device.lock().unwrap().clone();
        self.switch_device(backend, Some(device_name))
    }

    /// Get mutable reference to effect chain (for configuring DSP effects)
    ///
    /// # Returns
    /// Returns the effect chain from the underlying PlaybackManager.
    /// Effects are applied in order before volume control.
    ///
    /// # Example
    /// ```no_run
    /// use soul_audio::effects::{ParametricEq, EqBand};
    ///
    /// # fn example(playback: &mut soul_audio_desktop::DesktopPlayback) {
    /// playback.with_effect_chain(|chain| {
    ///     let mut eq = ParametricEq::new();
    ///     eq.set_low_band(EqBand::low_shelf(80.0, 3.0));
    ///     chain.add_effect(Box::new(eq));
    /// });
    /// # }
    /// ```
    #[cfg(feature = "effects")]
    pub fn with_effect_chain<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut soul_audio::effects::EffectChain) -> R,
    {
        let mut manager = self.manager.lock().unwrap();
        f(manager.effect_chain_mut())
    }

    // ===== Volume Leveling =====

    /// Set volume leveling mode (ReplayGain track/album, EBU R128, etc.)
    pub fn set_volume_leveling_mode(&self, mode: soul_playback::NormalizationMode) {
        let mut manager = self.manager.lock().unwrap();
        manager.set_volume_leveling_mode(mode);
    }

    /// Get current volume leveling mode
    pub fn get_volume_leveling_mode(&self) -> soul_playback::NormalizationMode {
        let manager = self.manager.lock().unwrap();
        manager.get_volume_leveling_mode()
    }

    /// Set track gain for current track (called when loading track)
    ///
    /// # Arguments
    /// * `gain_db` - ReplayGain value in dB
    /// * `peak_dbfs` - Peak value in dBFS (for clipping prevention)
    pub fn set_track_gain(&self, gain_db: f64, peak_dbfs: f64) {
        let mut manager = self.manager.lock().unwrap();
        manager.set_track_gain(gain_db, peak_dbfs);
    }

    /// Set album gain for current track (called when loading track)
    ///
    /// # Arguments
    /// * `gain_db` - Album ReplayGain value in dB
    /// * `peak_dbfs` - Album peak value in dBFS
    pub fn set_album_gain(&self, gain_db: f64, peak_dbfs: f64) {
        let mut manager = self.manager.lock().unwrap();
        manager.set_album_gain(gain_db, peak_dbfs);
    }

    /// Clear gain values (for new track without loudness data)
    pub fn clear_loudness_gains(&self) {
        let mut manager = self.manager.lock().unwrap();
        manager.clear_loudness_gains();
    }

    /// Set pre-amp gain for volume leveling (-12 to +12 dB)
    pub fn set_loudness_preamp(&self, preamp_db: f64) {
        let mut manager = self.manager.lock().unwrap();
        manager.set_loudness_preamp(preamp_db);
    }

    /// Get pre-amp gain
    pub fn get_loudness_preamp(&self) -> f64 {
        let manager = self.manager.lock().unwrap();
        manager.get_loudness_preamp()
    }

    /// Set whether clipping prevention is enabled
    pub fn set_prevent_clipping(&self, prevent: bool) {
        let mut manager = self.manager.lock().unwrap();
        manager.set_prevent_clipping(prevent);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_desktop_playback() {
        let result = DesktopPlayback::new(PlaybackConfig::default());

        // May fail if no audio device available
        match result {
            Ok(_) => {
                // Success
            }
            Err(e) => {
                eprintln!(
                    "Note: Audio device not available in test environment: {}",
                    e
                );
            }
        }
    }

    #[test]
    fn test_create_with_default_backend() {
        let result = DesktopPlayback::new_with_device(
            PlaybackConfig::default(),
            crate::AudioBackend::Default,
            None,
        );

        match result {
            Ok(playback) => {
                assert_eq!(
                    playback.get_current_backend(),
                    crate::AudioBackend::Default
                );
                assert!(!playback.get_current_device().is_empty());
            }
            Err(e) => {
                eprintln!(
                    "Note: Audio device not available in test environment: {}",
                    e
                );
            }
        }
    }

    #[test]
    fn test_get_current_device_info() {
        let result = DesktopPlayback::new(PlaybackConfig::default());

        match result {
            Ok(playback) => {
                let backend = playback.get_current_backend();
                let device = playback.get_current_device();

                eprintln!("Current backend: {:?}", backend);
                eprintln!("Current device: {}", device);

                assert_eq!(backend, crate::AudioBackend::Default);
                assert!(!device.is_empty());
            }
            Err(e) => {
                eprintln!(
                    "Note: Audio device not available in test environment: {}",
                    e
                );
            }
        }
    }

    #[test]
    fn test_switch_device_to_default() {
        let result = DesktopPlayback::new(PlaybackConfig::default());

        match result {
            Ok(mut playback) => {
                let original_device = playback.get_current_device();
                eprintln!("Original device: {}", original_device);

                // Try to switch to default device again (should succeed)
                let switch_result =
                    playback.switch_device(crate::AudioBackend::Default, None);

                match switch_result {
                    Ok(_) => {
                        let new_device = playback.get_current_device();
                        eprintln!("After switch device: {}", new_device);
                        assert!(!new_device.is_empty());
                    }
                    Err(e) => {
                        eprintln!("Device switch failed (expected on some systems): {}", e);
                    }
                }
            }
            Err(e) => {
                eprintln!(
                    "Note: Audio device not available in test environment: {}",
                    e
                );
            }
        }
    }

    #[test]
    fn test_switch_device_preserves_backend() {
        let result = DesktopPlayback::new(PlaybackConfig::default());

        match result {
            Ok(mut playback) => {
                // Switch to default backend explicitly
                if let Ok(_) = playback.switch_device(crate::AudioBackend::Default, None) {
                    assert_eq!(
                        playback.get_current_backend(),
                        crate::AudioBackend::Default
                    );
                }
            }
            Err(e) => {
                eprintln!(
                    "Note: Audio device not available in test environment: {}",
                    e
                );
            }
        }
    }

    #[test]
    fn test_switch_device_invalid_device() {
        let result = DesktopPlayback::new(PlaybackConfig::default());

        match result {
            Ok(mut playback) => {
                // Try to switch to a device that doesn't exist
                let switch_result = playback.switch_device(
                    crate::AudioBackend::Default,
                    Some("NonexistentDevice123456789".to_string()),
                );

                // Should fail
                assert!(
                    switch_result.is_err(),
                    "Switching to nonexistent device should fail"
                );
            }
            Err(e) => {
                eprintln!(
                    "Note: Audio device not available in test environment: {}",
                    e
                );
            }
        }
    }
}
