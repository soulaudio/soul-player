//! Audio metadata extraction using Symphonia
//!
//! This module provides comprehensive metadata extraction from audio files,
//! including tag data and embedded album art.

use crate::error::{AudioError, Result};
use std::collections::HashMap;
use std::path::Path;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::{MetadataOptions, StandardTagKey, Value, Visual};
use symphonia::core::probe::Hint;

/// Type of album art
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlbumArtType {
    /// Front cover
    FrontCover,
    /// Back cover
    BackCover,
    /// Leaflet page
    Leaflet,
    /// Media (CD label)
    Media,
    /// Lead artist/performer
    LeadArtist,
    /// Artist/performer
    Artist,
    /// Conductor
    Conductor,
    /// Band/orchestra
    Band,
    /// Composer
    Composer,
    /// Lyricist/writer
    Lyricist,
    /// Recording location
    RecordingLocation,
    /// During recording
    DuringRecording,
    /// During performance
    DuringPerformance,
    /// Movie/video screenshot
    ScreenCapture,
    /// Bright colored fish
    BrightColoredFish,
    /// Illustration
    Illustration,
    /// Band/artist logo
    BandLogo,
    /// Publisher/studio logo
    PublisherLogo,
    /// Other/unknown type
    Other,
}

impl From<Option<symphonia::core::meta::StandardVisualKey>> for AlbumArtType {
    fn from(key: Option<symphonia::core::meta::StandardVisualKey>) -> Self {
        use symphonia::core::meta::StandardVisualKey;
        match key {
            Some(StandardVisualKey::FrontCover) => AlbumArtType::FrontCover,
            Some(StandardVisualKey::BackCover) => AlbumArtType::BackCover,
            Some(StandardVisualKey::Leaflet) => AlbumArtType::Leaflet,
            Some(StandardVisualKey::Media) => AlbumArtType::Media,
            Some(StandardVisualKey::LeadArtistPerformerSoloist) => AlbumArtType::LeadArtist,
            Some(StandardVisualKey::ArtistPerformer) => AlbumArtType::Artist,
            Some(StandardVisualKey::Conductor) => AlbumArtType::Conductor,
            Some(StandardVisualKey::BandOrchestra) => AlbumArtType::Band,
            Some(StandardVisualKey::Composer) => AlbumArtType::Composer,
            Some(StandardVisualKey::Lyricist) => AlbumArtType::Lyricist,
            Some(StandardVisualKey::RecordingLocation) => AlbumArtType::RecordingLocation,
            Some(StandardVisualKey::RecordingSession) => AlbumArtType::DuringRecording,
            Some(StandardVisualKey::Performance) => AlbumArtType::DuringPerformance,
            Some(StandardVisualKey::ScreenCapture) => AlbumArtType::ScreenCapture,
            Some(StandardVisualKey::Illustration) => AlbumArtType::Illustration,
            // BandLogo and PublisherLogo types map to Other
            // OtherIcon, FileIcon, and None also map to Other
            _ => AlbumArtType::Other,
        }
    }
}

/// Embedded album art data
#[derive(Debug, Clone)]
pub struct AlbumArt {
    /// Raw image data
    pub data: Vec<u8>,
    /// MIME type (e.g., "image/jpeg", "image/png")
    pub mime_type: String,
    /// Type of artwork (front cover, back cover, etc.)
    pub art_type: AlbumArtType,
    /// Optional description
    pub description: Option<String>,
}

impl AlbumArt {
    /// Create new album art
    pub fn new(data: Vec<u8>, mime_type: impl Into<String>, art_type: AlbumArtType) -> Self {
        Self {
            data,
            mime_type: mime_type.into(),
            art_type,
            description: None,
        }
    }

    /// Set description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Check if this is a front cover
    pub fn is_front_cover(&self) -> bool {
        self.art_type == AlbumArtType::FrontCover
    }

