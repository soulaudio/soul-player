//! Comprehensive integration tests for soul-storage
//!
//! Tests the complete database layer using the modern vertical slice architecture.
//! These tests verify:
//! - Users, artists, albums, tracks, playlists
//! - Multi-source track availability
//! - Playlist sharing and permissions
//! - Play history and statistics
//! - Data integrity and cascading deletes

mod test_helpers;

use soul_core::types::*;
use test_helpers::*;

// ============================================================================
// USER TESTS
// ============================================================================

#[tokio::test]
async fn test_user_lifecycle() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    // Create users
    let user1 = create_test_user(pool, "Alice").await;
    let user2 = create_test_user(pool, "Bob").await;
    let user3 = create_test_user(pool, "Charlie").await;

    // Verify each user has unique ID
    assert_ne!(user1, user2);
    assert_ne!(user2, user3);

    // Verify user IDs are valid UUIDs
    assert!(user1.as_str().contains('-'));
}

// ============================================================================
// ARTIST & ALBUM TESTS
// ============================================================================

#[tokio::test]
async fn test_artist_and_album_creation() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    // Create artist
    let artist = create_test_artist(pool, "Queen", Some("Queen")).await;

    // Create albums for this artist
    let album1 = create_test_album(pool, "A Night at the Opera", Some(artist), Some(1975)).await;
    let album2 = create_test_album(pool, "News of the World", Some(artist), Some(1977)).await;

    assert_ne!(album1, album2);
}

// ============================================================================
// TRACK TESTS
// ============================================================================

#[tokio::test]
async fn test_track_full_lifecycle() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    // Create supporting data
    let artist = create_test_artist(pool, "Queen", None).await;
    let album = create_test_album(pool, "A Night at the Opera", Some(artist), Some(1975)).await;

    // Create track with full metadata
    let track = soul_storage::tracks::create(
        pool,
        CreateTrack {
            title: "Bohemian Rhapsody".to_string(),
            artist_id: Some(artist),
            album_id: Some(album),
            album_artist_id: Some(artist),
            track_number: Some(1),
            disc_number: Some(1),
            year: Some(1975),
            duration_seconds: Some(354.0), // 5:54
            bitrate: Some(320),
            sample_rate: Some(44100),
            channels: Some(2),
            file_format: "mp3".to_string(),
            file_hash: Some("abc123".to_string()),
            origin_source_id: 1,
            local_file_path: Some("/music/queen/bohemian.mp3".to_string()),
            musicbrainz_recording_id: None,
            fingerprint: None,
        },
    )
    .await
    .expect("Failed to create track");

    // Verify denormalized data
    assert_eq!(track.title, "Bohemian Rhapsody");
    assert_eq!(track.artist_name, Some("Queen".to_string()));
    assert_eq!(track.album_title, Some("A Night at the Opera".to_string()));
    assert_eq!(track.duration_seconds, Some(354.0));

    // Retrieve by ID
    let retrieved = soul_storage::tracks::get_by_id(pool, track.id.clone())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(retrieved.title, "Bohemian Rhapsody");

    // Update track
    let updated = soul_storage::tracks::update(
        pool,
        track.id.clone(),
        UpdateTrack {
            duration_seconds: Some(355.0), // Corrected duration
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(updated.duration_seconds, Some(355.0));

    // Delete track
    soul_storage::tracks::delete(pool, track.id)
        .await
        .expect("Failed to delete track");
}

#[tokio::test]
async fn test_track_search() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let artist1 = create_test_artist(pool, "Queen", None).await;
    let artist2 = create_test_artist(pool, "Led Zeppelin", None).await;

    // Create tracks
    create_test_track(
        pool,
        "Bohemian Rhapsody",
        Some(artist1),
        None,
        1,
        Some("/m/1.mp3"),
    )
    .await;
    create_test_track(
        pool,
        "Another One Bites the Dust",
        Some(artist1),
        None,
        1,
        Some("/m/2.mp3"),
    )
    .await;
    create_test_track(
        pool,
        "Stairway to Heaven",
        Some(artist2),
        None,
        1,
        Some("/m/3.mp3"),
    )
    .await;

    // Search by artist name
    let queen_tracks = soul_storage::tracks::search(pool, "Queen")
        .await
        .expect("Search failed");
    assert_eq!(queen_tracks.len(), 2);

    // Search by title
    let stairway = soul_storage::tracks::search(pool, "Stairway")
        .await
        .expect("Search failed");
    assert_eq!(stairway.len(), 1);
    assert_eq!(stairway[0].title, "Stairway to Heaven");

    // No results
    let empty = soul_storage::tracks::search(pool, "Nonexistent")
        .await
        .expect("Search failed");
    assert_eq!(empty.len(), 0);
}

