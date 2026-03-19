# Incremental Scanning Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make library rescanning near-instant when only a few files changed, by adding directory-level skipping, filesystem watching, and quick hashing.

**Architecture:** Three-layer optimization: (1) skip unchanged directories by comparing dir mtime, (2) auto-trigger scans via OS filesystem watcher, (3) use 64KB quick hash instead of full-file SHA256 for new file dedup. Also removes dead "force reimport" code.

**Tech Stack:** Rust (soul-importer, soul-storage), SQLx migrations, notify crate (already in deps), Tauri commands, React/TypeScript UI, Playwright CDP E2E tests.

**Spec:** `docs/superpowers/specs/2026-03-19-incremental-scanning-design.md`

---

## File Map

### Files to Create
- `libraries/soul-storage/migrations/20260319000001_create_scanned_directories.sql`
- `libraries/soul-storage/migrations/20260319000002_add_quick_hash_to_tracks.sql`
- `libraries/soul-storage/src/scanned_directories/mod.rs`
- `applications/desktop/e2e-tests/tests/playwright/incremental-scan.spec.js`

### Files to Modify
- `libraries/soul-importer/src/library_scanner.rs` — remove force_metadata_refresh, add dir-level skipping, quick hash flow
- `libraries/soul-importer/src/scanner.rs` — add `scan_directory_incremental()`, export `SUPPORTED_EXTENSIONS`
- `libraries/soul-importer/src/hash_computer.rs` — add `compute_quick_hash()`
- `libraries/soul-importer/src/metadata.rs` — add `calculate_quick_hash()`
- `libraries/soul-importer/src/watcher.rs` — fix EventProcessor, use shared extensions, add callback pattern
- `libraries/soul-storage/src/lib.rs` — add `pub mod scanned_directories`
- `libraries/soul-storage/src/tracks/mod.rs` — add `find_by_quick_hash()`, update `create()` and `update_file_path()`
- `applications/desktop/src-tauri/src/app_state.rs` — add watcher field
- `applications/desktop/src-tauri/src/main.rs` — wire up watcher on startup
- `applications/desktop/src-tauri/src/library_settings.rs` — remove force_refresh params, add watcher lifecycle
- `applications/shared/src/components/settings/LibrarySettingsPage.tsx` — remove force reimport button
- `applications/shared/src/i18n/en-US.json` — remove forceRefresh keys
- `applications/shared/src/i18n/de.json` — remove forceRefresh keys
- `applications/shared/src/i18n/ja.json` — remove forceRefresh keys
- `applications/desktop/src/i18n/en-US.json` — remove forceRefresh keys

### Files to Delete
- `applications/desktop/e2e-tests/tests/playwright/force-reimport-progress.spec.js`

### Test Files to Modify
- `libraries/soul-importer/tests/progress_callback_tests.rs` — remove force_metadata_refresh test
- `libraries/soul-importer/tests/edge_case_tests.rs` — remove force_metadata_refresh test
- `applications/desktop/e2e-tests/tests/playwright/import-and-scan.spec.js` — remove `forceRefresh: true` usage

---

## Task 1: Remove Force Reimport — Dead Code Cleanup

**Files:**
- Modify: `libraries/soul-importer/src/library_scanner.rs:51,66,78-80,211,247`
- Modify: `applications/desktop/src-tauri/src/library_settings.rs:160,164,192,231,234,256`
- Modify: `applications/shared/src/components/settings/LibrarySettingsPage.tsx:238,241,627-641`
- Modify: `applications/shared/src/i18n/en-US.json:467-468`
- Modify: `applications/shared/src/i18n/de.json:507-508`
- Modify: `applications/shared/src/i18n/ja.json:467-468`
- Modify: `applications/desktop/src/i18n/en-US.json:217-218`
- Modify: `libraries/soul-importer/tests/progress_callback_tests.rs:247-268`
- Modify: `libraries/soul-importer/tests/edge_case_tests.rs:159-219`
- Modify: `applications/desktop/e2e-tests/tests/playwright/import-and-scan.spec.js:332`
- Delete: `applications/desktop/e2e-tests/tests/playwright/force-reimport-progress.spec.js`

