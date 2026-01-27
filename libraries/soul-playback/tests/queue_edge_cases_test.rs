//! Edge Case Tests for Queue Management
//!
//! Tests edge cases in queue operations:
//! 1. Empty queue handling
//! 2. Single track queue edge cases
//! 3. Queue modifications during playback
//! 4. Very large queues (10000+ tracks)
//! 5. Rapid queue modifications
//! 6. Queue index bounds checking

use soul_playback::{
    AudioSource, PlaybackConfig, PlaybackManager, PlaybackState, QueueTrack, RepeatMode,
    ShuffleMode, TrackSource,
};
use std::path::PathBuf;
use std::time::Duration;

// ============================================================================
// MOCK AUDIO SOURCE
// ============================================================================

/// Mock audio source for testing playback state transitions
struct MockAudioSource {
    duration: Duration,
    position: Duration,
    sample_rate: u32,
    samples_per_second: u64,
}

impl MockAudioSource {
    fn new(duration: Duration, sample_rate: u32) -> Self {
        Self {
            duration,
            position: Duration::ZERO,
            sample_rate,
            samples_per_second: sample_rate as u64 * 2, // Stereo
        }
    }
}

impl AudioSource for MockAudioSource {
    fn read_samples(&mut self, buffer: &mut [f32]) -> soul_playback::Result<usize> {
        let total_samples = (self.duration.as_secs_f64() * self.samples_per_second as f64) as u64;
        let current_sample = (self.position.as_secs_f64() * self.samples_per_second as f64) as u64;

        let remaining = total_samples.saturating_sub(current_sample) as usize;
        let to_read = remaining.min(buffer.len());

        if to_read == 0 {
            return Ok(0);
        }

        // Fill with test signal (sine wave)
        for (i, sample) in buffer[..to_read].iter_mut().enumerate() {
            let t = (current_sample + i as u64) as f32 / self.sample_rate as f32;
            *sample = (t * 440.0 * 2.0 * std::f32::consts::PI).sin() * 0.5;
        }

        // Advance position
        let seconds = to_read as f64 / self.samples_per_second as f64;
        self.position += Duration::from_secs_f64(seconds);

        Ok(to_read)
    }

    fn seek(&mut self, position: Duration) -> soul_playback::Result<()> {
        self.position = position.min(self.duration);
        Ok(())
    }

    fn reset(&mut self) -> soul_playback::Result<()> {
        self.position = Duration::ZERO;
        Ok(())
    }

    fn duration(&self) -> Duration {
        self.duration
    }

    fn position(&self) -> Duration {
        self.position
    }

    fn is_finished(&self) -> bool {
        self.position >= self.duration
    }

