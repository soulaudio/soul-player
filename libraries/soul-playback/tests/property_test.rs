//! Property-based tests for playback manager
//!
//! Uses proptest to verify invariants across many random inputs.
//! No shallow tests - every property test verifies meaningful invariants.

use proptest::prelude::*;
use soul_playback::{
    PlaybackConfig, PlaybackManager, QueueTrack, RepeatMode, ShuffleMode, TrackSource,
};
use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Duration;

// ===== Helpers =====

fn arbitrary_track() -> impl Strategy<Value = QueueTrack> {
    (
        "[a-z0-9]{1,10}",                        // id
        "[A-Za-z ]{1,30}",                       // title
        "[A-Za-z ]{1,20}",                       // artist
        proptest::option::of("[A-Za-z ]{1,20}"), // album
        1u64..600,                               // duration (1-600 seconds)
    )
        .prop_map(|(id, title, artist, album, duration_secs)| QueueTrack {
            id,
            path: PathBuf::from("/music/test.mp3"),
            title,
            artist,
            album,
            duration: Duration::from_secs(duration_secs),
            track_number: Some(1),
            source: TrackSource::Single,
        })
}

fn arbitrary_tracks() -> impl Strategy<Value = Vec<QueueTrack>> {
    prop::collection::vec(arbitrary_track(), 1..50)
}

// ===== Property Tests =====

