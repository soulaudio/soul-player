//! Integration tests for artists and albums vertical slices
//!
//! Tests artist and album CRUD operations including:
//! - Creating artists with sort names and MusicBrainz IDs
//! - Finding artists by name
//! - Creating albums with artist relationships
//! - Querying albums by artist
//! - Proper ordering and indexing

mod test_helpers;

use soul_core::types::*;
use test_helpers::*;

// ============================================================================
// Artist Tests
// ============================================================================

#[tokio::test]
async fn test_create_and_get_artist() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let artist = soul_storage::artists::create(
        pool,
        CreateArtist {
            name: "The Beatles".to_string(),
            sort_name: Some("Beatles, The".to_string()),
            musicbrainz_id: Some("b10bbbfc-cf9e-42e0-be17-e2c3e1d2600d".to_string()),
        },
    )
    .await
    .expect("Failed to create artist");

    assert_eq!(artist.name, "The Beatles");
    assert_eq!(artist.sort_name, Some("Beatles, The".to_string()));
    assert_eq!(
        artist.musicbrainz_id,
        Some("b10bbbfc-cf9e-42e0-be17-e2c3e1d2600d".to_string())
    );

    // Retrieve by ID
    let retrieved = soul_storage::artists::get_by_id(pool, artist.id)
        .await
        .expect("Failed to get artist")
        .expect("Artist not found");

    assert_eq!(retrieved.id, artist.id);
    assert_eq!(retrieved.name, "The Beatles");
}

#[tokio::test]
async fn test_find_artist_by_name() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    // Create artist
    soul_storage::artists::create(
        pool,
        CreateArtist {
            name: "Pink Floyd".to_string(),
            sort_name: None,
            musicbrainz_id: None,
        },
    )
    .await
    .unwrap();

    // Find by exact name
    let found = soul_storage::artists::find_by_name(pool, "Pink Floyd")
        .await
        .expect("Query failed")
        .expect("Artist not found");

    assert_eq!(found.name, "Pink Floyd");

    // Case-sensitive: shouldn't find with different case
    let not_found = soul_storage::artists::find_by_name(pool, "pink floyd")
        .await
        .expect("Query failed");

    assert!(not_found.is_none(), "Search should be case-sensitive");
}

#[tokio::test]
async fn test_get_all_artists_sorted() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    // Create artists with sort names
    soul_storage::artists::create(
        pool,
        CreateArtist {
            name: "The Beatles".to_string(),
            sort_name: Some("Beatles, The".to_string()),
            musicbrainz_id: None,
        },
    )
    .await
    .unwrap();

    soul_storage::artists::create(
        pool,
        CreateArtist {
            name: "Pink Floyd".to_string(),
            sort_name: Some("Pink Floyd".to_string()),
            musicbrainz_id: None,
        },
    )
    .await
    .unwrap();

    soul_storage::artists::create(
        pool,
        CreateArtist {
            name: "The Who".to_string(),
            sort_name: Some("Who, The".to_string()),
            musicbrainz_id: None,
        },
    )
    .await
    .unwrap();

    // Get all artists (should be sorted by sort_name)
    let artists = soul_storage::artists::get_all(pool)
        .await
        .expect("Failed to get artists");

    assert_eq!(artists.len(), 3);

    // Verify ordering: "Beatles, The" < "Pink Floyd" < "Who, The"
    assert_eq!(artists[0].name, "The Beatles");
    assert_eq!(artists[1].name, "Pink Floyd");
    assert_eq!(artists[2].name, "The Who");
}

#[tokio::test]
async fn test_artist_without_sort_name() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let artist = soul_storage::artists::create(
        pool,
        CreateArtist {
            name: "Radiohead".to_string(),
            sort_name: None,
            musicbrainz_id: None,
        },
    )
    .await
    .unwrap();

    assert_eq!(artist.name, "Radiohead");
    assert!(artist.sort_name.is_none());
}

// ============================================================================
// Album Tests
// ============================================================================

#[tokio::test]
async fn test_create_and_get_album() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    // Create artist first
    let artist = soul_storage::artists::create(
        pool,
        CreateArtist {
            name: "Led Zeppelin".to_string(),
            sort_name: None,
            musicbrainz_id: None,
        },
    )
    .await
    .unwrap();

    // Create album
    let album = soul_storage::albums::create(
        pool,
        CreateAlbum {
            title: "Led Zeppelin IV".to_string(),
            artist_id: Some(artist.id),
            year: Some(1971),
            musicbrainz_id: Some("test-mb-id".to_string()),
        },
    )
    .await
    .expect("Failed to create album");

    assert_eq!(album.title, "Led Zeppelin IV");
    assert_eq!(album.artist_id, Some(artist.id));
    assert_eq!(album.artist_name, Some("Led Zeppelin".to_string()));
    assert_eq!(album.year, Some(1971));

    // Retrieve by ID
    let retrieved = soul_storage::albums::get_by_id(pool, album.id)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(retrieved.title, "Led Zeppelin IV");
    assert_eq!(retrieved.artist_name, Some("Led Zeppelin".to_string()));
}

