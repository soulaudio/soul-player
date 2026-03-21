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

    let folder = "/music/shared-folder";

    // Create album "Greatest Hits" for artist1
    soul_storage::albums::create(
        &pool,
        CreateAlbum {
            title: "Greatest Hits".to_string(),
            artist_id: Some(artist1.id),
            year: None,
            musicbrainz_id: None,
            folder_path: folder.to_string(),
        },
    )
    .await
    .unwrap();

    let mut cache = EntityCache::preload(&pool).await.unwrap();

    // Find album for artist1 - should match existing
    let result1 = matcher
        .find_or_create_album_cached(&pool, "Greatest Hits", Some(artist1.id), folder, &mut cache)
        .await
        .unwrap();
    assert_eq!(result1.confidence, 100);
    assert_eq!(result1.match_type, MatchType::Exact);

    // Find same album title for artist2 - should create new (different artist scope)
    let result2 = matcher
        .find_or_create_album_cached(&pool, "Greatest Hits", Some(artist2.id), folder, &mut cache)
        .await
        .unwrap();
    assert_eq!(result2.match_type, MatchType::Created);
    assert_eq!(result2.entity.artist_id, Some(artist2.id));

    // The two albums should have different IDs
    assert_ne!(result1.entity.id, result2.entity.id);
}

/// Same artist/title in sibling folders at the same depth → separate albums.
///
/// Two sibling folders (neither is a subfolder of the other) that happen to share
/// the same title+artist must never be merged. Each folder gets its own album record.
/// The only exception is when `are_discs_of_same_album` applies: siblings whose shared
/// parent folder is named after the album (e.g. `.../Live Album/Disc 1` + `.../Live Album/Disc 2`).
///
/// `/music/disc1` and `/music/disc2` do NOT qualify as disc folders of the same album
/// because their shared parent (`music`) is not named after the album title.
#[tokio::test]
async fn test_entity_cache_albums_same_artist_different_folders_same_album() {
    let pool = setup_test_db().await;
    let matcher = FuzzyMatcher::new();

    let artist = soul_storage::artists::create(
        &pool,
        CreateArtist {
            name: "The Artist".to_string(),
            sort_name: Some("Artist".to_string()),
            musicbrainz_id: None,
        },
    )
    .await
    .unwrap();

    let folder1 = "/music/disc1";
    let folder2 = "/music/disc2";

    soul_storage::albums::create(
        &pool,
        CreateAlbum {
            title: "Live Album".to_string(),
            artist_id: Some(artist.id),
            year: None,
            musicbrainz_id: None,
            folder_path: folder1.to_string(),
        },
    )
    .await
    .unwrap();

    let mut cache = EntityCache::preload(&pool).await.unwrap();

    // Same title + artist in folder1 → existing album
    let result1 = matcher
        .find_or_create_album_cached(&pool, "Live Album", Some(artist.id), folder1, &mut cache)
        .await
        .unwrap();
    assert_eq!(result1.match_type, MatchType::Exact);

    // Same title + artist in a sibling folder → separate album.
    // disc1 and disc2 are at the same depth and share no album-named parent → must NOT merge.
    let result2 = matcher
        .find_or_create_album_cached(&pool, "Live Album", Some(artist.id), folder2, &mut cache)
        .await
        .unwrap();
    assert_eq!(
        result2.match_type,
        MatchType::Created,
        "Same-depth sibling folders must create a separate album, not merge"
    );
    assert_ne!(
        result1.entity.id, result2.entity.id,
        "Sibling folders must have different album IDs"
    );
}

