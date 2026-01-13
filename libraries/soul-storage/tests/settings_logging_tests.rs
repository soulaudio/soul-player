use soul_storage::{create_pool, run_migrations, settings};

#[tokio::test]
async fn test_logging_enabled_setting_default() {
    let pool = create_pool("sqlite::memory:").await.unwrap();
    run_migrations(&pool).await.unwrap();

    sqlx::query("INSERT INTO users (id, name, created_at) VALUES ('1', 'Test User', 1234567890)")
        .execute(&pool)
        .await
        .unwrap();

    // Default should be None (no preference set)
    let result = settings::get_logging_enabled(&pool, "1").await.unwrap();
    assert_eq!(result, None);
}

#[tokio::test]
async fn test_enable_logging() {
    let pool = create_pool("sqlite::memory:").await.unwrap();
    run_migrations(&pool).await.unwrap();

    sqlx::query("INSERT INTO users (id, name, created_at) VALUES ('1', 'Test User', 1234567890)")
        .execute(&pool)
        .await
        .unwrap();

    // Enable logging
    settings::set_setting(
        &pool,
        "1",
        settings::SETTING_LOGGING_ENABLED,
        &serde_json::json!(true),
    )
    .await
    .unwrap();

    let result = settings::get_logging_enabled(&pool, "1").await.unwrap();
    assert_eq!(result, Some(true));
}

#[tokio::test]
async fn test_disable_logging() {
    let pool = create_pool("sqlite::memory:").await.unwrap();
    run_migrations(&pool).await.unwrap();

    sqlx::query("INSERT INTO users (id, name, created_at) VALUES ('1', 'Test User', 1234567890)")
        .execute(&pool)
        .await
        .unwrap();

    // Disable logging explicitly
    settings::set_setting(
        &pool,
        "1",
        settings::SETTING_LOGGING_ENABLED,
        &serde_json::json!(false),
    )
    .await
    .unwrap();

    let result = settings::get_logging_enabled(&pool, "1").await.unwrap();
    assert_eq!(result, Some(false));
}

#[tokio::test]
async fn test_toggle_logging_multiple_times() {
    let pool = create_pool("sqlite::memory:").await.unwrap();
    run_migrations(&pool).await.unwrap();

    sqlx::query("INSERT INTO users (id, name, created_at) VALUES ('1', 'Test User', 1234567890)")
        .execute(&pool)
        .await
        .unwrap();

    // Enable -> Disable -> Enable
    settings::set_setting(
        &pool,
        "1",
        settings::SETTING_LOGGING_ENABLED,
        &serde_json::json!(true),
    )
    .await
    .unwrap();
    assert_eq!(
        settings::get_logging_enabled(&pool, "1").await.unwrap(),
        Some(true)
    );

    settings::set_setting(
        &pool,
        "1",
        settings::SETTING_LOGGING_ENABLED,
        &serde_json::json!(false),
    )
    .await
    .unwrap();
    assert_eq!(
        settings::get_logging_enabled(&pool, "1").await.unwrap(),
        Some(false)
    );

    settings::set_setting(
        &pool,
        "1",
        settings::SETTING_LOGGING_ENABLED,
        &serde_json::json!(true),
    )
    .await
    .unwrap();
    assert_eq!(
        settings::get_logging_enabled(&pool, "1").await.unwrap(),
        Some(true)
    );
}

#[tokio::test]
async fn test_logging_setting_multi_user_isolation() {
    let pool = create_pool("sqlite::memory:").await.unwrap();
    run_migrations(&pool).await.unwrap();

    sqlx::query("INSERT INTO users (id, name, created_at) VALUES ('1', 'User 1', 1234567890)")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO users (id, name, created_at) VALUES ('2', 'User 2', 1234567890)")
        .execute(&pool)
        .await
        .unwrap();

    // User 1 enables logging
    settings::set_setting(
        &pool,
        "1",
        settings::SETTING_LOGGING_ENABLED,
        &serde_json::json!(true),
    )
    .await
    .unwrap();

    // User 2 disables logging
    settings::set_setting(
        &pool,
        "2",
        settings::SETTING_LOGGING_ENABLED,
        &serde_json::json!(false),
    )
    .await
    .unwrap();

    // Verify isolation
    assert_eq!(
        settings::get_logging_enabled(&pool, "1").await.unwrap(),
        Some(true)
    );
    assert_eq!(
        settings::get_logging_enabled(&pool, "2").await.unwrap(),
        Some(false)
    );
}

#[tokio::test]
async fn test_delete_logging_preference() {
    let pool = create_pool("sqlite::memory:").await.unwrap();
    run_migrations(&pool).await.unwrap();

    sqlx::query("INSERT INTO users (id, name, created_at) VALUES ('1', 'Test User', 1234567890)")
        .execute(&pool)
        .await
        .unwrap();

    // Set preference
    settings::set_setting(
        &pool,
        "1",
        settings::SETTING_LOGGING_ENABLED,
        &serde_json::json!(true),
    )
    .await
    .unwrap();

    // Delete preference (reset to default)
    let deleted = settings::delete_setting(&pool, "1", settings::SETTING_LOGGING_ENABLED)
        .await
        .unwrap();
    assert!(deleted);

    // Should return None now
    let result = settings::get_logging_enabled(&pool, "1").await.unwrap();
    assert_eq!(result, None);
}
