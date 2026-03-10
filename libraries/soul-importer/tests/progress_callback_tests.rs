//! TDD tests for scan progress_callback emission.
//!
//! Root causes these tests guard against:
//!   1. progress_callback never called for scans with < 10 files
//!      (flush_progress fired only every 10 files in Phase 2).
//!   2. progress_callback never called for entirely-skipped scans
//!      (final flush used bare update_counts, not flush_progress).
//!
//! Each test verifies:
//!   - At least one callback fires with total_files > 0
//!   - The first callback has total_files set correctly
//!   - The final callback has processed == total_files (or close)

mod test_helpers;

use soul_core::types::CreateLibrarySource;
use soul_importer::library_scanner::{LibraryScanner, ScanStats};
use std::fs;
use std::io::Write;
use std::sync::{Arc, Mutex};
use tempfile::TempDir;

// ── helpers ─────────────────────────────────────────────────────────────────

fn create_fake_audio_file(dir: &std::path::Path, name: &str) {
    let path = dir.join(name);
    let mut f = fs::File::create(&path).expect("create file");
    // Minimal FLAC magic bytes so the scanner recognises the extension
    f.write_all(b"fLaC\x00\x00\x00\x22").unwrap();
    f.write_all(&[0u8; 512]).unwrap();
}

async fn create_source(pool: &sqlx::SqlitePool, path: &str) -> soul_core::types::LibrarySource {
    soul_storage::library_sources::create(
        pool,
        "user1",
        "device1",
        &CreateLibrarySource {
            name: "Test".to_string(),
            path: path.to_string(),
            sync_deletes: false,
        },
    )
    .await
    .expect("create library source")
}

// ── tests ────────────────────────────────────────────────────────────────────

/// When a library has 5 files (< 10, below the every-10-files flush threshold),
/// the progress_callback MUST still be called at least twice:
///   1. Immediately after set_total_files (initial state: total=5, processed=0)
///   2. At the end of the scan (final state: processed=5)
#[tokio::test]
async fn test_progress_callback_called_for_small_library() {
    let pool = test_helpers::setup_test_db().await;
    let temp = TempDir::new().unwrap();

    for i in 0..5 {
        create_fake_audio_file(temp.path(), &format!("track_{:02}.flac", i + 1));
    }

    let source = create_source(&pool, &temp.path().display().to_string()).await;

    let calls: Arc<Mutex<Vec<ScanStats>>> = Arc::new(Mutex::new(Vec::new()));
    let calls_clone = Arc::clone(&calls);

    let scanner =
        LibraryScanner::new(pool.clone(), "user1", "device1").on_progress(Box::new(move |stats| {
            calls_clone.lock().unwrap().push(stats.clone());
        }));

    scanner
        .scan_source(&source)
        .await
        .expect("scan should succeed");

    let captured = calls.lock().unwrap();

    // Must have been called at least twice: initial + final
    assert!(
        captured.len() >= 2,
        "expected >= 2 progress callbacks for 5-file scan, got {}",
        captured.len()
    );

    // First call: total_files is set, processed may be 0
    let first = &captured[0];
    assert_eq!(
        first.total_files, 5,
        "first callback must report total_files = 5"
    );
    assert_eq!(
        first.processed, 0,
        "first callback must report processed = 0 (initial state)"
    );

    // Last call: all files accounted for
    let last = captured.last().unwrap();
    assert_eq!(
        last.total_files, 5,
        "final callback must still report total_files = 5"
    );
    assert_eq!(
        last.processed, 5,
        "final callback must report processed = total_files"
    );
}

/// When all files are unchanged (normal rescan, nothing to re-extract),
/// the progress_callback MUST still fire — previously the final flush used
/// bare update_counts which never called the callback.
#[tokio::test]
async fn test_progress_callback_called_when_all_files_skipped() {
    let pool = test_helpers::setup_test_db().await;
    let temp = TempDir::new().unwrap();

    for i in 0..3 {
        create_fake_audio_file(temp.path(), &format!("track_{:02}.flac", i + 1));
    }

    let source = create_source(&pool, &temp.path().display().to_string()).await;

    // First scan: import the files (they'll error on metadata but still be processed)
    LibraryScanner::new(pool.clone(), "user1", "device1")
        .scan_source(&source)
        .await
        .expect("first scan");

    // Second scan: files are unchanged → all skipped in Phase 1
    let calls: Arc<Mutex<Vec<ScanStats>>> = Arc::new(Mutex::new(Vec::new()));
    let calls_clone = Arc::clone(&calls);

    LibraryScanner::new(pool.clone(), "user1", "device1")
        .on_progress(Box::new(move |stats| {
            calls_clone.lock().unwrap().push(stats.clone());
        }))
        .scan_source(&source)
        .await
        .expect("second scan");

    let captured = calls.lock().unwrap();

    // Must have fired even though all files were skipped
    assert!(
        !captured.is_empty(),
        "progress_callback must fire even when all files are skipped; got 0 calls"
    );

    // total_files must be correct in at least the first call
    assert_eq!(
        captured[0].total_files, 3,
        "first callback must know there are 3 files total"
    );

    // Final callback must show all files processed (they were skipped but counted)
    let last = captured.last().unwrap();
    assert_eq!(
        last.processed, 3,
        "skipped files still count as processed in final callback"
    );
}

