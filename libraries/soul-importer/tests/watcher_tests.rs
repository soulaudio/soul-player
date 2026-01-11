//! Integration tests for filesystem watcher

mod test_helpers;

use soul_core::types::CreateLibrarySource;
use soul_importer::watcher::{LibraryWatcher, WatcherConfig, WatcherEvent};
use std::fs;
use std::io::Write;
use std::time::Duration;
use tempfile::TempDir;

/// Create a test audio file (fake FLAC with minimal header)
fn create_test_audio_file(path: &std::path::Path, filename: &str) -> std::path::PathBuf {
    let file_path = path.join(filename);
    let mut file = fs::File::create(&file_path).expect("Failed to create test file");

    // Write a fake FLAC header (fLaC magic bytes + minimal metadata)
    file.write_all(b"fLaC\x00\x00\x00\x22")
        .expect("Failed to write header");
    file.write_all(&[0u8; 1000])
        .expect("Failed to write padding");
    file.flush().expect("Failed to flush");

    file_path
}

/// Helper to create a library source for testing
async fn create_test_source(
    pool: &sqlx::SqlitePool,
    user_id: &str,
    device_id: &str,
    name: &str,
    path: &str,
) -> soul_core::types::LibrarySource {
    soul_storage::library_sources::create(
        pool,
        user_id,
        device_id,
        &CreateLibrarySource {
            name: name.to_string(),
            path: path.to_string(),
            sync_deletes: false,
        },
    )
    .await
    .expect("Failed to create library source")
}

#[tokio::test]
async fn test_watcher_creation() {
    let pool = test_helpers::setup_test_db().await;

    let watcher = LibraryWatcher::new(pool.clone(), "user1", "device1");
    assert_eq!(watcher.watcher_count().await, 0);
}

#[tokio::test]
async fn test_watcher_with_config() {
    let pool = test_helpers::setup_test_db().await;

    let config = WatcherConfig {
        debounce_duration: Duration::from_millis(100),
        batch_events: true,
        max_batch_size: 50,
    };

    let watcher = LibraryWatcher::new(pool.clone(), "user1", "device1").with_config(config);

    assert_eq!(watcher.watcher_count().await, 0);
}

#[tokio::test]
async fn test_watch_source() {
    let pool = test_helpers::setup_test_db().await;

    // Create a temporary directory
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Create a library source
    let source = create_test_source(
        &pool,
        "user1",
        "device1",
        "Test Source",
        &temp_dir.path().display().to_string(),
    )
    .await;

    let watcher = LibraryWatcher::new(pool.clone(), "user1", "device1");

    // Start watching
    watcher
        .watch_source(&source)
        .await
        .expect("watch_source should succeed");

    assert_eq!(watcher.watcher_count().await, 1);
    assert!(watcher.is_watching(source.id).await);
}

#[tokio::test]
async fn test_unwatch_source() {
    let pool = test_helpers::setup_test_db().await;

    // Create a temporary directory
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Create a library source
    let source = create_test_source(
        &pool,
        "user1",
        "device1",
        "Test Source",
        &temp_dir.path().display().to_string(),
    )
    .await;

    let watcher = LibraryWatcher::new(pool.clone(), "user1", "device1");

    // Start watching
    watcher
        .watch_source(&source)
        .await
        .expect("watch_source should succeed");
    assert_eq!(watcher.watcher_count().await, 1);

    // Stop watching
    watcher
        .unwatch_source(source.id)
        .await
        .expect("unwatch_source should succeed");
    assert_eq!(watcher.watcher_count().await, 0);
    assert!(!watcher.is_watching(source.id).await);
}

#[tokio::test]
async fn test_stop_watching() {
    let pool = test_helpers::setup_test_db().await;

    // Create two temporary directories
    let temp_dir1 = TempDir::new().expect("Failed to create temp dir 1");
    let temp_dir2 = TempDir::new().expect("Failed to create temp dir 2");

    // Create two library sources
    let source1 = create_test_source(
        &pool,
        "user1",
        "device1",
        "Source 1",
        &temp_dir1.path().display().to_string(),
    )
    .await;

    let source2 = create_test_source(
        &pool,
        "user1",
        "device1",
        "Source 2",
        &temp_dir2.path().display().to_string(),
    )
    .await;

    let watcher = LibraryWatcher::new(pool.clone(), "user1", "device1");

    // Start watching both
    watcher
        .watch_source(&source1)
        .await
        .expect("watch_source 1 should succeed");
    watcher
        .watch_source(&source2)
        .await
        .expect("watch_source 2 should succeed");
    assert_eq!(watcher.watcher_count().await, 2);

    // Stop all
    watcher
        .stop_watching()
        .await
        .expect("stop_watching should succeed");
    assert_eq!(watcher.watcher_count().await, 0);
}

