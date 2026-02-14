//! Integration tests for playback state persistence

use soul_storage::settings;
use sqlx::SqlitePool;

#[tokio::test]
async fn test_save_and_restore_playback_session() {
    let pool = setup_test_database().await;

    // Save session
    let user_id = "1";
    settings::set_setting(
        &pool,
        user_id,
        "playback.current_track_id",
        &serde_json::json!(1),
    )
    .await
    .unwrap();
    settings::set_setting(
        &pool,
        user_id,
        "playback.queue_track_ids",
        &serde_json::json!(vec![1, 2]),
    )
    .await
    .unwrap();
    settings::set_setting(
        &pool,
        user_id,
        "playback.queue_index",
        &serde_json::json!(0),
    )
    .await
    .unwrap();
    settings::set_setting(&pool, user_id, "playback.volume", &serde_json::json!(0.75))
        .await
        .unwrap();
    settings::set_setting(
        &pool,
        user_id,
        "playback.repeat_mode",
        &serde_json::json!("all"),
    )
    .await
    .unwrap();

    // Restore session
    let current_track_id = settings::get_setting(&pool, user_id, "playback.current_track_id")
        .await
        .unwrap()
        .and_then(|v| v.as_i64())
        .unwrap();

    let volume = settings::get_setting(&pool, user_id, "playback.volume")
        .await
        .unwrap()
        .and_then(|v| v.as_f64())
        .unwrap();

    // Verify
    assert_eq!(current_track_id, 1);
    assert_eq!(volume, 0.75);
}

async fn setup_test_database() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();

    // Run migrations
    soul_storage::run_migrations(&pool).await.unwrap();

    // Create default user
    let created_at = chrono::Utc::now().timestamp();
    sqlx::query("INSERT INTO users (id, name, created_at) VALUES (?, ?, ?)")
        .bind("1")
        .bind("test_user")
        .bind(created_at)
        .execute(&pool)
        .await
        .unwrap();

    pool
}
