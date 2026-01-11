//! End-to-end tests for library management user flows
//!
//! Tests complete user journeys including onboarding, missing file handling,
//! file relocation, and library source management.

mod test_helpers;

use soul_core::types::{
    CreateLibrarySource, ExternalFileAction, ImportAction, ImportDestination, ScanProgressStatus,
    ScanStatus, UpdateExternalFileSettings, UpdateLibrarySource, UpdateManagedLibrarySettings,
};
use std::fs;
use tempfile::TempDir;

// =============================================================================
// User Onboarding Flow Tests
// =============================================================================

/// Test complete first-time user setup flow
#[tokio::test]
async fn test_onboarding_flow_watched_folder() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    let user_id = "new_user";
    let device_id = "desktop_1";

    // Step 1: Check if onboarding is needed (should be true for new user)
    let sources = soul_storage::library_sources::get_by_user_device(&pool, user_id, device_id)
        .await
        .unwrap();
    let managed = soul_storage::managed_library_settings::get(&pool, user_id, device_id)
        .await
        .unwrap();
    let onboarding_needed = sources.is_empty() && managed.is_none();
    assert!(onboarding_needed, "New user should need onboarding");

    // Step 2: User adds a watched folder during onboarding
    let source = soul_storage::library_sources::create(
        &pool,
        user_id,
        device_id,
        &CreateLibrarySource {
            name: "My Music".to_string(),
            path: temp_dir.path().display().to_string(),
            sync_deletes: false,
        },
    )
    .await
    .unwrap();

    // Step 3: Verify source was created
    assert_eq!(source.name, "My Music");
    assert!(source.enabled);
    assert_eq!(source.scan_status, ScanStatus::Idle);

    // Step 4: Verify onboarding is no longer needed
    let sources = soul_storage::library_sources::get_by_user_device(&pool, user_id, device_id)
        .await
        .unwrap();
    assert!(
        !sources.is_empty(),
        "User should have sources after onboarding"
    );
}

/// Test complete first-time setup with managed library
#[tokio::test]
async fn test_onboarding_flow_managed_library() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    let user_id = "new_user";
    let device_id = "desktop_1";

    // User chooses managed library during onboarding
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

    // Verify settings
    let settings = soul_storage::managed_library_settings::get(&pool, user_id, device_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        settings.path_template,
        "{AlbumArtist}/{Year} - {Album}/{TrackNo} - {Title}"
    );
    assert_eq!(settings.import_action, ImportAction::Copy);

    // Verify onboarding is no longer needed
    let is_configured =
        soul_storage::managed_library_settings::is_configured(&pool, user_id, device_id)
            .await
            .unwrap();
    assert!(is_configured);
}

/// Test hybrid setup with both watched and managed library
#[tokio::test]
async fn test_onboarding_flow_hybrid() {
    let pool = test_helpers::setup_test_db().await;
    let watched_dir = TempDir::new().unwrap();
    let managed_dir = TempDir::new().unwrap();

    let user_id = "new_user";
    let device_id = "desktop_1";

    // User adds a watched folder
    soul_storage::library_sources::create(
        &pool,
        user_id,
        device_id,
        &CreateLibrarySource {
            name: "Existing FLAC Collection".to_string(),
            path: watched_dir.path().display().to_string(),
            sync_deletes: true, // Delete from DB when files are removed
        },
    )
    .await
    .unwrap();

    // User also sets up managed library for new imports
    soul_storage::managed_library_settings::upsert(
        &pool,
        user_id,
        device_id,
        &UpdateManagedLibrarySettings {
            library_path: managed_dir.path().display().to_string(),
            path_template: "{AlbumArtist}/{Album}/{TrackNo} - {Title}".to_string(),
            import_action: ImportAction::Move, // Move imported files
        },
    )
    .await
    .unwrap();

    // Verify both are configured
    let sources = soul_storage::library_sources::get_by_user_device(&pool, user_id, device_id)
        .await
        .unwrap();
    let managed = soul_storage::managed_library_settings::get(&pool, user_id, device_id)
        .await
        .unwrap();

    assert_eq!(sources.len(), 1);
    assert!(managed.is_some());
    assert_eq!(managed.unwrap().import_action, ImportAction::Move);
}

// =============================================================================
// Missing File Handling Tests
// =============================================================================

