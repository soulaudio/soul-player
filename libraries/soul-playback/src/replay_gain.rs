//! Simple ReplayGain support for playback normalization
//!
//! Implements ReplayGain 2.0 standard (-18 LUFS reference) using pre-analyzed
//! gain values from track metadata tags.
//!
//! # Features
//! - Track gain: Normalize individual tracks
//! - Album gain: Normalize full albums with consistent volume
//! - Pre-amp: User adjustment (-15 to +15 dB)
//! - Clipping prevention: Optional peak limiting
//!
//! # Implementation
//! ReplayGain is extremely simple compared to LUFS normalization:
//! 1. Read gain value from metadata (stored in dB)
//! 2. Convert dB to linear gain: `10^(dB/20)`
//! 3. Multiply audio samples by linear gain
//!
//! That's it! No realtime analysis, no complex DSP, just a multiply operation.

use serde::{Deserialize, Serialize};

/// ReplayGain normalization mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReplayGainMode {
    /// No normalization
    Off,
    /// Use per-track gain (normalizes each track independently)
    Track,
    /// Use album gain (keeps relative volume within album)
    Album,
}

impl Default for ReplayGainMode {
    fn default() -> Self {
        Self::Off
    }
}

impl ReplayGainMode {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Track => "track",
            Self::Album => "album",
        }
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "off" => Some(Self::Off),
            "track" => Some(Self::Track),
            "album" => Some(Self::Album),
            _ => None,
        }
    }
}

/// ReplayGain values for a track
#[derive(Debug, Clone, Copy)]
pub struct ReplayGainValues {
    /// Track gain in dB (from REPLAYGAIN_TRACK_GAIN tag)
    pub track_gain_db: Option<f32>,
    /// Track peak value (linear, 0.0 to 1.0+)
    pub track_peak: Option<f32>,
    /// Album gain in dB (from REPLAYGAIN_ALBUM_GAIN tag)
    pub album_gain_db: Option<f32>,
    /// Album peak value (linear, 0.0 to 1.0+)
    pub album_peak: Option<f32>,
}

impl Default for ReplayGainValues {
    fn default() -> Self {
        Self {
            track_gain_db: None,
            track_peak: None,
            album_gain_db: None,
            album_peak: None,
        }
    }
}

impl ReplayGainValues {
    /// Create with no ReplayGain values
    pub fn none() -> Self {
        Self::default()
    }

    /// Get the gain value based on mode
    pub fn gain_for_mode(&self, mode: ReplayGainMode) -> Option<f32> {
        match mode {
            ReplayGainMode::Off => None,
            ReplayGainMode::Track => self.track_gain_db,
            ReplayGainMode::Album => self.album_gain_db.or(self.track_gain_db),
        }
    }

    /// Get the peak value based on mode
    pub fn peak_for_mode(&self, mode: ReplayGainMode) -> Option<f32> {
        match mode {
            ReplayGainMode::Off => None,
            ReplayGainMode::Track => self.track_peak,
            ReplayGainMode::Album => self.album_peak.or(self.track_peak),
        }
    }
}

/// ReplayGain processor for audio playback
///
/// Applies gain adjustment to audio samples based on ReplayGain metadata.
/// All processing is done in the audio callback, so no allocations are allowed.
#[derive(Debug, Clone)]
pub struct ReplayGainProcessor {
    /// Current normalization mode
    mode: ReplayGainMode,

    /// Pre-amp adjustment in dB (-15.0 to +15.0)
    preamp_db: f32,

    /// Whether to prevent clipping
    prevent_clipping: bool,

    /// Current track's ReplayGain values
    current_values: ReplayGainValues,

    /// Cached linear gain multiplier (updated when values/mode/preamp change)
    /// This is the actual multiplier applied to samples
    linear_gain: f32,
}

