//! Album artwork discovery from filesystem
//!
//! This module provides functionality for discovering album artwork files
//! in music directories. It searches for common artwork filenames like
//! "cover", "folder", "front", etc.

use std::path::{Path, PathBuf};

/// Discover folder artwork in a directory
///
/// Looks for common artwork filenames: cover, folder, front, album, artwork
/// Supports extensions: jpg, jpeg, png, webp, gif, bmp
///
/// # Arguments
///
/// * `folder` - Directory path to search for artwork
///
/// # Returns
///
/// Path to the artwork file if found, None otherwise
///
/// # Examples
///
/// ```no_run
/// use std::path::Path;
/// use soul_importer::artwork_discovery::discover_folder_artwork;
///
/// let folder = Path::new("/music/artist/album");
/// if let Some(artwork_path) = discover_folder_artwork(folder) {
///     println!("Found artwork: {}", artwork_path.display());
/// }
/// ```
pub fn discover_folder_artwork(folder: &Path) -> Option<PathBuf> {
    const FILENAMES: &[&str] = &["cover", "folder", "front", "album", "artwork"];
    const EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp", "gif", "bmp"];

    for name in FILENAMES {
        for ext in EXTENSIONS {
            let path = folder.join(format!("{}.{}", name, ext));
            if path.exists() {
                tracing::debug!(
                    "[ARTWORK] Found artwork: {} in {}",
                    path.display(),
                    folder.display()
                );
                return Some(path);
            }
        }
    }

    tracing::debug!("[ARTWORK] No artwork found in {}", folder.display());
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_discover_folder_artwork_cover_jpg() {
        let temp_dir = TempDir::new().unwrap();
        let cover_path = temp_dir.path().join("cover.jpg");
        fs::write(&cover_path, b"fake image data").unwrap();

        let result = discover_folder_artwork(temp_dir.path());
        assert!(result.is_some());
        assert_eq!(result.unwrap(), cover_path);
    }

    #[test]
    fn test_discover_folder_artwork_folder_png() {
        let temp_dir = TempDir::new().unwrap();
        let folder_path = temp_dir.path().join("folder.png");
        fs::write(&folder_path, b"fake image data").unwrap();

        let result = discover_folder_artwork(temp_dir.path());
        assert!(result.is_some());
        assert_eq!(result.unwrap(), folder_path);
    }

    #[test]
    fn test_discover_folder_artwork_priority() {
        let temp_dir = TempDir::new().unwrap();
        let cover_path = temp_dir.path().join("cover.jpg");
        let folder_path = temp_dir.path().join("folder.jpg");
        fs::write(&cover_path, b"fake image data").unwrap();
        fs::write(&folder_path, b"fake image data").unwrap();

        // Should prefer "cover" over "folder"
        let result = discover_folder_artwork(temp_dir.path());
        assert!(result.is_some());
        assert_eq!(result.unwrap(), cover_path);
    }

    #[test]
    fn test_discover_folder_artwork_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let result = discover_folder_artwork(temp_dir.path());
        assert!(result.is_none());
    }

    #[test]
    fn test_discover_folder_artwork_webp() {
        let temp_dir = TempDir::new().unwrap();
        let artwork_path = temp_dir.path().join("album.webp");
        fs::write(&artwork_path, b"fake image data").unwrap();

        let result = discover_folder_artwork(temp_dir.path());
        assert!(result.is_some());
        assert_eq!(result.unwrap(), artwork_path);
    }

    #[test]
    fn test_discover_folder_artwork_case_sensitive() {
        // This test verifies that we only look for lowercase filenames
        // On case-sensitive filesystems, COVER.jpg should not be found
        let temp_dir = TempDir::new().unwrap();
        let cover_path = temp_dir.path().join("COVER.JPG");
        fs::write(&cover_path, b"fake image data").unwrap();

        let result = discover_folder_artwork(temp_dir.path());

        // On Windows (case-insensitive), this will find the file
        // On Linux/macOS (case-sensitive), this won't find it
        #[cfg(windows)]
        assert!(result.is_some());

        #[cfg(not(windows))]
        assert!(result.is_none());
    }
}
