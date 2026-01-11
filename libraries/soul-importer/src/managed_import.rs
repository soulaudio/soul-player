//! Managed library import functionality
//!
//! Handles importing files into an organized managed library structure.
//! Features:
//! - Path template-based organization
//! - SHA256 duplicate detection
//! - Safe file copy with verification
//! - Progress tracking

use crate::{metadata, path_template::PathTemplate, ImportError, Result};
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use std::fs::{self, File};
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use tracing::{debug, info};

/// Default buffer size for file operations (64KB)
const BUFFER_SIZE: usize = 64 * 1024;

/// Import action (copy or move)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImportAction {
    /// Copy file to library (preserves original)
    #[default]
    Copy,
    /// Move file to library (removes original)
    Move,
}

/// Result of importing a single file
#[derive(Debug, Clone)]
pub enum ImportResult {
    /// File was imported successfully
    Imported {
        source_path: PathBuf,
        dest_path: PathBuf,
        content_hash: String,
    },
    /// File was skipped (duplicate)
    Skipped {
        source_path: PathBuf,
        reason: SkipReason,
    },
    /// Import failed
    Failed { source_path: PathBuf, error: String },
}

/// Reason for skipping a file
#[derive(Debug, Clone)]
pub enum SkipReason {
    /// Exact duplicate (same hash) already in library
    Duplicate { existing_path: PathBuf },
    /// Not an audio file
    NotAudioFile,
    /// File doesn't exist
    FileNotFound,
}

/// Statistics from an import operation
#[derive(Debug, Default, Clone)]
pub struct ImportStats {
    pub total_files: usize,
    pub imported: usize,
    pub skipped_duplicate: usize,
    pub skipped_not_audio: usize,
    pub failed: usize,
}

/// Progress callback for import operations
pub type ProgressCallback = Box<dyn Fn(&ImportStats, &ImportResult) + Send + Sync>;

/// Managed library importer
pub struct ManagedImporter {
    pool: SqlitePool,
    library_path: PathBuf,
    template: PathTemplate,
    action: ImportAction,
    verify_copy: bool,
    progress_callback: Option<ProgressCallback>,
}

impl ManagedImporter {
    /// Create a new managed importer
    pub fn new(pool: SqlitePool, library_path: impl Into<PathBuf>) -> Self {
        Self {
            pool,
            library_path: library_path.into(),
            template: PathTemplate::default(),
            action: ImportAction::Copy,
            verify_copy: true,
            progress_callback: None,
        }
    }

    /// Set the path template
    pub fn with_template(mut self, template: PathTemplate) -> Self {
        self.template = template;
        self
    }

    /// Set the import action (copy or move)
    pub fn with_action(mut self, action: ImportAction) -> Self {
        self.action = action;
        self
    }

    /// Set whether to verify copied files
    pub fn with_verification(mut self, verify: bool) -> Self {
        self.verify_copy = verify;
        self
    }

    /// Set progress callback
    pub fn on_progress(mut self, callback: ProgressCallback) -> Self {
        self.progress_callback = Some(callback);
        self
    }

    /// Import a single file
    pub async fn import_file(&self, source_path: &Path) -> Result<ImportResult> {
        // Check if file exists
        if !source_path.exists() {
            return Ok(ImportResult::Skipped {
                source_path: source_path.to_path_buf(),
                reason: SkipReason::FileNotFound,
            });
        }

        // Check if it's an audio file
        if !is_audio_file(source_path) {
            return Ok(ImportResult::Skipped {
                source_path: source_path.to_path_buf(),
                reason: SkipReason::NotAudioFile,
            });
        }

        // Compute content hash for duplicate detection
        let content_hash = compute_file_hash(source_path)?;

        // Check for duplicates
        if let Some(existing) = self.find_duplicate(&content_hash).await? {
            return Ok(ImportResult::Skipped {
                source_path: source_path.to_path_buf(),
                reason: SkipReason::Duplicate {
                    existing_path: existing,
                },
            });
        }

        // Extract metadata
        let extracted_metadata = metadata::extract_metadata(source_path)?;

        // Resolve destination path using template
        let relative_path = self.template.resolve(&extracted_metadata, source_path);
        let dest_path = self.library_path.join(&relative_path);

        // Ensure parent directory exists
        if let Some(parent) = dest_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Handle existing file at destination
        let final_dest = if dest_path.exists() {
            self.resolve_conflict(&dest_path)?
        } else {
            dest_path.clone()
        };

        // Copy or move the file
        match self.action {
            ImportAction::Copy => {
                copy_file_verified(source_path, &final_dest, self.verify_copy)?;
            }
            ImportAction::Move => {
                move_file(source_path, &final_dest)?;
            }
        }

        info!(
            "Imported: {:?} -> {:?}",
            source_path.file_name(),
            relative_path
        );

        Ok(ImportResult::Imported {
            source_path: source_path.to_path_buf(),
            dest_path: final_dest,
            content_hash,
        })
    }

    /// Import multiple files
    pub async fn import_files(&self, source_paths: &[PathBuf]) -> Result<Vec<ImportResult>> {
        let mut results = Vec::with_capacity(source_paths.len());
        let mut stats = ImportStats {
            total_files: source_paths.len(),
            ..Default::default()
        };

        for source_path in source_paths {
            let result = self.import_file(source_path).await?;

            // Update stats
            match &result {
                ImportResult::Imported { .. } => stats.imported += 1,
                ImportResult::Skipped { reason, .. } => match reason {
                    SkipReason::Duplicate { .. } => stats.skipped_duplicate += 1,
                    SkipReason::NotAudioFile => stats.skipped_not_audio += 1,
                    SkipReason::FileNotFound => stats.failed += 1,
                },
                ImportResult::Failed { .. } => stats.failed += 1,
            }

            // Call progress callback
            if let Some(ref callback) = self.progress_callback {
                callback(&stats, &result);
            }

            results.push(result);
        }

        Ok(results)
    }

