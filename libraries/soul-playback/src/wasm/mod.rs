//! WASM bindings for soul-playback
//!
//! This module provides WebAssembly bindings for the PlaybackManager,
//! allowing the core playback logic to be used in web browsers.

#[cfg(feature = "wasm")]
pub mod types;

#[cfg(feature = "wasm")]
pub mod manager;

#[cfg(feature = "wasm")]
pub use manager::WasmPlaybackManager;

#[cfg(feature = "wasm")]
pub use types::WasmQueueTrack;