- [ ] **Step 1: Remove `force_metadata_refresh` from LibraryScanner**

In `libraries/soul-importer/src/library_scanner.rs`:
- Remove field `force_metadata_refresh: bool` (line 51)
- Remove `force_metadata_refresh: false` from `new()` (line 66)
- Remove builder method `pub fn force_metadata_refresh(...)` (lines 78-81)
- Remove `let force_refresh = self.force_metadata_refresh;` (line 211)
- Change `if unchanged && !force_refresh {` to `if unchanged {` (line 247)

- [ ] **Step 2: Remove `force_refresh` from Tauri commands**

In `applications/desktop/src-tauri/src/library_settings.rs`:
- `rescan_library_source` (line 162): remove `force_refresh: Option<bool>` param, remove `.force_metadata_refresh(...)` call (line 192)
- `rescan_all_sources` (line 233): remove `force_refresh: Option<bool>` param, remove `.force_metadata_refresh(...)` call (line 256)
- Remove the doc comments about force_refresh (lines 160, 231)

- [ ] **Step 3: Remove Force Re-import button from UI**

In `LibrarySettingsPage.tsx`:
- Change `handleRescanAll` signature from `async (forceRefresh = false)` to `async ()` (line 238)
- Change invoke from `{ forceRefresh }` to `{}` (line 241)
- Delete the Force Re-import Tooltip+button block (lines 627-641)

- [ ] **Step 4: Remove i18n keys**

Remove `forceRefresh` and `forceRefreshDescription` from:
- `applications/shared/src/i18n/en-US.json` (lines 467-468)
- `applications/shared/src/i18n/de.json` (lines 507-508)
- `applications/shared/src/i18n/ja.json` (lines 467-468)
- `applications/desktop/src/i18n/en-US.json` (lines 217-218)

- [ ] **Step 5: Remove/update tests**

- Delete `applications/desktop/e2e-tests/tests/playwright/force-reimport-progress.spec.js`
- In `libraries/soul-importer/tests/progress_callback_tests.rs`: delete `test_progress_callback_with_force_refresh` test (lines 247-268+)
- In `libraries/soul-importer/tests/edge_case_tests.rs`: delete `test_force_metadata_refresh_reprocesses_unchanged_files` test (lines 159-219+) and update module doc comment (line 5)
- In `applications/desktop/e2e-tests/tests/playwright/import-and-scan.spec.js`: remove `forceRefresh: true` from line 332

- [ ] **Step 6: Verify compilation**

Run: `cargo check -p soul-importer && cargo check -p soul-player-desktop`
Expected: Compiles without errors

- [ ] **Step 7: Run existing tests**

Run: `cargo test -p soul-importer`
Expected: All remaining tests pass

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "refactor: remove force reimport dead code"
```

---

## Task 2: Directory-Level Mtime Skipping — Migration & Storage

**Files:**
- Create: `libraries/soul-storage/migrations/20260319000001_create_scanned_directories.sql`
- Create: `libraries/soul-storage/src/scanned_directories/mod.rs`
- Modify: `libraries/soul-storage/src/lib.rs:62` (add pub mod)

- [ ] **Step 1: Create migration**

Create `libraries/soul-storage/migrations/20260319000001_create_scanned_directories.sql`:
```sql
-- Directory-level mtime tracking for incremental scanning.
-- Stores the last-known mtime of each directory so unchanged
-- directories can be skipped entirely during rescan.

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

- [ ] **Step 2: Create scanned_directories storage module**

Create `libraries/soul-storage/src/scanned_directories/mod.rs` following the `scan_progress` pattern:
- `get_by_source(pool, source_id) -> Result<Vec<ScannedDirectory>>`
- `upsert_batch(pool, source_id, dirs: &[(String, i64, i64)])` — path, dir_mtime, file_count
- `delete_by_source(pool, source_id)`

