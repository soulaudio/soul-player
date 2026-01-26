//! Main importer orchestration - brings together scanning, metadata, fuzzy matching, and copying

use crate::{
    copy, fuzzy::FuzzyMatcher, metadata, scanner::FileScanner, FileManagementStrategy,
    ImportConfig, ImportError, ImportProgress, ImportResult, ImportSummary, Result,
};
use sqlx::SqlitePool;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tokio::sync::mpsc;

/// Music importer orchestrator
pub struct MusicImporter {
    pool: SqlitePool,
    config: ImportConfig,
}

impl MusicImporter {
    /// Create a new music importer
    pub fn new(pool: SqlitePool, config: ImportConfig) -> Self {
        Self { pool, config }
    }

    /// Import files from a directory
    ///
    /// Returns a channel for receiving progress updates
    pub async fn import_directory(
        &self,
        directory: &Path,
    ) -> Result<(
        mpsc::Receiver<ImportProgress>,
        tokio::task::JoinHandle<Result<ImportSummary>>,
    )> {
        tracing::info!(directory = %directory.display(), "[Importer] Starting directory import");
        let scan_start = std::time::Instant::now();

        let scanner = FileScanner::new();
        let files = scanner.scan_directory(directory)?;

        let scan_duration = scan_start.elapsed();
        tracing::info!(
            directory = %directory.display(),
            file_count = files.len(),
            scan_duration_ms = scan_duration.as_millis(),
            "[Importer] Directory scan completed"
        );

        self.import_files(&files).await
    }

    /// Import specific files
    ///
    /// Returns a channel for receiving progress updates and a handle to the import task
    pub async fn import_files(
        &self,
        files: &[PathBuf],
    ) -> Result<(
        mpsc::Receiver<ImportProgress>,
        tokio::task::JoinHandle<Result<ImportSummary>>,
    )> {
        let (tx, rx) = mpsc::channel(100);

        let files = files.to_vec();
        let pool = self.pool.clone();
        let config = self.config.clone();
        let fuzzy_matcher = FuzzyMatcher::new();

        let handle = tokio::spawn(async move {
            Self::import_files_impl(files, pool, config, fuzzy_matcher, tx).await
        });

        Ok((rx, handle))
    }

    /// Internal implementation of file import
    async fn import_files_impl(
        files: Vec<PathBuf>,
        pool: SqlitePool,
        config: ImportConfig,
        fuzzy_matcher: FuzzyMatcher,
        progress_tx: mpsc::Sender<ImportProgress>,
    ) -> Result<ImportSummary> {
        tracing::info!(
            total_files = files.len(),
            strategy = ?config.file_strategy,
            skip_duplicates = config.skip_duplicates,
            "[Importer] Starting batch import"
        );
        let start_time = Instant::now();
        let total_files = files.len();

        let mut progress = ImportProgress::new(total_files);
        let mut require_review = Vec::new();
        let mut errors = Vec::new();

        // Send initial progress
        let _ = progress_tx.send(progress.clone()).await;

        for (idx, file_path) in files.iter().enumerate() {
            let file_start = std::time::Instant::now();
            progress.current_file = Some(file_path.clone());
            let _ = progress_tx.send(progress.clone()).await;

            tracing::debug!(
                file_path = %file_path.display(),
                progress = format!("{}/{}", idx + 1, total_files),
                "[Importer] Processing file"
            );

            match Self::import_single_file(file_path, &pool, &config, &fuzzy_matcher).await {
                Ok(result) => {
                    let file_duration = file_start.elapsed();
                    tracing::info!(
                        file_path = %file_path.display(),
                        requires_review = result.requires_review,
                        duration_ms = file_duration.as_millis(),
                        "[Importer] Successfully imported"
                    );

                    // Warn about slow imports
                    if file_duration.as_millis() > 5000 {
                        tracing::warn!(
                            file_path = %file_path.display(),
                            duration_ms = file_duration.as_millis(),
                            "[Importer] Slow file import detected"
                        );
                    }

                    if result.requires_review {
                        require_review.push(result);
                    }
                    progress.successful_imports += 1;
                }
                Err(ImportError::Duplicate(msg)) => {
                    tracing::debug!(
                        file_path = %file_path.display(),
                        message = %msg,
                        "[Importer] Skipping duplicate"
                    );
                    progress.skipped_duplicates += 1;
                }
                Err(e) => {
                    let file_duration = file_start.elapsed();
                    tracing::error!(
                        file_path = %file_path.display(),
                        error = %e,
                        duration_ms = file_duration.as_millis(),
                        "[Importer] Failed to import"
                    );
                    errors.push((file_path.clone(), e.to_string()));
                    progress.failed_imports += 1;
                }
            }

            progress.processed_files += 1;

            // Update estimated time remaining
            let elapsed = start_time.elapsed().as_secs();
            if progress.processed_files > 0 {
                let avg_per_file = elapsed / progress.processed_files as u64;
                let remaining_files = total_files - progress.processed_files;
                progress.estimated_seconds_remaining = Some(avg_per_file * remaining_files as u64);
            }

            let _ = progress_tx.send(progress.clone()).await;
        }

        let total_duration = start_time.elapsed();
        let avg_per_file = if progress.processed_files > 0 {
            total_duration.as_millis() / progress.processed_files as u128
        } else {
            0
        };

        tracing::info!(
            total_processed = progress.processed_files,
            successful = progress.successful_imports,
            duplicates = progress.skipped_duplicates,
            failed = progress.failed_imports,
            require_review = require_review.len(),
            total_duration_secs = total_duration.as_secs(),
            avg_per_file_ms = avg_per_file,
            "[Importer] Batch import completed"
        );

        Ok(ImportSummary {
            total_processed: progress.processed_files,
            successful: progress.successful_imports,
            duplicates_skipped: progress.skipped_duplicates,
            failed: progress.failed_imports,
            require_review,
            errors,
            duration_seconds: total_duration.as_secs(),
        })
    }

