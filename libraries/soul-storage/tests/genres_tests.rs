use soul_core::types::CreateGenre;

mod test_helpers;
use test_helpers::setup_test_db;

#[tokio::test]
async fn test_create_and_get_genre() {
    let pool = setup_test_db().await;

    // Create a genre
    let genre = soul_storage::genres::create(
        &pool,
        CreateGenre {
            name: "Rock".to_string(),
            canonical_name: "Rock".to_string(),
        },
    )
    .await
    .unwrap();

    assert_eq!(genre.name, "Rock");
    assert_eq!(genre.canonical_name, "Rock");

    // Get by ID
    let fetched = soul_storage::genres::get_by_id(&pool, genre.id)
        .await
        .unwrap();
    assert!(fetched.is_some());
    assert_eq!(fetched.unwrap().name, "Rock");
}

#[tokio::test]
async fn test_find_genre_by_name() {
    let pool = setup_test_db().await;

    // Create a genre
    soul_storage::genres::create(
        &pool,
        CreateGenre {
            name: "Hip-Hop".to_string(),
            canonical_name: "Hip-Hop".to_string(),
        },
    )
    .await
    .unwrap();

    // Find by exact name
    let found = soul_storage::genres::find_by_name(&pool, "Hip-Hop")
        .await
        .unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().name, "Hip-Hop");

    // Not found
    let not_found = soul_storage::genres::find_by_name(&pool, "Jazz")
        .await
        .unwrap();
    assert!(not_found.is_none());
}

#[tokio::test]
async fn test_find_genre_by_canonical_name() {
    let pool = setup_test_db().await;

    // Create a genre with variant name
    soul_storage::genres::create(
        &pool,
        CreateGenre {
            name: "hip hop".to_string(),
            canonical_name: "Hip-Hop".to_string(),
        },
    )
    .await
    .unwrap();

    // Find by canonical name
    let found = soul_storage::genres::find_by_canonical_name(&pool, "Hip-Hop")
        .await
        .unwrap();
    assert!(found.is_some());
    let genre = found.unwrap();
    assert_eq!(genre.canonical_name, "Hip-Hop");
    assert_eq!(genre.name, "hip hop");
}

#[tokio::test]
async fn test_get_all_genres() {
    let pool = setup_test_db().await;

    // Create multiple genres
    soul_storage::genres::create(
        &pool,
        CreateGenre {
            name: "Rock".to_string(),
            canonical_name: "Rock".to_string(),
        },
    )
    .await
    .unwrap();

    soul_storage::genres::create(
        &pool,
        CreateGenre {
            name: "Jazz".to_string(),
            canonical_name: "Jazz".to_string(),
        },
    )
    .await
    .unwrap();

    soul_storage::genres::create(
        &pool,
        CreateGenre {
            name: "Electronic".to_string(),
            canonical_name: "Electronic".to_string(),
        },
    )
    .await
    .unwrap();

    let all_genres = soul_storage::genres::get_all(&pool).await.unwrap();
    assert_eq!(all_genres.len(), 3);

    let names: Vec<_> = all_genres.iter().map(|g| g.name.as_str()).collect();
    assert!(names.contains(&"Rock"));
    assert!(names.contains(&"Jazz"));
    assert!(names.contains(&"Electronic"));
}

#[tokio::test]
async fn test_add_genre_to_track() {
    let pool = setup_test_db().await;

    // Create a source and track first
    let source_id = test_helpers::create_test_source(&pool, "Local", "local").await;
    let track_id = test_helpers::create_test_track(
        &pool,
        "Test Song",
        None,
        None,
        source_id,
        Some("/path/to/song.mp3"),
    )
    .await;

    // Create a genre
    let genre = soul_storage::genres::create(
        &pool,
        CreateGenre {
            name: "Rock".to_string(),
            canonical_name: "Rock".to_string(),
        },
    )
    .await
    .unwrap();

    // Add genre to track
    soul_storage::genres::add_to_track(&pool, track_id.clone(), genre.id)
        .await
        .unwrap();

    // Get genres for track
    let track_genres = soul_storage::genres::get_by_track(&pool, track_id)
        .await
        .unwrap();

    assert_eq!(track_genres.len(), 1);
    assert_eq!(track_genres[0].name, "Rock");
}

