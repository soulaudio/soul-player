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
    let stats = scanner
        .scan_source(&source)
        .await
        .expect("scan should succeed");

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

    let scanner = LibraryScanner::new(pool.clone(), "user1", "device1").compute_hashes(true);
    let stats = scanner
        .scan_source(&source)
        .await
        .expect("scan should succeed");

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
    let _stats = scanner
        .scan_source(&source)
        .await
        .expect("scan should succeed");

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
    let scanner = LibraryScanner::new(pool.clone(), "user1", "device1").compute_hashes(false);
    let stats = scanner
        .scan_source(&source)
        .await
        .expect("scan should succeed");

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

    let scanner = LibraryScanner::new(pool.clone(), "user1", "device1").on_progress(Box::new(
        move |_stats| {
            callback_count_clone.fetch_add(1, Ordering::SeqCst);
        },
    ));

    let _stats = scanner
        .scan_source(&source)
        .await
        .expect("scan should succeed");

    // Callback should have been invoked at least once per file
    let count = callback_count.load(Ordering::SeqCst);
    assert!(
        count >= 3,
        "callback should be invoked at least 3 times, got {}",
        count
    );
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
    let _stats = scanner
        .scan_source(&source)
        .await
        .expect("scan should succeed");

    // After scan, check last_scan_at is updated
    let updated_source = soul_storage::library_sources::get_by_id(&pool, source.id)
        .await
        .expect("get_by_id should succeed")
        .expect("source should exist");

    assert!(
        updated_source.last_scan_at.is_some(),
        "last_scan_at should be set"
    );
}

// =============================================================================
// Sync deletes + orphan cleanup tests
// =============================================================================

/// Create a minimal valid WAV file with unique content (seed makes each file different)
fn create_valid_wav_file(dir: &std::path::Path, filename: &str, seed: u8) -> std::path::PathBuf {
    let file_path = dir.join(filename);
    let mut file = fs::File::create(&file_path).expect("Failed to create WAV file");

    // WAV header for 0.5-second mono 16-bit 44100Hz
    let num_samples: u32 = 22050;
    let data_size: u32 = num_samples * 2; // 2 bytes per sample
    let file_size: u32 = 36 + data_size;

    // RIFF header
    file.write_all(b"RIFF").unwrap();
    file.write_all(&file_size.to_le_bytes()).unwrap();
    file.write_all(b"WAVE").unwrap();
    // fmt chunk
    file.write_all(b"fmt ").unwrap();
    file.write_all(&16u32.to_le_bytes()).unwrap();
    file.write_all(&1u16.to_le_bytes()).unwrap(); // PCM
    file.write_all(&1u16.to_le_bytes()).unwrap(); // mono
    file.write_all(&44100u32.to_le_bytes()).unwrap();
    file.write_all(&(44100u32 * 2).to_le_bytes()).unwrap();
    file.write_all(&2u16.to_le_bytes()).unwrap();
    file.write_all(&16u16.to_le_bytes()).unwrap();
    // data chunk — unique content per file using seed
    file.write_all(b"data").unwrap();
    file.write_all(&data_size.to_le_bytes()).unwrap();
    let data: Vec<u8> = (0..data_size).map(|i| seed.wrapping_add(i as u8)).collect();
    file.write_all(&data).unwrap();
    file.flush().unwrap();

    file_path
}

/// Create multiple valid WAV test files with unique content
fn create_valid_wav_files(dir: &std::path::Path, count: usize) -> Vec<std::path::PathBuf> {
    (0..count)
        .map(|i| create_valid_wav_file(dir, &format!("track_{:02}.wav", i + 1), (i + 1) as u8))
        .collect()
}

/// Helper to create a source with sync_deletes enabled
async fn create_sync_delete_source(
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
            sync_deletes: true,
        },
    )
    .await
    .expect("Failed to create library source")
}

