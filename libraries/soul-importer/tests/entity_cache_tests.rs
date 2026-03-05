use soul_core::types::{CreateAlbum, CreateArtist};
use soul_importer::fuzzy::{EntityCache, FuzzyMatcher};
use soul_importer::MatchType;

mod test_helpers;
use test_helpers::setup_test_db;

#[tokio::test]
async fn test_entity_cache_preload_artists() {
    let pool = setup_test_db().await;

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

    // Verify normalized lookup works for all 100 artists
    for i in 0..100 {
        let normalized = format!("artist {}", i); // lowercase
        let result = cache.find_artist_by_normalized(&normalized);
        assert!(
            result.is_some(),
            "Expected to find artist {} in cache",
            normalized
        );
    }
}

#[tokio::test]
async fn test_entity_cache_insert_updates_cache() {
    let pool = setup_test_db().await;

    let mut cache = EntityCache::preload(&pool).await.unwrap();

    // Cache should be empty initially
    assert!(cache.find_artist_by_normalized("queen").is_none());

    // Insert a new artist into the cache
    cache.insert_artist(42, "Queen");

    // Now it should be found
    let result = cache.find_artist_by_normalized("queen");
    assert!(result.is_some());
    assert_eq!(result.unwrap().0, 42);
}

#[tokio::test]
async fn test_entity_cache_avoids_db_scan() {
    let pool = setup_test_db().await;
    let matcher = FuzzyMatcher::new();

    // Create an artist in DB
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

    // Preload cache
    let mut cache = EntityCache::preload(&pool).await.unwrap();

    // Use cached matcher - should find via cache, not DB scan
    let result = matcher
        .find_or_create_artist_cached(&pool, "The Beatles", &mut cache)
        .await
        .unwrap();

    assert_eq!(result.entity.name, "The Beatles");
    assert_eq!(result.confidence, 100);
    assert_eq!(result.match_type, MatchType::Exact);
}

#[tokio::test]
async fn test_entity_cache_normalized_lookup_case_insensitive() {
    let pool = setup_test_db().await;
    let matcher = FuzzyMatcher::new();

    // Create an artist
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

    let mut cache = EntityCache::preload(&pool).await.unwrap();

    // Lookup with different casing
    let result = matcher
        .find_or_create_artist_cached(&pool, "RADIOHEAD", &mut cache)
        .await
        .unwrap();

    assert_eq!(result.entity.name, "Radiohead");
    assert_eq!(result.confidence, 95);
    assert_eq!(result.match_type, MatchType::Normalized);
}

#[tokio::test]
async fn test_entity_cache_albums_scoped_by_artist() {
    let pool = setup_test_db().await;
    let matcher = FuzzyMatcher::new();

    // Create two artists
    let artist1 = soul_storage::artists::create(
        &pool,
        CreateArtist {
            name: "Artist One".to_string(),
            sort_name: Some("Artist One".to_string()),
            musicbrainz_id: None,
        },
    )
    .await
    .unwrap();

    let artist2 = soul_storage::artists::create(
        &pool,
        CreateArtist {
            name: "Artist Two".to_string(),
            sort_name: Some("Artist Two".to_string()),
            musicbrainz_id: None,
        },
    )
    .await
    .unwrap();

    // Create album "Greatest Hits" for artist1
    soul_storage::albums::create(
        &pool,
        CreateAlbum {
            title: "Greatest Hits".to_string(),
            artist_id: Some(artist1.id),
            year: None,
            musicbrainz_id: None,
        },
    )
    .await
    .unwrap();

    let mut cache = EntityCache::preload(&pool).await.unwrap();

    // Find album for artist1 - should match existing
    let result1 = matcher
        .find_or_create_album_cached(&pool, "Greatest Hits", Some(artist1.id), &mut cache)
        .await
        .unwrap();
    assert_eq!(result1.confidence, 100);
    assert_eq!(result1.match_type, MatchType::Exact);

    // Find same album title for artist2 - should create new (different artist scope)
    let result2 = matcher
        .find_or_create_album_cached(&pool, "Greatest Hits", Some(artist2.id), &mut cache)
        .await
        .unwrap();
    assert_eq!(result2.match_type, MatchType::Created);
    assert_eq!(result2.entity.artist_id, Some(artist2.id));

    // The two albums should have different IDs
    assert_ne!(result1.entity.id, result2.entity.id);
}
