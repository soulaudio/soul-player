//! ReplayGain tag reading and writing
//!
//! Supports reading and writing ReplayGain tags in various formats:
//! - ID3v2 (MP3): TXXX frames with "REPLAYGAIN_*" descriptions
//! - Vorbis Comments (FLAC, OGG): REPLAYGAIN_* fields
//! - APE tags: REPLAYGAIN_* fields
//! - MP4/AAC: iTunes-style ----:com.apple.iTunes:* atoms

use crate::error::{LoudnessError, Result};
use crate::{AlbumGain, TrackGain};
use lofty::{Probe, TagExt, TaggedFileExt};
use std::path::Path;
use tracing::debug;

/// ReplayGain tag values read from a file
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ReplayGainTags {
    /// Track gain in dB
    pub track_gain: Option<f64>,
    /// Track peak (linear, 0.0-1.0+)
    pub track_peak: Option<f64>,
    /// Album gain in dB
    pub album_gain: Option<f64>,
    /// Album peak (linear, 0.0-1.0+)
    pub album_peak: Option<f64>,
    /// Reference loudness in dB (usually -14 for RG1 or -18 for RG2)
    pub reference_loudness: Option<f64>,
}

impl ReplayGainTags {
    /// Check if any ReplayGain tags are present
    pub fn has_track_tags(&self) -> bool {
        self.track_gain.is_some()
    }

    /// Check if album-level tags are present
    pub fn has_album_tags(&self) -> bool {
        self.album_gain.is_some()
    }

    /// Convert track peak from linear to dB
    pub fn track_peak_db(&self) -> Option<f64> {
        self.track_peak.map(|p| 20.0 * p.log10())
    }

    /// Convert album peak from linear to dB
    pub fn album_peak_db(&self) -> Option<f64> {
        self.album_peak.map(|p| 20.0 * p.log10())
    }
}

impl From<&TrackGain> for ReplayGainTags {
    fn from(gain: &TrackGain) -> Self {
        Self {
            track_gain: Some(gain.gain_db),
            track_peak: Some(10.0_f64.powf(gain.peak_dbfs / 20.0)), // Convert dB to linear
            album_gain: None,
            album_peak: None,
            reference_loudness: Some(gain.reference_lufs),
        }
    }
}

/// Parse a gain value from a string (e.g., "-5.23 dB" -> -5.23)
fn parse_gain(s: &str) -> Option<f64> {
    let s = s.trim();
    let s = s.strip_suffix(" dB").unwrap_or(s);
    let s = s.strip_suffix("dB").unwrap_or(s);
    s.trim().parse().ok()
}

/// Parse a peak value from a string
fn parse_peak(s: &str) -> Option<f64> {
    s.trim().parse().ok()
}

/// Read ReplayGain tags from an audio file
///
/// # Arguments
/// * `path` - Path to the audio file
///
/// # Returns
/// ReplayGain tag values, or empty struct if no tags found
pub fn read_replaygain_tags<P: AsRef<Path>>(path: P) -> Result<ReplayGainTags> {
    let path = path.as_ref();

    if !path.exists() {
        return Err(LoudnessError::FileNotFound(path.display().to_string()));
    }

    let tagged_file = Probe::open(path)?.read()?;

    let mut tags = ReplayGainTags::default();

    // Try to read from primary tag first, then others
    if let Some(tag) = tagged_file.primary_tag() {
        // Check for ReplayGain keys
        if let Some(value) = tag.get_string(&lofty::ItemKey::ReplayGainTrackGain) {
            if let Some(gain) = parse_gain(value) {
                debug!("Found track gain: {} dB", gain);
                tags.track_gain = Some(gain);
            }
        }

        if let Some(value) = tag.get_string(&lofty::ItemKey::ReplayGainTrackPeak) {
            if let Some(peak) = parse_peak(value) {
                debug!("Found track peak: {}", peak);
                tags.track_peak = Some(peak);
            }
        }

        if let Some(value) = tag.get_string(&lofty::ItemKey::ReplayGainAlbumGain) {
            if let Some(gain) = parse_gain(value) {
                debug!("Found album gain: {} dB", gain);
                tags.album_gain = Some(gain);
            }
        }

        if let Some(value) = tag.get_string(&lofty::ItemKey::ReplayGainAlbumPeak) {
            if let Some(peak) = parse_peak(value) {
                debug!("Found album peak: {}", peak);
                tags.album_peak = Some(peak);
            }
        }
    }

    // If primary tag didn't have it, try other tags
    if !tags.has_track_tags() {
        for tag in tagged_file.tags() {
            if let Some(value) = tag.get_string(&lofty::ItemKey::ReplayGainTrackGain) {
                if let Some(gain) = parse_gain(value) {
                    tags.track_gain = Some(gain);
                }
            }
            if let Some(value) = tag.get_string(&lofty::ItemKey::ReplayGainTrackPeak) {
                if let Some(peak) = parse_peak(value) {
                    tags.track_peak = Some(peak);
                }
            }
            if let Some(value) = tag.get_string(&lofty::ItemKey::ReplayGainAlbumGain) {
                if let Some(gain) = parse_gain(value) {
                    tags.album_gain = Some(gain);
                }
            }
            if let Some(value) = tag.get_string(&lofty::ItemKey::ReplayGainAlbumPeak) {
                if let Some(peak) = parse_peak(value) {
                    tags.album_peak = Some(peak);
                }
            }

            if tags.has_track_tags() {
                break;
            }
        }
    }

    Ok(tags)
}