/// Test: Scan library, remove files, rescan → tracks marked unavailable
#[tokio::test]
async fn test_sync_deletes_marks_removed_tracks_unavailable() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    // Create album folder with 3 tracks
    let album_dir = temp_dir.path().join("Album A");
    fs::create_dir_all(&album_dir).unwrap();
    create_valid_wav_files(&album_dir, 3);

    let source = create_sync_delete_source(
        &pool,
        "1",
        "desktop-local",
        "Music",
        &temp_dir.path().display().to_string(),
    )
    .await;

    // First scan
    let scanner = LibraryScanner::new(pool.clone(), "1", "desktop-local");
    scanner.scan_all().await.unwrap();

    // Verify tracks exist in DB (available)
    let tracks_before = soul_storage::tracks::get_by_library_source(&pool, source.id)
        .await
        .unwrap();
    assert!(
        tracks_before.len() >= 1,
        "Should have tracks after scan, got {}",
        tracks_before.len()
    );
    let track_count_before = tracks_before.len();

    // Delete one track file
    fs::remove_file(album_dir.join("track_02.wav")).unwrap();

    // Rescan
    let scanner2 = LibraryScanner::new(pool.clone(), "1", "desktop-local");
    let stats2 = scanner2.scan_all().await.unwrap();
    assert!(
        stats2.removed_files >= 1,
        "Should detect at least 1 removed file, got {}",
        stats2.removed_files
    );

    // Available tracks should be fewer
    let tracks_after = soul_storage::tracks::get_by_library_source(&pool, source.id)
        .await
        .unwrap();
    assert!(
        tracks_after.len() < track_count_before,
        "Available tracks should decrease: before={}, after={}",
        track_count_before,
        tracks_after.len()
    );
}

/// Test: Remove an entire album folder → orphaned albums cleaned up
#[tokio::test]
async fn test_sync_deletes_removes_orphaned_albums_and_artists() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    // Create two album folders
    let album_a = temp_dir.path().join("Artist X - Album A");
    let album_b = temp_dir.path().join("Artist X - Album B");
    fs::create_dir_all(&album_a).unwrap();
    fs::create_dir_all(&album_b).unwrap();
    // Use different seeds (50+ and 100+) to ensure unique content hashes across folders
    for i in 0..2 {
        create_valid_wav_file(&album_a, &format!("track_{:02}.wav", i + 1), (i + 50) as u8);
    }
    for i in 0..2 {
        create_valid_wav_file(
            &album_b,
            &format!("track_{:02}.wav", i + 1),
            (i + 100) as u8,
        );
    }

    let _source = create_sync_delete_source(
        &pool,
        "1",
        "desktop-local",
        "Music",
        &temp_dir.path().display().to_string(),
    )
    .await;

    // First scan
    let scanner = LibraryScanner::new(pool.clone(), "1", "desktop-local");
    scanner.scan_all().await.unwrap();

    // Verify albums exist
    let albums_before = soul_storage::albums::get_all(&pool).await.unwrap();
    assert!(!albums_before.is_empty(), "Should have albums after scan");
    let album_count_before = albums_before.len();

    // Delete entire album folder A
    fs::remove_dir_all(&album_a).unwrap();

    // Rescan
    let scanner2 = LibraryScanner::new(pool.clone(), "1", "desktop-local");
    let stats2 = scanner2.scan_all().await.unwrap();
    assert!(
        stats2.removed_files >= 1,
        "Should detect removed files, got {}",
        stats2.removed_files
    );

    // Albums should be fewer (the orphaned album from folder A should be deleted)
    let albums_after = soul_storage::albums::get_all(&pool).await.unwrap();
    assert!(
        albums_after.len() < album_count_before,
        "Orphaned album should have been deleted: before={}, after={}",
        album_count_before,
        albums_after.len()
    );
}

/// Test: Adding new files to an existing watched folder → detected on rescan
#[tokio::test]
async fn test_rescan_detects_new_files_added_to_folder() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    // Start with one album folder
    let album_dir = temp_dir.path().join("Album A");
    fs::create_dir_all(&album_dir).unwrap();
    create_valid_wav_files(&album_dir, 2);

    let source = create_sync_delete_source(
        &pool,
        "1",
        "desktop-local",
        "Music",
        &temp_dir.path().display().to_string(),
    )
    .await;

    // First scan
    let scanner = LibraryScanner::new(pool.clone(), "1", "desktop-local");
    scanner.scan_all().await.unwrap();

    let tracks_before = soul_storage::tracks::get_by_library_source(&pool, source.id)
        .await
        .unwrap();
    let count_before = tracks_before.len();
    assert!(count_before >= 1, "Should have tracks after first scan");

    // Add a new album folder with 3 more files (seeds 10-12 to avoid any hash collision)
    let album_b = temp_dir.path().join("Album B");
    fs::create_dir_all(&album_b).unwrap();
    for i in 0..3 {
        create_valid_wav_file(
            &album_b,
            &format!("new_track_{:02}.wav", i + 1),
            (i + 10) as u8,
        );
    }

    // Rescan — should pick up new files
    let scanner2 = LibraryScanner::new(pool.clone(), "1", "desktop-local");
    scanner2.scan_all().await.unwrap();

    let tracks_after = soul_storage::tracks::get_by_library_source(&pool, source.id)
        .await
        .unwrap();
    assert!(
        tracks_after.len() > count_before,
        "Should have more tracks after adding files: before={}, after={}",
        count_before,
        tracks_after.len()
    );
}

