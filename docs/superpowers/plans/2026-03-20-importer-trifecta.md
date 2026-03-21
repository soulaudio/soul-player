# Importer Trifecta Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix BWF WAV null-duration tracks, merge subfolder albums with the same metadata, and speed up large-library scans via parallel stat + auto-scaled workers + batched transactions.

**Architecture:** Three independent changes to `libraries/soul-importer`. BWF WAV adds a RIFF chunk walker in `metadata.rs` (mirrors `extract_dsd_metadata`). Subfolder merge adds a lookup step in `fuzzy.rs` between normalized-match and create-new. Scan perf touches `scanner.rs` (rayon Phase-0) and `library_scanner.rs` (concurrency + batch size).

**Tech Stack:** Rust, sqlx, tokio, rayon (new dep), criterion (already present), lofty (existing)

---

## Files Changed

| File | Change |
|---|---|
| `libraries/soul-importer/src/metadata.rs` | Add `extract_wav_metadata()`, call before lofty for `.wav`/`.wave` |
| `libraries/soul-importer/tests/bwf_metadata_tests.rs` | NEW — BWF WAV unit tests |
| `libraries/soul-importer/src/fuzzy.rs` | Add subfolder match step in `find_or_create_album()` and `find_or_create_album_cached()` |
| `libraries/soul-importer/tests/entity_cache_tests.rs` | Add subfolder merge tests |
| `libraries/soul-importer/tests/scanner_import_e2e_tests.rs` | Add subfolder E2E + scan perf tests |
| `libraries/soul-importer/src/scanner.rs` | Replace sequential stat loop with rayon::par_iter() |
| `libraries/soul-importer/src/library_scanner.rs` | Auto-scale concurrency, batch=100, transactional flush |
| `libraries/soul-importer/Cargo.toml` | Add `rayon` dependency |
| `libraries/soul-importer/benches/scan_benchmark.rs` | Add parallel-stat, concurrency-scale, and batch-write benchmarks |

---

## Task 1: BWF WAV — Failing Tests

**Files:**
- Create: `libraries/soul-importer/tests/bwf_metadata_tests.rs`

- [ ] **Step 1: Create the test file with helpers and failing tests**

```rust
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
    data.extend_from_slice(&1u16.to_le_bytes());           // AudioFormat = PCM
    data.extend_from_slice(&channels.to_le_bytes());
    data.extend_from_slice(&sample_rate.to_le_bytes());
    let byte_rate = sample_rate * channels as u32 * bits_per_sample as u32 / 8;
    data.extend_from_slice(&byte_rate.to_le_bytes());
    let block_align = channels * bits_per_sample / 8;
    data.extend_from_slice(&block_align.to_le_bytes());
    data.extend_from_slice(&bits_per_sample.to_le_bytes());
    data
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
    // 44100 Hz, 2ch, 16-bit, 2s → 44100 * 2 * 2 / (44100 * 2 * 2) = 2s
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
    let bytes = build_wav(44100, 2, 16, 44100, true, Some("My Title"), Some("Some Artist"));
    let f = write_wav_file(&bytes);
    let meta = extract_metadata(f.path()).unwrap();
    assert_eq!(meta.title.as_deref(), Some("My Title"));
}

#[test]
fn bwf_wav_id3_artist_populated() {
    let bytes = build_wav(44100, 2, 16, 44100, true, Some("My Title"), Some("Some Artist"));
    let f = write_wav_file(&bytes);
    let meta = extract_metadata(f.path()).unwrap();
    assert_eq!(meta.raw_artists_native.as_deref(), Some("Some Artist"));
}

#[test]
fn bwf_wav_no_id3_chunk_returns_ok_with_no_title() {
    let bytes = build_wav(44100, 2, 16, 44100, true, None, None);
    let f = write_wav_file(&bytes);
    let meta = extract_metadata(f.path()).unwrap();
    assert!(meta.title.is_none());
    assert!(meta.duration_seconds.is_some(), "duration must still be present");
}

#[test]
fn bwf_wav_standard_wav_still_works_via_lofty() {
    // A standard WAV with no bext chunk should still parse (via lofty fallback or our parser).
    let bytes = build_wav(44100, 2, 16, 44100, false, None, None);
    let f = write_wav_file(&bytes);
    let meta = extract_metadata(f.path()).unwrap();
    assert!(meta.duration_seconds.is_some());
}

#[test]
fn bwf_wav_truncated_fmt_returns_error() {
    // Craft a WAV with a fmt chunk that's only 4 bytes (too short)
    let truncated_fmt = riff_chunk(b"fmt ", &[0u8; 4]);
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
    assert!(result.is_err(), "truncated fmt should return Err");
}
```

- [ ] **Step 2: Register the test module in lib.rs or tests/mod.rs**

Check if there's a `tests/mod.rs` or if test files are auto-discovered. In `soul-importer`, tests are standalone files in `tests/`. Cargo discovers them automatically — no registration needed.

- [ ] **Step 3: Verify tests fail**

```bash
cd libraries/soul-importer
cargo test --test bwf_metadata_tests 2>&1 | head -40
```

Expected: `error[E0425]: cannot find function 'extract_wav_metadata'` or similar compile error / all tests fail because lofty rejects the bext chunk.

---

## Task 2: BWF WAV — Implementation

**Files:**
- Modify: `libraries/soul-importer/src/metadata.rs`

- [ ] **Step 1: Add `extract_wav_metadata()` before the existing DSD check**

Open `metadata.rs`. The function goes between the public `extract_metadata()` entry point and the existing private `extract_dsd_metadata()`. Add after the imports, before `pub fn extract_metadata`:

