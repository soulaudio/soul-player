//! Integration tests for tracks vertical slice
//!
//! Tests track operations including:
//! - CRUD operations with denormalized data
//! - Multi-source availability tracking
//! - Play history and statistics
//! - Transaction correctness
//! - Filtering by artist, album, source

mod test_helpers;

use test_helpers::*;
use soul_core::types::*;

#[tokio::test]
async fn test_create_and_get_track() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let artist = create_test_artist(pool, "Test Artist", None).await;
    let album = create_test_album(pool, "Test Album", Some(artist), Some(2024)).await;

    let track = soul_storage::tracks::create(
        pool,
        CreateTrack {
            title: "Test Song".to_string(),
            artist_id: Some(artist),
            album_id: Some(album),
            album_artist_id: None,
            track_number: Some(1),
            disc_number: Some(1),
            year: Some(2024),
            duration_seconds: Some(180.5),
            bitrate: Some(320),
            sample_rate: Some(44100),
            channels: Some(2),
            file_format: "mp3".to_string(),
            origin_source_id: 1, // Default local source
            local_file_path: Some("/music/test.mp3".to_string()),
            musicbrainz_recording_id: None,
            fingerprint: None,
        },
    )
    .await
    .expect("Failed to create track");

    assert_eq!(track.title, "Test Song");
    assert_eq!(track.artist_id, Some(artist));
    assert_eq!(track.album_id, Some(album));
    assert_eq!(track.duration_seconds, Some(180.5));

    // Verify denormalized data
    assert_eq!(track.artist_name, Some("Test Artist".to_string()));
    assert_eq!(track.album_title, Some("Test Album".to_string()));

    // Retrieve by ID
    let retrieved = soul_storage::tracks::get_by_id(pool, track.id)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(retrieved.id, track.id);
    assert_eq!(retrieved.title, "Test Song");
}

#[tokio::test]
async fn test_track_availability_tracking() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let track_id = create_test_track(
        pool,
        "Available Track",
        None,
        None,
        1,
        Some("/music/track.mp3"),
    )
    .await;

    // Get availability
    let availability = soul_storage::tracks::get_availability(pool, track_id)
        .await
        .expect("Failed to get availability");

    assert!(!availability.is_empty());
    assert_eq!(availability[0].source_id, 1);
    assert_eq!(availability[0].status, AvailabilityStatus::LocalFile);
    assert_eq!(availability[0].local_file_path, Some("/music/track.mp3".to_string()));
}

#[tokio::test]
async fn test_track_with_multiple_sources() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    // Create a server source
    let server_id = create_test_source(pool, "Test Server", "server").await;

    // Create track from server
    let track_id = create_test_track(pool, "Multi-Source Track", None, None, server_id, None).await;

    // Add track availability in local cache
    sqlx::query!(
        "INSERT INTO track_sources (track_id, source_id, status, local_file_path)
         VALUES (?, 1, 'cached', '/cache/track.mp3')",
        track_id
    )
    .execute(pool)
    .await
    .unwrap();

    // Add stream-only availability from another source
    let server2_id = create_test_source(pool, "Server 2", "server").await;
    sqlx::query!(
        "INSERT INTO track_sources (track_id, source_id, status, server_path)
         VALUES (?, ?, 'stream_only', '/api/tracks/stream')",
        track_id,
        server2_id
    )
    .execute(pool)
    .await
    .unwrap();

    // Get availability
    let availability = soul_storage::tracks::get_availability(pool, track_id)
        .await
        .unwrap();

    assert_eq!(availability.len(), 3);

    // Verify each source
    let cached = availability.iter().find(|a| a.status == AvailabilityStatus::Cached).unwrap();
    assert_eq!(cached.source_id, 1);
    assert_eq!(cached.local_file_path, Some("/cache/track.mp3".to_string()));

    let stream = availability
        .iter()
        .find(|a| a.status == AvailabilityStatus::StreamOnly)
        .unwrap();
    assert_eq!(stream.server_path, Some("/api/tracks/stream".to_string()));
}