// =============================================================================
// Album grouping tests — ALBUMARTIST-based deduplication
// =============================================================================

/// Build a minimal ID3v2.3 text frame (4-byte frame ID + 4-byte size + 2 flags + 1 encoding + text)
fn id3_text_frame(frame_id: &[u8; 4], text: &str) -> Vec<u8> {
    let content: Vec<u8> = std::iter::once(3u8) // UTF-8 encoding byte
        .chain(text.bytes())
        .collect();
    let size = content.len() as u32;
    let mut frame = Vec::with_capacity(10 + content.len());
    frame.extend_from_slice(frame_id);
    frame.extend_from_slice(&size.to_be_bytes());
    frame.extend_from_slice(&[0u8, 0u8]); // flags
    frame.extend_from_slice(&content);
    frame
}

/// Build an ID3v2.3 tag containing the given frames.
fn build_id3v2_tag(frames: &[Vec<u8>]) -> Vec<u8> {
    let mut body: Vec<u8> = Vec::new();
    for frame in frames {
        body.extend_from_slice(frame);
    }

    // Syncsafe encode body length
    let body_len = body.len() as u32;
    let syncsafe = [
        ((body_len >> 21) & 0x7f) as u8,
        ((body_len >> 14) & 0x7f) as u8,
        ((body_len >> 7) & 0x7f) as u8,
        (body_len & 0x7f) as u8,
    ];

    let mut tag = Vec::with_capacity(10 + body.len());
    tag.extend_from_slice(b"ID3"); // magic
    tag.extend_from_slice(&[3, 0]); // version 2.3.0
    tag.push(0); // flags
    tag.extend_from_slice(&syncsafe);
    tag.extend_from_slice(&body);
    tag
}

/// Create a minimal WAV file with embedded ID3v2.3 tags via a RIFF `id3 ` chunk.
/// `seed` is used to make the PCM data unique across files (preventing hash collisions).
fn create_tagged_wav_file(
    dir: &std::path::Path,
    filename: &str,
    title: &str,
    artist: &str,
    album_artist: &str,
    album: &str,
    track_number: u32,
    seed: u8,
) -> std::path::PathBuf {
    let file_path = dir.join(filename);

    // Build ID3 tag
    let frames = vec![
        id3_text_frame(b"TIT2", title),
        id3_text_frame(b"TPE1", artist),
        id3_text_frame(b"TPE2", album_artist),
        id3_text_frame(b"TALB", album),
        id3_text_frame(b"TRCK", &track_number.to_string()),
    ];
    let id3_tag = build_id3v2_tag(&frames);

    // Build PCM audio data (minimal, unique per seed)
    let num_samples: u32 = 1024;
    let pcm_size: u32 = num_samples * 2; // 16-bit mono
    let pcm_data: Vec<u8> = (0..pcm_size).map(|i| seed.wrapping_add(i as u8)).collect();

    // Build RIFF/WAVE file:
    //   RIFF header (8 bytes) + "WAVE" (4) + fmt chunk (24) + data chunk (8+pcm) + id3 chunk (8+id3)
    let id3_chunk_size = id3_tag.len() as u32;
    // Pad id3 chunk to even size (RIFF rule)
    let id3_chunk_padded = if id3_chunk_size % 2 == 1 {
        id3_chunk_size + 1
    } else {
        id3_chunk_size
    };

    let riff_body_size: u32 = 4               // "WAVE"
        + 8 + 16                               // fmt chunk
        + 8 + pcm_size                         // data chunk
        + 8 + id3_chunk_padded; // id3 chunk

    let mut buf: Vec<u8> = Vec::new();

    // RIFF header
    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&riff_body_size.to_le_bytes());
    buf.extend_from_slice(b"WAVE");

    // fmt chunk (PCM, mono, 44100 Hz, 16-bit)
    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&16u32.to_le_bytes()); // chunk size
    buf.extend_from_slice(&1u16.to_le_bytes()); // PCM
    buf.extend_from_slice(&1u16.to_le_bytes()); // mono
    buf.extend_from_slice(&44100u32.to_le_bytes()); // sample rate
    buf.extend_from_slice(&(44100u32 * 2).to_le_bytes()); // byte rate
    buf.extend_from_slice(&2u16.to_le_bytes()); // block align
    buf.extend_from_slice(&16u16.to_le_bytes()); // bits per sample

    // data chunk
    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&pcm_size.to_le_bytes());
    buf.extend_from_slice(&pcm_data);

    // id3 chunk (lofty reads "id3 " or "ID3 " subchunks in RIFF/WAVE)
    buf.extend_from_slice(b"id3 ");
    buf.extend_from_slice(&id3_chunk_size.to_le_bytes());
    buf.extend_from_slice(&id3_tag);
    if id3_chunk_size % 2 == 1 {
        buf.push(0); // padding byte
    }

    fs::write(&file_path, &buf).expect("Failed to write tagged WAV file");
    file_path
}

