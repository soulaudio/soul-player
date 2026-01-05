/// File storage service tests
/// Tests file operations, path validation, and quality variant management
mod common;

use soul_core::TrackId;
use soul_server::{
    config::{AudioFormat, Quality},
    services::FileStorage,
};
use std::path::Path;
use tempfile::TempDir;

/// Test file storage initialization creates directory structure
#[tokio::test]
async fn test_file_storage_initialization() {
    let temp_dir = TempDir::new().unwrap();
    let storage_path = temp_dir.path().to_path_buf();

    let storage = FileStorage::new(storage_path.clone());
    storage.initialize().await.unwrap();

    // Verify base directory exists
    assert!(storage_path.exists(), "Base storage path should be created");

    // Verify quality subdirectories are created
    for quality in &[Quality::Original, Quality::High, Quality::Medium, Quality::Low] {
        let quality_path = storage_path.join(quality.subdirectory());
        assert!(quality_path.exists(), "Quality subdirectory {} should be created", quality.subdirectory());
    }
}

/// Test storing original file
#[tokio::test]
async fn test_store_original_file() {
    let temp_dir = TempDir::new().unwrap();
    let storage_path = temp_dir.path().to_path_buf();

    let storage = FileStorage::new(storage_path.clone());
    storage.initialize().await.unwrap();

    let track_id = TrackId::new("test-track-123".to_string());
    let file_data = b"fake audio data for testing";

    // Store original file
    let stored_path = storage.store_original(&track_id, "mp3", file_data).await.unwrap();

    // Verify file exists at returned path
    assert!(stored_path.exists(), "Stored file should exist");

    // Verify file contents
    let contents = std::fs::read(&stored_path).unwrap();
    assert_eq!(contents, file_data, "File contents should match");

    // Verify file is in original directory
    assert!(stored_path.starts_with(storage_path.join("original")),
        "File should be in original directory");

    // Verify file has correct extension
    assert_eq!(stored_path.extension().unwrap(), "mp3",
        "File should have correct extension");
}

/// Test storing transcoded file
#[tokio::test]
async fn test_store_variant_file() {
    let temp_dir = TempDir::new().unwrap();
    let storage_path = temp_dir.path().to_path_buf();

    let storage = FileStorage::new(storage_path.clone());
    storage.initialize().await.unwrap();

    let track_id = TrackId::new("test-track-456".to_string());
    let file_data = b"transcoded mp3 data";

    // Store transcoded file
    let stored_path = storage.store_variant(
        &track_id,
        Quality::High,
        AudioFormat::Mp3,
        file_data
    ).await.unwrap();

    // Verify file exists
    assert!(stored_path.exists(), "Transcoded file should exist");

    // Verify file contents
    let contents = std::fs::read(&stored_path).unwrap();
    assert_eq!(contents, file_data, "File contents should match");

    // Verify file is in high quality directory
    assert!(stored_path.starts_with(storage_path.join("high")),
        "File should be in high quality directory");

    // Verify file has mp3 extension
    assert_eq!(stored_path.extension().unwrap(), "mp3");
}

