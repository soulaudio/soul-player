//! Integration tests for playlists vertical slice
//!
//! Tests playlist operations including:
//! - CRUD with user ownership
//! - Permission system (read/write sharing)
//! - Track ordering and reordering
//! - Favorite playlists
//! - Transaction correctness for multi-step operations

mod test_helpers;

use soul_core::types::*;
use test_helpers::*;

#[tokio::test]
async fn test_create_and_get_playlist() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let user_id = create_test_user(pool, "testuser").await;

    let playlist = soul_storage::playlists::create(
        pool,
        CreatePlaylist {
            name: "My Favorites".to_string(),
            description: Some("Best songs ever".to_string()),
            owner_id: user_id.clone(),
            is_favorite: false,
        },
    )
    .await
    .expect("Failed to create playlist");

    assert_eq!(playlist.name, "My Favorites");
    assert_eq!(playlist.description, Some("Best songs ever".to_string()));
    assert_eq!(playlist.owner_id, user_id);
    assert!(!playlist.is_favorite);

    // Retrieve by ID
    let retrieved = soul_storage::playlists::get_by_id(pool, playlist.id.clone(), user_id)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(retrieved.id, playlist.id);
    assert_eq!(retrieved.name, "My Favorites");
}

#[tokio::test]
async fn test_get_user_playlists() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let user1 = create_test_user(pool, "user1").await;
    let user2 = create_test_user(pool, "user2").await;

    // User 1 creates playlists
    create_test_playlist(pool, "User 1 Playlist A", user1.clone()).await;
    create_test_playlist(pool, "User 1 Playlist B", user1.clone()).await;

    // User 2 creates playlist
    create_test_playlist(pool, "User 2 Playlist", user2).await;

    // Get user 1's playlists
    let user1_playlists = soul_storage::playlists::get_user_playlists(pool, user1.clone())
        .await
        .unwrap();

    assert_eq!(user1_playlists.len(), 2);

    for playlist in &user1_playlists {
        assert_eq!(playlist.owner_id, user1);
    }
}

#[tokio::test]
async fn test_add_tracks_to_playlist() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let user_id = create_test_user(pool, "testuser").await;
    let playlist_id = create_test_playlist(pool, "Test Playlist", user_id.clone()).await;

    let track1 = create_test_track(pool, "Track 1", None, None, 1, Some("/music/1.mp3")).await;
    let track2 = create_test_track(pool, "Track 2", None, None, 1, Some("/music/2.mp3")).await;

    // Add tracks
    soul_storage::playlists::add_track(pool, playlist_id.clone(), track1.clone(), user_id.clone())
        .await
        .expect("Failed to add track");

    soul_storage::playlists::add_track(pool, playlist_id.clone(), track2.clone(), user_id.clone())
        .await
        .expect("Failed to add track");

    // Get playlist with tracks
    let playlist = soul_storage::playlists::get_with_tracks(pool, playlist_id, user_id)
        .await
        .unwrap()
        .unwrap();

    let tracks = playlist.tracks.unwrap();
    assert_eq!(tracks.len(), 2);

    // Verify ordering
    assert_eq!(tracks[0].track_id, track1);
    assert_eq!(tracks[0].position, 0);
    assert_eq!(tracks[1].track_id, track2);
    assert_eq!(tracks[1].position, 1);
}

