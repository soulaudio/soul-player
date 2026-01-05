/// Database implementation
use crate::error::{Result, StorageError};
use soul_core::{
    Permission, Playlist, PlaylistId, PlaylistShare, Storage, Track, TrackId, User, UserId,
};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use sqlx::Row;
use std::path::Path;
use std::str::FromStr;

/// SQLite database with multi-user support
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    /// Create a new database connection
    ///
    /// # Errors
    /// Returns an error if the connection fails or migrations fail
    pub async fn new(database_url: &str) -> Result<Self> {
        let options = SqliteConnectOptions::from_str(database_url)?.create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;

        // Run migrations manually for reliability across different execution contexts
        Self::run_migrations(&pool).await?;

        Ok(Self { pool })
    }

    /// Create database from an existing pool (for testing)
    pub fn from_pool(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Create an in-memory database (for testing)
    #[cfg(test)]
    pub async fn in_memory() -> Result<Self> {
        Self::new("sqlite::memory:").await
    }

    /// Get a reference to the underlying pool (for testing)
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Run database migrations
    async fn run_migrations(pool: &SqlitePool) -> Result<()> {
        // Embedded migrations for reliability
        const MIGRATIONS: &[&str] = &[
            include_str!("../migrations/20250105000001_create_users.sql"),
            include_str!("../migrations/20250105000002_create_tracks.sql"),
            include_str!("../migrations/20250105000003_create_playlists.sql"),
            include_str!("../migrations/20250105000004_create_playlist_tracks.sql"),
            include_str!("../migrations/20250105000005_create_playlist_shares.sql"),
            include_str!("../migrations/20250105000006_create_user_credentials.sql"),
            include_str!("../migrations/20250105000007_create_track_variants.sql"),
            include_str!("../migrations/20250105000008_create_sync_log.sql"),
        ];

        for migration in MIGRATIONS {
            sqlx::query(migration)
                .execute(pool)
                .await
                .map_err(|e| StorageError::Migration(e.to_string()))?;
        }

        Ok(())
    }
}

#[allow(async_fn_in_trait)]
impl Storage for Database {
    // User operations

    async fn create_user(&self, name: &str) -> soul_core::Result<User> {
        let user = User::new(name);

        sqlx::query("INSERT INTO users (id, name, created_at) VALUES (?, ?, ?)")
            .bind(user.id.as_str())
            .bind(&user.name)
            .bind(user.created_at.timestamp())
            .execute(&self.pool)
            .await
            .map_err(|e| soul_core::SoulError::storage(e.to_string()))?;

        Ok(user)
    }

    async fn get_user(&self, id: &UserId) -> soul_core::Result<User> {
        let row = sqlx::query("SELECT id, name, created_at FROM users WHERE id = ?")
            .bind(id.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| soul_core::SoulError::storage(e.to_string()))?
            .ok_or_else(|| soul_core::SoulError::not_found("User", id.as_str()))?;

        Ok(User::with_id(
            UserId::new(row.get::<String, _>("id")),
            row.get::<String, _>("name"),
            chrono::DateTime::from_timestamp(row.get::<i64, _>("created_at"), 0)
                .ok_or_else(|| soul_core::SoulError::storage("Invalid timestamp"))?,
        ))
    }

    async fn get_all_users(&self) -> soul_core::Result<Vec<User>> {
        let rows = sqlx::query("SELECT id, name, created_at FROM users ORDER BY name")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| soul_core::SoulError::storage(e.to_string()))?;

