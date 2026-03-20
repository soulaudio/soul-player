//! TDD tests for DSF/DSD metadata extraction against real files in D:/music.
//!
//! These tests validate the full `extract_metadata` pipeline on real-world DSF
//! files sourced from the developer's music library. They are NOT unit tests —
//! they exercise the complete extraction path including binary parsing, ID3v2
//! decoding, and syncsafe integer handling.
//!
//! Files used (committed to developer machine only — tests skipped if absent):
//!   D:/music/City Pop/Hiroshi Sato/Orient/01 - Kalimba Night.dsf   (DSD64, 2ch, 266s)
//!   D:/music/City Pop/Taeko Ohnuki/Mignonne ミニヨン/01 - Jajauma Musume.dsf
//!
//! Run with:
//!   cargo test --test dsd_metadata_tests -- --nocapture

use soul_importer::metadata::extract_metadata;
use std::path::Path;

/// Skip the test gracefully if the DSF file doesn't exist on this machine.
macro_rules! require_file {
    ($path:expr) => {{
        let p = Path::new($path);
        if !p.exists() {
            eprintln!("SKIP: {} not found", $path);
            return;
        }
        p
    }};
}

// ---------------------------------------------------------------------------
// Group 1: DSD chunk parsing — sample rate, duration, channel count
// ---------------------------------------------------------------------------

#[test]
fn dsf_sample_rate_is_2822400_hz() {
    let path = require_file!("D:/music/City Pop/Hiroshi Sato/Orient/01 - Kalimba Night.dsf");
    let meta = extract_metadata(path).expect("extract_metadata should not error on valid DSF");
    assert_eq!(
        meta.sample_rate,
        Some(2822400),
        "DSD64 DSF must report 2822400 Hz sample rate"
    );
}

#[test]
fn dsf_channel_count_is_stereo() {
    let path = require_file!("D:/music/City Pop/Hiroshi Sato/Orient/01 - Kalimba Night.dsf");
    let meta = extract_metadata(path).expect("extract_metadata should not error on valid DSF");
    assert_eq!(meta.channels, Some(2), "Stereo DSF must report 2 channels");
}

#[test]
fn dsf_duration_is_sane() {
    // Kalimba Night is ~266 seconds
    let path = require_file!("D:/music/City Pop/Hiroshi Sato/Orient/01 - Kalimba Night.dsf");
    let meta = extract_metadata(path).expect("extract_metadata should not error on valid DSF");
    let dur = meta
        .duration_seconds
        .expect("DSF file must have duration_seconds");
    assert!(
        dur > 200.0 && dur < 350.0,
        "Kalimba Night duration should be ~266s, got {dur:.1}s"
    );
}

#[test]
fn dsf_duration_is_not_millions_of_seconds() {
    // Regression test: early DSD parser bug returned sample_count as duration.
    let path = require_file!("D:/music/City Pop/Hiroshi Sato/Orient/01 - Kalimba Night.dsf");
    let meta = extract_metadata(path).expect("extract_metadata should not error on valid DSF");
    let dur = meta
        .duration_seconds
        .expect("DSF file must have duration_seconds");
    assert!(
        dur < 3600.0,
        "Duration must be < 1h; got {dur:.0}s — raw sample count was probably used directly"
    );
}

#[test]
fn dsf_format_field_is_dsf() {
    let path = require_file!("D:/music/City Pop/Hiroshi Sato/Orient/01 - Kalimba Night.dsf");
    let meta = extract_metadata(path).expect("extract_metadata should not error on valid DSF");
    assert_eq!(
        meta.file_format.to_uppercase(),
        "DSF",
        "file_format must be 'DSF' for .dsf files"
    );
}

// ---------------------------------------------------------------------------
// Group 2: ID3v2 tag decoding — title, artist, album, track number
// ---------------------------------------------------------------------------

#[test]
fn dsf_title_is_populated_from_id3v2() {
    let path = require_file!("D:/music/City Pop/Hiroshi Sato/Orient/01 - Kalimba Night.dsf");
    let meta = extract_metadata(path).expect("extract_metadata should not error on valid DSF");
    let title = meta.title.expect("DSF ID3v2 tag must have TIT2 (title)");
    assert!(
        !title.is_empty(),
        "Title must not be empty; got empty string"
    );
    // "Kalimba Night" is the known title in this file
    assert_eq!(
        title.to_lowercase(),
        "kalimba night",
        "Title must match ID3v2 TIT2 tag"
    );
}

