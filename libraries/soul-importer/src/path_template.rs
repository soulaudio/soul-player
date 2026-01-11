//! Path template parser and resolver for managed library organization
//!
//! Supports templates like `{AlbumArtist}/{Year} - {Album}/{TrackNo} - {Title}`
//! with automatic fallbacks and sanitization for filesystem safety.
//!
//! # Available Placeholders
//!
//! | Placeholder | Description | Fallback |
//! |-------------|-------------|----------|
//! | `{Artist}` | Track artist | "Unknown Artist" |
//! | `{AlbumArtist}` | Album artist | Falls back to `{Artist}` |
//! | `{Album}` | Album title | "Unknown Album" |
//! | `{Title}` | Track title | Filename without extension |
//! | `{TrackNo}` | Track number (zero-padded) | "00" |
//! | `{DiscNo}` | Disc number | "1" |
//! | `{Year}` | Release year | "0000" |
//! | `{Genre}` | Primary genre | "Unknown" |
//! | `{Composer}` | Composer | Empty string |

use crate::metadata::ExtractedMetadata;
use std::path::{Path, PathBuf};

/// Default path template (Audiophile style)
pub const DEFAULT_TEMPLATE: &str = "{AlbumArtist}/{Year} - {Album}/{TrackNo} - {Title}";

/// Simple path template (no year)
pub const SIMPLE_TEMPLATE: &str = "{AlbumArtist}/{Album}/{TrackNo} - {Title}";

/// Genre-first path template
pub const GENRE_TEMPLATE: &str = "{Genre}/{AlbumArtist}/{Album}/{TrackNo} - {Title}";

/// Path template parser and resolver
#[derive(Debug, Clone)]
pub struct PathTemplate {
    /// The template string with placeholders
    template: String,
    /// Whether to add disc subfolder for multi-disc albums
    add_disc_folder: bool,
    /// Whether to include track artist for compilations
    compilation_artist_in_filename: bool,
}

impl Default for PathTemplate {
    fn default() -> Self {
        Self {
            template: DEFAULT_TEMPLATE.to_string(),
            add_disc_folder: true,
            compilation_artist_in_filename: true,
        }
    }
}

impl PathTemplate {
    /// Create a new path template
    pub fn new(template: impl Into<String>) -> Self {
        Self {
            template: template.into(),
            add_disc_folder: true,
            compilation_artist_in_filename: true,
        }
    }

    /// Set whether to add disc subfolder for multi-disc albums
    pub fn with_disc_folder(mut self, add_disc_folder: bool) -> Self {
        self.add_disc_folder = add_disc_folder;
        self
    }

    /// Set whether to include track artist in filename for compilations
    pub fn with_compilation_artist(mut self, include: bool) -> Self {
        self.compilation_artist_in_filename = include;
        self
    }

    /// Get the template string
    pub fn template(&self) -> &str {
        &self.template
    }

    /// Resolve the template to a path using the provided metadata
    ///
    /// # Arguments
    ///
    /// * `metadata` - The extracted metadata from the audio file
    /// * `source_path` - Original file path (used for filename fallback and extension)
    ///
    /// # Returns
    ///
    /// A relative path from the library root
    pub fn resolve(&self, metadata: &ExtractedMetadata, source_path: &Path) -> PathBuf {
        let extension = source_path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("flac");

        // Detect if this is a multi-disc album
        let is_multi_disc =
            metadata.disc_number.map(|_| true).unwrap_or(false) && metadata.disc_number != Some(1);

        // Detect if this is a compilation/VA album
        let is_compilation = self.is_compilation(metadata);

        // Build the resolved path
        let mut resolved = self.template.clone();

        // Resolve all placeholders
        resolved =
            self.resolve_placeholder(&resolved, "AlbumArtist", || self.get_album_artist(metadata));
        resolved = self.resolve_placeholder(&resolved, "Artist", || self.get_artist(metadata));
        resolved = self.resolve_placeholder(&resolved, "Album", || self.get_album(metadata));
        resolved =
            self.resolve_placeholder(&resolved, "Title", || self.get_title(metadata, source_path));
        resolved =
            self.resolve_placeholder(&resolved, "TrackNo", || self.get_track_number(metadata));
        resolved = self.resolve_placeholder(&resolved, "DiscNo", || self.get_disc_number(metadata));
        resolved = self.resolve_placeholder(&resolved, "Year", || self.get_year(metadata));
        resolved = self.resolve_placeholder(&resolved, "Genre", || self.get_genre(metadata));
        resolved = self.resolve_placeholder(&resolved, "Composer", || self.get_composer(metadata));

        // Handle multi-disc albums
        if self.add_disc_folder && is_multi_disc {
            resolved = self.insert_disc_folder(&resolved, metadata);
        }

        // Handle compilation albums (add artist to filename)
        if self.compilation_artist_in_filename && is_compilation {
            resolved = self.add_compilation_artist(&resolved, metadata);
        }

        // Add file extension
        let resolved = format!("{}.{}", resolved, extension);

        // Convert to path and sanitize each component
        self.sanitize_path(&resolved)
    }