All queries use `sqlx::query!()` compile-time macros. Return types use plain structs (not soul_core types since this is internal to scanning).

- [ ] **Step 3: Register module in lib.rs**

Add `pub mod scanned_directories;` to `libraries/soul-storage/src/lib.rs` after line 62 (after `pub mod scan_progress;`).

- [ ] **Step 4: Run migration and prepare SQLx cache**

```bash
cd libraries/soul-storage
sqlx migrate run --source migrations
cargo sqlx prepare -- --lib
```

- [ ] **Step 5: Verify compilation**

Run: `cargo check -p soul-storage`
Expected: Compiles without errors

- [ ] **Step 6: Commit**

```bash
git add libraries/soul-storage/
git commit -m "feat: add scanned_directories table and storage module"
```

---

## Task 3: Directory-Level Mtime Skipping — Scanner Integration

**Files:**
- Modify: `libraries/soul-importer/src/scanner.rs` — add `scan_directory_incremental()`, make `SUPPORTED_EXTENSIONS` pub
- Modify: `libraries/soul-importer/src/library_scanner.rs` — use incremental scan, fix soft-delete

- [ ] **Step 1: Make SUPPORTED_EXTENSIONS public in scanner.rs**

In `libraries/soul-importer/src/scanner.rs` line 8, change:
```rust
const SUPPORTED_EXTENSIONS: &[&str] = &[...];
```
to:
```rust
pub const SUPPORTED_EXTENSIONS: &[&str] = &[...];
```

- [ ] **Step 2: Add data types to scanner.rs**

Add to `libraries/soul-importer/src/scanner.rs`:
```rust
/// Info about a previously scanned directory, loaded from DB
#[derive(Debug, Clone)]
pub struct StoredDirInfo {
    pub dir_mtime: i64,
    pub file_count: i64,
}

/// Info about a directory after scanning, to be persisted to DB
#[derive(Debug, Clone)]
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
    /// File paths from unchanged directories (for soft-delete correctness)
    pub unchanged_dir_file_paths: Vec<String>,
}
```

- [ ] **Step 3: Implement `scan_directory_incremental()`**

Add method to `FileScanner` in `scanner.rs`:
```rust
pub fn scan_directory_incremental(
    &self,
    path: &Path,
    stored_dirs: &HashMap<String, StoredDirInfo>,
) -> Result<IncrementalScanResult> {
    // 1. Walk directory tree, collecting directory entries
    // 2. For each directory: stat it, compare mtime against stored_dirs
    // 3. If unchanged: increment unchanged_dir_count, skip file listing
    // 4. If changed/new: list audio files in that dir (non-recursive)
    // 5. Return IncrementalScanResult with changed files and dir info
    //
    // For soft-delete correctness: load known file paths from DB for
    // unchanged dirs into unchanged_dir_file_paths so they aren't
    // falsely marked as missing.
}
```

The key algorithm: use `WalkDir` with `min_depth(0)` to iterate directories. For each dir entry, get its mtime. If it matches the stored mtime, skip it (add its known files to `unchanged_dir_file_paths`). If it doesn't match or is new, list audio files in that single directory level using `std::fs::read_dir()`.

- [ ] **Step 4: Update `LibraryScanner::scan_source()` to use incremental scan**

In `library_scanner.rs`, replace the `scan_directory` call (lines 154-185) with:
1. Load stored dirs from DB: `soul_storage::scanned_directories::get_by_source(&self.pool, source.id)`
2. Build `HashMap<String, StoredDirInfo>` from results
3. Call `scan_directory_incremental()` inside `spawn_blocking`
4. Use `result.changed_files` instead of flat file list for Phase 1 filtering
5. Pre-populate `seen_paths` with `result.unchanged_dir_file_paths` to prevent false soft-deletes
6. After scan completes, call `soul_storage::scanned_directories::upsert_batch()` with `result.scanned_dirs`

- [ ] **Step 5: Add unit tests for incremental scanning**