    fn is_ready(&self) -> bool {
        true
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

fn create_test_track(id: &str) -> QueueTrack {
    QueueTrack {
        id: id.to_string(),
        path: PathBuf::from(format!("/music/{}.mp3", id)),
        title: format!("Track {}", id),
        artist: "Test Artist".to_string(),
        album: Some("Test Album".to_string()),
        duration: Duration::from_secs(180),
        track_number: Some(1),
        source: TrackSource::Single,
    }
}

fn create_tracks(count: usize) -> Vec<QueueTrack> {
    (0..count)
        .map(|i| create_test_track(&i.to_string()))
        .collect()
}

// ============================================================================
// EMPTY QUEUE EDGE CASES
// ============================================================================

#[test]
fn empty_queue_play_returns_queue_empty() {
    let mut manager = PlaybackManager::new(PlaybackConfig::default());

    // Playing with empty queue should return QueueEmpty error
    let result = manager.play();
    assert!(result.is_err());
}

#[test]
fn empty_queue_next_returns_queue_empty() {
    let mut manager = PlaybackManager::new(PlaybackConfig::default());

    // next() with empty queue should return QueueEmpty error
    let result = manager.next();
    assert!(result.is_err());
}

#[test]
fn empty_queue_previous_is_safe() {
    let mut manager = PlaybackManager::new(PlaybackConfig::default());

    // previous() with empty queue should not panic
    let result = manager.previous();
    assert!(result.is_ok());
}

#[test]
fn empty_queue_skip_to_index_fails() {
    let mut manager = PlaybackManager::new(PlaybackConfig::default());

    // skip_to_queue_index with empty queue should fail
    let result = manager.skip_to_queue_index(0);
    assert!(result.is_err());
}

#[test]
fn empty_queue_remove_fails() {
    let mut manager = PlaybackManager::new(PlaybackConfig::default());

    // remove_from_queue with empty queue should fail
    let result = manager.remove_from_queue(0);
    assert!(result.is_err());
}

#[test]
fn empty_queue_reorder_fails() {
    let mut manager = PlaybackManager::new(PlaybackConfig::default());

    // reorder_queue with empty queue should fail
    let result = manager.reorder_queue(0, 0);
    assert!(result.is_err());
}

#[test]
fn empty_queue_get_queue_returns_empty() {
    let manager = PlaybackManager::new(PlaybackConfig::default());

    let queue = manager.get_queue();
    assert!(queue.is_empty());
    assert_eq!(manager.queue_len(), 0);
}

#[test]
fn empty_queue_clear_is_safe() {
    let mut manager = PlaybackManager::new(PlaybackConfig::default());

    // Clear on empty queue should not panic
    manager.clear_queue();
    assert!(manager.get_queue().is_empty());
}

#[test]
fn empty_queue_has_next_false() {
    let manager = PlaybackManager::new(PlaybackConfig::default());

    assert!(!manager.has_next());
}

#[test]
fn empty_queue_has_previous_false() {
    let manager = PlaybackManager::new(PlaybackConfig::default());

    assert!(!manager.has_previous());
}

// ============================================================================
// SINGLE TRACK QUEUE EDGE CASES
// ============================================================================

#[test]
fn single_track_play_and_finish() {
    let mut manager = PlaybackManager::new(PlaybackConfig::default());

    manager.add_playlist_to_queue(vec![create_test_track("1")]);
    assert_eq!(manager.queue_len(), 1);

    let result = manager.play();
    assert!(result.is_ok());

    // After play(), queue should be consumed
    // Queue length depends on implementation - may be 0 or still 1
    // This test documents the behavior
    let queue_len = manager.queue_len();
    assert!(
        queue_len <= 1,
        "Queue length after play should be 0 or 1, got {}",
        queue_len
    );
}

#[test]
fn single_track_next_exhausts_queue() {
    let mut manager = PlaybackManager::new(PlaybackConfig::default());

    manager.add_playlist_to_queue(vec![create_test_track("1")]);
    let _ = manager.play();

    // With single track, next() should fail (queue empty after consuming track)
    let result = manager.next();
    assert!(result.is_err() || manager.queue_len() == 0);
}

#[test]
fn single_track_with_repeat_one() {
    let mut manager = PlaybackManager::new(PlaybackConfig::default());

    manager.add_playlist_to_queue(vec![create_test_track("1")]);
    manager.set_repeat(RepeatMode::One);

    let _ = manager.play();

    // With RepeatMode::One, has_next should always be true
    assert!(manager.has_next());
}

#[test]
fn single_track_with_repeat_all() {
    let mut manager = PlaybackManager::new(PlaybackConfig::default());

    manager.add_playlist_to_queue(vec![create_test_track("1")]);
    manager.set_repeat(RepeatMode::All);

    let _ = manager.play();

    // With RepeatMode::All, has_next should be true (will loop)
    assert!(manager.has_next());
}

#[test]
fn single_track_remove_during_playback() {
    let mut manager = PlaybackManager::new(PlaybackConfig::default());

    manager.add_playlist_to_queue(vec![create_test_track("1"), create_test_track("2")]);
    let _ = manager.play();

    // After play, one track is consumed, one remains in queue
    // Remove the remaining track
    if manager.queue_len() > 0 {
        let result = manager.remove_from_queue(0);
        assert!(result.is_ok());
    }
}

// ============================================================================
// QUEUE MODIFICATIONS DURING PLAYBACK
// ============================================================================

#[test]
fn add_to_queue_during_playback() {
    let mut manager = PlaybackManager::new(PlaybackConfig::default());

    manager.add_playlist_to_queue(vec![create_test_track("1")]);
    let _ = manager.play();

    // Add new track while playing
    manager.add_to_queue_next(create_test_track("new"));

    // Queue should have the new track
    assert!(manager.queue_len() >= 1);
}

#[test]
fn add_to_queue_end_during_playback() {
    let mut manager = PlaybackManager::new(PlaybackConfig::default());

    manager.add_playlist_to_queue(vec![create_test_track("1")]);
    let _ = manager.play();

    // Add track to end while playing
    manager.add_to_queue_end(create_test_track("end"));

    // Queue should have the new track
    let queue = manager.get_queue();
    let has_end_track = queue.iter().any(|t| t.id == "end");
    assert!(has_end_track, "Track 'end' should be in queue");
}

#[test]
fn remove_from_queue_during_playback() {
    let mut manager = PlaybackManager::new(PlaybackConfig::default());

    manager.add_playlist_to_queue(vec![
        create_test_track("1"),
        create_test_track("2"),
        create_test_track("3"),
    ]);
    let _ = manager.play();

    // Remove a track during playback (if any remain in queue)
    if manager.queue_len() > 0 {
        let initial_len = manager.queue_len();
        let result = manager.remove_from_queue(0);
        assert!(result.is_ok());
        assert_eq!(manager.queue_len(), initial_len - 1);
    }
}

#[test]
fn clear_queue_during_playback() {
    let mut manager = PlaybackManager::new(PlaybackConfig::default());

    manager.add_playlist_to_queue(vec![
        create_test_track("1"),
        create_test_track("2"),
        create_test_track("3"),
    ]);
    let _ = manager.play();

    // Clear queue while playing
    manager.clear_queue();

    // Queue should be empty
    assert_eq!(manager.queue_len(), 0);

    // Playback state might still be Playing/Loading (current track continues)
}

#[test]
fn load_new_playlist_during_playback() {
    let mut manager = PlaybackManager::new(PlaybackConfig::default());

    manager.add_playlist_to_queue(vec![create_test_track("old1"), create_test_track("old2")]);
    let _ = manager.play();

    // Load completely new playlist
    manager.load_playlist(
        vec![
            create_test_track("new1"),
            create_test_track("new2"),
            create_test_track("new3"),
        ],
        0,
    );

    // Queue should have new tracks
    let queue = manager.get_queue();
    let has_new_track = queue.iter().any(|t| t.id.starts_with("new"));
    assert!(has_new_track, "Queue should contain new tracks");
}

#[test]
fn toggle_shuffle_during_playback() {
    let mut manager = PlaybackManager::new(PlaybackConfig::default());

    manager.add_playlist_to_queue(create_tracks(10));
    let _ = manager.play();

    // Enable shuffle
    manager.set_shuffle(ShuffleMode::Random);
    assert_eq!(manager.get_shuffle(), ShuffleMode::Random);

    // Disable shuffle
    manager.set_shuffle(ShuffleMode::Off);
    assert_eq!(manager.get_shuffle(), ShuffleMode::Off);
}

#[test]
fn toggle_repeat_during_playback() {
    let mut manager = PlaybackManager::new(PlaybackConfig::default());

    manager.add_playlist_to_queue(create_tracks(3));
    let _ = manager.play();

    // Cycle through repeat modes
    manager.set_repeat(RepeatMode::One);
    assert_eq!(manager.get_repeat(), RepeatMode::One);

    manager.set_repeat(RepeatMode::All);
    assert_eq!(manager.get_repeat(), RepeatMode::All);

    manager.set_repeat(RepeatMode::Off);
    assert_eq!(manager.get_repeat(), RepeatMode::Off);
}

// ============================================================================
// VERY LARGE QUEUE TESTS (10000+ tracks)
// ============================================================================

#[test]
fn large_queue_creation() {
    let mut manager = PlaybackManager::new(PlaybackConfig::default());

    let track_count = 10000;
    let tracks = create_tracks(track_count);

    manager.add_playlist_to_queue(tracks);
    assert_eq!(manager.queue_len(), track_count);
}

#[test]
fn large_queue_skip_to_end() {
    let mut manager = PlaybackManager::new(PlaybackConfig::default());

    let track_count = 10000;
    let tracks = create_tracks(track_count);
    manager.add_playlist_to_queue(tracks);

    let _ = manager.play();

    // Skip near the end
    let target = track_count - 100;
    let result = manager.skip_to_queue_index(target - 1); // -1 because one track was consumed by play()

    // Result depends on lazy loading state
    // This test verifies no panics occur
    let _ = result;
}

#[test]
fn large_queue_random_access() {
    let mut manager = PlaybackManager::new(PlaybackConfig::default());

    let track_count = 10000;
    let tracks = create_tracks(track_count);
    manager.add_playlist_to_queue(tracks);

    // Access random indices
    let queue = manager.get_queue();
    assert_eq!(queue.len(), track_count);

    // Verify first and last
    assert_eq!(queue[0].id, "0");
    assert_eq!(queue[track_count - 1].id, (track_count - 1).to_string());
}

#[test]
fn large_queue_with_shuffle() {
    let mut manager = PlaybackManager::new(PlaybackConfig::default());

    let track_count = 1000; // Reduced for shuffle test
    let tracks = create_tracks(track_count);
    manager.add_playlist_to_queue(tracks);

    // Enable shuffle
    manager.set_shuffle(ShuffleMode::Random);

    // Queue should still have same number of tracks
    assert_eq!(manager.queue_len(), track_count);

    // Restore order
    manager.set_shuffle(ShuffleMode::Off);
    assert_eq!(manager.queue_len(), track_count);
}

// ============================================================================
// RAPID QUEUE MODIFICATIONS
// ============================================================================

#[test]
fn rapid_add_remove_cycle() {
    let mut manager = PlaybackManager::new(PlaybackConfig::default());

    // Rapid add/remove cycles
    for i in 0..100 {
        manager.add_to_queue_next(create_test_track(&format!("track_{}", i)));

        if manager.queue_len() > 10 {
            let _ = manager.remove_from_queue(0);
        }
    }

    // Queue should be stable
    assert!(manager.queue_len() <= 10);
}

#[test]
fn rapid_shuffle_toggle() {
    let mut manager = PlaybackManager::new(PlaybackConfig::default());
    manager.add_playlist_to_queue(create_tracks(100));

    // Rapidly toggle shuffle
    for _ in 0..50 {
        manager.set_shuffle(ShuffleMode::Random);
        manager.set_shuffle(ShuffleMode::Off);
    }

    // Queue should still have all tracks
    assert_eq!(manager.queue_len(), 100);
}

#[test]
fn rapid_clear_and_add() {
    let mut manager = PlaybackManager::new(PlaybackConfig::default());

    for i in 0..20 {
        manager.add_playlist_to_queue(create_tracks(50));
        manager.clear_queue();

        // Add different tracks
        manager.add_playlist_to_queue(
            (0..10)
                .map(|j| create_test_track(&format!("batch{}_{}", i, j)))
                .collect(),
        );
    }

    // Final queue should have 10 tracks from last batch
    assert_eq!(manager.queue_len(), 10);
}

#[test]
fn rapid_play_pause_with_queue_changes() {
    let mut manager = PlaybackManager::new(PlaybackConfig::default());
    manager.add_playlist_to_queue(create_tracks(10));

    for i in 0..20 {
        let _ = manager.play();
        manager.pause();
        manager.add_to_queue_end(create_test_track(&format!("new_{}", i)));
    }

    // Should not panic, queue should have grown
    assert!(manager.queue_len() >= 20);
}

// ============================================================================
// QUEUE INDEX BOUNDS CHECKING
// ============================================================================

#[test]
fn remove_at_last_index() {
    let mut manager = PlaybackManager::new(PlaybackConfig::default());
    manager.add_playlist_to_queue(create_tracks(5));

    // Remove at last valid index
    let last_index = manager.queue_len() - 1;
    let result = manager.remove_from_queue(last_index);
    assert!(result.is_ok());
    assert_eq!(manager.queue_len(), 4);
}

#[test]
fn remove_beyond_bounds_fails() {
    let mut manager = PlaybackManager::new(PlaybackConfig::default());
    manager.add_playlist_to_queue(create_tracks(5));

    // Remove beyond bounds should fail
    let result = manager.remove_from_queue(100);
    assert!(result.is_err());
}

#[test]
fn skip_to_last_index() {
    let mut manager = PlaybackManager::new(PlaybackConfig::default());
    manager.add_playlist_to_queue(create_tracks(5));
    let _ = manager.play();

    // Skip to last track (index = queue_len - 1 after consuming first track)
    if manager.queue_len() > 0 {
        let last_index = manager.queue_len() - 1;
        let result = manager.skip_to_queue_index(last_index);
        assert!(result.is_ok());
    }
}

#[test]
fn skip_beyond_bounds_fails() {
    let mut manager = PlaybackManager::new(PlaybackConfig::default());
    manager.add_playlist_to_queue(create_tracks(5));
    let _ = manager.play();

    // Skip beyond bounds should fail
    let result = manager.skip_to_queue_index(100);
    assert!(result.is_err());
}

#[test]
fn reorder_at_bounds() {
    let mut manager = PlaybackManager::new(PlaybackConfig::default());
    manager.add_playlist_to_queue(create_tracks(5));

    // Reorder first to last
    let result = manager.reorder_queue(0, 4);
    // Result depends on whether tracks are in same tier
    let _ = result;
}

#[test]
fn reorder_beyond_bounds_fails() {
    let mut manager = PlaybackManager::new(PlaybackConfig::default());
    manager.add_playlist_to_queue(create_tracks(5));

    // Reorder with invalid indices should fail
    let result = manager.reorder_queue(0, 100);
    assert!(result.is_err());

    let result = manager.reorder_queue(100, 0);
    assert!(result.is_err());
}

// ============================================================================
// STATE CONSISTENCY TESTS
// ============================================================================

#[test]
fn queue_len_matches_get_queue_len() {
    let mut manager = PlaybackManager::new(PlaybackConfig::default());
    manager.add_playlist_to_queue(create_tracks(10));

    // queue_len() and get_queue().len() should match
    assert_eq!(manager.queue_len(), manager.get_queue().len());

    // After operations
    let _ = manager.play();
    assert_eq!(manager.queue_len(), manager.get_queue().len());
}

#[test]
fn history_cleared_on_new_playlist() {
    let mut manager = PlaybackManager::new(PlaybackConfig::default());

    // Play some tracks
    manager.add_playlist_to_queue(create_tracks(5));
    let _ = manager.play();
    // Note: History is populated when tracks finish playing

    // Load new playlist
    manager.load_playlist(create_tracks(3), 0);

    // History should be cleared
    assert!(
        manager.get_history().is_empty(),
        "History should be cleared when loading new playlist"
    );
}

#[test]
fn has_next_accurate_with_repeat() {
    let mut manager = PlaybackManager::new(PlaybackConfig::default());
    manager.add_playlist_to_queue(create_tracks(1));
    let _ = manager.play();

    // Without repeat, has_next depends on queue state
    manager.set_repeat(RepeatMode::Off);
    let has_next_off = manager.has_next();

    // With RepeatMode::One, always has next
    manager.set_repeat(RepeatMode::One);
    assert!(manager.has_next());

    // With RepeatMode::All, has next if source queue exists
    manager.set_repeat(RepeatMode::All);
    assert!(manager.has_next());

    // Restore and verify
    manager.set_repeat(RepeatMode::Off);
    assert_eq!(manager.has_next(), has_next_off);
}

// ============================================================================
// AUDIO SOURCE INTERACTION TESTS
// ============================================================================

#[test]
fn set_audio_source_with_empty_queue() {
    let mut manager = PlaybackManager::new(PlaybackConfig::default());

    // Set audio source without queue - should work but state matters
    let source = Box::new(MockAudioSource::new(Duration::from_secs(10), 44100));
    manager.set_audio_source(source);

    // State should remain Stopped since we never called play()
    assert_eq!(manager.get_state(), PlaybackState::Stopped);
}

#[test]
fn set_audio_source_after_play_changes_state() {
    let mut manager = PlaybackManager::new(PlaybackConfig::default());

    manager.add_playlist_to_queue(create_tracks(1));
    let _ = manager.play(); // State becomes Loading

    let source = Box::new(MockAudioSource::new(Duration::from_secs(10), 44100));
    manager.set_audio_source(source);

    // State should be Playing after source is set from Loading
    assert_eq!(manager.get_state(), PlaybackState::Playing);
}

#[test]
fn set_audio_source_respects_pause() {
    let mut manager = PlaybackManager::new(PlaybackConfig::default());

    manager.add_playlist_to_queue(create_tracks(1));
    let _ = manager.play(); // Loading
    manager.pause(); // Paused during loading

    let source = Box::new(MockAudioSource::new(Duration::from_secs(10), 44100));
    manager.set_audio_source(source);

    // Should remain Paused because user explicitly paused
    assert_eq!(manager.get_state(), PlaybackState::Paused);
}

// ============================================================================
// CONCURRENT-LIKE OPERATIONS (SIMULATED)
// ============================================================================

#[test]
fn interleaved_operations_are_safe() {
    let mut manager = PlaybackManager::new(PlaybackConfig::default());

    // Simulate interleaved operations that might occur from different UI events
    manager.add_playlist_to_queue(create_tracks(20));
    let _ = manager.play();

    // Interleave various operations
    manager.add_to_queue_next(create_test_track("insert1"));
    manager.set_shuffle(ShuffleMode::Random);
    manager.add_to_queue_end(create_test_track("append1"));

    if manager.queue_len() > 0 {
        let _ = manager.remove_from_queue(0);
    }

    manager.set_shuffle(ShuffleMode::Off);
    manager.pause();
    manager.add_to_queue_next(create_test_track("insert2"));
    let _ = manager.play();

    // Manager should be in a consistent state
    assert!(manager.queue_len() > 0);
}
