//! Integration tests for large library scenarios
//!
//! Tests performance and correctness with big libraries (10k+ tracks)
//! to ensure pagination and data handling works correctly.

mod test_helpers;

use soul_core::types::*;
use test_helpers::*;

/// Number of tracks to create for large library tests
const LARGE_LIBRARY_SIZE: usize = 10_000;
/// Number of albums for large library tests
const LARGE_ALBUM_COUNT: usize = 500;
/// Number of artists for large library tests
const LARGE_ARTIST_COUNT: usize = 200;

/// Helper to batch insert tracks for performance
async fn batch_insert_tracks(
    pool: &sqlx::SqlitePool,
    count: usize,
    artist_ids: &[ArtistId],
    album_ids: &[AlbumId],
    source_id: SourceId,
) -> Vec<TrackId> {
    let mut track_ids = Vec::with_capacity(count);

    // Insert in batches of 100 for better performance
    for batch_start in (0..count).step_by(100) {
        let batch_end = (batch_start + 100).min(count);

        for i in batch_start..batch_end {
            let artist_id = if artist_ids.is_empty() {
                None
            } else {
                Some(artist_ids[i % artist_ids.len()])
            };

            let album_id = if album_ids.is_empty() {
                None
            } else {
                Some(album_ids[i % album_ids.len()])
            };

            let title = format!("Track {:05}", i);
            let file_path = format!("/music/track_{:05}.mp3", i);

            let result = sqlx::query(
                "INSERT INTO tracks (title, artist_id, album_id, origin_source_id, file_format,
                 duration_seconds, track_number, created_at, updated_at)
                 VALUES (?, ?, ?, ?, 'mp3', ?, ?, datetime('now'), datetime('now'))"
            )
            .bind(&title)
            .bind(artist_id)
            .bind(album_id)
            .bind(source_id)
            .bind(180.0 + (i as f64 % 120.0))  // Duration between 180-300s
            .bind((i % 15) as i32 + 1)  // Track number 1-15
            .execute(pool)
            .await
            .expect("Failed to create track");

            let track_id = result.last_insert_rowid();
            track_ids.push(TrackId::new(track_id.to_string()));

            // Create track availability
            sqlx::query(
                "INSERT INTO track_sources (track_id, source_id, status, local_file_path)
                 VALUES (?, ?, 'local_file', ?)",
            )
            .bind(track_id)
            .bind(source_id)
            .bind(&file_path)
            .execute(pool)
            .await
            .expect("Failed to create track availability");
        }
    }

    track_ids
}

/// Helper to batch insert artists
async fn batch_insert_artists(pool: &sqlx::SqlitePool, count: usize) -> Vec<ArtistId> {
    let mut artist_ids = Vec::with_capacity(count);

    for i in 0..count {
        let name = format!("Artist {:04}", i);
        let result = sqlx::query("INSERT INTO artists (name, sort_name) VALUES (?, ?)")
            .bind(&name)
            .bind(&name)
            .execute(pool)
            .await
            .expect("Failed to create artist");

        artist_ids.push(result.last_insert_rowid());
    }

    artist_ids
}

/// Helper to batch insert albums
async fn batch_insert_albums(
    pool: &sqlx::SqlitePool,
    count: usize,
    artist_ids: &[ArtistId],
) -> Vec<AlbumId> {
    let mut album_ids = Vec::with_capacity(count);

    for i in 0..count {
        let title = format!("Album {:04}", i);
        let artist_id = if artist_ids.is_empty() {
            None
        } else {
            Some(artist_ids[i % artist_ids.len()])
        };
        let year = 1990 + (i % 35) as i32; // Years 1990-2024

        let result = sqlx::query("INSERT INTO albums (title, artist_id, year) VALUES (?, ?, ?)")
            .bind(&title)
            .bind(artist_id)
            .bind(year)
            .execute(pool)
            .await
            .expect("Failed to create album");

        album_ids.push(result.last_insert_rowid());
    }

    album_ids
}

#[tokio::test]
async fn test_large_library_get_all_tracks() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    // Create test data
    let artists = batch_insert_artists(pool, LARGE_ARTIST_COUNT).await;
    let albums = batch_insert_albums(pool, LARGE_ALBUM_COUNT, &artists).await;
    let _tracks = batch_insert_tracks(pool, LARGE_LIBRARY_SIZE, &artists, &albums, 1).await;

    // Test retrieving all tracks
    let start = std::time::Instant::now();
    let all_tracks = soul_storage::tracks::get_all(pool).await.unwrap();
    let elapsed = start.elapsed();

    assert_eq!(
        all_tracks.len(),
        LARGE_LIBRARY_SIZE,
        "Should retrieve all {} tracks",
        LARGE_LIBRARY_SIZE
    );

    // Performance assertion: should complete in reasonable time
    assert!(
        elapsed.as_millis() < 5000,
        "get_all should complete in under 5 seconds, took {:?}",
        elapsed
    );

    println!(
        "Retrieved {} tracks in {:?}",
        all_tracks.len(),
        elapsed
    );
}

