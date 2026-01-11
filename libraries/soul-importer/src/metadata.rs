//! Metadata extraction from audio files
//!
//! This module provides two extraction backends:
//! - `extract_metadata`: Uses lofty (default, more comprehensive tag support)
//! - `extract_metadata_symphonia`: Uses Symphonia (better for audio properties)
//!
//! Both return the same `ExtractedMetadata` struct for compatibility.

use crate::{ImportError, Result};
use lofty::{Accessor, AudioFile, Probe, TaggedFile, TaggedFileExt};
use std::path::Path;

/// Parsed folder name components
#[derive(Debug, Clone, Default)]
pub struct FolderMetadata {
    pub artist: Option<String>,
    pub album: Option<String>,
    pub year: Option<i32>,
}

/// Parse folder name for Artist - Album pattern
///
/// Supports common patterns:
/// - `Artist - Album`
/// - `Artist - Year - Album`
/// - `Year - Artist - Album`
///
/// Uses ` - ` (space-hyphen-space) as delimiter to avoid splitting
/// on hyphens within names.
pub fn parse_folder_name(folder_name: &str) -> FolderMetadata {
    let parts: Vec<&str> = folder_name.split(" - ").collect();

    match parts.len() {
        2 => {
            // Artist - Album
            let (artist, album) = (parts[0].trim(), parts[1].trim());

            // Check if first part is a year
            if let Some(year) = parse_year(artist) {
                FolderMetadata {
                    artist: None,
                    album: Some(album.to_string()),
                    year: Some(year),
                }
            } else {
                FolderMetadata {
                    artist: Some(artist.to_string()),
                    album: Some(album.to_string()),
                    year: None,
                }
            }
        }
        3 => {
            // Could be: Artist - Year - Album OR Year - Artist - Album
            let (p1, p2, p3) = (parts[0].trim(), parts[1].trim(), parts[2].trim());

            if let Some(year) = parse_year(p1) {
                // Year - Artist - Album
                FolderMetadata {
                    artist: Some(p2.to_string()),
                    album: Some(p3.to_string()),
                    year: Some(year),
                }
            } else if let Some(year) = parse_year(p2) {
                // Artist - Year - Album
                FolderMetadata {
                    artist: Some(p1.to_string()),
                    album: Some(p3.to_string()),
                    year: Some(year),
                }
            } else {
                // No year found, treat as Artist - Album (with dash in album)
                FolderMetadata {
                    artist: Some(p1.to_string()),
                    album: Some(format!("{} - {}", p2, p3)),
                    year: None,
                }
            }
        }
        n if n > 3 => {
            // First part is artist, rest is album (may contain dashes)
            let artist = parts[0].trim();
            let album_parts = &parts[1..];

            // Check if second part is a year
            if let Some(year) = parse_year(parts[1].trim()) {
                let album = album_parts[1..].join(" - ");
                FolderMetadata {
                    artist: Some(artist.to_string()),
                    album: if album.is_empty() {
                        None
                    } else {
                        Some(album)
                    },
                    year: Some(year),
                }
            } else {
                FolderMetadata {
                    artist: Some(artist.to_string()),
                    album: Some(album_parts.join(" - ")),
                    year: None,
                }
            }
        }
        _ => FolderMetadata::default(),
    }
}

/// Parse a string as a year (1900-2099)
fn parse_year(s: &str) -> Option<i32> {
    s.parse::<i32>()
        .ok()
        .filter(|&y| (1900..=2099).contains(&y))
}

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

/// Find the best tag from a file that has actual metadata
///
/// Files can have multiple tag types (ID3v1, ID3v2, APEv2, Vorbis comments).
/// The primary tag might be empty while another tag has all the data.
/// This function scores each tag and returns the one with the most useful metadata.
fn find_best_tag(file: &TaggedFile) -> Option<&lofty::Tag> {
    let tags = file.tags();

    if tags.is_empty() {
        return None;
    }

    // Score each tag by how much useful metadata it has
    let score_tag = |tag: &lofty::Tag| -> usize {
        let mut score = 0;
        if tag.artist().is_some() {
            score += 3; // Artist is most important
        }
        if tag.album().is_some() {
            score += 2;
        }
        if tag.title().is_some() {
            score += 1;
        }
        if tag.genre().is_some() {
            score += 1;
        }
        if tag.year().is_some() {
            score += 1;
        }
        score
    };

    // Find tag with highest score, fall back to primary_tag if all are empty
    let best = tags.iter().max_by_key(|t| score_tag(t));

    // If best tag has no data, try primary_tag as last resort
    if best.map(|t| score_tag(t)).unwrap_or(0) == 0 {
        file.primary_tag()
    } else {
        best
    }
}