#[tokio::test]
async fn test_add_multiple_genres_to_track() {
    let pool = setup_test_db().await;

    // Create source and track
    let source_id = test_helpers::create_test_source(&pool, "Local", "local").await;
    let track_id = test_helpers::create_test_track(
        &pool,
        "Test Song",
        None,
        None,
        source_id,
        Some("/path/to/song.mp3"),
    )
    .await;

    // Create genres
    let rock = soul_storage::genres::create(
        &pool,
        CreateGenre {
            name: "Rock".to_string(),
            canonical_name: "Rock".to_string(),
        },
    )
    .await
    .unwrap();

    let alternative = soul_storage::genres::create(
        &pool,
        CreateGenre {
            name: "Alternative".to_string(),
            canonical_name: "Alternative".to_string(),
        },
    )
    .await
    .unwrap();

    // Add both genres
    soul_storage::genres::add_to_track(&pool, track_id.clone(), rock.id)
        .await
        .unwrap();
    soul_storage::genres::add_to_track(&pool, track_id.clone(), alternative.id)
        .await
        .unwrap();

    // Get genres for track
    let track_genres = soul_storage::genres::get_by_track(&pool, track_id)
        .await
        .unwrap();

    assert_eq!(track_genres.len(), 2);
    let names: Vec<_> = track_genres.iter().map(|g| g.name.as_str()).collect();
    assert!(names.contains(&"Rock"));
    assert!(names.contains(&"Alternative"));
}

#[tokio::test]
async fn test_add_duplicate_genre_to_track_is_idempotent() {
    let pool = setup_test_db().await;

    // Create source and track
    let source_id = test_helpers::create_test_source(&pool, "Local", "local").await;
    let track_id = test_helpers::create_test_track(
        &pool,
        "Test Song",
        None,
        None,
        source_id,
        Some("/path/to/song.mp3"),
    )
    .await;

    let genre = soul_storage::genres::create(
        &pool,
        CreateGenre {
            name: "Rock".to_string(),
            canonical_name: "Rock".to_string(),
        },
    )
    .await
    .unwrap();

    // Add genre twice
    soul_storage::genres::add_to_track(&pool, track_id.clone(), genre.id)
        .await
        .unwrap();
    soul_storage::genres::add_to_track(&pool, track_id.clone(), genre.id)
        .await
        .unwrap();

    // Should only have one entry
    let track_genres = soul_storage::genres::get_by_track(&pool, track_id)
        .await
        .unwrap();

    assert_eq!(track_genres.len(), 1);
}

#[tokio::test]
async fn test_remove_genre_from_track() {
    let pool = setup_test_db().await;

    // Create source and track
    let source_id = test_helpers::create_test_source(&pool, "Local", "local").await;
    let track_id = test_helpers::create_test_track(
        &pool,
        "Test Song",
        None,
        None,
        source_id,
        Some("/path/to/song.mp3"),
    )
    .await;

    let genre = soul_storage::genres::create(
        &pool,
        CreateGenre {
            name: "Rock".to_string(),
            canonical_name: "Rock".to_string(),
        },
    )
    .await
    .unwrap();

    // Add genre
    soul_storage::genres::add_to_track(&pool, track_id.clone(), genre.id)
        .await
        .unwrap();

    // Verify it's there
    let genres = soul_storage::genres::get_by_track(&pool, track_id.clone())
        .await
        .unwrap();
    assert_eq!(genres.len(), 1);

    // Remove genre
    soul_storage::genres::remove_from_track(&pool, track_id.clone(), genre.id)
        .await
        .unwrap();

    // Verify it's gone
    let genres = soul_storage::genres::get_by_track(&pool, track_id)
        .await
        .unwrap();
    assert_eq!(genres.len(), 0);
}

#[tokio::test]
async fn test_clear_track_genres() {
    let pool = setup_test_db().await;

    // Create source and track
    let source_id = test_helpers::create_test_source(&pool, "Local", "local").await;
    let track_id = test_helpers::create_test_track(
        &pool,
        "Test Song",
        None,
        None,
        source_id,
        Some("/path/to/song.mp3"),
    )
    .await;

    // Create multiple genres
    let rock = soul_storage::genres::create(
        &pool,
        CreateGenre {
            name: "Rock".to_string(),
            canonical_name: "Rock".to_string(),
        },
    )
    .await
    .unwrap();

    let jazz = soul_storage::genres::create(
        &pool,
        CreateGenre {
            name: "Jazz".to_string(),
            canonical_name: "Jazz".to_string(),
        },
    )
    .await
    .unwrap();

    // Add both genres
    soul_storage::genres::add_to_track(&pool, track_id.clone(), rock.id)
        .await
        .unwrap();
    soul_storage::genres::add_to_track(&pool, track_id.clone(), jazz.id)
        .await
        .unwrap();

    // Verify they're there
    let genres = soul_storage::genres::get_by_track(&pool, track_id.clone())
        .await
        .unwrap();
    assert_eq!(genres.len(), 2);

    // Clear all genres
    soul_storage::genres::clear_track_genres(&pool, track_id.clone())
        .await
        .unwrap();

    // Verify they're gone
    let genres = soul_storage::genres::get_by_track(&pool, track_id)
        .await
        .unwrap();
    assert_eq!(genres.len(), 0);
}
