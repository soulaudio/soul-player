//! Playback timing constants shared between backend and frontend
//!
//! This module defines timing constants used for position updates and event handling.
//! These values are exported to the frontend via Tauri commands to ensure synchronization.

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Default position update interval in milliseconds
///
/// This determines how frequently the backend emits position updates during playback.
/// 100ms provides the best balance for responsive seeking and smooth progress:
/// - UI responsiveness: 10 updates/second for smooth progress bar
/// - Seek latency: ~220ms total (100ms update + 120ms ignore window)
/// - CPU usage: Negligible overhead (modern systems handle 100ms intervals easily)
/// - Event system: Well within acceptable limits
///
/// **Why 100ms?**
/// - Seeks feel instant (<250ms is perceived as immediate by users)
/// - Progress bar updates are smooth (10fps is sufficient for linear progress)
/// - Ignore window is minimal (120ms = 100ms * 1.2)
/// - Recommended by audio player best practices (react-h5-audio-player, wavesurfer.js)
///
/// Previous value of 500ms caused 1100ms seek latency (500ms + 600ms ignore),
/// making seeks feel sluggish and unresponsive.
pub const DEFAULT_POSITION_UPDATE_INTERVAL_MS: u64 = 100;

/// Minimum allowed position update interval in milliseconds
///
/// Prevents excessive event traffic that could overwhelm the event system
/// or cause performance issues.
pub const MIN_POSITION_UPDATE_INTERVAL_MS: u64 = 50;

/// Maximum allowed position update interval in milliseconds
///
/// Ensures UI remains responsive. Longer intervals may cause jerky progress bars.
pub const MAX_POSITION_UPDATE_INTERVAL_MS: u64 = 2000;

/// Device event deduplication window in milliseconds
///
/// Platform APIs (CoreAudio, PipeWire, WinRT) can emit duplicate events.
/// Events of the same type for the same device within this window are ignored.
pub const DEVICE_EVENT_DEDUP_WINDOW_MS: u64 = 500;

/// Frontend ignore window multiplier
///
/// After a seek operation, the frontend ignores position updates from the backend
/// for a duration calculated as: `position_update_interval * IGNORE_WINDOW_MULTIPLIER`
///
/// This prevents race conditions where:
/// 1. User seeks to position X
/// 2. Backend is still emitting old position Y
/// 3. Progress bar jumps back to Y briefly before settling at X
///
/// The multiplier of 1.2 means we ignore updates for slightly longer than one update cycle,
/// ensuring the backend has time to process the seek and emit the new position.
pub const IGNORE_WINDOW_MULTIPLIER: f64 = 1.2;

/// Playback timing configuration
///
/// Exported to frontend via `get_playback_timing_config` command.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackTimingConfig {
    /// Position update interval in milliseconds
    pub position_update_interval_ms: u64,

    /// Ignore window duration in milliseconds (for frontend seek operations)
    pub ignore_window_ms: u64,

    /// Device event deduplication window in milliseconds
    pub device_event_dedup_window_ms: u64,
}

impl PlaybackTimingConfig {
    /// Create default timing configuration
    pub fn default() -> Self {
        Self::with_position_interval(DEFAULT_POSITION_UPDATE_INTERVAL_MS)
    }

    /// Create timing configuration with custom position update interval
    ///
    /// The interval is clamped to the allowed range [MIN, MAX].
    /// The ignore window is automatically calculated as interval * multiplier.
    pub fn with_position_interval(interval_ms: u64) -> Self {
        let clamped_interval = interval_ms.clamp(
            MIN_POSITION_UPDATE_INTERVAL_MS,
            MAX_POSITION_UPDATE_INTERVAL_MS,
        );

        let ignore_window = (clamped_interval as f64 * IGNORE_WINDOW_MULTIPLIER) as u64;

        Self {
            position_update_interval_ms: clamped_interval,
            ignore_window_ms: ignore_window,
            device_event_dedup_window_ms: DEVICE_EVENT_DEDUP_WINDOW_MS,
        }
    }

    /// Get position update interval as Duration
    pub fn position_update_duration(&self) -> Duration {
        Duration::from_millis(self.position_update_interval_ms)
    }

    /// Get ignore window as Duration
    pub fn ignore_window_duration(&self) -> Duration {
        Duration::from_millis(self.ignore_window_ms)
    }

    /// Get device deduplication window as Duration
    pub fn device_dedup_duration(&self) -> Duration {
        Duration::from_millis(self.device_event_dedup_window_ms)
    }

    /// Validate the configuration
    ///
    /// Returns true if all values are within valid ranges.
    pub fn validate(&self) -> bool {
        self.position_update_interval_ms >= MIN_POSITION_UPDATE_INTERVAL_MS
            && self.position_update_interval_ms <= MAX_POSITION_UPDATE_INTERVAL_MS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = PlaybackTimingConfig::default();
        assert_eq!(config.position_update_interval_ms, 100);
        assert_eq!(config.ignore_window_ms, 120); // 100 * 1.2
        assert_eq!(config.device_event_dedup_window_ms, 500);
        assert!(config.validate());
    }

    #[test]
    fn test_custom_interval() {
        let config = PlaybackTimingConfig::with_position_interval(100);
        assert_eq!(config.position_update_interval_ms, 100);
        assert_eq!(config.ignore_window_ms, 120); // 100 * 1.2
    }

    #[test]
    fn test_interval_clamping() {
        // Too low - should clamp to min
        let config = PlaybackTimingConfig::with_position_interval(10);
        assert_eq!(
            config.position_update_interval_ms,
            MIN_POSITION_UPDATE_INTERVAL_MS
        );

        // Too high - should clamp to max
        let config = PlaybackTimingConfig::with_position_interval(5000);
        assert_eq!(
            config.position_update_interval_ms,
            MAX_POSITION_UPDATE_INTERVAL_MS
        );
    }

    #[test]
    fn test_duration_conversion() {
        let config = PlaybackTimingConfig::default();
        assert_eq!(
            config.position_update_duration(),
            Duration::from_millis(100)
        );
        assert_eq!(config.ignore_window_duration(), Duration::from_millis(120));
    }
}
