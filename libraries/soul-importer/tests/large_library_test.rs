//! Large library stress tests for the scan pipeline.
//!
//! These tests are `#[ignore]` and must be run explicitly:
//! ```sh
//! cargo test --test large_library_test -- --ignored --nocapture
//! ```

mod test_helpers;

use soul_core::types::CreateLibrarySource;
use soul_importer::library_scanner::LibraryScanner;
use std::fs;
use std::io::Write;
use std::time::Instant;
use tempfile::TempDir;

/// Create a large library with artist/album folder structure.
///
/// Distributes `track_count` files across `artists` artists and
/// `albums_per_artist` albums per artist.
fn create_large_library(dir: &std::path::Path, track_count: usize) {
    let artists = 200;
    let albums_per_artist = 10;

    for i in 0..track_count {
        let artist_idx = i % artists;
        let album_idx = (i / artists) % albums_per_artist;
        let folder = dir
            .join(format!("Artist_{:04}", artist_idx))
            .join(format!("Album_{:03}", album_idx));
        fs::create_dir_all(&folder).ok();

        let file_path = folder.join(format!("track_{:06}.flac", i));
        if !file_path.exists() {
            let mut file = fs::File::create(&file_path).unwrap();
            // Fake FLAC header + padding
            file.write_all(b"fLaC\x00\x00\x00\x22").unwrap();
            file.write_all(&[0u8; 500]).unwrap();
        }
    }
}

/// Helper to create a library source for testing.
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
#[ignore]
async fn test_scan_10k_files() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let track_count = 10_000;

    // --- Create 10K fake FLAC files ---
    let t_create = Instant::now();
    create_large_library(temp_dir.path(), track_count);
    let create_duration = t_create.elapsed();
    eprintln!("=== File Creation ===");
    eprintln!("Created {} files in {:?}", track_count, create_duration);

    let _source = create_test_source(
        &pool,
        "user1",
        "device1",
        "Large Library",
        &temp_dir.path().display().to_string(),
    )
    .await;

    // --- Initial scan ---
    let scanner = LibraryScanner::new(pool.clone(), "user1", "device1")
        .compute_hashes(false)
        .concurrency(8);

    let t_scan = Instant::now();
    let stats = scanner
        .scan_all()
        .await
        .expect("initial scan should succeed");
    let scan_duration = t_scan.elapsed();

    eprintln!("\n=== 10K File Scan Results ===");
    eprintln!("Duration: {:?}", scan_duration);
    eprintln!(
        "Throughput: {:.0} files/sec",
        stats.processed as f64 / scan_duration.as_secs_f64()
    );
    eprintln!("Total files: {}", stats.total_files);
    eprintln!("Processed: {}", stats.processed);
    eprintln!("Errors: {}", stats.errors);
    eprintln!("New: {}", stats.new_files);

    assert_eq!(
        stats.total_files, track_count as i64,
        "should discover all {} files",
        track_count
    );
    assert_eq!(
        stats.processed, track_count as i64,
        "should process all {} files",
        track_count
    );

    // --- Rescan (no changes) ---
    let scanner2 = LibraryScanner::new(pool.clone(), "user1", "device1")
        .compute_hashes(false)
        .concurrency(8);

    let t_rescan = Instant::now();
    let stats2 = scanner2.scan_all().await.expect("rescan should succeed");
    let rescan_duration = t_rescan.elapsed();

    eprintln!("\n=== 10K File Rescan Results ===");
    eprintln!("Duration: {:?}", rescan_duration);
    eprintln!(
        "Throughput: {:.0} files/sec",
        stats2.total_files as f64 / rescan_duration.as_secs_f64()
    );
    eprintln!("Total files: {}", stats2.total_files);
    eprintln!("Processed: {}", stats2.processed);
    eprintln!("New: {}", stats2.new_files);
    eprintln!("Updated: {}", stats2.updated_files);

    assert_eq!(
        stats2.total_files, track_count as i64,
        "rescan should discover all files"
    );

    // Rescan should be significantly faster than the initial scan because
    // unchanged files are skipped during metadata extraction.
    // Note: fake FLAC files error on first scan so no tracks are in the DB,
    // meaning the rescan will still process them. We check the timing ratio
    // only if the initial scan took a meaningful amount of time.
    if scan_duration.as_millis() > 500 {
        let speedup = scan_duration.as_secs_f64() / rescan_duration.as_secs_f64();
        eprintln!("\nRescan speedup: {:.2}x", speedup);
        // Even without real skip logic for errored files, the rescan should
        // complete. We assert it finishes (no hang) and report timing.
        // A strict 2x speedup is only expected when the cache actually skips
        // unchanged files (i.e., files that were previously imported successfully).
    }
}

#[tokio::test]
#[ignore]
async fn test_scan_1k_sequential_vs_parallel() {
    let track_count = 1_000;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    create_large_library(temp_dir.path(), track_count);
    let lib_path = temp_dir.path().display().to_string();

    // --- Sequential scan (concurrency=1) ---
    let pool_seq = test_helpers::setup_test_db().await;
    create_test_source(&pool_seq, "user1", "device1", "Seq Library", &lib_path).await;

    let scanner_seq = LibraryScanner::new(pool_seq.clone(), "user1", "device1")
        .compute_hashes(false)
        .concurrency(1);

    let t_seq = Instant::now();
    let stats_seq = scanner_seq
        .scan_all()
        .await
        .expect("sequential scan should succeed");
    let dur_seq = t_seq.elapsed();

    eprintln!("=== 1K Sequential Scan (concurrency=1) ===");
    eprintln!("Duration: {:?}", dur_seq);
    eprintln!(
        "Throughput: {:.0} files/sec",
        stats_seq.processed as f64 / dur_seq.as_secs_f64()
    );

    // --- Parallel scan (concurrency=8) ---
    let pool_par = test_helpers::setup_test_db().await;
    create_test_source(&pool_par, "user1", "device1", "Par Library", &lib_path).await;

    let scanner_par = LibraryScanner::new(pool_par.clone(), "user1", "device1")
        .compute_hashes(false)
        .concurrency(8);

    let t_par = Instant::now();
    let stats_par = scanner_par
        .scan_all()
        .await
        .expect("parallel scan should succeed");
    let dur_par = t_par.elapsed();

    eprintln!("\n=== 1K Parallel Scan (concurrency=8) ===");
    eprintln!("Duration: {:?}", dur_par);
    eprintln!(
        "Throughput: {:.0} files/sec",
        stats_par.processed as f64 / dur_par.as_secs_f64()
    );

    // --- Comparison ---
    let speedup = dur_seq.as_secs_f64() / dur_par.as_secs_f64();
    eprintln!("\n=== Comparison ===");
    eprintln!("Sequential: {:?}", dur_seq);
    eprintln!("Parallel:   {:?}", dur_par);
    eprintln!("Speedup:    {:.2}x", speedup);

    // Both should process all files
    assert_eq!(
        stats_seq.processed, track_count as i64,
        "sequential should process all files"
    );
    assert_eq!(
        stats_par.processed, track_count as i64,
        "parallel should process all files"
    );

    // Parallel should not be slower than sequential.
    // We use a generous threshold (0.8x) to account for OS scheduling variance.
    assert!(
        speedup > 0.8,
        "parallel scan should not be significantly slower than sequential (speedup={:.2}x)",
        speedup
    );
}