/// For a library with exactly 10 files (at the flush boundary), the callback
/// fires: once at init, once at the every-10 flush, and once at the final flush.
/// The final callback must report processed = 10.
#[tokio::test]
async fn test_progress_callback_at_ten_file_boundary() {
    let pool = test_helpers::setup_test_db().await;
    let temp = TempDir::new().unwrap();

    for i in 0..10 {
        create_fake_audio_file(temp.path(), &format!("track_{:02}.flac", i + 1));
    }

    let source = create_source(&pool, &temp.path().display().to_string()).await;

    let calls: Arc<Mutex<Vec<ScanStats>>> = Arc::new(Mutex::new(Vec::new()));
    let calls_clone = Arc::clone(&calls);

    LibraryScanner::new(pool.clone(), "user1", "device1")
        .on_progress(Box::new(move |stats| {
            calls_clone.lock().unwrap().push(stats.clone());
        }))
        .scan_source(&source)
        .await
        .expect("scan");

    let captured = calls.lock().unwrap();

    assert!(
        !captured.is_empty(),
        "progress_callback must fire for 10-file scan"
    );

    let first = &captured[0];
    assert_eq!(first.total_files, 10);
    assert_eq!(first.processed, 0);

    let last = captured.last().unwrap();
    assert_eq!(last.total_files, 10);
    assert_eq!(last.processed, 10);
}

/// For a library with more than 10 files (e.g., 15), the callback fires
/// at init + at the every-10 flush (after file 10) + at the final flush.
/// The total callback count must be >= 3.
#[tokio::test]
async fn test_progress_callback_multiple_flushes_for_large_library() {
    let pool = test_helpers::setup_test_db().await;
    let temp = TempDir::new().unwrap();

    for i in 0..15 {
        create_fake_audio_file(temp.path(), &format!("track_{:02}.flac", i + 1));
    }

    let source = create_source(&pool, &temp.path().display().to_string()).await;

    let calls: Arc<Mutex<Vec<ScanStats>>> = Arc::new(Mutex::new(Vec::new()));
    let calls_clone = Arc::clone(&calls);

    LibraryScanner::new(pool.clone(), "user1", "device1")
        .on_progress(Box::new(move |stats| {
            calls_clone.lock().unwrap().push(stats.clone());
        }))
        .scan_source(&source)
        .await
        .expect("scan");

    let captured = calls.lock().unwrap();

    // 1 initial + 1 at every-10 + 1 final = at least 3
    assert!(
        captured.len() >= 3,
        "expected >= 3 callbacks for 15-file scan (init + mid + final), got {}",
        captured.len()
    );

    let last = captured.last().unwrap();
    assert_eq!(last.total_files, 15);
    assert_eq!(last.processed, 15);
}

/// Force-refresh scan re-extracts all files even if unchanged. The callback
/// MUST fire with total_files > 0 and final processed == total_files.
#[tokio::test]
async fn test_progress_callback_with_force_refresh() {
    let pool = test_helpers::setup_test_db().await;
    let temp = TempDir::new().unwrap();

    for i in 0..4 {
        create_fake_audio_file(temp.path(), &format!("track_{:02}.flac", i + 1));
    }

    let source = create_source(&pool, &temp.path().display().to_string()).await;

    // First scan to populate DB
    LibraryScanner::new(pool.clone(), "user1", "device1")
        .scan_source(&source)
        .await
        .expect("initial scan");

    // Force-refresh scan: all files re-processed even if unchanged
    let calls: Arc<Mutex<Vec<ScanStats>>> = Arc::new(Mutex::new(Vec::new()));
    let calls_clone = Arc::clone(&calls);

    LibraryScanner::new(pool.clone(), "user1", "device1")
        .force_metadata_refresh(true)
        .on_progress(Box::new(move |stats| {
            calls_clone.lock().unwrap().push(stats.clone());
        }))
        .scan_source(&source)
        .await
        .expect("force-refresh scan");

    let captured = calls.lock().unwrap();

    assert!(
        !captured.is_empty(),
        "progress_callback must fire during force-refresh scan"
    );

    let first = &captured[0];
    assert_eq!(
        first.total_files, 4,
        "total_files must be 4 on force-refresh"
    );
    assert_eq!(first.processed, 0, "first callback shows processed = 0");

    let last = captured.last().unwrap();
    assert_eq!(
        last.processed, 4,
        "final callback shows all 4 files processed"
    );
}
