//! Playback management for Tauri desktop application
//!
//! This module wraps the DesktopPlayback system and provides
//! a clean interface for Tauri commands and event emission.

use serde::Serialize;
use soul_audio_desktop::{DesktopPlayback, PlaybackCommand, PlaybackEvent};
use soul_playback::{PlaybackConfig, QueueTrack, RepeatMode, ShuffleMode};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

/// Track info for frontend events (with duration in seconds)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct FrontendTrackEvent {
    id: String,
    title: String,
    artist: String,
    album: Option<String>,
    duration: f64, // seconds
    cover_art_path: Option<String>,
}

impl From<&QueueTrack> for FrontendTrackEvent {
    fn from(track: &QueueTrack) -> Self {
        Self {
            id: track.id.clone(),
            title: track.title.clone(),
            artist: track.artist.clone(),
            album: track.album.clone(),
            duration: track.duration.as_secs_f64(),
            cover_art_path: Some(format!("artwork://track/{}", track.id)),
        }
    }
}

/// Playback manager for Tauri application
///
/// Wraps DesktopPlayback and handles event emission to frontend.
pub struct PlaybackManager {
    playback: Arc<Mutex<DesktopPlayback>>,
    app_handle: AppHandle,
    #[cfg(feature = "effects")]
    effect_slots: Arc<Mutex<[Option<crate::dsp_commands::EffectSlotState>; 4]>>,
}

impl PlaybackManager {
    /// Create a new playback manager
    pub fn new(app_handle: AppHandle) -> Result<Self, String> {
        // Create playback config
        let config = PlaybackConfig {
            history_size: 50,
            volume: 80, // 80%
            shuffle: ShuffleMode::Off,
            repeat: RepeatMode::Off,
            gapless: true,
        };

        // Create desktop playback system
        let playback = DesktopPlayback::new(config).map_err(|e| e.to_string())?;
        let playback = Arc::new(Mutex::new(playback));

        // Start event emission thread
        {
            let playback_clone = Arc::clone(&playback);
            let app_handle_clone = app_handle.clone();

            thread::spawn(move || {
                Self::event_emission_loop(playback_clone, app_handle_clone);
            });
        }

        Ok(Self {
            playback,
            app_handle,
            #[cfg(feature = "effects")]
            effect_slots: Arc::new(Mutex::new([None, None, None, None])),
        })
    }

    /// Event emission loop that runs in background thread
    ///
    /// Polls for playback events and emits them to the frontend via Tauri events.
    fn event_emission_loop(playback: Arc<Mutex<DesktopPlayback>>, app_handle: AppHandle) {
        let mut last_position_emit = std::time::Instant::now();

        loop {
            // Poll for events
            let event = {
                let pb = playback.lock().unwrap();
                pb.try_recv_event()
            };

            if let Some(event) = event {
                // Emit to frontend
                let _ = match &event {
                    PlaybackEvent::StateChanged(state) => {
                        app_handle.emit("playback:state-changed", state)
                    }
                    PlaybackEvent::TrackChanged(track) => {
                        // Convert QueueTrack to FrontendTrackEvent with duration in seconds
                        let frontend_track = track.as_ref().map(FrontendTrackEvent::from);
                        if let Some(ref t) = frontend_track {
                            eprintln!("[playback] Track changed: id={}, title={}, coverArtPath={:?}",
                                t.id, t.title, t.cover_art_path);
                        } else {
                            eprintln!("[playback] Track changed: None");
                        }
                        app_handle.emit("playback:track-changed", frontend_track)
                    }
                    PlaybackEvent::PositionUpdated(position) => {
                        app_handle.emit("playback:position-updated", position)
                    }
                    PlaybackEvent::VolumeChanged(volume) => {
                        app_handle.emit("playback:volume-changed", volume)
                    }
                    PlaybackEvent::QueueUpdated => app_handle.emit("playback:queue-updated", ()),
                    PlaybackEvent::Error(error) => app_handle.emit("playback:error", error),
                };
            }

            // Emit position updates every 250ms during playback
            if last_position_emit.elapsed() >= Duration::from_millis(250) {
                let pb = playback.lock().unwrap();
                let position = pb.get_position();
                let state = pb.get_state();
                drop(pb);

                if state == soul_playback::PlaybackState::Playing {
                    let _ = app_handle.emit("playback:position-updated", position.as_secs_f64());
                }

                last_position_emit = std::time::Instant::now();
            }

            // Sleep briefly to avoid busy-waiting
            thread::sleep(Duration::from_millis(50));
        }
    }