/// Test detecting when a file is moved/deleted after being added to library
#[tokio::test]
async fn test_missing_file_detection_on_rescan() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    let user_id = "user1";
    let device_id = "device1";

    // Create test file
    let file_path = temp_dir.path().join("song.flac");
    fs::write(&file_path, b"fake flac content for testing").unwrap();

    // Create library source
    let source = soul_storage::library_sources::create(
        &pool,
        user_id,
        device_id,
        &CreateLibrarySource {
            name: "Test Library".to_string(),
            path: temp_dir.path().display().to_string(),
            sync_deletes: false, // Don't auto-delete
        },
    )
    .await
    .unwrap();

    // Record initial state - file exists
    assert!(file_path.exists());

    // Simulate file being deleted externally
    fs::remove_file(&file_path).unwrap();
    assert!(!file_path.exists());

    // On rescan, scanner should detect missing file
    // This would trigger a "file_missing" status on the track
    // (The actual implementation would be in LibraryScanner)

    // Verify source can still be retrieved for rescanning
    let fetched = soul_storage::library_sources::get_by_id(&pool, source.id)
        .await
        .unwrap()
        .unwrap();
    assert!(fetched.enabled);
}

/// Test sync_deletes behavior
#[tokio::test]
async fn test_sync_deletes_setting() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    let user_id = "user1";
    let device_id = "device1";

    // Create source with sync_deletes enabled
    let source = soul_storage::library_sources::create(
        &pool,
        user_id,
        device_id,
        &CreateLibrarySource {
            name: "Auto-sync Library".to_string(),
            path: temp_dir.path().display().to_string(),
            sync_deletes: true,
        },
    )
    .await
    .unwrap();

    assert!(source.sync_deletes);

    // User can disable sync_deletes
    soul_storage::library_sources::update(
        &pool,
        source.id,
        &UpdateLibrarySource {
            name: None,
            enabled: None,
            sync_deletes: Some(false),
        },
    )
    .await
    .unwrap();

    let updated = soul_storage::library_sources::get_by_id(&pool, source.id)
        .await
        .unwrap()
        .unwrap();
    assert!(!updated.sync_deletes);
}

// =============================================================================
// Library Source Management Tests
// =============================================================================

/// Test adding, modifying, and removing library sources
#[tokio::test]
async fn test_library_source_lifecycle() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir1 = TempDir::new().unwrap();
    let temp_dir2 = TempDir::new().unwrap();

    let user_id = "user1";
    let device_id = "device1";

    // Add first source
    let source1 = soul_storage::library_sources::create(
        &pool,
        user_id,
        device_id,
        &CreateLibrarySource {
            name: "Primary Music".to_string(),
            path: temp_dir1.path().display().to_string(),
            sync_deletes: false,
        },
    )
    .await
    .unwrap();

    // Add second source
    let source2 = soul_storage::library_sources::create(
        &pool,
        user_id,
        device_id,
        &CreateLibrarySource {
            name: "External Drive".to_string(),
            path: temp_dir2.path().display().to_string(),
            sync_deletes: true,
        },
    )
    .await
    .unwrap();

    // Verify both sources exist
    let sources = soul_storage::library_sources::get_by_user_device(&pool, user_id, device_id)
        .await
        .unwrap();
    assert_eq!(sources.len(), 2);

    // Disable external drive source (e.g., when disconnected)
    soul_storage::library_sources::set_enabled(&pool, source2.id, false)
        .await
        .unwrap();

    // Verify only enabled sources are returned
    let enabled = soul_storage::library_sources::get_enabled(&pool, user_id, device_id)
        .await
        .unwrap();
    assert_eq!(enabled.len(), 1);
    assert_eq!(enabled[0].id, source1.id);

    // Remove external drive source
    let deleted = soul_storage::library_sources::delete(&pool, source2.id)
        .await
        .unwrap();
    assert!(deleted);

    // Verify only one source remains
    let sources = soul_storage::library_sources::get_by_user_device(&pool, user_id, device_id)
        .await
        .unwrap();
    assert_eq!(sources.len(), 1);
}