```rust
/// Parse a WAV/BWF file directly from its RIFF chunks.
///
/// Handles Broadcast Wave Format (BWF) files that contain a `bext` chunk
/// before the `fmt` chunk — lofty 0.18 rejects these with "abnormally large
/// data" errors. We walk the RIFF chunk list ourselves:
///
///   fmt  → sample_rate, channels, bits_per_sample
///   data → size → duration_seconds
///   id3  / ID3  → hand raw bytes to lofty ID3v2 parser
///   bext → skipped
///
/// Returns `Ok(RawMetadata)` on success. Returns `Err` on malformed files
/// (truncated headers, missing fmt/data chunks) so the caller can fall
/// through to lofty or surface the error.
fn extract_wav_metadata(path: &Path) -> Result<RawMetadata> {
    use std::io::{Read, Seek, SeekFrom};

    let mut f = std::fs::File::open(path)
        .map_err(|e| ImportError::Metadata(format!("Cannot open WAV: {}", e)))?;

    // Read RIFF header: "RIFF" (4) + file_size (4) + "WAVE" (4)
    let mut header = [0u8; 12];
    f.read_exact(&mut header)
        .map_err(|_| ImportError::Metadata("WAV: file too short for RIFF header".into()))?;
    if &header[0..4] != b"RIFF" {
        return Err(ImportError::Metadata("WAV: not a RIFF file".into()));
    }
    if &header[8..12] != b"WAVE" {
        return Err(ImportError::Metadata("WAV: RIFF type is not WAVE".into()));
    }

    let riff_size = u32::from_le_bytes(header[4..8].try_into().unwrap()) as u64;
    let end_pos = 8 + riff_size; // absolute byte position of RIFF end

    // State collected from chunks
    let mut sample_rate: Option<u32> = None;
    let mut channels: Option<u16> = None;
    let mut bits_per_sample: Option<u16> = None;
    let mut data_size: Option<u64> = None;
    let mut id3_bytes: Option<Vec<u8>> = None;

    // Walk chunks
    loop {
        let pos = f.stream_position()
            .map_err(|e| ImportError::Metadata(format!("WAV: seek error: {}", e)))?;
        if pos + 8 > end_pos {
            break; // No more complete chunk headers
        }

        let mut chunk_hdr = [0u8; 8];
        match f.read_exact(&mut chunk_hdr) {
            Ok(()) => {}
            Err(_) => break,
        }
        let chunk_id = &chunk_hdr[0..4];
        let chunk_size = u32::from_le_bytes(chunk_hdr[4..8].try_into().unwrap()) as u64;
        // RIFF chunks are word-aligned — padded to even size on disk
        let chunk_size_padded = chunk_size + (chunk_size & 1);

        match chunk_id {
            b"fmt " => {
                if chunk_size < 16 {
                    return Err(ImportError::Metadata(format!(
                        "WAV: fmt chunk too small ({} bytes)", chunk_size
                    )));
                }
                let mut fmt_data = vec![0u8; chunk_size as usize];
                f.read_exact(&mut fmt_data)
                    .map_err(|_| ImportError::Metadata("WAV: truncated fmt chunk".into()))?;
                // Skip padding if any
                if chunk_size_padded > chunk_size {
                    f.seek(SeekFrom::Current((chunk_size_padded - chunk_size) as i64)).ok();
                }
                // audio_format (2), num_channels (2), sample_rate (4), byte_rate (4),
                // block_align (2), bits_per_sample (2)
                channels = Some(u16::from_le_bytes([fmt_data[2], fmt_data[3]]));
                sample_rate = Some(u32::from_le_bytes([
                    fmt_data[4], fmt_data[5], fmt_data[6], fmt_data[7],
                ]));
                bits_per_sample = Some(u16::from_le_bytes([fmt_data[14], fmt_data[15]]));
            }
            b"data" => {
                data_size = Some(chunk_size);
                f.seek(SeekFrom::Current(chunk_size_padded as i64))
                    .map_err(|e| ImportError::Metadata(format!("WAV: seek past data: {}", e)))?;
            }
            b"id3 " | b"ID3 " => {
                let mut raw = vec![0u8; chunk_size as usize];
                f.read_exact(&mut raw)
                    .map_err(|_| ImportError::Metadata("WAV: truncated id3 chunk".into()))?;
                if chunk_size_padded > chunk_size {
                    f.seek(SeekFrom::Current((chunk_size_padded - chunk_size) as i64)).ok();
                }
                id3_bytes = Some(raw);
            }
            _ => {
                // Skip unknown chunk (bext, JUNK, LIST, etc.)
                f.seek(SeekFrom::Current(chunk_size_padded as i64)).ok();
            }
        }
    }

    let sr = sample_rate.ok_or_else(|| ImportError::Metadata("WAV: no fmt chunk found".into()))?;
    let ch = channels.ok_or_else(|| ImportError::Metadata("WAV: no channels in fmt".into()))?;
    let bps = bits_per_sample
        .ok_or_else(|| ImportError::Metadata("WAV: no bits_per_sample in fmt".into()))?;
    let data_bytes = data_size
        .ok_or_else(|| ImportError::Metadata("WAV: no data chunk found".into()))?;

    let bytes_per_sample = bps as u64 / 8;
    let frame_size = ch as u64 * bytes_per_sample;
    let duration_seconds = if sr > 0 && frame_size > 0 {
        data_bytes as f64 / (sr as f64 * frame_size as f64)
    } else {
        0.0
    };

    // Parse embedded ID3v2 tag if present
    let mut title: Option<String> = None;
    let mut raw_artists_native: Option<String> = None;
    let mut album: Option<String> = None;
    let mut album_artist: Option<String> = None;
    let mut track_number: Option<u32> = None;
    let mut disc_number: Option<u32> = None;
    let mut year: Option<i32> = None;
    let mut genres: Option<Vec<String>> = None;

    if let Some(raw_id3) = id3_bytes {
        use lofty::id3::v2::Id3v2Tag;
        use lofty::prelude::*;

        if let Ok(tag) = Id3v2Tag::read_from(&mut std::io::Cursor::new(&raw_id3)) {
            title = tag.title().map(|s| s.to_string());
            raw_artists_native = tag.artist().map(|s| s.to_string());
            album = tag.album().map(|s| s.to_string());
            album_artist = tag
                .get_text("TPE2")
                .map(|s| s.to_string());
            track_number = tag.track();
            disc_number = tag.disk();
            year = tag.year().map(|y| y as i32);
            genres = tag.genre().map(|g| vec![g.to_string()]);
        }
    }

    Ok(RawMetadata {
        title,
        raw_artists_native,
        album,
        album_artist,
        track_number,
        disc_number,
        year,
        genres,
        duration_seconds: Some(duration_seconds),
        bitrate: None,
        sample_rate: Some(sr),
        channels: Some(ch as u32),
        bits_per_sample: Some(bps as u32),
        file_format: Some("WAV".to_string()),
    })
}
```

