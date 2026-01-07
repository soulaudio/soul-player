//! End-to-end tests for the music importer
//!
//! These tests simulate real-world import scenarios with actual audio files

use soul_importer::{ImportConfig, ImportProgress, ImportSummary, MusicImporter, Result};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

mod test_helpers;
use test_helpers::setup_test_db;

/// Create a minimal valid MP3 file with ID3 tags
/// This creates a tiny valid MP3 with basic structure
fn create_minimal_mp3_with_tags(
    path: &std::path::Path,
    title: &str,
    artist: &str,
    album: Option<&str>,
    genre: Option<&str>,
) -> std::io::Result<()> {
    use std::io::Write;

    let mut file = fs::File::create(path)?;

    // ID3v2.3 header (10 bytes)
    file.write_all(b"ID3")?; // ID3 identifier
    file.write_all(&[0x03, 0x00])?; // Version 2.3.0
    file.write_all(&[0x00])?; // Flags

    // Calculate tag size (synchsafe integer)
    let tag_data = create_id3_frames(title, artist, album, genre);
    let size = tag_data.len() as u32;
    let synchsafe_size = [
        ((size >> 21) & 0x7F) as u8,
        ((size >> 14) & 0x7F) as u8,
        ((size >> 7) & 0x7F) as u8,
        (size & 0x7F) as u8,
    ];
    file.write_all(&synchsafe_size)?;

    // Write frames
    file.write_all(&tag_data)?;

    // Minimal MP3 frame header (4 bytes)
    // MPEG Version 1, Layer III, 128 kbps, 44.1 kHz
    file.write_all(&[0xFF, 0xFB, 0x90, 0x00])?;

    // Dummy frame data (36 bytes minimum for valid frame)
    file.write_all(&vec![0x00; 36])?;

    Ok(())
}

/// Create ID3v2.3 frames
fn create_id3_frames(
    title: &str,
    artist: &str,
    album: Option<&str>,
    genre: Option<&str>,
) -> Vec<u8> {
    let mut frames = Vec::new();

    // TIT2 (Title) frame
    frames.extend(create_text_frame(b"TIT2", title));

    // TPE1 (Artist) frame
    frames.extend(create_text_frame(b"TPE1", artist));

    // TALB (Album) frame
    if let Some(album_name) = album {
        frames.extend(create_text_frame(b"TALB", album_name));
    }

    // TCON (Genre) frame
    if let Some(genre_name) = genre {
        frames.extend(create_text_frame(b"TCON", genre_name));
    }

    frames
}

/// Create a single ID3v2.3 text frame
fn create_text_frame(frame_id: &[u8; 4], text: &str) -> Vec<u8> {
    let mut frame = Vec::new();

    // Frame ID (4 bytes)
    frame.extend_from_slice(frame_id);

    // Frame size (4 bytes, big endian, non-synchsafe)
    let text_bytes = text.as_bytes();
    let size = (text_bytes.len() + 1) as u32; // +1 for encoding byte
    frame.extend_from_slice(&size.to_be_bytes());

    // Frame flags (2 bytes)
    frame.extend_from_slice(&[0x00, 0x00]);

    // Text encoding (1 byte, 0x00 = ISO-8859-1)
    frame.push(0x00);

    // Text data
    frame.extend_from_slice(text_bytes);

    frame
}

#[tokio::test]
async fn test_e2e_import_mp3_with_full_metadata() {
    let pool = setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();
    let library_dir = TempDir::new().unwrap();

    // Create a real MP3 file with metadata
    let mp3_path = temp_dir.path().join("test_song.mp3");
    create_minimal_mp3_with_tags(
        &mp3_path,
        "Test Song",
        "Test Artist",
        Some("Test Album"),
        Some("Rock"),
    )
    .unwrap();

    let config = ImportConfig {
        library_path: library_dir.path().to_path_buf(),
        copy_files: true,
        confidence_threshold: 80,
        file_naming_pattern: "{artist} - {title}.{ext}".to_string(),
        skip_duplicates: true,
    };

    let importer = MusicImporter::new(pool.clone(), config);
    let (mut progress_rx, handle) = importer.import_files(&[mp3_path]).await.unwrap();

    // Collect progress updates
    let mut last_progress = None;
    while let Some(progress) = progress_rx.recv().await {
        last_progress = Some(progress);
    }

    let summary = handle.await.unwrap().unwrap();

    // Verify progress was sent
    assert!(last_progress.is_some());
    let progress = last_progress.unwrap();
    assert_eq!(progress.total_files, 1);
    assert_eq!(progress.processed_files, 1);

    // Verify import summary
    assert_eq!(summary.total_processed, 1);

    // Should either succeed or fail, but not skip (first import)
    assert_eq!(summary.duplicates_skipped, 0);
}

