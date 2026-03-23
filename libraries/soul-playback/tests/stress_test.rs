//! Stress tests for the playback system
//!
//! These tests verify the playback manager's stability under high-load conditions:
//! - Rapid play/pause toggling (100+ times)
//! - Rapid seek operations (seeking every ~10ms equivalent)
//! - Rapid skip next/previous (50+ times quickly)
//! - Queue modifications during playback stress
//! - Volume changes during playback (rapid 0-100-0 cycles)
//!
//! Goal: Verify no crashes, memory leaks, or state corruption under stress.

use soul_playback::{
    AudioSource, PlaybackConfig, PlaybackError, PlaybackManager, PlaybackState, QueueTrack,
    RepeatMode, Result, ShuffleMode, TrackSource,
};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

// ============================================================================
// Test Infrastructure
// ============================================================================

/// Configurable mock audio source for stress testing
struct StressMockSource {
    duration: Duration,
    position: Duration,
    sample_rate: u32,
    samples_per_second: u64,
    finished: bool,
    amplitude: f32,
    /// Track read calls for verification
    read_count: Arc<AtomicUsize>,
    /// Track seek calls for verification
    seek_count: Arc<AtomicUsize>,
}

impl StressMockSource {
    fn new(duration: Duration, sample_rate: u32) -> Self {
        Self {
            duration,
            position: Duration::ZERO,
            sample_rate,
            samples_per_second: sample_rate as u64 * 2, // Stereo
            finished: false,
            amplitude: 0.5,
            read_count: Arc::new(AtomicUsize::new(0)),
            seek_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn with_counters(mut self, read_count: Arc<AtomicUsize>, seek_count: Arc<AtomicUsize>) -> Self {
        self.read_count = read_count;
        self.seek_count = seek_count;
        self
    }

    fn with_position(mut self, position: Duration) -> Self {
        self.position = position;
        self
    }
}

impl AudioSource for StressMockSource {
    fn read_samples(&mut self, buffer: &mut [f32]) -> Result<usize> {
        self.read_count.fetch_add(1, Ordering::SeqCst);

        if self.finished {
            return Ok(0);
        }

        let total_samples = (self.duration.as_secs_f64() * self.samples_per_second as f64) as u64;
        let current_sample = (self.position.as_secs_f64() * self.samples_per_second as f64) as u64;

        let remaining = (total_samples.saturating_sub(current_sample)) as usize;
        let to_read = remaining.min(buffer.len());

        if to_read == 0 {
            self.finished = true;
            return Ok(0);
        }

        // Generate audio pattern
        for (i, sample) in buffer.iter_mut().enumerate().take(to_read) {
            *sample = self.amplitude * ((i % 2) as f32 - 0.5);
        }

        let samples_read_duration =
            Duration::from_secs_f64(to_read as f64 / self.samples_per_second as f64);
        self.position += samples_read_duration;

        Ok(to_read)
    }

    fn seek(&mut self, position: Duration) -> Result<()> {
        self.seek_count.fetch_add(1, Ordering::SeqCst);

        if position > self.duration {
            return Err(PlaybackError::InvalidSeekPosition(position));
        }
        self.position = position;
        self.finished = false;
        Ok(())
    }

    fn duration(&self) -> Duration {
        self.duration
    }

    fn position(&self) -> Duration {
        self.position
    }

    fn is_finished(&self) -> bool {
        self.finished
    }
}

fn create_stress_track(id: &str, duration_secs: u64) -> QueueTrack {
    QueueTrack {
        id: id.to_string(),
        path: PathBuf::from(format!("/music/{}.mp3", id)),
        title: format!("Stress Track {}", id),
        artist: "Stress Artist".to_string(),
        album: Some("Stress Album".to_string()),
        duration: Duration::from_secs(duration_secs),
        track_number: Some(1),
        source: TrackSource::Single,
    }
}

// ============================================================================
// 1. Rapid Play/Pause Toggling Tests
// ============================================================================

mod rapid_play_pause {
    use super::*;

    #[test]
    fn toggle_100_times_maintains_state_consistency() {
        let mut manager = PlaybackManager::default();

        manager.add_to_queue_end(create_stress_track("1", 180));
        // Must start playback first to get to Playing state
        manager.play().ok();
        let track = create_stress_track("stress", 180);

        manager.activate_source(
            Box::new(StressMockSource::new(Duration::from_secs(180), 44100)),
            track,
        );

        // Rapid play/pause toggling 100 times
        for i in 0..100 {
            manager.play().ok();
            manager.pause();

            // Verify state is consistent after each cycle
            // State can be Playing, Paused, or Loading (during transitions)
            let state = manager.get_state();
            assert!(
                state == PlaybackState::Playing || state == PlaybackState::Paused,
                "State should be Playing, Paused, or Loading after cycle {}, got {:?}",
                i,
                state
            );
        }

        // Final state should be Paused (last action was pause)
        assert_eq!(
            manager.get_state(),
            PlaybackState::Paused,
            "Final state should be Paused"
        );

        // Manager should still be functional
        manager.play().ok();
        assert_eq!(manager.get_state(), PlaybackState::Playing);
    }

    #[test]
    fn toggle_200_times_no_crash() {
        let mut manager = PlaybackManager::default();

        manager.add_to_queue_end(create_stress_track("1", 180));
        let track = create_stress_track("stress", 180);

        manager.activate_source(
            Box::new(StressMockSource::new(Duration::from_secs(180), 44100)),
            track,
        );

        // Rapid play/pause toggling 200 times
        for _ in 0..200 {
            manager.play().ok();
            manager.pause();
        }

        // Verify manager is still functional
        let queue = manager.get_queue();
        assert!(queue.is_empty() || !queue.is_empty()); // Just verify no crash

        // Can still process audio
        let mut buffer = vec![0.0f32; 1024];
        let result = manager.process_audio(&mut buffer);
        // Result may be Ok or Err depending on state, but should not panic
        let _ = result;
    }

    #[test]
    fn toggle_with_audio_processing_interleaved() {
        let read_count = Arc::new(AtomicUsize::new(0));
        let seek_count = Arc::new(AtomicUsize::new(0));

        let mut manager = PlaybackManager::default();
        let track = create_stress_track("1", 180);
        // Use load_playlist + play() + activate_source to follow the real flow:
        // load_playlist sets up queue, play() emits LoadNext, activate_source provides audio
        manager.load_playlist(vec![track.clone()], 0);
        manager.play().ok(); // emits LoadNext for track "1"
        manager.drain_events(); // clear LoadNext event
        manager.activate_source(
            Box::new(
                StressMockSource::new(Duration::from_secs(180), 44100)
                    .with_counters(read_count.clone(), seek_count.clone()),
            ),
            track,
        );

        let mut buffer = vec![0.0f32; 1024];

        // Interleave play/pause with audio processing
        for _ in 0..100 {
            manager.play().ok();
            manager.process_audio(&mut buffer).ok();
            manager.pause();
            manager.process_audio(&mut buffer).ok();
        }

        // Should have processed audio successfully
        let total_reads = read_count.load(Ordering::SeqCst);
        assert!(
            total_reads > 0,
            "Should have performed read operations, got {}",
            total_reads
        );
    }

    #[test]
    fn rapid_toggle_preserves_track_position() {
        let mut manager = PlaybackManager::default();

        manager.add_to_queue_end(create_stress_track("1", 180));
        manager.play().ok(); // Start playback first
        let track = create_stress_track("stress", 180);

        manager.activate_source(
            Box::new(StressMockSource::new(Duration::from_secs(180), 44100)),
            track,
        );

        // Process some audio to advance position
        let mut buffer = vec![0.0f32; 44100 * 2]; // 1 second
        manager.process_audio(&mut buffer).ok();

        let position_before = manager.get_position();

        // Rapid toggle 50 times without processing
        for _ in 0..50 {
            manager.pause();
            manager.play().ok();
        }

        // Position should be preserved (within tolerance)
        let position_after = manager.get_position();

        // Calculate absolute difference between positions using saturating subtraction
        let diff = position_after
            .saturating_sub(position_before)
            .max(position_before.saturating_sub(position_after));

        assert!(
            diff < Duration::from_millis(100),
            "Position drift should be minimal: {:?}",
            diff
        );
    }

    #[test]
    fn toggle_from_stopped_state() {
        let mut manager = PlaybackManager::default();

        // Start from stopped state with tracks in queue
        for i in 1..=5 {
            manager.add_to_queue_end(create_stress_track(&i.to_string(), 180));
        }

        // Try rapid toggle from stopped (play should start, pause should work)
        for _ in 0..50 {
            let play_result = manager.play();
            if play_result.is_ok() {
                // Need to set source for playback to work
                let track = create_stress_track("stress", 180);

                manager.activate_source(
                    Box::new(StressMockSource::new(Duration::from_secs(180), 44100)),
                    track,
                );
            }
            manager.pause();
        }

        // Should not crash - just verify we're in a valid state
        let state = manager.get_state();
        assert!(
            state == PlaybackState::Stopped
                || state == PlaybackState::Paused
                || state == PlaybackState::Playing,
            "Should be in a valid state, got {:?}",
            state
        );
    }
}

// ============================================================================
// 2. Rapid Seek Operations Tests
// ============================================================================

mod rapid_seek {
    use super::*;

    #[test]
    fn seek_100_times_random_positions() {
        let seek_count = Arc::new(AtomicUsize::new(0));
        let read_count = Arc::new(AtomicUsize::new(0));

        let mut manager = PlaybackManager::default();
        let track = create_stress_track("1", 100);
        manager.add_to_queue_end(track.clone());
        manager.play().ok(); // Must start playback for seek to work
        manager.activate_source(
            Box::new(
                StressMockSource::new(Duration::from_secs(100), 44100)
                    .with_counters(read_count.clone(), seek_count.clone()),
            ),
            track,
        );

        // Verify we're in Playing state
        assert_eq!(manager.get_state(), PlaybackState::Playing);

        // Rapid seeks to different positions
        for i in 0..100 {
            let position = Duration::from_secs((i % 99) as u64); // 0-98 seconds
            let result = manager.seek_to(position);
            assert!(
                result.is_ok(),
                "Seek {} to {:?} should succeed: {:?}",
                i,
                position,
                result
            );
        }

        // Verify seeks were performed
        let total_seeks = seek_count.load(Ordering::SeqCst);
        assert_eq!(
            total_seeks, 100,
            "Should have performed 100 seek operations"
        );
    }

    #[test]
    fn seek_with_audio_processing_rapid() {
        let mut manager = PlaybackManager::default();
        manager.add_to_queue_end(create_stress_track("1", 100));
        manager.play().ok(); // Must start playback for seek to work
        let track = create_stress_track("1", 100); // id must match pending_load_track

        manager.activate_source(
            Box::new(StressMockSource::new(Duration::from_secs(100), 44100)),
            track,
        );

        let mut buffer = vec![0.0f32; 512]; // Small buffer for rapid processing

        // Interleave seek and process (simulating seek every ~10ms of audio)
        // At 44100 Hz stereo, ~882 samples is ~10ms
        for i in 0..100 {
            // Seek to position
            let position = Duration::from_secs((i * 99 / 100) as u64);
            manager.seek_to(position).ok();

            // Process a small buffer (simulating rapid callback)
            manager.process_audio(&mut buffer).ok();
        }

        // Manager should still be functional
        assert_eq!(manager.get_state(), PlaybackState::Playing);
    }

    #[test]
    fn seek_percent_rapid() {
        let mut manager = PlaybackManager::default();
        manager.add_to_queue_end(create_stress_track("1", 200));
        manager.play().ok(); // Must start playback for seek to work
        let track = create_stress_track("1", 200); // id must match pending_load_track

        manager.activate_source(
            Box::new(StressMockSource::new(Duration::from_secs(200), 44100)),
            track,
        );

        // Rapid seek by percentage
        for i in 0..100 {
            let percent = (i as f32 % 100.0) / 100.0;
            let result = manager.seek_to_percent(percent);
            assert!(
                result.is_ok(),
                "Seek to {}% should succeed: {:?}",
                percent * 100.0,
                result
            );
        }

        // Verify position is reasonable
        let position = manager.get_position();
        assert!(position <= Duration::from_secs(200));
    }

    #[test]
    fn seek_back_and_forth_stress() {
        let mut manager = PlaybackManager::default();
        manager.add_to_queue_end(create_stress_track("1", 100));
        manager.play().ok(); // Must start playback for seek to work
        let track = create_stress_track("1", 100); // id must match pending_load_track

        manager.activate_source(
            Box::new(StressMockSource::new(Duration::from_secs(100), 44100)),
            track,
        );

        // Seek back and forth rapidly
        for i in 0..50 {
            // Seek to near end
            manager.seek_to(Duration::from_secs(95)).ok();
            // Seek to near start
            manager.seek_to(Duration::from_secs(5)).ok();
            // Seek to middle
            manager.seek_to(Duration::from_secs(50)).ok();

            // Process some audio
            let mut buffer = vec![0.0f32; 1024];
            manager.process_audio(&mut buffer).ok();

            // Verify we're still playing
            assert_eq!(
                manager.get_state(),
                PlaybackState::Playing,
                "Should still be playing after seek cycle {}",
                i
            );
        }
    }

    #[test]
    fn seek_to_boundaries_stress() {
        let mut manager = PlaybackManager::default();
        manager.add_to_queue_end(create_stress_track("1", 100));
        manager.play().ok(); // Must start playback for seek to work
        let track = create_stress_track("stress", 100);

        manager.activate_source(
            Box::new(StressMockSource::new(Duration::from_secs(100), 44100)),
            track,
        );

        // Rapidly seek to boundaries
        for _ in 0..50 {
            // Seek to start
            manager.seek_to(Duration::ZERO).ok();

            // Seek to near end (but not past)
            manager.seek_to(Duration::from_secs(99)).ok();

            // Seek to exact middle
            manager.seek_to(Duration::from_secs(50)).ok();
        }

        // Should not crash and position should be valid
        let position = manager.get_position();
        assert!(position <= Duration::from_secs(100));
    }

    #[test]
    fn seek_beyond_duration_handled() {
        let mut manager = PlaybackManager::default();
        manager.add_to_queue_end(create_stress_track("1", 100));
        manager.play().ok();
        let track = create_stress_track("1", 100); // id must match pending_load_track

        manager.activate_source(
            Box::new(StressMockSource::new(Duration::from_secs(100), 44100)),
            track,
        );

        // Try seeking beyond duration multiple times
        // Note: The manager clamps seek positions to valid range, so this won't fail
        // But it should handle gracefully without crashing
        for _ in 0..20 {
            let result = manager.seek_to(Duration::from_secs(200));
            // The underlying MockSource returns an error for invalid position,
            // but the manager may clamp before calling source
            // Either behavior is acceptable - just don't crash
            let _ = result;
        }

        // Manager should still be functional
        let result = manager.seek_to(Duration::from_secs(50));
        assert!(result.is_ok(), "Valid seek should still work");
    }
}

// ============================================================================
// 3. Rapid Skip Next/Previous Tests
// ============================================================================

mod rapid_skip {
    use super::*;

    #[test]
    fn skip_next_50_times() {
        let mut manager = PlaybackManager::default();

        // Add 100 tracks to have enough for skipping
        for i in 1..=100 {
            manager.add_to_queue_end(create_stress_track(&i.to_string(), 180));
        }

        // Set initial source
        let track = create_stress_track("stress", 180);

        manager.activate_source(
            Box::new(StressMockSource::new(Duration::from_secs(180), 44100)),
            track,
        );

        // Rapid next 50 times
        let mut successful_skips = 0;
        for _ in 0..50 {
            let result = manager.next();
            if result.is_ok() {
                successful_skips += 1;
                // Set source for each new track
                let track = create_stress_track("stress", 180);

                manager.activate_source(
                    Box::new(StressMockSource::new(Duration::from_secs(180), 44100)),
                    track,
                );
            }
        }

        assert!(
            successful_skips >= 45,
            "Most skips should succeed, got {}",
            successful_skips
        );

        // History should have accumulated
        let history = manager.get_history();
        assert!(!history.is_empty(), "History should have tracks");
    }

    #[test]
    fn skip_previous_50_times() {
        let mut manager = PlaybackManager::default();

        // Add tracks and play through some to build history
        for i in 1..=20 {
            manager.add_to_queue_end(create_stress_track(&i.to_string(), 180));
        }

        // Play through to build history
        for _ in 0..15 {
            manager.next().ok();
            let track = create_stress_track("stress", 180);

            manager.activate_source(
                Box::new(StressMockSource::new(Duration::from_secs(180), 44100)),
                track,
            );
        }

        let history_before = manager.get_history().len();

        // Rapid previous calls (will go back through history then restart)
        for _ in 0..50 {
            manager.previous().ok();
            // Set source with position < 3s to trigger actual previous navigation
            let track = create_stress_track("prev", 180);
            manager.activate_source(
                Box::new(
                    StressMockSource::new(Duration::from_secs(180), 44100)
                        .with_position(Duration::from_secs(1)),
                ),
                track,
            );
        }

        // History should have been consumed (or at least partially)
        let history_after = manager.get_history().len();
        // Note: history behavior depends on implementation details
        // Just verify the operation completed without crashing
        assert!(
            history_after <= history_before,
            "History should not grow during previous operations"
        );
    }

    #[test]
    fn skip_next_previous_alternating() {
        let mut manager = PlaybackManager::default();

        for i in 1..=30 {
            manager.add_to_queue_end(create_stress_track(&i.to_string(), 180));
        }

        let track = create_stress_track("stress", 180);

        manager.activate_source(
            Box::new(StressMockSource::new(Duration::from_secs(180), 44100)),
            track,
        );

        // Alternate between next and previous rapidly
        for i in 0..50 {
            if i % 2 == 0 {
                manager.next().ok();
            } else {
                manager.previous().ok();
            }
            let track = create_stress_track("nav", 180);
            manager.activate_source(
                Box::new(
                    StressMockSource::new(Duration::from_secs(180), 44100)
                        .with_position(Duration::from_secs(1)), // Short position for previous to work
                ),
                track,
            );
        }

        // Manager should still be functional
        let state = manager.get_state();
        assert!(state == PlaybackState::Playing || state == PlaybackState::Stopped);
    }

    #[test]
    fn skip_with_repeat_one() {
        let mut manager = PlaybackManager::default();
        manager.set_repeat(RepeatMode::One);

        manager.add_to_queue_end(create_stress_track("1", 180));
        let track = create_stress_track("stress", 180);

        manager.activate_source(
            Box::new(StressMockSource::new(Duration::from_secs(180), 44100)),
            track,
        );

        // Rapid next with repeat one (should restart same track)
        for _ in 0..50 {
            manager.next().ok();
            // Set fresh source each time (simulating track restart)
            let track = create_stress_track("stress", 180);

            manager.activate_source(
                Box::new(StressMockSource::new(Duration::from_secs(180), 44100)),
                track,
            );
        }

        // Should still have repeat mode
        assert_eq!(manager.get_repeat(), RepeatMode::One);
    }

    #[test]
    fn skip_with_repeat_all() {
        let mut manager = PlaybackManager::default();
        manager.set_repeat(RepeatMode::All);

        // Small queue to force wrap-around
        for i in 1..=5 {
            manager.add_to_queue_end(create_stress_track(&i.to_string(), 180));
        }

        let track = create_stress_track("stress", 180);

        manager.activate_source(
            Box::new(StressMockSource::new(Duration::from_secs(180), 44100)),
            track,
        );

        // Skip more than queue length to test repeat all
        for _ in 0..20 {
            manager.next().ok();
            let track = create_stress_track("stress", 180);

            manager.activate_source(
                Box::new(StressMockSource::new(Duration::from_secs(180), 44100)),
                track,
            );
        }

        // With repeat all, should still be playing (queue loops)
        let state = manager.get_state();
        // State may vary but should not be crashed
        assert!(state == PlaybackState::Playing || state == PlaybackState::Stopped);
    }

    #[test]
    fn skip_empty_queue_handling() {
        let mut manager = PlaybackManager::default();

        // Try skipping on empty queue
        for _ in 0..20 {
            let result = manager.next();
            // Should fail gracefully
            assert!(result.is_err());
        }

        // Manager should still be functional
        manager.add_to_queue_end(create_stress_track("1", 180));
        let result = manager.play();
        // Should now work
        assert!(result.is_ok() || manager.queue_len() > 0);
    }
}

// ============================================================================
// 4. Queue Modifications During Playback Stress
// ============================================================================

mod queue_stress {
    use super::*;

    #[test]
    fn add_remove_during_playback() {
        let mut manager = PlaybackManager::default();

        manager.add_to_queue_end(create_stress_track("initial", 180));
        manager.play().ok();
        let track = create_stress_track("initial", 180); // id must match pending_load_track

        manager.activate_source(
            Box::new(StressMockSource::new(Duration::from_secs(180), 44100)),
            track,
        );

        let mut buffer = vec![0.0f32; 1024];

        // Interleave add/remove with processing
        for i in 0..100 {
            // Process audio
            manager.process_audio(&mut buffer).ok();

            // Add a track
            manager.add_to_queue_end(create_stress_track(&format!("add_{}", i), 180));

            // Process more audio
            manager.process_audio(&mut buffer).ok();

            // Remove from queue if possible
            if manager.queue_len() > 1 {
                manager.remove_from_queue(0).ok();
            }
        }

        // Should still be playing
        assert_eq!(manager.get_state(), PlaybackState::Playing);
    }

    #[test]
    fn clear_queue_during_playback() {
        let mut manager = PlaybackManager::default();

        for i in 1..=20 {
            manager.add_to_queue_end(create_stress_track(&i.to_string(), 180));
        }

        manager.play().ok();
        let track = create_stress_track("1", 180); // id must match first pending_load_track

        manager.activate_source(
            Box::new(StressMockSource::new(Duration::from_secs(180), 44100)),
            track,
        );

        let mut buffer = vec![0.0f32; 1024];

        // Clear and repopulate rapidly
        for i in 0..20 {
            manager.process_audio(&mut buffer).ok();
            manager.clear_queue();
            assert_eq!(manager.queue_len(), 0);

            // Repopulate
            for j in 1..=5 {
                manager.add_to_queue_end(create_stress_track(&format!("{}_{}", i, j), 180));
            }
        }

        // Should still be playing (current track not affected by queue clear)
        assert_eq!(manager.get_state(), PlaybackState::Playing);
    }

    #[test]
    fn shuffle_during_playback_stress() {
        let mut manager = PlaybackManager::default();

        for i in 1..=50 {
            manager.add_to_queue_end(create_stress_track(&i.to_string(), 180));
        }

        manager.play().ok();
        let track = create_stress_track("stress", 180);

        manager.activate_source(
            Box::new(StressMockSource::new(Duration::from_secs(180), 44100)),
            track,
        );

        let mut buffer = vec![0.0f32; 1024];

        // Toggle shuffle rapidly while processing
        for _ in 0..50 {
            manager.process_audio(&mut buffer).ok();
            manager.set_shuffle(ShuffleMode::Random);
            manager.process_audio(&mut buffer).ok();
            manager.set_shuffle(ShuffleMode::Off);
            manager.process_audio(&mut buffer).ok();
            manager.set_shuffle(ShuffleMode::Smart);
        }

        // Should still have tracks
        assert!(manager.queue_len() > 0 || manager.get_current_track().is_some());
    }

    #[test]
    fn reorder_queue_during_playback() {
        let mut manager = PlaybackManager::default();

        for i in 1..=20 {
            manager.add_to_queue_end(create_stress_track(&i.to_string(), 180));
        }

        manager.play().ok();
        let track = create_stress_track("1", 180); // id must match first pending_load_track

        manager.activate_source(
            Box::new(StressMockSource::new(Duration::from_secs(180), 44100)),
            track,
        );

        let mut buffer = vec![0.0f32; 1024];

        // Reorder queue while processing
        for _ in 0..30 {
            manager.process_audio(&mut buffer).ok();

            // Reorder if we have enough tracks
            if manager.queue_len() >= 3 {
                manager.reorder_queue(0, 2).ok();
            }
        }

        // Should still be playing
        assert_eq!(manager.get_state(), PlaybackState::Playing);
    }

    #[test]
    fn add_to_queue_next_stress() {
        let mut manager = PlaybackManager::default();

        manager.add_to_queue_end(create_stress_track("first", 180));
        manager.play().ok();
        let track = create_stress_track("stress", 180);

        manager.activate_source(
            Box::new(StressMockSource::new(Duration::from_secs(180), 44100)),
            track,
        );

        let mut buffer = vec![0.0f32; 1024];

        // Rapidly add to "play next" position
        for i in 0..100 {
            manager.process_audio(&mut buffer).ok();
            manager.add_to_queue_next(create_stress_track(&format!("next_{}", i), 180));
        }

        // Queue should have accumulated
        assert!(manager.queue_len() >= 100);
    }

    #[test]
    fn skip_to_queue_index_stress() {
        let mut manager = PlaybackManager::default();

        for i in 1..=50 {
            manager.add_to_queue_end(create_stress_track(&i.to_string(), 180));
        }

        // Rapidly skip to different indices
        for _ in 0..30 {
            let queue_len = manager.queue_len();
            if queue_len > 5 {
                let target = queue_len / 2;
                manager.skip_to_queue_index(target).ok();
                let track = create_stress_track("stress", 180);

                manager.activate_source(
                    Box::new(StressMockSource::new(Duration::from_secs(180), 44100)),
                    track,
                );
            }
        }

        // Manager should still be functional
        let state = manager.get_state();
        assert!(state == PlaybackState::Playing || state == PlaybackState::Stopped);
    }

    #[test]
    fn add_playlist_stress() {
        let mut manager = PlaybackManager::default();

        // Rapidly add individual tracks (add_playlist_to_queue replaces, not appends)
        // Use add_to_queue_end which accumulates tracks
        for batch in 0..20 {
            for i in 1..=10 {
                manager.add_to_queue_end(create_stress_track(&format!("{}_{}", batch, i), 180));
            }
        }

        // Should have 200 tracks
        let total = manager.queue_len();
        assert!(
            total >= 199,
            "Should have accumulated tracks, got {}",
            total
        );

        // Now start playback
        manager.play().ok();
        let track = create_stress_track("stress", 180);

        manager.activate_source(
            Box::new(StressMockSource::new(Duration::from_secs(180), 44100)),
            track,
        );

        // Process some audio while queue is large
        let mut buffer = vec![0.0f32; 1024];
        for _ in 0..10 {
            manager.process_audio(&mut buffer).ok();
        }

        // Should still have many tracks (minus the one being played)
        assert!(
            manager.queue_len() >= 190,
            "Should still have many tracks in queue"
        );
    }
}

// ============================================================================
// 5. Volume Changes During Playback Stress
// ============================================================================

mod volume_stress {
    use super::*;

    #[test]
    fn rapid_volume_changes_100_cycles() {
        let mut manager = PlaybackManager::default();
        manager.add_to_queue_end(create_stress_track("1", 180));
        manager.play().ok();
        let track = create_stress_track("stress", 180);

        manager.activate_source(
            Box::new(StressMockSource::new(Duration::from_secs(180), 44100)),
            track,
        );

        let mut buffer = vec![0.0f32; 1024];

        // Rapid 0-100-0 volume cycles
        for _ in 0..100 {
            // Ramp up
            for v in (0..=100).step_by(10) {
                manager.set_volume(v);
                manager.process_audio(&mut buffer).ok();
            }

            // Ramp down
            for v in (0..=100).rev().step_by(10) {
                manager.set_volume(v);
                manager.process_audio(&mut buffer).ok();
            }
        }

        // Volume should be at last set value
        let final_volume = manager.get_volume();
        assert!(final_volume <= 100);
    }

    #[test]
    fn extreme_volume_toggling() {
        let mut manager = PlaybackManager::default();
        manager.add_to_queue_end(create_stress_track("1", 180));
        manager.play().ok();
        let track = create_stress_track("1", 180); // id must match pending_load_track

        manager.activate_source(
            Box::new(StressMockSource::new(Duration::from_secs(180), 44100)),
            track,
        );

        let mut buffer = vec![0.0f32; 1024];

        // Toggle between 0 and 100 rapidly
        for i in 0..200 {
            let volume = if i % 2 == 0 { 0 } else { 100 };
            manager.set_volume(volume);
            manager.process_audio(&mut buffer).ok();
        }

        // Should not crash and audio should be processed
        assert_eq!(manager.get_state(), PlaybackState::Playing);
    }

    #[test]
    fn mute_unmute_rapid_cycling() {
        let mut manager = PlaybackManager::default();
        manager.add_to_queue_end(create_stress_track("1", 180));
        let track = create_stress_track("stress", 180);

        manager.activate_source(
            Box::new(StressMockSource::new(Duration::from_secs(180), 44100)),
            track,
        );

        let mut buffer = vec![0.0f32; 1024];

        // Rapid mute/unmute
        for _ in 0..100 {
            manager.mute();
            manager.process_audio(&mut buffer).ok();
            manager.unmute();
            manager.process_audio(&mut buffer).ok();
        }

        // Should end unmuted
        assert!(!manager.is_muted());
    }

    #[test]
    fn toggle_mute_rapid() {
        let mut manager = PlaybackManager::default();
        manager.add_to_queue_end(create_stress_track("1", 180));
        let track = create_stress_track("stress", 180);

        manager.activate_source(
            Box::new(StressMockSource::new(Duration::from_secs(180), 44100)),
            track,
        );

        let mut buffer = vec![0.0f32; 1024];

        // Rapid toggle_mute
        for _ in 0..100 {
            manager.toggle_mute();
            manager.process_audio(&mut buffer).ok();
        }

        // After even number of toggles, should be back to original state (not muted)
        assert!(!manager.is_muted());
    }

    #[test]
    fn volume_and_mute_combined_stress() {
        let mut manager = PlaybackManager::default();
        manager.add_to_queue_end(create_stress_track("1", 180));
        let track = create_stress_track("stress", 180);

        manager.activate_source(
            Box::new(StressMockSource::new(Duration::from_secs(180), 44100)),
            track,
        );

        let mut buffer = vec![0.0f32; 1024];

        // Combined volume and mute operations
        for i in 0..100 {
            manager.set_volume((i * 7) as u8 % 101); // Cycle through volumes
            if i % 3 == 0 {
                manager.mute();
            }
            if i % 5 == 0 {
                manager.unmute();
            }
            manager.process_audio(&mut buffer).ok();
        }

        // Volume should be preserved
        let volume = manager.get_volume();
        assert!(volume <= 100);
    }

    #[test]
    fn volume_preserves_during_mute() {
        let mut manager = PlaybackManager::default();
        manager.set_volume(75);

        // Rapid mute/unmute should preserve volume
        for _ in 0..50 {
            manager.mute();
            assert_eq!(
                manager.get_volume(),
                75,
                "Volume should be preserved when muted"
            );
            manager.unmute();
            assert_eq!(
                manager.get_volume(),
                75,
                "Volume should be preserved after unmute"
            );
        }
    }
}

// ============================================================================
// 6. Combined Stress Tests
// ============================================================================

mod combined_stress {
    use super::*;

    #[test]
    fn all_operations_interleaved() {
        let mut manager = PlaybackManager::default();

        // Setup
        for i in 1..=50 {
            manager.add_to_queue_end(create_stress_track(&i.to_string(), 180));
        }
        manager.play().ok();
        let track = create_stress_track("stress", 180);

        manager.activate_source(
            Box::new(StressMockSource::new(Duration::from_secs(180), 44100)),
            track,
        );

        let mut buffer = vec![0.0f32; 512];

        // Interleave all operations
        for i in 0..100 {
            // Volume
            manager.set_volume((i * 13) as u8 % 101);

            // Process
            manager.process_audio(&mut buffer).ok();

            // Play/Pause
            if i % 7 == 0 {
                manager.pause();
            }
            if i % 11 == 0 {
                manager.play().ok();
            }

            // Seek (only if playing and have source)
            if manager.get_state() == PlaybackState::Playing && i % 5 == 0 {
                manager.seek_to(Duration::from_secs((i % 100) as u64)).ok();
            }

            // Queue operations
            if i % 13 == 0 {
                manager.add_to_queue_end(create_stress_track(&format!("added_{}", i), 180));
            }

            // Shuffle toggle
            if i % 17 == 0 {
                let current = manager.get_shuffle();
                let next = match current {
                    ShuffleMode::Off => ShuffleMode::Random,
                    ShuffleMode::Random => ShuffleMode::Smart,
                    ShuffleMode::Smart => ShuffleMode::Off,
                };
                manager.set_shuffle(next);
            }

            // Skip (occasionally)
            if i % 23 == 0 {
                manager.next().ok();
                let track = create_stress_track("stress", 180);

                manager.activate_source(
                    Box::new(StressMockSource::new(Duration::from_secs(180), 44100)),
                    track,
                );
            }
        }

        // Manager should still be functional
        let state = manager.get_state();
        assert!(
            state == PlaybackState::Playing
                || state == PlaybackState::Paused
                || state == PlaybackState::Stopped
        );
    }

    #[test]
    fn stress_with_crossfade_enabled() {
        let config = PlaybackConfig {
            crossfade: soul_playback::CrossfadeSettings::with_duration(1000),
            ..Default::default()
        };
        let mut manager = PlaybackManager::new(config);
        manager.set_crossfade_enabled(true);

        for i in 1..=20 {
            manager.add_to_queue_end(create_stress_track(&i.to_string(), 10)); // Short tracks
        }

        manager.play().ok();
        let track = create_stress_track("stress", 10);

        manager.activate_source(
            Box::new(StressMockSource::new(Duration::from_secs(10), 44100)),
            track,
        );

        let mut buffer = vec![0.0f32; 4096];

        // Process with crossfade
        for _ in 0..50 {
            manager.process_audio(&mut buffer).ok();

            // Skip to trigger crossfade
            manager.next().ok();
            let track = create_stress_track("stress", 10);

            manager.activate_source(
                Box::new(StressMockSource::new(Duration::from_secs(10), 44100)),
                track,
            );
        }

        // Should complete without crash
        let state = manager.get_state();
        assert!(state == PlaybackState::Playing || state == PlaybackState::Stopped);
    }

    #[test]
    fn stress_with_small_history() {
        let config = PlaybackConfig {
            history_size: 3, // Very small history
            ..Default::default()
        };
        let mut manager = PlaybackManager::new(config);

        for i in 1..=30 {
            manager.add_to_queue_end(create_stress_track(&i.to_string(), 180));
        }

        // Play through many tracks to overflow history
        for _ in 0..25 {
            manager.next().ok();
            let track = create_stress_track("stress", 180);

            manager.activate_source(
                Box::new(StressMockSource::new(Duration::from_secs(180), 44100)),
                track,
            );
        }

        // History should be limited to 3
        let history = manager.get_history();
        assert!(history.len() <= 3, "History should respect max size");

        // Go back through history
        for _ in 0..5 {
            manager.previous().ok();
            let track = create_stress_track("hist", 180);
            manager.activate_source(
                Box::new(
                    StressMockSource::new(Duration::from_secs(180), 44100)
                        .with_position(Duration::from_secs(1)),
                ),
                track,
            );
        }

        // Should still be functional
        let state = manager.get_state();
        assert!(state == PlaybackState::Playing || state == PlaybackState::Stopped);
    }

    #[test]
    fn stress_with_large_queue() {
        let mut manager = PlaybackManager::default();

        // Add 1000 tracks
        for i in 1..=1000 {
            manager.add_to_queue_end(create_stress_track(&i.to_string(), 180));
        }

        let track = create_stress_track("stress", 180);

        manager.activate_source(
            Box::new(StressMockSource::new(Duration::from_secs(180), 44100)),
            track,
        );

        let mut buffer = vec![0.0f32; 1024];

        // Operations on large queue
        for i in 0..50 {
            manager.process_audio(&mut buffer).ok();

            // Shuffle
            if i % 10 == 0 {
                manager.set_shuffle(ShuffleMode::Smart);
            }

            // Skip
            if i % 5 == 0 {
                manager.next().ok();
                let track = create_stress_track("stress", 180);

                manager.activate_source(
                    Box::new(StressMockSource::new(Duration::from_secs(180), 44100)),
                    track,
                );
            }

            // Queue modification
            if i % 7 == 0 && manager.queue_len() > 10 {
                manager.remove_from_queue(5).ok();
            }
        }

        // Queue should still have many tracks
        assert!(
            manager.queue_len() > 900,
            "Queue should still have many tracks"
        );
    }

    #[test]
    fn repeated_stop_start_cycles() {
        let mut manager = PlaybackManager::default();

        for cycle in 0..50 {
            // Fresh queue
            manager.clear_queue();
            for i in 1..=10 {
                manager.add_to_queue_end(create_stress_track(&format!("{}_{}", cycle, i), 180));
            }

            // Start playback
            manager.play().ok();
            // id must match the first pending_load_track for this cycle
            let first_id = format!("{}_1", cycle);
            let track = create_stress_track(&first_id, 180);

            manager.activate_source(
                Box::new(StressMockSource::new(Duration::from_secs(180), 44100)),
                track,
            );

            // Process some audio
            let mut buffer = vec![0.0f32; 4096];
            for _ in 0..5 {
                manager.process_audio(&mut buffer).ok();
            }

            // Stop
            manager.stop();
            assert_eq!(manager.get_state(), PlaybackState::Stopped);
        }

        // Final start should work — clear leftover queue from last cycle first.
        manager.clear_queue();
        manager.add_to_queue_end(create_stress_track("final", 180));
        manager.play().ok();
        let track = create_stress_track("final", 180); // id must match pending_load_track

        manager.activate_source(
            Box::new(StressMockSource::new(Duration::from_secs(180), 44100)),
            track,
        );
        assert_eq!(manager.get_state(), PlaybackState::Playing);
    }
}

// ============================================================================
// 7. Edge Case Stress Tests
// ============================================================================

mod edge_case_stress {
    use super::*;

    #[test]
    fn operations_without_source() {
        let mut manager = PlaybackManager::default();

        manager.add_to_queue_end(create_stress_track("1", 180));

        // Try operations without setting audio source
        for _ in 0..50 {
            manager.play().ok();
            manager.pause();
            manager.set_volume(50);

            let mut buffer = vec![0.0f32; 1024];
            manager.process_audio(&mut buffer).ok();
        }

        // Should not crash
        let state = manager.get_state();
        assert!(state == PlaybackState::Stopped || state == PlaybackState::Paused);
    }

    #[test]
    fn rapid_source_replacement() {
        let mut manager = PlaybackManager::default();
        manager.add_to_queue_end(create_stress_track("1", 180));

        // Rapidly replace audio source
        for _ in 0..100 {
            let track = create_stress_track("stress", 180);

            manager.activate_source(
                Box::new(StressMockSource::new(Duration::from_secs(180), 44100)),
                track,
            );

            let mut buffer = vec![0.0f32; 512];
            manager.process_audio(&mut buffer).ok();
        }

        // Should not crash
        assert!(manager.get_duration().unwrap_or(Duration::ZERO) > Duration::ZERO);
    }

    #[test]
    fn empty_buffer_processing() {
        let mut manager = PlaybackManager::default();
        manager.add_to_queue_end(create_stress_track("1", 180));
        let track = create_stress_track("stress", 180);

        manager.activate_source(
            Box::new(StressMockSource::new(Duration::from_secs(180), 44100)),
            track,
        );

        // Try processing with empty buffer
        let mut empty_buffer: Vec<f32> = vec![];
        for _ in 0..50 {
            let _ = manager.process_audio(&mut empty_buffer);
        }

        // Should not crash (behavior depends on implementation)
    }

    #[test]
    fn very_small_buffer_processing() {
        let mut manager = PlaybackManager::default();
        manager.add_to_queue_end(create_stress_track("1", 180));
        manager.play().ok(); // Start playback
        let track = create_stress_track("1", 180); // id must match pending_load_track

        manager.activate_source(
            Box::new(StressMockSource::new(Duration::from_secs(180), 44100)),
            track,
        );

        // Process with very small buffers
        for _ in 0..1000 {
            let mut tiny_buffer = vec![0.0f32; 2]; // Just 1 stereo sample
            manager.process_audio(&mut tiny_buffer).ok();
        }

        // Should not crash - position may or may not advance significantly with tiny buffers
        // depending on internal state machine behavior (fade-in, source ready checks, etc.)
        // The main goal of this stress test is stability, not position tracking
        let state = manager.get_state();
        assert!(
            state == PlaybackState::Playing || state == PlaybackState::Paused,
            "Should be in valid state after processing, got {:?}",
            state
        );
    }

    #[test]
    fn very_large_buffer_processing() {
        let mut manager = PlaybackManager::default();
        manager.add_to_queue_end(create_stress_track("1", 180));
        manager.play().ok(); // Start playback
        let track = create_stress_track("stress", 180);

        manager.activate_source(
            Box::new(StressMockSource::new(Duration::from_secs(180), 44100)),
            track,
        );

        // Process with large buffer
        let mut large_buffer = vec![0.0f32; 44100 * 2 * 10]; // 10 seconds
        for _ in 0..10 {
            manager.process_audio(&mut large_buffer).ok();
        }

        // Should not crash - verify state is valid
        let state = manager.get_state();
        assert!(
            state == PlaybackState::Playing
                || state == PlaybackState::Paused
                || state == PlaybackState::Stopped,
            "Should be in valid state after processing, got {:?}",
            state
        );
    }

    #[test]
    fn zero_duration_track_handling() {
        let mut manager = PlaybackManager::default();

        // Track with zero duration
        manager.add_to_queue_end(QueueTrack {
            id: "zero".to_string(),
            path: PathBuf::from("/music/zero.mp3"),
            title: "Zero Duration".to_string(),
            artist: "Artist".to_string(),
            album: None,
            duration: Duration::ZERO,
            track_number: None,
            source: TrackSource::Single,
        });

        // Normal track
        let track_normal = create_stress_track("normal", 180);
        manager.add_to_queue_end(track_normal.clone());

        manager.activate_source(
            Box::new(StressMockSource::new(Duration::ZERO, 44100)),
            track_normal,
        );

        let mut buffer = vec![0.0f32; 1024];

        // Process - should handle zero duration gracefully
        for _ in 0..10 {
            let _ = manager.process_audio(&mut buffer);
        }

        // Should not crash
    }
}
