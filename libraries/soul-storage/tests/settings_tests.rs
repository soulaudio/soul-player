use soul_storage::{create_pool, run_migrations, settings};

#[tokio::test]
async fn test_set_and_get_setting() {
    let pool = create_pool("sqlite::memory:").await.unwrap();
    run_migrations(&pool).await.unwrap();

    // Create a test user
    sqlx::query("INSERT INTO users (id, name, created_at) VALUES ('1', 'Test User', 1234567890)")
        .execute(&pool)
        .await
        .unwrap();

    // Set a setting
    let value = serde_json::json!("dark");
    settings::set_setting(&pool, "1", settings::SETTING_THEME, &value)
        .await
        .unwrap();

    // Get the setting
    let result = settings::get_setting(&pool, "1", settings::SETTING_THEME)
        .await
        .unwrap();

    assert_eq!(result, Some(value));
}

#[tokio::test]
async fn test_get_non_existent_setting() {
    let pool = create_pool("sqlite::memory:").await.unwrap();
    run_migrations(&pool).await.unwrap();

    sqlx::query("INSERT INTO users (id, name, created_at) VALUES ('1', 'Test User', 1234567890)")
        .execute(&pool)
        .await
        .unwrap();

    let result = settings::get_setting(&pool, "1", "non_existent_key")
        .await
        .unwrap();

    assert_eq!(result, None);
}

#[tokio::test]
async fn test_update_existing_setting() {
    let pool = create_pool("sqlite::memory:").await.unwrap();
    run_migrations(&pool).await.unwrap();

    sqlx::query("INSERT INTO users (id, name, created_at) VALUES ('1', 'Test User', 1234567890)")
        .execute(&pool)
        .await
        .unwrap();

    // Set initial value
    let value1 = serde_json::json!("light");
    settings::set_setting(&pool, "1", settings::SETTING_THEME, &value1)
        .await
        .unwrap();

    // Update to new value
    let value2 = serde_json::json!("dark");
    settings::set_setting(&pool, "1", settings::SETTING_THEME, &value2)
        .await
        .unwrap();

    // Verify updated value
    let result = settings::get_setting(&pool, "1", settings::SETTING_THEME)
        .await
        .unwrap();

    assert_eq!(result, Some(value2));
}

#[tokio::test]
async fn test_get_all_settings() {
    let pool = create_pool("sqlite::memory:").await.unwrap();
    run_migrations(&pool).await.unwrap();

    sqlx::query("INSERT INTO users (id, name, created_at) VALUES ('1', 'Test User', 1234567890)")
        .execute(&pool)
        .await
        .unwrap();

    // Set multiple settings
    settings::set_setting(
        &pool,
        "1",
        settings::SETTING_THEME,
        &serde_json::json!("dark"),
    )
    .await
    .unwrap();
    settings::set_setting(
        &pool,
        "1",
        settings::SETTING_LOCALE,
        &serde_json::json!("en-US"),
    )
    .await
    .unwrap();
    settings::set_setting(&pool, "1", settings::SETTING_VOLUME, &serde_json::json!(75))
        .await
        .unwrap();

    // Get all settings
    let all_settings = settings::get_all_settings(&pool, "1").await.unwrap();

    assert_eq!(all_settings.len(), 3);
    assert!(all_settings
        .iter()
        .any(|s| s.key == settings::SETTING_THEME));
    assert!(all_settings
        .iter()
        .any(|s| s.key == settings::SETTING_LOCALE));
    assert!(all_settings
        .iter()
        .any(|s| s.key == settings::SETTING_VOLUME));
}

#[tokio::test]
async fn test_delete_setting() {
    let pool = create_pool("sqlite::memory:").await.unwrap();
    run_migrations(&pool).await.unwrap();

    sqlx::query("INSERT INTO users (id, name, created_at) VALUES ('1', 'Test User', 1234567890)")
        .execute(&pool)
        .await
        .unwrap();

    // Set a setting
    settings::set_setting(
        &pool,
        "1",
        settings::SETTING_THEME,
        &serde_json::json!("dark"),
    )
    .await
    .unwrap();

    // Delete the setting
    let deleted = settings::delete_setting(&pool, "1", settings::SETTING_THEME)
        .await
        .unwrap();

    assert!(deleted);

    // Verify it's gone
    let result = settings::get_setting(&pool, "1", settings::SETTING_THEME)
        .await
        .unwrap();

    assert_eq!(result, None);
}

