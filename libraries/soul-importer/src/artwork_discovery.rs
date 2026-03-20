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

    // 1. Standard names in this folder (highest priority)
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

    // 2. Fall back to the immediate parent directory (one level up only).
    //    Handles multi-disc layouts like Album/Disc 1/track.flac where
    //    Album/cover.jpg lives at the album root level.
    if let Some(parent) = folder.parent() {
        for name in FILENAMES {
            for ext in EXTENSIONS {
                let path = parent.join(format!("{}.{}", name, ext));
                if path.exists() {
                    tracing::debug!(
                        "[ARTWORK] Found artwork in parent dir: {} for folder {}",
                        path.display(),
                        folder.display()
                    );
                    return Some(path);
                }
            }
        }
    }

    // 3. Any image file in this folder (Discogs/bandcamp downloads, etc.)
    //    Excludes macOS resource forks (._filename).
    //    Does NOT extend to parent — the parent fallback only uses standard names
    //    to avoid picking up unrelated images in shared directories.
    if let Some(any) = find_any_image_in_dir(folder) {
        tracing::debug!(
            "[ARTWORK] Found fallback image: {} in {}",
            any.display(),
            folder.display()
        );
        return Some(any);
    }

    tracing::debug!("[ARTWORK] No artwork found in {}", folder.display());
    None
}

/// Find any image file in a directory, excluding resource forks.
fn find_any_image_in_dir(dir: &Path) -> Option<PathBuf> {
    const IMAGE_EXTS: &[&str] = &["jpg", "jpeg", "png", "webp", "gif", "bmp"];

    let entries = std::fs::read_dir(dir).ok()?;
    let mut candidates: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name();
            let name_str = name.to_string_lossy();
            // Skip resource forks
            if name_str.starts_with("._") {
                return false;
            }
            let lower = name_str.to_lowercase();
            IMAGE_EXTS
                .iter()
                .any(|ext| lower.ends_with(&format!(".{ext}")))
        })
        .map(|e| e.path())
        .collect();

    if candidates.is_empty() {
        return None;
    }

    // Prefer larger files (more likely to be full-res album art, not thumbnails)
    candidates.sort_by_key(|p| {
        std::fs::metadata(p)
            .map(|m| std::cmp::Reverse(m.len()))
            .unwrap_or(std::cmp::Reverse(0))
    });

    candidates.into_iter().next()
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
    fn test_discover_folder_artwork_parent_dir_fallback() {
        // Multi-disc layout: Album/Disc 1/track.flac with Album/cover.jpg
        let temp_dir = TempDir::new().unwrap();
        let album_dir = temp_dir.path().join("Album");
        let disc_dir = album_dir.join("Disc 1");
        fs::create_dir_all(&disc_dir).unwrap();
        // Create a dummy track file in the disc subdirectory
        fs::write(disc_dir.join("track.flac"), b"").unwrap();
        // Place cover art at the album level (one level up)
        let cover_path = album_dir.join("cover.jpg");
        fs::write(&cover_path, b"fake image data").unwrap();

        // Should find the cover in the parent directory
        let result = discover_folder_artwork(&disc_dir);
        assert!(
            result.is_some(),
            "Expected cover.jpg to be found in parent dir"
        );
        assert_eq!(result.unwrap(), cover_path);
    }

    #[test]
    fn test_discover_folder_artwork_no_grandparent_climb() {
        // Cover is two levels up — should NOT be found
        let temp_dir = TempDir::new().unwrap();
        let album_dir = temp_dir.path().join("Album");
        let disc_dir = album_dir.join("Disc 1");
        let track_dir = disc_dir.join("Bonus");
        fs::create_dir_all(&track_dir).unwrap();
        // Cover art is at the grandparent (album) level, two levels above track_dir
        let cover_path = album_dir.join("cover.jpg");
        fs::write(&cover_path, b"fake image data").unwrap();

        // Should NOT find the grandparent's cover
        let result = discover_folder_artwork(&track_dir);
        assert!(
            result.is_none(),
            "Should not climb more than 1 level up (found {:?})",
            result
        );
    }

    #[test]
    fn test_discover_folder_artwork_discogs_style_filename() {
        // Real-world: Discogs downloads use names like "R-16384242-1607809641-1411.jpg"
        // These don't match any standard name but should still be picked up as a fallback.
        let temp_dir = TempDir::new().unwrap();
        let discogs_img = temp_dir.path().join("R-16384242-1607809641-1411.jpg");
        fs::write(&discogs_img, b"fake jpeg data").unwrap();

        let result = discover_folder_artwork(temp_dir.path());
        assert!(
            result.is_some(),
            "Should find Discogs-style image as fallback when no standard cover name exists"
        );
    }

    #[test]
    fn test_discover_folder_artwork_any_jpg_fallback() {
        // Any .jpg in the folder should be picked up when no standard name matches.
        let temp_dir = TempDir::new().unwrap();
        let img = temp_dir.path().join("bandcamp_art_2048.jpg");
        fs::write(&img, b"fake jpeg data").unwrap();

        let result = discover_folder_artwork(temp_dir.path());
        assert!(result.is_some(), "Any .jpg should be picked as fallback");
    }

    #[test]
    fn test_discover_folder_artwork_named_beats_any_fallback() {
        // Standard "cover.jpg" should always win over any random image.
        let temp_dir = TempDir::new().unwrap();
        let cover = temp_dir.path().join("cover.jpg");
        let random = temp_dir.path().join("R-99999-12345.jpg");
        fs::write(&cover, b"cover data").unwrap();
        fs::write(&random, b"discogs data").unwrap();

        let result = discover_folder_artwork(temp_dir.path());
        assert_eq!(
            result.unwrap(),
            cover,
            "Named 'cover.jpg' must win over random filenames"
        );
    }

    #[test]
    fn test_discover_folder_artwork_resource_fork_skipped() {
        // macOS resource fork files (._filename) must not be picked as artwork.
        let temp_dir = TempDir::new().unwrap();
        let fork = temp_dir.path().join("._R-16384242-1607809641-1411.jpg");
        fs::write(&fork, b"fake resource fork").unwrap();

        let result = discover_folder_artwork(temp_dir.path());
        assert!(
            result.is_none(),
            "Resource fork files (._) must be excluded"
        );
    }

    #[test]
    fn test_discover_folder_artwork_case_sensitive() {
        // This test verifies that we only look for lowercase filenames
        // On case-sensitive filesystems, COVER.jpg should not be found
        let temp_dir = TempDir::new().unwrap();
        let cover_path = temp_dir.path().join("COVER.JPG");
        fs::write(&cover_path, b"fake image data").unwrap();

        let result = discover_folder_artwork(temp_dir.path());

        // On Windows and macOS (case-insensitive filesystems), this will find the file.
        // On Linux (case-sensitive), this won't find it.
        #[cfg(any(windows, target_os = "macos"))]
        assert!(result.is_some());

        #[cfg(target_os = "linux")]
        assert!(result.is_none());
    }
}
