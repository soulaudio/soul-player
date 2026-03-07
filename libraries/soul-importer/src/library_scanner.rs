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
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
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
    /// Maximum number of concurrent metadata extraction tasks
    concurrency: usize,
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
            concurrency: 8,
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

    /// Set maximum number of concurrent metadata extraction tasks (default: 8)
    pub fn concurrency(mut self, max: usize) -> Self {
        self.concurrency = max.max(1);
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

        // Verify path exists (async to avoid blocking on network/slow storage)
        if !tokio::fs::try_exists(source_path).await.unwrap_or(false) {
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

        // Scan the directory (wrap blocking WalkDir in spawn_blocking to avoid macOS spinning wheel)
        let source_path_buf = source_path.to_path_buf();
        let files = match tokio::task::spawn_blocking(move || {
            let scanner = FileScanner::new();
            scanner.scan_directory(&source_path_buf)
        })
        .await
        {
            Ok(Ok(files)) => files,
            Ok(Err(e)) => {
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
            Err(e) => {
                let err_msg = format!("Directory scan task panicked: {}", e);
                soul_storage::scan_progress::fail(&self.pool, progress.id, &err_msg).await?;
                soul_storage::library_sources::set_scan_status(
                    &self.pool,
                    source.id,
                    ScanStatus::Error,
                    Some(&err_msg),
                )
                .await?;
                return Err(ImportError::Unknown(err_msg));
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

        // ── Phase 1: Filter unchanged files (cheap stat-only pass) ──
        // Separate files into unchanged (skip) and needs-processing buckets.
        // Wrapped in spawn_blocking to avoid blocking the Tokio runtime with
        // synchronous fs::metadata calls on potentially thousands of files.
        let force_refresh = self.force_metadata_refresh;
        let phase1_result = tokio::task::spawn_blocking(move || {
            let mut files_to_process: Vec<(PathBuf, i64, i64, Option<ExistingTrack>)> = Vec::new();
            let mut skipped_count: i64 = 0;
            let mut error_count: i64 = 0;
            let mut processed_count: i64 = 0;

            for file_path in &files {
                let path_str = file_path.display().to_string();
                seen_paths.insert(path_str.clone(), true);

                // Get file metadata (mtime + size) for change detection
                let fs_meta = match std::fs::metadata(file_path) {
                    Ok(m) => m,
                    Err(e) => {
                        tracing::warn!("Failed to stat file {:?}: {}", file_path, e);
                        error_count += 1;
                        processed_count += 1;
                        skipped_count += 1;
                        continue;
                    }
                };
                let file_size = fs_meta.len() as i64;
                let file_mtime = fs_meta
                    .modified()
                    .map(|t| {
                        t.duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_secs() as i64)
                            .unwrap_or(0)
                    })
                    .unwrap_or(0);

                if let Some(existing) = existing_tracks.get(&path_str) {
                    let unchanged = existing.file_size == Some(file_size)
                        && existing.file_mtime == Some(file_mtime);

                    if unchanged && !force_refresh {
                        // Unchanged — skip
                        processed_count += 1;
                        skipped_count += 1;
                        continue;
                    }
                    // Changed or force-refresh — need to reprocess
                    files_to_process.push((
                        file_path.clone(),
                        file_size,
                        file_mtime,
                        Some(existing.clone()),
                    ));
                } else {
                    // New file
                    files_to_process.push((file_path.clone(), file_size, file_mtime, None));
                }
            }
            (
                files_to_process,
                seen_paths,
                existing_tracks,
                skipped_count,
                error_count,
                processed_count,
            )
        })
        .await
        .map_err(|e| ImportError::Unknown(format!("Phase 1 filter task panicked: {}", e)))?;

        let (
            files_to_process,
            seen_paths,
            existing_tracks,
            skipped_count,
            phase1_errors,
            phase1_processed,
        ) = phase1_result;
        stats.errors += phase1_errors;
        stats.processed += phase1_processed;

        // Flush progress for all skipped files in one batch
        if skipped_count > 0 {
            soul_storage::scan_progress::update_counts(
                &self.pool,
                progress.id,
                skipped_count,
                0,
                0,
                0,
                0,
            )
            .await?;
        }

        // ── Phase 2: Parallel metadata extraction → sequential DB writes ──
        // Spawn up to `concurrency` extraction tasks ahead of the consumer.
        // The consumer awaits the oldest task, writes to DB, then loops.
        // This keeps N extractions in-flight while the DB write is happening.

        let max_inflight = self.concurrency;
        let mut inflight: VecDeque<tokio::task::JoinHandle<ExtractionResult>> = VecDeque::new();

        // Preload entity cache once for the entire scan
        let mut cache = crate::fuzzy::EntityCache::preload(&self.pool).await?;

        let mut batch_processed: i64 = 0;
        let mut batch_new: i64 = 0;
        let mut batch_updated: i64 = 0;
        let mut batch_errors: i64 = 0;
        let mut total_phase2_processed: usize = 0;

        let files_to_process_len = files_to_process.len();

        for (file_path, file_size, file_mtime, existing) in files_to_process {
            // Spawn metadata extraction task
            let fp = file_path.clone();
            let handle: tokio::task::JoinHandle<ExtractionResult> = tokio::spawn(async move {
                let extractor = MetadataExtractor::new();
                match extractor.extract_metadata(&fp).await {
                    Ok(raw) => Ok((file_path, file_size, file_mtime, existing, raw)),
                    Err(e) => Err((file_path, e)),
                }
            });
            inflight.push_back(handle);

            // When buffer is full, consume the oldest result
            while inflight.len() >= max_inflight {
                if let Some(handle) = inflight.pop_front() {
                    self.process_extraction_result(
                        handle,
                        source.id,
                        &mut cache,
                        &mut stats,
                        &mut batch_processed,
                        &mut batch_new,
                        &mut batch_updated,
                        &mut batch_errors,
                    )
                    .await?;

                    total_phase2_processed += 1;

                    // Flush progress every 10 files
                    if total_phase2_processed % 10 == 0 {
                        self.flush_progress(
                            progress.id,
                            &mut batch_processed,
                            &mut batch_new,
                            &mut batch_updated,
                            &mut batch_errors,
                            &stats,
                        )
                        .await?;
                    }
                }
            }
        }

        // Drain remaining in-flight tasks
        while let Some(handle) = inflight.pop_front() {
            self.process_extraction_result(
                handle,
                source.id,
                &mut cache,
                &mut stats,
                &mut batch_processed,
                &mut batch_new,
                &mut batch_updated,
                &mut batch_errors,
            )
            .await?;

            total_phase2_processed += 1;

            if total_phase2_processed % 10 == 0 && total_phase2_processed < files_to_process_len {
                self.flush_progress(
                    progress.id,
                    &mut batch_processed,
                    &mut batch_new,
                    &mut batch_updated,
                    &mut batch_errors,
                    &stats,
                )
                .await?;
            }
        }

        // Final flush of remaining batch counters
        if batch_processed > 0 || batch_errors > 0 {
            soul_storage::scan_progress::update_counts(
                &self.pool,
                progress.id,
                batch_processed,
                batch_new,
                batch_updated,
                0,
                batch_errors,
            )
            .await?;
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

    /// Process the result of a single metadata extraction task.
    /// Handles entity matching and DB writes sequentially.
    #[allow(clippy::too_many_arguments)]
    async fn process_extraction_result(
        &self,
        handle: tokio::task::JoinHandle<ExtractionResult>,
        source_id: i64,
        cache: &mut crate::fuzzy::EntityCache,
        stats: &mut ScanStats,
        batch_processed: &mut i64,
        batch_new: &mut i64,
        batch_updated: &mut i64,
        batch_errors: &mut i64,
    ) -> Result<()> {
        let result = match handle.await {
            Ok(inner) => inner,
            Err(e) => {
                tracing::warn!("Extraction task panicked: {}", e);
                stats.errors += 1;
                stats.processed += 1;
                *batch_processed += 1;
                *batch_errors += 1;
                return Ok(());
            }
        };

        match result {
            Ok((file_path, file_size, file_mtime, existing, raw)) => {
                // Sequential: entity matching (using cache) + DB write
                let action = self
                    .process_extracted_file(
                        &file_path, source_id, file_size, file_mtime, existing, raw, cache,
                    )
                    .await;

                match action {
                    Ok(FileAction::New) => {
                        stats.new_files += 1;
                        *batch_new += 1;
                    }
                    Ok(FileAction::Updated) => {
                        stats.updated_files += 1;
                        *batch_updated += 1;
                    }
                    Ok(FileAction::Relocated) => {
                        stats.relocated_files += 1;
                        stats.updated_files += 1;
                        *batch_updated += 1;
                    }
                    Ok(FileAction::Unchanged) => {}
                    Err(e) => {
                        tracing::warn!("Failed to process file {:?}: {}", file_path, e);
                        stats.errors += 1;
                        *batch_errors += 1;
                    }
                }
                stats.processed += 1;
                *batch_processed += 1;
            }
            Err((file_path, e)) => {
                tracing::warn!("Metadata extraction failed for {:?}: {}", file_path, e);
                stats.errors += 1;
                stats.processed += 1;
                *batch_processed += 1;
                *batch_errors += 1;
            }
        }

        // Call progress callback
        if let Some(ref callback) = self.progress_callback {
            callback(stats);
        }

        Ok(())
    }

    /// Process a file with pre-extracted metadata: entity matching + DB write.
    #[allow(clippy::too_many_arguments)]
    async fn process_extracted_file(
        &self,
        file_path: &Path,
        source_id: i64,
        file_size: i64,
        file_mtime: i64,
        existing: Option<ExistingTrack>,
        raw: crate::metadata::ExtractedMetadata,
        cache: &mut crate::fuzzy::EntityCache,
    ) -> Result<FileAction> {
        if let Some(existing) = existing {
            // Existing track — update metadata
            let processor =
                FileProcessor::new(&self.pool, &self.metadata_extractor, self.compute_hashes);
            processor
                .update_track_with_metadata(
                    existing.id,
                    file_path,
                    file_size,
                    file_mtime,
                    raw,
                    cache,
                )
                .await?;
            return Ok(FileAction::Updated);
        }

        // New file — check for relocation by hash
        let content_hash = if self.compute_hashes {
            Some(hash_computer::compute_file_hash(file_path).await?)
        } else {
            None
        };

        if let Some(ref hash) = content_hash {
            if let Some(track) = soul_storage::tracks::find_by_hash(&self.pool, hash).await? {
                let path_str = file_path.display().to_string();
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

        // Truly new file — import with pre-extracted metadata
        let processor =
            FileProcessor::new(&self.pool, &self.metadata_extractor, self.compute_hashes);
        processor
            .import_with_metadata(
                file_path,
                source_id,
                file_size,
                file_mtime,
                content_hash,
                raw,
                cache,
            )
            .await?;
        Ok(FileAction::New)
    }

    /// Flush batched progress counters to the database and invoke callback.
    async fn flush_progress(
        &self,
        progress_id: i64,
        batch_processed: &mut i64,
        batch_new: &mut i64,
        batch_updated: &mut i64,
        batch_errors: &mut i64,
        stats: &ScanStats,
    ) -> Result<()> {
        if *batch_processed > 0 || *batch_errors > 0 {
            soul_storage::scan_progress::update_counts(
                &self.pool,
                progress_id,
                *batch_processed,
                *batch_new,
                *batch_updated,
                0,
                *batch_errors,
            )
            .await?;
            *batch_processed = 0;
            *batch_new = 0;
            *batch_updated = 0;
            *batch_errors = 0;
        }

        if let Some(ref callback) = self.progress_callback {
            callback(stats);
        }

        Ok(())
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

/// Result type for parallel metadata extraction tasks.
type ExtractionResult = std::result::Result<
    (
        PathBuf,
        i64,
        i64,
        Option<ExistingTrack>,
        crate::metadata::ExtractedMetadata,
    ),
    (PathBuf, crate::ImportError),
>;

/// Represents an existing track in the database
#[derive(Debug, Clone)]
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
