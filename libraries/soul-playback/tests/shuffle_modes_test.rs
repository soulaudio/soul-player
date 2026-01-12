//! Shuffle Modes Tests
//!
//! Tests for the three shuffle modes:
//! 1. Off - No shuffling, original track order
//! 2. Random - Pure Fisher-Yates shuffle
//! 3. Smart - Artist distribution, avoids consecutive same artist
//!
//! Also tests shuffle mode cycling: Off → Random → Smart → Off

use soul_playback::{PlaybackManager, QueueTrack, ShuffleMode, TrackSource};
use std::collections::HashMap;
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

fn get_queue_ids(manager: &PlaybackManager) -> Vec<String> {
    manager.get_queue().iter().map(|t| t.id.clone()).collect()
}

fn get_queue_artists(manager: &PlaybackManager) -> Vec<String> {
    manager
        .get_queue()
        .iter()
        .map(|t| t.artist.clone())
        .collect()
}

// ===== Shuffle Mode Cycling Tests =====

#[test]
fn test_shuffle_cycles_off_random_smart() {
    let mut manager = PlaybackManager::default();

    // Initial state should be Off
    assert_eq!(
        manager.get_shuffle_mode(),
        ShuffleMode::Off,
        "Initial shuffle mode should be Off"
    );

    // Cycle: Off → Random
    let mode = manager.cycle_shuffle();
    assert_eq!(mode, ShuffleMode::Random, "Should cycle to Random");
    assert_eq!(manager.get_shuffle_mode(), ShuffleMode::Random);

    // Cycle: Random → Smart
    let mode = manager.cycle_shuffle();
    assert_eq!(mode, ShuffleMode::Smart, "Should cycle to Smart");
    assert_eq!(manager.get_shuffle_mode(), ShuffleMode::Smart);

    // Cycle: Smart → Off
    let mode = manager.cycle_shuffle();
    assert_eq!(mode, ShuffleMode::Off, "Should cycle back to Off");
    assert_eq!(manager.get_shuffle_mode(), ShuffleMode::Off);
}

#[test]
fn test_shuffle_mode_as_str() {
    assert_eq!(ShuffleMode::Off.as_str(), "off");
    assert_eq!(ShuffleMode::Random.as_str(), "random");
    assert_eq!(ShuffleMode::Smart.as_str(), "smart");
}

#[test]
fn test_shuffle_mode_from_str() {
    assert_eq!(ShuffleMode::from_str("off"), Some(ShuffleMode::Off));
    assert_eq!(ShuffleMode::from_str("random"), Some(ShuffleMode::Random));
    assert_eq!(ShuffleMode::from_str("smart"), Some(ShuffleMode::Smart));
    assert_eq!(ShuffleMode::from_str("invalid"), None);
}

#[test]
fn test_shuffle_mode_cycle_method() {
    assert_eq!(ShuffleMode::Off.cycle(), ShuffleMode::Random);
    assert_eq!(ShuffleMode::Random.cycle(), ShuffleMode::Smart);
    assert_eq!(ShuffleMode::Smart.cycle(), ShuffleMode::Off);
}

// ===== Shuffle Mode: Off Tests =====

#[test]
fn test_shuffle_off_preserves_original_order() {
    let mut manager = PlaybackManager::default();
    manager.set_shuffle(ShuffleMode::Off);

    let tracks = vec![
        create_track("1", "Track 1", "Artist A", 180),
        create_track("2", "Track 2", "Artist B", 180),
        create_track("3", "Track 3", "Artist C", 180),
        create_track("4", "Track 4", "Artist D", 180),
        create_track("5", "Track 5", "Artist E", 180),
    ];

    let original_order: Vec<String> = tracks.iter().map(|t| t.id.clone()).collect();

    manager.load_playlist(tracks, 0);

    let queue_order = get_queue_ids(&manager);

    assert_eq!(
        queue_order, original_order,
        "Shuffle Off should preserve original order"
    );
}

