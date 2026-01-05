/// Integration tests for metadata reader and library scanner
///
/// These tests verify that metadata extraction and library scanning work correctly
/// with real audio files and database integration.

use soul_core::{MetadataReader, Storage};
use soul_metadata::{LoftyMetadataReader, LibraryScanner, ScanConfig};
use soul_storage::Database;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Helper to create a test WAV file with metadata
fn create_test_wav_with_metadata(
    path: &PathBuf,
    _title: &str,
    _artist: &str,
    _album: &str,
) -> std::io::Result<()> {
    // Create a minimal WAV file (44.1kHz, stereo, 0.1 seconds)
    let sample_rate = 44100u32;
    let channels = 2u16;
    let duration_secs = 0.1f32;
    let num_samples = (sample_rate as f32 * duration_secs) as usize;

    let mut file = File::create(path)?;

    let byte_rate = sample_rate * channels as u32 * 2;
    let block_align = channels * 2;
    let data_size = (num_samples * channels as usize * 2) as u32;
    let chunk_size = 36 + data_size;

    // Write RIFF header
    file.write_all(b"RIFF")?;
    file.write_all(&chunk_size.to_le_bytes())?;
    file.write_all(b"WAVE")?;

    // Write fmt chunk
    file.write_all(b"fmt ")?;
    file.write_all(&16u32.to_le_bytes())?;
    file.write_all(&1u16.to_le_bytes())?; // PCM
    file.write_all(&channels.to_le_bytes())?;
    file.write_all(&sample_rate.to_le_bytes())?;
    file.write_all(&byte_rate.to_le_bytes())?;
    file.write_all(&block_align.to_le_bytes())?;
    file.write_all(&16u16.to_le_bytes())?;

    // Write data chunk
    file.write_all(b"data")?;
    file.write_all(&data_size.to_le_bytes())?;

    // Generate silent audio data
    let zeros = vec![0u8; data_size as usize];
    file.write_all(&zeros)?;

    Ok(())
}

/// Helper to create a test database
async fn create_test_db() -> Database {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let db_url = format!("sqlite://{}", db_path.to_str().unwrap());

    let db = Database::new(&db_url)
        .await
        .expect("Failed to create test database");

    // Keep temp_dir alive
    std::mem::forget(temp_dir);

    db
}

#[tokio::test]
async fn test_metadata_reader_basic() {
    let temp_dir = tempfile::tempdir().unwrap();
    let wav_path = temp_dir.path().join("test.wav");

    create_test_wav_with_metadata(&wav_path, "Test Song", "Test Artist", "Test Album").unwrap();

    let reader = LoftyMetadataReader::new();
    let result = reader.read(&wav_path);

    if let Err(e) = &result {
        eprintln!("Error reading metadata: {:?}", e);
    }

    assert!(result.is_ok(), "Failed to read metadata: {:?}", result.err());
}

#[tokio::test]
async fn test_metadata_reader_duration() {
    let temp_dir = tempfile::tempdir().unwrap();
    let wav_path = temp_dir.path().join("test.wav");

    create_test_wav_with_metadata(&wav_path, "Test Song", "Test Artist", "Test Album").unwrap();

    let reader = LoftyMetadataReader::new();
    let metadata = reader.read(&wav_path).unwrap();

    // Should have duration (approximately 100ms = 100 milliseconds)
    assert!(metadata.duration_ms.is_some());
    let duration = metadata.duration_ms.unwrap();
    assert!(duration > 50 && duration < 150, "Duration should be around 100ms, got: {}", duration);
}

#[tokio::test]
async fn test_metadata_reader_nonexistent_file() {
    let reader = LoftyMetadataReader::new();
    let result = reader.read(&PathBuf::from("/nonexistent/file.mp3"));

    assert!(result.is_err());
}

#[tokio::test]
async fn test_metadata_reader_invalid_file() {
    let temp_dir = tempfile::tempdir().unwrap();
    let invalid_path = temp_dir.path().join("invalid.wav");

    let mut file = File::create(&invalid_path).unwrap();
    file.write_all(b"Not a valid audio file").unwrap();
    drop(file);

    let reader = LoftyMetadataReader::new();
    let result = reader.read(&invalid_path);

    assert!(result.is_err());
}

#[tokio::test]
async fn test_scanner_single_file() {
    let temp_dir = tempfile::tempdir().unwrap();
    let music_dir = temp_dir.path().join("music");
    fs::create_dir(&music_dir).unwrap();

    // Create a test file
    let wav_path = music_dir.join("song1.wav");
    create_test_wav_with_metadata(&wav_path, "Song 1", "Artist 1", "Album 1").unwrap();

    // Create database and scanner
    let db = create_test_db().await;
    let scanner = LibraryScanner::new(Arc::new(db));

    // Scan the directory
    let stats = scanner.scan(&music_dir, None).await.unwrap();

    assert_eq!(stats.files_discovered, 1);
    assert_eq!(stats.files_scanned, 1);
    assert_eq!(stats.tracks_added, 1);
    assert_eq!(stats.errors.len(), 0);
}