    /// Import a single file
    async fn import_single_file(
        file_path: &Path,
        pool: &SqlitePool,
        config: &ImportConfig,
        fuzzy_matcher: &FuzzyMatcher,
    ) -> Result<ImportResult> {
        // Extract metadata (wrap in spawn_blocking to avoid blocking async runtime)
        tracing::debug!(file_path = %file_path.display(), "[Importer] Extracting metadata");
        let metadata_start = std::time::Instant::now();
        let file_path_clone = file_path.to_path_buf();
        let metadata =
            tokio::task::spawn_blocking(move || metadata::extract_metadata(&file_path_clone))
                .await
                .map_err(|e| {
                    ImportError::Unknown(format!("Metadata extraction task failed: {}", e))
                })??;
        let metadata_duration = metadata_start.elapsed();

        if metadata_duration.as_millis() > 1000 {
            tracing::warn!(
                file_path = %file_path.display(),
                duration_ms = metadata_duration.as_millis(),
                "[Importer] Slow metadata extraction"
            );
        }

        // Calculate file hash for duplicate detection (wrap in spawn_blocking)
        tracing::debug!(file_path = %file_path.display(), "[Importer] Calculating file hash");
        let hash_start = std::time::Instant::now();
        let file_path_clone = file_path.to_path_buf();
        let file_hash =
            tokio::task::spawn_blocking(move || metadata::calculate_file_hash(&file_path_clone))
                .await
                .map_err(|e| {
                    ImportError::Unknown(format!("Hash calculation task failed: {}", e))
                })??;
        let hash_duration = hash_start.elapsed();

        tracing::debug!(
            metadata_ms = metadata_duration.as_millis(),
            hash_ms = hash_duration.as_millis(),
            "[Importer] Metadata and hash completed"
        );

        // Check for duplicates
        if config.skip_duplicates
            && (soul_storage::tracks::find_by_hash(pool, &file_hash).await?).is_some()
        {
            return Err(ImportError::Duplicate(format!(
                "File already exists: {}",
                file_path.display()
            )));
        }

        // Handle file according to strategy (move/copy/reference)
        tracing::debug!(
            file_path = ?file_path,
            strategy = ?config.file_strategy,
            library_path = ?config.library_path,
            "[Importer] Processing file"
        );

        let library_path = match config.file_strategy {
            FileManagementStrategy::Copy => {
                tracing::info!(
                    source = %file_path.display(),
                    "[Importer] COPY to library"
                );
                copy::copy_to_library(file_path, &config.library_path, &metadata)?
            }
            FileManagementStrategy::Move => {
                tracing::info!(
                    source = %file_path.display(),
                    "[Importer] MOVE to library"
                );
                copy::move_to_library(file_path, &config.library_path, &metadata)?
            }
            FileManagementStrategy::Reference => {
                tracing::info!(
                    file_path = %file_path.display(),
                    "[Importer] REFERENCE - keeping in original location"
                );
                // Keep file in original location - just reference it
                file_path.to_path_buf()
            }
        };

        tracing::debug!(
            result_path = ?library_path,
            "[Importer] File placed in library"
        );

        // Fuzzy match artist
        tracing::debug!("[Importer] Fuzzy matching artist/album/genres");
        let fuzzy_start = std::time::Instant::now();

        let artist_match = if let Some(artist_name) = &metadata.artist {
            Some(
                fuzzy_matcher
                    .find_or_create_artist(pool, artist_name)
                    .await?,
            )
        } else {
            None
        };

        // Fuzzy match album
        let album_match = if let Some(album_title) = &metadata.album {
            let artist_id = artist_match.as_ref().map(|m| m.entity.id);
            Some(
                fuzzy_matcher
                    .find_or_create_album(pool, album_title, artist_id)
                    .await?,
            )
        } else {
            None
        };

        // Fuzzy match genres
        let mut genre_matches = Vec::new();
        for genre_name in &metadata.genres {
            let genre_match = fuzzy_matcher.find_or_create_genre(pool, genre_name).await?;
            genre_matches.push(genre_match);
        }

        let fuzzy_duration = fuzzy_start.elapsed();
        tracing::debug!(
            duration_ms = fuzzy_duration.as_millis(),
            artist_matched = artist_match.is_some(),
            album_matched = album_match.is_some(),
            genre_count = genre_matches.len(),
            "[Importer] Fuzzy matching completed"
        );

        // Determine if review is required (any match below threshold)
        let requires_review = artist_match
            .as_ref()
            .map(|m| m.confidence < config.confidence_threshold)
            .unwrap_or(false)
            || album_match
                .as_ref()
                .map(|m| m.confidence < config.confidence_threshold)
                .unwrap_or(false)
            || genre_matches
                .iter()
                .any(|m| m.confidence < config.confidence_threshold);

        // Create track record in database
        use soul_core::types::CreateTrack;
        use soul_storage::tracks;

        // Create track struct
        let create_track = CreateTrack {
            title: metadata.title.clone().unwrap_or_else(|| {
                file_path
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "Unknown".to_string())
            }),
            artist_id: artist_match.as_ref().map(|m| m.entity.id),
            album_id: album_match.as_ref().map(|m| m.entity.id),
            album_artist_id: metadata.album_artist.as_ref().and_then(|_| {
                // If there's a separate album artist, try to match it
                // For now, just use the track artist if available
                artist_match.as_ref().map(|m| m.entity.id)
            }),
            track_number: metadata.track_number.map(|n| n as i32),
            disc_number: metadata.disc_number.map(|n| n as i32),
            year: metadata.year,
            duration_seconds: metadata.duration_seconds,
            bitrate: metadata.bitrate.map(|b| b as i32),
            sample_rate: metadata.sample_rate.map(|s| s as i32),
            channels: metadata.channels.map(|c| c as i32),
            file_format: library_path
                .extension()
                .map(|e| e.to_string_lossy().to_uppercase())
                .unwrap_or_else(|| "UNKNOWN".to_string()),
            file_hash: Some(file_hash.clone()),
            origin_source_id: 1, // Default local source
            local_file_path: Some(library_path.display().to_string()),
            musicbrainz_recording_id: None,
            fingerprint: None,
        };

        // Insert track into database
        tracing::debug!(
            title = %create_track.title,
            "[Importer] Inserting track into database"
        );
        let db_start = std::time::Instant::now();
        let created_track = tracks::create(pool, create_track).await?;

        // Insert track-genre relationships
        for genre_match in &genre_matches {
            soul_storage::genres::add_to_track(
                pool,
                created_track.id.clone(),
                genre_match.entity.id,
            )
            .await?;
        }

        let db_duration = db_start.elapsed();
        tracing::debug!(
            duration_ms = db_duration.as_millis(),
            track_id = %created_track.id,
            "[Importer] Database insertion completed"
        );

        Ok(ImportResult {
            track_id: 0, // Legacy field, track ID is now the string
            source_path: file_path.to_path_buf(),
            library_path,
            artist_match,
            album_match,
            genre_matches,
            requires_review,
            warnings: Vec::new(),
        })
    }
}