#[test]
fn dsf_artist_is_populated_from_id3v2() {
    let path = require_file!("D:/music/City Pop/Hiroshi Sato/Orient/01 - Kalimba Night.dsf");
    let meta = extract_metadata(path).expect("extract_metadata should not error on valid DSF");
    assert!(
        !meta.artists.is_empty(),
        "DSF ID3v2 tag must produce at least one artist"
    );
    let artist_lower = meta.artists[0].to_lowercase();
    assert!(
        artist_lower.contains("hiroshi") || artist_lower.contains("sato"),
        "Artist name should contain 'hiroshi' or 'sato', got: {}",
        meta.artists[0]
    );
}

#[test]
fn dsf_album_is_populated_from_id3v2() {
    let path = require_file!("D:/music/City Pop/Hiroshi Sato/Orient/01 - Kalimba Night.dsf");
    let meta = extract_metadata(path).expect("extract_metadata should not error on valid DSF");
    let album = meta.album.expect("DSF ID3v2 tag must have TALB (album)");
    assert!(
        !album.is_empty(),
        "Album must not be empty for tagged DSF file"
    );
}

#[test]
fn dsf_track_number_parsed_from_slash_format() {
    // Real DSF files use "1/8" format in TRCK frame. We only want the track number (1), not 8.
    let path = require_file!("D:/music/City Pop/Hiroshi Sato/Orient/01 - Kalimba Night.dsf");
    let meta = extract_metadata(path).expect("extract_metadata should not error on valid DSF");
    let track_num = meta
        .track_number
        .expect("TRCK frame must parse to a track number");
    assert!(
        track_num >= 1 && track_num <= 99,
        "Track number must be 1-99, got {track_num}"
    );
    assert_eq!(track_num, 1, "First track must have track_number = 1");
}

#[test]
fn dsf_track_number_is_not_the_total_count() {
    // TRCK "3/8" must yield 3, not 8.
    let path =
        require_file!("D:/music/City Pop/Hiroshi Sato/Orient/03 - Tsuki No Ko No Namae Wa Leo.dsf");
    let meta = extract_metadata(path).expect("extract_metadata should not error on valid DSF");
    if let Some(track_num) = meta.track_number {
        assert_eq!(
            track_num, 3,
            "Track 3 must have track_number=3, not the total count"
        );
    }
}

// ---------------------------------------------------------------------------
// Group 3: Multiple DSF albums — ensure different albums are distinct
// ---------------------------------------------------------------------------

#[test]
fn two_dsf_albums_have_different_album_names() {
    let path_sato = require_file!("D:/music/City Pop/Hiroshi Sato/Orient/01 - Kalimba Night.dsf");
    let path_ohnuki =
        require_file!("D:/music/City Pop/Taeko Ohnuki/Mignonne ミニヨン/01 - Jajauma Musume.dsf");

    let meta_sato =
        extract_metadata(path_sato).expect("extract_metadata should not error on valid DSF");
    let meta_ohnuki =
        extract_metadata(path_ohnuki).expect("extract_metadata should not error on valid DSF");

    let album_sato = meta_sato.album.as_deref().unwrap_or("").to_lowercase();
    let album_ohnuki = meta_ohnuki.album.as_deref().unwrap_or("").to_lowercase();

    assert_ne!(
        album_sato, album_ohnuki,
        "Two different DSF albums must not have the same album name after extraction"
    );
}

#[test]
fn two_dsf_albums_have_different_artists() {
    let path_sato = require_file!("D:/music/City Pop/Hiroshi Sato/Orient/01 - Kalimba Night.dsf");
    let path_ohnuki =
        require_file!("D:/music/City Pop/Taeko Ohnuki/Mignonne ミニヨン/01 - Jajauma Musume.dsf");

    let meta_sato =
        extract_metadata(path_sato).expect("extract_metadata should not error on valid DSF");
    let meta_ohnuki =
        extract_metadata(path_ohnuki).expect("extract_metadata should not error on valid DSF");

    let artist_sato = meta_sato.artists.first().cloned().unwrap_or_default();
    let artist_ohnuki = meta_ohnuki.artists.first().cloned().unwrap_or_default();

    assert_ne!(
        artist_sato.to_lowercase(),
        artist_ohnuki.to_lowercase(),
        "Hiroshi Sato and Taeko Ohnuki must extract as distinct artists"
    );
}

// ---------------------------------------------------------------------------
// Group 4: Resource fork files (._) must not crash the extractor
// ---------------------------------------------------------------------------

#[test]
fn dsf_resource_fork_file_does_not_panic() {
    // macOS resource fork files start with `._` — they are not valid audio.
    // The scanner should have filtered them, but if extract_metadata is called
    // directly it must not panic — it should return an error gracefully.
    let path = Path::new("D:/music/City Pop/Hiroshi Sato/Orient/._01 - Kalimba Night.dsf");
    if !path.exists() {
        eprintln!("SKIP: resource fork file not present on this machine");
        return;
    }
    // Must not panic; returning an error is acceptable
    let result = extract_metadata(path);
    // We don't assert Ok/Err — just that it doesn't panic
    drop(result);
}

