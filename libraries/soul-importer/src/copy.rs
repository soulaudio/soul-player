//! File copying to managed library with organized naming

use crate::{metadata::ExtractedMetadata, ImportError, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Copy a file to the managed library with organized naming
///
/// # Arguments
///
/// * `source_path` - Original file path
/// * `library_path` - Base library directory (e.g., ~/Music/soul-player/library)
/// * `metadata` - Extracted metadata for naming
///
/// # Returns
///
/// New path in the library
pub fn copy_to_library(
    source_path: &Path,
    library_path: &Path,
    metadata: &ExtractedMetadata,
) -> Result<PathBuf> {
    // Ensure library directory exists
    if !library_path.exists() {
        fs::create_dir_all(library_path)?;
    }

    // Generate filename: "Artist - Track.ext"
    let filename = generate_filename(source_path, metadata)?;

    // Build destination path
    let mut dest_path = library_path.join(&filename);

    // Handle filename conflicts
    if dest_path.exists() {
        dest_path = resolve_filename_conflict(library_path, &filename)?;
    }

    // Copy file
    fs::copy(source_path, &dest_path)?;

    Ok(dest_path)
}

/// Generate filename from metadata: "Artist - Track.ext"
///
/// Falls back to original filename if metadata is missing
pub fn generate_filename(source_path: &Path, metadata: &ExtractedMetadata) -> Result<String> {
    let extension = source_path
        .extension()
        .and_then(|ext| ext.to_str())
        .ok_or_else(|| ImportError::InvalidPath("File has no extension".to_string()))?;

    // Get artist (prefer album_artist, fall back to artist)
    let artist = metadata
        .album_artist
        .as_ref()
        .or(metadata.artist.as_ref());

    // Get title
    let title = metadata.title.as_ref();

    let filename = match (artist, title) {
        (Some(artist), Some(title)) => {
            // Sanitize artist and title for filesystem
            let clean_artist = sanitize_filename_part(artist);
            let clean_title = sanitize_filename_part(title);
            format!("{} - {}.{}", clean_artist, clean_title, extension)
        }
        (None, Some(title)) => {
            // No artist - just use title
            let clean_title = sanitize_filename_part(title);
            format!("{}.{}", clean_title, extension)
        }
        _ => {
            // No metadata - use original filename
            source_path
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| ImportError::InvalidPath("Invalid source filename".to_string()))?
                .to_string()
        }
    };

    Ok(filename)
}

/// Sanitize a string for use in filenames
///
/// Removes/replaces characters that are invalid on common filesystems
pub fn sanitize_filename_part(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            // Invalid on Windows: < > : " / \ | ? *
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            // Control characters
            c if c.is_control() => '_',
            // Keep everything else
            c => c,
        })
        .collect::<String>()
        .trim()
        .to_string()
}

/// Resolve filename conflict by appending a counter
///
/// "song.mp3" -> "song-1.mp3" -> "song-2.mp3" etc.
fn resolve_filename_conflict(library_path: &Path, original_filename: &str) -> Result<PathBuf> {
    let path = Path::new(original_filename);
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| ImportError::InvalidPath("Invalid filename".to_string()))?;
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("");

    // Try appending -1, -2, -3, etc. until we find an available name
    for counter in 1..1000 {
        let new_filename = if extension.is_empty() {
            format!("{}-{}", stem, counter)
        } else {
            format!("{}-{}.{}", stem, counter, extension)
        };

        let new_path = library_path.join(&new_filename);
        if !new_path.exists() {
            return Ok(new_path);
        }
    }

    Err(ImportError::Unknown(
        "Could not resolve filename conflict after 1000 attempts".to_string(),
    ))
}