    /// Detect MIME type from image magic bytes if not provided
    pub fn detect_mime_type(&self) -> &str {
        if !self.mime_type.is_empty() && self.mime_type != "application/octet-stream" {
            return &self.mime_type;
        }

        // Detect from magic bytes
        if self.data.len() >= 3 {
            // JPEG: starts with 0xFF 0xD8 0xFF
            if self.data[0] == 0xFF && self.data[1] == 0xD8 && self.data[2] == 0xFF {
                return "image/jpeg";
            }
            // PNG: starts with 0x89 0x50 0x4E 0x47 (0x89 'P' 'N' 'G')
            if self.data.len() >= 4
                && self.data[0] == 0x89
                && self.data[1] == 0x50
                && self.data[2] == 0x4E
                && self.data[3] == 0x47
            {
                return "image/png";
            }
            // GIF: starts with "GIF"
            if self.data.len() >= 3
                && self.data[0] == b'G'
                && self.data[1] == b'I'
                && self.data[2] == b'F'
            {
                return "image/gif";
            }
            // BMP: starts with "BM"
            if self.data.len() >= 2 && self.data[0] == b'B' && self.data[1] == b'M' {
                return "image/bmp";
            }
            // WebP: starts with "RIFF" and contains "WEBP"
            if self.data.len() >= 12
                && &self.data[0..4] == b"RIFF"
                && &self.data[8..12] == b"WEBP"
            {
                return "image/webp";
            }
        }

        &self.mime_type
    }
}

/// Audio metadata extracted from a file
#[derive(Debug, Clone, Default)]
pub struct AudioMetadata {
    /// Track title
    pub title: Option<String>,
    /// Track artist
    pub artist: Option<String>,
    /// Album name
    pub album: Option<String>,
    /// Album artist (may differ from track artist for compilations)
    pub album_artist: Option<String>,
    /// Release year
    pub year: Option<i32>,
    /// Track number within the disc
    pub track_number: Option<u32>,
    /// Total number of tracks on the disc
    pub track_total: Option<u32>,
    /// Disc number
    pub disc_number: Option<u32>,
    /// Total number of discs
    pub disc_total: Option<u32>,
    /// Genre
    pub genre: Option<String>,
    /// Composer
    pub composer: Option<String>,
    /// Duration in seconds
    pub duration_seconds: Option<f64>,
    /// Sample rate in Hz
    pub sample_rate: Option<u32>,
    /// Number of audio channels
    pub channels: Option<u8>,
    /// Bit depth (bits per sample)
    pub bit_depth: Option<u32>,
    /// Bitrate in kbps
    pub bitrate: Option<u32>,
    /// Comment
    pub comment: Option<String>,
    /// Lyrics
    pub lyrics: Option<String>,
    /// Conductor
    pub conductor: Option<String>,
    /// Label/Publisher
    pub label: Option<String>,
    /// Copyright
    pub copyright: Option<String>,
    /// Original release date
    pub original_date: Option<String>,
    /// MusicBrainz Recording ID
    pub musicbrainz_recording_id: Option<String>,
    /// MusicBrainz Album ID
    pub musicbrainz_album_id: Option<String>,
    /// MusicBrainz Artist ID
    pub musicbrainz_artist_id: Option<String>,
    /// MusicBrainz Release Group ID
    pub musicbrainz_release_group_id: Option<String>,
    /// ISRC (International Standard Recording Code)
    pub isrc: Option<String>,
    /// Catalog number
    pub catalog_number: Option<String>,
    /// Barcode
    pub barcode: Option<String>,
    /// ReplayGain track gain in dB
    pub replaygain_track_gain: Option<f32>,
    /// ReplayGain track peak
    pub replaygain_track_peak: Option<f32>,
    /// ReplayGain album gain in dB
    pub replaygain_album_gain: Option<f32>,
    /// ReplayGain album peak
    pub replaygain_album_peak: Option<f32>,
    /// Embedded album art (first/primary image)
    pub album_art: Option<AlbumArt>,
    /// All embedded images
    pub all_album_art: Vec<AlbumArt>,
    /// Custom/extended tags
    pub custom_tags: HashMap<String, Vec<String>>,
}

impl AudioMetadata {
    /// Create new empty metadata
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if metadata is sparse (only has filename-derived title)
    pub fn is_sparse(&self) -> bool {
        self.artist.is_none() && self.album.is_none() && self.genre.is_none()
    }