#[tokio::test]
async fn test_track_by_artist_and_album() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let artist = create_test_artist(pool, "Queen", None).await;
    let album = create_test_album(pool, "Greatest Hits", Some(artist), Some(1981)).await;

    // Create tracks in album
    create_test_track(
        pool,
        "We Will Rock You",
        Some(artist),
        Some(album),
        1,
        Some("/m/1.mp3"),
    )
    .await;
    create_test_track(
        pool,
        "We Are the Champions",
        Some(artist),
        Some(album),
        1,
        Some("/m/2.mp3"),
    )
    .await;

    // Get tracks by artist
    let artist_tracks = soul_storage::tracks::get_by_artist(pool, artist)
        .await
        .unwrap();
    assert!(artist_tracks.len() >= 2);

    // Get tracks by album
    let album_tracks = soul_storage::tracks::get_by_album(pool, album)
        .await
        .unwrap();
    assert_eq!(album_tracks.len(), 2);
}

#[tokio::test]
async fn test_multi_source_tracks() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    // Create multiple sources
    let server1 = create_test_source(pool, "Server 1", "server").await;
    let server2 = create_test_source(pool, "Server 2", "server").await;

    // Create track from server1
    let track_id = create_test_track(pool, "Shared Song", None, None, server1, None).await;

    // Add availability from server2 (same track available on multiple servers)
    sqlx::query!(
        "INSERT INTO track_sources (track_id, source_id, status, server_path)
         VALUES (?, ?, 'stream_only', '/api/stream')",
        track_id,
        server2
    )
    .execute(pool)
    .await
    .unwrap();

    // Get availability
    let availability = soul_storage::tracks::get_availability(pool, track_id)
        .await
        .unwrap();
    assert_eq!(availability.len(), 2);

    // Get tracks by source
    let server1_tracks = soul_storage::tracks::get_by_source(pool, server1)
        .await
        .unwrap();
    assert_eq!(server1_tracks.len(), 1);
}

// ============================================================================
// PLAYLIST TESTS
// ============================================================================

#[tokio::test]
async fn test_playlist_full_workflow() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let user = create_test_user(pool, "Alice").await;

    // Create playlist
    let playlist = soul_storage::playlists::create(
        pool,
        CreatePlaylist {
            name: "My Favorites".to_string(),
            description: Some("Best songs ever".to_string()),
            owner_id: user.clone(),
            is_favorite: false,
        },
    )
    .await
    .unwrap();

    assert_eq!(playlist.name, "My Favorites");
    assert_eq!(playlist.owner_id, user);

    // Create tracks
    let track1 = create_test_track(pool, "Song 1", None, None, 1, Some("/m/1.mp3")).await;
    let track2 = create_test_track(pool, "Song 2", None, None, 1, Some("/m/2.mp3")).await;
    let track3 = create_test_track(pool, "Song 3", None, None, 1, Some("/m/3.mp3")).await;

    // Add tracks to playlist
    soul_storage::playlists::add_track(pool, playlist.id.clone(), track1.clone(), user.clone())
        .await
        .unwrap();
    soul_storage::playlists::add_track(pool, playlist.id.clone(), track2.clone(), user.clone())
        .await
        .unwrap();
    soul_storage::playlists::add_track(pool, playlist.id.clone(), track3.clone(), user.clone())
        .await
        .unwrap();

    // Get playlist with tracks
    let with_tracks =
        soul_storage::playlists::get_with_tracks(pool, playlist.id.clone(), user.clone())
            .await
            .unwrap()
            .unwrap();
    let tracks = with_tracks.tracks.unwrap();
    assert_eq!(tracks.len(), 3);
    assert_eq!(tracks[0].track_id, track1);
    assert_eq!(tracks[1].track_id, track2);
    assert_eq!(tracks[2].track_id, track3);

    // Remove middle track
    soul_storage::playlists::remove_track(pool, playlist.id.clone(), track2, user.clone())
        .await
        .unwrap();

    let after_remove =
        soul_storage::playlists::get_with_tracks(pool, playlist.id.clone(), user.clone())
            .await
            .unwrap()
            .unwrap();
    assert_eq!(after_remove.tracks.unwrap().len(), 2);

    // Delete playlist
    soul_storage::playlists::delete(pool, playlist.id, user)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_playlist_user_isolation() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let alice = create_test_user(pool, "Alice").await;
    let bob = create_test_user(pool, "Bob").await;

    // Alice creates playlists
    create_test_playlist(pool, "Alice's Rock", alice.clone()).await;
    create_test_playlist(pool, "Alice's Jazz", alice.clone()).await;

    // Bob creates playlists
    create_test_playlist(pool, "Bob's Classics", bob.clone()).await;

    // Get Alice's playlists
    let alice_playlists = soul_storage::playlists::get_user_playlists(pool, alice)
        .await
        .unwrap();
    assert_eq!(alice_playlists.len(), 2);
    assert!(alice_playlists
        .iter()
        .all(|p| p.name.starts_with("Alice's")));

    // Get Bob's playlists
    let bob_playlists = soul_storage::playlists::get_user_playlists(pool, bob)
        .await
        .unwrap();
    assert_eq!(bob_playlists.len(), 1);
    assert_eq!(bob_playlists[0].name, "Bob's Classics");
}

