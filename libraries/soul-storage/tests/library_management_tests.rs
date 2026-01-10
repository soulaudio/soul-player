//! Integration tests for library management modules
//!
//! Tests for library_sources, managed_library_settings, and scan_progress modules.

use soul_core::types::{
    CreateLibrarySource, ImportAction, ScanProgressStatus, ScanStatus, UpdateLibrarySource,
    UpdateManagedLibrarySettings,
};

mod test_helpers;
use test_helpers::setup_test_db;

// =============================================================================
// Library Sources Tests
// =============================================================================

#[tokio::test]
async fn test_create_and_get_library_source() {
    let pool = setup_test_db().await;

    let source = soul_storage::library_sources::create(
        &pool,
        "user1",
        "device1",
        &CreateLibrarySource {
            name: "My Music".to_string(),
            path: "/home/user/Music".to_string(),
            sync_deletes: true,
        },
    )
    .await
    .unwrap();

    assert_eq!(source.name, "My Music");
    assert_eq!(source.path, "/home/user/Music");
    assert!(source.enabled);
    assert!(source.sync_deletes);
    assert_eq!(source.scan_status, ScanStatus::Idle);
    assert!(source.last_scan_at.is_none());

    // Retrieve by ID
    let fetched = soul_storage::library_sources::get_by_id(&pool, source.id)
        .await
        .unwrap();
    assert!(fetched.is_some());
    let fetched = fetched.unwrap();
    assert_eq!(fetched.name, "My Music");
    assert_eq!(fetched.user_id, "user1");
    assert_eq!(fetched.device_id, "device1");
}

#[tokio::test]
async fn test_get_library_sources_by_user_device() {
    let pool = setup_test_db().await;

    // Create multiple sources for the same user/device
    soul_storage::library_sources::create(
        &pool,
        "user1",
        "device1",
        &CreateLibrarySource {
            name: "FLAC Collection".to_string(),
            path: "/data/flac".to_string(),
            sync_deletes: true,
        },
    )
    .await
    .unwrap();

    soul_storage::library_sources::create(
        &pool,
        "user1",
        "device1",
        &CreateLibrarySource {
            name: "MP3 Archive".to_string(),
            path: "/data/mp3".to_string(),
            sync_deletes: false,
        },
    )
    .await
    .unwrap();

    // Create source for different user
    soul_storage::library_sources::create(
        &pool,
        "user2",
        "device1",
        &CreateLibrarySource {
            name: "User2 Music".to_string(),
            path: "/home/user2/Music".to_string(),
            sync_deletes: true,
        },
    )
    .await
    .unwrap();

    // Get sources for user1/device1
    let sources = soul_storage::library_sources::get_by_user_device(&pool, "user1", "device1")
        .await
        .unwrap();
    assert_eq!(sources.len(), 2);

    // Verify ordering by name
    assert_eq!(sources[0].name, "FLAC Collection");
    assert_eq!(sources[1].name, "MP3 Archive");

    // Get sources for user2/device1
    let sources = soul_storage::library_sources::get_by_user_device(&pool, "user2", "device1")
        .await
        .unwrap();
    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0].name, "User2 Music");
}

#[tokio::test]
async fn test_get_enabled_library_sources() {
    let pool = setup_test_db().await;

    let source1 = soul_storage::library_sources::create(
        &pool,
        "user1",
        "device1",
        &CreateLibrarySource {
            name: "Enabled Source".to_string(),
            path: "/data/enabled".to_string(),
            sync_deletes: true,
        },
    )
    .await
    .unwrap();

    let source2 = soul_storage::library_sources::create(
        &pool,
        "user1",
        "device1",
        &CreateLibrarySource {
            name: "Disabled Source".to_string(),
            path: "/data/disabled".to_string(),
            sync_deletes: true,
        },
    )
    .await
    .unwrap();

    // Disable one source
    soul_storage::library_sources::update(
        &pool,
        source2.id,
        &UpdateLibrarySource {
            name: None,
            enabled: Some(false),
            sync_deletes: None,
        },
    )
    .await
    .unwrap();

    // Get only enabled sources
    let enabled = soul_storage::library_sources::get_enabled(&pool, "user1", "device1")
        .await
        .unwrap();
    assert_eq!(enabled.len(), 1);
    assert_eq!(enabled[0].id, source1.id);
}

