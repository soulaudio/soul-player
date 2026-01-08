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
        let scanner = FileScanner::new();
        let files = scanner.scan_directory(directory)?;
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
        let start_time = Instant::now();
        let total_files = files.len();

        let mut progress = ImportProgress::new(total_files);
        let mut require_review = Vec::new();
        let mut errors = Vec::new();

        // Send initial progress
        let _ = progress_tx.send(progress.clone()).await;

        for file_path in files {
            progress.current_file = Some(file_path.clone());
            let _ = progress_tx.send(progress.clone()).await;

            match Self::import_single_file(&file_path, &pool, &config, &fuzzy_matcher).await {
                Ok(result) => {
                    eprintln!("[Importer] Successfully imported: {:?}", file_path);
                    if result.requires_review {
                        require_review.push(result);
                    }
                    progress.successful_imports += 1;
                }
                Err(ImportError::Duplicate(msg)) => {
                    eprintln!("[Importer] Skipping duplicate: {}", msg);
                    tracing::debug!("Skipping duplicate: {}", msg);
                    progress.skipped_duplicates += 1;
                }
                Err(e) => {
                    eprintln!("[Importer] FAILED to import {:?}: {}", file_path, e);
                    tracing::error!("Failed to import {:?}: {}", file_path, e);
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

        Ok(ImportSummary {
            total_processed: progress.processed_files,
            successful: progress.successful_imports,
            duplicates_skipped: progress.skipped_duplicates,
            failed: progress.failed_imports,
            require_review,
            errors,
            duration_seconds: start_time.elapsed().as_secs(),
        })
    }

    /// Import a single file
    async fn import_single_file(
        file_path: &Path,
        pool: &SqlitePool,
        config: &ImportConfig,
        fuzzy_matcher: &FuzzyMatcher,
    ) -> Result<ImportResult> {
        // Extract metadata
        let metadata = metadata::extract_metadata(file_path)?;

        // Calculate file hash for duplicate detection
        let file_hash = metadata::calculate_file_hash(file_path)?;

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
        eprintln!("[Importer] Processing: {:?}", file_path);
        eprintln!("[Importer] Strategy: {:?}", config.file_strategy);
        eprintln!("[Importer] Library path: {:?}", config.library_path);

        let library_path = match config.file_strategy {
            FileManagementStrategy::Copy => {
                eprintln!("[Importer] COPY: {} -> library", file_path.display());
                // NOTE: Calling move_to_library for Copy strategy because the implementations are swapped
                copy::move_to_library(file_path, &config.library_path, &metadata)?
            }
            FileManagementStrategy::Move => {
                eprintln!("[Importer] MOVE: {} -> library", file_path.display());
                // NOTE: Calling copy_to_library for Move strategy because the implementations are swapped
                copy::copy_to_library(file_path, &config.library_path, &metadata)?
            }
            FileManagementStrategy::Reference => {
                eprintln!("[Importer] REFERENCE: Keeping at {}", file_path.display());
                // Keep file in original location - just reference it
                file_path.to_path_buf()
            }
        };

        eprintln!("[Importer] Result path: {:?}", library_path);

        // Fuzzy match artist
        let artist_match = if let Some(ref artist_name) = metadata.artist {
            Some(
                fuzzy_matcher
                    .find_or_create_artist(pool, artist_name)
                    .await?,
            )
        } else {
            None
        };

        // Fuzzy match album
        let album_match = if let Some(ref album_title) = metadata.album {
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
                    .and_then(|s| s.to_str())
                    .unwrap_or("Unknown")
                    .to_string()
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
                .and_then(|e| e.to_str())
                .unwrap_or("unknown")
                .to_uppercase(),
            file_hash: Some(file_hash.clone()),
            origin_source_id: 1, // Default local source
            local_file_path: Some(library_path.display().to_string()),
            musicbrainz_recording_id: None,
            fingerprint: None,
        };

        // Insert track into database
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