Add tests to `scanner.rs` `#[cfg(test)]` module:
- `test_scan_directory_incremental_skips_unchanged` — create temp dir, scan once to get dir info, scan again with same info → 0 changed files
- `test_scan_directory_incremental_detects_new_dir` — scan with empty stored_dirs → all files returned
- `test_scan_directory_incremental_detects_changed_dir` — scan with wrong mtime in stored_dirs → files returned
- `test_unchanged_dir_file_paths_populated` — verify unchanged dirs populate file paths for soft-delete safety

- [ ] **Step 6: Verify compilation and tests**

```bash
cargo check -p soul-importer
cargo test -p soul-importer
```

- [ ] **Step 7: Commit**

```bash
git add libraries/soul-importer/ libraries/soul-storage/
git commit -m "feat: directory-level mtime skipping for incremental scanning"
```

---

## Task 4: Quick Hash — Migration, Storage & Hash Computer

**Files:**
- Create: `libraries/soul-storage/migrations/20260319000002_add_quick_hash_to_tracks.sql`
- Modify: `libraries/soul-storage/src/tracks/mod.rs` — add `find_by_quick_hash()`, update `create()`, `update_file_path()`
- Modify: `libraries/soul-importer/src/metadata.rs` — add `calculate_quick_hash()`
- Modify: `libraries/soul-importer/src/hash_computer.rs` — add `compute_quick_hash()`

- [ ] **Step 1: Create migration**

Create `libraries/soul-storage/migrations/20260319000002_add_quick_hash_to_tracks.sql`:
```sql
-- Quick hash (SHA256 of first 64KB) for fast new-file deduplication.
-- Avoids reading entire file content for relocation detection.

ALTER TABLE tracks ADD COLUMN quick_hash TEXT;
CREATE INDEX idx_tracks_quick_hash ON tracks(quick_hash);
```

- [ ] **Step 2: Add `find_by_quick_hash()` to tracks module**

In `libraries/soul-storage/src/tracks/mod.rs`, add near `find_by_hash()` (line 1624):
```rust
pub async fn find_by_quick_hash(pool: &SqlitePool, quick_hash: &str) -> Result<Option<Track>> {
    let track = sqlx::query_as!(
        Track,
        "SELECT * FROM tracks WHERE quick_hash = ? AND is_available = 1 LIMIT 1",
        quick_hash
    )
    .fetch_optional(pool)
    .await?;
    Ok(track)
}
```

- [ ] **Step 3: Update `create()` and `update_file_path()` in tracks module**

Update the `create()` function to include `quick_hash` in the INSERT.
Update `update_file_path()` to accept and set `quick_hash`:
```rust
pub async fn update_file_path(
    pool: &SqlitePool,
    track_id: &str,
    file_path: &str,
    source_id: i64,
    file_size: i64,
    file_mtime: i64,
    quick_hash: Option<&str>,
) -> Result<()>
```
Update all call sites in `library_scanner.rs`.

- [ ] **Step 4: Add `calculate_quick_hash()` to metadata.rs**

In `libraries/soul-importer/src/metadata.rs`, add:
```rust
pub fn calculate_quick_hash(path: &Path) -> Result<String, ImportError> {
    use sha2::{Digest, Sha256};
    use std::io::Read;

    let mut file = std::fs::File::open(path)
        .map_err(|e| ImportError::Io(e))?;
    let mut buffer = [0u8; 65536]; // 64KB
    let bytes_read = file.read(&mut buffer)
        .map_err(|e| ImportError::Io(e))?;
    let hash = Sha256::digest(&buffer[..bytes_read]);
    Ok(format!("{:x}", hash))
}
```

- [ ] **Step 5: Add `compute_quick_hash()` to hash_computer.rs**

