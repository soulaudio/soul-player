/// Core traits for Soul Player
use crate::error::Result;
use crate::types::{
    AudioBuffer, Permission, Playlist, PlaylistId, PlaylistShare, Track, TrackId, TrackMetadata,
    User, UserId,
};
use std::path::Path;

/// Audio decoder trait
///
/// Implementers decode audio files into `AudioBuffer` format
pub trait AudioDecoder: Send {
    /// Decode an audio file from the given path
    ///
    /// # Errors
    /// Returns an error if the file cannot be read or decoded
    fn decode(&mut self, path: &Path) -> Result<AudioBuffer>;

    /// Check if the decoder supports the given file format
    fn supports_format(&self, path: &Path) -> bool;
}

/// Audio output trait
///
/// Implementers play audio buffers to output devices
pub trait AudioOutput: Send {
    /// Play an audio buffer
    ///
    /// # Errors
    /// Returns an error if playback fails
    fn play(&mut self, buffer: &AudioBuffer) -> Result<()>;

    /// Pause playback
    fn pause(&mut self) -> Result<()>;

    /// Resume playback
    fn resume(&mut self) -> Result<()>;

    /// Stop playback and clear the buffer
    fn stop(&mut self) -> Result<()>;

    /// Set the volume (0.0 = silent, 1.0 = full volume)
    fn set_volume(&mut self, volume: f32) -> Result<()>;

    /// Get the current volume
    fn volume(&self) -> f32;
}

/// Audio effect trait
///
/// Implementers process audio buffers in real-time
///
/// **CRITICAL**: No allocations allowed in `process` method!
pub trait AudioEffect: Send {
    /// Process audio samples in-place
    ///
    /// # Parameters
    /// - `buffer`: Audio samples to process (modified in-place)
    /// - `sample_rate`: Sample rate in Hz
    ///
    /// # Safety
    /// This method is called in the audio thread and must be real-time safe:
    /// - No allocations
    /// - No locks
    /// - No blocking I/O
    fn process(&mut self, buffer: &mut [f32], sample_rate: u32);

    /// Reset the effect state
    fn reset(&mut self);
}

/// Storage trait
///
/// Implementers provide database operations for tracks, playlists, and users
#[allow(async_fn_in_trait)]
pub trait Storage: Send + Sync {
    // User operations

    /// Create a new user
    async fn create_user(&self, name: &str) -> Result<User>;

    /// Get a user by ID
    async fn get_user(&self, id: &UserId) -> Result<User>;

    /// Get all users
    async fn get_all_users(&self) -> Result<Vec<User>>;

    // Track operations

    /// Add a track to the library
    async fn add_track(&self, track: Track) -> Result<TrackId>;

    /// Get a track by ID
    async fn get_track(&self, id: &TrackId) -> Result<Track>;

    /// Get all tracks
    async fn get_all_tracks(&self) -> Result<Vec<Track>>;

    /// Search tracks by title, artist, or album
    async fn search_tracks(&self, query: &str) -> Result<Vec<Track>>;

    /// Delete a track
    async fn delete_track(&self, id: &TrackId) -> Result<()>;

    // Playlist operations

    /// Create a new playlist for a user
    async fn create_playlist(&self, user_id: &UserId, name: &str) -> Result<Playlist>;

    /// Get a playlist by ID
    async fn get_playlist(&self, id: &PlaylistId) -> Result<Playlist>;

    /// Get all playlists owned by a user
    async fn get_user_playlists(&self, user_id: &UserId) -> Result<Vec<Playlist>>;

    /// Get all playlists accessible to a user (owned + shared)
    async fn get_accessible_playlists(&self, user_id: &UserId) -> Result<Vec<Playlist>>;

    /// Add a track to a playlist
    async fn add_track_to_playlist(
        &self,
        playlist_id: &PlaylistId,
        track_id: &TrackId,
    ) -> Result<()>;

    /// Get all tracks in a playlist (ordered by position)
    async fn get_playlist_tracks(&self, playlist_id: &PlaylistId) -> Result<Vec<Track>>;

    /// Remove a track from a playlist
    async fn remove_track_from_playlist(
        &self,
        playlist_id: &PlaylistId,
        track_id: &TrackId,
    ) -> Result<()>;

    /// Delete a playlist
    async fn delete_playlist(&self, id: &PlaylistId) -> Result<()>;

    /// Share a playlist with another user
    async fn share_playlist(
        &self,
        playlist_id: &PlaylistId,
        shared_with_user_id: &UserId,
        permission: Permission,
    ) -> Result<()>;

    /// Get all shares for a playlist
    async fn get_playlist_shares(&self, playlist_id: &PlaylistId) -> Result<Vec<PlaylistShare>>;

    /// Unshare a playlist with a user
    async fn unshare_playlist(&self, playlist_id: &PlaylistId, user_id: &UserId) -> Result<()>;
}

/// Metadata reader trait
///
/// Implementers extract metadata from audio files
pub trait MetadataReader: Send {
    /// Read metadata from an audio file
    ///
    /// # Errors
    /// Returns an error if the file cannot be read or parsed
    fn read(&self, path: &Path) -> Result<TrackMetadata>;

    /// Write metadata to an audio file
    ///
    /// # Errors
    /// Returns an error if the file cannot be written
    fn write(&self, path: &Path, metadata: &TrackMetadata) -> Result<()>;
}