#[tokio::test]
async fn test_large_library_get_all_albums() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    // Create test data
    let artists = batch_insert_artists(pool, LARGE_ARTIST_COUNT).await;
    let _albums = batch_insert_albums(pool, LARGE_ALBUM_COUNT, &artists).await;

    // Test retrieving all albums
    let start = std::time::Instant::now();
    let all_albums = soul_storage::albums::get_all(pool).await.unwrap();
    let elapsed = start.elapsed();

    assert_eq!(
        all_albums.len(),
        LARGE_ALBUM_COUNT,
        "Should retrieve all {} albums",
        LARGE_ALBUM_COUNT
    );

    // Performance assertion
    assert!(
        elapsed.as_millis() < 1000,
        "get_all albums should complete in under 1 second, took {:?}",
        elapsed
    );

    println!("Retrieved {} albums in {:?}", all_albums.len(), elapsed);
}

#[tokio::test]
async fn test_large_library_get_all_artists() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    // Create test data
    let _artists = batch_insert_artists(pool, LARGE_ARTIST_COUNT).await;

    // Test retrieving all artists
    let start = std::time::Instant::now();
    let all_artists = soul_storage::artists::get_all(pool).await.unwrap();
    let elapsed = start.elapsed();

    assert_eq!(
        all_artists.len(),
        LARGE_ARTIST_COUNT,
        "Should retrieve all {} artists",
        LARGE_ARTIST_COUNT
    );

    // Performance assertion
    assert!(
        elapsed.as_millis() < 500,
        "get_all artists should complete in under 500ms, took {:?}",
        elapsed
    );

    println!(
        "Retrieved {} artists in {:?}",
        all_artists.len(),
        elapsed
    );
}

#[tokio::test]
async fn test_large_library_search_tracks() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    // Create test data
    let artists = batch_insert_artists(pool, 50).await;
    let albums = batch_insert_albums(pool, 100, &artists).await;
    let _tracks = batch_insert_tracks(pool, 5000, &artists, &albums, 1).await;

    // Test search performance
    let start = std::time::Instant::now();

    // Search by title pattern
    let results: Vec<(i64, String)> = sqlx::query_as(
        "SELECT id, title FROM tracks WHERE title LIKE ? LIMIT 100"
    )
    .bind("%Track 001%")
    .fetch_all(pool)
    .await
    .unwrap();

    let elapsed = start.elapsed();

    // Should find tracks matching pattern (Track 00100, Track 00101, etc.)
    assert!(
        !results.is_empty(),
        "Should find tracks matching search pattern"
    );

    // Performance assertion
    assert!(
        elapsed.as_millis() < 100,
        "Search should complete in under 100ms, took {:?}",
        elapsed
    );

    println!(
        "Search found {} tracks in {:?}",
        results.len(),
        elapsed
    );
}

#[tokio::test]
async fn test_large_library_tracks_by_artist() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    // Create test data
    let artists = batch_insert_artists(pool, 100).await;
    let albums = batch_insert_albums(pool, 200, &artists).await;
    let _tracks = batch_insert_tracks(pool, 5000, &artists, &albums, 1).await;

    // Test getting tracks by a specific artist
    let target_artist = artists[0];

    let start = std::time::Instant::now();
    let artist_tracks = soul_storage::tracks::get_by_artist(pool, target_artist)
        .await
        .unwrap();
    let elapsed = start.elapsed();

    // Artist 0 should have tracks (5000 / 100 = ~50 tracks)
    assert!(
        artist_tracks.len() >= 40, // Allow some variance
        "Artist should have multiple tracks, found {}",
        artist_tracks.len()
    );

    // All tracks should belong to the correct artist
    for track in &artist_tracks {
        assert_eq!(
            track.artist_id,
            Some(target_artist),
            "All tracks should belong to the target artist"
        );
    }

    // Performance assertion
    assert!(
        elapsed.as_millis() < 200,
        "get_by_artist should complete in under 200ms, took {:?}",
        elapsed
    );

    println!(
        "Found {} tracks for artist in {:?}",
        artist_tracks.len(),
        elapsed
    );
}