    /// Resolve a single placeholder in the template
    fn resolve_placeholder<F>(&self, template: &str, name: &str, value_fn: F) -> String
    where
        F: FnOnce() -> String,
    {
        let placeholder = format!("{{{}}}", name);
        if template.contains(&placeholder) {
            template.replace(&placeholder, &value_fn())
        } else {
            template.to_string()
        }
    }

    /// Get the album artist with fallback
    fn get_album_artist(&self, metadata: &ExtractedMetadata) -> String {
        metadata
            .album_artist
            .as_ref()
            .or(metadata.artist.as_ref())
            .map(|s| s.as_str())
            .unwrap_or("Unknown Artist")
            .to_string()
    }

    /// Get the artist with fallback
    fn get_artist(&self, metadata: &ExtractedMetadata) -> String {
        metadata
            .artist
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("Unknown Artist")
            .to_string()
    }

    /// Get the album with fallback
    fn get_album(&self, metadata: &ExtractedMetadata) -> String {
        metadata
            .album
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("Unknown Album")
            .to_string()
    }

    /// Get the title with fallback to filename
    fn get_title(&self, metadata: &ExtractedMetadata, source_path: &Path) -> String {
        metadata
            .title
            .as_ref()
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                source_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Unknown")
                    .to_string()
            })
    }

    /// Get the track number (zero-padded)
    fn get_track_number(&self, metadata: &ExtractedMetadata) -> String {
        metadata
            .track_number
            .map(|n| format!("{:02}", n))
            .unwrap_or_else(|| "00".to_string())
    }

    /// Get the disc number
    fn get_disc_number(&self, metadata: &ExtractedMetadata) -> String {
        metadata
            .disc_number
            .map(|n| n.to_string())
            .unwrap_or_else(|| "1".to_string())
    }

    /// Get the year with fallback
    fn get_year(&self, metadata: &ExtractedMetadata) -> String {
        metadata
            .year
            .map(|y| y.to_string())
            .unwrap_or_else(|| "0000".to_string())
    }

    /// Get the primary genre with fallback
    fn get_genre(&self, metadata: &ExtractedMetadata) -> String {
        metadata
            .genres
            .first()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Unknown".to_string())
    }

    /// Get the composer (empty string if not available)
    fn get_composer(&self, _metadata: &ExtractedMetadata) -> String {
        // TODO: Add composer field to ExtractedMetadata
        String::new()
    }

    /// Check if this is a compilation/VA album
    fn is_compilation(&self, metadata: &ExtractedMetadata) -> bool {
        let album_artist = metadata.album_artist.as_deref().unwrap_or("");
        let artist = metadata.artist.as_deref().unwrap_or("");

        // Check for common VA indicators
        let va_indicators = [
            "various artists",
            "various",
            "va",
            "compilation",
            "soundtrack",
            "ost",
        ];

        let lower_album_artist = album_artist.to_lowercase();
        for indicator in &va_indicators {
            if lower_album_artist.contains(indicator) {
                return true;
            }
        }

        // Check if album artist is empty but track artist exists
        if album_artist.is_empty() && !artist.is_empty() {
            return false; // Not a compilation, just missing album artist
        }

        // Check if album artist differs significantly from track artist
        if !album_artist.is_empty() && !artist.is_empty() && album_artist != artist {
            // Could be a featured artist or compilation
            // For now, only treat as compilation if album artist is a VA indicator
            return false;
        }

        false
    }

    /// Insert disc folder for multi-disc albums
    fn insert_disc_folder(&self, path: &str, metadata: &ExtractedMetadata) -> String {
        // Find the last "/" before the filename
        if let Some(last_sep) = path.rfind('/') {
            let (dir, file) = path.split_at(last_sep);
            let disc_num = metadata.disc_number.unwrap_or(1);
            format!("{}/Disc {}{}", dir, disc_num, file)
        } else {
            // No directory separator, just add disc folder before
            let disc_num = metadata.disc_number.unwrap_or(1);
            format!("Disc {}/{}", disc_num, path)
        }
    }

    /// Add artist to filename for compilation albums
    fn add_compilation_artist(&self, path: &str, metadata: &ExtractedMetadata) -> String {
        let artist = metadata.artist.as_deref().unwrap_or("Unknown Artist");

        // Find the track number in the filename and insert artist after it
        if let Some(last_sep) = path.rfind('/') {
            let (dir, file) = path.split_at(last_sep + 1);
            // Try to find "XX - " pattern (track number)
            if let Some(dash_pos) = file.find(" - ") {
                let (track_num, rest) = file.split_at(dash_pos + 3);
                format!("{}{}{} - {}", dir, track_num, artist, rest)
            } else {
                // No track number pattern, prepend artist
                format!("{}{} - {}", dir, artist, file)
            }
        } else {
            // No directory, just add artist to filename
            format!("{} - {}", artist, path)
        }
    }

    /// Sanitize a path string, making each component filesystem-safe
    fn sanitize_path(&self, path: &str) -> PathBuf {
        let components: Vec<&str> = path.split('/').collect();
        let mut result = PathBuf::new();

        for component in components {
            if !component.is_empty() {
                result.push(sanitize_path_component(component));
            }
        }

        result
    }
}