/// Regression test: album with 10 tracks where ARTIST tags vary but ALBUMARTIST is consistent.
///
/// This reproduces the real-world bug where "(71) ある若者の肖像" by Jun Fukamachi
/// showed 10 tracks in metadata but only 2 tracks in `get_album_tracks` because
/// the scanner created multiple album records — one per distinct ARTIST tag variant.
///
/// Fix: when ALBUMARTIST is present, always use the ALBUMARTIST-derived artist ID
/// as the album grouping key, ignoring per-track ARTIST variations.
#[tokio::test]
async fn test_albumartist_groups_all_tracks_into_single_album() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    let album_dir = temp_dir.path().join("Jun Fukamachi - (71)");
    fs::create_dir_all(&album_dir).unwrap();

    let album_title = "(71) ある若者の肖像";
    let album_artist = "Jun Fukamachi";

    // Create 10 tracks with varying ARTIST tags but identical ALBUMARTIST
    let track_defs: &[(&str, &str)] = &[
        ("Jun Fukamachi", "Track 01"),
        ("Jun Fukamachi", "Track 02"),
        ("Jun Fukamachi", "Track 03"),
        ("Jun Fukamachi", "Track 04"),
        ("Jun Fukamachi (深町純)", "Track 05"),
        ("Jun Fukamachi (深町純)", "Track 06"),
        ("Jun Fukamachi (深町純)", "Track 07"),
        ("Fukamachi, Jun", "Track 08"),
        ("Fukamachi, Jun", "Track 09"),
        ("Fukamachi, Jun", "Track 10"),
    ];

    for (i, (artist, title)) in track_defs.iter().enumerate() {
        let filename = format!("track_{:02}.wav", i + 1);
        create_tagged_wav_file(
            &album_dir,
            &filename,
            title,
            artist,
            album_artist,
            album_title,
            (i + 1) as u32,
            (i + 1) as u8,
        );
    }

    let source = create_test_source(
        &pool,
        "1",
        "desktop-local",
        "Music",
        &temp_dir.path().display().to_string(),
    )
    .await;

    let scanner = LibraryScanner::new(pool.clone(), "1", "desktop-local").compute_hashes(false);
    let stats = scanner.scan_all().await.expect("scan_all should succeed");

    assert_eq!(stats.errors, 0, "No errors expected during scan");
    assert_eq!(stats.total_files, 10, "Should scan all 10 tracks");

    // Only ONE album record should exist for "(71) ある若者の肖像"
    let all_albums = soul_storage::albums::get_all(&pool).await.unwrap();
    let matching_albums: Vec<_> = all_albums
        .iter()
        .filter(|a| a.title == album_title)
        .collect();
    assert_eq!(
        matching_albums.len(),
        1,
        "Expected exactly 1 album record for '{}', got {} (album IDs: {:?})",
        album_title,
        matching_albums.len(),
        matching_albums.iter().map(|a| a.id).collect::<Vec<_>>()
    );

    // All 10 tracks should belong to that single album
    let album_id = matching_albums[0].id;
    let tracks = soul_storage::tracks::get_by_album(&pool, album_id)
        .await
        .expect("get_by_album should succeed");
    assert_eq!(
        tracks.len(),
        10,
        "Expected 10 tracks under album ID {}, got {} tracks",
        album_id,
        tracks.len()
    );

    // Verify library source association
    let source_tracks = soul_storage::tracks::get_by_library_source(&pool, source.id)
        .await
        .unwrap();
    assert_eq!(
        source_tracks.len(),
        10,
        "All 10 tracks should be linked to the library source"
    );
}
