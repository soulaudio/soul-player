//! Metadata extraction from audio files
//!
//! This module provides two extraction backends:
//! - `extract_metadata`: Uses lofty (default, more comprehensive tag support)
//! - `extract_metadata_symphonia`: Uses Symphonia (better for audio properties)
//!
//! Both return the same `ExtractedMetadata` struct for compatibility.

use crate::{ImportError, Result};
use lofty::{Accessor, AudioFile, Probe, TaggedFileExt};
use std::path::Path;

/// Extracted metadata from an audio file
#[derive(Debug, Clone)]
pub struct ExtractedMetadata {
    /// Track title
    pub title: Option<String>,

    /// Artist name
    pub artist: Option<String>,

    /// Album title
    pub album: Option<String>,

    /// Album artist (may differ from track artist)
    pub album_artist: Option<String>,

    /// Track number
    pub track_number: Option<u32>,

    /// Disc number
    pub disc_number: Option<u32>,

    /// Year
    pub year: Option<i32>,

    /// Genres (can be multiple)
    pub genres: Vec<String>,

    /// Duration in seconds
    pub duration_seconds: Option<f64>,

    /// Bitrate in kbps
    pub bitrate: Option<u32>,

    /// Sample rate in Hz
    pub sample_rate: Option<u32>,

    /// Number of channels
    pub channels: Option<u8>,

    /// File format (extension)
    pub file_format: String,

    /// MusicBrainz Recording ID
    pub musicbrainz_recording_id: Option<String>,

    /// Composer
    pub composer: Option<String>,

    /// Embedded album art (raw data and MIME type)
    pub album_art: Option<(Vec<u8>, String)>,
}

impl ExtractedMetadata {
    /// Check if metadata is mostly empty (only title or filename)
    pub fn is_sparse(&self) -> bool {
        self.artist.is_none() && self.album.is_none() && self.genres.is_empty()
    }
}

/// Extract metadata from an audio file
pub fn extract_metadata(path: &Path) -> Result<ExtractedMetadata> {
    let tagged_file = Probe::open(path)
        .map_err(|e| ImportError::Metadata(format!("Failed to open file: {}", e)))?
        .read()
        .map_err(|e| ImportError::Metadata(format!("Failed to read file: {}", e)))?;

    // Get primary tag (prefer ID3v2 for MP3, Vorbis for OGG/FLAC)
    let tag = tagged_file.primary_tag().or(tagged_file.first_tag());

    // Extract audio properties
    let properties = tagged_file.properties();
    let duration_seconds = properties.duration().as_secs_f64();
    let bitrate = properties.audio_bitrate();
    let sample_rate = properties.sample_rate();
    let channels = properties.channels();

    // Extract tag metadata
    let (title, artist, album, album_artist, track_number, disc_number, year, genres) =
        if let Some(tag) = tag {
            let title = tag.title().map(|s| s.to_string());
            let artist = tag.artist().map(|s| s.to_string());
            let album = tag.album().map(|s| s.to_string());
            let album_artist = tag
                .get_string(&lofty::ItemKey::AlbumArtist)
                .map(|s| s.to_string());
            let track_number = tag.track();
            let disc_number = tag.disk();
            let year = tag.year().map(|y| y as i32);

            // Extract genres (can be multiple, separated by various delimiters)
            let genres: Vec<String> = tag
                .genre()
                .map(|g: std::borrow::Cow<'_, str>| {
                    g.split(&[',', ';', '/'][..])
                        .map(|s: &str| s.trim().to_string())
                        .filter(|s: &String| !s.is_empty())
                        .collect::<Vec<String>>()
                })
                .unwrap_or_default();

            (
                title,
                artist,
                album,
                album_artist,
                track_number,
                disc_number,
                year,
                genres,
            )
        } else {
            (None, None, None, None, None, None, None, Vec::new())
        };

    // Fallback: Use filename as title if no title in tags
    let title: Option<String> = title.or_else(|| {
        path.file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
    });

    // Get file format from extension
    let file_format = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_lowercase())
        .unwrap_or_else(|| "unknown".to_string());

    // Extract MusicBrainz Recording ID
    let musicbrainz_recording_id = tag.and_then(|t| {
        t.get_string(&lofty::ItemKey::MusicBrainzRecordingId)
            .map(|s| s.to_string())
    });

    // Extract composer
    let composer = tag.and_then(|t| {
        t.get_string(&lofty::ItemKey::Composer)
            .map(|s| s.to_string())
    });

    // Extract album art
    let album_art = tag.and_then(|t| {
        t.pictures().first().map(|pic| {
            let mime = pic.mime_type().map(|m| m.as_str().to_string())
                .unwrap_or_else(|| "image/jpeg".to_string());
            (pic.data().to_vec(), mime)
        })
    });

    Ok(ExtractedMetadata {
        title,
        artist,
        album,
        album_artist,
        track_number,
        disc_number,
        year,
        genres,
        duration_seconds: Some(duration_seconds),
        bitrate,
        sample_rate,
        channels,
        file_format,
        musicbrainz_recording_id,
        composer,
        album_art,
    })
}

