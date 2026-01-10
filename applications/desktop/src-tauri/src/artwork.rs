//! Album artwork extraction and serving via custom protocol
//!
//! Provides on-demand artwork extraction from audio files using the soul-artwork library.
//! Implements artwork:// protocol for efficient image serving with built-in LRU caching.

use soul_artwork::ArtworkExtractor;
use soul_core::types::{AlbumId, TrackId};
use sqlx::SqlitePool;
use std::sync::Arc;
use tauri::http::Response;

/// Manages artwork extraction with caching
pub struct ArtworkManager {
    extractor: Arc<ArtworkExtractor>,
    pool: SqlitePool,
}

impl ArtworkManager {
    /// Create a new artwork manager
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `cache_size` - Number of images to cache in memory (default: 100)
    pub fn new(pool: SqlitePool, cache_size: usize) -> Self {
        Self {
            extractor: Arc::new(ArtworkExtractor::new(cache_size)),
            pool,
        }
    }

    /// Get artwork for an album
    ///
    /// Finds any track from the album and extracts its embedded artwork.
    ///
    /// # Arguments
    /// * `album_id` - Album ID
    pub async fn get_album_artwork(&self, album_id: AlbumId) -> Result<Option<Vec<u8>>, String> {
        // Get any track from this album
        let track = self.get_track_from_album(album_id).await?;

        if let Some(track_id) = track {
            self.get_track_artwork(track_id).await
        } else {
            Ok(None)
        }
    }

    /// Get artwork for a specific track
    ///
    /// Extracts artwork from the track's audio file.
    ///
    /// # Arguments
    /// * `track_id` - Track ID
    pub async fn get_track_artwork(&self, track_id: TrackId) -> Result<Option<Vec<u8>>, String> {
        // Get track file path
        let file_path = self.get_track_file_path(track_id).await?;

        if let Some(path) = file_path {
            // Extract artwork using soul-artwork
            match self.extractor.extract(&path) {
                Ok(Some(artwork)) => Ok(Some(artwork.data)),
                Ok(None) => Ok(None),
                Err(e) => {
                    eprintln!("Failed to extract artwork from {}: {}", path.display(), e);
                    Ok(None)
                }
            }
        } else {
            Ok(None)
        }
    }

    /// Get artwork with MIME type for HTTP response
    pub async fn get_album_artwork_with_mime(
        &self,
        album_id: AlbumId,
    ) -> Result<Option<(Vec<u8>, String)>, String> {
        let track = self.get_track_from_album(album_id).await?;

        if let Some(track_id) = track {
            self.get_track_artwork_with_mime(track_id).await
        } else {
            Ok(None)
        }
    }

    /// Get artwork with MIME type for a specific track
    pub async fn get_track_artwork_with_mime(
        &self,
        track_id: TrackId,
    ) -> Result<Option<(Vec<u8>, String)>, String> {
        let file_path = self.get_track_file_path(track_id).await?;

        if let Some(path) = file_path {
            match self.extractor.extract(&path) {
                Ok(Some(artwork)) => Ok(Some((artwork.data, artwork.mime_type))),
                Ok(None) => Ok(None),
                Err(e) => {
                    eprintln!("Failed to extract artwork from {}: {}", path.display(), e);
                    Ok(None)
                }
            }
        } else {
            Ok(None)
        }
    }

    /// Helper: Get any track from an album
    async fn get_track_from_album(&self, album_id: AlbumId) -> Result<Option<TrackId>, String> {
        let result = sqlx::query!("SELECT id FROM tracks WHERE album_id = ? LIMIT 1", album_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| e.to_string())?;

        Ok(result.map(|row| TrackId::new(row.id.to_string())))
    }

    /// Helper: Get file path for a track
    async fn get_track_file_path(
        &self,
        track_id: TrackId,
    ) -> Result<Option<std::path::PathBuf>, String> {
        // Get track with availability info
        let track = soul_storage::tracks::get_by_id(&self.pool, track_id)
            .await
            .map_err(|e| e.to_string())?;

        if let Some(track) = track {
            // Find first local file path
            let file_path = track.availability.iter().find_map(|avail| {
                if matches!(
                    avail.status,
                    soul_core::types::AvailabilityStatus::LocalFile
                        | soul_core::types::AvailabilityStatus::Cached
                ) {
                    avail.local_file_path.clone().map(std::path::PathBuf::from)
                } else {
                    None
                }
            });

            Ok(file_path)
        } else {
            Ok(None)
        }
    }
}

/// Handle artwork:// protocol requests
///
/// URL format:
/// - artwork://album/<album_id> - Get artwork for an album
/// - artwork://track/<track_id> - Get artwork for a track
pub async fn handle_artwork_request(
    manager: &ArtworkManager,
    uri: &str,
) -> Result<Response<Vec<u8>>, Box<dyn std::error::Error>> {
    eprintln!("[artwork] Handling request: {}", uri);

    // Parse URI: artwork://album/123 or artwork://track/456
    let path = uri
        .strip_prefix("artwork://")
        .ok_or("Invalid artwork URI")?;

    eprintln!("[artwork] Path after prefix: {}", path);

    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() != 2 {
        eprintln!("[artwork] ERROR: Invalid URI format, expected 2 parts, got {}", parts.len());
        return Err("Invalid artwork URI format".into());
    }

    let (entity_type, id_str) = (parts[0], parts[1]);
    eprintln!("[artwork] Entity type: {}, ID string: {}", entity_type, id_str);

    let id: i64 = id_str.parse().map_err(|e| {
        eprintln!("[artwork] ERROR: Failed to parse ID '{}': {:?}", id_str, e);
        "Invalid ID"
    })?;

    eprintln!("[artwork] Parsed ID: {}", id);

    let artwork = match entity_type {
        "album" => {
            eprintln!("[artwork] Fetching artwork for album {}", id);
            manager.get_album_artwork_with_mime(id).await?
        }
        "track" => {
            eprintln!("[artwork] Fetching artwork for track {}", id);
            let result = manager
                .get_track_artwork_with_mime(TrackId::new(id.to_string()))
                .await?;
            eprintln!("[artwork] Track artwork result: {}", if result.is_some() { "found" } else { "not found" });
            result
        }
        _ => {
            eprintln!("[artwork] ERROR: Unknown entity type: {}", entity_type);
            return Err("Unknown entity type".into());
        }
    };

    if let Some((data, mime_type)) = artwork {
        eprintln!("[artwork] SUCCESS: Returning {} bytes of {}", data.len(), mime_type);
        // Return image with proper MIME type and caching headers
        Response::builder()
            .status(200)
            .header("Content-Type", mime_type)
            .header("Cache-Control", "public, max-age=31536000") // Cache for 1 year
            .body(data)
            .map_err(|e| e.into())
    } else {
        eprintln!("[artwork] No artwork found for {} {}", entity_type, id);
        // No artwork found - return 404
        Response::builder()
            .status(404)
            .header("Content-Type", "text/plain")
            .body(b"No artwork found".to_vec())
            .map_err(|e| e.into())
    }
}