/// Test storing multiple quality variants
#[tokio::test]
async fn test_store_multiple_quality_variants() {
    let temp_dir = TempDir::new().unwrap();
    let storage_path = temp_dir.path().to_path_buf();

    let storage = FileStorage::new(storage_path.clone());
    storage.initialize().await.unwrap();

    let track_id = TrackId::new("test-track-multi".to_string());

    // Store original
    let original_data = b"original flac data";
    storage.store_original(&track_id, "flac", original_data).await.unwrap();

    // Store high quality mp3
    let high_data = b"high quality mp3";
    storage.store_variant(&track_id, Quality::High, AudioFormat::Mp3, high_data).await.unwrap();

    // Store medium quality ogg
    let medium_data = b"medium quality ogg";
    storage.store_variant(&track_id, Quality::Medium, AudioFormat::Ogg, medium_data).await.unwrap();

    // Store low quality mp3
    let low_data = b"low quality mp3";
    storage.store_variant(&track_id, Quality::Low, AudioFormat::Mp3, low_data).await.unwrap();

    // Verify all files exist and have correct content
    let original_path = storage.get_track_path(&track_id, Quality::Original, Some(AudioFormat::Flac)).unwrap();
    assert_eq!(std::fs::read(&original_path).unwrap(), original_data);

    let high_path = storage.get_track_path(&track_id, Quality::High, Some(AudioFormat::Mp3)).unwrap();
    assert_eq!(std::fs::read(&high_path).unwrap(), high_data);

    let medium_path = storage.get_track_path(&track_id, Quality::Medium, Some(AudioFormat::Ogg)).unwrap();
    assert_eq!(std::fs::read(&medium_path).unwrap(), medium_data);

    let low_path = storage.get_track_path(&track_id, Quality::Low, Some(AudioFormat::Mp3)).unwrap();
    assert_eq!(std::fs::read(&low_path).unwrap(), low_data);
}

/// Test getting track path for different qualities
#[tokio::test]
async fn test_get_track_path() {
    let temp_dir = TempDir::new().unwrap();
    let storage_path = temp_dir.path().to_path_buf();

    let storage = FileStorage::new(storage_path.clone());
    storage.initialize().await.unwrap();

    let track_id = TrackId::new("test-track-path".to_string());

    // Store files
    storage.store_original(&track_id, "mp3", b"data").await.unwrap();
    storage.store_variant(&track_id, Quality::High, AudioFormat::Mp3, b"data").await.unwrap();

    // Get paths
    let original_path = storage.get_track_path(&track_id, Quality::Original, Some(AudioFormat::Mp3)).unwrap();
    let high_path = storage.get_track_path(&track_id, Quality::High, Some(AudioFormat::Mp3)).unwrap();

    assert!(original_path.to_string_lossy().contains("original"));
    assert!(high_path.to_string_lossy().contains("high"));
}

/// Test best available quality selection when exact quality not available
#[tokio::test]
async fn test_best_available_quality_fallback() {
    let temp_dir = TempDir::new().unwrap();
    let storage_path = temp_dir.path().to_path_buf();

    let storage = FileStorage::new(storage_path.clone());
    storage.initialize().await.unwrap();

    let track_id = TrackId::new("test-track-fallback".to_string());

    // Only store original and low quality
    storage.store_original(&track_id, "flac", b"original").await.unwrap();
    storage.store_variant(&track_id, Quality::Low, AudioFormat::Mp3, b"low").await.unwrap();

    // Request high quality - should fall back to original
    let quality = storage.get_best_available_quality(&track_id, Quality::High);
    assert_eq!(quality, Quality::Original, "Should fall back to original when high not available");

    // Request medium quality - should fall back to original
    let quality = storage.get_best_available_quality(&track_id, Quality::Medium);
    assert_eq!(quality, Quality::Original, "Should fall back to original when medium not available");

    // Request low quality - should return low
    let quality = storage.get_best_available_quality(&track_id, Quality::Low);
    assert_eq!(quality, Quality::Low, "Should return low when available");

    // Request original - should return original
    let quality = storage.get_best_available_quality(&track_id, Quality::Original);
    assert_eq!(quality, Quality::Original, "Should return original when available");
}

/// Test best available quality with only transcoded files
#[tokio::test]
async fn test_best_available_quality_transcoded_only() {
    let temp_dir = TempDir::new().unwrap();
    let storage_path = temp_dir.path().to_path_buf();

    let storage = FileStorage::new(storage_path.clone());
    storage.initialize().await.unwrap();

    let track_id = TrackId::new("test-track-transcoded".to_string());

    // Only store medium quality
    storage.store_variant(&track_id, Quality::Medium, AudioFormat::Mp3, b"medium").await.unwrap();

    // Request high quality - should fall back to medium
    let quality = storage.get_best_available_quality(&track_id, Quality::High);
    assert_eq!(quality, Quality::Medium, "Should fall back to next best available");

    // Request original - should fall back to medium
    let quality = storage.get_best_available_quality(&track_id, Quality::Original);
    assert_eq!(quality, Quality::Medium, "Should fall back to best available");
}

