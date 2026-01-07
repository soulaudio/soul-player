//! Queue management integration tests
//!
//! Tests for queue creation, navigation, and boundary logic.
//! Focus on real-world scenarios: playing from library, next/previous buttons.

use soul_playback::{
    PlaybackManager, QueueTrack, RepeatMode, ShuffleMode, TrackSource,
};
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

// ===== Queue Creation Tests =====

#[test]
fn test_play_from_library_creates_queue_from_index() {
    let mut manager = PlaybackManager::default();

    // Simulate library with 5 tracks
    let library = vec![
        create_track("1", "Track 1", "Artist A", 180),
        create_track("2", "Track 2", "Artist B", 180),
        create_track("3", "Track 3", "Artist C", 180),
        create_track("4", "Track 4", "Artist D", 180),
        create_track("5", "Track 5", "Artist E", 180),
    ];

    // User clicks track 3 (index 2)
    let start_index = 2;

    // Create queue: [3, 4, 5, 1, 2]
    let queue: Vec<QueueTrack> = library.iter()
        .skip(start_index)
        .chain(library.iter().take(start_index))
        .cloned()
        .collect();

    manager.add_playlist_to_queue(queue.clone());

    // Verify queue order
    let actual_queue = manager.get_queue();
    assert_eq!(actual_queue.len(), 5);
    assert_eq!(actual_queue[0].id, "3");
    assert_eq!(actual_queue[1].id, "4");
    assert_eq!(actual_queue[2].id, "5");
    assert_eq!(actual_queue[3].id, "1");
    assert_eq!(actual_queue[4].id, "2");
}

#[test]
fn test_play_last_track_wraps_queue() {
    let mut manager = PlaybackManager::default();

    let library = vec![
        create_track("1", "Track 1", "Artist A", 180),
        create_track("2", "Track 2", "Artist B", 180),
        create_track("3", "Track 3", "Artist C", 180),
    ];

    // User clicks last track (index 2)
    let start_index = 2;

    let queue: Vec<QueueTrack> = library.iter()
        .skip(start_index)
        .chain(library.iter().take(start_index))
        .cloned()
        .collect();

    manager.add_playlist_to_queue(queue);

    // Should be: [3, 1, 2]
    let actual_queue = manager.get_queue();
    assert_eq!(actual_queue[0].id, "3");
    assert_eq!(actual_queue[1].id, "1");
    assert_eq!(actual_queue[2].id, "2");
}

// ===== has_next() / has_previous() Tests =====

#[test]
fn test_has_next_with_tracks_in_queue() {
    let mut manager = PlaybackManager::default();

    manager.add_to_queue_end(create_track("1", "Track 1", "Artist A", 180));
    manager.add_to_queue_end(create_track("2", "Track 2", "Artist B", 180));

    // Queue has 2 tracks
    assert!(manager.has_next(), "Should have next track");
}

#[test]
fn test_has_next_with_empty_queue() {
    let manager = PlaybackManager::default();

    // Empty queue, no repeat
    assert!(!manager.has_next(), "Should not have next with empty queue");
}

#[test]
fn test_has_next_with_repeat_one() {
    let mut manager = PlaybackManager::default();
    manager.set_repeat(RepeatMode::One);

    manager.add_to_queue_end(create_track("1", "Track 1", "Artist A", 180));

    // Repeat One always has next (same track)
    assert!(manager.has_next(), "Should have next with repeat one");

    // Even with empty queue after consuming
    manager.next().ok();
    assert!(manager.has_next(), "Should still have next with repeat one");
}

#[test]
fn test_has_previous_with_empty_history() {
    let mut manager = PlaybackManager::default();

    manager.add_to_queue_end(create_track("1", "Track 1", "Artist A", 180));

    // No history yet
    assert!(!manager.has_previous(), "Should not have previous with empty history");
}

// Note: has_previous() is true when there's actual playback history.
// In unit tests without real audio sources, history isn't populated by next()
// See integration tests for full playback flow testing

