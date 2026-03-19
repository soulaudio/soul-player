//! File scanning for audio files

use crate::{ImportError, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Supported audio file extensions (shared with watcher module)
pub const SUPPORTED_EXTENSIONS: &[&str] = &[
    "mp3", "flac", "ogg", "wav", "aac", "m4a", "opus", "dsf", "dff", "dsdiff",
];

/// Scanner for audio files in directories
#[derive(Default)]
pub struct FileScanner {
    /// Whether to follow symbolic links
    follow_links: bool,

    /// Maximum depth to traverse (-1 for unlimited)
    max_depth: Option<usize>,
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

            // Skip macOS metadata files
            if let Some(filename) = path.file_name() {
                let filename_str = filename.to_string_lossy();
                if filename_str.starts_with("._")      // AppleDouble resource forks
                    || filename_str == ".DS_Store"     // Finder metadata
                    || filename_str == ".localized"    // Localization markers
                    || filename_str == "Icon\r"
                // Custom folder icons
                {
                    continue;
                }
            }

            // Check if file has supported extension
            if is_audio_file(path) {
                audio_files.push(path.to_path_buf());
            }
        }

        Ok(audio_files)
    }

    /// Scan a directory incrementally, skipping directories whose mtime hasn't changed.
    ///
    /// For each subdirectory in the tree:
    /// - If its mtime matches the stored value → skip (files are unchanged)
    /// - If its mtime differs or it's new → list audio files in that directory
    ///
    /// Returns files that need processing, plus metadata for updating the DB.
    pub fn scan_directory_incremental(
        &self,
        path: &Path,
        stored_dirs: &HashMap<String, StoredDirInfo>,
    ) -> Result<IncrementalScanResult> {
        if !path.exists() {
            return Err(ImportError::FileNotFound(path.display().to_string()));
        }
        if !path.is_dir() {
            return Err(ImportError::InvalidPath(format!(
                "{} is not a directory",
                path.display()
            )));
        }

        let mut changed_files = Vec::new();
        let mut unchanged_dir_count: i64 = 0;
        let mut scanned_dirs = Vec::new();
        // (unchanged_dir_file_paths removed — caller populates seen_paths from DB)

        // Walk directory tree, only looking at directory entries
        let mut walker = WalkDir::new(path).follow_links(self.follow_links);
        if let Some(depth) = self.max_depth {
            walker = walker.max_depth(depth);
        }

        // Collect all directories first
        let mut directories: Vec<PathBuf> = Vec::new();
        for entry in walker.into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_dir() {
                directories.push(entry.path().to_path_buf());
            }
        }

        for dir in &directories {
            let dir_str = dir.display().to_string();

            // Get directory mtime
            let dir_mtime = match std::fs::metadata(dir) {
                Ok(m) => m
                    .modified()
                    .map(|t| {
                        t.duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_secs() as i64)
                            .unwrap_or(0)
                    })
                    .unwrap_or(0),
                Err(e) => {
                    tracing::warn!("Failed to stat directory {:?}: {}", dir, e);
                    continue;
                }
            };

            // Check if directory has changed
            if let Some(stored) = stored_dirs.get(&dir_str) {
                if stored.dir_mtime == dir_mtime {
                    // Mtime unchanged — quick-verify entry count as a safety check.
                    // On NTFS, rapid file additions within the same second may not
                    // update directory mtime. Counting entries is cheap (no per-file stat).
                    let actual_count = std::fs::read_dir(dir)
                        .map(|entries| {
                            entries
                                .filter_map(|e| e.ok())
                                .filter(|e| {
                                    // Use file_type() from DirEntry — avoids stat() syscall
                                    e.file_type().map(|ft| ft.is_file()).unwrap_or(false)
                                        && is_audio_file(&e.path())
                                })
                                .count() as i64
                        })
                        .unwrap_or(0);

                    if actual_count == stored.file_count {
                        // Truly unchanged — skip. No file paths collected here;
                        // caller populates seen_paths from DB existing tracks.
                        unchanged_dir_count += 1;
                        scanned_dirs.push(ScannedDirInfo {
                            path: dir_str,
                            dir_mtime,
                            file_count: stored.file_count,
                        });
                        continue;
                    }
                    // File count mismatch despite same mtime — fall through to rescan
                    tracing::debug!(
                        "[SCAN] Dir mtime unchanged but file count differs ({} vs {}): {}",
                        stored.file_count,
                        actual_count,
                        dir_str
                    );
                }
            }

            // Changed or new directory — list audio files (non-recursive)
            let mut file_count: i64 = 0;
            match std::fs::read_dir(dir) {
                Ok(entries) => {
                    for entry in entries.filter_map(|e| e.ok()) {
                        let entry_path = entry.path();
                        if !entry_path.is_file() {
                            continue;
                        }
                        // Skip macOS metadata files
                        if let Some(filename) = entry_path.file_name() {
                            let filename_str = filename.to_string_lossy();
                            if filename_str.starts_with("._")
                                || filename_str == ".DS_Store"
                                || filename_str == ".localized"
                                || filename_str == "Icon\r"
                            {
                                continue;
                            }
                        }
                        if is_audio_file(&entry_path) {
                            changed_files.push(entry_path);
                            file_count += 1;
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to read directory {:?}: {}", dir, e);
                }
            }

            scanned_dirs.push(ScannedDirInfo {
                path: dir_str,
                dir_mtime,
                file_count,
            });
        }

        tracing::info!(
            "[SCAN] Incremental scan: {} dirs checked, {} unchanged (skipped), {} files to process",
            directories.len(),
            unchanged_dir_count,
            changed_files.len()
        );

        Ok(IncrementalScanResult {
            changed_files,
            unchanged_dir_count,
            scanned_dirs,
        })
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

/// Info about a previously scanned directory, loaded from DB
#[derive(Debug, Clone)]
pub struct StoredDirInfo {
    pub dir_mtime: i64,
    pub file_count: i64,
}

/// Info about a directory after scanning, to be persisted to DB
#[derive(Debug, Clone)]
pub struct ScannedDirInfo {
    pub path: String,
    pub dir_mtime: i64,
    pub file_count: i64,
}

/// Result of an incremental directory scan
pub struct IncrementalScanResult {
    /// Audio files in directories whose mtime changed or that are new
    pub changed_files: Vec<PathBuf>,
    /// Number of directories skipped (unchanged mtime)
    pub unchanged_dir_count: i64,
    /// Updated directory info to persist to DB
    pub scanned_dirs: Vec<ScannedDirInfo>,
}

/// Check if a file is a supported audio file
pub fn is_audio_file(path: &Path) -> bool {
    path.extension()
        .map(|ext| {
            let ext_str = ext.to_string_lossy().to_lowercase();
            SUPPORTED_EXTENSIONS.contains(&ext_str.as_str())
        })
        .unwrap_or(false)
}

/// Get the audio file extension from a path
pub fn get_audio_extension(path: &Path) -> Option<String> {
    path.extension()
        .map(|ext| ext.to_string_lossy().to_lowercase())
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

    #[test]
    fn test_scan_directory_incremental_detects_new_dir() {
        let temp = TempDir::new().unwrap();
        let base = temp.path();

        fs::write(base.join("song1.mp3"), b"fake mp3").unwrap();
        let subdir = base.join("album");
        fs::create_dir(&subdir).unwrap();
        fs::write(subdir.join("song2.flac"), b"fake flac").unwrap();

        // No stored dirs — everything is new
        let stored = HashMap::new();
        let scanner = FileScanner::new();
        let result = scanner.scan_directory_incremental(base, &stored).unwrap();

        assert_eq!(result.unchanged_dir_count, 0);
        assert!(result.changed_files.len() >= 2);
        assert!(result.changed_files.iter().any(|p| p.ends_with("song1.mp3")));
        assert!(result.changed_files.iter().any(|p| p.ends_with("song2.flac")));
    }

    #[test]
    fn test_scan_directory_incremental_skips_unchanged() {
        let temp = TempDir::new().unwrap();
        let base = temp.path();

        let subdir = base.join("album");
        fs::create_dir(&subdir).unwrap();
        fs::write(subdir.join("song1.mp3"), b"fake mp3").unwrap();

        // First scan to get real mtimes
        let scanner = FileScanner::new();
        let first_result = scanner
            .scan_directory_incremental(base, &HashMap::new())
            .unwrap();

        // Build stored dirs from first scan
        let stored: HashMap<String, StoredDirInfo> = first_result
            .scanned_dirs
            .into_iter()
            .map(|d| (d.path, StoredDirInfo { dir_mtime: d.dir_mtime, file_count: d.file_count }))
            .collect();

        // Second scan with stored dirs — nothing changed
        let second_result = scanner.scan_directory_incremental(base, &stored).unwrap();

        assert_eq!(second_result.changed_files.len(), 0);
        assert!(second_result.unchanged_dir_count > 0);
    }

    #[test]
    fn test_scan_directory_incremental_detects_changed_dir() {
        let temp = TempDir::new().unwrap();
        let base = temp.path();

        let subdir = base.join("album");
        fs::create_dir(&subdir).unwrap();
        fs::write(subdir.join("song1.mp3"), b"fake mp3").unwrap();

        // Store dirs with a wrong mtime (simulating a change)
        let mut stored = HashMap::new();
        stored.insert(
            subdir.display().to_string(),
            StoredDirInfo { dir_mtime: 0, file_count: 1 },
        );

        let scanner = FileScanner::new();
        let result = scanner.scan_directory_incremental(base, &stored).unwrap();

        // The album dir has a different mtime than 0, so it should be rescanned
        assert!(result.changed_files.iter().any(|p| p.ends_with("song1.mp3")));
    }

    #[test]
    fn test_unchanged_dirs_are_skipped_no_filesystem_io() {
        let temp = TempDir::new().unwrap();
        let base = temp.path();

        let subdir = base.join("album");
        fs::create_dir(&subdir).unwrap();
        fs::write(subdir.join("song1.mp3"), b"fake mp3").unwrap();
        fs::write(subdir.join("song2.flac"), b"fake flac").unwrap();

        // First scan to get real mtimes
        let scanner = FileScanner::new();
        let first_result = scanner
            .scan_directory_incremental(base, &HashMap::new())
            .unwrap();

        let stored: HashMap<String, StoredDirInfo> = first_result
            .scanned_dirs
            .into_iter()
            .map(|d| (d.path, StoredDirInfo { dir_mtime: d.dir_mtime, file_count: d.file_count }))
            .collect();

        // Second scan — all dirs should be skipped (no changed files)
        let second_result = scanner.scan_directory_incremental(base, &stored).unwrap();

        assert_eq!(second_result.changed_files.len(), 0);
        // Both root dir and album subdir should be skipped
        assert!(second_result.unchanged_dir_count >= 2);
    }

    #[test]
    fn bench_incremental_scan_10k_files() {
        use std::time::Instant;

        let temp = TempDir::new().unwrap();
        let base = temp.path();

        // Create 500 directories with 20 files each = 10,000 files
        for i in 0..500 {
            let dir = base.join(format!("album_{:04}", i));
            fs::create_dir(&dir).unwrap();
            for j in 0..20 {
                fs::write(dir.join(format!("track_{:02}.mp3", j)), b"fake").unwrap();
            }
        }

        let scanner = FileScanner::new();

        // First scan (all new — baseline)
        let start = Instant::now();
        let result1 = scanner.scan_directory_incremental(base, &HashMap::new()).unwrap();
        let first_scan = start.elapsed();

        assert_eq!(result1.changed_files.len(), 10_000);
        assert_eq!(result1.unchanged_dir_count, 0);

        // Build stored dirs from first scan
        let stored: HashMap<String, StoredDirInfo> = result1.scanned_dirs
            .into_iter()
            .map(|d| (d.path, StoredDirInfo { dir_mtime: d.dir_mtime, file_count: d.file_count }))
            .collect();

        // Second scan (all unchanged)
        let start = Instant::now();
        let result2 = scanner.scan_directory_incremental(base, &stored).unwrap();
        let second_scan = start.elapsed();

        assert_eq!(result2.changed_files.len(), 0);
        assert!(result2.unchanged_dir_count >= 500);

        // Add 1 album (12 files)
        let new_dir = base.join("new_album");
        fs::create_dir(&new_dir).unwrap();
        for j in 0..12 {
            fs::write(new_dir.join(format!("new_{:02}.mp3", j)), b"fake").unwrap();
        }

        let start = Instant::now();
        let result3 = scanner.scan_directory_incremental(base, &stored).unwrap();
        let third_scan = start.elapsed();

        assert_eq!(result3.changed_files.len(), 12);

        eprintln!("\n=== INCREMENTAL SCAN BENCHMARK (10,000 files / 500 dirs) ===");
        eprintln!("First scan (all new):     {:?}", first_scan);
        eprintln!("Unchanged rescan:         {:?} ({:.0}x faster)", second_scan,
            first_scan.as_micros() as f64 / second_scan.as_micros().max(1) as f64);
        eprintln!("1 album added rescan:     {:?} ({:.0}x faster)", third_scan,
            first_scan.as_micros() as f64 / third_scan.as_micros().max(1) as f64);
        eprintln!("============================================================\n");
    }
}
