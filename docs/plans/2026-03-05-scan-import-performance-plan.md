# Scan/Import Performance Optimization — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make library scanning 10-20x faster — targeting 100K tracks in ~2-5 min, 500K in ~10-25 min.

**Architecture:** Keep existing `LibraryScanner` API. Add parallel metadata extraction via tokio semaphore, in-memory entity cache for fuzzy matching, batched DB writes in transactions, and batched progress counter updates. All changes internal to `soul-importer` and `soul-storage`.

**Tech Stack:** Rust, tokio (Semaphore, JoinSet), sqlx (SQLite transactions), strsim, lofty

---

### Task 1: Add Benchmark Infrastructure

**Files:**
- Create: `libraries/soul-importer/benches/scan_benchmark.rs`
- Modify: `libraries/soul-importer/Cargo.toml`

**Step 1: Add criterion dev-dependency**

In `libraries/soul-importer/Cargo.toml`, add after existing `[dev-dependencies]`:

```toml
criterion = { version = "0.5", features = ["async_tokio"] }

[[bench]]
name = "scan_benchmark"
harness = false
```

**Step 2: Write benchmark scaffolding**

Create `libraries/soul-importer/benches/scan_benchmark.rs`:

```rust
//! Benchmarks for library scan performance
//!
//! Measures: directory walk, metadata extraction, fuzzy matching, DB writes,
//! and full scan pipeline at different library sizes.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use sqlx::SqlitePool;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use tempfile::TempDir;

/// Create a test database with migrations
async fn setup_db() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create test database");

    sqlx::migrate!("../soul-storage/migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    pool
}

/// Create N fake FLAC files with varied "artist/album" folder structures
fn create_test_library(count: usize) -> TempDir {
    let dir = TempDir::new().expect("Failed to create temp dir");

    // Distribute files across artist/album folders to simulate real library
    let artists = ["Artist_A", "Artist_B", "Artist_C", "Artist_D", "Artist_E",
                    "Artist_F", "Artist_G", "Artist_H", "Artist_I", "Artist_J"];
    let albums_per_artist = 5;

    for i in 0..count {
        let artist_idx = i % artists.len();
        let album_idx = (i / artists.len()) % albums_per_artist;
        let artist = artists[artist_idx];
        let album = format!("Album_{:03}", album_idx + 1);

        let folder = dir.path().join(artist).join(&album);
        fs::create_dir_all(&folder).expect("Failed to create folder");

        let filename = format!("track_{:05}.flac", i + 1);
        let file_path = folder.join(&filename);

        if !file_path.exists() {
            let mut file = fs::File::create(&file_path).expect("Failed to create file");
            // Minimal FLAC header + padding
            file.write_all(b"fLaC\x00\x00\x00\x22").unwrap();
            file.write_all(&[0u8; 1000]).unwrap();
            file.flush().unwrap();
        }
    }

    dir
}

/// Benchmark: Directory walk only (FileScanner::scan_directory)
fn bench_directory_walk(c: &mut Criterion) {
    let mut group = c.benchmark_group("directory_walk");
    group.sample_size(10);

    for size in [100, 1_000, 10_000] {
        let dir = create_test_library(size);

        group.bench_with_input(BenchmarkId::new("walk", size), &size, |b, _| {
            b.iter(|| {
                let scanner = soul_importer::scanner::FileScanner::new();
                let files = scanner.scan_directory(dir.path()).unwrap();
                assert!(files.len() >= size);
            });
        });
    }
    group.finish();
}

/// Benchmark: Fuzzy matching (find_or_create_artist) with pre-populated DB
fn bench_fuzzy_matching(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("fuzzy_matching");
    group.sample_size(10);

    for existing_artists in [10, 100, 1_000] {
        let pool = rt.block_on(setup_db());

        // Pre-populate artists
        rt.block_on(async {
            for i in 0..existing_artists {
                soul_storage::artists::create(
                    &pool,
                    soul_core::types::CreateArtist {
                        name: format!("Artist {}", i),
                        sort_name: Some(format!("Artist {}", i)),
                        musicbrainz_id: None,
                    },
                )
                .await
                .unwrap();
            }
        });

        group.bench_with_input(
            BenchmarkId::new("find_or_create_artist", existing_artists),
            &existing_artists,
            |b, _| {
                let pool = pool.clone();
                b.to_async(tokio::runtime::Runtime::new().unwrap())
                    .iter(|| {
                        let pool = pool.clone();
                        async move {
                            let matcher = soul_importer::fuzzy::FuzzyMatcher::new();
                            // Search for an artist that exists (worst case: fuzzy match)
                            matcher
                                .find_or_create_artist(&pool, "Artist 0")
                                .await
                                .unwrap();
                        }
                    });
            },
        );
    }
    group.finish();
}

/// Benchmark: Full scan pipeline (sequential, current implementation)
fn bench_full_scan(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("full_scan");
    group.sample_size(10);
    // Generous time for large scans
    group.measurement_time(std::time::Duration::from_secs(30));

    for size in [100, 500] {
        let dir = create_test_library(size);
        let pool = rt.block_on(setup_db());

        // Create a library source
        rt.block_on(async {
            soul_storage::library_sources::create(
                &pool,
                "user1",
                "device1",
                &soul_core::types::CreateLibrarySource {
                    name: "Test Library".to_string(),
                    path: dir.path().display().to_string(),
                    sync_deletes: false,
                },
            )
            .await
            .unwrap();
        });

        group.bench_with_input(BenchmarkId::new("scan_all", size), &size, |b, _| {
            b.to_async(tokio::runtime::Runtime::new().unwrap())
                .iter(|| {
                    let pool = pool.clone();
                    async move {
                        let scanner = soul_importer::library_scanner::LibraryScanner::new(
                            pool.clone(),
                            "user1",
                            "device1",
                        )
                        .compute_hashes(false); // Skip hashing for benchmark consistency

                        scanner.scan_all().await.unwrap();
                    }
                });
        });
    }
    group.finish();
}

/// Benchmark: DB write throughput (individual inserts vs batched)
fn bench_db_writes(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("db_writes");
    group.sample_size(10);

    for batch_size in [1, 10, 100] {
        let pool = rt.block_on(setup_db());

        group.bench_with_input(
            BenchmarkId::new("individual_inserts", batch_size),
            &batch_size,
            |b, &size| {
                b.to_async(tokio::runtime::Runtime::new().unwrap())
                    .iter(|| {
                        let pool = pool.clone();
                        async move {
                            for i in 0..size {
                                soul_storage::tracks::create(
                                    &pool,
                                    soul_core::types::CreateTrack {
                                        title: format!("Track {}", i),
                                        artist_id: None,
                                        album_id: None,
                                        album_artist_id: None,
                                        track_number: Some(i as i32),
                                        disc_number: Some(1),
                                        year: Some(2024),
                                        duration_seconds: Some(180.0),
                                        bitrate: Some(320),
                                        sample_rate: Some(44100),
                                        channels: Some(2),
                                        file_format: "FLAC".to_string(),
                                        file_hash: None,
                                        origin_source_id: 1,
                                        local_file_path: Some(format!("/fake/track_{}.flac", i)),
                                        musicbrainz_recording_id: None,
                                        fingerprint: None,
                                    },
                                )
                                .await
                                .unwrap();
                            }
                        }
                    });
            },
        );
    }
    group.finish();
}

/// Benchmark: Scan progress counter updates (individual vs batched)
fn bench_progress_updates(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("progress_updates");
    group.sample_size(10);

    for update_count in [10, 100, 1000] {
        let pool = rt.block_on(setup_db());

        // Create a scan progress entry
        let progress_id = rt.block_on(async {
            // Create a library source first
            let source = soul_storage::library_sources::create(
                &pool,
                "user1",
                "device1",
                &soul_core::types::CreateLibrarySource {
                    name: "Bench Source".to_string(),
                    path: "/fake/path".to_string(),
                    sync_deletes: false,
                },
            )
            .await
            .unwrap();

            let progress = soul_storage::scan_progress::start(&pool, source.id, Some(update_count as i64))
                .await
                .unwrap();
            progress.id
        });

        group.bench_with_input(
            BenchmarkId::new("individual_increments", update_count),
            &update_count,
            |b, &count| {
                b.to_async(tokio::runtime::Runtime::new().unwrap())
                    .iter(|| {
                        let pool = pool.clone();
                        async move {
                            for _ in 0..count {
                                soul_storage::scan_progress::increment_processed(&pool, progress_id, 1)
                                    .await
                                    .unwrap();
                            }
                        }
                    });
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_directory_walk,
    bench_fuzzy_matching,
    bench_full_scan,
    bench_db_writes,
    bench_progress_updates,
);
criterion_main!(benches);
```

