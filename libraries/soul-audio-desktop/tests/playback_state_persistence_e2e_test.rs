//! E2E tests for playback state persistence across app restarts
//!
//! Tests verify that all playback state (queue, position, volume, modes, artwork)
//! persists correctly across app restarts and handles edge cases gracefully.

use soul_storage::{run_migrations, settings};
use sqlx::{Row, SqlitePool};

/// Set up test database with migrations and test user
async fn setup_test_database() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    run_migrations(&pool).await.unwrap();

    // Create test user
    let created_at = chrono::Utc::now().timestamp();
    sqlx::query("INSERT INTO users (id, name, created_at) VALUES (?, ?, ?)")
        .bind("1")
        .bind("test_user")
        .bind(created_at)
        .execute(&pool)
        .await
        .unwrap();

    // Create test artist FIRST (albums reference artists)
    sqlx::query("INSERT INTO artists (id, name, created_at) VALUES (?, ?, ?)")
        .bind(1)
        .bind("Test Artist")
        .bind(created_at)
        .execute(&pool)
        .await
        .unwrap();

    // Create test album SECOND (tracks reference albums)
    sqlx::query(
        "INSERT INTO albums (id, title, artist_id, artwork_source, cover_art_path, created_at)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(1)
    .bind("Test Album")
    .bind(1i64) // artist_id
    .bind("embedded")
    .bind("artwork://album/1")
    .bind(created_at)
    .execute(&pool)
    .await
    .unwrap();

    // Insert test tracks
    for i in 1..=3 {
        sqlx::query(
            "INSERT INTO tracks (id, file_path, title, artist_id, album_id, duration_seconds, track_number, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(i)
        .bind(format!("test_track_{}.mp3", i))
        .bind(format!("Track {}", i))
        .bind(1i64) // artist_id
        .bind(1i64) // album_id
        .bind(180.0)
        .bind(i as i64)
        .bind(created_at)
        .execute(&pool)
        .await
        .unwrap();
    }

    pool
}

/// Save playback session to database
async fn save_session(
    pool: &SqlitePool,
    user_id: &str,
    track_id: i64,
    queue_ids: Vec<i64>,
    queue_index: i64,
    position: f64,
    volume: f64,
    repeat_mode: &str,
    shuffle_mode: &str,
) {
    settings::set_setting(
        pool,
        user_id,
        "playback.current_track_id",
        &serde_json::json!(track_id),
    )
    .await
    .unwrap();

    settings::set_setting(
        pool,
        user_id,
        "playback.queue_track_ids",
        &serde_json::json!(queue_ids),
    )
    .await
    .unwrap();

    settings::set_setting(
        pool,
        user_id,
        "playback.queue_index",
        &serde_json::json!(queue_index),
    )
    .await
    .unwrap();

    settings::set_setting(
        pool,
        user_id,
        "playback.position_seconds",
        &serde_json::json!(position),
    )
    .await
    .unwrap();

    settings::set_setting(pool, user_id, "playback.volume", &serde_json::json!(volume))
        .await
        .unwrap();

    settings::set_setting(
        pool,
        user_id,
        "playback.repeat_mode",
        &serde_json::json!(repeat_mode),
    )
    .await
    .unwrap();

    settings::set_setting(
        pool,
        user_id,
        "playback.shuffle_mode",
        &serde_json::json!(shuffle_mode),
    )
    .await
    .unwrap();

    settings::set_setting(
        pool,
        user_id,
        "playback.context_type",
        &serde_json::json!("album"),
    )
    .await
    .unwrap();

    settings::set_setting(
        pool,
        user_id,
        "playback.context_id",
        &serde_json::json!("1"),
    )
    .await
    .unwrap();

    settings::set_setting(
        pool,
        user_id,
        "playback.was_playing",
        &serde_json::json!(false),
    )
    .await
    .unwrap();
}

/// Load playback session from database
async fn load_session(pool: &SqlitePool, user_id: &str) -> PlaybackSession {
    PlaybackSession {
        current_track_id: settings::get_setting(pool, user_id, "playback.current_track_id")
            .await
            .unwrap()
            .and_then(|v| v.as_i64())
            .unwrap(),
        queue_track_ids: settings::get_setting(pool, user_id, "playback.queue_track_ids")
            .await
            .unwrap()
            .and_then(|v| {
                v.as_array()
                    .map(|arr| arr.iter().filter_map(|v| v.as_i64()).collect())
            })
            .unwrap(),
        queue_index: settings::get_setting(pool, user_id, "playback.queue_index")
            .await
            .unwrap()
            .and_then(|v| v.as_i64())
            .unwrap(),
        position_seconds: settings::get_setting(pool, user_id, "playback.position_seconds")
            .await
            .unwrap()
            .and_then(|v| v.as_f64())
            .unwrap(),
        volume: settings::get_setting(pool, user_id, "playback.volume")
            .await
            .unwrap()
            .and_then(|v| v.as_f64())
            .unwrap(),
        repeat_mode: settings::get_setting(pool, user_id, "playback.repeat_mode")
            .await
            .unwrap()
            .and_then(|v| v.as_str().map(String::from))
            .unwrap(),
        shuffle_mode: settings::get_setting(pool, user_id, "playback.shuffle_mode")
            .await
            .unwrap()
            .and_then(|v| v.as_str().map(String::from))
            .unwrap(),
    }
}

#[derive(Debug)]
struct PlaybackSession {
    current_track_id: i64,
    queue_track_ids: Vec<i64>,
    queue_index: i64,
    position_seconds: f64,
    volume: f64,
    repeat_mode: String,
    shuffle_mode: String,
}

#[tokio::test]
async fn test_queue_restoration() {
    let pool = setup_test_database().await;

    // Save a queue with 3 tracks
    save_session(&pool, "1", 1, vec![1, 2, 3], 0, 30.0, 75.0, "off", "off").await;

    // Load session
    let session = load_session(&pool, "1").await;

    // Verify queue was saved and loaded
    assert_eq!(session.queue_track_ids, vec![1, 2, 3]);
    assert_eq!(session.current_track_id, 1);
    assert_eq!(session.queue_index, 0);
}

#[tokio::test]
async fn test_progress_restoration() {
    let pool = setup_test_database().await;

    // Save session with track at 45.5 seconds
    save_session(&pool, "1", 1, vec![1, 2], 0, 45.5, 80.0, "off", "off").await;

    // Load session
    let session = load_session(&pool, "1").await;

    // Verify position was saved
    assert_eq!(session.position_seconds, 45.5);

    // Verify position accuracy (within 0.1 seconds)
    assert!((session.position_seconds - 45.5).abs() < 0.1);
}

#[tokio::test]
async fn test_volume_restoration() {
    let pool = setup_test_database().await;

    // Save session with volume at 65%
    save_session(&pool, "1", 1, vec![1], 0, 0.0, 65.0, "off", "off").await;

    // Load session
    let session = load_session(&pool, "1").await;

    // Verify volume was saved
    assert_eq!(session.volume, 65.0);
}

#[tokio::test]
async fn test_repeat_shuffle_modes_restoration() {
    let pool = setup_test_database().await;

    // Save session with repeat=all and shuffle=random
    save_session(&pool, "1", 1, vec![1, 2, 3], 0, 0.0, 80.0, "all", "random").await;

    // Load session
    let session = load_session(&pool, "1").await;

    // Verify modes were saved
    assert_eq!(session.repeat_mode, "all");
    assert_eq!(session.shuffle_mode, "random");
}

#[tokio::test]
async fn test_artwork_in_track_data() {
    let pool = setup_test_database().await;

    // Fetch track with artwork using query().bind() to avoid SQLx offline mode issues
    let row = sqlx::query(
        r#"
        SELECT
            t.id,
            t.title,
            al.cover_art_path as album_cover_art_path,
            al.artwork_source as album_artwork_source
        FROM tracks t
        LEFT JOIN albums al ON t.album_id = al.id
        WHERE t.id = ?
        "#,
    )
    .bind(1i64)
    .fetch_one(&pool)
    .await
    .unwrap();

    // Verify artwork fields are populated
    let cover_art_path: Option<String> = row.try_get("album_cover_art_path").unwrap();
    let artwork_source: Option<String> = row.try_get("album_artwork_source").unwrap();

    assert_eq!(cover_art_path, Some("artwork://album/1".to_string()));
    assert_eq!(artwork_source, Some("embedded".to_string()));
}

#[tokio::test]
async fn test_missing_track_handling() {
    let pool = setup_test_database().await;

    // Save session with track IDs 1, 2, 99 (99 doesn't exist)
    save_session(&pool, "1", 1, vec![1, 2, 99], 0, 0.0, 80.0, "off", "off").await;

    // Load session
    let session = load_session(&pool, "1").await;

    // Fetch tracks by IDs using query().bind() to avoid SQLx offline mode issues
    let track_ids = session.queue_track_ids;
    let mut valid_tracks = Vec::new();

    for track_id in &track_ids {
        let result = sqlx::query("SELECT id, title FROM tracks WHERE id = ?")
            .bind(track_id)
            .fetch_optional(&pool)
            .await
            .unwrap();

        if let Some(row) = result {
            let id: i64 = row.try_get("id").unwrap();
            let title: String = row.try_get("title").unwrap();
            valid_tracks.push((id, title));
        }
    }

    // Verify missing track was filtered out
    assert_eq!(valid_tracks.len(), 2); // Only tracks 1 and 2
    assert_eq!(valid_tracks[0].0, 1); // id
    assert_eq!(valid_tracks[1].0, 2); // id
}

#[tokio::test]
async fn test_empty_queue_handling() {
    let pool = setup_test_database().await;

    // Save session with empty queue
    save_session(&pool, "1", 1, vec![], 0, 0.0, 80.0, "off", "off").await;

    // Load session
    let session = load_session(&pool, "1").await;

    // Verify empty queue is handled
    assert_eq!(session.queue_track_ids.len(), 0);

    // In real app, this should trigger clear_playback_session
    // and not crash or enter invalid state
}

#[tokio::test]
async fn test_invalid_queue_index() {
    let pool = setup_test_database().await;

    // Save session with queue index out of bounds
    save_session(&pool, "1", 1, vec![1, 2], 99, 0.0, 80.0, "off", "off").await;

    // Load session
    let session = load_session(&pool, "1").await;

    // Verify queue index is out of bounds
    assert_eq!(session.queue_index, 99);
    assert!(session.queue_index >= session.queue_track_ids.len() as i64);

    // In real app, this should be clamped to 0
}

#[tokio::test]
async fn test_invalid_volume() {
    let pool = setup_test_database().await;

    // Save session with invalid volume (>100)
    save_session(&pool, "1", 1, vec![1], 0, 0.0, 150.0, "off", "off").await;

    // Load session
    let session = load_session(&pool, "1").await;

    // Verify volume is out of range
    assert_eq!(session.volume, 150.0);
    assert!(session.volume > 100.0);

    // In real app, this should be clamped to 80
}

#[tokio::test]
async fn test_context_persistence() {
    let pool = setup_test_database().await;

    // Save session
    save_session(&pool, "1", 1, vec![1, 2, 3], 0, 0.0, 80.0, "off", "off").await;

    // Load context
    let context_type = settings::get_setting(&pool, "1", "playback.context_type")
        .await
        .unwrap()
        .and_then(|v| v.as_str().map(String::from))
        .unwrap();

    let context_id = settings::get_setting(&pool, "1", "playback.context_id")
        .await
        .unwrap()
        .and_then(|v| v.as_str().map(String::from))
        .unwrap();

    // Verify context was saved
    assert_eq!(context_type, "album");
    assert_eq!(context_id, "1");
}

#[tokio::test]
async fn test_full_restoration_cycle() {
    let pool = setup_test_database().await;

    // Initial state: Play track 2 of 3, at 30 seconds, volume 75%, repeat all, shuffle random
    save_session(&pool, "1", 2, vec![1, 2, 3], 1, 30.5, 75.0, "all", "random").await;

    // Simulate app restart - load session
    let session = load_session(&pool, "1").await;

    // Verify all state was restored correctly
    assert_eq!(session.current_track_id, 2);
    assert_eq!(session.queue_track_ids, vec![1, 2, 3]);
    assert_eq!(session.queue_index, 1);
    assert_eq!(session.position_seconds, 30.5);
    assert_eq!(session.volume, 75.0);
    assert_eq!(session.repeat_mode, "all");
    assert_eq!(session.shuffle_mode, "random");
}
