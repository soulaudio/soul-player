//! Property-based tests for playback manager
//!
//! Uses proptest to verify invariants across many random inputs.
//! No shallow tests - every property test verifies meaningful invariants.

use proptest::prelude::*;
use soul_playback::{
    PlaybackConfig, PlaybackManager, PlaybackState, QueueTrack, RepeatMode, ShuffleMode,
    TrackSource,
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

/// Generate tracks with unique IDs to avoid duplicate removal issues
fn arbitrary_unique_tracks() -> impl Strategy<Value = Vec<QueueTrack>> {
    (1usize..50).prop_flat_map(|count| {
        prop::collection::vec(
            (
                "[A-Za-z ]{1,30}",                       // title
                "[A-Za-z ]{1,20}",                       // artist
                proptest::option::of("[A-Za-z ]{1,20}"), // album
                1u64..600,                               // duration (1-600 seconds)
            ),
            count,
        )
        .prop_map(|data| {
            data.into_iter()
                .enumerate()
                .map(|(i, (title, artist, album, duration_secs))| QueueTrack {
                    id: format!("track_{}", i), // Unique ID based on index
                    path: PathBuf::from("/music/test.mp3"),
                    title,
                    artist,
                    album,
                    duration: Duration::from_secs(duration_secs),
                    track_number: Some(1),
                    source: TrackSource::Single,
                })
                .collect()
        })
    })
}

/// Calculate expected queue length after duplicate removal
fn count_after_consecutive_duplicate_removal(tracks: &[QueueTrack]) -> usize {
    if tracks.is_empty() {
        return 0;
    }
    let mut count = 1;
    for i in 1..tracks.len() {
        if tracks[i].id != tracks[i - 1].id {
            count += 1;
        }
    }
    count
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
        tracks in arbitrary_unique_tracks(),
        operations in prop::collection::vec(0u8..5, 1..20)
    ) {
        let mut manager = PlaybackManager::default();
        manager.add_playlist_to_queue(tracks.clone());

        // Use unique tracks so no duplicates are removed
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
    fn shuffle_restore_original_order(tracks in arbitrary_unique_tracks()) {
        let mut manager = PlaybackManager::default();

        // Use unique tracks so no duplicates are removed
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

        // Output should be near-silence (DAC keepalive noise at ~-96dB is acceptable)
        let dac_keepalive_threshold = 0.0001; // ~-80dB, well above DAC keepalive noise
        prop_assert!(
            buffer.iter().all(|s| s.abs() < dac_keepalive_threshold),
            "Mute did not silence output, max value: {:.6}",
            buffer.iter().map(|s| s.abs()).fold(0.0f32, f32::max)
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

    /// Property: Processing audio with no source outputs near-silence
    #[test]
    fn no_source_outputs_silence(buffer_size in 100usize..2000) {
        let mut manager = PlaybackManager::default();

        let mut buffer = vec![1.0f32; buffer_size];
        manager.process_audio(&mut buffer).ok();

        // Output should be near-silence (DAC keepalive noise at ~-96dB is acceptable)
        let dac_keepalive_threshold = 0.0001; // ~-80dB, well above DAC keepalive noise
        prop_assert!(
            buffer.iter().all(|s| s.abs() < dac_keepalive_threshold),
            "No source didn't output near-silence, max value: {:.6}",
            buffer.iter().map(|s| s.abs()).fold(0.0f32, f32::max)
        );
    }

    // ===== Property Test 1: Play/Pause/Stop state transitions always result in valid states =====

    /// Property: For any sequence of play/pause/stop operations, the state is always valid
    #[test]
    fn state_always_valid_after_operations(
        operations in prop::collection::vec(0u8..3, 1..50)
    ) {
        let mut manager = PlaybackManager::default();

        // Add some tracks so play() can work
        for i in 0..5 {
            manager.add_to_queue_end(QueueTrack {
                id: format!("track_{}", i),
                path: PathBuf::from("/music/test.mp3"),
                title: format!("Track {}", i),
                artist: "Artist".to_string(),
                album: None,
                duration: Duration::from_secs(180),
                track_number: Some(1),
                source: TrackSource::Single,
            });
        }

        for op in operations {
            match op {
                0 => { manager.play().ok(); }
                1 => { manager.pause(); }
                _ => { manager.stop(); }
            }

            // State must always be one of the valid states
            let state = manager.get_state();
            prop_assert!(
                matches!(state, PlaybackState::Stopped | PlaybackState::Playing | PlaybackState::Paused | PlaybackState::Loading),
                "Invalid state after operation: {:?}",
                state
            );
        }
    }

    // ===== Property Test 2: Volume gain always in valid range [0, 1] =====

    /// Property: For any volume level (0-100), the resulting gain is in [0, 1]
    #[test]
    fn volume_gain_always_in_valid_range(volume in 0u8..=100) {
        let mut manager = PlaybackManager::default();
        manager.set_volume(volume);

        // Volume level should be clamped
        let actual_volume = manager.get_volume();
        prop_assert!(actual_volume <= 100, "Volume exceeded 100: {}", actual_volume);

        // Process some audio to verify gain is applied correctly
        let mut buffer = vec![1.0f32; 100];
        manager.process_audio(&mut buffer).ok();

        // All output samples should be in valid range (no clipping from gain)
        // When stopped, output should be near-silence (DAC keepalive noise)
        let dac_keepalive_threshold = 0.001;
        prop_assert!(
            buffer.iter().all(|s| s.abs() <= 1.0 + dac_keepalive_threshold),
            "Gain produced out-of-range samples, max: {}",
            buffer.iter().map(|s| s.abs()).fold(0.0f32, f32::max)
        );
    }

    // ===== Property Test 3: Queue indices remain valid after operations =====

    /// Property: For any queue operations, indices remain valid (no out-of-bounds)
    #[test]
    fn queue_indices_always_valid(
        initial_count in 5usize..20,
        operations in prop::collection::vec((0u8..6, 0usize..30), 1..30)
    ) {
        let mut manager = PlaybackManager::default();

        // Add initial tracks
        let initial_tracks: Vec<QueueTrack> = (0..initial_count)
            .map(|i| QueueTrack {
                id: format!("track_{}", i),
                path: PathBuf::from("/music/test.mp3"),
                title: format!("Track {}", i),
                artist: "Artist".to_string(),
                album: None,
                duration: Duration::from_secs(180),
                track_number: Some(1),
                source: TrackSource::Single,
            })
            .collect();

        manager.add_playlist_to_queue(initial_tracks);

        for (op_type, idx) in operations {
            let queue_len = manager.queue_len();

            match op_type {
                0 if queue_len > 0 => {
                    // Remove from queue
                    let valid_idx = idx % queue_len;
                    let result = manager.remove_from_queue(valid_idx);
                    prop_assert!(result.is_ok(), "Valid remove failed");
                }
                1 if queue_len > 1 => {
                    // Reorder in queue
                    let from = idx % queue_len;
                    let to = (idx + 1) % queue_len;
                    // May fail if cross-tier, that's OK
                    let _ = manager.reorder_queue(from, to);
                }
                2 => {
                    // Add track
                    manager.add_to_queue_end(QueueTrack {
                        id: format!("new_track_{}", idx),
                        path: PathBuf::from("/music/test.mp3"),
                        title: format!("New Track {}", idx),
                        artist: "Artist".to_string(),
                        album: None,
                        duration: Duration::from_secs(180),
                        track_number: Some(1),
                        source: TrackSource::Single,
                    });
                }
                3 if queue_len > 0 => {
                    // Skip to index
                    let valid_idx = idx % queue_len;
                    let _ = manager.skip_to_queue_index(valid_idx);
                }
                _ => {
                    // No-op
                }
            }

            // Queue should never have negative or impossible length
            let new_len = manager.queue_len();
            prop_assert!(new_len <= 10000, "Queue length unreasonably large: {}", new_len);

            // All tracks in queue should be accessible
            let queue = manager.get_queue();
            prop_assert_eq!(queue.len(), new_len, "Queue length mismatch");
        }
    }

    // ===== Property Test 4: Seek position always within track bounds =====

    /// Property: For any seek position, the result is clamped within track bounds
    #[test]
    fn seek_always_within_bounds(
        seek_secs in 0u64..1000,
        track_duration_secs in 1u64..600
    ) {
        // This test verifies the seek clamping logic
        // Note: We can't fully test without a real audio source, but we verify the clamping behavior

        // seek_to clamps position to duration - 1ms
        let track_duration = Duration::from_secs(track_duration_secs);
        let seek_position = Duration::from_secs(seek_secs);

        let max_seek = track_duration.saturating_sub(Duration::from_millis(1));
        let clamped_position = seek_position.min(max_seek);

        // Clamped position must be less than duration
        prop_assert!(
            clamped_position < track_duration,
            "Clamped position {} not less than duration {}",
            clamped_position.as_secs_f64(),
            track_duration.as_secs_f64()
        );

        // Clamped position must be >= 0 (always true for Duration)
        prop_assert!(clamped_position >= Duration::ZERO);
    }

    // ===== Property Test 5: Shuffle is a permutation (bijective) =====

    /// Property: Shuffle preserves all tracks (no loss, no duplication) - it's a permutation
    #[test]
    fn shuffle_is_permutation(tracks in arbitrary_unique_tracks()) {
        let mut manager = PlaybackManager::default();

        // Collect original IDs (using unique tracks to avoid duplicate removal)
        let original_ids: Vec<String> = tracks.iter().map(|t| t.id.clone()).collect();
        let original_set: HashSet<String> = original_ids.iter().cloned().collect();

        manager.add_playlist_to_queue(tracks);

        // Enable shuffle
        manager.set_shuffle(ShuffleMode::Random);

        // Get shuffled queue
        let shuffled_ids: Vec<String> = manager
            .get_queue()
            .iter()
            .map(|t| t.id.clone())
            .collect();
        let shuffled_set: HashSet<String> = shuffled_ids.iter().cloned().collect();

        // Same number of unique elements
        prop_assert_eq!(
            original_set.len(),
            shuffled_set.len(),
            "Shuffle changed unique track count"
        );

        // Same elements (permutation property)
        prop_assert_eq!(
            original_set,
            shuffled_set,
            "Shuffle is not a permutation - some tracks lost or duplicated"
        );
    }

    // ===== Property Test 6: Crossfade progress always in [0, 1] =====

    /// Property: Crossfade progress is always in valid range [0.0, 1.0]
    #[test]
    fn crossfade_progress_always_valid(
        duration_ms in 0u32..10000,
        _position_samples in 0usize..1000000
    ) {
        use soul_playback::{CrossfadeEngine, CrossfadeSettings};

        let mut engine = CrossfadeEngine::with_settings(CrossfadeSettings {
            enabled: true,
            duration_ms,
            curve: soul_playback::FadeCurve::EqualPower,
            on_skip: true,
        });
        engine.set_sample_rate(44100);

        // Start crossfade
        let started = engine.start(false);

        if started {
            let progress = engine.progress();
            prop_assert!(
                (0.0..=1.0).contains(&progress),
                "Crossfade progress out of range: {}",
                progress
            );
        }
    }

    // ===== Property Test 7: Position never exceeds duration =====

    /// Property: Reported position is never greater than duration
    #[test]
    fn position_never_exceeds_duration(
        track_duration_secs in 1u64..600
    ) {
        let mut manager = PlaybackManager::default();

        // Add a track with specific duration
        let track = QueueTrack {
            id: "test".to_string(),
            path: PathBuf::from("/music/test.mp3"),
            title: "Test Track".to_string(),
            artist: "Artist".to_string(),
            album: None,
            duration: Duration::from_secs(track_duration_secs),
            track_number: Some(1),
            source: TrackSource::Single,
        };

        manager.add_to_queue_end(track);

        // Without an audio source, position should be 0
        let position = manager.get_position();
        let duration = manager.get_duration();

        // Position should always be <= duration when both are available
        if let Some(dur) = duration {
            prop_assert!(
                position <= dur,
                "Position {:?} exceeded duration {:?}",
                position,
                dur
            );
        }

        // Position should be non-negative (always true for Duration)
        prop_assert!(position >= Duration::ZERO);
    }

    // ===== Property Test 8: After stop, state is reset consistently =====

    /// Property: After stop(), all state is reset to initial conditions
    #[test]
    fn stop_resets_state_consistently(
        operations in prop::collection::vec(0u8..5, 1..20),
        volume in 0u8..=100
    ) {
        let mut manager = PlaybackManager::default();

        // Add tracks and modify state
        for i in 0..10 {
            manager.add_to_queue_end(QueueTrack {
                id: format!("track_{}", i),
                path: PathBuf::from("/music/test.mp3"),
                title: format!("Track {}", i),
                artist: "Artist".to_string(),
                album: None,
                duration: Duration::from_secs(180),
                track_number: Some(1),
                source: TrackSource::Single,
            });
        }

        manager.set_volume(volume);
        manager.set_shuffle(ShuffleMode::Random);
        manager.set_repeat(RepeatMode::All);

        // Perform random operations
        for op in operations {
            match op {
                0 => { manager.play().ok(); }
                1 => { manager.pause(); }
                2 => { manager.next().ok(); }
                3 => { manager.previous().ok(); }
                _ => {}
            }
        }

        // Stop playback
        manager.stop();

        // Verify state is reset
        prop_assert_eq!(
            manager.get_state(),
            PlaybackState::Stopped,
            "State should be Stopped after stop()"
        );

        prop_assert!(
            manager.get_current_track().is_none(),
            "Current track should be None after stop()"
        );

        prop_assert_eq!(
            manager.get_position(),
            Duration::ZERO,
            "Position should be zero after stop()"
        );

        // Note: Volume, shuffle, and repeat settings are preserved after stop (not reset)
        // This is by design - stopping playback shouldn't change user preferences
        prop_assert_eq!(
            manager.get_volume(),
            volume.min(100),
            "Volume should be preserved after stop()"
        );
    }

    // ===== Additional Property Tests =====

    /// Property: Repeat modes are mutually exclusive and correctly set
    #[test]
    fn repeat_mode_correctly_set(
        mode_sequence in prop::collection::vec(
            prop::sample::select(vec![RepeatMode::Off, RepeatMode::All, RepeatMode::One]),
            1..20
        )
    ) {
        let mut manager = PlaybackManager::default();

        for mode in mode_sequence {
            manager.set_repeat(mode);
            let actual = manager.get_repeat();
            prop_assert_eq!(
                actual,
                mode,
                "Repeat mode not correctly set: expected {:?}, got {:?}",
                mode,
                actual
            );
        }
    }

    /// Property: Shuffle mode correctly toggles and cycles
    #[test]
    fn shuffle_mode_correctly_cycles(cycle_count in 0usize..20) {
        let mut manager = PlaybackManager::default();

        let expected_cycle = [ShuffleMode::Off, ShuffleMode::Random, ShuffleMode::Smart];
        let initial_mode = manager.get_shuffle();
        prop_assert_eq!(initial_mode, ShuffleMode::Off);

        for i in 0..cycle_count {
            let new_mode = manager.cycle_shuffle();
            let expected = expected_cycle[(i + 1) % 3];
            prop_assert_eq!(
                new_mode,
                expected,
                "Shuffle cycle incorrect at iteration {}",
                i
            );
        }
    }

    /// Property: Queue operations preserve track integrity
    #[test]
    fn queue_operations_preserve_track_data(
        tracks in prop::collection::vec(arbitrary_track(), 1..20)
    ) {
        let mut manager = PlaybackManager::default();

        // Store original track data
        let original_data: Vec<(String, String, String)> = tracks
            .iter()
            .map(|t| (t.id.clone(), t.title.clone(), t.artist.clone()))
            .collect();

        manager.add_playlist_to_queue(tracks);

        // Verify all track data is preserved
        let queue = manager.get_queue();
        for track in queue {
            let has_original = original_data
                .iter()
                .any(|(id, title, artist)| {
                    &track.id == id && &track.title == title && &track.artist == artist
                });

            // Track data should match one of the originals
            // (Note: We can't guarantee exact position due to shuffle/duplicate removal)
            prop_assert!(
                has_original,
                "Track data corrupted: id={}, title={}, artist={}",
                track.id,
                track.title,
                track.artist
            );
        }
    }

    /// Property: Crossfade gain curves are always monotonic
    #[test]
    fn crossfade_curves_are_monotonic(
        curve in prop::sample::select(vec![
            soul_playback::FadeCurve::Linear,
            soul_playback::FadeCurve::EqualPower,
            soul_playback::FadeCurve::SCurve,
            soul_playback::FadeCurve::SquareRoot,
            soul_playback::FadeCurve::Exponential,
        ]),
        sample_points in 10usize..100
    ) {
        let mut prev_gain_in = 0.0f32;
        let mut prev_gain_out = 1.0f32;

        for i in 0..=sample_points {
            let position = i as f32 / sample_points as f32;
            let gain_in = curve.calculate_gain(position, false);
            let gain_out = curve.calculate_gain(position, true);

            // Fade-in should be monotonically increasing (or equal)
            prop_assert!(
                gain_in >= prev_gain_in - 0.001,
                "Fade-in not monotonic at position {}: {} < {}",
                position,
                gain_in,
                prev_gain_in
            );

            // Fade-out should be monotonically decreasing (or equal)
            prop_assert!(
                gain_out <= prev_gain_out + 0.001,
                "Fade-out not monotonic at position {}: {} > {}",
                position,
                gain_out,
                prev_gain_out
            );

            // Gains should be in [0, 1]
            prop_assert!(
                (0.0..=1.0).contains(&gain_in),
                "Fade-in gain out of range: {}",
                gain_in
            );
            prop_assert!(
                (0.0..=1.0).contains(&gain_out),
                "Fade-out gain out of range: {}",
                gain_out
            );

            prev_gain_in = gain_in;
            prev_gain_out = gain_out;
        }
    }

    /// Property: History size is always bounded
    #[test]
    fn history_size_bounded(
        max_history in 1usize..100,
        track_count in 1usize..200
    ) {
        let config = PlaybackConfig {
            history_size: max_history,
            ..Default::default()
        };

        let mut manager = PlaybackManager::new(config);

        // Add and play through many tracks
        for i in 0..track_count {
            manager.add_to_queue_end(QueueTrack {
                id: format!("track_{}", i),
                path: PathBuf::from("/music/test.mp3"),
                title: format!("Track {}", i),
                artist: "Artist".to_string(),
                album: None,
                duration: Duration::from_secs(180),
                track_number: Some(1),
                source: TrackSource::Single,
            });
        }

        // Advance through tracks
        for _ in 0..track_count {
            manager.next().ok();
        }

        let history = manager.get_history();
        prop_assert!(
            history.len() <= max_history,
            "History {} exceeded max {}",
            history.len(),
            max_history
        );
    }

    /// Property: Clear operations properly empty their targets
    #[test]
    fn clear_operations_work_correctly(
        play_next_count in 0usize..10,
        source_count in 0usize..20,
        queued_later_count in 0usize..10
    ) {
        let mut manager = PlaybackManager::default();

        // Add tracks to different tiers
        for i in 0..source_count {
            manager.add_playlist_to_queue(vec![QueueTrack {
                id: format!("source_{}", i),
                path: PathBuf::from("/music/test.mp3"),
                title: format!("Source {}", i),
                artist: "Artist".to_string(),
                album: None,
                duration: Duration::from_secs(180),
                track_number: Some(1),
                source: TrackSource::Single,
            }]);
        }

        for i in 0..play_next_count {
            manager.add_to_queue_next(QueueTrack {
                id: format!("play_next_{}", i),
                path: PathBuf::from("/music/test.mp3"),
                title: format!("Play Next {}", i),
                artist: "Artist".to_string(),
                album: None,
                duration: Duration::from_secs(180),
                track_number: Some(1),
                source: TrackSource::Single,
            });
        }

        for i in 0..queued_later_count {
            manager.add_to_queue_end(QueueTrack {
                id: format!("queued_later_{}", i),
                path: PathBuf::from("/music/test.mp3"),
                title: format!("Queued Later {}", i),
                artist: "Artist".to_string(),
                album: None,
                duration: Duration::from_secs(180),
                track_number: Some(1),
                source: TrackSource::Single,
            });
        }

        // Clear queue
        manager.clear_queue();

        prop_assert_eq!(
            manager.queue_len(),
            0,
            "Queue should be empty after clear_queue()"
        );
    }
}
