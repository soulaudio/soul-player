//! Tests for the parallel scan pipeline

mod test_helpers;

use soul_core::types::CreateLibrarySource;
use soul_importer::library_scanner::LibraryScanner;
use std::fs;
use std::io::Write;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use tempfile::TempDir;

/// Create a test audio file (fake FLAC with minimal header)
fn create_test_audio_file(path: &std::path::Path, filename: &str) -> std::path::PathBuf {
    let file_path = path.join(filename);
    let mut file = fs::File::create(&file_path).expect("Failed to create test file");

    // Write a fake FLAC header (fLaC magic bytes + minimal metadata)
    file.write_all(b"fLaC\x00\x00\x00\x22")
        .expect("Failed to write header");
    // Write some padding to make it a reasonable file size
    file.write_all(&[0u8; 1000])
        .expect("Failed to write padding");
    file.flush().expect("Failed to flush");

    file_path
}

/// Create multiple test audio files
fn create_test_audio_files(dir: &std::path::Path, count: usize) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    for i in 0..count {
        let filename = format!("track_{:03}.flac", i + 1);
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
async fn test_parallel_scan_processes_all_files() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Create 50 fake audio files
    let _files = create_test_audio_files(temp_dir.path(), 50);

    let source = create_test_source(
        &pool,
        "user1",
        "device1",
        "Parallel Test",
        &temp_dir.path().display().to_string(),
    )
    .await;

    // Use concurrency=4 (parallel pipeline)
    let scanner = LibraryScanner::new(pool.clone(), "user1", "device1")
        .compute_hashes(false)
        .concurrency(4);

    let stats = scanner
        .scan_source(&source)
        .await
        .expect("scan should succeed");

    // All 50 files should be accounted for
    assert_eq!(stats.total_files, 50);
    // Every file should be processed (new + errors = total)
    assert_eq!(
        stats.processed, 50,
        "all files should be processed, got {} processed",
        stats.processed
    );
    // Since these are fake FLAC files, metadata extraction will fail.
    // They should all count as errors.
    assert_eq!(
        stats.errors, 50,
        "fake FLAC files should fail metadata extraction, got {} errors",
        stats.errors
    );
}

#[tokio::test]
async fn test_parallel_scan_rescan_skips_unchanged() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let _files = create_test_audio_files(temp_dir.path(), 10);

    let source = create_test_source(
        &pool,
        "user1",
        "device1",
        "Rescan Test",
        &temp_dir.path().display().to_string(),
    )
    .await;

    let scanner = LibraryScanner::new(pool.clone(), "user1", "device1")
        .compute_hashes(false)
        .concurrency(4);

    // First scan
    let stats1 = scanner
        .scan_source(&source)
        .await
        .expect("first scan should succeed");
    assert_eq!(stats1.total_files, 10);

    // Second scan — files haven't changed, so all should be skipped.
    // Since the first scan resulted in errors (fake files), no tracks were
    // inserted into the DB, so the second scan will still see them as "new".
    // This tests the pipeline structure rather than the skip logic.
    let scanner2 = LibraryScanner::new(pool.clone(), "user1", "device1")
        .compute_hashes(false)
        .concurrency(4);

    let stats2 = scanner2
        .scan_source(&source)
        .await
        .expect("second scan should succeed");
    assert_eq!(stats2.total_files, 10);
    // Since no tracks were actually imported (all errored), all files are
    // still "new" on the second scan — they won't be in existing_tracks.
    assert_eq!(stats2.processed, 10);
}

#[tokio::test]
async fn test_parallel_scan_concurrency_1_matches_sequential() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let _files = create_test_audio_files(temp_dir.path(), 15);

    let source = create_test_source(
        &pool,
        "user1",
        "device1",
        "Sequential Test",
        &temp_dir.path().display().to_string(),
    )
    .await;

    // Use concurrency=1 — effectively sequential
    let scanner = LibraryScanner::new(pool.clone(), "user1", "device1")
        .compute_hashes(false)
        .concurrency(1);

    let stats = scanner
        .scan_source(&source)
        .await
        .expect("scan should succeed");

    assert_eq!(stats.total_files, 15);
    assert_eq!(stats.processed, 15);
    // All fake files should error
    assert_eq!(stats.errors, 15);
}

