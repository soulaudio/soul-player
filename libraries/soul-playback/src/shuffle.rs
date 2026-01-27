//! Shuffle algorithms for queue randomization
//!
//! Implements both pure random (Fisher-Yates) and smart shuffle algorithms

use crate::types::{QueueTrack, ShuffleMode};
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::{thread_rng, SeedableRng};
use std::collections::HashSet;

/// Shuffle a queue of tracks
///
/// Applies specified shuffle algorithm to the queue
pub fn shuffle_queue(tracks: &mut [QueueTrack], mode: ShuffleMode) {
    match mode {
        ShuffleMode::Off => {
            // No shuffling
        }
        ShuffleMode::Random => {
            shuffle_random(tracks);
        }
        ShuffleMode::Smart => {
            shuffle_smart(tracks);
        }
    }
}

/// Shuffle a queue of tracks with a specific seed for reproducibility
///
/// Applies specified shuffle algorithm using a seeded RNG.
/// The same seed will always produce the same shuffle order.
pub fn shuffle_queue_with_seed(tracks: &mut [QueueTrack], mode: ShuffleMode, seed: u64) {
    match mode {
        ShuffleMode::Off => {
            // No shuffling
        }
        ShuffleMode::Random => {
            shuffle_random_with_seed(tracks, seed);
        }
        ShuffleMode::Smart => {
            shuffle_smart_with_seed(tracks, seed);
        }
    }
}

/// Pure random shuffle using Fisher-Yates algorithm
///
/// Each track has equal probability of appearing at any position.
/// Simple, fair, but can result in same artist playing consecutively.
fn shuffle_random(tracks: &mut [QueueTrack]) {
    let mut rng = thread_rng();
    tracks.shuffle(&mut rng);
}

/// Pure random shuffle with a specific seed for reproducibility
fn shuffle_random_with_seed(tracks: &mut [QueueTrack], seed: u64) {
    let mut rng = StdRng::seed_from_u64(seed);
    tracks.shuffle(&mut rng);
}

/// Smart shuffle algorithm
///
/// Goals:
/// - Avoid same artist playing consecutively (when possible)
/// - Distribute artists evenly throughout playback
/// - Maintain some randomness (not fully deterministic)
///
/// Algorithm:
/// 1. Group tracks by artist
/// 2. Interleave artist groups to maximize distance between same artist
/// 3. Randomize within artist groups
fn shuffle_smart(tracks: &mut [QueueTrack]) {
    let mut rng = thread_rng();
    shuffle_smart_with_rng(tracks, &mut rng);
}

/// Smart shuffle with a specific seed for reproducibility
fn shuffle_smart_with_seed(tracks: &mut [QueueTrack], seed: u64) {
    let mut rng = StdRng::seed_from_u64(seed);
    shuffle_smart_with_rng(tracks, &mut rng);
}