- [ ] **Step 2: Insert the WAV call-site before the DSD check in `extract_metadata()`**

Find this block in `extract_metadata()` (around line 274):

```rust
    if ext == "dsf" || ext == "dff" || ext == "dsdiff" {
        return extract_dsd_metadata(path);
    }
```

Insert immediately after (or replace with):

```rust
    if ext == "dsf" || ext == "dff" || ext == "dsdiff" {
        return extract_dsd_metadata(path);
    }

    // BWF WAV: try our RIFF chunk parser first. On success return immediately.
    // On failure fall through to lofty — standard WAV files that lofty handles
    // correctly are unaffected.
    if ext == "wav" || ext == "wave" {
        match extract_wav_metadata(path) {
            Ok(meta) => return Ok(meta),
            Err(e) => {
                tracing::debug!(
                    file_path = %path.display(),
                    error = %e,
                    "[Metadata] WAV custom parser failed, falling through to lofty"
                );
                // Fall through to lofty below
            }
        }
    }
```

- [ ] **Step 3: Check that `RawMetadata` has `bits_per_sample` field**

```bash
cd libraries/soul-importer
grep -n "bits_per_sample" src/metadata.rs | head -20
```

If missing, add it to the `RawMetadata` struct and all construction sites (grep for `RawMetadata {`).

- [ ] **Step 4: Check lofty ID3v2 import path**

```bash
grep -n "Id3v2Tag\|id3::v2\|lofty::id3" libraries/soul-importer/src/metadata.rs | head -10
```

Adjust the import in `extract_wav_metadata` to match what lofty actually exports. The ID3v2 parse call may need to be `Id3v2Tag::read_from(reader)` — check lofty 0.18 API. If `get_text` isn't available, use:
```rust
tag.get(&lofty::id3::v2::FrameId::Valid(Cow::Borrowed("TPE2")))
```

- [ ] **Step 5: Run the BWF tests**

```bash
cd libraries/soul-importer
cargo test --test bwf_metadata_tests 2>&1
```

Expected: all 8 tests pass.

- [ ] **Step 6: Run the full importer test suite**

```bash
cargo test -p soul-importer 2>&1 | tail -20
```

Expected: all 51+ existing tests still pass.

- [ ] **Step 7: Commit**

```bash
cd D:/dev/soulaudio/soul-player
git add libraries/soul-importer/src/metadata.rs libraries/soul-importer/tests/bwf_metadata_tests.rs
git commit -m "feat(importer): parse BWF WAV RIFF chunks directly — fix null duration for bext files"
```

---

## Task 3: Subfolder Album Merge — Failing Tests

**Files:**
- Modify: `libraries/soul-importer/tests/entity_cache_tests.rs`

The `find_or_create_album()` and `find_or_create_album_cached()` functions in `fuzzy.rs` are used at different call sites. The cached variant is what the scanner actually uses (via `MetadataExtractor`). We need to fix both.

- [ ] **Step 1: Add failing tests to `entity_cache_tests.rs`**

Find the end of the existing tests in the file and append:

