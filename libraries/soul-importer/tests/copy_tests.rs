use soul_importer::copy::{copy_to_library, sanitize_filename_part};
use soul_importer::metadata::ExtractedMetadata;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

#[test]
fn test_sanitize_filename_part() {
    assert_eq!(sanitize_filename_part("Valid Name"), "Valid Name");
    assert_eq!(sanitize_filename_part("Artist/Album"), "Artist_Album");
    assert_eq!(sanitize_filename_part("Song: The Remix"), "Song_ The Remix");
    assert_eq!(sanitize_filename_part("A<B>C"), "A_B_C");
    assert_eq!(sanitize_filename_part("Path\\To\\File"), "Path_To_File");
    assert_eq!(sanitize_filename_part("File|Name"), "File_Name");
    assert_eq!(sanitize_filename_part("Question?"), "Question_");
    assert_eq!(sanitize_filename_part("Star*"), "Star_");
    assert_eq!(sanitize_filename_part("  Trimmed  "), "Trimmed");
}

#[test]
fn test_generate_filename_with_artist_and_title() {
    let metadata = ExtractedMetadata {
        title: Some("Bohemian Rhapsody".to_string()),
        artist: Some("Queen".to_string()),
        album: None,
        album_artist: None,
        track_number: None,
        disc_number: None,
        year: None,
        genres: Vec::new(),
        duration_seconds: None,
        bitrate: None,
        sample_rate: None,
        channels: None,
        file_format: "mp3".to_string(),
    };

    let source = Path::new("/path/to/song.mp3");
    let filename = soul_importer::copy::generate_filename(source, &metadata).unwrap();
    assert_eq!(filename, "Queen - Bohemian Rhapsody.mp3");
}

#[test]
fn test_generate_filename_prefers_album_artist() {
    let metadata = ExtractedMetadata {
        title: Some("Song".to_string()),
        artist: Some("Track Artist".to_string()),
        album: None,
        album_artist: Some("Album Artist".to_string()),
        track_number: None,
        disc_number: None,
        year: None,
        genres: Vec::new(),
        duration_seconds: None,
        bitrate: None,
        sample_rate: None,
        channels: None,
        file_format: "flac".to_string(),
    };

    let source = Path::new("/path/to/song.flac");
    let filename = soul_importer::copy::generate_filename(source, &metadata).unwrap();
    assert_eq!(filename, "Album Artist - Song.flac");
}

#[test]
fn test_generate_filename_no_artist() {
    let metadata = ExtractedMetadata {
        title: Some("Unknown Track".to_string()),
        artist: None,
        album: None,
        album_artist: None,
        track_number: None,
        disc_number: None,
        year: None,
        genres: Vec::new(),
        duration_seconds: None,
        bitrate: None,
        sample_rate: None,
        channels: None,
        file_format: "mp3".to_string(),
    };

    let source = Path::new("/path/to/song.mp3");
    let filename = soul_importer::copy::generate_filename(source, &metadata).unwrap();
    assert_eq!(filename, "Unknown Track.mp3");
}

#[test]
fn test_generate_filename_fallback_to_original() {
    let metadata = ExtractedMetadata {
        title: None,
        artist: None,
        album: None,
        album_artist: None,
        track_number: None,
        disc_number: None,
        year: None,
        genres: Vec::new(),
        duration_seconds: None,
        bitrate: None,
        sample_rate: None,
        channels: None,
        file_format: "mp3".to_string(),
    };

    let source = Path::new("/path/to/original_song.mp3");
    let filename = soul_importer::copy::generate_filename(source, &metadata).unwrap();
    assert_eq!(filename, "original_song.mp3");
}

#[test]
fn test_generate_filename_sanitizes_special_chars() {
    let metadata = ExtractedMetadata {
        title: Some("Song: Part 1".to_string()),
        artist: Some("Artist/Band".to_string()),
        album: None,
        album_artist: None,
        track_number: None,
        disc_number: None,
        year: None,
        genres: Vec::new(),
        duration_seconds: None,
        bitrate: None,
        sample_rate: None,
        channels: None,
        file_format: "mp3".to_string(),
    };

    let source = Path::new("/path/to/song.mp3");
    let filename = soul_importer::copy::generate_filename(source, &metadata).unwrap();
    assert_eq!(filename, "Artist_Band - Song_ Part 1.mp3");
}

