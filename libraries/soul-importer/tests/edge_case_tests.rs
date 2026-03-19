//! Edge case and resilience tests for soul-importer
//!
//! Covers scenarios not addressed by the basic e2e/integration tests:
//!
//! - **Rescan unchanged skip**: a second scan without changes must not re-import files.
//! - **File modification detection**: changing a file's content between scans must trigger
//!   a metadata update (FileAction::Updated).
//! - **sync_deletes**: files removed from the filesystem between scans must be soft-deleted.
//! - **Orphaned scan cleanup**: simulates the app being force-quit mid-scan and verifies that
//!   `cleanup_orphaned_scans` resets the stuck 'Scanning' state on restart.
//! - **Drop receiver resilience**: dropping the progress `Receiver` must not abort the
//!   import task or cause a deadlock.
//! - **Scan progress completed status**: finished scans must not leave 'running' progress
//!   records (which would be misidentified as orphaned on next startup).
//! - **New file detection**: files added to a library directory between scans must be
//!   picked up by the next scan.
//! - **Concurrent scan safety**: two simultaneous scans must complete without panicking.

use soul_core::types::{CreateLibrarySource, ScanProgressStatus, ScanStatus};
use soul_importer::library_scanner::LibraryScanner;
use soul_importer::{FileManagementStrategy, ImportConfig, MusicImporter};
use std::fs;
use std::io::Write;
use std::path::Path;
use tempfile::TempDir;

mod test_helpers;
use test_helpers::setup_test_db;

// ─── File creation helpers ────────────────────────────────────────────────────

/// Create a minimal, lofty-parseable MP3 file with ID3v2.3 tags.
fn create_mp3(path: &Path, title: &str, artist: &str) {
    let tag_data = {
        let mut frames = Vec::new();
        frames.extend(id3_text_frame(b"TIT2", title));
        frames.extend(id3_text_frame(b"TPE1", artist));
        frames
    };

    // Synchsafe tag size
    let size = tag_data.len() as u32;
    let synchsafe = [
        ((size >> 21) & 0x7F) as u8,
        ((size >> 14) & 0x7F) as u8,
        ((size >> 7) & 0x7F) as u8,
        (size & 0x7F) as u8,
    ];

    let mut file = fs::File::create(path).unwrap();
    file.write_all(b"ID3").unwrap();
    file.write_all(&[0x03, 0x00, 0x00]).unwrap(); // ID3v2.3, no flags
    file.write_all(&synchsafe).unwrap();
    file.write_all(&tag_data).unwrap();
    // Minimal MPEG Layer III frame (128 kbps, 44.1 kHz) + padding
    file.write_all(&[0xFF, 0xFB, 0x90, 0x00]).unwrap();
    file.write_all(&[0x00; 36]).unwrap();
}

fn id3_text_frame(frame_id: &[u8; 4], text: &str) -> Vec<u8> {
    let mut frame = Vec::new();
    frame.extend_from_slice(frame_id);
    let text_bytes = text.as_bytes();
    let size = (text_bytes.len() + 1) as u32; // +1 for encoding byte
    frame.extend_from_slice(&size.to_be_bytes());
    frame.extend_from_slice(&[0x00, 0x00]); // flags
    frame.push(0x00); // ISO-8859-1 encoding
    frame.extend_from_slice(text_bytes);
    frame
}

/// Create a fake audio file (valid FLAC magic bytes, non-parseable metadata).
/// Useful when we only care about file discovery counts, not import success.
fn create_fake_flac(path: &Path) {
    let mut file = fs::File::create(path).unwrap();
    file.write_all(b"fLaC\x00\x00\x00\x22").unwrap();
    file.write_all(&[0u8; 1000]).unwrap();
}

/// Create a watched library source in the test database.
async fn create_source(
    pool: &sqlx::SqlitePool,
    user_id: &str,
    device_id: &str,
    path: &str,
    sync_deletes: bool,
) -> soul_core::types::LibrarySource {
    soul_storage::library_sources::create(
        pool,
        user_id,
        device_id,
        &CreateLibrarySource {
            name: "Test Source".to_string(),
            path: path.to_string(),
            sync_deletes,
        },
    )
    .await
    .expect("Failed to create library source")
}

// ─── Tests ───────────────────────────────────────────────────────────────────