```rust
// ── Subfolder album merge tests ───────────────────────────────────────────────

#[tokio::test]
async fn subfolder_bsides_merges_into_parent_album() {
    let pool = setup_test_db().await;
    let extractor = FuzzyExtractor::new();
    let mut cache = EntityCache::preload(&pool).await.unwrap();

    let artist = extractor
        .find_or_create_artist_cached(&pool, "Tame Impala", &mut cache)
        .await
        .unwrap();
    let aid = Some(artist.entity.id);

    // Root album created first
    let root = extractor
        .find_or_create_album_cached(&pool, "Currents", aid, "/music/Tame Impala/Currents", &mut cache)
        .await
        .unwrap();

    // B-Sides in subfolder — same title + artist
    let bsides = extractor
        .find_or_create_album_cached(&pool, "Currents", aid, "/music/Tame Impala/Currents/B-Sides", &mut cache)
        .await
        .unwrap();

    // Must be the SAME album record
    assert_eq!(root.entity.id, bsides.entity.id, "B-Sides should merge into parent album");
}

#[tokio::test]
async fn subfolder_disc_two_merges_into_parent_album() {
    let pool = setup_test_db().await;
    let extractor = FuzzyExtractor::new();
    let mut cache = EntityCache::preload(&pool).await.unwrap();

    let artist = extractor
        .find_or_create_artist_cached(&pool, "Artist X", &mut cache)
        .await
        .unwrap();
    let aid = Some(artist.entity.id);

    let disc1 = extractor
        .find_or_create_album_cached(&pool, "The Album", aid, "/music/Artist X/The Album/Disc 1", &mut cache)
        .await
        .unwrap();
    let disc2 = extractor
        .find_or_create_album_cached(&pool, "The Album", aid, "/music/Artist X/The Album/Disc 2", &mut cache)
        .await
        .unwrap();

    assert_eq!(disc1.entity.id, disc2.entity.id, "Disc 2 must merge with Disc 1");
}

#[tokio::test]
async fn subfolder_discovery_order_independent() {
    let pool = setup_test_db().await;
    let extractor = FuzzyExtractor::new();
    let mut cache = EntityCache::preload(&pool).await.unwrap();

    let artist = extractor
        .find_or_create_artist_cached(&pool, "Artist Y", &mut cache)
        .await
        .unwrap();
    let aid = Some(artist.entity.id);

    // Subfolder discovered BEFORE root
    let bsides = extractor
        .find_or_create_album_cached(&pool, "Deep Blue", aid, "/music/Artist Y/Deep Blue/Extras", &mut cache)
        .await
        .unwrap();
    let root = extractor
        .find_or_create_album_cached(&pool, "Deep Blue", aid, "/music/Artist Y/Deep Blue", &mut cache)
        .await
        .unwrap();

    assert_eq!(root.entity.id, bsides.entity.id, "Root discovered after B-Sides — must merge");
}

#[tokio::test]
async fn subfolder_album_folder_path_set_to_parent() {
    let pool = setup_test_db().await;
    let extractor = FuzzyExtractor::new();
    let mut cache = EntityCache::preload(&pool).await.unwrap();

    let artist = extractor
        .find_or_create_artist_cached(&pool, "Artist Z", &mut cache)
        .await
        .unwrap();
    let aid = Some(artist.entity.id);

    // Subfolder first
    extractor
        .find_or_create_album_cached(&pool, "Ocean", aid, "/music/Artist Z/Ocean/Extras", &mut cache)
        .await
        .unwrap();
    // Root second — should update the stored folder_path to the shorter one
    let result = extractor
        .find_or_create_album_cached(&pool, "Ocean", aid, "/music/Artist Z/Ocean", &mut cache)
        .await
        .unwrap();

    assert_eq!(result.entity.folder_path, "/music/Artist Z/Ocean",
        "folder_path should be promoted to the outermost (shortest) path");
}

#[tokio::test]
async fn sibling_folders_same_title_not_merged() {
    let pool = setup_test_db().await;
    let extractor = FuzzyExtractor::new();
    let mut cache = EntityCache::preload(&pool).await.unwrap();

    let artist = extractor
        .find_or_create_artist_cached(&pool, "Various Artists", &mut cache)
        .await
        .unwrap();
    let aid = Some(artist.entity.id);

    // Two sibling folders — neither is a subfolder of the other
    let album_a = extractor
        .find_or_create_album_cached(&pool, "Hits", aid, "/music/Various Artists/Hits 2020", &mut cache)
        .await
        .unwrap();
    let album_b = extractor
        .find_or_create_album_cached(&pool, "Hits", aid, "/music/Various Artists/Hits 2021", &mut cache)
        .await
        .unwrap();

    assert_ne!(album_a.entity.id, album_b.entity.id, "Sibling folders must NOT be merged");
}

#[tokio::test]
async fn different_artist_not_merged() {
    let pool = setup_test_db().await;
    let extractor = FuzzyExtractor::new();
    let mut cache = EntityCache::preload(&pool).await.unwrap();

    let artist_a = extractor
        .find_or_create_artist_cached(&pool, "Artist A", &mut cache)
        .await
        .unwrap();
    let artist_b = extractor
        .find_or_create_artist_cached(&pool, "Artist B", &mut cache)
        .await
        .unwrap();

    let album_a = extractor
        .find_or_create_album_cached(&pool, "Debut", Some(artist_a.entity.id), "/music/Artist A/Debut/Extras", &mut cache)
        .await
        .unwrap();
    let album_b = extractor
        .find_or_create_album_cached(&pool, "Debut", Some(artist_b.entity.id), "/music/Artist B/Debut", &mut cache)
        .await
        .unwrap();

    assert_ne!(album_a.entity.id, album_b.entity.id, "Different artists must NOT be merged");
}

#[tokio::test]
async fn rescan_idempotent_after_merge() {
    let pool = setup_test_db().await;
    let extractor = FuzzyExtractor::new();
    let mut cache = EntityCache::preload(&pool).await.unwrap();

    let artist = extractor
        .find_or_create_artist_cached(&pool, "Idempotent Artist", &mut cache)
        .await
        .unwrap();
    let aid = Some(artist.entity.id);

    // First scan
    let a1 = extractor
        .find_or_create_album_cached(&pool, "Idempotent", aid, "/music/Idempotent Artist/Album", &mut cache)
        .await
        .unwrap();
    let b1 = extractor
        .find_or_create_album_cached(&pool, "Idempotent", aid, "/music/Idempotent Artist/Album/Extras", &mut cache)
        .await
        .unwrap();
    assert_eq!(a1.entity.id, b1.entity.id);

    // Second scan — fresh cache
    let mut cache2 = EntityCache::preload(&pool).await.unwrap();
    let a2 = extractor
        .find_or_create_album_cached(&pool, "Idempotent", aid, "/music/Idempotent Artist/Album", &mut cache2)
        .await
        .unwrap();
    let b2 = extractor
        .find_or_create_album_cached(&pool, "Idempotent", aid, "/music/Idempotent Artist/Album/Extras", &mut cache2)
        .await
        .unwrap();

    assert_eq!(a2.entity.id, b2.entity.id, "Rescan must not create a new album");
    assert_eq!(a1.entity.id, a2.entity.id, "Must be the same album as first scan");
}
```

- [ ] **Step 2: Run the new tests to confirm they fail**

```bash
cd libraries/soul-importer
cargo test --test entity_cache_tests subfolder 2>&1
```

Expected: all 7 subfolder tests fail (merge is not implemented yet).

---

## Task 4: Subfolder Album Merge — Implementation

**Files:**
- Modify: `libraries/soul-importer/src/fuzzy.rs`

The scanner uses `find_or_create_album_cached()`. We also fix `find_or_create_album()` for consistency (used in some test helpers). We need:

1. A helper `fn is_subfolder(parent: &str, child: &str) -> bool`
2. A subfolder match step in both `find_or_create_album()` and `find_or_create_album_cached()`
3. When the new folder is the parent of an existing album's folder, update the album's `folder_path` in the DB

- [ ] **Step 1: Add `is_subfolder` helper at module level in `fuzzy.rs`**

Add near the top of the file (after imports):

```rust
/// Returns true if `child` is a direct or indirect subfolder of `parent`.
/// Uses path-separator-aware prefix matching to avoid false positives
/// (e.g. "/music/Album" must not match "/music/AlbumExtra").
fn is_subfolder(parent: &str, child: &str) -> bool {
    let p = parent.trim_end_matches(['/', '\\']);
    child.starts_with(&format!("{}/", p)) || child.starts_with(&format!("{}\\", p))
}

#[cfg(test)]
mod subfolder_tests {
    use super::*;
    #[test] fn forward_slash() { assert!(is_subfolder("/a/b", "/a/b/c")); }
    #[test] fn backslash() { assert!(is_subfolder("C:\\a\\b", "C:\\a\\b\\c")); }
    #[test] fn sibling_no_match() { assert!(!is_subfolder("/a/b", "/a/bc")); }
    #[test] fn exact_no_match() { assert!(!is_subfolder("/a/b", "/a/b")); }
    #[test] fn child_to_parent_no_match() { assert!(!is_subfolder("/a/b/c", "/a/b")); }
}
```

