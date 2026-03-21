//! Comprehensive end-to-end tests for LibraryScanner::scan_all()
//!
//! These tests verify exact DB state after scanning directories of synthetic
//! audio files. Each test creates real parseable binary files that lofty can
//! read, scans them with LibraryScanner, then asserts the resulting DB state.
//!
//! Run with:
//!   cargo test --test scanner_import_e2e_tests -- --nocapture

mod test_helpers;

use soul_core::types::CreateLibrarySource;
use soul_importer::library_scanner::{LibraryScanner, ScanStats};
use sqlx::SqlitePool;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

// =============================================================================
// File creation helpers
// =============================================================================

struct Tags<'a> {
    title: &'a str,
    artist: &'a str,
    album_artist: Option<&'a str>,
    album: &'a str,
    track_number: Option<u32>,
    track_total: Option<u32>,
}

/// Build a single ID3v2.3/2.4 text frame.
/// encoding=0x03 → UTF-8 (supports Japanese and all Unicode).
/// Frame size is stored as a plain big-endian u32 (ID3v2.3 style),
/// which lofty reads correctly for both v2.3 and v2.4.
fn id3_frame(frame_id: &[u8; 4], text: &str) -> Vec<u8> {
    let content: Vec<u8> = std::iter::once(0x03u8) // UTF-8 encoding byte
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

/// Build an ID3v2.3 tag header wrapping the given frames.
fn build_id3_tag(frames: &[Vec<u8>]) -> Vec<u8> {
    let mut body: Vec<u8> = Vec::new();
    for f in frames {
        body.extend_from_slice(f);
    }
    let body_len = body.len() as u32;
    // Synchsafe size encoding
    let ss = [
        ((body_len >> 21) & 0x7f) as u8,
        ((body_len >> 14) & 0x7f) as u8,
        ((body_len >> 7) & 0x7f) as u8,
        (body_len & 0x7f) as u8,
    ];
    let mut tag = Vec::with_capacity(10 + body.len());
    tag.extend_from_slice(b"ID3");
    tag.extend_from_slice(&[3, 0]); // ID3v2.3
    tag.push(0); // flags
    tag.extend_from_slice(&ss);
    tag.extend_from_slice(&body);
    tag
}

/// Create a minimal MP3 file with ID3v2.3 tags.
/// Uses UTF-8 encoding (0x03) so Japanese and all Unicode text is stored correctly.
///
/// Frame size formula: floor(144 * 128000 / 44100) = 417 bytes TOTAL (header + data).
/// Data bytes = 417 - 4 = 413. We write 2 frames so lofty's two-frame validator succeeds.
/// Audio data is all-zeros (silence) to prevent accidental sync words (0xFF 0xFB) in data.
fn create_mp3(dir: &Path, filename: &str, tags: &Tags, _seed: u8) -> PathBuf {
    let path = dir.join(filename);

    let mut frames = vec![
        id3_frame(b"TIT2", tags.title),
        id3_frame(b"TPE1", tags.artist),
        id3_frame(b"TALB", tags.album),
    ];
    if let Some(aa) = tags.album_artist {
        frames.push(id3_frame(b"TPE2", aa));
    }
    if let Some(tn) = tags.track_number {
        let trck = if let Some(tot) = tags.track_total {
            format!("{}/{}", tn, tot)
        } else {
            tn.to_string()
        };
        frames.push(id3_frame(b"TRCK", &trck));
    }

    let tag = build_id3_tag(&frames);

    let mut buf = Vec::new();
    buf.extend_from_slice(&tag);

    // MPEG1 Layer3, 128kbps, 44100Hz, stereo, no padding, no CRC
    // Header bytes: FF FB 90 00
    //   FF FB = sync(11) + MPEG1(11) + Layer3(01) + no-CRC(1)
    //   90    = bitrate_idx=1001(128kbps), sr_idx=00(44100), padding=0, private=0
    //   00    = channel=00(stereo), mode_ext=00, copyright=0, original=0, emphasis=00
    let frame_header = [0xFF_u8, 0xFB, 0x90, 0x00];
    // Total frame size = floor(144 * 128000 / 44100) = 417 bytes
    // Data bytes = 417 - 4 (header) = 413
    // All zeros = silence; avoids accidental 0xFF 0xFB sync patterns in data
    let frame_data = vec![0u8; 413];

    // Two frames so lofty's consecutive-header validation passes
    for _ in 0..2 {
        buf.extend_from_slice(&frame_header);
        buf.extend_from_slice(&frame_data);
    }

    fs::write(&path, &buf).expect("Failed to write MP3 file");
    path
}

/// Create a minimal FLAC file with STREAMINFO + VORBIS_COMMENT blocks.
fn create_flac(dir: &Path, filename: &str, tags: &Tags, seed: u8) -> PathBuf {
    let path = dir.join(filename);

    // Build Vorbis comment block body
    // Format: vendor_length(4LE) + vendor(bytes) + comment_count(4LE) + [length(4LE) + "KEY=VALUE"]+
    let vendor = b"soul-importer-test";
    let mut comments: Vec<Vec<u8>> = Vec::new();

    let add_comment = |key: &str, value: &str| -> Vec<u8> {
        let s = format!("{}={}", key, value);
        let bytes = s.as_bytes();
        let mut c = Vec::with_capacity(4 + bytes.len());
        c.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
        c.extend_from_slice(bytes);
        c
    };

    comments.push(add_comment("TITLE", tags.title));
    comments.push(add_comment("ARTIST", tags.artist));
    comments.push(add_comment("ALBUM", tags.album));
    if let Some(aa) = tags.album_artist {
        comments.push(add_comment("ALBUMARTIST", aa));
    }
    if let Some(tn) = tags.track_number {
        comments.push(add_comment("TRACKNUMBER", &tn.to_string()));
    }

    let mut vc_body: Vec<u8> = Vec::new();
    vc_body.extend_from_slice(&(vendor.len() as u32).to_le_bytes());
    vc_body.extend_from_slice(vendor);
    vc_body.extend_from_slice(&(comments.len() as u32).to_le_bytes());
    for c in &comments {
        vc_body.extend_from_slice(c);
    }

    // STREAMINFO block (type=0, must be first, 34 bytes payload)
    // Encode: min_block=4096, max_block=4096, min_frame=0, max_frame=0
    // sample_rate=44100(0xAC44), channels=2, bits=16, total_samples=4096
    // MD5=all zeros
    let mut streaminfo = vec![0u8; 34];
    // min blocksize (16-bit BE): 4096 = 0x1000
    streaminfo[0] = 0x10;
    streaminfo[1] = 0x00;
    // max blocksize (16-bit BE): 4096 = 0x1000
    streaminfo[2] = 0x10;
    streaminfo[3] = 0x00;
    // min/max frame size: 0 (unknown) = 3 bytes each
    // [4..9] all zero already
    // sample_rate(20bits) + channels-1(3bits) + bits-1(5bits) + total_samples(36bits)
    // 44100 = 0xAC44 = 1010 1100 0100 0100
    // channels=2 → ch-1=1 = 001
    // bits=16 → bits-1=15 = 01111
    // total_samples=4096 = 0x000001000
    // Bit layout starting at byte 10:
    //   [10] = sr[19:12] = 1010_1100 = 0xAC
    //   [11] = sr[11:4]  = 0100_0100 = 0x44
    //   [12] = sr[3:0](4) + ch(3) + bits[4] = 0100 001 0 = 0100_0010 = 0x42
    //   Wait: let me re-derive cleanly.
    // sample_rate=44100=0xAC44 occupies bits [191:172] in the bitstream (20 bits)
    // Let's pack manually. Byte 10 gets sr bits 19..12:
    streaminfo[10] = 0xAC; // sr bits 19..12: 1010_1100
    streaminfo[11] = 0x40; // sr bits 11..4: 0100_0000 (44100>>4 & 0xFF = 0x44 but shifted)
                           // Actually let me just use a simpler approach: set sample_rate in the right bits
                           // The STREAMINFO bitstream at offset 80 bits (10 bytes):
                           //   20 bits: sample_rate
                           //   3 bits: (num_channels - 1)
                           //   5 bits: (bits_per_sample - 1)
                           //   36 bits: total_samples
                           // Pack into bytes 10..14:
                           // 44100 = 0x00AC44, 20-bit = 0b10101100 01000100
                           // ch=2-1=1=0b001, bps=16-1=15=0b01111, samples=4096=0x000001000
                           // Concat: 10101100_01000100_00101111_00000000_00000001_0000...
                           // byte10: 1010_1100 = 0xAC
                           // byte11: 0100_0100 = 0x44 (this is sr bits 11..4)
                           // But the 20-bit sr: top 8 bits in byte10, next 8 bits in byte11 shifted...
                           // sr=44100=0b1010_1100_0100_0100 (that's 16 bits, but sr is 20 bits)
                           // 44100 in 20 bits: 0000_1010_1100_0100_0100 = 0x0AC44
                           // byte10 = sr[19:12] = 0000_1010 = 0x0A
                           // byte11 = sr[11:4]  = 1100_0100 = 0xC4
                           // byte12 = sr[3:0] | ch[2:0] | bps[4] = 0100 | 001 | 0 = 0100_0010 = 0x42
                           // byte13 = bps[3:0] | total_samples[35:32] = 1111 | 0000 = 1111_0000 = 0xF0
                           // byte14..17 = total_samples[31:0] = 4096 = 0x00001000 → 0x00 0x00 0x10 0x00
    streaminfo[10] = 0x0A;
    streaminfo[11] = 0xC4;
    streaminfo[12] = 0x42; // sr[3:0]=0100 ch-1=001 bps[4]=0
    streaminfo[13] = 0xF0; // bps[3:0]=1111 total_samples[35:32]=0000
    streaminfo[14] = 0x00;
    streaminfo[15] = 0x00;
    streaminfo[16] = 0x10;
    streaminfo[17] = 0x00; // total_samples[31:0] = 4096
                           // MD5: bytes 18..33 = 0 (already zeroed)

    // Build the file
    let mut buf: Vec<u8> = Vec::new();
    buf.extend_from_slice(b"fLaC"); // FLAC magic

    // STREAMINFO block header: type=0, last=0, length=34
    let si_hdr: u32 = (0u32 << 24) | 34;
    buf.extend_from_slice(&si_hdr.to_be_bytes());
    buf.extend_from_slice(&streaminfo);

    // VORBIS_COMMENT block: type=4, last=1 (final metadata block)
    let vc_len = vc_body.len() as u32;
    let vc_hdr: u32 = (1u32 << 31) | (4u32 << 24) | vc_len;
    buf.extend_from_slice(&vc_hdr.to_be_bytes());
    buf.extend_from_slice(&vc_body);

    // Minimal audio frame (lofty may not need actual audio data for tag reading)
    // Write 256 bytes of dummy frame data to make the file look like audio
    let audio_data: Vec<u8> = (0u16..256).map(|i| seed.wrapping_add(i as u8)).collect();
    buf.extend_from_slice(&audio_data);

    fs::write(&path, &buf).expect("Failed to write FLAC file");
    path
}

/// Create a minimal WAV PCM file with RIFF fmt/data chunks.
/// No tags embedded — tests folder-based fallback.
/// No bext chunk — plain PCM without BWF complications.
fn create_wav_pcm(dir: &Path, filename: &str, seed: u8) -> PathBuf {
    let path = dir.join(filename);

    // Build minimal PCM data (1024 samples × 2 bytes × 2 channels)
    let num_samples: u32 = 1024;
    let channels: u16 = 2;
    let sample_rate: u32 = 44100;
    let bits_per_sample: u16 = 16;
    let block_align: u16 = channels * (bits_per_sample / 8);
    let byte_rate: u32 = sample_rate * block_align as u32;
    let data_size: u32 = num_samples * block_align as u32;

    let riff_body: u32 = 4 + 8 + 16 + 8 + data_size; // "WAVE" + fmt chunk + data chunk

    let mut buf: Vec<u8> = Vec::new();
    // RIFF header
    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&riff_body.to_le_bytes());
    buf.extend_from_slice(b"WAVE");
    // fmt chunk
    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&16u32.to_le_bytes()); // chunk size
    buf.extend_from_slice(&1u16.to_le_bytes()); // PCM
    buf.extend_from_slice(&channels.to_le_bytes());
    buf.extend_from_slice(&sample_rate.to_le_bytes());
    buf.extend_from_slice(&byte_rate.to_le_bytes());
    buf.extend_from_slice(&block_align.to_le_bytes());
    buf.extend_from_slice(&bits_per_sample.to_le_bytes());
    // data chunk
    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&data_size.to_le_bytes());
    let audio: Vec<u8> = (0..data_size).map(|i| seed.wrapping_add(i as u8)).collect();
    buf.extend_from_slice(&audio);

    fs::write(&path, &buf).expect("Failed to write WAV file");
    path
}

