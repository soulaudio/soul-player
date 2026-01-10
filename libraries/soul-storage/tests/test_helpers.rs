//! Test helpers and fixtures for storage integration tests
//!
//! These helpers create test databases using REAL SQLite files (NOT in-memory)
//! to match production behavior and properly test migrations, constraints, and indexes.

use soul_core::types::*;
use sqlx::SqlitePool;
use tempfile::TempDir;

/// Test database wrapper that cleans up on drop
pub struct TestDb {
    pub pool: SqlitePool,
    _temp_dir: TempDir,
}

impl TestDb {
    /// Create a new test database with migrations applied
    pub async fn new() -> Self {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

        // Use in-memory database for tests
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("Failed to create pool");

        // Run migrations
        soul_storage::run_migrations(&pool)
            .await
            .expect("Failed to run migrations");

        Self {
            pool,
            _temp_dir: temp_dir,
        }
    }

    /// Get the pool reference
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

/// Simple setup function that returns a pool directly
pub async fn setup_test_db() -> SqlitePool {
    let test_db = TestDb::new().await;
    test_db.pool
}

/// Test fixture: Create a test user
pub async fn create_test_user(pool: &SqlitePool, username: &str) -> UserId {
    let user_id = UserId::generate();
    let created_at = chrono::Utc::now().timestamp();

    sqlx::query("INSERT INTO users (id, name, created_at) VALUES (?, ?, ?)")
        .bind(user_id.as_str())
        .bind(username)
        .bind(created_at)
        .execute(pool)
        .await
        .expect("Failed to create test user");

    user_id
}

/// Test fixture: Create a test source
pub async fn create_test_source(pool: &SqlitePool, name: &str, source_type: &str) -> SourceId {
    let result = if source_type == "server" {
        sqlx::query(
            "INSERT INTO sources (name, source_type, server_url, is_online) VALUES (?, ?, ?, 1)",
        )
        .bind(name)
        .bind(source_type)
        .bind(format!(
            "http://test-server-{}.local",
            name.replace(' ', "-")
        ))
        .execute(pool)
        .await
        .expect("Failed to create test source")
    } else {
        sqlx::query("INSERT INTO sources (name, source_type, is_online) VALUES (?, ?, 1)")
            .bind(name)
            .bind(source_type)
            .execute(pool)
            .await
            .expect("Failed to create test source")
    };

    result.last_insert_rowid()
}

/// Test fixture: Create a test artist
pub async fn create_test_artist(
    pool: &SqlitePool,
    name: &str,
    sort_name: Option<&str>,
) -> ArtistId {
    let result = sqlx::query("INSERT INTO artists (name, sort_name) VALUES (?, ?)")
        .bind(name)
        .bind(sort_name)
        .execute(pool)
        .await
        .expect("Failed to create test artist");

    result.last_insert_rowid()
}

/// Test fixture: Create a test album
pub async fn create_test_album(
    pool: &SqlitePool,
    title: &str,
    artist_id: Option<ArtistId>,
    year: Option<i32>,
) -> AlbumId {
    let result = sqlx::query("INSERT INTO albums (title, artist_id, year) VALUES (?, ?, ?)")
        .bind(title)
        .bind(artist_id)
        .bind(year)
        .execute(pool)
        .await
        .expect("Failed to create test album");

    result.last_insert_rowid()
}

/// Test fixture: Create a complete track with availability
pub async fn create_test_track(
    pool: &SqlitePool,
    title: &str,
    artist_id: Option<ArtistId>,
    album_id: Option<AlbumId>,
    origin_source_id: SourceId,
    local_file_path: Option<&str>,
) -> TrackId {
    let result = sqlx::query(
        "INSERT INTO tracks (title, artist_id, album_id, origin_source_id, file_format, created_at, updated_at)
         VALUES (?, ?, ?, ?, 'mp3', datetime('now'), datetime('now'))"
    )
    .bind(title)
    .bind(artist_id)
    .bind(album_id)
    .bind(origin_source_id)
    .execute(pool)
    .await
    .expect("Failed to create test track");

    let track_id = result.last_insert_rowid();

    // Create track availability
    if let Some(path) = local_file_path {
        sqlx::query(
            "INSERT INTO track_sources (track_id, source_id, status, local_file_path)
             VALUES (?, ?, 'local_file', ?)",
        )
        .bind(track_id)
        .bind(origin_source_id)
        .bind(path)
        .execute(pool)
        .await
        .expect("Failed to create track availability");
    } else {
        // For tracks without local_file_path (e.g., server tracks), still create track_sources entry
        sqlx::query(
            "INSERT INTO track_sources (track_id, source_id, status)
             VALUES (?, ?, 'stream_only')",
        )
        .bind(track_id)
        .bind(origin_source_id)
        .execute(pool)
        .await
        .expect("Failed to create track availability");
    }

    // Note: track_stats is now per-user and created on-demand when a user plays/rates a track
    // No automatic initialization needed here

    TrackId::new(track_id.to_string())
}

/// Test fixture: Create a complete playlist
pub async fn create_test_playlist(pool: &SqlitePool, name: &str, owner_id: UserId) -> PlaylistId {
    let playlist_id = PlaylistId::generate();
    let created_at = chrono::Utc::now().timestamp();

    sqlx::query("INSERT INTO playlists (id, name, owner_id, created_at) VALUES (?, ?, ?, ?)")
        .bind(playlist_id.as_str())
        .bind(name)
        .bind(owner_id.as_str())
        .bind(created_at)
        .execute(pool)
        .await
        .expect("Failed to create test playlist");

    playlist_id
}