#[tokio::test]
async fn test_get_tracks_by_artist() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let artist = create_test_artist(pool, "Test Artist", None).await;
    let album1 = create_test_album(pool, "Album A", Some(artist), Some(2020)).await;
    let album2 = create_test_album(pool, "Album B", Some(artist), Some(2021)).await;

    // Create tracks in different albums
    create_test_track(pool, "Song 1", Some(artist), Some(album1), 1, Some("/music/1.mp3")).await;
    create_test_track(pool, "Song 2", Some(artist), Some(album2), 1, Some("/music/2.mp3")).await;
    create_test_track(pool, "Song 3", Some(artist), Some(album1), 1, Some("/music/3.mp3")).await;

    let tracks = soul_storage::tracks::get_by_artist(pool, artist)
        .await
        .unwrap();

    assert!(tracks.len() >= 3);

    // All tracks should have artist_name denormalized
    for track in &tracks {
        assert_eq!(track.artist_name, Some("Test Artist".to_string()));
    }
}

#[tokio::test]
async fn test_get_tracks_by_album() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let album = create_test_album(pool, "Test Album", None, None).await;

    // Create tracks with different disc and track numbers
    let track1 = soul_storage::tracks::create(
        pool,
        CreateTrack {
            title: "Disc 1 Track 1".to_string(),
            artist_id: None,
            album_id: Some(album),
            album_artist_id: None,
            track_number: Some(1),
            disc_number: Some(1),
            year: None,
            duration_seconds: None,
            bitrate: None,
            sample_rate: None,
            channels: None,
            file_format: "mp3".to_string(),
            origin_source_id: 1,
            local_file_path: Some("/music/d1t1.mp3".to_string()),
            musicbrainz_recording_id: None,
            fingerprint: None,
        },
    )
    .await
    .unwrap();

    let track2 = soul_storage::tracks::create(
        pool,
        CreateTrack {
            title: "Disc 1 Track 2".to_string(),
            artist_id: None,
            album_id: Some(album),
            album_artist_id: None,
            track_number: Some(2),
            disc_number: Some(1),
            year: None,
            duration_seconds: None,
            bitrate: None,
            sample_rate: None,
            channels: None,
            file_format: "mp3".to_string(),
            origin_source_id: 1,
            local_file_path: Some("/music/d1t2.mp3".to_string()),
            musicbrainz_recording_id: None,
            fingerprint: None,
        },
    )
    .await
    .unwrap();

    let tracks = soul_storage::tracks::get_by_album(pool, album)
        .await
        .unwrap();

    assert_eq!(tracks.len(), 2);

    // Verify ordering (disc_number, track_number)
    assert_eq!(tracks[0].id, track1.id);
    assert_eq!(tracks[1].id, track2.id);
}

#[tokio::test]
async fn test_get_tracks_by_source() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let server = create_test_source(pool, "Server", "server").await;

    // Create tracks from different sources
    create_test_track(pool, "Local Track", None, None, 1, Some("/music/local.mp3")).await;
    create_test_track(pool, "Server Track", None, None, server, None).await;

    // Get tracks from server
    let server_tracks = soul_storage::tracks::get_by_source(pool, server)
        .await
        .unwrap();

    assert_eq!(server_tracks.len(), 1);
    assert_eq!(server_tracks[0].title, "Server Track");
    assert_eq!(server_tracks[0].origin_source_id, server);
}

#[tokio::test]
async fn test_update_track() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let track_id = create_test_track(pool, "Original Title", None, None, 1, Some("/music/track.mp3")).await;

    // Update track
    let updated = soul_storage::tracks::update(
        pool,
        track_id,
        UpdateTrack {
            title: Some("Updated Title".to_string()),
            duration_seconds: Some(200.5),
            bitrate: Some(256),
            ..Default::default()
        },
    )
    .await
    .expect("Failed to update track");

    assert_eq!(updated.title, "Updated Title");
    assert_eq!(updated.duration_seconds, Some(200.5));
    assert_eq!(updated.bitrate, Some(256));
}