#[test]
fn test_copy_to_library() {
    let temp = TempDir::new().unwrap();
    let source_dir = temp.path().join("source");
    let library_dir = temp.path().join("library");

    fs::create_dir(&source_dir).unwrap();

    // Create test file
    let source_file = source_dir.join("test.mp3");
    fs::write(&source_file, b"fake mp3 data").unwrap();

    let metadata = ExtractedMetadata {
        title: Some("Test Song".to_string()),
        artist: Some("Test Artist".to_string()),
        album: None,
        album_artist: None,
        track_number: None,
        disc_number: None,
        year: None,
        genres: Vec::new(),
        duration_seconds: None,
        bitrate: None,
        sample_rate: None,
        channels: None,
        file_format: "mp3".to_string(),
    };

    let dest_path = copy_to_library(&source_file, &library_dir, &metadata).unwrap();

    assert!(dest_path.exists());
    assert!(dest_path.ends_with("Test Artist - Test Song.mp3"));
    assert_eq!(
        fs::read(&dest_path).unwrap(),
        b"fake mp3 data",
        "File content should match"
    );

    // Verify original still exists
    assert!(source_file.exists());
}

#[test]
fn test_copy_to_library_creates_directory() {
    let temp = TempDir::new().unwrap();
    let source_file = temp.path().join("test.mp3");
    fs::write(&source_file, b"data").unwrap();

    let library_dir = temp.path().join("nonexistent/library");

    let metadata = ExtractedMetadata {
        title: Some("Song".to_string()),
        artist: Some("Artist".to_string()),
        album: None,
        album_artist: None,
        track_number: None,
        disc_number: None,
        year: None,
        genres: Vec::new(),
        duration_seconds: None,
        bitrate: None,
        sample_rate: None,
        channels: None,
        file_format: "mp3".to_string(),
    };

    let dest_path = copy_to_library(&source_file, &library_dir, &metadata).unwrap();

    assert!(library_dir.exists());
    assert!(dest_path.exists());
}

#[test]
fn test_copy_to_library_handles_conflicts() {
    let temp = TempDir::new().unwrap();
    let library_dir = temp.path();

    // Create existing file
    fs::write(library_dir.join("Artist - Song.mp3"), b"existing").unwrap();

    let source_file = temp.path().join("source.mp3");
    fs::write(&source_file, b"new data").unwrap();

    let metadata = ExtractedMetadata {
        title: Some("Song".to_string()),
        artist: Some("Artist".to_string()),
        album: None,
        album_artist: None,
        track_number: None,
        disc_number: None,
        year: None,
        genres: Vec::new(),
        duration_seconds: None,
        bitrate: None,
        sample_rate: None,
        channels: None,
        file_format: "mp3".to_string(),
    };

    let dest_path = copy_to_library(&source_file, library_dir, &metadata).unwrap();

    // Should create with -1 suffix
    assert!(dest_path.ends_with("Artist - Song-1.mp3"));
    assert_eq!(fs::read(&dest_path).unwrap(), b"new data");

    // Original file should still exist
    assert_eq!(
        fs::read(library_dir.join("Artist - Song.mp3")).unwrap(),
        b"existing"
    );
}

#[test]
fn test_copy_to_library_multiple_conflicts() {
    let temp = TempDir::new().unwrap();
    let library_dir = temp.path();

    // Create existing files
    fs::write(library_dir.join("Song.mp3"), b"1").unwrap();
    fs::write(library_dir.join("Song-1.mp3"), b"2").unwrap();
    fs::write(library_dir.join("Song-2.mp3"), b"3").unwrap();

    let source_file = temp.path().join("source.mp3");
    fs::write(&source_file, b"4").unwrap();

    let metadata = ExtractedMetadata {
        title: Some("Song".to_string()),
        artist: None,
        album: None,
        album_artist: None,
        track_number: None,
        disc_number: None,
        year: None,
        genres: Vec::new(),
        duration_seconds: None,
        bitrate: None,
        sample_rate: None,
        channels: None,
        file_format: "mp3".to_string(),
    };

    let dest_path = copy_to_library(&source_file, library_dir, &metadata).unwrap();

    // Should create with -3 suffix
    assert!(dest_path.ends_with("Song-3.mp3"));
    assert_eq!(fs::read(&dest_path).unwrap(), b"4");
}
