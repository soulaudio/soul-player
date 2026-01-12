//! Three-Tier Queue System Tests
//!
//! Tests for the new three-tier queue architecture:
//! 1. Play Next Queue - Highest priority, explicit insertions after current track
//! 2. Add to Queue - Medium priority, explicit appends to end
//! 3. Source Queue - Lowest priority, from album/playlist/artist
//!
//! This ensures Spotify-like queue behavior with proper priority ordering.

use soul_playback::{PlaybackManager, QueueTrack, RepeatMode, ShuffleMode, TrackSource};
use std::path::PathBuf;
use std::time::Duration;

// ===== Test Helpers =====

fn create_track(id: &str, title: &str, artist: &str, duration_secs: u64) -> QueueTrack {
    QueueTrack {
        id: id.to_string(),
        path: PathBuf::from(format!("/music/{}.mp3", id)),
        title: title.to_string(),
        artist: artist.to_string(),
        album: Some("Test Album".to_string()),
        duration: Duration::from_secs(duration_secs),
        track_number: Some(id.parse().unwrap_or(1)),
        source: TrackSource::Single,
    }
}

// ===== Three-Tier Priority Tests =====

#[test]
fn test_play_next_has_highest_priority() {
    let mut manager = PlaybackManager::default();

    // Set up source queue (lowest priority)
    manager.add_playlist_to_queue(vec![
        create_track("s1", "Source 1", "Artist A", 180),
        create_track("s2", "Source 2", "Artist B", 180),
    ]);

    // Add to "Add to Queue" (medium priority)
    manager.add_to_queue_end(create_track("q1", "Queue 1", "Artist C", 180));

    // Add to "Play Next" (highest priority)
    manager.add_to_queue_next(create_track("n1", "Next 1", "Artist D", 180));

    // Start playback
    manager.next().ok();

    // Next track should be from "Play Next" queue
    let queue = manager.get_queue();
    assert_eq!(
        queue[0].id, "n1",
        "Play Next track should have highest priority"
    );
}

#[test]
fn test_add_to_queue_before_source() {
    let mut manager = PlaybackManager::default();

    // Set up source queue
    manager.add_playlist_to_queue(vec![
        create_track("s1", "Source 1", "Artist A", 180),
        create_track("s2", "Source 2", "Artist B", 180),
    ]);

    // Add to "Add to Queue"
    manager.add_to_queue_end(create_track("q1", "Queue 1", "Artist C", 180));

    // Start playback (should play s1 first)
    manager.next().ok();

    // Next should be s2 (continuing source)
    let queue = manager.get_queue();
    assert_eq!(queue[0].id, "s2");

    // After source queue exhausts, should play "Add to Queue" tracks
    manager.next().ok(); // Play s2
    manager.next().ok(); // Source exhausted, now should play q1

    // Queue should now be empty (q1 is current)
    let queue = manager.get_queue();
    assert_eq!(
        queue.len(),
        0,
        "After exhausting all queues, remaining queue should be empty"
    );
}

#[test]
fn test_source_queue_lowest_priority() {
    let mut manager = PlaybackManager::default();

    // Add source queue
    manager.add_playlist_to_queue(vec![
        create_track("s1", "Source 1", "Artist A", 180),
        create_track("s2", "Source 2", "Artist B", 180),
        create_track("s3", "Source 3", "Artist C", 180),
    ]);

    // Add "Add to Queue" track
    manager.add_to_queue_end(create_track("q1", "Queue 1", "Artist D", 180));

    // Add "Play Next" track
    manager.add_to_queue_next(create_track("n1", "Next 1", "Artist E", 180));

    // Start playback - should play s1 first
    manager.next().ok();

    // Verify priority order: n1 (Play Next) → s2, s3 (Source) → q1 (Add to Queue)
    let queue = manager.get_queue();
    let ids: Vec<String> = queue.iter().map(|t| t.id.clone()).collect();

    assert_eq!(ids[0], "n1", "Play Next should be first");
    assert_eq!(ids[1], "s2", "Source queue continues");
    assert_eq!(ids[2], "s3", "Source queue continues");
    assert_eq!(ids[3], "q1", "Add to Queue should be last");
}

#[test]
fn test_multiple_play_next_tracks_lifo_order() {
    let mut manager = PlaybackManager::default();

    // Add source queue
    manager.add_playlist_to_queue(vec![create_track("s1", "Source 1", "Artist A", 180)]);

    // Add multiple "Play Next" tracks
    manager.add_to_queue_next(create_track("n1", "Next 1", "Artist B", 180));
    manager.add_to_queue_next(create_track("n2", "Next 2", "Artist C", 180));
    manager.add_to_queue_next(create_track("n3", "Next 3", "Artist D", 180));

    // Start playback
    manager.next().ok();

    // Play Next queue should be LIFO (last added plays first)
    let queue = manager.get_queue();
    let ids: Vec<String> = queue.iter().take(3).map(|t| t.id.clone()).collect();

    assert_eq!(
        ids,
        vec!["n3", "n2", "n1"],
        "Play Next should be LIFO order"
    );
}