/// Extract metadata from an audio file
pub fn extract_metadata(path: &Path) -> Result<ExtractedMetadata> {
    let tagged_file = Probe::open(path)
        .map_err(|e| ImportError::Metadata(format!("Failed to open file: {}", e)))?
        .read()
        .map_err(|e| ImportError::Metadata(format!("Failed to read file: {}", e)))?;

    // Find the best tag - prefer one with artist metadata
    // Files may have multiple tag types (ID3v1, ID3v2, APEv2, Vorbis) with data in different places
    let tag = find_best_tag(&tagged_file);

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

    // Fallback: Parse parent folder name for artist/album if missing from tags
    let folder_meta = if artist.is_none() || album.is_none() {
        let folder_name = path.parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str());

        if let Some(name) = folder_name {
            let parsed = parse_folder_name(name);
            eprintln!(
                "[metadata] Folder fallback for {:?}: folder='{}' -> artist={:?}, album={:?}",
                path.file_name(),
                name,
                parsed.artist,
                parsed.album
            );
            Some(parsed)
        } else {
            None
        }
    } else {
        eprintln!(
            "[metadata] Tags found for {:?}: artist={:?}, album={:?}",
            path.file_name(),
            artist,
            album
        );
        None
    };

    let artist = artist.or_else(|| folder_meta.as_ref().and_then(|m| m.artist.clone()));
    let album = album.or_else(|| folder_meta.as_ref().and_then(|m| m.album.clone()));
    let year = year.or_else(|| folder_meta.as_ref().and_then(|m| m.year));

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

    #[test]
    fn test_parse_folder_name_artist_album() {
        let result = parse_folder_name("Sebastián Stupák - Down- Below the Surface");
        assert_eq!(result.artist, Some("Sebastián Stupák".to_string()));
        assert_eq!(result.album, Some("Down- Below the Surface".to_string()));
        assert_eq!(result.year, None);
    }

    #[test]
    fn test_parse_folder_name_artist_year_album() {
        let result = parse_folder_name("Queen - 1975 - A Night at the Opera");
        assert_eq!(result.artist, Some("Queen".to_string()));
        assert_eq!(result.album, Some("A Night at the Opera".to_string()));
        assert_eq!(result.year, Some(1975));
    }

    #[test]
    fn test_parse_folder_name_year_artist_album() {
        let result = parse_folder_name("2020 - Artist Name - Album Title");
        assert_eq!(result.artist, Some("Artist Name".to_string()));
        assert_eq!(result.album, Some("Album Title".to_string()));
        assert_eq!(result.year, Some(2020));
    }

    #[test]
    fn test_parse_folder_name_no_pattern() {
        let result = parse_folder_name("Just an Album Name");
        assert_eq!(result.artist, None);
        assert_eq!(result.album, None);
        assert_eq!(result.year, None);
    }

    #[test]
    fn test_parse_folder_name_album_with_dashes() {
        let result = parse_folder_name("Artist - Part 1 - The Beginning - Remastered");
        assert_eq!(result.artist, Some("Artist".to_string()));
        assert_eq!(result.album, Some("Part 1 - The Beginning - Remastered".to_string()));
        assert_eq!(result.year, None);
    }

    #[test]
    fn test_parse_year() {
        assert_eq!(parse_year("1975"), Some(1975));
        assert_eq!(parse_year("2024"), Some(2024));
        assert_eq!(parse_year("1899"), None); // too old
        assert_eq!(parse_year("2100"), None); // too new
        assert_eq!(parse_year("abc"), None);
        assert_eq!(parse_year(""), None);
    }
}
