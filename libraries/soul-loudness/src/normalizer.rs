//! Loudness normalization for playback
//!
//! Applies gain to audio during playback based on ReplayGain/EBU R128 analysis.

use crate::{
    AlbumGain, ReplayGainTags, TrackGain, TruePeakLimiter, EBU_R128_BROADCAST_LUFS,
    EBU_R128_STREAMING_LUFS, MAX_PREAMP_DB, MIN_PREAMP_DB, REPLAYGAIN_REFERENCE_LUFS,
};

/// Normalization mode for playback
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NormalizationMode {
    /// No normalization applied
    #[default]
    Disabled,
    /// ReplayGain track mode (per-track normalization to -18 LUFS)
    ReplayGainTrack,
    /// ReplayGain album mode (album-relative normalization)
    ReplayGainAlbum,
    /// EBU R128 broadcast level (-23 LUFS)
    EbuR128Broadcast,
    /// EBU R128 streaming level (-14 LUFS)
    EbuR128Streaming,
}

impl NormalizationMode {
    /// Get the reference level in LUFS for this mode
    pub fn reference_lufs(&self) -> Option<f64> {
        match self {
            Self::Disabled => None,
            Self::ReplayGainTrack | Self::ReplayGainAlbum => Some(REPLAYGAIN_REFERENCE_LUFS),
            Self::EbuR128Broadcast => Some(EBU_R128_BROADCAST_LUFS),
            Self::EbuR128Streaming => Some(EBU_R128_STREAMING_LUFS),
        }
    }

    /// Parse from string (for settings persistence)
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "disabled" | "off" | "none" => Some(Self::Disabled),
            "replaygain_track" | "track" | "rg_track" => Some(Self::ReplayGainTrack),
            "replaygain_album" | "album" | "rg_album" => Some(Self::ReplayGainAlbum),
            "ebu_r128" | "ebur128" | "broadcast" => Some(Self::EbuR128Broadcast),
            "streaming" | "ebu_streaming" => Some(Self::EbuR128Streaming),
            _ => None,
        }
    }

    /// Convert to string for settings persistence
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::ReplayGainTrack => "replaygain_track",
            Self::ReplayGainAlbum => "replaygain_album",
            Self::EbuR128Broadcast => "ebu_r128",
            Self::EbuR128Streaming => "streaming",
        }
    }
}

/// Loudness normalizer for playback
///
/// Applies gain based on track/album ReplayGain or EBU R128 values.
/// Includes a true peak limiter to prevent clipping.
///
/// # Example
///
/// ```ignore
/// use soul_loudness::{LoudnessNormalizer, NormalizationMode};
///
/// let mut normalizer = LoudnessNormalizer::new(44100, 2);
/// normalizer.set_mode(NormalizationMode::ReplayGainTrack);
///
/// // Set gain for current track
/// normalizer.set_track_gain(-5.0, -1.0);
///
/// // Process audio (samples is an &mut [f32] of interleaved audio)
/// normalizer.process(&mut samples);
/// ```
pub struct LoudnessNormalizer {
    /// Current normalization mode
    mode: NormalizationMode,
    /// Pre-amplification gain in dB (-12 to +12)
    preamp_db: f64,
    /// Current track gain in dB
    track_gain_db: Option<f64>,
    /// Current track peak in dBFS
    track_peak_dbfs: Option<f64>,
    /// Current album gain in dB
    album_gain_db: Option<f64>,
    /// Current album peak in dBFS
    album_peak_dbfs: Option<f64>,
    /// True peak limiter
    limiter: TruePeakLimiter,
    /// Whether to prevent clipping (apply limiting if gain would clip)
    prevent_clipping: bool,
    /// Whether to use internal limiter (false = external limiter expected)
    use_internal_limiter: bool,
    /// Fallback gain when no ReplayGain tags are available (dB)
    fallback_gain_db: f64,
    /// Current linear gain being applied (cached for efficiency)
    current_linear_gain: f32,
    /// Whether gain needs recalculation
    gain_dirty: bool,
}

impl LoudnessNormalizer {
    /// Create a new loudness normalizer
    pub fn new(sample_rate: u32, channels: usize) -> Self {
        Self {
            mode: NormalizationMode::Disabled,
            preamp_db: 0.0,
            track_gain_db: None,
            track_peak_dbfs: None,
            album_gain_db: None,
            album_peak_dbfs: None,
            limiter: TruePeakLimiter::new(sample_rate, channels),
            prevent_clipping: true,
            use_internal_limiter: true, // Default: use internal limiter
            fallback_gain_db: 0.0,
            current_linear_gain: 1.0,
            gain_dirty: true,
        }
    }

    /// Set whether to use internal limiter
    ///
    /// When false, the normalizer only applies gain and expects an external
    /// limiter to be applied later in the signal chain (e.g., after volume).
    /// This is the recommended configuration for high-quality audio processing.
    pub fn set_use_internal_limiter(&mut self, use_internal: bool) {
        self.use_internal_limiter = use_internal;
    }