    /// Get the primary album art (front cover preferred)
    pub fn primary_album_art(&self) -> Option<&AlbumArt> {
        // First try to find front cover in all art
        for art in &self.all_album_art {
            if art.is_front_cover() {
                return Some(art);
            }
        }
        // Fall back to first available art
        self.all_album_art.first().or(self.album_art.as_ref())
    }

    /// Add a custom tag
    pub fn add_custom_tag(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.custom_tags
            .entry(key.into())
            .or_default()
            .push(value.into());
    }

    /// Get a custom tag value
    pub fn get_custom_tag(&self, key: &str) -> Option<&str> {
        self.custom_tags.get(key).and_then(|v| v.first().map(|s| s.as_str()))
    }
}

/// Extract metadata from an audio file using Symphonia
///
/// This function probes the file and extracts all available metadata
/// including tags and embedded artwork.
pub fn extract_metadata(path: &Path) -> Result<AudioMetadata> {
    // Check if file exists
    if !path.exists() {
        return Err(AudioError::FileNotFound(path.display().to_string()));
    }

    // Open the file
    let file = std::fs::File::open(path).map_err(AudioError::Io)?;

    // Create media source
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    // Create hint from extension
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    // Probe the media source with metadata enabled
    let mut probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|e| AudioError::DecodeError(format!("Failed to probe file: {}", e)))?;

    let mut metadata = AudioMetadata::new();

    // Extract audio properties from the default track
    if let Some(track) = probed.format.default_track() {
        let params = &track.codec_params;

        metadata.sample_rate = params.sample_rate;
        metadata.channels = params.channels.map(|c| c.count() as u8);
        metadata.bit_depth = params.bits_per_sample;

        // Calculate duration if we have time_base and n_frames
        if let (Some(time_base), Some(n_frames)) = (params.time_base, params.n_frames) {
            let duration = time_base.calc_time(n_frames);
            metadata.duration_seconds = Some(duration.seconds as f64 + duration.frac);
        }
    }

    // Extract metadata from the probe result
    // Symphonia provides metadata in two places:
    // 1. probed.metadata - metadata found during initial probe (e.g., ID3v2 at start)
    // 2. probed.format.metadata() - metadata found in container (e.g., Vorbis comments)

    // First, try the probe metadata
    if let Some(metadata_log) = probed.metadata.get() {
        if let Some(revision) = metadata_log.current() {
            extract_from_revision(revision, &mut metadata);
        }
    }

    // Then try container metadata (may override probe metadata)
    let format_metadata = probed.format.metadata();
    if let Some(revision) = format_metadata.current() {
        extract_from_revision(revision, &mut metadata);
    }

    // If no title was found, use filename as fallback
    if metadata.title.is_none() {
        metadata.title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string());
    }

    Ok(metadata)
}