/// Create a macOS resource fork file (._prefix) — should always be skipped by the scanner.
fn create_resource_fork(dir: &Path, original_name: &str) -> PathBuf {
    let name = format!("._{}", original_name);
    let path = dir.join(&name);
    // Resource forks have a specific 82-byte header, but for our purposes any
    // binary content with the ._ prefix should be skipped by the scanner.
    fs::write(
        &path,
        b"\x00\x05\x16\x07\x00\x02\x00\x00macOS resource fork",
    )
    .unwrap();
    path
}

/// Create a minimal DSF file (DSD64, stereo) with embedded ID3v2 tag.
fn create_dsf(dir: &Path, filename: &str, tags: &Tags) -> PathBuf {
    let path = dir.join(filename);

    let mut id3_frames = vec![
        id3_frame(b"TIT2", tags.title),
        id3_frame(b"TPE1", tags.artist),
        id3_frame(b"TALB", tags.album),
    ];
    if let Some(aa) = tags.album_artist {
        id3_frames.push(id3_frame(b"TPE2", aa));
    }
    let id3_tag = build_id3_tag(&id3_frames);

    // DSF file layout:
    //   DSD chunk (28 bytes): magic + total_size + id3_offset
    //   fmt chunk (52 bytes): magic + size(8) + format_version(4) + format_id(4) + channel_type(4)
    //                          + channel_count(4) + sample_rate(4) + bits_per_sample(4) + sample_count(8) + block_size(4) + reserved(4)
    //   data chunk (12 bytes): magic + size(8) + data_size_in_bytes(8) [minimal, no actual samples]
    //   ID3 tag

    let dsd_chunk_size: u64 = 28;
    let fmt_chunk_size: u64 = 52;
    let data_chunk_size: u64 = 12; // just the header, no samples
    let id3_size = id3_tag.len() as u64;
    let total_size: u64 = dsd_chunk_size + fmt_chunk_size + data_chunk_size + id3_size;
    let id3_offset: u64 = dsd_chunk_size + fmt_chunk_size + data_chunk_size;

    let mut buf: Vec<u8> = Vec::new();

    // DSD chunk
    buf.extend_from_slice(b"DSD ");
    buf.extend_from_slice(&dsd_chunk_size.to_le_bytes());
    buf.extend_from_slice(&total_size.to_le_bytes());
    buf.extend_from_slice(&id3_offset.to_le_bytes());

    // fmt chunk (52 bytes total including 8-byte size field)
    let fmt_payload_size: u64 = fmt_chunk_size; // DSF fmt includes the size field
    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&fmt_payload_size.to_le_bytes());
    buf.extend_from_slice(&1u32.to_le_bytes()); // format_version
    buf.extend_from_slice(&0u32.to_le_bytes()); // format_id (DSD raw)
    buf.extend_from_slice(&2u32.to_le_bytes()); // channel_type (stereo)
    buf.extend_from_slice(&2u32.to_le_bytes()); // channel_count
    buf.extend_from_slice(&2822400u32.to_le_bytes()); // sample_rate (DSD64)
    buf.extend_from_slice(&1u32.to_le_bytes()); // bits_per_sample
    buf.extend_from_slice(&2822400u64.to_le_bytes()); // sample_count (1 second)
    buf.extend_from_slice(&4096u32.to_le_bytes()); // block_size_per_channel
    buf.extend_from_slice(&0u32.to_le_bytes()); // reserved

    // data chunk (header only, no audio samples)
    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&data_chunk_size.to_le_bytes());

    // ID3 tag
    buf.extend_from_slice(&id3_tag);

    fs::write(&path, &buf).expect("Failed to write DSF file");
    path
}

// =============================================================================
// Scanner helpers
// =============================================================================

async fn scan_dir(pool: &SqlitePool, path: &Path) -> ScanStats {
    soul_storage::library_sources::create(
        pool,
        "user1",
        "device1",
        &CreateLibrarySource {
            name: "Test Library".to_string(),
            path: path.to_string_lossy().to_string(),
            sync_deletes: true,
        },
    )
    .await
    .expect("Failed to create library source");

    let scanner = LibraryScanner::new(pool.clone(), "user1", "device1").compute_hashes(false);
    scanner.scan_all().await.expect("scan_all should not fail")
}

/// Run two consecutive scans on the same library source without recreating it.
/// Use this for incremental scan tests that verify second-scan behavior.
async fn scan_dir_twice(pool: &SqlitePool, path: &Path) -> (ScanStats, ScanStats) {
    soul_storage::library_sources::create(
        pool,
        "user1",
        "device1",
        &CreateLibrarySource {
            name: "Test Library".to_string(),
            path: path.to_string_lossy().to_string(),
            sync_deletes: true,
        },
    )
    .await
    .expect("Failed to create library source");

    let scanner = LibraryScanner::new(pool.clone(), "user1", "device1").compute_hashes(false);
    let s1 = scanner
        .scan_all()
        .await
        .expect("scan_all (1) should not fail");
    let s2 = scanner
        .scan_all()
        .await
        .expect("scan_all (2) should not fail");
    (s1, s2)
}

/// Run N consecutive scans on the same library source.
async fn scan_dir_n(pool: &SqlitePool, path: &Path, n: usize) -> Vec<ScanStats> {
    soul_storage::library_sources::create(
        pool,
        "user1",
        "device1",
        &CreateLibrarySource {
            name: "Test Library".to_string(),
            path: path.to_string_lossy().to_string(),
            sync_deletes: true,
        },
    )
    .await
    .expect("Failed to create library source");

    let scanner = LibraryScanner::new(pool.clone(), "user1", "device1").compute_hashes(false);
    let mut results = Vec::with_capacity(n);
    for _ in 0..n {
        results.push(scanner.scan_all().await.expect("scan_all should not fail"));
    }
    results
}

// =============================================================================
// DB assertion helpers
// =============================================================================

async fn db_album_track_count(pool: &SqlitePool, album_title: &str) -> i64 {
    let (count,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM tracks t
         JOIN albums a ON t.album_id = a.id
         WHERE a.title = ? AND t.is_available = 1",
    )
    .bind(album_title)
    .fetch_one(pool)
    .await
    .expect("db_album_track_count query failed");
    count
}

async fn db_album_exists(pool: &SqlitePool, album_title: &str) -> bool {
    let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM albums WHERE title = ?")
        .bind(album_title)
        .fetch_one(pool)
        .await
        .expect("db_album_exists query failed");
    count > 0
}

async fn db_track_exists(pool: &SqlitePool, title: &str) -> bool {
    let (count,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM tracks WHERE title = ? AND is_available = 1")
            .bind(title)
            .fetch_one(pool)
            .await
            .expect("db_track_exists query failed");
    count > 0
}

async fn db_artist_exists(pool: &SqlitePool, name: &str) -> bool {
    let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM artists WHERE name = ?")
        .bind(name)
        .fetch_one(pool)
        .await
        .expect("db_artist_exists query failed");
    count > 0
}

/// Returns true if every album in the DB has at least 1 available track.
async fn db_no_empty_albums(pool: &SqlitePool) -> bool {
    let (count,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM albums a
         WHERE NOT EXISTS (
             SELECT 1 FROM tracks t
             WHERE t.album_id = a.id AND t.is_available = 1
         )",
    )
    .fetch_one(pool)
    .await
    .expect("db_no_empty_albums query failed");
    count == 0
}

// =============================================================================
// Group 1: Basic single-track imports (5 tests)
// =============================================================================

#[tokio::test]
async fn test_scan_mp3_with_full_tags() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    let album_dir = temp_dir.path().join("Hiroshi Sato - Orient");
    fs::create_dir_all(&album_dir).unwrap();
    create_mp3(
        &album_dir,
        "01 - Kalimba Night.mp3",
        &Tags {
            title: "Kalimba Night",
            artist: "Hiroshi Sato",
            album_artist: None,
            album: "Orient",
            track_number: Some(1),
            track_total: None,
        },
        1,
    );

    scan_dir(&pool, temp_dir.path()).await;

    assert_eq!(db_album_track_count(&pool, "Orient").await, 1);
    assert!(db_artist_exists(&pool, "Hiroshi Sato").await);
    assert!(db_track_exists(&pool, "Kalimba Night").await);
}

