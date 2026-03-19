# Incremental Scanning Optimization

**Date:** 2026-03-19
**Status:** Approved

## Problem

When rescanning a library after adding one album to a watched folder, the scanner takes ~1 hour because it:
1. Walks every directory and stats every file (even unchanged ones)
2. Computes full SHA256 hash of entire file content for every new file
3. Has no automatic detection — user must manually click "Rescan"

## Solution

Four changes, implemented in order:

1. **Remove force reimport** — dead code cleanup
2. **Phase 1: Directory-level mtime skipping** — skip unchanged directories entirely
3. **Phase 2: Wire up filesystem watcher** — automatic background detection
4. **Phase 3: Quick hash** — 64KB hash instead of full-file SHA256 for new file dedup

## Detailed Design

### 1. Remove Force Reimport

Remove the `force_metadata_refresh` code path entirely.

**Files to modify:**
- `libraries/soul-importer/src/library_scanner.rs` — remove `force_metadata_refresh` field and builder method
- `applications/desktop/src-tauri/src/library_settings.rs` — remove `force_refresh` parameter from `rescan_library_source` and `rescan_all_sources` commands
- `applications/shared/src/components/settings/LibrarySettingsPage.tsx` — remove "Force Re-import" button and `handleRescanAll(true)` call
- `applications/shared/src/i18n/en-US.json` — remove `forceRefresh`, `forceRefreshDescription` keys
- `applications/shared/src/i18n/de.json` — remove same keys
- `applications/shared/src/i18n/ja.json` — remove same keys

**Tests to update/remove:**
- `libraries/soul-importer/tests/progress_callback_tests.rs` — remove `force_metadata_refresh` usage
- `libraries/soul-importer/tests/edge_case_tests.rs` — remove `force_metadata_refresh` usage
- Delete `applications/desktop/e2e-tests/tests/playwright/force-reimport-progress.spec.js` if it exists

### 2. Phase 1: Directory-Level Mtime Skipping

#### New Migration (separate file)

```sql
CREATE TABLE IF NOT EXISTS scanned_directories (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    library_source_id INTEGER NOT NULL REFERENCES library_sources(id) ON DELETE CASCADE,
    path TEXT NOT NULL,
    dir_mtime INTEGER NOT NULL DEFAULT 0,
    file_count INTEGER NOT NULL DEFAULT 0,
    last_scanned_at INTEGER NOT NULL DEFAULT 0,
    UNIQUE(library_source_id, path)
);

CREATE INDEX idx_scanned_dirs_source ON scanned_directories(library_source_id);
```

#### Data Types

```rust
/// Info about a previously scanned directory, loaded from DB
pub struct StoredDirInfo {
    pub dir_mtime: i64,
    pub file_count: i64,
}

/// Info about a directory after scanning, to be persisted to DB
pub struct ScannedDirInfo {
    pub path: String,
    pub dir_mtime: i64,
    pub file_count: i64,
}

/// Result of an incremental directory scan
pub struct IncrementalScanResult {
    pub changed_files: Vec<PathBuf>,
    pub unchanged_dir_count: i64,
    pub scanned_dirs: Vec<ScannedDirInfo>,
}
```

#### Scanner Changes

New method on `FileScanner`: `scan_directory_incremental()` that accepts a `HashMap<String, StoredDirInfo>` of previously scanned directories and returns `IncrementalScanResult`.

