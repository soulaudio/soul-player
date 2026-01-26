//! File processing for library scanning
//!
//! This module handles the processing of individual audio files during library
//! scanning, including importing new files and updating existing track metadata.

use crate::{
    artwork_discovery, hash_computer, metadata_extractor::MetadataExtractor, ImportError, Result,
};
use soul_core::types::{CreateTrack, TrackId};
use sqlx::SqlitePool;
use std::path::Path;

/// Action taken for a file during scanning
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileAction {
    /// File is new, was imported
    New,
    /// File existed and was updated
    Updated,
    /// File existed and was unchanged
    Unchanged,
    /// File was relocated (same hash, different path)
    Relocated,
}

/// Context for file processing operations
pub struct FileProcessor<'a> {
    pool: &'a SqlitePool,
    metadata_extractor: &'a MetadataExtractor,
    compute_hashes: bool,
}

impl<'a> FileProcessor<'a> {
    /// Create a new file processor
    pub fn new(
        pool: &'a SqlitePool,
        metadata_extractor: &'a MetadataExtractor,
        compute_hashes: bool,
    ) -> Self {
        Self {
            pool,
            metadata_extractor,
            compute_hashes,
        }
    }

    /// Import a new file into the library
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the audio file
    /// * `source_id` - Library source ID
    /// * `file_size` - File size in bytes
    /// * `file_mtime` - File modification time (Unix timestamp)
    /// * `content_hash` - Optional content hash for deduplication
    ///
    /// # Errors
    ///
    /// Returns an error if metadata extraction or database operations fail
    pub async fn import_new_file(
        &self,
        file_path: &Path,
        source_id: i64,
        file_size: i64,
        file_mtime: i64,
        content_hash: Option<String>,
    ) -> Result<()> {
        tracing::info!("[IMPORT] Importing new file: {}", file_path.display());

        // Extract metadata and match entities
        let processed = self
            .metadata_extractor
            .extract_and_match(self.pool, file_path)
            .await?;

        // Create the track
        let create_track = CreateTrack {
            title: processed.raw.title.clone().unwrap_or_else(|| {
                file_path
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "Unknown".to_string())
            }),
            artist_id: processed.artist_id,
            album_id: processed.album_id,
            album_artist_id: processed.album_artist_id,
            track_number: processed.raw.track_number.map(|n| n as i32),
            disc_number: processed.raw.disc_number.map(|n| n as i32),
            year: processed.raw.year,
            duration_seconds: processed.raw.duration_seconds,
            bitrate: processed.raw.bitrate.map(|b| b as i32),
            sample_rate: processed.raw.sample_rate.map(|s| s as i32),
            channels: processed.raw.channels.map(|c| c as i32),
            file_format: processed.raw.file_format.to_uppercase(),
            file_hash: content_hash,
            origin_source_id: 1, // Default local source
            local_file_path: Some(file_path.display().to_string()),
            musicbrainz_recording_id: processed.raw.musicbrainz_recording_id.clone(),
            fingerprint: None,
        };

        let track = soul_storage::tracks::create(self.pool, create_track).await?;

        // Parse track ID to i64 for the storage function
        let track_id: i64 = track
            .id
            .as_str()
            .parse()
            .map_err(|_| ImportError::Unknown(format!("Invalid track ID: {}", track.id)))?;

        // Update library-specific fields
        soul_storage::tracks::set_library_source(
            self.pool, track_id, source_id, file_size, file_mtime,
        )
        .await?;

        // Add genres to track
        let track_id_typed = TrackId::new(track_id.to_string());
        self.metadata_extractor
            .add_genres_to_track(self.pool, track_id_typed, &processed.genre_ids)
            .await?;