impl Default for ReplayGainProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl ReplayGainProcessor {
    /// Create a new ReplayGain processor (disabled by default)
    pub fn new() -> Self {
        Self {
            mode: ReplayGainMode::Off,
            preamp_db: 0.0,
            prevent_clipping: true,
            current_values: ReplayGainValues::none(),
            linear_gain: 1.0, // Unity gain (no change)
        }
    }

    /// Set the normalization mode
    pub fn set_mode(&mut self, mode: ReplayGainMode) {
        if self.mode != mode {
            self.mode = mode;
            self.recalculate_gain();
        }
    }

    /// Get the current mode
    pub fn mode(&self) -> ReplayGainMode {
        self.mode
    }

    /// Set pre-amp adjustment in dB (-15.0 to +15.0)
    pub fn set_preamp_db(&mut self, db: f32) {
        let clamped = db.clamp(-15.0, 15.0);
        if (self.preamp_db - clamped).abs() > 0.01 {
            self.preamp_db = clamped;
            self.recalculate_gain();
        }
    }

    /// Get pre-amp adjustment in dB
    pub fn preamp_db(&self) -> f32 {
        self.preamp_db
    }

    /// Set whether to prevent clipping
    pub fn set_prevent_clipping(&mut self, prevent: bool) {
        if self.prevent_clipping != prevent {
            self.prevent_clipping = prevent;
            self.recalculate_gain();
        }
    }

    /// Get whether clipping prevention is enabled
    pub fn prevent_clipping(&self) -> bool {
        self.prevent_clipping
    }

    /// Update ReplayGain values for current track
    pub fn set_track_values(&mut self, values: ReplayGainValues) {
        self.current_values = values;
        self.recalculate_gain();
    }

    /// Clear track values (e.g., when track ends)
    pub fn clear_track_values(&mut self) {
        self.current_values = ReplayGainValues::none();
        self.recalculate_gain();
    }

    /// Get the effective gain in dB (for display/debugging)
    pub fn effective_gain_db(&self) -> f32 {
        if self.linear_gain > 0.0 {
            20.0 * self.linear_gain.log10()
        } else {
            -100.0 // Silence
        }
    }

    /// Recalculate the linear gain multiplier
    fn recalculate_gain(&mut self) {
        // Start with unity gain
        let mut total_gain_db = 0.0;

        // Add ReplayGain if enabled
        if self.mode != ReplayGainMode::Off {
            if let Some(gain_db) = self.current_values.gain_for_mode(self.mode) {
                total_gain_db += gain_db;
            }
        }

        // Add pre-amp
        total_gain_db += self.preamp_db;

        // Apply clipping prevention if enabled
        if self.prevent_clipping && self.mode != ReplayGainMode::Off {
            if let Some(peak) = self.current_values.peak_for_mode(self.mode) {
                if peak > 0.0 {
                    // Calculate max safe gain: the gain that would bring peak to 1.0
                    // If peak is 0.5 and gain is +6dB (2x), result would be 1.0 (no clip)
                    // If peak is 0.5 and gain is +12dB (4x), result would be 2.0 (clip!)
                    // So max_safe_gain_db = -20*log10(peak)
                    let max_safe_gain_db = -20.0 * peak.log10();
                    total_gain_db = total_gain_db.min(max_safe_gain_db);
                }
            }
        }

        // Convert dB to linear gain: 10^(dB/20)
        self.linear_gain = db_to_linear(total_gain_db);

        tracing::debug!(
            mode = ?self.mode,
            gain_db = %total_gain_db,
            linear_gain = %self.linear_gain,
            prevent_clipping = %self.prevent_clipping,
            "[ReplayGain] Recalculated gain"
        );
    }

    /// Process audio samples (apply gain)
    ///
    /// This is called in the audio callback, so it must be fast and never allocate.
    /// The gain is pre-calculated in `recalculate_gain()`, so this is just a multiply.
    #[inline]
    pub fn process(&self, buffer: &mut [f32]) {
        // Fast path: if gain is unity (1.0), skip processing
        if (self.linear_gain - 1.0).abs() < 0.0001 {
            return;
        }

        // Apply gain to all samples
        for sample in buffer.iter_mut() {
            *sample *= self.linear_gain;
        }
    }