#[tokio::test]
async fn test_scanner_multiple_files() {
    let temp_dir = tempfile::tempdir().unwrap();
    let music_dir = temp_dir.path().join("music");
    fs::create_dir(&music_dir).unwrap();

    // Create multiple test files
    for i in 1..=5 {
        let wav_path = music_dir.join(format!("song{}.wav", i));
        create_test_wav_with_metadata(
            &wav_path,
            &format!("Song {}", i),
            &format!("Artist {}", i),
            "Test Album",
        )
        .unwrap();
    }

    let db = create_test_db().await;
    let scanner = LibraryScanner::new(Arc::new(db));

    let stats = scanner.scan(&music_dir, None).await.unwrap();

    assert_eq!(stats.files_discovered, 5);
    assert_eq!(stats.files_scanned, 5);
    assert_eq!(stats.tracks_added, 5);
    assert_eq!(stats.errors.len(), 0);
}

#[tokio::test]
async fn test_scanner_nested_directories() {
    let temp_dir = tempfile::tempdir().unwrap();
    let music_dir = temp_dir.path().join("music");
    fs::create_dir(&music_dir).unwrap();

    // Create nested directory structure
    let artist1_dir = music_dir.join("Artist 1");
    let artist2_dir = music_dir.join("Artist 2");
    fs::create_dir(&artist1_dir).unwrap();
    fs::create_dir(&artist2_dir).unwrap();

    // Add files to nested directories
    create_test_wav_with_metadata(&artist1_dir.join("song1.wav"), "Song 1", "Artist 1", "Album 1")
        .unwrap();
    create_test_wav_with_metadata(&artist1_dir.join("song2.wav"), "Song 2", "Artist 1", "Album 1")
        .unwrap();
    create_test_wav_with_metadata(&artist2_dir.join("song3.wav"), "Song 3", "Artist 2", "Album 2")
        .unwrap();

    let db = create_test_db().await;
    let scanner = LibraryScanner::new(Arc::new(db));

    let stats = scanner.scan(&music_dir, None).await.unwrap();

    assert_eq!(stats.files_discovered, 3);
    assert_eq!(stats.files_scanned, 3);
    assert_eq!(stats.tracks_added, 3);
}

#[tokio::test]
async fn test_scanner_mixed_file_types() {
    let temp_dir = tempfile::tempdir().unwrap();
    let music_dir = temp_dir.path().join("music");
    fs::create_dir(&music_dir).unwrap();

    // Create audio files
    create_test_wav_with_metadata(&music_dir.join("song1.wav"), "Song 1", "Artist 1", "Album 1")
        .unwrap();
    create_test_wav_with_metadata(&music_dir.join("song2.wav"), "Song 2", "Artist 1", "Album 1")
        .unwrap();

    // Create non-audio files (should be ignored)
    File::create(music_dir.join("readme.txt"))
        .unwrap()
        .write_all(b"This is a text file")
        .unwrap();
    File::create(music_dir.join("cover.jpg"))
        .unwrap()
        .write_all(b"Fake image data")
        .unwrap();

    let db = create_test_db().await;
    let scanner = LibraryScanner::new(Arc::new(db));

    let stats = scanner.scan(&music_dir, None).await.unwrap();

    // Should only find WAV files
    assert_eq!(stats.files_discovered, 2);
    assert_eq!(stats.files_scanned, 2);
    assert_eq!(stats.tracks_added, 2);
}

#[tokio::test]
async fn test_scanner_progress_reporting() {
    let temp_dir = tempfile::tempdir().unwrap();
    let music_dir = temp_dir.path().join("music");
    fs::create_dir(&music_dir).unwrap();

    // Create test files
    for i in 1..=3 {
        let wav_path = music_dir.join(format!("song{}.wav", i));
        create_test_wav_with_metadata(&wav_path, &format!("Song {}", i), "Artist", "Album")
            .unwrap();
    }

    let db = create_test_db().await;
    let scanner = LibraryScanner::new(Arc::new(db));

    // Create progress channel
    let (tx, mut rx) = mpsc::channel(100);

    // Spawn scanner task
    let scan_handle = tokio::spawn({
        let music_dir = music_dir.clone();
        async move { scanner.scan(&music_dir, Some(tx)).await }
    });

    // Collect progress updates
    let mut progress_updates = Vec::new();
    while let Some(progress) = rx.recv().await {
        progress_updates.push(progress);
    }

    let stats = scan_handle.await.unwrap().unwrap();

    // Verify we got progress updates
    assert!(!progress_updates.is_empty(), "Should receive progress updates");

    // Check for Started event
    let has_started = progress_updates
        .iter()
        .any(|p| matches!(p, soul_metadata::ScanProgress::Started { .. }));
    assert!(has_started, "Should receive Started event");

    // Check for Completed event
    let has_completed = progress_updates
        .iter()
        .any(|p| matches!(p, soul_metadata::ScanProgress::Completed { .. }));
    assert!(has_completed, "Should receive Completed event");

    // Verify final stats
    assert_eq!(stats.files_discovered, 3);
    assert_eq!(stats.tracks_added, 3);
}

