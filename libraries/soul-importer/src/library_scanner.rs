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
    /// The most recently processed file path (display name only, for UI progress toast)
    pub current_file: Option<String>,
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
            concurrency: 8,
        }
    }

    /// Set whether to compute content hashes (default: true)
    pub fn compute_hashes(mut self, compute: bool) -> Self {
        self.compute_hashes = compute;
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

        // ── Directory-level incremental scan ──
        // Load stored directory mtimes from DB, then only scan directories
        // whose mtime has changed. Unchanged directories are skipped entirely.
        let stored_dirs_raw: Vec<soul_storage::scanned_directories::ScannedDirectory> =
            soul_storage::scanned_directories::get_by_source(&self.pool, source.id).await?;
        let stored_dirs: HashMap<String, crate::scanner::StoredDirInfo> = stored_dirs_raw
            .into_iter()
            .map(|d| {
                (
                    d.path.clone(),
                    crate::scanner::StoredDirInfo {
                        dir_mtime: d.dir_mtime,
                        file_count: d.file_count,
                    },
                )
            })
            .collect();

        let source_path_buf = source_path.to_path_buf();
        let scan_result = match tokio::task::spawn_blocking(move || {
            let scanner = FileScanner::new();
            scanner.scan_directory_incremental(&source_path_buf, &stored_dirs)
        })
        .await
        {
            Ok(Ok(result)) => result,
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

        let files = scan_result.changed_files;
        let scanned_dirs_to_persist = scan_result.scanned_dirs;

        // Get existing tracks early — needed for total count and seen_paths
        let existing_tracks = self.get_existing_tracks_map(source.id).await?;

        // Total files = sum of file_count across all scanned directories.
        // Each ScannedDirInfo has file_count: for unchanged dirs it's the stored count,
        // for changed dirs it's the freshly counted value.
        let total_files_in_library: i64 =
            scanned_dirs_to_persist.iter().map(|d| d.file_count).sum();

        // Update total file count
        soul_storage::scan_progress::set_total_files(
            &self.pool,
            progress.id,
            total_files_in_library,
        )
        .await?;

        let mut stats = ScanStats {
            total_files: total_files_in_library,
            ..Default::default()
        };

        // Emit initial progress so the frontend knows the total immediately.
        if let Some(ref callback) = self.progress_callback {
            callback(&stats);
        }

        // Collect ALL directories found during the scan (changed + unchanged).
        // Directories that were deleted from disk won't appear here.
        let all_scanned_dir_paths: std::collections::HashSet<String> = scanned_dirs_to_persist
            .iter()
            .map(|d| d.path.clone())
            .collect();
        // Directories that were rescanned (changed/new) — files here must be
        // re-confirmed on disk; they're NOT pre-added to seen_paths.
        let changed_dir_paths: std::collections::HashSet<String> = scanned_dirs_to_persist
            .iter()
            .filter(|d| d.changed)
            .map(|d| d.path.clone())
            .collect();

        let mut seen_paths: HashMap<String, bool> = HashMap::new();
        for file_path in existing_tracks.keys() {
            let parent = std::path::Path::new(file_path)
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();

            // Skip files whose parent directory no longer exists on disk
            if !all_scanned_dir_paths.contains(&parent) {
                continue;
            }
            // Skip files in changed directories — must be re-confirmed in Phase 1
            if changed_dir_paths.contains(&parent) {
                continue;
            }
            // File is in an unchanged directory that still exists — trust it
            seen_paths.insert(file_path.clone(), true);
        }

        // Count directory-level skips as processed (unchanged files)
        let dir_skipped_count = (total_files_in_library - files.len() as i64).max(0);
        if dir_skipped_count > 0 {
            stats.processed += dir_skipped_count;
            soul_storage::scan_progress::update_counts(
                &self.pool,
                progress.id,
                dir_skipped_count,
                0,
                0,
                0,
                0,
            )
            .await?;
        }

        // ── Phase 1: Filter unchanged files (cheap stat-only pass) ──
        // Separate files into unchanged (skip) and needs-processing buckets.
        // Wrapped in spawn_blocking to avoid blocking the Tokio runtime with
        // synchronous fs::metadata calls on potentially thousands of files.
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

                    if unchanged {
                        // Unchanged — skip
                        processed_count += 1;
                        skipped_count += 1;
                        continue;
                    }
                    // Changed — need to reprocess
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

        // Final flush of remaining batch counters and emit final progress.
        // Using flush_progress (instead of a bare update_counts) ensures the
        // progress_callback fires with the final stats for every scan, including
        // those with < 10 files that never hit the every-10-files flush above.
        self.flush_progress(
            progress.id,
            &mut batch_processed,
            &mut batch_new,
            &mut batch_updated,
            &mut batch_errors,
            &stats,
        )
        .await?;

        // ── Artwork refresh for changed directories ──
        // When only an image is added/changed in a directory, audio files pass Phase 1
        // unchanged and are skipped — file_processor never runs, cover_art_path stays stale.
        // Re-run artwork discovery for every album whose folder_path is in a changed dir.
        self.refresh_artwork_for_changed_dirs(&changed_dir_paths)
            .await?;

        // Handle missing files (soft delete) and clean up orphaned albums/artists
        if source.sync_deletes {
            let removed = self
                .mark_missing_files_unavailable(source.id, &seen_paths, &existing_tracks)
                .await?;
            stats.removed_files = removed;
            if removed > 0 {
                soul_storage::scan_progress::increment_removed(&self.pool, progress.id, removed)
                    .await?;

                // Clean up albums and artists that no longer have any available tracks
                let orphaned_albums = soul_storage::albums::delete_orphaned(&self.pool).await?;
                let orphaned_artists = soul_storage::artists::delete_orphaned(&self.pool).await?;
                if orphaned_albums > 0 || orphaned_artists > 0 {
                    tracing::info!(
                        "[SCAN] Cleaned up {} orphaned albums, {} orphaned artists",
                        orphaned_albums,
                        orphaned_artists
                    );
                }
            }
        }

        // Persist updated directory mtimes for next incremental scan
        let dirs_batch: Vec<(String, i64, i64)> = scanned_dirs_to_persist
            .into_iter()
            .map(|d| (d.path, d.dir_mtime, d.file_count))
            .collect();
        soul_storage::scanned_directories::upsert_batch(&self.pool, source.id, &dirs_batch).await?;

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
                stats.current_file = file_path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned());
            }
            Err((file_path, e)) => {
                tracing::warn!("Metadata extraction failed for {:?}: {}", file_path, e);
                stats.errors += 1;
                stats.processed += 1;
                *batch_processed += 1;
                *batch_errors += 1;
                stats.current_file = file_path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned());
            }
        }

        // Call progress callback per file (lightweight — callback only emits an event,
        // DB flush happens separately on a batched schedule)
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
                    None,
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
    /// Re-run artwork discovery for albums in directories that changed during this scan.
    ///
    /// Covers the case where only an image file was added/modified — audio files pass
    /// Phase 1's mtime/size check unchanged, so file_processor never runs for them.
    /// This pass updates cover_art_path for any album whose folder changed.
    async fn refresh_artwork_for_changed_dirs(
        &self,
        changed_dirs: &std::collections::HashSet<String>,
    ) -> Result<()> {
        if changed_dirs.is_empty() {
            return Ok(());
        }

        // Fetch all albums that have a folder_path (non-NULL)
        let rows: Vec<(i64, String, Option<String>)> = sqlx::query_as(
            "SELECT id, folder_path, cover_art_path FROM albums WHERE folder_path IS NOT NULL",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut updated: i64 = 0;
        for (album_id, folder_str, current_cover) in rows {
            if !changed_dirs.contains(&folder_str) {
                continue;
            }
            let new_cover = crate::artwork_discovery::discover_folder_artwork(
                std::path::Path::new(&folder_str),
            )
            .map(|p| p.to_string_lossy().to_string());

            if new_cover != current_cover {
                soul_storage::albums::update_cover_art_path(
                    &self.pool,
                    album_id,
                    new_cover.as_deref(),
                )
                .await?;
                updated += 1;
                tracing::debug!(
                    "[SCAN] Artwork refreshed for album_id={} folder={}",
                    album_id,
                    folder_str
                );
            }
        }

        if updated > 0 {
            tracing::info!(
                "[SCAN] Refreshed artwork for {} album(s) in changed directories",
                updated
            );
        }
        Ok(())
    }

    async fn mark_missing_files_unavailable(
        &self,
        _source_id: i64,
        seen_paths: &HashMap<String, bool>,
        existing_tracks: &HashMap<String, ExistingTrack>,
    ) -> Result<i64> {
        let mut removed_count = 0;

        tracing::debug!(
            "[SCAN] mark_missing_files_unavailable: {} existing tracks, {} seen paths",
            existing_tracks.len(),
            seen_paths.len()
        );

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