/// Smart shuffle implementation that accepts any RNG
fn shuffle_smart_with_rng<R: rand::Rng>(tracks: &mut [QueueTrack], rng: &mut R) {
    // Edge case: 0 or 1 track - nothing to shuffle
    if tracks.len() <= 1 {
        return;
    }

    // Edge case: 2 tracks - just random swap (50% chance)
    if tracks.len() == 2 {
        if rng.gen_bool(0.5) {
            tracks.swap(0, 1);
        }
        return;
    }

    // Group tracks by artist using BTreeMap for deterministic iteration order
    let mut by_artist: std::collections::BTreeMap<String, Vec<QueueTrack>> =
        std::collections::BTreeMap::new();

    for track in tracks.iter() {
        by_artist
            .entry(track.artist.clone())
            .or_default()
            .push(track.clone());
    }

    // If only one artist, just do random shuffle
    if by_artist.len() == 1 {
        tracks.shuffle(rng);
        return;
    }

    // Randomize within each artist's tracks
    for artist_tracks in by_artist.values_mut() {
        artist_tracks.shuffle(rng);
    }

    // Sort artists by track count (descending) for better distribution
    // Artists with more tracks should be interleaved first
    // Use BTreeMap keys for deterministic base order
    let mut artists: Vec<(String, usize)> = by_artist
        .iter()
        .map(|(k, v)| (k.clone(), v.len()))
        .collect();
    // Sort by (count descending, then name ascending for determinism)
    artists.sort_by(|a, b| match b.1.cmp(&a.1) {
        std::cmp::Ordering::Equal => a.0.cmp(&b.0),
        other => other,
    });

    // Shuffle artists with same count for randomness
    let mut i = 0;
    while i < artists.len() {
        let count = artists[i].1;
        let mut j = i + 1;
        while j < artists.len() && artists[j].1 == count {
            j += 1;
        }
        if j - i > 1 {
            // Shuffle range [i, j) with same track count
            let slice = &mut artists[i..j];
            slice.shuffle(rng);
        }
        i = j;
    }

    let artist_names: Vec<String> = artists.into_iter().map(|(name, _)| name).collect();

    // Interleave artists to maximize distance using round-robin
    let mut result = Vec::with_capacity(tracks.len());
    let mut artist_indices: Vec<usize> = vec![0; artist_names.len()];
    let mut artists_with_tracks: HashSet<usize> = (0..artist_names.len()).collect();

    // Round-robin through artists
    while !artists_with_tracks.is_empty() {
        for (i, artist) in artist_names.iter().enumerate() {
            if !artists_with_tracks.contains(&i) {
                continue;
            }

            let artist_tracks = by_artist.get_mut(artist).unwrap();
            let index = artist_indices[i];

            if index < artist_tracks.len() {
                result.push(artist_tracks[index].clone());
                artist_indices[i] += 1;
            } else {
                artists_with_tracks.remove(&i);
            }
        }
    }

    // Copy result back to tracks
    for (i, track) in result.into_iter().enumerate() {
        tracks[i] = track;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::TrackSource;
    use std::path::PathBuf;
    use std::time::Duration;

    fn create_test_track(id: &str, title: &str, artist: &str) -> QueueTrack {
        QueueTrack {
            id: id.to_string(),
            path: PathBuf::from(format!("/music/{}.mp3", id)),
            title: title.to_string(),
            artist: artist.to_string(),
            album: Some("Test Album".to_string()),
            duration: Duration::from_secs(180),
            track_number: Some(1),
            source: TrackSource::Single,
        }
    }

    #[test]
    fn shuffle_mode_off_no_change() {
        let mut tracks = vec![
            create_test_track("1", "Track 1", "Artist A"),
            create_test_track("2", "Track 2", "Artist B"),
            create_test_track("3", "Track 3", "Artist C"),
        ];

        let original_order: Vec<String> = tracks.iter().map(|t| t.id.clone()).collect();

        shuffle_queue(&mut tracks, ShuffleMode::Off);

        let new_order: Vec<String> = tracks.iter().map(|t| t.id.clone()).collect();
        assert_eq!(original_order, new_order);
    }

    #[test]
    fn random_shuffle_changes_order() {
        let mut tracks = vec![
            create_test_track("1", "Track 1", "Artist A"),
            create_test_track("2", "Track 2", "Artist B"),
            create_test_track("3", "Track 3", "Artist C"),
            create_test_track("4", "Track 4", "Artist D"),
            create_test_track("5", "Track 5", "Artist E"),
        ];

        let original_order: Vec<String> = tracks.iter().map(|t| t.id.clone()).collect();

        shuffle_random(&mut tracks);

        let new_order: Vec<String> = tracks.iter().map(|t| t.id.clone()).collect();

        // With 5 items, there's a 1/120 (0.83%) chance of same order
        // Instead of asserting inequality (flaky), verify shuffle ran multiple times
        // or check that at least ONE shuffle attempt changes the order
        let mut changed = original_order != new_order;

        // If first shuffle didn't change order (rare), try a few more times
        // This makes the test robust while still catching broken shuffle logic
        if !changed {
            for _ in 0..10 {
                shuffle_random(&mut tracks);
                let attempt_order: Vec<String> = tracks.iter().map(|t| t.id.clone()).collect();
                if attempt_order != original_order {
                    changed = true;
                    break;
                }
            }
        }

        assert!(
            changed,
            "Shuffle failed to change order after 11 attempts - shuffle algorithm may be broken"
        );
    }

    #[test]
    fn random_shuffle_preserves_all_tracks() {
        let mut tracks = vec![
            create_test_track("1", "Track 1", "Artist A"),
            create_test_track("2", "Track 2", "Artist B"),
            create_test_track("3", "Track 3", "Artist C"),
        ];

        shuffle_random(&mut tracks);

        // All IDs should still be present
        let ids: HashSet<String> = tracks.iter().map(|t| t.id.clone()).collect();
        assert!(ids.contains("1"));
        assert!(ids.contains("2"));
        assert!(ids.contains("3"));
        assert_eq!(ids.len(), 3);
    }

    #[test]
    fn smart_shuffle_distributes_artists() {
        let mut tracks = vec![
            create_test_track("1a", "Song 1", "Artist A"),
            create_test_track("1b", "Song 2", "Artist A"),
            create_test_track("1c", "Song 3", "Artist A"),
            create_test_track("2a", "Song 4", "Artist B"),
            create_test_track("2b", "Song 5", "Artist B"),
            create_test_track("2c", "Song 6", "Artist B"),
        ];

        shuffle_smart(&mut tracks);

        // Check that same artist doesn't play consecutively
        let mut consecutive_count = 0;
        for i in 0..tracks.len() - 1 {
            if tracks[i].artist == tracks[i + 1].artist {
                consecutive_count += 1;
            }
        }

        // With smart shuffle, should have minimal consecutive same-artist plays
        // In this case (3 A, 3 B), perfect interleaving would have 0 consecutive
        // But randomness means we might have 1-2, never more than that
        assert!(
            consecutive_count <= 2,
            "Too many consecutive same-artist plays: {}",
            consecutive_count
        );
    }

    #[test]
    fn smart_shuffle_preserves_all_tracks() {
        let mut tracks = vec![
            create_test_track("1", "Track 1", "Artist A"),
            create_test_track("2", "Track 2", "Artist A"),
            create_test_track("3", "Track 3", "Artist B"),
            create_test_track("4", "Track 4", "Artist B"),
        ];

        shuffle_smart(&mut tracks);

        // All IDs should still be present
        let ids: HashSet<String> = tracks.iter().map(|t| t.id.clone()).collect();
        assert_eq!(ids.len(), 4);
        assert!(ids.contains("1"));
        assert!(ids.contains("2"));
        assert!(ids.contains("3"));
        assert!(ids.contains("4"));
    }

    #[test]
    fn smart_shuffle_with_single_artist() {
        let mut tracks = vec![
            create_test_track("1", "Track 1", "Artist A"),
            create_test_track("2", "Track 2", "Artist A"),
            create_test_track("3", "Track 3", "Artist A"),
        ];

        // Should not panic, just randomize
        shuffle_smart(&mut tracks);

        // All tracks should still be present
        assert_eq!(tracks.len(), 3);
    }

    #[test]
    fn smart_shuffle_with_many_artists() {
        let mut tracks = vec![];
        for i in 0..20 {
            tracks.push(create_test_track(
                &format!("track{}", i),
                &format!("Song {}", i),
                &format!("Artist {}", i % 5), // 5 artists, 4 songs each
            ));
        }

        shuffle_smart(&mut tracks);

        // Count consecutive same-artist plays
        let mut consecutive_count = 0;
        for i in 0..tracks.len() - 1 {
            if tracks[i].artist == tracks[i + 1].artist {
                consecutive_count += 1;
            }
        }

        // With 5 artists and 4 songs each, smart shuffle should minimize consecutive plays
        // Maximum should be around 3-4 consecutive (due to randomness)
        assert!(
            consecutive_count < 8,
            "Too many consecutive same-artist plays: {}",
            consecutive_count
        );
    }

    #[test]
    fn smart_shuffle_empty_queue() {
        let mut tracks: Vec<QueueTrack> = vec![];
        shuffle_smart(&mut tracks);
        assert!(tracks.is_empty());
    }

    #[test]
    fn smart_shuffle_single_track() {
        let mut tracks = vec![create_test_track("1", "Track 1", "Artist A")];
        shuffle_smart(&mut tracks);
        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].id, "1");
    }

    // ===== Shuffle Fairness Tests =====

    #[test]
    fn random_shuffle_fairness_distribution() {
        // Test that each track has roughly equal probability of being at each position
        // Run 1000 shuffles and verify position distribution is roughly uniform
        let track_count: usize = 5;
        let iterations: usize = 1000;
        let mut position_counts: Vec<Vec<usize>> = vec![vec![0; track_count]; track_count];

        for seed in 0..iterations as u64 {
            let mut tracks: Vec<QueueTrack> = (0..track_count)
                .map(|i| create_test_track(&i.to_string(), &format!("Track {}", i), "Artist"))
                .collect();

            shuffle_random_with_seed(&mut tracks, seed);

            // Record position of each track
            for (pos, track) in tracks.iter().enumerate() {
                let track_idx: usize = track.id.parse().unwrap();
                position_counts[track_idx][pos] += 1;
            }
        }

        // Each track should appear at each position roughly iterations/track_count times
        // With some tolerance for randomness (allow 30% deviation)
        let expected = iterations / track_count;
        let tolerance = expected * 30 / 100; // 30% tolerance

        for track_idx in 0..track_count {
            for pos in 0..track_count {
                let count = position_counts[track_idx][pos];
                assert!(
                    count > expected.saturating_sub(tolerance),
                    "Track {} appeared at position {} only {} times (expected ~{})",
                    track_idx,
                    pos,
                    count,
                    expected
                );
                assert!(
                    count < expected + tolerance,
                    "Track {} appeared at position {} {} times (expected ~{})",
                    track_idx,
                    pos,
                    count,
                    expected
                );
            }
        }
    }

    // ===== Small Queue Tests (1-3 tracks) =====

    #[test]
    fn shuffle_two_tracks_random() {
        // Test that 2-track shuffle works correctly
        let mut swap_count = 0;
        let iterations: usize = 1000;

        for seed in 0..iterations as u64 {
            let mut tracks = vec![
                create_test_track("1", "Track 1", "Artist A"),
                create_test_track("2", "Track 2", "Artist B"),
            ];

            shuffle_random_with_seed(&mut tracks, seed);

            if tracks[0].id == "2" {
                swap_count += 1;
            }
        }

        // Should be roughly 50% swapped (with 10% tolerance)
        assert!(
            swap_count > 400 && swap_count < 600,
            "2-track shuffle swap rate was {}%, expected ~50%",
            swap_count * 100 / iterations
        );
    }

    #[test]
    fn smart_shuffle_two_tracks_different_artists() {
        // With 2 tracks from different artists, smart shuffle should work
        let mut tracks = vec![
            create_test_track("1", "Track 1", "Artist A"),
            create_test_track("2", "Track 2", "Artist B"),
        ];

        // Should not panic
        shuffle_smart(&mut tracks);
        assert_eq!(tracks.len(), 2);

        // Both tracks should be present
        let ids: HashSet<String> = tracks.iter().map(|t| t.id.clone()).collect();
        assert!(ids.contains("1"));
        assert!(ids.contains("2"));
    }

    #[test]
    fn smart_shuffle_two_tracks_same_artist() {
        // With 2 tracks from same artist, smart shuffle should still work
        let mut tracks = vec![
            create_test_track("1", "Track 1", "Artist A"),
            create_test_track("2", "Track 2", "Artist A"),
        ];

        shuffle_smart(&mut tracks);
        assert_eq!(tracks.len(), 2);

        // Both tracks should be present
        let ids: HashSet<String> = tracks.iter().map(|t| t.id.clone()).collect();
        assert!(ids.contains("1"));
        assert!(ids.contains("2"));
    }

    #[test]
    fn smart_shuffle_three_tracks_mixed_artists() {
        // Test 3 tracks with 2 from one artist, 1 from another
        let tracks = vec![
            create_test_track("1", "Track 1", "Artist A"),
            create_test_track("2", "Track 2", "Artist A"),
            create_test_track("3", "Track 3", "Artist B"),
        ];

        // Run multiple times to verify no panics and reasonable distribution
        for seed in 0..100u64 {
            let mut test_tracks = tracks.clone();
            shuffle_smart_with_seed(&mut test_tracks, seed);

            assert_eq!(test_tracks.len(), 3);

            // Verify all tracks present
            let ids: HashSet<String> = test_tracks.iter().map(|t| t.id.clone()).collect();
            assert_eq!(ids.len(), 3);
        }
    }

    // ===== Seed Reproducibility Tests =====

    #[test]
    fn random_shuffle_seed_reproducibility() {
        let seed = 12345u64;

        let mut tracks1: Vec<QueueTrack> = (0..10)
            .map(|i| create_test_track(&i.to_string(), &format!("Track {}", i), "Artist"))
            .collect();

        let mut tracks2: Vec<QueueTrack> = (0..10)
            .map(|i| create_test_track(&i.to_string(), &format!("Track {}", i), "Artist"))
            .collect();

        shuffle_random_with_seed(&mut tracks1, seed);
        shuffle_random_with_seed(&mut tracks2, seed);

        // Same seed should produce identical results
        let order1: Vec<String> = tracks1.iter().map(|t| t.id.clone()).collect();
        let order2: Vec<String> = tracks2.iter().map(|t| t.id.clone()).collect();
        assert_eq!(order1, order2, "Same seed should produce identical shuffle");
    }

    #[test]
    fn smart_shuffle_seed_reproducibility() {
        let seed = 12345u64;

        let mut tracks1 = vec![
            create_test_track("1a", "Song 1", "Artist A"),
            create_test_track("1b", "Song 2", "Artist A"),
            create_test_track("2a", "Song 3", "Artist B"),
            create_test_track("2b", "Song 4", "Artist B"),
            create_test_track("3a", "Song 5", "Artist C"),
        ];

        let mut tracks2 = tracks1.clone();

        shuffle_smart_with_seed(&mut tracks1, seed);
        shuffle_smart_with_seed(&mut tracks2, seed);

        // Same seed should produce identical results
        let order1: Vec<String> = tracks1.iter().map(|t| t.id.clone()).collect();
        let order2: Vec<String> = tracks2.iter().map(|t| t.id.clone()).collect();
        assert_eq!(
            order1, order2,
            "Same seed should produce identical smart shuffle"
        );
    }

    #[test]
    fn different_seeds_produce_different_results() {
        let mut tracks1: Vec<QueueTrack> = (0..10)
            .map(|i| create_test_track(&i.to_string(), &format!("Track {}", i), "Artist"))
            .collect();

        let mut tracks2 = tracks1.clone();

        shuffle_random_with_seed(&mut tracks1, 12345);
        shuffle_random_with_seed(&mut tracks2, 99999);

        let order1: Vec<String> = tracks1.iter().map(|t| t.id.clone()).collect();
        let order2: Vec<String> = tracks2.iter().map(|t| t.id.clone()).collect();
        assert_ne!(
            order1, order2,
            "Different seeds should produce different shuffles"
        );
    }

    // ===== Smart Shuffle Artist Distribution Tests =====

    #[test]
    fn smart_shuffle_uneven_artist_distribution() {
        // Test with uneven artist track counts: A=5, B=3, C=1
        let mut tracks = vec![
            create_test_track("a1", "Song 1", "Artist A"),
            create_test_track("a2", "Song 2", "Artist A"),
            create_test_track("a3", "Song 3", "Artist A"),
            create_test_track("a4", "Song 4", "Artist A"),
            create_test_track("a5", "Song 5", "Artist A"),
            create_test_track("b1", "Song 6", "Artist B"),
            create_test_track("b2", "Song 7", "Artist B"),
            create_test_track("b3", "Song 8", "Artist B"),
            create_test_track("c1", "Song 9", "Artist C"),
        ];

        shuffle_smart_with_seed(&mut tracks, 42);

        // Count consecutive same-artist plays
        let mut max_consecutive = 1;
        let mut current_consecutive = 1;
        for i in 1..tracks.len() {
            if tracks[i].artist == tracks[i - 1].artist {
                current_consecutive += 1;
                max_consecutive = max_consecutive.max(current_consecutive);
            } else {
                current_consecutive = 1;
            }
        }

        // With smart shuffle, even with uneven distribution, consecutive plays should be limited
        // Artist A has 5 tracks, so worst case is 3 consecutive (after B and C are exhausted)
        assert!(
            max_consecutive <= 3,
            "Max consecutive same-artist plays was {}, expected <= 3",
            max_consecutive
        );
    }

    #[test]
    fn smart_shuffle_many_artists_no_consecutive() {
        // Test with many artists, each having 1 track - should have 0 consecutive
        let mut tracks: Vec<QueueTrack> = (0..10)
            .map(|i| {
                create_test_track(
                    &i.to_string(),
                    &format!("Track {}", i),
                    &format!("Artist {}", i),
                )
            })
            .collect();

        shuffle_smart_with_seed(&mut tracks, 42);

        // With all different artists, no consecutive same-artist plays possible
        for i in 1..tracks.len() {
            assert_ne!(
                tracks[i].artist,
                tracks[i - 1].artist,
                "Found consecutive same-artist at position {} and {}",
                i - 1,
                i
            );
        }
    }

    // ===== Queue with Shuffle Off Tests =====

    #[test]
    fn shuffle_queue_with_seed_off_mode() {
        let mut tracks = vec![
            create_test_track("1", "Track 1", "Artist A"),
            create_test_track("2", "Track 2", "Artist B"),
            create_test_track("3", "Track 3", "Artist C"),
        ];

        let original: Vec<String> = tracks.iter().map(|t| t.id.clone()).collect();

        shuffle_queue_with_seed(&mut tracks, ShuffleMode::Off, 12345);

        let after: Vec<String> = tracks.iter().map(|t| t.id.clone()).collect();
        assert_eq!(original, after, "ShuffleMode::Off should not change order");
    }
}