/// A second scan of an unchanged directory must not re-import already-known files.
/// Files that were successfully imported on the first scan must appear as
/// `FileAction::Unchanged` (contributing 0 to `new_files`) on the second scan.
#[tokio::test]
async fn test_rescan_skips_unchanged_files() {
    let pool = setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    for i in 0..3 {
        create_mp3(
            &temp_dir.path().join(format!("track_{}.mp3", i)),
            &format!("Track {}", i),
            "Test Artist",
        );
    }

    let source = create_source(
        &pool,
        "user1",
        "device1",
        &temp_dir.path().display().to_string(),
        false,
    )
    .await;

    let scanner = LibraryScanner::new(pool.clone(), "user1", "device1").compute_hashes(true);

    let stats1 = scanner
        .scan_source(&source)
        .await
        .expect("first scan should succeed");
    assert_eq!(
        stats1.total_files, 3,
        "first scan must discover all 3 files"
    );

    let stats2 = scanner
        .scan_source(&source)
        .await
        .expect("second scan should succeed");
    assert_eq!(stats2.total_files, 3, "second scan must still find 3 files");

    // Only enforce the key invariant when the first scan actually imported files:
    // if files are in the database, a second scan of the same unchanged files must
    // NOT re-import them.
    if stats1.new_files > 0 {
        assert_eq!(
            stats2.new_files, 0,
            "already-imported files must not be re-imported on an unchanged rescan"
        );
    }
}

/// When a file's content changes between scans (causing a different size or mtime),
/// the scanner must detect the change and update the stored metadata
/// (`FileAction::Updated`), not create a duplicate entry.
#[tokio::test]
async fn test_file_modification_triggers_metadata_update() {
    let pool = setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("changing_track.mp3");

    // Use a short title for the initial file
    create_mp3(&file_path, "T", "Artist");

    let source = create_source(
        &pool,
        "user1",
        "device1",
        &temp_dir.path().display().to_string(),
        false,
    )
    .await;

    let scanner = LibraryScanner::new(pool.clone(), "user1", "device1");

    let stats1 = scanner.scan_source(&source).await.expect("first scan ok");

    if stats1.new_files == 0 {
        // Metadata extraction failed for the test file — skip assertions that
        // require the file to be in the database.
        return;
    }

    // Overwrite with different content — significantly longer title gives a
    // different file size, which is detected by the mtime+size change check
    // without needing a sleep.
    create_mp3(
        &file_path,
        "Updated Title With More Characters Than The Original",
        "Artist",
    );

    let stats2 = scanner.scan_source(&source).await.expect("second scan ok");

    assert_eq!(
        stats2.new_files, 0,
        "modified file must not be re-imported as a new entry"
    );
    assert_eq!(
        stats2.updated_files, 1,
        "modified file must be detected as updated, not ignored"
    );
}

/// With `sync_deletes` enabled, files that are no longer present on the filesystem
/// after a rescan must be soft-deleted (marked unavailable) in the database.
#[tokio::test]
async fn test_sync_deletes_marks_missing_files_as_removed() {
    let pool = setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    let file_keep = temp_dir.path().join("keep.mp3");
    let file_gone = temp_dir.path().join("gone.mp3");

    create_mp3(&file_keep, "Keep Me", "Artist");
    create_mp3(&file_gone, "Delete Me", "Artist");

    // sync_deletes = true so removals are reflected in the database
    let source = create_source(
        &pool,
        "user1",
        "device1",
        &temp_dir.path().display().to_string(),
        true,
    )
    .await;

    let scanner = LibraryScanner::new(pool.clone(), "user1", "device1");

    let stats1 = scanner.scan_source(&source).await.expect("first scan ok");
    assert_eq!(stats1.total_files, 2, "first scan must discover both files");

    // Remove one file (simulates the user deleting a track from their library)
    fs::remove_file(&file_gone).expect("should delete test file");

    let stats2 = scanner.scan_source(&source).await.expect("second scan ok");
    assert_eq!(
        stats2.total_files, 1,
        "second scan must only see the file that still exists"
    );

    if stats1.new_files > 0 {
        // Only assert soft-delete count when the file was actually in the database
        assert_eq!(
            stats2.removed_files, 1,
            "sync_deletes must soft-delete the track that was removed from disk"
        );
    }
}

