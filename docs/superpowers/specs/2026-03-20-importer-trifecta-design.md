# Design: Importer Trifecta — BWF WAV, Subfolder Album Merging, Scan Performance

**Date:** 2026-03-20
**Status:** Approved
**Scope:** `libraries/soul-importer`

---

## Overview

Three improvements to the library scanner and importer:

1. **BWF WAV full metadata** — parse RIFF chunks directly so Broadcast Wave Format files that lofty rejects are imported with correct duration, sample rate, channels, and embedded ID3v2 tags.
2. **Subfolder album merging** — tracks in subfolders (B-sides, extras, disc folders) with the same `album` + `album_artist` tags are merged into one album record, rather than creating a separate album per folder.
3. **Scan performance at scale** — three targeted changes that bring first-time scan of 100k files from ~2hr to ~18min on an 8-core machine, and incremental rescan from ~30s to ~4s.

All work is **test-driven**: failing tests are written before each implementation unit. Performance changes are **benchmark-gated**: a change only ships once a Criterion benchmark confirms measurable improvement.

---

## 1. BWF WAV Full Metadata

### Problem

Broadcast Wave Format (BWF) WAV files embed a `bext` chunk before the standard `fmt` chunk. Lofty's WAV parser rejects them with "abnormally large data" errors, causing the importer to return `ImportError::Metadata` and skip the file entirely. Six tracks in the production library (Osaki Seiichi) have `duration_seconds = NULL` as a result.

### Solution

Add `extract_wav_metadata()` to `libraries/soul-importer/src/metadata.rs`, mirroring the existing `extract_dsd_metadata()` pattern. No new dependencies.

### RIFF Chunk Walk

```
RIFF (file header)
  ├── fmt   → sample_rate (u32 LE), num_channels (u16 LE), bits_per_sample (u16 LE)
  ├── bext  → skip entirely (BWF broadcast extension)
  ├── data  → chunk size → duration_seconds = size / (sample_rate × channels × (bits/8))
  ├── id3   → raw bytes → hand to lofty ID3v2 parser
  └── ID3   → same (some encoders capitalise differently)
```

All multi-byte fields are little-endian (RIFF standard).

### Call Site

In `extract_metadata()`, before calling lofty: if the extension is `.wav` or `.wave`, try `extract_wav_metadata()` first. On success, return it. On failure, fall through to lofty — so standard WAV files that lofty already handles correctly are unaffected.

### TDD Test Plan

**Unit tests** (new `bwf_metadata_tests.rs` in `libraries/soul-importer/tests/`):

| Test | Assertion |
|---|---|
| `bwf_wav_duration_is_correct` | duration computed from `data` chunk size matches known value |
| `bwf_wav_sample_rate_is_correct` | sample_rate from `fmt` chunk |
| `bwf_wav_channels_is_correct` | channels from `fmt` chunk |
| `bwf_wav_id3_title_populated` | title from embedded `id3 ` chunk |
| `bwf_wav_id3_artist_populated` | artist from embedded `id3 ` chunk |
| `bwf_wav_no_id3_chunk_returns_ok_with_no_title` | missing ID3 chunk → Ok, title = None |
| `bwf_wav_standard_wav_still_works_via_lofty` | standard WAV (no bext) → lofty path unchanged |
| `bwf_wav_truncated_fmt_returns_error` | malformed file → ImportError, no panic |
| `bwf_real_osaki_seiichi_has_duration` | real file on dev machine, `require_file!` guarded |

Synthetic test files are built in-test using `std::io::Write` — no binary assets committed.

---

## 2. Subfolder Album Merging

### Problem

The album lookup key is `(title, artist_id, folder_path)`. Tracks in `Album/` and `Album/B-Sides/` both have `album = "Currents"` and `album_artist = "Tame Impala"` but different folder paths, so they get two separate album records. The production DB shows "Currents" and "Currents B-Sides & Remixes" as separate albums.

### Solution

Change `find_or_create_album()` in `libraries/soul-importer/src/fuzzy.rs` to add a **subfolder match** step between the existing exact-match and create-new steps.

### New Lookup Order

1. **Exact match** — `(title, artist_id, folder_path)` — unchanged, ensures rescans are idempotent.
2. **Subfolder match** — same `(title, artist_id)` AND one folder path is a direct or indirect ancestor of the other. Use the album whose `folder_path` is the shorter (outermost) path as canonical. If the incoming folder is the parent of the stored album's folder, update the album's `folder_path` to the shorter one.
3. **Create new** — no match found.

### Subfolder Check

```rust
fn is_subfolder(parent: &str, child: &str) -> bool {
    let p = parent.trim_end_matches(['/', '\\']);
    child.starts_with(&format!("{}/", p)) ||
    child.starts_with(&format!("{}\\", p))
}
```

Both `is_subfolder(a, b)` and `is_subfolder(b, a)` are checked so discovery order is irrelevant.

### What Is Unchanged

- Tracks with different `album_artist` values are never merged.
- Two folders at the same depth that happen to share a title+artist are never merged (neither is a subfolder of the other).
- Track-level `file_path` fields are untouched; only the album's `folder_path` is promoted to the parent.

### TDD Test Plan