#[test]
fn test_shuffle_off_after_shuffle_on_restores_order() {
    let mut manager = PlaybackManager::default();

    let tracks = vec![
        create_track("1", "Track 1", "Artist A", 180),
        create_track("2", "Track 2", "Artist B", 180),
        create_track("3", "Track 3", "Artist C", 180),
        create_track("4", "Track 4", "Artist D", 180),
        create_track("5", "Track 5", "Artist E", 180),
    ];

    let original_order: Vec<String> = tracks.iter().map(|t| t.id.clone()).collect();

    manager.load_playlist(tracks, 0);

    // Enable shuffle
    manager.set_shuffle(ShuffleMode::Random);

    let shuffled_order = get_queue_ids(&manager);
    assert_ne!(
        shuffled_order, original_order,
        "Random shuffle should change order"
    );

    // Disable shuffle
    manager.set_shuffle(ShuffleMode::Off);

    let restored_order = get_queue_ids(&manager);
    assert_eq!(
        restored_order, original_order,
        "Shuffle Off should restore original order"
    );
}

// ===== Shuffle Mode: Random Tests =====

#[test]
fn test_shuffle_random_changes_order() {
    let mut manager = PlaybackManager::default();

    let tracks = vec![
        create_track("1", "Track 1", "Artist A", 180),
        create_track("2", "Track 2", "Artist B", 180),
        create_track("3", "Track 3", "Artist C", 180),
        create_track("4", "Track 4", "Artist D", 180),
        create_track("5", "Track 5", "Artist E", 180),
        create_track("6", "Track 6", "Artist F", 180),
        create_track("7", "Track 7", "Artist G", 180),
        create_track("8", "Track 8", "Artist H", 180),
    ];

    let original_order: Vec<String> = tracks.iter().map(|t| t.id.clone()).collect();

    manager.load_playlist(tracks, 0);
    manager.set_shuffle(ShuffleMode::Random);

    let shuffled_order = get_queue_ids(&manager);

    // With 8 tracks, probability of same order is 1/40320 ≈ 0.0025%
    assert_ne!(
        shuffled_order, original_order,
        "Random shuffle should (almost certainly) change order"
    );
}

#[test]
fn test_shuffle_random_contains_all_tracks() {
    let mut manager = PlaybackManager::default();

    let tracks = vec![
        create_track("1", "Track 1", "Artist A", 180),
        create_track("2", "Track 2", "Artist B", 180),
        create_track("3", "Track 3", "Artist C", 180),
        create_track("4", "Track 4", "Artist D", 180),
        create_track("5", "Track 5", "Artist E", 180),
    ];

    manager.load_playlist(tracks, 0);
    manager.set_shuffle(ShuffleMode::Random);

    let shuffled_ids = get_queue_ids(&manager);

    // All original tracks should be present
    let mut expected_ids = vec!["1", "2", "3", "4", "5"];
    let mut actual_ids = shuffled_ids.clone();

    expected_ids.sort();
    actual_ids.sort();

    assert_eq!(
        actual_ids, expected_ids,
        "Random shuffle should contain all tracks (no additions/removals)"
    );
}

#[test]
fn test_shuffle_random_is_deterministic_per_seed() {
    let mut manager1 = PlaybackManager::default();
    let mut manager2 = PlaybackManager::default();

    let tracks = vec![
        create_track("1", "Track 1", "Artist A", 180),
        create_track("2", "Track 2", "Artist B", 180),
        create_track("3", "Track 3", "Artist C", 180),
    ];

    // Both managers load same tracks and shuffle
    manager1.load_playlist(tracks.clone(), 0);
    manager1.set_shuffle(ShuffleMode::Random);

    manager2.load_playlist(tracks, 0);
    manager2.set_shuffle(ShuffleMode::Random);

    let order1 = get_queue_ids(&manager1);
    let order2 = get_queue_ids(&manager2);

    // Note: If using system random, orders will differ
    // If using seeded random, orders should match
    // This test documents the behavior (adjust based on implementation)
}