#[tokio::test]
async fn test_update_library_source() {
    let pool = setup_test_db().await;

    let source = soul_storage::library_sources::create(
        &pool,
        "user1",
        "device1",
        &CreateLibrarySource {
            name: "Original Name".to_string(),
            path: "/data/music".to_string(),
            sync_deletes: true,
        },
    )
    .await
    .unwrap();

    // Update name and sync_deletes
    let updated = soul_storage::library_sources::update(
        &pool,
        source.id,
        &UpdateLibrarySource {
            name: Some("New Name".to_string()),
            enabled: None,
            sync_deletes: Some(false),
        },
    )
    .await
    .unwrap();
    assert!(updated);

    // Verify changes
    let fetched = soul_storage::library_sources::get_by_id(&pool, source.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fetched.name, "New Name");
    assert!(!fetched.sync_deletes);
    assert!(fetched.enabled); // Should remain unchanged
}

#[tokio::test]
async fn test_delete_library_source() {
    let pool = setup_test_db().await;

    let source = soul_storage::library_sources::create(
        &pool,
        "user1",
        "device1",
        &CreateLibrarySource {
            name: "To Delete".to_string(),
            path: "/data/temp".to_string(),
            sync_deletes: true,
        },
    )
    .await
    .unwrap();

    // Delete
    let deleted = soul_storage::library_sources::delete(&pool, source.id)
        .await
        .unwrap();
    assert!(deleted);

    // Verify deleted
    let fetched = soul_storage::library_sources::get_by_id(&pool, source.id)
        .await
        .unwrap();
    assert!(fetched.is_none());

    // Delete non-existent returns false
    let deleted = soul_storage::library_sources::delete(&pool, source.id)
        .await
        .unwrap();
    assert!(!deleted);
}

#[tokio::test]
async fn test_set_scan_status() {
    let pool = setup_test_db().await;

    let source = soul_storage::library_sources::create(
        &pool,
        "user1",
        "device1",
        &CreateLibrarySource {
            name: "Source".to_string(),
            path: "/data/music".to_string(),
            sync_deletes: true,
        },
    )
    .await
    .unwrap();

    // Set to scanning
    soul_storage::library_sources::set_scan_status(&pool, source.id, ScanStatus::Scanning, None)
        .await
        .unwrap();

    let fetched = soul_storage::library_sources::get_by_id(&pool, source.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fetched.scan_status, ScanStatus::Scanning);
    assert!(fetched.error_message.is_none());

    // Set to error with message
    soul_storage::library_sources::set_scan_status(
        &pool,
        source.id,
        ScanStatus::Error,
        Some("Permission denied"),
    )
    .await
    .unwrap();

    let fetched = soul_storage::library_sources::get_by_id(&pool, source.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fetched.scan_status, ScanStatus::Error);
    assert_eq!(fetched.error_message, Some("Permission denied".to_string()));
}

#[tokio::test]
async fn test_path_exists() {
    let pool = setup_test_db().await;

    soul_storage::library_sources::create(
        &pool,
        "user1",
        "device1",
        &CreateLibrarySource {
            name: "Source".to_string(),
            path: "/data/music".to_string(),
            sync_deletes: true,
        },
    )
    .await
    .unwrap();

    // Same path exists
    let exists =
        soul_storage::library_sources::path_exists(&pool, "user1", "device1", "/data/music")
            .await
            .unwrap();
    assert!(exists);

    // Different path doesn't exist
    let exists =
        soul_storage::library_sources::path_exists(&pool, "user1", "device1", "/data/other")
            .await
            .unwrap();
    assert!(!exists);

    // Same path for different user doesn't exist
    let exists =
        soul_storage::library_sources::path_exists(&pool, "user2", "device1", "/data/music")
            .await
            .unwrap();
    assert!(!exists);
}

#[tokio::test]
async fn test_library_source_count() {
    let pool = setup_test_db().await;

    // Initially zero
    let count = soul_storage::library_sources::count(&pool, "user1", "device1")
        .await
        .unwrap();
    assert_eq!(count, 0);

    // Add sources
    for i in 0..3 {
        soul_storage::library_sources::create(
            &pool,
            "user1",
            "device1",
            &CreateLibrarySource {
                name: format!("Source {}", i),
                path: format!("/data/{}", i),
                sync_deletes: true,
            },
        )
        .await
        .unwrap();
    }

    let count = soul_storage::library_sources::count(&pool, "user1", "device1")
        .await
        .unwrap();
    assert_eq!(count, 3);
}

