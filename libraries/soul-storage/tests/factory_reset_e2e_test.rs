//! End-to-end tests for factory reset functionality
//!
//! Tests the complete factory reset flow including:
//! - Onboarding state detection
//! - Data deletion simulation
//! - Restoration to fresh state

mod test_helpers;

use soul_core::types::{CreateLibrarySource, ImportAction, UpdateManagedLibrarySettings};
use sqlx::SqlitePool;
use tempfile::TempDir;

/// Helper to delete all user configuration from database
/// Simulates what happens when the app data directory is deleted and recreated
async fn simulate_factory_reset(pool: &SqlitePool) {
    // Delete all user data tables (in dependency order)
    // Use ignore to handle tables that might not exist
    let _ = sqlx::query("DELETE FROM playlist_tracks")
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM playlists").execute(pool).await;
    let _ = sqlx::query("DELETE FROM track_stats").execute(pool).await;
    let _ = sqlx::query("DELETE FROM track_sources").execute(pool).await;
    let _ = sqlx::query("DELETE FROM tracks").execute(pool).await;
    let _ = sqlx::query("DELETE FROM albums").execute(pool).await;
    let _ = sqlx::query("DELETE FROM artists").execute(pool).await;
    let _ = sqlx::query("DELETE FROM scan_progress").execute(pool).await;
    let _ = sqlx::query("DELETE FROM library_sources")
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM managed_library_settings")
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM external_file_settings")
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM user_settings").execute(pool).await;
    let _ = sqlx::query("DELETE FROM keyboard_shortcuts")
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM window_state").execute(pool).await;
    let _ = sqlx::query("DELETE FROM users").execute(pool).await;
    let _ = sqlx::query("DELETE FROM sources").execute(pool).await;
    let _ = sqlx::query("DELETE FROM devices").execute(pool).await;
    let _ = sqlx::query("DELETE FROM playback_contexts")
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM user_playback_state")
        .execute(pool)
        .await;
}

// =============================================================================
// Factory Reset Flow Tests
// =============================================================================

/// Test complete factory reset flow with watched folder setup
#[tokio::test]
async fn test_factory_reset_watched_folder_flow() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    let user_id = "desktop_user";
    let device_id = "desktop_001";

    // Step 1: Fresh database should require onboarding
    let onboarding_needed =
        soul_storage::library_sources::check_onboarding_needed(&pool, user_id, device_id)
            .await
            .unwrap();
    assert!(
        onboarding_needed,
        "Fresh database should require onboarding"
    );

    // Step 2: User completes onboarding with watched folder
    let source = soul_storage::library_sources::create(
        &pool,
        user_id,
        device_id,
        &CreateLibrarySource {
            name: "My Music Library".to_string(),
            path: temp_dir.path().display().to_string(),
            sync_deletes: false,
        },
    )
    .await
    .unwrap();

    // Step 3: Verify onboarding is no longer needed
    let onboarding_needed =
        soul_storage::library_sources::check_onboarding_needed(&pool, user_id, device_id)
            .await
            .unwrap();
    assert!(
        !onboarding_needed,
        "Onboarding should not be needed after setup"
    );

    // Step 4: Add some user data (simulate actual usage)
    let artist = test_helpers::create_test_artist(&pool, "Test Artist", None).await;
    let album =
        test_helpers::create_test_album(&pool, "Test Album", Some(artist), Some(2024)).await;
    let local_source = test_helpers::create_test_source(&pool, "Local Device", "local").await;
    let _track = test_helpers::create_test_track(
        &pool,
        "Test Track",
        Some(artist),
        Some(album),
        local_source,
        Some("/path/to/track.flac"),
    )
    .await;

    // Step 5: Verify we have data
    let sources = soul_storage::library_sources::get_by_user_device(&pool, user_id, device_id)
        .await
        .unwrap();
    assert_eq!(sources.len(), 1, "Should have one library source");
    assert_eq!(sources[0].id, source.id);

    // Step 6: Simulate factory reset (delete all data)
    simulate_factory_reset(&pool).await;

    // Step 7: Verify database is empty (like a fresh install)
    let sources = soul_storage::library_sources::get_by_user_device(&pool, user_id, device_id)
        .await
        .unwrap();
    assert_eq!(sources.len(), 0, "All library sources should be deleted");

    let managed = soul_storage::managed_library_settings::get(&pool, user_id, device_id)
        .await
        .unwrap();
    assert!(
        managed.is_none(),
        "Managed library settings should be deleted"
    );

    // Step 8: Verify onboarding is needed again
    let onboarding_needed =
        soul_storage::library_sources::check_onboarding_needed(&pool, user_id, device_id)
            .await
            .unwrap();
    assert!(
        onboarding_needed,
        "Onboarding should be required after factory reset"
    );
}