/// Tracks with featuring artists ("Artist A feat. Artist B") must share the same album
/// as tracks by the plain album artist ("Artist A"), when ALBUMARTIST tag matches.
///
/// This simulates importing two tracks from the same folder/album:
///   - Track 1: artist="Artist A",                album_artist="Artist A" → normal track
///   - Track 2: artist="Artist A feat. Artist B", album_artist="Artist A" → featuring track
///
/// Both should resolve to the SAME album row because album matching must use
/// the album_artist (ALBUMARTIST tag), not the track artist (ARTIST tag).
#[tokio::test]
async fn test_featuring_artist_tracks_share_same_album() {
    let pool = setup_test_db().await;
    let matcher = FuzzyMatcher::new();

    // Pre-create the album artist
    let album_artist = soul_storage::artists::create(
        &pool,
        CreateArtist {
            name: "Artist A".to_string(),
            sort_name: Some("Artist A".to_string()),
            musicbrainz_id: None,
        },
    )
    .await
    .unwrap();

    // The featuring artist (has its own artist row)
    let feat_artist = soul_storage::artists::create(
        &pool,
        CreateArtist {
            name: "Artist A feat. Artist B".to_string(),
            sort_name: Some("Artist A feat. Artist B".to_string()),
            musicbrainz_id: None,
        },
    )
    .await
    .unwrap();

    let folder = "/music/artist-a/great-album";
    let mut cache = EntityCache::preload(&pool).await.unwrap();

    // Track 1: plain artist, album_artist matches — creates the album
    let result1 = matcher
        .find_or_create_album_cached(
            &pool,
            "Great Album",
            Some(album_artist.id), // album_artist_id used for matching
            folder,
            &mut cache,
        )
        .await
        .unwrap();
    assert_eq!(result1.match_type, MatchType::Created);

    // Track 2: featuring artist, but album_artist still "Artist A"
    // Must find the SAME album, not create a new one.
    // Callers must pass album_artist_id (not track artist_id) here.
    let result2 = matcher
        .find_or_create_album_cached(
            &pool,
            "Great Album",
            Some(album_artist.id), // album_artist_id used for matching (not feat_artist.id)
            folder,
            &mut cache,
        )
        .await
        .unwrap();
    assert_ne!(
        result2.match_type,
        MatchType::Created,
        "Featuring-artist track must reuse the existing album, not create a duplicate. \
         Album matching must use album_artist_id, not track artist_id."
    );
    assert_eq!(
        result1.entity.id, result2.entity.id,
        "Both tracks must share the same album ID"
    );

    // Verify: if we naively pass the featuring-artist's ID (the bug), a new album is created
    let result_bug = matcher
        .find_or_create_album_cached(
            &pool,
            "Great Album",
            Some(feat_artist.id), // BUG: using track artist_id instead of album_artist_id
            folder,
            &mut cache,
        )
        .await
        .unwrap();
    assert_eq!(
        result_bug.match_type,
        MatchType::Created,
        "Passing the featuring-artist ID creates a duplicate album — this is what the bug does"
    );
    assert_ne!(
        result1.entity.id, result_bug.entity.id,
        "The duplicate album must be distinct from the correct one"
    );
}

// ── Subfolder album merge tests ───────────────────────────────────────────────

#[tokio::test]
async fn subfolder_bsides_merges_into_parent_album() {
    let pool = setup_test_db().await;
    let matcher = FuzzyMatcher::new();
    let mut cache = EntityCache::preload(&pool).await.unwrap();

    let artist = matcher
        .find_or_create_artist_cached(&pool, "Tame Impala", &mut cache)
        .await
        .unwrap();
    let aid = Some(artist.entity.id);

    // Root album created first
    let root = matcher
        .find_or_create_album_cached(
            &pool,
            "Currents",
            aid,
            "/music/Tame Impala/Currents",
            &mut cache,
        )
        .await
        .unwrap();

    // B-Sides in subfolder — same title + artist
    let bsides = matcher
        .find_or_create_album_cached(
            &pool,
            "Currents",
            aid,
            "/music/Tame Impala/Currents/B-Sides",
            &mut cache,
        )
        .await
        .unwrap();

    // Must be the SAME album record
    assert_eq!(
        root.entity.id, bsides.entity.id,
        "B-Sides should merge into parent album"
    );
}

#[tokio::test]
async fn subfolder_disc_two_merges_into_parent_album() {
    let pool = setup_test_db().await;
    let matcher = FuzzyMatcher::new();
    let mut cache = EntityCache::preload(&pool).await.unwrap();

    let artist = matcher
        .find_or_create_artist_cached(&pool, "Artist X", &mut cache)
        .await
        .unwrap();
    let aid = Some(artist.entity.id);

    let disc1 = matcher
        .find_or_create_album_cached(
            &pool,
            "The Album",
            aid,
            "/music/Artist X/The Album/Disc 1",
            &mut cache,
        )
        .await
        .unwrap();
    let disc2 = matcher
        .find_or_create_album_cached(
            &pool,
            "The Album",
            aid,
            "/music/Artist X/The Album/Disc 2",
            &mut cache,
        )
        .await
        .unwrap();

    assert_eq!(
        disc1.entity.id, disc2.entity.id,
        "Disc 2 must merge with Disc 1"
    );
}

#[tokio::test]
async fn subfolder_discovery_order_independent() {
    let pool = setup_test_db().await;
    let matcher = FuzzyMatcher::new();
    let mut cache = EntityCache::preload(&pool).await.unwrap();

    let artist = matcher
        .find_or_create_artist_cached(&pool, "Artist Y", &mut cache)
        .await
        .unwrap();
    let aid = Some(artist.entity.id);

    // Subfolder discovered BEFORE root
    let extras = matcher
        .find_or_create_album_cached(
            &pool,
            "Deep Blue",
            aid,
            "/music/Artist Y/Deep Blue/Extras",
            &mut cache,
        )
        .await
        .unwrap();
    let root = matcher
        .find_or_create_album_cached(
            &pool,
            "Deep Blue",
            aid,
            "/music/Artist Y/Deep Blue",
            &mut cache,
        )
        .await
        .unwrap();

    assert_eq!(
        root.entity.id, extras.entity.id,
        "Root discovered after subfolder — must merge"
    );
}