// =============================================================================
// Managed Library Settings Tests
// =============================================================================

#[tokio::test]
async fn test_create_and_get_managed_library_settings() {
    let pool = setup_test_db().await;

    // Initially none
    let settings = soul_storage::managed_library_settings::get(&pool, "user1", "device1")
        .await
        .unwrap();
    assert!(settings.is_none());

    // Create settings
    soul_storage::managed_library_settings::upsert(
        &pool,
        "user1",
        "device1",
        &UpdateManagedLibrarySettings {
            library_path: "/home/user/Music/Soul Player".to_string(),
            path_template: "{AlbumArtist}/{Year} - {Album}/{TrackNo} - {Title}".to_string(),
            import_action: ImportAction::Copy,
        },
    )
    .await
    .unwrap();

    // Retrieve
    let settings = soul_storage::managed_library_settings::get(&pool, "user1", "device1")
        .await
        .unwrap();
    assert!(settings.is_some());
    let settings = settings.unwrap();
    assert_eq!(settings.library_path, "/home/user/Music/Soul Player");
    assert_eq!(
        settings.path_template,
        "{AlbumArtist}/{Year} - {Album}/{TrackNo} - {Title}"
    );
    assert_eq!(settings.import_action, ImportAction::Copy);
}

#[tokio::test]
async fn test_get_or_create_managed_library_settings() {
    let pool = setup_test_db().await;

    // Get or create with default path
    let settings = soul_storage::managed_library_settings::get_or_create(
        &pool,
        "user1",
        "device1",
        "/home/user/Music/Default",
    )
    .await
    .unwrap();

    assert_eq!(settings.library_path, "/home/user/Music/Default");
    // Default template should be used
    assert!(!settings.path_template.is_empty());

    // Calling again returns existing settings
    let settings2 = soul_storage::managed_library_settings::get_or_create(
        &pool,
        "user1",
        "device1",
        "/home/user/Music/Different", // Different default path
    )
    .await
    .unwrap();

    // Should have original path, not the new default
    assert_eq!(settings2.library_path, "/home/user/Music/Default");
    assert_eq!(settings.id, settings2.id);
}

#[tokio::test]
async fn test_upsert_managed_library_settings() {
    let pool = setup_test_db().await;

    // Initial insert
    soul_storage::managed_library_settings::upsert(
        &pool,
        "user1",
        "device1",
        &UpdateManagedLibrarySettings {
            library_path: "/path/v1".to_string(),
            path_template: "{Artist}/{Album}".to_string(),
            import_action: ImportAction::Copy,
        },
    )
    .await
    .unwrap();

    let settings = soul_storage::managed_library_settings::get(&pool, "user1", "device1")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(settings.library_path, "/path/v1");

    // Upsert (update)
    soul_storage::managed_library_settings::upsert(
        &pool,
        "user1",
        "device1",
        &UpdateManagedLibrarySettings {
            library_path: "/path/v2".to_string(),
            path_template: "{AlbumArtist}/{Album}".to_string(),
            import_action: ImportAction::Move,
        },
    )
    .await
    .unwrap();

    let settings = soul_storage::managed_library_settings::get(&pool, "user1", "device1")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(settings.library_path, "/path/v2");
    assert_eq!(settings.path_template, "{AlbumArtist}/{Album}");
    assert_eq!(settings.import_action, ImportAction::Move);
}

#[tokio::test]
async fn test_set_individual_managed_library_settings() {
    let pool = setup_test_db().await;

    // Create initial settings
    soul_storage::managed_library_settings::upsert(
        &pool,
        "user1",
        "device1",
        &UpdateManagedLibrarySettings {
            library_path: "/path/original".to_string(),
            path_template: "{Artist}/{Album}".to_string(),
            import_action: ImportAction::Copy,
        },
    )
    .await
    .unwrap();

    // Update just the library path
    let updated =
        soul_storage::managed_library_settings::set_library_path(&pool, "user1", "device1", "/path/new")
            .await
            .unwrap();
    assert!(updated);

    let settings = soul_storage::managed_library_settings::get(&pool, "user1", "device1")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(settings.library_path, "/path/new");
    assert_eq!(settings.path_template, "{Artist}/{Album}"); // Unchanged

    // Update just the template
    let updated = soul_storage::managed_library_settings::set_path_template(
        &pool,
        "user1",
        "device1",
        "{AlbumArtist}/{Year} - {Album}",
    )
    .await
    .unwrap();
    assert!(updated);

    let settings = soul_storage::managed_library_settings::get(&pool, "user1", "device1")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(settings.path_template, "{AlbumArtist}/{Year} - {Album}");

    // Update just the import action
    let updated = soul_storage::managed_library_settings::set_import_action(
        &pool,
        "user1",
        "device1",
        ImportAction::Move,
    )
    .await
    .unwrap();
    assert!(updated);

    let settings = soul_storage::managed_library_settings::get(&pool, "user1", "device1")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(settings.import_action, ImportAction::Move);
}

