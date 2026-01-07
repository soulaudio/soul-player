//! Shuffle algorithms for queue randomization
//!
//! Implements both pure random (Fisher-Yates) and smart shuffle algorithms

use crate::types::{QueueTrack, ShuffleMode};
use rand::seq::SliceRandom;
use rand::thread_rng;
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

/// Pure random shuffle using Fisher-Yates algorithm
///
/// Each track has equal probability of appearing at any position.
/// Simple, fair, but can result in same artist playing consecutively.
fn shuffle_random(tracks: &mut [QueueTrack]) {
    let mut rng = thread_rng();
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
    if tracks.len() <= 2 {
        // Not enough tracks for smart shuffling
        shuffle_random(tracks);
        return;
    }

    let mut rng = thread_rng();

    // Group tracks by artist
    let mut by_artist: std::collections::HashMap<String, Vec<QueueTrack>> =
        std::collections::HashMap::new();

    for track in tracks.iter() {
        by_artist
            .entry(track.artist.clone())
            .or_default()
            .push(track.clone());
    }

    // Randomize within each artist's tracks
    for artist_tracks in by_artist.values_mut() {
        artist_tracks.shuffle(&mut rng);
    }

    // Get artist keys and shuffle them
    let mut artists: Vec<String> = by_artist.keys().cloned().collect();
    artists.shuffle(&mut rng);

    // Interleave artists to maximize distance
    let mut result = Vec::with_capacity(tracks.len());
    let mut artist_indices: Vec<usize> = vec![0; artists.len()];
    let mut artists_with_tracks: HashSet<usize> = (0..artists.len()).collect();

    // Round-robin through artists
    while !artists_with_tracks.is_empty() {
        for (i, artist) in artists.iter().enumerate() {
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

        // Very unlikely to be in same order (probability: 1/120)
        // If this fails occasionally, it's just bad luck, not a bug
        assert_ne!(original_order, new_order);
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
}
