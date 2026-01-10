//! ReplayGain 2.0 calculation
//!
//! ReplayGain 2.0 is based on EBU R128 loudness measurement and uses
//! -18 LUFS as the reference level (updated from the original -14 dB RMS).
//!
//! # Gain Calculation
//!
//! - Track Gain = Reference Level (-18 LUFS) - Track Integrated Loudness
//! - Album Gain = Reference Level (-18 LUFS) - Album Average Loudness
//!
//! # Peak Values
//!
//! True peak values are used to prevent clipping when applying gain.
//! If (gain + true_peak) > 0 dB, clipping will occur.

use crate::{LoudnessInfo, REPLAYGAIN_REFERENCE_LUFS};

/// Track-level ReplayGain information
#[derive(Debug, Clone, PartialEq)]
pub struct TrackGain {
    /// Gain to apply in dB (can be negative for loud tracks)
    pub gain_db: f64,
    /// True peak in dBFS
    pub peak_dbfs: f64,
    /// Original integrated loudness in LUFS
    pub integrated_lufs: f64,
    /// Reference level used for calculation
    pub reference_lufs: f64,
}

impl TrackGain {
    /// Check if applying this gain would cause clipping
    pub fn would_clip(&self) -> bool {
        self.gain_db + self.peak_dbfs > 0.0
    }

    /// Get the safe gain (limited to prevent clipping)
    pub fn safe_gain(&self) -> f64 {
        let max_gain = -self.peak_dbfs;
        self.gain_db.min(max_gain)
    }

    /// Convert gain to linear multiplier
    pub fn linear_gain(&self) -> f64 {
        10.0_f64.powf(self.gain_db / 20.0)
    }

    /// Convert safe gain to linear multiplier
    pub fn safe_linear_gain(&self) -> f64 {
        10.0_f64.powf(self.safe_gain() / 20.0)
    }
}

/// Album-level ReplayGain information
#[derive(Debug, Clone, PartialEq)]
pub struct AlbumGain {
    /// Gain to apply in dB for album normalization
    pub gain_db: f64,
    /// Maximum true peak across all tracks in dBFS
    pub peak_dbfs: f64,
    /// Average integrated loudness of all tracks in LUFS
    pub average_lufs: f64,
    /// Number of tracks analyzed
    pub track_count: usize,
    /// Reference level used for calculation
    pub reference_lufs: f64,
}

impl AlbumGain {
    /// Check if applying this gain would cause clipping on any track
    pub fn would_clip(&self) -> bool {
        self.gain_db + self.peak_dbfs > 0.0
    }

    /// Get the safe gain (limited to prevent clipping)
    pub fn safe_gain(&self) -> f64 {
        let max_gain = -self.peak_dbfs;
        self.gain_db.min(max_gain)
    }

    /// Convert gain to linear multiplier
    pub fn linear_gain(&self) -> f64 {
        10.0_f64.powf(self.gain_db / 20.0)
    }

    /// Convert safe gain to linear multiplier
    pub fn safe_linear_gain(&self) -> f64 {
        10.0_f64.powf(self.safe_gain() / 20.0)
    }
}

/// Calculator for ReplayGain values
pub struct ReplayGainCalculator {
    /// Reference loudness level (default: -18 LUFS for RG2)
    reference_lufs: f64,
}

impl ReplayGainCalculator {
    /// Create a new calculator with the default reference level (-18 LUFS)
    pub fn new() -> Self {
        Self {
            reference_lufs: REPLAYGAIN_REFERENCE_LUFS,
        }
    }

    /// Create a calculator with a custom reference level
    ///
    /// # Arguments
    /// * `reference_lufs` - Custom reference level in LUFS
    pub fn with_reference(reference_lufs: f64) -> Self {
        Self { reference_lufs }
    }

    /// Calculate track gain from loudness info
    pub fn track_gain(&self, info: &LoudnessInfo) -> TrackGain {
        let gain_db = self.reference_lufs - info.integrated_lufs;

        TrackGain {
            gain_db,
            peak_dbfs: info.true_peak_dbfs,
            integrated_lufs: info.integrated_lufs,
            reference_lufs: self.reference_lufs,
        }
    }

    /// Calculate album gain from multiple track loudness infos
    ///
    /// Album gain uses the average loudness across all tracks, weighted by duration.
    /// The peak value is the maximum true peak across all tracks.
    pub fn album_gain(&self, tracks: &[LoudnessInfo]) -> Option<AlbumGain> {
        if tracks.is_empty() {
            return None;
        }

        // Calculate duration-weighted average loudness
        // We need to convert from LUFS (log scale) to linear power, average, then convert back
        let mut total_power = 0.0_f64;
        let mut total_duration = 0.0_f64;
        let mut max_peak = -f64::INFINITY;

        for track in tracks {
            let duration = track.duration_seconds;
            // Convert LUFS to linear power: 10^(LUFS/10)
            let power = 10.0_f64.powf(track.integrated_lufs / 10.0);
            total_power += power * duration;
            total_duration += duration;

            if track.true_peak_dbfs > max_peak {
                max_peak = track.true_peak_dbfs;
            }
        }

        if total_duration <= 0.0 {
            return None;
        }

        // Calculate average power and convert back to LUFS
        let avg_power = total_power / total_duration;
        let average_lufs = 10.0 * avg_power.log10();
        let gain_db = self.reference_lufs - average_lufs;

        Some(AlbumGain {
            gain_db,
            peak_dbfs: max_peak,
            average_lufs,
            track_count: tracks.len(),
            reference_lufs: self.reference_lufs,
        })
    }