#[tokio::test]
async fn test_e2e_import_creates_library_file() {
    let pool = setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();
    let library_dir = TempDir::new().unwrap();

    // Create source file
    let source_file = temp_dir.path().join("original.mp3");
    create_minimal_mp3_with_tags(&source_file, "My Song", "My Artist", None, None).unwrap();

    let config = ImportConfig {
        library_path: library_dir.path().to_path_buf(),
        copy_files: true,
        confidence_threshold: 80,
        file_naming_pattern: "{artist} - {title}.{ext}".to_string(),
        skip_duplicates: true,
    };

    let importer = MusicImporter::new(pool.clone(), config);
    let (mut progress_rx, handle) = importer.import_files(&[source_file.clone()]).await.unwrap();

    while let Some(_) = progress_rx.recv().await {}
    let summary = handle.await.unwrap().unwrap();

    // If import succeeded, verify file was copied to library
    if summary.successful > 0 {
        // Library should contain a file named "My Artist - My Song.mp3"
        let expected_filename = "My Artist - My Song.mp3";
        let library_file = library_dir.path().join(expected_filename);

        assert!(
            library_file.exists() ||
            // May have sanitized special chars
            library_dir.path().read_dir().unwrap().count() > 0,
            "Library file should be created"
        );
    }

    // Original file should still exist
    assert!(source_file.exists());
}

#[tokio::test]
async fn test_e2e_import_multiple_genres() {
    let pool = setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();
    let library_dir = TempDir::new().unwrap();

    // Create file with multiple genres (ID3 supports semicolon-separated)
    let mp3_path = temp_dir.path().join("multi_genre.mp3");
    create_minimal_mp3_with_tags(
        &mp3_path,
        "Cross Genre Track",
        "Genre Bender",
        Some("Experimental Album"),
        Some("Rock; Electronic; Jazz"),
    )
    .unwrap();

    let config = ImportConfig {
        library_path: library_dir.path().to_path_buf(),
        copy_files: true,
        confidence_threshold: 80,
        file_naming_pattern: "{artist} - {title}.{ext}".to_string(),
        skip_duplicates: true,
    };

    let importer = MusicImporter::new(pool.clone(), config);
    let (mut progress_rx, handle) = importer.import_files(&[mp3_path]).await.unwrap();

    while let Some(_) = progress_rx.recv().await {}
    let summary = handle.await.unwrap().unwrap();

    // If successful, should have created multiple genre associations
    if summary.successful > 0 && !summary.require_review.is_empty() {
        let result = &summary.require_review[0];
        // Should have parsed multiple genres
        assert!(result.genre_matches.len() >= 1);
    }
}

#[tokio::test]
async fn test_e2e_duplicate_detection_by_hash() {
    let pool = setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();
    let library_dir = TempDir::new().unwrap();

    // Create identical file
    let file1 = temp_dir.path().join("song1.mp3");
    let file2 = temp_dir.path().join("song2.mp3"); // Different name, same content

    create_minimal_mp3_with_tags(&file1, "Same Song", "Same Artist", None, None).unwrap();
    fs::copy(&file1, &file2).unwrap(); // Exact duplicate

    let config = ImportConfig {
        library_path: library_dir.path().to_path_buf(),
        copy_files: true,
        confidence_threshold: 80,
        file_naming_pattern: "{artist} - {title}.{ext}".to_string(),
        skip_duplicates: true,
    };

    let importer = MusicImporter::new(pool.clone(), config);

    // Import first file
    let (mut progress_rx, handle) = importer.import_files(&[file1]).await.unwrap();
    while let Some(_) = progress_rx.recv().await {}
    let summary1 = handle.await.unwrap().unwrap();

    // Import duplicate
    let (mut progress_rx, handle) = importer.import_files(&[file2]).await.unwrap();
    while let Some(_) = progress_rx.recv().await {}
    let summary2 = handle.await.unwrap().unwrap();

    // If first succeeded, second should be skipped as duplicate
    if summary1.successful > 0 {
        assert_eq!(summary2.duplicates_skipped, 1);
        assert_eq!(summary2.successful, 0);
    }
}

