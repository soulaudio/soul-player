//! Storage trait for multi-source architecture

use async_trait::async_trait;
use crate::error::Result;
use crate::types::*;

/// Storage context providing access to database operations
///
/// This trait abstracts storage operations to support both local SQLite
/// and remote API implementations.
#[async_trait]
pub trait StorageContext: Send + Sync {
    /// Get the current user ID
    fn user_id(&self) -> i64;

    // ========================================================================
    // Sources
    // ========================================================================

    /// Get all sources
    async fn get_sources(&self) -> Result<Vec<Source>>;

    /// Get source by ID
    async fn get_source(&self, id: SourceId) -> Result<Option<Source>>;

    /// Get currently active server
    async fn get_active_server(&self) -> Result<Option<Source>>;

    /// Create a new source
    async fn create_source(&self, source: CreateSource) -> Result<Source>;

    /// Set active server (deactivates others)
    async fn set_active_server(&self, id: SourceId) -> Result<()>;

    /// Update source online/offline status
    async fn update_source_status(&self, id: SourceId, is_online: bool) -> Result<()>;

    // ========================================================================
    // Tracks
    // ========================================================================

    /// Get all tracks
    async fn get_all_tracks(&self) -> Result<Vec<Track>>;

    /// Get track by ID
    async fn get_track_by_id(&self, id: i64) -> Result<Option<Track>>;

    /// Get tracks by source
    async fn get_tracks_by_source(&self, source_id: SourceId) -> Result<Vec<Track>>;

    /// Get tracks by artist
    async fn get_tracks_by_artist(&self, artist_id: ArtistId) -> Result<Vec<Track>>;

    /// Get tracks by album
    async fn get_tracks_by_album(&self, album_id: AlbumId) -> Result<Vec<Track>>;

    /// Create a new track
    async fn create_track(&self, track: CreateTrack) -> Result<Track>;

    /// Update a track
    async fn update_track(&self, id: i64, track: UpdateTrack) -> Result<Track>;

    /// Delete a track
    async fn delete_track(&self, id: i64) -> Result<()>;

    /// Get track availability across sources
    async fn get_track_availability(&self, track_id: i64) -> Result<Vec<TrackAvailability>>;

    // ========================================================================
    // Artists
    // ========================================================================

    /// Get all artists
    async fn get_all_artists(&self) -> Result<Vec<Artist>>;

    /// Get artist by ID
    async fn get_artist_by_id(&self, id: ArtistId) -> Result<Option<Artist>>;

    /// Find artist by exact name
    async fn find_artist_by_name(&self, name: &str) -> Result<Option<Artist>>;

    /// Create a new artist
    async fn create_artist(&self, artist: CreateArtist) -> Result<Artist>;

    // ========================================================================
    // Albums
    // ========================================================================

    /// Get all albums
    async fn get_all_albums(&self) -> Result<Vec<Album>>;

    /// Get album by ID
    async fn get_album_by_id(&self, id: AlbumId) -> Result<Option<Album>>;

    /// Get albums by artist
    async fn get_albums_by_artist(&self, artist_id: ArtistId) -> Result<Vec<Album>>;

    /// Create a new album
    async fn create_album(&self, album: CreateAlbum) -> Result<Album>;

    // ========================================================================
    // Playlists
    // ========================================================================

    /// Get user's playlists (owned + shared)
    async fn get_user_playlists(&self) -> Result<Vec<Playlist>>;

    /// Get playlist by ID
    async fn get_playlist_by_id(&self, id: i64) -> Result<Option<Playlist>>;

    /// Get playlist with tracks
    async fn get_playlist_with_tracks(&self, id: i64) -> Result<Option<Playlist>>;

    /// Create a new playlist
    async fn create_playlist(&self, playlist: CreatePlaylist) -> Result<Playlist>;

    /// Add track to playlist
    async fn add_track_to_playlist(&self, playlist_id: i64, track_id: i64) -> Result<()>;

    /// Remove track from playlist
    async fn remove_track_from_playlist(&self, playlist_id: i64, track_id: i64) -> Result<()>;

    /// Delete playlist
    async fn delete_playlist(&self, id: i64) -> Result<()>;

    // ========================================================================
    // Play History & Stats
    // ========================================================================

    /// Record a play
    async fn record_play(&self, track_id: i64, duration_seconds: Option<f64>, completed: bool) -> Result<()>;

    /// Get recently played tracks
    async fn get_recently_played(&self, limit: i32) -> Result<Vec<Track>>;

    /// Get play count for a track
    async fn get_play_count(&self, track_id: i64) -> Result<i32>;
}