    /// Convenience function to calculate track gain with default reference
    pub fn calculate_track_gain(info: &LoudnessInfo) -> TrackGain {
        Self::new().track_gain(info)
    }

    /// Convenience function to calculate album gain with default reference
    pub fn calculate_album_gain(tracks: &[LoudnessInfo]) -> Option<AlbumGain> {
        Self::new().album_gain(tracks)
    }
}

impl Default for ReplayGainCalculator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_loudness_info(integrated_lufs: f64, true_peak_dbfs: f64, duration: f64) -> LoudnessInfo {
        LoudnessInfo {
            integrated_lufs,
            loudness_range_lu: 5.0,
            true_peak_dbfs,
            sample_peak_dbfs: true_peak_dbfs + 0.5,
            duration_seconds: duration,
            sample_rate: 44100,
            channels: 2,
        }
    }

    #[test]
    fn test_track_gain_quiet() {
        let calc = ReplayGainCalculator::new();
        let info = make_loudness_info(-23.0, -6.0, 180.0);
        let gain = calc.track_gain(&info);

        // -18 - (-23) = +5 dB
        assert!((gain.gain_db - 5.0).abs() < 0.001);
        assert_eq!(gain.integrated_lufs, -23.0);
        assert!(!gain.would_clip()); // -6 + 5 = -1 dBTP, no clipping
    }

    #[test]
    fn test_track_gain_loud() {
        let calc = ReplayGainCalculator::new();
        let info = make_loudness_info(-10.0, -0.5, 180.0);
        let gain = calc.track_gain(&info);

        // -18 - (-10) = -8 dB
        assert!((gain.gain_db - (-8.0)).abs() < 0.001);
        assert!(!gain.would_clip()); // -0.5 + (-8) = -8.5 dBTP, no clipping
    }

    #[test]
    fn test_track_gain_clipping() {
        let calc = ReplayGainCalculator::new();
        let info = make_loudness_info(-28.0, -3.0, 180.0);
        let gain = calc.track_gain(&info);

        // -18 - (-28) = +10 dB
        assert!((gain.gain_db - 10.0).abs() < 0.001);
        // -3 + 10 = +7 dBTP, would clip!
        assert!(gain.would_clip());

        // Safe gain should be limited to +3 dB
        assert!((gain.safe_gain() - 3.0).abs() < 0.001);
    }

    #[test]
    fn test_album_gain_calculation() {
        let calc = ReplayGainCalculator::new();
        let tracks = vec![
            make_loudness_info(-15.0, -1.0, 180.0), // Loud track
            make_loudness_info(-23.0, -6.0, 240.0), // Quiet track
            make_loudness_info(-19.0, -3.0, 200.0), // Medium track
        ];

        let album = calc.album_gain(&tracks).unwrap();

        // Average should be weighted by duration, somewhere between -23 and -15
        assert!(album.average_lufs > -23.0 && album.average_lufs < -15.0);
        assert_eq!(album.track_count, 3);
        // Max peak is -1.0 from the first track
        assert!((album.peak_dbfs - (-1.0)).abs() < 0.001);
    }

    #[test]
    fn test_album_gain_empty() {
        let calc = ReplayGainCalculator::new();
        assert!(calc.album_gain(&[]).is_none());
    }

    #[test]
    fn test_linear_gain_conversion() {
        let calc = ReplayGainCalculator::new();
        let info = make_loudness_info(-18.0, -6.0, 180.0);
        let gain = calc.track_gain(&info);

        // 0 dB gain should be 1.0 linear
        assert!((gain.gain_db).abs() < 0.001);
        assert!((gain.linear_gain() - 1.0).abs() < 0.001);

        // Test +6 dB = 2.0 linear
        let info2 = make_loudness_info(-24.0, -6.0, 180.0);
        let gain2 = calc.track_gain(&info2);
        assert!((gain2.gain_db - 6.0).abs() < 0.001);
        assert!((gain2.linear_gain() - 2.0).abs() < 0.01);

        // Test -6 dB = 0.5 linear
        let info3 = make_loudness_info(-12.0, -6.0, 180.0);
        let gain3 = calc.track_gain(&info3);
        assert!((gain3.gain_db - (-6.0)).abs() < 0.001);
        assert!((gain3.linear_gain() - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_custom_reference_level() {
        // EBU R128 broadcast uses -23 LUFS
        let calc = ReplayGainCalculator::with_reference(-23.0);
        let info = make_loudness_info(-18.0, -6.0, 180.0);
        let gain = calc.track_gain(&info);

        // -23 - (-18) = -5 dB
        assert!((gain.gain_db - (-5.0)).abs() < 0.001);
        assert_eq!(gain.reference_lufs, -23.0);
    }
}
