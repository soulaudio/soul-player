/// Library scanner implementation
use crate::error::Result;
use crate::reader::LoftyMetadataReader;
use soul_core::{MetadataReader, Storage, Track};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::mpsc;

/// Scan configuration
#[derive(Debug, Clone)]
pub struct ScanConfig {
    /// Use parallel processing (default: true)
    pub parallel: bool,

    /// Number of worker threads (default: num_cpus)
    pub num_threads: usize,

    /// Calculate file hashes for deduplication (default: true)
    pub use_file_hashing: bool,

    /// Supported audio file extensions
    pub extensions: Vec<String>,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            parallel: true,
            num_threads: num_cpus::get(),
            use_file_hashing: true,
            extensions: vec![
                "mp3".to_string(),
                "flac".to_string(),
                "ogg".to_string(),
                "opus".to_string(),
                "wav".to_string(),
                "m4a".to_string(),
                "aac".to_string(),
            ],
        }
    }
}

/// Scan progress updates
#[derive(Debug, Clone)]
pub enum ScanProgress {
    /// Scanning started
    Started { total_files: usize },

    /// File scanned
    FileScanned {
        path: PathBuf,
        success: bool,
        error: Option<String>,
    },

    /// Scanning completed
    Completed { stats: ScanStats },
}

/// Scan statistics
#[derive(Debug, Clone, Default)]
pub struct ScanStats {
    /// Number of files discovered
    pub files_discovered: usize,

    /// Number of files scanned
    pub files_scanned: usize,

    /// Number of tracks added to database
    pub tracks_added: usize,

    /// Number of tracks skipped (already in database)
    pub tracks_skipped: usize,

    /// Errors encountered
    pub errors: Vec<(PathBuf, String)>,
}

/// Library scanner
pub struct LibraryScanner<S: Storage + 'static> {
    reader: LoftyMetadataReader,
    db: Arc<S>,
    config: ScanConfig,
}

impl<S: Storage + 'static> LibraryScanner<S> {
    /// Create a new library scanner
    pub fn new(db: Arc<S>) -> Self {
        Self {
            reader: LoftyMetadataReader::new(),
            db,
            config: ScanConfig::default(),
        }
    }

    /// Create a scanner with custom configuration
    pub fn with_config(db: Arc<S>, config: ScanConfig) -> Self {
        Self {
            reader: LoftyMetadataReader::new(),
            db,
            config,
        }
    }

    /// Scan a directory for audio files
    ///
    /// # Arguments
    /// * `path` - Directory to scan
    /// * `progress_tx` - Optional channel for progress updates
    pub async fn scan(
        &self,
        path: &Path,
        progress_tx: Option<mpsc::Sender<ScanProgress>>,
    ) -> Result<ScanStats> {
        let mut stats = ScanStats::default();

        // Discover audio files
        let files = self.discover_files(path)?;
        stats.files_discovered = files.len();

        // Send started progress
        if let Some(ref tx) = progress_tx {
            let _ = tx
                .send(ScanProgress::Started {
                    total_files: files.len(),
                })
                .await;
        }

        // Process files (sequential for MVP, parallel later)
        for file_path in files {
            let result = self.process_file(&file_path).await;
            stats.files_scanned += 1;

            match result {
                Ok(true) => stats.tracks_added += 1,
                Ok(false) => stats.tracks_skipped += 1,
                Err(e) => {
                    stats.errors.push((file_path.clone(), e.to_string()));
                    if let Some(ref tx) = progress_tx {
                        let _ = tx
                            .send(ScanProgress::FileScanned {
                                path: file_path,
                                success: false,
                                error: Some(e.to_string()),
                            })
                            .await;
                    }
                    continue;
                }
            }

            // Send progress update
            if let Some(ref tx) = progress_tx {
                let _ = tx
                    .send(ScanProgress::FileScanned {
                        path: file_path,
                        success: true,
                        error: None,
                    })
                    .await;
            }
        }

        // Send completed progress
        if let Some(ref tx) = progress_tx {
            let _ = tx
                .send(ScanProgress::Completed {
                    stats: stats.clone(),
                })
                .await;
        }

        Ok(stats)
    }

    /// Discover audio files in a directory recursively
    fn discover_files(&self, path: &Path) -> Result<Vec<PathBuf>> {
        // Check if path exists
        if !path.exists() {
            return Err(crate::error::MetadataError::FileNotFound(
                path.display().to_string(),
            ));
        }

        let mut files = Vec::new();

        if path.is_file() {
            if self.is_supported_file(path) {
                files.push(path.to_path_buf());
            }
            return Ok(files);
        }

        for entry in walkdir::WalkDir::new(path)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.is_file() && self.is_supported_file(path) {
                files.push(path.to_path_buf());
            }
        }

        Ok(files)
    }

    /// Check if file is a supported audio format
    fn is_supported_file(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| self.config.extensions.contains(&e.to_lowercase()))
            .unwrap_or(false)
    }

    /// Process a single file
    ///
    /// Returns: Ok(true) if added, Ok(false) if skipped, Err on error
    async fn process_file(&self, path: &Path) -> Result<bool> {
        // Read metadata
        let metadata = self
            .reader
            .read(path)
            .map_err(|e| crate::error::MetadataError::ParseError(e.to_string()))?;

        // Create track
        let mut track = Track::new(
            metadata.title.clone().unwrap_or_else(|| {
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Unknown")
                    .to_string()
            }),
            path.to_path_buf(),
        );

        // Apply metadata
        track.artist = metadata.artist;
        track.album = metadata.album;
        track.album_artist = metadata.album_artist;
        track.track_number = metadata.track_number;
        track.disc_number = metadata.disc_number;
        track.year = metadata.year;
        track.genre = metadata.genre;
        track.duration_ms = metadata.duration_ms;

        // TODO: Calculate file hash if enabled
        // For MVP, use file path as uniqueness check

        // Check if track already exists
        // For MVP, we'll just add it (no duplicate detection yet)
        // TODO: Check by file_path or file_hash

        // Add to database
        self.db
            .add_track(track)
            .await
            .map_err(|e| crate::error::MetadataError::ParseError(e.to_string()))?;

        Ok(true)
    }
}

// TODO: Implement parallel processing
// TODO: Implement file hashing
// TODO: Implement incremental scanning (check existing tracks)