/// Test factory reset with managed library setup
#[tokio::test]
async fn test_factory_reset_managed_library_flow() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    let user_id = "desktop_user";
    let device_id = "desktop_001";

    // Step 1: Fresh state requires onboarding
    let onboarding_needed =
        soul_storage::library_sources::check_onboarding_needed(&pool, user_id, device_id)
            .await
            .unwrap();
    assert!(onboarding_needed);

    // Step 2: User sets up managed library
    soul_storage::managed_library_settings::upsert(
        &pool,
        user_id,
        device_id,
        &UpdateManagedLibrarySettings {
            library_path: temp_dir.path().display().to_string(),
            path_template: "{AlbumArtist}/{Year} - {Album}/{TrackNo} - {Title}".to_string(),
            import_action: ImportAction::Copy,
        },
    )
    .await
    .unwrap();

    // Step 3: Verify onboarding complete
    let onboarding_needed =
        soul_storage::library_sources::check_onboarding_needed(&pool, user_id, device_id)
            .await
            .unwrap();
    assert!(!onboarding_needed);

    let managed = soul_storage::managed_library_settings::get(&pool, user_id, device_id)
        .await
        .unwrap();
    assert!(managed.is_some());

    // Step 4: Factory reset
    simulate_factory_reset(&pool).await;

    // Step 5: Verify reset to fresh state
    let onboarding_needed =
        soul_storage::library_sources::check_onboarding_needed(&pool, user_id, device_id)
            .await
            .unwrap();
    assert!(
        onboarding_needed,
        "Should require onboarding after factory reset"
    );

    let managed = soul_storage::managed_library_settings::get(&pool, user_id, device_id)
        .await
        .unwrap();
    assert!(managed.is_none(), "Managed settings should be deleted");
}

/// Test factory reset with hybrid setup (both watched and managed)
#[tokio::test]
async fn test_factory_reset_hybrid_setup_flow() {
    let pool = test_helpers::setup_test_db().await;
    let watched_dir = TempDir::new().unwrap();
    let managed_dir = TempDir::new().unwrap();

    let user_id = "desktop_user";
    let device_id = "desktop_001";

    // Step 1: Setup hybrid configuration
    soul_storage::library_sources::create(
        &pool,
        user_id,
        device_id,
        &CreateLibrarySource {
            name: "Watched Folder".to_string(),
            path: watched_dir.path().display().to_string(),
            sync_deletes: true,
        },
    )
    .await
    .unwrap();

    soul_storage::managed_library_settings::upsert(
        &pool,
        user_id,
        device_id,
        &UpdateManagedLibrarySettings {
            library_path: managed_dir.path().display().to_string(),
            path_template: "{Artist}/{Album}/{Title}".to_string(),
            import_action: ImportAction::Move,
        },
    )
    .await
    .unwrap();

    // Step 2: Verify both configurations exist
    let sources = soul_storage::library_sources::get_by_user_device(&pool, user_id, device_id)
        .await
        .unwrap();
    let managed = soul_storage::managed_library_settings::get(&pool, user_id, device_id)
        .await
        .unwrap();

    assert_eq!(sources.len(), 1);
    assert!(managed.is_some());

    let onboarding_needed =
        soul_storage::library_sources::check_onboarding_needed(&pool, user_id, device_id)
            .await
            .unwrap();
    assert!(!onboarding_needed, "Should not need onboarding with setup");

    // Step 3: Factory reset
    simulate_factory_reset(&pool).await;

    // Step 4: Verify complete reset
    let sources = soul_storage::library_sources::get_by_user_device(&pool, user_id, device_id)
        .await
        .unwrap();
    let managed = soul_storage::managed_library_settings::get(&pool, user_id, device_id)
        .await
        .unwrap();

    assert_eq!(sources.len(), 0, "All library sources deleted");
    assert!(managed.is_none(), "Managed settings deleted");

    let onboarding_needed =
        soul_storage::library_sources::check_onboarding_needed(&pool, user_id, device_id)
            .await
            .unwrap();
    assert!(
        onboarding_needed,
        "Should require onboarding after factory reset"
    );
}