In `libraries/soul-importer/src/hash_computer.rs`, add async wrapper:
```rust
pub async fn compute_quick_hash(file_path: &Path) -> Result<String> {
    let file_path_owned = file_path.to_path_buf();
    let file_path_for_log = file_path.to_path_buf();

    let hash_task =
        tokio::task::spawn_blocking(move || metadata::calculate_quick_hash(&file_path_owned));

    tokio::time::timeout(std::time::Duration::from_secs(10), hash_task)
        .await
        .map_err(|_| {
            ImportError::Metadata(format!(
                "Quick hash timeout (10s) for: {}",
                file_path_for_log.display()
            ))
        })?
        .map_err(|e| ImportError::Metadata(format!("Quick hash task failed: {}", e)))?
}
```

- [ ] **Step 6: Update scanner flow for quick hash**

In `library_scanner.rs` `process_extracted_file()`, change the new-file path (lines 562-600):
1. Compute quick hash instead of full hash: `hash_computer::compute_quick_hash(file_path).await?`
2. Look up by quick hash: `soul_storage::tracks::find_by_quick_hash(&self.pool, &quick_hash)`
3. If no match → import as new with quick_hash only (no content_hash)
4. If match → compute full hash to confirm, then relocate or import as new

- [ ] **Step 7: Run migration and prepare SQLx**

```bash
cd libraries/soul-storage
sqlx migrate run --source migrations
cargo sqlx prepare -- --lib
```

- [ ] **Step 8: Add unit tests**

In `hash_computer.rs` tests:
- `test_compute_quick_hash_success` — create temp file, compute quick hash, verify non-empty
- `test_compute_quick_hash_consistency` — same file → same hash
- `test_compute_quick_hash_differs_for_different_files` — different content → different hash

- [ ] **Step 9: Verify compilation and tests**

```bash
cargo check -p soul-importer -p soul-storage
cargo test -p soul-importer
```

- [ ] **Step 10: Commit**

```bash
git add libraries/
git commit -m "feat: quick hash (64KB) for fast new-file deduplication"
```

---

## Task 5: Wire Up Filesystem Watcher

**Files:**
- Modify: `libraries/soul-importer/src/watcher.rs` — fix EventProcessor, callback pattern, shared extensions
- Modify: `applications/desktop/src-tauri/src/app_state.rs` — add watcher field
- Modify: `applications/desktop/src-tauri/src/main.rs` — start watcher on startup
- Modify: `applications/desktop/src-tauri/src/library_settings.rs` — watcher lifecycle hooks

- [ ] **Step 1: Consolidate audio extensions in watcher.rs**

In `libraries/soul-importer/src/watcher.rs`, replace the local `is_audio_file()` function (lines 387-398) to use the shared constant:
```rust
fn is_audio_file(path: &Path) -> bool {
    crate::scanner::is_audio_file(path)
}
```

- [ ] **Step 2: Add callback pattern to EventProcessor**

In `watcher.rs`, add scan event types and callback:
```rust
pub enum ScanEvent {
    Started,
    Progress { processed: i64, total: i64, current_file: Option<String> },
    Complete,
}

pub type ScanEventCallback = Arc<dyn Fn(ScanEvent) + Send + Sync>;
```

Add `scan_callback: Option<ScanEventCallback>` field to `EventProcessor`.
Add builder method: `pub fn on_scan_event(mut self, cb: ScanEventCallback) -> Self`

- [ ] **Step 3: Fix EventProcessor to do one scan per flush**

In `watcher.rs` `flush_source()` (line 267): instead of iterating events and calling `handle_event()` for each, collapse all events into a single `scan_source()` call:
```rust
pub async fn flush_source(&mut self, source_id: i64) -> Result<()> {
    if let Some(events) = self.pending.remove(&source_id) {
        if events.is_empty() { return Ok(()); }

        // Skip if already scanning this source
        if self.scanning.contains(&source_id) {
            // Re-queue events for next flush
            self.pending.insert(source_id, events);
            return Ok(());
        }

        self.scanning.insert(source_id);
        let source = soul_storage::library_sources::get_by_id(&self.pool, source_id).await?;
        if let Some(source) = source {
            let scanner = LibraryScanner::new(
                self.pool.clone(), self.user_id.clone(), self.device_id.clone()
            );
            // Attach progress callback if scan_callback is set
            let scanner = if let Some(ref cb) = self.scan_callback {
                let cb = cb.clone();
                scanner.on_progress(Box::new(move |stats| {
                    cb(ScanEvent::Progress {
                        processed: stats.processed,
                        total: stats.total_files,
                        current_file: stats.current_file.clone(),
                    });
                }))
            } else { scanner };

            if let Some(ref cb) = self.scan_callback { cb(ScanEvent::Started); }
            match scanner.scan_source(&source).await {
                Ok(stats) => info!("Watcher scan: {} new, {} updated", stats.new_files, stats.updated_files),
                Err(e) => error!("Watcher scan failed: {}", e),
            }
            if let Some(ref cb) = self.scan_callback { cb(ScanEvent::Complete); }
        }
        self.scanning.remove(&source_id);
    }
    Ok(())
}
```