#[tokio::test]
async fn test_delete_non_existent_setting() {
    let pool = create_pool("sqlite::memory:").await.unwrap();
    run_migrations(&pool).await.unwrap();

    sqlx::query("INSERT INTO users (id, name, created_at) VALUES ('1', 'Test User', 1234567890)")
        .execute(&pool)
        .await
        .unwrap();

    // Try to delete a non-existent setting
    let deleted = settings::delete_setting(&pool, "1", "non_existent_key")
        .await
        .unwrap();

    assert!(!deleted);
}

#[tokio::test]
async fn test_multi_user_isolation() {
    let pool = create_pool("sqlite::memory:").await.unwrap();
    run_migrations(&pool).await.unwrap();

    // Create two users
    sqlx::query("INSERT INTO users (id, name, created_at) VALUES ('1', 'User 1', 1234567890)")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO users (id, name, created_at) VALUES ('2', 'User 2', 1234567890)")
        .execute(&pool)
        .await
        .unwrap();

    // Set different themes for each user
    settings::set_setting(
        &pool,
        "1",
        settings::SETTING_THEME,
        &serde_json::json!("light"),
    )
    .await
    .unwrap();
    settings::set_setting(
        &pool,
        "2",
        settings::SETTING_THEME,
        &serde_json::json!("dark"),
    )
    .await
    .unwrap();

    // Verify each user gets their own setting
    let user1_theme = settings::get_setting(&pool, "1", settings::SETTING_THEME)
        .await
        .unwrap();
    let user2_theme = settings::get_setting(&pool, "2", settings::SETTING_THEME)
        .await
        .unwrap();

    assert_eq!(user1_theme, Some(serde_json::json!("light")));
    assert_eq!(user2_theme, Some(serde_json::json!("dark")));
}

#[tokio::test]
async fn test_json_value_types() {
    let pool = create_pool("sqlite::memory:").await.unwrap();
    run_migrations(&pool).await.unwrap();

    sqlx::query("INSERT INTO users (id, name, created_at) VALUES ('1', 'Test User', 1234567890)")
        .execute(&pool)
        .await
        .unwrap();

    // Test different JSON types
    let string_val = serde_json::json!("string value");
    let number_val = serde_json::json!(42);
    let bool_val = serde_json::json!(true);
    let object_val = serde_json::json!({"nested": "object"});
    let array_val = serde_json::json!([1, 2, 3]);

    settings::set_setting(&pool, "1", "key_string", &string_val)
        .await
        .unwrap();
    settings::set_setting(&pool, "1", "key_number", &number_val)
        .await
        .unwrap();
    settings::set_setting(&pool, "1", "key_bool", &bool_val)
        .await
        .unwrap();
    settings::set_setting(&pool, "1", "key_object", &object_val)
        .await
        .unwrap();
    settings::set_setting(&pool, "1", "key_array", &array_val)
        .await
        .unwrap();

    assert_eq!(
        settings::get_setting(&pool, "1", "key_string")
            .await
            .unwrap(),
        Some(string_val)
    );
    assert_eq!(
        settings::get_setting(&pool, "1", "key_number")
            .await
            .unwrap(),
        Some(number_val)
    );
    assert_eq!(
        settings::get_setting(&pool, "1", "key_bool").await.unwrap(),
        Some(bool_val)
    );
    assert_eq!(
        settings::get_setting(&pool, "1", "key_object")
            .await
            .unwrap(),
        Some(object_val)
    );
    assert_eq!(
        settings::get_setting(&pool, "1", "key_array")
            .await
            .unwrap(),
        Some(array_val)
    );
}