#[tokio::test]
async fn test_playlist_sharing() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let alice = create_test_user(pool, "Alice").await;
    let bob = create_test_user(pool, "Bob").await;

    // Alice creates a playlist
    let playlist = create_test_playlist(pool, "Shared Playlist", alice.clone()).await;

    // Alice shares with Bob (read permission)
    soul_storage::playlists::share_playlist(
        pool,
        playlist.clone(),
        bob.clone(),
        "read",
        alice.clone(),
    )
    .await
    .unwrap();

    // Bob can now see it
    let bob_playlists = soul_storage::playlists::get_user_playlists(pool, bob.clone())
        .await
        .unwrap();
    assert_eq!(bob_playlists.len(), 1);
    assert_eq!(bob_playlists[0].id, playlist);

    // Unshare
    soul_storage::playlists::unshare_playlist(pool, playlist.clone(), bob.clone(), alice)
        .await
        .unwrap();

    // Bob no longer sees it
    let bob_playlists_after = soul_storage::playlists::get_user_playlists(pool, bob)
        .await
        .unwrap();
    assert_eq!(bob_playlists_after.len(), 0);
}

#[tokio::test]
async fn test_playlist_write_permissions() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let alice = create_test_user(pool, "Alice").await;
    let bob = create_test_user(pool, "Bob").await;

    let playlist = create_test_playlist(pool, "Collaborative", alice.clone()).await;
    let track = create_test_track(pool, "Song", None, None, 1, Some("/m/song.mp3")).await;

    // Share with write permission
    soul_storage::playlists::share_playlist(
        pool,
        playlist.clone(),
        bob.clone(),
        "write",
        alice.clone(),
    )
    .await
    .unwrap();

    // Bob can add tracks
    soul_storage::playlists::add_track(pool, playlist.clone(), track.clone(), bob.clone())
        .await
        .expect("Bob should be able to add tracks with write permission");

    // Verify track was added
    let with_tracks = soul_storage::playlists::get_with_tracks(pool, playlist, alice)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(with_tracks.tracks.unwrap().len(), 1);
}

#[tokio::test]
async fn test_playlist_reordering() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let user = create_test_user(pool, "Alice").await;
    let playlist = create_test_playlist(pool, "To Reorder", user.clone()).await;

    let track1 = create_test_track(pool, "First", None, None, 1, Some("/m/1.mp3")).await;
    let track2 = create_test_track(pool, "Second", None, None, 1, Some("/m/2.mp3")).await;
    let track3 = create_test_track(pool, "Third", None, None, 1, Some("/m/3.mp3")).await;

    // Add tracks
    soul_storage::playlists::add_track(pool, playlist.clone(), track1.clone(), user.clone())
        .await
        .unwrap();
    soul_storage::playlists::add_track(pool, playlist.clone(), track2.clone(), user.clone())
        .await
        .unwrap();
    soul_storage::playlists::add_track(pool, playlist.clone(), track3.clone(), user.clone())
        .await
        .unwrap();

    // Move track1 from position 0 to position 2 (becomes last)
    soul_storage::playlists::reorder_tracks(
        pool,
        playlist.clone(),
        track1.clone(),
        2,
        user.clone(),
    )
    .await
    .unwrap();

    // Verify new order: [track2, track3, track1]
    let reordered = soul_storage::playlists::get_with_tracks(pool, playlist, user)
        .await
        .unwrap()
        .unwrap();
    let tracks = reordered.tracks.unwrap();
    assert_eq!(tracks[0].track_id, track2); // moved up from pos 1 to pos 0
    assert_eq!(tracks[1].track_id, track3); // moved up from pos 2 to pos 1
    assert_eq!(tracks[2].track_id, track1); // moved from pos 0 to pos 2
}