#[tokio::test]
async fn test_remove_track_from_playlist_reorders() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let user_id = create_test_user(pool, "testuser").await;
    let playlist_id = create_test_playlist(pool, "Test", user_id.clone()).await;

    let track1 = create_test_track(pool, "Track 1", None, None, 1, Some("/1.mp3")).await;
    let track2 = create_test_track(pool, "Track 2", None, None, 1, Some("/2.mp3")).await;
    let track3 = create_test_track(pool, "Track 3", None, None, 1, Some("/3.mp3")).await;

    // Add three tracks
    soul_storage::playlists::add_track(pool, playlist_id.clone(), track1.clone(), user_id.clone())
        .await
        .unwrap();
    soul_storage::playlists::add_track(pool, playlist_id.clone(), track2.clone(), user_id.clone())
        .await
        .unwrap();
    soul_storage::playlists::add_track(pool, playlist_id.clone(), track3.clone(), user_id.clone())
        .await
        .unwrap();

    // Remove middle track
    soul_storage::playlists::remove_track(pool, playlist_id.clone(), track2, user_id.clone())
        .await
        .expect("Failed to remove track");

    // Get playlist tracks
    let playlist = soul_storage::playlists::get_with_tracks(pool, playlist_id, user_id)
        .await
        .unwrap()
        .unwrap();

    let tracks = playlist.tracks.unwrap();
    assert_eq!(tracks.len(), 2);

    // Positions should be reordered (0, 1)
    assert_eq!(tracks[0].track_id, track1);
    assert_eq!(tracks[0].position, 0);
    assert_eq!(tracks[1].track_id, track3);
    assert_eq!(tracks[1].position, 1);
}

#[tokio::test]
async fn test_reorder_tracks_in_playlist() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let user_id = create_test_user(pool, "testuser").await;
    let playlist_id = create_test_playlist(pool, "Test", user_id.clone()).await;

    let track1 = create_test_track(pool, "Track 1", None, None, 1, Some("/1.mp3")).await;
    let track2 = create_test_track(pool, "Track 2", None, None, 1, Some("/2.mp3")).await;
    let track3 = create_test_track(pool, "Track 3", None, None, 1, Some("/3.mp3")).await;

    soul_storage::playlists::add_track(pool, playlist_id.clone(), track1.clone(), user_id.clone())
        .await
        .unwrap();
    soul_storage::playlists::add_track(pool, playlist_id.clone(), track2.clone(), user_id.clone())
        .await
        .unwrap();
    soul_storage::playlists::add_track(pool, playlist_id.clone(), track3.clone(), user_id.clone())
        .await
        .unwrap();

    // Move track 3 to position 0 (first)
    soul_storage::playlists::reorder_tracks(
        pool,
        playlist_id.clone(),
        track3.clone(),
        0,
        user_id.clone(),
    )
    .await
    .expect("Failed to reorder");

    // Get playlist tracks
    let playlist = soul_storage::playlists::get_with_tracks(pool, playlist_id, user_id)
        .await
        .unwrap()
        .unwrap();

    let tracks = playlist.tracks.unwrap();

    // New order should be: track3, track1, track2
    assert_eq!(tracks[0].track_id, track3);
    assert_eq!(tracks[1].track_id, track1);
    assert_eq!(tracks[2].track_id, track2);
}

#[tokio::test]
async fn test_delete_playlist() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let user_id = create_test_user(pool, "testuser").await;
    let playlist_id = create_test_playlist(pool, "To Delete", user_id.clone()).await;

    // Add a track
    let track_id = create_test_track(pool, "Track", None, None, 1, Some("/music/track.mp3")).await;
    soul_storage::playlists::add_track(
        pool,
        playlist_id.clone(),
        track_id.clone(),
        user_id.clone(),
    )
    .await
    .unwrap();

    // Delete playlist
    soul_storage::playlists::delete(pool, playlist_id.clone(), user_id.clone())
        .await
        .expect("Failed to delete playlist");

    // Playlist should be gone
    let result = soul_storage::playlists::get_by_id(pool, playlist_id.clone(), user_id)
        .await
        .unwrap();
    assert!(result.is_none());

    // Verify playlist_tracks entries deleted (cascade)
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM playlist_tracks WHERE playlist_id = ?")
            .bind(playlist_id)
            .fetch_one(pool)
            .await
            .unwrap();

    assert_eq!(count, 0);

    // Track should still exist
    assert!(soul_storage::tracks::get_by_id(pool, track_id)
        .await
        .unwrap()
        .is_some());
}

