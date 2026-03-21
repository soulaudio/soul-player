// libraries/soul-importer/tests/bwf_metadata_tests.rs
//! Unit tests for BWF WAV metadata extraction (extract_wav_metadata).

use soul_importer::metadata::extract_metadata;
use std::io::Write;
use tempfile::NamedTempFile;

// ── Helper: build a minimal RIFF/WAV file in memory ──────────────────────────

fn riff_chunk(id: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(8 + data.len());
    out.extend_from_slice(id);
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend_from_slice(data);
    // RIFF chunks must be word-aligned (even size)
    if data.len() % 2 != 0 {
        out.push(0);
    }
    out
}

/// Build a fmt chunk: PCM, stereo, 44100 Hz, 16-bit.
fn fmt_chunk(sample_rate: u32, channels: u16, bits_per_sample: u16) -> Vec<u8> {
    let mut data = Vec::with_capacity(16);
    data.extend_from_slice(&1u16.to_le_bytes()); // AudioFormat = PCM
    data.extend_from_slice(&channels.to_le_bytes());
    data.extend_from_slice(&sample_rate.to_le_bytes());
    let byte_rate = sample_rate * channels as u32 * bits_per_sample as u32 / 8;
    data.extend_from_slice(&byte_rate.to_le_bytes());
    let block_align = channels * bits_per_sample / 8;
    data.extend_from_slice(&block_align.to_le_bytes());
    data.extend_from_slice(&bits_per_sample.to_le_bytes());
    data
}

/// Build an ID3v2.3 text frame with UTF-16 LE encoding (encoding byte = 0x01, BOM included).
fn id3_frame_utf16(id: &[u8; 4], text: &str) -> Vec<u8> {
    let mut payload: Vec<u8> = vec![0x01]; // encoding = UTF-16
                                           // Write BOM (LE)
    payload.extend_from_slice(&[0xFF, 0xFE]);
    // Encode as UTF-16 LE
    for c in text.encode_utf16() {
        payload.extend_from_slice(&c.to_le_bytes());
    }
    // Null terminator
    payload.extend_from_slice(&[0x00, 0x00]);
    let size = payload.len() as u32;
    let mut frame = Vec::new();
    frame.extend_from_slice(id);
    frame.extend_from_slice(&size.to_be_bytes());
    frame.extend_from_slice(&[0u8, 0u8]); // flags
    frame.extend_from_slice(&payload);
    frame
}

/// Build a minimal ID3v2.3 tag with UTF-16 encoded title and artist.
fn minimal_id3_utf16(title: &str, artist: &str) -> Vec<u8> {
    let mut frames = id3_frame_utf16(b"TIT2", title);
    frames.extend(id3_frame_utf16(b"TPE1", artist));
    let tag_size = frames.len() as u32;
    let s0 = ((tag_size >> 21) & 0x7F) as u8;
    let s1 = ((tag_size >> 14) & 0x7F) as u8;
    let s2 = ((tag_size >> 7) & 0x7F) as u8;
    let s3 = (tag_size & 0x7F) as u8;
    let mut tag = Vec::new();
    tag.extend_from_slice(b"ID3");
    tag.push(3);
    tag.push(0);
    tag.push(0);
    tag.extend_from_slice(&[s0, s1, s2, s3]);
    tag.extend_from_slice(&frames);
    tag
}

/// Build a minimal ID3v2.3 tag containing title and artist.
fn minimal_id3(title: &str, artist: &str) -> Vec<u8> {
    fn id3_frame(id: &[u8; 4], text: &str) -> Vec<u8> {
        let mut payload = vec![0u8]; // encoding = Latin-1
        payload.extend_from_slice(text.as_bytes());
        let size = payload.len() as u32;
        let mut frame = Vec::new();
        frame.extend_from_slice(id);
        frame.extend_from_slice(&size.to_be_bytes());
        frame.extend_from_slice(&[0u8, 0u8]); // flags
        frame.extend_from_slice(&payload);
        frame
    }

    let mut frames = id3_frame(b"TIT2", title);
    frames.extend(id3_frame(b"TPE1", artist));

    let tag_size = frames.len() as u32;
    // ID3v2.3 syncsafe size encoding
    let s0 = ((tag_size >> 21) & 0x7F) as u8;
    let s1 = ((tag_size >> 14) & 0x7F) as u8;
    let s2 = ((tag_size >> 7) & 0x7F) as u8;
    let s3 = (tag_size & 0x7F) as u8;

    let mut tag = Vec::new();
    tag.extend_from_slice(b"ID3");
    tag.push(3); // version 2.3
    tag.push(0); // revision
    tag.push(0); // flags
    tag.extend_from_slice(&[s0, s1, s2, s3]);
    tag.extend_from_slice(&frames);
    tag
}

