//! File scanning for audio files

use crate::{ImportError, Result};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Supported audio file extensions
const SUPPORTED_EXTENSIONS: &[&str] = &["mp3", "flac", "ogg", "wav", "aac", "m4a", "opus"];

/// Scanner for audio files in directories
pub struct FileScanner {
    /// Whether to follow symbolic links
    follow_links: bool,

    /// Maximum depth to traverse (-1 for unlimited)
    max_depth: Option<usize>,
}

impl Default for FileScanner {
    fn default() -> Self {
        Self {
            follow_links: false,
            max_depth: None,
        }
    }
}

impl FileScanner {
    /// Create a new file scanner
    pub fn new() -> Self {
        Self::default()
    }

    /// Set whether to follow symbolic links
    pub fn follow_links(mut self, follow: bool) -> Self {
        self.follow_links = follow;
        self
    }

    /// Set maximum directory depth to traverse
    pub fn max_depth(mut self, depth: usize) -> Self {
        self.max_depth = Some(depth);
        self
    }

    /// Scan a directory for audio files
    ///
    /// # Arguments
    ///
    /// * `path` - Directory path to scan
    ///
    /// # Returns
    ///
    /// List of audio file paths found
    pub fn scan_directory(&self, path: &Path) -> Result<Vec<PathBuf>> {
        if !path.exists() {
            return Err(ImportError::FileNotFound(path.display().to_string()));
        }

        if !path.is_dir() {
            return Err(ImportError::InvalidPath(format!(
                "{} is not a directory",
                path.display()
            )));
        }

        let mut audio_files = Vec::new();
        let mut walker = WalkDir::new(path).follow_links(self.follow_links);

        if let Some(depth) = self.max_depth {
            walker = walker.max_depth(depth);
        }

        for entry in walker.into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();

            // Skip directories
            if path.is_dir() {
                continue;
            }

            // Check if file has supported extension
            if is_audio_file(path) {
                audio_files.push(path.to_path_buf());
            }
        }

        Ok(audio_files)
    }

    /// Scan multiple directories for audio files
    pub fn scan_directories(&self, paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
        let mut all_files = Vec::new();

        for path in paths {
            match self.scan_directory(path) {
                Ok(mut files) => all_files.append(&mut files),
                Err(e) => {
                    tracing::warn!("Failed to scan {}: {}", path.display(), e);
                }
            }
        }

        Ok(all_files)
    }

    /// Validate individual files
    ///
    /// Returns only valid audio files from the list
    pub fn validate_files(&self, paths: &[PathBuf]) -> Vec<PathBuf> {
        paths
            .iter()
            .filter(|path| path.exists() && path.is_file() && is_audio_file(path))
            .cloned()
            .collect()
    }
}

/// Check if a file is a supported audio file
pub fn is_audio_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| SUPPORTED_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// Get the audio file extension from a path
pub fn get_audio_extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_lowercase())
        .filter(|ext| SUPPORTED_EXTENSIONS.contains(&ext.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_is_audio_file() {
        assert!(is_audio_file(Path::new("test.mp3")));
        assert!(is_audio_file(Path::new("test.MP3")));
        assert!(is_audio_file(Path::new("test.flac")));
        assert!(is_audio_file(Path::new("test.ogg")));
        assert!(!is_audio_file(Path::new("test.txt")));
        assert!(!is_audio_file(Path::new("test")));
    }

    #[test]
    fn test_get_audio_extension() {
        assert_eq!(
            get_audio_extension(Path::new("test.mp3")),
            Some("mp3".to_string())
        );
        assert_eq!(
            get_audio_extension(Path::new("test.MP3")),
            Some("mp3".to_string())
        );
        assert_eq!(get_audio_extension(Path::new("test.txt")), None);
    }

    #[test]
    fn test_scan_directory() {
        let temp = TempDir::new().unwrap();
        let base = temp.path();

        // Create test files
        fs::write(base.join("song1.mp3"), b"fake mp3").unwrap();
        fs::write(base.join("song2.flac"), b"fake flac").unwrap();
        fs::write(base.join("readme.txt"), b"not audio").unwrap();

        // Create subdirectory
        let subdir = base.join("subdir");
        fs::create_dir(&subdir).unwrap();
        fs::write(subdir.join("song3.ogg"), b"fake ogg").unwrap();

        let scanner = FileScanner::new();
        let files = scanner.scan_directory(base).unwrap();

        assert_eq!(files.len(), 3);
        assert!(files.iter().any(|p| p.ends_with("song1.mp3")));
        assert!(files.iter().any(|p| p.ends_with("song2.flac")));
        assert!(files.iter().any(|p| p.ends_with("song3.ogg")));
        assert!(!files.iter().any(|p| p.ends_with("readme.txt")));
    }

    #[test]
    fn test_scan_with_max_depth() {
        let temp = TempDir::new().unwrap();
        let base = temp.path();

        fs::write(base.join("song1.mp3"), b"fake mp3").unwrap();

        let subdir = base.join("subdir");
        fs::create_dir(&subdir).unwrap();
        fs::write(subdir.join("song2.mp3"), b"fake mp3").unwrap();

        // Scan with max_depth = 1 (only base directory)
        let scanner = FileScanner::new().max_depth(1);
        let files = scanner.scan_directory(base).unwrap();

        assert_eq!(files.len(), 1);
        assert!(files.iter().any(|p| p.ends_with("song1.mp3")));
        assert!(!files.iter().any(|p| p.ends_with("song2.mp3")));
    }
}