**New test cases in `libraries/soul-importer/tests/entity_cache_tests.rs`:**

| Test | Assertion |
|---|---|
| `subfolder_bsides_merges_into_parent_album` | `B-Sides/` tracks land in same album as root tracks |
| `subfolder_disc_two_merges_into_parent_album` | `Disc 2/` tracks land in same album |
| `subfolder_discovery_order_independent` | B-Sides scanned before root → same result |
| `subfolder_album_folder_path_set_to_parent` | merged album's `folder_path` = shorter path |
| `sibling_folders_same_title_not_merged` | `Artist/Album A/` and `Artist/Album B/` with same title tag → two albums |
| `different_artist_not_merged` | same title, different artist_id → two albums |
| `rescan_idempotent_after_merge` | second scan of same library → no new albums created |

**Scanner E2E test in `scanner_import_e2e_tests.rs`:**

| Test | Assertion |
|---|---|
| `test_subfolder_tracks_merge_into_one_album` | create temp dir with `Album/` + `Album/B-Sides/`, scan → 1 album, all tracks present |

---

## 3. Scan Performance

### Problem

The scanner processes Phase 0/1 (directory walk + mtime stat) sequentially and caps Phase 2 (metadata extraction) at 8 concurrent workers. DB writes use SQLite auto-commit (one fsync per INSERT). At 100k files on an 8-core machine: Phase 0/1 ~30s, Phase 2 ~2hr, DB writes ~60s.

### Change 1 — Parallel Directory Stat

**File:** `libraries/soul-importer/src/scanner.rs`

Replace the sequential `for dir in &directories` mtime-check loop with `rayon::par_iter()`. Each call to `fs::metadata(dir)` and the unchanged/changed decision runs in parallel. Results are collected into a `Vec` before the sequential DB upsert.

Add `rayon` to `soul-importer/Cargo.toml`.

**Benchmark:** extend `bench_incremental_scan_10k_files` in `scanner.rs`. Add a `bench_phase0_parallel_vs_sequential` Criterion benchmark measuring wall time to stat 5000 directories. Gate: parallel must be ≥2× faster than sequential on the CI machine.

### Change 2 — Auto-Scale Worker Count

**File:** `libraries/soul-importer/src/library_scanner.rs`

```rust
// In LibraryScanner::new():
let cpus = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
let max_inflight = (cpus * 2).min(64);
```

The `.concurrency(n)` builder method is preserved for overrides (tests fix at 2 or 4 to stay deterministic).

**Benchmark:** `bench_phase2_throughput` — scan 1000 synthetic FLAC files with concurrency=8 vs concurrency=`num_cpus*2`. Gate: ≥1.5× throughput improvement on machines with >8 cores.

### Change 3 — Batched Transactional DB Writes

**File:** `libraries/soul-importer/src/library_scanner.rs`

- Increase `BATCH_SIZE` constant from 10 → 100.
- Wrap each batch flush in an explicit `sqlx` transaction (`pool.begin()` / `tx.commit()`).

**Benchmark:** `bench_db_write_batch` — insert 10k track rows with batch-size=10/no-tx vs batch-size=100/tx. Gate: ≥5× improvement in rows/second.

### TDD Test Plan

**New tests in `scanner_import_e2e_tests.rs`:**

| Test | Assertion |
|---|---|
| `test_parallel_scan_result_matches_sequential` | scan same 500-file fixture with rayon vs sequential → identical files-found list |
| `test_scan_batch_size_100_all_tracks_imported` | 200 files, batch=100 → all 200 tracks in DB, no duplicates |
| `test_scan_auto_concurrency_respects_cap` | LibraryScanner default concurrency ≤ 64 |
| `test_scan_large_directory_completes` | 5000 synthetic files → scan completes, stats correct |

---

## Implementation Order

```
1. BWF WAV tests  →  BWF WAV implementation
2. Subfolder merge tests  →  Subfolder merge implementation
3. Scan benchmarks (baseline)  →  Parallel stat  →  benchmark confirms
4. Scan benchmarks (baseline)  →  Auto-scale workers  →  benchmark confirms
5. Scan benchmarks (baseline)  →  Batched DB writes  →  benchmark confirms
```

Each step: write test → confirm it fails → implement → confirm it passes → move on.

---

## Files Changed

| File | Change |
|---|---|
| `soul-importer/src/metadata.rs` | add `extract_wav_metadata()`, call it for `.wav`/`.wave` |
| `soul-importer/tests/bwf_metadata_tests.rs` | new: BWF WAV tests |
| `soul-importer/src/fuzzy.rs` | add subfolder match step in `find_or_create_album()` |
| `soul-importer/tests/entity_cache_tests.rs` | add subfolder merge tests |
| `soul-importer/tests/scanner_import_e2e_tests.rs` | add E2E subfolder + perf tests |
| `soul-importer/src/scanner.rs` | rayon parallel stat in Phase 0/1 |
| `soul-importer/src/library_scanner.rs` | auto-scale workers, batch=100, transactional flush |
| `soul-importer/Cargo.toml` | add `rayon` |
| `soul-importer/benches/scanner_bench.rs` | new: Criterion benchmarks for all three perf changes |