Algorithm:
1. Walk directory tree using `walkdir` but only check directory entries
2. For each directory, call `fs::metadata()` to get its mtime
3. Compare against stored `dir_mtime` from DB
4. If unchanged → skip (don't list files inside)
5. If changed or new → list all audio files in that directory (non-recursive, just that level)
6. Return collected files for processing

**Important:** This method is blocking (uses `walkdir` + `fs::metadata`) and MUST be called inside `tokio::task::spawn_blocking`, same as the existing `scan_directory()` usage in `library_scanner.rs`.

`LibraryScanner::scan_source()` changes:
- Load `scanned_directories` for this source from DB before scanning
- Use `scan_directory_incremental()` instead of `scan_directory()` (inside `spawn_blocking`)
- After processing, upsert `scanned_directories` with new mtime/file_count values

**Soft-delete correctness:** The soft-delete pass checks `seen_paths` against `existing_tracks` to find missing files. With directory skipping, files from unchanged directories won't be in `seen_paths`. To prevent false soft-deletes:
- Pre-populate `seen_paths` with all known file paths from unchanged directories (loaded from DB at scan start)
- Only files from changed directories can be newly missing
- This ensures unchanged-directory files are never falsely marked unavailable

#### Storage Layer

New module `soul_storage::scanned_directories` (add `pub mod scanned_directories;` to `soul-storage/src/lib.rs`):
- `get_by_source(pool, source_id) -> Vec<ScannedDirectory>`
- `upsert_batch(pool, source_id, dirs: &[ScannedDirUpdate])` — bulk upsert after scan
- `delete_by_source(pool, source_id)` — clear when source is removed (CASCADE handles this)

#### SQLx Prepare

After migration, run `cargo sqlx prepare` in `libraries/soul-storage`.

### 3. Phase 2: Wire Up Filesystem Watcher

#### AppState Changes

Add to `AppState`:
```rust
/// Filesystem watcher for automatic library scanning
pub watcher: Option<Arc<soul_importer::watcher::LibraryWatcher>>,
```

The `Arc<LibraryWatcher>` is needed so Tauri commands can call `watch_source()`/`unwatch_source()` when library sources are added/removed.

#### Startup Integration (`main.rs`)

After AppState is created and managed, before audio pre-warm:
1. Create `LibraryWatcher` with pool, user_id, device_id
2. Call `start_watching()` to watch all enabled sources
3. Take the event receiver
4. Spawn `run_event_loop()` as a background Tokio task
5. Guard with `PLAYWRIGHT_TEST_DIR` env var check (skip in tests)

```rust
if !is_playwright_test {
    let mut watcher = LibraryWatcher::new(pool.clone(), state.user_id.clone(), device_id.clone());
    watcher.start_watching().await.ok();
    if let Some(rx) = watcher.take_event_receiver() {
        let pool_clone = pool.clone();
        let user_id = state.user_id.clone();
        let device_id_clone = device_id.clone();
        tauri::async_runtime::spawn(async move {
            run_event_loop(pool_clone, user_id, device_id_clone, rx).await;
        });
    }
    // Store Arc<LibraryWatcher> in managed state for Tauri commands
}
```

#### EventProcessor Changes

**Platform-agnostic event emission:** The `EventProcessor` lives in `libraries/soul-importer` which MUST NOT depend on platform crates (per CLAUDE.md). Instead of adding `AppHandle` as a field, use a callback pattern (same as `LibraryScanner`'s `ProgressCallback`):

```rust
pub type ScanEventCallback = Box<dyn Fn(ScanEvent) + Send + Sync>;

pub enum ScanEvent {
    Started,
    Progress { processed: i64, total: i64, current_file: Option<String> },
    Complete,
}
```

The Tauri layer in `main.rs` provides the callback that emits events via `AppHandle`.

**Scan deduplication:** `flush_source()` currently iterates events and calls `handle_event()` for each one. Refactor to collapse all events into a single `scan_source()` call per flush. Add `AtomicBool` per source_id to prevent overlapping scans.

#### Audio Extension List Consolidation

The watcher's `is_audio_file()` has a different extension list from `scanner.rs`. Consolidate into a single shared constant in `scanner.rs` (which already has `SUPPORTED_EXTENSIONS`) and import it in `watcher.rs`.

#### Lifecycle Hooks

In `library_settings.rs`:
- `add_library_source` → after DB insert, call `watcher.watch_source()` on the new source
- `remove_library_source` → call `watcher.unwatch_source()` before DB delete

Access the watcher via `State<'_, Arc<LibraryWatcher>>` or from `AppState`.

### 4. Phase 3: Quick Hash for New File Dedup

#### New Migration (separate file)

```sql
ALTER TABLE tracks ADD COLUMN quick_hash TEXT;
CREATE INDEX idx_tracks_quick_hash ON tracks(quick_hash);
```

Existing tracks get `quick_hash = NULL`. It will be populated on next scan that touches the file.

#### Hash Computer Changes

Add sync function to `metadata.rs` (following existing pattern where sync logic lives in `metadata.rs`):
```rust
pub fn calculate_quick_hash(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    let mut buffer = [0u8; 65536]; // 64KB
    let bytes_read = file.read(&mut buffer)?;
    let hash = Sha256::digest(&buffer[..bytes_read]);
    Ok(format!("{:x}", hash))
}
```

Add async wrapper to `hash_computer.rs` (following existing `compute_file_hash` pattern):
```rust
pub async fn compute_quick_hash(file_path: &Path) -> Result<String> {
    let file_path_owned = file_path.to_path_buf();
    let hash_task = tokio::task::spawn_blocking(move || {
        metadata::calculate_quick_hash(&file_path_owned)
    });
    tokio::time::timeout(Duration::from_secs(10), hash_task)
        .await
        .map_err(|_| ImportError::Metadata("Quick hash timeout".into()))?
        .map_err(|e| ImportError::Metadata(format!("Quick hash task failed: {}", e)))?
}
```

#### Scanner Flow Change

In `process_extracted_file()` for new files (no existing track):
1. Compute quick hash (~1ms)
2. Look up `quick_hash` in DB via `soul_storage::tracks::find_by_quick_hash()`
3. If no match → truly new file, import with quick hash only (skip full hash)
4. If match found → compute full hash to confirm relocation (avoid false positives)
5. If full hash matches → relocated file, update path
6. If full hash doesn't match → different file with same first 64KB (rare), import as new

**Note on relocation detection for quick-hash-only files:** Files imported after this change will only have `quick_hash`, not `content_hash`. If later moved, relocation detection uses quick hash first (fast path). The full `content_hash` field remains nullable. This is an acceptable trade-off — quick hash is sufficient for most relocation scenarios, and false positives are caught by the full-hash confirmation step.

#### Storage Layer Changes

Add to `soul_storage::tracks`:
- `find_by_quick_hash(pool, hash) -> Option<Track>`
- Update `create()` to accept optional `quick_hash` parameter
- Update `update_file_path()` signature to also accept optional `quick_hash` — update all call sites in `library_scanner.rs`

## Testing

### Unit Tests (Rust)

**Phase 1 — Directory skipping:**
- `test_scan_directory_incremental_skips_unchanged` — mock dir with same mtime, verify 0 files returned
- `test_scan_directory_incremental_detects_new_dir` — new directory not in DB, verify files returned
- `test_scan_directory_incremental_detects_changed_dir` — dir with different mtime, verify files returned
- `test_scanned_directories_upsert_batch` — verify DB round-trip
- `test_soft_delete_does_not_affect_unchanged_dirs` — files from skipped dirs are NOT marked unavailable

**Phase 3 — Quick hash:**
- `test_quick_hash_consistency` — same file produces same quick hash
- `test_quick_hash_differs_from_full` — quick hash != full hash (different inputs)
- `test_quick_hash_different_files` — different files produce different quick hashes
- `test_find_by_quick_hash` — DB lookup works

### E2E Tests (Playwright CDP)

New spec file: `applications/desktop/e2e-tests/tests/playwright/incremental-scan.spec.js`

**Test 1: Rescan skips unchanged library**
- Seed DB with tracks + scanned_directories
- Trigger `rescan_all_sources` IPC
- Verify scan stats show 0 new / 0 updated / 0 removed (not timing-based)
- Verify existing tracks are unchanged

**Test 2: Rescan detects new album**
- Seed DB with existing tracks
- Create new WAV files in a new subdirectory of the watched folder
- Trigger `rescan_all_sources` IPC
- Verify new tracks appear in DB
- Verify `scan-progress` event shows correct count

**Test 3: Rescan detects deleted files**
- Seed DB with tracks pointing to files
- Delete some of the seeded WAV files
- Trigger rescan
- Verify tracks marked as unavailable

**Test 4: Force reimport button removed**
- Navigate to Library Settings page
- Verify "Force Re-import" button does NOT exist
- Verify "Rescan All" button still exists

**Test 5: Watcher auto-detects new files**
- Test as Rust integration test (not E2E) since watcher is disabled under `PLAYWRIGHT_TEST_DIR`
- Create `LibraryWatcher` + `EventProcessor` in test, add file to watched dir, verify scan triggers

### UI Tests

**In `incremental-scan.spec.js`:**

**Test 6: Rescan button triggers scan with progress**
- Navigate to Library Settings
- Click "Rescan All" button
- Verify scan-progress events are emitted
- Verify scan completes (scan-complete event)

**Test 7: Rescan button shows correct stats after adding files**
- Create new WAV files in watched folder
- Click "Rescan All"
- Verify the progress shows correct new/total counts

## Migration Order

1. Remove force reimport (no migration needed)
2. Add `scanned_directories` migration (separate migration file)
3. Add `quick_hash` column migration (separate migration file)

Separate migration files because this project uses timestamped migrations with SHA-384 checksums.

## Performance Expectations

| Scenario | Before | After |
|----------|--------|-------|
| Rescan, nothing changed (10k files, 500 dirs) | ~60s (10k stats) | ~1s (500 dir mtimes) |
| Added 1 album (12 tracks) | ~60s (10k stats + 12 hashes) | ~1.5s (500 dir mtimes + 12 stats + 12 quick hashes) |
| Added 500 new large FLACs | ~60s stats + ~500s full hashes | ~1s dir check + ~1s quick hashes + ~30s metadata (8x parallel) |
| Watcher: drop album into folder | Manual rescan required | Auto-detected in ~2-5s |