#[tokio::test]
async fn subfolder_album_folder_path_set_to_parent() {
    let pool = setup_test_db().await;
    let matcher = FuzzyMatcher::new();
    let mut cache = EntityCache::preload(&pool).await.unwrap();

    let artist = matcher
        .find_or_create_artist_cached(&pool, "Artist Z", &mut cache)
        .await
        .unwrap();
    let aid = Some(artist.entity.id);

    // Subfolder first — creates album with subfolder path
    matcher
        .find_or_create_album_cached(
            &pool,
            "Ocean",
            aid,
            "/music/Artist Z/Ocean/Extras",
            &mut cache,
        )
        .await
        .unwrap();
    // Root second — should update the stored folder_path to the shorter (parent) path
    let result = matcher
        .find_or_create_album_cached(&pool, "Ocean", aid, "/music/Artist Z/Ocean", &mut cache)
        .await
        .unwrap();

    assert_eq!(
        result.entity.folder_path, "/music/Artist Z/Ocean",
        "folder_path should be promoted to the outermost (shortest) path"
    );
}

#[tokio::test]
async fn sibling_folders_same_title_not_merged() {
    let pool = setup_test_db().await;
    let matcher = FuzzyMatcher::new();
    let mut cache = EntityCache::preload(&pool).await.unwrap();

    let artist = matcher
        .find_or_create_artist_cached(&pool, "Various Artists", &mut cache)
        .await
        .unwrap();
    let aid = Some(artist.entity.id);

    // Two sibling folders — neither is a subfolder of the other
    let album_a = matcher
        .find_or_create_album_cached(
            &pool,
            "Hits",
            aid,
            "/music/Various Artists/Hits 2020",
            &mut cache,
        )
        .await
        .unwrap();
    let album_b = matcher
        .find_or_create_album_cached(
            &pool,
            "Hits",
            aid,
            "/music/Various Artists/Hits 2021",
            &mut cache,
        )
        .await
        .unwrap();

    assert_ne!(
        album_a.entity.id, album_b.entity.id,
        "Sibling folders must NOT be merged"
    );
}

#[tokio::test]
async fn different_artist_not_merged() {
    let pool = setup_test_db().await;
    let matcher = FuzzyMatcher::new();
    let mut cache = EntityCache::preload(&pool).await.unwrap();

    let artist_a = matcher
        .find_or_create_artist_cached(&pool, "Artist A", &mut cache)
        .await
        .unwrap();
    let artist_b = matcher
        .find_or_create_artist_cached(&pool, "Artist B", &mut cache)
        .await
        .unwrap();

    let album_a = matcher
        .find_or_create_album_cached(
            &pool,
            "Debut",
            Some(artist_a.entity.id),
            "/music/Artist A/Debut/Extras",
            &mut cache,
        )
        .await
        .unwrap();
    let album_b = matcher
        .find_or_create_album_cached(
            &pool,
            "Debut",
            Some(artist_b.entity.id),
            "/music/Artist B/Debut",
            &mut cache,
        )
        .await
        .unwrap();

    assert_ne!(
        album_a.entity.id, album_b.entity.id,
        "Different artists must NOT be merged"
    );
}

#[tokio::test]
async fn rescan_idempotent_after_merge() {
    let pool = setup_test_db().await;
    let matcher = FuzzyMatcher::new();
    let mut cache = EntityCache::preload(&pool).await.unwrap();

    let artist = matcher
        .find_or_create_artist_cached(&pool, "Idempotent Artist", &mut cache)
        .await
        .unwrap();
    let aid = Some(artist.entity.id);

    // First scan
    let a1 = matcher
        .find_or_create_album_cached(
            &pool,
            "Idempotent",
            aid,
            "/music/Idempotent Artist/Album",
            &mut cache,
        )
        .await
        .unwrap();
    let b1 = matcher
        .find_or_create_album_cached(
            &pool,
            "Idempotent",
            aid,
            "/music/Idempotent Artist/Album/Extras",
            &mut cache,
        )
        .await
        .unwrap();
    assert_eq!(a1.entity.id, b1.entity.id, "First scan: must merge");

    // Second scan — fresh cache simulates a new scan session
    let mut cache2 = EntityCache::preload(&pool).await.unwrap();
    let a2 = matcher
        .find_or_create_album_cached(
            &pool,
            "Idempotent",
            aid,
            "/music/Idempotent Artist/Album",
            &mut cache2,
        )
        .await
        .unwrap();
    let b2 = matcher
        .find_or_create_album_cached(
            &pool,
            "Idempotent",
            aid,
            "/music/Idempotent Artist/Album/Extras",
            &mut cache2,
        )
        .await
        .unwrap();

    assert_eq!(
        a2.entity.id, b2.entity.id,
        "Second scan: must not create a new album"
    );
    assert_eq!(
        a1.entity.id, a2.entity.id,
        "Must be the same album as first scan"
    );
}
