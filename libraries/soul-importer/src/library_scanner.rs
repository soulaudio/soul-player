//! Library scanner for watched folders
//!
//! Scans library sources (watched folders) and synchronizes with the database.
//! Uses mtime + size for change detection and content hash for file relocation.
//!
//! This module orchestrates filesystem traversal and delegates hash computation,
//! metadata extraction, and file processing to specialized modules.

use crate::{
    file_processor::{FileAction, FileProcessor},
    hash_computer,
    metadata_extractor::MetadataExtractor,
    scanner::FileScanner,
    ImportError, Result,
};
use soul_core::types::{LibrarySource, ScanStatus};
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

/// Statistics from a library scan
#[derive(Debug, Default, Clone)]
pub struct ScanStats {
    pub total_files: i64,
    pub processed: i64,
    pub new_files: i64,
    pub updated_files: i64,
    pub removed_files: i64,
    pub relocated_files: i64,
    pub errors: i64,
}

/// Callback for scan progress updates
pub type ProgressCallback = Box<dyn Fn(&ScanStats) + Send + Sync>;

/// Library scanner for watched folders
pub struct LibraryScanner {
    pool: SqlitePool,
    user_id: String,
    device_id: String,
    /// Whether to compute content hash for all files (expensive but enables relocation detection)
    compute_hashes: bool,
    /// Progress callback
    progress_callback: Option<ProgressCallback>,
    /// Metadata extractor with fuzzy matching
    metadata_extractor: MetadataExtractor,
    /// Force re-extraction of metadata even for unchanged files
    force_metadata_refresh: bool,
}

impl LibraryScanner {
    /// Create a new library scanner
    pub fn new(pool: SqlitePool, user_id: impl Into<String>, device_id: impl Into<String>) -> Self {
        Self {
            pool,
            user_id: user_id.into(),
            device_id: device_id.into(),
            compute_hashes: true,
            progress_callback: None,
            metadata_extractor: MetadataExtractor::new(),
            force_metadata_refresh: false,
        }
    }

    /// Set whether to compute content hashes (default: true)
    pub fn compute_hashes(mut self, compute: bool) -> Self {
        self.compute_hashes = compute;
        self
    }

    /// Force re-extraction of metadata even for unchanged files
    pub fn force_metadata_refresh(mut self, force: bool) -> Self {
        self.force_metadata_refresh = force;
        self
    }

    /// Set progress callback
    pub fn on_progress(mut self, callback: ProgressCallback) -> Self {
        self.progress_callback = Some(callback);
        self
    }

    /// Scan all enabled library sources for this user/device
    pub async fn scan_all(&self) -> Result<ScanStats> {
        let sources =
            soul_storage::library_sources::get_enabled(&self.pool, &self.user_id, &self.device_id)
                .await?;

        let mut total_stats = ScanStats::default();

        for source in sources {
            match self.scan_source(&source).await {
                Ok(stats) => {
                    total_stats.total_files += stats.total_files;
                    total_stats.processed += stats.processed;
                    total_stats.new_files += stats.new_files;
                    total_stats.updated_files += stats.updated_files;
                    total_stats.removed_files += stats.removed_files;
                    total_stats.relocated_files += stats.relocated_files;
                    total_stats.errors += stats.errors;
                }
                Err(e) => {
                    tracing::error!("Failed to scan source {}: {}", source.name, e);
                    total_stats.errors += 1;
                }
            }
        }

        Ok(total_stats)
    }

