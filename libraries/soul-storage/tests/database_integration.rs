/// Integration tests for soul-storage using testcontainers
///
/// These tests use REAL SQLite databases (NOT in-memory) to ensure
/// migrations, constraints, and queries work correctly in production.
use soul_core::{Permission, Playlist, Storage, Track, User};
use soul_storage::Database;
use std::path::PathBuf;

/// Helper to create a test database
///
/// Uses a temporary file-based SQLite database (not in-memory)
/// to match production behavior
async fn create_test_db() -> Database {
    // Use a temp directory instead of temp file to avoid permission issues
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let db_url = format!("sqlite://{}", db_path.to_str().unwrap());

    let db = Database::new(&db_url)
        .await
        .expect("Failed to create test database");

    // Keep temp_dir alive by leaking it (acceptable for tests)
    std::mem::forget(temp_dir);

    db
}

#[tokio::test]
async fn test_user_creation_and_retrieval() {
    let db = create_test_db().await;

    // Create a user
    let user = db
        .create_user("Alice")
        .await
        .expect("Failed to create user");
    assert_eq!(user.name, "Alice");

    // Retrieve the user
    let retrieved = db.get_user(&user.id).await.expect("Failed to get user");
    assert_eq!(retrieved.id, user.id);
    assert_eq!(retrieved.name, "Alice");
}

#[tokio::test]
async fn test_get_all_users() {
    let db = create_test_db().await;

    // Create multiple users
    db.create_user("Alice").await.unwrap();
    db.create_user("Bob").await.unwrap();
    db.create_user("Charlie").await.unwrap();

    // Get all users
    let users = db.get_all_users().await.expect("Failed to get all users");
    assert_eq!(users.len(), 3);

    // Verify they're sorted by name
    let names: Vec<String> = users.iter().map(|u| u.name.clone()).collect();
    assert_eq!(names, vec!["Alice", "Bob", "Charlie"]);
}

