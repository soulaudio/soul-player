//! WASM-compatible PlaybackManager wrapper

use wasm_bindgen::prelude::*;
use js_sys::Function;
use crate::{PlaybackManager, PlaybackConfig, PlaybackError, PlaybackState, ShuffleMode, RepeatMode};
use super::types::WasmQueueTrack;
use std::time::Duration;

/// WASM-compatible playback manager
///
/// This wraps the core PlaybackManager with a JavaScript-friendly API.
#[wasm_bindgen]
pub struct WasmPlaybackManager {
    inner: PlaybackManager,

    // Event callbacks
    on_state_change: Option<Function>,
    on_track_change: Option<Function>,
    on_queue_change: Option<Function>,
    on_error: Option<Function>,
}

#[wasm_bindgen]
impl WasmPlaybackManager {
    /// Create a new playback manager
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        // Enable panic hooks for better error messages in console
        #[cfg(feature = "wasm")]
        console_error_panic_hook::set_once();

        Self {
            inner: PlaybackManager::new(PlaybackConfig::default()),
            on_state_change: None,
            on_track_change: None,
            on_queue_change: None,
            on_error: None,
        }
    }

    // ===== Playback Control =====

    /// Start or resume playback
    pub fn play(&mut self) -> Result<(), JsValue> {
        let old_state = self.inner.get_state();

        self.inner
            .play()
            .map_err(|e| self.handle_error(e))?;

        let new_state = self.inner.get_state();

        self.emit_state_change();

        // Only emit track change if we transitioned to Loading state
        // (meaning a new track is being loaded, not just resuming from pause)
        if new_state == PlaybackState::Loading && old_state != PlaybackState::Paused {
            self.emit_track_change();
        }

        Ok(())
    }

    /// Pause playback
    pub fn pause(&mut self) {
        self.inner.pause();
        self.emit_state_change();
    }

    /// Stop playback
    pub fn stop(&mut self) {
        self.inner.stop();
        self.emit_state_change();
    }

    /// Skip to next track
    pub fn next(&mut self) -> Result<(), JsValue> {
        self.inner.next().map_err(|e| self.handle_error(e))?;
        self.emit_track_change();
        self.emit_queue_change();
        Ok(())
    }

    /// Go to previous track
    pub fn previous(&mut self) -> Result<(), JsValue> {
        self.inner
            .previous()
            .map_err(|e| self.handle_error(e))?;
        self.emit_track_change();
        self.emit_queue_change();
        Ok(())
    }

    // ===== Volume Control =====

    /// Set volume (0-100)
    #[wasm_bindgen(js_name = setVolume)]
    pub fn set_volume(&mut self, level: u8) {
        self.inner.set_volume(level.clamp(0, 100));
    }

    /// Get current volume (0-100)
    #[wasm_bindgen(js_name = getVolume)]
    pub fn get_volume(&self) -> u8 {
        self.inner.get_volume()
    }

    /// Mute audio
    pub fn mute(&mut self) {
        self.inner.mute();
    }

    /// Unmute audio
    pub fn unmute(&mut self) {
        self.inner.unmute();
    }

    /// Toggle mute
    #[wasm_bindgen(js_name = toggleMute)]
    pub fn toggle_mute(&mut self) {
        self.inner.toggle_mute();
    }

    /// Check if muted
    #[wasm_bindgen(js_name = isMuted)]
    pub fn is_muted(&self) -> bool {
        self.inner.is_muted()
    }

    // ===== Seeking =====

    /// Seek to position in seconds
    #[wasm_bindgen(js_name = seekTo)]
    pub fn seek_to(&mut self, position_secs: f64) -> Result<(), JsValue> {
        self.inner
            .seek_to(Duration::from_secs_f64(position_secs))
            .map_err(|e| self.handle_error(e))
    }

    /// Seek to position by percentage (0.0 - 1.0)
    #[wasm_bindgen(js_name = seekToPercent)]
    pub fn seek_to_percent(&mut self, percent: f32) -> Result<(), JsValue> {
        self.inner
            .seek_to_percent(percent)
            .map_err(|e| self.handle_error(e))
    }

    // ===== State Queries =====

    /// Get current playback state as string
    #[wasm_bindgen(js_name = getState)]
    pub fn get_state(&self) -> String {
        match self.inner.get_state() {
            PlaybackState::Stopped => "stopped".to_string(),
            PlaybackState::Playing => "playing".to_string(),
            PlaybackState::Paused => "paused".to_string(),
            PlaybackState::Loading => "loading".to_string(),
        }
    }

    /// Get current position in seconds
    #[wasm_bindgen(js_name = getPosition)]
    pub fn get_position(&self) -> f64 {
        self.inner.get_position().as_secs_f64()
    }

    /// Get duration of current track in seconds
    #[wasm_bindgen(js_name = getDuration)]
    pub fn get_duration(&self) -> Option<f64> {
        self.inner.get_duration().map(|d| d.as_secs_f64())
    }

    /// Get queue length
    #[wasm_bindgen(js_name = queueLength)]
    pub fn queue_length(&self) -> usize {
        self.inner.queue_len()
    }

    /// Check if there is a next track
    #[wasm_bindgen(js_name = hasNext)]
    pub fn has_next(&self) -> bool {
        self.inner.has_next()
    }

    /// Check if there is a previous track
    #[wasm_bindgen(js_name = hasPrevious)]
    pub fn has_previous(&self) -> bool {
        self.inner.has_previous()
    }

    // ===== Queue Management =====

    /// Add track to play next (explicit queue)
    #[wasm_bindgen(js_name = addToQueueNext)]
    pub fn add_to_queue_next(&mut self, track: WasmQueueTrack) {
        self.inner.add_to_queue_next(track.into());
        self.emit_queue_change();
    }

    /// Add track to end of queue (explicit queue)
    #[wasm_bindgen(js_name = addToQueueEnd)]
    pub fn add_to_queue_end(&mut self, track: WasmQueueTrack) {
        self.inner.add_to_queue_end(track.into());
        self.emit_queue_change();
    }

    /// Load playlist as source queue
    #[wasm_bindgen(js_name = loadPlaylist)]
    pub fn load_playlist(&mut self, tracks: JsValue) -> Result<(), JsValue> {
        let wasm_tracks: Vec<WasmQueueTrack> = serde_wasm_bindgen::from_value(tracks)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse tracks: {}", e)))?;

        let queue_tracks: Vec<_> = wasm_tracks.into_iter().map(|t| t.into()).collect();

        self.inner.add_playlist_to_queue(queue_tracks);
        self.emit_queue_change();
        Ok(())
    }

    /// Clear entire queue
    #[wasm_bindgen(js_name = clearQueue)]
    pub fn clear_queue(&mut self) {
        self.inner.clear_queue();
        self.emit_queue_change();
    }

    /// Get all tracks in queue as JSON
    #[wasm_bindgen(js_name = getQueue)]
    pub fn get_queue(&self) -> JsValue {
        let tracks: Vec<WasmQueueTrack> = self
            .inner
            .get_queue()
            .iter()
            .map(|t| WasmQueueTrack::from(*t))
            .collect();

        serde_wasm_bindgen::to_value(&tracks).unwrap_or(JsValue::NULL)
    }

    /// Skip to track at queue index
    #[wasm_bindgen(js_name = skipToQueueIndex)]
    pub fn skip_to_queue_index(&mut self, index: usize) -> Result<(), JsValue> {
        self.inner
            .skip_to_queue_index(index)
            .map_err(|e| self.handle_error(e))?;
        self.emit_track_change();
        self.emit_queue_change();
        Ok(())
    }

    /// Remove track from queue by index
    #[wasm_bindgen(js_name = removeFromQueue)]
    pub fn remove_from_queue(&mut self, index: usize) -> Result<JsValue, JsValue> {
        let removed_track = self.inner
            .remove_from_queue(index)
            .map_err(|e| self.handle_error(e))?;

        let wasm_track = WasmQueueTrack::from(&removed_track);
        self.emit_queue_change();

        serde_wasm_bindgen::to_value(&wasm_track)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }

    /// Append tracks to existing queue
    #[wasm_bindgen(js_name = appendToQueue)]
    pub fn append_to_queue(&mut self, tracks: JsValue) -> Result<(), JsValue> {
        let wasm_tracks: Vec<WasmQueueTrack> = serde_wasm_bindgen::from_value(tracks)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse tracks: {}", e)))?;

        let queue_tracks: Vec<_> = wasm_tracks.into_iter().map(|t| t.into()).collect();

        self.inner.append_to_queue(queue_tracks);
        self.emit_queue_change();
        Ok(())
    }

    /// Get playback history
    #[wasm_bindgen(js_name = getHistory)]
    pub fn get_history(&self) -> JsValue {
        let history: Vec<WasmQueueTrack> = self
            .inner
            .get_history()
            .iter()
            .map(|t| WasmQueueTrack::from(*t))
            .collect();

        serde_wasm_bindgen::to_value(&history).unwrap_or(JsValue::NULL)
    }

    // ===== Shuffle & Repeat =====

    /// Set shuffle mode ("off" | "random" | "smart")
    #[wasm_bindgen(js_name = setShuffle)]
    pub fn set_shuffle(&mut self, mode: &str) -> Result<(), JsValue> {
        let shuffle = match mode {
            "off" => ShuffleMode::Off,
            "random" => ShuffleMode::Random,
            "smart" => ShuffleMode::Smart,
            _ => return Err(JsValue::from_str("Invalid shuffle mode. Use 'off', 'random', or 'smart'")),
        };

        self.inner.set_shuffle(shuffle);
        self.emit_queue_change();
        Ok(())
    }

    /// Get current shuffle mode
    #[wasm_bindgen(js_name = getShuffle)]
    pub fn get_shuffle(&self) -> String {
        match self.inner.get_shuffle() {
            ShuffleMode::Off => "off".to_string(),
            ShuffleMode::Random => "random".to_string(),
            ShuffleMode::Smart => "smart".to_string(),
        }
    }

    /// Set repeat mode ("off" | "all" | "one")
    #[wasm_bindgen(js_name = setRepeat)]
    pub fn set_repeat(&mut self, mode: &str) -> Result<(), JsValue> {
        let repeat = match mode {
            "off" => RepeatMode::Off,
            "all" => RepeatMode::All,
            "one" => RepeatMode::One,
            _ => return Err(JsValue::from_str("Invalid repeat mode. Use 'off', 'all', or 'one'")),
        };

        self.inner.set_repeat(repeat);
        Ok(())
    }

    /// Get current repeat mode
    #[wasm_bindgen(js_name = getRepeat)]
    pub fn get_repeat(&self) -> String {
        match self.inner.get_repeat() {
            RepeatMode::Off => "off".to_string(),
            RepeatMode::All => "all".to_string(),
            RepeatMode::One => "one".to_string(),
        }
    }

    // ===== Event Listeners =====

    /// Register state change callback
    #[wasm_bindgen(js_name = onStateChange)]
    pub fn on_state_change(&mut self, callback: Function) {
        self.on_state_change = Some(callback);
    }

    /// Register track change callback
    #[wasm_bindgen(js_name = onTrackChange)]
    pub fn on_track_change(&mut self, callback: Function) {
        self.on_track_change = Some(callback);
    }

    /// Register queue change callback
    #[wasm_bindgen(js_name = onQueueChange)]
    pub fn on_queue_change(&mut self, callback: Function) {
        self.on_queue_change = Some(callback);
    }

    /// Register error callback
    #[wasm_bindgen(js_name = onError)]
    pub fn on_error(&mut self, callback: Function) {
        self.on_error = Some(callback);
    }

    // ===== Internal Event Emitters =====

    fn emit_state_change(&self) {
        if let Some(ref cb) = self.on_state_change {
            let state = self.get_state();
            cb.call1(&JsValue::NULL, &JsValue::from_str(&state))
                .ok();
        }
    }

    fn emit_track_change(&self) {
        if let Some(ref cb) = self.on_track_change {
            if let Some(track) = self.inner.get_current_track() {
                let wasm_track = WasmQueueTrack::from(track);
                if let Ok(js_track) = serde_wasm_bindgen::to_value(&wasm_track) {
                    cb.call1(&JsValue::NULL, &js_track).ok();
                }
            } else {
                cb.call1(&JsValue::NULL, &JsValue::NULL).ok();
            }
        }
    }

    fn emit_queue_change(&self) {
        if let Some(ref cb) = self.on_queue_change {
            cb.call0(&JsValue::NULL).ok();
        }
    }

    fn handle_error(&self, error: PlaybackError) -> JsValue {
        let err_msg = error.to_string();

        // Emit error event
        if let Some(ref cb) = self.on_error {
            cb.call1(&JsValue::NULL, &JsValue::from_str(&err_msg))
                .ok();
        }

        JsValue::from_str(&err_msg)
    }
}

/// Default implementation
impl Default for WasmPlaybackManager {
    fn default() -> Self {
        Self::new()
    }
}
