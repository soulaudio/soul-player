//! Metadata extraction from audio files
//!
//! This module provides two extraction backends:
//! - `extract_metadata`: Uses lofty (default, more comprehensive tag support)
//! - `extract_metadata_symphonia`: Uses Symphonia (better for audio properties)
//!
//! Both return the same `ExtractedMetadata` struct for compatibility.

use crate::{ImportError, Result};
use lofty::{Accessor, AudioFile, Probe, TagType, TaggedFile, TaggedFileExt};
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
                    album: if album.is_empty() { None } else { Some(album) },
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

/// Decode an ID3v2 text frame value.
///
/// ID3v2 text frames begin with a single encoding byte:
///   0x00 — ISO-8859-1 (Latin-1): each byte maps directly to the same Unicode code point
///   0x01 — UTF-16 with BOM (almost always UTF-16 LE with 0xFF 0xFE BOM)
///   0x02 — UTF-16 BE without BOM
///   0x03 — UTF-8
///
/// Previously this fell through to `from_utf8_lossy` for UTF-16, producing
/// garbled null-padded strings (e.g. "O\0s\0a\0k\0i\0" instead of "Osaki").
/// ISO-8859-1 was also incorrectly decoded via from_utf8_lossy, turning bytes
/// 0x80–0xFF (é, ü, ñ, etc.) into U+FFFD replacement characters.
fn decode_id3_text(frame_data: &[u8]) -> Option<String> {
    if frame_data.is_empty() {
        return None;
    }
    let encoding = frame_data[0];
    let raw = &frame_data[1..];
    let s = match encoding {
        0 => {
            // ISO-8859-1: every byte maps 1:1 to the Unicode code point of the same value.
            // This cannot use from_utf8_lossy (bytes 0x80–0xFF are not valid UTF-8).
            raw.iter()
                .copied()
                .take_while(|&b| b != 0) // strip null terminator
                .map(char::from)
                .collect()
        }
        1 => {
            // UTF-16 with BOM — strip 0xFF 0xFE / 0xFE 0xFF BOM if present.
            let (is_be, payload) = if raw.starts_with(&[0xFF, 0xFE]) {
                (false, &raw[2..])
            } else if raw.starts_with(&[0xFE, 0xFF]) {
                (true, &raw[2..])
            } else {
                // No BOM — assume LE (most common in practice).
                (false, raw)
            };
            let units: Vec<u16> = payload
                .chunks(2)
                .filter(|c| c.len() == 2)
                .map(|c| {
                    if is_be {
                        u16::from_be_bytes([c[0], c[1]])
                    } else {
                        u16::from_le_bytes([c[0], c[1]])
                    }
                })
                .take_while(|&u| u != 0) // strip null terminator
                .collect();
            String::from_utf16_lossy(&units)
        }
        2 => {
            // UTF-16 BE without BOM.
            let units: Vec<u16> = raw
                .chunks(2)
                .filter(|c| c.len() == 2)
                .map(|c| u16::from_be_bytes([c[0], c[1]]))
                .take_while(|&u| u != 0)
                .collect();
            String::from_utf16_lossy(&units)
        }
        // 0x03 = UTF-8, and any unknown encoding — treat as UTF-8 lossy.
        _ => String::from_utf8_lossy(raw)
            .trim_end_matches('\0')
            .to_string(),
    };
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

/// Parse a string as a year (1900-2099)
fn parse_year(s: &str) -> Option<i32> {
    s.parse::<i32>()
        .ok()
        .filter(|&y| (1900..=2099).contains(&y))
}

/// Split a single artist tag string on featuring credits only.
///
/// Only splits on `feat.` / `ft.` variants — these unambiguously indicate a
/// second artist credit in a single-value field (e.g. "Artist A feat. Artist B").
///
/// Deliberately does NOT split on `,`, `;`, `&`, or `x` because those characters
/// legitimately appear in band names (e.g. "Earth, Wind & Fire", "Simon & Garfunkel").
/// When metadata natively provides multiple ARTIST values (Vorbis multi-tag, ID3v2.4
/// null-separated), those are used directly and this function is not needed.
pub fn split_artists(raw: &str) -> Vec<String> {
    // Only featuring credit delimiters — all case variants.
    const DELIMITERS: &[&str] = &[
        " feat. ", " Feat. ", " FEAT. ", " feat ", " Feat ", " FEAT ", " ft. ", " Ft. ", " FT. ",
        " ft ", " Ft ", " FT ",
    ];

    let mut results = vec![raw.to_string()];
    for &delim in DELIMITERS {
        results = results
            .into_iter()
            .flat_map(|s| {
                s.split(delim)
                    .map(|p| p.trim().to_string())
                    .collect::<Vec<_>>()
            })
            .collect();
    }

    results.into_iter().filter(|s| !s.is_empty()).collect()
}

/// Extracted metadata from an audio file
#[derive(Debug, Clone)]
pub struct ExtractedMetadata {
    /// Track title
    pub title: Option<String>,

    /// Artist names (split from tag; may be multiple)
    pub artists: Vec<String>,

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
        self.artists.is_empty() && self.album.is_none() && self.genres.is_empty()
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

    // Score each tag by how much useful metadata it has.
    // ID3v1 is ASCII-only — non-Latin characters become '?' substitutions.
    // Penalise ID3v1 so that ID3v2/Vorbis/APE tags win when both are present.
    // Also penalise any tag whose important fields contain only '?' (corrupted).
    let score_tag = |tag: &lofty::Tag| -> i32 {
        // ID3v1 base penalty: prefer richer tag formats for non-ASCII support
        let mut score: i32 = if tag.tag_type() == TagType::Id3v1 {
            -10
        } else {
            0
        };

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

        // Penalise '?' substitutions — indicates the encoding couldn't represent the chars
        let has_question_marks = |s: std::borrow::Cow<'_, str>| s.contains('?');
        if tag.artist().map(has_question_marks).unwrap_or(false) {
            score -= 2;
        }
        if tag.album().map(has_question_marks).unwrap_or(false) {
            score -= 1;
        }
        if tag.title().map(has_question_marks).unwrap_or(false) {
            score -= 1;
        }

        score
    };

    // Find tag with highest score, fall back to primary_tag if all are empty
    let best = tags.iter().max_by_key(|t| score_tag(t));

    // If best tag has no data, try primary_tag as last resort
    if best.map(score_tag).unwrap_or(0) <= 0 {
        file.primary_tag()
    } else {
        best
    }
}

/// Extract metadata from an audio file
pub fn extract_metadata(path: &Path) -> Result<ExtractedMetadata> {
    tracing::debug!(file_path = %path.display(), "[Metadata] Extracting metadata");
    let start = std::time::Instant::now();

    // Check if this is a DSD file (DSF/DFF) — lofty 0.18 doesn't support these natively
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    if ext == "dsf" || ext == "dff" || ext == "dsdiff" {
        return extract_dsd_metadata(path);
    }

    // BWF WAV: try our RIFF chunk parser first.
    // Falls through to lofty on failure (standard WAVs lofty handles correctly).
    if ext == "wav" || ext == "wave" {
        match extract_wav_metadata(path) {
            Ok(meta) => return Ok(meta),
            Err(e) => {
                tracing::debug!(
                    file_path = %path.display(),
                    error = %e,
                    "[Metadata] WAV custom parser failed, falling through to lofty"
                );
            }
        }
    }

    let probe_start = std::time::Instant::now();
    let tagged_file_result = Probe::open(path)
        .map_err(|e| {
            tracing::error!(file_path = %path.display(), error = %e, "[Metadata] Failed to open file");
            ImportError::Metadata(format!("Failed to open file: {}", e))
        })?
        .read();

    // If lofty fails to parse the file, return an error so the scanner can skip it
    // and count it as an error rather than silently importing with wrong metadata.
    //
    // TODO(BWF WAV): BWF WAV files with large bext chunks cause lofty to emit
    // "abnormally large data" errors even though the file is playable. A proper fix
    // should parse the RIFF fmt chunk directly (sample_rate, channels from fmt; duration
    // from data chunk size) and read the ID3 tag separately — similar to how
    // extract_dsd_metadata() handles DSF files. Do NOT use a catch-all path-only
    // fallback: that caused tracks with large APIC frames in their ID3 tags to be
    // routed to wrong albums based on folder names rather than their actual ID3 metadata.
    let tagged_file = tagged_file_result.map_err(|e| {
        tracing::error!(
            file_path = %path.display(),
            error = %e,
            "[Metadata] lofty failed to read file; skipping (see BWF WAV TODO for proper fix)"
        );
        ImportError::Metadata(format!("lofty failed to read '{}': {}", path.display(), e))
    })?;
    let probe_duration = probe_start.elapsed();

    // Find the best tag - prefer one with artist metadata
    // Files may have multiple tag types (ID3v1, ID3v2, APEv2, Vorbis) with data in different places
    let tag_count = tagged_file.tags().len();
    tracing::debug!(tag_count, "[Metadata] Finding best tag from available tags");
    let tag = find_best_tag(&tagged_file);

    // Extract audio properties
    let properties = tagged_file.properties();
    let duration_seconds = properties.duration().as_secs_f64();
    let bitrate = properties.audio_bitrate();
    let sample_rate = properties.sample_rate();
    let channels = properties.channels();

    // Extract tag metadata
    let (title, raw_artists_native, album, album_artist, track_number, disc_number, year, genres) =
        if let Some(tag) = tag {
            let title = tag.title().map(|s| s.to_string());

            // Native multi-value artist extraction.
            // Vorbis Comments (FLAC/OGG) natively support multiple ARTIST= entries.
            // ID3v2.4 may store multiple values in a single text frame separated by \0.
            // Using get_items avoids relying on lofty's concatenated artist() helper.
            let raw_artists_native: Vec<String> = tag
                .items()
                .filter(|item| item.key() == &lofty::ItemKey::TrackArtist)
                .filter_map(|item| item.value().text().map(|s| s.to_string()))
                .flat_map(|s| {
                    // Split null-separated ID3v2.4 values into separate entries
                    s.split('\0')
                        .map(|p| p.trim().to_string())
                        .collect::<Vec<_>>()
                })
                .filter(|s| !s.is_empty())
                .collect();
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
                raw_artists_native,
                album,
                album_artist,
                track_number,
                disc_number,
                year,
                genres,
            )
        } else {
            (None, Vec::new(), None, None, None, None, None, Vec::new())
        };

    // Fallback: Use filename as title if no title in tags
    let title: Option<String> =
        title.or_else(|| path.file_stem().map(|s| s.to_string_lossy().into_owned()));

    // Attempt to fix mojibake: ID3v2 Latin-1-declared frames that actually contain
    // UTF-8 bytes are read by lofty as Windows-1252, producing garbled characters.
    // If all code points are ≤ U+00FF and the bytes form valid UTF-8, re-interpret.
    let fix_mojibake = |s: String| -> String {
        if s.chars().all(|c| (c as u32) <= 0xFF) && s.chars().any(|c| (c as u32) > 0x7F) {
            let bytes: Vec<u8> = s.chars().map(|c| c as u8).collect();
            if let Ok(fixed) = std::str::from_utf8(&bytes) {
                if fixed != s {
                    tracing::debug!(original = %s, fixed = %fixed, "[metadata] Mojibake corrected");
                    return fixed.to_string();
                }
            }
        }
        s
    };

    let title = title.map(&fix_mojibake);
    let native_artists: Vec<String> = raw_artists_native.into_iter().map(&fix_mojibake).collect();
    let album = album.map(&fix_mojibake);
    let album_artist = album_artist.map(&fix_mojibake);
    let genres: Vec<String> = genres.into_iter().map(&fix_mojibake).collect();

    // Fallback: Parse parent folder name for artist/album only when tags are missing.
    // Never override tag data with folder data — doing so caused duplicate albums on
    // force-reimport when the folder-derived name differed from the tag-derived name.
    let folder_meta = if native_artists.is_empty() || album.is_none() {
        let parent = path.parent();
        let folder_name = parent
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned());

        if let Some(name) = folder_name {
            let mut parsed = parse_folder_name(&name);

            // If folder parsing didn't extract artist/album (e.g., just "Album Name"),
            // try the grandparent folder for artist name
            if parsed.artist.is_none() && parsed.album.is_none() {
                // Treat immediate parent as album name
                parsed.album = Some(name.clone());

                // Try grandparent for artist
                if let Some(grandparent) = parent.and_then(|p| p.parent()) {
                    if let Some(grandparent_name) = grandparent.file_name() {
                        let gp_str = grandparent_name.to_string_lossy();
                        parsed.artist = Some(gp_str.to_string());
                    }
                }
            }

            tracing::debug!(
                file = ?path.file_name(),
                folder = %name,
                artist = ?parsed.artist,
                album = ?parsed.album,
                "[metadata] Folder fallback for missing tags"
            );
            Some(parsed)
        } else {
            None
        }
    } else {
        tracing::debug!(
            file = ?path.file_name(),
            artists = ?native_artists,
            album = ?album,
            "[metadata] Tags found"
        );
        None
    };

    // Tag data takes priority; folder metadata only fills in missing values.
    let artists: Vec<String> = if !native_artists.is_empty() {
        native_artists
            .into_iter()
            .flat_map(|a| split_artists(&a))
            .collect()
    } else if let Some(folder_artist) = folder_meta.as_ref().and_then(|m| m.artist.clone()) {
        split_artists(&folder_artist)
    } else {
        Vec::new()
    };
    let album = album.or_else(|| folder_meta.as_ref().and_then(|m| m.album.clone()));
    let year = year.or_else(|| folder_meta.as_ref().and_then(|m| m.year));

    // Get file format from extension
    let file_format = path
        .extension()
        .map(|ext| ext.to_string_lossy().to_lowercase())
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
            let mime = pic
                .mime_type()
                .map(|m| m.as_str().to_string())
                .unwrap_or_else(|| "image/jpeg".to_string());
            (pic.data().to_vec(), mime)
        })
    });

    let total_duration = start.elapsed();
    let _has_metadata = !artists.is_empty() || album.is_some();

    tracing::info!(
        file_path = %path.display(),
        has_title = title.is_some(),
        artist_count = artists.len(),
        has_album = album.is_some(),
        genre_count = genres.len(),
        tag_count,
        probe_ms = probe_duration.as_millis(),
        total_ms = total_duration.as_millis(),
        "[Metadata] Extraction completed"
    );

    if total_duration.as_millis() > 500 {
        tracing::warn!(
            file_path = %path.display(),
            duration_ms = total_duration.as_millis(),
            "[Metadata] Slow metadata extraction detected"
        );
    }

    Ok(ExtractedMetadata {
        title,
        artists,
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

/// Extract metadata from WAV/BWF files by walking RIFF chunks directly.
///
/// This handles BWF (Broadcast Wave Format) files that contain large `bext` chunks before
/// the `fmt` chunk, which cause lofty to emit "abnormally large data" errors even though
/// the file is valid and playable. Standard WAV files are also handled correctly here.
///
/// Chunk layout walked:
/// - `fmt ` → sample_rate, channels, bits_per_sample (must be ≥ 16 bytes)
/// - `data` → chunk size used to compute duration
/// - `id3 ` / `ID3 ` → raw bytes parsed as ID3v2 frames (same logic as DSD extractor)
/// - anything else (`bext`, `JUNK`, `LIST`, …) → skipped
///
/// Returns `Err` if the RIFF header is malformed, or if the `fmt` or `data` chunk is missing.
/// The caller falls back to lofty on any error.
fn extract_wav_metadata(path: &Path) -> Result<ExtractedMetadata> {
    use std::io::{Read, Seek, SeekFrom};

    tracing::debug!(file_path = %path.display(), "[Metadata] WAV/BWF custom parser active");

    let file_format = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_else(|| "wav".to_string());

    let mut file = std::fs::File::open(path)
        .map_err(|e| ImportError::Metadata(format!("Failed to open WAV file: {}", e)))?;

    // ── RIFF header (12 bytes): "RIFF" + file_size u32 LE + "WAVE" ──────────
    let mut riff_header = [0u8; 12];
    file.read_exact(&mut riff_header)
        .map_err(|e| ImportError::Metadata(format!("WAV: failed to read RIFF header: {}", e)))?;
    if &riff_header[0..4] != b"RIFF" {
        return Err(ImportError::Metadata("WAV: not a RIFF file".to_string()));
    }
    if &riff_header[8..12] != b"WAVE" {
        return Err(ImportError::Metadata(
            "WAV: RIFF type is not WAVE".to_string(),
        ));
    }

    // ── Walk chunks ───────────────────────────────────────────────────────────
    let mut sample_rate: Option<u32> = None;
    let mut channels: Option<u8> = None;
    let mut bits_per_sample: Option<u16> = None;
    let mut data_bytes: Option<u64> = None;

    let mut title: Option<String> = None;
    let mut artists: Vec<String> = Vec::new();
    let mut album: Option<String> = None;
    let mut album_artist: Option<String> = None;
    let mut track_number: Option<u32> = None;
    let mut disc_number: Option<u32> = None;
    let mut year: Option<i32> = None;
    let mut genres: Vec<String> = Vec::new();
    let mut album_art: Option<(Vec<u8>, String)> = None;

    loop {
        let mut chunk_header = [0u8; 8];
        match file.read_exact(&mut chunk_header) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => {
                return Err(ImportError::Metadata(format!(
                    "WAV: error reading chunk header: {}",
                    e
                )))
            }
        }

        let chunk_id = &chunk_header[0..4];
        let chunk_size = u32::from_le_bytes(chunk_header[4..8].try_into().unwrap_or([0; 4])) as u64;
        // RIFF chunks are word-aligned: odd-sized chunks have a 1-byte pad
        let padded_size = chunk_size + (chunk_size & 1);

        if chunk_id == b"fmt " {
            if chunk_size < 16 {
                return Err(ImportError::Metadata(format!(
                    "WAV: fmt chunk too small ({} bytes, need ≥ 16)",
                    chunk_size
                )));
            }
            let read_size = chunk_size.min(40) as usize;
            let mut fmt_data = vec![0u8; read_size];
            file.read_exact(&mut fmt_data).map_err(|e| {
                ImportError::Metadata(format!("WAV: failed to read fmt chunk: {}", e))
            })?;
            // fmt chunk layout (PCM):
            //  [0..2]  AudioFormat (u16)
            //  [2..4]  NumChannels (u16)
            //  [4..8]  SampleRate  (u32)
            //  [8..12] ByteRate    (u32)
            // [12..14] BlockAlign  (u16)
            // [14..16] BitsPerSample (u16)
            let ch = u16::from_le_bytes(fmt_data[2..4].try_into().unwrap_or([0; 2]));
            let sr = u32::from_le_bytes(fmt_data[4..8].try_into().unwrap_or([0; 4]));
            let bps = u16::from_le_bytes(fmt_data[14..16].try_into().unwrap_or([0; 2]));
            channels = Some(ch as u8);
            sample_rate = Some(sr);
            bits_per_sample = Some(bps);
            // Seek past any remaining fmt bytes (e.g. extensible format extra fields)
            let remaining = padded_size - read_size as u64;
            if remaining > 0 {
                file.seek(SeekFrom::Current(remaining as i64))
                    .map_err(|e| {
                        ImportError::Metadata(format!("WAV: seek past fmt remainder failed: {}", e))
                    })?;
            }
        } else if chunk_id == b"data" {
            data_bytes = Some(chunk_size);
            // Seek past audio data — we only need the size
            file.seek(SeekFrom::Current(padded_size as i64))
                .map_err(|e| {
                    ImportError::Metadata(format!("WAV: seek past data chunk failed: {}", e))
                })?;
        } else if chunk_id == b"id3 " || chunk_id == b"ID3 " {
            // Read the raw ID3v2 bytes and parse frames manually
            let id3_raw_size = chunk_size as usize;
            if id3_raw_size < 10 {
                // Too small to be a valid ID3v2 tag — skip
                file.seek(SeekFrom::Current(padded_size as i64))
                    .map_err(|e| {
                        ImportError::Metadata(format!("WAV: seek past id3 chunk failed: {}", e))
                    })?;
                continue;
            }

            // Read just the 10-byte ID3v2 header to get the payload size
            let mut id3_header = [0u8; 10];
            file.read_exact(&mut id3_header).map_err(|e| {
                ImportError::Metadata(format!("WAV: failed to read id3 header: {}", e))
            })?;

            if &id3_header[0..3] != b"ID3" {
                // Not an ID3v2 header — skip the rest of this chunk
                let remaining = padded_size - 10;
                file.seek(SeekFrom::Current(remaining as i64))
                    .map_err(|e| {
                        ImportError::Metadata(format!(
                            "WAV: seek past non-ID3 id3 chunk failed: {}",
                            e
                        ))
                    })?;
                continue;
            }

            // ID3v2 syncsafe size
            let tag_size = ((id3_header[6] as u32 & 0x7f) << 21)
                | ((id3_header[7] as u32 & 0x7f) << 14)
                | ((id3_header[8] as u32 & 0x7f) << 7)
                | (id3_header[9] as u32 & 0x7f);

            let payload_to_read = (tag_size as u64).min(chunk_size - 10);
            let mut tag_data = vec![0u8; payload_to_read as usize];
            file.read_exact(&mut tag_data).map_err(|e| {
                ImportError::Metadata(format!("WAV: failed to read id3 payload: {}", e))
            })?;

            // Seek past any chunk padding
            let consumed = 10 + payload_to_read;
            let skip = padded_size.saturating_sub(consumed);
            if skip > 0 {
                file.seek(SeekFrom::Current(skip as i64)).map_err(|e| {
                    ImportError::Metadata(format!("WAV: seek past id3 chunk tail failed: {}", e))
                })?;
            }

            // Parse ID3v2.3/2.4 frames — identical logic to extract_dsd_metadata
            let mut pos = 0usize;
            while pos + 10 <= tag_data.len() {
                let fid = &tag_data[pos..pos + 4];
                if fid[0] == 0 {
                    break; // padding
                }
                let fsize =
                    u32::from_be_bytes(tag_data[pos + 4..pos + 8].try_into().unwrap_or([0; 4]))
                        as usize;
                if fsize == 0 || pos + 10 + fsize > tag_data.len() {
                    break;
                }
                let fdata = &tag_data[pos + 10..pos + 10 + fsize];

                let text_val = || decode_id3_text(fdata);

                match fid {
                    b"TIT2" => title = text_val(),
                    b"TPE1" => {
                        if let Some(a) = text_val() {
                            artists = split_artists(&a);
                        }
                    }
                    b"TPE2" => album_artist = text_val(),
                    b"TALB" => album = text_val(),
                    b"TRCK" => {
                        if let Some(s) = text_val() {
                            track_number = s.split('/').next().and_then(|n| n.parse::<u32>().ok());
                        }
                    }
                    b"TPOS" => {
                        if let Some(s) = text_val() {
                            disc_number = s.split('/').next().and_then(|n| n.parse::<u32>().ok());
                        }
                    }
                    b"TYER" | b"TDRC" => {
                        if year.is_none() {
                            if let Some(s) = text_val() {
                                year = s.get(..4).and_then(|y| y.parse::<i32>().ok());
                            }
                        }
                    }
                    b"TCON" => {
                        if let Some(g) = text_val() {
                            genres = g
                                .split(&[';', ',', '/'][..])
                                .map(|s| s.trim().to_string())
                                .filter(|s| !s.is_empty())
                                .collect();
                        }
                    }
                    b"APIC" => {
                        if fdata.len() > 4 {
                            let after_enc = &fdata[1..];
                            if let Some(mime_end) = after_enc.iter().position(|&b| b == 0) {
                                let mime =
                                    String::from_utf8_lossy(&after_enc[..mime_end]).to_string();
                                let after_mime = &after_enc[mime_end + 1..];
                                if after_mime.len() > 1 {
                                    let after_type = &after_mime[1..];
                                    if let Some(desc_end) = after_type.iter().position(|&b| b == 0)
                                    {
                                        let pic_data = &after_type[desc_end + 1..];
                                        if !pic_data.is_empty() {
                                            album_art = Some((pic_data.to_vec(), mime));
                                        }
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }

                pos += 10 + fsize;
            }
        } else {
            // Unknown / unneeded chunk (bext, JUNK, LIST, etc.) — skip
            file.seek(SeekFrom::Current(padded_size as i64))
                .map_err(|e| {
                    ImportError::Metadata(format!(
                        "WAV: seek past chunk {:?} failed: {}",
                        std::str::from_utf8(chunk_id).unwrap_or("????"),
                        e
                    ))
                })?;
        }
    }

    // ── Validate required chunks ──────────────────────────────────────────────
    let sr =
        sample_rate.ok_or_else(|| ImportError::Metadata("WAV: fmt chunk missing".to_string()))?;
    let ch = channels
        .ok_or_else(|| ImportError::Metadata("WAV: fmt chunk missing (channels)".to_string()))?;
    let bps = bits_per_sample.ok_or_else(|| {
        ImportError::Metadata("WAV: fmt chunk missing (bits_per_sample)".to_string())
    })?;
    let data_sz =
        data_bytes.ok_or_else(|| ImportError::Metadata("WAV: data chunk missing".to_string()))?;

    // duration = data_bytes / byte_rate;  byte_rate = sample_rate × channels × (bits/8)
    let bytes_per_second = sr as f64 * ch as f64 * bps as f64 / 8.0;
    let duration_seconds = if bytes_per_second > 0.0 {
        Some(data_sz as f64 / bytes_per_second)
    } else {
        None
    };

    // Folder/filename fallback for missing metadata (mirrors extract_dsd_metadata)
    if artists.is_empty() || album.is_none() {
        let parent = path.parent();
        let folder_name = parent
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned());

        if let Some(name) = folder_name {
            let parsed = parse_folder_name(&name);
            if artists.is_empty() {
                if let Some(ref a) = parsed.artist {
                    artists = split_artists(a);
                } else if let Some(grandparent) = parent.and_then(|p| p.parent()) {
                    if let Some(gp_name) = grandparent.file_name() {
                        artists = vec![gp_name.to_string_lossy().to_string()];
                    }
                }
            }
            if album.is_none() {
                album = parsed.album.or(Some(name));
            }
            if year.is_none() {
                year = parsed.year;
            }
        }
    }

    Ok(ExtractedMetadata {
        title,
        artists,
        album,
        album_artist,
        track_number,
        disc_number,
        year,
        genres,
        duration_seconds,
        bitrate: None,
        sample_rate: Some(sr),
        channels: Some(ch),
        file_format,
        musicbrainz_recording_id: None,
        composer: None,
        album_art,
    })
}

/// Extract metadata from DSD files (DSF/DFF) that lofty 0.18 doesn't support natively.
///
/// DSF files have: DSD chunk (28 bytes) → fmt chunk → data chunk → optional ID3v2 tag.
/// We parse the binary header for audio properties and read the ID3v2 tag for metadata.
/// For DFF or when parsing fails, folder/filename metadata is used as fallback.
fn extract_dsd_metadata(path: &Path) -> Result<ExtractedMetadata> {
    use std::io::{Read, Seek, SeekFrom};

    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let file_format = ext.clone();

    tracing::info!(file_path = %path.display(), "[Metadata] DSD file detected, using DSD extractor");

    let file_size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);

    let mut title = None;
    let mut artists: Vec<String> = Vec::new();
    let mut album = None;
    let mut album_artist = None;
    let mut track_number = None;
    let mut disc_number = None;
    let mut year = None;
    let mut genres: Vec<String> = Vec::new();
    let mut duration_seconds = None;
    let mut sample_rate = None;
    let mut channels = None;
    let mut album_art = None;

    if ext == "dsf" {
        if let Ok(mut file) = std::fs::File::open(path) {
            let mut header = [0u8; 28];
            if file.read_exact(&mut header).is_ok() && &header[0..4] == b"DSD " {
                let id3_offset = u64::from_le_bytes(header[20..28].try_into().unwrap_or([0; 8]));

                // Read fmt chunk for audio properties
                let mut fmt_id = [0u8; 4];
                if file.read_exact(&mut fmt_id).is_ok() && &fmt_id == b"fmt " {
                    let mut fmt_data = [0u8; 48];
                    if file.read_exact(&mut fmt_data).is_ok() {
                        // DSF fmt chunk layout (offsets from after "fmt " marker):
                        // [0..8]   chunk size (u64)
                        // [8..12]  format version (u32)
                        // [12..16] format ID (u32)
                        // [16..20] channel type (u32)
                        // [20..24] channel count (u32)
                        // [24..28] sample rate (u32) — e.g. 2822400 for DSD64
                        // [28..32] bits per sample (u32)
                        // [32..40] sample count per channel (u64)
                        let ch = u32::from_le_bytes(fmt_data[20..24].try_into().unwrap_or([0; 4]));
                        let sr = u32::from_le_bytes(fmt_data[24..28].try_into().unwrap_or([0; 4]));
                        let samples =
                            u64::from_le_bytes(fmt_data[32..40].try_into().unwrap_or([0; 8]));
                        channels = Some(ch as u8);
                        sample_rate = Some(sr);
                        if sr > 0 {
                            duration_seconds = Some(samples as f64 / sr as f64);
                        }
                    }
                }

                // Read ID3v2 tag at the offset (DSF embeds ID3v2 after the data chunk).
                // Parse ID3v2 frames manually since lofty 0.18 can't read standalone ID3v2 buffers.
                if id3_offset > 0
                    && id3_offset < file_size
                    && file.seek(SeekFrom::Start(id3_offset)).is_ok()
                {
                    let mut id3_header = [0u8; 10];
                    if file.read_exact(&mut id3_header).is_ok() && &id3_header[0..3] == b"ID3" {
                        let tag_size = ((id3_header[6] as u32 & 0x7f) << 21)
                            | ((id3_header[7] as u32 & 0x7f) << 14)
                            | ((id3_header[8] as u32 & 0x7f) << 7)
                            | (id3_header[9] as u32 & 0x7f);

                        let mut tag_data = vec![0u8; tag_size as usize];
                        if file.read_exact(&mut tag_data).is_ok() {
                            // Parse ID3v2.3/2.4 frames (4-byte ID, 4-byte size, 2-byte flags)
                            let mut pos = 0;
                            while pos + 10 <= tag_data.len() {
                                let fid = &tag_data[pos..pos + 4];
                                if fid[0] == 0 {
                                    break;
                                } // padding
                                let fsize = u32::from_be_bytes(
                                    tag_data[pos + 4..pos + 8].try_into().unwrap_or([0; 4]),
                                ) as usize;
                                if fsize == 0 || pos + 10 + fsize > tag_data.len() {
                                    break;
                                }
                                let fdata = &tag_data[pos + 10..pos + 10 + fsize];

                                // Text frames: first byte is encoding, rest is text.
                                let text_val = || decode_id3_text(fdata);

                                match fid {
                                    b"TIT2" => title = text_val(),
                                    b"TPE1" => {
                                        if let Some(a) = text_val() {
                                            artists = split_artists(&a);
                                        }
                                    }
                                    b"TPE2" => album_artist = text_val(),
                                    b"TALB" => album = text_val(),
                                    b"TRCK" => {
                                        if let Some(s) = text_val() {
                                            track_number = s
                                                .split('/')
                                                .next()
                                                .and_then(|n| n.parse::<u32>().ok());
                                        }
                                    }
                                    b"TPOS" => {
                                        if let Some(s) = text_val() {
                                            disc_number = s
                                                .split('/')
                                                .next()
                                                .and_then(|n| n.parse::<u32>().ok());
                                        }
                                    }
                                    b"TYER" | b"TDRC" => {
                                        if year.is_none() {
                                            if let Some(s) = text_val() {
                                                year =
                                                    s.get(..4).and_then(|y| y.parse::<i32>().ok());
                                            }
                                        }
                                    }
                                    b"TCON" => {
                                        if let Some(g) = text_val() {
                                            genres = g
                                                .split(&[';', ',', '/'][..])
                                                .map(|s| s.trim().to_string())
                                                .filter(|s| !s.is_empty())
                                                .collect();
                                        }
                                    }
                                    b"APIC" => {
                                        // Embedded picture: encoding(1) + mime(null-term) + pic_type(1) + desc(null-term) + data
                                        if fdata.len() > 4 {
                                            let after_enc = &fdata[1..];
                                            if let Some(mime_end) =
                                                after_enc.iter().position(|&b| b == 0)
                                            {
                                                let mime =
                                                    String::from_utf8_lossy(&after_enc[..mime_end])
                                                        .to_string();
                                                let after_mime = &after_enc[mime_end + 1..]; // skip null
                                                if after_mime.len() > 1 {
                                                    // skip pic_type byte + description (null-terminated)
                                                    let after_type = &after_mime[1..];
                                                    if let Some(desc_end) =
                                                        after_type.iter().position(|&b| b == 0)
                                                    {
                                                        let pic_data = &after_type[desc_end + 1..];
                                                        if !pic_data.is_empty() {
                                                            album_art =
                                                                Some((pic_data.to_vec(), mime));
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    _ => {} // skip unknown frames
                                }
                                pos += 10 + fsize;
                            }
                        }
                    }
                    if title.is_some() || !artists.is_empty() {
                        tracing::info!(
                            file_path = %path.display(),
                            title = ?title,
                            artist_count = artists.len(),
                            "[Metadata] DSD ID3v2 tag parsed successfully"
                        );
                    }
                }
            }
        }
    }

    // Folder/filename fallback for missing metadata
    if artists.is_empty() || album.is_none() {
        let parent = path.parent();
        let folder_name = parent
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned());

        if let Some(name) = folder_name {
            let parsed = parse_folder_name(&name);
            if artists.is_empty() {
                if let Some(ref a) = parsed.artist {
                    artists = split_artists(a);
                } else if let Some(grandparent) = parent.and_then(|p| p.parent()) {
                    if let Some(gp_name) = grandparent.file_name() {
                        artists = vec![gp_name.to_string_lossy().to_string()];
                    }
                }
            }
            if album.is_none() {
                album = parsed.album.or(Some(name));
            }
            if year.is_none() {
                year = parsed.year;
            }
        }
    }

    if title.is_none() {
        title = path.file_stem().map(|s| s.to_string_lossy().to_string());
    }

    Ok(ExtractedMetadata {
        title,
        artists,
        album,
        album_artist,
        track_number,
        disc_number,
        year,
        genres,
        duration_seconds,
        bitrate: None,
        sample_rate,
        channels,
        file_format,
        musicbrainz_recording_id: None,
        composer: None,
        album_art,
    })
}

/// Calculate SHA-256 hash of a file for duplicate detection
pub fn calculate_file_hash(path: &Path) -> Result<String> {
    use sha2::{Digest, Sha256};
    use std::fs::File;
    use std::io::Read;

    tracing::debug!(file_path = %path.display(), "[Metadata] Calculating file hash");
    let start = std::time::Instant::now();

    let mut file = File::open(path).map_err(|e| {
        tracing::error!(file_path = %path.display(), error = %e, "[Metadata] Failed to open file for hashing");
        e
    })?;

    let file_size = file.metadata().ok().map(|m| m.len()).unwrap_or(0);

    // Warn about large files (>100MB)
    if file_size > 100_000_000 {
        tracing::warn!(
            file_path = %path.display(),
            size_mb = file_size as f32 / 1_000_000.0,
            "[Metadata] Large file detected - hashing may take time"
        );
    }

    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];
    let mut total_bytes = 0u64;

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
        total_bytes += bytes_read as u64;
    }

    let hash = hasher.finalize();
    let hash_string = hex::encode(hash);

    let duration = start.elapsed();

    tracing::debug!(
        file_path = %path.display(),
        size_bytes = total_bytes,
        duration_ms = duration.as_millis(),
        hash = &hash_string[..16], // Log first 16 chars of hash
        "[Metadata] Hash calculation completed"
    );

    if duration.as_millis() > 1000 {
        tracing::warn!(
            file_path = %path.display(),
            size_mb = total_bytes as f32 / 1_000_000.0,
            duration_ms = duration.as_millis(),
            "[Metadata] Slow hash calculation detected"
        );
    }

    Ok(hash_string)
}

/// Calculate a quick hash of the first 64KB of a file for fast dedup.
/// This is much faster than hashing the entire file and sufficient
/// for initial duplicate detection.
pub fn calculate_quick_hash(path: &Path) -> Result<String> {
    use sha2::{Digest, Sha256};
    use std::io::Read;

    let start = std::time::Instant::now();
    let mut file = std::fs::File::open(path).map_err(|e| {
        tracing::error!(
            file_path = %path.display(),
            error = %e,
            "[Metadata] Failed to open file for quick hashing"
        );
        e
    })?;
    let mut buffer = [0u8; 65536]; // 64KB
    let bytes_read = file.read(&mut buffer).map_err(|e| {
        tracing::error!(
            file_path = %path.display(),
            error = %e,
            "[Metadata] Failed to read file for quick hashing"
        );
        e
    })?;
    let hash = Sha256::digest(&buffer[..bytes_read]);
    let result = hex::encode(hash);

    let elapsed = start.elapsed();
    if elapsed.as_millis() > 100 {
        tracing::warn!(
            file_path = %path.display(),
            duration_ms = elapsed.as_millis(),
            "[Metadata] Slow quick hash calculation"
        );
    } else {
        tracing::debug!(
            file_path = %path.display(),
            duration_ms = elapsed.as_millis(),
            hash = &result[..16],
            "[Metadata] Quick hash calculation completed"
        );
    }

    Ok(result)
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

        let artists: Vec<String> = meta.artist.map(|a| split_artists(&a)).unwrap_or_default();

        Self {
            title: meta.title,
            artists,
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
            artists: Vec::new(),
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
            artists: vec!["Artist".to_string()],
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
        assert_eq!(
            result.album,
            Some("Part 1 - The Beginning - Remastered".to_string())
        );
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

    #[test]
    fn test_split_artists_single() {
        assert_eq!(split_artists("Skinshape"), vec!["Skinshape"]);
    }

    #[test]
    fn test_split_artists_feat_period() {
        assert_eq!(
            split_artists("Skinshape feat. Wu-Lu"),
            vec!["Skinshape", "Wu-Lu"]
        );
    }

    #[test]
    fn test_split_artists_comma_not_split() {
        // Commas are part of band names (e.g. "Earth, Wind & Fire") — do NOT split
        assert_eq!(
            split_artists("Artist A, Artist B, Artist C"),
            vec!["Artist A, Artist B, Artist C"]
        );
    }

    #[test]
    fn test_split_artists_ampersand_not_split() {
        // Ampersands in band names must NOT be split
        assert_eq!(
            split_artists("Bonobo & Erykah Badu"),
            vec!["Bonobo & Erykah Badu"]
        );
    }

    #[test]
    fn test_split_artists_earth_wind_fire() {
        // Classic example: "Earth, Wind & Fire" must remain a single artist
        assert_eq!(
            split_artists("Earth, Wind & Fire"),
            vec!["Earth, Wind & Fire"]
        );
    }

    #[test]
    fn test_split_artists_mixed() {
        // feat. still splits; & after feat. stays with the featured artist name
        assert_eq!(
            split_artists("Madlib feat. Guilty Simpson & MED"),
            vec!["Madlib", "Guilty Simpson & MED"]
        );
    }

    #[test]
    fn test_split_artists_hyphen_not_split() {
        // Hyphens within names must NOT be split
        assert_eq!(split_artists("Wu-Tang Clan"), vec!["Wu-Tang Clan"]);
    }

    #[test]
    fn test_split_artists_feat_capitalized() {
        assert_eq!(
            split_artists("Skinshape Feat. Wu-Lu"),
            vec!["Skinshape", "Wu-Lu"]
        );
    }
}