#[tokio::test]
async fn test_duplicate_track_prevention() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let user = create_test_user(pool, "Alice").await;
    let playlist = create_test_playlist(pool, "No Dupes", user.clone()).await;
    let track = create_test_track(pool, "Song", None, None, 1, Some("/m/song.mp3")).await;

    // Add track once
    soul_storage::playlists::add_track(pool, playlist.clone(), track.clone(), user.clone())
        .await
        .unwrap();

    // Try adding again - should be silently ignored (ON CONFLICT DO NOTHING)
    soul_storage::playlists::add_track(pool, playlist.clone(), track, user.clone())
        .await
        .unwrap();

    // Should still only have 1 track
    let with_tracks = soul_storage::playlists::get_with_tracks(pool, playlist, user)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(with_tracks.tracks.unwrap().len(), 1);
}

// ============================================================================
// PLAY HISTORY & STATISTICS TESTS
// ============================================================================

#[tokio::test]
async fn test_play_tracking() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let user = create_test_user(pool, "Alice").await;
    let track = create_test_track(pool, "Popular Song", None, None, 1, Some("/m/song.mp3")).await;

    // Record a completed play
    soul_storage::tracks::record_play(pool, user.clone(), track.clone(), Some(180.0), true)
        .await
        .unwrap();

    // Check play count
    let play_count = soul_storage::tracks::get_play_count(pool, track.clone())
        .await
        .unwrap();
    assert_eq!(play_count, 1);

    // Record another play
    soul_storage::tracks::record_play(pool, user.clone(), track.clone(), Some(180.0), true)
        .await
        .unwrap();

    let play_count_2 = soul_storage::tracks::get_play_count(pool, track)
        .await
        .unwrap();
    assert_eq!(play_count_2, 2);
}

#[tokio::test]
async fn test_skip_tracking() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let user = create_test_user(pool, "Alice").await;
    let track = create_test_track(pool, "Skipped Song", None, None, 1, Some("/m/song.mp3")).await;

    // Record a skip (not completed)
    soul_storage::tracks::record_play(pool, user.clone(), track.clone(), Some(10.0), false)
        .await
        .unwrap();

    // Play count should still be 0
    let play_count = soul_storage::tracks::get_play_count(pool, track.clone())
        .await
        .unwrap();
    assert_eq!(play_count, 0);

    // Verify skip was recorded in track_stats
    let stats: (i32, i32) = sqlx::query_as(
        "SELECT play_count, skip_count FROM track_stats WHERE track_id = ? AND user_id = ?",
    )
    .bind(track)
    .bind(user.as_str())
    .fetch_one(pool)
    .await
    .unwrap();

    assert_eq!(stats.0, 0); // play_count
    assert_eq!(stats.1, 1); // skip_count
}

#[tokio::test]
async fn test_recently_played() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let user = create_test_user(pool, "Alice").await;

    let track1 = create_test_track(pool, "First", None, None, 1, Some("/m/1.mp3")).await;
    let track2 = create_test_track(pool, "Second", None, None, 1, Some("/m/2.mp3")).await;
    let track3 = create_test_track(pool, "Third", None, None, 1, Some("/m/3.mp3")).await;

    // Play in order with delays
    soul_storage::tracks::record_play(pool, user.clone(), track1, None, true)
        .await
        .unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    soul_storage::tracks::record_play(pool, user.clone(), track2.clone(), None, true)
        .await
        .unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    soul_storage::tracks::record_play(pool, user.clone(), track3.clone(), None, true)
        .await
        .unwrap();

    // Get recently played (limit 2)
    let recent = soul_storage::tracks::get_recently_played(pool, user, 2)
        .await
        .unwrap();

    assert_eq!(recent.len(), 2);
    // Should be in reverse chronological order
    assert_eq!(recent[0].id, track3);
    assert_eq!(recent[1].id, track2);
}

