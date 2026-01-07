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
    let result = sqlx::query("INSERT INTO users (name) VALUES (?)")
        .bind(username)
        .execute(pool)
        .await
        .expect("Failed to create test user");

    result.last_insert_rowid()
}

/// Test fixture: Create a test source
pub async fn create_test_source(pool: &SqlitePool, name: &str, source_type: &str) -> SourceId {
    let result = sqlx::query("INSERT INTO sources (name, source_type, is_online) VALUES (?, ?, 1)")
        .bind(name)
        .bind(source_type)
        .execute(pool)
        .await
        .expect("Failed to create test source");

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
) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    // Generate unique track ID using timestamp + random component
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_micros();
    let track_id = format!("track-{}-{}", timestamp, rand::random::<u32>());

    let file_path = local_file_path.unwrap_or("/test/path/song.mp3");
    let added_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    sqlx::query(
        "INSERT INTO tracks (id, title, artist_id, album_id, origin_source_id, file_format, file_path, added_at)
         VALUES (?, ?, ?, ?, ?, 'mp3', ?, ?)"
    )
    .bind(&track_id)
    .bind(title)
    .bind(artist_id)
    .bind(album_id)
    .bind(origin_source_id)
    .bind(file_path)
    .bind(added_at)
    .execute(pool)
    .await
    .expect("Failed to create test track");

    // Create track availability
    if let Some(path) = local_file_path {
        sqlx::query(
            "INSERT INTO track_sources (track_id, source_id, status, local_file_path)
             VALUES (?, ?, 'local_file', ?)",
        )
        .bind(&track_id)
        .bind(origin_source_id)
        .bind(path)
        .execute(pool)
        .await
        .expect("Failed to create track availability");
    }

    // Initialize stats
    sqlx::query(
        "INSERT INTO track_stats (track_id, play_count, skip_count)
         VALUES (?, 0, 0)",
    )
    .bind(&track_id)
    .execute(pool)
    .await
    .expect("Failed to create track stats");

    track_id
}

/// Test fixture: Create a complete playlist
pub async fn create_test_playlist(pool: &SqlitePool, name: &str, owner_id: UserId) -> PlaylistId {
    let result = sqlx::query("INSERT INTO playlists (name, owner_id) VALUES (?, ?)")
        .bind(name)
        .bind(owner_id)
        .execute(pool)
        .await
        .expect("Failed to create test playlist");

    result.last_insert_rowid()
}