        rows.iter()
            .map(|row| {
                Ok(User::with_id(
                    UserId::new(row.get::<String, _>("id")),
                    row.get::<String, _>("name"),
                    chrono::DateTime::from_timestamp(row.get::<i64, _>("created_at"), 0)
                        .ok_or_else(|| soul_core::SoulError::storage("Invalid timestamp"))?,
                ))
            })
            .collect()
    }

    // Track operations

    async fn add_track(&self, track: Track) -> soul_core::Result<TrackId> {
        sqlx::query(
            "INSERT INTO tracks (id, title, artist, album, album_artist, track_number, disc_number, year, genre, duration_ms, file_path, file_hash, added_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(track.id.as_str())
        .bind(&track.title)
        .bind(&track.artist)
        .bind(&track.album)
        .bind(&track.album_artist)
        .bind(track.track_number.map(|n| n as i64))
        .bind(track.disc_number.map(|n| n as i64))
        .bind(track.year.map(|n| n as i64))
        .bind(&track.genre)
        .bind(track.duration_ms.map(|d| d as i64))
        .bind(track.file_path.to_string_lossy().to_string())
        .bind(&track.file_hash)
        .bind(track.added_at.timestamp())
        .execute(&self.pool)
        .await
        .map_err(|e| soul_core::SoulError::storage(e.to_string()))?;

        Ok(track.id)
    }

    async fn get_track(&self, id: &TrackId) -> soul_core::Result<Track> {
        let row = sqlx::query(
            "SELECT id, title, artist, album, album_artist, track_number, disc_number, year, genre, duration_ms, file_path, file_hash, added_at
             FROM tracks WHERE id = ?"
        )
        .bind(id.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| soul_core::SoulError::storage(e.to_string()))?
        .ok_or_else(|| soul_core::SoulError::not_found("Track", id.as_str()))?;

        Ok(Track {
            id: TrackId::new(row.get::<String, _>("id")),
            title: row.get("title"),
            artist: row.get("artist"),
            album: row.get("album"),
            album_artist: row.get("album_artist"),
            track_number: row.get::<Option<i64>, _>("track_number").map(|n| n as u32),
            disc_number: row.get::<Option<i64>, _>("disc_number").map(|n| n as u32),
            year: row.get::<Option<i64>, _>("year").map(|n| n as u32),
            genre: row.get("genre"),
            duration_ms: row.get::<Option<i64>, _>("duration_ms").map(|d| d as u64),
            file_path: Path::new(&row.get::<String, _>("file_path")).to_path_buf(),
            file_hash: row.get("file_hash"),
            added_at: chrono::DateTime::from_timestamp(row.get::<i64, _>("added_at"), 0)
                .ok_or_else(|| soul_core::SoulError::storage("Invalid timestamp"))?,
        })
    }

    async fn get_all_tracks(&self) -> soul_core::Result<Vec<Track>> {
        let rows = sqlx::query(
            "SELECT id, title, artist, album, album_artist, track_number, disc_number, year, genre, duration_ms, file_path, file_hash, added_at
             FROM tracks ORDER BY title"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| soul_core::SoulError::storage(e.to_string()))?;

        rows.iter()
            .map(|row| {
                Ok(Track {
                    id: TrackId::new(row.get::<String, _>("id")),
                    title: row.get("title"),
                    artist: row.get("artist"),
                    album: row.get("album"),
                    album_artist: row.get("album_artist"),
                    track_number: row.get::<Option<i64>, _>("track_number").map(|n| n as u32),
                    disc_number: row.get::<Option<i64>, _>("disc_number").map(|n| n as u32),
                    year: row.get::<Option<i64>, _>("year").map(|n| n as u32),
                    genre: row.get("genre"),
                    duration_ms: row.get::<Option<i64>, _>("duration_ms").map(|d| d as u64),
                    file_path: Path::new(&row.get::<String, _>("file_path")).to_path_buf(),
                    file_hash: row.get("file_hash"),
                    added_at: chrono::DateTime::from_timestamp(row.get::<i64, _>("added_at"), 0)
                        .ok_or_else(|| soul_core::SoulError::storage("Invalid timestamp"))?,
                })
            })
            .collect()
    }

    async fn search_tracks(&self, query: &str) -> soul_core::Result<Vec<Track>> {
        let search_pattern = format!("%{}%", query);

        let rows = sqlx::query(
            "SELECT id, title, artist, album, album_artist, track_number, disc_number, year, genre, duration_ms, file_path, file_hash, added_at
             FROM tracks
             WHERE title LIKE ? OR artist LIKE ? OR album LIKE ?
             ORDER BY title"
        )
        .bind(&search_pattern)
        .bind(&search_pattern)
        .bind(&search_pattern)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| soul_core::SoulError::storage(e.to_string()))?;

        rows.iter()
            .map(|row| {
                Ok(Track {
                    id: TrackId::new(row.get::<String, _>("id")),
                    title: row.get("title"),
                    artist: row.get("artist"),
                    album: row.get("album"),
                    album_artist: row.get("album_artist"),
                    track_number: row.get::<Option<i64>, _>("track_number").map(|n| n as u32),
                    disc_number: row.get::<Option<i64>, _>("disc_number").map(|n| n as u32),
                    year: row.get::<Option<i64>, _>("year").map(|n| n as u32),
                    genre: row.get("genre"),
                    duration_ms: row.get::<Option<i64>, _>("duration_ms").map(|d| d as u64),
                    file_path: Path::new(&row.get::<String, _>("file_path")).to_path_buf(),
                    file_hash: row.get("file_hash"),
                    added_at: chrono::DateTime::from_timestamp(row.get::<i64, _>("added_at"), 0)
                        .ok_or_else(|| soul_core::SoulError::storage("Invalid timestamp"))?,
                })
            })
            .collect()
    }

    async fn delete_track(&self, id: &TrackId) -> soul_core::Result<()> {
        let result = sqlx::query("DELETE FROM tracks WHERE id = ?")
            .bind(id.as_str())
            .execute(&self.pool)
            .await
            .map_err(|e| soul_core::SoulError::storage(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(soul_core::SoulError::not_found("Track", id.as_str()));
        }

        Ok(())
    }

    // Playlist operations

    async fn create_playlist(&self, user_id: &UserId, name: &str) -> soul_core::Result<Playlist> {
        let playlist = Playlist::new(user_id.clone(), name);

        sqlx::query("INSERT INTO playlists (id, owner_id, name, created_at) VALUES (?, ?, ?, ?)")
            .bind(playlist.id.as_str())
            .bind(user_id.as_str())
            .bind(&playlist.name)
            .bind(playlist.created_at.timestamp())
            .execute(&self.pool)
            .await
            .map_err(|e| soul_core::SoulError::storage(e.to_string()))?;

        Ok(playlist)
    }

    async fn get_playlist(&self, id: &PlaylistId) -> soul_core::Result<Playlist> {
        let row = sqlx::query("SELECT id, owner_id, name, created_at FROM playlists WHERE id = ?")
            .bind(id.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| soul_core::SoulError::storage(e.to_string()))?
            .ok_or_else(|| soul_core::SoulError::not_found("Playlist", id.as_str()))?;

        Ok(Playlist::with_id(
            PlaylistId::new(row.get::<String, _>("id")),
            UserId::new(row.get::<String, _>("owner_id")),
            row.get::<String, _>("name"),
            chrono::DateTime::from_timestamp(row.get::<i64, _>("created_at"), 0)
                .ok_or_else(|| soul_core::SoulError::storage("Invalid timestamp"))?,
        ))
    }

    async fn get_user_playlists(&self, user_id: &UserId) -> soul_core::Result<Vec<Playlist>> {
        let rows = sqlx::query(
            "SELECT id, owner_id, name, created_at FROM playlists WHERE owner_id = ? ORDER BY name",
        )
        .bind(user_id.as_str())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| soul_core::SoulError::storage(e.to_string()))?;

        rows.iter()
            .map(|row| {
                Ok(Playlist::with_id(
                    PlaylistId::new(row.get::<String, _>("id")),
                    UserId::new(row.get::<String, _>("owner_id")),
                    row.get::<String, _>("name"),
                    chrono::DateTime::from_timestamp(row.get::<i64, _>("created_at"), 0)
                        .ok_or_else(|| soul_core::SoulError::storage("Invalid timestamp"))?,
                ))
            })
            .collect()
    }

    async fn get_accessible_playlists(&self, user_id: &UserId) -> soul_core::Result<Vec<Playlist>> {
        let rows = sqlx::query(
            "SELECT id, owner_id, name, created_at FROM playlists
             WHERE owner_id = ? OR id IN (
                SELECT playlist_id FROM playlist_shares WHERE shared_with_user_id = ?
             )
             ORDER BY name",
        )
        .bind(user_id.as_str())
        .bind(user_id.as_str())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| soul_core::SoulError::storage(e.to_string()))?;

        rows.iter()
            .map(|row| {
                Ok(Playlist::with_id(
                    PlaylistId::new(row.get::<String, _>("id")),
                    UserId::new(row.get::<String, _>("owner_id")),
                    row.get::<String, _>("name"),
                    chrono::DateTime::from_timestamp(row.get::<i64, _>("created_at"), 0)
                        .ok_or_else(|| soul_core::SoulError::storage("Invalid timestamp"))?,
                ))
            })
            .collect()
    }

    async fn add_track_to_playlist(
        &self,
        playlist_id: &PlaylistId,
        track_id: &TrackId,
    ) -> soul_core::Result<()> {
        // Get the current max position
        let max_position: Option<i64> =
            sqlx::query_scalar("SELECT MAX(position) FROM playlist_tracks WHERE playlist_id = ?")
                .bind(playlist_id.as_str())
                .fetch_one(&self.pool)
                .await
                .map_err(|e| soul_core::SoulError::storage(e.to_string()))?;

        let position = max_position.map_or(0, |p| p + 1);

        sqlx::query(
            "INSERT INTO playlist_tracks (playlist_id, track_id, position, added_at) VALUES (?, ?, ?, ?)"
        )
        .bind(playlist_id.as_str())
        .bind(track_id.as_str())
        .bind(position)
        .bind(chrono::Utc::now().timestamp())
        .execute(&self.pool)
        .await
        .map_err(|e| soul_core::SoulError::storage(e.to_string()))?;

        Ok(())
    }

    async fn get_playlist_tracks(&self, playlist_id: &PlaylistId) -> soul_core::Result<Vec<Track>> {
        let rows = sqlx::query(
            "SELECT t.id, t.title, t.artist, t.album, t.album_artist, t.track_number, t.disc_number, t.year, t.genre, t.duration_ms, t.file_path, t.file_hash, t.added_at
             FROM tracks t
             INNER JOIN playlist_tracks pt ON t.id = pt.track_id
             WHERE pt.playlist_id = ?
             ORDER BY pt.position"
        )
        .bind(playlist_id.as_str())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| soul_core::SoulError::storage(e.to_string()))?;

        rows.iter()
            .map(|row| {
                Ok(Track {
                    id: TrackId::new(row.get::<String, _>("id")),
                    title: row.get("title"),
                    artist: row.get("artist"),
                    album: row.get("album"),
                    album_artist: row.get("album_artist"),
                    track_number: row.get::<Option<i64>, _>("track_number").map(|n| n as u32),
                    disc_number: row.get::<Option<i64>, _>("disc_number").map(|n| n as u32),
                    year: row.get::<Option<i64>, _>("year").map(|n| n as u32),
                    genre: row.get("genre"),
                    duration_ms: row.get::<Option<i64>, _>("duration_ms").map(|d| d as u64),
                    file_path: Path::new(&row.get::<String, _>("file_path")).to_path_buf(),
                    file_hash: row.get("file_hash"),
                    added_at: chrono::DateTime::from_timestamp(row.get::<i64, _>("added_at"), 0)
                        .ok_or_else(|| soul_core::SoulError::storage("Invalid timestamp"))?,
                })
            })
            .collect()
    }

    async fn remove_track_from_playlist(
        &self,
        playlist_id: &PlaylistId,
        track_id: &TrackId,
    ) -> soul_core::Result<()> {
        sqlx::query("DELETE FROM playlist_tracks WHERE playlist_id = ? AND track_id = ?")
            .bind(playlist_id.as_str())
            .bind(track_id.as_str())
            .execute(&self.pool)
            .await
            .map_err(|e| soul_core::SoulError::storage(e.to_string()))?;

        Ok(())
    }

    async fn delete_playlist(&self, id: &PlaylistId) -> soul_core::Result<()> {
        let result = sqlx::query("DELETE FROM playlists WHERE id = ?")
            .bind(id.as_str())
            .execute(&self.pool)
            .await
            .map_err(|e| soul_core::SoulError::storage(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(soul_core::SoulError::not_found("Playlist", id.as_str()));
        }

        Ok(())
    }

    async fn share_playlist(
        &self,
        playlist_id: &PlaylistId,
        shared_with_user_id: &UserId,
        permission: Permission,
    ) -> soul_core::Result<()> {
        sqlx::query(
            "INSERT INTO playlist_shares (playlist_id, shared_with_user_id, permission, shared_at)
             VALUES (?, ?, ?, ?)",
        )
        .bind(playlist_id.as_str())
        .bind(shared_with_user_id.as_str())
        .bind(permission.as_str())
        .bind(chrono::Utc::now().timestamp())
        .execute(&self.pool)
        .await
        .map_err(|e| soul_core::SoulError::storage(e.to_string()))?;

        Ok(())
    }

    async fn get_playlist_shares(
        &self,
        playlist_id: &PlaylistId,
    ) -> soul_core::Result<Vec<PlaylistShare>> {
        let rows = sqlx::query(
            "SELECT playlist_id, shared_with_user_id, permission, shared_at
             FROM playlist_shares WHERE playlist_id = ?",
        )
        .bind(playlist_id.as_str())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| soul_core::SoulError::storage(e.to_string()))?;

        rows.iter()
            .map(|row| {
                let permission_str = row.get::<String, _>("permission");
                let permission = Permission::from_str(&permission_str).ok_or_else(|| {
                    soul_core::SoulError::storage(format!("Invalid permission: {}", permission_str))
                })?;

                Ok(PlaylistShare {
                    playlist_id: PlaylistId::new(row.get::<String, _>("playlist_id")),
                    shared_with_user_id: UserId::new(row.get::<String, _>("shared_with_user_id")),
                    permission,
                    shared_at: chrono::DateTime::from_timestamp(row.get::<i64, _>("shared_at"), 0)
                        .ok_or_else(|| soul_core::SoulError::storage("Invalid timestamp"))?,
                })
            })
            .collect()
    }

    async fn unshare_playlist(
        &self,
        playlist_id: &PlaylistId,
        user_id: &UserId,
    ) -> soul_core::Result<()> {
        sqlx::query(
            "DELETE FROM playlist_shares WHERE playlist_id = ? AND shared_with_user_id = ?",
        )
        .bind(playlist_id.as_str())
        .bind(user_id.as_str())
        .execute(&self.pool)
        .await
        .map_err(|e| soul_core::SoulError::storage(e.to_string()))?;

        Ok(())
    }
}