#[tokio::test]
async fn test_update_track_partial() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let original = soul_storage::tracks::create(
        pool,
        CreateTrack {
            title: "Test".to_string(),
            artist_id: None,
            album_id: None,
            album_artist_id: None,
            track_number: Some(1),
            disc_number: None,
            year: Some(2024),
            duration_seconds: Some(180.0),
            bitrate: Some(320),
            sample_rate: None,
            channels: None,
            file_format: "mp3".to_string(),
            origin_source_id: 1,
            local_file_path: Some("/music/test.mp3".to_string()),
            musicbrainz_recording_id: None,
            fingerprint: None,
        },
    )
    .await
    .unwrap();

    // Update only year
    let updated = soul_storage::tracks::update(
        pool,
        original.id,
        UpdateTrack {
            year: Some(2025),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    // Other fields should remain unchanged
    assert_eq!(updated.title, "Test");
    assert_eq!(updated.year, Some(2025));
    assert_eq!(updated.duration_seconds, Some(180.0));
    assert_eq!(updated.bitrate, Some(320));
}

#[tokio::test]
async fn test_delete_track() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let track_id = create_test_track(pool, "To Delete", None, None, 1, Some("/music/delete.mp3")).await;

    // Verify exists
    assert!(soul_storage::tracks::get_by_id(pool, track_id).await.unwrap().is_some());

    // Delete
    soul_storage::tracks::delete(pool, track_id)
        .await
        .expect("Failed to delete track");

    // Verify deleted
    assert!(soul_storage::tracks::get_by_id(pool, track_id).await.unwrap().is_none());

    // Verify track_sources also deleted (cascade)
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM track_sources WHERE track_id = ?")
        .bind(track_id)
        .fetch_one(pool)
        .await
        .unwrap();

    assert_eq!(count, 0);
}

#[tokio::test]
async fn test_record_play() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let user_id = create_test_user(pool, "testuser").await;
    let track_id = create_test_track(pool, "Test Song", None, None, 1, Some("/music/test.mp3")).await;

    // Record a completed play
    soul_storage::tracks::record_play(pool, user_id, track_id, Some(180.0), true)
        .await
        .expect("Failed to record play");

    // Check play count
    let play_count = soul_storage::tracks::get_play_count(pool, track_id)
        .await
        .unwrap();

    assert_eq!(play_count, 1);

    // Record a skip (not completed)
    soul_storage::tracks::record_play(pool, user_id, track_id, Some(30.0), false)
        .await
        .unwrap();

    // Verify stats
    let stats: (i32, i32) = sqlx::query_as(
        "SELECT play_count, skip_count FROM track_stats WHERE track_id = ?"
    )
    .bind(track_id)
    .fetch_one(pool)
    .await
    .unwrap();

    assert_eq!(stats.0, 1); // play_count
    assert_eq!(stats.1, 1); // skip_count
}

#[tokio::test]
async fn test_get_recently_played() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let user_id = create_test_user(pool, "testuser").await;

    let track1 = create_test_track(pool, "Track 1", None, None, 1, Some("/music/1.mp3")).await;
    let track2 = create_test_track(pool, "Track 2", None, None, 1, Some("/music/2.mp3")).await;
    let track3 = create_test_track(pool, "Track 3", None, None, 1, Some("/music/3.mp3")).await;

    // Play tracks in order
    soul_storage::tracks::record_play(pool, user_id, track1, None, true).await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    soul_storage::tracks::record_play(pool, user_id, track2, None, true).await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    soul_storage::tracks::record_play(pool, user_id, track3, None, true).await.unwrap();

    // Get recently played (limit 2)
    let recent = soul_storage::tracks::get_recently_played(pool, user_id, 2)
        .await
        .unwrap();

    assert_eq!(recent.len(), 2);

    // Should be in reverse chronological order
    assert_eq!(recent[0].id, track3);
    assert_eq!(recent[1].id, track2);
}

#[tokio::test]
async fn test_track_deletion_cascades_to_availability() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let track_id = create_test_track(pool, "Test", None, None, 1, Some("/music/test.mp3")).await;

    // Verify track_sources entry exists
    let count_before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM track_sources WHERE track_id = ?")
        .bind(track_id)
        .fetch_one(pool)
        .await
        .unwrap();

    assert!(count_before > 0);

    // Delete track
    soul_storage::tracks::delete(pool, track_id).await.unwrap();

    // Verify track_sources entry deleted
    let count_after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM track_sources WHERE track_id = ?")
        .bind(track_id)
        .fetch_one(pool)
        .await
        .unwrap();

    assert_eq!(count_after, 0);
}

#[tokio::test]
async fn test_create_track_initializes_stats() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let track_id = create_test_track(pool, "Test", None, None, 1, Some("/music/test.mp3")).await;

    // Verify stats row created
    let stats: (i32, i32) = sqlx::query_as(
        "SELECT play_count, skip_count FROM track_stats WHERE track_id = ?"
    )
    .bind(track_id)
    .fetch_one(pool)
    .await
    .expect("Stats should be initialized");

    assert_eq!(stats.0, 0);
    assert_eq!(stats.1, 0);
}