    /// Play a track from local file
    ///
    /// # Arguments
    /// * `track` - Track metadata including file path
    pub fn play_track(&self, track: QueueTrack) -> Result<(), String> {
        let playback = self.playback.lock().map_err(|e| e.to_string())?;

        // Clear queue and add this track
        playback
            .send_command(PlaybackCommand::ClearQueue)
            .map_err(|e| e.to_string())?;

        playback
            .send_command(PlaybackCommand::AddToQueue(track))
            .map_err(|e| e.to_string())?;

        // Start playback
        playback
            .send_command(PlaybackCommand::Play)
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    /// Play
    pub fn play(&self) -> Result<(), String> {
        let playback = self.playback.lock().map_err(|e| e.to_string())?;
        playback
            .send_command(PlaybackCommand::Play)
            .map_err(|e| e.to_string())
    }

    /// Pause
    pub fn pause(&self) -> Result<(), String> {
        let playback = self.playback.lock().map_err(|e| e.to_string())?;
        playback
            .send_command(PlaybackCommand::Pause)
            .map_err(|e| e.to_string())
    }

    /// Stop
    pub fn stop(&self) -> Result<(), String> {
        let playback = self.playback.lock().map_err(|e| e.to_string())?;
        playback
            .send_command(PlaybackCommand::Stop)
            .map_err(|e| e.to_string())
    }

    /// Next track
    pub fn next(&self) -> Result<(), String> {
        let playback = self.playback.lock().map_err(|e| e.to_string())?;
        playback
            .send_command(PlaybackCommand::Next)
            .map_err(|e| e.to_string())
    }

    /// Previous track
    pub fn previous(&self) -> Result<(), String> {
        let playback = self.playback.lock().map_err(|e| e.to_string())?;
        playback
            .send_command(PlaybackCommand::Previous)
            .map_err(|e| e.to_string())
    }

    /// Seek to position (in seconds)
    pub fn seek(&self, position: f64) -> Result<(), String> {
        let playback = self.playback.lock().map_err(|e| e.to_string())?;
        playback
            .send_command(PlaybackCommand::Seek(position))
            .map_err(|e| e.to_string())
    }

    /// Set volume (0-100)
    pub fn set_volume(&self, volume: u8) -> Result<(), String> {
        let volume = volume.clamp(0, 100);
        let playback = self.playback.lock().map_err(|e| e.to_string())?;
        playback
            .send_command(PlaybackCommand::SetVolume(volume))
            .map_err(|e| e.to_string())
    }

    /// Mute
    pub fn mute(&self) -> Result<(), String> {
        let playback = self.playback.lock().map_err(|e| e.to_string())?;
        playback
            .send_command(PlaybackCommand::Mute)
            .map_err(|e| e.to_string())
    }

    /// Unmute
    pub fn unmute(&self) -> Result<(), String> {
        let playback = self.playback.lock().map_err(|e| e.to_string())?;
        playback
            .send_command(PlaybackCommand::Unmute)
            .map_err(|e| e.to_string())
    }

    /// Set shuffle mode
    pub fn set_shuffle(&self, mode: ShuffleMode) -> Result<(), String> {
        let playback = self.playback.lock().map_err(|e| e.to_string())?;
        playback
            .send_command(PlaybackCommand::SetShuffle(mode))
            .map_err(|e| e.to_string())
    }

    /// Set repeat mode
    pub fn set_repeat(&self, mode: RepeatMode) -> Result<(), String> {
        let playback = self.playback.lock().map_err(|e| e.to_string())?;
        playback
            .send_command(PlaybackCommand::SetRepeat(mode))
            .map_err(|e| e.to_string())
    }

    /// Get queue
    pub fn get_queue(&self) -> Vec<QueueTrack> {
        let playback = self.playback.lock().unwrap();
        playback.get_queue()
    }

    /// Check if there is a next track
    pub fn has_next(&self) -> bool {
        let playback = self.playback.lock().unwrap();
        playback.has_next()
    }

    /// Check if there is a previous track
    pub fn has_previous(&self) -> bool {
        let playback = self.playback.lock().unwrap();
        playback.has_previous()
    }

    /// Add track to queue
    pub fn add_to_queue(&self, track: QueueTrack) -> Result<(), String> {
        let playback = self.playback.lock().map_err(|e| e.to_string())?;
        playback
            .send_command(PlaybackCommand::AddToQueue(track))
            .map_err(|e| e.to_string())
    }

    /// Remove track from queue by index
    pub fn remove_from_queue(&self, index: usize) -> Result<(), String> {
        let playback = self.playback.lock().map_err(|e| e.to_string())?;
        playback
            .send_command(PlaybackCommand::RemoveFromQueue(index))
            .map_err(|e| e.to_string())
    }

    /// Clear queue
    pub fn clear_queue(&self) -> Result<(), String> {
        let playback = self.playback.lock().map_err(|e| e.to_string())?;
        playback
            .send_command(PlaybackCommand::ClearQueue)
            .map_err(|e| e.to_string())
    }

    /// Skip to track at queue index
    pub fn skip_to_queue_index(&self, index: usize) -> Result<(), String> {
        let playback = self.playback.lock().map_err(|e| e.to_string())?;
        playback
            .send_command(PlaybackCommand::SkipToQueueIndex(index))
            .map_err(|e| e.to_string())
    }

    /// Load playlist/album as source queue (replaces playback context)
    pub fn load_playlist(&self, tracks: Vec<QueueTrack>) -> Result<(), String> {
        let playback = self.playback.lock().map_err(|e| e.to_string())?;
        playback
            .send_command(PlaybackCommand::LoadPlaylist(tracks))
            .map_err(|e| e.to_string())
    }

    /// Switch audio output device
    ///
    /// # Arguments
    /// * `backend` - Audio backend to use
    /// * `device_name` - Device name to switch to (None for default device)
    ///
    /// # Returns
    /// * `Ok(())` - Device switched successfully
    /// * `Err(_)` - Failed to switch device
    pub fn switch_device(
        &self,
        backend: soul_audio_desktop::AudioBackend,
        device_name: Option<String>,
    ) -> Result<(), String> {
        let mut playback = self.playback.lock().map_err(|e| e.to_string())?;
        playback
            .switch_device(backend, device_name)
            .map_err(|e| e.to_string())
    }

    /// Get current audio backend
    pub fn get_current_backend(&self) -> soul_audio_desktop::AudioBackend {
        let playback = self.playback.lock().unwrap();
        playback.get_current_backend()
    }

    /// Get current device name
    pub fn get_current_device(&self) -> String {
        let playback = self.playback.lock().unwrap();
        playback.get_current_device()
    }

    // ===== DSP Effect Chain =====

    /// Get effect slots state
    #[cfg(feature = "effects")]
    pub fn get_effect_slots(&self) -> Result<[Option<crate::dsp_commands::EffectSlotState>; 4], String> {
        let slots = self.effect_slots.lock().map_err(|e| e.to_string())?;
        Ok(slots.clone())
    }

    /// Set effect in a slot and rebuild the effect chain
    #[cfg(feature = "effects")]
    pub fn set_effect_slot(&self, slot_index: usize, effect: Option<crate::dsp_commands::EffectSlotState>) -> Result<(), String> {
        if slot_index >= 4 {
            return Err("Slot index must be 0-3".to_string());
        }

        // Update slot
        {
            let mut slots = self.effect_slots.lock().map_err(|e| e.to_string())?;
            slots[slot_index] = effect;
        }

        // Rebuild effect chain
        self.rebuild_effect_chain()
    }

    /// Rebuild the entire effect chain from current slot state
    #[cfg(feature = "effects")]
    fn rebuild_effect_chain(&self) -> Result<(), String> {
        use soul_audio::effects::{ParametricEq, Compressor, Limiter};
        use crate::dsp_commands::{EffectType, EffectSlotState};

        let slots = self.effect_slots.lock().map_err(|e| e.to_string())?;

        self.with_effect_chain(|chain| {
            // Clear existing effects
            chain.clear();

            // Add effects from slots
            for slot in slots.iter() {
                if let Some(slot_state) = slot {
                    let effect: Box<dyn soul_audio::effects::AudioEffect> = match &slot_state.effect {
                        EffectType::Eq { bands } => {
                            let eq_bands: Vec<_> = bands.iter().map(|b| b.clone().into()).collect();
                            let mut eq = ParametricEq::new(eq_bands);
                            eq.set_enabled(slot_state.enabled);
                            Box::new(eq)
                        }
                        EffectType::Compressor { settings } => {
                            let mut comp = Compressor::new(settings.clone().into());
                            comp.set_enabled(slot_state.enabled);
                            Box::new(comp)
                        }
                        EffectType::Limiter { settings } => {
                            let mut lim = Limiter::new(settings.clone().into());
                            lim.set_enabled(slot_state.enabled);
                            Box::new(lim)
                        }
                    };
                    chain.add_effect(effect);
                }
            }
        })
    }

    /// Access the effect chain for configuration
    #[cfg(feature = "effects")]
    pub fn with_effect_chain<F, R>(&self, f: F) -> Result<R, String>
    where
        F: FnOnce(&mut soul_audio::effects::EffectChain) -> R,
    {
        let mut playback = self.playback.lock().map_err(|e| e.to_string())?;
        let effect_chain = playback.effect_chain_mut();
        Ok(f(effect_chain))
    }
}