/// Sanitize a single path component for filesystem safety
///
/// Removes/replaces characters that are invalid on common filesystems
pub fn sanitize_path_component(s: &str) -> String {
    let sanitized: String = s
        .chars()
        .map(|c| match c {
            // Invalid on Windows: < > : " / \ | ? *
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            // Control characters
            c if c.is_control() => '_',
            // Keep everything else
            c => c,
        })
        .collect();

    // Trim whitespace and dots (Windows doesn't like trailing dots)
    let trimmed = sanitized.trim().trim_end_matches('.');

    // Handle reserved names on Windows
    let reserved = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];

    let upper = trimmed.to_uppercase();
    if reserved.contains(&upper.as_str()) {
        format!("_{}", trimmed)
    } else if trimmed.is_empty() {
        "_".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Preset templates for common use cases
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplatePreset {
    /// `{AlbumArtist}/{Year} - {Album}/{TrackNo} - {Title}` (default)
    Audiophile,
    /// `{AlbumArtist}/{Album}/{TrackNo} - {Title}`
    Simple,
    /// `{Genre}/{AlbumArtist}/{Album}/{TrackNo} - {Title}`
    GenreFirst,
}

impl TemplatePreset {
    /// Get the template string for this preset
    pub fn template(&self) -> &'static str {
        match self {
            TemplatePreset::Audiophile => DEFAULT_TEMPLATE,
            TemplatePreset::Simple => SIMPLE_TEMPLATE,
            TemplatePreset::GenreFirst => GENRE_TEMPLATE,
        }
    }

    /// Create a PathTemplate from this preset
    pub fn to_path_template(&self) -> PathTemplate {
        PathTemplate::new(self.template())
    }
}