- [ ] **Step 2: Add subfolder match step to `find_or_create_album()`**

Replace the "No match — create new album" block in `find_or_create_album()` (around line 214) with:

```rust
        // Subfolder match — same (title, artist_id) AND one folder is an ancestor of the other.
        // Use the outermost (shorter) path as canonical.
        for album in &albums {
            if normalize_string(&album.title) == normalized_title && album.artist_id == artist_id {
                let stored = &album.folder_path;
                if is_subfolder(stored, folder_path) {
                    // Existing album is parent, incoming is subfolder — merge into existing
                    return Ok(FuzzyMatch {
                        entity: album.clone(),
                        confidence: 90,
                        match_type: MatchType::Normalized,
                    });
                }
                if is_subfolder(folder_path, stored) {
                    // Incoming folder is parent of stored — promote folder_path to incoming
                    soul_storage::albums::update_folder_path(pool, album.id, folder_path).await?;
                    let mut promoted = album.clone();
                    promoted.folder_path = folder_path.to_string();
                    return Ok(FuzzyMatch {
                        entity: promoted,
                        confidence: 90,
                        match_type: MatchType::Normalized,
                    });
                }
            }
        }

        // No match — create new album
```

- [ ] **Step 3: Add subfolder match step to `find_or_create_album_cached()`**

The cached variant doesn't store folder_path in the cache key. The cache key is `(normalized_title, artist_id)`. Currently it hits cache and returns without checking folder_path. We need to add a post-cache-hit check:

Replace the cache hit block (around line 305-326) with:

```rust
        // O(1) cache lookup keyed by (normalized_title, artist_id)
        if let Some((id, original_title)) =
            cache.find_album_by_normalized(&normalized_title, artist_id)
        {
            let album = soul_storage::albums::get_by_id(pool, id)
                .await?
                .ok_or_else(|| {
                    crate::ImportError::Metadata(format!("Cached album id {} not found in DB", id))
                })?;

            // Check if this is a subfolder merge case:
            // - If incoming folder_path == stored: exact hit, return normally.
            // - If stored is parent of incoming: merge (return stored album).
            // - If incoming is parent of stored: promote folder_path in DB.
            // - Otherwise: it's a different album in a sibling folder — fall through to create.
            if album.folder_path == folder_path {
                // Exact folder match — normal cache hit
            } else if is_subfolder(&album.folder_path, folder_path) {
                // Stored is parent, incoming is subfolder — merge into stored
                let confidence = if original_title == title { 100 } else { 95 };
                let match_type = if confidence == 100 { MatchType::Exact } else { MatchType::Normalized };
                return Ok(FuzzyMatch { entity: album, confidence, match_type });
            } else if is_subfolder(folder_path, &album.folder_path) {
                // Incoming is parent — promote folder_path in DB
                soul_storage::albums::update_folder_path(pool, id, folder_path).await?;
                let mut promoted = album;
                promoted.folder_path = folder_path.to_string();
                let confidence = if original_title == title { 100 } else { 95 };
                let match_type = if confidence == 100 { MatchType::Exact } else { MatchType::Normalized };
                return Ok(FuzzyMatch { entity: promoted, confidence, match_type });
            } else {
                // Sibling folder with same title+artist — this is a DIFFERENT album.
                // Do NOT return the cached hit; fall through to create a new one.
                // NOTE: We intentionally do NOT return here — let the code fall to create.
                // We must skip the normal confidence return below.
                let new_album = soul_storage::albums::create(
                    pool,
                    CreateAlbum {
                        title: title.to_string(),
                        artist_id,
                        year: None,
                        musicbrainz_id: None,
                        folder_path: folder_path.to_string(),
                    },
                )
                .await?;
                // Don't update the cache for this artist+title key — it already maps
                // to the other album. The cache lookup will keep returning the first album
                // for the same artist+title, but hits will be checked for folder match.
                return Ok(FuzzyMatch {
                    entity: new_album,
                    confidence: 100,
                    match_type: MatchType::Created,
                });
            }

            let confidence = if original_title == title { 100 } else { 95 };
            let match_type = if confidence == 100 { MatchType::Exact } else { MatchType::Normalized };
            return Ok(FuzzyMatch { entity: album, confidence, match_type });
        }
```

- [ ] **Step 4: Add `update_folder_path` to `soul-storage/albums`**

Check if it exists:

```bash
grep -n "update_folder_path\|fn update" libraries/soul-storage/src/albums/mod.rs | head -10
```

If missing, add to `libraries/soul-storage/src/albums/mod.rs`:

```rust
/// Update the folder_path of an album (used when a parent folder is discovered
/// after a subfolder, to promote the canonical path to the outermost location).
pub async fn update_folder_path(pool: &SqlitePool, id: AlbumId, folder_path: &str) -> Result<()> {
    sqlx::query!(
        "UPDATE albums SET folder_path = ? WHERE id = ?",
        folder_path,
        id
    )
    .execute(pool)
    .await?;
    Ok(())
}
```

Run `cargo sqlx prepare` in `libraries/soul-storage` if sqlx offline mode is used:

```bash
cd libraries/soul-storage
DATABASE_URL="sqlite:$(cat .env | grep DATABASE_URL | cut -d= -f2-)" cargo sqlx prepare -- --lib
```

Or use the xtask:

```bash
cargo xtask setup sqlx
```

- [ ] **Step 5: Run the subfolder tests**

```bash
cd libraries/soul-importer
cargo test --test entity_cache_tests subfolder 2>&1
```

Expected: all 7 subfolder tests pass.

- [ ] **Step 6: Run the full test suite**

```bash
cargo test -p soul-importer 2>&1 | tail -20
```

Expected: all tests pass.

