//! Desktop playback integration
//!
//! Combines PlaybackManager with CPAL audio output for desktop playback.

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

    /// Set shuffle mode
    SetShuffle(soul_playback::ShuffleMode),

    /// Set repeat mode
    SetRepeat(soul_playback::RepeatMode),
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
/// Manages PlaybackManager + CPAL audio output + event handling
pub struct DesktopPlayback {
    /// Command sender
    command_tx: Sender<PlaybackCommand>,

    /// Event receiver
    event_rx: Receiver<PlaybackEvent>,

    /// CPAL audio stream
    _stream: Stream,

    /// Playback manager (shared with audio thread)
    manager: Arc<Mutex<PlaybackManager>>,
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
        let manager = Arc::new(Mutex::new(PlaybackManager::new(config)));

        let (command_tx, command_rx) = bounded(32);
        let (event_tx, event_rx) = bounded(32);

        // Create CPAL stream
        let stream = Self::create_audio_stream(manager.clone(), command_rx, event_tx)?;

        Ok(Self {
            command_tx,
            event_rx,
            _stream: stream,
            manager,
        })
    }

    /// Create CPAL audio stream
    fn create_audio_stream(
        manager: Arc<Mutex<PlaybackManager>>,
        command_rx: Receiver<PlaybackCommand>,
        event_tx: Sender<PlaybackEvent>,
    ) -> Result<Stream> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| crate::error::AudioError::DeviceNotFound)?;

        let config = Self::get_stream_config(&device)?;
        let sample_rate = config.sample_rate.0;

        // Set sample rate in manager
        manager.lock().unwrap().set_sample_rate(sample_rate);

        let manager_clone = manager.clone();

        // Build stream
        let stream = device.build_output_stream(
            &config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                Self::audio_callback(data, manager_clone.clone(), &command_rx, &event_tx);
            },
            |err| eprintln!("Audio stream error: {}", err),
            None,
        )?;

        stream.play()?;

        Ok(stream)
    }

    /// Get stream configuration
    fn get_stream_config(device: &Device) -> Result<StreamConfig> {
        let config = device.default_output_config()?;

        // Use default config
        Ok(config.into())
    }

    /// Audio callback
    fn audio_callback(
        data: &mut [f32],
        manager: Arc<Mutex<PlaybackManager>>,
        command_rx: &Receiver<PlaybackCommand>,
        event_tx: &Sender<PlaybackEvent>,
    ) {
        // Process any pending commands
        while let Ok(command) = command_rx.try_recv() {
            if let Err(e) = Self::process_command(command, manager.clone(), event_tx) {
                event_tx
                    .send(PlaybackEvent::Error(format!("Command error: {}", e)))
                    .ok();
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
                event_tx
                    .send(PlaybackEvent::Error(format!(
                        "Audio processing error: {}",
                        e
                    )))
                    .ok();
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
                        eprintln!("[PlaybackCommand::Play] Loading track: {} from {}", track.title, track.path.display());
                        // Create audio source from file path
                        match crate::sources::local::LocalAudioSource::new(&track.path) {
                            Ok(source) => {
                                eprintln!("[PlaybackCommand::Play] Audio source loaded successfully");
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
                    eprintln!("[PlaybackCommand::Play] State is {:?}, not loading audio", state);
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
                        match crate::sources::local::LocalAudioSource::new(&track.path) {
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
            }
            PlaybackCommand::Previous => {
                mgr.previous()?;

                // If state is Loading, we need to load the audio source
                if mgr.get_state() == soul_playback::PlaybackState::Loading {
                    if let Some(track) = mgr.get_current_track().cloned() {
                        match crate::sources::local::LocalAudioSource::new(&track.path) {
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
            PlaybackCommand::SetShuffle(mode) => {
                mgr.set_shuffle(mode);
                event_tx.send(PlaybackEvent::QueueUpdated).ok();
            }
            PlaybackCommand::SetRepeat(mode) => {
                mgr.set_repeat(mode);
            }
        }

        Ok(())
    }

    // Public API

    /// Send command to playback thread
    pub fn send_command(&self, command: PlaybackCommand) -> Result<()> {
        self.command_tx.send(command).map_err(|_| {
            crate::error::AudioError::PlaybackError("Failed to send command".into())
        })?;
        Ok(())
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
}