        // Discover folder artwork if album exists (wrap in spawn_blocking to avoid blocking async runtime)
        if let Some(album_id) = processed.album_id {
            if let Some(folder_path) = file_path.parent() {
                let folder_path_buf = folder_path.to_path_buf();
                let artwork_path = tokio::task::spawn_blocking(move || {
                    artwork_discovery::discover_folder_artwork(&folder_path_buf)
                })
                .await
                .unwrap_or(None);

                if let Some(artwork_path) = artwork_path {
                    // Record in database that this album has folder artwork
                    soul_storage::albums::set_artwork_source(
                        self.pool,
                        album_id,
                        "folder",
                        &artwork_path.to_string_lossy(),
                    )
                    .await
                    .ok(); // Ignore errors - artwork is optional
                }
            }
        }

        tracing::info!(
            "[IMPORT] Successfully imported: {} (track_id: {})",
            file_path.display(),
            track_id
        );

        Ok(())
    }

    /// Update track metadata after file change
    ///
    /// # Arguments
    ///
    /// * `track_id` - Database track ID
    /// * `file_path` - Path to the audio file
    /// * `file_size` - File size in bytes
    /// * `file_mtime` - File modification time (Unix timestamp)
    ///
    /// # Errors
    ///
    /// Returns an error if metadata extraction or database operations fail
    pub async fn update_track_metadata(
        &self,
        track_id: i64,
        file_path: &Path,
        file_size: i64,
        file_mtime: i64,
    ) -> Result<()> {
        tracing::info!(
            "[UPDATE] Updating metadata for track {}: {}",
            track_id,
            file_path.display()
        );

        // Extract metadata and match entities
        let processed = self
            .metadata_extractor
            .extract_and_match(self.pool, file_path)
            .await?;

        // Compute content hash if enabled
        let content_hash = if self.compute_hashes {
            Some(hash_computer::compute_file_hash(file_path).await?)
        } else {
            None
        };

        // Update the track using the storage function
        soul_storage::tracks::update_file_metadata(
            self.pool,
            track_id,
            processed.raw.title.as_deref(),
            processed.raw.track_number,
            processed.raw.disc_number,
            processed.raw.year,
            processed.raw.duration_seconds,
            processed.raw.bitrate,
            processed.raw.sample_rate,
            processed.raw.channels,
            &processed.raw.file_format,
            file_size,
            file_mtime,
            content_hash.as_deref(),
        )
        .await?;

        // Update artist/album relationships if we have them
        if processed.artist_id.is_some() || processed.album_id.is_some() {
            soul_storage::tracks::update_artist_album(
                self.pool,
                track_id,
                processed.artist_id,
                processed.album_id,
            )
            .await?;
        }

        // Discover folder artwork if album exists (same as on import, wrap in spawn_blocking to avoid blocking async runtime)
        if let Some(album_id) = processed.album_id {
            if let Some(folder_path) = file_path.parent() {
                let folder_path_buf = folder_path.to_path_buf();
                let artwork_path = tokio::task::spawn_blocking(move || {
                    artwork_discovery::discover_folder_artwork(&folder_path_buf)
                })
                .await
                .unwrap_or(None);

                if let Some(artwork_path) = artwork_path {
                    // Record in database that this album has folder artwork
                    soul_storage::albums::set_artwork_source(
                        self.pool,
                        album_id,
                        "folder",
                        &artwork_path.to_string_lossy(),
                    )
                    .await
                    .ok(); // Ignore errors - artwork is optional
                }
            }
        }

        tracing::info!(
            "[UPDATE] Successfully updated track {}: {}",
            track_id,
            file_path.display()
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_action_equality() {
        assert_eq!(FileAction::New, FileAction::New);
        assert_ne!(FileAction::New, FileAction::Updated);
        assert_ne!(FileAction::Updated, FileAction::Unchanged);
        assert_ne!(FileAction::Relocated, FileAction::New);
    }

    #[test]
    fn test_file_action_debug() {
        let action = FileAction::New;
        let debug_str = format!("{:?}", action);
        assert_eq!(debug_str, "New");
    }

    #[test]
    fn test_file_action_clone() {
        let action1 = FileAction::Updated;
        let action2 = action1;
        assert_eq!(action1, action2);
    }
}