#[tokio::test]
async fn test_large_library_tracks_by_album() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    // Create test data
    let artists = batch_insert_artists(pool, 50).await;
    let albums = batch_insert_albums(pool, 200, &artists).await;
    let _tracks = batch_insert_tracks(pool, 5000, &artists, &albums, 1).await;

    // Test getting tracks by a specific album
    let target_album = albums[0];

    let start = std::time::Instant::now();
    let album_tracks = soul_storage::tracks::get_by_album(pool, target_album)
        .await
        .unwrap();
    let elapsed = start.elapsed();

    // Album 0 should have tracks (5000 / 200 = ~25 tracks)
    assert!(
        album_tracks.len() >= 20,
        "Album should have multiple tracks, found {}",
        album_tracks.len()
    );

    // All tracks should belong to the correct album
    for track in &album_tracks {
        assert_eq!(
            track.album_id,
            Some(target_album),
            "All tracks should belong to the target album"
        );
    }

    // Performance assertion
    assert!(
        elapsed.as_millis() < 100,
        "get_by_album should complete in under 100ms, took {:?}",
        elapsed
    );

    println!(
        "Found {} tracks for album in {:?}",
        album_tracks.len(),
        elapsed
    );
}

#[tokio::test]
async fn test_large_library_pagination_simulation() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    // Create test data
    let artists = batch_insert_artists(pool, 100).await;
    let albums = batch_insert_albums(pool, 200, &artists).await;
    let _tracks = batch_insert_tracks(pool, 5000, &artists, &albums, 1).await;

    // Simulate pagination with LIMIT/OFFSET
    let page_size = 100;
    let mut total_fetched = 0;
    let mut page = 0;

    let start = std::time::Instant::now();

    loop {
        let offset = page * page_size;
        let tracks: Vec<(i64, String)> = sqlx::query_as(
            "SELECT id, title FROM tracks ORDER BY id LIMIT ? OFFSET ?"
        )
        .bind(page_size as i64)
        .bind(offset as i64)
        .fetch_all(pool)
        .await
        .unwrap();

        if tracks.is_empty() {
            break;
        }

        total_fetched += tracks.len();
        page += 1;

        // Safety: break if we've fetched way more than expected
        if page > 100 {
            break;
        }
    }

    let elapsed = start.elapsed();

    assert_eq!(
        total_fetched, 5000,
        "Should fetch all tracks through pagination"
    );

    // Performance: pagination should be efficient
    assert!(
        elapsed.as_millis() < 2000,
        "Full pagination should complete in under 2 seconds, took {:?}",
        elapsed
    );

    println!(
        "Paginated through {} tracks in {} pages, took {:?}",
        total_fetched, page, elapsed
    );
}

#[tokio::test]
async fn test_large_library_concurrent_reads() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    // Create test data
    let artists = batch_insert_artists(pool, 50).await;
    let albums = batch_insert_albums(pool, 100, &artists).await;
    let _tracks = batch_insert_tracks(pool, 2000, &artists, &albums, 1).await;

    // Simulate concurrent reads (like UI virtualization requesting multiple ranges)
    let start = std::time::Instant::now();

    let handles: Vec<_> = (0..10)
        .map(|i| {
            let pool = pool.clone();
            tokio::spawn(async move {
                let offset = i * 100;
                let tracks: Vec<(i64, String)> = sqlx::query_as(
                    "SELECT id, title FROM tracks ORDER BY id LIMIT 100 OFFSET ?"
                )
                .bind(offset as i64)
                .fetch_all(&pool)
                .await
                .unwrap();
                tracks.len()
            })
        })
        .collect();

    let mut total = 0;
    for handle in handles {
        total += handle.await.unwrap();
    }

    let elapsed = start.elapsed();

    assert_eq!(total, 1000, "Should fetch 1000 tracks across all concurrent requests");

    // Performance: concurrent reads should be efficient
    assert!(
        elapsed.as_millis() < 500,
        "Concurrent reads should complete in under 500ms, took {:?}",
        elapsed
    );

    println!(
        "Concurrent reads fetched {} tracks in {:?}",
        total, elapsed
    );
}

#[tokio::test]
async fn test_large_library_filter_by_format() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    // Create tracks with different formats
    for i in 0..1000 {
        let format = match i % 4 {
            0 => "mp3",
            1 => "flac",
            2 => "aac",
            _ => "wav",
        };

        sqlx::query(
            "INSERT INTO tracks (title, origin_source_id, file_format, created_at, updated_at)
             VALUES (?, 1, ?, datetime('now'), datetime('now'))"
        )
        .bind(format!("Track {}", i))
        .bind(format)
        .execute(pool)
        .await
        .unwrap();
    }

    // Test filtering by format
    let start = std::time::Instant::now();

    let flac_tracks: Vec<(i64, String, String)> = sqlx::query_as(
        "SELECT id, title, file_format FROM tracks WHERE file_format = ?"
    )
    .bind("flac")
    .fetch_all(pool)
    .await
    .unwrap();

    let elapsed = start.elapsed();

    // Should have ~250 FLAC tracks
    assert_eq!(
        flac_tracks.len(),
        250,
        "Should find 250 FLAC tracks"
    );

    // All should be FLAC
    for (_, _, format) in &flac_tracks {
        assert_eq!(format, "flac");
    }

    // Performance assertion
    assert!(
        elapsed.as_millis() < 50,
        "Format filter should complete in under 50ms, took {:?}",
        elapsed
    );

    println!(
        "Filtered {} FLAC tracks in {:?}",
        flac_tracks.len(),
        elapsed
    );
}