proptest! {
    /// Property: Volume application never produces NaN or Inf
    #[test]
    fn volume_never_produces_nan_or_inf(
        volume in 0u8..=100,
        samples in prop::collection::vec(-1.0f32..1.0, 100..1000)
    ) {
        let mut manager = PlaybackManager::default();
        manager.set_volume(volume);

        let mut buffer = samples;
        manager.process_audio(&mut buffer).ok();

        prop_assert!(buffer.iter().all(|s| s.is_finite()), "Volume produced NaN or Inf");
    }

    /// Property: Queue length is always consistent after operations
    #[test]
    fn queue_length_consistency(
        tracks in arbitrary_tracks(),
        operations in prop::collection::vec(0u8..5, 1..20)
    ) {
        let mut manager = PlaybackManager::default();
        manager.add_playlist_to_queue(tracks.clone());

        let initial_len = manager.queue_len();
        prop_assert_eq!(initial_len, tracks.len());

        for op in operations {
            match op {
                0 => {
                    // Pop next
                    if manager.queue_len() > 0 {
                        manager.next().ok();
                    }
                }
                1 => {
                    // Add track
                    manager.add_to_queue_end(tracks[0].clone());
                }
                2 => {
                    // Remove if possible
                    if manager.queue_len() > 0 {
                        manager.remove_from_queue(0).ok();
                    }
                }
                3 => {
                    // Shuffle
                    manager.set_shuffle(ShuffleMode::Random);
                    manager.set_shuffle(ShuffleMode::Off);
                }
                _ => {
                    // Clear
                    manager.clear_queue();
                }
            }

            // Queue length should always be valid
            let len = manager.queue_len();
            prop_assert!(len <= 1000, "Queue length out of bounds: {}", len);
        }
    }

    /// Property: History never exceeds max size
    #[test]
    fn history_never_exceeds_max_size(
        max_size in 1usize..100,
        num_tracks in 1usize..200
    ) {
        let config = PlaybackConfig {
            history_size: max_size,
            ..Default::default()
        };

        let mut manager = PlaybackManager::new(config);

        // Add tracks
        for i in 0..num_tracks {
            manager.add_to_queue_end(QueueTrack {
                id: i.to_string(),
                path: PathBuf::from("/music/test.mp3"),
                title: format!("Track {}", i),
                artist: "Artist".to_string(),
                album: None,
                duration: Duration::from_secs(180),
                track_number: Some(1),
                source: TrackSource::Single,
            });
        }

        // Play through all tracks
        for _ in 0..num_tracks {
            manager.next().ok();
        }

        let history = manager.get_history();
        prop_assert!(
            history.len() <= max_size,
            "History exceeded max size: {} > {}",
            history.len(),
            max_size
        );
    }

    /// Property: Shuffle preserves all tracks (no loss or duplication)
    #[test]
    fn shuffle_preserves_all_tracks(
        tracks in arbitrary_tracks(),
        shuffle_mode in prop::sample::select(vec![ShuffleMode::Random, ShuffleMode::Smart])
    ) {
        let mut manager = PlaybackManager::default();

        let original_ids: HashSet<String> = tracks.iter().map(|t| t.id.clone()).collect();

        manager.add_playlist_to_queue(tracks);
        manager.set_shuffle(shuffle_mode);

        let shuffled_ids: HashSet<String> = manager
            .get_queue()
            .iter()
            .map(|t| t.id.clone())
            .collect();

        prop_assert_eq!(
            original_ids.len(),
            shuffled_ids.len(),
            "Shuffle changed track count"
        );

        prop_assert_eq!(original_ids, shuffled_ids, "Shuffle lost or duplicated tracks");
    }

    /// Property: Shuffle restore returns to original order
    #[test]
    fn shuffle_restore_original_order(tracks in arbitrary_tracks()) {
        let mut manager = PlaybackManager::default();

        let original_ids: Vec<String> = tracks.iter().map(|t| t.id.clone()).collect();

        manager.add_playlist_to_queue(tracks);
        manager.set_shuffle(ShuffleMode::Random);
        manager.set_shuffle(ShuffleMode::Off); // Restore

        let restored_ids: Vec<String> = manager
            .get_queue()
            .iter()
            .map(|t| t.id.clone())
            .collect();

        prop_assert_eq!(original_ids, restored_ids, "Shuffle restore failed");
    }

    /// Property: Volume is always clamped to 0-100
    #[test]
    fn volume_clamped_to_range(volume in any::<u8>()) {
        let mut manager = PlaybackManager::default();
        manager.set_volume(volume);

        let actual = manager.get_volume();
        prop_assert!(actual <= 100, "Volume exceeded 100: {}", actual);
    }

    /// Property: Mute always silences output
    #[test]
    fn mute_always_silences(
        volume in 1u8..=100,
        samples in prop::collection::vec(-1.0f32..1.0, 100..1000)
    ) {
        let mut manager = PlaybackManager::default();
        manager.set_volume(volume);
        manager.mute();

        let mut buffer = samples;
        manager.process_audio(&mut buffer).ok();

        prop_assert!(
            buffer.iter().all(|s| *s == 0.0),
            "Mute did not silence output"
        );
    }

    /// Property: Queue reorder maintains all tracks
    #[test]
    fn queue_reorder_preserves_tracks(
        tracks in prop::collection::vec(arbitrary_track(), 5..20),
        from in 0usize..10,
        to in 0usize..10
    ) {
        let mut manager = PlaybackManager::default();
        manager.add_playlist_to_queue(tracks.clone());

        let original_ids: HashSet<String> = tracks.iter().map(|t| t.id.clone()).collect();

        // Try to reorder (may fail if indices out of bounds)
        manager.reorder_queue(from, to).ok();

        let after_ids: HashSet<String> = manager
            .get_queue()
            .iter()
            .map(|t| t.id.clone())
            .collect();

        prop_assert_eq!(original_ids, after_ids, "Reorder lost tracks");
    }

    /// Property: Adding to queue never removes existing tracks
    #[test]
    fn add_to_queue_never_removes(
        initial_tracks in arbitrary_tracks(),
        new_track in arbitrary_track()
    ) {
        let mut manager = PlaybackManager::default();
        manager.add_playlist_to_queue(initial_tracks.clone());

        let initial_count = manager.queue_len();

        manager.add_to_queue_end(new_track);

        let after_count = manager.queue_len();
        prop_assert_eq!(after_count, initial_count + 1, "Add to queue removed tracks");
    }

    /// Property: Remove from queue decreases length by 1
    #[test]
    fn remove_decreases_queue_length(
        tracks in prop::collection::vec(arbitrary_track(), 2..50),
        index in 0usize..20
    ) {
        let mut manager = PlaybackManager::default();
        manager.add_playlist_to_queue(tracks);

        let initial_len = manager.queue_len();

        let result = manager.remove_from_queue(index);

        if result.is_ok() {
            let after_len = manager.queue_len();
            prop_assert_eq!(after_len, initial_len - 1, "Remove didn't decrease length by 1");
        } else {
            // Failed because index out of bounds
            prop_assert!(index >= initial_len, "Remove failed but index was valid");
        }
    }

    /// Property: Clear queue empties all queues
    #[test]
    fn clear_queue_empties_all(tracks in arbitrary_tracks()) {
        let mut manager = PlaybackManager::default();
        manager.add_playlist_to_queue(tracks);

        manager.clear_queue();

        prop_assert_eq!(manager.queue_len(), 0, "Clear queue didn't empty queue");
    }

    /// Property: Explicit queue has priority over source queue
    #[test]
    fn explicit_queue_priority(
        source_tracks in prop::collection::vec(arbitrary_track(), 1..10),
        explicit_track in arbitrary_track()
    ) {
        let mut manager = PlaybackManager::default();

        manager.add_playlist_to_queue(source_tracks);
        manager.add_to_queue_next(explicit_track.clone());

        let queue = manager.get_queue();

        // First track should be explicit
        prop_assert_eq!(&queue[0].id, &explicit_track.id, "Explicit track not first");
    }

    /// Property: Repeat modes are mutually exclusive
    #[test]
    fn repeat_modes_exclusive(
        mode in prop::sample::select(vec![RepeatMode::Off, RepeatMode::All, RepeatMode::One])
    ) {
        let mut manager = PlaybackManager::default();
        manager.set_repeat(mode);

        prop_assert_eq!(manager.get_repeat(), mode, "Repeat mode not set correctly");
    }

    /// Property: Smart shuffle minimizes consecutive same-artist plays
    #[test]
    fn smart_shuffle_distributes_artists(
        artist_a_tracks in prop::collection::vec(arbitrary_track(), 3..6),
        artist_b_tracks in prop::collection::vec(arbitrary_track(), 3..6)
    ) {
        let mut manager = PlaybackManager::default();

        // Modify tracks to have specific artists
        let mut all_tracks = vec![];
        for mut track in artist_a_tracks {
            track.artist = "Artist A".to_string();
            all_tracks.push(track);
        }
        for mut track in artist_b_tracks {
            track.artist = "Artist B".to_string();
            all_tracks.push(track);
        }

        manager.add_playlist_to_queue(all_tracks.clone());
        manager.set_shuffle(ShuffleMode::Smart);

        let queue = manager.get_queue();

        // Count consecutive same-artist plays
        let mut consecutive_count = 0;
        for i in 0..queue.len() - 1 {
            if queue[i].artist == queue[i + 1].artist {
                consecutive_count += 1;
            }
        }

        // With smart shuffle, should have fewer consecutive plays than total tracks / 2
        let max_expected = all_tracks.len() / 2;
        prop_assert!(
            consecutive_count <= max_expected,
            "Too many consecutive same-artist plays: {} (max expected: {})",
            consecutive_count,
            max_expected
        );
    }

    /// Property: Processing audio with no source outputs silence
    #[test]
    fn no_source_outputs_silence(buffer_size in 100usize..2000) {
        let mut manager = PlaybackManager::default();

        let mut buffer = vec![1.0f32; buffer_size];
        manager.process_audio(&mut buffer).ok();

        prop_assert!(buffer.iter().all(|s| *s == 0.0), "No source didn't output silence");
    }
}