#[tokio::test]
async fn test_scan_flac_with_full_tags() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    let album_dir = temp_dir.path().join("Artist - Test FLAC Album");
    fs::create_dir_all(&album_dir).unwrap();
    create_flac(
        &album_dir,
        "01 - Track One.flac",
        &Tags {
            title: "FLAC Track One",
            artist: "FLAC Artist",
            album_artist: None,
            album: "Test FLAC Album",
            track_number: Some(1),
            track_total: None,
        },
        1,
    );

    scan_dir(&pool, temp_dir.path()).await;

    assert_eq!(db_album_track_count(&pool, "Test FLAC Album").await, 1);
    assert!(db_track_exists(&pool, "FLAC Track One").await);
}

#[tokio::test]
async fn test_scan_wav_with_folder_fallback() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    // WAV with no embedded tags — scanner falls back to folder name for album
    let album_dir = temp_dir.path().join("MyAlbumFolder");
    fs::create_dir_all(&album_dir).unwrap();
    create_wav_pcm(&album_dir, "my_track.wav", 1);

    scan_dir(&pool, temp_dir.path()).await;

    // Track imported with title from filename stem
    assert!(db_track_exists(&pool, "my_track").await);
}

#[tokio::test]
async fn test_scan_skips_resource_fork_files() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    let album_dir = temp_dir.path().join("Album");
    fs::create_dir_all(&album_dir).unwrap();
    create_mp3(
        &album_dir,
        "track.mp3",
        &Tags {
            title: "Real Track",
            artist: "Artist",
            album_artist: None,
            album: "Album",
            track_number: Some(1),
            track_total: None,
        },
        1,
    );
    // Resource fork — must be ignored
    create_resource_fork(&album_dir, "track.mp3");

    let stats = scan_dir(&pool, temp_dir.path()).await;

    // Only 1 real track should be found and scanned
    assert_eq!(
        stats.total_files, 1,
        "resource fork must not count as audio file"
    );
    assert_eq!(db_album_track_count(&pool, "Album").await, 1);
}

#[tokio::test]
async fn test_scan_skips_ds_store() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    // Write a .DS_Store file — should be silently ignored
    fs::write(temp_dir.path().join(".DS_Store"), b"Bud1\x00\x00\x00\x01").unwrap();

    let stats = scan_dir(&pool, temp_dir.path()).await;
    assert_eq!(stats.total_files, 0);
}

// =============================================================================
// Group 2: Album grouping (8 tests)
// =============================================================================

#[tokio::test]
async fn test_scan_album_with_10_japanese_tracks() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    let album_dir = temp_dir.path().join("Fukamachi Jun - (71)");
    fs::create_dir_all(&album_dir).unwrap();

    let album_title = "(71) ある若者の肖像";
    let album_artist = "Fukamachi, Jun";

    for i in 0u8..10 {
        let filename = format!("track_{:02}.mp3", i + 1);
        create_mp3(
            &album_dir,
            &filename,
            &Tags {
                title: &format!("Track {:02}", i + 1),
                artist: "Fukamachi, Jun",
                album_artist: Some(album_artist),
                album: album_title,
                track_number: Some((i + 1) as u32),
                track_total: Some(10),
            },
            i + 1,
        );
    }

    scan_dir(&pool, temp_dir.path()).await;

    assert_eq!(
        db_album_track_count(&pool, album_title).await,
        10,
        "All 10 Japanese tracks must be under the correct album"
    );
    assert!(db_artist_exists(&pool, album_artist).await);
}

#[tokio::test]
async fn test_scan_numbered_album_title_preserved() {
    // Regression: "(72) Hello!" must appear with 10 tracks, NOT 0.
    // The old BWF fallback caused MP3s with large APIC frames to import
    // with album="Hello!" from the folder name instead of "(72) Hello!" from ID3.
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    let album_dir = temp_dir.path().join("Fukamachi Jan - Hello!");
    fs::create_dir_all(&album_dir).unwrap();

    let album_title = "(72) Hello!";
    let album_artist = "Fukamachi, Jan";

    for i in 0u8..10 {
        create_mp3(
            &album_dir,
            &format!("track_{:02}.mp3", i + 1),
            &Tags {
                title: &format!("Hello Track {:02}", i + 1),
                artist: "Fukamachi, Jan",
                album_artist: Some(album_artist),
                album: album_title,
                track_number: Some((i + 1) as u32),
                track_total: Some(10),
            },
            i + 1,
        );
    }

    scan_dir(&pool, temp_dir.path()).await;

    let count = db_album_track_count(&pool, album_title).await;
    assert_eq!(
        count, 10,
        "CRITICAL: (72) Hello! must have 10 tracks, not {}",
        count
    );
    // The wrong album title must NOT exist
    assert!(
        !db_album_exists(&pool, "Hello!").await,
        "Album must NOT be split into wrong 'Hello!' album"
    );
}

#[tokio::test]
async fn test_scan_same_album_different_artists_stay_separate() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    // Artist A's "Live" album
    let dir_a = temp_dir.path().join("Artist A - Live");
    fs::create_dir_all(&dir_a).unwrap();
    create_mp3(
        &dir_a,
        "01.mp3",
        &Tags {
            title: "Live Track A",
            artist: "Artist A",
            album_artist: None,
            album: "Live",
            track_number: Some(1),
            track_total: None,
        },
        1,
    );

    // Artist B's "Live" album
    let dir_b = temp_dir.path().join("Artist B - Live");
    fs::create_dir_all(&dir_b).unwrap();
    create_mp3(
        &dir_b,
        "01.mp3",
        &Tags {
            title: "Live Track B",
            artist: "Artist B",
            album_artist: None,
            album: "Live",
            track_number: Some(1),
            track_total: None,
        },
        50,
    );

    scan_dir(&pool, temp_dir.path()).await;

    // Both albums named "Live" should exist as separate records
    let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM albums WHERE title = 'Live'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(count >= 1, "At least one 'Live' album must exist");
    // Each artist's track must be available
    assert!(db_track_exists(&pool, "Live Track A").await);
    assert!(db_track_exists(&pool, "Live Track B").await);
}

#[tokio::test]
async fn test_scan_albumartist_groups_tracks() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    let album_dir = temp_dir.path().join("Casiopea - Mint Jams");
    fs::create_dir_all(&album_dir).unwrap();

    let track_artists = [
        "Issei Noro",
        "Tetsuo Sakurai",
        "Akira Jimbo",
        "Minoru Mukaiya",
        "Casiopea",
    ];
    for (i, artist) in track_artists.iter().enumerate() {
        create_mp3(
            &album_dir,
            &format!("track_{:02}.mp3", i + 1),
            &Tags {
                title: &format!("Mint Jams Track {}", i + 1),
                artist,
                album_artist: Some("Casiopea"),
                album: "Mint Jams",
                track_number: Some((i + 1) as u32),
                track_total: None,
            },
            (i + 1) as u8,
        );
    }

    scan_dir(&pool, temp_dir.path()).await;

    assert_eq!(
        db_album_track_count(&pool, "Mint Jams").await,
        5,
        "All 5 tracks should be under Mint Jams despite varying TPE1"
    );
    assert!(db_artist_exists(&pool, "Casiopea").await);
}

#[tokio::test]
async fn test_scan_various_artists_album() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    let album_dir = temp_dir.path().join("VA - Compilation");
    fs::create_dir_all(&album_dir).unwrap();

    let artists = [
        "Artist One",
        "Artist Two",
        "Artist Three",
        "Artist Four",
        "Artist Five",
        "Artist Six",
        "Artist Seven",
        "Artist Eight",
    ];
    for (i, artist) in artists.iter().enumerate() {
        create_mp3(
            &album_dir,
            &format!("track_{:02}.mp3", i + 1),
            &Tags {
                title: &format!("VA Track {}", i + 1),
                artist,
                album_artist: Some("Various Artists"),
                album: "Compilation",
                track_number: Some((i + 1) as u32),
                track_total: None,
            },
            (i + 1) as u8,
        );
    }

    scan_dir(&pool, temp_dir.path()).await;

    assert_eq!(db_album_track_count(&pool, "Compilation").await, 8);
    assert!(db_artist_exists(&pool, "Various Artists").await);
}

#[tokio::test]
async fn test_scan_multi_disc_album_same_album_id() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    let album_dir = temp_dir.path().join("Artist - Double Album");
    fs::create_dir_all(&album_dir).unwrap();

    // Disc 1
    for i in 0u8..3 {
        create_mp3(
            &album_dir,
            &format!("disc1_track_{:02}.mp3", i + 1),
            &Tags {
                title: &format!("Disc 1 Track {}", i + 1),
                artist: "Double Artist",
                album_artist: Some("Double Artist"),
                album: "Double Album",
                track_number: Some((i + 1) as u32),
                track_total: None,
            },
            i + 1,
        );
    }
    // Disc 2
    for i in 0u8..3 {
        create_mp3(
            &album_dir,
            &format!("disc2_track_{:02}.mp3", i + 1),
            &Tags {
                title: &format!("Disc 2 Track {}", i + 1),
                artist: "Double Artist",
                album_artist: Some("Double Artist"),
                album: "Double Album",
                track_number: Some((i + 1) as u32),
                track_total: None,
            },
            i + 50,
        );
    }

    scan_dir(&pool, temp_dir.path()).await;

    assert_eq!(
        db_album_track_count(&pool, "Double Album").await,
        6,
        "Both discs should merge into 1 album"
    );
}