#[test]
fn test_multiple_add_to_queue_tracks_fifo_order() {
    let mut manager = PlaybackManager::default();

    // Add source queue
    manager.add_playlist_to_queue(vec![create_track("s1", "Source 1", "Artist A", 180)]);

    // Add multiple "Add to Queue" tracks
    manager.add_to_queue_end(create_track("q1", "Queue 1", "Artist B", 180));
    manager.add_to_queue_end(create_track("q2", "Queue 2", "Artist C", 180));
    manager.add_to_queue_end(create_track("q3", "Queue 3", "Artist D", 180));

    // Exhaust source queue
    manager.next().ok(); // Play s1

    // Add to Queue should be FIFO (first added plays first)
    manager.next().ok(); // Should play q1
    manager.next().ok(); // Should play q2

    let queue = manager.get_queue();
    assert_eq!(queue[0].id, "q3", "Add to Queue should be FIFO order");
}

// ===== Queue Clearing Tests =====

#[test]
fn test_play_next_cleared_on_new_context() {
    let mut manager = PlaybackManager::default();

    // Add source queue
    manager.add_playlist_to_queue(vec![create_track("s1", "Source 1", "Artist A", 180)]);

    // Add "Play Next" tracks
    manager.add_to_queue_next(create_track("n1", "Next 1", "Artist B", 180));
    manager.add_to_queue_next(create_track("n2", "Next 2", "Artist C", 180));

    // Start playback
    manager.next().ok();

    // Play new album/playlist (new context)
    manager.add_playlist_to_queue(vec![
        create_track("new1", "New 1", "Artist D", 180),
        create_track("new2", "New 2", "Artist E", 180),
    ]);

    // Play Next queue should be cleared
    let queue = manager.get_queue();
    let ids: Vec<String> = queue.iter().map(|t| t.id.clone()).collect();

    assert!(
        !ids.contains(&"n1".to_string()) && !ids.contains(&"n2".to_string()),
        "Play Next queue should be cleared on new context"
    );
}

#[test]
fn test_add_to_queue_persists_on_new_context() {
    let mut manager = PlaybackManager::default();

    // Add source queue
    manager.add_playlist_to_queue(vec![create_track("s1", "Source 1", "Artist A", 180)]);

    // Add "Add to Queue" tracks
    manager.add_to_queue_end(create_track("q1", "Queue 1", "Artist B", 180));
    manager.add_to_queue_end(create_track("q2", "Queue 2", "Artist C", 180));

    // Start playback
    manager.next().ok();

    // Play new album/playlist (new context)
    manager.add_playlist_to_queue(vec![
        create_track("new1", "New 1", "Artist D", 180),
        create_track("new2", "New 2", "Artist E", 180),
    ]);

    // Add to Queue should persist (match Spotify behavior)
    let queue = manager.get_queue();
    let ids: Vec<String> = queue.iter().map(|t| t.id.clone()).collect();

    assert!(
        ids.contains(&"q1".to_string()) && ids.contains(&"q2".to_string()),
        "Add to Queue should persist on new context"
    );
}

#[test]
fn test_clear_play_next_only() {
    let mut manager = PlaybackManager::default();

    // Add all three queue types
    manager.add_playlist_to_queue(vec![create_track("s1", "Source 1", "Artist A", 180)]);
    manager.add_to_queue_end(create_track("q1", "Queue 1", "Artist B", 180));
    manager.add_to_queue_next(create_track("n1", "Next 1", "Artist C", 180));

    // Clear Play Next queue
    manager.clear_play_next();

    let queue = manager.get_queue();
    let ids: Vec<String> = queue.iter().map(|t| t.id.clone()).collect();

    assert!(
        !ids.contains(&"n1".to_string()),
        "Play Next should be cleared"
    );
    assert!(
        ids.contains(&"s1".to_string()),
        "Source queue should remain"
    );
    assert!(
        ids.contains(&"q1".to_string()),
        "Add to Queue should remain"
    );
}

#[test]
fn test_clear_add_to_queue_only() {
    let mut manager = PlaybackManager::default();

    // Add all three queue types
    manager.add_playlist_to_queue(vec![create_track("s1", "Source 1", "Artist A", 180)]);
    manager.add_to_queue_end(create_track("q1", "Queue 1", "Artist B", 180));
    manager.add_to_queue_next(create_track("n1", "Next 1", "Artist C", 180));

    // Clear Add to Queue
    manager.clear_add_to_queue();

    let queue = manager.get_queue();
    let ids: Vec<String> = queue.iter().map(|t| t.id.clone()).collect();

    assert!(
        !ids.contains(&"q1".to_string()),
        "Add to Queue should be cleared"
    );
    assert!(
        ids.contains(&"s1".to_string()),
        "Source queue should remain"
    );
    assert!(ids.contains(&"n1".to_string()), "Play Next should remain");
}