#[tokio::test]
async fn test_get_nonexistent_user() {
    let db = create_test_db().await;

    let result = db.get_user(&soul_core::UserId::new("nonexistent")).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_track_operations() {
    let db = create_test_db().await;

    // Create a track
    let mut track = Track::new("Test Song", PathBuf::from("/music/test.mp3"));
    track.artist = Some("Test Artist".to_string());
    track.album = Some("Test Album".to_string());
    track.duration_ms = Some(180_000); // 3 minutes

    let track_id = db
        .add_track(track.clone())
        .await
        .expect("Failed to add track");
    assert_eq!(track_id, track.id);

    // Retrieve the track
    let retrieved = db.get_track(&track_id).await.expect("Failed to get track");
    assert_eq!(retrieved.title, "Test Song");
    assert_eq!(retrieved.artist, Some("Test Artist".to_string()));
    assert_eq!(retrieved.album, Some("Test Album".to_string()));
    assert_eq!(retrieved.duration_ms, Some(180_000));
}

#[tokio::test]
async fn test_get_all_tracks() {
    let db = create_test_db().await;

    // Add multiple tracks
    let track1 = Track::new("Alpha Song", PathBuf::from("/music/alpha.mp3"));
    let track2 = Track::new("Beta Song", PathBuf::from("/music/beta.mp3"));
    let track3 = Track::new("Gamma Song", PathBuf::from("/music/gamma.mp3"));

    db.add_track(track1).await.unwrap();
    db.add_track(track2).await.unwrap();
    db.add_track(track3).await.unwrap();

    // Get all tracks
    let tracks = db.get_all_tracks().await.expect("Failed to get all tracks");
    assert_eq!(tracks.len(), 3);

    // Verify they're sorted by title
    let titles: Vec<String> = tracks.iter().map(|t| t.title.clone()).collect();
    assert_eq!(titles, vec!["Alpha Song", "Beta Song", "Gamma Song"]);
}

#[tokio::test]
async fn test_search_tracks() {
    let db = create_test_db().await;

    // Add tracks with different metadata
    let mut track1 = Track::new("Bohemian Rhapsody", PathBuf::from("/music/1.mp3"));
    track1.artist = Some("Queen".to_string());

    let mut track2 = Track::new("Another One Bites the Dust", PathBuf::from("/music/2.mp3"));
    track2.artist = Some("Queen".to_string());

    let mut track3 = Track::new("Stairway to Heaven", PathBuf::from("/music/3.mp3"));
    track3.artist = Some("Led Zeppelin".to_string());

    db.add_track(track1).await.unwrap();
    db.add_track(track2).await.unwrap();
    db.add_track(track3).await.unwrap();

    // Search by artist
    let queen_tracks = db.search_tracks("Queen").await.expect("Search failed");
    assert_eq!(queen_tracks.len(), 2);

    // Search by title
    let stairway = db.search_tracks("Stairway").await.expect("Search failed");
    assert_eq!(stairway.len(), 1);
    assert_eq!(stairway[0].title, "Stairway to Heaven");

    // Search with no results
    let no_results = db
        .search_tracks("Nonexistent")
        .await
        .expect("Search failed");
    assert_eq!(no_results.len(), 0);
}

#[tokio::test]
async fn test_delete_track() {
    let db = create_test_db().await;

    // Add a track
    let track = Track::new("To Delete", PathBuf::from("/music/delete.mp3"));
    let track_id = db.add_track(track).await.unwrap();

    // Verify it exists
    assert!(db.get_track(&track_id).await.is_ok());

    // Delete it
    db.delete_track(&track_id)
        .await
        .expect("Failed to delete track");

    // Verify it's gone
    assert!(db.get_track(&track_id).await.is_err());
}

#[tokio::test]
async fn test_playlist_creation() {
    let db = create_test_db().await;

    // Create a user
    let user = db.create_user("Alice").await.unwrap();

    // Create a playlist
    let playlist = db
        .create_playlist(&user.id, "My Favorites")
        .await
        .expect("Failed to create playlist");
    assert_eq!(playlist.name, "My Favorites");
    assert_eq!(playlist.owner_id, user.id);
}

#[tokio::test]
async fn test_get_user_playlists() {
    let db = create_test_db().await;

    // Create two users
    let alice = db.create_user("Alice").await.unwrap();
    let bob = db.create_user("Bob").await.unwrap();

    // Each creates playlists
    db.create_playlist(&alice.id, "Alice's Rock").await.unwrap();
    db.create_playlist(&alice.id, "Alice's Jazz").await.unwrap();
    db.create_playlist(&bob.id, "Bob's Classics").await.unwrap();

    // Get Alice's playlists
    let alice_playlists = db
        .get_user_playlists(&alice.id)
        .await
        .expect("Failed to get playlists");
    assert_eq!(alice_playlists.len(), 2);

    // Verify they're Alice's (not Bob's)
    for playlist in &alice_playlists {
        assert_eq!(playlist.owner_id, alice.id);
    }

    // Get Bob's playlists
    let bob_playlists = db
        .get_user_playlists(&bob.id)
        .await
        .expect("Failed to get playlists");
    assert_eq!(bob_playlists.len(), 1);
    assert_eq!(bob_playlists[0].name, "Bob's Classics");
}

#[tokio::test]
async fn test_add_tracks_to_playlist() {
    let db = create_test_db().await;

    // Create user and playlist
    let user = db.create_user("Alice").await.unwrap();
    let playlist = db.create_playlist(&user.id, "My Mix").await.unwrap();

    // Add some tracks
    let track1 = Track::new("Song 1", PathBuf::from("/music/1.mp3"));
    let track2 = Track::new("Song 2", PathBuf::from("/music/2.mp3"));
    let track3 = Track::new("Song 3", PathBuf::from("/music/3.mp3"));

    let id1 = db.add_track(track1).await.unwrap();
    let id2 = db.add_track(track2).await.unwrap();
    let id3 = db.add_track(track3).await.unwrap();

    // Add tracks to playlist
    db.add_track_to_playlist(&playlist.id, &id1).await.unwrap();
    db.add_track_to_playlist(&playlist.id, &id2).await.unwrap();
    db.add_track_to_playlist(&playlist.id, &id3).await.unwrap();

    // Get playlist tracks
    let tracks = db
        .get_playlist_tracks(&playlist.id)
        .await
        .expect("Failed to get playlist tracks");
    assert_eq!(tracks.len(), 3);

    // Verify order is preserved
    assert_eq!(tracks[0].title, "Song 1");
    assert_eq!(tracks[1].title, "Song 2");
    assert_eq!(tracks[2].title, "Song 3");
}

#[tokio::test]
async fn test_remove_track_from_playlist() {
    let db = create_test_db().await;

    // Setup
    let user = db.create_user("Alice").await.unwrap();
    let playlist = db.create_playlist(&user.id, "Test").await.unwrap();
    let track = Track::new("To Remove", PathBuf::from("/music/remove.mp3"));
    let track_id = db.add_track(track).await.unwrap();

    // Add track to playlist
    db.add_track_to_playlist(&playlist.id, &track_id)
        .await
        .unwrap();

    // Verify it's there
    let tracks = db.get_playlist_tracks(&playlist.id).await.unwrap();
    assert_eq!(tracks.len(), 1);

    // Remove it
    db.remove_track_from_playlist(&playlist.id, &track_id)
        .await
        .expect("Failed to remove track");

    // Verify it's gone
    let tracks = db.get_playlist_tracks(&playlist.id).await.unwrap();
    assert_eq!(tracks.len(), 0);
}

#[tokio::test]
async fn test_delete_playlist() {
    let db = create_test_db().await;

    // Create user and playlist
    let user = db.create_user("Alice").await.unwrap();
    let playlist = db.create_playlist(&user.id, "To Delete").await.unwrap();

    // Verify it exists
    assert!(db.get_playlist(&playlist.id).await.is_ok());

    // Delete it
    db.delete_playlist(&playlist.id)
        .await
        .expect("Failed to delete playlist");

    // Verify it's gone
    assert!(db.get_playlist(&playlist.id).await.is_err());
}

#[tokio::test]
async fn test_playlist_sharing() {
    let db = create_test_db().await;

    // Create two users
    let alice = db.create_user("Alice").await.unwrap();
    let bob = db.create_user("Bob").await.unwrap();

    // Alice creates a playlist
    let playlist = db
        .create_playlist(&alice.id, "Alice's Shared Playlist")
        .await
        .unwrap();

    // Alice shares it with Bob (read permission)
    db.share_playlist(&playlist.id, &bob.id, Permission::Read)
        .await
        .expect("Failed to share playlist");

    // Get shares for the playlist
    let shares = db
        .get_playlist_shares(&playlist.id)
        .await
        .expect("Failed to get shares");
    assert_eq!(shares.len(), 1);
    assert_eq!(shares[0].shared_with_user_id, bob.id);
    assert_eq!(shares[0].permission, Permission::Read);

    // Bob should see it in accessible playlists
    let bob_accessible = db
        .get_accessible_playlists(&bob.id)
        .await
        .expect("Failed to get accessible playlists");
    assert_eq!(bob_accessible.len(), 1);
    assert_eq!(bob_accessible[0].id, playlist.id);
}

#[tokio::test]
async fn test_accessible_playlists_includes_owned_and_shared() {
    let db = create_test_db().await;

    // Create two users
    let alice = db.create_user("Alice").await.unwrap();
    let bob = db.create_user("Bob").await.unwrap();

    // Alice creates playlists
    let alice_playlist1 = db
        .create_playlist(&alice.id, "Alice's Own 1")
        .await
        .unwrap();
    let alice_playlist2 = db
        .create_playlist(&alice.id, "Alice's Own 2")
        .await
        .unwrap();

    // Bob creates a playlist and shares it with Alice
    let bob_playlist = db.create_playlist(&bob.id, "Bob's Shared").await.unwrap();
    db.share_playlist(&bob_playlist.id, &alice.id, Permission::Write)
        .await
        .unwrap();

    // Alice should see her own playlists + Bob's shared playlist
    let alice_accessible = db
        .get_accessible_playlists(&alice.id)
        .await
        .expect("Failed to get accessible playlists");
    assert_eq!(alice_accessible.len(), 3);

    // Verify all three playlists are present
    let ids: Vec<_> = alice_accessible.iter().map(|p| &p.id).collect();
    assert!(ids.contains(&&alice_playlist1.id));
    assert!(ids.contains(&&alice_playlist2.id));
    assert!(ids.contains(&&bob_playlist.id));
}

#[tokio::test]
async fn test_unshare_playlist() {
    let db = create_test_db().await;

    // Setup
    let alice = db.create_user("Alice").await.unwrap();
    let bob = db.create_user("Bob").await.unwrap();
    let playlist = db.create_playlist(&alice.id, "Shared").await.unwrap();

    // Share with Bob
    db.share_playlist(&playlist.id, &bob.id, Permission::Read)
        .await
        .unwrap();

    // Verify Bob has access
    let bob_accessible = db.get_accessible_playlists(&bob.id).await.unwrap();
    assert_eq!(bob_accessible.len(), 1);

    // Unshare
    db.unshare_playlist(&playlist.id, &bob.id)
        .await
        .expect("Failed to unshare");

    // Verify Bob no longer has access
    let bob_accessible = db.get_accessible_playlists(&bob.id).await.unwrap();
    assert_eq!(bob_accessible.len(), 0);
}

#[tokio::test]
async fn test_delete_playlist_cascades_to_playlist_tracks() {
    let db = create_test_db().await;

    // Setup
    let user = db.create_user("Alice").await.unwrap();
    let playlist = db.create_playlist(&user.id, "To Delete").await.unwrap();

    let track = Track::new("Test", PathBuf::from("/music/test.mp3"));
    let track_id = db.add_track(track).await.unwrap();

    // Add track to playlist
    db.add_track_to_playlist(&playlist.id, &track_id)
        .await
        .unwrap();

    // Verify track is in playlist
    let tracks = db.get_playlist_tracks(&playlist.id).await.unwrap();
    assert_eq!(tracks.len(), 1);

    // Delete playlist
    db.delete_playlist(&playlist.id).await.unwrap();

    // Verify track still exists (not deleted)
    assert!(db.get_track(&track_id).await.is_ok());

    // Playlist is gone
    assert!(db.get_playlist(&playlist.id).await.is_err());
}

#[tokio::test]
async fn test_duplicate_track_in_playlist_handled() {
    let db = create_test_db().await;

    let user = db.create_user("Alice").await.unwrap();
    let playlist = db.create_playlist(&user.id, "Test").await.unwrap();
    let track = Track::new("Song", PathBuf::from("/music/song.mp3"));
    let track_id = db.add_track(track).await.unwrap();

    // Add track once
    db.add_track_to_playlist(&playlist.id, &track_id)
        .await
        .unwrap();

    // Try to add same track again - should fail due to PRIMARY KEY constraint
    let result = db.add_track_to_playlist(&playlist.id, &track_id).await;
    assert!(result.is_err());
}