/// Extract metadata from a Symphonia metadata revision
fn extract_from_revision(
    revision: &symphonia::core::meta::MetadataRevision,
    metadata: &mut AudioMetadata,
) {
    // Extract tags
    for tag in revision.tags() {
        let value = extract_tag_value(&tag.value);
        if value.is_empty() {
            continue;
        }

        // Handle standard tag keys
        if let Some(std_key) = tag.std_key {
            match std_key {
                StandardTagKey::TrackTitle => {
                    metadata.title = Some(value.clone());
                }
                StandardTagKey::Artist => {
                    metadata.artist = Some(value.clone());
                }
                StandardTagKey::Album => {
                    metadata.album = Some(value.clone());
                }
                StandardTagKey::AlbumArtist => {
                    metadata.album_artist = Some(value.clone());
                }
                StandardTagKey::Date | StandardTagKey::OriginalDate => {
                    // Try to parse year from date string
                    if metadata.year.is_none() {
                        metadata.year = parse_year(&value);
                    }
                    if std_key == StandardTagKey::OriginalDate {
                        metadata.original_date = Some(value.clone());
                    }
                }
                StandardTagKey::TrackNumber => {
                    let (num, total) = parse_track_number(&value);
                    metadata.track_number = num;
                    if total.is_some() && metadata.track_total.is_none() {
                        metadata.track_total = total;
                    }
                }
                StandardTagKey::TrackTotal => {
                    if let Ok(n) = value.parse::<u32>() {
                        metadata.track_total = Some(n);
                    }
                }
                StandardTagKey::DiscNumber => {
                    let (num, total) = parse_track_number(&value);
                    metadata.disc_number = num;
                    if total.is_some() && metadata.disc_total.is_none() {
                        metadata.disc_total = total;
                    }
                }
                StandardTagKey::DiscTotal => {
                    if let Ok(n) = value.parse::<u32>() {
                        metadata.disc_total = Some(n);
                    }
                }
                StandardTagKey::Genre => {
                    metadata.genre = Some(value.clone());
                }
                StandardTagKey::Composer => {
                    metadata.composer = Some(value.clone());
                }
                StandardTagKey::Comment => {
                    metadata.comment = Some(value.clone());
                }
                StandardTagKey::Lyrics => {
                    metadata.lyrics = Some(value.clone());
                }
                StandardTagKey::Conductor => {
                    metadata.conductor = Some(value.clone());
                }
                StandardTagKey::Label => {
                    metadata.label = Some(value.clone());
                }
                StandardTagKey::Copyright => {
                    metadata.copyright = Some(value.clone());
                }
                StandardTagKey::MusicBrainzRecordingId => {
                    metadata.musicbrainz_recording_id = Some(value.clone());
                }
                StandardTagKey::MusicBrainzAlbumId => {
                    metadata.musicbrainz_album_id = Some(value.clone());
                }
                StandardTagKey::MusicBrainzArtistId => {
                    metadata.musicbrainz_artist_id = Some(value.clone());
                }
                StandardTagKey::MusicBrainzReleaseGroupId => {
                    metadata.musicbrainz_release_group_id = Some(value.clone());
                }
                StandardTagKey::IdentIsrc => {
                    metadata.isrc = Some(value.clone());
                }
                StandardTagKey::IdentCatalogNumber => {
                    metadata.catalog_number = Some(value.clone());
                }
                StandardTagKey::IdentBarcode => {
                    metadata.barcode = Some(value.clone());
                }
                StandardTagKey::ReplayGainTrackGain => {
                    metadata.replaygain_track_gain = parse_replaygain(&value);
                }
                StandardTagKey::ReplayGainTrackPeak => {
                    metadata.replaygain_track_peak = value.parse().ok();
                }
                StandardTagKey::ReplayGainAlbumGain => {
                    metadata.replaygain_album_gain = parse_replaygain(&value);
                }
                StandardTagKey::ReplayGainAlbumPeak => {
                    metadata.replaygain_album_peak = value.parse().ok();
                }
                _ => {
                    // Store as custom tag
                    metadata.add_custom_tag(tag.key.clone(), value);
                }
            }
        } else {
            // Non-standard tag - store as custom
            metadata.add_custom_tag(tag.key.clone(), value);
        }
    }

    // Extract album art
    for visual in revision.visuals() {
        let art = extract_album_art(visual);
        metadata.all_album_art.push(art);
    }

    // Set primary album art
    if metadata.album_art.is_none() && !metadata.all_album_art.is_empty() {
        // Prefer front cover
        let primary = metadata.all_album_art
            .iter()
            .find(|a| a.is_front_cover())
            .cloned()
            .unwrap_or_else(|| metadata.all_album_art[0].clone());
        metadata.album_art = Some(primary);
    }
}

/// Extract string value from a Symphonia tag Value
fn extract_tag_value(value: &Value) -> String {
    match value {
        Value::Binary(_) => String::new(), // Skip binary values for text tags
        Value::Boolean(b) => b.to_string(),
        Value::Flag => String::new(),
        Value::Float(f) => f.to_string(),
        Value::SignedInt(i) => i.to_string(),
        Value::UnsignedInt(u) => u.to_string(),
        Value::String(s) => s.trim().to_string(),
    }
}

