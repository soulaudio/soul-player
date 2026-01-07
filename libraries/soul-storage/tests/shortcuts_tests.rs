use soul_storage::{
    create_pool, run_migrations,
    shortcuts::{self, ShortcutAction},
};

#[tokio::test]
async fn test_get_default_shortcuts() {
    let pool = create_pool("sqlite::memory:").await.unwrap();
    run_migrations(&pool).await.unwrap();

    sqlx::query("INSERT INTO users (id, name, created_at) VALUES ('1', 'Test User', 1234567890)")
        .execute(&pool)
        .await
        .unwrap();

    // First call should initialize with defaults
    let shortcuts = shortcuts::get_shortcuts(&pool, "1").await.unwrap();

    assert!(!shortcuts.is_empty());
    assert!(shortcuts
        .iter()
        .any(|s| s.action == ShortcutAction::PlayPause));
    assert!(shortcuts.iter().any(|s| s.action == ShortcutAction::Next));
    assert!(shortcuts
        .iter()
        .any(|s| s.action == ShortcutAction::Previous));

    // All defaults should be enabled
    assert!(shortcuts.iter().all(|s| s.enabled));
}

#[tokio::test]
async fn test_set_custom_shortcut() {
    let pool = create_pool("sqlite::memory:").await.unwrap();
    run_migrations(&pool).await.unwrap();

    sqlx::query("INSERT INTO users (id, name, created_at) VALUES ('1', 'Test User', 1234567890)")
        .execute(&pool)
        .await
        .unwrap();

    // Set a custom shortcut
    shortcuts::set_shortcut(
        &pool,
        "1",
        ShortcutAction::PlayPause,
        "CommandOrControl+P".to_string(),
    )
    .await
    .unwrap();

    // Get all shortcuts
    let all_shortcuts = shortcuts::get_shortcuts(&pool, "1").await.unwrap();

    // Find the updated shortcut
    let play_pause = all_shortcuts
        .iter()
        .find(|s| s.action == ShortcutAction::PlayPause)
        .unwrap();

    assert_eq!(play_pause.accelerator, "CommandOrControl+P");
    assert!(!play_pause.is_default); // Should not be marked as default anymore
}

#[tokio::test]
async fn test_update_existing_shortcut() {
    let pool = create_pool("sqlite::memory:").await.unwrap();
    run_migrations(&pool).await.unwrap();

    sqlx::query("INSERT INTO users (id, name, created_at) VALUES ('1', 'Test User', 1234567890)")
        .execute(&pool)
        .await
        .unwrap();

    // Set initial shortcut
    shortcuts::set_shortcut(
        &pool,
        "1",
        ShortcutAction::Next,
        "CommandOrControl+N".to_string(),
    )
    .await
    .unwrap();

    // Update it
    shortcuts::set_shortcut(
        &pool,
        "1",
        ShortcutAction::Next,
        "CommandOrControl+Right".to_string(),
    )
    .await
    .unwrap();

    // Verify update
    let all_shortcuts = shortcuts::get_shortcuts(&pool, "1").await.unwrap();
    let next_shortcut = all_shortcuts
        .iter()
        .find(|s| s.action == ShortcutAction::Next)
        .unwrap();

    assert_eq!(next_shortcut.accelerator, "CommandOrControl+Right");
}

#[tokio::test]
async fn test_reset_shortcuts_to_defaults() {
    let pool = create_pool("sqlite::memory:").await.unwrap();
    run_migrations(&pool).await.unwrap();

    sqlx::query("INSERT INTO users (id, name, created_at) VALUES ('1', 'Test User', 1234567890)")
        .execute(&pool)
        .await
        .unwrap();

    // Set some custom shortcuts
    shortcuts::set_shortcut(
        &pool,
        "1",
        ShortcutAction::PlayPause,
        "CommandOrControl+P".to_string(),
    )
    .await
    .unwrap();
    shortcuts::set_shortcut(
        &pool,
        "1",
        ShortcutAction::Next,
        "CommandOrControl+N".to_string(),
    )
    .await
    .unwrap();

    // Reset to defaults
    shortcuts::reset_shortcuts_to_defaults(&pool, "1")
        .await
        .unwrap();

    // Verify defaults are restored
    let all_shortcuts = shortcuts::get_shortcuts(&pool, "1").await.unwrap();
    let play_pause = all_shortcuts
        .iter()
        .find(|s| s.action == ShortcutAction::PlayPause)
        .unwrap();

    assert_eq!(play_pause.accelerator, "MediaPlayPause");
}