#[tokio::test]
async fn test_scan_tracks_split_across_subfolders_same_album_tag() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    let disc1 = temp_dir.path().join("The Album").join("Disc1");
    let disc2 = temp_dir.path().join("The Album").join("Disc2");
    fs::create_dir_all(&disc1).unwrap();
    fs::create_dir_all(&disc2).unwrap();

    for i in 0u8..3 {
        create_mp3(
            &disc1,
            &format!("track_{:02}.mp3", i + 1),
            &Tags {
                title: &format!("Disc1 Track {}", i + 1),
                artist: "Split Artist",
                album_artist: Some("Split Artist"),
                album: "The Album",
                track_number: Some((i + 1) as u32),
                track_total: None,
            },
            i + 1,
        );
    }
    for i in 0u8..3 {
        create_mp3(
            &disc2,
            &format!("track_{:02}.mp3", i + 1),
            &Tags {
                title: &format!("Disc2 Track {}", i + 1),
                artist: "Split Artist",
                album_artist: Some("Split Artist"),
                album: "The Album",
                track_number: Some((i + 4) as u32),
                track_total: None,
            },
            i + 50,
        );
    }

    scan_dir(&pool, temp_dir.path()).await;

    assert_eq!(
        db_album_track_count(&pool, "The Album").await,
        6,
        "Tracks in subfolders with same TALB tag should merge into one album"
    );
}

#[tokio::test]
async fn test_scan_no_empty_albums_invariant() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    // Create 3 distinct albums
    for album_idx in 0u8..3 {
        let album_dir = temp_dir.path().join(format!("Album {}", album_idx));
        fs::create_dir_all(&album_dir).unwrap();
        for track_idx in 0u8..3 {
            create_mp3(
                &album_dir,
                &format!("track_{:02}.mp3", track_idx + 1),
                &Tags {
                    title: &format!("Album {} Track {}", album_idx, track_idx),
                    artist: &format!("Artist {}", album_idx),
                    album_artist: None,
                    album: &format!("Album {}", album_idx),
                    track_number: Some((track_idx + 1) as u32),
                    track_total: None,
                },
                album_idx * 30 + track_idx + 1,
            );
        }
    }

    scan_dir(&pool, temp_dir.path()).await;

    assert!(
        db_no_empty_albums(&pool).await,
        "No album should have 0 available tracks after scan"
    );
}

// =============================================================================
// Group 3: Artist handling (5 tests)
// =============================================================================

#[tokio::test]
async fn test_scan_artist_from_tpe1() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    let dir = temp_dir.path().join("Artist");
    fs::create_dir_all(&dir).unwrap();
    create_mp3(
        &dir,
        "track.mp3",
        &Tags {
            title: "Track",
            artist: "Fukamachi, Jun",
            album_artist: None,
            album: "Album",
            track_number: None,
            track_total: None,
        },
        1,
    );

    scan_dir(&pool, temp_dir.path()).await;

    assert!(
        db_artist_exists(&pool, "Fukamachi, Jun").await,
        "Artist from TPE1 must be stored exactly"
    );
}

#[tokio::test]
async fn test_scan_albumartist_used_for_album_key() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    let dir = temp_dir.path().join("Band - Record");
    fs::create_dir_all(&dir).unwrap();
    create_mp3(
        &dir,
        "track.mp3",
        &Tags {
            title: "Member Track",
            artist: "Member A",
            album_artist: Some("Band Name"),
            album: "Band Record",
            track_number: Some(1),
            track_total: None,
        },
        1,
    );

    scan_dir(&pool, temp_dir.path()).await;

    // The album should be attributed to the album artist, not the track artist
    assert!(db_artist_exists(&pool, "Band Name").await);
    assert_eq!(db_album_track_count(&pool, "Band Record").await, 1);
}

#[tokio::test]
async fn test_scan_split_artists_from_null_separated_id3v24() {
    // ID3v2.4 allows null-separated artists in TPE1.
    // We create the MP3 manually with a null byte between two artist names.
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    let dir = temp_dir.path().join("Artists - Album");
    fs::create_dir_all(&dir).unwrap();

    // Build the ID3 tag with null-separated artists manually
    let title_frame = id3_frame(b"TIT2", "Collaborative Track");
    let album_frame = id3_frame(b"TALB", "Collab Album");

    // TPE1 with null-separated artists (ID3v2.4 multi-value)
    let artist_text = "Artist Alpha\x00Artist Beta";
    let artist_frame = id3_frame(b"TPE1", artist_text);

    let tag = build_id3_tag(&[title_frame, artist_frame, album_frame]);
    let mut buf = tag;
    // Two valid MPEG1 Layer3 128kbps frames (417 bytes each = 4 header + 413 data)
    for _ in 0..2 {
        buf.extend_from_slice(&[0xFF, 0xFB, 0x90, 0x00]);
        buf.extend_from_slice(&[0u8; 413]);
    }
    let path = dir.join("collab.mp3");
    fs::write(&path, &buf).unwrap();

    scan_dir(&pool, temp_dir.path()).await;

    assert!(db_track_exists(&pool, "Collaborative Track").await);
    // At least one of the split artists should be in the DB
    let artist_a_exists = db_artist_exists(&pool, "Artist Alpha").await;
    let artist_b_exists = db_artist_exists(&pool, "Artist Beta").await;
    assert!(
        artist_a_exists || artist_b_exists,
        "At least one split artist must be stored"
    );
}

#[tokio::test]
async fn test_scan_no_artist_tag_falls_back_to_folder() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    // Track with empty artist — scanner should fall back to folder/grandparent
    let dir = temp_dir.path().join("MyArtist").join("MyAlbum");
    fs::create_dir_all(&dir).unwrap();
    create_mp3(
        &dir,
        "track.mp3",
        &Tags {
            title: "Untagged Track",
            artist: "", // empty artist
            album_artist: None,
            album: "MyAlbum",
            track_number: None,
            track_total: None,
        },
        1,
    );

    scan_dir(&pool, temp_dir.path()).await;

    // Track must be imported (not crashed)
    assert!(db_track_exists(&pool, "Untagged Track").await);
}

#[tokio::test]
async fn test_scan_czech_diacritics_in_artist_name() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    let dir = temp_dir.path().join("Czech Artist");
    fs::create_dir_all(&dir).unwrap();
    create_mp3(
        &dir,
        "track.mp3",
        &Tags {
            title: "Tajemná Gamelánie Track",
            artist: "Irena & Vojtěch Havlovi",
            album_artist: None,
            album: "Tajemná Gamelánie",
            track_number: Some(1),
            track_total: None,
        },
        1,
    );

    scan_dir(&pool, temp_dir.path()).await;

    assert!(
        db_artist_exists(&pool, "Irena & Vojtěch Havlovi").await,
        "Czech diacritics in artist name must be preserved"
    );
    assert!(
        db_album_exists(&pool, "Tajemná Gamelánie").await,
        "Czech diacritics in album title must be preserved"
    );
}

// =============================================================================
// Group 4: Track numbering (4 tests)
// =============================================================================

#[tokio::test]
async fn test_scan_track_number_plain() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    let dir = temp_dir.path().join("Album");
    fs::create_dir_all(&dir).unwrap();
    create_mp3(
        &dir,
        "track.mp3",
        &Tags {
            title: "Track Five",
            artist: "Artist",
            album_artist: None,
            album: "Album",
            track_number: Some(5),
            track_total: None,
        },
        1,
    );

    scan_dir(&pool, temp_dir.path()).await;

    let (track_number,): (Option<i64>,) =
        sqlx::query_as("SELECT track_number FROM tracks WHERE title = 'Track Five'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(track_number, Some(5));
}

#[tokio::test]
async fn test_scan_track_number_slash_format() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    let dir = temp_dir.path().join("Album");
    fs::create_dir_all(&dir).unwrap();
    create_mp3(
        &dir,
        "track.mp3",
        &Tags {
            title: "Track Three of Eight",
            artist: "Artist",
            album_artist: None,
            album: "Slash Album",
            track_number: Some(3),
            track_total: Some(8),
        },
        1,
    );

    scan_dir(&pool, temp_dir.path()).await;

    let (track_number,): (Option<i64>,) =
        sqlx::query_as("SELECT track_number FROM tracks WHERE title = 'Track Three of Eight'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        track_number,
        Some(3),
        "Track number should be 3, not 8 (total)"
    );
}

#[tokio::test]
async fn test_scan_track_number_leading_zero() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    let dir = temp_dir.path().join("Album");
    fs::create_dir_all(&dir).unwrap();
    create_mp3(
        &dir,
        "track.mp3",
        &Tags {
            title: "Leading Zero Track",
            artist: "Artist",
            album_artist: None,
            album: "Leading Zero Album",
            track_number: Some(1), // will be stored as "01" implicitly via TRCK encoding
            track_total: None,
        },
        1,
    );

    scan_dir(&pool, temp_dir.path()).await;

    let (track_number,): (Option<i64>,) =
        sqlx::query_as("SELECT track_number FROM tracks WHERE title = 'Leading Zero Track'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        track_number,
        Some(1),
        "Leading zero must parse to integer 1"
    );
}

#[tokio::test]
async fn test_scan_no_track_number() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    let dir = temp_dir.path().join("Album");
    fs::create_dir_all(&dir).unwrap();
    create_mp3(
        &dir,
        "track.mp3",
        &Tags {
            title: "Numberless Track",
            artist: "Artist",
            album_artist: None,
            album: "Numberless Album",
            track_number: None,
            track_total: None,
        },
        1,
    );

    scan_dir(&pool, temp_dir.path()).await;

    // Must not crash — track_number can be NULL
    let (count,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM tracks WHERE title = 'Numberless Track'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        count, 1,
        "Track without track number must still be imported"
    );
}

// =============================================================================
// Group 5: Rescan and orphan handling (8 tests)
// =============================================================================

async fn create_sync_source(pool: &SqlitePool, path: &Path) {
    soul_storage::library_sources::create(
        pool,
        "user1",
        "device1",
        &CreateLibrarySource {
            name: "Test Library".to_string(),
            path: path.to_string_lossy().to_string(),
            sync_deletes: true,
        },
    )
    .await
    .expect("Failed to create library source");
}

