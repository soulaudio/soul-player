//! Integration tests for track statistics and play tracking
//!
//! Tests play tracking operations including:
//! - Play count recording and incrementing
//! - Skip count tracking
//! - Top tracks query with proper sorting
//! - Per-user isolation of statistics
//! - Completion threshold logic

mod test_helpers;

use test_helpers::*;

#[tokio::test]
async fn test_record_play_increments_count() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let user_id = create_test_user(pool, "test_user").await;
    let artist_id = create_test_artist(pool, "Test Artist", None).await;
    let album_id = create_test_album(pool, "Test Album", Some(artist_id), Some(2024)).await;
    let track_id = create_test_track(
        pool,
        "Test Track",
        Some(artist_id),
        Some(album_id),
        1,
        Some("/music/test.mp3"),
    )
    .await;

    // Initial play count should be 0
    let initial_count =
        soul_storage::tracks::get_play_count(pool, user_id.clone(), track_id.clone())
            .await
            .expect("Failed to get initial play count");
    assert_eq!(initial_count, 0);

    // Record a play (completed)
    soul_storage::tracks::record_play(pool, user_id.clone(), track_id.clone(), Some(180.0), true)
        .await
        .expect("Failed to record play");

    // Play count should be 1
    let count_after_one_play =
        soul_storage::tracks::get_play_count(pool, user_id.clone(), track_id.clone())
            .await
            .expect("Failed to get play count");
    assert_eq!(count_after_one_play, 1);

    // Record another play
    soul_storage::tracks::record_play(pool, user_id.clone(), track_id.clone(), Some(180.0), true)
        .await
        .expect("Failed to record second play");

    // Play count should be 2
    let count_after_two_plays =
        soul_storage::tracks::get_play_count(pool, user_id.clone(), track_id.clone())
            .await
            .expect("Failed to get play count");
    assert_eq!(count_after_two_plays, 2);
}

#[tokio::test]
async fn test_record_skip_increments_skip_count() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let user_id = create_test_user(pool, "test_user").await;
    let artist_id = create_test_artist(pool, "Test Artist", None).await;
    let track_id = create_test_track(
        pool,
        "Test Track",
        Some(artist_id),
        None,
        1,
        Some("/music/test.mp3"),
    )
    .await;

    // Record a skip (completed=false)
    soul_storage::tracks::record_play(pool, user_id.clone(), track_id.clone(), Some(180.0), false)
        .await
        .expect("Failed to record skip");

    // Query track_stats directly to check skip_count
    let stats = sqlx::query!(
        "SELECT play_count, skip_count FROM track_stats WHERE user_id = ? AND track_id = ?",
        user_id,
        track_id
    )
    .fetch_optional(pool)
    .await
    .expect("Failed to fetch track stats");

    let stats = stats.expect("No stats record found");
    assert_eq!(
        stats.play_count, 0,
        "Play count should be 0 for skipped track"
    );
    assert_eq!(stats.skip_count, 1, "Skip count should be 1");

    // Record another skip
    soul_storage::tracks::record_play(pool, user_id.clone(), track_id.clone(), Some(180.0), false)
        .await
        .expect("Failed to record second skip");

    let stats = sqlx::query!(
        "SELECT play_count, skip_count FROM track_stats WHERE user_id = ? AND track_id = ?",
        user_id,
        track_id
    )
    .fetch_optional(pool)
    .await
    .expect("Failed to fetch track stats");

    let stats = stats.expect("No stats record found");
    assert_eq!(stats.play_count, 0, "Play count should still be 0");
    assert_eq!(stats.skip_count, 2, "Skip count should be 2");
}