/// Write ReplayGain tags to an audio file
///
/// # Arguments
/// * `path` - Path to the audio file
/// * `track_gain` - Track gain to write (optional)
/// * `album_gain` - Album gain to write (optional)
///
/// # Notes
/// - Creates appropriate tag type based on file format
/// - Preserves existing non-ReplayGain tags
pub fn write_replaygain_tags<P: AsRef<Path>>(
    path: P,
    track_gain: Option<&TrackGain>,
    album_gain: Option<&AlbumGain>,
) -> Result<()> {
    let path = path.as_ref();

    if !path.exists() {
        return Err(LoudnessError::FileNotFound(path.display().to_string()));
    }

    let mut tagged_file = Probe::open(path)?.read()?;

    // Get or create the primary tag
    let tag_type = tagged_file.primary_tag_type();
    let tag = match tagged_file.tag_mut(tag_type) {
        Some(t) => t,
        None => {
            // Create a new tag if none exists
            tagged_file.insert_tag(lofty::Tag::new(tag_type));
            tagged_file.tag_mut(tag_type).unwrap()
        }
    };

    // Write track gain tags
    if let Some(tg) = track_gain {
        tag.insert_text(
            lofty::ItemKey::ReplayGainTrackGain,
            format!("{:.2} dB", tg.gain_db),
        );

        // Convert peak from dB to linear
        let peak_linear = 10.0_f64.powf(tg.peak_dbfs / 20.0);
        tag.insert_text(
            lofty::ItemKey::ReplayGainTrackPeak,
            format!("{:.6}", peak_linear),
        );
    }

    // Write album gain tags
    if let Some(ag) = album_gain {
        tag.insert_text(
            lofty::ItemKey::ReplayGainAlbumGain,
            format!("{:.2} dB", ag.gain_db),
        );

        // Convert peak from dB to linear
        let peak_linear = 10.0_f64.powf(ag.peak_dbfs / 20.0);
        tag.insert_text(
            lofty::ItemKey::ReplayGainAlbumPeak,
            format!("{:.6}", peak_linear),
        );
    }

    // Save the tag to the file
    tag.save_to_path(path)
        .map_err(|e: lofty::error::LoftyError| LoudnessError::TagWriteError(e.to_string()))?;

    debug!("Wrote ReplayGain tags to {:?}", path);

    Ok(())
}

/// Remove all ReplayGain tags from a file
///
/// # Arguments
/// * `path` - Path to the audio file
pub fn remove_replaygain_tags<P: AsRef<Path>>(path: P) -> Result<()> {
    let path = path.as_ref();

    if !path.exists() {
        return Err(LoudnessError::FileNotFound(path.display().to_string()));
    }

    let mut tagged_file = Probe::open(path)?.read()?;

    let replaygain_keys = [
        lofty::ItemKey::ReplayGainTrackGain,
        lofty::ItemKey::ReplayGainTrackPeak,
        lofty::ItemKey::ReplayGainAlbumGain,
        lofty::ItemKey::ReplayGainAlbumPeak,
    ];

    // Get the primary tag and remove keys
    let tag_type = tagged_file.primary_tag_type();
    if let Some(tag) = tagged_file.tag_mut(tag_type) {
        for key in &replaygain_keys {
            tag.remove_key(key);
        }
        tag.save_to_path(path)
            .map_err(|e: lofty::error::LoftyError| LoudnessError::TagWriteError(e.to_string()))?;
    }

    debug!("Removed ReplayGain tags from {:?}", path);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_gain_values() {
        assert_eq!(parse_gain("-5.23 dB"), Some(-5.23));
        assert_eq!(parse_gain("3.5dB"), Some(3.5));
        assert_eq!(parse_gain("-10.0"), Some(-10.0));
        assert_eq!(parse_gain("  2.5 dB  "), Some(2.5));
        assert!(parse_gain("invalid").is_none());
    }

    #[test]
    fn test_replaygain_tags_from_track_gain() {
        let track_gain = TrackGain {
            gain_db: -5.5,
            peak_dbfs: -1.0,
            integrated_lufs: -12.5,
            reference_lufs: -18.0,
        };

        let tags: ReplayGainTags = (&track_gain).into();
        assert_eq!(tags.track_gain, Some(-5.5));
        assert!(tags.has_track_tags());
        assert!(!tags.has_album_tags());

        // Peak should be converted from dB to linear
        // -1 dB = 10^(-1/20) ~ 0.891
        let expected_peak = 10.0_f64.powf(-1.0 / 20.0);
        assert!((tags.track_peak.unwrap() - expected_peak).abs() < 0.001);
    }

    #[test]
    fn test_peak_db_conversion() {
        let tags = ReplayGainTags {
            track_gain: Some(-5.0),
            track_peak: Some(0.891), // ~ -1 dB
            album_gain: Some(-4.0),
            album_peak: Some(1.0), // 0 dB
            reference_loudness: Some(-18.0),
        };

        let track_peak_db = tags.track_peak_db().unwrap();
        assert!((track_peak_db - (-1.0)).abs() < 0.1);

        let album_peak_db = tags.album_peak_db().unwrap();
        assert!((album_peak_db - 0.0).abs() < 0.001);
    }
}