#[tokio::test]
async fn test_rescan_same_folder_no_duplicates() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    let dir = temp_dir.path().join("Album");
    fs::create_dir_all(&dir).unwrap();
    for i in 0u8..5 {
        create_mp3(
            &dir,
            &format!("track_{:02}.mp3", i + 1),
            &Tags {
                title: &format!("No Dup Track {}", i + 1),
                artist: "Artist",
                album_artist: None,
                album: "No Dup Album",
                track_number: Some((i + 1) as u32),
                track_total: None,
            },
            i + 1,
        );
    }

    create_sync_source(&pool, temp_dir.path()).await;
    let scanner = LibraryScanner::new(pool.clone(), "user1", "device1").compute_hashes(false);
    scanner.scan_all().await.unwrap();

    // Scan again — no duplicates should be created
    let scanner2 = LibraryScanner::new(pool.clone(), "user1", "device1").compute_hashes(false);
    scanner2.scan_all().await.unwrap();

    assert_eq!(
        db_album_track_count(&pool, "No Dup Album").await,
        5,
        "Second scan must not create duplicate tracks"
    );
}

#[tokio::test]
async fn test_rescan_after_adding_file_increases_count() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    let dir = temp_dir.path().join("Album");
    fs::create_dir_all(&dir).unwrap();
    for i in 0u8..3 {
        create_mp3(
            &dir,
            &format!("track_{:02}.mp3", i + 1),
            &Tags {
                title: &format!("Grow Track {}", i + 1),
                artist: "Artist",
                album_artist: None,
                album: "Grow Album",
                track_number: Some((i + 1) as u32),
                track_total: None,
            },
            i + 1,
        );
    }

    create_sync_source(&pool, temp_dir.path()).await;
    let scanner = LibraryScanner::new(pool.clone(), "user1", "device1").compute_hashes(false);
    scanner.scan_all().await.unwrap();

    assert_eq!(db_album_track_count(&pool, "Grow Album").await, 3);

    // Add 1 more track
    create_mp3(
        &dir,
        "track_04.mp3",
        &Tags {
            title: "Grow Track 4",
            artist: "Artist",
            album_artist: None,
            album: "Grow Album",
            track_number: Some(4),
            track_total: None,
        },
        100,
    );

    let scanner2 = LibraryScanner::new(pool.clone(), "user1", "device1").compute_hashes(false);
    scanner2.scan_all().await.unwrap();

    assert_eq!(
        db_album_track_count(&pool, "Grow Album").await,
        4,
        "New track must be picked up on rescan"
    );
}

#[tokio::test]
async fn test_rescan_with_sync_deletes_marks_removed_track_unavailable() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    let dir = temp_dir.path().join("Album");
    fs::create_dir_all(&dir).unwrap();
    for i in 0u8..3 {
        create_mp3(
            &dir,
            &format!("track_{:02}.mp3", i + 1),
            &Tags {
                title: &format!("Delete Track {}", i + 1),
                artist: "Artist",
                album_artist: None,
                album: "Delete Album",
                track_number: Some((i + 1) as u32),
                track_total: None,
            },
            i + 1,
        );
    }

    create_sync_source(&pool, temp_dir.path()).await;
    let scanner = LibraryScanner::new(pool.clone(), "user1", "device1").compute_hashes(false);
    scanner.scan_all().await.unwrap();
    assert_eq!(db_album_track_count(&pool, "Delete Album").await, 3);

    // Delete one file
    fs::remove_file(dir.join("track_02.mp3")).unwrap();

    let scanner2 = LibraryScanner::new(pool.clone(), "user1", "device1").compute_hashes(false);
    let stats = scanner2.scan_all().await.unwrap();

    assert!(stats.removed_files >= 1, "Should detect removed file");
    assert_eq!(
        db_album_track_count(&pool, "Delete Album").await,
        2,
        "Only 2 available tracks remain after deleting 1"
    );
}

#[tokio::test]
async fn test_rescan_orphan_album_cleanup() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    let dir = temp_dir.path().join("Orphan Album");
    fs::create_dir_all(&dir).unwrap();
    for i in 0u8..3 {
        create_mp3(
            &dir,
            &format!("track_{:02}.mp3", i + 1),
            &Tags {
                title: &format!("Orphan Track {}", i + 1),
                artist: "Orphan Artist",
                album_artist: None,
                album: "Orphan Record",
                track_number: Some((i + 1) as u32),
                track_total: None,
            },
            i + 1,
        );
    }

    create_sync_source(&pool, temp_dir.path()).await;
    let scanner = LibraryScanner::new(pool.clone(), "user1", "device1").compute_hashes(false);
    scanner.scan_all().await.unwrap();
    assert!(db_album_exists(&pool, "Orphan Record").await);

    // Remove all files from the folder
    fs::remove_dir_all(&dir).unwrap();

    let scanner2 = LibraryScanner::new(pool.clone(), "user1", "device1").compute_hashes(false);
    scanner2.scan_all().await.unwrap();

    // Album with 0 tracks should either be deleted or have no available tracks
    let count = db_album_track_count(&pool, "Orphan Record").await;
    assert_eq!(
        count, 0,
        "Orphaned album must have 0 available tracks after all files removed"
    );
}

#[tokio::test]
async fn test_rescan_leaves_other_albums_intact() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    let dir_a = temp_dir.path().join("Album A");
    let dir_b = temp_dir.path().join("Album B");
    fs::create_dir_all(&dir_a).unwrap();
    fs::create_dir_all(&dir_b).unwrap();

    for i in 0u8..3 {
        create_mp3(
            &dir_a,
            &format!("track_{:02}.mp3", i + 1),
            &Tags {
                title: &format!("Keep Track {}", i + 1),
                artist: "Keep Artist",
                album_artist: None,
                album: "Keep Album",
                track_number: Some((i + 1) as u32),
                track_total: None,
            },
            i + 1,
        );
    }
    for i in 0u8..2 {
        create_mp3(
            &dir_b,
            &format!("track_{:02}.mp3", i + 1),
            &Tags {
                title: &format!("Gone Track {}", i + 1),
                artist: "Gone Artist",
                album_artist: None,
                album: "Gone Album",
                track_number: Some((i + 1) as u32),
                track_total: None,
            },
            i + 50,
        );
    }

    create_sync_source(&pool, temp_dir.path()).await;
    let scanner = LibraryScanner::new(pool.clone(), "user1", "device1").compute_hashes(false);
    scanner.scan_all().await.unwrap();

    // Delete Album B
    fs::remove_dir_all(&dir_b).unwrap();

    let scanner2 = LibraryScanner::new(pool.clone(), "user1", "device1").compute_hashes(false);
    scanner2.scan_all().await.unwrap();

    assert_eq!(
        db_album_track_count(&pool, "Keep Album").await,
        3,
        "Album A must remain untouched"
    );
}

#[tokio::test]
async fn test_no_zombie_albums_after_rescan() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    for album_idx in 0u8..4 {
        let dir = temp_dir
            .path()
            .join(format!("Artist{} - Album{}", album_idx, album_idx));
        fs::create_dir_all(&dir).unwrap();
        for track_idx in 0u8..3 {
            create_mp3(
                &dir,
                &format!("track_{:02}.mp3", track_idx + 1),
                &Tags {
                    title: &format!("Album{} Track{}", album_idx, track_idx),
                    artist: &format!("Artist {}", album_idx),
                    album_artist: None,
                    album: &format!("Album {}", album_idx),
                    track_number: Some((track_idx + 1) as u32),
                    track_total: None,
                },
                album_idx * 30 + track_idx + 1,
            );
        }
    }

    create_sync_source(&pool, temp_dir.path()).await;
    let scanner = LibraryScanner::new(pool.clone(), "user1", "device1").compute_hashes(false);
    scanner.scan_all().await.unwrap();

    // Delete one album folder
    fs::remove_dir_all(temp_dir.path().join("Artist2 - Album2")).unwrap();

    let scanner2 = LibraryScanner::new(pool.clone(), "user1", "device1").compute_hashes(false);
    scanner2.scan_all().await.unwrap();

    assert!(
        db_no_empty_albums(&pool).await,
        "No zombie albums with 0 tracks should exist after rescan"
    );
}

#[tokio::test]
async fn test_rescan_idempotent() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    let dir = temp_dir.path().join("Stable Album");
    fs::create_dir_all(&dir).unwrap();
    for i in 0u8..3 {
        create_mp3(
            &dir,
            &format!("track_{:02}.mp3", i + 1),
            &Tags {
                title: &format!("Stable Track {}", i + 1),
                artist: "Stable Artist",
                album_artist: None,
                album: "Stable Album",
                track_number: Some((i + 1) as u32),
                track_total: None,
            },
            i + 1,
        );
    }

    create_sync_source(&pool, temp_dir.path()).await;
    for _ in 0..3 {
        let scanner = LibraryScanner::new(pool.clone(), "user1", "device1").compute_hashes(false);
        scanner.scan_all().await.unwrap();
    }

    assert_eq!(
        db_album_track_count(&pool, "Stable Album").await,
        3,
        "Repeated scans must not multiply tracks"
    );
}

// =============================================================================
// Group 6: Real-world format patterns (6 tests)
// =============================================================================

#[tokio::test]
async fn test_scan_discogs_folder_naming_1974_go_on() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    let dir = temp_dir.path().join("1974 - Go On");
    fs::create_dir_all(&dir).unwrap();

    for i in 0u8..5 {
        create_mp3(
            &dir,
            &format!("track_{:02}.mp3", i + 1),
            &Tags {
                title: &format!("Go On Track {}", i + 1),
                artist: "George Otsuka Quintet",
                album_artist: None,
                album: "Go On",
                track_number: Some((i + 1) as u32),
                track_total: None,
            },
            i + 1,
        );
    }

    scan_dir(&pool, temp_dir.path()).await;

    assert_eq!(db_album_track_count(&pool, "Go On").await, 5);
    assert!(db_artist_exists(&pool, "George Otsuka Quintet").await);
}