#[tokio::test]
async fn test_share_playlist_with_read_permission() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let owner = create_test_user(pool, "owner").await;
    let shared_user = create_test_user(pool, "shared").await;

    let playlist_id = create_test_playlist(pool, "Shared Playlist", owner.clone()).await;

    // Share with read permission
    soul_storage::playlists::share_playlist(
        pool,
        playlist_id.clone(),
        shared_user.clone(),
        "read",
        owner.clone(),
    )
    .await
    .expect("Failed to share playlist");

    // Shared user should see it in their playlists
    let playlists = soul_storage::playlists::get_user_playlists(pool, shared_user.clone())
        .await
        .unwrap();

    assert_eq!(playlists.len(), 1);
    assert_eq!(playlists[0].id, playlist_id);
    assert_eq!(playlists[0].owner_id, owner); // Still owned by original owner

    // Shared user can view
    let retrieved = soul_storage::playlists::get_by_id(pool, playlist_id, shared_user)
        .await
        .unwrap();

    assert!(retrieved.is_some());
}

#[tokio::test]
async fn test_shared_user_with_read_permission_cannot_modify() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let owner = create_test_user(pool, "owner").await;
    let shared_user = create_test_user(pool, "shared").await;

    let playlist_id = create_test_playlist(pool, "Shared Playlist", owner.clone()).await;
    let track_id = create_test_track(pool, "Track", None, None, 1, Some("/music/track.mp3")).await;

    // Share with read-only permission
    soul_storage::playlists::share_playlist(
        pool,
        playlist_id.clone(),
        shared_user.clone(),
        "read",
        owner,
    )
    .await
    .unwrap();

    // Shared user tries to add track (should fail)
    let result = soul_storage::playlists::add_track(pool, playlist_id, track_id, shared_user).await;

    assert!(
        result.is_err(),
        "Read-only user should not be able to add tracks"
    );
}

#[tokio::test]
async fn test_shared_user_with_write_permission_can_modify() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let owner = create_test_user(pool, "owner").await;
    let shared_user = create_test_user(pool, "shared").await;

    let playlist_id = create_test_playlist(pool, "Shared Playlist", owner.clone()).await;
    let track_id = create_test_track(pool, "Track", None, None, 1, Some("/music/track.mp3")).await;

    // Share with write permission
    soul_storage::playlists::share_playlist(
        pool,
        playlist_id.clone(),
        shared_user.clone(),
        "write",
        owner.clone(),
    )
    .await
    .unwrap();

    // Shared user can add track
    soul_storage::playlists::add_track(pool, playlist_id.clone(), track_id, shared_user)
        .await
        .expect("Write user should be able to add tracks");

    // Verify track was added
    let playlist = soul_storage::playlists::get_with_tracks(pool, playlist_id, owner)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(playlist.tracks.unwrap().len(), 1);
}

#[tokio::test]
async fn test_only_owner_can_delete_playlist() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let owner = create_test_user(pool, "owner").await;
    let shared_user = create_test_user(pool, "shared").await;

    let playlist_id = create_test_playlist(pool, "Shared Playlist", owner.clone()).await;

    // Share with write permission
    soul_storage::playlists::share_playlist(
        pool,
        playlist_id.clone(),
        shared_user.clone(),
        "write",
        owner.clone(),
    )
    .await
    .unwrap();

    // Shared user tries to delete (should fail even with write permission)
    let result = soul_storage::playlists::delete(pool, playlist_id.clone(), shared_user).await;

    assert!(
        result.is_err(),
        "Only owner should be able to delete playlist"
    );

    // Owner can delete
    soul_storage::playlists::delete(pool, playlist_id, owner)
        .await
        .expect("Owner should be able to delete");
}

