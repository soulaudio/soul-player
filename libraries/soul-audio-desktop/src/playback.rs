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
        let (stream, actual_device_name) = Self::create_audio_stream(
            manager.clone(),
            command_rx.clone(),
            event_tx.clone(),
            backend,
            device_name,
        )?;

        let stream = Arc::new(Mutex::new(Some(stream)));
        let current_backend = Arc::new(Mutex::new(backend));
        let current_device = Arc::new(Mutex::new(actual_device_name));

        Ok(Self {
            command_tx,
            event_rx,
            event_tx,
            stream,
            manager,
            current_backend,
            current_device,
        })
    }

    /// Create CPAL audio stream
    ///
    /// Returns (Stream, device_name)
    fn create_audio_stream(
        manager: Arc<Mutex<PlaybackManager>>,
        command_rx: Receiver<PlaybackCommand>,
        event_tx: Sender<PlaybackEvent>,
        backend: crate::AudioBackend,
        device_name: Option<String>,
    ) -> Result<(Stream, String)> {
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
        let sample_rate = config.sample_rate.0;
        let channels = config.channels;

        // Set sample rate and channel count in manager
        {
            let mut mgr = manager.lock().unwrap();
            mgr.set_sample_rate(sample_rate);
            mgr.set_output_channels(channels);
        }

        eprintln!("[CPAL] Building output stream with config: sample_rate={}, channels={}, buffer_size={:?}, format={:?}",
            config.sample_rate.0, config.channels, config.buffer_size, sample_format);

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
                device.build_output_stream(
                    &config,
                    move |data: &mut [i32], _: &cpal::OutputCallbackInfo| {
                        callback_count += 1;
                        Self::audio_callback_i32(data, manager_clone.clone(), &command_rx, &event_tx, &mut f32_buffer, callback_count, stream_id);
                    },
                    |err| eprintln!("[CPAL] Audio stream error callback: {}", err),
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
        eprintln!("[CPAL]   Sample rate: {} Hz", config.sample_rate.0);
        eprintln!("[CPAL]   Channels: {}", config.channels);
        eprintln!("[CPAL]   Sample format: {:?}", sample_format);
        eprintln!("[CPAL]   Buffer size: {:?}", config.buffer_size);
        eprintln!("[CPAL] ==========================================");
        eprintln!("[CPAL] Audio callbacks should start momentarily...");

        Ok((stream, actual_device_name))
    }

    /// Get stream configuration
    /// Returns (StreamConfig, SampleFormat)
    fn get_stream_config(device: &Device) -> Result<(StreamConfig, cpal::SampleFormat)> {
        // First, check all supported output configs
        eprintln!("[CPAL] Checking supported output configurations...");

        let supported_configs: Vec<_> = device
            .supported_output_configs()
            .map(|configs| configs.collect())
            .unwrap_or_default();

        for cfg in &supported_configs {
            eprintln!("[CPAL]   Supported: channels={}, sample_rate={:?}-{:?}, format={:?}",
                cfg.channels(),
                cfg.min_sample_rate().0,
                cfg.max_sample_rate().0,
                cfg.sample_format());
        }

        // Priority order: prefer stereo with highest sample rate, then format preference
        // For ASIO, sample rate MUST match what's configured in the driver control panel

        // First, find all stereo configs and sort by sample rate (highest first)
        let mut stereo_configs: Vec<_> = supported_configs.iter()
            .filter(|c| c.channels() == 2)
            .collect();
        stereo_configs.sort_by(|a, b| b.max_sample_rate().0.cmp(&a.max_sample_rate().0));

        // Prefer f32 > i32 > i16, but prioritize higher sample rates
        let best_config = stereo_configs.iter()
            .find(|c| c.sample_format() == cpal::SampleFormat::F32)
            .or_else(|| stereo_configs.iter().find(|c| c.sample_format() == cpal::SampleFormat::I32))
            .or_else(|| stereo_configs.iter().find(|c| c.sample_format() == cpal::SampleFormat::I16))
            .copied()
            .or_else(|| supported_configs.iter().find(|c| c.sample_format() == cpal::SampleFormat::F32))
            .or_else(|| supported_configs.iter().find(|c| c.sample_format() == cpal::SampleFormat::I32))
            .or_else(|| supported_configs.iter().find(|c| c.sample_format() == cpal::SampleFormat::I16));

        let config = if let Some(cfg) = best_config {
            // For ASIO with fixed sample rates, use the highest available
            // For range-based sample rates, prefer 96kHz > 48kHz > max
            let sample_rate = if cfg.min_sample_rate() == cfg.max_sample_rate() {
                // Fixed sample rate - use as-is
                cfg.max_sample_rate()
            } else if cfg.min_sample_rate().0 <= 96000 && cfg.max_sample_rate().0 >= 96000 {
                cpal::SampleRate(96000)
            } else if cfg.min_sample_rate().0 <= 48000 && cfg.max_sample_rate().0 >= 48000 {
                cpal::SampleRate(48000)
            } else {
                cfg.max_sample_rate()
            };
            cfg.clone().with_sample_rate(sample_rate)
        } else {
            // Fall back to default config
            eprintln!("[CPAL] No suitable config found, using default");
            device.default_output_config()?
        };

        let sample_format = config.sample_format();

        eprintln!("[CPAL] Selected config:");
        eprintln!("  - Sample rate: {}", config.sample_rate().0);
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
                // Successfully processed audio
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
        // Debug: log callback invocation with per-stream counter
        // Log first 10 callbacks for each new stream
        if callback_count == 1 {
            eprintln!("[audio_callback_i32] *** FIRST CALLBACK FOR STREAM {:?} ***", stream_id);
            eprintln!("[audio_callback_i32]   Buffer size: {} samples ({} frames stereo)",
                data.len(), data.len() / 2);
        } else if callback_count <= 10 {
            eprintln!("[audio_callback_i32] Stream {:?} call #{}, buffer: {} samples",
                stream_id, callback_count, data.len());
        } else if callback_count == 11 {
            eprintln!("[audio_callback_i32] Stream {:?}: further callback logs suppressed", stream_id);
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
        match self.command_tx.try_send(command.clone()) {
            Ok(()) => Ok(()),
            Err(crossbeam_channel::TrySendError::Full(_)) => {
                eprintln!("[DesktopPlayback] Warning: Command channel full, dropping command: {:?}", command);
                // Return Ok to not fail the operation - the command is just dropped
                // This can happen when switching audio devices and callbacks aren't running yet
                Ok(())
            }
            Err(crossbeam_channel::TrySendError::Disconnected(_)) => {
                Err(crate::error::AudioError::PlaybackError(
                    "Command channel disconnected".into(),
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
        {
            let mut stream_guard = self.stream.lock().unwrap();
            if let Some(stream) = stream_guard.take() {
                // Stream will be dropped and audio will stop
                drop(stream);
                eprintln!("[DesktopPlayback] Old stream dropped");
            }
        }

        // Small delay to ensure old stream is fully stopped
        std::thread::sleep(std::time::Duration::from_millis(50));

        // Create new command channel for the new stream
        let (new_command_tx, new_command_rx) = bounded(32);

        // Update command_tx for this instance
        self.command_tx = new_command_tx.clone();

        // Create new stream with new device, reusing the same event_tx
        let (new_stream, actual_device_name) = Self::create_audio_stream(
            self.manager.clone(),
            new_command_rx,
            self.event_tx.clone(),
            backend,
            device_name.clone(),
        )?;

        eprintln!(
            "[DesktopPlayback] New stream created for device: {}",
            actual_device_name
        );

        // Store new stream
        {
            let mut stream_guard = self.stream.lock().unwrap();
            *stream_guard = Some(new_stream);
        }

        // Update current backend and device
        {
            *self.current_backend.lock().unwrap() = backend;
            *self.current_device.lock().unwrap() = actual_device_name.clone();
        }

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

        eprintln!("[DesktopPlayback] Device switch complete");
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