#[tokio::test]
async fn test_large_library_album_track_counts() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    // Create test data
    let artists = batch_insert_artists(pool, 50).await;
    let albums = batch_insert_albums(pool, 100, &artists).await;
    let _tracks = batch_insert_tracks(pool, 2000, &artists, &albums, 1).await;

    // Test getting album track counts
    let start = std::time::Instant::now();

    let album_counts: Vec<(i64, i64)> = sqlx::query_as(
        "SELECT album_id, COUNT(*) as track_count
         FROM tracks
         WHERE album_id IS NOT NULL
         GROUP BY album_id"
    )
    .fetch_all(pool)
    .await
    .unwrap();

    let elapsed = start.elapsed();

    assert_eq!(
        album_counts.len(),
        100,
        "Should have counts for all 100 albums"
    );

    // Each album should have tracks (2000 / 100 = 20 per album)
    for (_, count) in &album_counts {
        assert!(
            *count >= 15 && *count <= 25,
            "Each album should have ~20 tracks, found {}",
            count
        );
    }

    // Performance assertion
    assert!(
        elapsed.as_millis() < 100,
        "Album count aggregation should complete in under 100ms, took {:?}",
        elapsed
    );

    println!(
        "Computed track counts for {} albums in {:?}",
        album_counts.len(),
        elapsed
    );
}

#[tokio::test]
async fn test_large_library_artist_track_counts() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    // Create test data
    let artists = batch_insert_artists(pool, 100).await;
    let albums = batch_insert_albums(pool, 200, &artists).await;
    let _tracks = batch_insert_tracks(pool, 5000, &artists, &albums, 1).await;

    // Test getting artist track counts with album counts
    let start = std::time::Instant::now();

    let artist_stats: Vec<(i64, String, i64, i64)> = sqlx::query_as(
        "SELECT
            a.id,
            a.name,
            (SELECT COUNT(*) FROM tracks t WHERE t.artist_id = a.id) as track_count,
            (SELECT COUNT(*) FROM albums al WHERE al.artist_id = a.id) as album_count
         FROM artists a"
    )
    .fetch_all(pool)
    .await
    .unwrap();

    let elapsed = start.elapsed();

    assert_eq!(
        artist_stats.len(),
        100,
        "Should have stats for all 100 artists"
    );

    // Performance assertion
    assert!(
        elapsed.as_millis() < 500,
        "Artist stats aggregation should complete in under 500ms, took {:?}",
        elapsed
    );

    println!(
        "Computed stats for {} artists in {:?}",
        artist_stats.len(),
        elapsed
    );
}

#[tokio::test]
async fn test_large_library_memory_efficient_iteration() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    // Create test data
    let artists = batch_insert_artists(pool, 50).await;
    let albums = batch_insert_albums(pool, 100, &artists).await;
    let _tracks = batch_insert_tracks(pool, 3000, &artists, &albums, 1).await;

    // Test streaming/chunked iteration to avoid loading all into memory at once
    let start = std::time::Instant::now();

    let mut total_processed = 0;
    let chunk_size = 500;
    let mut last_id = 0i64;

    loop {
        // Keyset pagination (more efficient than OFFSET for large datasets)
        let chunk: Vec<(i64, String)> = sqlx::query_as(
            "SELECT id, title FROM tracks WHERE id > ? ORDER BY id LIMIT ?"
        )
        .bind(last_id)
        .bind(chunk_size as i64)
        .fetch_all(pool)
        .await
        .unwrap();

        if chunk.is_empty() {
            break;
        }

        total_processed += chunk.len();
        last_id = chunk.last().unwrap().0;

        // Safety limit
        if total_processed > 5000 {
            break;
        }
    }

    let elapsed = start.elapsed();

    assert_eq!(
        total_processed, 3000,
        "Should process all 3000 tracks"
    );

    // Keyset pagination should be very efficient
    assert!(
        elapsed.as_millis() < 500,
        "Keyset pagination should complete in under 500ms, took {:?}",
        elapsed
    );

    println!(
        "Processed {} tracks with keyset pagination in {:?}",
        total_processed, elapsed
    );
}
