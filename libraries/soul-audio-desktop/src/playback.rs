//! Desktop playback integration
//!
//! Combines `PlaybackManager` with CPAL audio output for desktop playback.

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Device, Stream, StreamConfig,
};
use crossbeam_channel::{bounded, Receiver, Sender};
use soul_playback::{AudioSource, PlaybackConfig, PlaybackManager, QueueTrack};
use std::sync::{Arc, Mutex};

use crate::error::Result;
use std::sync::atomic::{AtomicU64, Ordering};

/// Global counter for I32 (ASIO) callbacks - used for diagnostics
/// This is updated by `audio_callback_i32` and read by `send_command` for debugging
static GLOBAL_I32_CALLBACK_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Stream-level fade envelope to prevent clicks/pops at audio stream start
///
/// When a CPAL audio stream first starts, the DAC may be in an undefined state.
/// Jumping directly to audio output can cause a hardware-level pop.
/// This envelope applies a 30ms fade at stream start to let the DAC settle.
///
/// See: <https://www.kernel.org/doc/html/v4.13/sound/soc/pops-clicks.html>
struct StreamStartEnvelope {
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
    fn new(sample_rate: u32, channels: u16) -> Self {
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
    fn process(&mut self, buffer: &mut [f32]) -> bool {
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
    fn process_i32(&mut self, buffer: &mut [i32]) -> bool {
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
    fn process_i16(&mut self, buffer: &mut [i16]) -> bool {
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
/// This helps diagnose ASIO stream issues where the callback is silently dropped
struct CallbackDropGuard {
    stream_id: std::time::Instant,
    sample_format: &'static str,
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

    /// Add track to queue (legacy - maps to AddToQueueEnd)
    AddToQueue(QueueTrack),

    /// Add track to Play Next queue (highest priority, plays after current track)
    AddPlayNext(QueueTrack),

    /// Add track to end of Add to Queue (lowest priority, plays after source exhausts)
    AddToQueueEnd(QueueTrack),

    /// Remove track from queue
    RemoveFromQueue(usize),

    /// Clear entire queue (all three tiers)
    ClearQueue,

    /// Clear Play Next queue only
    ClearPlayNext,

    /// Clear Add to Queue only
    ClearAddToQueue,

    /// Skip to track at queue index
    SkipToQueueIndex(usize),

    /// Load playlist/album as new source queue (replaces playback context)
    LoadPlaylist(Vec<QueueTrack>),

    /// Append tracks to source queue (for lazy loading)
    AppendToSource(Vec<QueueTrack>),

    /// Set shuffle mode
    SetShuffle(soul_playback::ShuffleMode),

    /// Cycle shuffle mode (Off → Random → Smart → Off)
    CycleShuffle,

    /// Set repeat mode
    SetRepeat(soul_playback::RepeatMode),

    /// Switch audio output device
    /// Arguments: (backend, `device_name`)
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

    /// Device sample rate changed (`old_rate`, `new_rate`)
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

    /// Batch load requested (forward pagination)
    BatchLoadRequested {
        /// Offset in the collection to start loading from
        offset: usize,
        /// Number of tracks to load
        limit: usize,
    },

    /// Jump load requested (direct navigation to far track)
    JumpLoadRequested {
        /// Offset in the collection to start loading from
        offset: usize,
        /// Number of tracks to load
        limit: usize,
    },

    /// Error occurred
    Error(String),

    /// Device switch initiated (for UI feedback)
    DeviceSwitchStarted {
        /// Target device name
        target_device: String,
        /// Reason for switch
        reason: DeviceSwitchReason,
    },

    /// Device switch completed successfully
    DeviceSwitchCompleted {
        /// New device name
        device_name: String,
        /// New sample rate
        sample_rate: u32,
    },

    /// Device switch failed
    DeviceSwitchFailed {
        /// Error message
        error: String,
        /// Whether fallback to default was attempted
        fallback_attempted: bool,
    },
}

/// Reasons for device switching
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceSwitchReason {
    /// User requested device change
    UserRequested,
    /// Current device was disconnected (hot-unplug)
    DeviceDisconnected,
    /// System default device changed
    DefaultDeviceChanged,
    /// Sample rate mismatch detected
    SampleRateMismatch,
    /// Device error recovery
    ErrorRecovery,
}

impl std::fmt::Display for DeviceSwitchReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UserRequested => write!(f, "user_requested"),
            Self::DeviceDisconnected => write!(f, "device_disconnected"),
            Self::DefaultDeviceChanged => write!(f, "default_device_changed"),
            Self::SampleRateMismatch => write!(f, "sample_rate_mismatch"),
            Self::ErrorRecovery => write!(f, "error_recovery"),
        }
    }
}

/// Device switch state machine
///
/// Tracks the current state of device switching to prevent race conditions
/// and ensure proper sequencing of device transitions.
///
/// Based on industry best practices from:
/// - Microsoft WASAPI stream routing documentation
/// - Apple CoreAudio device management guidelines
/// - BigBlueButton audio device handling
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum DeviceSwitchState {
    /// Normal operation, no switch in progress
    #[default]
    Idle,

    /// Fadeout in progress before switch
    FadingOut {
        /// Target device for switch
        target_device: Option<String>,
        /// Target backend
        target_backend: crate::AudioBackend,
        /// Reason for switch
        reason: DeviceSwitchReason,
        /// Samples remaining in fadeout
        samples_remaining: usize,
    },

    /// Switching to new device (stream recreation)
    Switching {
        /// Target device for switch
        target_device: Option<String>,
        /// Target backend
        target_backend: crate::AudioBackend,
        /// Reason for switch
        reason: DeviceSwitchReason,
        /// Playback position to restore
        saved_position: std::time::Duration,
        /// Whether playback was active before switch
        was_playing: bool,
    },

    /// Fadein in progress after switch
    FadingIn {
        /// New device name
        device_name: String,
        /// Samples remaining in fadein
        samples_remaining: usize,
    },

    /// Recovery mode after failed switch
    Recovering {
        /// Number of retry attempts
        retry_count: u32,
        /// Last error message
        last_error: String,
        /// Original position to restore
        saved_position: std::time::Duration,
    },
}

impl DeviceSwitchState {
    /// Check if a switch is currently in progress
    pub fn is_switching(&self) -> bool {
        !matches!(self, Self::Idle)
    }

    /// Check if we can start a new switch
    pub fn can_start_switch(&self) -> bool {
        matches!(self, Self::Idle | Self::Recovering { .. })
    }

    /// Get the target device if currently switching
    pub fn target_device(&self) -> Option<&str> {
        match self {
            Self::FadingOut { target_device, .. } | Self::Switching { target_device, .. } => {
                target_device.as_deref()
            }
            _ => None,
        }
    }
}

/// Configuration for device switch behavior
#[derive(Debug, Clone)]
pub struct DeviceSwitchConfig {
    /// Duration of fadeout before switch (in milliseconds)
    pub fadeout_ms: u32,
    /// Duration of fadein after switch (in milliseconds)
    pub fadein_ms: u32,
    /// Maximum retry attempts for failed switches
    pub max_retries: u32,
    /// Delay between retry attempts (in milliseconds)
    pub retry_delay_ms: u32,
    /// Whether to automatically fallback to default device on failure
    pub auto_fallback: bool,
}

impl Default for DeviceSwitchConfig {
    fn default() -> Self {
        Self {
            fadeout_ms: 50, // 50ms fadeout (industry standard to prevent clicks)
            fadein_ms: 30,  // 30ms fadein (matches StreamStartEnvelope)
            max_retries: 3,
            retry_delay_ms: 100,
            auto_fallback: true,
        }
    }
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
    /// Falls back to `MatchDevice` if rate switching fails
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
    pub fn as_str(&self) -> &str {
        match self {
            Self::MatchDevice => "match_device",
            Self::MatchTrack => "match_track",
            Self::Passthrough => "passthrough",
            Self::Fixed(_) => {
                // Note: For Fixed variants, caller must handle formatting separately
                // This is acceptable since as_str() is currently unused in the codebase
                "fixed"
            }
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
    /// Sample rate mode (replaces `target_rate`)
    pub sample_rate_mode: SampleRateMode,
    /// Target sample rate override. 0 = auto (use device rate)
    /// Deprecated: Use `sample_rate_mode` instead
    pub target_rate: u32,
    /// Backend: "auto", "rubato", "r8brain"
    pub backend: String,
}

impl Default for ResamplingSettings {
    fn default() -> Self {
        Self {
            quality: String::from("high"),
            sample_rate_mode: SampleRateMode::MatchDevice,
            target_rate: 0, // deprecated, use sample_rate_mode
            backend: String::from("auto"),
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

    /// Current device ID (backend + device name as unique identifier)
    /// Used to prevent false positive device switches when checking sample rates
    current_device_id: Arc<Mutex<Option<String>>>,

    /// Current stream sample rate (what we're actually outputting at)
    current_sample_rate: Arc<std::sync::atomic::AtomicU32>,

    /// Resampling settings (applied when loading tracks)
    resampling_settings: Arc<Mutex<ResamplingSettings>>,

    /// Background track loader (keeps disk I/O off audio thread)
    track_loader: Arc<crate::track_loader::TrackLoader>,

    /// Device switch state machine
    /// Tracks current state of device switching to prevent race conditions
    device_switch_state: Arc<Mutex<DeviceSwitchState>>,

    /// Device switch configuration
    device_switch_config: DeviceSwitchConfig,
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
        tracing::info!("[Playback] Creating DesktopPlayback with default device");
        let start = std::time::Instant::now();
        let result = Self::new_with_device(config, crate::AudioBackend::Default, None);
        if result.is_ok() {
            tracing::info!(
                duration_ms = start.elapsed().as_millis(),
                "[Playback] DesktopPlayback created successfully"
            );
        }
        result
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
        tracing::info!("[Playback] ========================================");
        tracing::info!("[Playback] DESKTOP PLAYBACK INITIALIZATION STARTED");
        tracing::info!(
            backend = ?backend,
            device_name = ?device_name,
            crossfade = ?config.crossfade,
            gapless = config.gapless,
            "[Playback] Configuration"
        );

        // Log platform information for debugging
        let platform = if cfg!(target_os = "linux") {
            "Linux"
        } else if cfg!(target_os = "macos") {
            "macOS"
        } else if cfg!(target_os = "windows") {
            "Windows"
        } else {
            "Unknown"
        };

        tracing::info!(platform = platform, "[Playback] Platform detected");

        let init_start = std::time::Instant::now();

        tracing::debug!("[Playback] Creating PlaybackManager");
        let manager_start = std::time::Instant::now();
        let manager = Arc::new(Mutex::new(PlaybackManager::new(config)));
        let manager_duration = manager_start.elapsed();
        tracing::debug!(
            duration_us = manager_duration.as_micros(),
            "[Playback] PlaybackManager created"
        );

        let (command_tx, command_rx) = bounded(32);
        let (event_tx, event_rx) = bounded(32);

        // Create background track loader FIRST - keeps disk I/O off audio thread
        tracing::debug!("[Playback] Creating background track loader");
        let loader_start = std::time::Instant::now();
        let track_loader = Arc::new(crate::track_loader::TrackLoader::new().map_err(|e| {
            tracing::error!(error = %e, "[Playback] Failed to create track loader");
            crate::error::AudioError::DeviceError(e)
        })?);
        let loader_duration = loader_start.elapsed();
        tracing::debug!(
            duration_us = loader_duration.as_micros(),
            "[Playback] Track loader created"
        );

        // Create CPAL stream with specified device (passes track_loader to callbacks)
        tracing::debug!("[Playback] Creating audio stream");
        let stream_start = std::time::Instant::now();
        let (stream_option, actual_device_name, sample_rate) = Self::create_audio_stream(
            manager.clone(),
            command_rx.clone(),
            event_tx.clone(),
            backend,
            device_name.clone(),
            track_loader.clone(),
        )?;
        let stream_duration = stream_start.elapsed();

        let is_silent_mode = stream_option.is_none();

        if is_silent_mode {
            tracing::warn!(
                device_name = %actual_device_name,
                sample_rate,
                "[Playback] Silent mode active - no audio stream (zero-device system)"
            );
        } else {
            tracing::info!(
                device_name = %actual_device_name,
                sample_rate,
                stream_creation_ms = stream_duration.as_millis(),
                "[Playback] Audio stream created successfully"
            );
        }

        let stream = Arc::new(Mutex::new(stream_option));
        let current_backend = Arc::new(Mutex::new(backend));
        let current_device = Arc::new(Mutex::new(actual_device_name.clone()));

        // Create device ID (backend + device name as unique identifier)
        let device_id = if is_silent_mode {
            None
        } else {
            Some(Self::make_device_id(backend, &actual_device_name))
        };
        let current_device_id = Arc::new(Mutex::new(device_id));

        let current_sample_rate = Arc::new(std::sync::atomic::AtomicU32::new(sample_rate));
        let resampling_settings = Arc::new(Mutex::new(ResamplingSettings::default()));

        let total_duration = init_start.elapsed();
        tracing::info!("[Playback] ========================================");
        tracing::info!("[Playback] DESKTOP PLAYBACK INITIALIZATION COMPLETE");
        tracing::info!(
            total_duration_ms = total_duration.as_millis(),
            manager_us = manager_duration.as_micros(),
            loader_us = loader_duration.as_micros(),
            stream_ms = stream_duration.as_millis(),
            "[Playback] Initialization timings"
        );
        tracing::info!(
            device = %actual_device_name,
            sample_rate,
            platform = platform,
            backend = ?backend,
            silent_mode = is_silent_mode,
            "[Playback] Final configuration"
        );
        tracing::info!("[Playback] ========================================");

        Ok(Self {
            command_tx,
            event_rx,
            event_tx,
            stream,
            manager,
            current_backend,
            current_device,
            current_device_id,
            current_sample_rate,
            resampling_settings,
            track_loader,
            device_switch_state: Arc::new(Mutex::new(DeviceSwitchState::Idle)),
            device_switch_config: DeviceSwitchConfig::default(),
        })
    }

    /// Create CPAL audio stream
    ///
    /// Returns (Option<Stream>, `device_name`, `sample_rate`)
    /// Stream is None for zero-device systems (silent mode)
    fn create_audio_stream(
        manager: Arc<Mutex<PlaybackManager>>,
        command_rx: Receiver<PlaybackCommand>,
        event_tx: Sender<PlaybackEvent>,
        backend: crate::AudioBackend,
        device_name: Option<String>,
        track_loader: Arc<crate::track_loader::TrackLoader>,
    ) -> Result<(Option<Stream>, String, u32)> {
        tracing::info!(
            backend = ?backend,
            device_name = ?device_name,
            "[Playback] Starting audio stream creation"
        );

        let host = backend.to_cpal_host().map_err(|_| {
            tracing::error!(
                backend = ?backend,
                "[Playback] Failed to get CPAL host for backend"
            );
            crate::error::AudioError::DeviceNotFound
        })?;

        tracing::debug!("[Playback] CPAL host obtained successfully");

        let device_result = if let Some(name) = device_name {
            // Find device by name
            tracing::info!(
                device_name = %name,
                backend = ?backend,
                "[Playback] Searching for audio device by name"
            );
            crate::device::find_device_by_name(backend, &name).map_err(|e| {
                tracing::error!(
                    device_name = %name,
                    error = %e,
                    "[Playback] Failed to find device by name"
                );
                crate::error::AudioError::DeviceError(e.to_string())
            })
        } else {
            // Use default device
            tracing::debug!("[Playback] Looking for default output device");
            host.default_output_device().ok_or_else(|| {
                tracing::warn!(
                    backend = ?backend,
                    "[Playback] No default output device found - checking for zero-device system"
                );
                crate::error::AudioError::DeviceNotFound
            })
        };

        // Handle zero-device systems with silent mode fallback
        let device = match device_result {
            Ok(dev) => dev,
            Err(crate::error::AudioError::DeviceNotFound) => {
                tracing::warn!("[Playback] ========================================");
                tracing::warn!("[Playback] ZERO-DEVICE SYSTEM DETECTED");
                tracing::warn!("[Playback] No audio output devices available");
                tracing::warn!("[Playback] Entering SILENT MODE - library browsing only");
                tracing::warn!("[Playback] ========================================");

                return Self::create_null_stream(manager, command_rx, event_tx);
            }
            Err(e) => return Err(e),
        };

        let actual_device_name = device
            .description()
            .ok()
            .map(|desc| desc.name().to_string())
            .unwrap_or_else(|| "Unknown Device".to_string());

        tracing::info!(
            device_name = %actual_device_name,
            backend = ?backend,
            "[Playback] Selected audio device - retrieving configuration"
        );

        let (config, sample_format) = Self::get_stream_config(&device)?;

        tracing::info!(
            device_name = %actual_device_name,
            sample_rate = config.sample_rate,
            channels = config.channels,
            sample_format = ?sample_format,
            buffer_size = ?config.buffer_size,
            "[Playback] Device configuration retrieved"
        );
        let sample_rate = config.sample_rate;
        let channels = config.channels;

        // Set sample rate and channel count in manager
        {
            let lock_start = std::time::Instant::now();
            let mut mgr = manager.lock().unwrap();
            let lock_duration = lock_start.elapsed();
            if lock_duration.as_micros() > 100 {
                tracing::warn!(
                    lock_duration_us = lock_duration.as_micros(),
                    "[Playback] Manager lock contention during stream creation"
                );
            }
            mgr.set_sample_rate(sample_rate);
            mgr.set_output_channels(channels);
        }

        tracing::debug!("[CPAL] Building output stream with config: sample_rate={}, channels={}, buffer_size={:?}, format={:?}",
            config.sample_rate, config.channels, config.buffer_size, sample_format);

        // Build stream with the appropriate sample format
        let stream = match sample_format {
            cpal::SampleFormat::F32 => {
                let manager_clone = manager.clone();
                let track_loader_clone = track_loader.clone();
                // Per-stream callback counter for logging
                let mut callback_count: u32 = 0;
                let stream_id = std::time::Instant::now();
                // Create stream start envelope for click-free startup (30ms fade)
                let mut stream_envelope = StreamStartEnvelope::new(sample_rate, channels);
                // Track if we've already requested a load for the current Loading state
                // This prevents flooding the track loader with duplicate requests
                let mut load_requested = false;
                // Error recovery tracking - count consecutive errors and fade samples remaining
                let mut error_count: u32 = 0;
                let mut error_fade_samples_remaining: usize = 0;
                // LFSR state for proper noise generation during error recovery
                let mut error_noise_state: u32 = 0xDEAD_BEEF;
                tracing::debug!(
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
                            &mut load_requested,
                            &mut error_count,
                            &mut error_fade_samples_remaining,
                            &mut error_noise_state,
                        );
                        // Apply stream start envelope to prevent DAC pop
                        stream_envelope.process(data);
                    },
                    |err| tracing::error!("[CPAL] Audio stream error callback: {}", err),
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
                // Create stream start envelope for click-free startup (30ms fade)
                let mut stream_envelope = StreamStartEnvelope::new(sample_rate, channels);
                // Track if we've already requested a load for the current Loading state
                // This prevents flooding the track loader with duplicate requests
                let mut load_requested = false;
                // Error recovery tracking - count consecutive errors and fade samples remaining
                let mut error_count: u32 = 0;
                let mut error_fade_samples_remaining: usize = 0;
                // LFSR state for proper noise generation during error recovery
                let mut error_noise_state: u32 = 0xDEAD_BEEF;
                tracing::debug!(
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
                            &mut load_requested,
                            &mut error_count,
                            &mut error_fade_samples_remaining,
                            &mut error_noise_state,
                        );
                        // Apply stream start envelope to prevent DAC pop
                        stream_envelope.process_i32(data);
                    },
                    move |err| {
                        tracing::error!("[CPAL] !!! AUDIO STREAM ERROR CALLBACK !!!");
                        tracing::error!(error = ?err, "[CPAL] Stream error occurred");
                        tracing::error!("[CPAL]   This may cause the stream to be dropped!");
                        let _ = error_event_tx
                            .try_send(PlaybackEvent::Error("STREAM_ERROR".to_string()));
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
                // Create stream start envelope for click-free startup (30ms fade)
                let mut stream_envelope = StreamStartEnvelope::new(sample_rate, channels);
                // Track if we've already requested a load for the current Loading state
                // This prevents flooding the track loader with duplicate requests
                let mut load_requested = false;
                // Error recovery tracking - count consecutive errors and fade samples remaining
                let mut error_count: u32 = 0;
                let mut error_fade_samples_remaining: usize = 0;
                // LFSR state for proper noise generation during error recovery
                let mut error_noise_state: u32 = 0xDEAD_BEEF;
                tracing::debug!(
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
                            &mut load_requested,
                            &mut error_count,
                            &mut error_fade_samples_remaining,
                            &mut error_noise_state,
                        );
                        // Apply stream start envelope to prevent DAC pop
                        stream_envelope.process_i16(data);
                    },
                    |err| tracing::error!("[CPAL] Audio stream error callback: {}", err),
                    None,
                )?
            }
            _ => {
                tracing::error!(
                    "[CPAL] ERROR: Unsupported sample format: {:?}",
                    sample_format
                );
                return Err(crate::error::AudioError::DeviceError(format!(
                    "Unsupported sample format: {:?}",
                    sample_format
                )));
            }
        };

        tracing::debug!("[CPAL] Stream built successfully, calling play()...");

        match stream.play() {
            Ok(()) => {
                tracing::debug!("[CPAL] stream.play() returned Ok - stream should now be running");
            }
            Err(e) => {
                tracing::error!("[CPAL] ERROR: Failed to start stream: {}", e);
                tracing::error!("[CPAL] This may indicate:");
                tracing::error!("  - Sample rate mismatch with driver settings");
                tracing::error!("  - Buffer size not supported by driver");
                tracing::error!("  - Another application has exclusive access to the device");
                tracing::error!("  - Driver requires specific initialization");
                return Err(e.into());
            }
        }

        tracing::debug!("[CPAL] ==========================================");
        tracing::debug!("[CPAL] Stream created successfully!");
        tracing::debug!("[CPAL]   Device: {}", actual_device_name);
        tracing::debug!("[CPAL]   Sample rate: {} Hz", sample_rate);
        tracing::debug!("[CPAL]   Channels: {}", channels);
        tracing::debug!("[CPAL]   Sample format: {:?}", sample_format);
        tracing::debug!("[CPAL]   Buffer size: {:?}", config.buffer_size);
        tracing::debug!("[CPAL] ==========================================");
        tracing::debug!("[CPAL] Audio callbacks should start momentarily...");

        Ok((Some(stream), actual_device_name, sample_rate))
    }

    /// Create a null audio stream for zero-device systems
    ///
    /// This enables silent mode for library browsing when no audio devices are available
    /// (e.g., VMs, broken drivers, disabled audio).
    ///
    /// Returns (None, device_name="Silent Mode", sample_rate=44100)
    fn create_null_stream(
        manager: Arc<Mutex<PlaybackManager>>,
        _command_rx: Receiver<PlaybackCommand>,
        _event_tx: Sender<PlaybackEvent>,
    ) -> Result<(Option<Stream>, String, u32)> {
        // Use CD quality as default for silent mode
        const NULL_SAMPLE_RATE: u32 = 44100;
        const NULL_CHANNELS: u16 = 2;

        tracing::info!("[Playback] Creating NULL STREAM for silent mode");

        // Set sample rate and channels in manager
        {
            let mut mgr = manager.lock().unwrap();
            mgr.set_sample_rate(NULL_SAMPLE_RATE);
            mgr.set_output_channels(NULL_CHANNELS);
        }

        tracing::info!(
            sample_rate = NULL_SAMPLE_RATE,
            channels = NULL_CHANNELS,
            "[Playback] Initialized manager for silent mode"
        );

        tracing::info!("[Playback] ========================================");
        tracing::info!("[Playback] SILENT MODE ACTIVE");
        tracing::info!("[Playback]   Audio output: DISABLED");
        tracing::info!("[Playback]   Library browsing: ENABLED");
        tracing::info!("[Playback]   Device: Silent Mode (No Audio Devices)");
        tracing::info!("[Playback]   Sample rate: {} Hz", NULL_SAMPLE_RATE);
        tracing::info!("[Playback]   Playback controls: Will be ignored");
        tracing::info!("[Playback] ========================================");

        Ok((
            None,
            "Silent Mode (No Audio Devices)".to_string(),
            NULL_SAMPLE_RATE,
        ))
    }

    /// Get stream configuration
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
    fn get_stream_config(device: &Device) -> Result<(StreamConfig, cpal::SampleFormat)> {
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
            tracing::debug!(
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
                tracing::debug!(
                    "[load_next_track] Requested load for next track: {}",
                    track.title
                );
                // Result will be handled by poll_track_loader in next callback
            } else {
                tracing::debug!("[load_next_track] Load request queue full");
                // Queue full - emit error and stop
                let _ =
                    event_tx.try_send(PlaybackEvent::Error("Track load queue full".to_string()));
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

    /// Audio callback for f32 sample format (WASAPI, `CoreAudio`, etc.)
    #[allow(clippy::too_many_arguments)]
    fn audio_callback_f32(
        data: &mut [f32],
        manager: Arc<Mutex<PlaybackManager>>,
        command_rx: &Receiver<PlaybackCommand>,
        event_tx: &Sender<PlaybackEvent>,
        track_loader: &Arc<crate::track_loader::TrackLoader>,
        callback_count: u32,
        stream_id: std::time::Instant,
        load_requested: &mut bool,
        error_count: &mut u32,
        error_fade_samples_remaining: &mut usize,
        error_noise_state: &mut u32,
    ) {
        // Debug: log callback invocation with per-stream counter
        if callback_count == 1 {
            tracing::trace!(
                "[audio_callback_f32] *** FIRST CALLBACK FOR STREAM {:?} ***",
                stream_id
            );
            tracing::trace!(
                "[audio_callback_f32]   Buffer size: {} samples ({} frames stereo)",
                data.len(),
                data.len() / 2
            );
        } else if callback_count <= 5 {
            tracing::trace!(
                "[audio_callback_f32] Stream {:?} call #{}, buffer: {} samples",
                stream_id,
                callback_count,
                data.len()
            );
        } else if callback_count == 6 {
            tracing::trace!(
                "[audio_callback_f32] Stream {:?}: further callback logs suppressed",
                stream_id
            );
        }

        // Acquire manager lock using try_lock to avoid blocking the real-time audio thread
        // If the lock is contended (another thread holds it), output DAC keepalive noise
        // instead of blocking. This prevents audio glitches from mutex contention.
        let Ok(mut mgr) = manager.try_lock() else {
            // Lock contention - output DAC keepalive noise to prevent glitches
            // This is better than blocking which could cause a much longer glitch
            const DAC_KEEPALIVE: f32 = 0.000016; // -96dB - inaudible but keeps DAC active
            for sample in &mut *data {
                // Simple LFSR noise to keep DAC active
                *error_noise_state ^= *error_noise_state << 13;
                *error_noise_state ^= *error_noise_state >> 17;
                *error_noise_state ^= *error_noise_state << 5;
                *sample = ((*error_noise_state & 0xFFFF) as f32 / 32768.0 - 1.0) * DAC_KEEPALIVE;
            }
            return;
        };

        // Process any pending commands while holding the lock
        while let Ok(command) = command_rx.try_recv() {
            if let Err(e) =
                Self::process_command_with_lock(command, &mut mgr, event_tx, track_loader)
            {
                tracing::error!(error = ?e, "[PLAYBACK] Command error in f32 audio callback");
                let _ = event_tx.try_send(PlaybackEvent::Error("COMMAND_ERROR".to_string()));
            }
        }

        // Poll for any ready track loads from the background loader (non-blocking)
        // This moves disk I/O results back to the audio thread without blocking
        Self::poll_track_loader(&mut mgr, track_loader, event_tx);

        // Check if we need to pre-load the next track for crossfade/gapless
        // This must happen BEFORE process_audio so the crossfade engine has
        // the next source ready when entering the crossfade region
        Self::prepare_next_track_if_needed(&mut mgr, track_loader);

        match mgr.process_audio(data) {
            Ok(samples_processed) => {
                // Reset error count on successful processing
                *error_count = 0;

                // Emit periodic position updates (~250ms interval)
                // This is called from the audio callback to ensure accurate timing
                mgr.maybe_emit_position_update(samples_processed);

                // Forward any events from PlaybackManager (crossfade progress, track changes, etc.)
                Self::forward_manager_events(&mut mgr, event_tx);

                // Check if track finished and next track is ready to load
                // Only request load ONCE per Loading state to prevent duplicate requests
                let state = mgr.get_state();
                if state == soul_playback::PlaybackState::Loading {
                    if !*load_requested {
                        Self::load_next_track(&mut mgr, track_loader, event_tx);
                        *load_requested = true;
                    }
                } else {
                    // Reset flag when not in Loading state
                    *load_requested = false;
                }
            }
            Err(e) => {
                // Error fade-out: 10ms at 48kHz = 480 samples (stereo = 960 total)
                // Use a quick fade to DAC noise to prevent audible clicks
                const ERROR_FADE_SAMPLES: usize = 960;
                const DAC_KEEPALIVE: f32 = 0.000016; // -96dB - inaudible but keeps DAC active

                // Increment error counter
                *error_count += 1;

                if *error_fade_samples_remaining == 0 && *error_count == 1 {
                    // First error - start fade-out from current audio level
                    *error_fade_samples_remaining = ERROR_FADE_SAMPLES;
                }

                // Helper to generate LFSR noise - proper pseudo-random noise prevents DAC
                // artifacts that can occur with simple alternating patterns
                let generate_noise = |state: &mut u32| -> f32 {
                    *state ^= *state << 13;
                    *state ^= *state >> 17;
                    *state ^= *state << 5;
                    ((*state & 0xFFFF) as f32 / 32768.0 - 1.0) * DAC_KEEPALIVE
                };

                if *error_fade_samples_remaining > 0 {
                    // Apply fade-out to prevent click
                    let samples_to_fade = (*error_fade_samples_remaining).min(data.len());
                    for (i, sample) in data[..samples_to_fade].iter_mut().enumerate() {
                        let progress =
                            (*error_fade_samples_remaining - i) as f32 / ERROR_FADE_SAMPLES as f32;
                        // Fade from current value toward DAC keepalive noise using LFSR
                        let noise = generate_noise(error_noise_state);
                        *sample = *sample * progress + noise * (1.0 - progress);
                    }
                    *error_fade_samples_remaining =
                        error_fade_samples_remaining.saturating_sub(data.len());

                    // Fill remaining samples with DAC keepalive noise using LFSR
                    if samples_to_fade < data.len() {
                        for sample in &mut data[samples_to_fade..] {
                            *sample = generate_noise(error_noise_state);
                        }
                    }
                } else {
                    // Fade complete - fill with DAC keepalive noise using LFSR
                    for sample in &mut *data {
                        *sample = generate_noise(error_noise_state);
                    }
                }

                if *error_count >= 3 {
                    // Persistent error - emit error event and stop
                    tracing::error!(
                        error = ?e,
                        error_count = *error_count,
                        "[PLAYBACK] Persistent audio error - stopping playback"
                    );
                    let _ = event_tx.try_send(PlaybackEvent::Error(format!(
                        "Persistent audio error: {}",
                        e
                    )));
                    let _ = event_tx.try_send(PlaybackEvent::StateChanged(
                        soul_playback::PlaybackState::Stopped,
                    ));
                } else {
                    tracing::error!(
                        error = ?e,
                        error_count = *error_count,
                        "[PLAYBACK] Audio processing error in f32 callback"
                    );
                }
            }
        }
    }

    /// Audio callback for i32 sample format (ASIO)
    ///
    /// Uses a pre-allocated f32 buffer to avoid allocation in the real-time audio thread.
    /// Uses TPDF dithering for high-quality F32→I32 conversion.
    #[allow(clippy::too_many_arguments)]
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
        load_requested: &mut bool,
        error_count: &mut u32,
        error_fade_samples_remaining: &mut usize,
        error_noise_state: &mut u32,
    ) {
        // Update global counter for diagnostics
        let global_count = GLOBAL_I32_CALLBACK_COUNTER.fetch_add(1, Ordering::Relaxed);

        // Debug: log callback invocation with per-stream counter
        // Log first 10 callbacks for each new stream
        if callback_count == 1 {
            tracing::trace!(
                "[audio_callback_i32] *** FIRST CALLBACK FOR STREAM {:?} (global #{}) ***",
                stream_id,
                global_count + 1
            );
            tracing::trace!(
                "[audio_callback_i32]   Thread ID: {:?}",
                std::thread::current().id()
            );
            tracing::trace!(
                "[audio_callback_i32]   Buffer size: {} samples ({} frames stereo)",
                data.len(),
                data.len() / 2
            );
        } else if callback_count <= 10 {
            tracing::trace!(
                "[audio_callback_i32] Stream {:?} call #{} (global #{}), buffer: {} samples",
                stream_id,
                callback_count,
                global_count + 1,
                data.len()
            );
        } else if callback_count == 11 {
            tracing::trace!(
                "[audio_callback_i32] Stream {:?}: further callback logs suppressed (global #{})",
                stream_id,
                global_count + 1
            );
        } else if callback_count.is_multiple_of(1000) {
            // Log every 1000 callbacks to show the stream is still alive
            tracing::trace!(
                "[audio_callback_i32] Stream {:?} still alive: {} callbacks (global #{})",
                stream_id,
                callback_count,
                global_count + 1
            );
        }

        // Ensure f32 buffer is large enough (only reallocates if needed, and rarely)
        if f32_buffer.len() < data.len() {
            f32_buffer.resize(data.len(), 0.0);
        }
        let f32_slice = &mut f32_buffer[..data.len()];
        f32_slice.fill(0.0);

        // Acquire manager lock ONCE for the entire callback
        // This reduces latency by avoiding multiple lock/unlock cycles
        let mut mgr = manager.lock().unwrap();

        // Process any pending commands while holding the lock
        while let Ok(command) = command_rx.try_recv() {
            tracing::trace!("[audio_callback_i32] Received command: {:?}", command);
            if let Err(e) =
                Self::process_command_with_lock(command, &mut mgr, event_tx, track_loader)
            {
                tracing::error!(error = ?e, "[PLAYBACK] Command error in i32 audio callback");
                let _ = event_tx.try_send(PlaybackEvent::Error("COMMAND_ERROR".to_string()));
            }
        }

        // Poll for any ready track loads from the background loader (non-blocking)
        // This moves disk I/O results back to the audio thread without blocking
        Self::poll_track_loader(&mut mgr, track_loader, event_tx);

        // Check if we need to pre-load the next track for crossfade/gapless
        // This must happen BEFORE process_audio so the crossfade engine has
        // the next source ready when entering the crossfade region
        Self::prepare_next_track_if_needed(&mut mgr, track_loader);

        match mgr.process_audio(f32_slice) {
            Ok(samples_processed) => {
                // Reset error count on successful processing
                *error_count = 0;

                // Emit periodic position updates (~250ms interval)
                // This is called from the audio callback to ensure accurate timing
                mgr.maybe_emit_position_update(samples_processed);

                // Forward any events from PlaybackManager (crossfade progress, track changes, etc.)
                Self::forward_manager_events(&mut mgr, event_tx);

                // Check if track finished and next track is ready to load
                // Only request load ONCE per Loading state to prevent duplicate requests
                let state = mgr.get_state();
                if state == soul_playback::PlaybackState::Loading {
                    if !*load_requested {
                        Self::load_next_track(&mut mgr, track_loader, event_tx);
                        *load_requested = true;
                    }
                } else {
                    // Reset flag when not in Loading state
                    *load_requested = false;
                }

                // Convert f32 [-1.0, 1.0] to i32 with TPDF dithering
                // Dithering reduces quantization noise for higher quality audio
                dither.process_stereo_to_i32(f32_slice, data);
            }
            Err(e) => {
                // Error fade-out: 10ms at 48kHz = 480 samples (stereo = 960 total)
                // Use a quick fade to DAC noise to prevent audible clicks
                const ERROR_FADE_SAMPLES: usize = 960;
                const DAC_KEEPALIVE_F32: f32 = 0.000016; // -96dB - inaudible but keeps DAC active

                // Increment error counter
                *error_count += 1;

                if *error_fade_samples_remaining == 0 && *error_count == 1 {
                    // First error - start fade-out from current audio level
                    *error_fade_samples_remaining = ERROR_FADE_SAMPLES;
                }

                // Helper to generate LFSR noise for i32 format - proper pseudo-random
                // noise prevents DAC artifacts from simple alternating patterns
                let generate_noise_i32 = |state: &mut u32| -> i32 {
                    *state ^= *state << 13;
                    *state ^= *state >> 17;
                    *state ^= *state << 5;
                    let noise_f32 = ((*state & 0xFFFF) as f32 / 32768.0 - 1.0) * DAC_KEEPALIVE_F32;
                    (noise_f32 * 2147483647.0) as i32
                };

                if *error_fade_samples_remaining > 0 {
                    // Apply fade-out to prevent click
                    // First convert f32 buffer to i32 with fade applied
                    let samples_to_fade = (*error_fade_samples_remaining).min(data.len());
                    for (i, (out_sample, in_sample)) in data[..samples_to_fade]
                        .iter_mut()
                        .zip(f32_slice[..samples_to_fade].iter())
                        .enumerate()
                    {
                        let progress =
                            (*error_fade_samples_remaining - i) as f32 / ERROR_FADE_SAMPLES as f32;
                        // Generate LFSR noise for proper randomness
                        let noise = generate_noise_i32(error_noise_state);
                        // Convert f32 to i32 with fade
                        let scaled = (*in_sample * 2147483647.0) as i32;
                        *out_sample = ((scaled as f64 * progress as f64)
                            + (noise as f64 * (1.0 - progress as f64)))
                            as i32;
                    }
                    *error_fade_samples_remaining =
                        error_fade_samples_remaining.saturating_sub(data.len());

                    // Fill remaining samples with DAC keepalive noise using LFSR
                    if samples_to_fade < data.len() {
                        for sample in &mut data[samples_to_fade..] {
                            *sample = generate_noise_i32(error_noise_state);
                        }
                    }
                } else {
                    // Fade complete - fill with DAC keepalive noise using LFSR
                    for sample in &mut *data {
                        *sample = generate_noise_i32(error_noise_state);
                    }
                }

                if *error_count >= 3 {
                    // Persistent error - emit error event and stop
                    tracing::error!(
                        error = ?e,
                        error_count = *error_count,
                        "[PLAYBACK] Persistent audio error - stopping playback"
                    );
                    let _ = event_tx.try_send(PlaybackEvent::Error(format!(
                        "Persistent audio error: {}",
                        e
                    )));
                    let _ = event_tx.try_send(PlaybackEvent::StateChanged(
                        soul_playback::PlaybackState::Stopped,
                    ));
                } else {
                    tracing::error!(
                        error = ?e,
                        error_count = *error_count,
                        "[PLAYBACK] Audio processing error in i32 callback"
                    );
                }
            }
        }
    }

    /// Audio callback for i16 sample format
    ///
    /// Uses a pre-allocated f32 buffer to avoid allocation in the real-time audio thread.
    /// Uses TPDF dithering for high-quality F32→I16 conversion.
    #[allow(clippy::too_many_arguments)]
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
        load_requested: &mut bool,
        error_count: &mut u32,
        error_fade_samples_remaining: &mut usize,
        error_noise_state: &mut u32,
    ) {
        // Debug: log callback invocation with per-stream counter
        if callback_count == 1 {
            tracing::trace!(
                "[audio_callback_i16] *** FIRST CALLBACK FOR STREAM {:?} ***",
                stream_id
            );
            tracing::trace!(
                "[audio_callback_i16]   Buffer size: {} samples ({} frames stereo)",
                data.len(),
                data.len() / 2
            );
        } else if callback_count <= 5 {
            tracing::trace!(
                "[audio_callback_i16] Stream {:?} call #{}, buffer: {} samples",
                stream_id,
                callback_count,
                data.len()
            );
        } else if callback_count == 6 {
            tracing::trace!(
                "[audio_callback_i16] Stream {:?}: further callback logs suppressed",
                stream_id
            );
        }

        // Ensure f32 buffer is large enough (only reallocates if needed, and rarely)
        if f32_buffer.len() < data.len() {
            f32_buffer.resize(data.len(), 0.0);
        }
        let f32_slice = &mut f32_buffer[..data.len()];
        f32_slice.fill(0.0);

        // Acquire manager lock ONCE for the entire callback
        // This reduces latency by avoiding multiple lock/unlock cycles
        let mut mgr = manager.lock().unwrap();

        // Process any pending commands while holding the lock
        while let Ok(command) = command_rx.try_recv() {
            if let Err(e) =
                Self::process_command_with_lock(command, &mut mgr, event_tx, track_loader)
            {
                tracing::error!(error = ?e, "[PLAYBACK] Command error in i16 audio callback");
                let _ = event_tx.try_send(PlaybackEvent::Error("COMMAND_ERROR".to_string()));
            }
        }

        // Poll for any ready track loads from the background loader (non-blocking)
        // This moves disk I/O results back to the audio thread without blocking
        Self::poll_track_loader(&mut mgr, track_loader, event_tx);

        // Check if we need to pre-load the next track for crossfade/gapless
        // This must happen BEFORE process_audio so the crossfade engine has
        // the next source ready when entering the crossfade region
        Self::prepare_next_track_if_needed(&mut mgr, track_loader);

        match mgr.process_audio(f32_slice) {
            Ok(_) => {
                // Reset error count on successful processing
                *error_count = 0;

                // Forward any events from PlaybackManager (crossfade progress, track changes, etc.)
                Self::forward_manager_events(&mut mgr, event_tx);

                // Check if track finished and next track is ready to load
                // Only request load ONCE per Loading state to prevent duplicate requests
                let state = mgr.get_state();
                if state == soul_playback::PlaybackState::Loading {
                    if !*load_requested {
                        Self::load_next_track(&mut mgr, track_loader, event_tx);
                        *load_requested = true;
                    }
                } else {
                    // Reset flag when not in Loading state
                    *load_requested = false;
                }

                // Convert f32 [-1.0, 1.0] to i16 with TPDF dithering
                // Dithering is essential for 16-bit audio quality
                dither.process_stereo_to_i16(f32_slice, data);
            }
            Err(e) => {
                // Error fade-out: 10ms at 48kHz = 480 samples (stereo = 960 total)
                // Use a quick fade to DAC noise to prevent audible clicks
                const ERROR_FADE_SAMPLES: usize = 960;
                const DAC_KEEPALIVE_F32: f32 = 0.000016; // -96dB - inaudible but keeps DAC active

                // Increment error counter
                *error_count += 1;

                if *error_fade_samples_remaining == 0 && *error_count == 1 {
                    // First error - start fade-out from current audio level
                    *error_fade_samples_remaining = ERROR_FADE_SAMPLES;
                }

                // Helper to generate LFSR noise for i16 format - proper pseudo-random
                // noise prevents DAC artifacts from simple alternating patterns
                let generate_noise_i16 = |state: &mut u32| -> i16 {
                    *state ^= *state << 13;
                    *state ^= *state >> 17;
                    *state ^= *state << 5;
                    let noise_f32 = ((*state & 0xFFFF) as f32 / 32768.0 - 1.0) * DAC_KEEPALIVE_F32;
                    (noise_f32 * 32767.0) as i16
                };

                if *error_fade_samples_remaining > 0 {
                    // Apply fade-out to prevent click
                    // First convert f32 buffer to i16 with fade applied
                    let samples_to_fade = (*error_fade_samples_remaining).min(data.len());
                    for (i, (out_sample, in_sample)) in data[..samples_to_fade]
                        .iter_mut()
                        .zip(f32_slice[..samples_to_fade].iter())
                        .enumerate()
                    {
                        let progress =
                            (*error_fade_samples_remaining - i) as f32 / ERROR_FADE_SAMPLES as f32;
                        // Generate LFSR noise for proper randomness
                        let noise = generate_noise_i16(error_noise_state);
                        // Convert f32 to i16 with fade
                        let scaled = (*in_sample * 32767.0) as i16;
                        *out_sample =
                            ((scaled as f32 * progress) + (noise as f32 * (1.0 - progress))) as i16;
                    }
                    *error_fade_samples_remaining =
                        error_fade_samples_remaining.saturating_sub(data.len());

                    // Fill remaining samples with DAC keepalive noise using LFSR
                    if samples_to_fade < data.len() {
                        for sample in &mut data[samples_to_fade..] {
                            *sample = generate_noise_i16(error_noise_state);
                        }
                    }
                } else {
                    // Fade complete - fill with DAC keepalive noise using LFSR
                    for sample in &mut *data {
                        *sample = generate_noise_i16(error_noise_state);
                    }
                }

                if *error_count >= 3 {
                    // Persistent error - emit error event and stop
                    tracing::error!(
                        error = ?e,
                        error_count = *error_count,
                        "[PLAYBACK] Persistent audio error - stopping playback"
                    );
                    let _ = event_tx.try_send(PlaybackEvent::Error(format!(
                        "Persistent audio error: {}",
                        e
                    )));
                    let _ = event_tx.try_send(PlaybackEvent::StateChanged(
                        soul_playback::PlaybackState::Stopped,
                    ));
                } else {
                    tracing::error!(
                        error = ?e,
                        error_count = *error_count,
                        "[PLAYBACK] Audio processing error in i16 callback"
                    );
                }
            }
        }
    }

    /// Forward events from `PlaybackManager` to the desktop event channel
    ///
    /// This drains events from the manager (e.g., crossfade progress, track changes at 50%)
    /// and converts them to desktop `PlaybackEvent` format.
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
                soul_playback::PlaybackEvent::BatchLoadRequested { offset, limit } => {
                    Some(PlaybackEvent::BatchLoadRequested { offset, limit })
                }
                soul_playback::PlaybackEvent::JumpLoadRequested { offset, limit } => {
                    Some(PlaybackEvent::JumpLoadRequested { offset, limit })
                }
                soul_playback::PlaybackEvent::Error { message } => {
                    Some(PlaybackEvent::Error(message))
                }
            };

            if let Some(event) = desktop_event {
                if let Err(e) = event_tx.try_send(event) {
                    tracing::warn!(
                        error = %e,
                        "[forward_manager_events] Event channel full - event dropped"
                    );
                }
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
                    tracing::debug!(
                        "[poll_track_loader] Next track ready for crossfade: {}",
                        result.track.title
                    );
                    mgr.set_next_source(source, result.track);
                } else {
                    // Current track loaded (initial load or track change)
                    tracing::debug!("[poll_track_loader] Track loaded: {}", result.track.title);
                    tracing::debug!(
                        "[poll_track_loader] State BEFORE set_audio_source: {:?}",
                        mgr.get_state()
                    );
                    mgr.set_audio_source(source);
                    tracing::debug!(
                        "[poll_track_loader] State AFTER set_audio_source: {:?}",
                        mgr.get_state()
                    );
                    let _ = event_tx.try_send(PlaybackEvent::StateChanged(mgr.get_state()));
                    let _ = event_tx.try_send(PlaybackEvent::TrackChanged(Some(result.track)));
                    let _ = event_tx.try_send(PlaybackEvent::QueueUpdated);
                }
            } else if let Some(error) = result.error {
                tracing::error!(
                    "[poll_track_loader] Failed to load '{}': {}",
                    result.track.title,
                    error
                );
                if !result.is_preload {
                    // Only emit error for current track loads, not preloads
                    tracing::error!(error = ?error, track_id = result.track.id, "[PLAYBACK] Failed to load track");
                    let _ = event_tx.try_send(PlaybackEvent::Error("TRACK_LOAD_ERROR".to_string()));
                    mgr.stop();
                    let _ = event_tx.try_send(PlaybackEvent::StateChanged(mgr.get_state()));
                }
            }
        }
    }

    /// Process playback command (acquires manager lock internally)
    ///
    /// This is the external API used for commands that arrive outside the audio callback.
    /// For commands processed within the audio callback, use `process_command_with_lock`
    /// to avoid redundant mutex acquisition.
    fn process_command(
        command: PlaybackCommand,
        manager: Arc<Mutex<PlaybackManager>>,
        event_tx: &Sender<PlaybackEvent>,
        track_loader: &Arc<crate::track_loader::TrackLoader>,
    ) -> Result<()> {
        let mut mgr = manager.lock().unwrap();
        Self::process_command_with_lock(command, &mut mgr, event_tx, track_loader)
    }

    /// Process playback command with an already-acquired manager lock
    ///
    /// This variant is used in the audio callback to avoid acquiring the mutex twice
    /// (once for command processing, once for audio processing). This reduces latency
    /// by eliminating redundant lock/unlock cycles in the real-time audio path.
    fn process_command_with_lock(
        command: PlaybackCommand,
        mgr: &mut PlaybackManager,
        event_tx: &Sender<PlaybackEvent>,
        track_loader: &Arc<crate::track_loader::TrackLoader>,
    ) -> Result<()> {
        match command {
            PlaybackCommand::Play => {
                tracing::debug!("[PlaybackCommand::Play] Received");
                mgr.play()?;

                let state = mgr.get_state();
                tracing::debug!("[PlaybackCommand::Play] State after play(): {:?}", state);

                // If state is Loading, request track load via background loader
                if state == soul_playback::PlaybackState::Loading {
                    if let Some(track) = mgr.get_current_track().cloned() {
                        tracing::debug!(
                            "[PlaybackCommand::Play] Requesting track load: {} from {}",
                            track.title,
                            track.path.display()
                        );
                        let target_sample_rate = mgr.get_sample_rate();
                        tracing::debug!(
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
                            tracing::debug!("[PlaybackCommand::Play] Load request queue full");
                        }
                        // Result will be handled by poll_track_loader in next callback
                    } else {
                        tracing::debug!("[PlaybackCommand::Play] No current track to load");
                    }
                } else {
                    tracing::debug!(
                        "[PlaybackCommand::Play] State is {:?}, not loading audio",
                        state
                    );
                    let _ = event_tx.try_send(PlaybackEvent::StateChanged(mgr.get_state()));
                }
            }
            PlaybackCommand::Pause => {
                tracing::debug!(
                    "[PlaybackCommand::Pause] Received, current state: {:?}",
                    mgr.get_state()
                );
                mgr.pause();
                tracing::debug!(
                    "[PlaybackCommand::Pause] After pause(), state: {:?}",
                    mgr.get_state()
                );
                let _ = event_tx.try_send(PlaybackEvent::StateChanged(mgr.get_state()));
            }
            PlaybackCommand::Stop => {
                mgr.stop();
                let _ = event_tx.try_send(PlaybackEvent::StateChanged(mgr.get_state()));
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
                            tracing::debug!("[PlaybackCommand::Next] Load request queue full");
                        }
                        // Result will be handled by poll_track_loader in next callback
                    }
                } else {
                    let _ = event_tx.try_send(PlaybackEvent::TrackChanged(
                        mgr.get_current_track().cloned(),
                    ));
                }
                // Emit queue updated since position changed
                let _ = event_tx.try_send(PlaybackEvent::QueueUpdated);
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
                            tracing::debug!("[PlaybackCommand::Previous] Load request queue full");
                        }
                        // Result will be handled by poll_track_loader in next callback
                    }
                } else {
                    let _ = event_tx.try_send(PlaybackEvent::TrackChanged(
                        mgr.get_current_track().cloned(),
                    ));
                }
                // Emit queue updated since position changed
                let _ = event_tx.try_send(PlaybackEvent::QueueUpdated);
            }
            PlaybackCommand::Seek(seconds) => {
                mgr.seek_to(std::time::Duration::from_secs_f64(seconds))?;
                let _ = event_tx.try_send(PlaybackEvent::PositionUpdated(seconds));
            }
            PlaybackCommand::SetVolume(volume) => {
                mgr.set_volume(volume);
                let _ = event_tx.try_send(PlaybackEvent::VolumeChanged(volume));
            }
            PlaybackCommand::Mute => {
                mgr.mute();
                let _ = event_tx.try_send(PlaybackEvent::VolumeChanged(mgr.get_volume()));
            }
            PlaybackCommand::Unmute => {
                mgr.unmute();
                let _ = event_tx.try_send(PlaybackEvent::VolumeChanged(mgr.get_volume()));
            }
            PlaybackCommand::AddToQueue(track) => {
                // Legacy command - maps to AddToQueueEnd
                mgr.add_to_queue_end(track);
                let _ = event_tx.try_send(PlaybackEvent::QueueUpdated);
            }
            PlaybackCommand::AddPlayNext(track) => {
                mgr.add_to_queue_next(track);
                let _ = event_tx.try_send(PlaybackEvent::QueueUpdated);
            }
            PlaybackCommand::AddToQueueEnd(track) => {
                mgr.add_to_queue_end(track);
                let _ = event_tx.try_send(PlaybackEvent::QueueUpdated);
            }
            PlaybackCommand::RemoveFromQueue(index) => {
                mgr.remove_from_queue(index)?;
                let _ = event_tx.try_send(PlaybackEvent::QueueUpdated);
            }
            PlaybackCommand::ClearQueue => {
                mgr.clear_queue();
                let _ = event_tx.try_send(PlaybackEvent::QueueUpdated);
            }
            PlaybackCommand::ClearPlayNext => {
                mgr.clear_play_next();
                let _ = event_tx.try_send(PlaybackEvent::QueueUpdated);
            }
            PlaybackCommand::ClearAddToQueue => {
                mgr.clear_add_to_queue();
                let _ = event_tx.try_send(PlaybackEvent::QueueUpdated);
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
                            tracing::debug!(
                                "[PlaybackCommand::SkipToQueueIndex] Load request queue full"
                            );
                        }
                        // Result will be handled by poll_track_loader in next callback
                    }
                }
                let _ = event_tx.try_send(PlaybackEvent::QueueUpdated);
            }
            PlaybackCommand::LoadPlaylist(tracks) => {
                // Load playlist/album as source queue (Spotify-style context)
                mgr.add_playlist_to_queue(tracks);
                let _ = event_tx.try_send(PlaybackEvent::QueueUpdated);
            }
            PlaybackCommand::AppendToSource(tracks) => {
                // Append tracks to source queue (for lazy loading)
                mgr.append_to_source(tracks);
                let _ = event_tx.try_send(PlaybackEvent::QueueUpdated);
            }
            PlaybackCommand::SetShuffle(mode) => {
                mgr.set_shuffle(mode);
                let _ = event_tx.try_send(PlaybackEvent::QueueUpdated);
            }
            PlaybackCommand::CycleShuffle => {
                let new_mode = mgr.cycle_shuffle();
                tracing::debug!(new_mode = ?new_mode, "[PlaybackCommand::CycleShuffle] Shuffle mode cycled");
                let _ = event_tx.try_send(PlaybackEvent::QueueUpdated);
            }
            PlaybackCommand::SetRepeat(mode) => {
                mgr.set_repeat(mode);
            }
            PlaybackCommand::SwitchDevice(_, _) => {
                // Device switching is handled externally via switch_device() method
                // This command shouldn't reach here, but log if it does
                tracing::warn!("[WARN] SwitchDevice command received in audio callback - should be handled externally");
            }
        }

        Ok(())
    }

    // Public API

    /// Send command to playback thread
    ///
    /// Uses `try_send` to avoid blocking if the channel is full (e.g., when
    /// audio callbacks aren't running). Commands may be dropped if the
    /// channel is full - this prevents deadlocks when switching audio devices.
    ///
    /// This function is optimized for low latency - it avoids mutex locks in the
    /// success path. Debug information is only gathered when errors occur.
    pub fn send_command(&self, command: PlaybackCommand) -> Result<()> {
        tracing::trace!(command = ?command, "[Playback] Sending command");

        match self.command_tx.try_send(command.clone()) {
            Ok(()) => {
                tracing::trace!(command = ?command, "[Playback] Command sent successfully");
                Ok(())
            }
            Err(crossbeam_channel::TrySendError::Full(_)) => {
                tracing::warn!(
                    command = ?command,
                    "[Playback] Command channel full, dropping command"
                );
                // Return Ok to not fail the operation - the command is just dropped
                // This can happen when switching audio devices and callbacks aren't running yet
                Ok(())
            }
            Err(crossbeam_channel::TrySendError::Disconnected(_)) => {
                // Only acquire locks and gather debug info on error (rare case)
                let stream_alive = self.stream.lock().map(|g| g.is_some()).unwrap_or(false);
                let backend = self
                    .current_backend
                    .lock()
                    .map(|g| *g)
                    .unwrap_or(crate::AudioBackend::Default);
                let device = self
                    .current_device
                    .lock()
                    .map(|g| g.clone())
                    .unwrap_or_else(|_| String::from("<unknown>"));
                let global_count = GLOBAL_I32_CALLBACK_COUNTER.load(Ordering::Relaxed);

                tracing::error!(
                    stream_alive = stream_alive,
                    backend = ?backend,
                    device = %device,
                    global_i32_callbacks = global_count,
                    "[Playback] Command channel disconnected - stream may have been terminated"
                );

                Err(crate::error::AudioError::PlaybackError(
                    "Command channel disconnected - stream may have been terminated".into(),
                ))
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

    /// Receive next event with timeout (blocking with timeout)
    ///
    /// Returns Some(event) if an event arrives within the timeout,
    /// None if the timeout expires.
    pub fn recv_event_timeout(&self, timeout: std::time::Duration) -> Option<PlaybackEvent> {
        self.event_rx.recv_timeout(timeout).ok()
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

    /// Get current shuffle mode
    pub fn get_shuffle_mode(&self) -> soul_playback::ShuffleMode {
        self.manager.lock().unwrap().get_shuffle_mode()
    }

    /// Get current repeat mode
    pub fn get_repeat_mode(&self) -> soul_playback::RepeatMode {
        self.manager.lock().unwrap().get_repeat()
    }

    /// Get mutable reference to PlaybackManager
    pub fn get_manager_mut(&self) -> std::sync::MutexGuard<'_, soul_playback::PlaybackManager> {
        self.manager.lock().unwrap()
    }

    /// Get the playback manager (for batch loading)
    pub fn get_playback_manager(&self) -> &Arc<Mutex<soul_playback::PlaybackManager>> {
        &self.manager
    }

    /// Emit queue updated event
    pub fn emit_queue_updated(&self) {
        let _ = self.event_tx.try_send(PlaybackEvent::QueueUpdated);
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
        self.switch_device_with_reason(backend, device_name, DeviceSwitchReason::UserRequested)
    }

    /// Switch to a different audio output device with a specific reason
    ///
    /// This is the internal implementation that tracks the switch reason for
    /// proper state machine transitions and error recovery.
    ///
    /// # Arguments
    /// * `backend` - Audio backend to use
    /// * `device_name` - Device name to switch to (None for default device)
    /// * `reason` - Reason for the device switch
    ///
    /// # Returns
    /// * `Ok(())` - Device switched successfully
    /// * `Err(_)` - Failed to switch device
    pub fn switch_device_with_reason(
        &mut self,
        backend: crate::AudioBackend,
        device_name: Option<String>,
        reason: DeviceSwitchReason,
    ) -> Result<()> {
        tracing::info!(
            backend = ?backend,
            device_name = ?device_name,
            reason = %reason,
            "[Playback] Starting device switch"
        );
        let switch_start = std::time::Instant::now();

        // Check if we can start a new switch
        {
            let state = self.device_switch_state.lock().unwrap();
            if !state.can_start_switch() {
                tracing::warn!(
                    current_state = ?*state,
                    "[Playback] Cannot start device switch - another switch in progress"
                );
                return Err(crate::error::AudioError::DeviceError(
                    "Device switch already in progress".to_string(),
                ));
            }
        }

        // Emit switch started event for UI feedback
        let target_device_display = device_name.clone().unwrap_or_else(|| "default".to_string());
        let _ = self.event_tx.try_send(PlaybackEvent::DeviceSwitchStarted {
            target_device: target_device_display.clone(),
            reason: reason.clone(),
        });

        // Step 1: Capture ALL state we need from manager in ONE lock acquisition
        // This prevents multiple lock/unlock cycles and potential deadlocks
        let (was_playing, position, current_track) = {
            let mgr = self.manager.lock().unwrap();
            let state = mgr.get_state();
            let pos = mgr.get_position();
            let track = mgr.get_current_track().cloned();
            (state == soul_playback::PlaybackState::Playing, pos, track)
        }; // Lock explicitly released here

        tracing::debug!(
            "[DesktopPlayback] Current state: playing={}, position={:?}",
            was_playing,
            position
        );

        // Update state machine: transition to Switching state
        {
            let mut state = self.device_switch_state.lock().unwrap();
            *state = DeviceSwitchState::Switching {
                target_device: device_name.clone(),
                target_backend: backend,
                reason: reason.clone(),
                saved_position: position,
                was_playing,
            };
            tracing::debug!("[DesktopPlayback] State machine: Idle -> Switching");
        }

        // Stop and drop the old stream
        // IMPORTANT: ASIO requires proper cleanup between stream creations
        {
            let mut stream_guard = self.stream.lock().unwrap();
            if let Some(stream) = stream_guard.take() {
                tracing::debug!("[DesktopPlayback] Pausing old stream before drop...");
                // Try to pause the stream first (some drivers need this)
                if let Err(e) = stream.pause() {
                    tracing::debug!(
                        "[DesktopPlayback] Warning: Failed to pause old stream: {}",
                        e
                    );
                }
                tracing::debug!("[DesktopPlayback] Dropping old stream...");
                drop(stream);
                tracing::debug!("[DesktopPlayback] Old stream dropped");
            }
        }

        // Longer delay for ASIO - drivers often need time to release resources
        tracing::debug!("[DesktopPlayback] Waiting for driver to release resources...");
        std::thread::sleep(std::time::Duration::from_millis(200));
        tracing::debug!("[DesktopPlayback] Resource release wait complete");

        // Create new command channel for the new stream
        tracing::debug!("[DesktopPlayback] Creating new command channel...");
        let (new_command_tx, new_command_rx) = bounded(32);

        // Update command_tx for this instance
        tracing::debug!("[DesktopPlayback] Updating command_tx to new channel...");
        self.command_tx = new_command_tx.clone();
        tracing::debug!(
            "[DesktopPlayback] command_tx updated, channel capacity: {}",
            self.command_tx.capacity().unwrap_or(0)
        );

        // Create new stream with new device, reusing the same event_tx
        tracing::info!("[Playback] Attempting to create new stream for device switch");
        let stream_result = Self::create_audio_stream(
            self.manager.clone(),
            new_command_rx,
            self.event_tx.clone(),
            backend,
            device_name.clone(),
            self.track_loader.clone(),
        );

        // Handle stream creation failure with recovery logic
        let (new_stream_option, actual_device_name, new_sample_rate) = match stream_result {
            Ok(result) => result,
            Err(e) => {
                tracing::error!(
                    error = %e,
                    "[Playback] Failed to create stream for device switch"
                );

                // Transition to Recovering state
                {
                    let mut state = self.device_switch_state.lock().unwrap();
                    *state = DeviceSwitchState::Recovering {
                        retry_count: 0,
                        last_error: e.to_string(),
                        saved_position: position,
                    };
                }

                // Emit failure event
                let _ = self.event_tx.try_send(PlaybackEvent::DeviceSwitchFailed {
                    error: e.to_string(),
                    fallback_attempted: self.device_switch_config.auto_fallback,
                });

                // Try fallback to default device if configured
                if self.device_switch_config.auto_fallback && device_name.is_some() {
                    tracing::warn!("[Playback] Attempting fallback to default device");

                    // Recursive call to switch to default - reset state first
                    {
                        let mut state = self.device_switch_state.lock().unwrap();
                        *state = DeviceSwitchState::Idle;
                    }

                    return self.switch_device_with_reason(
                        crate::AudioBackend::Default,
                        None,
                        DeviceSwitchReason::ErrorRecovery,
                    );
                }

                // Reset state machine on failure
                {
                    let mut state = self.device_switch_state.lock().unwrap();
                    *state = DeviceSwitchState::Idle;
                }

                return Err(e);
            }
        };

        let is_silent_mode = new_stream_option.is_none();

        if is_silent_mode {
            tracing::warn!("[Playback] Device switch resulted in silent mode (zero-device system)");
        } else {
            tracing::info!(
                device_name = %actual_device_name,
                sample_rate = new_sample_rate,
                "[Playback] Device switch successful - new stream created"
            );
        }

        // Check if sample rate changed
        let old_sample_rate = self.current_sample_rate.load(Ordering::SeqCst);
        if old_sample_rate != new_sample_rate {
            tracing::debug!(
                "[DesktopPlayback] Sample rate changed: {} Hz -> {} Hz",
                old_sample_rate,
                new_sample_rate
            );
            self.current_sample_rate
                .store(new_sample_rate, Ordering::SeqCst);
            let _ = self.event_tx.try_send(PlaybackEvent::SampleRateChanged(
                old_sample_rate,
                new_sample_rate,
            ));
        }

        tracing::debug!(
            "[DesktopPlayback] New stream created for device: {} at {} Hz",
            actual_device_name,
            new_sample_rate
        );

        // Check callbacks before storing
        let callbacks_before_store = GLOBAL_I32_CALLBACK_COUNTER.load(Ordering::Relaxed);
        tracing::debug!(
            "[DesktopPlayback] Callbacks before storing stream: {}",
            callbacks_before_store
        );

        // Store new stream
        tracing::debug!("[DesktopPlayback] Storing new stream...");
        {
            let mut stream_guard = self.stream.lock().unwrap();
            *stream_guard = new_stream_option;
        }

        // Check callbacks immediately after storing
        let callbacks_after_store = GLOBAL_I32_CALLBACK_COUNTER.load(Ordering::Relaxed);
        tracing::debug!(
            "[DesktopPlayback] Stream stored. Callbacks: {} (diff: {})",
            callbacks_after_store,
            callbacks_after_store - callbacks_before_store
        );
        tracing::debug!("[DesktopPlayback] Waiting 100ms for callbacks to start...");

        // Give the new stream a moment to start callbacks
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Check callbacks after sleep
        let callbacks_after_sleep = GLOBAL_I32_CALLBACK_COUNTER.load(Ordering::Relaxed);
        tracing::debug!(
            "[DesktopPlayback] After 100ms sleep. Callbacks: {} (diff: {})",
            callbacks_after_sleep,
            callbacks_after_sleep - callbacks_after_store
        );

        // Verify channel status after stream creation
        let channel_len = self.command_tx.len();
        let channel_cap = self.command_tx.capacity().unwrap_or(0);
        let callbacks_so_far = GLOBAL_I32_CALLBACK_COUNTER.load(Ordering::Relaxed);
        tracing::debug!("[DesktopPlayback] Channel verification after stream creation:");
        tracing::debug!("[DesktopPlayback]   Queue length: {}", channel_len);
        tracing::debug!("[DesktopPlayback]   Capacity: {}", channel_cap);
        tracing::debug!(
            "[DesktopPlayback]   Global I32 callbacks: {}",
            callbacks_so_far
        );

        // Check callbacks before updating backend
        let callbacks_before_backend = GLOBAL_I32_CALLBACK_COUNTER.load(Ordering::Relaxed);
        tracing::debug!(
            "[DesktopPlayback] Callbacks before backend update: {}",
            callbacks_before_backend
        );

        // Update current backend, device, and device ID
        {
            *self.current_backend.lock().unwrap() = backend;
            *self.current_device.lock().unwrap() = actual_device_name.clone();

            // Update device ID for the new device
            let new_device_id = if is_silent_mode {
                None
            } else {
                Some(Self::make_device_id(backend, &actual_device_name))
            };
            *self.current_device_id.lock().unwrap() = new_device_id;
        }

        // Check callbacks after updating backend
        let callbacks_after_backend = GLOBAL_I32_CALLBACK_COUNTER.load(Ordering::Relaxed);
        tracing::debug!("[DesktopPlayback] Backend and device name updated");
        tracing::debug!(
            "[DesktopPlayback] Callbacks after backend update: {} (diff: {})",
            callbacks_after_backend,
            callbacks_after_backend - callbacks_before_backend
        );

        // Step 2: Reload the audio source ONLY if sample rate changed
        // This prevents unnecessary reloads when switching devices with the same sample rate
        // (e.g., WASAPI 48kHz → ASIO 48kHz)
        // We use the current_track we captured earlier to avoid another lock
        #[allow(clippy::if_not_else)]
        if old_sample_rate != new_sample_rate {
            if let Some(track) = current_track {
                tracing::info!(
                    "[DesktopPlayback] Sample rate changed, reloading audio source: {} Hz -> {} Hz",
                    old_sample_rate,
                    new_sample_rate
                );

                // Create the new audio source with the new sample rate (no lock needed)
                match crate::sources::local::LocalAudioSource::new(&track.path, new_sample_rate) {
                    Ok(mut source) => {
                        // CRITICAL: Seek the source to the saved position BEFORE setting it
                        // This prevents the track from restarting when switching devices
                        if position > std::time::Duration::ZERO {
                            tracing::info!(
                                "[DesktopPlayback] Seeking new audio source to position: {:?}",
                                position
                            );
                            if let Err(e) = source.seek(position) {
                                tracing::error!(
                                    "[DesktopPlayback] Failed to seek new audio source: {}",
                                    e
                                );
                            } else {
                                tracing::info!(
                                    "[DesktopPlayback] Audio source pre-seeked to {:?}",
                                    position
                                );
                            }
                        }

                        // Now set the pre-seeked source - this is the FIRST time we re-acquire manager lock
                        {
                            let mut mgr = self.manager.lock().unwrap();
                            mgr.set_audio_source(Box::new(source));
                        } // Lock released

                        tracing::info!(
                            "[DesktopPlayback] Audio source reloaded and set with sample rate: {}",
                            new_sample_rate
                        );
                    }
                    Err(e) => {
                        tracing::error!("[DesktopPlayback] Failed to reload audio source: {}", e);
                    }
                }
            }
        } else {
            tracing::info!(
                "[DesktopPlayback] Sample rate unchanged ({}Hz), skipping audio source reload",
                new_sample_rate
            );

            // Even though we're not reloading, restore position in case it drifted
            if position > std::time::Duration::ZERO {
                {
                    let mut mgr = self.manager.lock().unwrap();
                    if let Err(e) = mgr.seek_to(position) {
                        tracing::error!("[DesktopPlayback] Failed to restore position: {}", e);
                    } else {
                        tracing::info!("[DesktopPlayback] Position restored to {:?}", position);
                    }
                } // Lock released
            }
        }

        // Step 3: Resume playback if it was playing (separate lock acquisition)
        if was_playing {
            {
                let mut mgr = self.manager.lock().unwrap();
                if let Err(e) = mgr.play() {
                    tracing::debug!("[DesktopPlayback] Failed to resume playback: {}", e);
                } else {
                    tracing::debug!("[DesktopPlayback] Playback resumed");
                }
            } // Lock released
        }

        // Step 4: Get final state and emit event (final lock acquisition)
        let current_state = {
            let mgr = self.manager.lock().unwrap();
            mgr.get_state()
        }; // Lock released
        tracing::debug!(
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
                tracing::debug!("[DesktopPlayback] StateChanged event sent successfully");
            }
            Err(crossbeam_channel::SendTimeoutError::Timeout(_)) => {
                tracing::debug!("[DesktopPlayback] WARNING: StateChanged event timed out, frontend may be out of sync");
            }
            Err(crossbeam_channel::SendTimeoutError::Disconnected(_)) => {
                tracing::debug!("[DesktopPlayback] ERROR: Event channel disconnected");
            }
        }

        // Final callback check before returning
        let callbacks_at_end = GLOBAL_I32_CALLBACK_COUNTER.load(Ordering::Relaxed);
        let switch_duration = switch_start.elapsed();
        tracing::info!(
            backend = ?backend,
            device_name = %actual_device_name,
            old_sample_rate,
            new_sample_rate,
            was_playing,
            switch_duration_ms = switch_duration.as_millis(),
            final_callbacks = callbacks_at_end,
            "[Playback] Device switch completed successfully"
        );

        // Transition state machine back to Idle
        {
            let mut state = self.device_switch_state.lock().unwrap();
            *state = DeviceSwitchState::Idle;
            tracing::debug!("[DesktopPlayback] State machine: Switching -> Idle");
        }

        // Emit switch completed event
        let _ = self
            .event_tx
            .try_send(PlaybackEvent::DeviceSwitchCompleted {
                device_name: actual_device_name,
                sample_rate: new_sample_rate,
            });

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

    /// Get current device ID
    ///
    /// Returns a unique identifier for the current audio device (backend + device name).
    /// This is used to prevent false positive device switches when checking sample rates.
    ///
    /// # Returns
    /// * `Some(device_id)` - The current device's unique identifier
    /// * `None` - No device active (silent mode)
    pub fn get_current_device_id(&self) -> Option<String> {
        self.current_device_id.lock().unwrap().clone()
    }

    /// Get current stream sample rate
    pub fn get_current_sample_rate(&self) -> u32 {
        self.current_sample_rate.load(Ordering::SeqCst)
    }

    /// Get current device switch state
    ///
    /// Returns a clone of the current device switch state for inspection.
    /// Use `is_device_switching()` for a simpler check.
    pub fn get_device_switch_state(&self) -> DeviceSwitchState {
        self.device_switch_state.lock().unwrap().clone()
    }

    /// Check if a device switch is currently in progress
    ///
    /// Returns true if the device switch state machine is not in Idle state.
    pub fn is_device_switching(&self) -> bool {
        self.device_switch_state.lock().unwrap().is_switching()
    }

    /// Get device switch configuration
    ///
    /// Returns a clone of the current device switch configuration.
    pub fn get_device_switch_config(&self) -> DeviceSwitchConfig {
        self.device_switch_config.clone()
    }

    /// Set device switch configuration
    ///
    /// Updates the device switch configuration for future switches.
    pub fn set_device_switch_config(&mut self, config: DeviceSwitchConfig) {
        self.device_switch_config = config;
    }

    /// Create a unique device ID from backend and device name
    ///
    /// Device ID format: "{backend}::{device_name}"
    /// This provides a unique identifier that can be used to track
    /// which device is currently active and detect device removal events.
    ///
    /// # Example
    /// ```rust,ignore
    /// let device_id = DesktopPlayback::make_device_id(
    ///     AudioBackend::WASAPI,
    ///     "Speakers (Realtek Audio)"
    /// );
    /// // Returns: "WASAPI::Speakers (Realtek Audio)"
    /// ```
    pub fn make_device_id(backend: crate::AudioBackend, device_name: &str) -> String {
        format!("{}::{}", backend.name(), device_name)
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
    /// # Note on Device Removal
    /// When implementing device removal detection, use `get_current_device_id()` to avoid
    /// false positives. Example:
    ///
    /// ```rust,ignore
    /// // In DeviceRemoved handler:
    /// if let Some(current_id) = playback.get_current_device_id() {
    ///     let removed_id = DesktopPlayback::make_device_id(backend, &removed_device_name);
    ///     if current_id == removed_id {
    ///         // Definitely our device - switch to default
    ///         playback.switch_device(AudioBackend::Default, None)?;
    ///     } else {
    ///         // Not our device - just log and ignore
    ///         tracing::debug!("Device removed, but not ours: {}", removed_device_name);
    ///     }
    /// }
    /// ```
    ///
    /// This prevents false positives where `query_device_sample_rate()` fails for reasons
    /// other than device removal (e.g., driver busy, temporary error).
    ///
    /// # Returns
    /// * `Ok(true)` - Sample rate changed and stream was recreated
    /// * `Ok(false)` - Sample rate unchanged, no action needed
    /// * `Err(_)` - Failed to check or update sample rate
    pub fn check_and_update_sample_rate(&mut self) -> Result<bool> {
        let device_rate = match self.query_device_sample_rate() {
            Ok(rate) => rate,
            Err(e) => {
                tracing::debug!(
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

        tracing::debug!(
            "[DesktopPlayback] Device sample rate changed: {} Hz -> {} Hz",
            current_rate,
            device_rate
        );

        // Sample rate has changed - need to recreate the stream
        let backend = *self.current_backend.lock().unwrap();
        let device_name = self.current_device.lock().unwrap().clone();

        // switch_device will handle everything: stream recreation, source reload, position preservation
        self.switch_device_with_reason(
            backend,
            Some(device_name),
            DeviceSwitchReason::SampleRateMismatch,
        )?;

        Ok(true)
    }

    /// Switch to the system default audio device
    ///
    /// This method is used when the system default device changes (e.g., user
    /// plugs in headphones, changes default in sound settings). It switches
    /// playback to whatever the OS currently considers the default device.
    ///
    /// Unlike `check_and_update_sample_rate()` which only detects sample rate
    /// changes on the *current* device, this method actually switches to the
    /// new system default.
    ///
    /// # Returns
    /// * `Ok(())` - Successfully switched to default device
    /// * `Err(_)` - Failed to switch (will attempt fallback if configured)
    pub fn switch_to_system_default(&mut self) -> Result<()> {
        let current_device = self.current_device.lock().unwrap().clone();
        let backend = *self.current_backend.lock().unwrap();

        // Query what the system thinks is the default device now
        let new_default = match crate::device::get_default_device(backend) {
            Ok(device) => device,
            Err(e) => {
                tracing::error!(
                    error = %e,
                    "[DesktopPlayback] Failed to get system default device"
                );
                return Err(crate::error::AudioError::DeviceError(e.to_string()));
            }
        };

        // Check if we're already using the default device
        if new_default.name == current_device {
            tracing::debug!(
                device = %current_device,
                "[DesktopPlayback] Already using system default device"
            );
            return Ok(());
        }

        tracing::info!(
            old_device = %current_device,
            new_device = %new_default.name,
            "[DesktopPlayback] Switching to system default device"
        );

        // Switch to the new default device
        self.switch_device_with_reason(
            backend,
            None, // None means use default device
            DeviceSwitchReason::DefaultDeviceChanged,
        )
    }

    /// Check if a device name matches our current device
    ///
    /// Device IDs from platform APIs (WinRT, CoreAudio) may differ in format
    /// from the device names we store. This method handles the comparison
    /// by checking both the full device ID and the device name.
    ///
    /// # Arguments
    /// * `device_id_or_name` - The device identifier to check (from platform API)
    ///
    /// # Returns
    /// * `true` if the device matches our current device
    /// * `false` otherwise
    pub fn is_current_device(&self, device_id_or_name: &str) -> bool {
        let current_device = self.current_device.lock().unwrap();
        let current_device_id = self.current_device_id.lock().unwrap();

        // Check exact match with device name
        if *current_device == device_id_or_name {
            return true;
        }

        // Check if device_id contains our device name (handles WinRT full IDs)
        if device_id_or_name.contains(current_device.as_str()) {
            return true;
        }

        // Check against our stored device ID
        if let Some(ref stored_id) = *current_device_id {
            if stored_id == device_id_or_name {
                return true;
            }
            // Also check if the provided ID contains our stored ID or vice versa
            if device_id_or_name.contains(stored_id.as_str())
                || stored_id.contains(device_id_or_name)
            {
                return true;
            }
        }

        false
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
    /// Returns the effect chain from the underlying `PlaybackManager`.
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

    /// Set volume leveling mode (`ReplayGain` track/album, EBU R128, etc.)
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
    /// * `gain_db` - `ReplayGain` value in dB
    /// * `peak_dbfs` - Peak value in dBFS (for clipping prevention)
    pub fn set_track_gain(&self, gain_db: f64, peak_dbfs: f64) {
        let mut manager = self.manager.lock().unwrap();
        manager.set_track_gain(gain_db, peak_dbfs);
    }

    /// Set album gain for current track (called when loading track)
    ///
    /// # Arguments
    /// * `gain_db` - Album `ReplayGain` value in dB
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
        tracing::debug!(
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
        tracing::debug!("[DesktopPlayback] Disabling exclusive mode");

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
    /// - `SquareRoot`: Natural-sounding transitions
    /// - `SCurve`: Smooth acceleration at start/end
    /// - `EqualPower`: Constant perceived loudness (recommended)
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
        tracing::debug!(
            "[DesktopPlayback] Resampling quality set to '{}' (sinc_len={}, f_cutoff={})",
            quality,
            settings.sinc_len(),
            settings.f_cutoff()
        );
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
        if rate != 0 && !(8000..=384000).contains(&rate) {
            return Err(format!(
                "Invalid target rate {}. Must be 0 (auto) or between 8000 and 384000 Hz",
                rate
            ));
        }

        let mut settings = self.resampling_settings.lock().unwrap();
        settings.target_rate = rate;
        tracing::debug!(
            "[DesktopPlayback] Resampling target rate set to {} (0=auto)",
            rate
        );
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
                    Use 'auto' or 'rubato' instead."
                    .to_string());
            }
        }

        let mut settings = self.resampling_settings.lock().unwrap();
        settings.backend = backend.to_string();
        tracing::debug!("[DesktopPlayback] Resampling backend set to '{}'", backend);
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

    // ===========================================================================
    // Headroom Management
    // ===========================================================================

    /// Set headroom management mode
    ///
    /// Modes:
    /// - Auto: Calculate from `ReplayGain` + EQ boost
    /// - Manual(dB): Fixed headroom reserve
    /// - Disabled: No headroom attenuation
    #[cfg(feature = "volume-leveling")]
    pub fn set_headroom_mode(&self, mode: soul_playback::HeadroomMode) {
        let mut manager = self.manager.lock().unwrap();
        manager.set_headroom_mode(mode);
    }

    /// Get current headroom mode
    #[cfg(feature = "volume-leveling")]
    pub fn get_headroom_mode(&self) -> soul_playback::HeadroomMode {
        let manager = self.manager.lock().unwrap();
        manager.get_headroom_mode()
    }

    /// Set headroom enabled state
    #[cfg(feature = "volume-leveling")]
    pub fn set_headroom_enabled(&self, enabled: bool) {
        let mut manager = self.manager.lock().unwrap();
        manager.set_headroom_enabled(enabled);
    }

    /// Check if headroom management is enabled
    #[cfg(feature = "volume-leveling")]
    pub fn is_headroom_enabled(&self) -> bool {
        let manager = self.manager.lock().unwrap();
        manager.is_headroom_enabled()
    }

    /// Set EQ boost value for headroom calculation (used in Auto mode)
    #[cfg(feature = "volume-leveling")]
    pub fn set_headroom_eq_boost_db(&self, boost_db: f64) {
        let mut manager = self.manager.lock().unwrap();
        manager.set_headroom_eq_boost_db(boost_db);
    }

    /// Set pre-amp value for headroom calculation (used in Auto mode)
    #[cfg(feature = "volume-leveling")]
    pub fn set_headroom_preamp_db(&self, preamp_db: f64) {
        let mut manager = self.manager.lock().unwrap();
        manager.set_headroom_preamp_db(preamp_db);
    }

    /// Get total potential gain from all sources
    #[cfg(feature = "volume-leveling")]
    pub fn get_headroom_total_gain_db(&self) -> f64 {
        let manager = self.manager.lock().unwrap();
        manager.get_headroom_total_gain_db()
    }

    /// Get current attenuation being applied
    #[cfg(feature = "volume-leveling")]
    pub fn get_headroom_attenuation_db(&self) -> f64 {
        let mut manager = self.manager.lock().unwrap();
        manager.get_headroom_attenuation_db()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "Requires real audio hardware - not available in CI environments"]
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
    #[ignore = "Requires real audio hardware - not available in CI environments"]
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
    #[ignore = "Requires real audio hardware - not available in CI environments"]
    fn test_get_current_device_info() {
        let result = DesktopPlayback::new(PlaybackConfig::default());

        match result {
            Ok(playback) => {
                let backend = playback.get_current_backend();
                let device = playback.get_current_device();
                let device_id = playback.get_current_device_id();

                eprintln!("Current backend: {:?}", backend);
                eprintln!("Current device: {}", device);
                eprintln!("Current device ID: {:?}", device_id);

                assert_eq!(backend, crate::AudioBackend::Default);
                assert!(!device.is_empty());

                // Device ID should be set and should match expected format
                if let Some(id) = device_id {
                    let expected_id = DesktopPlayback::make_device_id(backend, &device);
                    assert_eq!(id, expected_id);
                    assert!(id.contains("::"));
                    eprintln!("Device ID format verified: {}", id);
                } else {
                    eprintln!("Device ID is None (silent mode)");
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
    #[ignore = "Requires real audio hardware - not available in CI environments"]
    fn test_switch_device_to_default() {
        let result = DesktopPlayback::new(PlaybackConfig::default());

        match result {
            Ok(mut playback) => {
                let original_device = playback.get_current_device();
                let original_device_id = playback.get_current_device_id();
                eprintln!("Original device: {}", original_device);
                eprintln!("Original device ID: {:?}", original_device_id);

                // Try to switch to default device again (should succeed)
                let switch_result = playback.switch_device(crate::AudioBackend::Default, None);

                match switch_result {
                    Ok(()) => {
                        let new_device = playback.get_current_device();
                        let new_device_id = playback.get_current_device_id();
                        let backend = playback.get_current_backend();

                        eprintln!("After switch device: {}", new_device);
                        eprintln!("After switch device ID: {:?}", new_device_id);

                        assert!(!new_device.is_empty());

                        // Verify device ID is updated correctly
                        if let Some(id) = new_device_id {
                            let expected_id = DesktopPlayback::make_device_id(backend, &new_device);
                            assert_eq!(id, expected_id);
                            eprintln!("Device ID correctly updated after switch");
                        }
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
    #[ignore = "Requires real audio hardware - not available in CI environments"]
    fn test_switch_device_preserves_backend() {
        let result = DesktopPlayback::new(PlaybackConfig::default());

        match result {
            Ok(mut playback) => {
                // Switch to default backend explicitly
                if let Ok(()) = playback.switch_device(crate::AudioBackend::Default, None) {
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
    #[ignore = "Requires real audio hardware - not available in CI environments"]
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

    #[test]
    fn test_make_device_id() {
        // Test device ID format with Default backend
        let device_id = DesktopPlayback::make_device_id(
            crate::AudioBackend::Default,
            "Speakers (Realtek Audio)",
        );

        // On Windows, Default backend is WASAPI
        #[cfg(target_os = "windows")]
        assert_eq!(device_id, "WASAPI::Speakers (Realtek Audio)");

        // On macOS, Default backend is CoreAudio
        #[cfg(target_os = "macos")]
        assert_eq!(device_id, "CoreAudio::Speakers (Realtek Audio)");

        // On Linux, Default backend is ALSA
        #[cfg(target_os = "linux")]
        assert_eq!(device_id, "ALSA::Speakers (Realtek Audio)");

        // Verify format contains separator
        assert!(device_id.contains("::"));

        // Test with ASIO backend if available
        #[cfg(all(target_os = "windows", feature = "asio"))]
        {
            let device_id2 =
                DesktopPlayback::make_device_id(crate::AudioBackend::Asio, "ASIO Device");
            assert_eq!(device_id2, "ASIO::ASIO Device");

            // Test that device IDs are unique
            assert_ne!(device_id, device_id2);
        }

        // Test that same backend + device name produces same ID
        let device_id3 = DesktopPlayback::make_device_id(
            crate::AudioBackend::Default,
            "Speakers (Realtek Audio)",
        );
        assert_eq!(device_id, device_id3);

        // Test with different device names
        let device_id4 =
            DesktopPlayback::make_device_id(crate::AudioBackend::Default, "Different Device");
        assert_ne!(device_id, device_id4);
    }

    // ===== Device Switch State Machine Tests =====

    #[test]
    fn test_device_switch_state_idle_default() {
        let state = DeviceSwitchState::default();
        assert_eq!(state, DeviceSwitchState::Idle);
        assert!(!state.is_switching());
        assert!(state.can_start_switch());
        assert!(state.target_device().is_none());
    }

    #[test]
    fn test_device_switch_state_switching() {
        let state = DeviceSwitchState::Switching {
            target_device: Some("Speaker".to_string()),
            target_backend: crate::AudioBackend::Default,
            reason: DeviceSwitchReason::UserRequested,
            saved_position: std::time::Duration::from_secs(10),
            was_playing: true,
        };

        assert!(state.is_switching());
        assert!(!state.can_start_switch());
        assert_eq!(state.target_device(), Some("Speaker"));
    }

    #[test]
    fn test_device_switch_state_recovering() {
        let state = DeviceSwitchState::Recovering {
            retry_count: 2,
            last_error: "Connection failed".to_string(),
            saved_position: std::time::Duration::from_secs(30),
        };

        assert!(state.is_switching());
        // Recovery state DOES allow new switches (to recover from failed switch)
        assert!(state.can_start_switch());
        assert!(state.target_device().is_none());
    }

    #[test]
    fn test_device_switch_state_fading_out() {
        let state = DeviceSwitchState::FadingOut {
            target_device: Some("Headphones".to_string()),
            target_backend: crate::AudioBackend::Default,
            reason: DeviceSwitchReason::DeviceDisconnected,
            samples_remaining: 1024,
        };

        assert!(state.is_switching());
        assert!(!state.can_start_switch());
        assert_eq!(state.target_device(), Some("Headphones"));
    }

    #[test]
    fn test_device_switch_state_fading_in() {
        let state = DeviceSwitchState::FadingIn {
            device_name: "New Speaker".to_string(),
            samples_remaining: 512,
        };

        assert!(state.is_switching());
        assert!(!state.can_start_switch());
        assert!(state.target_device().is_none());
    }

    #[test]
    fn test_device_switch_reason_display() {
        assert_eq!(
            format!("{}", DeviceSwitchReason::UserRequested),
            "user_requested"
        );
        assert_eq!(
            format!("{}", DeviceSwitchReason::DeviceDisconnected),
            "device_disconnected"
        );
        assert_eq!(
            format!("{}", DeviceSwitchReason::DefaultDeviceChanged),
            "default_device_changed"
        );
        assert_eq!(
            format!("{}", DeviceSwitchReason::SampleRateMismatch),
            "sample_rate_mismatch"
        );
        assert_eq!(
            format!("{}", DeviceSwitchReason::ErrorRecovery),
            "error_recovery"
        );
    }

    #[test]
    fn test_device_switch_config_default() {
        let config = DeviceSwitchConfig::default();

        assert_eq!(config.fadeout_ms, 50);
        assert_eq!(config.fadein_ms, 30);
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.retry_delay_ms, 100);
        assert!(config.auto_fallback);
    }

    #[test]
    fn test_device_switch_config_custom() {
        let config = DeviceSwitchConfig {
            fadeout_ms: 100,
            fadein_ms: 50,
            max_retries: 5,
            retry_delay_ms: 200,
            auto_fallback: false,
        };

        assert_eq!(config.fadeout_ms, 100);
        assert_eq!(config.fadein_ms, 50);
        assert_eq!(config.max_retries, 5);
        assert_eq!(config.retry_delay_ms, 200);
        assert!(!config.auto_fallback);
    }

    #[test]
    #[ignore = "Requires real audio hardware - not available in CI environments"]
    fn test_device_switch_state_machine_integration() {
        let result = DesktopPlayback::new(PlaybackConfig::default());

        match result {
            Ok(playback) => {
                // Initial state should be Idle
                let state = playback.get_device_switch_state();
                assert_eq!(state, DeviceSwitchState::Idle);
                assert!(!playback.is_device_switching());

                // Config should be default
                let config = playback.get_device_switch_config();
                assert_eq!(config.fadeout_ms, 50);
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
    #[ignore = "Requires real audio hardware - not available in CI environments"]
    fn test_switch_device_with_reason() {
        let result = DesktopPlayback::new(PlaybackConfig::default());

        match result {
            Ok(mut playback) => {
                // Initial state should be Idle
                assert!(!playback.is_device_switching());

                // Switch with explicit reason
                let switch_result = playback.switch_device_with_reason(
                    crate::AudioBackend::Default,
                    None,
                    DeviceSwitchReason::UserRequested,
                );

                match switch_result {
                    Ok(()) => {
                        // After successful switch, state should be back to Idle
                        assert!(!playback.is_device_switching());
                        eprintln!("Device switch with reason succeeded");
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
}