#[tokio::test]
async fn test_start_watching_all_enabled() {
    let pool = test_helpers::setup_test_db().await;

    // Create two temporary directories
    let temp_dir1 = TempDir::new().expect("Failed to create temp dir 1");
    let temp_dir2 = TempDir::new().expect("Failed to create temp dir 2");

    // Create two library sources (both enabled by default)
    create_test_source(
        &pool,
        "user1",
        "device1",
        "Source 1",
        &temp_dir1.path().display().to_string(),
    )
    .await;

    create_test_source(
        &pool,
        "user1",
        "device1",
        "Source 2",
        &temp_dir2.path().display().to_string(),
    )
    .await;

    let watcher = LibraryWatcher::new(pool.clone(), "user1", "device1");

    // Start watching all enabled sources
    watcher
        .start_watching()
        .await
        .expect("start_watching should succeed");
    assert_eq!(watcher.watcher_count().await, 2);
}

#[tokio::test]
async fn test_watcher_ignores_nonexistent_path() {
    let pool = test_helpers::setup_test_db().await;

    // Create a library source with non-existent path
    let source = create_test_source(
        &pool,
        "user1",
        "device1",
        "Bad Source",
        "/nonexistent/path/that/does/not/exist",
    )
    .await;

    let watcher = LibraryWatcher::new(pool.clone(), "user1", "device1");

    // Should not fail, just skip the non-existent path
    watcher
        .watch_source(&source)
        .await
        .expect("watch_source should succeed (silently skip)");
    assert_eq!(watcher.watcher_count().await, 0);
}

#[tokio::test]
async fn test_take_event_receiver() {
    let pool = test_helpers::setup_test_db().await;

    let mut watcher = LibraryWatcher::new(pool.clone(), "user1", "device1");

    // First call should return Some
    let rx = watcher.take_event_receiver();
    assert!(rx.is_some());

    // Second call should return None
    let rx2 = watcher.take_event_receiver();
    assert!(rx2.is_none());
}

#[tokio::test]
async fn test_watcher_event_variants() {
    // Test that WatcherEvent can represent all event types
    let created = WatcherEvent::Created(std::path::PathBuf::from("/test/file.flac"));
    let modified = WatcherEvent::Modified(std::path::PathBuf::from("/test/file.flac"));
    let removed = WatcherEvent::Removed(std::path::PathBuf::from("/test/file.flac"));
    let renamed = WatcherEvent::Renamed(
        std::path::PathBuf::from("/test/old.flac"),
        std::path::PathBuf::from("/test/new.flac"),
    );

    // Just verify they can be created and formatted
    assert!(format!("{:?}", created).contains("Created"));
    assert!(format!("{:?}", modified).contains("Modified"));
    assert!(format!("{:?}", removed).contains("Removed"));
    assert!(format!("{:?}", renamed).contains("Renamed"));
}

#[tokio::test]
async fn test_user_device_isolation() {
    let pool = test_helpers::setup_test_db().await;

    // Create temp directories
    let temp_dir1 = TempDir::new().expect("Failed to create temp dir 1");
    let temp_dir2 = TempDir::new().expect("Failed to create temp dir 2");

    // Create sources for different users/devices
    create_test_source(
        &pool,
        "user1",
        "device1",
        "User1 Source",
        &temp_dir1.path().display().to_string(),
    )
    .await;

    create_test_source(
        &pool,
        "user2",
        "device2",
        "User2 Source",
        &temp_dir2.path().display().to_string(),
    )
    .await;

    // Watcher for user1/device1 should only see user1's source
    let watcher1 = LibraryWatcher::new(pool.clone(), "user1", "device1");
    watcher1
        .start_watching()
        .await
        .expect("start_watching should succeed");
    assert_eq!(watcher1.watcher_count().await, 1);

    // Watcher for user2/device2 should only see user2's source
    let watcher2 = LibraryWatcher::new(pool.clone(), "user2", "device2");
    watcher2
        .start_watching()
        .await
        .expect("start_watching should succeed");
    assert_eq!(watcher2.watcher_count().await, 1);
}