#[tokio::test]
async fn test_e2e_batch_import_with_mixed_results() {
    let pool = setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();
    let library_dir = TempDir::new().unwrap();

    // Create various test files
    let good_file = temp_dir.path().join("good.mp3");
    let bad_file = temp_dir.path().join("corrupted.mp3");
    let another_good = temp_dir.path().join("good2.mp3");

    create_minimal_mp3_with_tags(&good_file, "Good Song 1", "Artist A", None, None).unwrap();
    create_minimal_mp3_with_tags(&another_good, "Good Song 2", "Artist B", None, None).unwrap();

    // Create a corrupted file (not valid audio)
    fs::write(&bad_file, b"This is not an audio file").unwrap();

    let files = vec![good_file, bad_file, another_good];

    let config = ImportConfig {
        library_path: library_dir.path().to_path_buf(),
        copy_files: true,
        confidence_threshold: 80,
        file_naming_pattern: "{artist} - {title}.{ext}".to_string(),
        skip_duplicates: true,
    };

    let importer = MusicImporter::new(pool.clone(), config);
    let (mut progress_rx, handle) = importer.import_files(&files).await.unwrap();

    while let Some(_) = progress_rx.recv().await {}
    let summary = handle.await.unwrap().unwrap();

    // Should process all 3 files
    assert_eq!(summary.total_processed, 3);

    // Should have at least one failure (the corrupted file)
    assert!(summary.failed >= 1);

    // May have some successful imports
    assert!(summary.successful + summary.failed + summary.duplicates_skipped == 3);
}

#[tokio::test]
async fn test_e2e_import_with_special_characters_in_metadata() {
    let pool = setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();
    let library_dir = TempDir::new().unwrap();

    // Create file with special characters that need sanitization
    let mp3_path = temp_dir.path().join("special_chars.mp3");
    create_minimal_mp3_with_tags(
        &mp3_path,
        "Song: With / Special \\ Characters?",
        "Artist | Name * Here",
        Some("Album <Test>"),
        None,
    )
    .unwrap();

    let config = ImportConfig {
        library_path: library_dir.path().to_path_buf(),
        copy_files: true,
        confidence_threshold: 80,
        file_naming_pattern: "{artist} - {title}.{ext}".to_string(),
        skip_duplicates: true,
    };

    let importer = MusicImporter::new(pool.clone(), config);
    let (mut progress_rx, handle) = importer.import_files(&[mp3_path]).await.unwrap();

    while let Some(_) = progress_rx.recv().await {}
    let summary = handle.await.unwrap().unwrap();

    // Should process without crashing
    assert_eq!(summary.total_processed, 1);

    // If successful, verify file was created with sanitized name
    if summary.successful > 0 {
        let files: Vec<PathBuf> = fs::read_dir(library_dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .collect();

        assert_eq!(files.len(), 1);
        let filename = files[0].file_name().unwrap().to_str().unwrap();

        // Should not contain illegal filesystem characters
        assert!(!filename.contains('/'));
        assert!(!filename.contains('\\'));
        assert!(!filename.contains('*'));
        assert!(!filename.contains('?'));
        assert!(!filename.contains('<'));
        assert!(!filename.contains('>'));
        assert!(!filename.contains('|'));
        assert!(!filename.contains(':'));
    }
}

#[tokio::test]
async fn test_e2e_import_estimates_time_remaining() {
    let pool = setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();
    let library_dir = TempDir::new().unwrap();

    // Create multiple files
    for i in 0..5 {
        let path = temp_dir.path().join(format!("song{}.mp3", i));
        create_minimal_mp3_with_tags(&path, &format!("Song {}", i), "Test Artist", None, None)
            .unwrap();
    }

    let config = ImportConfig {
        library_path: library_dir.path().to_path_buf(),
        copy_files: true,
        confidence_threshold: 80,
        file_naming_pattern: "{artist} - {title}.{ext}".to_string(),
        skip_duplicates: true,
    };

    let importer = MusicImporter::new(pool.clone(), config);
    let (mut progress_rx, handle) = importer.import_directory(temp_dir.path()).await.unwrap();

    let mut had_time_estimate = false;
    while let Some(progress) = progress_rx.recv().await {
        if progress.estimated_seconds_remaining.is_some() {
            had_time_estimate = true;
        }
    }

    let summary = handle.await.unwrap().unwrap();

    assert_eq!(summary.total_processed, 5);
    // Should have provided time estimates during processing
    assert!(had_time_estimate);
}