Add `scanning: HashSet<i64>` field to `EventProcessor`.

- [ ] **Step 4: Update `run_event_loop` to accept callback**

Update signature:
```rust
pub async fn run_event_loop(
    pool: SqlitePool,
    user_id: String,
    device_id: String,
    mut event_rx: mpsc::Receiver<(i64, WatcherEvent)>,
    scan_callback: Option<ScanEventCallback>,
)
```

- [ ] **Step 5: Add watcher to AppState**

In `applications/desktop/src-tauri/src/app_state.rs`, add:
```rust
use std::sync::Arc;
use soul_importer::watcher::LibraryWatcher;

// In AppState struct:
pub watcher: Option<Arc<LibraryWatcher>>,
```

Initialize as `None` in `new()`, set during startup.

- [ ] **Step 6: Wire up watcher in main.rs**

After AppState is created and managed (~line 2833), before audio pre-warm:
```rust
// Start filesystem watcher for automatic library scanning
if !is_playwright_test {
    let device_id = library_settings::get_device_id();
    let mut watcher = soul_importer::watcher::LibraryWatcher::new(
        pool.clone(), "1".to_string(), device_id.clone()
    );
    if let Err(e) = watcher.start_watching().await {
        tracing::warn!("[Startup] Failed to start library watcher: {}", e);
    }
    let rx = watcher.take_event_receiver();
    let watcher_arc = Arc::new(watcher);
    // Store in managed state for Tauri commands
    app_handle.manage(watcher_arc.clone());

    if let Some(rx) = rx {
        let pool_clone = pool.clone();
        let app_clone = app_handle.clone();
        tauri::async_runtime::spawn(async move {
            let callback = Arc::new(move |event| {
                match event {
                    soul_importer::watcher::ScanEvent::Started => { let _ = app_clone.emit("scan-started", ()); }
                    soul_importer::watcher::ScanEvent::Progress { processed, total, current_file } => {
                        let _ = app_clone.emit("scan-progress", serde_json::json!({
                            "processed": processed, "total": total, "currentFile": current_file
                        }));
                    }
                    soul_importer::watcher::ScanEvent::Complete => { let _ = app_clone.emit("scan-complete", ()); }
                }
            });
            soul_importer::watcher::run_event_loop(
                pool_clone, "1".to_string(), device_id, rx, Some(callback)
            ).await;
        });
    }
    tracing::info!("[Startup] Library watcher started");
} else {
    tracing::info!("[Startup] Skipping library watcher in test environment");
}
```

- [ ] **Step 7: Add watcher lifecycle to library_settings.rs**

In `add_library_source()`, after DB insert, watch the new source:
```rust
if let Some(watcher) = app.try_state::<Arc<soul_importer::watcher::LibraryWatcher>>() {
    let _ = watcher.watch_source(&soul_core::types::LibrarySource { /* from created source */ }).await;
}
```

In `remove_library_source()`, before DB delete, unwatch:
```rust
if let Some(watcher) = app.try_state::<Arc<soul_importer::watcher::LibraryWatcher>>() {
    let _ = watcher.unwatch_source(source_id).await;
}
```

Add `app: AppHandle` parameter to both commands if not already present.

