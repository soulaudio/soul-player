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
        eprintln!(
            "[CallbackDropGuard] !!! {} stream {:?} callback closure is being DROPPED !!!",
            self.sample_format, self.stream_id
        );
        eprintln!(
            "[CallbackDropGuard] This means the ASIO/audio callback will no longer be called."
        );
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

    /// Track changed (emitted at correct time: 50% crossfade or immediately for gapless)
    TrackChanged(Option<QueueTrack>),

    /// Position updated (in seconds)
    PositionUpdated(f64),

    /// Volume changed
    VolumeChanged(u8),

    /// Queue updated
    QueueUpdated,

    /// Device sample rate changed (old_rate, new_rate)
    SampleRateChanged(u32, u32),

    /// Crossfade started between two tracks
    CrossfadeStarted {
        /// ID of the outgoing track
        from_track_id: String,
        /// ID of the incoming track
        to_track_id: String,
        /// Duration in milliseconds
        duration_ms: u32,
    },

    /// Crossfade progress update (for UI animations)
    CrossfadeProgress {
        /// Progress from 0.0 to 1.0
        progress: f32,
        /// Whether metadata has been switched (at 50%)
        metadata_switched: bool,
    },

    /// Crossfade completed
    CrossfadeCompleted,

    /// Error occurred
    Error(String),
}

/// Sample rate mode for playback
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SampleRateMode {
    /// Resample all audio to device's current sample rate (default)
    /// This is the most compatible mode - works with all devices
    #[default]
    MatchDevice,
    /// Switch device sample rate to match track's native rate when possible
    /// Requires exclusive mode for most audio APIs
    /// Falls back to MatchDevice if rate switching fails
    MatchTrack,
    /// No resampling - send audio at native rate (requires exclusive mode)
    /// Only works if device supports the track's sample rate
    Passthrough,
    /// Fixed output rate - always resample to this rate
    Fixed(u32),
}

impl SampleRateMode {
    /// Parse from string for settings persistence
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "match_device" | "device" | "auto" => Some(Self::MatchDevice),
            "match_track" | "track" => Some(Self::MatchTrack),
            "passthrough" | "native" | "bitperfect" => Some(Self::Passthrough),
            s if s.starts_with("fixed:") => {
                let rate_str = s.trim_start_matches("fixed:");
                rate_str.parse::<u32>().ok().map(Self::Fixed)
            }
            s => s.parse::<u32>().ok().map(Self::Fixed),
        }
    }

    /// Convert to string for settings persistence
    pub fn as_str(&self) -> String {
        match self {
            Self::MatchDevice => "match_device".to_string(),
            Self::MatchTrack => "match_track".to_string(),
            Self::Passthrough => "passthrough".to_string(),
            Self::Fixed(rate) => format!("fixed:{}", rate),
        }
    }

    /// Check if this mode requires exclusive device access
    pub fn requires_exclusive(&self) -> bool {
        matches!(self, Self::MatchTrack | Self::Passthrough)
    }

    /// Get the target sample rate for a given track and device
    ///
    /// # Arguments
    /// * `track_rate` - Native sample rate of the track
    /// * `device_rate` - Current sample rate of the device
    /// * `device_supported_rates` - Sample rates supported by the device
    ///
    /// # Returns
    /// Target sample rate for output, and whether resampling is needed
    pub fn resolve_rate(
        &self,
        track_rate: u32,
        device_rate: u32,
        device_supported_rates: Option<&[u32]>,
    ) -> (u32, bool) {
        match self {
            Self::MatchDevice => (device_rate, track_rate != device_rate),
            Self::MatchTrack => {
                // Try to use track's native rate if device supports it
                if let Some(rates) = device_supported_rates {
                    if rates.contains(&track_rate) {
                        return (track_rate, false);
                    }
                }
                // Fall back to device rate
                (device_rate, track_rate != device_rate)
            }
            Self::Passthrough => {
                // Send at native rate - assume device can handle it
                // (caller should verify device supports the rate)
                (track_rate, false)
            }
            Self::Fixed(target) => (*target, track_rate != *target),
        }
    }
}

/// Resampling settings for audio playback
///
/// These settings control how audio is converted between different sample rates.
/// Changes take effect when the next track is loaded.
#[derive(Debug, Clone)]
pub struct ResamplingSettings {
    /// Quality preset: "fast", "balanced", "high", "maximum"
    pub quality: String,
    /// Sample rate mode (replaces target_rate)
    pub sample_rate_mode: SampleRateMode,
    /// Target sample rate override. 0 = auto (use device rate)
    /// Deprecated: Use sample_rate_mode instead
    pub target_rate: u32,
    /// Backend: "auto", "rubato", "r8brain"
    pub backend: String,
}

impl Default for ResamplingSettings {
    fn default() -> Self {
        Self {
            quality: "high".to_string(),
            sample_rate_mode: SampleRateMode::MatchDevice,
            target_rate: 0, // deprecated, use sample_rate_mode
            backend: "auto".to_string(),
        }
    }
}

impl ResamplingSettings {
    /// Get the sinc filter length based on quality preset
    pub fn sinc_len(&self) -> usize {
        match self.quality.as_str() {
            "fast" => 64,
            "balanced" => 128,
            "high" => 256,
            "maximum" => 512,
            _ => 256, // default to high
        }
    }

    /// Get the cutoff frequency based on quality preset
    pub fn f_cutoff(&self) -> f32 {
        match self.quality.as_str() {
            "fast" => 0.90,
            "balanced" => 0.95,
            "high" => 0.99,
            "maximum" => 0.995,
            _ => 0.99, // default to high
        }
    }

    /// Get the oversampling factor based on quality preset
    pub fn oversampling_factor(&self) -> usize {
        match self.quality.as_str() {
            "fast" => 128,
            "balanced" => 256,
            "high" => 256,
            "maximum" => 512,
            _ => 256, // default to high
        }
    }
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

    /// Resampling settings (applied when loading tracks)
    resampling_settings: Arc<Mutex<ResamplingSettings>>,

