//! WASM-compatible type definitions

use crate::{QueueTrack, TrackSource};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use wasm_bindgen::prelude::*;

/// WASM-compatible queue track
///
/// This is a simplified version of QueueTrack that uses JS-compatible types
/// (String instead of PathBuf, f64 instead of Duration).
#[derive(Serialize, Deserialize, Clone, Debug)]
#[wasm_bindgen]
pub struct WasmQueueTrack {
    id: String,
    path: String,
    title: String,
    artist: String,
    album: Option<String>,
    duration_secs: f64,
    track_number: Option<u32>,
}

#[wasm_bindgen]
impl WasmQueueTrack {
    /// Create a new queue track
    #[wasm_bindgen(constructor)]
    pub fn new(
        id: String,
        path: String,
        title: String,
        artist: String,
        duration_secs: f64,
    ) -> Self {
        Self {
            id,
            path,
            title,
            artist,
            album: None,
            duration_secs,
            track_number: None,
        }
    }

    // Getters for all fields
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.id.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn path(&self) -> String {
        self.path.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn title(&self) -> String {
        self.title.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn artist(&self) -> String {
        self.artist.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn album(&self) -> Option<String> {
        self.album.clone()
    }

    #[wasm_bindgen(getter, js_name = durationSecs)]
    pub fn duration_secs(&self) -> f64 {
        self.duration_secs
    }

    #[wasm_bindgen(getter, js_name = trackNumber)]
    pub fn track_number(&self) -> Option<u32> {
        self.track_number
    }

    // Setters
    #[wasm_bindgen(setter)]
    pub fn set_album(&mut self, album: Option<String>) {
        self.album = album;
    }

    #[wasm_bindgen(setter, js_name = trackNumber)]
    pub fn set_track_number(&mut self, track_number: Option<u32>) {
        self.track_number = track_number;
    }
}

// Conversion from internal QueueTrack to WASM type
impl From<&QueueTrack> for WasmQueueTrack {
    fn from(track: &QueueTrack) -> Self {
        Self {
            id: track.id.clone(),
            path: track.path.to_string_lossy().to_string(),
            title: track.title.clone(),
            artist: track.artist.clone(),
            album: track.album.clone(),
            duration_secs: track.duration.as_secs_f64(),
            track_number: track.track_number,
        }
    }
}

// Conversion from WASM type to internal QueueTrack
impl From<WasmQueueTrack> for QueueTrack {
    fn from(track: WasmQueueTrack) -> Self {
        Self {
            id: track.id,
            path: PathBuf::from(track.path),
            title: track.title,
            artist: track.artist,
            album: track.album,
            duration: Duration::from_secs_f64(track.duration_secs),
            track_number: track.track_number,
            source: TrackSource::Single,
        }
    }
}