    /// Check if internal limiter is enabled
    pub fn uses_internal_limiter(&self) -> bool {
        self.use_internal_limiter
    }

    /// Set the normalization mode
    pub fn set_mode(&mut self, mode: NormalizationMode) {
        if self.mode != mode {
            self.mode = mode;
            self.gain_dirty = true;
        }
    }

    /// Get the current normalization mode
    pub fn mode(&self) -> NormalizationMode {
        self.mode
    }

    /// Set pre-amplification gain in dB (-12 to +12)
    pub fn set_preamp_db(&mut self, preamp_db: f64) {
        let clamped = preamp_db.clamp(MIN_PREAMP_DB, MAX_PREAMP_DB);
        if (self.preamp_db - clamped).abs() > 0.001 {
            self.preamp_db = clamped;
            self.gain_dirty = true;
        }
    }

    /// Get the current pre-amp gain in dB
    pub fn preamp_db(&self) -> f64 {
        self.preamp_db
    }

    /// Set track gain from TrackGain struct
    pub fn set_track_gain_from_rg(&mut self, gain: &TrackGain) {
        self.set_track_gain(gain.gain_db, gain.peak_dbfs);
    }

    /// Set album gain from AlbumGain struct
    pub fn set_album_gain_from_rg(&mut self, gain: &AlbumGain) {
        self.set_album_gain(gain.gain_db, gain.peak_dbfs);
    }

    /// Set track gain from ReplayGain tags
    pub fn set_gains_from_tags(&mut self, tags: &ReplayGainTags) {
        if let Some(track_gain) = tags.track_gain {
            let track_peak = tags.track_peak_db().unwrap_or(0.0);
            self.set_track_gain(track_gain, track_peak);
        }
        if let Some(album_gain) = tags.album_gain {
            let album_peak = tags.album_peak_db().unwrap_or(0.0);
            self.set_album_gain(album_gain, album_peak);
        }
    }

    /// Set track gain directly
    pub fn set_track_gain(&mut self, gain_db: f64, peak_dbfs: f64) {
        self.track_gain_db = Some(gain_db);
        self.track_peak_dbfs = Some(peak_dbfs);
        self.gain_dirty = true;
    }

    /// Set album gain directly
    pub fn set_album_gain(&mut self, gain_db: f64, peak_dbfs: f64) {
        self.album_gain_db = Some(gain_db);
        self.album_peak_dbfs = Some(peak_dbfs);
        self.gain_dirty = true;
    }

    /// Clear all gain values (for new track)
    pub fn clear_gains(&mut self) {
        self.track_gain_db = None;
        self.track_peak_dbfs = None;
        self.album_gain_db = None;
        self.album_peak_dbfs = None;
        self.gain_dirty = true;
    }

    /// Set whether to prevent clipping
    pub fn set_prevent_clipping(&mut self, prevent: bool) {
        self.prevent_clipping = prevent;
    }

    /// Set fallback gain when no ReplayGain tags are available
    pub fn set_fallback_gain_db(&mut self, gain_db: f64) {
        if (self.fallback_gain_db - gain_db).abs() > 0.001 {
            self.fallback_gain_db = gain_db;
            self.gain_dirty = true;
        }
    }

    /// Get the effective gain in dB based on current mode and settings
    pub fn effective_gain_db(&mut self) -> f64 {
        self.update_gain();
        20.0 * (self.current_linear_gain as f64).log10()
    }

    /// Process audio buffer in place
    pub fn process(&mut self, samples: &mut [f32]) {
        if self.mode == NormalizationMode::Disabled {
            return;
        }

        self.update_gain();

        // Apply gain
        if (self.current_linear_gain - 1.0).abs() > 0.0001 {
            for sample in samples.iter_mut() {
                *sample *= self.current_linear_gain;
            }
        }

        // Apply limiter if enabled, clipping prevention is on, AND we're using internal limiter
        if self.prevent_clipping && self.use_internal_limiter {
            self.limiter.process(samples);
        }
    }

    /// Update the cached linear gain if dirty
    fn update_gain(&mut self) {
        if !self.gain_dirty {
            return;
        }

        let (gain_db, peak_dbfs) = match self.mode {
            NormalizationMode::Disabled => {
                self.current_linear_gain = 1.0;
                self.gain_dirty = false;
                return;
            }
            NormalizationMode::ReplayGainTrack => (
                self.track_gain_db.unwrap_or(self.fallback_gain_db),
                self.track_peak_dbfs.unwrap_or(0.0),
            ),
            NormalizationMode::ReplayGainAlbum => {
                // Prefer album gain, fall back to track gain
                let gain = self
                    .album_gain_db
                    .or(self.track_gain_db)
                    .unwrap_or(self.fallback_gain_db);
                let peak = self.album_peak_dbfs.or(self.track_peak_dbfs).unwrap_or(0.0);
                (gain, peak)
            }
            NormalizationMode::EbuR128Broadcast | NormalizationMode::EbuR128Streaming => {
                // For EBU modes, we adjust the gain based on reference level difference
                // If track was analyzed at -18 LUFS (RG2), adjust for new reference
                let reference = self.mode.reference_lufs().unwrap_or(-18.0);
                let adjustment = reference - REPLAYGAIN_REFERENCE_LUFS; // e.g., -23 - (-18) = -5
                let base_gain = self.track_gain_db.unwrap_or(self.fallback_gain_db);
                (base_gain + adjustment, self.track_peak_dbfs.unwrap_or(0.0))
            }
        };

        // Apply pre-amp
        let total_gain_db = gain_db + self.preamp_db;

        // Check for clipping and limit if necessary
        let final_gain_db = if self.prevent_clipping && (total_gain_db + peak_dbfs > 0.0) {
            // Limit gain to prevent clipping
            -peak_dbfs
        } else {
            total_gain_db
        };

        // Convert to linear
        self.current_linear_gain = 10.0_f32.powf(final_gain_db as f32 / 20.0);
        self.gain_dirty = false;
    }