- [ ] **Step 8: Verify compilation**

```bash
cargo check -p soul-importer -p soul-player-desktop
```

- [ ] **Step 9: Commit**

```bash
git add libraries/soul-importer/ applications/desktop/src-tauri/
git commit -m "feat: wire up filesystem watcher for automatic library scanning"
```

---

## Task 6: E2E and UI Tests

**Files:**
- Create: `applications/desktop/e2e-tests/tests/playwright/incremental-scan.spec.js`

- [ ] **Step 1: Create incremental-scan.spec.js**

Create `applications/desktop/e2e-tests/tests/playwright/incremental-scan.spec.js` with these tests:

**Test 1: Force reimport button removed**
- Navigate to Settings > Library
- Assert `data-testid="force-reimport-button"` does NOT exist
- Assert `data-testid="rescan-all-button"` DOES exist

**Test 2: Rescan skips unchanged library**
- Seed DB with tracks + `scanned_directories` entries (matching dir mtimes)
- Trigger `rescan_all_sources` IPC
- Listen for `scan-complete` event
- Verify scan stats: 0 new, 0 updated, 0 removed

**Test 3: Rescan detects new album**
- Seed DB with existing tracks
- Create new WAV files in a new subdirectory of the test music folder
- Trigger `rescan_all_sources` IPC
- Wait for `scan-complete`
- Query DB: verify new tracks exist
- Verify scan stats show correct new count

**Test 4: Rescan detects deleted files**
- Seed DB with tracks pointing to WAV files
- Delete some WAV files from disk
- Trigger rescan
- Verify deleted tracks are marked `is_available = 0`

**Test 5: Rescan button triggers scan with progress UI**
- Navigate to Library Settings
- Click "Rescan All" button
- Listen for `scan-started` event
- Listen for `scan-progress` events (verify at least one fires)
- Listen for `scan-complete` event

**Test 6: Quick hash dedup — moved file detected as relocation**
- Seed DB with track that has a `quick_hash`
- Move the WAV file to a different directory within the watched folder
- Trigger rescan
- Verify track path updated (not duplicated)

**Test 7: Rescan All button shows correct stats after adding files**
- Create new WAV files in watched folder
- Click "Rescan All" button via UI
- Verify progress shows correct new/total counts in scan-progress events

Follow the existing test patterns from `import-and-scan.spec.js` for DB seeding, IPC calls, and event listening. Use `playwright-global-setup.js` seed data patterns.

- [ ] **Step 2: Run the E2E tests**

```bash
cd applications/desktop/e2e-tests
npx playwright test --config playwright.cdp.config.js tests/playwright/incremental-scan.spec.js
```

Expected: All tests pass

- [ ] **Step 3: Run full Playwright suite to check for regressions**

```bash
cd applications/desktop/e2e-tests
npx playwright test --config playwright.cdp.config.js
```

Expected: No regressions from removed force-reimport tests, all other tests pass

- [ ] **Step 4: Commit**

```bash
git add applications/desktop/e2e-tests/
git commit -m "test: E2E tests for incremental scanning and UI verification"
```

---

## Task 7: Final Integration Verification

- [ ] **Step 1: Run full pre-commit checks**

```bash
cargo xtask check precommit
```

Expected: All checks pass (fmt, clippy, test, typescript, lint)

- [ ] **Step 2: Run SQLx prepare to ensure cache is up to date**

```bash
cd libraries/soul-storage
cargo sqlx prepare -- --lib
git add .sqlx/
```

- [ ] **Step 3: Manual smoke test (if possible)**

1. Start the desktop app: `cargo xtask dev desktop`
2. Go to Settings > Library — verify no "Force Re-import" button
3. Click "Rescan All" — verify it completes quickly
4. Add a new album folder to a watched directory
5. Verify the watcher picks it up automatically within ~5 seconds
6. Check that new tracks appear in the library

- [ ] **Step 4: Final commit with any fixes**

```bash
git add -A
git commit -m "chore: finalize incremental scanning — SQLx cache update"
```