/// Build a complete BWF WAV with optional bext chunk and optional id3 chunk.
fn build_wav(
    sample_rate: u32,
    channels: u16,
    bits_per_sample: u16,
    data_samples: u32,
    include_bext: bool,
    id3_title: Option<&str>,
    id3_artist: Option<&str>,
) -> Vec<u8> {
    let fmt = riff_chunk(b"fmt ", &fmt_chunk(sample_rate, channels, bits_per_sample));
    let bext = if include_bext {
        // Minimal bext: 602-byte body (all zeros)
        riff_chunk(b"bext", &vec![0u8; 602])
    } else {
        vec![]
    };
    let data_bytes = data_samples * channels as u32 * bits_per_sample as u32 / 8;
    let data = riff_chunk(b"data", &vec![0u8; data_bytes as usize]);

    let mut id3_chunk = vec![];
    if let (Some(title), Some(artist)) = (id3_title, id3_artist) {
        id3_chunk = riff_chunk(b"id3 ", &minimal_id3(title, artist));
    }

    let mut riff_body = Vec::new();
    riff_body.extend_from_slice(b"WAVE");
    riff_body.extend_from_slice(&fmt);
    riff_body.extend_from_slice(&bext);
    riff_body.extend_from_slice(&data);
    riff_body.extend_from_slice(&id3_chunk);

    let mut wav = Vec::new();
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(riff_body.len() as u32).to_le_bytes());
    wav.extend_from_slice(&riff_body);
    wav
}

