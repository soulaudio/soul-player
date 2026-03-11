//! Metadata extraction and entity matching
//!
//! This module handles extraction of audio metadata from files and fuzzy matching
//! of artists, albums, and genres. It provides a higher-level interface over the
//! raw metadata extraction, handling timeouts and entity creation/matching.

use crate::{fuzzy::FuzzyMatcher, metadata, ImportError, Result};
use soul_core::types::{AlbumId, ArtistId, GenreId, TrackId};
use sqlx::SqlitePool;
use std::path::Path;

/// Extracted and matched metadata for a track
#[derive(Debug)]
pub struct ProcessedMetadata {
    /// Raw metadata from the file
    pub raw: metadata::ExtractedMetadata,
    /// Resolved artist IDs for all artists (position = index; [0] = primary artist)
    pub artist_ids: Vec<ArtistId>,
    /// Matched album ID (if album tag exists)
    pub album_id: Option<AlbumId>,
    /// Matched album artist ID (if album artist tag exists and differs from artist)
    pub album_artist_id: Option<ArtistId>,
    /// Matched genre IDs
    pub genre_ids: Vec<GenreId>,
}

/// Metadata extractor with fuzzy matching
pub struct MetadataExtractor {
    fuzzy_matcher: FuzzyMatcher,
}

impl MetadataExtractor {
    /// Create a new metadata extractor
    pub fn new() -> Self {
        Self {
            fuzzy_matcher: FuzzyMatcher::new(),
        }
    }

    /// Get a reference to the underlying fuzzy matcher
    pub fn fuzzy_matcher(&self) -> &FuzzyMatcher {
        &self.fuzzy_matcher
    }

    /// Extract metadata from a file with timeout protection
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the audio file
    ///
    /// # Returns
    ///
    /// Raw metadata extracted from the file
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Metadata extraction times out (30 seconds)
    /// - Metadata extraction task panics
    /// - File cannot be read or parsed
    pub async fn extract_metadata(&self, file_path: &Path) -> Result<metadata::ExtractedMetadata> {
        tracing::info!("[METADATA] Processing: {}", file_path.display());

        let file_path_owned = file_path.to_path_buf();
        let file_path_for_log = file_path.to_path_buf();

        let meta_task =
            tokio::task::spawn_blocking(move || metadata::extract_metadata(&file_path_owned));

        let meta = tokio::time::timeout(std::time::Duration::from_secs(30), meta_task)
            .await
            .map_err(|_| {
                tracing::error!(
                    "[METADATA] TIMEOUT on file: {}",
                    file_path_for_log.display()
                );
                ImportError::Metadata(format!(
                    "Metadata extraction timeout (30s) for: {}",
                    file_path_for_log.display()
                ))
            })?
            .map_err(|e| {
                ImportError::Metadata(format!("Metadata extraction task failed: {}", e))
            })??;

        tracing::info!(
            "[METADATA] Extracted: file={}, artist={:?}, album={:?}",
            file_path.display(),
            meta.artists.first(),
            meta.album
        );

        Ok(meta)
    }

    /// Extract metadata and match entities (artists, albums, genres)
    ///
    /// This method extracts metadata from the file and performs fuzzy matching
    /// to find or create corresponding database entities.
    ///
    /// # Arguments
    ///
    /// * `pool` - Database connection pool
    /// * `file_path` - Path to the audio file
    ///
    /// # Returns
    ///
    /// Processed metadata with matched entity IDs
    ///
    /// # Errors
    ///
    /// Returns an error if metadata extraction or entity matching fails
    pub async fn extract_and_match(
        &self,
        pool: &SqlitePool,
        file_path: &Path,
    ) -> Result<ProcessedMetadata> {
        let raw = self.extract_metadata(file_path).await?;

        // Resolve all artists from the tag (may be multiple after splitting)
        let mut artist_ids: Vec<ArtistId> = Vec::new();
        for artist_name in &raw.artists {
            let artist_match = self
                .fuzzy_matcher
                .find_or_create_artist(pool, artist_name)
                .await?;
            artist_ids.push(artist_match.entity.id);
        }
        // Convenience: primary artist (position 0) for tracks.artist_id backward compat
        let primary_artist_id = artist_ids.first().copied();

        // Derive folder path for strict album isolation
        let album_folder = file_path
            .parent()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();

        // Fuzzy match album artist (if different from track artist).
        // Must be resolved before album matching so we can use album_artist_id
        // as the album key — preventing feat. artist tracks from creating duplicates.
        let album_artist_id = if let Some(ref album_artist_name) = raw.album_artist {
            if raw.artists.first().map(String::as_str) != Some(album_artist_name.as_str()) {
                let artist_match = self
                    .fuzzy_matcher
                    .find_or_create_artist(pool, album_artist_name)
                    .await?;
                Some(artist_match.entity.id)
            } else {
                primary_artist_id
            }
        } else {
            None
        };

        // Match album (linked to album_artist + folder for strict isolation).
        // Use album_artist_id when available so that "Artist A feat. Artist B" tracks still
        // group under the same album as plain "Artist A" tracks (same ALBUMARTIST tag).
        let album_key_artist = album_artist_id.or(primary_artist_id);
        let album_id = if let Some(ref album_title) = raw.album {
            let album_match = self
                .fuzzy_matcher
                .find_or_create_album(pool, album_title, album_key_artist, &album_folder)
                .await?;
            Some(album_match.entity.id)
        } else {
            None
        };

        // Fuzzy match genres
        let mut genre_ids = Vec::new();
        for genre_name in &raw.genres {
            let genre_match = self
                .fuzzy_matcher
                .find_or_create_genre(pool, genre_name)
                .await?;
            genre_ids.push(genre_match.entity.id);
        }

        Ok(ProcessedMetadata {
            raw,
            artist_ids,
            album_id,
            album_artist_id,
            genre_ids,
        })
    }

    /// Add genres to a track
    ///
    /// # Arguments
    ///
    /// * `pool` - Database connection pool
    /// * `track_id` - Track to add genres to
    /// * `genre_ids` - Genre IDs to add
    ///
    /// # Errors
    ///
    /// Returns an error if database operations fail
    pub async fn add_genres_to_track(
        &self,
        pool: &SqlitePool,
        track_id: TrackId,
        genre_ids: &[GenreId],
    ) -> Result<()> {
        for genre_id in genre_ids {
            soul_storage::genres::add_to_track(pool, track_id.clone(), *genre_id).await?;
        }
        Ok(())
    }

    /// Write all artists to the track_artists junction table.
    /// Clears existing associations first (idempotent for re-imports).
    pub async fn add_artists_to_track(
        &self,
        pool: &SqlitePool,
        track_id: &TrackId,
        artist_ids: &[ArtistId],
    ) -> Result<()> {
        soul_storage::track_artists::clear_for_track(pool, track_id).await?;
        for (position, &artist_id) in artist_ids.iter().enumerate() {
            soul_storage::track_artists::add_to_track(pool, track_id, artist_id, position as i64)
                .await?;
        }
        Ok(())
    }
}

impl Default for MetadataExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata_extractor_creation() {
        let _extractor = MetadataExtractor::new();
        // MetadataExtractor created successfully
    }

    #[test]
    fn test_default_implementation() {
        let _extractor = MetadataExtractor::default();
        // MetadataExtractor created successfully via Default trait
    }
}