    /// Scan a specific library source
    pub async fn scan_source(&self, source: &LibrarySource) -> Result<ScanStats> {
        let start_time = Instant::now();
        let source_path = Path::new(&source.path);

        // Verify path exists
        if !source_path.exists() {
            soul_storage::library_sources::set_scan_status(
                &self.pool,
                source.id,
                ScanStatus::Error,
                Some("Path does not exist"),
            )
            .await?;
            return Err(ImportError::FileNotFound(source.path.clone()));
        }

        // Set source status to scanning
        soul_storage::library_sources::set_scan_status(
            &self.pool,
            source.id,
            ScanStatus::Scanning,
            None,
        )
        .await?;

        // Start scan progress tracking
        let progress = soul_storage::scan_progress::start(&self.pool, source.id, None).await?;

        // Scan the directory
        let scanner = FileScanner::new();
        let files = match scanner.scan_directory(source_path) {
            Ok(files) => files,
            Err(e) => {
                soul_storage::scan_progress::fail(&self.pool, progress.id, &e.to_string()).await?;
                soul_storage::library_sources::set_scan_status(
                    &self.pool,
                    source.id,
                    ScanStatus::Error,
                    Some(&e.to_string()),
                )
                .await?;
                return Err(e);
            }
        };

        // Update total file count
        soul_storage::scan_progress::set_total_files(&self.pool, progress.id, files.len() as i64)
            .await?;

        let mut stats = ScanStats {
            total_files: files.len() as i64,
            ..Default::default()
        };

        // Get existing tracks for this source to detect changes
        let existing_tracks = self.get_existing_tracks_map(source.id).await?;
        let mut seen_paths: HashMap<String, bool> = HashMap::new();

        // Process each file
        for file_path in &files {
            let path_str = file_path.display().to_string();
            seen_paths.insert(path_str.clone(), true);

            match self
                .process_file(file_path, source.id, &existing_tracks)
                .await
            {
                Ok(action) => {
                    stats.processed += 1;
                    match action {
                        FileAction::New => {
                            stats.new_files += 1;
                            soul_storage::scan_progress::increment_new(&self.pool, progress.id, 1)
                                .await?;
                        }
                        FileAction::Updated => {
                            stats.updated_files += 1;
                            soul_storage::scan_progress::increment_updated(
                                &self.pool,
                                progress.id,
                                1,
                            )
                            .await?;
                        }
                        FileAction::Unchanged => {}
                        FileAction::Relocated => {
                            stats.relocated_files += 1;
                            stats.updated_files += 1;
                            soul_storage::scan_progress::increment_updated(
                                &self.pool,
                                progress.id,
                                1,
                            )
                            .await?;
                        }
                    }
                    soul_storage::scan_progress::increment_processed(&self.pool, progress.id, 1)
                        .await?;
                }
                Err(e) => {
                    tracing::warn!("Failed to process file {:?}: {}", file_path, e);
                    stats.errors += 1;
                    soul_storage::scan_progress::increment_errors(&self.pool, progress.id, 1)
                        .await?;
                }
            }

            // Call progress callback
            if let Some(ref callback) = self.progress_callback {
                callback(&stats);
            }
        }

        // Handle missing files (soft delete)
        if source.sync_deletes {
            let removed = self
                .mark_missing_files_unavailable(source.id, &seen_paths, &existing_tracks)
                .await?;
            stats.removed_files = removed;
            if removed > 0 {
                soul_storage::scan_progress::increment_removed(&self.pool, progress.id, removed)
                    .await?;
            }
        }

        // Complete the scan
        soul_storage::scan_progress::complete(&self.pool, progress.id).await?;

        // Update source status and last scan time
        let scan_time = chrono::Utc::now().timestamp();
        soul_storage::library_sources::set_last_scan_at(&self.pool, source.id, scan_time).await?;

        // Set scan status back to Idle
        soul_storage::library_sources::set_scan_status(
            &self.pool,
            source.id,
            ScanStatus::Idle,
            None,
        )
        .await?;

        tracing::info!(
            "Scan completed for {} in {:?}: {} new, {} updated, {} removed, {} errors",
            source.name,
            start_time.elapsed(),
            stats.new_files,
            stats.updated_files,
            stats.removed_files,
            stats.errors
        );

        Ok(stats)
    }