/// Calculate SHA-256 hash of a file for duplicate detection
pub fn calculate_file_hash(path: &Path) -> Result<String> {
    use sha2::{Digest, Sha256};
    use std::fs::File;
    use std::io::Read;

    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    let hash = hasher.finalize();
    Ok(hex::encode(hash))
}

/// Extract metadata from an audio file using Symphonia
///
/// This provides an alternative to the lofty-based extraction.
/// Symphonia is the same decoder used for audio playback, so it may
/// provide more accurate audio property information for some formats.
///
/// # Arguments
/// * `path` - Path to the audio file
///
/// # Returns
/// * `ExtractedMetadata` with all available metadata
pub fn extract_metadata_symphonia(path: &Path) -> Result<ExtractedMetadata> {
    let audio_metadata = soul_audio::extract_metadata(path)
        .map_err(|e| ImportError::Metadata(e.to_string()))?;

    // Convert genre from single string to vec
    let genres = audio_metadata
        .genre
        .map(|g| {
            g.split(&[',', ';', '/'][..])
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect::<Vec<String>>()
        })
        .unwrap_or_default();

    // Get file format from extension
    let file_format = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_lowercase())
        .unwrap_or_else(|| "unknown".to_string());

    // Convert album art
    let album_art = audio_metadata.album_art.map(|art| {
        (art.data, art.mime_type)
    });

    Ok(ExtractedMetadata {
        title: audio_metadata.title,
        artist: audio_metadata.artist,
        album: audio_metadata.album,
        album_artist: audio_metadata.album_artist,
        track_number: audio_metadata.track_number,
        disc_number: audio_metadata.disc_number,
        year: audio_metadata.year,
        genres,
        duration_seconds: audio_metadata.duration_seconds,
        bitrate: audio_metadata.bitrate,
        sample_rate: audio_metadata.sample_rate,
        channels: audio_metadata.channels,
        file_format,
        musicbrainz_recording_id: audio_metadata.musicbrainz_recording_id,
        composer: audio_metadata.composer,
        album_art,
    })
}

/// Convert soul-audio AudioMetadata to ExtractedMetadata
impl From<soul_audio::AudioMetadata> for ExtractedMetadata {
    fn from(meta: soul_audio::AudioMetadata) -> Self {
        let genres = meta
            .genre
            .map(|g| {
                g.split(&[',', ';', '/'][..])
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<String>>()
            })
            .unwrap_or_default();

        let album_art = meta.album_art.map(|art| (art.data, art.mime_type));

        Self {
            title: meta.title,
            artist: meta.artist,
            album: meta.album,
            album_artist: meta.album_artist,
            track_number: meta.track_number,
            disc_number: meta.disc_number,
            year: meta.year,
            genres,
            duration_seconds: meta.duration_seconds,
            bitrate: meta.bitrate,
            sample_rate: meta.sample_rate,
            channels: meta.channels,
            file_format: "unknown".to_string(), // Would need path to determine
            musicbrainz_recording_id: meta.musicbrainz_recording_id,
            composer: meta.composer,
            album_art,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata_is_sparse() {
        let sparse = ExtractedMetadata {
            title: Some("Test".to_string()),
            artist: None,
            album: None,
            album_artist: None,
            track_number: None,
            disc_number: None,
            year: None,
            genres: Vec::new(),
            duration_seconds: Some(180.0),
            bitrate: None,
            sample_rate: None,
            channels: None,
            file_format: "mp3".to_string(),
            musicbrainz_recording_id: None,
            composer: None,
            album_art: None,
        };

        assert!(sparse.is_sparse());

        let not_sparse = ExtractedMetadata {
            title: Some("Test".to_string()),
            artist: Some("Artist".to_string()),
            album: None,
            album_artist: None,
            track_number: None,
            disc_number: None,
            year: None,
            genres: Vec::new(),
            duration_seconds: Some(180.0),
            bitrate: None,
            sample_rate: None,
            channels: None,
            file_format: "mp3".to_string(),
            musicbrainz_recording_id: None,
            composer: None,
            album_art: None,
        };

        assert!(!not_sparse.is_sparse());
    }
}
