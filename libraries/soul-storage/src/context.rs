use crate::{albums, artists, playlists, sources, tracks, users};
use async_trait::async_trait;
use soul_core::{error::Result, storage::StorageContext, types::*};
use sqlx::SqlitePool;

/// Local storage context using `SQLite`
pub struct LocalStorageContext {
    pool: SqlitePool,
    user_id: UserId,
}

impl LocalStorageContext {
    pub fn new(pool: SqlitePool, user_id: UserId) -> Self {
        Self { pool, user_id }
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

#[async_trait]
impl StorageContext for LocalStorageContext {
    fn user_id(&self) -> UserId {
        self.user_id.clone()
    }

    // Sources
    async fn get_sources(&self) -> Result<Vec<Source>> {
        sources::get_all(&self.pool).await
    }

    async fn get_source(&self, id: SourceId) -> Result<Option<Source>> {
        sources::get_by_id(&self.pool, id).await
    }

    async fn get_active_server(&self) -> Result<Option<Source>> {
        sources::get_active_server(&self.pool).await
    }

    async fn create_source(&self, source: CreateSource) -> Result<Source> {
        sources::create(&self.pool, source).await
    }

    async fn set_active_server(&self, id: SourceId) -> Result<()> {
        sources::set_active(&self.pool, id).await
    }

    async fn update_source_status(&self, id: SourceId, is_online: bool) -> Result<()> {
        sources::update_status(&self.pool, id, is_online).await
    }

    // Tracks
    async fn get_all_tracks(&self) -> Result<Vec<Track>> {
        tracks::get_all(&self.pool).await
    }

    async fn get_track_by_id(&self, id: TrackId) -> Result<Option<Track>> {
        tracks::get_by_id(&self.pool, id).await
    }

    async fn get_tracks_by_source(&self, source_id: SourceId) -> Result<Vec<Track>> {
        tracks::get_by_source(&self.pool, source_id).await
    }

    async fn get_tracks_by_artist(&self, artist_id: ArtistId) -> Result<Vec<Track>> {
        tracks::get_by_artist(&self.pool, artist_id).await
    }

    async fn get_tracks_by_album(&self, album_id: AlbumId) -> Result<Vec<Track>> {
        tracks::get_by_album(&self.pool, album_id).await
    }

    async fn create_track(&self, track: CreateTrack) -> Result<Track> {
        tracks::create(&self.pool, track).await
    }

    async fn update_track(&self, id: TrackId, track: UpdateTrack) -> Result<Track> {
        tracks::update(&self.pool, id, track).await
    }

    async fn delete_track(&self, id: TrackId) -> Result<()> {
        tracks::delete(&self.pool, id).await
    }

    async fn get_track_availability(&self, track_id: TrackId) -> Result<Vec<TrackAvailability>> {
        tracks::get_availability(&self.pool, track_id).await
    }

    // Artists
    async fn get_all_artists(&self) -> Result<Vec<Artist>> {
        artists::get_all(&self.pool).await
    }

    async fn get_artist_by_id(&self, id: ArtistId) -> Result<Option<Artist>> {
        artists::get_by_id(&self.pool, id).await
    }

    async fn find_artist_by_name(&self, name: &str) -> Result<Option<Artist>> {
        artists::find_by_name(&self.pool, name).await
    }

    async fn create_artist(&self, artist: CreateArtist) -> Result<Artist> {
        artists::create(&self.pool, artist).await
    }

    // Albums
    async fn get_all_albums(&self) -> Result<Vec<Album>> {
        albums::get_all(&self.pool).await
    }

    async fn get_album_by_id(&self, id: AlbumId) -> Result<Option<Album>> {
        albums::get_by_id(&self.pool, id).await
    }

    async fn get_albums_by_artist(&self, artist_id: ArtistId) -> Result<Vec<Album>> {
        albums::get_by_artist(&self.pool, artist_id).await
    }

    async fn create_album(&self, album: CreateAlbum) -> Result<Album> {
        albums::create(&self.pool, album).await
    }

    // Playlists
    async fn get_user_playlists(&self) -> Result<Vec<Playlist>> {
        playlists::get_user_playlists(&self.pool, self.user_id.clone()).await
    }

    async fn get_playlist_by_id(&self, id: PlaylistId) -> Result<Option<Playlist>> {
        playlists::get_by_id(&self.pool, id, self.user_id.clone()).await
    }

    async fn get_playlist_with_tracks(&self, id: PlaylistId) -> Result<Option<Playlist>> {
        playlists::get_with_tracks(&self.pool, id, self.user_id.clone()).await
    }

    async fn create_playlist(&self, playlist: CreatePlaylist) -> Result<Playlist> {
        playlists::create(&self.pool, playlist).await
    }

    async fn add_track_to_playlist(
        &self,
        playlist_id: PlaylistId,
        track_id: TrackId,
    ) -> Result<()> {
        playlists::add_track(&self.pool, playlist_id, track_id, self.user_id.clone()).await
    }

    async fn remove_track_from_playlist(
        &self,
        playlist_id: PlaylistId,
        track_id: TrackId,
    ) -> Result<()> {
        playlists::remove_track(&self.pool, playlist_id, track_id, self.user_id.clone()).await
    }

    async fn delete_playlist(&self, id: PlaylistId) -> Result<()> {
        playlists::delete(&self.pool, id, self.user_id.clone()).await
    }

    // Play History & Stats
    async fn record_play(
        &self,
        track_id: TrackId,
        duration_seconds: Option<f64>,
        completed: bool,
    ) -> Result<()> {
        tracks::record_play(
            &self.pool,
            self.user_id.clone(),
            track_id,
            duration_seconds,
            completed,
        )
        .await
    }

    async fn get_recently_played(&self, limit: i32) -> Result<Vec<Track>> {
        tracks::get_recently_played(&self.pool, self.user_id.clone(), limit).await
    }

    async fn get_play_count(&self, track_id: TrackId) -> Result<i32> {
        tracks::get_play_count(&self.pool, track_id).await
    }

    async fn search_tracks(&self, query: &str) -> Result<Vec<Track>> {
        tracks::search(&self.pool, query).await
    }

    async fn get_all_users(&self) -> Result<Vec<User>> {
        users::get_all(&self.pool).await.map_err(Into::into)
    }
}