#[tokio::test]
async fn test_delete_managed_library_settings() {
    let pool = setup_test_db().await;

    soul_storage::managed_library_settings::upsert(
        &pool,
        "user1",
        "device1",
        &UpdateManagedLibrarySettings {
            library_path: "/path".to_string(),
            path_template: "{Artist}".to_string(),
            import_action: ImportAction::Copy,
        },
    )
    .await
    .unwrap();

    // Delete
    let deleted = soul_storage::managed_library_settings::delete(&pool, "user1", "device1")
        .await
        .unwrap();
    assert!(deleted);

    // Verify deleted
    let settings = soul_storage::managed_library_settings::get(&pool, "user1", "device1")
        .await
        .unwrap();
    assert!(settings.is_none());

    // Delete non-existent returns false
    let deleted = soul_storage::managed_library_settings::delete(&pool, "user1", "device1")
        .await
        .unwrap();
    assert!(!deleted);
}

#[tokio::test]
async fn test_is_configured_managed_library() {
    let pool = setup_test_db().await;

    // Initially not configured
    let configured =
        soul_storage::managed_library_settings::is_configured(&pool, "user1", "device1")
            .await
            .unwrap();
    assert!(!configured);

    // Configure
    soul_storage::managed_library_settings::upsert(
        &pool,
        "user1",
        "device1",
        &UpdateManagedLibrarySettings {
            library_path: "/path".to_string(),
            path_template: "{Artist}".to_string(),
            import_action: ImportAction::Copy,
        },
    )
    .await
    .unwrap();

    // Now configured
    let configured =
        soul_storage::managed_library_settings::is_configured(&pool, "user1", "device1")
            .await
            .unwrap();
    assert!(configured);
}

#[tokio::test]
async fn test_managed_library_multi_user_isolation() {
    let pool = setup_test_db().await;

    // User 1 settings
    soul_storage::managed_library_settings::upsert(
        &pool,
        "user1",
        "device1",
        &UpdateManagedLibrarySettings {
            library_path: "/user1/Music".to_string(),
            path_template: "{Artist}/{Album}".to_string(),
            import_action: ImportAction::Copy,
        },
    )
    .await
    .unwrap();

    // User 2 settings
    soul_storage::managed_library_settings::upsert(
        &pool,
        "user2",
        "device1",
        &UpdateManagedLibrarySettings {
            library_path: "/user2/Music".to_string(),
            path_template: "{AlbumArtist}/{Year}".to_string(),
            import_action: ImportAction::Move,
        },
    )
    .await
    .unwrap();

    // Verify isolation
    let user1 = soul_storage::managed_library_settings::get(&pool, "user1", "device1")
        .await
        .unwrap()
        .unwrap();
    let user2 = soul_storage::managed_library_settings::get(&pool, "user2", "device1")
        .await
        .unwrap()
        .unwrap();

    assert_eq!(user1.library_path, "/user1/Music");
    assert_eq!(user2.library_path, "/user2/Music");
    assert_eq!(user1.import_action, ImportAction::Copy);
    assert_eq!(user2.import_action, ImportAction::Move);
}

// =============================================================================
// Scan Progress Tests
// =============================================================================

#[tokio::test]
async fn test_start_and_get_scan_progress() {
    let pool = setup_test_db().await;

    // Create a library source first
    let source = soul_storage::library_sources::create(
        &pool,
        "user1",
        "device1",
        &CreateLibrarySource {
            name: "Source".to_string(),
            path: "/data".to_string(),
            sync_deletes: true,
        },
    )
    .await
    .unwrap();

    // Start a scan
    let progress = soul_storage::scan_progress::start(&pool, source.id, Some(1000))
        .await
        .unwrap();

    assert_eq!(progress.library_source_id, source.id);
    assert_eq!(progress.total_files, Some(1000));
    assert_eq!(progress.processed_files, 0);
    assert_eq!(progress.new_files, 0);
    assert_eq!(progress.updated_files, 0);
    assert_eq!(progress.removed_files, 0);
    assert_eq!(progress.errors, 0);
    assert_eq!(progress.status, ScanProgressStatus::Running);
    assert!(progress.completed_at.is_none());

    // Retrieve by ID
    let fetched = soul_storage::scan_progress::get_by_id(&pool, progress.id)
        .await
        .unwrap();
    assert!(fetched.is_some());
    assert_eq!(fetched.unwrap().id, progress.id);
}