#[tokio::test]
async fn test_get_albums_by_artist() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let artist = soul_storage::artists::create(
        pool,
        CreateArtist {
            name: "David Bowie".to_string(),
            sort_name: None,
            musicbrainz_id: None,
        },
    )
    .await
    .unwrap();

    // Create multiple albums
    soul_storage::albums::create(
        pool,
        CreateAlbum {
            title: "The Rise and Fall of Ziggy Stardust".to_string(),
            artist_id: Some(artist.id),
            year: Some(1972),
            musicbrainz_id: None,
        },
    )
    .await
    .unwrap();

    soul_storage::albums::create(
        pool,
        CreateAlbum {
            title: "Heroes".to_string(),
            artist_id: Some(artist.id),
            year: Some(1977),
            musicbrainz_id: None,
        },
    )
    .await
    .unwrap();

    soul_storage::albums::create(
        pool,
        CreateAlbum {
            title: "Let's Dance".to_string(),
            artist_id: Some(artist.id),
            year: Some(1983),
            musicbrainz_id: None,
        },
    )
    .await
    .unwrap();

    // Get albums by artist (should be sorted by year DESC, then title)
    let albums = soul_storage::albums::get_by_artist(pool, artist.id)
        .await
        .unwrap();

    assert_eq!(albums.len(), 3);

    // Verify ordering: 1983 > 1977 > 1972
    assert_eq!(albums[0].title, "Let's Dance");
    assert_eq!(albums[1].title, "Heroes");
    assert_eq!(albums[2].title, "The Rise and Fall of Ziggy Stardust");
}

#[tokio::test]
async fn test_get_all_albums() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let artist = soul_storage::artists::create(
        pool,
        CreateArtist {
            name: "Test Artist".to_string(),
            sort_name: None,
            musicbrainz_id: None,
        },
    )
    .await
    .unwrap();

    soul_storage::albums::create(
        pool,
        CreateAlbum {
            title: "Album A".to_string(),
            artist_id: Some(artist.id),
            year: None,
            musicbrainz_id: None,
        },
    )
    .await
    .unwrap();

    soul_storage::albums::create(
        pool,
        CreateAlbum {
            title: "Album B".to_string(),
            artist_id: Some(artist.id),
            year: None,
            musicbrainz_id: None,
        },
    )
    .await
    .unwrap();

    let albums = soul_storage::albums::get_all(pool).await.unwrap();

    assert!(albums.len() >= 2);

    // Verify artist names are denormalized
    for album in albums {
        assert!(album.artist_name.is_some());
    }
}

#[tokio::test]
async fn test_album_without_artist() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    // Create album without artist (e.g., compilation)
    let album = soul_storage::albums::create(
        pool,
        CreateAlbum {
            title: "Various Artists Compilation".to_string(),
            artist_id: None,
            year: Some(2000),
            musicbrainz_id: None,
        },
    )
    .await
    .unwrap();

    assert_eq!(album.title, "Various Artists Compilation");
    assert!(album.artist_id.is_none());
    assert!(album.artist_name.is_none());
}

#[tokio::test]
async fn test_artist_deletion_sets_album_artist_to_null() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let artist = create_test_artist(pool, "Test Artist", None).await;

    let album = soul_storage::albums::create(
        pool,
        CreateAlbum {
            title: "Test Album".to_string(),
            artist_id: Some(artist),
            year: None,
            musicbrainz_id: None,
        },
    )
    .await
    .unwrap();

    // Delete artist (ON DELETE SET NULL)
    sqlx::query!("DELETE FROM artists WHERE id = ?", artist)
        .execute(pool)
        .await
        .unwrap();

    // Album should still exist with artist_id = NULL
    let retrieved = soul_storage::albums::get_by_id(pool, album.id)
        .await
        .unwrap()
        .unwrap();

    assert!(retrieved.artist_id.is_none());
    assert!(retrieved.artist_name.is_none());
}

#[tokio::test]
async fn test_musicbrainz_id_uniqueness() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let artist = create_test_artist(pool, "Artist", None).await;

    // Create first album with MusicBrainz ID
    soul_storage::albums::create(
        pool,
        CreateAlbum {
            title: "Album 1".to_string(),
            artist_id: Some(artist),
            year: None,
            musicbrainz_id: Some("unique-mb-id".to_string()),
        },
    )
    .await
    .unwrap();

    // Try to create another album with the same MusicBrainz ID
    let result = soul_storage::albums::create(
        pool,
        CreateAlbum {
            title: "Album 2".to_string(),
            artist_id: Some(artist),
            year: None,
            musicbrainz_id: Some("unique-mb-id".to_string()),
        },
    )
    .await;

    assert!(result.is_err(), "Duplicate MusicBrainz ID should fail");
}
