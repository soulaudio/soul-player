//! Lazy-initialized playback manager
//!
//! This module provides a LazyPlaybackManager that defers audio engine
//! initialization until the first playback request. This dramatically
//! improves startup time by avoiding expensive audio device enumeration
//! and stream initialization during app launch.
//!
//! Performance impact:
//! - Removes 200-800ms from startup (macOS CoreAudio can take 300-1000ms)
//! - Audio engine initializes on first play/pause/skip command
//! - Settings are restored in background after initialization

use crate::app_state::AppState;
use crate::playback::PlaybackManager;
use crate::{audio_settings, dsp_commands, loudness};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::OnceCell;

/// Lazy-initialized playback manager
///
/// Audio engine only initializes on first playback request.
/// Subsequent requests use the initialized instance instantly.
pub struct LazyPlaybackManager {
    inner: Arc<OnceCell<PlaybackManager>>,
    app: AppHandle,
}

impl LazyPlaybackManager {
    /// Create a new lazy playback manager
    ///
    /// The audio engine is NOT initialized at this point.
    /// Initialization happens on first call to `get()`.
    pub fn new(app: AppHandle) -> Self {
        tracing::info!("[LazyPlaybackManager] Created (audio engine NOT initialized yet)");
        Self {
            inner: Arc::new(OnceCell::new()),
            app,
        }
    }

    /// Get or initialize the playback manager
    ///
    /// First call initializes the audio engine (~200-800ms on macOS).
    /// Subsequent calls return the initialized instance instantly.
    ///
    /// Settings (audio device, DSP, volume leveling) are restored
    /// immediately after initialization completes. This is still deferred
    /// from app startup (happens on first playback), but adds ~50-100ms
    /// to the first playback command.
    pub async fn get(&self) -> Result<&PlaybackManager, String> {
        self.inner
            .get_or_try_init(|| async {
                tracing::info!("🎵 First playback request - initializing audio engine...");
                let start = std::time::Instant::now();

                // Initialize PlaybackManager (audio device enumeration + stream creation)
                let pm = PlaybackManager::new(self.app.clone())
                    .map_err(|e| format!("Failed to initialize playback: {}", e))?;

                let init_duration = start.elapsed();
                tracing::info!(
                    "🎵 Audio engine initialized in {}ms",
                    init_duration.as_millis()
                );

                // Emit event to frontend (optional: show "Audio ready" toast)
                if let Err(e) = self.app.emit("audio:initialized", ()) {
                    tracing::warn!("Failed to emit audio:initialized event: {}", e);
                }

                // Restore audio settings (device, volume leveling, DSP chain)
                // This happens synchronously but is still deferred from app startup
                let state = self.app.state::<AppState>();
                restore_audio_settings(&pm, &state).await;

                Ok(pm)
            })
            .await
    }

    /// Check if the audio engine is initialized
    ///
    /// This does NOT trigger initialization if not already done.
    /// Useful for checking state without side effects.
    #[allow(dead_code)]
    pub fn is_initialized(&self) -> bool {
        self.inner.get().is_some()
    }
}

/// Restore audio settings after audio engine initialization
///
/// This happens synchronously after the audio engine initializes on first playback.
/// Settings restored: audio device, volume leveling mode, DSP effect chain.
async fn restore_audio_settings(playback: &PlaybackManager, state: &AppState) {
    let start = std::time::Instant::now();
    tracing::info!("[LazyPlayback] Restoring audio settings...");

    let mut errors = Vec::new();

    // Restore audio device
    if let Err(e) = audio_settings::initialize_audio_device(playback, state).await {
        tracing::warn!("[LazyPlayback] Failed to restore audio device: {}", e);
        errors.push(format!("Audio device: {}", e));
    }

    // Restore volume leveling
    if let Err(e) = loudness::initialize_volume_leveling_mode(playback, state).await {
        tracing::warn!("[LazyPlayback] Failed to restore volume leveling: {}", e);
        errors.push(format!("Volume leveling: {}", e));
    }

    // Restore DSP chain (this function logs its own errors)
    dsp_commands::restore_dsp_chain_from_database(playback, &state.pool, &state.user_id).await;

    let restore_duration = start.elapsed();
    if errors.is_empty() {
        tracing::info!(
            "[LazyPlayback] Audio settings restored in {}ms",
            restore_duration.as_millis()
        );
    } else {
        tracing::warn!(
            "[LazyPlayback] Audio settings partially restored in {}ms ({} errors)",
            restore_duration.as_millis(),
            errors.len()
        );
    }
}