#[tokio::test]
async fn test_unshare_playlist() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let owner = create_test_user(pool, "owner").await;
    let shared_user = create_test_user(pool, "shared").await;

    let playlist_id = create_test_playlist(pool, "Shared Playlist", owner.clone()).await;

    // Share
    soul_storage::playlists::share_playlist(
        pool,
        playlist_id.clone(),
        shared_user.clone(),
        "read",
        owner.clone(),
    )
    .await
    .unwrap();

    // Verify shared user can see it
    let playlists_before = soul_storage::playlists::get_user_playlists(pool, shared_user.clone())
        .await
        .unwrap();

    assert_eq!(playlists_before.len(), 1);

    // Unshare
    soul_storage::playlists::unshare_playlist(pool, playlist_id, shared_user.clone(), owner)
        .await
        .expect("Failed to unshare");

    // Shared user should no longer see it
    let playlists_after = soul_storage::playlists::get_user_playlists(pool, shared_user)
        .await
        .unwrap();

    assert_eq!(playlists_after.len(), 0);
}

#[tokio::test]
async fn test_favorite_playlists_sorted_first() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let user_id = create_test_user(pool, "testuser").await;

    // Create regular playlist
    soul_storage::playlists::create(
        pool,
        CreatePlaylist {
            name: "Regular Playlist".to_string(),
            description: None,
            owner_id: user_id.clone(),
            is_favorite: false,
        },
    )
    .await
    .unwrap();

    // Create favorite playlist
    soul_storage::playlists::create(
        pool,
        CreatePlaylist {
            name: "Favorite Playlist".to_string(),
            description: None,
            owner_id: user_id.clone(),
            is_favorite: true,
        },
    )
    .await
    .unwrap();

    // Get user playlists (favorites should come first)
    let playlists = soul_storage::playlists::get_user_playlists(pool, user_id)
        .await
        .unwrap();

    assert_eq!(playlists.len(), 2);
    assert!(playlists[0].is_favorite);
    assert_eq!(playlists[0].name, "Favorite Playlist");
}

#[tokio::test]
async fn test_playlist_updated_at_changes_on_modifications() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let user_id = create_test_user(pool, "testuser").await;
    let playlist_id = create_test_playlist(pool, "Test", user_id.clone()).await;

    let playlist_before =
        soul_storage::playlists::get_by_id(pool, playlist_id.clone(), user_id.clone())
            .await
            .unwrap()
            .unwrap();

    let updated_at_before = playlist_before.updated_at;

    // Wait at least 1 second for Unix timestamp to change
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Add a track
    let track_id = create_test_track(pool, "Track", None, None, 1, Some("/music/track.mp3")).await;
    soul_storage::playlists::add_track(pool, playlist_id.clone(), track_id, user_id.clone())
        .await
        .unwrap();

    // Check updated_at changed
    let playlist_after = soul_storage::playlists::get_by_id(pool, playlist_id, user_id)
        .await
        .unwrap()
        .unwrap();

    assert!(playlist_after.updated_at > updated_at_before);
}

#[tokio::test]
async fn test_cannot_add_duplicate_track_to_playlist() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let user_id = create_test_user(pool, "testuser").await;
    let playlist_id = create_test_playlist(pool, "Test", user_id.clone()).await;
    let track_id = create_test_track(pool, "Track", None, None, 1, Some("/music/track.mp3")).await;

    // Add track once
    soul_storage::playlists::add_track(
        pool,
        playlist_id.clone(),
        track_id.clone(),
        user_id.clone(),
    )
    .await
    .unwrap();

    // Try to add same track again - ON CONFLICT DO NOTHING should silently ignore
    soul_storage::playlists::add_track(pool, playlist_id.clone(), track_id, user_id.clone())
        .await
        .expect("Should succeed but do nothing");

    // Verify only one entry
    let playlist = soul_storage::playlists::get_with_tracks(pool, playlist_id, user_id)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(playlist.tracks.unwrap().len(), 1);
}