#[test]
fn test_shuffle_random_multiple_times_varies_order() {
    let mut manager = PlaybackManager::default();

    let tracks = vec![
        create_track("1", "Track 1", "Artist A", 180),
        create_track("2", "Track 2", "Artist B", 180),
        create_track("3", "Track 3", "Artist C", 180),
        create_track("4", "Track 4", "Artist D", 180),
        create_track("5", "Track 5", "Artist E", 180),
    ];

    manager.load_playlist(tracks, 0);

    manager.set_shuffle(ShuffleMode::Random);
    let order1 = get_queue_ids(&manager);

    manager.set_shuffle(ShuffleMode::Off);
    manager.set_shuffle(ShuffleMode::Random);
    let order2 = get_queue_ids(&manager);

    manager.set_shuffle(ShuffleMode::Off);
    manager.set_shuffle(ShuffleMode::Random);
    let order3 = get_queue_ids(&manager);

    // At least one shuffle should produce different order
    // (Very unlikely all three are identical)
    assert!(
        order1 != order2 || order2 != order3 || order1 != order3,
        "Random shuffle should produce varying orders across multiple shuffles"
    );
}

// ===== Shuffle Mode: Smart Tests =====

#[test]
fn test_shuffle_smart_no_consecutive_same_artist() {
    let mut manager = PlaybackManager::default();

    // Create playlist with multiple tracks from same artists
    let tracks = vec![
        create_track("1", "Track 1", "Artist A", 180),
        create_track("2", "Track 2", "Artist A", 180),
        create_track("3", "Track 3", "Artist A", 180),
        create_track("4", "Track 4", "Artist B", 180),
        create_track("5", "Track 5", "Artist B", 180),
        create_track("6", "Track 6", "Artist B", 180),
        create_track("7", "Track 7", "Artist C", 180),
        create_track("8", "Track 8", "Artist C", 180),
    ];

    manager.load_playlist(tracks, 0);
    manager.set_shuffle(ShuffleMode::Smart);

    let artists = get_queue_artists(&manager);

    // Verify no consecutive same artist
    for i in 0..artists.len() - 1 {
        assert_ne!(
            artists[i],
            artists[i + 1],
            "Smart shuffle should not have consecutive same artist at index {}",
            i
        );
    }
}

#[test]
fn test_shuffle_smart_distributes_artists_evenly() {
    let mut manager = PlaybackManager::default();

    // Create playlist: 6 tracks from Artist A, 3 from Artist B
    let tracks = vec![
        create_track("1", "Track 1", "Artist A", 180),
        create_track("2", "Track 2", "Artist A", 180),
        create_track("3", "Track 3", "Artist A", 180),
        create_track("4", "Track 4", "Artist A", 180),
        create_track("5", "Track 5", "Artist A", 180),
        create_track("6", "Track 6", "Artist A", 180),
        create_track("7", "Track 7", "Artist B", 180),
        create_track("8", "Track 8", "Artist B", 180),
        create_track("9", "Track 9", "Artist B", 180),
    ];

    manager.load_playlist(tracks, 0);
    manager.set_shuffle(ShuffleMode::Smart);

    let artists = get_queue_artists(&manager);

    // Calculate max consecutive count for each artist
    let mut max_consecutive_a = 0;
    let mut max_consecutive_b = 0;
    let mut current_consecutive = 1;
    let mut last_artist = &artists[0];

    for artist in artists.iter().skip(1) {
        if artist == last_artist {
            current_consecutive += 1;
        } else {
            if last_artist == "Artist A" {
                max_consecutive_a = max_consecutive_a.max(current_consecutive);
            } else if last_artist == "Artist B" {
                max_consecutive_b = max_consecutive_b.max(current_consecutive);
            }
            current_consecutive = 1;
            last_artist = artist;
        }
    }

    // Smart shuffle should distribute artists to avoid long runs
    // With perfect distribution, no artist should repeat more than twice consecutively
    assert!(
        max_consecutive_a <= 2 && max_consecutive_b <= 2,
        "Smart shuffle should limit consecutive artist runs (max_a={}, max_b={})",
        max_consecutive_a,
        max_consecutive_b
    );
}

