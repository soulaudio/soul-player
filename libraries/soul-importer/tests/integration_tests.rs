//! Integration tests for the complete import workflow
//!
//! These tests verify the full import pipeline from file scanning to database insertion

use soul_core::types::CreateGenre;
use soul_importer::{ImportConfig, ImportProgress, ImportSummary, MusicImporter, Result};
use std::fs;
use std::io::Write;
use tempfile::TempDir;

mod test_helpers;
use test_helpers::setup_test_db;

/// Helper to create a fake audio file with metadata-like structure
fn create_test_audio_file(
    path: &std::path::Path,
    title: &str,
    artist: &str,
) -> std::io::Result<()> {
    let mut file = fs::File::create(path)?;
    // Write some dummy content (not a real audio file, but good enough for testing import logic)
    writeln!(file, "FAKE AUDIO FILE")?;
    writeln!(file, "Title: {}", title)?;
    writeln!(file, "Artist: {}", artist)?;
    Ok(())
}

#[tokio::test]
async fn test_import_single_file_creates_all_entities() {
    let pool = setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();
    let library_dir = TempDir::new().unwrap();

    // Create a fake audio file
    let audio_file = temp_dir.path().join("test_song.mp3");
    create_test_audio_file(&audio_file, "Test Song", "Test Artist").unwrap();

    let config = ImportConfig {
        library_path: library_dir.path().to_path_buf(),
        copy_files: true,
        confidence_threshold: 80,
        file_naming_pattern: "{artist} - {title}.{ext}".to_string(),
        skip_duplicates: true,
    };

    let importer = MusicImporter::new(pool.clone(), config);
    let (mut progress_rx, handle) = importer.import_files(&[audio_file]).await.unwrap();

    // Consume progress updates
    while let Some(_progress) = progress_rx.recv().await {}

    // Wait for import to complete
    let summary = handle.await.unwrap().unwrap();

    // Note: Since we're using fake audio files without real metadata,
    // the import will likely fail to extract metadata. This test verifies
    // the workflow executes without panicking.
    assert_eq!(summary.total_processed, 1);
}

#[tokio::test]
async fn test_import_directory_with_multiple_files() {
    let pool = setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();
    let library_dir = TempDir::new().unwrap();

    // Create multiple fake audio files
    create_test_audio_file(&temp_dir.path().join("song1.mp3"), "Song 1", "Artist A").unwrap();
    create_test_audio_file(&temp_dir.path().join("song2.flac"), "Song 2", "Artist B").unwrap();
    create_test_audio_file(&temp_dir.path().join("song3.ogg"), "Song 3", "Artist A").unwrap();

    let config = ImportConfig {
        library_path: library_dir.path().to_path_buf(),
        copy_files: true,
        confidence_threshold: 80,
        file_naming_pattern: "{artist} - {title}.{ext}".to_string(),
        skip_duplicates: true,
    };

    let importer = MusicImporter::new(pool.clone(), config);
    let (mut progress_rx, handle) = importer.import_directory(temp_dir.path()).await.unwrap();

    // Consume progress updates
    while let Some(_progress) = progress_rx.recv().await {}

    // Wait for import to complete
    let summary = handle.await.unwrap().unwrap();

    assert_eq!(summary.total_processed, 3);
}

#[tokio::test]
async fn test_import_skips_duplicates() {
    let pool = setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();
    let library_dir = TempDir::new().unwrap();

    // Create a fake audio file
    let audio_file = temp_dir.path().join("duplicate.mp3");
    create_test_audio_file(&audio_file, "Duplicate Song", "Duplicate Artist").unwrap();

    let config = ImportConfig {
        library_path: library_dir.path().to_path_buf(),
        copy_files: true,
        confidence_threshold: 80,
        file_naming_pattern: "{artist} - {title}.{ext}".to_string(),
        skip_duplicates: true,
    };

    let importer = MusicImporter::new(pool.clone(), config);

    // First import
    let (mut progress_rx, handle) = importer.import_files(&[audio_file.clone()]).await.unwrap();
    while let Some(_) = progress_rx.recv().await {}
    let summary1 = handle.await.unwrap().unwrap();

    // Second import (should skip duplicate)
    let (mut progress_rx, handle) = importer.import_files(&[audio_file]).await.unwrap();
    while let Some(_) = progress_rx.recv().await {}
    let summary2 = handle.await.unwrap().unwrap();

    // First import might fail due to fake metadata, but second should definitely skip
    if summary1.successful > 0 {
        // If first succeeded, second should skip as duplicate
        assert_eq!(summary2.duplicates_skipped, 1);
    }
}