#[tokio::test]
async fn test_top_tracks_query_sorts_correctly() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let user_id = create_test_user(pool, "test_user").await;
    let artist_id = create_test_artist(pool, "Popular Artist", None).await;
    let album_id = create_test_album(pool, "Greatest Hits", Some(artist_id), Some(2024)).await;

    // Create 5 tracks with different play counts
    let track1 = create_test_track(
        pool,
        "Track 1",
        Some(artist_id),
        Some(album_id),
        1,
        Some("/music/1.mp3"),
    )
    .await;
    let track2 = create_test_track(
        pool,
        "Track 2",
        Some(artist_id),
        Some(album_id),
        1,
        Some("/music/2.mp3"),
    )
    .await;
    let track3 = create_test_track(
        pool,
        "Track 3",
        Some(artist_id),
        Some(album_id),
        1,
        Some("/music/3.mp3"),
    )
    .await;
    let track4 = create_test_track(
        pool,
        "Track 4",
        Some(artist_id),
        Some(album_id),
        1,
        Some("/music/4.mp3"),
    )
    .await;
    let track5 = create_test_track(
        pool,
        "Track 5",
        Some(artist_id),
        Some(album_id),
        1,
        Some("/music/5.mp3"),
    )
    .await;

    // Record plays: track3 (5 plays), track1 (3 plays), track4 (2 plays), track2 (1 play), track5 (0 plays)
    for _ in 0..5 {
        soul_storage::tracks::record_play(pool, user_id.clone(), track3.clone(), Some(180.0), true)
            .await
            .unwrap();
    }
    for _ in 0..3 {
        soul_storage::tracks::record_play(pool, user_id.clone(), track1.clone(), Some(180.0), true)
            .await
            .unwrap();
    }
    for _ in 0..2 {
        soul_storage::tracks::record_play(pool, user_id.clone(), track4.clone(), Some(180.0), true)
            .await
            .unwrap();
    }
    soul_storage::tracks::record_play(pool, user_id.clone(), track2.clone(), Some(180.0), true)
        .await
        .unwrap();
    // track5 has 0 plays

    // Get top tracks
    let top_tracks =
        soul_storage::tracks::get_top_tracks_by_artist(pool, user_id.clone(), artist_id, 10)
            .await
            .expect("Failed to get top tracks");

    // Should have 5 tracks
    assert_eq!(top_tracks.len(), 5);

    // Should be sorted by play count DESC
    assert_eq!(
        top_tracks[0].id, track3,
        "Track 3 should be first (5 plays)"
    );
    assert_eq!(
        top_tracks[1].id, track1,
        "Track 1 should be second (3 plays)"
    );
    assert_eq!(
        top_tracks[2].id, track4,
        "Track 4 should be third (2 plays)"
    );
    assert_eq!(
        top_tracks[3].id, track2,
        "Track 2 should be fourth (1 play)"
    );
    assert_eq!(
        top_tracks[4].id, track5,
        "Track 5 should be fifth (0 plays)"
    );

    // Test limit parameter
    let top_3 = soul_storage::tracks::get_top_tracks_by_artist(pool, user_id.clone(), artist_id, 3)
        .await
        .expect("Failed to get top 3 tracks");

    assert_eq!(top_3.len(), 3);
    assert_eq!(top_3[0].id, track3);
    assert_eq!(top_3[1].id, track1);
    assert_eq!(top_3[2].id, track4);
}