/// Test preventing duplicate paths
#[tokio::test]
async fn test_duplicate_path_prevention() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    let user_id = "user1";
    let device_id = "device1";
    let path = temp_dir.path().display().to_string();

    // Add first source
    soul_storage::library_sources::create(
        &pool,
        user_id,
        device_id,
        &CreateLibrarySource {
            name: "First Source".to_string(),
            path: path.clone(),
            sync_deletes: false,
        },
    )
    .await
    .unwrap();

    // Check if path already exists before adding
    let exists = soul_storage::library_sources::path_exists(&pool, user_id, device_id, &path)
        .await
        .unwrap();
    assert!(exists, "Path should be detected as existing");

    // Different user can add same path
    let exists_other_user =
        soul_storage::library_sources::path_exists(&pool, "user2", device_id, &path)
            .await
            .unwrap();
    assert!(
        !exists_other_user,
        "Same path should be allowed for different user"
    );
}

/// Test source renaming
#[tokio::test]
async fn test_rename_library_source() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    let user_id = "user1";
    let device_id = "device1";

    let source = soul_storage::library_sources::create(
        &pool,
        user_id,
        device_id,
        &CreateLibrarySource {
            name: "Old Name".to_string(),
            path: temp_dir.path().display().to_string(),
            sync_deletes: false,
        },
    )
    .await
    .unwrap();

    // Rename the source
    soul_storage::library_sources::update(
        &pool,
        source.id,
        &UpdateLibrarySource {
            name: Some("New Better Name".to_string()),
            enabled: None,
            sync_deletes: None,
        },
    )
    .await
    .unwrap();

    let updated = soul_storage::library_sources::get_by_id(&pool, source.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated.name, "New Better Name");
    // Path should remain unchanged
    assert_eq!(updated.path, temp_dir.path().display().to_string());
}

// =============================================================================
// Scan Progress Flow Tests
// =============================================================================

/// Test complete scan flow with progress tracking
#[tokio::test]
async fn test_scan_progress_flow() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    let user_id = "user1";
    let device_id = "device1";

    // Create source
    let source = soul_storage::library_sources::create(
        &pool,
        user_id,
        device_id,
        &CreateLibrarySource {
            name: "Test Library".to_string(),
            path: temp_dir.path().display().to_string(),
            sync_deletes: false,
        },
    )
    .await
    .unwrap();

    // Start a scan
    let progress = soul_storage::scan_progress::start(&pool, source.id, Some(100))
        .await
        .unwrap();
    assert_eq!(progress.status, ScanProgressStatus::Running);
    assert_eq!(progress.total_files, Some(100));

    // Simulate progress updates
    soul_storage::scan_progress::increment_processed(&pool, progress.id, 25)
        .await
        .unwrap();
    soul_storage::scan_progress::increment_new(&pool, progress.id, 20)
        .await
        .unwrap();
    soul_storage::scan_progress::increment_updated(&pool, progress.id, 5)
        .await
        .unwrap();

    // Check progress
    let current = soul_storage::scan_progress::get_running(&pool, source.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(current.processed_files, 25);
    assert_eq!(current.new_files, 20);
    assert_eq!(current.updated_files, 5);

    // Calculate percentage
    let percentage = (current.processed_files as f32 / current.total_files.unwrap() as f32) * 100.0;
    assert_eq!(percentage, 25.0);

    // Complete the scan
    soul_storage::scan_progress::complete(&pool, progress.id)
        .await
        .unwrap();

    let completed = soul_storage::scan_progress::get_by_id(&pool, progress.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(completed.status, ScanProgressStatus::Completed);
    assert!(completed.completed_at.is_some());
}

/// Test cancelled scan handling
#[tokio::test]
async fn test_cancelled_scan_flow() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    let user_id = "user1";
    let device_id = "device1";

    let source = soul_storage::library_sources::create(
        &pool,
        user_id,
        device_id,
        &CreateLibrarySource {
            name: "Test Library".to_string(),
            path: temp_dir.path().display().to_string(),
            sync_deletes: false,
        },
    )
    .await
    .unwrap();

    // Start scan
    let progress = soul_storage::scan_progress::start(&pool, source.id, Some(1000))
        .await
        .unwrap();

    // Simulate some progress
    soul_storage::scan_progress::increment_processed(&pool, progress.id, 100)
        .await
        .unwrap();

    // User cancels the scan
    soul_storage::scan_progress::cancel(&pool, progress.id)
        .await
        .unwrap();

    let cancelled = soul_storage::scan_progress::get_by_id(&pool, progress.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(cancelled.status, ScanProgressStatus::Cancelled);
    assert!(cancelled.completed_at.is_some());
    assert_eq!(cancelled.processed_files, 100); // Should preserve progress

    // Verify no running scan
    let running = soul_storage::scan_progress::get_running(&pool, source.id)
        .await
        .unwrap();
    assert!(running.is_none());
}

// =============================================================================
// External File Settings Flow Tests
// =============================================================================

/// Test complete external file settings workflow
#[tokio::test]
async fn test_external_file_settings_workflow() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    let user_id = "user1";
    let device_id = "device1";

    // Create a library source (watched folder)
    let source = soul_storage::library_sources::create(
        &pool,
        user_id,
        device_id,
        &CreateLibrarySource {
            name: "Music Library".to_string(),
            path: temp_dir.path().display().to_string(),
            sync_deletes: false,
        },
    )
    .await
    .unwrap();

    // Set up external file handling preferences
    soul_storage::external_file_settings::upsert(
        &pool,
        user_id,
        device_id,
        &UpdateExternalFileSettings {
            default_action: ExternalFileAction::Import,
            import_destination: ImportDestination::Watched,
            import_to_source_id: Some(source.id),
            show_import_notification: true,
        },
    )
    .await
    .unwrap();

    let settings = soul_storage::external_file_settings::get(&pool, user_id, device_id)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(settings.default_action, ExternalFileAction::Import);
    assert_eq!(settings.import_destination, ImportDestination::Watched);
    assert_eq!(settings.import_to_source_id, Some(source.id));

    // Change to managed library
    soul_storage::external_file_settings::set_import_destination(
        &pool,
        user_id,
        device_id,
        ImportDestination::Managed,
        None, // No source needed for managed library
    )
    .await
    .unwrap();

    let updated = soul_storage::external_file_settings::get(&pool, user_id, device_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated.import_destination, ImportDestination::Managed);
    assert!(updated.import_to_source_id.is_none());
}