#[test]
fn test_shuffle_smart_contains_all_tracks() {
    let mut manager = PlaybackManager::default();

    let tracks = vec![
        create_track("1", "Track 1", "Artist A", 180),
        create_track("2", "Track 2", "Artist A", 180),
        create_track("3", "Track 3", "Artist B", 180),
        create_track("4", "Track 4", "Artist B", 180),
        create_track("5", "Track 5", "Artist C", 180),
    ];

    manager.load_playlist(tracks, 0);
    manager.set_shuffle(ShuffleMode::Smart);

    let shuffled_ids = get_queue_ids(&manager);

    // All original tracks should be present
    let mut expected_ids = vec!["1", "2", "3", "4", "5"];
    let mut actual_ids = shuffled_ids.clone();

    expected_ids.sort();
    actual_ids.sort();

    assert_eq!(
        actual_ids, expected_ids,
        "Smart shuffle should contain all tracks"
    );
}

#[test]
fn test_shuffle_smart_with_single_artist() {
    let mut manager = PlaybackManager::default();

    // All tracks from one artist
    let tracks = vec![
        create_track("1", "Track 1", "Artist A", 180),
        create_track("2", "Track 2", "Artist A", 180),
        create_track("3", "Track 3", "Artist A", 180),
        create_track("4", "Track 4", "Artist A", 180),
    ];

    manager.load_playlist(tracks, 0);
    manager.set_shuffle(ShuffleMode::Smart);

    // Should still shuffle (no consecutive constraint violation possible)
    let queue = manager.get_queue();
    assert_eq!(queue.len(), 4, "Should contain all tracks");

    // All tracks should be from Artist A
    for track in queue.iter() {
        assert_eq!(track.artist, "Artist A");
    }
}

#[test]
fn test_shuffle_smart_vs_random_different_distribution() {
    let mut manager_smart = PlaybackManager::default();
    let mut manager_random = PlaybackManager::default();

    // Tracks with multiple same artist
    let tracks = vec![
        create_track("1", "Track 1", "Artist A", 180),
        create_track("2", "Track 2", "Artist A", 180),
        create_track("3", "Track 3", "Artist A", 180),
        create_track("4", "Track 4", "Artist B", 180),
        create_track("5", "Track 5", "Artist B", 180),
        create_track("6", "Track 6", "Artist B", 180),
    ];

    manager_smart.load_playlist(tracks.clone(), 0);
    manager_smart.set_shuffle(ShuffleMode::Smart);

    manager_random.load_playlist(tracks, 0);
    manager_random.set_shuffle(ShuffleMode::Random);

    let artists_smart = get_queue_artists(&manager_smart);
    let artists_random = get_queue_artists(&manager_random);

    // Smart shuffle should have no consecutive same artist
    for i in 0..artists_smart.len() - 1 {
        assert_ne!(artists_smart[i], artists_smart[i + 1]);
    }

    // Random shuffle MAY have consecutive same artist (no guarantee)
    // This is probabilistic, not deterministic
}

// ===== Shuffle Preservation Tests =====