#[tokio::test]
async fn test_per_user_isolation() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let user1 = create_test_user(pool, "user1").await;
    let user2 = create_test_user(pool, "user2").await;
    let artist_id = create_test_artist(pool, "Shared Artist", None).await;
    let track_id = create_test_track(
        pool,
        "Shared Track",
        Some(artist_id),
        None,
        1,
        Some("/music/shared.mp3"),
    )
    .await;

    // User 1 plays the track 3 times
    for _ in 0..3 {
        soul_storage::tracks::record_play(pool, user1.clone(), track_id.clone(), Some(180.0), true)
            .await
            .unwrap();
    }

    // User 2 plays the track 1 time
    soul_storage::tracks::record_play(pool, user2.clone(), track_id.clone(), Some(180.0), true)
        .await
        .unwrap();

    // User 1 should see 3 plays
    let user1_count = soul_storage::tracks::get_play_count(pool, user1.clone(), track_id.clone())
        .await
        .expect("Failed to get user1 play count");
    assert_eq!(user1_count, 3);

    // User 2 should see 1 play
    let user2_count = soul_storage::tracks::get_play_count(pool, user2.clone(), track_id.clone())
        .await
        .expect("Failed to get user2 play count");
    assert_eq!(user2_count, 1);

    // User 1's top tracks should show the shared track
    let user1_top =
        soul_storage::tracks::get_top_tracks_by_artist(pool, user1.clone(), artist_id, 10)
            .await
            .expect("Failed to get user1 top tracks");
    assert_eq!(user1_top.len(), 1);
    assert_eq!(user1_top[0].id, track_id);

    // User 2's top tracks should also show the shared track
    let user2_top =
        soul_storage::tracks::get_top_tracks_by_artist(pool, user2.clone(), artist_id, 10)
            .await
            .expect("Failed to get user2 top tracks");
    assert_eq!(user2_top.len(), 1);
    assert_eq!(user2_top[0].id, track_id);

    // But the ordering should differ if we add more tracks
    let track2 = create_test_track(
        pool,
        "Track 2",
        Some(artist_id),
        None,
        1,
        Some("/music/2.mp3"),
    )
    .await;

    // User 2 plays track2 twice (more than track_id for user2)
    for _ in 0..2 {
        soul_storage::tracks::record_play(pool, user2.clone(), track2.clone(), Some(180.0), true)
            .await
            .unwrap();
    }

    let user2_top_updated =
        soul_storage::tracks::get_top_tracks_by_artist(pool, user2.clone(), artist_id, 10)
            .await
            .expect("Failed to get user2 top tracks");

    // For user2, track2 (2 plays) should be first, track_id (1 play) should be second
    assert_eq!(user2_top_updated[0].id, track2);
    assert_eq!(user2_top_updated[1].id, track_id);

    // For user1, track_id (3 plays) should still be first
    let user1_top_updated =
        soul_storage::tracks::get_top_tracks_by_artist(pool, user1.clone(), artist_id, 10)
            .await
            .expect("Failed to get user1 top tracks");
    assert_eq!(user1_top_updated[0].id, track_id);
    assert_eq!(user1_top_updated[1].id, track2); // track2 has 0 plays for user1
}

#[tokio::test]
async fn test_completion_threshold_behavior() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let user_id = create_test_user(pool, "test_user").await;
    let artist_id = create_test_artist(pool, "Test Artist", None).await;
    let track_id = create_test_track(
        pool,
        "Test Track",
        Some(artist_id),
        None,
        1,
        Some("/music/test.mp3"),
    )
    .await;

    // Record a completed play (simulating 80%+ completion)
    soul_storage::tracks::record_play(pool, user_id.clone(), track_id.clone(), Some(180.0), true)
        .await
        .expect("Failed to record completed play");

    let stats_after_complete = sqlx::query!(
        "SELECT play_count, skip_count FROM track_stats WHERE user_id = ? AND track_id = ?",
        user_id,
        track_id
    )
    .fetch_optional(pool)
    .await
    .expect("Failed to fetch stats");

    let stats = stats_after_complete.expect("No stats found");
    assert_eq!(
        stats.play_count, 1,
        "Completed play should increment play_count"
    );
    assert_eq!(
        stats.skip_count, 0,
        "Completed play should not increment skip_count"
    );

    // Record an incomplete play (simulating <80% completion)
    soul_storage::tracks::record_play(pool, user_id.clone(), track_id.clone(), Some(180.0), false)
        .await
        .expect("Failed to record incomplete play");

    let stats_after_skip = sqlx::query!(
        "SELECT play_count, skip_count FROM track_stats WHERE user_id = ? AND track_id = ?",
        user_id,
        track_id
    )
    .fetch_optional(pool)
    .await
    .expect("Failed to fetch stats");

    let stats = stats_after_skip.expect("No stats found");
    assert_eq!(stats.play_count, 1, "Skip should not increment play_count");
    assert_eq!(stats.skip_count, 1, "Skip should increment skip_count");
}