- [ ] **Step 7: Add the E2E test for subfolder merging in scanner_import_e2e_tests.rs**

Find the end of the existing tests and append:

```rust
#[tokio::test]
async fn test_subfolder_tracks_merge_into_one_album() {
    let pool = setup_test_pool().await;
    let dir = TempDir::new().unwrap();

    // Root directory with one track
    let root_track = dir.path().join("track_01.flac");
    create_flac_with_tags(
        &root_track,
        "Track One",
        "Subfolder Artist",
        "Subfolder Album",
        1,
    );

    // Subfolder with another track, same album+artist metadata
    let sub = dir.path().join("B-Sides");
    std::fs::create_dir_all(&sub).unwrap();
    let sub_track = sub.join("track_02.flac");
    create_flac_with_tags(
        &sub_track,
        "Track Two (B-Side)",
        "Subfolder Artist",
        "Subfolder Album",
        2,
    );

    let (scanner, source) = make_scanner_with_source(&pool, dir.path()).await;
    scanner.scan_source(&source).await.unwrap();

    let albums = soul_storage::albums::get_all(&pool).await.unwrap();
    assert_eq!(albums.len(), 1, "subfolder tracks should merge into one album, got: {:?}",
        albums.iter().map(|a| &a.folder_path).collect::<Vec<_>>());

    let tracks = soul_storage::tracks::get_by_album(&pool, albums[0].id).await.unwrap();
    assert_eq!(tracks.len(), 2, "both tracks should be under the single album");
}
```

- [ ] **Step 8: Run the E2E subfolder test**

```bash
cd libraries/soul-importer
cargo test --test scanner_import_e2e_tests test_subfolder_tracks_merge_into_one_album 2>&1
```

Expected: passes.

- [ ] **Step 9: Commit**

```bash
cd D:/dev/soulaudio/soul-player
git add libraries/soul-importer/src/fuzzy.rs \
        libraries/soul-storage/src/albums/mod.rs \
        libraries/soul-importer/tests/entity_cache_tests.rs \
        libraries/soul-importer/tests/scanner_import_e2e_tests.rs \
        libraries/soul-storage/.sqlx/
git commit -m "feat(importer): merge subfolder albums — B-Sides/Disc-N fold into parent album record"
```

---

## Task 5: Scan Performance — Add rayon and Benchmark Baseline

**Files:**
- Modify: `libraries/soul-importer/Cargo.toml`
- Modify: `libraries/soul-importer/benches/scan_benchmark.rs`

- [ ] **Step 1: Add rayon to Cargo.toml**

In `libraries/soul-importer/Cargo.toml`, under `[dependencies]`:

```toml
rayon = "1"
```

- [ ] **Step 2: Run `cargo check` to confirm rayon resolves**

```bash
cd libraries/soul-importer
cargo check 2>&1 | tail -5
```

- [ ] **Step 3: Add benchmark baseline for parallel stat**

In `libraries/soul-importer/benches/scan_benchmark.rs`, add a new benchmark group at the end:

```rust
fn bench_phase0_parallel_vs_sequential(c: &mut Criterion) {
    use std::fs;
    use tempfile::TempDir;

    // Create 5000 directories
    let dir = TempDir::new().unwrap();
    let dirs: Vec<std::path::PathBuf> = (0..5000)
        .map(|i| {
            let p = dir.path().join(format!("dir_{:05}", i));
            fs::create_dir_all(&p).unwrap();
            p
        })
        .collect();

    let mut group = c.benchmark_group("phase0_stat");
    group.sample_size(10);

    group.bench_function("sequential_5000_dirs", |b| {
        b.iter(|| {
            let _results: Vec<i64> = dirs
                .iter()
                .map(|d| {
                    fs::metadata(d)
                        .ok()
                        .and_then(|m| m.modified().ok())
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_millis() as i64)
                        .unwrap_or(0)
                })
                .collect();
        });
    });

    group.bench_function("parallel_rayon_5000_dirs", |b| {
        use rayon::prelude::*;
        b.iter(|| {
            let _results: Vec<i64> = dirs
                .par_iter()
                .map(|d| {
                    fs::metadata(d)
                        .ok()
                        .and_then(|m| m.modified().ok())
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_millis() as i64)
                        .unwrap_or(0)
                })
                .collect();
        });
    });

    group.finish();
}

criterion_group!(
    perf_benches,
    bench_phase0_parallel_vs_sequential,
);
```

Also add `perf_benches` to the `criterion_main!` call at the bottom:

```rust
criterion_main!(scan_benches, perf_benches);
```

- [ ] **Step 4: Run the baseline benchmark (sequential only)**

```bash
cd libraries/soul-importer
cargo bench --bench scan_benchmark -- phase0_stat/sequential 2>&1 | tail -10
```

Record the result. This is the baseline.

- [ ] **Step 5: Commit**

```bash
cd D:/dev/soulaudio/soul-player
git add libraries/soul-importer/Cargo.toml libraries/soul-importer/benches/scan_benchmark.rs Cargo.lock
git commit -m "chore(importer): add rayon dep and Phase-0 benchmark baseline"
```

---

## Task 6: Scan Performance — Parallel Directory Stat

**Files:**
- Modify: `libraries/soul-importer/src/scanner.rs`

- [ ] **Step 1: Write a failing test for parallel result correctness**

In `libraries/soul-importer/tests/scanner_import_e2e_tests.rs`, add:

```rust
#[tokio::test]
async fn test_parallel_scan_result_matches_sequential() {
    // Create 200 directories each with one FLAC
    let dir = TempDir::new().unwrap();
    for i in 0..200 {
        let sub = dir.path().join(format!("artist_{:03}", i));
        std::fs::create_dir_all(&sub).unwrap();
        let track = sub.join("track.flac");
        create_flac_with_tags(&track, "Title", "Artist", "Album", 1);
    }

    let pool = setup_test_pool().await;
    let (scanner, source) = make_scanner_with_source(&pool, dir.path()).await;
    let stats = scanner.scan_source(&source).await.unwrap();

    assert_eq!(stats.new_files, 200, "should find all 200 files");
    assert_eq!(stats.errors, 0);
}
```