/// Test file existence checking
#[tokio::test]
async fn test_has_quality() {
    let temp_dir = TempDir::new().unwrap();
    let storage_path = temp_dir.path().to_path_buf();

    let storage = FileStorage::new(storage_path.clone());
    storage.initialize().await.unwrap();

    let track_id = TrackId::new("test-exists".to_string());

    // File should not exist initially
    assert!(!storage.has_quality(&track_id, Quality::Original));

    // Store file
    storage.store_original(&track_id, "mp3", b"data").await.unwrap();

    // File should now exist
    assert!(storage.has_quality(&track_id, Quality::Original));

    // Different quality should not exist
    assert!(!storage.has_quality(&track_id, Quality::High));
}

/// Test deleting track and all its files
#[tokio::test]
async fn test_delete_track() {
    let temp_dir = TempDir::new().unwrap();
    let storage_path = temp_dir.path().to_path_buf();

    let storage = FileStorage::new(storage_path.clone());
    storage.initialize().await.unwrap();

    let track_id = TrackId::new("test-delete".to_string());

    // Store multiple files
    storage.store_original(&track_id, "flac", b"original").await.unwrap();
    storage.store_variant(&track_id, Quality::High, AudioFormat::Mp3, b"high").await.unwrap();
    storage.store_variant(&track_id, Quality::Medium, AudioFormat::Ogg, b"medium").await.unwrap();
    storage.store_variant(&track_id, Quality::Low, AudioFormat::Mp3, b"low").await.unwrap();

    // Verify all files exist
    assert!(storage.has_quality(&track_id, Quality::Original));
    assert!(storage.has_quality(&track_id, Quality::High));
    assert!(storage.has_quality(&track_id, Quality::Medium));
    assert!(storage.has_quality(&track_id, Quality::Low));

    // Delete track
    storage.delete_track(&track_id).await.unwrap();

    // Verify all files are deleted
    assert!(!storage.has_quality(&track_id, Quality::Original));
    assert!(!storage.has_quality(&track_id, Quality::High));
    assert!(!storage.has_quality(&track_id, Quality::Medium));
    assert!(!storage.has_quality(&track_id, Quality::Low));
}

/// Test path validation prevents directory traversal
#[tokio::test]
async fn test_path_validation_directory_traversal() {
    let temp_dir = TempDir::new().unwrap();
    let storage_path = temp_dir.path().to_path_buf();

    let storage = FileStorage::new(storage_path.clone());
    storage.initialize().await.unwrap();

    // Attempt directory traversal
    let malicious_path = storage_path.join("../../../etc/passwd");
    let result = storage.validate_path(&malicious_path);

    assert!(result.is_err(), "Directory traversal should be rejected");
}

/// Test path validation allows valid paths within storage
#[tokio::test]
async fn test_path_validation_valid_path() {
    let temp_dir = TempDir::new().unwrap();
    let storage_path = temp_dir.path().to_path_buf();

    let storage = FileStorage::new(storage_path.clone());
    storage.initialize().await.unwrap();

    // Create a file first (validate_path uses canonicalize which requires file to exist)
    let track_id = TrackId::new("test-validation".to_string());
    storage.store_original(&track_id, "mp3", b"data").await.unwrap();

    // Get the actual path and validate it
    let valid_path = storage.get_track_path(&track_id, Quality::Original, Some(AudioFormat::Mp3)).unwrap();
    let result = storage.validate_path(&valid_path);

    assert!(result.is_ok(), "Valid path within storage should be accepted");
}