    /// Check if ReplayGain is effectively active (non-zero gain will be applied)
    pub fn is_active(&self) -> bool {
        self.mode != ReplayGainMode::Off && (self.linear_gain - 1.0).abs() > 0.0001
    }
}

/// Convert dB to linear gain
#[inline]
fn db_to_linear(db: f32) -> f32 {
    10.0_f32.powf(db / 20.0)
}

/// Convert linear gain to dB
#[inline]
fn linear_to_db(linear: f32) -> f32 {
    if linear > 0.0 {
        20.0 * linear.log10()
    } else {
        -100.0 // Silence
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_db_conversion() {
        // 0 dB = 1.0 (unity gain)
        assert!((db_to_linear(0.0) - 1.0).abs() < 0.0001);
        assert!((linear_to_db(1.0) - 0.0).abs() < 0.0001);

        // +6 dB ≈ 2.0 (double amplitude)
        assert!((db_to_linear(6.0) - 2.0).abs() < 0.01);
        assert!((linear_to_db(2.0) - 6.0).abs() < 0.1); // Relaxed tolerance

        // -6 dB ≈ 0.5 (half amplitude)
        assert!((db_to_linear(-6.0) - 0.5).abs() < 0.01);
        assert!((linear_to_db(0.5) - (-6.0)).abs() < 0.1); // Relaxed tolerance

        // -20 dB = 0.1
        assert!((db_to_linear(-20.0) - 0.1).abs() < 0.001);
    }

    #[test]
    fn test_replay_gain_mode_default() {
        let processor = ReplayGainProcessor::new();
        assert_eq!(processor.mode(), ReplayGainMode::Off);
        assert!(!processor.is_active());
    }

    #[test]
    fn test_replay_gain_track_mode() {
        let mut processor = ReplayGainProcessor::new();
        processor.set_mode(ReplayGainMode::Track);

        let values = ReplayGainValues {
            track_gain_db: Some(-5.0),
            track_peak: Some(0.9),
            album_gain_db: Some(-3.0),
            album_peak: Some(0.95),
        };
        processor.set_track_values(values);

        // Should use track gain (-5 dB)
        let expected_linear = db_to_linear(-5.0);
        assert!((processor.linear_gain - expected_linear).abs() < 0.001);
        assert!(processor.is_active());
    }

    #[test]
    fn test_replay_gain_album_mode() {
        let mut processor = ReplayGainProcessor::new();
        processor.set_mode(ReplayGainMode::Album);

        let values = ReplayGainValues {
            track_gain_db: Some(-5.0),
            track_peak: Some(0.9),
            album_gain_db: Some(-3.0),
            album_peak: Some(0.95),
        };
        processor.set_track_values(values);

        // Should use album gain (-3 dB)
        let expected_linear = db_to_linear(-3.0);
        assert!((processor.linear_gain - expected_linear).abs() < 0.001);
    }

    #[test]
    fn test_replay_gain_album_fallback_to_track() {
        let mut processor = ReplayGainProcessor::new();
        processor.set_mode(ReplayGainMode::Album);

        let values = ReplayGainValues {
            track_gain_db: Some(-5.0),
            track_peak: Some(0.9),
            album_gain_db: None, // No album gain
            album_peak: None,
        };
        processor.set_track_values(values);

        // Should fall back to track gain
        let expected_linear = db_to_linear(-5.0);
        assert!((processor.linear_gain - expected_linear).abs() < 0.001);
    }

    #[test]
    fn test_preamp_adjustment() {
        let mut processor = ReplayGainProcessor::new();
        processor.set_mode(ReplayGainMode::Track);
        processor.set_preamp_db(3.0); // +3 dB preamp

        let values = ReplayGainValues {
            track_gain_db: Some(-5.0),
            track_peak: Some(0.9),
            album_gain_db: None,
            album_peak: None,
        };
        processor.set_track_values(values);

        // Total gain should be -5 + 3 = -2 dB
        let expected_linear = db_to_linear(-2.0);
        assert!((processor.linear_gain - expected_linear).abs() < 0.001);
    }

    #[test]
    fn test_clipping_prevention() {
        let mut processor = ReplayGainProcessor::new();
        processor.set_mode(ReplayGainMode::Track);
        processor.set_prevent_clipping(true);

        // Peak is 0.5, gain is +12 dB (4x)
        // Without prevention: 0.5 * 4 = 2.0 (clips!)
        // With prevention: should limit gain to +6 dB (brings 0.5 to 1.0)
        let values = ReplayGainValues {
            track_gain_db: Some(12.0), // Very high gain
            track_peak: Some(0.5),     // Peak is 0.5
            album_gain_db: None,
            album_peak: None,
        };
        processor.set_track_values(values);

        // Expected max safe gain: -20*log10(0.5) = 6.02 dB
        let expected_linear = db_to_linear(6.0);
        assert!((processor.linear_gain - expected_linear).abs() < 0.1);
    }

    #[test]
    fn test_no_clipping_prevention() {
        let mut processor = ReplayGainProcessor::new();
        processor.set_mode(ReplayGainMode::Track);
        processor.set_prevent_clipping(false);

        let values = ReplayGainValues {
            track_gain_db: Some(12.0),
            track_peak: Some(0.5),
            album_gain_db: None,
            album_peak: None,
        };
        processor.set_track_values(values);

        // Should use full +12 dB gain (will clip, but user disabled prevention)
        let expected_linear = db_to_linear(12.0);
        assert!((processor.linear_gain - expected_linear).abs() < 0.001);
    }

    #[test]
    fn test_process_applies_gain() {
        let mut processor = ReplayGainProcessor::new();
        processor.set_mode(ReplayGainMode::Track);

        let values = ReplayGainValues {
            track_gain_db: Some(-6.0), // Half volume
            track_peak: Some(0.9),
            album_gain_db: None,
            album_peak: None,
        };
        processor.set_track_values(values);

        let mut buffer = vec![1.0, 0.5, -0.5, -1.0];
        processor.process(&mut buffer);

        // -6 dB ≈ 0.5x
        let expected = vec![0.5, 0.25, -0.25, -0.5];
        for (actual, expected) in buffer.iter().zip(expected.iter()) {
            assert!((actual - expected).abs() < 0.01);
        }
    }

    #[test]
    fn test_process_off_mode_no_change() {
        let mut processor = ReplayGainProcessor::new();
        processor.set_mode(ReplayGainMode::Off);

        let values = ReplayGainValues {
            track_gain_db: Some(-6.0),
            track_peak: Some(0.9),
            album_gain_db: None,
            album_peak: None,
        };
        processor.set_track_values(values);

        let original = vec![1.0, 0.5, -0.5, -1.0];
        let mut buffer = original.clone();
        processor.process(&mut buffer);

        // Should not modify samples when Off
        assert_eq!(buffer, original);
    }

    #[test]
    fn test_mode_string_conversion() {
        assert_eq!(ReplayGainMode::Off.as_str(), "off");
        assert_eq!(ReplayGainMode::Track.as_str(), "track");
        assert_eq!(ReplayGainMode::Album.as_str(), "album");

        assert_eq!(ReplayGainMode::from_str("off"), Some(ReplayGainMode::Off));
        assert_eq!(
            ReplayGainMode::from_str("track"),
            Some(ReplayGainMode::Track)
        );
        assert_eq!(
            ReplayGainMode::from_str("album"),
            Some(ReplayGainMode::Album)
        );
        assert_eq!(ReplayGainMode::from_str("invalid"), None);
    }
}