#[tokio::test]
async fn test_per_user_statistics() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let alice = create_test_user(pool, "Alice").await;
    let bob = create_test_user(pool, "Bob").await;
    let track = create_test_track(pool, "Shared Song", None, None, 1, Some("/m/song.mp3")).await;

    // Alice plays it twice
    soul_storage::tracks::record_play(pool, alice.clone(), track.clone(), None, true)
        .await
        .unwrap();
    soul_storage::tracks::record_play(pool, alice.clone(), track.clone(), None, true)
        .await
        .unwrap();

    // Bob plays it once
    soul_storage::tracks::record_play(pool, bob.clone(), track.clone(), None, true)
        .await
        .unwrap();

    // Get Alice's recently played
    let alice_recent = soul_storage::tracks::get_recently_played(pool, alice, 10)
        .await
        .unwrap();
    assert_eq!(alice_recent.len(), 1);

    // Get Bob's recently played
    let bob_recent = soul_storage::tracks::get_recently_played(pool, bob, 10)
        .await
        .unwrap();
    assert_eq!(bob_recent.len(), 1);

    // Note: get_play_count returns play count from ONE user's stats (whichever row it finds first)
    // since track_stats is per-user. For a total, we'd need to sum across users manually.
    let play_count = soul_storage::tracks::get_play_count(pool, track)
        .await
        .unwrap();
    assert!(play_count >= 1 && play_count <= 2); // Either Alice's (2) or Bob's (1) count
}

// ============================================================================
// CASCADE & REFERENTIAL INTEGRITY TESTS
// ============================================================================

#[tokio::test]
async fn test_delete_track_cascades() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let user = create_test_user(pool, "Alice").await;
    let track = create_test_track(pool, "To Delete", None, None, 1, Some("/m/delete.mp3")).await;

    // Add to playlist
    let playlist = create_test_playlist(pool, "Test", user.clone()).await;
    soul_storage::playlists::add_track(pool, playlist.clone(), track.clone(), user.clone())
        .await
        .unwrap();

    // Record play
    soul_storage::tracks::record_play(pool, user.clone(), track.clone(), None, true)
        .await
        .unwrap();

    // Delete track
    soul_storage::tracks::delete(pool, track.clone())
        .await
        .unwrap();

    // Verify track_sources deleted
    let sources_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM track_sources WHERE track_id = ?")
            .bind(track.clone())
            .fetch_one(pool)
            .await
            .unwrap();
    assert_eq!(sources_count, 0);

    // Verify playlist_tracks entry deleted
    let playlist_tracks_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM playlist_tracks WHERE track_id = ?")
            .bind(track.clone())
            .fetch_one(pool)
            .await
            .unwrap();
    assert_eq!(playlist_tracks_count, 0);

    // Verify play history also deleted (CASCADE on track_id)
    let history_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM play_history WHERE track_id = ?")
            .bind(track)
            .fetch_one(pool)
            .await
            .unwrap();
    assert_eq!(history_count, 0); // Play history cascades with track deletion
}

#[tokio::test]
async fn test_delete_playlist_cascades() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let user = create_test_user(pool, "Alice").await;
    let playlist = create_test_playlist(pool, "To Delete", user.clone()).await;
    let track = create_test_track(pool, "Song", None, None, 1, Some("/m/song.mp3")).await;

    // Add track to playlist
    soul_storage::playlists::add_track(pool, playlist.clone(), track.clone(), user.clone())
        .await
        .unwrap();

    // Delete playlist
    soul_storage::playlists::delete(pool, playlist.clone(), user)
        .await
        .unwrap();

    // Verify playlist_tracks deleted
    let tracks_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM playlist_tracks WHERE playlist_id = ?")
            .bind(playlist.as_str())
            .fetch_one(pool)
            .await
            .unwrap();
    assert_eq!(tracks_count, 0);

    // Verify track still exists (not cascade deleted)
    let track_exists = soul_storage::tracks::get_by_id(pool, track).await.unwrap();
    assert!(track_exists.is_some());
}

#[tokio::test]
async fn test_delete_user_cleans_associations() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let user = create_test_user(pool, "ToDelete").await;
    let playlist = create_test_playlist(pool, "User's Playlist", user.clone()).await;

    // Delete user (cascade should remove playlists)
    let user_id = user.as_str();
    sqlx::query!("DELETE FROM users WHERE id = ?", user_id)
        .execute(pool)
        .await
        .unwrap();

    // Verify playlist deleted
    let playlist_exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM playlists WHERE id = ?")
        .bind(playlist.as_str())
        .fetch_one(pool)
        .await
        .unwrap();
    assert_eq!(playlist_exists, 0);
}