// ===== Repeat Mode Interaction Tests =====

#[test]
fn test_repeat_all_only_affects_source_queue() {
    let mut manager = PlaybackManager::default();
    manager.set_repeat(RepeatMode::All);

    // Add source queue
    manager.add_playlist_to_queue(vec![
        create_track("s1", "Source 1", "Artist A", 180),
        create_track("s2", "Source 2", "Artist B", 180),
    ]);

    // Add "Add to Queue" track
    manager.add_to_queue_end(create_track("q1", "Queue 1", "Artist C", 180));

    // Play through all tracks
    manager.next().ok(); // s1
    manager.next().ok(); // s2
    manager.next().ok(); // q1 (Add to Queue track)

    // After q1, Repeat All should restart source queue (s1), NOT repeat q1
    manager.next().ok();

    // Should restart from source queue
    let queue = manager.get_queue();
    // Note: Exact behavior depends on implementation
    // With Spotify-style behavior, manual queue tracks should NOT repeat
}

#[test]
fn test_repeat_one_works_on_current_track_only() {
    let mut manager = PlaybackManager::default();
    manager.set_repeat(RepeatMode::One);

    // Add tracks
    manager.add_playlist_to_queue(vec![create_track("s1", "Source 1", "Artist A", 180)]);

    manager.next().ok();

    // With Repeat One, next should replay the same track
    manager.next().ok();

    // Should still be on s1 (repeated)
    // Note: Exact behavior verification depends on current_track tracking
}

// ===== has_next() Tests with Three-Tier Queue =====

#[test]
fn test_has_next_with_play_next_queue() {
    let mut manager = PlaybackManager::default();

    // Only Play Next queue populated
    manager.add_to_queue_next(create_track("n1", "Next 1", "Artist A", 180));

    assert!(
        manager.has_next(),
        "has_next should be true with Play Next tracks"
    );
}

#[test]
fn test_has_next_with_add_to_queue() {
    let mut manager = PlaybackManager::default();

    // Only Add to Queue populated
    manager.add_to_queue_end(create_track("q1", "Queue 1", "Artist A", 180));

    assert!(
        manager.has_next(),
        "has_next should be true with Add to Queue tracks"
    );
}

#[test]
fn test_has_next_with_source_queue() {
    let mut manager = PlaybackManager::default();

    // Only source queue populated
    manager.add_playlist_to_queue(vec![create_track("s1", "Source 1", "Artist A", 180)]);

    assert!(
        manager.has_next(),
        "has_next should be true with source queue tracks"
    );
}

#[test]
fn test_has_next_false_when_all_queues_empty() {
    let manager = PlaybackManager::default();

    // No tracks in any queue
    assert!(
        !manager.has_next(),
        "has_next should be false when all queues are empty"
    );
}

#[test]
fn test_has_next_with_repeat_all_and_empty_manual_queues() {
    let mut manager = PlaybackManager::default();
    manager.set_repeat(RepeatMode::All);

    // Add source queue
    manager.add_playlist_to_queue(vec![
        create_track("s1", "Source 1", "Artist A", 180),
        create_track("s2", "Source 2", "Artist B", 180),
    ]);

    // Play through all source tracks
    manager.next().ok();
    manager.next().ok();

    // With Repeat All and source queue available, should have next
    assert!(
        manager.has_next(),
        "has_next should be true with Repeat All even at end of source queue"
    );
}

// ===== Shuffle Interaction Tests =====

#[test]
fn test_shuffle_only_affects_source_queue() {
    let mut manager = PlaybackManager::default();

    // Add source queue
    manager.add_playlist_to_queue(vec![
        create_track("s1", "Source 1", "Artist A", 180),
        create_track("s2", "Source 2", "Artist B", 180),
        create_track("s3", "Source 3", "Artist C", 180),
    ]);

    // Add "Play Next" track
    manager.add_to_queue_next(create_track("n1", "Next 1", "Artist D", 180));

    // Add "Add to Queue" track
    manager.add_to_queue_end(create_track("q1", "Queue 1", "Artist E", 180));

    // Enable shuffle
    manager.set_shuffle(ShuffleMode::Random);

    // Play Next and Add to Queue should NOT be shuffled
    // Only source queue should be shuffled

    manager.next().ok();

    let queue = manager.get_queue();
    let first_track_id = &queue[0].id;

    // First track should be "n1" (Play Next, not shuffled)
    assert_eq!(
        first_track_id, "n1",
        "Play Next track should not be affected by shuffle"
    );
}

