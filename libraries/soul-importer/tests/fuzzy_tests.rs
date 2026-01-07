use soul_core::types::CreateArtist;
use soul_importer::fuzzy::FuzzyMatcher;
use soul_importer::MatchType;

mod test_helpers;
use test_helpers::setup_test_db;

#[tokio::test]
async fn test_artist_exact_match() {
    let pool = setup_test_db().await;
    let matcher = FuzzyMatcher::new();

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

    // Find with exact match
    let result = matcher
        .find_or_create_artist(&pool, "The Beatles")
        .await
        .unwrap();

    assert_eq!(result.confidence, 100);
    assert_eq!(result.match_type, MatchType::Exact);
    assert_eq!(result.entity.name, "The Beatles");
}

#[tokio::test]
async fn test_artist_normalized_match() {
    let pool = setup_test_db().await;
    let matcher = FuzzyMatcher::new();

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

    // Find with case variation
    let result = matcher
        .find_or_create_artist(&pool, "the beatles")
        .await
        .unwrap();

    assert_eq!(result.confidence, 95);
    assert_eq!(result.match_type, MatchType::Normalized);
    assert_eq!(result.entity.name, "The Beatles");
}

#[tokio::test]
async fn test_artist_fuzzy_match() {
    let pool = setup_test_db().await;
    let matcher = FuzzyMatcher::new();

    // Create an artist
    soul_storage::artists::create(
        &pool,
        CreateArtist {
            name: "Metallica".to_string(),
            sort_name: Some("Metallica".to_string()),
            musicbrainz_id: None,
        },
    )
    .await
    .unwrap();

    // Find with typo (should still match with high confidence)
    let result = matcher
        .find_or_create_artist(&pool, "Metalica")
        .await
        .unwrap();

    assert!(result.confidence >= 60);
    assert!(result.confidence < 100);
    assert_eq!(result.match_type, MatchType::Fuzzy);
    assert_eq!(result.entity.name, "Metallica");
}

#[tokio::test]
async fn test_artist_creates_new_when_no_match() {
    let pool = setup_test_db().await;
    let matcher = FuzzyMatcher::new();

    // Find non-existent artist
    let result = matcher.find_or_create_artist(&pool, "Queen").await.unwrap();

    assert_eq!(result.confidence, 100);
    assert_eq!(result.match_type, MatchType::Created);
    assert_eq!(result.entity.name, "Queen");

    // Verify it was created in database
    let found = soul_storage::artists::find_by_name(&pool, "Queen")
        .await
        .unwrap();
    assert!(found.is_some());
}

#[tokio::test]
async fn test_album_exact_match() {
    let pool = setup_test_db().await;
    let matcher = FuzzyMatcher::new();

    // Create artist first
    let artist = soul_storage::artists::create(
        &pool,
        CreateArtist {
            name: "Pink Floyd".to_string(),
            sort_name: Some("Pink Floyd".to_string()),
            musicbrainz_id: None,
        },
    )
    .await
    .unwrap();

    // Create album
    soul_storage::albums::create(
        &pool,
        soul_core::types::CreateAlbum {
            title: "Dark Side of the Moon".to_string(),
            artist_id: Some(artist.id),
            year: Some(1973),
            musicbrainz_id: None,
        },
    )
    .await
    .unwrap();

    // Find with exact match
    let result = matcher
        .find_or_create_album(&pool, "Dark Side of the Moon", Some(artist.id))
        .await
        .unwrap();

    assert_eq!(result.confidence, 100);
    assert_eq!(result.match_type, MatchType::Exact);
    assert_eq!(result.entity.title, "Dark Side of the Moon");
}

#[tokio::test]
async fn test_album_normalized_match() {
    let pool = setup_test_db().await;
    let matcher = FuzzyMatcher::new();

    let artist = soul_storage::artists::create(
        &pool,
        CreateArtist {
            name: "Radiohead".to_string(),
            sort_name: Some("Radiohead".to_string()),
            musicbrainz_id: None,
        },
    )
    .await
    .unwrap();

    soul_storage::albums::create(
        &pool,
        soul_core::types::CreateAlbum {
            title: "OK Computer".to_string(),
            artist_id: Some(artist.id),
            year: None,
            musicbrainz_id: None,
        },
    )
    .await
    .unwrap();

    // Find with case variation
    let result = matcher
        .find_or_create_album(&pool, "ok computer", Some(artist.id))
        .await
        .unwrap();

    assert_eq!(result.confidence, 95);
    assert_eq!(result.match_type, MatchType::Normalized);
    assert_eq!(result.entity.title, "OK Computer");
}

#[tokio::test]
async fn test_album_different_artists_no_match() {
    let pool = setup_test_db().await;
    let matcher = FuzzyMatcher::new();

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

    // Create album for artist1
    soul_storage::albums::create(
        &pool,
        soul_core::types::CreateAlbum {
            title: "Album".to_string(),
            artist_id: Some(artist1.id),
            year: None,
            musicbrainz_id: None,
        },
    )
    .await
    .unwrap();

    // Try to find for artist2 - should create new
    let result = matcher
        .find_or_create_album(&pool, "Album", Some(artist2.id))
        .await
        .unwrap();

    assert_eq!(result.match_type, MatchType::Created);
    assert_eq!(result.entity.artist_id, Some(artist2.id));
}

#[tokio::test]
async fn test_genre_canonicalization() {
    let pool = setup_test_db().await;
    let matcher = FuzzyMatcher::new();

    // Create genre with variant spelling
    let result1 = matcher
        .find_or_create_genre(&pool, "hip hop")
        .await
        .unwrap();

    assert_eq!(result1.entity.canonical_name, "Hip-Hop");

    // Try different variant - should match same canonical
    let result2 = matcher
        .find_or_create_genre(&pool, "Hip-Hop")
        .await
        .unwrap();

    // Should find the same genre
    assert_eq!(result1.entity.id, result2.entity.id);
    assert_eq!(result2.entity.canonical_name, "Hip-Hop");
}

#[tokio::test]
async fn test_genre_various_canonicalizations() {
    let pool = setup_test_db().await;
    let matcher = FuzzyMatcher::new();

    // Test various genre canonicalizations
    let test_cases = vec![
        ("r&b", "R&B"),
        ("rnb", "R&B"),
        ("edm", "EDM"),
        ("alt rock", "Alternative Rock"),
        ("indie pop", "Indie Pop"),
        ("k pop", "K-Pop"),
    ];

    for (input, expected_canonical) in test_cases {
        let result = matcher.find_or_create_genre(&pool, input).await.unwrap();

        assert_eq!(
            result.entity.canonical_name, expected_canonical,
            "Failed for input: {}",
            input
        );
    }
}

#[tokio::test]
async fn test_genre_creates_with_title_case() {
    let pool = setup_test_db().await;
    let matcher = FuzzyMatcher::new();

    // Create genre that doesn't have special canonicalization
    let result = matcher
        .find_or_create_genre(&pool, "progressive metal")
        .await
        .unwrap();

    assert_eq!(result.entity.canonical_name, "Progressive Metal");
    assert_eq!(result.match_type, MatchType::Created);
}
