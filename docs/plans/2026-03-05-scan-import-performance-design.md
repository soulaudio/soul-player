# Scan/Import Performance Optimization — Design

**Date:** 2026-03-05
**Target:** 100K–500K track libraries in minutes, not hours

## Problem

Current scan/import is fully sequential. Per-file cost:
1. `std::fs::metadata()` — sync I/O
2. `lofty` tag extraction — ~1-5ms/file
3. Fuzzy matching — loads ALL artists from DB per file (O(n²))
4. Individual DB INSERT per track
5. 3-4 DB writes per file for progress counters
6. SHA256 hash of full file content for new files

At 100K files: estimated 1-3 hours. At 500K: 6-15 hours.

## Solution: Approach A — Parallel + Cache

### 1. Parallel Metadata Extraction
- `tokio::sync::Semaphore`-gated concurrent extraction (default 8 workers)
- Filter unchanged files (mtime+size) BEFORE any I/O
- `spawn_blocking` for lofty (already exists, just run multiple in flight)

### 2. In-Memory Entity Cache
- Load all artists/albums/genres once at scan start
- `HashMap<String, EntityId>` keyed by normalized name → O(1) lookup
- Levenshtein only on cache miss
- New entities added to cache immediately

### 3. Batched DB Writes
- Accumulate tracks in buffer (batch size ~100-500)
- Flush in single transaction with multi-row INSERT
- Same for genre associations and library source updates

### 4. Batched Progress Updates
- Accumulate counts in memory
- Flush to DB + emit frontend event every 100 files or 500ms

### 5. Deferred Hash Computation
- Skip hashing on initial scan (nothing to relocate against)
- Only hash truly new files on subsequent scans

## Files Changed

| File | Change |
|------|--------|
| `library_scanner.rs` | Parallel loop, batched progress |
| `fuzzy.rs` | `EntityCache` struct |
| `file_processor.rs` | Batch-aware import |
| `soul-storage/tracks.rs` | `create_batch()` |
| `soul-storage/genres.rs` | `add_to_tracks_batch()` |
| `soul-storage/scan_progress.rs` | `update_counts()` bulk |

## Expected Impact

| Library | Current | After |
|---------|---------|-------|
| 5K | ~2-5 min | ~15-30s |
| 100K | ~1-3 hrs | ~2-5 min |
| 500K | ~6-15 hrs | ~10-25 min |