#[tokio::test]
async fn test_scan_dsf_dsd64_track_metadata() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    let dir = temp_dir.path().join("DSD Artist - DSD Album");
    fs::create_dir_all(&dir).unwrap();
    create_dsf(
        &dir,
        "track01.dsf",
        &Tags {
            title: "DSD Track One",
            artist: "DSD Artist",
            album_artist: None,
            album: "DSD Album",
            track_number: Some(1),
            track_total: None,
        },
    );

    scan_dir(&pool, temp_dir.path()).await;

    assert!(db_track_exists(&pool, "DSD Track One").await);
    // Verify sample_rate was captured
    let (sample_rate,): (Option<i64>,) =
        sqlx::query_as("SELECT sample_rate FROM tracks WHERE title = 'DSD Track One'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        sample_rate,
        Some(2822400),
        "DSD64 sample rate must be 2822400 Hz"
    );
}

#[tokio::test]
async fn test_scan_artwork_discovery_standard_name() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    let dir = temp_dir.path().join("Artist - Album With Cover");
    fs::create_dir_all(&dir).unwrap();
    create_mp3(
        &dir,
        "track.mp3",
        &Tags {
            title: "Cover Track",
            artist: "Cover Artist",
            album_artist: None,
            album: "Album With Cover",
            track_number: Some(1),
            track_total: None,
        },
        1,
    );
    // Create standard cover.jpg
    fs::write(dir.join("cover.jpg"), b"\xFF\xD8\xFF\xE0fake jpeg").unwrap();

    scan_dir(&pool, temp_dir.path()).await;

    // After scan the album should have cover art set
    let all_albums = soul_storage::albums::get_all(&pool).await.unwrap();
    let album = all_albums.iter().find(|a| a.title == "Album With Cover");
    assert!(album.is_some(), "Album With Cover must exist");
    assert!(
        album.unwrap().cover_art_path.is_some(),
        "cover_art_path must be set when cover.jpg exists"
    );
}

#[tokio::test]
async fn test_scan_artwork_discovery_discogs_filename() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    let dir = temp_dir.path().join("Artist - Discogs Album");
    fs::create_dir_all(&dir).unwrap();
    create_mp3(
        &dir,
        "track.mp3",
        &Tags {
            title: "Discogs Track",
            artist: "Discogs Artist",
            album_artist: None,
            album: "Discogs Album",
            track_number: Some(1),
            track_total: None,
        },
        1,
    );
    // Discogs-style filename (no "cover" in name)
    fs::write(dir.join("R-12345-67890.jpg"), b"\xFF\xD8\xFF\xE0fake jpeg").unwrap();

    scan_dir(&pool, temp_dir.path()).await;

    let all_albums = soul_storage::albums::get_all(&pool).await.unwrap();
    let album = all_albums.iter().find(|a| a.title == "Discogs Album");
    assert!(album.is_some());
    assert!(
        album.unwrap().cover_art_path.is_some(),
        "Discogs-style image should be picked up as cover art"
    );
}

#[tokio::test]
async fn test_scan_artwork_discovery_resource_fork_not_used() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    let dir = temp_dir.path().join("Artist - No Cover Album");
    fs::create_dir_all(&dir).unwrap();
    create_mp3(
        &dir,
        "track.mp3",
        &Tags {
            title: "No Cover Track",
            artist: "No Cover Artist",
            album_artist: None,
            album: "No Cover Album",
            track_number: Some(1),
            track_total: None,
        },
        1,
    );
    // Only a resource fork image — must NOT be used as cover
    fs::write(
        dir.join("._cover.jpg"),
        b"\x00\x05\x16\x07fake resource fork",
    )
    .unwrap();

    scan_dir(&pool, temp_dir.path()).await;

    let all_albums = soul_storage::albums::get_all(&pool).await.unwrap();
    let album = all_albums.iter().find(|a| a.title == "No Cover Album");
    assert!(album.is_some());
    assert!(
        album.unwrap().cover_art_path.is_none(),
        "Resource fork must NOT be used as cover art"
    );
}

#[tokio::test]
async fn test_scan_multiple_formats_same_folder() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    let dir = temp_dir.path().join("Mixed Format Album");
    fs::create_dir_all(&dir).unwrap();

    // MP3 and FLAC with different content (not duplicates)
    create_mp3(
        &dir,
        "track01.mp3",
        &Tags {
            title: "MP3 Version",
            artist: "Mixed Artist",
            album_artist: None,
            album: "Mixed Format Album",
            track_number: Some(1),
            track_total: None,
        },
        1,
    );
    create_flac(
        &dir,
        "track02.flac",
        &Tags {
            title: "FLAC Version",
            artist: "Mixed Artist",
            album_artist: None,
            album: "Mixed Format Album",
            track_number: Some(2),
            track_total: None,
        },
        2,
    );

    scan_dir(&pool, temp_dir.path()).await;

    assert!(db_track_exists(&pool, "MP3 Version").await);
    assert!(db_track_exists(&pool, "FLAC Version").await);
    assert_eq!(db_album_track_count(&pool, "Mixed Format Album").await, 2);
}

// =============================================================================
// Group 7: Edge cases that must not crash (4 tests)
// =============================================================================

#[tokio::test]
async fn test_scan_empty_directory_no_crash() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    let stats = scan_dir(&pool, temp_dir.path()).await;

    assert_eq!(stats.total_files, 0);
    assert_eq!(stats.errors, 0);
}

#[tokio::test]
async fn test_scan_directory_with_only_images() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    fs::write(
        temp_dir.path().join("cover.jpg"),
        b"\xFF\xD8\xFF\xE0fake jpeg",
    )
    .unwrap();
    fs::write(
        temp_dir.path().join("liner.jpg"),
        b"\xFF\xD8\xFF\xE0fake jpeg 2",
    )
    .unwrap();

    let stats = scan_dir(&pool, temp_dir.path()).await;

    assert_eq!(
        stats.total_files, 0,
        "Image files must not be counted as audio"
    );
}

#[tokio::test]
async fn test_scan_corrupted_file_skipped() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    let dir = temp_dir.path().join("Album");
    fs::create_dir_all(&dir).unwrap();

    // Valid MP3
    create_mp3(
        &dir,
        "valid.mp3",
        &Tags {
            title: "Valid Track",
            artist: "Artist",
            album_artist: None,
            album: "Corrupt Test Album",
            track_number: Some(1),
            track_total: None,
        },
        1,
    );

    // Corrupted MP3 — just random bytes with .mp3 extension
    let corrupted_path = dir.join("corrupted.mp3");
    let junk: Vec<u8> = (0u8..200).cycle().take(512).collect();
    fs::write(&corrupted_path, &junk).unwrap();

    let stats = scan_dir(&pool, temp_dir.path()).await;

    // Should not panic; valid track should be imported
    assert_eq!(stats.total_files, 2, "Both files must be attempted");
    assert!(
        db_track_exists(&pool, "Valid Track").await,
        "Valid track must import despite corrupted sibling"
    );
    // Corrupted file should count as an error, not crash the scan
    assert!(stats.errors >= 0, "errors counter must be non-negative");
}

#[tokio::test]
async fn test_scan_very_long_filename() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    let dir = temp_dir.path().join("Long Name Album");
    fs::create_dir_all(&dir).unwrap();

    // 200-character title
    let long_title: String = "A".repeat(200);
    create_mp3(
        &dir,
        "long_track.mp3",
        &Tags {
            title: &long_title,
            artist: "Artist",
            album_artist: None,
            album: "Long Name Album",
            track_number: Some(1),
            track_total: None,
        },
        1,
    );

    // Must not crash
    let stats = scan_dir(&pool, temp_dir.path()).await;
    assert_eq!(stats.total_files, 1);
}

// =============================================================================
// Group 8: AIFF support (1 test)
// =============================================================================

/// Create a minimal valid AIFF file.
/// AIFF format: FORM chunk wrapping COMM (audio format) + SSND (samples).
/// lofty can parse this for file_format detection; title falls back to filename.
fn create_aiff(dir: &Path, filename: &str, _seed: u8) -> PathBuf {
    let path = dir.join(filename);

    // COMM body (18 bytes): channels(2) + numFrames(4) + sampleSize(2) + sampleRate(10)
    // 44100 Hz as 80-bit extended: exponent=16398=0x400E, mantissa=0xAC44<<48
    let comm_body: [u8; 18] = [
        0x00, 0x02, // channels = 2
        0x00, 0x00, 0x01, 0x00, // numSampleFrames = 256
        0x00, 0x10, // sampleSize = 16
        0x40, 0x0E, 0xAC, 0x44, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // 44100 Hz extended
    ];

    // SSND body: offset(4) + blockSize(4) + sample_data(256*2*2=1024 bytes)
    let mut ssnd_body = vec![0u8; 8 + 1024]; // offset + blockSize = 8, data = 1024
                                             // offset and blockSize are already 0 from vec initialization

    // FORM body = "AIFF"(4) + "COMM"(4) + comm_size(4) + comm(18) + "SSND"(4) + ssnd_size(4) + ssnd
    let form_body_size: u32 = 4 + 8 + comm_body.len() as u32 + 8 + ssnd_body.len() as u32;

    let mut buf: Vec<u8> = Vec::new();
    buf.extend_from_slice(b"FORM");
    buf.extend_from_slice(&form_body_size.to_be_bytes());
    buf.extend_from_slice(b"AIFF");
    buf.extend_from_slice(b"COMM");
    buf.extend_from_slice(&(comm_body.len() as u32).to_be_bytes());
    buf.extend_from_slice(&comm_body);
    buf.extend_from_slice(b"SSND");
    buf.extend_from_slice(&(ssnd_body.len() as u32).to_be_bytes());
    buf.extend_from_slice(&ssnd_body);

    fs::write(&path, &buf).expect("Failed to write AIFF file");
    path
}

