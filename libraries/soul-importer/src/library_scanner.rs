//! Library scanner for watched folders
//!
//! Scans library sources (watched folders) and synchronizes with the database.
//! Uses mtime + size for change detection and content hash for file relocation.

use crate::{fuzzy::FuzzyMatcher, metadata, scanner::FileScanner, ImportError, Result};
use soul_core::types::{CreateTrack, LibrarySource, ScanStatus};
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
    /// Fuzzy matcher for artist/album/genre matching
    fuzzy_matcher: FuzzyMatcher,
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
            fuzzy_matcher: FuzzyMatcher::new(),
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

        tracing::info!(
            "Scan completed for {} in {:?}: {} new, {} updated, {} removed",
            source.name,
            start_time.elapsed(),
            stats.new_files,
            stats.updated_files,
            stats.removed_files
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
            self.update_track_metadata(existing.id, file_path, file_size, file_mtime)
                .await?;
            return Ok(FileAction::Updated);
        }

        // File is new - check if it's a relocated file (by hash)
        let content_hash = if self.compute_hashes {
            Some(metadata::calculate_file_hash(file_path)?)
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
        self.import_new_file(file_path, source_id, file_size, file_mtime, content_hash)
            .await?;
        Ok(FileAction::New)
    }

    /// Import a new file into the library
    async fn import_new_file(
        &self,
        file_path: &Path,
        source_id: i64,
        file_size: i64,
        file_mtime: i64,
        content_hash: Option<String>,
    ) -> Result<()> {
        // Extract metadata
        let meta = metadata::extract_metadata(file_path)?;
        tracing::info!(
            "import_new_file: file={}, artist={:?}, album={:?}",
            file_path.display(),
            meta.artist,
            meta.album
        );

        // Fuzzy match artist
        let artist_id = if let Some(ref artist_name) = meta.artist {
            let artist_match = self
                .fuzzy_matcher
                .find_or_create_artist(&self.pool, artist_name)
                .await?;
            Some(artist_match.entity.id)
        } else {
            None
        };

        // Fuzzy match album (linked to artist if available)
        let album_id = if let Some(ref album_title) = meta.album {
            let album_match = self
                .fuzzy_matcher
                .find_or_create_album(&self.pool, album_title, artist_id)
                .await?;
            Some(album_match.entity.id)
        } else {
            None
        };

        // Fuzzy match album artist (if different from track artist)
        let album_artist_id = if let Some(ref album_artist_name) = meta.album_artist {
            // Only create separate album artist if different from track artist
            if meta.artist.as_ref() != Some(album_artist_name) {
                let artist_match = self
                    .fuzzy_matcher
                    .find_or_create_artist(&self.pool, album_artist_name)
                    .await?;
                Some(artist_match.entity.id)
            } else {
                artist_id
            }
        } else {
            None
        };

        // Create the track
        let create_track = CreateTrack {
            title: meta.title.clone().unwrap_or_else(|| {
                file_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Unknown")
                    .to_string()
            }),
            artist_id,
            album_id,
            album_artist_id,
            track_number: meta.track_number.map(|n| n as i32),
            disc_number: meta.disc_number.map(|n| n as i32),
            year: meta.year,
            duration_seconds: meta.duration_seconds,
            bitrate: meta.bitrate.map(|b| b as i32),
            sample_rate: meta.sample_rate.map(|s| s as i32),
            channels: meta.channels.map(|c| c as i32),
            file_format: meta.file_format.to_uppercase(),
            file_hash: content_hash,
            origin_source_id: 1, // Default local source
            local_file_path: Some(file_path.display().to_string()),
            musicbrainz_recording_id: meta.musicbrainz_recording_id.clone(),
            fingerprint: None,
        };

        let track = soul_storage::tracks::create(&self.pool, create_track).await?;

        // Parse track ID to i64 for the storage function
        let track_id: i64 = track
            .id
            .as_str()
            .parse()
            .map_err(|_| ImportError::Unknown(format!("Invalid track ID: {}", track.id)))?;

        // Update library-specific fields
        soul_storage::tracks::set_library_source(
            &self.pool, track_id, source_id, file_size, file_mtime,
        )
        .await?;

        // Add genres to track
        let track_id_typed = soul_core::types::TrackId::new(track_id.to_string());
        for genre_name in &meta.genres {
            let genre_match = self
                .fuzzy_matcher
                .find_or_create_genre(&self.pool, genre_name)
                .await?;
            soul_storage::genres::add_to_track(&self.pool, track_id_typed.clone(), genre_match.entity.id).await?;
        }

        Ok(())
    }

    /// Update track metadata after file change
    async fn update_track_metadata(
        &self,
        track_id: i64,
        file_path: &Path,
        file_size: i64,
        file_mtime: i64,
    ) -> Result<()> {
        // Re-extract metadata
        let meta = metadata::extract_metadata(file_path)?;
        tracing::info!(
            "update_track_metadata: file={}, artist={:?}, album={:?}",
            file_path.display(),
            meta.artist,
            meta.album
        );
        let content_hash = if self.compute_hashes {
            Some(metadata::calculate_file_hash(file_path)?)
        } else {
            None
        };

        // Fuzzy match artist
        let artist_id = if let Some(ref artist_name) = meta.artist {
            let artist_match = self
                .fuzzy_matcher
                .find_or_create_artist(&self.pool, artist_name)
                .await?;
            Some(artist_match.entity.id)
        } else {
            None
        };

        // Fuzzy match album (linked to artist if available)
        let album_id = if let Some(ref album_title) = meta.album {
            let album_match = self
                .fuzzy_matcher
                .find_or_create_album(&self.pool, album_title, artist_id)
                .await?;
            Some(album_match.entity.id)
        } else {
            None
        };

        // Update the track using the storage function
        soul_storage::tracks::update_file_metadata(
            &self.pool,
            track_id,
            meta.title.as_deref(),
            meta.track_number,
            meta.disc_number,
            meta.year,
            meta.duration_seconds,
            meta.bitrate,
            meta.sample_rate,
            meta.channels,
            &meta.file_format,
            file_size,
            file_mtime,
            content_hash.as_deref(),
        )
        .await?;

        // Update artist/album relationships if we have them
        if artist_id.is_some() || album_id.is_some() {
            soul_storage::tracks::update_artist_album(
                &self.pool,
                track_id,
                artist_id,
                album_id,
            )
            .await?;
        }

        Ok(())
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

/// Action taken for a file during scanning
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileAction {
    /// File is new, was imported
    New,
    /// File existed and was updated
    Updated,
    /// File existed and was unchanged
    Unchanged,
    /// File was relocated (same hash, different path)
    Relocated,
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

    #[test]
    fn test_file_action_equality() {
        assert_eq!(FileAction::New, FileAction::New);
        assert_ne!(FileAction::New, FileAction::Updated);
    }
}