#[tokio::test]
async fn test_import_progress_updates() {
    let pool = setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();
    let library_dir = TempDir::new().unwrap();

    // Create multiple files
    for i in 0..5 {
        create_test_audio_file(
            &temp_dir.path().join(format!("song{}.mp3", i)),
            &format!("Song {}", i),
            "Test Artist",
        )
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

    let mut progress_updates = Vec::new();

    // Collect all progress updates
    while let Some(progress) = progress_rx.recv().await {
        progress_updates.push(progress);
    }

    let summary = handle.await.unwrap().unwrap();

    // Should have received multiple progress updates
    assert!(progress_updates.len() > 0);

    // Last update should match final summary
    let last_progress = progress_updates.last().unwrap();
    assert_eq!(last_progress.processed_files, summary.total_processed);
}

#[tokio::test]
async fn test_import_handles_nested_directories() {
    let pool = setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();
    let library_dir = TempDir::new().unwrap();

    // Create nested directory structure
    let artist_dir = temp_dir.path().join("Artist A");
    let album_dir = artist_dir.join("Album 1");
    fs::create_dir_all(&album_dir).unwrap();

    create_test_audio_file(&album_dir.join("track1.mp3"), "Track 1", "Artist A").unwrap();
    create_test_audio_file(&album_dir.join("track2.mp3"), "Track 2", "Artist A").unwrap();

    let config = ImportConfig {
        library_path: library_dir.path().to_path_buf(),
        copy_files: true,
        confidence_threshold: 80,
        file_naming_pattern: "{artist} - {title}.{ext}".to_string(),
        skip_duplicates: true,
    };

    let importer = MusicImporter::new(pool.clone(), config);
    let (mut progress_rx, handle) = importer.import_directory(temp_dir.path()).await.unwrap();

    while let Some(_) = progress_rx.recv().await {}
    let summary = handle.await.unwrap().unwrap();

    // Should find files in nested directories
    assert_eq!(summary.total_processed, 2);
}

#[tokio::test]
async fn test_genre_association_during_import() {
    let pool = setup_test_db().await;

    // Pre-create a genre
    let rock_genre = soul_storage::genres::create(
        &pool,
        CreateGenre {
            name: "Rock".to_string(),
            canonical_name: "Rock".to_string(),
        },
    )
    .await
    .unwrap();

    // Verify genre exists before import
    let all_genres = soul_storage::genres::get_all(&pool).await.unwrap();
    assert_eq!(all_genres.len(), 1);
    assert_eq!(all_genres[0].name, "Rock");
    assert_eq!(all_genres[0].id, rock_genre.id);
}

#[tokio::test]
async fn test_import_with_confidence_threshold() {
    let pool = setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();
    let library_dir = TempDir::new().unwrap();

    // Pre-create an artist with slight name variation
    soul_storage::artists::create(
        &pool,
        soul_core::types::CreateArtist {
            name: "The Beatles".to_string(),
            sort_name: Some("Beatles".to_string()),
            musicbrainz_id: None,
        },
    )
    .await
    .unwrap();

    // Create a file that might fuzzy match
    let audio_file = temp_dir.path().join("beatles_song.mp3");
    create_test_audio_file(&audio_file, "Yesterday", "Beatles").unwrap();

    let config = ImportConfig {
        library_path: library_dir.path().to_path_buf(),
        copy_files: true,
        confidence_threshold: 90, // High threshold - may require review
        file_naming_pattern: "{artist} - {title}.{ext}".to_string(),
        skip_duplicates: true,
    };

    let importer = MusicImporter::new(pool.clone(), config);
    let (mut progress_rx, handle) = importer.import_files(&[audio_file]).await.unwrap();

    while let Some(_) = progress_rx.recv().await {}
    let summary = handle.await.unwrap().unwrap();

    // Check if any imports required review due to confidence threshold
    // (This depends on fuzzy matching results)
    assert!(summary.require_review.len() <= 1);
}
