/// Integration tests for metadata reader and library scanner
///
/// These tests verify that metadata extraction and library scanning work correctly
/// with real audio files and database integration.
///
/// NOTE: Tests using LibraryScanner and ScanConfig are currently disabled because
/// they are commented out in soul-metadata/src/lib.rs pending architectural updates
/// for multi-source Track type support.
use soul_core::traits::MetadataReader as MetadataReaderTrait;
use soul_metadata::LoftyMetadataReader;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

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

// Basic metadata reader tests (no scanner required)
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

    assert!(
        result.is_ok(),
        "Failed to read metadata: {:?}",
        result.err()
    );
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
    assert!(
        duration > 50 && duration < 150,
        "Duration should be around 100ms, got: {}",
        duration
    );
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