/// Extract album art from a Symphonia Visual
fn extract_album_art(visual: &Visual) -> AlbumArt {
    let art_type = AlbumArtType::from(visual.usage);
    let mut art = AlbumArt::new(
        visual.data.to_vec(),
        visual.media_type.clone(),
        art_type,
    );

    // Add any tags as description
    if !visual.tags.is_empty() {
        let desc: Vec<String> = visual.tags.iter()
            .map(|t| format!("{}: {}", t.key, extract_tag_value(&t.value)))
            .collect();
        art.description = Some(desc.join(", "));
    }

    art
}

/// Parse year from various date formats
fn parse_year(value: &str) -> Option<i32> {
    // Try direct year parse (e.g., "2024")
    if let Ok(year) = value.parse::<i32>() {
        if (1000..=9999).contains(&year) {
            return Some(year);
        }
    }

    // Try to extract year from ISO date (e.g., "2024-06-15")
    if value.len() >= 4 {
        if let Ok(year) = value[..4].parse::<i32>() {
            if (1000..=9999).contains(&year) {
                return Some(year);
            }
        }
    }

    None
}

/// Parse track/disc number which may be in "N" or "N/M" format
fn parse_track_number(value: &str) -> (Option<u32>, Option<u32>) {
    if let Some(slash_pos) = value.find('/') {
        let num = value[..slash_pos].trim().parse().ok();
        let total = value[slash_pos + 1..].trim().parse().ok();
        (num, total)
    } else {
        (value.trim().parse().ok(), None)
    }
}

/// Parse ReplayGain value (e.g., "-3.45 dB" or "+1.23 dB")
fn parse_replaygain(value: &str) -> Option<f32> {
    let value = value.trim().to_lowercase();
    let value = value.trim_end_matches("db").trim();
    value.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_year() {
        assert_eq!(parse_year("2024"), Some(2024));
        assert_eq!(parse_year("2024-06-15"), Some(2024));
        assert_eq!(parse_year("1999"), Some(1999));
        assert_eq!(parse_year("invalid"), None);
        assert_eq!(parse_year("123"), None); // Too short
    }

    #[test]
    fn test_parse_track_number() {
        assert_eq!(parse_track_number("5"), (Some(5), None));
        assert_eq!(parse_track_number("5/12"), (Some(5), Some(12)));
        assert_eq!(parse_track_number(" 7 / 15 "), (Some(7), Some(15)));
        assert_eq!(parse_track_number("invalid"), (None, None));
    }

    #[test]
    fn test_parse_replaygain() {
        assert_eq!(parse_replaygain("-3.45 dB"), Some(-3.45));
        assert_eq!(parse_replaygain("+1.23 dB"), Some(1.23));
        assert_eq!(parse_replaygain("-3.45"), Some(-3.45));
        assert_eq!(parse_replaygain("invalid"), None);
    }

    #[test]
    fn test_album_art_detect_mime() {
        // JPEG magic bytes
        let jpeg = AlbumArt::new(
            vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x00],
            "",
            AlbumArtType::FrontCover,
        );
        assert_eq!(jpeg.detect_mime_type(), "image/jpeg");

        // PNG magic bytes
        let png = AlbumArt::new(
            vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A],
            "",
            AlbumArtType::FrontCover,
        );
        assert_eq!(png.detect_mime_type(), "image/png");

        // Already has mime type
        let with_mime = AlbumArt::new(
            vec![0x00, 0x00],
            "image/webp",
            AlbumArtType::FrontCover,
        );
        assert_eq!(with_mime.detect_mime_type(), "image/webp");
    }

    #[test]
    fn test_metadata_is_sparse() {
        let mut metadata = AudioMetadata::new();
        assert!(metadata.is_sparse());

        metadata.artist = Some("Artist".to_string());
        assert!(!metadata.is_sparse());

        metadata.artist = None;
        metadata.album = Some("Album".to_string());
        assert!(!metadata.is_sparse());
    }

    #[test]
    fn test_custom_tags() {
        let mut metadata = AudioMetadata::new();
        metadata.add_custom_tag("BPM", "128");
        metadata.add_custom_tag("BPM", "130"); // Multiple values

        assert_eq!(metadata.get_custom_tag("BPM"), Some("128"));
        assert_eq!(metadata.custom_tags.get("BPM").map(|v| v.len()), Some(2));
    }
}