// ============================================================================
// COMPREHENSIVE WORKFLOW TEST
// ============================================================================

#[tokio::test]
async fn test_complete_music_library_workflow() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    // 1. Create users
    let alice = create_test_user(pool, "Alice").await;
    let bob = create_test_user(pool, "Bob").await;

    // 2. Build music library
    let queen = create_test_artist(pool, "Queen", None).await;
    let album = create_test_album(pool, "Greatest Hits", Some(queen), Some(1981)).await;

    let track1 = soul_storage::tracks::create(
        pool,
        CreateTrack {
            title: "We Will Rock You".to_string(),
            artist_id: Some(queen),
            album_id: Some(album),
            album_artist_id: Some(queen),
            track_number: Some(1),
            disc_number: Some(1),
            year: Some(1981),
            duration_seconds: Some(122.0),
            bitrate: Some(320),
            sample_rate: Some(44100),
            channels: Some(2),
            file_format: "mp3".to_string(),
            file_hash: None,
            origin_source_id: 1,
            local_file_path: Some("/music/queen/rock.mp3".to_string()),
            musicbrainz_recording_id: None,
            fingerprint: None,
        },
    )
    .await
    .unwrap();

    let track2 = soul_storage::tracks::create(
        pool,
        CreateTrack {
            title: "We Are the Champions".to_string(),
            artist_id: Some(queen),
            album_id: Some(album),
            album_artist_id: Some(queen),
            track_number: Some(2),
            disc_number: Some(1),
            year: Some(1981),
            duration_seconds: Some(179.0),
            bitrate: Some(320),
            sample_rate: Some(44100),
            channels: Some(2),
            file_format: "mp3".to_string(),
            file_hash: None,
            origin_source_id: 1,
            local_file_path: Some("/music/queen/champions.mp3".to_string()),
            musicbrainz_recording_id: None,
            fingerprint: None,
        },
    )
    .await
    .unwrap();

    // 3. Alice creates playlists
    let workout = create_test_playlist(pool, "Workout Mix", alice.clone()).await;
    soul_storage::playlists::add_track(pool, workout.clone(), track1.id.clone(), alice.clone())
        .await
        .unwrap();

    let chill = create_test_playlist(pool, "Chill Vibes", alice.clone()).await;
    soul_storage::playlists::add_track(pool, chill.clone(), track2.id.clone(), alice.clone())
        .await
        .unwrap();

    // 4. Alice shares workout playlist with Bob
    soul_storage::playlists::share_playlist(
        pool,
        workout.clone(),
        bob.clone(),
        "write",
        alice.clone(),
    )
    .await
    .unwrap();

    // 5. Bob adds a track to shared playlist
    soul_storage::playlists::add_track(pool, workout.clone(), track2.id.clone(), bob.clone())
        .await
        .unwrap();

    // 6. Alice plays tracks
    soul_storage::tracks::record_play(pool, alice.clone(), track1.id.clone(), Some(122.0), true)
        .await
        .unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    soul_storage::tracks::record_play(pool, alice.clone(), track2.id.clone(), Some(179.0), true)
        .await
        .unwrap();

    // 7. Verify complete state
    // Alice has 2 playlists
    let alice_playlists = soul_storage::playlists::get_user_playlists(pool, alice.clone())
        .await
        .unwrap();
    assert_eq!(alice_playlists.len(), 2);

    // Bob has access to 1 shared playlist
    let bob_playlists = soul_storage::playlists::get_user_playlists(pool, bob)
        .await
        .unwrap();
    assert_eq!(bob_playlists.len(), 1);

    // Workout playlist has 2 tracks (1 from Alice, 1 from Bob)
    let workout_tracks = soul_storage::playlists::get_with_tracks(pool, workout, alice.clone())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(workout_tracks.tracks.unwrap().len(), 2);

    // Alice's recently played shows both tracks
    let alice_recent = soul_storage::tracks::get_recently_played(pool, alice, 10)
        .await
        .unwrap();
    assert_eq!(alice_recent.len(), 2);

    // Search works
    let queen_tracks = soul_storage::tracks::search(pool, "Queen").await.unwrap();
    assert_eq!(queen_tracks.len(), 2);
}