#[tokio::test]
async fn test_get_running_scan() {
    let pool = setup_test_db().await;

    let source = soul_storage::library_sources::create(
        &pool,
        "user1",
        "device1",
        &CreateLibrarySource {
            name: "Source".to_string(),
            path: "/data".to_string(),
            sync_deletes: true,
        },
    )
    .await
    .unwrap();

    // No running scan initially
    let running = soul_storage::scan_progress::get_running(&pool, source.id)
        .await
        .unwrap();
    assert!(running.is_none());

    // Start a scan
    let progress = soul_storage::scan_progress::start(&pool, source.id, None)
        .await
        .unwrap();

    // Now there's a running scan
    let running = soul_storage::scan_progress::get_running(&pool, source.id)
        .await
        .unwrap();
    assert!(running.is_some());
    assert_eq!(running.unwrap().id, progress.id);
}

#[tokio::test]
async fn test_scan_progress_increment_counters() {
    let pool = setup_test_db().await;

    let source = soul_storage::library_sources::create(
        &pool,
        "user1",
        "device1",
        &CreateLibrarySource {
            name: "Source".to_string(),
            path: "/data".to_string(),
            sync_deletes: true,
        },
    )
    .await
    .unwrap();

    let progress = soul_storage::scan_progress::start(&pool, source.id, Some(100))
        .await
        .unwrap();

    // Increment processed
    soul_storage::scan_progress::increment_processed(&pool, progress.id, 10)
        .await
        .unwrap();
    soul_storage::scan_progress::increment_processed(&pool, progress.id, 5)
        .await
        .unwrap();

    // Increment new
    soul_storage::scan_progress::increment_new(&pool, progress.id, 8)
        .await
        .unwrap();

    // Increment updated
    soul_storage::scan_progress::increment_updated(&pool, progress.id, 5)
        .await
        .unwrap();

    // Increment removed
    soul_storage::scan_progress::increment_removed(&pool, progress.id, 2)
        .await
        .unwrap();

    // Increment errors
    soul_storage::scan_progress::increment_errors(&pool, progress.id, 1)
        .await
        .unwrap();

    // Verify
    let fetched = soul_storage::scan_progress::get_by_id(&pool, progress.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fetched.processed_files, 15); // 10 + 5
    assert_eq!(fetched.new_files, 8);
    assert_eq!(fetched.updated_files, 5);
    assert_eq!(fetched.removed_files, 2);
    assert_eq!(fetched.errors, 1);
}

#[tokio::test]
async fn test_scan_progress_complete() {
    let pool = setup_test_db().await;

    let source = soul_storage::library_sources::create(
        &pool,
        "user1",
        "device1",
        &CreateLibrarySource {
            name: "Source".to_string(),
            path: "/data".to_string(),
            sync_deletes: true,
        },
    )
    .await
    .unwrap();

    let progress = soul_storage::scan_progress::start(&pool, source.id, None)
        .await
        .unwrap();

    // Complete the scan
    soul_storage::scan_progress::complete(&pool, progress.id)
        .await
        .unwrap();

    let fetched = soul_storage::scan_progress::get_by_id(&pool, progress.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fetched.status, ScanProgressStatus::Completed);
    assert!(fetched.completed_at.is_some());

    // No longer shows as running
    let running = soul_storage::scan_progress::get_running(&pool, source.id)
        .await
        .unwrap();
    assert!(running.is_none());
}

#[tokio::test]
async fn test_scan_progress_fail() {
    let pool = setup_test_db().await;

    let source = soul_storage::library_sources::create(
        &pool,
        "user1",
        "device1",
        &CreateLibrarySource {
            name: "Source".to_string(),
            path: "/data".to_string(),
            sync_deletes: true,
        },
    )
    .await
    .unwrap();

    let progress = soul_storage::scan_progress::start(&pool, source.id, None)
        .await
        .unwrap();

    // Fail the scan
    soul_storage::scan_progress::fail(&pool, progress.id, "Disk full")
        .await
        .unwrap();

    let fetched = soul_storage::scan_progress::get_by_id(&pool, progress.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fetched.status, ScanProgressStatus::Failed);
    assert_eq!(fetched.error_message, Some("Disk full".to_string()));
    assert!(fetched.completed_at.is_some());
}