/// Test factory reset preserves database schema and migrations
#[tokio::test]
async fn test_factory_reset_preserves_schema() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    let user_id = "test_user";
    let device_id = "device_1";

    // Setup some configuration
    soul_storage::library_sources::create(
        &pool,
        user_id,
        device_id,
        &CreateLibrarySource {
            name: "Test Source".to_string(),
            path: temp_dir.path().display().to_string(),
            sync_deletes: false,
        },
    )
    .await
    .unwrap();

    // Factory reset (data deletion only, schema remains)
    simulate_factory_reset(&pool).await;

    // Verify schema still works - can insert new data
    let result = soul_storage::library_sources::create(
        &pool,
        user_id,
        device_id,
        &CreateLibrarySource {
            name: "New Source After Reset".to_string(),
            path: temp_dir.path().display().to_string(),
            sync_deletes: false,
        },
    )
    .await;

    assert!(result.is_ok(), "Schema should still be functional");

    let new_source = result.unwrap();
    assert_eq!(new_source.name, "New Source After Reset");
}

/// Test multi-device scenario - factory reset on one device doesn't affect others
/// (In real app, each device has its own database file)
#[tokio::test]
async fn test_factory_reset_multi_device_isolation() {
    let pool = test_helpers::setup_test_db().await;
    let desktop_dir = TempDir::new().unwrap();
    let laptop_dir = TempDir::new().unwrap();

    let user_id = "user1";
    let desktop = "desktop_home";
    let laptop = "laptop_work";

    // Setup desktop
    soul_storage::library_sources::create(
        &pool,
        user_id,
        desktop,
        &CreateLibrarySource {
            name: "Desktop Music".to_string(),
            path: desktop_dir.path().display().to_string(),
            sync_deletes: false,
        },
    )
    .await
    .unwrap();

    // Setup laptop
    soul_storage::library_sources::create(
        &pool,
        user_id,
        laptop,
        &CreateLibrarySource {
            name: "Laptop Music".to_string(),
            path: laptop_dir.path().display().to_string(),
            sync_deletes: false,
        },
    )
    .await
    .unwrap();

    // Verify both devices have configuration
    let desktop_onboarding =
        soul_storage::library_sources::check_onboarding_needed(&pool, user_id, desktop)
            .await
            .unwrap();
    let laptop_onboarding =
        soul_storage::library_sources::check_onboarding_needed(&pool, user_id, laptop)
            .await
            .unwrap();

    assert!(!desktop_onboarding, "Desktop should be configured");
    assert!(!laptop_onboarding, "Laptop should be configured");

    // Simulate factory reset (deletes ALL data in this test - simulates desktop reset)
    simulate_factory_reset(&pool).await;

    // Both devices would need onboarding since we deleted everything
    // In real app, only the device that was reset would need onboarding
    let desktop_onboarding =
        soul_storage::library_sources::check_onboarding_needed(&pool, user_id, desktop)
            .await
            .unwrap();
    let laptop_onboarding =
        soul_storage::library_sources::check_onboarding_needed(&pool, user_id, laptop)
            .await
            .unwrap();

    assert!(desktop_onboarding);
    assert!(laptop_onboarding);
}

/// Test factory reset clears all data including library sources
#[tokio::test]
async fn test_factory_reset_clears_all_settings() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    let user_id = "test_user";
    let device_id = "device_1";

    // Setup library
    soul_storage::library_sources::create(
        &pool,
        user_id,
        device_id,
        &CreateLibrarySource {
            name: "Music".to_string(),
            path: temp_dir.path().display().to_string(),
            sync_deletes: false,
        },
    )
    .await
    .unwrap();

    // Verify library source exists
    let sources = soul_storage::library_sources::get_by_user_device(&pool, user_id, device_id)
        .await
        .unwrap();
    assert_eq!(sources.len(), 1, "Should have one library source");

    // Factory reset
    simulate_factory_reset(&pool).await;

    // Verify all data is cleared
    let sources = soul_storage::library_sources::get_by_user_device(&pool, user_id, device_id)
        .await
        .unwrap();
    assert!(sources.is_empty(), "Library sources should be deleted");

    // Verify onboarding is required again
    let onboarding_needed =
        soul_storage::library_sources::check_onboarding_needed(&pool, user_id, device_id)
            .await
            .unwrap();
    assert!(onboarding_needed, "Should require onboarding after reset");
}