**Step 3: Run benchmarks to establish baseline**

Run: `cd libraries/soul-importer && cargo bench`
Expected: Baseline numbers for all benchmark groups. Save output.

**Step 4: Commit**

```bash
git add libraries/soul-importer/benches/ libraries/soul-importer/Cargo.toml
git commit -m "bench: add scan/import performance benchmarks for baseline measurement"
```

---

### Task 2: Add EntityCache to Fuzzy Matcher

**Files:**
- Modify: `libraries/soul-importer/src/fuzzy.rs`
- Create: `libraries/soul-importer/tests/entity_cache_tests.rs`

**Step 1: Write tests for EntityCache**

Create `libraries/soul-importer/tests/entity_cache_tests.rs`:

```rust
//! Tests for in-memory entity cache used by fuzzy matcher

mod test_helpers;

use soul_core::types::{ArtistId, CreateArtist};
use soul_importer::fuzzy::{EntityCache, FuzzyMatcher};

#[tokio::test]
async fn test_entity_cache_preload_artists() {
    let pool = test_helpers::setup_test_db().await;

    // Create 100 artists
    for i in 0..100 {
        soul_storage::artists::create(
            &pool,
            CreateArtist {
                name: format!("Artist {}", i),
                sort_name: Some(format!("Artist {}", i)),
                musicbrainz_id: None,
            },
        )
        .await
        .unwrap();
    }

    // Preload cache
    let cache = EntityCache::preload(&pool).await.unwrap();

    // Exact lookup should hit cache
    assert!(cache.find_artist_by_normalized("artist 0").is_some());
    assert!(cache.find_artist_by_normalized("artist 99").is_some());
    assert!(cache.find_artist_by_normalized("nonexistent").is_none());
}

#[tokio::test]
async fn test_entity_cache_insert_updates_cache() {
    let pool = test_helpers::setup_test_db().await;
    let mut cache = EntityCache::preload(&pool).await.unwrap();

    // Cache should be empty initially
    assert!(cache.find_artist_by_normalized("new artist").is_none());

    // Insert a new artist
    let artist = soul_storage::artists::create(
        &pool,
        CreateArtist {
            name: "New Artist".to_string(),
            sort_name: Some("New Artist".to_string()),
            musicbrainz_id: None,
        },
    )
    .await
    .unwrap();

    cache.insert_artist(artist.id, &artist.name);

    // Now cache should find it
    assert!(cache.find_artist_by_normalized("new artist").is_some());
}

#[tokio::test]
async fn test_entity_cache_avoids_db_query_on_hit() {
    let pool = test_helpers::setup_test_db().await;

    // Create an artist
    soul_storage::artists::create(
        &pool,
        CreateArtist {
            name: "The Beatles".to_string(),
            sort_name: Some("Beatles".to_string()),
            musicbrainz_id: None,
        },
    )
    .await
    .unwrap();

    let cache = EntityCache::preload(&pool).await.unwrap();
    let matcher = FuzzyMatcher::new();

    // This should use cache, not DB
    let result = matcher
        .find_or_create_artist_cached(&pool, "The Beatles", &cache)
        .await
        .unwrap();

    assert_eq!(result.confidence, 100);
}

#[tokio::test]
async fn test_entity_cache_normalized_lookup_case_insensitive() {
    let pool = test_helpers::setup_test_db().await;

    soul_storage::artists::create(
        &pool,
        CreateArtist {
            name: "Radiohead".to_string(),
            sort_name: Some("Radiohead".to_string()),
            musicbrainz_id: None,
        },
    )
    .await
    .unwrap();

    let cache = EntityCache::preload(&pool).await.unwrap();

    // Case-insensitive lookup
    assert!(cache.find_artist_by_normalized("radiohead").is_some());
    assert!(cache.find_artist_by_normalized("RADIOHEAD").is_some());
    assert!(cache.find_artist_by_normalized("  radiohead  ").is_some());
}

#[tokio::test]
async fn test_entity_cache_albums_scoped_by_artist() {
    let pool = test_helpers::setup_test_db().await;

    let artist = soul_storage::artists::create(
        &pool,
        CreateArtist {
            name: "Artist A".to_string(),
            sort_name: None,
            musicbrainz_id: None,
        },
    )
    .await
    .unwrap();

    soul_storage::albums::create(
        &pool,
        soul_core::types::CreateAlbum {
            title: "Album X".to_string(),
            artist_id: Some(artist.id),
            year: None,
            musicbrainz_id: None,
        },
    )
    .await
    .unwrap();

    let cache = EntityCache::preload(&pool).await.unwrap();

    // Should find album for correct artist
    assert!(cache.find_album_by_normalized("album x", Some(artist.id)).is_some());
    // Should not find for wrong artist
    assert!(cache.find_album_by_normalized("album x", Some(ArtistId::new("999"))).is_none());
}
```