    /// Get the latency in samples introduced by the normalizer
    ///
    /// Note: When using external limiter (use_internal_limiter = false),
    /// this returns 0 since the limiter latency is handled externally.
    pub fn latency_samples(&self) -> usize {
        if self.prevent_clipping
            && self.use_internal_limiter
            && self.mode != NormalizationMode::Disabled
        {
            self.limiter.latency_samples()
        } else {
            0
        }
    }

    /// Reset the normalizer state (e.g., between tracks)
    pub fn reset(&mut self) {
        self.limiter.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalizer_creation() {
        let normalizer = LoudnessNormalizer::new(44100, 2);
        assert_eq!(normalizer.mode(), NormalizationMode::Disabled);
        assert!((normalizer.preamp_db() - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_mode_parsing() {
        assert_eq!(
            NormalizationMode::from_str("disabled"),
            Some(NormalizationMode::Disabled)
        );
        assert_eq!(
            NormalizationMode::from_str("replaygain_track"),
            Some(NormalizationMode::ReplayGainTrack)
        );
        assert_eq!(
            NormalizationMode::from_str("ALBUM"),
            Some(NormalizationMode::ReplayGainAlbum)
        );
        assert_eq!(
            NormalizationMode::from_str("ebu_r128"),
            Some(NormalizationMode::EbuR128Broadcast)
        );
        assert_eq!(NormalizationMode::from_str("invalid"), None);
    }

    #[test]
    fn test_disabled_passthrough() {
        let mut normalizer = LoudnessNormalizer::new(44100, 2);
        normalizer.set_mode(NormalizationMode::Disabled);

        let mut samples = vec![0.5, -0.5, 0.3, -0.3];
        let original = samples.clone();
        normalizer.process(&mut samples);

        for (orig, processed) in original.iter().zip(samples.iter()) {
            assert!((orig - processed).abs() < 0.001);
        }
    }

    #[test]
    fn test_gain_application() {
        let mut normalizer = LoudnessNormalizer::new(44100, 2);
        normalizer.set_mode(NormalizationMode::ReplayGainTrack);
        normalizer.set_prevent_clipping(false); // Disable limiter for this test
        normalizer.set_track_gain(6.0, -6.0); // +6 dB gain

        // +6 dB = 2x amplitude
        let mut samples = vec![0.25, 0.25];
        normalizer.process(&mut samples);

        // Should be approximately doubled (0.25 * 2 = 0.5)
        for sample in &samples {
            assert!((*sample - 0.5).abs() < 0.01);
        }
    }

    #[test]
    fn test_preamp() {
        let mut normalizer = LoudnessNormalizer::new(44100, 2);
        normalizer.set_mode(NormalizationMode::ReplayGainTrack);
        normalizer.set_prevent_clipping(false);
        normalizer.set_track_gain(0.0, -6.0); // No base gain
        normalizer.set_preamp_db(6.0); // +6 dB preamp

        let mut samples = vec![0.25, 0.25];
        normalizer.process(&mut samples);

        // Should be doubled due to preamp
        for sample in &samples {
            assert!((*sample - 0.5).abs() < 0.01);
        }
    }

    #[test]
    fn test_preamp_clamping() {
        let mut normalizer = LoudnessNormalizer::new(44100, 2);

        normalizer.set_preamp_db(20.0); // Should clamp to +12
        assert!((normalizer.preamp_db() - 12.0).abs() < 0.001);

        normalizer.set_preamp_db(-20.0); // Should clamp to -12
        assert!((normalizer.preamp_db() - (-12.0)).abs() < 0.001);
    }

    #[test]
    fn test_clipping_prevention() {
        let mut normalizer = LoudnessNormalizer::new(44100, 2);
        normalizer.set_mode(NormalizationMode::ReplayGainTrack);
        normalizer.set_prevent_clipping(true);
        // Track with quiet content but high peak
        normalizer.set_track_gain(10.0, -2.0); // Would clip without limiting

        // After processing (including limiter), output should not exceed 1.0
        let mut samples = vec![0.9_f32; 1000];

        // Process multiple times to get through limiter latency
        for _ in 0..10 {
            normalizer.process(&mut samples);
        }

        for &sample in &samples {
            assert!(sample.abs() <= 1.001, "Sample {} exceeds threshold", sample);
        }
    }

    #[test]
    fn test_album_mode_fallback() {
        let mut normalizer = LoudnessNormalizer::new(44100, 2);
        normalizer.set_mode(NormalizationMode::ReplayGainAlbum);
        normalizer.set_prevent_clipping(false);

        // Set only track gain
        normalizer.set_track_gain(-5.0, -6.0);

        // Should use track gain as fallback
        let gain_db = normalizer.effective_gain_db();
        assert!((gain_db - (-5.0)).abs() < 0.1);
    }

    #[test]
    fn test_ebu_r128_adjustment() {
        let mut normalizer = LoudnessNormalizer::new(44100, 2);
        normalizer.set_mode(NormalizationMode::EbuR128Broadcast);
        normalizer.set_prevent_clipping(false);

        // Track analyzed at -18 LUFS reference (RG2)
        normalizer.set_track_gain(0.0, -6.0); // Would play at -18 LUFS

        // For broadcast (-23 LUFS), should apply -5 dB adjustment
        let gain_db = normalizer.effective_gain_db();
        assert!((gain_db - (-5.0)).abs() < 0.1);
    }

    // ==================== Fallback Behavior Tests ====================

    /// Test: Track with no ReplayGain tags should use fallback gain
    #[test]
    fn test_no_replaygain_tags_uses_fallback() {
        let mut normalizer = LoudnessNormalizer::new(44100, 2);
        normalizer.set_mode(NormalizationMode::ReplayGainTrack);
        normalizer.set_prevent_clipping(false);

        // No track or album gain set - should use fallback
        normalizer.set_fallback_gain_db(-3.0);

        let gain_db = normalizer.effective_gain_db();
        assert!(
            (gain_db - (-3.0)).abs() < 0.1,
            "Expected fallback gain -3.0 dB, got {} dB",
            gain_db
        );
    }

    /// Test: Fallback gain with default value (0 dB)
    #[test]
    fn test_no_tags_default_fallback_is_zero() {
        let mut normalizer = LoudnessNormalizer::new(44100, 2);
        normalizer.set_mode(NormalizationMode::ReplayGainTrack);
        normalizer.set_prevent_clipping(false);

        // No tags, no custom fallback - should be 0 dB
        let gain_db = normalizer.effective_gain_db();
        assert!(
            (gain_db - 0.0).abs() < 0.1,
            "Expected default fallback 0.0 dB, got {} dB",
            gain_db
        );
    }

    /// Test: Album mode with no tags falls back to fallback_gain_db
    #[test]
    fn test_album_mode_no_tags_uses_fallback() {
        let mut normalizer = LoudnessNormalizer::new(44100, 2);
        normalizer.set_mode(NormalizationMode::ReplayGainAlbum);
        normalizer.set_prevent_clipping(false);
        normalizer.set_fallback_gain_db(-6.0);

        // No album or track gain set
        let gain_db = normalizer.effective_gain_db();
        assert!(
            (gain_db - (-6.0)).abs() < 0.1,
            "Album mode without tags should use fallback"
        );
    }

    /// Test: EBU R128 mode with no tags applies reference adjustment to fallback
    #[test]
    fn test_ebu_mode_no_tags_adjusts_fallback() {
        let mut normalizer = LoudnessNormalizer::new(44100, 2);
        normalizer.set_mode(NormalizationMode::EbuR128Broadcast);
        normalizer.set_prevent_clipping(false);
        normalizer.set_fallback_gain_db(0.0);

        // EBU R128 broadcast is -23 LUFS, RG reference is -18 LUFS
        // Adjustment: -23 - (-18) = -5 dB
        let gain_db = normalizer.effective_gain_db();
        assert!(
            (gain_db - (-5.0)).abs() < 0.1,
            "EBU mode should apply -5 dB adjustment to fallback, got {} dB",
            gain_db
        );
    }

    // ==================== Track Gain vs Album Gain Preference Tests ====================

    /// Test: Album mode prefers album gain over track gain
    #[test]
    fn test_album_mode_prefers_album_gain() {
        let mut normalizer = LoudnessNormalizer::new(44100, 2);
        normalizer.set_mode(NormalizationMode::ReplayGainAlbum);
        normalizer.set_prevent_clipping(false);

        // Set both track and album gain
        normalizer.set_track_gain(-5.0, -6.0);
        normalizer.set_album_gain(-8.0, -4.0);

        let gain_db = normalizer.effective_gain_db();
        assert!(
            (gain_db - (-8.0)).abs() < 0.1,
            "Album mode should prefer album gain (-8 dB), got {} dB",
            gain_db
        );
    }

    /// Test: Track mode always uses track gain, ignoring album gain
    #[test]
    fn test_track_mode_uses_track_gain_only() {
        let mut normalizer = LoudnessNormalizer::new(44100, 2);
        normalizer.set_mode(NormalizationMode::ReplayGainTrack);
        normalizer.set_prevent_clipping(false);

        normalizer.set_track_gain(-5.0, -6.0);
        normalizer.set_album_gain(-8.0, -4.0);

        let gain_db = normalizer.effective_gain_db();
        assert!(
            (gain_db - (-5.0)).abs() < 0.1,
            "Track mode should use track gain (-5 dB), got {} dB",
            gain_db
        );
    }

    /// Test: Album mode uses album peak for clipping prevention
    #[test]
    fn test_album_mode_uses_album_peak() {
        let mut normalizer = LoudnessNormalizer::new(44100, 2);
        normalizer.set_mode(NormalizationMode::ReplayGainAlbum);
        normalizer.set_prevent_clipping(true);
        normalizer.set_use_internal_limiter(false); // Test gain limiting only

        // Album gain would cause clipping: -1 dBFS peak + 5 dB gain = +4 dBTP
        normalizer.set_track_gain(3.0, -6.0);
        normalizer.set_album_gain(5.0, -1.0);

        let gain_db = normalizer.effective_gain_db();
        // Safe gain should be limited to +1 dB to prevent clipping
        assert!(
            (gain_db - 1.0).abs() < 0.1,
            "Should limit gain to prevent clipping, got {} dB",
            gain_db
        );
    }

    // ==================== Extreme Gain Values Tests ====================

    /// Test: Extreme positive gain (+20 dB)
    #[test]
    fn test_extreme_positive_gain() {
        let mut normalizer = LoudnessNormalizer::new(44100, 2);
        normalizer.set_mode(NormalizationMode::ReplayGainTrack);
        normalizer.set_prevent_clipping(false);

        // +20 dB gain (very quiet track)
        normalizer.set_track_gain(20.0, -30.0);

        // +20 dB = 10x amplitude
        let mut samples = vec![0.05, 0.05];
        normalizer.process(&mut samples);

        let expected = 0.05 * 10.0; // 0.5
        for sample in &samples {
            assert!(
                (*sample - expected).abs() < 0.01,
                "Expected ~{}, got {}",
                expected,
                sample
            );
        }
    }

    /// Test: Extreme negative gain (-20 dB)
    #[test]
    fn test_extreme_negative_gain() {
        let mut normalizer = LoudnessNormalizer::new(44100, 2);
        normalizer.set_mode(NormalizationMode::ReplayGainTrack);
        normalizer.set_prevent_clipping(false);

        // -20 dB gain (very loud track)
        normalizer.set_track_gain(-20.0, -0.5);

        // -20 dB = 0.1x amplitude
        let mut samples = vec![0.5, 0.5];
        normalizer.process(&mut samples);

        let expected = 0.5 * 0.1; // 0.05
        for sample in &samples {
            assert!(
                (*sample - expected).abs() < 0.01,
                "Expected ~{}, got {}",
                expected,
                sample
            );
        }
    }

    /// Test: Zero gain passthrough (track at reference level)
    #[test]
    fn test_zero_gain_passthrough() {
        let mut normalizer = LoudnessNormalizer::new(44100, 2);
        normalizer.set_mode(NormalizationMode::ReplayGainTrack);
        normalizer.set_prevent_clipping(false);

        normalizer.set_track_gain(0.0, -6.0);

        let mut samples = vec![0.5, -0.5, 0.3, -0.3];
        let original = samples.clone();
        normalizer.process(&mut samples);

        for (orig, processed) in original.iter().zip(samples.iter()) {
            assert!(
                (orig - processed).abs() < 0.001,
                "Zero gain should pass through unchanged"
            );
        }
    }

    // ==================== Clipping Prevention Tests ====================

    /// Test: High gain + loud track triggers clipping prevention
    #[test]
    fn test_clipping_prevention_high_gain_loud_track() {
        let mut normalizer = LoudnessNormalizer::new(44100, 2);
        normalizer.set_mode(NormalizationMode::ReplayGainTrack);
        normalizer.set_prevent_clipping(true);
        normalizer.set_use_internal_limiter(false); // Test gain limiting only

        // Track with -1 dBFS peak and +10 dB gain would clip
        // Safe gain should be limited to +1 dB (-peak_dbfs)
        normalizer.set_track_gain(10.0, -1.0);

        let gain_db = normalizer.effective_gain_db();
        assert!(
            (gain_db - 1.0).abs() < 0.1,
            "Gain should be limited to +1 dB to prevent clipping, got {} dB",
            gain_db
        );
    }

    /// Test: Clipping prevention disabled allows over-unity gain
    #[test]
    fn test_clipping_prevention_disabled_allows_clipping() {
        let mut normalizer = LoudnessNormalizer::new(44100, 2);
        normalizer.set_mode(NormalizationMode::ReplayGainTrack);
        normalizer.set_prevent_clipping(false);

        // Would clip but prevention is disabled
        normalizer.set_track_gain(10.0, -1.0);

        let gain_db = normalizer.effective_gain_db();
        assert!(
            (gain_db - 10.0).abs() < 0.1,
            "Without clipping prevention, full gain should apply: {} dB",
            gain_db
        );
    }

    /// Test: Clipping prevention with 0 dBFS peak
    #[test]
    fn test_clipping_prevention_zero_peak() {
        let mut normalizer = LoudnessNormalizer::new(44100, 2);
        normalizer.set_mode(NormalizationMode::ReplayGainTrack);
        normalizer.set_prevent_clipping(true);
        normalizer.set_use_internal_limiter(false);

        // Peak at 0 dBFS - any positive gain would clip
        normalizer.set_track_gain(5.0, 0.0);

        let gain_db = normalizer.effective_gain_db();
        assert!(
            (gain_db - 0.0).abs() < 0.1,
            "With 0 dBFS peak, gain should be limited to 0 dB, got {} dB",
            gain_db
        );
    }

    /// Test: Track with headroom allows full gain
    #[test]
    fn test_track_with_headroom_allows_full_gain() {
        let mut normalizer = LoudnessNormalizer::new(44100, 2);
        normalizer.set_mode(NormalizationMode::ReplayGainTrack);
        normalizer.set_prevent_clipping(true);
        normalizer.set_use_internal_limiter(false);

        // Track with -10 dBFS peak and +5 dB gain = -5 dBTP (safe)
        normalizer.set_track_gain(5.0, -10.0);

        let gain_db = normalizer.effective_gain_db();
        assert!(
            (gain_db - 5.0).abs() < 0.1,
            "Track with headroom should allow full gain: {} dB",
            gain_db
        );
    }

    // ==================== Preamp Interaction Tests ====================

    /// Test: Preamp combined with track gain
    #[test]
    fn test_preamp_combines_with_track_gain() {
        let mut normalizer = LoudnessNormalizer::new(44100, 2);
        normalizer.set_mode(NormalizationMode::ReplayGainTrack);
        normalizer.set_prevent_clipping(false);

        normalizer.set_track_gain(3.0, -10.0);
        normalizer.set_preamp_db(3.0);

        // Total: 3 + 3 = 6 dB
        let gain_db = normalizer.effective_gain_db();
        assert!(
            (gain_db - 6.0).abs() < 0.1,
            "Preamp + track gain should combine: {} dB",
            gain_db
        );
    }

    /// Test: Preamp triggers clipping prevention
    #[test]
    fn test_preamp_triggers_clipping_prevention() {
        let mut normalizer = LoudnessNormalizer::new(44100, 2);
        normalizer.set_mode(NormalizationMode::ReplayGainTrack);
        normalizer.set_prevent_clipping(true);
        normalizer.set_use_internal_limiter(false);

        // Track gain alone is safe, but preamp pushes over
        normalizer.set_track_gain(2.0, -3.0); // 2 + (-3) = -1 dBTP, safe
        normalizer.set_preamp_db(5.0); // 2 + 5 + (-3) = +4 dBTP, would clip

        let gain_db = normalizer.effective_gain_db();
        // Should be limited to 3 dB (to get 0 dBTP)
        assert!(
            (gain_db - 3.0).abs() < 0.1,
            "Preamp should trigger clipping prevention, got {} dB",
            gain_db
        );
    }

    /// Test: Negative preamp reduces overall gain
    #[test]
    fn test_negative_preamp_reduces_gain() {
        let mut normalizer = LoudnessNormalizer::new(44100, 2);
        normalizer.set_mode(NormalizationMode::ReplayGainTrack);
        normalizer.set_prevent_clipping(false);

        normalizer.set_track_gain(6.0, -10.0);
        normalizer.set_preamp_db(-3.0);

        // Total: 6 - 3 = 3 dB
        let gain_db = normalizer.effective_gain_db();
        assert!(
            (gain_db - 3.0).abs() < 0.1,
            "Negative preamp should reduce gain: {} dB",
            gain_db
        );
    }

    // ==================== Gain Changes Between Tracks Tests ====================

    /// Test: Switching tracks updates gain correctly
    #[test]
    fn test_gain_changes_between_tracks() {
        let mut normalizer = LoudnessNormalizer::new(44100, 2);
        normalizer.set_mode(NormalizationMode::ReplayGainTrack);
        normalizer.set_prevent_clipping(false);

        // First track
        normalizer.set_track_gain(-5.0, -6.0);
        let gain1 = normalizer.effective_gain_db();
        assert!(
            (gain1 - (-5.0)).abs() < 0.1,
            "First track gain should be -5 dB"
        );

        // Second track (clear and set new gain)
        normalizer.clear_gains();
        normalizer.set_track_gain(3.0, -10.0);
        let gain2 = normalizer.effective_gain_db();
        assert!(
            (gain2 - 3.0).abs() < 0.1,
            "Second track gain should be 3 dB"
        );
    }

    /// Test: Clear gains resets to fallback
    #[test]
    fn test_clear_gains_resets_to_fallback() {
        let mut normalizer = LoudnessNormalizer::new(44100, 2);
        normalizer.set_mode(NormalizationMode::ReplayGainTrack);
        normalizer.set_prevent_clipping(false);
        normalizer.set_fallback_gain_db(-2.0);

        normalizer.set_track_gain(5.0, -6.0);
        let _ = normalizer.effective_gain_db(); // Process

        normalizer.clear_gains();
        let gain = normalizer.effective_gain_db();
        assert!(
            (gain - (-2.0)).abs() < 0.1,
            "After clear, should use fallback gain"
        );
    }

    /// Test: Reset clears limiter state
    #[test]
    fn test_reset_clears_limiter_state() {
        let mut normalizer = LoudnessNormalizer::new(44100, 2);
        normalizer.set_mode(NormalizationMode::ReplayGainTrack);
        normalizer.set_prevent_clipping(true);
        normalizer.set_track_gain(10.0, -2.0);

        // Process loud signal to engage limiter
        let mut samples = vec![0.9_f32; 1000];
        normalizer.process(&mut samples);

        // Reset should clear limiter state
        normalizer.reset();

        // Latency should still be reported correctly
        assert!(normalizer.latency_samples() > 0);
    }

    // ==================== Target Loudness Changes Tests ====================

    /// Test: Switching from track to album mode
    #[test]
    fn test_switch_track_to_album_mode() {
        let mut normalizer = LoudnessNormalizer::new(44100, 2);
        normalizer.set_prevent_clipping(false);

        normalizer.set_track_gain(-3.0, -6.0);
        normalizer.set_album_gain(-7.0, -4.0);

        normalizer.set_mode(NormalizationMode::ReplayGainTrack);
        let track_gain = normalizer.effective_gain_db();
        assert!((track_gain - (-3.0)).abs() < 0.1);

        normalizer.set_mode(NormalizationMode::ReplayGainAlbum);
        let album_gain = normalizer.effective_gain_db();
        assert!((album_gain - (-7.0)).abs() < 0.1);
    }

    /// Test: EBU R128 streaming vs broadcast
    #[test]
    fn test_ebu_streaming_vs_broadcast() {
        let mut normalizer = LoudnessNormalizer::new(44100, 2);
        normalizer.set_prevent_clipping(false);
        normalizer.set_track_gain(0.0, -10.0);

        // Broadcast: -23 LUFS (5 dB quieter than RG reference)
        normalizer.set_mode(NormalizationMode::EbuR128Broadcast);
        let broadcast_gain = normalizer.effective_gain_db();
        assert!(
            (broadcast_gain - (-5.0)).abs() < 0.1,
            "Broadcast should be -5 dB adjustment"
        );

        // Streaming: -14 LUFS (4 dB louder than RG reference)
        normalizer.set_mode(NormalizationMode::EbuR128Streaming);
        let streaming_gain = normalizer.effective_gain_db();
        assert!(
            (streaming_gain - 4.0).abs() < 0.1,
            "Streaming should be +4 dB adjustment"
        );
    }

    /// Test: Disabled mode ignores all settings
    #[test]
    fn test_disabled_mode_ignores_settings() {
        let mut normalizer = LoudnessNormalizer::new(44100, 2);
        normalizer.set_mode(NormalizationMode::Disabled);

        normalizer.set_track_gain(10.0, -1.0);
        normalizer.set_album_gain(15.0, -0.5);
        normalizer.set_preamp_db(12.0);

        let mut samples = vec![0.5, -0.5];
        let original = samples.clone();
        normalizer.process(&mut samples);

        for (orig, processed) in original.iter().zip(samples.iter()) {
            assert!(
                (orig - processed).abs() < 0.001,
                "Disabled mode should pass through unchanged"
            );
        }
    }

    // ==================== Internal Limiter Control Tests ====================

    /// Test: External limiter mode disables internal limiter
    #[test]
    fn test_external_limiter_mode() {
        let mut normalizer = LoudnessNormalizer::new(44100, 2);
        normalizer.set_mode(NormalizationMode::ReplayGainTrack);
        normalizer.set_prevent_clipping(true);
        normalizer.set_use_internal_limiter(false);

        assert!(!normalizer.uses_internal_limiter());
        assert_eq!(
            normalizer.latency_samples(),
            0,
            "External limiter mode should report 0 latency"
        );
    }

    /// Test: Internal limiter mode reports latency
    #[test]
    fn test_internal_limiter_reports_latency() {
        let mut normalizer = LoudnessNormalizer::new(44100, 2);
        normalizer.set_mode(NormalizationMode::ReplayGainTrack);
        normalizer.set_prevent_clipping(true);
        normalizer.set_use_internal_limiter(true);

        assert!(normalizer.uses_internal_limiter());
        assert!(
            normalizer.latency_samples() > 0,
            "Internal limiter should report latency"
        );
    }

    // ==================== ReplayGainTags Integration Tests ====================

    /// Test: Setting gains from ReplayGainTags struct
    #[test]
    fn test_set_gains_from_tags() {
        let mut normalizer = LoudnessNormalizer::new(44100, 2);
        normalizer.set_mode(NormalizationMode::ReplayGainTrack);
        normalizer.set_prevent_clipping(false);

        let tags = ReplayGainTags {
            track_gain: Some(-5.5),
            track_peak: Some(0.5), // Linear peak ~-6 dBFS
            album_gain: Some(-3.0),
            album_peak: Some(0.707), // Linear peak ~-3 dBFS
            reference_loudness: Some(-18.0),
        };

        normalizer.set_gains_from_tags(&tags);

        let gain_db = normalizer.effective_gain_db();
        assert!(
            (gain_db - (-5.5)).abs() < 0.1,
            "Should use track gain from tags"
        );
    }

    /// Test: Tags with only album gain in track mode uses fallback
    #[test]
    fn test_tags_album_only_in_track_mode() {
        let mut normalizer = LoudnessNormalizer::new(44100, 2);
        normalizer.set_mode(NormalizationMode::ReplayGainTrack);
        normalizer.set_prevent_clipping(false);
        normalizer.set_fallback_gain_db(-1.0);

        let tags = ReplayGainTags {
            track_gain: None,
            track_peak: None,
            album_gain: Some(-5.0),
            album_peak: Some(0.707),
            reference_loudness: Some(-18.0),
        };

        normalizer.set_gains_from_tags(&tags);

        let gain_db = normalizer.effective_gain_db();
        assert!(
            (gain_db - (-1.0)).abs() < 0.1,
            "Track mode with no track gain should use fallback"
        );
    }

    // ==================== Edge Cases ====================

    /// Test: Empty buffer processing
    #[test]
    fn test_empty_buffer_processing() {
        let mut normalizer = LoudnessNormalizer::new(44100, 2);
        normalizer.set_mode(NormalizationMode::ReplayGainTrack);
        normalizer.set_track_gain(6.0, -6.0);

        let mut empty: Vec<f32> = vec![];
        normalizer.process(&mut empty); // Should not panic
        assert_eq!(empty.len(), 0);
    }

    /// Test: Single sample processing
    #[test]
    fn test_single_sample_processing() {
        let mut normalizer = LoudnessNormalizer::new(44100, 1);
        normalizer.set_mode(NormalizationMode::ReplayGainTrack);
        normalizer.set_prevent_clipping(false);
        normalizer.set_track_gain(6.0, -20.0); // +6 dB = 2x

        let mut single = vec![0.25_f32];
        normalizer.process(&mut single);

        assert!(
            (single[0] - 0.5).abs() < 0.01,
            "Single sample should be processed correctly"
        );
    }

    /// Test: Very small gain values
    #[test]
    fn test_very_small_gain() {
        let mut normalizer = LoudnessNormalizer::new(44100, 2);
        normalizer.set_mode(NormalizationMode::ReplayGainTrack);
        normalizer.set_prevent_clipping(false);
        normalizer.set_track_gain(0.001, -10.0);

        let gain_db = normalizer.effective_gain_db();
        assert!(
            (gain_db - 0.001).abs() < 0.01,
            "Very small gain should be applied correctly"
        );
    }

    /// Test: Gain dirty flag optimization
    #[test]
    fn test_gain_dirty_flag_optimization() {
        let mut normalizer = LoudnessNormalizer::new(44100, 2);
        normalizer.set_mode(NormalizationMode::ReplayGainTrack);
        normalizer.set_prevent_clipping(false);
        normalizer.set_track_gain(3.0, -10.0);

        // First call calculates
        let gain1 = normalizer.effective_gain_db();

        // Second call without changes should return same value (cached)
        let gain2 = normalizer.effective_gain_db();
        assert_eq!(gain1, gain2, "Cached gain should be same");

        // Change should trigger recalculation
        normalizer.set_track_gain(5.0, -10.0);
        let gain3 = normalizer.effective_gain_db();
        assert!(
            (gain3 - gain1).abs() > 1.0,
            "Should recalculate after change"
        );
    }

    /// Test: Mode string roundtrip
    #[test]
    fn test_mode_string_roundtrip() {
        let modes = [
            NormalizationMode::Disabled,
            NormalizationMode::ReplayGainTrack,
            NormalizationMode::ReplayGainAlbum,
            NormalizationMode::EbuR128Broadcast,
            NormalizationMode::EbuR128Streaming,
        ];

        for mode in modes {
            let s = mode.as_str();
            let parsed = NormalizationMode::from_str(s);
            assert_eq!(
                parsed,
                Some(mode),
                "Mode {:?} should roundtrip through string",
                mode
            );
        }
    }

    /// Test: Reference LUFS for all modes
    #[test]
    fn test_reference_lufs_all_modes() {
        assert_eq!(NormalizationMode::Disabled.reference_lufs(), None);
        assert_eq!(
            NormalizationMode::ReplayGainTrack.reference_lufs(),
            Some(-18.0)
        );
        assert_eq!(
            NormalizationMode::ReplayGainAlbum.reference_lufs(),
            Some(-18.0)
        );
        assert_eq!(
            NormalizationMode::EbuR128Broadcast.reference_lufs(),
            Some(-23.0)
        );
        assert_eq!(
            NormalizationMode::EbuR128Streaming.reference_lufs(),
            Some(-14.0)
        );
    }
}