// =============================================================================
// Multi-Device Scenarios
// =============================================================================

/// Test same user on multiple devices
#[tokio::test]
async fn test_multi_device_library_management() {
    let pool = test_helpers::setup_test_db().await;
    let desktop_dir = TempDir::new().unwrap();
    let laptop_dir = TempDir::new().unwrap();

    let user_id = "user1";
    let desktop = "desktop-home";
    let laptop = "laptop-work";

    // User sets up desktop with FLAC library
    soul_storage::library_sources::create(
        &pool,
        user_id,
        desktop,
        &CreateLibrarySource {
            name: "FLAC Collection".to_string(),
            path: desktop_dir.path().display().to_string(),
            sync_deletes: true,
        },
    )
    .await
    .unwrap();

    // User sets up laptop with smaller library
    soul_storage::library_sources::create(
        &pool,
        user_id,
        laptop,
        &CreateLibrarySource {
            name: "Travel Music".to_string(),
            path: laptop_dir.path().display().to_string(),
            sync_deletes: false,
        },
    )
    .await
    .unwrap();

    // Verify sources are isolated per device
    let desktop_sources =
        soul_storage::library_sources::get_by_user_device(&pool, user_id, desktop)
            .await
            .unwrap();
    let laptop_sources = soul_storage::library_sources::get_by_user_device(&pool, user_id, laptop)
        .await
        .unwrap();

    assert_eq!(desktop_sources.len(), 1);
    assert_eq!(laptop_sources.len(), 1);
    assert_ne!(desktop_sources[0].path, laptop_sources[0].path);

    // Each device can have different external file settings
    soul_storage::external_file_settings::upsert(
        &pool,
        user_id,
        desktop,
        &UpdateExternalFileSettings {
            default_action: ExternalFileAction::Import, // Auto-import on desktop
            import_destination: ImportDestination::Watched,
            import_to_source_id: Some(desktop_sources[0].id),
            show_import_notification: false,
        },
    )
    .await
    .unwrap();

    soul_storage::external_file_settings::upsert(
        &pool,
        user_id,
        laptop,
        &UpdateExternalFileSettings {
            default_action: ExternalFileAction::Ask, // Ask on laptop
            import_destination: ImportDestination::Managed,
            import_to_source_id: None,
            show_import_notification: true,
        },
    )
    .await
    .unwrap();

    // Verify settings are isolated
    let desktop_settings = soul_storage::external_file_settings::get(&pool, user_id, desktop)
        .await
        .unwrap()
        .unwrap();
    let laptop_settings = soul_storage::external_file_settings::get(&pool, user_id, laptop)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(desktop_settings.default_action, ExternalFileAction::Import);
    assert_eq!(laptop_settings.default_action, ExternalFileAction::Ask);
}

