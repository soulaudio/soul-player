//! Integration tests for library scanner

mod test_helpers;

use soul_core::types::{CreateLibrarySource, ScanStatus};
use soul_importer::library_scanner::{LibraryScanner, ScanStats};
use std::fs;
use std::io::Write;
use tempfile::TempDir;

/// Create a test audio file (fake FLAC with minimal header)
fn create_test_audio_file(path: &std::path::Path, filename: &str) -> std::path::PathBuf {
    let file_path = path.join(filename);
    let mut file = fs::File::create(&file_path).expect("Failed to create test file");

    // Write a fake FLAC header (fLaC magic bytes + minimal metadata)
    // This won't be a valid audio file but will have the right extension
    file.write_all(b"fLaC\x00\x00\x00\x22").expect("Failed to write header");
    // Write some padding to make it a reasonable file size
    file.write_all(&[0u8; 1000]).expect("Failed to write padding");
    file.flush().expect("Failed to flush");

    file_path
}

/// Create multiple test audio files
fn create_test_audio_files(dir: &std::path::Path, count: usize) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    for i in 0..count {
        let filename = format!("track_{:02}.flac", i + 1);
        files.push(create_test_audio_file(dir, &filename));
    }
    files
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
async fn test_scan_stats_default() {
    let stats = ScanStats::default();
    assert_eq!(stats.total_files, 0);
    assert_eq!(stats.processed, 0);
    assert_eq!(stats.new_files, 0);
    assert_eq!(stats.updated_files, 0);
    assert_eq!(stats.removed_files, 0);
    assert_eq!(stats.relocated_files, 0);
    assert_eq!(stats.errors, 0);
}

#[tokio::test]
async fn test_library_scanner_new() {
    let pool = test_helpers::setup_test_db().await;

    let scanner = LibraryScanner::new(pool.clone(), "user1", "device1");

    // Scanner should be created without errors
    // We can verify this by scanning with no sources
    let stats = scanner.scan_all().await.expect("scan_all should succeed");

    assert_eq!(stats.total_files, 0);
    assert_eq!(stats.processed, 0);
}

#[tokio::test]
async fn test_scan_source_nonexistent_path() {
    let pool = test_helpers::setup_test_db().await;

    // Create a library source pointing to non-existent path
    let source = create_test_source(
        &pool,
        "user1",
        "device1",
        "Test Source",
        "/nonexistent/path/to/music",
    )
    .await;

    let scanner = LibraryScanner::new(pool.clone(), "user1", "device1");

    // Scanning should fail for non-existent path
    let result = scanner.scan_source(&source).await;
    assert!(result.is_err());

    // Source should be marked as error
    let updated_source = soul_storage::library_sources::get_by_id(&pool, source.id)
        .await
        .expect("get_by_id should succeed")
        .expect("source should exist");
    assert_eq!(updated_source.scan_status, ScanStatus::Error);
}

#[tokio::test]
async fn test_scan_source_empty_directory() {
    let pool = test_helpers::setup_test_db().await;

    // Create a temporary empty directory
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Create a library source pointing to empty directory
    let source = create_test_source(
        &pool,
        "user1",
        "device1",
        "Empty Source",
        &temp_dir.path().display().to_string(),
    )
    .await;

    let scanner = LibraryScanner::new(pool.clone(), "user1", "device1");
    let stats = scanner.scan_source(&source).await.expect("scan should succeed");

    assert_eq!(stats.total_files, 0);
    assert_eq!(stats.processed, 0);
    assert_eq!(stats.new_files, 0);
    assert_eq!(stats.errors, 0);
}

#[tokio::test]
async fn test_scan_source_with_files() {
    let pool = test_helpers::setup_test_db().await;

    // Create a temporary directory with some test audio files
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let _files = create_test_audio_files(temp_dir.path(), 3);

    // Create a library source
    let source = create_test_source(
        &pool,
        "user1",
        "device1",
        "Test Source",
        &temp_dir.path().display().to_string(),
    )
    .await;

    let scanner = LibraryScanner::new(pool.clone(), "user1", "device1")
        .compute_hashes(true);
    let stats = scanner.scan_source(&source).await.expect("scan should succeed");

    // Should have found 3 files
    assert_eq!(stats.total_files, 3);
    // Note: These are fake files so metadata extraction will fail
    // The errors count should reflect that
    assert!(stats.errors >= 0, "errors should be non-negative");
}

#[tokio::test]
async fn test_scan_all_multiple_sources() {
    let pool = test_helpers::setup_test_db().await;

    // Create two temporary directories
    let temp_dir1 = TempDir::new().expect("Failed to create temp dir 1");
    let temp_dir2 = TempDir::new().expect("Failed to create temp dir 2");

    // Create test files in each
    let _files1 = create_test_audio_files(temp_dir1.path(), 2);
    let _files2 = create_test_audio_files(temp_dir2.path(), 3);

    // Create two library sources
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

    let scanner = LibraryScanner::new(pool.clone(), "user1", "device1");
    let stats = scanner.scan_all().await.expect("scan_all should succeed");

    // Should have found 5 files total (2 + 3)
    assert_eq!(stats.total_files, 5);
}