    /// Background track loader (keeps disk I/O off audio thread)
    track_loader: Arc<crate::track_loader::TrackLoader>,
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

        // Create background track loader FIRST - keeps disk I/O off audio thread
        let track_loader = Arc::new(crate::track_loader::TrackLoader::new());

        // Create CPAL stream with specified device (passes track_loader to callbacks)
        let (stream, actual_device_name, sample_rate) = Self::create_audio_stream(
            manager.clone(),
            command_rx.clone(),
            event_tx.clone(),
            backend,
            device_name,
            track_loader.clone(),
        )?;

        let stream = Arc::new(Mutex::new(Some(stream)));
        let current_backend = Arc::new(Mutex::new(backend));
        let current_device = Arc::new(Mutex::new(actual_device_name));
        let current_sample_rate = Arc::new(std::sync::atomic::AtomicU32::new(sample_rate));
        let resampling_settings = Arc::new(Mutex::new(ResamplingSettings::default()));

        Ok(Self {
            command_tx,
            event_rx,
            event_tx,
            stream,
            manager,
            current_backend,
            current_device,
            current_sample_rate,
            resampling_settings,
            track_loader,
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
        track_loader: Arc<crate::track_loader::TrackLoader>,
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
                let track_loader_clone = track_loader.clone();
                // Per-stream callback counter for logging
                let mut callback_count: u32 = 0;
                let stream_id = std::time::Instant::now();
                eprintln!(
                    "[CPAL] Creating F32 stream callback (stream_id: {:?})",
                    stream_id
                );
                device.build_output_stream(
                    &config,
                    move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                        callback_count += 1;
                        Self::audio_callback_f32(
                            data,
                            manager_clone.clone(),
                            &command_rx,
                            &event_tx,
                            &track_loader_clone,
                            callback_count,
                            stream_id,
                        );
                    },
                    |err| eprintln!("[CPAL] Audio stream error callback: {}", err),
                    None,
                )?
            }
            cpal::SampleFormat::I32 => {
                let manager_clone = manager.clone();
                let track_loader_clone = track_loader.clone();
                // Pre-allocate conversion buffer to avoid allocation in audio callback
                // Use a reasonable default size that will be resized if needed
                let mut f32_buffer: Vec<f32> = Vec::with_capacity(4096);
                // Per-stream callback counter for logging
                let mut callback_count: u32 = 0;
                let stream_id = std::time::Instant::now();
                eprintln!(
                    "[CPAL] Creating I32 stream callback (stream_id: {:?})",
                    stream_id
                );

                // Clone event_tx for the error callback
                let error_event_tx = event_tx.clone();

                // Create drop guard to detect when callback is dropped
                let drop_guard = CallbackDropGuard {
                    stream_id,
                    sample_format: "I32",
                };

                // Create TPDF dither for high-quality F32→I32 conversion
                let mut dither = soul_audio::dither::StereoDither::new();

                device.build_output_stream(
                    &config,
                    move |data: &mut [i32], _: &cpal::OutputCallbackInfo| {
                        // Keep drop guard alive - when this closure is dropped, drop_guard is dropped
                        let _ = &drop_guard;
                        callback_count += 1;
                        Self::audio_callback_i32(
                            data,
                            manager_clone.clone(),
                            &command_rx,
                            &event_tx,
                            &track_loader_clone,
                            &mut f32_buffer,
                            &mut dither,
                            callback_count,
                            stream_id,
                        );
                    },
                    move |err| {
                        eprintln!("[CPAL] !!! AUDIO STREAM ERROR CALLBACK !!!");
                        eprintln!("[CPAL]   Error: {}", err);
                        eprintln!("[CPAL]   This may cause the stream to be dropped!");
                        let _ = error_event_tx
                            .try_send(PlaybackEvent::Error(format!("Stream error: {}", err)));
                    },
                    None,
                )?
            }
            cpal::SampleFormat::I16 => {
                let manager_clone = manager.clone();
                let track_loader_clone = track_loader.clone();
                // Pre-allocate conversion buffer to avoid allocation in audio callback
                let mut f32_buffer: Vec<f32> = Vec::with_capacity(4096);
                // Per-stream callback counter for logging
                let mut callback_count: u32 = 0;
                let stream_id = std::time::Instant::now();
                eprintln!(
                    "[CPAL] Creating I16 stream callback (stream_id: {:?})",
                    stream_id
                );
                // Create TPDF dither for high-quality F32→I16 conversion
                // This is especially important for 16-bit output where quantization is audible
                let mut dither = soul_audio::dither::StereoDither::new();

                device.build_output_stream(
                    &config,
                    move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                        callback_count += 1;
                        Self::audio_callback_i16(
                            data,
                            manager_clone.clone(),
                            &command_rx,
                            &event_tx,
                            &track_loader_clone,
                            &mut f32_buffer,
                            &mut dither,
                            callback_count,
                            stream_id,
                        );
                    },
                    |err| eprintln!("[CPAL] Audio stream error callback: {}", err),
                    None,
                )?
            }
            _ => {
                eprintln!(
                    "[CPAL] ERROR: Unsupported sample format: {:?}",
                    sample_format
                );
                return Err(crate::error::AudioError::DeviceError(format!(
                    "Unsupported sample format: {:?}",
                    sample_format
                ))
                .into());
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

        eprintln!(
            "[CPAL] Device's actual sample rate: {:?}",
            actual_sample_rate
        );
        eprintln!(
            "[CPAL] Device's default config: channels={}, format={:?}",
            default_config.channels(),
            default_config.sample_format()
        );

        // Also log supported configs for debugging
        eprintln!("[CPAL] Checking supported output configurations...");
        let supported_configs: Vec<_> = device
            .supported_output_configs()
            .map(|configs| configs.collect())
            .unwrap_or_default();

        for cfg in &supported_configs {
            eprintln!(
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
                c.min_sample_rate() <= actual_sample_rate
                    && c.max_sample_rate() >= actual_sample_rate
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
            cfg.clone().with_sample_rate(actual_sample_rate)
        } else {
            // Fall back to default config (which already has the actual sample rate)
            eprintln!("[CPAL] No matching config found, using default");
            default_config
        };

        let sample_format = config.sample_format();

        eprintln!("[CPAL] Selected config:");
        eprintln!(
            "  - Sample rate: {:?} (device's actual rate)",
            config.sample_rate()
        );
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
                eprintln!(
                    "[CPAL] Using fixed buffer size: {} frames (range: {}-{})",
                    buffer_size, min, max
                );
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

    /// Pre-load the next track for crossfade/gapless playback
    ///
    /// This function is called from audio callbacks to check if we should
    /// prepare the next track for crossfade. If we're approaching the
    /// crossfade region and don't have a next source loaded yet, we request
    /// loading via the background track loader.
    ///
    /// This is the key fix for crossfade not working - without pre-loading
    /// the next track's audio source, the crossfade engine has nothing to
    /// mix with the current track.
    fn prepare_next_track_if_needed(
        mgr: &mut PlaybackManager,
        track_loader: &Arc<crate::track_loader::TrackLoader>,
    ) {
        // Check if we should prepare (approaching crossfade region and no next source)
        if !mgr.should_prepare_next_track() {
            return;
        }

        // Get the next track from the queue without advancing
        let next_track = match mgr.peek_next_queue_track() {
            Some(track) => track.clone(),
            None => return, // No next track available
        };

        // Request loading the audio source for the next track (non-blocking)
        let target_sample_rate = mgr.get_sample_rate();
        let request = crate::track_loader::LoadRequest {
            path: std::path::PathBuf::from(&next_track.path),
            track: next_track.clone(),
            target_sample_rate,
            is_preload: true, // This is a preload for crossfade/gapless
        };

        if track_loader.request_load(request) {
            eprintln!(
                "[prepare_next_track] Requested preload for crossfade: {}",
                next_track.title
            );
            // Mark that we've requested a preload to avoid duplicate requests
            // The result will be handled by poll_track_loader
        }
        // If queue is full, we'll try again next callback
    }

    /// Load next track when track finishes (called from audio callbacks)
    ///
    /// This handles the case where `process_audio` detects track end and calls
    /// `handle_track_finished()` → `next()`, which sets state to Loading.
    /// We request loading via the background track loader (non-blocking).
    fn load_next_track(
        mgr: &mut PlaybackManager,
        track_loader: &Arc<crate::track_loader::TrackLoader>,
        event_tx: &Sender<PlaybackEvent>,
    ) {
        if let Some(track) = mgr.get_current_track().cloned() {
            let target_sample_rate = mgr.get_sample_rate();
            let request = crate::track_loader::LoadRequest {
                path: track.path.clone(),
                track: track.clone(),
                target_sample_rate,
                is_preload: false, // This is a current track load, not preload
            };

            if track_loader.request_load(request) {
                eprintln!(
                    "[load_next_track] Requested load for next track: {}",
                    track.title
                );
                // Result will be handled by poll_track_loader in next callback
            } else {
                eprintln!("[load_next_track] Load request queue full");
                // Queue full - emit error and stop
                let _ = event_tx.try_send(PlaybackEvent::Error(
                    "Track load queue full".to_string(),
                ));
                mgr.stop();
                let _ = event_tx.try_send(PlaybackEvent::StateChanged(mgr.get_state()));
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
        track_loader: &Arc<crate::track_loader::TrackLoader>,
        callback_count: u32,
        stream_id: std::time::Instant,
    ) {
        // Debug: log callback invocation with per-stream counter
        if callback_count == 1 {
            eprintln!(
                "[audio_callback_f32] *** FIRST CALLBACK FOR STREAM {:?} ***",
                stream_id
            );
            eprintln!(
                "[audio_callback_f32]   Buffer size: {} samples ({} frames stereo)",
                data.len(),
                data.len() / 2
            );
        } else if callback_count <= 5 {
            eprintln!(
                "[audio_callback_f32] Stream {:?} call #{}, buffer: {} samples",
                stream_id,
                callback_count,
                data.len()
            );
        } else if callback_count == 6 {
            eprintln!(
                "[audio_callback_f32] Stream {:?}: further callback logs suppressed",
                stream_id
            );
        }

        // Process any pending commands
        while let Ok(command) = command_rx.try_recv() {
            if let Err(e) = Self::process_command(command, manager.clone(), event_tx, track_loader) {
                let _ = event_tx.try_send(PlaybackEvent::Error(format!("Command error: {}", e)));
            }
        }

        // Get audio from playback manager
        let mut mgr = manager.lock().unwrap();

        // Poll for any ready track loads from the background loader (non-blocking)
        // This moves disk I/O results back to the audio thread without blocking
        Self::poll_track_loader(&mut mgr, track_loader, event_tx);

        // Check if we need to pre-load the next track for crossfade/gapless
        // This must happen BEFORE process_audio so the crossfade engine has
        // the next source ready when entering the crossfade region
        Self::prepare_next_track_if_needed(&mut mgr, track_loader);

        match mgr.process_audio(data) {
            Ok(_) => {
                // Forward any events from PlaybackManager (crossfade progress, track changes, etc.)
                Self::forward_manager_events(&mut mgr, event_tx);

                // Check if track finished and next track is ready to load
                if mgr.get_state() == soul_playback::PlaybackState::Loading {
                    Self::load_next_track(&mut mgr, track_loader, event_tx);
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
    /// Uses TPDF dithering for high-quality F32→I32 conversion.
    fn audio_callback_i32(
        data: &mut [i32],
        manager: Arc<Mutex<PlaybackManager>>,
        command_rx: &Receiver<PlaybackCommand>,
        event_tx: &Sender<PlaybackEvent>,
        track_loader: &Arc<crate::track_loader::TrackLoader>,
        f32_buffer: &mut Vec<f32>,
        dither: &mut soul_audio::dither::StereoDither,
        callback_count: u32,
        stream_id: std::time::Instant,
    ) {
        // Update global counter for diagnostics
        let global_count = GLOBAL_I32_CALLBACK_COUNTER.fetch_add(1, Ordering::Relaxed);

        // Debug: log callback invocation with per-stream counter
        // Log first 10 callbacks for each new stream
        if callback_count == 1 {
            eprintln!(
                "[audio_callback_i32] *** FIRST CALLBACK FOR STREAM {:?} (global #{}) ***",
                stream_id,
                global_count + 1
            );
            eprintln!(
                "[audio_callback_i32]   Thread ID: {:?}",
                std::thread::current().id()
            );
            eprintln!(
                "[audio_callback_i32]   Buffer size: {} samples ({} frames stereo)",
                data.len(),
                data.len() / 2
            );
        } else if callback_count <= 10 {
            eprintln!(
                "[audio_callback_i32] Stream {:?} call #{} (global #{}), buffer: {} samples",
                stream_id,
                callback_count,
                global_count + 1,
                data.len()
            );
        } else if callback_count == 11 {
            eprintln!(
                "[audio_callback_i32] Stream {:?}: further callback logs suppressed (global #{})",
                stream_id,
                global_count + 1
            );
        } else if callback_count % 1000 == 0 {
            // Log every 1000 callbacks to show the stream is still alive
            eprintln!(
                "[audio_callback_i32] Stream {:?} still alive: {} callbacks (global #{})",
                stream_id,
                callback_count,
                global_count + 1
            );
        }

        // Process any pending commands
        while let Ok(command) = command_rx.try_recv() {
            eprintln!("[audio_callback_i32] Received command: {:?}", command);
            if let Err(e) = Self::process_command(command, manager.clone(), event_tx, track_loader) {
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

        // Poll for any ready track loads from the background loader (non-blocking)
        // This moves disk I/O results back to the audio thread without blocking
        Self::poll_track_loader(&mut mgr, track_loader, event_tx);

        // Check if we need to pre-load the next track for crossfade/gapless
        // This must happen BEFORE process_audio so the crossfade engine has
        // the next source ready when entering the crossfade region
        Self::prepare_next_track_if_needed(&mut mgr, track_loader);

        match mgr.process_audio(f32_slice) {
            Ok(_) => {
                // Forward any events from PlaybackManager (crossfade progress, track changes, etc.)
                Self::forward_manager_events(&mut mgr, event_tx);

                // Check if track finished and next track is ready to load
                if mgr.get_state() == soul_playback::PlaybackState::Loading {
                    Self::load_next_track(&mut mgr, track_loader, event_tx);
                }

                // Convert f32 [-1.0, 1.0] to i32 with TPDF dithering
                // Dithering reduces quantization noise for higher quality audio
                dither.process_stereo_to_i32(f32_slice, data);
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
    /// Uses TPDF dithering for high-quality F32→I16 conversion.
    fn audio_callback_i16(
        data: &mut [i16],
        manager: Arc<Mutex<PlaybackManager>>,
        command_rx: &Receiver<PlaybackCommand>,
        event_tx: &Sender<PlaybackEvent>,
        track_loader: &Arc<crate::track_loader::TrackLoader>,
        f32_buffer: &mut Vec<f32>,
        dither: &mut soul_audio::dither::StereoDither,
        callback_count: u32,
        stream_id: std::time::Instant,
    ) {
        // Debug: log callback invocation with per-stream counter
        if callback_count == 1 {
            eprintln!(
                "[audio_callback_i16] *** FIRST CALLBACK FOR STREAM {:?} ***",
                stream_id
            );
            eprintln!(
                "[audio_callback_i16]   Buffer size: {} samples ({} frames stereo)",
                data.len(),
                data.len() / 2
            );
        } else if callback_count <= 5 {
            eprintln!(
                "[audio_callback_i16] Stream {:?} call #{}, buffer: {} samples",
                stream_id,
                callback_count,
                data.len()
            );
        } else if callback_count == 6 {
            eprintln!(
                "[audio_callback_i16] Stream {:?}: further callback logs suppressed",
                stream_id
            );
        }

        // Process any pending commands
        while let Ok(command) = command_rx.try_recv() {
            if let Err(e) = Self::process_command(command, manager.clone(), event_tx, track_loader) {
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

        // Poll for any ready track loads from the background loader (non-blocking)
        // This moves disk I/O results back to the audio thread without blocking
        Self::poll_track_loader(&mut mgr, track_loader, event_tx);

        // Check if we need to pre-load the next track for crossfade/gapless
        // This must happen BEFORE process_audio so the crossfade engine has
        // the next source ready when entering the crossfade region
        Self::prepare_next_track_if_needed(&mut mgr, track_loader);

        match mgr.process_audio(f32_slice) {
            Ok(_) => {
                // Forward any events from PlaybackManager (crossfade progress, track changes, etc.)
                Self::forward_manager_events(&mut mgr, event_tx);

                // Check if track finished and next track is ready to load
                if mgr.get_state() == soul_playback::PlaybackState::Loading {
                    Self::load_next_track(&mut mgr, track_loader, event_tx);
                }

                // Convert f32 [-1.0, 1.0] to i16 with TPDF dithering
                // Dithering is essential for 16-bit audio quality
                dither.process_stereo_to_i16(f32_slice, data);
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

    /// Forward events from PlaybackManager to the desktop event channel
    ///
    /// This drains events from the manager (e.g., crossfade progress, track changes at 50%)
    /// and converts them to desktop PlaybackEvent format.
    fn forward_manager_events(mgr: &mut PlaybackManager, event_tx: &Sender<PlaybackEvent>) {
        for event in mgr.drain_events() {
            let desktop_event = match event {
                soul_playback::PlaybackEvent::StateChanged { state } => {
                    // Convert PlaybackStateEvent to PlaybackState
                    let state = match state {
                        soul_playback::PlaybackStateEvent::Stopped => {
                            soul_playback::PlaybackState::Stopped
                        }
                        soul_playback::PlaybackStateEvent::Loading => {
                            soul_playback::PlaybackState::Loading
                        }
                        soul_playback::PlaybackStateEvent::Playing => {
                            soul_playback::PlaybackState::Playing
                        }
                        soul_playback::PlaybackStateEvent::Paused => {
                            soul_playback::PlaybackState::Paused
                        }
                        soul_playback::PlaybackStateEvent::Crossfading => {
                            // Map crossfading to Playing for UI compatibility
                            soul_playback::PlaybackState::Playing
                        }
                    };
                    Some(PlaybackEvent::StateChanged(state))
                }
                soul_playback::PlaybackEvent::TrackChanged {
                    track_id,
                    previous_track_id: _,
                } => {
                    // Get the full track info from the manager
                    // During crossfade at 50%, this is emitted with the NEW track ID
                    // Try to find the track in the queue or as current track
                    let track = if let Some(current) = mgr.get_current_track() {
                        if current.id == track_id {
                            Some(current.clone())
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                    Some(PlaybackEvent::TrackChanged(track))
                }
                soul_playback::PlaybackEvent::CrossfadeStarted {
                    from_track_id,
                    to_track_id,
                    duration_ms,
                } => Some(PlaybackEvent::CrossfadeStarted {
                    from_track_id,
                    to_track_id,
                    duration_ms,
                }),
                soul_playback::PlaybackEvent::CrossfadeProgress {
                    progress,
                    metadata_switched,
                } => Some(PlaybackEvent::CrossfadeProgress {
                    progress,
                    metadata_switched,
                }),
                soul_playback::PlaybackEvent::CrossfadeCompleted => {
                    Some(PlaybackEvent::CrossfadeCompleted)
                }
                soul_playback::PlaybackEvent::TrackFinished { track_id: _ } => {
                    // Already handled by track loading logic
                    None
                }
                soul_playback::PlaybackEvent::PositionUpdate {
                    position_ms,
                    duration_ms: _,
                } => Some(PlaybackEvent::PositionUpdated(position_ms as f64 / 1000.0)),
                soul_playback::PlaybackEvent::NextTrackPrepared { track_id: _ } => {
                    // Internal event, not needed for UI
                    None
                }
                soul_playback::PlaybackEvent::VolumeChanged { level, is_muted: _ } => {
                    Some(PlaybackEvent::VolumeChanged(level))
                }
                soul_playback::PlaybackEvent::QueueChanged { length: _ } => {
                    Some(PlaybackEvent::QueueUpdated)
                }
                soul_playback::PlaybackEvent::Error { message } => {
                    Some(PlaybackEvent::Error(message))
                }
            };

            if let Some(event) = desktop_event {
                let _ = event_tx.try_send(event);
            }
        }
    }

    /// Poll for ready track loads from the background loader (non-blocking)
    ///
    /// This is called from the audio callback to check if any track loads have completed.
    /// When a load completes, we set the source on the manager and emit events.
    fn poll_track_loader(
        mgr: &mut PlaybackManager,
        track_loader: &Arc<crate::track_loader::TrackLoader>,
        event_tx: &Sender<PlaybackEvent>,
    ) {
        while let Some(result) = track_loader.poll_ready() {
            if let Some(source) = result.source {
                if result.is_preload {
                    // Pre-loaded next track for crossfade/gapless
                    eprintln!(
                        "[poll_track_loader] Next track ready for crossfade: {}",
                        result.track.title
                    );
                    mgr.set_next_source(source, result.track);
                } else {
                    // Current track loaded (initial load or track change)
                    eprintln!(
                        "[poll_track_loader] Track loaded: {}",
                        result.track.title
                    );
                    mgr.set_audio_source(source);
                    let _ = event_tx.try_send(PlaybackEvent::StateChanged(mgr.get_state()));
                    let _ = event_tx.try_send(PlaybackEvent::TrackChanged(Some(result.track)));
                    let _ = event_tx.try_send(PlaybackEvent::QueueUpdated);
                }
            } else if let Some(error) = result.error {
                eprintln!(
                    "[poll_track_loader] Failed to load '{}': {}",
                    result.track.title, error
                );
                if !result.is_preload {
                    // Only emit error for current track loads, not preloads
                    let _ = event_tx.try_send(PlaybackEvent::Error(format!(
                        "Failed to load track: {}",
                        error
                    )));
                    mgr.stop();
                    let _ = event_tx.try_send(PlaybackEvent::StateChanged(mgr.get_state()));
                }
            }
        }
    }

    /// Process playback command
    fn process_command(
        command: PlaybackCommand,
        manager: Arc<Mutex<PlaybackManager>>,
        event_tx: &Sender<PlaybackEvent>,
        track_loader: &Arc<crate::track_loader::TrackLoader>,
    ) -> Result<()> {
        let mut mgr = manager.lock().unwrap();

        match command {
            PlaybackCommand::Play => {
                eprintln!("[PlaybackCommand::Play] Received");
                mgr.play()?;

                let state = mgr.get_state();
                eprintln!("[PlaybackCommand::Play] State after play(): {:?}", state);

                // If state is Loading, request track load via background loader
                if state == soul_playback::PlaybackState::Loading {
                    if let Some(track) = mgr.get_current_track().cloned() {
                        eprintln!(
                            "[PlaybackCommand::Play] Requesting track load: {} from {}",
                            track.title,
                            track.path.display()
                        );
                        let target_sample_rate = mgr.get_sample_rate();
                        eprintln!(
                            "[PlaybackCommand::Play] Target sample rate: {}",
                            target_sample_rate
                        );
                        // Request load via background loader (non-blocking)
                        let request = crate::track_loader::LoadRequest {
                            path: track.path.clone(),
                            track: track.clone(),
                            target_sample_rate,
                            is_preload: false,
                        };
                        if !track_loader.request_load(request) {
                            eprintln!("[PlaybackCommand::Play] Load request queue full");
                        }
                        // Result will be handled by poll_track_loader in next callback
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

                // If state is Loading, request track load via background loader
                if mgr.get_state() == soul_playback::PlaybackState::Loading {
                    if let Some(track) = mgr.get_current_track().cloned() {
                        let target_sample_rate = mgr.get_sample_rate();
                        let request = crate::track_loader::LoadRequest {
                            path: track.path.clone(),
                            track: track.clone(),
                            target_sample_rate,
                            is_preload: false,
                        };
                        if !track_loader.request_load(request) {
                            eprintln!("[PlaybackCommand::Next] Load request queue full");
                        }
                        // Result will be handled by poll_track_loader in next callback
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

                // If state is Loading, request track load via background loader
                if mgr.get_state() == soul_playback::PlaybackState::Loading {
                    if let Some(track) = mgr.get_current_track().cloned() {
                        let target_sample_rate = mgr.get_sample_rate();
                        let request = crate::track_loader::LoadRequest {
                            path: track.path.clone(),
                            track: track.clone(),
                            target_sample_rate,
                            is_preload: false,
                        };
                        if !track_loader.request_load(request) {
                            eprintln!("[PlaybackCommand::Previous] Load request queue full");
                        }
                        // Result will be handled by poll_track_loader in next callback
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

                // If state is Loading, request track load via background loader
                if mgr.get_state() == soul_playback::PlaybackState::Loading {
                    if let Some(track) = mgr.get_current_track().cloned() {
                        let target_sample_rate = mgr.get_sample_rate();
                        let request = crate::track_loader::LoadRequest {
                            path: track.path.clone(),
                            track: track.clone(),
                            target_sample_rate,
                            is_preload: false,
                        };
                        if !track_loader.request_load(request) {
                            eprintln!("[PlaybackCommand::SkipToQueueIndex] Load request queue full");
                        }
                        // Result will be handled by poll_track_loader in next callback
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

        eprintln!(
            "[DesktopPlayback] Stream alive: {}, Channel: len={}, cap={}, empty={}, full={}",
            stream_alive, channel_len, channel_capacity, channel_is_empty, channel_is_full
        );

        // Get current backend for context
        let backend = *self.current_backend.lock().unwrap();
        let device = self.current_device.lock().unwrap().clone();
        eprintln!(
            "[DesktopPlayback] Current backend: {:?}, device: {}",
            backend, device
        );

        match self.command_tx.try_send(command.clone()) {
            Ok(()) => {
                eprintln!("[DesktopPlayback] Command sent successfully");
                Ok(())
            }
            Err(crossbeam_channel::TrySendError::Full(_)) => {
                eprintln!(
                    "[DesktopPlayback] WARNING: Command channel FULL, dropping command: {:?}",
                    command
                );
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
                eprintln!(
                    "[DesktopPlayback] Global I32 callback count: {}",
                    global_count
                );

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
        eprintln!(
            "[DesktopPlayback]   Thread ID: {:?}",
            std::thread::current().id()
        );
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
                    eprintln!(
                        "[DesktopPlayback] Warning: Failed to pause old stream: {}",
                        e
                    );
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
        eprintln!(
            "[DesktopPlayback] command_tx updated, channel capacity: {}",
            self.command_tx.capacity().unwrap_or(0)
        );

        // Create new stream with new device, reusing the same event_tx
        let (new_stream, actual_device_name, new_sample_rate) = Self::create_audio_stream(
            self.manager.clone(),
            new_command_rx,
            self.event_tx.clone(),
            backend,
            device_name.clone(),
            self.track_loader.clone(),
        )?;

        // Check if sample rate changed
        let old_sample_rate = self.current_sample_rate.load(Ordering::SeqCst);
        if old_sample_rate != new_sample_rate {
            eprintln!(
                "[DesktopPlayback] Sample rate changed: {} Hz -> {} Hz",
                old_sample_rate, new_sample_rate
            );
            self.current_sample_rate
                .store(new_sample_rate, Ordering::SeqCst);
            let _ = self.event_tx.try_send(PlaybackEvent::SampleRateChanged(
                old_sample_rate,
                new_sample_rate,
            ));
        }

        eprintln!(
            "[DesktopPlayback] New stream created for device: {} at {} Hz",
            actual_device_name, new_sample_rate
        );

        // Check callbacks before storing
        let callbacks_before_store = GLOBAL_I32_CALLBACK_COUNTER.load(Ordering::Relaxed);
        eprintln!(
            "[DesktopPlayback] Callbacks before storing stream: {}",
            callbacks_before_store
        );

        // Store new stream
        eprintln!("[DesktopPlayback] Storing new stream...");
        {
            let mut stream_guard = self.stream.lock().unwrap();
            *stream_guard = Some(new_stream);
        }

        // Check callbacks immediately after storing
        let callbacks_after_store = GLOBAL_I32_CALLBACK_COUNTER.load(Ordering::Relaxed);
        eprintln!(
            "[DesktopPlayback] Stream stored. Callbacks: {} (diff: {})",
            callbacks_after_store,
            callbacks_after_store - callbacks_before_store
        );
        eprintln!("[DesktopPlayback] Waiting 100ms for callbacks to start...");

        // Give the new stream a moment to start callbacks
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Check callbacks after sleep
        let callbacks_after_sleep = GLOBAL_I32_CALLBACK_COUNTER.load(Ordering::Relaxed);
        eprintln!(
            "[DesktopPlayback] After 100ms sleep. Callbacks: {} (diff: {})",
            callbacks_after_sleep,
            callbacks_after_sleep - callbacks_after_store
        );

        // Verify channel status after stream creation
        let channel_len = self.command_tx.len();
        let channel_cap = self.command_tx.capacity().unwrap_or(0);
        let callbacks_so_far = GLOBAL_I32_CALLBACK_COUNTER.load(Ordering::Relaxed);
        eprintln!("[DesktopPlayback] Channel verification after stream creation:");
        eprintln!("[DesktopPlayback]   Queue length: {}", channel_len);
        eprintln!("[DesktopPlayback]   Capacity: {}", channel_cap);
        eprintln!(
            "[DesktopPlayback]   Global I32 callbacks: {}",
            callbacks_so_far
        );

        // Check callbacks before updating backend
        let callbacks_before_backend = GLOBAL_I32_CALLBACK_COUNTER.load(Ordering::Relaxed);
        eprintln!(
            "[DesktopPlayback] Callbacks before backend update: {}",
            callbacks_before_backend
        );

        // Update current backend and device
        {
            *self.current_backend.lock().unwrap() = backend;
            *self.current_device.lock().unwrap() = actual_device_name.clone();
        }

        // Check callbacks after updating backend
        let callbacks_after_backend = GLOBAL_I32_CALLBACK_COUNTER.load(Ordering::Relaxed);
        eprintln!("[DesktopPlayback] Backend and device name updated");
        eprintln!(
            "[DesktopPlayback] Callbacks after backend update: {} (diff: {})",
            callbacks_after_backend,
            callbacks_after_backend - callbacks_before_backend
        );

        // Reload the audio source with the new sample rate
        // This is necessary because the old audio source was created with the old device's sample rate
        let current_track = {
            let mgr = self.manager.lock().unwrap();
            mgr.get_current_track().cloned()
        };

        if let Some(track) = current_track {
            eprintln!("[DesktopPlayback] Reloading audio source for new sample rate");

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
                    eprintln!("[DesktopPlayback] Failed to reload audio source: {}", e);
                }
            }
        }

        // Restore position if we had one
        if position > std::time::Duration::ZERO {
            let mut mgr = self.manager.lock().unwrap();
            if let Err(e) = mgr.seek_to(position) {
                eprintln!("[DesktopPlayback] Failed to restore position: {}", e);
            } else {
                eprintln!("[DesktopPlayback] Position restored to {:?}", position);
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
        eprintln!(
            "[DesktopPlayback] Emitting StateChanged after device switch: {:?}",
            current_state
        );
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
        eprintln!(
            "[DesktopPlayback] Device switch complete. Final callback count: {}",
            callbacks_at_end
        );
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
                eprintln!(
                    "[DesktopPlayback] Failed to query device sample rate: {}",
                    e
                );
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

    // ===== Exclusive Mode / Bit-Perfect Output =====

    /// Get current latency information
    ///
    /// Returns buffer size, latency in milliseconds, and exclusive mode status.
    pub fn get_latency_info(&self) -> crate::LatencyInfo {
        // Get current buffer size from stream config
        // This is an estimate based on typical buffer sizes
        let sample_rate = self.current_sample_rate.load(Ordering::SeqCst);
        let buffer_samples = 512u32; // Default estimate

        let buffer_ms = if sample_rate > 0 {
            buffer_samples as f32 / sample_rate as f32 * 1000.0
        } else {
            11.6 // ~512 samples at 44100
        };

        crate::LatencyInfo {
            buffer_samples,
            buffer_ms,
            total_ms: buffer_ms + 5.0, // Add DAC latency estimate
            exclusive: false,          // Currently not tracking exclusive mode state
        }
    }

    /// Enable exclusive mode with configuration
    ///
    /// Switches to exclusive mode for bit-perfect playback:
    /// - WASAPI exclusive mode on Windows (bypasses OS mixer)
    /// - ASIO is inherently exclusive
    /// - Direct sample format output (no conversion)
    ///
    /// # Arguments
    /// * `config` - Exclusive mode configuration (sample rate, bit depth, buffer size)
    ///
    /// # Returns
    /// * `Ok(LatencyInfo)` - Latency info after switching to exclusive mode
    /// * `Err(_)` - Failed to enable exclusive mode
    pub fn set_exclusive_mode(
        &mut self,
        config: crate::ExclusiveConfig,
    ) -> Result<crate::LatencyInfo> {
        eprintln!(
            "[DesktopPlayback] Setting exclusive mode with config: {:?}",
            config
        );

        // For now, switch to the configured device/backend
        // Full exclusive mode implementation would require WASAPI-specific code
        let device_name = config.device_name.clone();
        self.switch_device(config.backend, device_name)?;

        // Calculate latency based on config
        let sample_rate = self.current_sample_rate.load(Ordering::SeqCst);
        let buffer_samples = config.buffer_frames.unwrap_or(256);
        let buffer_ms = buffer_samples as f32 / sample_rate as f32 * 1000.0;

        Ok(crate::LatencyInfo {
            buffer_samples,
            buffer_ms,
            total_ms: buffer_ms + 5.0,
            exclusive: config.exclusive_mode,
        })
    }

    /// Disable exclusive mode (return to shared mode)
    ///
    /// Switches back to the default shared mode output.
    pub fn disable_exclusive_mode(&mut self) -> Result<()> {
        eprintln!("[DesktopPlayback] Disabling exclusive mode");

        // Switch back to default device with default backend
        self.switch_device(crate::AudioBackend::Default, None)?;

        Ok(())
    }

    /// Check if currently in exclusive mode
    pub fn is_exclusive_mode(&self) -> bool {
        // ASIO is always exclusive mode
        let backend = *self.current_backend.lock().unwrap();
        match backend {
            #[cfg(all(target_os = "windows", feature = "asio"))]
            crate::AudioBackend::Asio => true,
            _ => false, // Default/WASAPI shared mode
        }
    }

    // ===== Crossfade Settings =====

    /// Set crossfade enabled/disabled
    ///
    /// When enabled, tracks will blend into each other during transitions.
    /// When disabled, gapless playback is used.
    pub fn set_crossfade_enabled(&self, enabled: bool) {
        let mut manager = self.manager.lock().unwrap();
        manager.set_crossfade_enabled(enabled);
    }

    /// Get current crossfade enabled state
    pub fn is_crossfade_enabled(&self) -> bool {
        let manager = self.manager.lock().unwrap();
        manager.is_crossfade_enabled()
    }

    /// Set crossfade duration in milliseconds
    ///
    /// Duration is capped at 10000ms (10 seconds).
    /// A duration of 0 means gapless playback (no crossfade).
    pub fn set_crossfade_duration(&self, duration_ms: u32) {
        let mut manager = self.manager.lock().unwrap();
        manager.set_crossfade_duration(duration_ms);
    }

    /// Get crossfade duration in milliseconds
    pub fn get_crossfade_duration(&self) -> u32 {
        let manager = self.manager.lock().unwrap();
        manager.get_crossfade_duration()
    }

    /// Set crossfade curve type
    ///
    /// See `FadeCurve` for available curve types:
    /// - Linear: Simple linear fade
    /// - SquareRoot: Natural-sounding transitions
    /// - SCurve: Smooth acceleration at start/end
    /// - EqualPower: Constant perceived loudness (recommended)
    pub fn set_crossfade_curve(&self, curve: soul_playback::FadeCurve) {
        let mut manager = self.manager.lock().unwrap();
        manager.set_crossfade_curve(curve);
    }

    /// Get crossfade curve type
    pub fn get_crossfade_curve(&self) -> soul_playback::FadeCurve {
        let manager = self.manager.lock().unwrap();
        manager.get_crossfade_curve()
    }

    /// Set whether crossfade should trigger on manual skip
    ///
    /// When true, crossfade will also be used when the user manually
    /// skips to the next track (not just auto-advance).
    pub fn set_crossfade_on_skip(&self, on_skip: bool) {
        let mut manager = self.manager.lock().unwrap();
        manager.set_crossfade_on_skip(on_skip);
    }

    // ===========================================================================
    // Resampling Settings
    // ===========================================================================

    /// Set resampling quality preset
    ///
    /// Quality presets control the filter parameters used during sample rate conversion:
    /// - "fast": 64-tap filter, 0.90 cutoff - low CPU usage
    /// - "balanced": 128-tap filter, 0.95 cutoff - good quality
    /// - "high": 256-tap filter, 0.99 cutoff - excellent quality (default)
    /// - "maximum": 512-tap filter, 0.995 cutoff - audiophile quality
    ///
    /// Note: Changes take effect when the next track is loaded. The current track
    /// continues playing with its existing resampler settings.
    pub fn set_resampling_quality(&mut self, quality: &str) -> std::result::Result<(), String> {
        let valid_qualities = ["fast", "balanced", "high", "maximum"];
        if !valid_qualities.contains(&quality) {
            return Err(format!(
                "Invalid quality '{}'. Must be one of: {}",
                quality,
                valid_qualities.join(", ")
            ));
        }

        let mut settings = self.resampling_settings.lock().unwrap();
        settings.quality = quality.to_string();
        eprintln!("[DesktopPlayback] Resampling quality set to '{}' (sinc_len={}, f_cutoff={})",
            quality, settings.sinc_len(), settings.f_cutoff());
        Ok(())
    }

    /// Get current resampling quality preset
    pub fn get_resampling_quality(&self) -> String {
        let settings = self.resampling_settings.lock().unwrap();
        settings.quality.clone()
    }

    /// Set resampling target sample rate
    ///
    /// - rate=0: Auto mode - match device native sample rate (default)
    /// - rate>0: Force specific output sample rate (e.g., 96000)
    ///
    /// Note: Changes take effect when the next track is loaded.
    pub fn set_resampling_target_rate(&mut self, rate: u32) -> std::result::Result<(), String> {
        if rate != 0 && (rate < 8000 || rate > 384000) {
            return Err(format!(
                "Invalid target rate {}. Must be 0 (auto) or between 8000 and 384000 Hz",
                rate
            ));
        }

        let mut settings = self.resampling_settings.lock().unwrap();
        settings.target_rate = rate;
        eprintln!("[DesktopPlayback] Resampling target rate set to {} (0=auto)", rate);
        Ok(())
    }

    /// Get current resampling target sample rate
    ///
    /// Returns 0 for auto mode (match device rate), or the specific rate in Hz.
    pub fn get_resampling_target_rate(&self) -> u32 {
        let settings = self.resampling_settings.lock().unwrap();
        settings.target_rate
    }

    /// Set resampling backend
    ///
    /// Backends:
    /// - "auto": Use best available (r8brain if compiled in, else rubato)
    /// - "rubato": Use Rubato library (always available)
    /// - "r8brain": Use r8brain library (requires r8brain feature flag)
    ///
    /// Note: Changes take effect when the next track is loaded.
    pub fn set_resampling_backend(&mut self, backend: &str) -> std::result::Result<(), String> {
        let valid_backends = ["auto", "rubato", "r8brain"];
        if !valid_backends.contains(&backend) {
            return Err(format!(
                "Invalid backend '{}'. Must be one of: {}",
                backend,
                valid_backends.join(", ")
            ));
        }

        // Check r8brain availability
        if backend == "r8brain" {
            #[cfg(not(feature = "r8brain"))]
            {
                return Err("r8brain backend is not available in this build. \
                    Use 'auto' or 'rubato' instead.".to_string());
            }
        }

        let mut settings = self.resampling_settings.lock().unwrap();
        settings.backend = backend.to_string();
        eprintln!("[DesktopPlayback] Resampling backend set to '{}'", backend);
        Ok(())
    }

    /// Get current resampling backend
    pub fn get_resampling_backend(&self) -> String {
        let settings = self.resampling_settings.lock().unwrap();
        settings.backend.clone()
    }

    /// Get current resampling settings (clone)
    ///
    /// Returns a copy of the current resampling settings for use when creating
    /// audio sources.
    pub fn get_resampling_settings(&self) -> ResamplingSettings {
        let settings = self.resampling_settings.lock().unwrap();
        settings.clone()
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
                assert_eq!(playback.get_current_backend(), crate::AudioBackend::Default);
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
                let switch_result = playback.switch_device(crate::AudioBackend::Default, None);

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
                    assert_eq!(playback.get_current_backend(), crate::AudioBackend::Default);
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
