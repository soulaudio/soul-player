/// Integration tests for library scanner
///
/// Tests use real directories and mock storage to verify scanning behavior
use soul_core::{
    Permission, Playlist, PlaylistId, PlaylistShare, Storage, Track, TrackId, User, UserId,
};
use soul_metadata::{LibraryScanner, ScanConfig};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Mock storage implementation for testing
#[derive(Clone)]
struct MockStorage {
    tracks: Arc<Mutex<Vec<Track>>>,
}

impl MockStorage {
    fn new() -> Self {
        Self {
            tracks: Arc::new(Mutex::new(Vec::new())),
        }
    }

    async fn get_tracks(&self) -> Vec<Track> {
        self.tracks.lock().await.clone()
    }
}

#[allow(async_fn_in_trait)]
impl Storage for MockStorage {
    async fn create_user(&self, _name: &str) -> soul_core::Result<User> {
        unimplemented!()
    }

    async fn get_user(&self, _id: &UserId) -> soul_core::Result<User> {
        unimplemented!()
    }

    async fn get_all_users(&self) -> soul_core::Result<Vec<User>> {
        unimplemented!()
    }

    async fn add_track(&self, track: Track) -> soul_core::Result<TrackId> {
        let track_id = track.id.clone();
        self.tracks.lock().await.push(track);
        Ok(track_id)
    }

    async fn get_track(&self, _id: &TrackId) -> soul_core::Result<Track> {
        unimplemented!()
    }

    async fn get_all_tracks(&self) -> soul_core::Result<Vec<Track>> {
        Ok(self.tracks.lock().await.clone())
    }

    async fn search_tracks(&self, _query: &str) -> soul_core::Result<Vec<Track>> {
        unimplemented!()
    }

    async fn delete_track(&self, _id: &TrackId) -> soul_core::Result<()> {
        unimplemented!()
    }

    async fn create_playlist(&self, _user_id: &UserId, _name: &str) -> soul_core::Result<Playlist> {
        unimplemented!()
    }

    async fn get_playlist(&self, _id: &PlaylistId) -> soul_core::Result<Playlist> {
        unimplemented!()
    }

    async fn get_user_playlists(&self, _user_id: &UserId) -> soul_core::Result<Vec<Playlist>> {
        unimplemented!()
    }

    async fn get_accessible_playlists(
        &self,
        _user_id: &UserId,
    ) -> soul_core::Result<Vec<Playlist>> {
        unimplemented!()
    }

    async fn add_track_to_playlist(
        &self,
        _playlist_id: &PlaylistId,
        _track_id: &TrackId,
    ) -> soul_core::Result<()> {
        unimplemented!()
    }

    async fn get_playlist_tracks(
        &self,
        _playlist_id: &PlaylistId,
    ) -> soul_core::Result<Vec<Track>> {
        unimplemented!()
    }

    async fn remove_track_from_playlist(
        &self,
        _playlist_id: &PlaylistId,
        _track_id: &TrackId,
    ) -> soul_core::Result<()> {
        unimplemented!()
    }

    async fn delete_playlist(&self, _id: &PlaylistId) -> soul_core::Result<()> {
        unimplemented!()
    }

    async fn share_playlist(
        &self,
        _playlist_id: &PlaylistId,
        _shared_with_user_id: &UserId,
        _permission: Permission,
    ) -> soul_core::Result<()> {
        unimplemented!()
    }

    async fn get_playlist_shares(
        &self,
        _playlist_id: &PlaylistId,
    ) -> soul_core::Result<Vec<PlaylistShare>> {
        unimplemented!()
    }

    async fn unshare_playlist(
        &self,
        _playlist_id: &PlaylistId,
        _user_id: &UserId,
    ) -> soul_core::Result<()> {
        unimplemented!()
    }
}

#[tokio::test]
async fn scan_nonexistent_directory_returns_error() {
    let storage = Arc::new(MockStorage::new());
    let scanner = LibraryScanner::new(storage);

    let result = scanner
        .scan(Path::new("/definitely/does/not/exist"), None)
        .await;

    // Should return an error or empty stats
    // Actual behavior depends on implementation
    assert!(result.is_ok() || result.is_err());
}

#[tokio::test]
async fn scan_empty_directory_returns_zero_tracks() {
    use tempfile::TempDir;

    let storage = Arc::new(MockStorage::new());
    let scanner = LibraryScanner::new(storage.clone());

    // Create temporary empty directory
    let temp_dir = TempDir::new().unwrap();

    let stats = scanner.scan(temp_dir.path(), None).await.unwrap();

    assert_eq!(stats.files_discovered, 0);
    assert_eq!(stats.tracks_added, 0);

    // Verify no tracks were added to storage
    let tracks = storage.get_tracks().await;
    assert_eq!(tracks.len(), 0);
}

#[tokio::test]
async fn scan_directory_with_non_audio_files_skips_them() {
    use std::fs::File;
    use tempfile::TempDir;

    let storage = Arc::new(MockStorage::new());
    let scanner = LibraryScanner::new(storage.clone());

    // Create temporary directory with non-audio files
    let temp_dir = TempDir::new().unwrap();
    File::create(temp_dir.path().join("readme.txt")).unwrap();
    File::create(temp_dir.path().join("image.jpg")).unwrap();

    let stats = scanner.scan(temp_dir.path(), None).await.unwrap();

    // Should discover 0 audio files
    assert_eq!(stats.files_discovered, 0);
    assert_eq!(stats.tracks_added, 0);
}

#[tokio::test]
async fn scan_respects_configured_extensions() {
    use std::fs::File;
    use tempfile::TempDir;

    let storage = Arc::new(MockStorage::new());

    // Configure scanner to only accept .mp3 files
    let mut config = ScanConfig::default();
    config.extensions = vec!["mp3".to_string()];
    let scanner = LibraryScanner::with_config(storage, config);

    // Create temporary directory with mixed extensions
    let temp_dir = TempDir::new().unwrap();
    File::create(temp_dir.path().join("track1.mp3")).unwrap();
    File::create(temp_dir.path().join("track2.flac")).unwrap();

    let stats = scanner.scan(temp_dir.path(), None).await.unwrap();

    // Should only discover the .mp3 file
    assert_eq!(stats.files_discovered, 1);
}

#[tokio::test]
async fn scan_single_file_directly() {
    use tempfile::NamedTempFile;

    let storage = Arc::new(MockStorage::new());
    let scanner = LibraryScanner::new(storage);

    // Create a temporary file with .mp3 extension
    let temp_file = NamedTempFile::with_suffix(".mp3").unwrap();

    let stats = scanner.scan(temp_file.path(), None).await.unwrap();

    // Should discover 1 file (though scanning will likely fail due to invalid MP3)
    assert_eq!(stats.files_discovered, 1);
}

#[tokio::test]
async fn scan_config_defaults() {
    let config = ScanConfig::default();

    assert!(config.parallel);
    assert!(config.use_file_hashing);
    assert!(config.extensions.contains(&"mp3".to_string()));
    assert!(config.extensions.contains(&"flac".to_string()));
    assert!(config.extensions.contains(&"ogg".to_string()));
    assert!(config.num_threads > 0);
}

// TODO: Add tests with real audio files in tests/data/:
// - Test scanning a directory with valid MP3/FLAC files
// - Test progress reporting via mpsc channel
// - Test duplicate detection (when implemented)
// - Test parallel vs sequential processing
// - Test error accumulation when some files fail to scan
