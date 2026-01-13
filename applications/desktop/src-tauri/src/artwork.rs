//! Album artwork extraction and serving via custom protocol
//!
//! Provides on-demand artwork extraction from audio files using the soul-artwork library.
//! Implements artwork:// protocol for efficient image serving with built-in LRU caching.
//! Supports custom artwork storage that overrides embedded artwork.

use soul_artwork::{ArtworkData, ArtworkExtractor, ArtworkWriter};
use soul_core::types::{AlbumId, ArtistId, PlaylistId, TrackId, UserId};
use sqlx::SqlitePool;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::http::Response;

/// Manages artwork extraction and custom artwork storage
pub struct ArtworkManager {
    extractor: Arc<ArtworkExtractor>,
    pool: SqlitePool,
    /// Base directory for storing custom artwork
    artwork_storage_path: PathBuf,
}

impl ArtworkManager {
    /// Create a new artwork manager
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `cache_size` - Number of images to cache in memory (default: 100)
    /// * `artwork_storage_path` - Base directory for custom artwork storage
    pub fn new(pool: SqlitePool, cache_size: usize, artwork_storage_path: PathBuf) -> Self {
        Self {
            extractor: Arc::new(ArtworkExtractor::new(cache_size)),
            pool,
            artwork_storage_path,
        }
    }

    /// Get the custom artwork directory for a specific entity type
    fn get_artwork_dir(&self, entity_type: &str) -> PathBuf {
        self.artwork_storage_path.join(entity_type)
    }

    /// Find custom artwork file for an entity
    fn find_custom_artwork(&self, entity_type: &str, id: &str) -> Option<PathBuf> {
        let dir = self.get_artwork_dir(entity_type);
        for ext in ["jpg", "jpeg", "png", "webp"] {
            let path = dir.join(format!("{}.{}", id, ext));
            if path.exists() {
                return Some(path);
            }
        }
        None
    }

    /// Read custom artwork from file
    fn read_custom_artwork(&self, path: &Path) -> Result<Option<(Vec<u8>, String)>, String> {
        let data = std::fs::read(path).map_err(|e| e.to_string())?;

        // Detect MIME type from extension
        let mime_type = match path.extension().and_then(|e| e.to_str()) {
            Some("png") => "image/png",
            Some("webp") => "image/webp",
            Some("gif") => "image/gif",
            _ => "image/jpeg",
        };

        Ok(Some((data, mime_type.to_string())))
    }

    /// Save custom artwork for an entity
    async fn save_custom_artwork(
        &self,
        entity_type: &str,
        id: &str,
        data: &[u8],
        mime_type: &str,
    ) -> Result<PathBuf, String> {
        let dir = self.get_artwork_dir(entity_type);
        std::fs::create_dir_all(&dir)
            .map_err(|e| format!("Failed to create artwork dir: {}", e))?;

        // Determine extension from MIME type
        let ext = match mime_type {
            "image/png" => "png",
            "image/webp" => "webp",
            "image/gif" => "gif",
            _ => "jpg",
        };

        // Remove any existing artwork for this entity
        self.remove_custom_artwork_files(entity_type, id)?;

        let path = dir.join(format!("{}.{}", id, ext));
        std::fs::write(&path, data).map_err(|e| format!("Failed to write artwork: {}", e))?;

        Ok(path)
    }

    /// Remove custom artwork files for an entity
    fn remove_custom_artwork_files(&self, entity_type: &str, id: &str) -> Result<(), String> {
        let dir = self.get_artwork_dir(entity_type);
        for ext in ["jpg", "jpeg", "png", "webp", "gif"] {
            let path = dir.join(format!("{}.{}", id, ext));
            if path.exists() {
                std::fs::remove_file(&path)
                    .map_err(|e| format!("Failed to remove old artwork: {}", e))?;
            }
        }
        Ok(())
    }