Run it to confirm it passes with the current sequential code (baseline):

```bash
cd libraries/soul-importer
cargo test --test scanner_import_e2e_tests test_parallel_scan_result_matches_sequential 2>&1
```

(This test should pass before the change — we're ensuring the parallel version gives the same result.)

- [ ] **Step 2: Replace the sequential stat loop with rayon**

In `scanner.rs`, find the sequential loop (around line 140):

```rust
        for dir in &directories {
            let dir_str = dir.display().to_string();

            // Get directory mtime ...
            let dir_mtime = match std::fs::metadata(dir) {
```

Replace the entire loop block with a rayon parallel version. The result is collected into a Vec, then the sequential DB-upsert part happens as before:

```rust
        use rayon::prelude::*;

        // Phase 0: stat all directories in parallel, collect (dir, mtime, file_count) tuples
        let stat_results: Vec<(PathBuf, i64, i64)> = directories
            .par_iter()
            .filter_map(|dir| {
                let dir_mtime = match std::fs::metadata(dir) {
                    Ok(m) => m
                        .modified()
                        .map(|t| {
                            t.duration_since(std::time::UNIX_EPOCH)
                                .map(|d| d.as_millis() as i64)
                                .unwrap_or(0)
                        })
                        .unwrap_or(0),
                    Err(e) => {
                        tracing::warn!("Failed to stat directory {:?}: {}", dir, e);
                        return None;
                    }
                };

                // Quick-verify file count as a safety net for NTFS same-second mtime
                let file_count = std::fs::read_dir(dir)
                    .map(|entries| {
                        entries
                            .filter_map(|e| e.ok())
                            .filter(|e| {
                                e.file_type().map(|ft| ft.is_file()).unwrap_or(false)
                                    && is_audio_file(&e.path())
                            })
                            .count() as i64
                    })
                    .unwrap_or(0);

                Some((dir.clone(), dir_mtime, file_count))
            })
            .collect();

        // Phase 0 sequential: decide changed/unchanged, collect files and scanned_dirs
        for (dir, dir_mtime, actual_count) in stat_results {
            let dir_str = dir.display().to_string();

            if let Some(stored) = stored_dirs.get(&dir_str) {
                if stored.dir_mtime == dir_mtime && actual_count == stored.file_count {
                    // Unchanged directory — record it but don't re-scan files
                    unchanged_dir_count += 1;
                    scanned_dirs.push(ScannedDirInfo {
                        path: dir_str,
                        dir_mtime,
                        file_count: stored.file_count,
                        changed: false,
                    });
                    continue;
                }
            }

            // Directory is new or changed — scan its files
            let audio_files: Vec<PathBuf> = std::fs::read_dir(&dir)
                .map(|entries| {
                    entries
                        .filter_map(|e| e.ok())
                        .map(|e| e.path())
                        .filter(|p| p.is_file() && is_audio_file(p))
                        .collect()
                })
                .unwrap_or_default();

            let file_count = audio_files.len() as i64;
            changed_files.extend(audio_files);

            scanned_dirs.push(ScannedDirInfo {
                path: dir_str,
                dir_mtime,
                file_count,
                changed: true,
            });
        }
```

> **Note:** The old loop did both the mtime stat AND the file collection in one pass. The new structure separates them: parallel stat → sequential decision + file collect. The file collection (`read_dir` for audio files) remains sequential to avoid spawning too many threads on directories that turn out to be unchanged. If your existing code differs, adapt to match the actual `ScannedDirInfo` struct fields.

- [ ] **Step 3: Run the parallel correctness test**

```bash
cd libraries/soul-importer
cargo test --test scanner_import_e2e_tests test_parallel_scan_result_matches_sequential 2>&1
```

Expected: passes.

- [ ] **Step 4: Run the full test suite**

```bash
cargo test -p soul-importer 2>&1 | tail -20
```

Expected: all tests pass.

- [ ] **Step 5: Run the Phase-0 benchmark to confirm speedup**

```bash
cargo bench --bench scan_benchmark -- phase0_stat 2>&1 | tail -20
```

Expected: `parallel_rayon_5000_dirs` is ≥2× faster than `sequential_5000_dirs` on a multi-core machine. If the machine has only 1-2 cores (CI), accept any result ≥1× (rayon gracefully degrades).

- [ ] **Step 6: Commit**

```bash
cd D:/dev/soulaudio/soul-player
git add libraries/soul-importer/src/scanner.rs
git commit -m "perf(importer): parallel directory stat with rayon — Phase-0 speedup on large libraries"
```

---

## Task 7: Scan Performance — Auto-Scale Concurrency + Batched Transactions

**Files:**
- Modify: `libraries/soul-importer/src/library_scanner.rs`
- Modify: `libraries/soul-importer/benches/scan_benchmark.rs`

- [ ] **Step 1: Add failing tests for auto-concurrency and batch size**

In `scanner_import_e2e_tests.rs`, add:

```rust
#[tokio::test]
async fn test_scan_auto_concurrency_respects_cap() {
    // LibraryScanner::new() without .concurrency() override should have concurrency <= 64
    use soul_importer::LibraryScanner;
    let pool = setup_test_pool().await;
    let scanner = LibraryScanner::new(pool, "1", "test-device");
    // Inspect via a public getter (add one if needed) or just verify scan completes
    // without hanging (concurrency is internal; we can't directly read it here).
    // This test verifies the cap indirectly via scan completion.
    assert!(scanner.concurrency() <= 64, "default concurrency must be <= 64");
}

#[tokio::test]
async fn test_scan_batch_size_100_all_tracks_imported() {
    let dir = TempDir::new().unwrap();
    for i in 0..200 {
        let sub = dir.path().join(format!("artist_{:03}", i));
        std::fs::create_dir_all(&sub).unwrap();
        create_flac_with_tags(&sub.join("track.flac"), "Title", "Artist", "Album", 1);
    }

    let pool = setup_test_pool().await;
    let (scanner, source) = make_scanner_with_source(&pool, dir.path()).await;
    let stats = scanner.scan_source(&source).await.unwrap();

    assert_eq!(stats.new_files, 200);
    assert_eq!(stats.errors, 0);
    // Verify all 200 are actually in the DB (not just counted)
    let tracks = soul_storage::tracks::get_all(&pool, "1").await.unwrap();
    assert_eq!(tracks.len(), 200);
}
```

- [ ] **Step 2: Add a `concurrency()` getter to `LibraryScanner`**

In `library_scanner.rs`, add after `pub fn concurrency(mut self, max: usize) -> Self`:

```rust
    /// Returns the current concurrency limit (for testing and diagnostics).
    pub fn concurrency_limit(&self) -> usize {
        self.concurrency
    }
```

Then update the test to use `scanner.concurrency_limit()`.

- [ ] **Step 3: Update `LibraryScanner::new()` to auto-scale concurrency**

In `library_scanner.rs`, replace:

```rust
            concurrency: 8,
```

with:

```rust
            concurrency: {
                let cpus = std::thread::available_parallelism()
                    .map(|n| n.get())
                    .unwrap_or(4);
                (cpus * 2).min(64)
            },
```

- [ ] **Step 4: Change the batch flush from 10 → 100 with transactional writes**

Find the flush condition in Phase 2 (around line 417):

```rust
                    // Flush progress every 10 files
                    if total_phase2_processed % 10 == 0 {
```

Change to:

```rust
                    // Flush progress every 100 files
                    if total_phase2_processed % 100 == 0 {
```

The `flush_progress` method writes to the `scan_progress` table. To also wrap the actual track inserts in transactions, find `process_extraction_result` (which calls `file_processor.process_file()`) and check if the `FileProcessor` already uses transactions. If not, look at `file_processor.rs`:

```bash
grep -n "transaction\|begin\|tx\|BEGIN" libraries/soul-importer/src/file_processor.rs | head -20
```

If there's no transaction wrapping, add one in `process_extraction_result` in `library_scanner.rs` by batching the DB work. However, the simplest and most impactful change is just increasing the batch flush size (the SQLite auto-commit model still groups writes within tokio tasks). The real win is the progress-table write frequency — going from 10→100 reduces progress writes by 10×.

> **Note:** If `soul-storage` functions already open individual connections per call, true transaction batching requires passing a `&mut Transaction` through the call chain — a larger refactor. Scope this change to: batch-size=100 for progress writes, and evaluate whether file-processor writes need wrapping. The benchmark in Step 6 will confirm whether further work is needed.

- [ ] **Step 5: Run the new tests**

```bash
cd libraries/soul-importer
cargo test --test scanner_import_e2e_tests test_scan_auto_concurrency_respects_cap test_scan_batch_size_100_all_tracks_imported 2>&1
```

Expected: both pass.

- [ ] **Step 6: Add concurrency and batch benchmarks**

In `scan_benchmark.rs`, add to the `perf_benches` group:

```rust
fn bench_db_write_batch(c: &mut Criterion) {
    // Synthetic: insert N rows with batch-10 vs batch-100
    // Measured via the flush_progress helper directly if possible,
    // or by running a full scan with different batch sizes.
    // Simplified version: just run a 200-file scan and compare flush counts.
    let rt = tokio::runtime::Runtime::new().unwrap();

    let mut group = c.benchmark_group("db_write_batch");
    group.sample_size(5);

    group.bench_function("scan_200_files", |b| {
        b.iter(|| {
            rt.block_on(async {
                let dir = tempfile::TempDir::new().unwrap();
                for i in 0..200u32 {
                    let sub = dir.path().join(format!("d{}", i));
                    std::fs::create_dir_all(&sub).unwrap();
                    // create minimal FLAC
                }
                // scan with default settings
            });
        });
    });

    group.finish();
}
```

> This is a placeholder — a proper DB write benchmark requires the ability to inject a pool. Use Criterion as an integration timing gate, not a microbenchmark. The meaningful verification is done via the E2E test in Step 5.

- [ ] **Step 7: Run the full test suite**

```bash
cargo test -p soul-importer 2>&1 | tail -20
```

Expected: all tests pass.

- [ ] **Step 8: Commit**

```bash
cd D:/dev/soulaudio/soul-player
git add libraries/soul-importer/src/library_scanner.rs \
        libraries/soul-importer/tests/scanner_import_e2e_tests.rs
git commit -m "perf(importer): auto-scale worker concurrency (cpu*2 capped at 64), batch flush every 100 files"
```

---

## Task 8: Final Verification

- [ ] **Step 1: Run the full importer test suite one last time**

```bash
cd libraries/soul-importer
cargo test -p soul-importer 2>&1 | tail -30
```

Expected: all tests pass, zero failures.

- [ ] **Step 2: Run pre-commit checks**

```bash
cd D:/dev/soulaudio/soul-player
cargo xtask check precommit 2>&1 | tail -20
```

Expected: fmt, clippy, tests all green.

- [ ] **Step 3: Run a benchmark summary**

```bash
cd libraries/soul-importer
cargo bench --bench scan_benchmark 2>&1 | grep -E "time:|thrpt:" | head -20
```

Record the numbers for reference.

- [ ] **Step 4: Final commit (if any formatting fixes needed)**

```bash
cd D:/dev/soulaudio/soul-player
cargo xtask check fmt --fix
git add -u
git commit -m "chore: fmt fixes after importer trifecta implementation"
```

---

## Quick Sanity Reference

### Running only a specific test file
```bash
cd libraries/soul-importer
cargo test --test bwf_metadata_tests          # BWF WAV unit tests
cargo test --test entity_cache_tests          # Album merge tests
cargo test --test scanner_import_e2e_tests    # E2E scanner tests
cargo test -p soul-importer                   # Everything
```

### Checking sqlx offline cache after adding a query
```bash
cd libraries/soul-storage
cargo xtask setup sqlx   # or: cargo sqlx prepare -- --lib
git add .sqlx/
```

### If lofty ID3v2 import path is wrong
Check lofty 0.18 re-exports:
```bash
grep -rn "pub use\|pub mod" libraries/soul-importer/src/metadata.rs
# Also:
cargo doc -p soul-importer --open
```