    /// Import all audio files from a directory
    pub async fn import_directory(&self, source_dir: &Path) -> Result<Vec<ImportResult>> {
        let mut files = Vec::new();

        // Collect all audio files
        for entry in walkdir::WalkDir::new(source_dir)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.is_file() && is_audio_file(path) {
                files.push(path.to_path_buf());
            }
        }

        self.import_files(&files).await
    }

    /// Find a duplicate file by hash
    async fn find_duplicate(&self, content_hash: &str) -> Result<Option<PathBuf>> {
        // Check in database for existing track with this hash
        let file_path =
            soul_storage::tracks::find_path_by_content_hash(&self.pool, content_hash).await?;

        Ok(file_path.map(PathBuf::from))
    }

    /// Resolve filename conflict by appending counter
    fn resolve_conflict(&self, dest_path: &Path) -> Result<PathBuf> {
        let stem = dest_path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| ImportError::InvalidPath("Invalid filename".to_string()))?;

        let extension = dest_path.extension().and_then(|s| s.to_str()).unwrap_or("");

        let parent = dest_path.parent().unwrap_or(Path::new(""));

        for counter in 1..1000 {
            let new_name = if extension.is_empty() {
                format!("{} ({})", stem, counter)
            } else {
                format!("{} ({}).{}", stem, counter, extension)
            };

            let new_path = parent.join(&new_name);
            if !new_path.exists() {
                return Ok(new_path);
            }
        }

        Err(ImportError::Unknown(
            "Could not resolve filename conflict after 1000 attempts".to_string(),
        ))
    }
}

/// Compute SHA256 hash of a file
pub fn compute_file_hash(path: &Path) -> Result<String> {
    let file = File::open(path)?;
    let mut reader = BufReader::with_capacity(BUFFER_SIZE, file);
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; BUFFER_SIZE];

    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    let hash = hasher.finalize();
    Ok(hex::encode(hash))
}

/// Copy a file with optional verification
pub fn copy_file_verified(source: &Path, dest: &Path, verify: bool) -> Result<()> {
    // Compute source hash if verification is enabled
    let source_hash = if verify {
        Some(compute_file_hash(source)?)
    } else {
        None
    };

    // Copy the file
    fs::copy(source, dest)?;

    // Verify if enabled
    if let Some(expected_hash) = source_hash {
        let actual_hash = compute_file_hash(dest)?;
        if expected_hash != actual_hash {
            // Delete the corrupted copy
            let _ = fs::remove_file(dest);
            return Err(ImportError::Unknown(format!(
                "File verification failed: hash mismatch for {:?}",
                dest
            )));
        }
        debug!("File verification passed: {:?}", dest);
    }

    Ok(())
}

/// Move a file (copy + delete source)
pub fn move_file(source: &Path, dest: &Path) -> Result<()> {
    // Try rename first (fast if on same filesystem)
    if fs::rename(source, dest).is_ok() {
        return Ok(());
    }

    // Fall back to copy + delete
    copy_file_verified(source, dest, true)?;
    fs::remove_file(source)?;

    Ok(())
}

/// Check if a path is an audio file based on extension
fn is_audio_file(path: &Path) -> bool {
    let audio_extensions = [
        "flac", "mp3", "m4a", "aac", "ogg", "opus", "wav", "aif", "aiff",
    ];

    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| audio_extensions.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_file(dir: &Path, name: &str, content: &[u8]) -> PathBuf {
        let path = dir.join(name);
        let mut file = File::create(&path).expect("Failed to create test file");
        file.write_all(content).expect("Failed to write test file");
        path
    }

    #[test]
    fn test_compute_file_hash() {
        let temp = TempDir::new().unwrap();
        let file = create_test_file(temp.path(), "test.txt", b"Hello, World!");

        let hash = compute_file_hash(&file).unwrap();

        // SHA256 of "Hello, World!"
        assert_eq!(
            hash,
            "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f"
        );
    }

    #[test]
    fn test_copy_file_verified() {
        let temp = TempDir::new().unwrap();
        let source = create_test_file(temp.path(), "source.txt", b"Test content");
        let dest = temp.path().join("dest.txt");

        copy_file_verified(&source, &dest, true).unwrap();

        assert!(dest.exists());
        assert_eq!(fs::read(&dest).unwrap(), b"Test content");
    }

    #[test]
    fn test_move_file() {
        let temp = TempDir::new().unwrap();
        let source = create_test_file(temp.path(), "source.txt", b"Test content");
        let dest = temp.path().join("dest.txt");

        move_file(&source, &dest).unwrap();

        assert!(!source.exists());
        assert!(dest.exists());
        assert_eq!(fs::read(&dest).unwrap(), b"Test content");
    }

    #[test]
    fn test_is_audio_file() {
        assert!(is_audio_file(Path::new("test.flac")));
        assert!(is_audio_file(Path::new("test.mp3")));
        assert!(is_audio_file(Path::new("test.FLAC")));
        assert!(is_audio_file(Path::new("/path/to/test.m4a")));
        assert!(!is_audio_file(Path::new("test.txt")));
        assert!(!is_audio_file(Path::new("test.jpg")));
    }

    #[test]
    fn test_import_action_default() {
        assert_eq!(ImportAction::default(), ImportAction::Copy);
    }

    #[test]
    fn test_import_stats_default() {
        let stats = ImportStats::default();
        assert_eq!(stats.total_files, 0);
        assert_eq!(stats.imported, 0);
        assert_eq!(stats.skipped_duplicate, 0);
        assert_eq!(stats.failed, 0);
    }
}
