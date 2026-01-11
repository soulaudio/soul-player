//! Playback management for Tauri desktop application
//!
//! This module wraps the DesktopPlayback system and provides
//! a clean interface for Tauri commands and event emission.

use serde::Serialize;
use soul_audio_desktop::{
    DesktopPlayback, ExclusiveConfig, LatencyInfo, PlaybackCommand, PlaybackEvent,
};
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
        let config = PlaybackConfig::default();

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
    /// Also polls for device sample rate changes periodically.
    fn event_emission_loop(playback: Arc<Mutex<DesktopPlayback>>, app_handle: AppHandle) {
        let mut last_position_emit = std::time::Instant::now();
        let mut last_sample_rate_check = std::time::Instant::now();

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
                            eprintln!(
                                "[playback] Track changed: id={}, title={}, coverArtPath={:?}",
                                t.id, t.title, t.cover_art_path
                            );
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
                    PlaybackEvent::SampleRateChanged(from, to) => {
                        eprintln!("[playback] Sample rate changed: {}Hz -> {}Hz", from, to);
                        app_handle.emit(
                            "playback:sample-rate-changed",
                            serde_json::json!({
                                "from": from,
                                "to": to
                            }),
                        )
                    }
                    PlaybackEvent::CrossfadeStarted {
                        from_track_id,
                        to_track_id,
                        duration_ms,
                    } => {
                        eprintln!(
                            "[playback] Crossfade started: {} -> {} ({}ms)",
                            from_track_id, to_track_id, duration_ms
                        );
                        app_handle.emit(
                            "playback:crossfade-started",
                            serde_json::json!({
                                "from_track_id": from_track_id,
                                "to_track_id": to_track_id,
                                "duration_ms": duration_ms
                            }),
                        )
                    }
                    PlaybackEvent::CrossfadeProgress {
                        progress,
                        metadata_switched,
                    } => {
                        // Only emit occasionally to avoid flooding the frontend
                        // The actual track change is emitted via TrackChanged at 50%
                        app_handle.emit(
                            "playback:crossfade-progress",
                            serde_json::json!({
                                "progress": progress,
                                "metadata_switched": metadata_switched
                            }),
                        )
                    }
                    PlaybackEvent::CrossfadeCompleted => {
                        eprintln!("[playback] Crossfade completed");
                        app_handle.emit("playback:crossfade-completed", ())
                    }
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

            // Check for device sample rate changes every 2 seconds
            // This detects when the user changes the device's sample rate externally
            // (e.g., via ASIO control panel or Windows sound settings)
            if last_sample_rate_check.elapsed() >= Duration::from_secs(2) {
                let mut pb = playback.lock().unwrap();
                match pb.check_and_update_sample_rate() {
                    Ok(true) => {
                        eprintln!("[playback] Device sample rate changed, stream recreated");
                    }
                    Ok(false) => {
                        // No change, nothing to do
                    }
                    Err(e) => {
                        // Don't spam errors, just log once per failure
                        eprintln!("[playback] Failed to check sample rate: {}", e);
                    }
                }
                drop(pb);
                last_sample_rate_check = std::time::Instant::now();
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

    /// Get current playback state
    pub fn get_state(&self) -> soul_playback::PlaybackState {
        let playback = self.playback.lock().unwrap();
        playback.get_state()
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
        eprintln!("[PlaybackManager] switch_device called, acquiring lock...");
        let mut playback = self.playback.lock().map_err(|e| e.to_string())?;
        eprintln!("[PlaybackManager] Lock acquired, calling DesktopPlayback::switch_device...");
        let result = playback
            .switch_device(backend, device_name)
            .map_err(|e| e.to_string());
        eprintln!("[PlaybackManager] DesktopPlayback::switch_device returned, releasing lock...");
        // Explicitly drop the guard to release the lock
        drop(playback);
        eprintln!("[PlaybackManager] Lock released, returning result");
        result
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

    /// Get current sample rate
    pub fn get_current_sample_rate(&self) -> u32 {
        let playback = self.playback.lock().unwrap();
        playback.get_current_sample_rate()
    }

    /// Manually trigger a sample rate check and update
    ///
    /// This is useful when the user knows they've changed device settings
    /// and wants to immediately update without waiting for the next poll.
    ///
    /// # Returns
    /// * `Ok(true)` - Sample rate changed and stream was recreated
    /// * `Ok(false)` - Sample rate unchanged
    /// * `Err(_)` - Failed to check or update
    pub fn refresh_sample_rate(&self) -> Result<bool, String> {
        let mut playback = self.playback.lock().map_err(|e| e.to_string())?;
        playback
            .check_and_update_sample_rate()
            .map_err(|e| e.to_string())
    }

    // ===== DSP Effect Chain =====

    /// Get effect slots state
    #[cfg(feature = "effects")]
    pub fn get_effect_slots(
        &self,
    ) -> Result<[Option<crate::dsp_commands::EffectSlotState>; 4], String> {
        let slots = self.effect_slots.lock().map_err(|e| e.to_string())?;
        Ok(slots.clone())
    }

    /// Set effect in a slot and rebuild the effect chain
    #[cfg(feature = "effects")]
    pub fn set_effect_slot(
        &self,
        slot_index: usize,
        effect: Option<crate::dsp_commands::EffectSlotState>,
    ) -> Result<(), String> {
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

    /// Update effect parameters in-place WITHOUT rebuilding the chain
    ///
    /// This preserves filter states and prevents audio artifacts (sizzle/pops)
    /// that occur when effects are recreated during parameter drags.
    #[cfg(feature = "effects")]
    pub fn update_effect_parameters_in_place(
        &self,
        slot_index: usize,
        effect: &crate::dsp_commands::EffectType,
    ) -> Result<bool, String> {
        use crate::dsp_commands::EffectType;
        use soul_audio::effects::{
            Compressor, Crossfeed, CrossfeedPreset, GraphicEq, Limiter, ParametricEq,
            StereoEnhancer,
        };

        if slot_index >= 4 {
            return Err("Slot index must be 0-3".to_string());
        }

        // Try to update in-place
        let updated = self.with_effect_chain(|chain| {
            match effect {
                EffectType::Eq { bands } => {
                    if let Some(eq) = chain.get_effect_as_mut::<ParametricEq>(slot_index) {
                        eq.set_bands(bands.iter().map(|b| b.clone().into()).collect());
                        true
                    } else {
                        false
                    }
                }
                EffectType::GraphicEq { settings } => {
                    if let Some(geq) = chain.get_effect_as_mut::<GraphicEq>(slot_index) {
                        for (i, &gain) in settings.gains.iter().enumerate() {
                            geq.set_band_gain(i, gain);
                        }
                        true
                    } else {
                        false
                    }
                }
                EffectType::Limiter { settings } => {
                    if let Some(lim) = chain.get_effect_as_mut::<Limiter>(slot_index) {
                        lim.set_threshold(settings.threshold_db);
                        lim.set_release(settings.release_ms);
                        true
                    } else {
                        false
                    }
                }
                EffectType::Compressor { settings } => {
                    if let Some(comp) = chain.get_effect_as_mut::<Compressor>(slot_index) {
                        // Use set_settings to update all parameters including knee
                        comp.set_settings(settings.clone().into());
                        true
                    } else {
                        false
                    }
                }
                EffectType::Stereo { settings } => {
                    if let Some(stereo) = chain.get_effect_as_mut::<StereoEnhancer>(slot_index) {
                        stereo.set_width(settings.width);
                        stereo.set_mid_gain_db(settings.mid_gain_db);
                        stereo.set_side_gain_db(settings.side_gain_db);
                        stereo.set_balance(settings.balance);
                        true
                    } else {
                        false
                    }
                }
                EffectType::Crossfeed { settings } => {
                    if let Some(cf) = chain.get_effect_as_mut::<Crossfeed>(slot_index) {
                        let preset = match settings.preset.as_str() {
                            "natural" => CrossfeedPreset::Natural,
                            "relaxed" => CrossfeedPreset::Relaxed,
                            "meier" => CrossfeedPreset::Meier,
                            _ => CrossfeedPreset::Custom,
                        };

                        if preset == CrossfeedPreset::Custom {
                            cf.set_level_db(settings.level_db);
                            cf.set_cutoff_hz(settings.cutoff_hz);
                        } else {
                            cf.set_preset(preset);
                        }
                        true
                    } else {
                        false
                    }
                }
                EffectType::Convolution { .. } => {
                    // Convolution can't be updated in-place (needs IR reload)
                    false
                }
            }
        })?;

        // Also update the stored slot state
        if updated {
            let mut slots = self.effect_slots.lock().map_err(|e| e.to_string())?;
            if let Some(ref mut slot_state) = slots[slot_index] {
                slot_state.effect = effect.clone();
            }
        }

        Ok(updated)
    }

    /// Rebuild the entire effect chain from current slot state
    #[cfg(feature = "effects")]
    fn rebuild_effect_chain(&self) -> Result<(), String> {
        use crate::dsp_commands::EffectType;
        use soul_audio::effects::{
            AudioEffect, Compressor, ConvolutionEngine, Crossfeed, CrossfeedPreset,
            CrossfeedSettings, GraphicEq, GraphicEqBands, Limiter, ParametricEq, StereoEnhancer,
            StereoSettings,
        };

        let slots = self.effect_slots.lock().map_err(|e| e.to_string())?;

        self.with_effect_chain(|chain| {
            // Clear existing effects
            chain.clear();

            // Add effects from slots
            for slot in slots.iter() {
                if let Some(slot_state) = slot {
                    let effect: Box<dyn soul_audio::effects::AudioEffect> = match &slot_state.effect
                    {
                        EffectType::Eq { bands } => {
                            let mut eq = ParametricEq::new();
                            eq.set_bands(bands.iter().map(|b| b.clone().into()).collect());
                            eq.set_enabled(slot_state.enabled);
                            Box::new(eq)
                        }
                        EffectType::Compressor { settings } => {
                            let mut comp = Compressor::with_settings(settings.clone().into());
                            comp.set_enabled(slot_state.enabled);
                            Box::new(comp)
                        }
                        EffectType::Limiter { settings } => {
                            let mut lim = Limiter::with_settings(settings.clone().into());
                            lim.set_enabled(slot_state.enabled);
                            Box::new(lim)
                        }
                        EffectType::Crossfeed { settings } => {
                            let preset = match settings.preset.as_str() {
                                "natural" => CrossfeedPreset::Natural,
                                "relaxed" => CrossfeedPreset::Relaxed,
                                "meier" => CrossfeedPreset::Meier,
                                _ => CrossfeedPreset::Custom,
                            };

                            let crossfeed_settings = if preset == CrossfeedPreset::Custom {
                                CrossfeedSettings::custom(settings.level_db, settings.cutoff_hz)
                            } else {
                                CrossfeedSettings::from_preset(preset)
                            };

                            let mut crossfeed = Crossfeed::with_settings(crossfeed_settings);
                            crossfeed.set_enabled(slot_state.enabled);
                            Box::new(crossfeed)
                        }
                        EffectType::Stereo { settings } => {
                            let stereo_settings = StereoSettings {
                                width: settings.width,
                                mid_gain_db: settings.mid_gain_db,
                                side_gain_db: settings.side_gain_db,
                                balance: settings.balance,
                            };

                            let mut stereo = StereoEnhancer::with_settings(stereo_settings);
                            stereo.set_enabled(slot_state.enabled);
                            Box::new(stereo)
                        }
                        EffectType::GraphicEq { settings } => {
                            let mut graphic_eq = if settings.band_count == 31 {
                                GraphicEq::new(GraphicEqBands::ThirtyOne)
                            } else {
                                GraphicEq::new_10_band()
                            };

                            // Apply gains if we have the right number
                            if settings.band_count == 10 && settings.gains.len() == 10 {
                                if let Ok(gains) = settings.gains.clone().try_into() {
                                    graphic_eq.set_gains_10(gains);
                                }
                            } else {
                                // For 31-band or custom, set each band individually
                                for (i, &gain) in settings.gains.iter().enumerate() {
                                    graphic_eq.set_band_gain(i, gain);
                                }
                            }

                            graphic_eq.set_enabled(slot_state.enabled);
                            Box::new(graphic_eq)
                        }
                        EffectType::Convolution { settings } => {
                            let mut conv = ConvolutionEngine::new();

                            // Load IR from file path if provided
                            if !settings.ir_file_path.is_empty() {
                                match conv.load_from_wav(&settings.ir_file_path) {
                                    Ok(()) => {
                                        conv.set_dry_wet_mix(settings.wet_dry_mix);
                                        // Note: pre_delay_ms and decay are UI-only for now
                                        // The ConvolutionEngine applies full IR as-is
                                        eprintln!(
                                            "[rebuild_effect_chain] Loaded IR: {}",
                                            settings.ir_file_path
                                        );
                                    }
                                    Err(e) => {
                                        eprintln!(
                                            "[rebuild_effect_chain] Failed to load IR file '{}': {}",
                                            settings.ir_file_path, e
                                        );
                                        // Keep the engine but it won't process anything
                                    }
                                }
                            }

                            conv.set_enabled(slot_state.enabled);
                            Box::new(conv)
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
        let playback = self.playback.lock().map_err(|e| e.to_string())?;
        Ok(playback.with_effect_chain(f))
    }

    // ===== Volume Leveling =====

    /// Set volume leveling mode (ReplayGain track/album, EBU R128, etc.)
    pub fn set_volume_leveling_mode(&self, mode: soul_playback::NormalizationMode) {
        let playback = self.playback.lock().unwrap();
        playback.set_volume_leveling_mode(mode);
    }

    /// Get current volume leveling mode
    pub fn get_volume_leveling_mode(&self) -> soul_playback::NormalizationMode {
        let playback = self.playback.lock().unwrap();
        playback.get_volume_leveling_mode()
    }

    /// Set track gain for current track (called when loading track)
    pub fn set_track_gain(&self, gain_db: f64, peak_dbfs: f64) {
        let playback = self.playback.lock().unwrap();
        playback.set_track_gain(gain_db, peak_dbfs);
    }

    /// Set album gain for current track (called when loading track)
    pub fn set_album_gain(&self, gain_db: f64, peak_dbfs: f64) {
        let playback = self.playback.lock().unwrap();
        playback.set_album_gain(gain_db, peak_dbfs);
    }

    /// Clear gain values (for new track without loudness data)
    pub fn clear_loudness_gains(&self) {
        let playback = self.playback.lock().unwrap();
        playback.clear_loudness_gains();
    }

    /// Set pre-amp gain for volume leveling
    pub fn set_loudness_preamp(&self, preamp_db: f64) {
        let playback = self.playback.lock().unwrap();
        playback.set_loudness_preamp(preamp_db);
    }

    /// Set whether to prevent clipping during volume leveling
    pub fn set_prevent_clipping(&self, prevent: bool) {
        let playback = self.playback.lock().unwrap();
        playback.set_prevent_clipping(prevent);
    }

    // ===== Exclusive Mode / Bit-Perfect Output =====

    /// Get current latency information
    pub fn get_latency_info(&self) -> LatencyInfo {
        let playback = self.playback.lock().unwrap();
        playback.get_latency_info()
    }

    /// Enable exclusive mode with configuration
    ///
    /// This switches to WASAPI exclusive mode (Windows) or maintains
    /// ASIO/JACK if configured, providing:
    /// - Bit-perfect output (no OS mixer)
    /// - Lower latency
    /// - Direct sample format control
    pub fn set_exclusive_mode(&self, config: ExclusiveConfig) -> Result<LatencyInfo, String> {
        let mut playback = self.playback.lock().map_err(|e| e.to_string())?;
        playback
            .set_exclusive_mode(config)
            .map_err(|e| e.to_string())
    }

    /// Disable exclusive mode (return to shared mode)
    pub fn disable_exclusive_mode(&self) -> Result<(), String> {
        let mut playback = self.playback.lock().map_err(|e| e.to_string())?;
        playback.disable_exclusive_mode().map_err(|e| e.to_string())
    }

    /// Check if currently in exclusive mode
    pub fn is_exclusive_mode(&self) -> bool {
        let playback = self.playback.lock().unwrap();
        playback.is_exclusive_mode()
    }

    // ===== Crossfade Settings =====

    /// Set crossfade enabled/disabled
    pub fn set_crossfade_enabled(&self, enabled: bool) {
        let playback = self.playback.lock().unwrap();
        playback.set_crossfade_enabled(enabled);
    }

    /// Get current crossfade enabled state
    pub fn is_crossfade_enabled(&self) -> bool {
        let playback = self.playback.lock().unwrap();
        playback.is_crossfade_enabled()
    }

    /// Set crossfade duration in milliseconds
    pub fn set_crossfade_duration(&self, duration_ms: u32) {
        let playback = self.playback.lock().unwrap();
        playback.set_crossfade_duration(duration_ms);
    }

    /// Get crossfade duration in milliseconds
    pub fn get_crossfade_duration(&self) -> u32 {
        let playback = self.playback.lock().unwrap();
        playback.get_crossfade_duration()
    }

    /// Set crossfade curve type
    pub fn set_crossfade_curve(&self, curve: soul_playback::FadeCurve) {
        let playback = self.playback.lock().unwrap();
        playback.set_crossfade_curve(curve);
    }

    /// Get crossfade curve type
    pub fn get_crossfade_curve(&self) -> soul_playback::FadeCurve {
        let playback = self.playback.lock().unwrap();
        playback.get_crossfade_curve()
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
    /// Changes apply when the next track is loaded.
    pub fn set_resampling_quality(&self, quality: &str) -> Result<(), String> {
        let mut playback = self.playback.lock().unwrap();
        playback.set_resampling_quality(quality)
    }

    /// Get current resampling quality preset
    pub fn get_resampling_quality(&self) -> String {
        let playback = self.playback.lock().unwrap();
        playback.get_resampling_quality()
    }

    /// Set resampling target sample rate
    ///
    /// - rate=0: Auto mode - match device native sample rate
    /// - rate>0: Force specific output sample rate (e.g., 96000)
    ///
    /// Changes apply when the next track is loaded.
    pub fn set_resampling_target_rate(&self, rate: u32) -> Result<(), String> {
        let mut playback = self.playback.lock().unwrap();
        playback.set_resampling_target_rate(rate)
    }

    /// Get current resampling target sample rate
    ///
    /// Returns 0 for auto mode, or the specific rate in Hz.
    pub fn get_resampling_target_rate(&self) -> u32 {
        let playback = self.playback.lock().unwrap();
        playback.get_resampling_target_rate()
    }

    /// Set resampling backend
    ///
    /// Backends:
    /// - "auto": Use best available (r8brain if compiled in, else rubato)
    /// - "rubato": Use Rubato library (always available)
    /// - "r8brain": Use r8brain library (requires feature flag)
    ///
    /// Changes apply when the next track is loaded.
    pub fn set_resampling_backend(&self, backend: &str) -> Result<(), String> {
        let mut playback = self.playback.lock().unwrap();
        playback.set_resampling_backend(backend)
    }

    /// Get current resampling backend
    pub fn get_resampling_backend(&self) -> String {
        let playback = self.playback.lock().unwrap();
        playback.get_resampling_backend()
    }

    // ===== Headroom Management =====

    /// Set headroom management mode
    pub fn set_headroom_mode(&self, mode: soul_playback::HeadroomMode) {
        let playback = self.playback.lock().unwrap();
        playback.set_headroom_mode(mode);
    }

    /// Get current headroom mode
    pub fn get_headroom_mode(&self) -> soul_playback::HeadroomMode {
        let playback = self.playback.lock().unwrap();
        playback.get_headroom_mode()
    }

    /// Set headroom enabled state
    pub fn set_headroom_enabled(&self, enabled: bool) {
        let playback = self.playback.lock().unwrap();
        playback.set_headroom_enabled(enabled);
    }

    /// Check if headroom management is enabled
    pub fn is_headroom_enabled(&self) -> bool {
        let playback = self.playback.lock().unwrap();
        playback.is_headroom_enabled()
    }

    /// Set EQ boost value for headroom calculation
    pub fn set_headroom_eq_boost_db(&self, boost_db: f64) {
        let playback = self.playback.lock().unwrap();
        playback.set_headroom_eq_boost_db(boost_db);
    }

    /// Set pre-amp value for headroom calculation
    pub fn set_headroom_preamp_db(&self, preamp_db: f64) {
        let playback = self.playback.lock().unwrap();
        playback.set_headroom_preamp_db(preamp_db);
    }

    /// Get total potential gain from all sources
    pub fn get_headroom_total_gain_db(&self) -> f64 {
        let playback = self.playback.lock().unwrap();
        playback.get_headroom_total_gain_db()
    }

    /// Get current attenuation being applied
    pub fn get_headroom_attenuation_db(&self) -> f64 {
        let playback = self.playback.lock().unwrap();
        playback.get_headroom_attenuation_db()
    }
}