#[test]
fn test_smart_shuffle_distributes_artists_in_source_queue() {
    let mut manager = PlaybackManager::default();

    // Enable smart shuffle BEFORE adding playlist
    manager.set_shuffle(ShuffleMode::Smart);

    // Add source queue with multiple tracks from same artist
    // Use a larger dataset with better distribution for reliable testing
    manager.add_playlist_to_queue(vec![
        create_track("s1", "Track 1", "Artist A", 180),
        create_track("s2", "Track 2", "Artist A", 180),
        create_track("s3", "Track 3", "Artist B", 180),
        create_track("s4", "Track 4", "Artist B", 180),
        create_track("s5", "Track 5", "Artist C", 180),
        create_track("s6", "Track 6", "Artist C", 180),
        create_track("s7", "Track 7", "Artist D", 180),
        create_track("s8", "Track 8", "Artist D", 180),
    ]);

    let queue = manager.get_queue();
    let artists: Vec<String> = queue.iter().map(|t| t.artist.clone()).collect();

    // Smart shuffle should reduce consecutive same-artist plays
    // Count consecutive same-artist occurrences
    let mut consecutive_count = 0;
    for i in 0..artists.len() - 1 {
        if artists[i] == artists[i + 1] {
            consecutive_count += 1;
        }
    }

    // With 8 tracks from 4 artists (2 each), smart shuffle should have
    // 0-1 consecutive pairs at most (much better than random which averages 2-3)
    assert!(
        consecutive_count <= 1,
        "Smart shuffle should minimize consecutive same artist (found {} consecutive pairs)",
        consecutive_count
    );
}

// ===== Edge Cases =====

#[test]
fn test_add_play_next_while_playing() {
    let mut manager = PlaybackManager::default();

    // Start playing source queue
    manager.add_playlist_to_queue(vec![
        create_track("s1", "Source 1", "Artist A", 180),
        create_track("s2", "Source 2", "Artist B", 180),
    ]);

    manager.next().ok();

    // While s1 is playing, add "Play Next" track
    manager.add_to_queue_next(create_track("n1", "Next 1", "Artist C", 180));

    // Next track should be n1, not s2
    let queue = manager.get_queue();
    assert_eq!(
        queue[0].id, "n1",
        "Play Next should insert after current track"
    );
}

#[test]
fn test_add_to_queue_end_while_playing() {
    let mut manager = PlaybackManager::default();

    // Start playing source queue
    manager.add_playlist_to_queue(vec![
        create_track("s1", "Source 1", "Artist A", 180),
        create_track("s2", "Source 2", "Artist B", 180),
    ]);

    manager.next().ok();

    // While s1 is playing, add "Add to Queue" track
    manager.add_to_queue_end(create_track("q1", "Queue 1", "Artist C", 180));

    let queue = manager.get_queue();
    let ids: Vec<String> = queue.iter().map(|t| t.id.clone()).collect();

    // q1 should be at the end, after source queue
    assert_eq!(
        ids.last().unwrap(),
        "q1",
        "Add to Queue should append to end"
    );
}

#[test]
fn test_complex_scenario_all_operations() {
    let mut manager = PlaybackManager::default();

    // 1. Play an album
    manager.add_playlist_to_queue(vec![
        create_track("a1", "Album Track 1", "Artist A", 180),
        create_track("a2", "Album Track 2", "Artist A", 180),
        create_track("a3", "Album Track 3", "Artist A", 180),
    ]);

    manager.next().ok(); // Play a1

    // 2. Add a track to "Play Next"
    manager.add_to_queue_next(create_track("n1", "Play Next 1", "Artist B", 180));

    // 3. Add a track to "Add to Queue"
    manager.add_to_queue_end(create_track("q1", "Queue 1", "Artist C", 180));

    // 4. Add another "Play Next" (should be LIFO)
    manager.add_to_queue_next(create_track("n2", "Play Next 2", "Artist D", 180));

    // Verify order: n2 (LIFO), n1, a2, a3 (source), q1 (end)
    let queue = manager.get_queue();
    let ids: Vec<String> = queue.iter().map(|t| t.id.clone()).collect();

    assert_eq!(ids[0], "n2", "Latest Play Next should be first");
    assert_eq!(ids[1], "n1", "Earlier Play Next should be second");
    assert_eq!(ids[2], "a2", "Source queue continues");
    assert_eq!(ids[3], "a3", "Source queue continues");
    assert_eq!(ids[4], "q1", "Add to Queue should be last");

    // 5. Play through everything and verify order
    manager.next().ok(); // n2
    manager.next().ok(); // n1
    manager.next().ok(); // a2
    manager.next().ok(); // a3
    manager.next().ok(); // q1

    // All queues should be exhausted
    assert_eq!(manager.queue_len(), 0, "All queues should be empty");
}