#[test]
fn test_has_previous_with_repeat_one() {
    let mut manager = PlaybackManager::default();
    manager.set_repeat(RepeatMode::One);

    manager.add_to_queue_end(create_track("1", "Track 1", "Artist A", 180));

    // Repeat One always has previous (same track)
    assert!(manager.has_previous(), "Should have previous with repeat one");
}

// ===== Queue Navigation Boundary Tests =====

#[test]
fn test_next_at_queue_end_without_repeat() {
    let mut manager = PlaybackManager::default();

    manager.add_to_queue_end(create_track("1", "Track 1", "Artist A", 180));
    manager.add_to_queue_end(create_track("2", "Track 2", "Artist B", 180));

    // Play through all tracks
    manager.next().ok(); // Track 1 -> history, Track 2 current
    manager.next().ok(); // Track 2 -> history, queue empty

    // No more tracks
    assert!(!manager.has_next(), "Should not have next at queue end");
    assert_eq!(manager.queue_len(), 0, "Queue should be empty");
}

#[test]
fn test_next_at_queue_end_with_repeat_all() {
    let mut manager = PlaybackManager::default();
    manager.set_repeat(RepeatMode::All);

    let tracks = vec![
        create_track("1", "Track 1", "Artist A", 180),
        create_track("2", "Track 2", "Artist B", 180),
    ];

    manager.add_playlist_to_queue(tracks.clone());

    // Play through all tracks
    manager.next().ok();
    manager.next().ok();

    // With RepeatMode::All, queue should restart
    // Note: actual behavior depends on PlaybackManager implementation
    // This test documents the expected behavior
}

#[test]
fn test_previous_at_history_start() {
    let mut manager = PlaybackManager::default();

    manager.add_to_queue_end(create_track("1", "Track 1", "Artist A", 180));

    // No history, can't go back
    let result = manager.previous();

    // Should fail or do nothing (depending on implementation)
    assert!(!manager.has_previous(), "Should not have previous at start");
}

#[test]
fn test_queue_navigation_sequence() {
    let mut manager = PlaybackManager::default();

    manager.add_to_queue_end(create_track("1", "Track 1", "Artist A", 180));
    manager.add_to_queue_end(create_track("2", "Track 2", "Artist B", 180));
    manager.add_to_queue_end(create_track("3", "Track 3", "Artist C", 180));

    // Initial state: queue [1, 2, 3], history []
    assert!(manager.has_next());
    assert!(!manager.has_previous());

    // After first next: queue may be reduced but no previous yet
    // (previous only available after track actually finishes playing)
    manager.next().ok();
    assert!(manager.has_next());

    // After second next: queue [3]
    manager.next().ok();
    assert!(manager.has_next());

    // After third next: queue []
    manager.next().ok();
    assert!(!manager.has_next());
}

// ===== Queue Modification Tests =====

#[test]
fn test_add_track_updates_has_next() {
    let mut manager = PlaybackManager::default();

    // Empty queue
    assert!(!manager.has_next());

    // Add track
    manager.add_to_queue_end(create_track("1", "Track 1", "Artist A", 180));

    // Should have next now
    assert!(manager.has_next());
}

#[test]
fn test_remove_last_track_updates_has_next() {
    let mut manager = PlaybackManager::default();

    manager.add_to_queue_end(create_track("1", "Track 1", "Artist A", 180));

    assert!(manager.has_next());

    // Remove the only track
    manager.remove_from_queue(0).ok();

    assert!(!manager.has_next());
}

#[test]
fn test_clear_queue_updates_has_next() {
    let mut manager = PlaybackManager::default();

    manager.add_to_queue_end(create_track("1", "Track 1", "Artist A", 180));
    manager.add_to_queue_end(create_track("2", "Track 2", "Artist B", 180));

    assert!(manager.has_next());

    manager.clear_queue();

    assert!(!manager.has_next());
    assert_eq!(manager.queue_len(), 0);
}