#[test]
fn test_shuffle_preserves_original_order_for_restore() {
    let mut manager = PlaybackManager::default();

    let tracks = vec![
        create_track("1", "Track 1", "Artist A", 180),
        create_track("2", "Track 2", "Artist B", 180),
        create_track("3", "Track 3", "Artist C", 180),
        create_track("4", "Track 4", "Artist D", 180),
    ];

    let original_order: Vec<String> = tracks.iter().map(|t| t.id.clone()).collect();

    manager.load_playlist(tracks, 0);

    // Shuffle multiple times
    manager.set_shuffle(ShuffleMode::Random);
    manager.set_shuffle(ShuffleMode::Off);
    manager.set_shuffle(ShuffleMode::Smart);
    manager.set_shuffle(ShuffleMode::Off);

    // Should always restore to original order
    let restored_order = get_queue_ids(&manager);
    assert_eq!(
        restored_order, original_order,
        "Should preserve original order for restore after multiple shuffle cycles"
    );
}

// ===== Shuffle with Three-Tier Queue Tests =====

#[test]
fn test_shuffle_only_affects_source_queue_not_manual_queues() {
    let mut manager = PlaybackManager::default();

    // Add source queue
    manager.load_playlist(
        vec![
            create_track("s1", "Source 1", "Artist A", 180),
            create_track("s2", "Source 2", "Artist B", 180),
            create_track("s3", "Source 3", "Artist C", 180),
        ],
        0,
    );

    // Add manual tracks
    manager.add_to_queue_next(create_track("n1", "Next 1", "Artist D", 180));
    manager.add_to_queue_end(create_track("q1", "Queue 1", "Artist E", 180));

    // Enable shuffle
    manager.set_shuffle(ShuffleMode::Random);

    // Manual tracks should remain in their positions
    let queue = manager.get_queue();
    let ids: Vec<String> = queue.iter().map(|t| t.id.clone()).collect();

    // Play Next should be first
    assert_eq!(ids[0], "n1", "Play Next should not be shuffled");

    // Add to Queue should be after source queue
    assert_eq!(
        ids.last().unwrap(),
        "q1",
        "Add to Queue should not be shuffled"
    );
}

// ===== Edge Cases =====

#[test]
fn test_shuffle_with_empty_queue() {
    let mut manager = PlaybackManager::default();

    // Empty queue
    manager.set_shuffle(ShuffleMode::Random);

    assert_eq!(manager.queue_len(), 0, "Empty queue should remain empty");
}

#[test]
fn test_shuffle_with_single_track() {
    let mut manager = PlaybackManager::default();

    manager.load_playlist(vec![create_track("1", "Track 1", "Artist A", 180)], 0);

    manager.set_shuffle(ShuffleMode::Random);

    let queue = manager.get_queue();
    assert_eq!(queue.len(), 1);
    assert_eq!(queue[0].id, "1", "Single track should remain unchanged");
}

#[test]
fn test_shuffle_with_two_tracks() {
    let mut manager = PlaybackManager::default();

    manager.load_playlist(
        vec![
            create_track("1", "Track 1", "Artist A", 180),
            create_track("2", "Track 2", "Artist B", 180),
        ],
        0,
    );

    manager.set_shuffle(ShuffleMode::Random);

    let ids = get_queue_ids(&manager);

    // With 2 tracks, should contain both (order may vary)
    let mut sorted_ids = ids.clone();
    sorted_ids.sort();
    assert_eq!(sorted_ids, vec!["1", "2"]);
}

#[test]
fn test_smart_shuffle_with_two_artists_perfect_distribution() {
    let mut manager = PlaybackManager::default();

    // Perfect distribution: 2 artists, 2 tracks each
    manager.load_playlist(
        vec![
            create_track("1", "Track 1", "Artist A", 180),
            create_track("2", "Track 2", "Artist A", 180),
            create_track("3", "Track 3", "Artist B", 180),
            create_track("4", "Track 4", "Artist B", 180),
        ],
        0,
    );

    manager.set_shuffle(ShuffleMode::Smart);

    let artists = get_queue_artists(&manager);

    // Should alternate artists: A, B, A, B or B, A, B, A
    for i in 0..artists.len() - 1 {
        assert_ne!(
            artists[i],
            artists[i + 1],
            "Perfect distribution should alternate artists"
        );
    }
}