/// Test path validation rejects absolute paths outside storage
#[tokio::test]
async fn test_path_validation_absolute_path_outside() {
    let temp_dir = TempDir::new().unwrap();
    let storage_path = temp_dir.path().to_path_buf();

    let storage = FileStorage::new(storage_path.clone());
    storage.initialize().await.unwrap();

    // Absolute path outside storage
    let malicious_path = Path::new("/etc/passwd");
    let result = storage.validate_path(malicious_path);

    assert!(result.is_err(), "Absolute path outside storage should be rejected");
}

/// Test storing file with unusual but valid characters in track ID
#[tokio::test]
async fn test_store_file_with_special_track_id() {
    let temp_dir = TempDir::new().unwrap();
    let storage_path = temp_dir.path().to_path_buf();

    let storage = FileStorage::new(storage_path.clone());
    storage.initialize().await.unwrap();

    // Track ID with dashes, underscores, and alphanumeric
    let track_id = TrackId::new("track-123_abc-XYZ".to_string());
    let file_data = b"test data";

    let stored_path = storage.store_original(&track_id, "mp3", file_data).await.unwrap();

    assert!(stored_path.exists());
    assert_eq!(std::fs::read(&stored_path).unwrap(), file_data);
}

/// Test concurrent file storage operations
#[tokio::test]
async fn test_concurrent_file_storage() {
    let temp_dir = TempDir::new().unwrap();
    let storage_path = temp_dir.path().to_path_buf();

    let storage = std::sync::Arc::new(FileStorage::new(storage_path.clone()));
    storage.initialize().await.unwrap();

    // Store 10 files concurrently
    let mut handles = vec![];
    for i in 0..10 {
        let storage_clone = std::sync::Arc::clone(&storage);
        let handle = tokio::spawn(async move {
            let track_id = TrackId::new(format!("track-{}", i));
            let data = format!("data for track {}", i).into_bytes();
            storage_clone.store_original(&track_id, "mp3", &data).await
        });
        handles.push(handle);
    }

    // Wait for all operations to complete
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok(), "Concurrent operation should succeed");
    }

    // Verify all files exist
    for i in 0..10 {
        let track_id = TrackId::new(format!("track-{}", i));
        assert!(storage.has_quality(&track_id, Quality::Original));
    }
}

/// Test storing empty file
#[tokio::test]
async fn test_store_empty_file() {
    let temp_dir = TempDir::new().unwrap();
    let storage_path = temp_dir.path().to_path_buf();

    let storage = FileStorage::new(storage_path.clone());
    storage.initialize().await.unwrap();

    let track_id = TrackId::new("empty-track".to_string());
    let empty_data: &[u8] = &[];

    // Store empty file
    let stored_path = storage.store_original(&track_id, "mp3", empty_data).await.unwrap();

    // Verify file exists and is empty
    assert!(stored_path.exists());
    let contents = std::fs::read(&stored_path).unwrap();
    assert_eq!(contents.len(), 0, "File should be empty");
}

/// Test overwriting existing file (same track ID and quality)
#[tokio::test]
async fn test_overwrite_existing_file() {
    let temp_dir = TempDir::new().unwrap();
    let storage_path = temp_dir.path().to_path_buf();

    let storage = FileStorage::new(storage_path.clone());
    storage.initialize().await.unwrap();

    let track_id = TrackId::new("overwrite-test".to_string());

    // Store original file
    let original_data = b"original data";
    storage.store_original(&track_id, "mp3", original_data).await.unwrap();

    // Overwrite with new data
    let new_data = b"new overwritten data";
    storage.store_original(&track_id, "mp3", new_data).await.unwrap();

    // Verify file contains new data
    let path = storage.get_track_path(&track_id, Quality::Original, Some(AudioFormat::Mp3)).unwrap();
    let contents = std::fs::read(&path).unwrap();
    assert_eq!(contents, new_data, "File should contain new data");
}