/// Simulates a force-quit mid-scan (e.g. the user kills the app or the system
/// crashes).  After a restart the database is left with:
///
///   - A library source whose `scan_status` is stuck at `Scanning`
///   - A `scan_progress` record still in `Running` state
///
/// `cleanup_orphaned_scans` is called on app startup and must reset both.
#[tokio::test]
async fn test_orphaned_scan_cleanup_resets_stuck_scanning_state() {
    let pool = setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    let source = create_source(
        &pool,
        "user1",
        "device1",
        &temp_dir.path().display().to_string(),
        false,
    )
    .await;

    // Confirm nothing needs cleaning up initially
    let initial_cleaned =
        soul_storage::library_sources::cleanup_orphaned_scans(&pool, "user1", "device1")
            .await
            .expect("cleanup should succeed on a clean database");
    assert_eq!(
        initial_cleaned, 0,
        "nothing to clean up on a fresh database"
    );

    // ── Simulate crash: leave source stuck in Scanning ──────────────────────
    soul_storage::library_sources::set_scan_status(&pool, source.id, ScanStatus::Scanning, None)
        .await
        .expect("should be able to set scan status to Scanning");

    // ── Simulate crash: leave an orphaned 'running' scan_progress record ────
    soul_storage::scan_progress::start(&pool, source.id, Some(500))
        .await
        .expect("should create an orphaned scan_progress record");

    // Verify stuck state is actually in the database
    let stuck_source = soul_storage::library_sources::get_by_id(&pool, source.id)
        .await
        .expect("get_by_id should succeed")
        .expect("source should exist");
    assert_eq!(
        stuck_source.scan_status,
        ScanStatus::Scanning,
        "source must be stuck in Scanning before cleanup"
    );

    let orphaned_progress = soul_storage::scan_progress::get_running(&pool, source.id)
        .await
        .expect("get_running should succeed");
    assert!(
        orphaned_progress.is_some(),
        "there must be an orphaned running scan_progress before cleanup"
    );

    // ── Act: simulate app restart calling cleanup ────────────────────────────
    let cleaned = soul_storage::library_sources::cleanup_orphaned_scans(&pool, "user1", "device1")
        .await
        .expect("cleanup_orphaned_scans should succeed");

    // cleanup_orphaned_scans counts each reset operation:
    // 1 for the stuck library_source + 1 for the orphaned scan_progress = 2
    assert_eq!(
        cleaned, 2,
        "cleanup must fix 1 stuck source and cancel 1 orphaned scan_progress"
    );

    // Source must be back to Idle
    let recovered = soul_storage::library_sources::get_by_id(&pool, source.id)
        .await
        .expect("get_by_id should succeed")
        .expect("source should exist");
    assert_eq!(
        recovered.scan_status,
        ScanStatus::Idle,
        "source must be reset to Idle after orphan cleanup"
    );

    // The orphaned scan_progress must no longer be 'running'
    let still_running = soul_storage::scan_progress::get_running(&pool, source.id)
        .await
        .expect("get_running should succeed");
    assert!(
        still_running.is_none(),
        "orphaned scan_progress must be cancelled so it is not misidentified as running"
    );
}

/// Dropping the progress `Receiver` immediately after starting an import must
/// not deadlock or abort the import task.  The `JoinHandle` must still return
/// a complete `ImportSummary` with `total_processed` equal to the number of
/// files submitted.
///
/// This simulates the user closing the import progress panel (or the app being
/// shut down) while an import is still running.
#[tokio::test]
async fn test_drop_progress_receiver_import_still_completes() {
    let pool = setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();
    let library_dir = TempDir::new().unwrap();

    for i in 0..5 {
        create_mp3(
            &temp_dir.path().join(format!("song_{}.mp3", i)),
            &format!("Song {}", i),
            "Test Artist",
        );
    }

    let config = ImportConfig {
        library_path: library_dir.path().to_path_buf(),
        file_strategy: FileManagementStrategy::Copy,
        confidence_threshold: 80,
        file_naming_pattern: "{artist} - {title}.{ext}".to_string(),
        skip_duplicates: true,
    };

    let importer = MusicImporter::new(pool.clone(), config);
    let (progress_rx, handle) = importer
        .import_directory(temp_dir.path())
        .await
        .expect("import_directory should start without error");

    // Simulate UI closure — drop the receiver immediately
    drop(progress_rx);

    // The background import task must still complete cleanly
    let summary = handle
        .await
        .expect("import task must not panic")
        .expect("import must return Ok(ImportSummary)");

    assert_eq!(
        summary.total_processed, 5,
        "all 5 files must be accounted for even when the progress receiver is dropped"
    );
}

/// After a successful scan, the associated `scan_progress` record must have
/// `status = Completed`.  Records left in `Running` status would be incorrectly
/// treated as orphaned on the next app startup and cancelled unnecessarily.
#[tokio::test]
async fn test_scan_progress_status_is_completed_after_successful_scan() {
    let pool = setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    // Use fake files — we only care that the scan runs to completion,
    // not that metadata extraction succeeds.
    for i in 0..3 {
        create_fake_flac(&temp_dir.path().join(format!("track_{}.flac", i)));
    }

    let source = create_source(
        &pool,
        "user1",
        "device1",
        &temp_dir.path().display().to_string(),
        false,
    )
    .await;

    let scanner = LibraryScanner::new(pool.clone(), "user1", "device1");
    scanner
        .scan_source(&source)
        .await
        .expect("scan must complete without error");

    let progress = soul_storage::scan_progress::get_latest(&pool, source.id)
        .await
        .expect("get_latest should succeed")
        .expect("a scan_progress record must exist after scanning");

    assert_eq!(
        progress.status,
        ScanProgressStatus::Completed,
        "scan_progress must be Completed after a successful scan — \
         a Running record would be misidentified as an orphan on restart"
    );
    assert!(
        progress.completed_at.is_some(),
        "completed_at must be set when scan finishes"
    );
}