// ---------------------------------------------------------------------------
// Group 5: Additional real DSF albums — metadata extraction
// ---------------------------------------------------------------------------

#[test]
fn marvin_gaye_dsf_has_correct_sample_rate() {
    let path = require_file!(
        "D:/music/Funk/Marvin Gaye/What's Going On/01 - MARVIN GAYE - What's Going On.dsf"
    );
    let meta = extract_metadata(path).expect("extract_metadata should not error");
    assert_eq!(
        meta.sample_rate,
        Some(2822400),
        "Marvin Gaye DSF must be DSD64 (2822400 Hz)"
    );
    assert!(
        meta.duration_seconds
            .map(|d| d > 0.0 && d < 3600.0)
            .unwrap_or(false),
        "Duration must be sane"
    );
}

#[test]
fn michael_jackson_off_the_wall_dsf_has_title() {
    let path = require_file!(
        "D:/music/Funk/Michael Jackson/Off The Wall/01 - Michael Jackson - Don't Stop 'Til You Get Enough.dsf"
    );
    let meta = extract_metadata(path).expect("extract_metadata should not error");
    assert!(meta.title.is_some(), "Off The Wall DSF must have title");
    assert!(
        !meta.artists.is_empty(),
        "Off The Wall DSF must have artist"
    );
}

#[test]
fn bill_evans_dsf_sample_rate_and_duration() {
    let path = require_file!("D:/music/Jazz/Bill Evans/Waltz for Debby/01 - MY FOOLISH HEART.dsf");
    let meta = extract_metadata(path).expect("extract_metadata should not error");
    let dur = meta
        .duration_seconds
        .expect("Bill Evans DSF must have duration");
    assert!(
        dur > 0.0 && dur < 3600.0,
        "Duration must be sane, got {dur}s"
    );
    assert_eq!(meta.sample_rate, Some(2822400), "Must be DSD64");
}

#[test]
fn osaki_seiichi_bwf_wav_does_not_error() {
    // BWF (Broadcast Wave Format) WAV files with `bext` chunks cause lofty to fail.
    // Verify that extract_metadata falls back gracefully and returns at minimum a title.
    let path = require_file!(
        "D:/music/Ambient/Osaki Seiichi/In The Footsteps Of A Lost Book Hidden In The Jungle Temple/1 Moving From The Beaming Sun, Shaded By The Tropical Jungle Leaves (Misty Waterfalls).wav"
    );
    let meta = extract_metadata(path)
        .expect("BWF WAV must not return an error — should fall back to path-only metadata");
    // Title must be populated (from filename at minimum)
    assert!(
        meta.title.is_some(),
        "BWF WAV fallback must produce a title from filename"
    );
    assert_eq!(
        meta.file_format.to_uppercase(),
        "WAV",
        "file_format must be WAV"
    );
}

// ---------------------------------------------------------------------------
// Group 6: Scanner-level integration — DSF files are found and parsed
// ---------------------------------------------------------------------------

#[test]
fn dsf_files_are_included_in_scanner_supported_extensions() {
    use soul_importer::scanner::FileScanner;
    use std::path::PathBuf;

    let orient_dir = PathBuf::from("D:/music/City Pop/Hiroshi Sato/Orient");
    if !orient_dir.exists() {
        eprintln!("SKIP: D:/music/City Pop/Hiroshi Sato/Orient not found");
        return;
    }

    let scanner = FileScanner::new();
    let files = scanner
        .scan_directory(&orient_dir)
        .expect("scan_directory must not fail on a real directory");

    let dsf_files: Vec<_> = files
        .iter()
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("dsf"))
        .collect();

    assert!(
        !dsf_files.is_empty(),
        "FileScanner must include .dsf files; found 0 in {}",
        orient_dir.display()
    );
}

#[test]
fn dsf_directory_scan_excludes_resource_forks() {
    use soul_importer::scanner::FileScanner;
    use std::path::PathBuf;

    let orient_dir = PathBuf::from("D:/music/City Pop/Hiroshi Sato/Orient");
    if !orient_dir.exists() {
        eprintln!("SKIP: D:/music/City Pop/Hiroshi Sato/Orient not found");
        return;
    }

    let scanner = FileScanner::new();
    let files = scanner
        .scan_directory(&orient_dir)
        .expect("scan_directory must not fail");

    let resource_forks: Vec<_> = files
        .iter()
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("._"))
                .unwrap_or(false)
        })
        .collect();

    assert!(
        resource_forks.is_empty(),
        "FileScanner must not yield resource fork files (._); found: {:?}",
        resource_forks
    );
}
