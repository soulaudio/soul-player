use soul_storage::{create_pool, run_migrations, window_state::{self, WindowState}};

#[tokio::test]
async fn test_get_default_window_state() {
    let pool = create_pool("sqlite::memory:").await.unwrap();
    run_migrations(&pool).await.unwrap();

    sqlx::query("INSERT INTO users (id, name, created_at) VALUES ('1', 'Test User', 1234567890)")
        .execute(&pool)
        .await
        .unwrap();

    // Get state for user with no saved state
    let state = window_state::get_window_state(&pool, "1").await.unwrap();

    // Should return default values
    assert_eq!(state.width, 1200);
    assert_eq!(state.height, 800);
    assert_eq!(state.maximized, false);
    assert_eq!(state.x, None);
    assert_eq!(state.y, None);
}

#[tokio::test]
async fn test_save_and_load_window_state() {
    let pool = create_pool("sqlite::memory:").await.unwrap();
    run_migrations(&pool).await.unwrap();

    sqlx::query("INSERT INTO users (id, name, created_at) VALUES ('1', 'Test User', 1234567890)")
        .execute(&pool)
        .await
        .unwrap();

    // Save a window state
    let original_state = WindowState {
        x: Some(100),
        y: Some(200),
        width: 1400,
        height: 900,
        maximized: false,
        last_route: Some("/library".to_string()),
    };

    window_state::save_window_state(&pool, "1", &original_state)
        .await
        .unwrap();

    // Load it back
    let loaded_state = window_state::get_window_state(&pool, "1").await.unwrap();

    assert_eq!(loaded_state.x, Some(100));
    assert_eq!(loaded_state.y, Some(200));
    assert_eq!(loaded_state.width, 1400);
    assert_eq!(loaded_state.height, 900);
    assert_eq!(loaded_state.maximized, false);
    assert_eq!(loaded_state.last_route, Some("/library".to_string()));
}

#[tokio::test]
async fn test_update_window_state() {
    let pool = create_pool("sqlite::memory:").await.unwrap();
    run_migrations(&pool).await.unwrap();

    sqlx::query("INSERT INTO users (id, name, created_at) VALUES ('1', 'Test User', 1234567890)")
        .execute(&pool)
        .await
        .unwrap();

    // Save initial state
    let state1 = WindowState {
        x: Some(100),
        y: Some(200),
        width: 1200,
        height: 800,
        maximized: false,
        last_route: Some("/".to_string()),
    };
    window_state::save_window_state(&pool, "1", &state1).await.unwrap();

    // Update to new state
    let state2 = WindowState {
        x: Some(300),
        y: Some(400),
        width: 1600,
        height: 1000,
        maximized: true,
        last_route: Some("/playlists".to_string()),
    };
    window_state::save_window_state(&pool, "1", &state2).await.unwrap();

    // Verify updated state
    let loaded = window_state::get_window_state(&pool, "1").await.unwrap();

    assert_eq!(loaded.x, Some(300));
    assert_eq!(loaded.y, Some(400));
    assert_eq!(loaded.width, 1600);
    assert_eq!(loaded.height, 1000);
    assert_eq!(loaded.maximized, true);
    assert_eq!(loaded.last_route, Some("/playlists".to_string()));
}

#[tokio::test]
async fn test_maximized_state() {
    let pool = create_pool("sqlite::memory:").await.unwrap();
    run_migrations(&pool).await.unwrap();

    sqlx::query("INSERT INTO users (id, name, created_at) VALUES ('1', 'Test User', 1234567890)")
        .execute(&pool)
        .await
        .unwrap();

    // Save maximized state
    let state = WindowState {
        x: None,  // Position doesn't matter when maximized
        y: None,
        width: 1920,
        height: 1080,
        maximized: true,
        last_route: None,
    };

    window_state::save_window_state(&pool, "1", &state).await.unwrap();
    let loaded = window_state::get_window_state(&pool, "1").await.unwrap();

    assert_eq!(loaded.maximized, true);
}

#[tokio::test]
async fn test_multi_user_window_states() {
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

    // Save different states for each user
    let state1 = WindowState {
        x: Some(0),
        y: Some(0),
        width: 1200,
        height: 800,
        maximized: false,
        last_route: Some("/library".to_string()),
    };
    let state2 = WindowState {
        x: Some(100),
        y: Some(100),
        width: 1600,
        height: 1000,
        maximized: true,
        last_route: Some("/playlists".to_string()),
    };

    window_state::save_window_state(&pool, "1", &state1).await.unwrap();
    window_state::save_window_state(&pool, "2", &state2).await.unwrap();

    // Verify each user gets their own state
    let loaded1 = window_state::get_window_state(&pool, "1").await.unwrap();
    let loaded2 = window_state::get_window_state(&pool, "2").await.unwrap();

    assert_eq!(loaded1.width, 1200);
    assert_eq!(loaded1.maximized, false);
    assert_eq!(loaded2.width, 1600);
    assert_eq!(loaded2.maximized, true);
}

#[tokio::test]
async fn test_none_position_values() {
    let pool = create_pool("sqlite::memory:").await.unwrap();
    run_migrations(&pool).await.unwrap();

    sqlx::query("INSERT INTO users (id, name, created_at) VALUES ('1', 'Test User', 1234567890)")
        .execute(&pool)
        .await
        .unwrap();

    // Save state with no position (centered or first launch)
    let state = WindowState {
        x: None,
        y: None,
        width: 1200,
        height: 800,
        maximized: false,
        last_route: Some("/".to_string()),
    };

    window_state::save_window_state(&pool, "1", &state).await.unwrap();
    let loaded = window_state::get_window_state(&pool, "1").await.unwrap();

    assert_eq!(loaded.x, None);
    assert_eq!(loaded.y, None);
}