#[tokio::test]
async fn test_scanner_with_errors() {
    let temp_dir = tempfile::tempdir().unwrap();
    let music_dir = temp_dir.path().join("music");
    fs::create_dir(&music_dir).unwrap();

    // Create a valid file
    create_test_wav_with_metadata(&music_dir.join("valid.wav"), "Valid Song", "Artist", "Album")
        .unwrap();

    // Create an invalid file with .wav extension
    let invalid_path = music_dir.join("invalid.wav");
    File::create(&invalid_path)
        .unwrap()
        .write_all(b"Not a valid WAV file")
        .unwrap();

    let db = create_test_db().await;
    let scanner = LibraryScanner::new(Arc::new(db));

    let stats = scanner.scan(&music_dir, None).await.unwrap();

    // Should discover both files
    assert_eq!(stats.files_discovered, 2);
    assert_eq!(stats.files_scanned, 2);

    // Should add the valid one
    assert_eq!(stats.tracks_added, 1);

    // Should have one error
    assert_eq!(stats.errors.len(), 1);
    assert_eq!(stats.errors[0].0, invalid_path);
}

#[tokio::test]
async fn test_scanner_custom_config() {
    let temp_dir = tempfile::tempdir().unwrap();
    let music_dir = temp_dir.path().join("music");
    fs::create_dir(&music_dir).unwrap();

    // Create files with different extensions
    create_test_wav_with_metadata(&music_dir.join("song.wav"), "Song", "Artist", "Album").unwrap();

    // Create custom config that only accepts .mp3 files
    let config = ScanConfig {
        parallel: false,
        num_threads: 1,
        use_file_hashing: false,
        extensions: vec!["mp3".to_string()],
    };

    let db = create_test_db().await;
    let scanner = LibraryScanner::with_config(Arc::new(db), config);

    let stats = scanner.scan(&music_dir, None).await.unwrap();

    // Should not find any files (only looking for mp3)
    assert_eq!(stats.files_discovered, 0);
    assert_eq!(stats.tracks_added, 0);
}

#[tokio::test]
async fn test_scanner_tracks_persisted_in_database() {
    let temp_dir = tempfile::tempdir().unwrap();
    let music_dir = temp_dir.path().join("music");
    fs::create_dir(&music_dir).unwrap();

    // Create test files
    create_test_wav_with_metadata(&music_dir.join("song1.wav"), "Song 1", "Artist 1", "Album 1")
        .unwrap();
    create_test_wav_with_metadata(&music_dir.join("song2.wav"), "Song 2", "Artist 2", "Album 2")
        .unwrap();

    let db: Arc<Database> = Arc::new(create_test_db().await);
    let scanner = LibraryScanner::new(Arc::clone(&db));

    // Scan the directory
    let stats = scanner.scan(&music_dir, None).await.unwrap();
    assert_eq!(stats.tracks_added, 2);

    // Verify tracks are in database
    let all_tracks = db.get_all_tracks().await.unwrap();
    assert_eq!(all_tracks.len(), 2);

    // Verify track details
    // Since our test WAV files don't have embedded metadata, the scanner
    // will use filenames as titles
    let titles: Vec<String> = all_tracks.iter().map(|t| t.title.clone()).collect();
    assert!(titles.contains(&"song1".to_string()) || titles.contains(&"Song 1".to_string()));
    assert!(titles.contains(&"song2".to_string()) || titles.contains(&"Song 2".to_string()));
}

#[tokio::test]
async fn test_scanner_empty_directory() {
    let temp_dir = tempfile::tempdir().unwrap();
    let music_dir = temp_dir.path().join("music");
    fs::create_dir(&music_dir).unwrap();

    let db = create_test_db().await;
    let scanner = LibraryScanner::new(Arc::new(db));

    let stats = scanner.scan(&music_dir, None).await.unwrap();

    assert_eq!(stats.files_discovered, 0);
    assert_eq!(stats.files_scanned, 0);
    assert_eq!(stats.tracks_added, 0);
}

#[tokio::test]
async fn test_scanner_nonexistent_directory() {
    let db = create_test_db().await;
    let scanner = LibraryScanner::new(Arc::new(db));

    let result = scanner
        .scan(&PathBuf::from("/nonexistent/directory"), None)
        .await;

    assert!(result.is_err());
}