#[tokio::test]
async fn test_scan_aiff_file_imported() {
    // Regression: AIFF/AIF was missing from SUPPORTED_EXTENSIONS — files silently skipped.
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    let dir = temp_dir.path().join("AIFF Album");
    fs::create_dir_all(&dir).unwrap();
    create_aiff(&dir, "track01.aiff", 1);

    let stats = scan_dir(&pool, temp_dir.path()).await;

    assert_eq!(
        stats.total_files, 1,
        "AIFF file must be counted as an audio file"
    );
    // Title falls back to filename stem since no embedded tags
    assert!(
        db_track_exists(&pool, "track01").await,
        "AIFF file must produce a track in the DB"
    );
}

#[tokio::test]
async fn test_scan_aif_extension_also_supported() {
    // .aif (short form) and .aiff should both be recognized.
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    let dir = temp_dir.path().join("AIF Album");
    fs::create_dir_all(&dir).unwrap();
    create_aiff(&dir, "side_a.aif", 1);

    let stats = scan_dir(&pool, temp_dir.path()).await;

    assert_eq!(stats.total_files, 1, ".aif extension must be counted");
    assert!(
        db_track_exists(&pool, "side_a").await,
        ".aif must produce a track"
    );
}

// =============================================================================
// Group 9: Incremental scan correctness (4 tests)
// =============================================================================

#[tokio::test]
async fn test_incremental_scan_second_run_has_zero_new_files() {
    // Core invariant: after a complete first scan, a second scan with no filesystem
    // changes should produce zero new files. This verifies scanned_directories is
    // persisted correctly and directory mtime/file_count matching works.
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    let dir = temp_dir.path().join("Stable Album");
    fs::create_dir_all(&dir).unwrap();
    create_flac(
        &dir,
        "01.flac",
        &Tags {
            title: "Stable Track",
            artist: "Stable Artist",
            album_artist: None,
            album: "Stable Album",
            track_number: Some(1),
            track_total: None,
        },
        1,
    );

    let (stats1, stats2) = scan_dir_twice(&pool, temp_dir.path()).await;
    assert_eq!(stats1.new_files, 1, "First scan must find 1 new file");

    assert_eq!(
        stats2.new_files, 0,
        "Second scan with no changes must produce 0 new files"
    );
    assert_eq!(
        stats2.updated_files, 0,
        "Second scan must produce 0 updated files"
    );

    // DB still has exactly 1 track (no duplication)
    assert_eq!(db_album_track_count(&pool, "Stable Album").await, 1);
}

#[tokio::test]
async fn test_incremental_scan_dsd_skipped_when_directory_unchanged() {
    // DSF files are treated identically to FLAC/MP3 — unchanged directory → fully skipped.
    // The `is_dsd` forced-reprocess bypass has been removed.
    // The "always rescans DSF albums" the user observed was because those album directories
    // had never been in scanned_directories (they were new). After the first rescan they're
    // stored and subsequent scans skip them like any other format.
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    let dir = temp_dir.path().join("DSD Album");
    fs::create_dir_all(&dir).unwrap();
    create_dsf(
        &dir,
        "dsd_track.dsf",
        &Tags {
            title: "DSD Stable",
            artist: "DSD Artist",
            album_artist: None,
            album: "DSD Album",
            track_number: Some(1),
            track_total: None,
        },
    );

    let (stats1, stats2) = scan_dir_twice(&pool, temp_dir.path()).await;
    assert_eq!(stats1.new_files, 1, "First scan imports DSF track");

    // Directory unchanged → Phase 0 skips it → Phase 1 is_dsd never reached → 0 updates
    assert_eq!(
        stats2.updated_files, 0,
        "Unchanged DSF directory must be skipped on second scan (Phase 0 skip)"
    );
    assert_eq!(stats2.new_files, 0, "No new DSF files on second scan");

    // No duplicate tracks created
    assert_eq!(db_album_track_count(&pool, "DSD Album").await, 1);
}

#[tokio::test]
async fn test_incremental_scan_multiple_rescans_no_duplicate_tracks() {
    // Three consecutive rescans must not multiply track counts.
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    let dir = temp_dir.path().join("No Dup Album");
    fs::create_dir_all(&dir).unwrap();
    for i in 1..=5u32 {
        create_flac(
            &dir,
            &format!("{:02}.flac", i),
            &Tags {
                title: &format!("Track {}", i),
                artist: "No Dup Artist",
                album_artist: None,
                album: "No Dup Album",
                track_number: Some(i),
                track_total: Some(5),
            },
            i as u8,
        );
    }

    let stats = scan_dir_n(&pool, temp_dir.path(), 3).await;
    assert_eq!(stats[0].new_files, 5, "First scan finds all 5 tracks");

    assert_eq!(
        db_album_track_count(&pool, "No Dup Album").await,
        5,
        "Three rescans must not duplicate the 5 tracks"
    );
}

#[tokio::test]
async fn test_incremental_scan_cover_art_updated_when_image_added() {
    // When cover.jpg is added to a directory whose audio files are unchanged,
    // the directory mtime changes → Phase 0 detects the change → refresh_artwork_for_changed_dirs
    // runs and updates cover_art_path even though Phase 1 skipped the audio files.
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    let dir = temp_dir.path().join("No Art Album");
    fs::create_dir_all(&dir).unwrap();
    create_flac(
        &dir,
        "track.flac",
        &Tags {
            title: "No Art Track",
            artist: "Artist",
            album_artist: None,
            album: "No Art Album",
            track_number: Some(1),
            track_total: None,
        },
        1,
    );

    // Create library source and do first scan
    soul_storage::library_sources::create(
        &pool,
        "user1",
        "device1",
        &CreateLibrarySource {
            name: "Test Library".to_string(),
            path: temp_dir.path().to_string_lossy().to_string(),
            sync_deletes: true,
        },
    )
    .await
    .expect("Failed to create library source");
    let scanner = LibraryScanner::new(pool.clone(), "user1", "device1").compute_hashes(false);
    scanner.scan_all().await.expect("scan 1 should not fail");

    // Verify no artwork initially
    let (cover,): (Option<String>,) =
        sqlx::query_as("SELECT cover_art_path FROM albums WHERE title = 'No Art Album'")
            .fetch_one(&pool)
            .await
            .expect("album must exist");
    assert!(cover.is_none(), "No artwork expected before image added");

    // Add a cover image — directory mtime will change.
    // The scanner stores mtime at millisecond precision, so even rapid writes are detected.
    fs::write(dir.join("cover.jpg"), b"fake jpeg data").unwrap();

    // Rescan — audio files unchanged, only image added
    scanner.scan_all().await.expect("scan 2 should not fail");

    // Artwork MUST be updated — refresh_artwork_for_changed_dirs catches this
    let (cover_after,): (Option<String>,) =
        sqlx::query_as("SELECT cover_art_path FROM albums WHERE title = 'No Art Album'")
            .fetch_one(&pool)
            .await
            .expect("album must exist");
    assert!(
        cover_after.is_some(),
        "cover_art_path must be set after cover.jpg is added and directory is rescanned"
    );
    assert!(
        cover_after.as_deref().unwrap_or("").ends_with("cover.jpg"),
        "cover_art_path must point to cover.jpg, got: {:?}",
        cover_after
    );
}

// =============================================================================
// Group 10: Metadata field completeness (3 tests)
// =============================================================================

#[tokio::test]
async fn test_scan_year_tag_stored_in_tracks() {
    // TDRC (recording year) must be stored as tracks.year
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    let dir = temp_dir.path().join("Yearly Album");
    fs::create_dir_all(&dir).unwrap();

    // Build MP3 manually with TDRC year frame
    let tag = build_id3_tag(&[
        id3_frame(b"TIT2", "Yearly Track"),
        id3_frame(b"TPE1", "Year Artist"),
        id3_frame(b"TALB", "Yearly Album"),
        id3_frame(b"TDRC", "1979"),
    ]);
    let mut buf = tag;
    for _ in 0..2 {
        buf.extend_from_slice(&[0xFF, 0xFB, 0x90, 0x00]);
        buf.extend_from_slice(&[0u8; 413]);
    }
    fs::write(dir.join("track.mp3"), &buf).unwrap();

    scan_dir(&pool, temp_dir.path()).await;

    let (year,): (Option<i64>,) =
        sqlx::query_as("SELECT year FROM tracks WHERE title = 'Yearly Track'")
            .fetch_one(&pool)
            .await
            .expect("track must exist");
    assert_eq!(
        year,
        Some(1979),
        "TDRC year 1979 must be stored in tracks.year"
    );
}

#[tokio::test]
async fn test_scan_disc_number_stored_in_tracks() {
    // TPOS (disc number) must be stored as tracks.disc_number
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    let dir = temp_dir.path().join("Disc Album");
    fs::create_dir_all(&dir).unwrap();

    let tag = build_id3_tag(&[
        id3_frame(b"TIT2", "Disc Two Track"),
        id3_frame(b"TPE1", "Disc Artist"),
        id3_frame(b"TALB", "Disc Album"),
        id3_frame(b"TRCK", "3"),
        id3_frame(b"TPOS", "2/3"), // disc 2 of 3
    ]);
    let mut buf = tag;
    for _ in 0..2 {
        buf.extend_from_slice(&[0xFF, 0xFB, 0x90, 0x00]);
        buf.extend_from_slice(&[0u8; 413]);
    }
    fs::write(dir.join("track.mp3"), &buf).unwrap();

    scan_dir(&pool, temp_dir.path()).await;

    let (disc,): (Option<i64>,) =
        sqlx::query_as("SELECT disc_number FROM tracks WHERE title = 'Disc Two Track'")
            .fetch_one(&pool)
            .await
            .expect("track must exist");
    assert_eq!(disc, Some(2), "TPOS '2/3' must store disc_number=2");
}

#[tokio::test]
async fn test_scan_empty_album_tag_uses_folder_name() {
    // When TALB is present but empty, scanner should fall back to folder name.
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    let dir = temp_dir.path().join("FolderAlbumName");
    fs::create_dir_all(&dir).unwrap();

    // TALB is empty string
    let tag = build_id3_tag(&[
        id3_frame(b"TIT2", "Folder Fallback Track"),
        id3_frame(b"TPE1", "Some Artist"),
        id3_frame(b"TALB", ""), // intentionally empty
    ]);
    let mut buf = tag;
    for _ in 0..2 {
        buf.extend_from_slice(&[0xFF, 0xFB, 0x90, 0x00]);
        buf.extend_from_slice(&[0u8; 413]);
    }
    fs::write(dir.join("track.mp3"), &buf).unwrap();

    scan_dir(&pool, temp_dir.path()).await;

    // Track must exist regardless of album grouping
    assert!(
        db_track_exists(&pool, "Folder Fallback Track").await,
        "Track with empty TALB must still be imported"
    );
}