fn write_wav_file(bytes: &[u8]) -> NamedTempFile {
    let mut f = tempfile::Builder::new().suffix(".wav").tempfile().unwrap();
    f.write_all(bytes).unwrap();
    f.flush().unwrap();
    f
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[test]
fn bwf_wav_duration_is_correct() {
    // 44100 Hz, 2ch, 16-bit, 2s → data_bytes / (sr * ch * bps/8) = 2s
    let bytes = build_wav(44100, 2, 16, 44100 * 2, true, None, None);
    let f = write_wav_file(&bytes);
    let meta = extract_metadata(f.path()).unwrap();
    let dur = meta.duration_seconds.expect("duration should be Some");
    assert!((dur - 2.0).abs() < 0.01, "expected ~2s, got {}", dur);
}

#[test]
fn bwf_wav_sample_rate_is_correct() {
    let bytes = build_wav(96000, 2, 24, 96000, true, None, None);
    let f = write_wav_file(&bytes);
    let meta = extract_metadata(f.path()).unwrap();
    assert_eq!(meta.sample_rate, Some(96000));
}

#[test]
fn bwf_wav_channels_is_correct() {
    let bytes = build_wav(44100, 6, 16, 44100, true, None, None);
    let f = write_wav_file(&bytes);
    let meta = extract_metadata(f.path()).unwrap();
    assert_eq!(meta.channels, Some(6));
}

#[test]
fn bwf_wav_id3_title_populated() {
    let bytes = build_wav(
        44100,
        2,
        16,
        44100,
        true,
        Some("My Title"),
        Some("Some Artist"),
    );
    let f = write_wav_file(&bytes);
    let meta = extract_metadata(f.path()).unwrap();
    assert_eq!(meta.title.as_deref(), Some("My Title"));
}

#[test]
fn bwf_wav_id3_artist_populated() {
    let bytes = build_wav(
        44100,
        2,
        16,
        44100,
        true,
        Some("My Title"),
        Some("Some Artist"),
    );
    let f = write_wav_file(&bytes);
    let meta = extract_metadata(f.path()).unwrap();
    // ExtractedMetadata has `artists: Vec<String>` (already split from the raw tag)
    assert!(
        meta.artists.iter().any(|a| a == "Some Artist"),
        "expected 'Some Artist' in artists, got {:?}",
        meta.artists
    );
}

#[test]
fn bwf_wav_no_id3_chunk_returns_ok_with_no_title() {
    let bytes = build_wav(44100, 2, 16, 44100, true, None, None);
    let f = write_wav_file(&bytes);
    let meta = extract_metadata(f.path()).unwrap();
    assert!(meta.title.is_none());
    assert!(
        meta.duration_seconds.is_some(),
        "duration must still be present"
    );
}

#[test]
fn bwf_wav_standard_wav_still_works_via_lofty() {
    // A standard WAV with no bext chunk should still parse (via our parser or lofty fallback).
    let bytes = build_wav(44100, 2, 16, 44100, false, None, None);
    let f = write_wav_file(&bytes);
    let meta = extract_metadata(f.path()).unwrap();
    assert!(meta.duration_seconds.is_some());
}

#[test]
fn bwf_wav_truncated_fmt_returns_error() {
    // Craft a WAV with a fmt chunk that's only 4 bytes (too short — valid fmt is 16+)
    let mut truncated_fmt = Vec::new();
    truncated_fmt.extend_from_slice(b"fmt ");
    truncated_fmt.extend_from_slice(&4u32.to_le_bytes()); // chunk size = 4
    truncated_fmt.extend_from_slice(&[0u8; 4]); // 4 bytes of data

    let data = riff_chunk(b"data", &[0u8; 100]);

    let mut riff_body = b"WAVE".to_vec();
    riff_body.extend(truncated_fmt);
    riff_body.extend(data);

    let mut wav = b"RIFF".to_vec();
    wav.extend_from_slice(&(riff_body.len() as u32).to_le_bytes());
    wav.extend(riff_body);

    let f = write_wav_file(&wav);
    // Should return an error (not panic)
    let result = extract_metadata(f.path());
    assert!(
        result.is_err(),
        "truncated fmt should return Err, got {:?}",
        result
    );
}

#[test]
fn bwf_wav_utf16_id3_tags_decoded_correctly() {
    // Osaki Seiichi uses UTF-16 LE ID3v2 tags inside BWF WAV files.
    // Previously these were read as raw bytes → garbled null-padded strings.
    let title = "Moving From The Beaming Sun";
    let artist = "Osaki Seiichi";

    // Build a WAV with UTF-16 encoded ID3 tags
    let fmt = riff_chunk(b"fmt ", &fmt_chunk(44100, 2, 16));
    let data = riff_chunk(b"data", &vec![0u8; 44100 * 2 * 2]); // 1 second
    let id3 = riff_chunk(b"id3 ", &minimal_id3_utf16(title, artist));

    let mut riff_body = Vec::new();
    riff_body.extend_from_slice(b"WAVE");
    riff_body.extend_from_slice(&fmt);
    riff_body.extend_from_slice(&data);
    riff_body.extend_from_slice(&id3);

    let mut wav = b"RIFF".to_vec();
    wav.extend_from_slice(&(riff_body.len() as u32).to_le_bytes());
    wav.extend_from_slice(&riff_body);

    let f = write_wav_file(&wav);
    let meta = extract_metadata(f.path()).unwrap();

    assert_eq!(
        meta.title.as_deref(),
        Some(title),
        "UTF-16 title should decode correctly, got {:?}",
        meta.title
    );
    assert!(
        meta.artists.iter().any(|a| a == artist),
        "UTF-16 artist should decode correctly, got {:?}",
        meta.artists
    );
}

#[test]
fn bwf_wav_latin1_id3_tags_decoded_correctly() {
    // Files tagged by older rippers (EAC, dBpoweramp default) use ISO-8859-1
    // for accented characters. Previously from_utf8_lossy turned bytes 0x80–0xFF
    // into U+FFFD; now they decode to their correct Unicode equivalents.
    //
    // "Gymnopédies" in Latin-1: the é = 0xE9
    // "Erik Satie"  — pure ASCII, used as the artist
    fn id3_frame_latin1(id: &[u8; 4], text_bytes: &[u8]) -> Vec<u8> {
        let mut payload = vec![0x00u8]; // encoding = ISO-8859-1
        payload.extend_from_slice(text_bytes);
        let size = payload.len() as u32;
        let mut frame = Vec::new();
        frame.extend_from_slice(id);
        frame.extend_from_slice(&size.to_be_bytes());
        frame.extend_from_slice(&[0u8, 0u8]); // flags
        frame.extend_from_slice(&payload);
        frame
    }

    // "Gymnopédies" with é as Latin-1 byte 0xE9
    let title_latin1: &[u8] = b"Gymnop\xe9dies";
    let artist_latin1: &[u8] = b"Erik Satie";

    let mut frames = id3_frame_latin1(b"TIT2", title_latin1);
    frames.extend(id3_frame_latin1(b"TPE1", artist_latin1));

    let tag_size = frames.len() as u32;
    let s0 = ((tag_size >> 21) & 0x7F) as u8;
    let s1 = ((tag_size >> 14) & 0x7F) as u8;
    let s2 = ((tag_size >> 7) & 0x7F) as u8;
    let s3 = (tag_size & 0x7F) as u8;
    let mut id3_tag = Vec::new();
    id3_tag.extend_from_slice(b"ID3");
    id3_tag.push(3);
    id3_tag.push(0);
    id3_tag.push(0);
    id3_tag.extend_from_slice(&[s0, s1, s2, s3]);
    id3_tag.extend_from_slice(&frames);

    let fmt = riff_chunk(b"fmt ", &fmt_chunk(44100, 2, 16));
    let data = riff_chunk(b"data", &vec![0u8; 44100 * 2 * 2]);
    let id3_chunk = riff_chunk(b"id3 ", &id3_tag);

    let mut riff_body = Vec::new();
    riff_body.extend_from_slice(b"WAVE");
    riff_body.extend_from_slice(&fmt);
    riff_body.extend_from_slice(&data);
    riff_body.extend_from_slice(&id3_chunk);

    let mut wav = b"RIFF".to_vec();
    wav.extend_from_slice(&(riff_body.len() as u32).to_le_bytes());
    wav.extend_from_slice(&riff_body);

    let f = write_wav_file(&wav);
    let meta = extract_metadata(f.path()).unwrap();

    assert_eq!(
        meta.title.as_deref(),
        Some("Gymnopédies"),
        "Latin-1 title with accented é should decode correctly, got {:?}",
        meta.title
    );
    assert!(
        meta.artists.iter().any(|a| a == "Erik Satie"),
        "Latin-1 artist should decode correctly, got {:?}",
        meta.artists
    );
}