/// Move a file to the managed library (instead of copying)
///
/// This is faster than copying but removes the file from its original location
pub fn move_to_library(
    source_path: &Path,
    library_path: &Path,
    metadata: &ExtractedMetadata,
) -> Result<PathBuf> {
    // Ensure library directory exists
    if !library_path.exists() {
        fs::create_dir_all(library_path)?;
    }

    // Generate filename
    let filename = generate_filename(source_path, metadata)?;

    // Build destination path
    let mut dest_path = library_path.join(&filename);

    // Handle filename conflicts
    if dest_path.exists() {
        dest_path = resolve_filename_conflict(library_path, &filename)?;
    }

    // Move file (rename if same filesystem, copy+delete otherwise)
    fs::rename(source_path, &dest_path)?;

    Ok(dest_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_sanitize_filename_part() {
        assert_eq!(sanitize_filename_part("Valid Name"), "Valid Name");
        assert_eq!(sanitize_filename_part("Artist/Album"), "Artist_Album");
        assert_eq!(
            sanitize_filename_part("Song: The Remix"),
            "Song_ The Remix"
        );
        assert_eq!(sanitize_filename_part("A<B>C"), "A_B_C");
        assert_eq!(sanitize_filename_part("  Trimmed  "), "Trimmed");
    }

    #[test]
    fn test_generate_filename_with_metadata() {
        let metadata = ExtractedMetadata {
            title: Some("Bohemian Rhapsody".to_string()),
            artist: Some("Queen".to_string()),
            album: None,
            album_artist: None,
            track_number: None,
            disc_number: None,
            year: None,
            genres: Vec::new(),
            duration_seconds: None,
            bitrate: None,
            sample_rate: None,
            channels: None,
            file_format: "mp3".to_string(),
        };

        let source = Path::new("/path/to/song.mp3");
        let filename = generate_filename(source, &metadata).unwrap();
        assert_eq!(filename, "Queen - Bohemian Rhapsody.mp3");
    }

    #[test]
    fn test_generate_filename_no_artist() {
        let metadata = ExtractedMetadata {
            title: Some("Unknown Track".to_string()),
            artist: None,
            album: None,
            album_artist: None,
            track_number: None,
            disc_number: None,
            year: None,
            genres: Vec::new(),
            duration_seconds: None,
            bitrate: None,
            sample_rate: None,
            channels: None,
            file_format: "mp3".to_string(),
        };

        let source = Path::new("/path/to/song.mp3");
        let filename = generate_filename(source, &metadata).unwrap();
        assert_eq!(filename, "Unknown Track.mp3");
    }

    #[test]
    fn test_generate_filename_fallback() {
        let metadata = ExtractedMetadata {
            title: None,
            artist: None,
            album: None,
            album_artist: None,
            track_number: None,
            disc_number: None,
            year: None,
            genres: Vec::new(),
            duration_seconds: None,
            bitrate: None,
            sample_rate: None,
            channels: None,
            file_format: "mp3".to_string(),
        };

        let source = Path::new("/path/to/original_song.mp3");
        let filename = generate_filename(source, &metadata).unwrap();
        assert_eq!(filename, "original_song.mp3");
    }

    #[test]
    fn test_copy_to_library() {
        let temp = TempDir::new().unwrap();
        let source_dir = temp.path().join("source");
        let library_dir = temp.path().join("library");

        fs::create_dir(&source_dir).unwrap();

        // Create test file
        let source_file = source_dir.join("test.mp3");
        fs::write(&source_file, b"fake mp3 data").unwrap();

        let metadata = ExtractedMetadata {
            title: Some("Test Song".to_string()),
            artist: Some("Test Artist".to_string()),
            album: None,
            album_artist: None,
            track_number: None,
            disc_number: None,
            year: None,
            genres: Vec::new(),
            duration_seconds: None,
            bitrate: None,
            sample_rate: None,
            channels: None,
            file_format: "mp3".to_string(),
        };

        let dest_path = copy_to_library(&source_file, &library_dir, &metadata).unwrap();

        assert!(dest_path.exists());
        assert!(dest_path.ends_with("Test Artist - Test Song.mp3"));
        assert_eq!(
            fs::read(&dest_path).unwrap(),
            b"fake mp3 data",
            "File content should match"
        );
    }

    #[test]
    fn test_filename_conflict_resolution() {
        let temp = TempDir::new().unwrap();
        let library_dir = temp.path();

        // Create existing file
        fs::write(library_dir.join("song.mp3"), b"existing").unwrap();

        let new_path = resolve_filename_conflict(library_dir, "song.mp3").unwrap();

        assert_eq!(new_path, library_dir.join("song-1.mp3"));

        // Create the -1 file and test again
        fs::write(&new_path, b"existing").unwrap();
        let newer_path = resolve_filename_conflict(library_dir, "song.mp3").unwrap();

        assert_eq!(newer_path, library_dir.join("song-2.mp3"));
    }
}
