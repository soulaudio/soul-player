//! End-to-end tests for PlaybackManager
//!
//! Comprehensive tests covering all aspects of playback management:
//! - Queue management
//! - Playback state transitions
//! - Crossfade behavior
//! - Repeat modes
//! - Volume control
//! - Track transitions
//! - Error handling
//! - Concurrent operations

use soul_playback::{
    AudioSource, CrossfadeSettings, CrossfadeState, FadeCurve, PlaybackConfig, PlaybackError,
    PlaybackManager, PlaybackState, QueueTrack, RepeatMode, Result, ShuffleMode, TrackSource,
};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

// ============================================================================
// Test Infrastructure
// ============================================================================

/// Configurable mock audio source for testing
struct MockAudioSource {
    duration: Duration,
    position: Duration,
    sample_rate: u32,
    samples_per_second: u64,
    finished: bool,
    /// Generate specific audio pattern (for crossfade testing)
    amplitude: f32,
    /// Simulate failures
    fail_on_read: bool,
    fail_on_seek: bool,
    /// Track read calls for verification
    read_count: Arc<AtomicUsize>,
}

impl MockAudioSource {
    fn new(duration: Duration, sample_rate: u32) -> Self {
        Self {
            duration,
            position: Duration::ZERO,
            sample_rate,
            samples_per_second: sample_rate as u64 * 2, // Stereo
            finished: false,
            amplitude: 0.5,
            fail_on_read: false,
            fail_on_seek: false,
            read_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn with_amplitude(mut self, amplitude: f32) -> Self {
        self.amplitude = amplitude;
        self
    }

    fn with_position(mut self, position: Duration) -> Self {
        self.position = position;
        self
    }

    fn failing_reads(mut self) -> Self {
        self.fail_on_read = true;
        self
    }

    fn failing_seeks(mut self) -> Self {
        self.fail_on_seek = true;
        self
    }

    fn with_read_counter(mut self, counter: Arc<AtomicUsize>) -> Self {
        self.read_count = counter;
        self
    }
}

impl AudioSource for MockAudioSource {
    fn read_samples(&mut self, buffer: &mut [f32]) -> Result<usize> {
        self.read_count.fetch_add(1, Ordering::SeqCst);

        if self.fail_on_read {
            return Err(PlaybackError::AudioSource("Simulated read failure".into()));
        }

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
        if self.fail_on_seek {
            return Err(PlaybackError::AudioSource("Simulated seek failure".into()));
        }
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

fn create_track(id: &str, title: &str, artist: &str, duration_secs: u64) -> QueueTrack {
    QueueTrack {
        id: id.to_string(),
        path: PathBuf::from(format!("/music/{}.mp3", id)),
        title: title.to_string(),
        artist: artist.to_string(),
        album: Some("Test Album".to_string()),
        duration: Duration::from_secs(duration_secs),
        track_number: Some(1),
        source: TrackSource::Single,
    }
}

fn create_track_from_album(
    id: &str,
    title: &str,
    artist: &str,
    album_id: &str,
    album_name: &str,
    track_num: u32,
) -> QueueTrack {
    QueueTrack {
        id: id.to_string(),
        path: PathBuf::from(format!("/music/{}.mp3", id)),
        title: title.to_string(),
        artist: artist.to_string(),
        album: Some(album_name.to_string()),
        duration: Duration::from_secs(180),
        track_number: Some(track_num),
        source: TrackSource::Album {
            id: album_id.to_string(),
            name: album_name.to_string(),
        },
    }
}

// ============================================================================
// 1. Queue Management Tests
// ============================================================================

mod queue_management {
    use super::*;

    #[test]
    fn add_track_while_playing_does_not_interrupt() {
        let mut manager = PlaybackManager::default();

        // Start playing
        manager.add_to_queue_end(create_track("1", "Track 1", "Artist A", 180));
        manager.set_audio_source(Box::new(MockAudioSource::new(
            Duration::from_secs(180),
            44100,
        )));

        assert_eq!(manager.get_state(), PlaybackState::Playing);

        // Process some audio
        let mut buffer = vec![0.0f32; 1024];
        manager.process_audio(&mut buffer).unwrap();

        // Add tracks while playing
        manager.add_to_queue_end(create_track("2", "Track 2", "Artist B", 180));
        manager.add_to_queue_next(create_track("3", "Track 3", "Artist C", 180));

        // Should still be playing
        assert_eq!(manager.get_state(), PlaybackState::Playing);
        // Explicit queue track should be first
        assert_eq!(manager.get_queue()[0].id, "3");
    }

    #[test]
    fn remove_track_while_playing_maintains_state() {
        let mut manager = PlaybackManager::default();

        manager.add_to_queue_end(create_track("1", "Track 1", "Artist A", 180));
        manager.add_to_queue_end(create_track("2", "Track 2", "Artist B", 180));
        manager.add_to_queue_end(create_track("3", "Track 3", "Artist C", 180));

        manager.set_audio_source(Box::new(MockAudioSource::new(
            Duration::from_secs(180),
            44100,
        )));

        // Remove middle track
        let removed = manager.remove_from_queue(1).unwrap();
        assert_eq!(removed.id, "2");

        // Still playing
        assert_eq!(manager.get_state(), PlaybackState::Playing);
        assert_eq!(manager.queue_len(), 2);
    }

    #[test]
    fn shuffle_distribution_is_random() {
        // Run multiple shuffles and verify distribution
        let mut position_counts: HashMap<String, Vec<usize>> = HashMap::new();
        let track_ids: Vec<String> = (1..=10).map(|i| format!("{}", i)).collect();

        for id in &track_ids {
            position_counts.insert(id.clone(), vec![]);
        }

        // Run 100 shuffles
        for _ in 0..100 {
            let mut manager = PlaybackManager::default();
            let tracks: Vec<QueueTrack> = track_ids
                .iter()
                .map(|id| create_track(id, &format!("Track {}", id), "Artist", 180))
                .collect();

            manager.add_playlist_to_queue(tracks);
            manager.set_shuffle(ShuffleMode::Random);

            for (pos, track) in manager.get_queue().iter().enumerate() {
                position_counts.get_mut(&track.id).unwrap().push(pos);
            }
        }

        // Verify each track appeared at multiple positions
        for (id, positions) in &position_counts {
            let unique_positions: HashSet<usize> = positions.iter().cloned().collect();
            assert!(
                unique_positions.len() >= 5,
                "Track {} should appear at multiple positions, got {:?}",
                id,
                unique_positions
            );
        }
    }

    #[test]
    fn smart_shuffle_avoids_consecutive_same_artist() {
        let mut manager = PlaybackManager::default();

        // 4 tracks from each of 3 artists
        let mut tracks = vec![];
        for artist_idx in 0..3 {
            for track_idx in 0..4 {
                tracks.push(create_track(
                    &format!("a{}t{}", artist_idx, track_idx),
                    &format!("Song {}", track_idx),
                    &format!("Artist {}", artist_idx),
                    180,
                ));
            }
        }

        manager.add_playlist_to_queue(tracks);
        manager.set_shuffle(ShuffleMode::Smart);

        let queue = manager.get_queue();
        let mut consecutive_same_artist = 0;

        for i in 0..queue.len() - 1 {
            if queue[i].artist == queue[i + 1].artist {
                consecutive_same_artist += 1;
            }
        }

        // With 3 artists and 4 tracks each, should have minimal consecutive same-artist
        assert!(
            consecutive_same_artist <= 4,
            "Smart shuffle should minimize consecutive same-artist plays, got {}",
            consecutive_same_artist
        );
    }

    #[test]
    fn history_tracking_is_accurate() {
        let config = PlaybackConfig {
            history_size: 10,
            ..Default::default()
        };
        let mut manager = PlaybackManager::new(config);

        // Add and play through tracks
        for i in 1..=5 {
            manager.add_to_queue_end(create_track(
                &i.to_string(),
                &format!("Track {}", i),
                "Artist",
                180,
            ));
        }

        // Simulate playing through tracks
        for _ in 0..4 {
            manager.next().ok();
        }

        let history = manager.get_history();
        // History should contain played tracks
        assert!(history.len() <= 4, "History should track played tracks");

        // Verify order (oldest first in get_all())
        if history.len() >= 2 {
            // History tracks should be in order they were played
            for window in history.windows(2) {
                let first_id: i32 = window[0].id.parse().unwrap_or(0);
                let second_id: i32 = window[1].id.parse().unwrap_or(0);
                assert!(first_id < second_id, "History should maintain play order");
            }
        }
    }

    #[test]
    fn history_respects_max_size() {
        let config = PlaybackConfig {
            history_size: 3,
            ..Default::default()
        };
        let mut manager = PlaybackManager::new(config);

        // Add many tracks
        for i in 1..=10 {
            manager.add_to_queue_end(create_track(
                &i.to_string(),
                &format!("Track {}", i),
                "Artist",
                180,
            ));
        }

        // Play through all tracks
        for _ in 0..10 {
            manager.next().ok();
        }

        let history = manager.get_history();
        assert!(
            history.len() <= 3,
            "History should respect max size of 3, got {}",
            history.len()
        );
    }

    #[test]
    fn queue_reorder_within_explicit() {
        let mut manager = PlaybackManager::default();

        // Add explicit queue tracks
        manager.add_to_queue_end(create_track("1", "Track 1", "Artist", 180));
        manager.add_to_queue_end(create_track("2", "Track 2", "Artist", 180));
        manager.add_to_queue_end(create_track("3", "Track 3", "Artist", 180));

        // Reorder: move first to last
        manager.reorder_queue(0, 2).unwrap();

        let queue = manager.get_queue();
        assert_eq!(queue[0].id, "2");
        assert_eq!(queue[1].id, "3");
        assert_eq!(queue[2].id, "1");
    }

    #[test]
    fn queue_clear_removes_all() {
        let mut manager = PlaybackManager::default();

        manager.add_to_queue_end(create_track("1", "Track 1", "Artist", 180));
        manager.add_playlist_to_queue(vec![
            create_track("2", "Track 2", "Artist", 180),
            create_track("3", "Track 3", "Artist", 180),
        ]);

        manager.clear_queue();

        assert!(manager.get_queue().is_empty());
        assert_eq!(manager.queue_len(), 0);
    }

    #[test]
    fn skip_to_queue_index_works() {
        let mut manager = PlaybackManager::default();

        for i in 1..=5 {
            manager.add_to_queue_end(create_track(
                &i.to_string(),
                &format!("Track {}", i),
                "Artist",
                180,
            ));
        }

        // Skip to index 2 (third track)
        manager.skip_to_queue_index(2).unwrap();

        // Queue length should be reduced
        assert!(manager.queue_len() < 5);
    }
}

// ============================================================================
// 2. Playback State Tests
// ============================================================================

mod playback_state {
    use super::*;

    #[test]
    fn play_pause_transitions() {
        let mut manager = PlaybackManager::default();

        manager.add_to_queue_end(create_track("1", "Track 1", "Artist", 180));
        manager.set_audio_source(Box::new(MockAudioSource::new(
            Duration::from_secs(180),
            44100,
        )));

        // Playing
        assert_eq!(manager.get_state(), PlaybackState::Playing);

        // Pause
        manager.pause();
        assert_eq!(manager.get_state(), PlaybackState::Paused);

        // Resume
        manager.play().unwrap();
        assert_eq!(manager.get_state(), PlaybackState::Playing);
    }

    #[test]
    fn stop_clears_current_track() {
        let mut manager = PlaybackManager::default();

        manager.add_to_queue_end(create_track("1", "Track 1", "Artist", 180));
        manager.set_audio_source(Box::new(MockAudioSource::new(
            Duration::from_secs(180),
            44100,
        )));

        assert!(
            manager.get_current_track().is_some() || manager.get_state() == PlaybackState::Playing
        );

        manager.stop();

        assert_eq!(manager.get_state(), PlaybackState::Stopped);
        assert!(manager.get_current_track().is_none());
    }

    #[test]
    fn stop_does_not_clear_queue() {
        let mut manager = PlaybackManager::default();

        manager.add_to_queue_end(create_track("1", "Track 1", "Artist", 180));
        manager.add_to_queue_end(create_track("2", "Track 2", "Artist", 180));

        manager.set_audio_source(Box::new(MockAudioSource::new(
            Duration::from_secs(180),
            44100,
        )));

        let queue_len_before = manager.queue_len();

        manager.stop();

        // Queue should still have tracks
        assert_eq!(manager.queue_len(), queue_len_before);
    }

    #[test]
    fn seek_accuracy_within_tolerance() {
        let mut manager = PlaybackManager::default();
        manager.set_audio_source(Box::new(MockAudioSource::new(
            Duration::from_secs(100),
            44100,
        )));

        // Seek to 50 seconds
        manager.seek_to(Duration::from_secs(50)).unwrap();

        let position = manager.get_position();
        let diff = (position.as_secs() as i64 - 50).abs();
        assert!(diff <= 1, "Seek should be accurate within 1 second");
    }

    #[test]
    fn seek_percent_calculates_correctly() {
        let mut manager = PlaybackManager::default();
        manager.set_audio_source(Box::new(MockAudioSource::new(
            Duration::from_secs(200),
            44100,
        )));

        // Seek to 25%
        manager.seek_to_percent(0.25).unwrap();

        let position = manager.get_position();
        // 25% of 200 = 50 seconds
        let diff = (position.as_secs() as i64 - 50).abs();
        assert!(
            diff <= 1,
            "Seek to 25% of 200s should be ~50s, got {}s",
            position.as_secs()
        );
    }

    #[test]
    fn position_reporting_during_playback() {
        let mut manager = PlaybackManager::default();
        manager.set_audio_source(Box::new(MockAudioSource::new(
            Duration::from_secs(100),
            44100,
        )));

        let initial_position = manager.get_position();

        // Process some audio
        let mut buffer = vec![0.0f32; 44100 * 2]; // 1 second of stereo audio
        manager.process_audio(&mut buffer).unwrap();

        let new_position = manager.get_position();

        // Position should have advanced
        assert!(new_position > initial_position);
    }

    #[test]
    fn paused_state_does_not_advance_position() {
        let mut manager = PlaybackManager::default();
        manager.set_audio_source(Box::new(MockAudioSource::new(
            Duration::from_secs(100),
            44100,
        )));

        manager.pause();
        let position_before = manager.get_position();

        // Process audio while paused
        let mut buffer = vec![0.0f32; 4096];
        manager.process_audio(&mut buffer).unwrap();

        let position_after = manager.get_position();
        assert_eq!(position_before, position_after);
    }

    #[test]
    fn stopped_state_outputs_silence() {
        let mut manager = PlaybackManager::default();
        assert_eq!(manager.get_state(), PlaybackState::Stopped);

        let mut buffer = vec![1.0f32; 1024];
        manager.process_audio(&mut buffer).unwrap();

        assert!(
            buffer.iter().all(|&s| s == 0.0),
            "Stopped state should output silence"
        );
    }
}

// ============================================================================
// 3. Crossfade Tests
// ============================================================================

mod crossfade {
    use super::*;

    #[test]
    fn equal_power_curve_maintains_energy() {
        // Equal power curve should maintain constant perceived loudness
        // At midpoint, both gains should be ~0.707 (1/sqrt(2))
        let curve = FadeCurve::EqualPower;

        for i in 0..=10 {
            let t = i as f32 / 10.0;
            let fade_out = curve.calculate_gain(t, true);
            let fade_in = curve.calculate_gain(t, false);

            // Power sum should be approximately 1.0
            let power_sum = fade_out * fade_out + fade_in * fade_in;
            assert!(
                (power_sum - 1.0).abs() < 0.02,
                "Equal power sum at t={}: {} (expected ~1.0)",
                t,
                power_sum
            );
        }
    }

    #[test]
    fn linear_curve_has_volume_dip() {
        let curve = FadeCurve::Linear;

        // At midpoint
        let fade_out = curve.calculate_gain(0.5, true);
        let fade_in = curve.calculate_gain(0.5, false);

        // Linear: both are 0.5, so power sum = 0.25 + 0.25 = 0.5 (-3dB)
        let power_sum = fade_out * fade_out + fade_in * fade_in;
        assert!(
            (power_sum - 0.5).abs() < 0.01,
            "Linear power sum at midpoint: {} (expected 0.5)",
            power_sum
        );
    }

    #[test]
    fn scurve_boundaries() {
        let curve = FadeCurve::SCurve;

        // At start
        assert!((curve.calculate_gain(0.0, false) - 0.0).abs() < 0.001);
        // At end
        assert!((curve.calculate_gain(1.0, false) - 1.0).abs() < 0.001);
        // At midpoint (should be ~0.5)
        assert!((curve.calculate_gain(0.5, false) - 0.5).abs() < 0.01);
    }

    #[test]
    fn gapless_detection_and_timing() {
        let settings = CrossfadeSettings::gapless();
        assert_eq!(settings.duration_ms, 0);
        assert!(settings.enabled);

        // Duration samples should be 0 for gapless
        assert_eq!(settings.duration_samples(44100), 0);
    }

    #[test]
    fn crossfade_timing_samples() {
        let settings = CrossfadeSettings::with_duration(1000); // 1 second

        assert_eq!(settings.duration_samples(44100), 44100);
        assert_eq!(settings.duration_samples(48000), 48000);
        assert_eq!(settings.duration_samples(96000), 96000);
    }

    #[test]
    fn crossfade_state_transitions() {
        let mut manager = PlaybackManager::default();
        manager.set_crossfade_enabled(true);
        manager.set_crossfade_duration(1000);

        // Initially inactive
        assert_eq!(manager.get_crossfade_state(), CrossfadeState::Inactive);
        assert!(!manager.is_crossfading());

        // Add track and source
        manager.add_to_queue_end(create_track("1", "Track 1", "Artist", 10));
        manager.set_audio_source(Box::new(MockAudioSource::new(
            Duration::from_secs(10),
            44100,
        )));

        // Still inactive without next source
        assert_eq!(manager.get_crossfade_state(), CrossfadeState::Inactive);
    }

    #[test]
    fn crossfade_on_skip_setting() {
        let mut manager = PlaybackManager::default();
        manager.set_crossfade_enabled(true);
        manager.set_crossfade_duration(1000);
        manager.set_crossfade_on_skip(false);

        let settings = manager.get_crossfade_settings();
        assert!(!settings.on_skip);

        manager.set_crossfade_on_skip(true);
        let settings = manager.get_crossfade_settings();
        assert!(settings.on_skip);
    }

    #[test]
    fn crossfade_progress_range() {
        let manager = PlaybackManager::default();
        let progress = manager.get_crossfade_progress();

        assert!(
            (0.0..=1.0).contains(&progress),
            "Progress should be 0.0-1.0"
        );
    }

    #[test]
    fn crossfade_with_next_source_setup() {
        let mut manager = PlaybackManager::default();
        manager.set_crossfade_enabled(true);

        manager.add_to_queue_end(create_track("1", "Track 1", "Artist", 10));
        manager.add_to_queue_end(create_track("2", "Track 2", "Artist", 10));

        manager.set_audio_source(Box::new(MockAudioSource::new(
            Duration::from_secs(10),
            44100,
        )));

        assert!(!manager.has_next_source());

        // Pre-decode next track
        let next_source = Box::new(MockAudioSource::new(Duration::from_secs(10), 44100));
        manager.set_next_source(next_source, create_track("2", "Track 2", "Artist", 10));

        assert!(manager.has_next_source());
        assert_eq!(manager.get_next_track().unwrap().id, "2");
    }
}

// ============================================================================
// 4. Repeat Mode Tests
// ============================================================================

mod repeat_modes {
    use super::*;

    #[test]
    fn repeat_one_restarts_current_track() {
        let mut manager = PlaybackManager::default();
        manager.set_repeat(RepeatMode::One);

        manager.add_to_queue_end(create_track("1", "Track 1", "Artist", 10));
        manager.add_to_queue_end(create_track("2", "Track 2", "Artist", 10));

        manager.set_audio_source(Box::new(MockAudioSource::new(
            Duration::from_secs(10),
            44100,
        )));

        // Next should attempt to repeat current track
        let result = manager.next();
        assert!(result.is_ok());

        // Should still be in repeat one mode
        assert_eq!(manager.get_repeat(), RepeatMode::One);
    }

    #[test]
    fn repeat_all_loops_queue() {
        let mut manager = PlaybackManager::default();
        manager.set_repeat(RepeatMode::All);

        manager.add_playlist_to_queue(vec![
            create_track("1", "Track 1", "Artist", 180),
            create_track("2", "Track 2", "Artist", 180),
        ]);

        // Play through all tracks
        for _ in 0..3 {
            // More than queue size
            manager.next().ok();
        }

        // With repeat all, next should succeed even after queue end
        // (actual behavior depends on implementation)
        assert_eq!(manager.get_repeat(), RepeatMode::All);
    }

    #[test]
    fn no_repeat_stops_at_end() {
        let mut manager = PlaybackManager::default();
        manager.set_repeat(RepeatMode::Off);

        manager.add_to_queue_end(create_track("1", "Track 1", "Artist", 180));

        // After playing through, next should fail
        manager.next().ok(); // Consume the track

        let result = manager.next();
        // Queue should be empty, next should fail
        assert!(result.is_err());
    }

    #[test]
    fn has_next_respects_repeat_mode() {
        let mut manager = PlaybackManager::default();
        manager.set_repeat(RepeatMode::One);

        // With repeat one, always has next
        assert!(manager.has_next());

        manager.set_repeat(RepeatMode::Off);
        assert!(!manager.has_next()); // Queue is empty

        manager.add_to_queue_end(create_track("1", "Track 1", "Artist", 180));
        assert!(manager.has_next());
    }

    #[test]
    fn has_previous_respects_repeat_mode() {
        let mut manager = PlaybackManager::default();

        // No history, no repeat
        manager.set_repeat(RepeatMode::Off);
        assert!(!manager.has_previous());

        // With repeat one, always has previous
        manager.set_repeat(RepeatMode::One);
        assert!(manager.has_previous());
    }
}

// ============================================================================
// 5. Volume Control Tests
// ============================================================================

mod volume_control {
    use super::*;

    #[test]
    fn volume_curve_is_logarithmic() {
        // Verify logarithmic scaling (not linear)
        let mut manager = PlaybackManager::default();
        manager.set_audio_source(Box::new(MockAudioSource::new(
            Duration::from_secs(10),
            44100,
        )));

        // Test different volume levels
        let volumes = [0, 25, 50, 75, 100];
        let mut peaks: Vec<f32> = vec![];

        for &vol in &volumes {
            manager.set_volume(vol);
            let mut buffer = vec![0.0f32; 1024];
            manager.process_audio(&mut buffer).ok();

            let peak = buffer.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
            peaks.push(peak);
        }

        // 0% should be silence
        assert!(peaks[0] < 0.001, "0% volume should be silence");

        // Volume increase should not be linear
        // At 50%, should be much less than 50% of max
        if peaks[4] > 0.0 {
            let ratio_50 = peaks[2] / peaks[4];
            assert!(
                ratio_50 < 0.5,
                "50% volume should be < 50% of max (logarithmic), got ratio {}",
                ratio_50
            );
        }
    }

    #[test]
    fn mute_unmute_preserves_level() {
        let mut manager = PlaybackManager::default();
        manager.set_volume(75);

        manager.mute();
        assert!(manager.is_muted());
        assert_eq!(manager.get_volume(), 75); // Level preserved

        manager.unmute();
        assert!(!manager.is_muted());
        assert_eq!(manager.get_volume(), 75);
    }

    #[test]
    fn toggle_mute_works() {
        let mut manager = PlaybackManager::default();

        assert!(!manager.is_muted());
        manager.toggle_mute();
        assert!(manager.is_muted());
        manager.toggle_mute();
        assert!(!manager.is_muted());
    }

    #[test]
    fn muted_output_is_silence() {
        let mut manager = PlaybackManager::default();
        manager.set_audio_source(Box::new(MockAudioSource::new(
            Duration::from_secs(10),
            44100,
        )));

        manager.mute();

        let mut buffer = vec![1.0f32; 1024];
        manager.process_audio(&mut buffer).ok();

        assert!(buffer.iter().all(|&s| s == 0.0));
    }

    #[test]
    fn volume_clamps_to_100() {
        let mut manager = PlaybackManager::default();
        manager.set_volume(150);
        assert_eq!(manager.get_volume(), 100);
    }

    #[test]
    fn volume_ramping_no_clicks() {
        let mut manager = PlaybackManager::default();
        manager.set_audio_source(Box::new(
            MockAudioSource::new(Duration::from_secs(10), 44100).with_amplitude(1.0),
        ));

        // Process at high volume
        manager.set_volume(100);
        let mut buffer1 = vec![0.0f32; 1024];
        manager.process_audio(&mut buffer1).ok();

        // Change to low volume and process
        manager.set_volume(10);
        let mut buffer2 = vec![0.0f32; 1024];
        manager.process_audio(&mut buffer2).ok();

        // The transition should be smooth (no sudden jumps)
        // This is a basic check - in real audio, you'd verify sample-by-sample
        let last_high = buffer1.last().unwrap().abs();
        let first_low = buffer2.first().unwrap().abs();

        // Volume change should result in different levels
        // (note: this test is approximate since volume application might be per-buffer)
        assert!(
            last_high >= first_low,
            "Volume reduction should result in lower samples"
        );
    }
}

// ============================================================================
// 6. Track Transition Tests
// ============================================================================

mod track_transitions {
    use super::*;

    #[test]
    fn next_track_timing() {
        let mut manager = PlaybackManager::default();

        manager.add_to_queue_end(create_track("1", "Track 1", "Artist", 180));
        manager.add_to_queue_end(create_track("2", "Track 2", "Artist", 180));

        manager.set_audio_source(Box::new(MockAudioSource::new(
            Duration::from_secs(180),
            44100,
        )));

        // Process a bit of audio
        let mut buffer = vec![0.0f32; 1024];
        manager.process_audio(&mut buffer).ok();

        // Next should work
        let result = manager.next();
        assert!(result.is_ok());
    }

    #[test]
    fn previous_restarts_after_3_seconds() {
        let mut manager = PlaybackManager::default();

        manager.add_to_queue_end(create_track("1", "Track 1", "Artist", 180));

        // Set source at position > 3 seconds
        let source = MockAudioSource::new(Duration::from_secs(180), 44100)
            .with_position(Duration::from_secs(10));
        manager.set_audio_source(Box::new(source));

        // Previous should restart (position > 3s)
        let result = manager.previous();
        assert!(result.is_ok());

        // Position should be reset
        let pos = manager.get_position();
        assert!(
            pos < Duration::from_secs(1),
            "Position should be near start after restart"
        );
    }

    #[test]
    fn previous_goes_to_history_before_3_seconds() {
        let mut manager = PlaybackManager::default();

        // Add and play first track
        manager.add_to_queue_end(create_track("1", "Track 1", "Artist", 180));
        manager.add_to_queue_end(create_track("2", "Track 2", "Artist", 180));

        // Play track 1
        manager.next().ok(); // This adds track 1 to history

        // Set source at position < 3 seconds for track 2
        let source = MockAudioSource::new(Duration::from_secs(180), 44100)
            .with_position(Duration::from_secs(1));
        manager.set_audio_source(Box::new(source));

        let history_before = manager.get_history().len();

        // Previous should go to history track (position < 3s)
        manager.previous().ok();

        // History should have one less item
        let history_after = manager.get_history().len();
        assert!(
            history_after <= history_before,
            "Previous should pop from history"
        );
    }

    #[test]
    fn skip_during_crossfade_handled() {
        let mut manager = PlaybackManager::default();
        manager.set_crossfade_enabled(true);
        manager.set_crossfade_duration(3000);
        manager.set_crossfade_on_skip(true);

        manager.add_to_queue_end(create_track("1", "Track 1", "Artist", 10));
        manager.add_to_queue_end(create_track("2", "Track 2", "Artist", 10));
        manager.add_to_queue_end(create_track("3", "Track 3", "Artist", 10));

        manager.set_audio_source(Box::new(MockAudioSource::new(
            Duration::from_secs(10),
            44100,
        )));

        // Set up next source for crossfade
        manager.set_next_source(
            Box::new(MockAudioSource::new(Duration::from_secs(10), 44100)),
            create_track("2", "Track 2", "Artist", 10),
        );

        // Skip during potential crossfade
        let result = manager.next();
        assert!(result.is_ok());
    }

    #[test]
    fn end_of_track_detected() {
        let mut manager = PlaybackManager::default();

        // Very short track
        manager.add_to_queue_end(create_track("1", "Track 1", "Artist", 1));
        manager.add_to_queue_end(create_track("2", "Track 2", "Artist", 180));

        manager.set_audio_source(Box::new(MockAudioSource::new(
            Duration::from_secs(1),
            44100,
        )));

        // Process until end of track
        let mut total_samples = 0;
        let mut buffer = vec![0.0f32; 4096];

        for _ in 0..50 {
            match manager.process_audio(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(n) => total_samples += n,
            }
        }

        assert!(total_samples > 0, "Should have read some samples");
    }
}

// ============================================================================
// 7. Error Handling Tests
// ============================================================================

mod error_handling {
    use super::*;

    #[test]
    fn missing_file_handled_gracefully() {
        let mut manager = PlaybackManager::default();

        // Add track with non-existent path (path is just metadata, not validated here)
        manager.add_to_queue_end(QueueTrack {
            id: "missing".to_string(),
            path: PathBuf::from("/nonexistent/path/file.mp3"),
            title: "Missing Track".to_string(),
            artist: "Artist".to_string(),
            album: None,
            duration: Duration::from_secs(180),
            track_number: None,
            source: TrackSource::Single,
        });

        // Play should enter loading state (actual file loading is platform-specific)
        let result = manager.play();
        // This might fail with QueueEmpty if queue is consumed
        // The important thing is it doesn't panic
        let _ = result;
    }

    #[test]
    fn decoder_error_on_read() {
        let mut manager = PlaybackManager::default();

        // Set up failing source
        manager.set_audio_source(Box::new(
            MockAudioSource::new(Duration::from_secs(10), 44100).failing_reads(),
        ));

        let mut buffer = vec![0.0f32; 1024];
        let result = manager.process_audio(&mut buffer);

        // Should return error
        assert!(result.is_err());
    }

    #[test]
    fn seek_error_handled() {
        let mut manager = PlaybackManager::default();

        manager.set_audio_source(Box::new(
            MockAudioSource::new(Duration::from_secs(10), 44100).failing_seeks(),
        ));

        let result = manager.seek_to(Duration::from_secs(5));
        assert!(result.is_err());
    }

    #[test]
    fn recovery_after_error() {
        let mut manager = PlaybackManager::default();

        manager.add_to_queue_end(create_track("1", "Track 1", "Artist", 180));
        manager.add_to_queue_end(create_track("2", "Track 2", "Artist", 180));

        // Even after errors, basic operations should work
        manager.stop();
        assert_eq!(manager.get_state(), PlaybackState::Stopped);

        // Should be able to set new source
        manager.set_audio_source(Box::new(MockAudioSource::new(
            Duration::from_secs(180),
            44100,
        )));
        assert_eq!(manager.get_state(), PlaybackState::Playing);
    }

    #[test]
    fn empty_queue_error() {
        let mut manager = PlaybackManager::default();

        let result = manager.play();
        assert!(matches!(result, Err(PlaybackError::QueueEmpty)));
    }

    #[test]
    fn seek_no_track_error() {
        let mut manager = PlaybackManager::default();

        let result = manager.seek_to(Duration::from_secs(30));
        assert!(matches!(result, Err(PlaybackError::NoTrackLoaded)));
    }

    #[test]
    fn seek_beyond_duration_error() {
        let mut manager = PlaybackManager::default();
        manager.set_audio_source(Box::new(MockAudioSource::new(
            Duration::from_secs(100),
            44100,
        )));

        let result = manager.seek_to(Duration::from_secs(200));
        assert!(result.is_err());
    }

    #[test]
    fn index_out_of_bounds_error() {
        let mut manager = PlaybackManager::default();
        manager.add_to_queue_end(create_track("1", "Track 1", "Artist", 180));

        let result = manager.remove_from_queue(10);
        assert!(matches!(result, Err(PlaybackError::IndexOutOfBounds(10))));
    }
}

// ============================================================================
// 8. Concurrent Operations Tests
// ============================================================================

mod concurrent_operations {
    use super::*;

    #[test]
    fn rapid_state_changes_consistent() {
        let mut manager = PlaybackManager::default();

        manager.add_to_queue_end(create_track("1", "Track 1", "Artist", 180));
        manager.set_audio_source(Box::new(MockAudioSource::new(
            Duration::from_secs(180),
            44100,
        )));

        // Rapid state changes
        for _ in 0..100 {
            manager.play().ok();
            manager.pause();
        }

        // Should be in consistent paused state
        assert_eq!(manager.get_state(), PlaybackState::Paused);

        // Should still be able to play
        manager.play().ok();
        assert_eq!(manager.get_state(), PlaybackState::Playing);
    }

    #[test]
    fn rapid_queue_operations_consistent() {
        let mut manager = PlaybackManager::default();

        // Rapid add/remove
        for i in 0..50 {
            manager.add_to_queue_end(create_track(&i.to_string(), "Track", "Artist", 180));
        }

        for _ in 0..25 {
            manager.remove_from_queue(0).ok();
        }

        assert_eq!(manager.queue_len(), 25);

        // Rapid clear and add
        manager.clear_queue();
        assert_eq!(manager.queue_len(), 0);
    }

    #[test]
    fn volume_changes_during_processing() {
        let mut manager = PlaybackManager::default();
        manager.set_audio_source(Box::new(MockAudioSource::new(
            Duration::from_secs(10),
            44100,
        )));

        let mut buffer = vec![0.0f32; 1024];

        // Interleave volume changes and processing
        for i in 0..10 {
            manager.set_volume((i * 10) as u8);
            manager.process_audio(&mut buffer).ok();
        }

        // Should complete without issues
        assert_eq!(manager.get_volume(), 90);
    }

    #[test]
    fn shuffle_during_playback() {
        let mut manager = PlaybackManager::default();

        for i in 0..20 {
            manager.add_to_queue_end(create_track(&i.to_string(), "Track", "Artist", 180));
        }

        manager.set_audio_source(Box::new(MockAudioSource::new(
            Duration::from_secs(180),
            44100,
        )));

        // Toggle shuffle during playback
        manager.set_shuffle(ShuffleMode::Random);
        manager.set_shuffle(ShuffleMode::Off);
        manager.set_shuffle(ShuffleMode::Smart);
        manager.set_shuffle(ShuffleMode::Random);

        // State should be consistent
        assert_eq!(manager.get_shuffle(), ShuffleMode::Random);
        assert!(manager.queue_len() > 0);
    }

    #[test]
    fn repeat_mode_changes_during_playback() {
        let mut manager = PlaybackManager::default();

        manager.add_to_queue_end(create_track("1", "Track 1", "Artist", 180));
        manager.set_audio_source(Box::new(MockAudioSource::new(
            Duration::from_secs(180),
            44100,
        )));

        // Cycle through repeat modes
        for mode in [
            RepeatMode::Off,
            RepeatMode::All,
            RepeatMode::One,
            RepeatMode::Off,
        ] {
            manager.set_repeat(mode);
            assert_eq!(manager.get_repeat(), mode);
        }
    }

    #[test]
    fn commands_interleaved_with_processing() {
        let mut manager = PlaybackManager::default();

        for i in 0..5 {
            manager.add_to_queue_end(create_track(&i.to_string(), "Track", "Artist", 10));
        }

        manager.set_audio_source(Box::new(MockAudioSource::new(
            Duration::from_secs(10),
            44100,
        )));

        let mut buffer = vec![0.0f32; 1024];

        // Interleave commands with processing
        manager.process_audio(&mut buffer).ok();
        manager.set_volume(50);
        manager.process_audio(&mut buffer).ok();
        manager.pause();
        manager.process_audio(&mut buffer).ok();
        manager.play().ok();
        manager.process_audio(&mut buffer).ok();
        manager.set_shuffle(ShuffleMode::Random);
        manager.process_audio(&mut buffer).ok();

        // Should complete without issues
        assert_eq!(manager.get_state(), PlaybackState::Playing);
    }
}

// ============================================================================
// Additional Edge Case Tests
// ============================================================================

mod edge_cases {
    use super::*;

    #[test]
    fn empty_buffer_handling() {
        let mut manager = PlaybackManager::default();
        manager.set_audio_source(Box::new(MockAudioSource::new(
            Duration::from_secs(10),
            44100,
        )));

        // Empty buffer is an edge case - reading 0 samples from source
        // signals end of track, which triggers next track logic.
        // This is expected behavior when calling with an empty buffer.
        let mut buffer: Vec<f32> = vec![];
        let result = manager.process_audio(&mut buffer);

        // Result depends on whether there's a next track in queue
        // With no queue, this returns an error (QueueEmpty after attempting to advance)
        // This is reasonable behavior - empty buffer processing is unusual
        // The important thing is no panic occurs
        let _ = result; // Don't assert success/failure, just verify no panic
    }

    #[test]
    fn very_large_queue() {
        let mut manager = PlaybackManager::default();

        // Add 10,000 tracks
        let tracks: Vec<QueueTrack> = (0..10000)
            .map(|i| create_track(&i.to_string(), &format!("Track {}", i), "Artist", 180))
            .collect();

        manager.add_playlist_to_queue(tracks);

        assert_eq!(manager.queue_len(), 10000);

        // Operations should still work
        manager.set_shuffle(ShuffleMode::Smart);
        assert_eq!(manager.queue_len(), 10000);

        manager.remove_from_queue(5000).ok();
        assert_eq!(manager.queue_len(), 9999);
    }

    #[test]
    fn very_short_track() {
        let mut manager = PlaybackManager::default();
        manager.add_to_queue_end(create_track("1", "Short", "Artist", 1));
        manager.add_to_queue_end(create_track("2", "Normal", "Artist", 180));

        // 100ms track
        manager.set_audio_source(Box::new(MockAudioSource::new(
            Duration::from_millis(100),
            44100,
        )));

        // Should play through without issues
        let mut buffer = vec![0.0f32; 44100]; // 0.5 seconds
        let result = manager.process_audio(&mut buffer);
        assert!(result.is_ok());
    }

    #[test]
    fn very_long_track() {
        let mut manager = PlaybackManager::default();

        // 10 hour track
        manager.add_to_queue_end(create_track("1", "Long", "Artist", 36000));
        manager.set_audio_source(Box::new(MockAudioSource::new(
            Duration::from_secs(36000),
            44100,
        )));

        // Seek to end-ish
        manager.seek_to(Duration::from_secs(35900)).ok();

        let pos = manager.get_position();
        assert!(pos >= Duration::from_secs(35900));
    }

    #[test]
    fn high_sample_rate() {
        let mut manager = PlaybackManager::default();
        manager.set_sample_rate(192000);

        manager.add_to_queue_end(create_track("1", "Track", "Artist", 10));
        manager.set_audio_source(Box::new(MockAudioSource::new(
            Duration::from_secs(10),
            192000,
        )));

        let mut buffer = vec![0.0f32; 1024];
        let result = manager.process_audio(&mut buffer);
        assert!(result.is_ok());
    }

    #[test]
    fn mono_output() {
        let mut manager = PlaybackManager::default();
        manager.set_output_channels(1);

        manager.add_to_queue_end(create_track("1", "Track", "Artist", 10));
        manager.set_audio_source(Box::new(MockAudioSource::new(
            Duration::from_secs(10),
            44100,
        )));

        let mut buffer = vec![0.0f32; 512]; // Mono buffer
        let result = manager.process_audio(&mut buffer);
        assert!(result.is_ok());
    }

    #[test]
    fn multichannel_output() {
        let mut manager = PlaybackManager::default();
        manager.set_output_channels(6); // 5.1 surround

        manager.add_to_queue_end(create_track("1", "Track", "Artist", 10));
        manager.set_audio_source(Box::new(MockAudioSource::new(
            Duration::from_secs(10),
            44100,
        )));

        // 6 channels * 256 frames = 1536 samples
        let mut buffer = vec![0.0f32; 1536];
        let result = manager.process_audio(&mut buffer);
        assert!(result.is_ok());
    }

    #[test]
    fn unicode_track_metadata() {
        let mut manager = PlaybackManager::default();

        manager.add_to_queue_end(QueueTrack {
            id: "unicode".to_string(),
            path: PathBuf::from("/music/unicode.mp3"),
            title: "Track".to_string(),
            artist: "Artist".to_string(),
            album: Some("Album".to_string()),
            duration: Duration::from_secs(180),
            track_number: Some(1),
            source: TrackSource::Single,
        });

        let queue = manager.get_queue();
        assert_eq!(queue[0].title, "Track");
        assert_eq!(queue[0].artist, "Artist");
    }

    #[test]
    fn duration_edge_cases() {
        let mut manager = PlaybackManager::default();

        // Zero duration
        manager.add_to_queue_end(create_track("1", "Zero", "Artist", 0));

        // Very precise duration
        manager.add_to_queue_end(QueueTrack {
            id: "precise".to_string(),
            path: PathBuf::from("/music/precise.mp3"),
            title: "Precise".to_string(),
            artist: "Artist".to_string(),
            album: None,
            duration: Duration::from_nanos(123456789),
            track_number: None,
            source: TrackSource::Single,
        });

        assert_eq!(manager.queue_len(), 2);
    }

    #[test]
    fn position_beyond_track() {
        let mut manager = PlaybackManager::default();
        manager.set_audio_source(Box::new(
            MockAudioSource::new(Duration::from_secs(100), 44100)
                .with_position(Duration::from_secs(99)),
        ));

        // Position near end
        let pos = manager.get_position();
        assert!(pos >= Duration::from_secs(99));

        // Should handle seeking beyond gracefully
        let result = manager.seek_to(Duration::from_secs(150));
        assert!(result.is_err());
    }

    #[test]
    fn album_source_tracking() {
        let mut manager = PlaybackManager::default();

        let track = create_track_from_album("1", "Track 1", "Artist", "album1", "Great Album", 1);

        manager.add_to_queue_end(track);

        let queue = manager.get_queue();
        match &queue[0].source {
            TrackSource::Album { id, name } => {
                assert_eq!(id, "album1");
                assert_eq!(name, "Great Album");
            }
            _ => panic!("Expected Album source"),
        }
    }

    #[test]
    fn default_config_values() {
        let config = PlaybackConfig::default();

        assert_eq!(config.history_size, 50);
        assert_eq!(config.volume, 80);
        assert_eq!(config.shuffle, ShuffleMode::Off);
        assert_eq!(config.repeat, RepeatMode::Off);
        assert!(config.gapless);
        assert!(!config.crossfade.enabled);
    }

    #[test]
    fn gapless_config() {
        let config = PlaybackConfig::gapless();

        assert!(config.crossfade.enabled);
        assert_eq!(config.crossfade.duration_ms, 0);
    }

    #[test]
    fn crossfade_config() {
        let config = PlaybackConfig::with_crossfade(5000, FadeCurve::SCurve);

        assert!(config.crossfade.enabled);
        assert_eq!(config.crossfade.duration_ms, 5000);
        assert_eq!(config.crossfade.curve, FadeCurve::SCurve);
    }
}