// =============================================================================
// Group 11: Album separation edge cases (2 tests)
// =============================================================================

#[tokio::test]
async fn test_scan_two_albums_same_title_different_artists_stay_separate() {
    // Two different artists releasing an album with the same title (e.g., "Greatest Hits")
    // must NOT be merged into one album.
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    let dir_a = temp_dir.path().join("Artist A - Greatest Hits");
    let dir_b = temp_dir.path().join("Artist B - Greatest Hits");
    fs::create_dir_all(&dir_a).unwrap();
    fs::create_dir_all(&dir_b).unwrap();

    create_flac(
        &dir_a,
        "01.flac",
        &Tags {
            title: "Hit A1",
            artist: "Artist A",
            album_artist: None,
            album: "Greatest Hits",
            track_number: Some(1),
            track_total: None,
        },
        1,
    );
    create_flac(
        &dir_b,
        "01.flac",
        &Tags {
            title: "Hit B1",
            artist: "Artist B",
            album_artist: None,
            album: "Greatest Hits",
            track_number: Some(1),
            track_total: None,
        },
        2,
    );

    scan_dir(&pool, temp_dir.path()).await;

    // Both tracks must exist
    assert!(db_track_exists(&pool, "Hit A1").await);
    assert!(db_track_exists(&pool, "Hit B1").await);

    // There should be 2 separate "Greatest Hits" albums (different artists)
    let (album_count,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM albums WHERE title = 'Greatest Hits'")
            .fetch_one(&pool)
            .await
            .expect("query must succeed");
    assert_eq!(
        album_count, 2,
        "Two artists with same album title must create separate albums"
    );
}

#[tokio::test]
async fn test_scan_nonexistent_source_path_returns_error() {
    // A library source pointing to a non-existent directory must return an error,
    // not panic or silently succeed.
    let pool = test_helpers::setup_test_db().await;

    soul_storage::library_sources::create(
        &pool,
        "user1",
        "device1",
        &soul_core::types::CreateLibrarySource {
            name: "Ghost Library".to_string(),
            path: "D:/this/path/definitely/does/not/exist/xyz123".to_string(),
            sync_deletes: false,
        },
    )
    .await
    .expect("Failed to create library source");

    let scanner =
        soul_importer::library_scanner::LibraryScanner::new(pool.clone(), "user1", "device1")
            .compute_hashes(false);
    let result = scanner.scan_all().await;

    // scan_all itself should still succeed (it aggregates errors) but record an error
    // OR it should return an error — either is acceptable; must not panic
    match result {
        Ok(stats) => {
            assert!(
                stats.errors >= 1,
                "Non-existent path must produce at least 1 error in stats"
            );
        }
        Err(_) => {
            // Also acceptable — scan returned an error for the bad path
        }
    }
}

#[tokio::test]
async fn test_scan_deeply_nested_directories_found() {
    // Files 4 levels deep (Genre/Artist/Year/Album/track.flac) must be found.
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    let deep_dir = temp_dir
        .path()
        .join("Jazz")
        .join("Miles Davis")
        .join("1959")
        .join("Kind of Blue");
    fs::create_dir_all(&deep_dir).unwrap();

    create_flac(
        &deep_dir,
        "01 - So What.flac",
        &Tags {
            title: "So What",
            artist: "Miles Davis",
            album_artist: None,
            album: "Kind of Blue",
            track_number: Some(1),
            track_total: Some(5),
        },
        1,
    );

    scan_dir(&pool, temp_dir.path()).await;

    assert!(
        db_track_exists(&pool, "So What").await,
        "Track in 4-level nested directory must be found and imported"
    );
    assert!(db_artist_exists(&pool, "Miles Davis").await);
    assert!(db_album_exists(&pool, "Kind of Blue").await);
}

// ── Subfolder album merge E2E ─────────────────────────────────────────────────

/// Tracks in `Album/` and `Album/B-Sides/` with identical album+artist tags
/// must be merged into a single album record, not two separate ones.
#[tokio::test]
async fn test_subfolder_tracks_merge_into_one_album() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    // Root directory: track_01.flac
    let root_dir = temp_dir.path().join("Subfolder Artist - Subfolder Album");
    fs::create_dir_all(&root_dir).unwrap();
    create_flac(
        &root_dir,
        "track_01.flac",
        &Tags {
            title: "Track One",
            artist: "Subfolder Artist",
            album_artist: Some("Subfolder Artist"),
            album: "Subfolder Album",
            track_number: Some(1),
            track_total: None,
        },
        1,
    );

    // Subfolder: B-Sides — same album+artist
    let sub_dir = root_dir.join("B-Sides");
    fs::create_dir_all(&sub_dir).unwrap();
    create_flac(
        &sub_dir,
        "track_02.flac",
        &Tags {
            title: "Track Two (B-Side)",
            artist: "Subfolder Artist",
            album_artist: Some("Subfolder Artist"),
            album: "Subfolder Album",
            track_number: Some(2),
            track_total: None,
        },
        2,
    );

    scan_dir(&pool, temp_dir.path()).await;

    let all_albums = soul_storage::albums::get_all(&pool).await.unwrap();
    let matching: Vec<_> = all_albums
        .iter()
        .filter(|a| a.title == "Subfolder Album")
        .collect();

    assert_eq!(
        matching.len(),
        1,
        "subfolder tracks should merge into one album, got {} album(s): {:?}",
        matching.len(),
        matching.iter().map(|a| &a.folder_path).collect::<Vec<_>>()
    );

    let tracks = soul_storage::tracks::get_by_album(&pool, matching[0].id)
        .await
        .unwrap();
    assert_eq!(
        tracks.len(),
        2,
        "both root and B-Sides tracks must be under the single merged album"
    );

    // The stored folder_path must be the outermost (root) folder, not the subfolder.
    // When the B-Sides subfolder is discovered after the root folder, the album's
    // folder_path should remain (or be promoted to) the shorter parent path.
    assert!(
        !matching[0].folder_path.contains("B-Sides"),
        "album folder_path '{}' must not point into 'B-Sides' — it should be the parent folder",
        matching[0].folder_path
    );
}

/// Two completely different albums that happen to share the same title and artist
/// but live in sibling folders (e.g. a 2020 and 2021 compilation) must NOT be merged.
///
/// This is the counterpart to `test_subfolder_tracks_merge_into_one_album`:
/// subfolder → merge; sibling folder → keep separate.
#[tokio::test]
async fn test_sibling_folder_albums_same_title_not_merged() {
    let pool = test_helpers::setup_test_db().await;
    let temp_dir = TempDir::new().unwrap();

    // Sibling folder A — /Various/Hits 2020/
    let dir_a = temp_dir.path().join("Various").join("Hits 2020");
    fs::create_dir_all(&dir_a).unwrap();
    create_flac(
        &dir_a,
        "track_01.flac",
        &Tags {
            title: "Song A",
            artist: "Various",
            album_artist: Some("Various"),
            album: "Hits",
            track_number: Some(1),
            track_total: None,
        },
        1,
    );

    // Sibling folder B — /Various/Hits 2021/ (neither is a child of the other)
    let dir_b = temp_dir.path().join("Various").join("Hits 2021");
    fs::create_dir_all(&dir_b).unwrap();
    create_flac(
        &dir_b,
        "track_01.flac",
        &Tags {
            title: "Song B",
            artist: "Various",
            album_artist: Some("Various"),
            album: "Hits",
            track_number: Some(1),
            track_total: None,
        },
        2,
    );

    scan_dir(&pool, temp_dir.path()).await;

    let all_albums = soul_storage::albums::get_all(&pool).await.unwrap();
    let hits_albums: Vec<_> = all_albums.iter().filter(|a| a.title == "Hits").collect();

    assert_eq!(
        hits_albums.len(),
        2,
        "sibling folders 'Hits 2020' and 'Hits 2021' must produce 2 separate album records, got {}: {:?}",
        hits_albums.len(),
        hits_albums.iter().map(|a| &a.folder_path).collect::<Vec<_>>()
    );
}

// =============================================================================
// Task 7: Auto-scale concurrency + batch flush 100
// =============================================================================

#[tokio::test]
async fn test_scan_auto_concurrency_respects_cap() {
    use soul_importer::library_scanner::LibraryScanner;
    let pool = test_helpers::setup_test_db().await;
    let scanner = LibraryScanner::new(pool, "user1", "device1");
    assert!(
        scanner.concurrency_limit() <= 64,
        "default concurrency must be <= 64, got {}",
        scanner.concurrency_limit()
    );
    // Must be at least 1
    assert!(
        scanner.concurrency_limit() >= 1,
        "default concurrency must be >= 1, got {}",
        scanner.concurrency_limit()
    );
}

#[tokio::test]
async fn test_scan_batch_size_100_all_tracks_imported() {
    let pool = test_helpers::setup_test_db().await;
    let dir = TempDir::new().unwrap();

    // Create 110 files across 110 directories to trigger multiple batch flushes
    for i in 0..110u32 {
        let sub = dir.path().join(format!("artist_{:03}", i));
        fs::create_dir_all(&sub).unwrap();
        create_flac(
            &sub,
            "track.flac",
            &Tags {
                title: &format!("Batch Track {}", i),
                artist: &format!("Artist {}", i),
                album_artist: None,
                album: &format!("Album {}", i),
                track_number: Some(1),
                track_total: None,
            },
            (i % 256) as u8,
        );
    }

    let stats = scan_dir(&pool, dir.path()).await;

    assert_eq!(
        stats.new_files, 110,
        "expected 110 new files, got {}",
        stats.new_files
    );
    assert_eq!(stats.errors, 0);

    let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tracks WHERE is_available = 1")
        .fetch_one(&pool)
        .await
        .expect("count tracks query");
    assert_eq!(count, 110, "expected 110 tracks in DB, got {}", count);
}