    /// Get a map of existing tracks for this source
    async fn get_existing_tracks_map(
        &self,
        source_id: i64,
    ) -> Result<HashMap<String, ExistingTrack>> {
        let tracks = soul_storage::tracks::get_by_library_source(&self.pool, source_id).await?;

        let mut map = HashMap::new();
        for track in tracks {
            if let Some(file_path) = track.file_path {
                map.insert(
                    file_path,
                    ExistingTrack {
                        id: track.id,
                        file_size: track.file_size,
                        file_mtime: track.file_mtime,
                        content_hash: track.content_hash,
                    },
                );
            }
        }

        Ok(map)
    }

    /// Process a single file
    async fn process_file(
        &self,
        file_path: &Path,
        source_id: i64,
        existing_tracks: &HashMap<String, ExistingTrack>,
    ) -> Result<FileAction> {
        let path_str = file_path.display().to_string();

        // Get file metadata
        let fs_meta = std::fs::metadata(file_path)?;
        let file_size = fs_meta.len() as i64;
        let file_mtime = fs_meta
            .modified()
            .map(|t| {
                t.duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0)
            })
            .unwrap_or(0);

        // Check if file exists in our database
        if let Some(existing) = existing_tracks.get(&path_str) {
            // File exists - check if it changed (or if we're forcing refresh)
            let unchanged =
                existing.file_size == Some(file_size) && existing.file_mtime == Some(file_mtime);

            if unchanged && !self.force_metadata_refresh {
                // Unchanged and no force refresh - skip
                return Ok(FileAction::Unchanged);
            }

            // File changed or force refresh - update metadata
            let processor =
                FileProcessor::new(&self.pool, &self.metadata_extractor, self.compute_hashes);
            processor
                .update_track_metadata(existing.id, file_path, file_size, file_mtime)
                .await?;
            return Ok(FileAction::Updated);
        }

        // File is new - check if it's a relocated file (by hash)
        let content_hash = if self.compute_hashes {
            Some(hash_computer::compute_file_hash(file_path).await?)
        } else {
            None
        };

        if let Some(ref hash) = content_hash {
            // Check if this hash exists elsewhere (file was moved)
            if let Some(track) = soul_storage::tracks::find_by_hash(&self.pool, hash).await? {
                // Update the track's path using the storage function
                soul_storage::tracks::update_file_path(
                    &self.pool,
                    track.id.as_str(),
                    &path_str,
                    source_id,
                    file_size,
                    file_mtime,
                )
                .await?;
                tracing::info!("Relocated track {} to {}", track.id, path_str);
                return Ok(FileAction::Relocated);
            }
        }

        // Truly new file - import it
        let processor =
            FileProcessor::new(&self.pool, &self.metadata_extractor, self.compute_hashes);
        processor
            .import_new_file(file_path, source_id, file_size, file_mtime, content_hash)
            .await?;
        Ok(FileAction::New)
    }

    /// Mark files that are no longer present as unavailable
    async fn mark_missing_files_unavailable(
        &self,
        _source_id: i64,
        seen_paths: &HashMap<String, bool>,
        existing_tracks: &HashMap<String, ExistingTrack>,
    ) -> Result<i64> {
        let mut removed_count = 0;

        for (file_path, track) in existing_tracks {
            if !seen_paths.contains_key(file_path) {
                // File not found in scan - mark as unavailable
                soul_storage::tracks::mark_unavailable(&self.pool, track.id).await?;
                tracing::debug!("Marked track {} as unavailable: {}", track.id, file_path);
                removed_count += 1;
            }
        }

        Ok(removed_count)
    }
}

/// Represents an existing track in the database
#[derive(Debug)]
struct ExistingTrack {
    id: i64,
    file_size: Option<i64>,
    file_mtime: Option<i64>,
    #[allow(dead_code)]
    content_hash: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_stats_default() {
        let stats = ScanStats::default();
        assert_eq!(stats.total_files, 0);
        assert_eq!(stats.processed, 0);
        assert_eq!(stats.new_files, 0);
    }
}