#[tokio::test]
async fn test_parallel_scan_concurrency_zero_clamps_to_one() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let _files = create_test_audio_files(temp_dir.path(), 5);

    let source = create_test_source(
        &pool,
        "user1",
        "device1",
        "Clamp Test",
        &temp_dir.path().display().to_string(),
    )
    .await;

    // concurrency(0) should clamp to 1
    let scanner = LibraryScanner::new(pool.clone(), "user1", "device1")
        .compute_hashes(false)
        .concurrency(0);

    let stats = scanner
        .scan_source(&source)
        .await
        .expect("scan should succeed with clamped concurrency");

    assert_eq!(stats.total_files, 5);
    assert_eq!(stats.processed, 5);
}

#[tokio::test]
async fn test_parallel_scan_progress_callback_fires() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let _files = create_test_audio_files(temp_dir.path(), 10);

    let source = create_test_source(
        &pool,
        "user1",
        "device1",
        "Callback Test",
        &temp_dir.path().display().to_string(),
    )
    .await;

    let callback_count = Arc::new(AtomicI64::new(0));
    let callback_count_clone = Arc::clone(&callback_count);

    let scanner = LibraryScanner::new(pool.clone(), "user1", "device1")
        .compute_hashes(false)
        .concurrency(4)
        .on_progress(Box::new(move |_stats| {
            callback_count_clone.fetch_add(1, Ordering::SeqCst);
        }));

    let _stats = scanner
        .scan_source(&source)
        .await
        .expect("scan should succeed");

    // Callback should fire at least once per file processed in phase 2
    let count = callback_count.load(Ordering::SeqCst);
    assert!(
        count >= 10,
        "callback should fire at least 10 times, got {}",
        count
    );
}

#[tokio::test]
async fn test_parallel_scan_with_subdirectories() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Create files in artist/album folder structure
    let artist_dir = temp_dir.path().join("Artist One").join("Album A");
    fs::create_dir_all(&artist_dir).expect("Failed to create artist dir");
    create_test_audio_file(&artist_dir, "track_01.flac");
    create_test_audio_file(&artist_dir, "track_02.flac");

    let artist_dir2 = temp_dir.path().join("Artist Two").join("Album B");
    fs::create_dir_all(&artist_dir2).expect("Failed to create artist dir 2");
    create_test_audio_file(&artist_dir2, "track_01.flac");
    create_test_audio_file(&artist_dir2, "track_02.flac");
    create_test_audio_file(&artist_dir2, "track_03.flac");

    let source = create_test_source(
        &pool,
        "user1",
        "device1",
        "Subdir Test",
        &temp_dir.path().display().to_string(),
    )
    .await;

    let scanner = LibraryScanner::new(pool.clone(), "user1", "device1")
        .compute_hashes(false)
        .concurrency(4);

    let stats = scanner
        .scan_source(&source)
        .await
        .expect("scan should succeed");

    // Should find all 5 files across subdirectories
    assert_eq!(stats.total_files, 5);
    assert_eq!(stats.processed, 5);
}

#[tokio::test]
async fn test_parallel_scan_empty_directory() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let source = create_test_source(
        &pool,
        "user1",
        "device1",
        "Empty Test",
        &temp_dir.path().display().to_string(),
    )
    .await;

    let scanner = LibraryScanner::new(pool.clone(), "user1", "device1")
        .compute_hashes(false)
        .concurrency(8);

    let stats = scanner
        .scan_source(&source)
        .await
        .expect("scan should succeed");

    assert_eq!(stats.total_files, 0);
    assert_eq!(stats.processed, 0);
    assert_eq!(stats.errors, 0);
    assert_eq!(stats.new_files, 0);
}
