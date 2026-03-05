//! Benchmarks for scan/import performance baseline measurement.
//!
//! Run with: `cargo bench -p soul-importer`
//!
//! These benchmarks measure:
//! 1. Directory walk performance (FileScanner::scan_directory)
//! 2. Fuzzy matching performance (FuzzyMatcher::find_or_create_artist)
//! 3. Full scan pipeline (LibraryScanner::scan_all)
//! 4. Database write performance (tracks::create)
//! 5. Progress update overhead (scan_progress::increment_processed)

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use sqlx::SqlitePool;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

use soul_core::types::{CreateArtist, CreateLibrarySource, CreateTrack};
use soul_importer::fuzzy::FuzzyMatcher;
use soul_importer::scanner::FileScanner;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Create a temporary directory tree with `count` fake FLAC files spread
/// across subdirectories (roughly 100 files per subdirectory).
fn create_test_audio_files(base: &Path, count: usize) -> Vec<PathBuf> {
    // Minimal FLAC header so FileScanner recognises the .flac extension.
    let fake_flac: Vec<u8> = {
        let mut buf = Vec::with_capacity(1042);
        buf.extend_from_slice(b"fLaC\x00\x00\x00\x22");
        buf.resize(1042, 0u8);
        buf
    };

    let mut paths = Vec::with_capacity(count);
    for i in 0..count {
        let subdir = base.join(format!("artist_{:04}", i / 100));
        if !subdir.exists() {
            fs::create_dir_all(&subdir).expect("create subdir");
        }
        let file_path = subdir.join(format!("track_{:05}.flac", i));
        fs::write(&file_path, &fake_flac).expect("write fake flac");
        paths.push(file_path);
    }
    paths
}

/// Set up an in-memory SQLite database with all migrations applied.
async fn setup_db() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("connect to in-memory db");
    sqlx::migrate!("../soul-storage/migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    pool
}

/// Create a library source row and return its ID.
async fn create_library_source(pool: &SqlitePool, path: &str) -> i64 {
    let source = soul_storage::library_sources::create(
        pool,
        "user1",
        "device1",
        &CreateLibrarySource {
            name: "Bench Source".to_string(),
            path: path.to_string(),
            sync_deletes: false,
        },
    )
    .await
    .expect("create library source");
    source.id
}

/// Build a `CreateTrack` with the given index for uniqueness.
fn make_create_track(index: usize, source_id: i64) -> CreateTrack {
    CreateTrack {
        title: format!("Bench Track {}", index),
        artist_id: None,
        album_id: None,
        album_artist_id: None,
        track_number: Some((index % 20) as i32 + 1),
        disc_number: Some(1),
        year: Some(2024),
        duration_seconds: Some(180.0),
        bitrate: Some(320),
        sample_rate: Some(44100),
        channels: Some(2),
        file_format: "flac".to_string(),
        file_hash: Some(format!("bench_hash_{:08x}", index)),
        origin_source_id: source_id,
        local_file_path: Some(format!("/fake/path/track_{}.flac", index)),
        musicbrainz_recording_id: None,
        fingerprint: None,
    }
}

// ---------------------------------------------------------------------------
// 1. Directory walk benchmarks
// ---------------------------------------------------------------------------

fn bench_directory_walk(c: &mut Criterion) {
    let mut group = c.benchmark_group("directory_walk");
    group.sample_size(10);

    for &file_count in &[100usize, 1_000, 10_000] {
        let temp = TempDir::new().expect("create temp dir");
        create_test_audio_files(temp.path(), file_count);

        group.bench_with_input(
            BenchmarkId::new("scan_directory", file_count),
            &file_count,
            |b, &count| {
                let scanner = FileScanner::new();
                b.iter(|| {
                    let files = scanner.scan_directory(temp.path()).expect("scan");
                    assert_eq!(files.len(), count);
                });
            },
        );
    }

    group.finish();
}

// ---------------------------------------------------------------------------
// 2. Fuzzy matching benchmarks
// ---------------------------------------------------------------------------