#[tokio::test]
async fn test_scan_progress_cancel() {
    let pool = setup_test_db().await;

    let source = soul_storage::library_sources::create(
        &pool,
        "user1",
        "device1",
        &CreateLibrarySource {
            name: "Source".to_string(),
            path: "/data".to_string(),
            sync_deletes: true,
        },
    )
    .await
    .unwrap();

    let progress = soul_storage::scan_progress::start(&pool, source.id, None)
        .await
        .unwrap();

    // Cancel the scan
    soul_storage::scan_progress::cancel(&pool, progress.id)
        .await
        .unwrap();

    let fetched = soul_storage::scan_progress::get_by_id(&pool, progress.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fetched.status, ScanProgressStatus::Cancelled);
    assert!(fetched.completed_at.is_some());
}

#[tokio::test]
async fn test_get_latest_scan() {
    let pool = setup_test_db().await;

    let source = soul_storage::library_sources::create(
        &pool,
        "user1",
        "device1",
        &CreateLibrarySource {
            name: "Source".to_string(),
            path: "/data".to_string(),
            sync_deletes: true,
        },
    )
    .await
    .unwrap();

    // Start and complete first scan
    let scan1 = soul_storage::scan_progress::start(&pool, source.id, Some(100))
        .await
        .unwrap();
    soul_storage::scan_progress::complete(&pool, scan1.id)
        .await
        .unwrap();

    // Start second scan
    let scan2 = soul_storage::scan_progress::start(&pool, source.id, Some(200))
        .await
        .unwrap();

    // Latest should be scan2
    let latest = soul_storage::scan_progress::get_latest(&pool, source.id)
        .await
        .unwrap();
    assert!(latest.is_some());
    assert_eq!(latest.unwrap().id, scan2.id);
}

#[tokio::test]
async fn test_cleanup_old_scans() {
    let pool = setup_test_db().await;

    let source = soul_storage::library_sources::create(
        &pool,
        "user1",
        "device1",
        &CreateLibrarySource {
            name: "Source".to_string(),
            path: "/data".to_string(),
            sync_deletes: true,
        },
    )
    .await
    .unwrap();

    // Create 5 completed scans
    let mut scan_ids = Vec::new();
    for i in 0..5 {
        let scan = soul_storage::scan_progress::start(&pool, source.id, Some(i * 100))
            .await
            .unwrap();
        soul_storage::scan_progress::complete(&pool, scan.id)
            .await
            .unwrap();
        scan_ids.push(scan.id);
    }

    // Cleanup keeping only the last 2
    let deleted = soul_storage::scan_progress::cleanup_old(&pool, source.id, 2)
        .await
        .unwrap();
    assert_eq!(deleted, 3); // Should delete 3 old scans

    // Verify only 2 remain
    let latest = soul_storage::scan_progress::get_latest(&pool, source.id)
        .await
        .unwrap();
    assert!(latest.is_some());

    // The latest scan should still exist
    assert_eq!(latest.unwrap().id, scan_ids[4]);

    // Oldest scans should be gone
    let scan1 = soul_storage::scan_progress::get_by_id(&pool, scan_ids[0])
        .await
        .unwrap();
    assert!(scan1.is_none());
}

#[tokio::test]
async fn test_set_total_files() {
    let pool = setup_test_db().await;

    let source = soul_storage::library_sources::create(
        &pool,
        "user1",
        "device1",
        &CreateLibrarySource {
            name: "Source".to_string(),
            path: "/data".to_string(),
            sync_deletes: true,
        },
    )
    .await
    .unwrap();

    // Start with unknown total
    let progress = soul_storage::scan_progress::start(&pool, source.id, None)
        .await
        .unwrap();
    assert!(progress.total_files.is_none());

    // Set total after counting files
    soul_storage::scan_progress::set_total_files(&pool, progress.id, 500)
        .await
        .unwrap();

    let fetched = soul_storage::scan_progress::get_by_id(&pool, progress.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fetched.total_files, Some(500));
}