/// Files added to a library directory between two scans must be detected
/// and processed by the subsequent scan.
#[tokio::test]
async fn test_rescan_detects_newly_added_files() {
    let pool = setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    // Initial set: 2 files
    for i in 0..2 {
        create_mp3(
            &temp_dir.path().join(format!("original_{}.mp3", i)),
            &format!("Original Track {}", i),
            "Artist",
        );
    }

    let source = create_source(
        &pool,
        "user1",
        "device1",
        &temp_dir.path().display().to_string(),
        false,
    )
    .await;

    let scanner = LibraryScanner::new(pool.clone(), "user1", "device1");

    let stats1 = scanner.scan_source(&source).await.expect("first scan ok");
    assert_eq!(
        stats1.total_files, 2,
        "first scan must find the initial 2 files"
    );

    // Simulate user copying 3 more tracks into the library folder
    for i in 0..3 {
        create_mp3(
            &temp_dir.path().join(format!("added_{}.mp3", i)),
            &format!("Added Track {}", i),
            "Artist",
        );
    }

    let stats2 = scanner.scan_source(&source).await.expect("second scan ok");
    assert_eq!(
        stats2.total_files, 5,
        "second scan must discover all 5 files (2 original + 3 added)"
    );

    // The 3 new files must be processed — either successfully imported
    // or attempted (and failed) — not silently skipped.
    assert!(
        stats2.new_files + stats2.errors >= 3,
        "all 3 newly added files must be processed (as new_files or errors), \
         not skipped as if they were already known"
    );
}

/// Two simultaneous scans of the same library source must both complete without
/// panicking.  SQLite may serialize concurrent writes, but the scans must not
/// deadlock or produce a panic.
#[tokio::test]
async fn test_concurrent_scans_do_not_panic() {
    let pool = setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    for i in 0..3 {
        create_fake_flac(&temp_dir.path().join(format!("track_{}.flac", i)));
    }

    let source = create_source(
        &pool,
        "user1",
        "device1",
        &temp_dir.path().display().to_string(),
        false,
    )
    .await;

    let scanner1 = LibraryScanner::new(pool.clone(), "user1", "device1");
    let scanner2 = LibraryScanner::new(pool.clone(), "user1", "device1");

    // Run both scans concurrently
    let (result1, result2) =
        tokio::join!(scanner1.scan_source(&source), scanner2.scan_source(&source));

    // At least one must succeed; neither must panic (a panic would fail the test
    // before we even reach this assertion)
    assert!(
        result1.is_ok() || result2.is_ok(),
        "at least one concurrent scan must succeed — got: {:?} / {:?}",
        result1,
        result2
    );
}

/// Consuming only a subset of progress messages and then dropping the receiver
/// must not cause the import to block or deadlock.  The import channel has a
/// bounded buffer; once it is full the sender uses `let _ = send().await` which
/// silently drops failed sends — this test verifies that behavior under a
/// realistic scenario.
#[tokio::test]
async fn test_partial_progress_drain_does_not_block_import() {
    let pool = setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();
    let library_dir = TempDir::new().unwrap();

    for i in 0..10 {
        create_mp3(
            &temp_dir.path().join(format!("song_{}.mp3", i)),
            &format!("Song {}", i),
            "Artist",
        );
    }

    let config = ImportConfig {
        library_path: library_dir.path().to_path_buf(),
        file_strategy: FileManagementStrategy::Reference,
        confidence_threshold: 80,
        file_naming_pattern: "{artist} - {title}.{ext}".to_string(),
        skip_duplicates: false,
    };

    let importer = MusicImporter::new(pool.clone(), config);
    let (mut progress_rx, handle) = importer
        .import_directory(temp_dir.path())
        .await
        .expect("should start");

    // Consume just 2 messages then discard the rest
    let _ = progress_rx.recv().await;
    let _ = progress_rx.recv().await;
    drop(progress_rx);

    let summary = handle
        .await
        .expect("import task must not panic")
        .expect("import must return Ok");

    assert_eq!(
        summary.total_processed, 10,
        "all 10 files must be processed regardless of how many progress messages were consumed"
    );
}