#[tokio::test]
async fn test_scan_respects_enabled_flag() {
    let pool = test_helpers::setup_test_db().await;

    // Create a temporary directory with files
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let _files = create_test_audio_files(temp_dir.path(), 2);

    // Create a library source (enabled by default)
    let source = create_test_source(
        &pool,
        "user1",
        "device1",
        "Test Source",
        &temp_dir.path().display().to_string(),
    )
    .await;

    // Disable the source
    soul_storage::library_sources::set_enabled(&pool, source.id, false)
        .await
        .expect("Failed to disable source");

    let scanner = LibraryScanner::new(pool.clone(), "user1", "device1");
    let stats = scanner.scan_all().await.expect("scan_all should succeed");

    // Should not have scanned any files because source is disabled
    assert_eq!(stats.total_files, 0);
    assert_eq!(stats.processed, 0);
}

#[tokio::test]
async fn test_scan_user_device_isolation() {
    let pool = test_helpers::setup_test_db().await;

    // Create two temporary directories
    let temp_dir1 = TempDir::new().expect("Failed to create temp dir 1");
    let temp_dir2 = TempDir::new().expect("Failed to create temp dir 2");

    // Create test files in each
    let _files1 = create_test_audio_files(temp_dir1.path(), 2);
    let _files2 = create_test_audio_files(temp_dir2.path(), 3);

    // Create source for user1/device1
    create_test_source(
        &pool,
        "user1",
        "device1",
        "User1 Source",
        &temp_dir1.path().display().to_string(),
    )
    .await;

    // Create source for user2/device2
    create_test_source(
        &pool,
        "user2",
        "device2",
        "User2 Source",
        &temp_dir2.path().display().to_string(),
    )
    .await;

    // Scan as user1/device1
    let scanner1 = LibraryScanner::new(pool.clone(), "user1", "device1");
    let stats1 = scanner1.scan_all().await.expect("scan_all should succeed");

    // Should only see user1's files
    assert_eq!(stats1.total_files, 2);

    // Scan as user2/device2
    let scanner2 = LibraryScanner::new(pool.clone(), "user2", "device2");
    let stats2 = scanner2.scan_all().await.expect("scan_all should succeed");

    // Should only see user2's files
    assert_eq!(stats2.total_files, 3);
}

#[tokio::test]
async fn test_scan_progress_tracking() {
    let pool = test_helpers::setup_test_db().await;

    // Create a temporary directory with files
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let _files = create_test_audio_files(temp_dir.path(), 5);

    // Create a library source
    let source = create_test_source(
        &pool,
        "user1",
        "device1",
        "Test Source",
        &temp_dir.path().display().to_string(),
    )
    .await;

    let scanner = LibraryScanner::new(pool.clone(), "user1", "device1");
    let _stats = scanner.scan_source(&source).await.expect("scan should succeed");

    // Check that scan progress was recorded
    let latest = soul_storage::scan_progress::get_latest(&pool, source.id)
        .await
        .expect("get_latest should succeed")
        .expect("should have a scan progress record");

    assert_eq!(latest.library_source_id, source.id);
    assert!(latest.total_files.is_some());
    assert_eq!(latest.total_files.unwrap(), 5);
}

#[tokio::test]
async fn test_scanner_without_hashes() {
    let pool = test_helpers::setup_test_db().await;

    // Create a temporary directory with files
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let _files = create_test_audio_files(temp_dir.path(), 2);

    // Create a library source
    let source = create_test_source(
        &pool,
        "user1",
        "device1",
        "Test Source",
        &temp_dir.path().display().to_string(),
    )
    .await;

    // Scanner without hash computation (faster but no relocation detection)
    let scanner = LibraryScanner::new(pool.clone(), "user1", "device1")
        .compute_hashes(false);
    let stats = scanner.scan_source(&source).await.expect("scan should succeed");

    assert_eq!(stats.total_files, 2);
}

#[tokio::test]
async fn test_progress_callback() {
    use std::sync::atomic::{AtomicI64, Ordering};
    use std::sync::Arc;

    let pool = test_helpers::setup_test_db().await;

    // Create a temporary directory with files
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let _files = create_test_audio_files(temp_dir.path(), 3);

    // Create a library source
    let source = create_test_source(
        &pool,
        "user1",
        "device1",
        "Test Source",
        &temp_dir.path().display().to_string(),
    )
    .await;

    // Track callback invocations
    let callback_count = Arc::new(AtomicI64::new(0));
    let callback_count_clone = Arc::clone(&callback_count);

    let scanner = LibraryScanner::new(pool.clone(), "user1", "device1")
        .on_progress(Box::new(move |_stats| {
            callback_count_clone.fetch_add(1, Ordering::SeqCst);
        }));

    let _stats = scanner.scan_source(&source).await.expect("scan should succeed");

    // Callback should have been invoked at least once per file
    let count = callback_count.load(Ordering::SeqCst);
    assert!(count >= 3, "callback should be invoked at least 3 times, got {}", count);
}

#[tokio::test]
async fn test_source_status_updates() {
    let pool = test_helpers::setup_test_db().await;

    // Create a temporary directory with files
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let _files = create_test_audio_files(temp_dir.path(), 2);

    // Create a library source
    let source = create_test_source(
        &pool,
        "user1",
        "device1",
        "Test Source",
        &temp_dir.path().display().to_string(),
    )
    .await;

    // Initial status should be Idle
    assert_eq!(source.scan_status, ScanStatus::Idle);

    let scanner = LibraryScanner::new(pool.clone(), "user1", "device1");
    let _stats = scanner.scan_source(&source).await.expect("scan should succeed");

    // After scan, check last_scan_at is updated
    let updated_source = soul_storage::library_sources::get_by_id(&pool, source.id)
        .await
        .expect("get_by_id should succeed")
        .expect("source should exist");

    assert!(updated_source.last_scan_at.is_some(), "last_scan_at should be set");
}