    /// Get artwork for an album
    ///
    /// Priority order:
    /// 1. Soul Player storage (user explicitly set via UI)
    /// 2. Folder artwork (cover.jpg, folder.jpg, etc.)
    /// 3. Embedded artwork (from track metadata)
    ///
    /// # Arguments
    /// * `album_id` - Album ID
    pub async fn get_album_artwork(&self, album_id: AlbumId) -> Result<Option<Vec<u8>>, String> {
        // Get artwork with MIME type, then extract just the data
        self.get_album_artwork_with_mime(album_id)
            .await
            .map(|opt| opt.map(|(data, _mime_type)| data))
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
    ///
    /// Priority order:
    /// 1. Soul Player storage (user explicitly set via UI)
    /// 2. Folder artwork (cover.jpg, folder.jpg, etc.)
    /// 3. Embedded artwork (from track metadata)
    pub async fn get_album_artwork_with_mime(
        &self,
        album_id: AlbumId,
    ) -> Result<Option<(Vec<u8>, String)>, String> {
        // Get album's artwork source preference
        let artwork_info = soul_storage::albums::get_artwork_source(&self.pool, album_id)
            .await
            .map_err(|e| e.to_string())?;

        // 1. Check Soul Player custom artwork (highest priority)
        if let Some(custom_path) = self.find_custom_artwork("albums", &album_id.to_string()) {
            if let Ok(Some(result)) = self.read_custom_artwork(&custom_path) {
                return Ok(Some(result));
            }
        }

        // 2. Check folder artwork (middle priority)
        // First try the recorded path from database
        if let Some((source, Some(path))) = artwork_info {
            if source == "folder" && std::path::Path::new(&path).exists() {
                if let Ok(data) = std::fs::read(&path) {
                    let mime_type = Self::guess_mime_from_path(&path);
                    return Ok(Some((data, mime_type)));
                }
            }
        }

        // Fallback: Auto-discover folder artwork if not in database
        if let Ok(Some(folder_path)) = self.discover_folder_artwork_for_album(album_id).await {
            if let Ok(data) = std::fs::read(&folder_path) {
                let mime_type = Self::guess_mime_from_path(&folder_path.to_string_lossy());
                return Ok(Some((data, mime_type)));
            }
        }

        // 3. Fall back to embedded artwork (lowest priority)
        let track = self.get_track_from_album(album_id).await?;

        if let Some(track_id) = track {
            self.get_track_artwork_with_mime(track_id).await
        } else {
            Ok(None)
        }
    }

    /// Get artwork with MIME type and source info (custom vs embedded)
    ///
    /// Priority order:
    /// 1. Soul Player storage (user explicitly set via UI)
    /// 2. Folder artwork (cover.jpg, folder.jpg, etc.)
    /// 3. Embedded artwork (from track metadata)
    ///
    /// Returns (data, mime_type, is_custom) where is_custom=true means from Soul Player storage
    pub async fn get_album_artwork_with_source(
        &self,
        album_id: AlbumId,
    ) -> Result<Option<(Vec<u8>, String, bool)>, String> {
        // Get album's artwork source preference
        let artwork_info = soul_storage::albums::get_artwork_source(&self.pool, album_id)
            .await
            .map_err(|e| e.to_string())?;

        // 1. Check Soul Player custom artwork (highest priority)
        if let Some(custom_path) = self.find_custom_artwork("albums", &album_id.to_string()) {
            if let Ok(Some((data, mime_type))) = self.read_custom_artwork(&custom_path) {
                return Ok(Some((data, mime_type, true)));
            }
        }

        // 2. Check folder artwork (middle priority)
        // First try the recorded path from database
        if let Some((source, Some(path))) = artwork_info {
            if source == "folder" && std::path::Path::new(&path).exists() {
                if let Ok(data) = std::fs::read(&path) {
                    let mime_type = Self::guess_mime_from_path(&path);
                    return Ok(Some((data, mime_type, false)));
                }
            }
        }

        // Fallback: Auto-discover folder artwork if not in database
        if let Ok(Some(folder_path)) = self.discover_folder_artwork_for_album(album_id).await {
            if let Ok(data) = std::fs::read(&folder_path) {
                let mime_type = Self::guess_mime_from_path(&folder_path.to_string_lossy());
                return Ok(Some((data, mime_type, false)));
            }
        }

        // 3. Fall back to embedded artwork (lowest priority)
        let track = self.get_track_from_album(album_id).await?;

        if let Some(track_id) = track {
            if let Some((data, mime_type)) = self.get_track_artwork_with_mime(track_id).await? {
                return Ok(Some((data, mime_type, false)));
            }
        }

        Ok(None)
    }

    /// Guess MIME type from file path
    fn guess_mime_from_path(path: &str) -> String {
        let path_lower = path.to_lowercase();
        if path_lower.ends_with(".png") {
            "image/png".to_string()
        } else if path_lower.ends_with(".webp") {
            "image/webp".to_string()
        } else if path_lower.ends_with(".gif") {
            "image/gif".to_string()
        } else if path_lower.ends_with(".bmp") {
            "image/bmp".to_string()
        } else {
            "image/jpeg".to_string()
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

    /// Get all track file paths for an album
    async fn get_all_album_track_paths(&self, album_id: AlbumId) -> Result<Vec<PathBuf>, String> {
        let rows = sqlx::query("SELECT id FROM tracks WHERE album_id = ?")
            .bind(album_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e: sqlx::Error| e.to_string())?;

        let mut paths = Vec::new();
        for row in rows {
            use sqlx::Row;
            let id: i64 = row.get("id");
            let track_id = TrackId::new(id.to_string());
            if let Some(path) = self.get_track_file_path(track_id).await? {
                paths.push(path);
            }
        }
        Ok(paths)
    }

    /// Find the album folder (folder with most tracks)
    async fn find_album_folder(&self, album_id: AlbumId) -> Result<PathBuf, String> {
        let track_paths = self.get_all_album_track_paths(album_id).await?;

        if track_paths.is_empty() {
            return Err("No local files found for album".to_string());
        }

        // Count tracks per folder
        let mut folder_counts: std::collections::HashMap<PathBuf, usize> =
            std::collections::HashMap::new();
        for track_path in track_paths {
            if let Some(parent) = track_path.parent() {
                *folder_counts.entry(parent.to_path_buf()).or_insert(0) += 1;
            }
        }

        // Return folder with most tracks
        folder_counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(path, _)| path)
            .ok_or_else(|| "Could not determine album folder".to_string())
    }

    /// Discover folder artwork in a directory
    ///
    /// Looks for common artwork filenames: cover, folder, front, album, artwork
    /// Supports extensions: jpg, jpeg, png, webp, gif, bmp
    fn discover_folder_artwork(folder: &Path) -> Option<PathBuf> {
        const FILENAMES: &[&str] = &["cover", "folder", "front", "album", "artwork"];
        const EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp", "gif", "bmp"];

        for name in FILENAMES {
            for ext in EXTENSIONS {
                let path = folder.join(format!("{}.{}", name, ext));
                if path.exists() {
                    return Some(path);
                }
            }
        }
        None
    }

    /// Discover folder artwork for an album
    async fn discover_folder_artwork_for_album(
        &self,
        album_id: AlbumId,
    ) -> Result<Option<PathBuf>, String> {
        let folder = self.find_album_folder(album_id).await?;
        Ok(Self::discover_folder_artwork(&folder))
    }

    // =========================================================================
    // Artwork Setting/Removal Methods
    // =========================================================================

    /// Set artwork for an album
    ///
    /// # Arguments
    /// * `album_id` - Album ID
    /// * `data` - Image data bytes
    /// * `mime_type` - MIME type (e.g., "image/jpeg")
    /// * `write_to_files` - If true, also write to embedded metadata in all album tracks
    /// * `use_soul_storage` - If true, save to Soul Player storage instead of album folder
    pub async fn set_album_artwork(
        &self,
        album_id: AlbumId,
        data: Vec<u8>,
        mime_type: &str,
        write_to_files: bool,
        use_soul_storage: bool,
    ) -> Result<(), String> {
        if use_soul_storage {
            // Save to Soul Player storage (managed separately)
            let artwork_path = self
                .save_custom_artwork("albums", &album_id.to_string(), &data, mime_type)
                .await?;

            tracing::info!(
                "Saved artwork to Soul Player storage: {}",
                artwork_path.display()
            );

            // Update database with artwork source
            soul_storage::albums::set_artwork_source(
                &self.pool,
                album_id,
                "soul_storage",
                &artwork_path.to_string_lossy(),
            )
            .await
            .map_err(|e| format!("Failed to update database: {}", e))?;
        } else {
            // Save to album folder as cover.{ext}
            let album_folder = self.find_album_folder(album_id).await?;

            // Determine file extension based on MIME type
            let ext = match mime_type {
                "image/png" => "png",
                "image/webp" => "webp",
                "image/gif" => "gif",
                "image/bmp" => "bmp",
                _ => "jpg",
            };

            // Write as cover.{ext} in album folder
            let cover_path = album_folder.join(format!("cover.{}", ext));
            std::fs::write(&cover_path, &data)
                .map_err(|e| format!("Failed to write artwork to album folder: {}", e))?;

            tracing::info!("Saved artwork to album folder: {}", cover_path.display());

            // Update database with artwork source
            soul_storage::albums::set_artwork_source(
                &self.pool,
                album_id,
                "folder",
                &cover_path.to_string_lossy(),
            )
            .await
            .map_err(|e| format!("Failed to update database: {}", e))?;
        }

        // Optionally write to audio files
        if write_to_files {
            let artwork = ArtworkData::new(data, mime_type.to_string());
            let track_paths = self.get_all_album_track_paths(album_id).await?;

            tracing::info!("Writing artwork to {} track files...", track_paths.len());

            let mut success_count = 0;
            let mut failure_count = 0;

            for path in track_paths {
                match ArtworkWriter::write_to_file(&path, &artwork) {
                    Ok(_) => {
                        success_count += 1;
                        tracing::debug!("Successfully wrote artwork to: {}", path.display());
                    }
                    Err(e) => {
                        failure_count += 1;
                        tracing::warn!("Failed to write artwork to {}: {}", path.display(), e);
                    }
                }
            }

            tracing::info!(
                "Artwork write summary: {} succeeded, {} failed",
                success_count,
                failure_count
            );

            if success_count == 0 && failure_count > 0 {
                return Err(format!(
                    "Failed to write artwork to all {} track files. Check file permissions and formats.",
                    failure_count
                ));
            }
        }

        // Clear cache to pick up new artwork
        self.extractor.clear_cache();

        Ok(())
    }

    /// Remove custom artwork from an album
    pub async fn remove_album_artwork(&self, album_id: AlbumId) -> Result<(), String> {
        self.remove_custom_artwork_files("albums", &album_id.to_string())?;

        // Clear artwork source from database (will fall back to folder or embedded)
        soul_storage::albums::set_artwork_source(&self.pool, album_id, "", "")
            .await
            .ok();

        soul_storage::albums::update_cover_art_path(&self.pool, album_id, None)
            .await
            .map_err(|e| format!("Failed to update database: {}", e))?;

        self.extractor.clear_cache();
        Ok(())
    }

    /// Get artwork for an artist
    pub async fn get_artist_artwork_with_mime(
        &self,
        artist_id: ArtistId,
    ) -> Result<Option<(Vec<u8>, String)>, String> {
        if let Some(custom_path) = self.find_custom_artwork("artists", &artist_id.to_string()) {
            return self.read_custom_artwork(&custom_path);
        }
        Ok(None)
    }

    /// Get artwork for an artist with source info
    pub async fn get_artist_artwork_with_source(
        &self,
        artist_id: ArtistId,
    ) -> Result<Option<(Vec<u8>, String, bool)>, String> {
        if let Some(custom_path) = self.find_custom_artwork("artists", &artist_id.to_string()) {
            if let Ok(Some((data, mime_type))) = self.read_custom_artwork(&custom_path) {
                return Ok(Some((data, mime_type, true)));
            }
        }
        Ok(None)
    }

    /// Set artwork for an artist
    pub async fn set_artist_artwork(
        &self,
        artist_id: ArtistId,
        data: Vec<u8>,
        mime_type: &str,
    ) -> Result<(), String> {
        let artwork_path = self
            .save_custom_artwork("artists", &artist_id.to_string(), &data, mime_type)
            .await?;

        soul_storage::artists::update_cover_art_path(
            &self.pool,
            artist_id,
            Some(&artwork_path.to_string_lossy()),
        )
        .await
        .map_err(|e| format!("Failed to update database: {}", e))?;

        Ok(())
    }

    /// Remove artwork from an artist
    pub async fn remove_artist_artwork(&self, artist_id: ArtistId) -> Result<(), String> {
        self.remove_custom_artwork_files("artists", &artist_id.to_string())?;

        soul_storage::artists::update_cover_art_path(&self.pool, artist_id, None)
            .await
            .map_err(|e| format!("Failed to update database: {}", e))?;

        Ok(())
    }

    /// Get artwork for a playlist
    pub async fn get_playlist_artwork_with_mime(
        &self,
        playlist_id: &PlaylistId,
    ) -> Result<Option<(Vec<u8>, String)>, String> {
        if let Some(custom_path) = self.find_custom_artwork("playlists", playlist_id.as_str()) {
            return self.read_custom_artwork(&custom_path);
        }
        Ok(None)
    }

    /// Get artwork for a playlist with source info
    pub async fn get_playlist_artwork_with_source(
        &self,
        playlist_id: &PlaylistId,
    ) -> Result<Option<(Vec<u8>, String, bool)>, String> {
        if let Some(custom_path) = self.find_custom_artwork("playlists", playlist_id.as_str()) {
            if let Ok(Some((data, mime_type))) = self.read_custom_artwork(&custom_path) {
                return Ok(Some((data, mime_type, true)));
            }
        }
        Ok(None)
    }

    /// Set artwork for a playlist
    pub async fn set_playlist_artwork(
        &self,
        user_id: &UserId,
        playlist_id: &PlaylistId,
        data: Vec<u8>,
        mime_type: &str,
    ) -> Result<(), String> {
        let artwork_path = self
            .save_custom_artwork("playlists", playlist_id.as_str(), &data, mime_type)
            .await?;

        soul_storage::playlists::update_cover_art_path(
            &self.pool,
            playlist_id.clone(),
            user_id.clone(),
            Some(&artwork_path.to_string_lossy()),
        )
        .await
        .map_err(|e| format!("Failed to update database: {}", e))?;

        Ok(())
    }

    /// Remove artwork from a playlist
    pub async fn remove_playlist_artwork(
        &self,
        user_id: &UserId,
        playlist_id: &PlaylistId,
    ) -> Result<(), String> {
        self.remove_custom_artwork_files("playlists", playlist_id.as_str())?;

        soul_storage::playlists::update_cover_art_path(
            &self.pool,
            playlist_id.clone(),
            user_id.clone(),
            None,
        )
        .await
        .map_err(|e| format!("Failed to update database: {}", e))?;

        Ok(())
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
        eprintln!(
            "[artwork] ERROR: Invalid URI format, expected 2 parts, got {}",
            parts.len()
        );
        return Err("Invalid artwork URI format".into());
    }

    let (entity_type, id_str) = (parts[0], parts[1]);
    eprintln!(
        "[artwork] Entity type: {}, ID string: {}",
        entity_type, id_str
    );

    let artwork = match entity_type {
        "album" => {
            let id: i64 = id_str.parse().map_err(|e| {
                eprintln!(
                    "[artwork] ERROR: Failed to parse album ID '{}': {:?}",
                    id_str, e
                );
                "Invalid album ID"
            })?;
            eprintln!("[artwork] Fetching artwork for album {}", id);
            manager.get_album_artwork_with_mime(id).await?
        }
        "track" => {
            let id: i64 = id_str.parse().map_err(|e| {
                eprintln!(
                    "[artwork] ERROR: Failed to parse track ID '{}': {:?}",
                    id_str, e
                );
                "Invalid track ID"
            })?;
            eprintln!("[artwork] Fetching artwork for track {}", id);
            let result = manager
                .get_track_artwork_with_mime(TrackId::new(id.to_string()))
                .await?;
            eprintln!(
                "[artwork] Track artwork result: {}",
                if result.is_some() {
                    "found"
                } else {
                    "not found"
                }
            );
            result
        }
        "artist" => {
            let id: i64 = id_str.parse().map_err(|e| {
                eprintln!(
                    "[artwork] ERROR: Failed to parse artist ID '{}': {:?}",
                    id_str, e
                );
                "Invalid artist ID"
            })?;
            eprintln!("[artwork] Fetching artwork for artist {}", id);
            manager.get_artist_artwork_with_mime(id).await?
        }
        "playlist" => {
            // Playlist IDs are UUIDs, not numeric
            eprintln!("[artwork] Fetching artwork for playlist {}", id_str);
            manager
                .get_playlist_artwork_with_mime(&PlaylistId::new(id_str.to_string()))
                .await?
        }
        _ => {
            eprintln!("[artwork] ERROR: Unknown entity type: {}", entity_type);
            return Err("Unknown entity type".into());
        }
    };

    if let Some((data, mime_type)) = artwork {
        eprintln!(
            "[artwork] SUCCESS: Returning {} bytes of {}",
            data.len(),
            mime_type
        );
        // Return image with proper MIME type and caching headers
        Response::builder()
            .status(200)
            .header("Content-Type", mime_type)
            .header("Cache-Control", "public, max-age=31536000") // Cache for 1 year
            .body(data)
            .map_err(|e| e.into())
    } else {
        eprintln!("[artwork] No artwork found for {} {}", entity_type, id_str);
        // No artwork found - return 404
        Response::builder()
            .status(404)
            .header("Content-Type", "text/plain")
            .body(b"No artwork found".to_vec())
            .map_err(|e| e.into())
    }
}