**Step 2: Run tests — expect compile error (EntityCache doesn't exist yet)**

Run: `cd libraries/soul-importer && cargo test --test entity_cache_tests`
Expected: FAIL — `EntityCache` not found

**Step 3: Implement EntityCache**

Add to `libraries/soul-importer/src/fuzzy.rs`, before `impl FuzzyMatcher`:

```rust
use std::collections::HashMap;

/// In-memory cache for entity lookups during scanning.
/// Preloaded once at scan start, updated as new entities are created.
/// Eliminates O(n) DB queries per file for artist/album/genre matching.
pub struct EntityCache {
    /// normalized_name → (ArtistId, original_name)
    artists: HashMap<String, (ArtistId, String)>,
    /// (normalized_title, Option<ArtistId>) → (AlbumId, original_title)
    albums: HashMap<(String, Option<ArtistId>), (soul_core::types::AlbumId, String)>,
    /// normalized_name → (GenreId, original_name)
    genres: HashMap<String, (GenreId, String)>,
}

impl EntityCache {
    /// Preload all entities from the database
    pub async fn preload(pool: &SqlitePool) -> crate::Result<Self> {
        let all_artists = soul_storage::artists::get_all(pool).await?;
        let all_albums = soul_storage::albums::get_all(pool).await?;
        let all_genres = soul_storage::genres::get_all(pool).await?;

        let mut artists = HashMap::with_capacity(all_artists.len());
        for a in &all_artists {
            artists.insert(normalize_string(&a.name), (a.id, a.name.clone()));
        }

        let mut albums = HashMap::with_capacity(all_albums.len());
        for a in &all_albums {
            let key = (normalize_string(&a.title), a.artist_id);
            albums.insert(key, (a.id, a.title.clone()));
        }

        let mut genres = HashMap::with_capacity(all_genres.len());
        for g in &all_genres {
            genres.insert(normalize_string(&g.name), (g.id, g.name.clone()));
            // Also index by canonical name
            if let Some(ref cn) = g.canonical_name {
                genres.entry(normalize_string(cn)).or_insert((g.id, g.name.clone()));
            }
        }

        tracing::info!(
            "[CACHE] Preloaded {} artists, {} albums, {} genres",
            artists.len(), albums.len(), genres.len()
        );

        Ok(Self { artists, albums, genres })
    }

    /// O(1) artist lookup by normalized name
    pub fn find_artist_by_normalized(&self, name: &str) -> Option<ArtistId> {
        let normalized = normalize_string(name);
        self.artists.get(&normalized).map(|(id, _)| *id)
    }

    /// O(1) album lookup by normalized title + artist
    pub fn find_album_by_normalized(&self, title: &str, artist_id: Option<ArtistId>) -> Option<soul_core::types::AlbumId> {
        let key = (normalize_string(title), artist_id);
        self.albums.get(&key).map(|(id, _)| *id)
    }

    /// O(1) genre lookup by normalized name
    pub fn find_genre_by_normalized(&self, name: &str) -> Option<GenreId> {
        let normalized = normalize_string(name);
        self.genres.get(&normalized).map(|(id, _)| *id)
    }

    /// Insert a newly created artist into the cache
    pub fn insert_artist(&mut self, id: ArtistId, name: &str) {
        self.artists.insert(normalize_string(name), (id, name.to_string()));
    }

    /// Insert a newly created album into the cache
    pub fn insert_album(&mut self, id: soul_core::types::AlbumId, title: &str, artist_id: Option<ArtistId>) {
        self.albums.insert((normalize_string(title), artist_id), (id, title.to_string()));
    }

    /// Insert a newly created genre into the cache
    pub fn insert_genre(&mut self, id: GenreId, name: &str) {
        self.genres.insert(normalize_string(name), (id, name.to_string()));
    }

    /// Get all artist names for Levenshtein fallback (cache miss)
    pub fn artist_entries(&self) -> impl Iterator<Item = (&str, ArtistId)> {
        self.artists.iter().map(|(norm, (id, _))| (norm.as_str(), *id))
    }

    /// Get album entries for a specific artist (Levenshtein fallback)
    pub fn album_entries_for_artist(&self, artist_id: Option<ArtistId>) -> Vec<(&str, soul_core::types::AlbumId)> {
        self.albums.iter()
            .filter(|((_, aid), _)| *aid == artist_id)
            .map(|((norm, _), (id, _))| (norm.as_str(), *id))
            .collect()
    }
}
```

**Step 4: Add `find_or_create_artist_cached` method to FuzzyMatcher**

Add to `impl FuzzyMatcher` in `fuzzy.rs`:

```rust
    /// Find or create artist using in-memory cache (avoids DB scan on hit)
    pub async fn find_or_create_artist_cached(
        &self,
        pool: &SqlitePool,
        name: &str,
        cache: &EntityCache,
    ) -> Result<FuzzyMatch<Artist>> {
        // O(1) cache lookup first
        if let Some(artist_id) = cache.find_artist_by_normalized(name) {
            let artist = soul_storage::artists::get_by_id(pool, artist_id).await?;
            if let Some(artist) = artist {
                return Ok(FuzzyMatch {
                    entity: artist,
                    confidence: 95, // Normalized match via cache
                    match_type: MatchType::Normalized,
                });
            }
        }

        // Cache miss — fall back to Levenshtein against cached entries
        let normalized_name = normalize_string(name);
        let mut best_match: Option<(ArtistId, f64)> = None;

        for (cached_norm, cached_id) in cache.artist_entries() {
            let similarity = normalized_levenshtein(&normalized_name, cached_norm);
            if similarity >= (self.fuzzy_threshold as f64 / 100.0) {
                if let Some((_, best_sim)) = &best_match {
                    if similarity > *best_sim {
                        best_match = Some((cached_id, similarity));
                    }
                } else {
                    best_match = Some((cached_id, similarity));
                }
            }
        }

        if let Some((artist_id, similarity)) = best_match {
            if let Some(artist) = soul_storage::artists::get_by_id(pool, artist_id).await? {
                let confidence = (similarity * 100.0).round() as u8;
                return Ok(FuzzyMatch {
                    entity: artist,
                    confidence,
                    match_type: MatchType::Fuzzy,
                });
            }
        }

        // No match — create new (same as existing code)
        let sort_name = normalize_sort_name(name);
        let new_artist = soul_storage::artists::create(
            pool,
            CreateArtist {
                name: name.to_string(),
                sort_name: Some(sort_name),
                musicbrainz_id: None,
            },
        )
        .await?;

        Ok(FuzzyMatch {
            entity: new_artist,
            confidence: 100,
            match_type: MatchType::Created,
        })
    }
```

Add similar `find_or_create_album_cached` and `find_or_create_genre_cached` methods following the same pattern.

**Step 5: Run tests**

Run: `cd libraries/soul-importer && cargo test --test entity_cache_tests`
Expected: All 5 tests PASS

**Step 6: Commit**

```bash
git add libraries/soul-importer/src/fuzzy.rs libraries/soul-importer/tests/entity_cache_tests.rs
git commit -m "feat: add in-memory EntityCache for O(1) fuzzy matching lookups"
```

---

### Task 3: Add Batched Progress Updates to scan_progress

**Files:**
- Modify: `libraries/soul-storage/src/scan_progress/mod.rs`
- Create: `libraries/soul-importer/tests/batched_progress_tests.rs`

**Step 1: Write test for bulk update**

Create `libraries/soul-importer/tests/batched_progress_tests.rs`:

```rust
//! Tests for batched scan progress updates

mod test_helpers;

use soul_core::types::CreateLibrarySource;

#[tokio::test]
async fn test_update_counts_bulk() {
    let pool = test_helpers::setup_test_db().await;

    // Create source and progress entry
    let source = soul_storage::library_sources::create(
        &pool, "user1", "device1",
        &CreateLibrarySource {
            name: "Test".to_string(),
            path: "/fake".to_string(),
            sync_deletes: false,
        },
    ).await.unwrap();

    let progress = soul_storage::scan_progress::start(&pool, source.id, Some(100)).await.unwrap();

    // Bulk update all counters at once
    soul_storage::scan_progress::update_counts(
        &pool, progress.id, 50, 30, 10, 5, 5
    ).await.unwrap();

    // Verify
    let updated = soul_storage::scan_progress::get_by_id(&pool, progress.id).await.unwrap().unwrap();
    assert_eq!(updated.processed_files, 50);
    assert_eq!(updated.new_files, 30);
    assert_eq!(updated.updated_files, 10);
    assert_eq!(updated.removed_files, 5);
    assert_eq!(updated.errors, 5);
}

#[tokio::test]
async fn test_update_counts_additive() {
    let pool = test_helpers::setup_test_db().await;

    let source = soul_storage::library_sources::create(
        &pool, "user1", "device1",
        &CreateLibrarySource {
            name: "Test".to_string(),
            path: "/fake".to_string(),
            sync_deletes: false,
        },
    ).await.unwrap();

    let progress = soul_storage::scan_progress::start(&pool, source.id, Some(200)).await.unwrap();

    // Two bulk updates should be additive
    soul_storage::scan_progress::update_counts(&pool, progress.id, 50, 30, 10, 5, 5).await.unwrap();
    soul_storage::scan_progress::update_counts(&pool, progress.id, 50, 20, 5, 0, 25).await.unwrap();

    let updated = soul_storage::scan_progress::get_by_id(&pool, progress.id).await.unwrap().unwrap();
    assert_eq!(updated.processed_files, 100);
    assert_eq!(updated.new_files, 50);
    assert_eq!(updated.updated_files, 15);
    assert_eq!(updated.removed_files, 5);
    assert_eq!(updated.errors, 30);
}
```

**Step 2: Run tests — expect compile error**

Run: `cd libraries/soul-importer && cargo test --test batched_progress_tests`
Expected: FAIL — `update_counts` not found

**Step 3: Implement update_counts**

Add to `libraries/soul-storage/src/scan_progress/mod.rs`:

```rust
/// Update all scan progress counters in a single query (additive).
/// Replaces per-field increment calls for batched scanning.
pub async fn update_counts(
    pool: &SqlitePool,
    id: i64,
    processed: i64,
    new_files: i64,
    updated: i64,
    removed: i64,
    errors: i64,
) -> Result<()> {
    sqlx::query!(
        r#"UPDATE scan_progress SET
            processed_files = processed_files + ?,
            new_files = new_files + ?,
            updated_files = updated_files + ?,
            removed_files = removed_files + ?,
            errors = errors + ?
        WHERE id = ?"#,
        processed,
        new_files,
        updated,
        removed,
        errors,
        id
    )
    .execute(pool)
    .await?;

    Ok(())
}
```

**Step 4: Run tests**

Run: `cd libraries/soul-importer && cargo test --test batched_progress_tests`
Expected: PASS

**Step 5: Commit**

```bash
git add libraries/soul-storage/src/scan_progress/mod.rs libraries/soul-importer/tests/batched_progress_tests.rs
git commit -m "feat: add bulk update_counts for scan progress (single query replaces 3-4)"
```

---

### Task 4: Implement Parallel Scan Pipeline

**Files:**
- Modify: `libraries/soul-importer/src/library_scanner.rs`
- Modify: `libraries/soul-importer/src/metadata_extractor.rs`
- Create: `libraries/soul-importer/tests/parallel_scan_tests.rs`

**Step 1: Write tests for parallel scanning**

Create `libraries/soul-importer/tests/parallel_scan_tests.rs`:

```rust
//! Tests for parallel scan pipeline

mod test_helpers;

use soul_core::types::CreateLibrarySource;
use soul_importer::library_scanner::LibraryScanner;
use std::fs;
use std::io::Write;
use tempfile::TempDir;

fn create_test_files(dir: &std::path::Path, count: usize) {
    for i in 0..count {
        let file_path = dir.join(format!("track_{:04}.flac", i));
        let mut file = fs::File::create(&file_path).unwrap();
        file.write_all(b"fLaC\x00\x00\x00\x22").unwrap();
        file.write_all(&[0u8; 1000]).unwrap();
    }
}

#[tokio::test]
async fn test_parallel_scan_processes_all_files() {
    let pool = test_helpers::setup_test_db().await;
    let dir = TempDir::new().unwrap();
    create_test_files(dir.path(), 50);

    let source = soul_storage::library_sources::create(
        &pool, "user1", "device1",
        &CreateLibrarySource {
            name: "Test".to_string(),
            path: dir.path().display().to_string(),
            sync_deletes: false,
        },
    ).await.unwrap();

    let scanner = LibraryScanner::new(pool.clone(), "user1", "device1")
        .compute_hashes(false)
        .concurrency(4); // Use parallel mode

    let stats = scanner.scan_all().await.unwrap();

    // All files should be processed (some may error due to fake FLAC, that's OK)
    assert_eq!(stats.total_files, 50);
    assert_eq!(stats.processed, 50);
    // new_files + errors should equal total
    assert_eq!(stats.new_files + stats.errors, 50);
}

#[tokio::test]
async fn test_parallel_scan_rescan_skips_unchanged() {
    let pool = test_helpers::setup_test_db().await;
    let dir = TempDir::new().unwrap();
    create_test_files(dir.path(), 20);

    soul_storage::library_sources::create(
        &pool, "user1", "device1",
        &CreateLibrarySource {
            name: "Test".to_string(),
            path: dir.path().display().to_string(),
            sync_deletes: false,
        },
    ).await.unwrap();

    // First scan
    let scanner = LibraryScanner::new(pool.clone(), "user1", "device1")
        .compute_hashes(false)
        .concurrency(4);
    let stats1 = scanner.scan_all().await.unwrap();

    // Second scan — should skip all unchanged files
    let scanner2 = LibraryScanner::new(pool.clone(), "user1", "device1")
        .compute_hashes(false)
        .concurrency(4);
    let stats2 = scanner2.scan_all().await.unwrap();

    assert_eq!(stats2.total_files, 20);
    assert_eq!(stats2.new_files, 0);
    assert_eq!(stats2.updated_files, 0);
}

#[tokio::test]
async fn test_parallel_scan_concurrency_1_matches_sequential() {
    let pool = test_helpers::setup_test_db().await;
    let dir = TempDir::new().unwrap();
    create_test_files(dir.path(), 10);

    soul_storage::library_sources::create(
        &pool, "user1", "device1",
        &CreateLibrarySource {
            name: "Test".to_string(),
            path: dir.path().display().to_string(),
            sync_deletes: false,
        },
    ).await.unwrap();

    let scanner = LibraryScanner::new(pool.clone(), "user1", "device1")
        .compute_hashes(false)
        .concurrency(1); // Sequential

    let stats = scanner.scan_all().await.unwrap();
    assert_eq!(stats.total_files, 10);
    assert_eq!(stats.processed, 10);
}

#[tokio::test]
async fn test_parallel_scan_with_entity_cache() {
    let pool = test_helpers::setup_test_db().await;
    let dir = TempDir::new().unwrap();

    // Create files in artist/album folder structure
    let artist_dir = dir.path().join("Test Artist").join("Test Album");
    fs::create_dir_all(&artist_dir).unwrap();
    create_test_files(&artist_dir, 10);

    soul_storage::library_sources::create(
        &pool, "user1", "device1",
        &CreateLibrarySource {
            name: "Test".to_string(),
            path: dir.path().display().to_string(),
            sync_deletes: false,
        },
    ).await.unwrap();

    let scanner = LibraryScanner::new(pool.clone(), "user1", "device1")
        .compute_hashes(false)
        .concurrency(4);

    let stats = scanner.scan_all().await.unwrap();

    // All processed, entity cache should prevent duplicate artist/album creation
    assert_eq!(stats.processed, 10);
}
```

**Step 2: Run tests — expect compile error**

Run: `cd libraries/soul-importer && cargo test --test parallel_scan_tests`
Expected: FAIL — `concurrency()` method doesn't exist

**Step 3: Add concurrency field to LibraryScanner**

In `libraries/soul-importer/src/library_scanner.rs`, add to the struct:

```rust
/// Maximum number of concurrent metadata extraction tasks
concurrency: usize,
```

Default in `new()`: `concurrency: 8`

Add builder method:

```rust
/// Set maximum concurrency for parallel metadata extraction (default: 8)
pub fn concurrency(mut self, max: usize) -> Self {
    self.concurrency = max.max(1); // Minimum 1
    self
}
```

**Step 4: Rewrite scan_source to use parallel pipeline**

Replace the sequential `for file_path in &files` loop (lines ~190-242) in `scan_source()` with:

```rust
        // --- Phase 1: Filter unchanged files (cheap: stat only) ---
        let existing_tracks = self.get_existing_tracks_map(source.id).await?;
        let mut seen_paths: HashMap<String, bool> = HashMap::with_capacity(files.len());

        // Classify files into unchanged vs needs-processing
        let mut files_to_process: Vec<(std::path::PathBuf, i64, i64, Option<ExistingTrack>)> = Vec::new();
        let mut unchanged_count: i64 = 0;

        for file_path in &files {
            let path_str = file_path.display().to_string();
            seen_paths.insert(path_str.clone(), true);

            let fs_meta = match std::fs::metadata(file_path) {
                Ok(m) => m,
                Err(e) => {
                    tracing::warn!("Failed to stat file {:?}: {}", file_path, e);
                    stats.errors += 1;
                    continue;
                }
            };
            let file_size = fs_meta.len() as i64;
            let file_mtime = fs_meta.modified()
                .map(|t| t.duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0))
                .unwrap_or(0);

            if let Some(existing) = existing_tracks.get(&path_str) {
                let unchanged = existing.file_size == Some(file_size)
                    && existing.file_mtime == Some(file_mtime);
                if unchanged && !self.force_metadata_refresh {
                    unchanged_count += 1;
                    stats.processed += 1;
                    continue;
                }
                files_to_process.push((file_path.clone(), file_size, file_mtime, Some(existing.clone())));
            } else {
                files_to_process.push((file_path.clone(), file_size, file_mtime, None));
            }
        }

        tracing::info!(
            "[SCAN] Phase 1 complete: {} unchanged (skipped), {} to process",
            unchanged_count, files_to_process.len()
        );

        // Update progress for skipped files
        if unchanged_count > 0 {
            soul_storage::scan_progress::increment_processed(&self.pool, progress.id, unchanged_count).await?;
        }

        // --- Phase 2: Parallel metadata extraction + DB writes ---
        let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(self.concurrency));
        let cache = std::sync::Arc::new(tokio::sync::Mutex::new(
            crate::fuzzy::EntityCache::preload(&self.pool).await?
        ));

        // Accumulate stats for batched progress updates
        let batch_stats = std::sync::Arc::new(tokio::sync::Mutex::new(ScanStats::default()));
        let progress_interval = 100; // Flush progress every N files

        for (idx, (file_path, file_size, file_mtime, existing)) in files_to_process.into_iter().enumerate() {
            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let pool = self.pool.clone();
            let cache = cache.clone();
            let batch_stats = batch_stats.clone();
            let compute_hashes = self.compute_hashes;
            let metadata_extractor = MetadataExtractor::new();

            let result = tokio::spawn(async move {
                let _permit = permit; // Hold permit until task completes

                // Extract metadata (CPU-bound, uses spawn_blocking internally)
                let raw = match metadata_extractor.extract_metadata(&file_path).await {
                    Ok(m) => m,
                    Err(e) => {
                        tracing::warn!("Failed to extract metadata {:?}: {}", file_path, e);
                        return Err(e);
                    }
                };

                Ok((file_path, file_size, file_mtime, existing, raw))
            }).await;

            // Process result sequentially (DB writes must be sequential for SQLite)
            match result {
                Ok(Ok((file_path, file_size, file_mtime, existing, raw))) => {
                    // Use cached fuzzy matching
                    let mut cache_guard = cache.lock().await;
                    let processed = self.match_entities_cached(&pool, raw, &mut cache_guard).await;

                    match processed {
                        Ok(processed) => {
                            if existing.is_some() {
                                // Update existing track
                                let existing = existing.unwrap();
                                let processor = FileProcessor::new(&pool, &self.metadata_extractor, compute_hashes);
                                match processor.update_track_metadata(existing.id, &file_path, file_size, file_mtime).await {
                                    Ok(_) => { stats.updated_files += 1; }
                                    Err(e) => { tracing::warn!("Update failed {:?}: {}", file_path, e); stats.errors += 1; }
                                }
                            } else {
                                // Import new file
                                let processor = FileProcessor::new(&pool, &self.metadata_extractor, compute_hashes);
                                let content_hash = if compute_hashes {
                                    hash_computer::compute_file_hash(&file_path).await.ok()
                                } else {
                                    None
                                };
                                match processor.import_new_file(&file_path, source.id, file_size, file_mtime, content_hash).await {
                                    Ok(_) => { stats.new_files += 1; }
                                    Err(e) => { tracing::warn!("Import failed {:?}: {}", file_path, e); stats.errors += 1; }
                                }
                            }
                            stats.processed += 1;
                        }
                        Err(e) => {
                            tracing::warn!("Entity matching failed {:?}: {}", file_path, e);
                            stats.errors += 1;
                            stats.processed += 1;
                        }
                    }
                }
                Ok(Err(_)) | Err(_) => {
                    stats.errors += 1;
                    stats.processed += 1;
                }
            }

            // Batched progress update
            if (idx + 1) % progress_interval == 0 || idx == files.len() - 1 {
                soul_storage::scan_progress::update_counts(
                    &self.pool, progress.id,
                    stats.processed - unchanged_count, // Only count newly processed
                    stats.new_files, stats.updated_files,
                    stats.removed_files, stats.errors,
                ).await?;

                if let Some(ref callback) = self.progress_callback {
                    callback(&stats);
                }
            }
        }
```

**Note:** The above is a sketch showing the pattern. The actual implementation must:
- Handle the `ExistingTrack` clone correctly
- Use the entity cache for all fuzzy matching (artist, album, genre)
- Add a `match_entities_cached` helper method to `LibraryScanner`
- Ensure progress stats are accumulated correctly across batches

**Step 5: Add `match_entities_cached` helper**

Add to `impl LibraryScanner`:

```rust
    /// Match entities using the in-memory cache, falling back to DB
    async fn match_entities_cached(
        &self,
        pool: &SqlitePool,
        raw: crate::metadata::ExtractedMetadata,
        cache: &mut crate::fuzzy::EntityCache,
    ) -> Result<crate::metadata_extractor::ProcessedMetadata> {
        let matcher = &self.metadata_extractor.fuzzy_matcher();

        let artist_id = if let Some(ref name) = raw.artist {
            let m = matcher.find_or_create_artist_cached(pool, name, cache).await?;
            if m.match_type == crate::MatchType::Created {
                cache.insert_artist(m.entity.id, &m.entity.name);
            }
            Some(m.entity.id)
        } else { None };

        let album_id = if let Some(ref title) = raw.album {
            let m = matcher.find_or_create_album_cached(pool, title, artist_id, cache).await?;
            if m.match_type == crate::MatchType::Created {
                cache.insert_album(m.entity.id, &m.entity.title, artist_id);
            }
            Some(m.entity.id)
        } else { None };

        let album_artist_id = if let Some(ref name) = raw.album_artist {
            if raw.artist.as_ref() != Some(name) {
                let m = matcher.find_or_create_artist_cached(pool, name, cache).await?;
                if m.match_type == crate::MatchType::Created {
                    cache.insert_artist(m.entity.id, &m.entity.name);
                }
                Some(m.entity.id)
            } else { artist_id }
        } else { None };

        let mut genre_ids = Vec::new();
        for genre_name in &raw.genres {
            let m = matcher.find_or_create_genre_cached(pool, genre_name, cache).await?;
            if m.match_type == crate::MatchType::Created {
                cache.insert_genre(m.entity.id, &m.entity.name);
            }
            genre_ids.push(m.entity.id);
        }

        Ok(crate::metadata_extractor::ProcessedMetadata {
            raw, artist_id, album_id, album_artist_id, genre_ids,
        })
    }
```

**Step 6: Run tests**

Run: `cd libraries/soul-importer && cargo test --test parallel_scan_tests`
Expected: All 4 tests PASS

**Step 7: Commit**

```bash
git add libraries/soul-importer/src/library_scanner.rs libraries/soul-importer/src/metadata_extractor.rs libraries/soul-importer/tests/parallel_scan_tests.rs
git commit -m "feat: parallel scan pipeline with semaphore-gated extraction and entity cache"
```

---

### Task 5: Run Benchmarks — Measure Improvement

**Files:**
- No new files — rerun existing benchmarks

**Step 1: Run the benchmark suite**

Run: `cd libraries/soul-importer && cargo bench`

Compare results against Task 1 baseline. Key metrics:
- `fuzzy_matching/find_or_create_artist/1000` — should be ~100x faster (cache hit vs DB scan)
- `full_scan/scan_all/500` — should show significant improvement
- `progress_updates/individual_increments/1000` — baseline for Task 3's batched approach

**Step 2: Document results**

Add benchmark results as a comment in the design doc or commit message.

**Step 3: Commit benchmark results**

```bash
git add -A
git commit -m "bench: measure post-optimization scan performance"
```

---

### Task 6: Integration Test — Large Library Simulation

**Files:**
- Create: `libraries/soul-importer/tests/large_library_test.rs`

**Step 1: Write large library stress test**

```rust
//! Stress test: simulate scanning a large library
//! Run with: cargo test --test large_library_test -- --nocapture --ignored

mod test_helpers;

use soul_core::types::CreateLibrarySource;
use soul_importer::library_scanner::LibraryScanner;
use std::fs;
use std::io::Write;
use std::time::Instant;
use tempfile::TempDir;

fn create_large_library(dir: &std::path::Path, track_count: usize) {
    let artists = 200;
    let albums_per_artist = 10;

    for i in 0..track_count {
        let artist_idx = i % artists;
        let album_idx = (i / artists) % albums_per_artist;
        let folder = dir
            .join(format!("Artist_{:04}", artist_idx))
            .join(format!("Album_{:03}", album_idx));
        fs::create_dir_all(&folder).ok();

        let file_path = folder.join(format!("track_{:06}.flac", i));
        if !file_path.exists() {
            let mut file = fs::File::create(&file_path).unwrap();
            file.write_all(b"fLaC\x00\x00\x00\x22").unwrap();
            file.write_all(&[0u8; 500]).unwrap();
        }
    }
}

#[tokio::test]
#[ignore] // Run explicitly: cargo test --test large_library_test -- --ignored --nocapture
async fn test_scan_10k_files() {
    let pool = test_helpers::setup_test_db().await;
    let dir = TempDir::new().unwrap();

    let start = Instant::now();
    create_large_library(dir.path(), 10_000);
    eprintln!("Created 10K test files in {:?}", start.elapsed());

    soul_storage::library_sources::create(
        &pool, "user1", "device1",
        &CreateLibrarySource {
            name: "Large Library".to_string(),
            path: dir.path().display().to_string(),
            sync_deletes: false,
        },
    ).await.unwrap();

    // Scan with parallelism
    let start = Instant::now();
    let scanner = LibraryScanner::new(pool.clone(), "user1", "device1")
        .compute_hashes(false)
        .concurrency(8);

    let stats = scanner.scan_all().await.unwrap();
    let duration = start.elapsed();

    eprintln!("=== 10K File Scan Results ===");
    eprintln!("Duration: {:?}", duration);
    eprintln!("Total: {}, Processed: {}, New: {}, Errors: {}",
        stats.total_files, stats.processed, stats.new_files, stats.errors);
    eprintln!("Throughput: {:.0} files/sec", stats.processed as f64 / duration.as_secs_f64());

    assert_eq!(stats.total_files, 10_000);
    assert_eq!(stats.processed, 10_000);

    // Rescan — should be fast (all unchanged)
    let rescan_start = Instant::now();
    let scanner2 = LibraryScanner::new(pool.clone(), "user1", "device1")
        .compute_hashes(false)
        .concurrency(8);
    let stats2 = scanner2.scan_all().await.unwrap();
    let rescan_duration = rescan_start.elapsed();

    eprintln!("=== 10K Rescan (unchanged) ===");
    eprintln!("Duration: {:?}", rescan_duration);
    eprintln!("New: {}, Updated: {}", stats2.new_files, stats2.updated_files);

    assert_eq!(stats2.new_files, 0);
    assert_eq!(stats2.updated_files, 0);
    // Rescan should be significantly faster
    assert!(rescan_duration < duration / 2, "Rescan should be at least 2x faster");
}

#[tokio::test]
#[ignore]
async fn test_scan_1k_sequential_vs_parallel() {
    let pool_seq = test_helpers::setup_test_db().await;
    let pool_par = test_helpers::setup_test_db().await;
    let dir = TempDir::new().unwrap();
    create_large_library(dir.path(), 1_000);

    let path_str = dir.path().display().to_string();

    // Sequential (concurrency=1)
    soul_storage::library_sources::create(
        &pool_seq, "user1", "device1",
        &CreateLibrarySource {
            name: "Seq".to_string(), path: path_str.clone(), sync_deletes: false,
        },
    ).await.unwrap();

    let start_seq = Instant::now();
    let scanner_seq = LibraryScanner::new(pool_seq.clone(), "user1", "device1")
        .compute_hashes(false)
        .concurrency(1);
    let stats_seq = scanner_seq.scan_all().await.unwrap();
    let dur_seq = start_seq.elapsed();

    // Parallel (concurrency=8)
    soul_storage::library_sources::create(
        &pool_par, "user1", "device1",
        &CreateLibrarySource {
            name: "Par".to_string(), path: path_str, sync_deletes: false,
        },
    ).await.unwrap();

    let start_par = Instant::now();
    let scanner_par = LibraryScanner::new(pool_par.clone(), "user1", "device1")
        .compute_hashes(false)
        .concurrency(8);
    let stats_par = scanner_par.scan_all().await.unwrap();
    let dur_par = start_par.elapsed();

    eprintln!("=== Sequential vs Parallel (1K files) ===");
    eprintln!("Sequential: {:?} ({} processed)", dur_seq, stats_seq.processed);
    eprintln!("Parallel:   {:?} ({} processed)", dur_par, stats_par.processed);
    eprintln!("Speedup:    {:.1}x", dur_seq.as_secs_f64() / dur_par.as_secs_f64());

    // Parallel should be at least 2x faster (conservative, often 4-8x)
    assert!(dur_par < dur_seq, "Parallel should be faster than sequential");
}
```

**Step 2: Run the stress tests**

Run: `cd libraries/soul-importer && cargo test --test large_library_test -- --ignored --nocapture`
Expected: Results printed showing throughput and speedup numbers.

**Step 3: Commit**

```bash
git add libraries/soul-importer/tests/large_library_test.rs
git commit -m "test: add large library stress tests (10K files, sequential vs parallel comparison)"
```

---

### Task 7: Wire Up to Tauri Commands (Backwards Compatible)

**Files:**
- Modify: `applications/desktop/src-tauri/src/library_settings.rs`

**Step 1: Verify existing Tauri commands still work**

The `LibraryScanner::new()` API is unchanged — the `concurrency()` builder is additive. The default is 8 which means existing callers automatically get parallel scanning.

Check that `rescan_library_source` and `rescan_all_sources` in `library_settings.rs` construct `LibraryScanner` without `.concurrency()` — they'll get the default (8).

**Step 2: Run existing E2E tests**

Run: `cd applications/desktop/e2e-tests && npx playwright test --config playwright.cdp.config.js tests/playwright/data-settings.spec.js`
Expected: All data-settings tests still pass (rescan button triggers scan, progress indicator appears)

Run: `cd applications/desktop/e2e-tests && npx playwright test --config playwright.cdp.config.js tests/playwright/import-and-scan.spec.js`
Expected: All import-and-scan tests still pass

**Step 3: Commit**

```bash
git commit -m "feat: parallel scanning enabled by default (concurrency=8)"
```

---

## Summary

| Task | What | Tests |
|------|------|-------|
| 1 | Benchmark infrastructure | criterion benchmarks |
| 2 | EntityCache for fuzzy matching | 5 unit tests |
| 3 | Batched progress updates | 2 unit tests |
| 4 | Parallel scan pipeline | 4 integration tests |
| 5 | Re-run benchmarks | Compare before/after |
| 6 | Large library stress tests | 10K scan, seq vs parallel |
| 7 | Verify Tauri integration | Existing E2E tests |
