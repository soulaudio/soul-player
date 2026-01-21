use soul_core::types::{CreateAlbum, CreateArtist, CreateTrack};
use soul_storage::{albums, artists, create_pool, run_migrations, tracks};
use std::time::Instant;
use tempfile::TempDir;

/// Helper to create a test database with migrations
async fn setup_test_db() -> (sqlx::SqlitePool, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test_performance.db");
    let database_url = format!("sqlite:{}", db_path.display());

    let pool = create_pool(&database_url)
        .await
        .expect("Failed to create pool");
    run_migrations(&pool)
        .await
        .expect("Failed to run migrations");

    (pool, temp_dir)
}

/// Helper to insert test data
async fn insert_test_data(pool: &sqlx::SqlitePool, num_tracks: usize) -> (i64, i64, i64) {
    let user_id = 1i64;

    let artist_id = artists::create(
        pool,
        CreateArtist {
            name: "Test Artist".to_string(),
            sort_name: Some("test-artist".to_string()),
            musicbrainz_id: None,
        },
    )
    .await
    .expect("Failed to create artist")
    .id;

    let album_id = albums::create(
        pool,
        CreateAlbum {
            title: "Test Album".to_string(),
            artist_id: Some(artist_id),
            year: None,
            musicbrainz_id: None,
        },
    )
    .await
    .expect("Failed to create album")
    .id;

    for i in 0..num_tracks {
        let track_title = format!("Test Track {}", i + 1);
        let file_path = format!("/test/path/track_{}.flac", i + 1);

        tracks::create(
            pool,
            CreateTrack {
                title: track_title,
                artist_id: Some(artist_id),
                album_id: Some(album_id),
                album_artist_id: None,
                track_number: Some(i as i32 + 1),
                disc_number: Some(1),
                year: None,
                duration_seconds: None,
                bitrate: None,
                sample_rate: None,
                channels: None,
                file_format: "flac".to_string(),
                file_hash: None,
                origin_source_id: 1, // local source
                local_file_path: Some(file_path),
                musicbrainz_recording_id: None,
                fingerprint: None,
            },
        )
        .await
        .expect("Failed to create track");
    }

    (user_id, artist_id, album_id)
}

#[tokio::test]
async fn test_get_all_paginated_performance() {
    let (pool, _temp_dir) = setup_test_db().await;
    let (_user_id, _, _) = insert_test_data(&pool, 1000).await;

    let start = Instant::now();
    let result = tracks::get_all_paginated(&pool, 0, 100)
        .await
        .expect("Failed to get tracks");
    let duration = start.elapsed();

    println!("get_all_paginated(100 tracks) took: {:?}", duration);
    assert!(!result.is_empty());

    // Will FAIL before fix (expect 200-500ms), should be <200ms after
    assert!(
        duration.as_millis() < 500,
        "took {}ms (target <200ms after fix)",
        duration.as_millis()
    );
}

#[tokio::test]
async fn test_get_by_album_paginated_performance() {
    let (pool, _temp_dir) = setup_test_db().await;
    let (_user_id, _, album_id) = insert_test_data(&pool, 50).await;

    let start = Instant::now();
    let result = tracks::get_by_album_paginated(&pool, album_id, 0, 50)
        .await
        .expect("Failed to get tracks");
    let duration = start.elapsed();

    println!("get_by_album_paginated(50 tracks) took: {:?}", duration);
    assert_eq!(result.len(), 50);

    assert!(
        duration.as_millis() < 300,
        "took {}ms (target <100ms after fix)",
        duration.as_millis()
    );
}

#[tokio::test]
async fn test_get_by_id_performance() {
    let (pool, _temp_dir) = setup_test_db().await;
    let (_user_id, _, _) = insert_test_data(&pool, 100).await;

    let all_tracks = tracks::get_all(&pool)
        .await
        .expect("Failed to get all tracks");
    let track_id = all_tracks[0].id.clone();

    let start = Instant::now();
    let result = tracks::get_by_id(&pool, track_id)
        .await
        .expect("Failed to get track");
    let duration = start.elapsed();

    println!("get_by_id took: {:?}", duration);
    assert!(result.is_some());

    assert!(
        duration.as_millis() < 100,
        "took {}ms (target <50ms after fix)",
        duration.as_millis()
    );
}