fn bench_fuzzy_matching(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let mut group = c.benchmark_group("fuzzy_matching");
    group.sample_size(10);

    for &artist_count in &[10usize, 100, 1_000] {
        // Set up DB with pre-populated artists
        let pool = rt.block_on(async {
            let pool = setup_db().await;
            for i in 0..artist_count {
                soul_storage::artists::create(
                    &pool,
                    CreateArtist {
                        name: format!("Artist Number {}", i),
                        sort_name: Some(format!("Artist Number {}", i)),
                        musicbrainz_id: None,
                    },
                )
                .await
                .expect("create artist");
            }
            pool
        });

        group.bench_with_input(
            BenchmarkId::new("find_or_create_artist_exact", artist_count),
            &artist_count,
            |b, _| {
                let matcher = FuzzyMatcher::new();
                b.iter(|| {
                    rt.block_on(async {
                        // Search for an existing artist (exact match path)
                        let _result = matcher
                            .find_or_create_artist(&pool, "Artist Number 0")
                            .await
                            .expect("find artist");
                    });
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("find_or_create_artist_fuzzy", artist_count),
            &artist_count,
            |b, _| {
                let matcher = FuzzyMatcher::new();
                b.iter(|| {
                    rt.block_on(async {
                        // Search for a slightly misspelled artist (fuzzy match path)
                        let _result = matcher
                            .find_or_create_artist(&pool, "Artst Number 0")
                            .await
                            .expect("find artist");
                    });
                });
            },
        );
    }

    group.finish();
}

// ---------------------------------------------------------------------------
// 3. Full scan benchmarks
// ---------------------------------------------------------------------------

fn bench_full_scan(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let mut group = c.benchmark_group("full_scan");
    // Full scans are slow; reduce iterations.
    group.sample_size(10);
    group.measurement_time(std::time::Duration::from_secs(30));

    for &file_count in &[100usize, 500] {
        let temp = TempDir::new().expect("create temp dir");
        create_test_audio_files(temp.path(), file_count);
        let path_str = temp.path().display().to_string();

        group.bench_with_input(
            BenchmarkId::new("scan_all", file_count),
            &file_count,
            |b, &count| {
                b.iter(|| {
                    rt.block_on(async {
                        // Fresh DB per iteration so we always measure "new file" path
                        let pool = setup_db().await;
                        let _source_id = create_library_source(&pool, &path_str).await;

                        let scanner = soul_importer::library_scanner::LibraryScanner::new(
                            pool.clone(),
                            "user1",
                            "device1",
                        )
                        .compute_hashes(false);

                        let stats = scanner.scan_all().await.expect("scan_all");
                        assert_eq!(stats.total_files, count as i64);
                    });
                });
            },
        );
    }

    group.finish();
}

// ---------------------------------------------------------------------------
// 4. DB write benchmarks (tracks::create)
// ---------------------------------------------------------------------------

fn bench_db_writes(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let mut group = c.benchmark_group("db_writes");
    group.sample_size(10);

    for &batch_size in &[1usize, 10, 100] {
        group.bench_with_input(
            BenchmarkId::new("tracks_create", batch_size),
            &batch_size,
            |b, &size| {
                b.iter(|| {
                    rt.block_on(async {
                        let pool = setup_db().await;
                        let source_id = create_library_source(&pool, "/fake/bench").await;

                        for i in 0..size {
                            let track = make_create_track(i, source_id);
                            soul_storage::tracks::create(&pool, track)
                                .await
                                .expect("create track");
                        }
                    });
                });
            },
        );
    }

    group.finish();
}

// ---------------------------------------------------------------------------
// 5. Progress update benchmarks
// ---------------------------------------------------------------------------

fn bench_progress_updates(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let mut group = c.benchmark_group("progress_updates");
    group.sample_size(10);

    for &update_count in &[10usize, 100, 1_000] {
        group.bench_with_input(
            BenchmarkId::new("increment_processed", update_count),
            &update_count,
            |b, &count| {
                b.iter(|| {
                    rt.block_on(async {
                        let pool = setup_db().await;
                        let source_id = create_library_source(&pool, "/fake/bench").await;
                        let progress = soul_storage::scan_progress::start(
                            &pool,
                            source_id,
                            Some(count as i64),
                        )
                        .await
                        .expect("start progress");

                        for _ in 0..count {
                            soul_storage::scan_progress::increment_processed(&pool, progress.id, 1)
                                .await
                                .expect("increment");
                        }
                    });
                });
            },
        );
    }

    group.finish();
}

// ---------------------------------------------------------------------------
// Criterion harness
// ---------------------------------------------------------------------------

criterion_group!(
    benches,
    bench_directory_walk,
    bench_fuzzy_matching,
    bench_full_scan,
    bench_db_writes,
    bench_progress_updates,
);
criterion_main!(benches);