#[tokio::test]
async fn test_top_tracks_with_no_plays() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let user_id = create_test_user(pool, "test_user").await;
    let artist_id = create_test_artist(pool, "New Artist", None).await;

    // Create tracks but don't play any
    create_test_track(
        pool,
        "Unplayed Track 1",
        Some(artist_id),
        None,
        1,
        Some("/music/1.mp3"),
    )
    .await;
    create_test_track(
        pool,
        "Unplayed Track 2",
        Some(artist_id),
        None,
        1,
        Some("/music/2.mp3"),
    )
    .await;

    // Should still return tracks, sorted by title since play_count is 0 for all
    let top_tracks =
        soul_storage::tracks::get_top_tracks_by_artist(pool, user_id.clone(), artist_id, 10)
            .await
            .expect("Failed to get top tracks");

    assert_eq!(top_tracks.len(), 2);
    // Both have 0 plays, so sorted alphabetically by title
    assert_eq!(top_tracks[0].title, "Unplayed Track 1");
    assert_eq!(top_tracks[1].title, "Unplayed Track 2");
}

#[tokio::test]
async fn test_play_history_records_created() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let user_id = create_test_user(pool, "test_user").await;
    let artist_id = create_test_artist(pool, "Test Artist", None).await;
    let track_id = create_test_track(
        pool,
        "Test Track",
        Some(artist_id),
        None,
        1,
        Some("/music/test.mp3"),
    )
    .await;

    // Record a play
    soul_storage::tracks::record_play(pool, user_id.clone(), track_id.clone(), Some(180.0), true)
        .await
        .expect("Failed to record play");

    // Check that a play_history record was created
    let history_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM play_history WHERE user_id = ? AND track_id = ?",
    )
    .bind(user_id.as_str())
    .bind(&track_id)
    .fetch_one(pool)
    .await
    .expect("Failed to count play history");

    assert_eq!(history_count, 1, "Should have one play history record");

    // Record another play
    soul_storage::tracks::record_play(pool, user_id.clone(), track_id.clone(), Some(200.0), true)
        .await
        .expect("Failed to record second play");

    let history_count_after = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM play_history WHERE user_id = ? AND track_id = ?",
    )
    .bind(user_id.as_str())
    .bind(track_id)
    .fetch_one(pool)
    .await
    .expect("Failed to count play history");

    assert_eq!(
        history_count_after, 2,
        "Should have two play history records"
    );
}

#[tokio::test]
async fn test_last_played_at_updates() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let user_id = create_test_user(pool, "test_user").await;
    let artist_id = create_test_artist(pool, "Test Artist", None).await;
    let track_id = create_test_track(
        pool,
        "Test Track",
        Some(artist_id),
        None,
        1,
        Some("/music/test.mp3"),
    )
    .await;

    // Record first play
    soul_storage::tracks::record_play(pool, user_id.clone(), track_id.clone(), Some(180.0), true)
        .await
        .expect("Failed to record play");

    let first_stats = sqlx::query!(
        "SELECT last_played_at FROM track_stats WHERE user_id = ? AND track_id = ?",
        user_id,
        track_id
    )
    .fetch_optional(pool)
    .await
    .expect("Failed to fetch stats");

    let first_last_played = first_stats.expect("No stats found").last_played_at;
    assert!(first_last_played.is_some(), "last_played_at should be set");

    // Wait a tiny bit to ensure timestamp differs
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    // Record second play
    soul_storage::tracks::record_play(pool, user_id.clone(), track_id.clone(), Some(180.0), true)
        .await
        .expect("Failed to record second play");

    let second_stats = sqlx::query!(
        "SELECT last_played_at FROM track_stats WHERE user_id = ? AND track_id = ?",
        user_id,
        track_id
    )
    .fetch_optional(pool)
    .await
    .expect("Failed to fetch stats");

    let second_last_played = second_stats.expect("No stats found").last_played_at;

    // last_played_at should have been updated (newer timestamp)
    assert!(
        second_last_played.unwrap() >= first_last_played.unwrap(),
        "last_played_at should be updated to newer timestamp"
    );
}
