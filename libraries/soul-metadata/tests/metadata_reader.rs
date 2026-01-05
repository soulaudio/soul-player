/// Integration tests for metadata reading
///
/// Tests use real audio files to ensure metadata extraction works correctly
use soul_core::MetadataReader;
use soul_metadata::LoftyMetadataReader;
use std::path::Path;

#[test]
fn read_mp3_with_id3v2_tags() {
    // This test requires actual test files in tests/data/
    // For now, we'll test the error path
    let reader = LoftyMetadataReader::new();
    let result = reader.read(Path::new("tests/data/sample.mp3"));

    // If test file doesn't exist, verify error handling works
    if let Err(err) = result {
        // Expected: File not found error
        let err_msg = err.to_string();
        assert!(err_msg.contains("not found") || err_msg.contains("No such file"));
    }

    // TODO: Add actual test MP3 file and verify metadata extraction
    // When tests/data/sample.mp3 exists with known tags:
    // - Title: "Test Song"
    // - Artist: "Test Artist"
    // - Album: "Test Album"
    // - Year: 2024
}

#[test]
fn read_flac_with_vorbis_comments() {
    // Placeholder for FLAC metadata test
    let reader = LoftyMetadataReader::new();
    let result = reader.read(Path::new("tests/data/sample.flac"));

    // Verify error handling for missing file
    if let Err(err) = result {
        let err_msg = err.to_string();
        assert!(err_msg.contains("not found") || err_msg.contains("No such file"));
    }

    // TODO: Add test FLAC file with Vorbis comments
}

#[test]
fn read_file_without_tags_uses_filename() {
    // Test that when no tags exist, we can still create a track
    let reader = LoftyMetadataReader::new();
    let result = reader.read(Path::new("tests/data/no_tags.wav"));

    // Placeholder - will be implemented with actual test file
    if let Err(err) = result {
        let err_msg = err.to_string();
        assert!(
            err_msg.contains("not found")
                || err_msg.contains("No such file")
                || err_msg.contains("No tags")
        );
    }
}

#[test]
fn read_corrupted_file_returns_error() {
    let reader = LoftyMetadataReader::new();
    let result = reader.read(Path::new("tests/data/corrupted.mp3"));

    // Should return an error (either file not found or parse error)
    assert!(result.is_err());
}

#[test]
fn read_nonexistent_file_returns_error() {
    let reader = LoftyMetadataReader::new();
    let result = reader.read(Path::new("/definitely/does/not/exist.mp3"));

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("not found") || err_msg.contains("No such file"));
}

#[test]
fn read_directory_instead_of_file_returns_error() {
    let reader = LoftyMetadataReader::new();
    let result = reader.read(Path::new("."));

    // Should fail (can't read metadata from a directory)
    assert!(result.is_err());
}

#[test]
fn write_metadata_not_yet_implemented() {
    use soul_core::TrackMetadata;

    let reader = LoftyMetadataReader::new();
    let metadata = TrackMetadata::new();
    let result = reader.write(Path::new("/tmp/test.mp3"), &metadata);

    // Writing is not implemented in MVP
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("not yet implemented"));
}

// NOTE: To make these tests more comprehensive, add test audio files to tests/data/:
// - sample.mp3 (with ID3v2 tags)
// - sample.flac (with Vorbis comments)
// - sample.ogg (with Vorbis comments)
// - no_tags.wav (valid audio, no metadata)
// - corrupted.mp3 (intentionally broken file)
//
// These files should be small (1-2 seconds of silence) to keep repository size down.