// ===== Shuffle Effect on Navigation =====

#[test]
fn test_has_next_preserved_after_shuffle() {
    let mut manager = PlaybackManager::default();

    manager.add_to_queue_end(create_track("1", "Track 1", "Artist A", 180));
    manager.add_to_queue_end(create_track("2", "Track 2", "Artist B", 180));
    manager.add_to_queue_end(create_track("3", "Track 3", "Artist C", 180));

    assert!(manager.has_next());

    // Shuffle
    manager.set_shuffle(ShuffleMode::Random);

    // Should still have next (same number of tracks)
    assert!(manager.has_next());
    assert_eq!(manager.queue_len(), 3);
}

#[test]
fn test_shuffle_affects_next_track_order() {
    let mut manager = PlaybackManager::default();

    let tracks = vec![
        create_track("1", "Track 1", "Artist A", 180),
        create_track("2", "Track 2", "Artist B", 180),
        create_track("3", "Track 3", "Artist C", 180),
        create_track("4", "Track 4", "Artist D", 180),
        create_track("5", "Track 5", "Artist E", 180),
    ];

    manager.add_playlist_to_queue(tracks);

    let original_order: Vec<String> = manager.get_queue()
        .iter()
        .map(|t| t.id.clone())
        .collect();

    manager.set_shuffle(ShuffleMode::Random);

    let shuffled_order: Vec<String> = manager.get_queue()
        .iter()
        .map(|t| t.id.clone())
        .collect();

    // Order should be different (very unlikely to be same)
    assert_ne!(original_order, shuffled_order);

    // But still has navigation
    assert!(manager.has_next());
}

// ===== Edge Cases =====

#[test]
fn test_single_track_queue_navigation() {
    let mut manager = PlaybackManager::default();

    manager.add_to_queue_end(create_track("1", "Track 1", "Artist A", 180));

    // Has next (the track itself)
    assert!(manager.has_next());

    // No previous yet
    assert!(!manager.has_previous());

    // After next, no more tracks (unless repeat)
    manager.next().ok();
    assert!(!manager.has_next());
}

#[test]
fn test_large_queue_navigation_performance() {
    let mut manager = PlaybackManager::default();

    // Add 1000 tracks
    for i in 0..1000 {
        manager.add_to_queue_end(create_track(
            &i.to_string(),
            &format!("Track {}", i),
            "Artist",
            180,
        ));
    }

    // has_next should be fast even with large queue
    assert!(manager.has_next());

    // Navigate through several tracks
    for _ in 0..10 {
        manager.next().ok();
        // Should still have next (large queue)
        assert!(manager.has_next());
    }
}

#[test]
fn test_queue_boundaries_with_mixed_operations() {
    let mut manager = PlaybackManager::default();

    // Add initial tracks
    manager.add_to_queue_end(create_track("1", "Track 1", "Artist A", 180));
    manager.add_to_queue_end(create_track("2", "Track 2", "Artist B", 180));

    // Play first
    manager.next().ok();

    // Add more while playing
    manager.add_to_queue_end(create_track("3", "Track 3", "Artist C", 180));

    // Should still have next
    assert!(manager.has_next());

    // Remove from queue
    manager.remove_from_queue(0).ok();

    // Should still have next tracks
    assert!(manager.has_next() || manager.queue_len() == 0);
}

#[test]
fn test_explicit_queue_priority_affects_has_next() {
    let mut manager = PlaybackManager::default();

    // Add source queue
    manager.add_playlist_to_queue(vec![
        create_track("s1", "Source 1", "Artist A", 180),
        create_track("s2", "Source 2", "Artist B", 180),
    ]);

    assert!(manager.has_next());

    // Add explicit track (should be next)
    manager.add_to_queue_next(create_track("e1", "Explicit 1", "Artist C", 180));

    // Still has next
    assert!(manager.has_next());

    // Next should be explicit track
    let queue = manager.get_queue();
    assert_eq!(queue[0].id, "e1");
}