impl From<TemplatePreset> for PathTemplate {
    fn from(preset: TemplatePreset) -> Self {
        preset.to_path_template()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_metadata() -> ExtractedMetadata {
        ExtractedMetadata {
            title: Some("Bohemian Rhapsody".to_string()),
            artist: Some("Queen".to_string()),
            album: Some("A Night at the Opera".to_string()),
            album_artist: Some("Queen".to_string()),
            track_number: Some(11),
            disc_number: Some(1),
            year: Some(1975),
            genres: vec!["Rock".to_string()],
            duration_seconds: Some(354.0),
            bitrate: Some(320),
            sample_rate: Some(44100),
            channels: Some(2),
            file_format: "flac".to_string(),
            musicbrainz_recording_id: None,
            composer: None,
            album_art: None,
        }
    }

    #[test]
    fn test_default_template() {
        let template = PathTemplate::default();
        let metadata = test_metadata();
        let source = Path::new("/path/to/song.flac");

        let path = template.resolve(&metadata, source);

        assert_eq!(
            path,
            PathBuf::from("Queen/1975 - A Night at the Opera/11 - Bohemian Rhapsody.flac")
        );
    }

    #[test]
    fn test_simple_template() {
        let template = PathTemplate::new(SIMPLE_TEMPLATE);
        let metadata = test_metadata();
        let source = Path::new("/path/to/song.flac");

        let path = template.resolve(&metadata, source);

        assert_eq!(
            path,
            PathBuf::from("Queen/A Night at the Opera/11 - Bohemian Rhapsody.flac")
        );
    }

    #[test]
    fn test_genre_template() {
        let template = PathTemplate::new(GENRE_TEMPLATE);
        let metadata = test_metadata();
        let source = Path::new("/path/to/song.flac");

        let path = template.resolve(&metadata, source);

        assert_eq!(
            path,
            PathBuf::from("Rock/Queen/A Night at the Opera/11 - Bohemian Rhapsody.flac")
        );
    }

    #[test]
    fn test_fallback_values() {
        let template = PathTemplate::default();
        let metadata = ExtractedMetadata {
            title: None,
            artist: None,
            album: None,
            album_artist: None,
            track_number: None,
            disc_number: None,
            year: None,
            genres: vec![],
            duration_seconds: None,
            bitrate: None,
            sample_rate: None,
            channels: None,
            file_format: "mp3".to_string(),
            musicbrainz_recording_id: None,
            composer: None,
            album_art: None,
        };
        let source = Path::new("/path/to/original_song.mp3");

        let path = template.resolve(&metadata, source);

        assert_eq!(
            path,
            PathBuf::from("Unknown Artist/0000 - Unknown Album/00 - original_song.mp3")
        );
    }

    #[test]
    fn test_album_artist_fallback() {
        let template = PathTemplate::default();
        let mut metadata = test_metadata();
        metadata.album_artist = None; // Remove album artist
        let source = Path::new("/path/to/song.flac");

        let path = template.resolve(&metadata, source);

        // Should fall back to track artist (use components for cross-platform)
        let components: Vec<_> = path.components().collect();
        assert_eq!(components[0].as_os_str().to_string_lossy(), "Queen");
    }

    #[test]
    fn test_multi_disc_album() {
        let template = PathTemplate::default();
        let mut metadata = test_metadata();
        metadata.disc_number = Some(2); // Disc 2
        let source = Path::new("/path/to/song.flac");

        let path = template.resolve(&metadata, source);

        assert!(path.to_string_lossy().contains("Disc 2"));
    }

    #[test]
    fn test_compilation_album() {
        let template = PathTemplate::default();
        let mut metadata = test_metadata();
        metadata.album_artist = Some("Various Artists".to_string());
        metadata.artist = Some("Freddie Mercury".to_string());
        let source = Path::new("/path/to/song.flac");

        let path = template.resolve(&metadata, source);

        // Should include track artist in filename
        assert!(path.to_string_lossy().contains("Freddie Mercury"));
    }

    #[test]
    fn test_sanitize_path_component() {
        assert_eq!(sanitize_path_component("Valid Name"), "Valid Name");
        assert_eq!(sanitize_path_component("Artist/Album"), "Artist_Album");
        assert_eq!(
            sanitize_path_component("Song: The Remix"),
            "Song_ The Remix"
        );
        assert_eq!(sanitize_path_component("A<B>C"), "A_B_C");
        assert_eq!(sanitize_path_component("  Trimmed  "), "Trimmed");
        assert_eq!(sanitize_path_component("trailing..."), "trailing");
        assert_eq!(sanitize_path_component("CON"), "_CON"); // Windows reserved
        assert_eq!(sanitize_path_component(""), "_");
    }

    #[test]
    fn test_track_number_padding() {
        let template = PathTemplate::default();
        let mut metadata = test_metadata();
        metadata.track_number = Some(5);
        let source = Path::new("/path/to/song.flac");

        let path = template.resolve(&metadata, source);

        // Check filename starts with "05 - " (cross-platform)
        let filename = path.file_name().unwrap().to_string_lossy();
        assert!(filename.starts_with("05 - "));
    }

    #[test]
    fn test_preset_conversion() {
        let template: PathTemplate = TemplatePreset::Audiophile.into();
        assert_eq!(template.template(), DEFAULT_TEMPLATE);

        let template: PathTemplate = TemplatePreset::Simple.into();
        assert_eq!(template.template(), SIMPLE_TEMPLATE);

        let template: PathTemplate = TemplatePreset::GenreFirst.into();
        assert_eq!(template.template(), GENRE_TEMPLATE);
    }

    #[test]
    fn test_disable_disc_folder() {
        let template = PathTemplate::default().with_disc_folder(false);
        let mut metadata = test_metadata();
        metadata.disc_number = Some(2);
        let source = Path::new("/path/to/song.flac");

        let path = template.resolve(&metadata, source);

        assert!(!path.to_string_lossy().contains("Disc"));
    }

    #[test]
    fn test_disable_compilation_artist() {
        let template = PathTemplate::default().with_compilation_artist(false);
        let mut metadata = test_metadata();
        metadata.album_artist = Some("Various Artists".to_string());
        metadata.artist = Some("Freddie Mercury".to_string());
        let source = Path::new("/path/to/song.flac");

        let path = template.resolve(&metadata, source);

        // Should NOT include track artist in filename
        assert!(!path.to_string_lossy().contains("Freddie Mercury - "));
    }
}