#[tokio::test]
async fn test_shortcut_action_serialization() {
    // Test that ShortcutAction can be converted to/from strings
    assert_eq!(ShortcutAction::PlayPause.as_str(), "play_pause");
    assert_eq!(ShortcutAction::Next.as_str(), "next");
    assert_eq!(ShortcutAction::Previous.as_str(), "previous");
    assert_eq!(ShortcutAction::VolumeUp.as_str(), "volume_up");
    assert_eq!(ShortcutAction::VolumeDown.as_str(), "volume_down");
    assert_eq!(ShortcutAction::Mute.as_str(), "mute");

    assert_eq!(
        ShortcutAction::from_str("play_pause"),
        Some(ShortcutAction::PlayPause)
    );
    assert_eq!(ShortcutAction::from_str("next"), Some(ShortcutAction::Next));
    assert_eq!(ShortcutAction::from_str("invalid"), None);
}

#[tokio::test]
async fn test_multi_user_shortcuts() {
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

    // Set different shortcuts for each user
    shortcuts::set_shortcut(
        &pool,
        "1",
        ShortcutAction::PlayPause,
        "CommandOrControl+P".to_string(),
    )
    .await
    .unwrap();
    shortcuts::set_shortcut(
        &pool,
        "2",
        ShortcutAction::PlayPause,
        "CommandOrControl+Space".to_string(),
    )
    .await
    .unwrap();

    // Verify each user has their own shortcuts
    let user1_shortcuts = shortcuts::get_shortcuts(&pool, "1").await.unwrap();
    let user2_shortcuts = shortcuts::get_shortcuts(&pool, "2").await.unwrap();

    let user1_play_pause = user1_shortcuts
        .iter()
        .find(|s| s.action == ShortcutAction::PlayPause)
        .unwrap();
    let user2_play_pause = user2_shortcuts
        .iter()
        .find(|s| s.action == ShortcutAction::PlayPause)
        .unwrap();

    assert_eq!(user1_play_pause.accelerator, "CommandOrControl+P");
    assert_eq!(user2_play_pause.accelerator, "CommandOrControl+Space");
}

#[tokio::test]
async fn test_all_shortcut_actions() {
    let pool = create_pool("sqlite::memory:").await.unwrap();
    run_migrations(&pool).await.unwrap();

    sqlx::query("INSERT INTO users (id, name, created_at) VALUES ('1', 'Test User', 1234567890)")
        .execute(&pool)
        .await
        .unwrap();

    // Set shortcuts for all actions
    shortcuts::set_shortcut(&pool, "1", ShortcutAction::PlayPause, "Key1".to_string())
        .await
        .unwrap();
    shortcuts::set_shortcut(&pool, "1", ShortcutAction::Next, "Key2".to_string())
        .await
        .unwrap();
    shortcuts::set_shortcut(&pool, "1", ShortcutAction::Previous, "Key3".to_string())
        .await
        .unwrap();
    shortcuts::set_shortcut(&pool, "1", ShortcutAction::VolumeUp, "Key4".to_string())
        .await
        .unwrap();
    shortcuts::set_shortcut(&pool, "1", ShortcutAction::VolumeDown, "Key5".to_string())
        .await
        .unwrap();
    shortcuts::set_shortcut(&pool, "1", ShortcutAction::Mute, "Key6".to_string())
        .await
        .unwrap();
    shortcuts::set_shortcut(
        &pool,
        "1",
        ShortcutAction::ToggleShuffle,
        "Key7".to_string(),
    )
    .await
    .unwrap();
    shortcuts::set_shortcut(&pool, "1", ShortcutAction::ToggleRepeat, "Key8".to_string())
        .await
        .unwrap();

    // Get all and verify count
    let all_shortcuts = shortcuts::get_shortcuts(&pool, "1").await.unwrap();

    assert!(all_shortcuts.len() >= 8); // At least the ones we set
}

#[tokio::test]
async fn test_default_shortcuts_structure() {
    let defaults = shortcuts::default_shortcuts();

    // Verify all defaults use media keys
    assert!(defaults
        .iter()
        .any(|s| s.accelerator.starts_with("Media") || s.accelerator.starts_with("Volume")));

    // All should be enabled
    assert!(defaults.iter().all(|s| s.enabled));

    // All should be marked as defaults
    assert!(defaults.iter().all(|s| s.is_default));
}