// =============================================================================
// Error Recovery Tests
// =============================================================================

/// Test handling scan failure gracefully
#[tokio::test]
async fn test_scan_failure_recovery() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    let user_id = "user1";
    let device_id = "device1";

    let source = soul_storage::library_sources::create(
        &pool,
        user_id,
        device_id,
        &CreateLibrarySource {
            name: "Test Library".to_string(),
            path: temp_dir.path().display().to_string(),
            sync_deletes: false,
        },
    )
    .await
    .unwrap();

    // Start a scan
    let progress = soul_storage::scan_progress::start(&pool, source.id, None)
        .await
        .unwrap();

    // Simulate failure
    soul_storage::scan_progress::fail(&pool, progress.id, "Permission denied: /path/to/file")
        .await
        .unwrap();

    let failed = soul_storage::scan_progress::get_by_id(&pool, progress.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(failed.status, ScanProgressStatus::Failed);
    assert_eq!(
        failed.error_message,
        Some("Permission denied: /path/to/file".to_string())
    );

    // Source should be marked with error status
    soul_storage::library_sources::set_scan_status(
        &pool,
        source.id,
        ScanStatus::Error,
        Some("Permission denied"),
    )
    .await
    .unwrap();

    let source_status = soul_storage::library_sources::get_by_id(&pool, source.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(source_status.scan_status, ScanStatus::Error);
    assert_eq!(
        source_status.error_message,
        Some("Permission denied".to_string())
    );

    // User can retry - start new scan
    let retry_progress = soul_storage::scan_progress::start(&pool, source.id, None)
        .await
        .unwrap();

    soul_storage::library_sources::set_scan_status(&pool, source.id, ScanStatus::Scanning, None)
        .await
        .unwrap();

    // Complete successfully this time
    soul_storage::scan_progress::complete(&pool, retry_progress.id)
        .await
        .unwrap();

    soul_storage::library_sources::set_scan_status(&pool, source.id, ScanStatus::Idle, None)
        .await
        .unwrap();

    let recovered = soul_storage::library_sources::get_by_id(&pool, source.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(recovered.scan_status, ScanStatus::Idle);
    assert!(recovered.error_message.is_none());
}

// =============================================================================
// Managed Library Settings Tests
// =============================================================================

/// Test path template changes
#[tokio::test]
async fn test_managed_library_template_changes() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    let user_id = "user1";
    let device_id = "device1";

    // Initial setup with audiophile template
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

    // User changes to simpler template
    soul_storage::managed_library_settings::set_path_template(
        &pool,
        user_id,
        device_id,
        "{Artist}/{Album}/{Title}",
    )
    .await
    .unwrap();

    let settings = soul_storage::managed_library_settings::get(&pool, user_id, device_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(settings.path_template, "{Artist}/{Album}/{Title}");
    // Other settings should be preserved
    assert_eq!(settings.import_action, ImportAction::Copy);
}

/// Test switching between copy and move
#[tokio::test]
async fn test_import_action_switch() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    let user_id = "user1";
    let device_id = "device1";

    // Start with copy
    soul_storage::managed_library_settings::upsert(
        &pool,
        user_id,
        device_id,
        &UpdateManagedLibrarySettings {
            library_path: temp_dir.path().display().to_string(),
            path_template: "{Artist}/{Album}/{Title}".to_string(),
            import_action: ImportAction::Copy,
        },
    )
    .await
    .unwrap();

    // Switch to move
    soul_storage::managed_library_settings::set_import_action(
        &pool,
        user_id,
        device_id,
        ImportAction::Move,
    )
    .await
    .unwrap();

    let settings = soul_storage::managed_library_settings::get(&pool, user_id, device_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(settings.import_action, ImportAction::Move);
}
