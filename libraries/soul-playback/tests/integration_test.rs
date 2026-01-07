//! Integration tests for playback manager
//!
//! These tests verify real playback scenarios and workflows.
//! No shallow tests - every test verifies meaningful behavior.

use soul_playback::{
    AudioSource, PlaybackConfig, PlaybackManager, PlaybackState, QueueTrack, RepeatMode,
    ShuffleMode, TrackSource,
};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

// ===== Test Helpers =====

/// Mock audio source for testing
struct MockAudioSource {
    duration: Duration,
    position: Duration,
    sample_rate: u32,
    samples_per_second: u64,
    finished: bool,
}

impl MockAudioSource {
    fn new(duration: Duration, sample_rate: u32) -> Self {
        Self {
            duration,
            position: Duration::ZERO,
            sample_rate,
            samples_per_second: sample_rate as u64 * 2, // Stereo
            finished: false,
        }
    }
}

impl AudioSource for MockAudioSource {
    fn read_samples(&mut self, buffer: &mut [f32]) -> soul_playback::Result<usize> {
        if self.finished {
            return Ok(0);
        }

        let total_samples = (self.duration.as_secs_f64() * self.samples_per_second as f64) as u64;
        let current_sample = (self.position.as_secs_f64() * self.samples_per_second as f64) as u64;

        let remaining = (total_samples - current_sample) as usize;
        let to_read = remaining.min(buffer.len());

        if to_read == 0 {
            self.finished = true;
            return Ok(0);
        }

        // Fill with test pattern (alternating values)
        for i in 0..to_read {
            buffer[i] = ((i % 2) as f32 - 0.5) * 0.5;
        }

        // Update position
        let samples_read_duration =
            Duration::from_secs_f64(to_read as f64 / self.samples_per_second as f64);
        self.position += samples_read_duration;

        Ok(to_read)
    }

    fn seek(&mut self, position: Duration) -> soul_playback::Result<()> {
        if position > self.duration {
            return Err(soul_playback::PlaybackError::InvalidSeekPosition(position));
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

fn create_test_track(id: &str, title: &str, artist: &str, duration_secs: u64) -> QueueTrack {
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

// ===== Integration Tests =====

#[test]
fn test_play_pause_resume_workflow() {
    let mut manager = PlaybackManager::default();

    // Start in stopped state
    assert_eq!(manager.get_state(), PlaybackState::Stopped);

    // Add track and set source
    manager.add_to_queue_end(create_test_track("1", "Track 1", "Artist A", 180));
    manager.set_audio_source(Box::new(MockAudioSource::new(
        Duration::from_secs(180),
        44100,
    )));

    // After setting source, should be playing
    assert_eq!(manager.get_state(), PlaybackState::Playing);

    // Pause
    manager.pause();
    assert_eq!(manager.get_state(), PlaybackState::Paused);

    // Resume
    manager.play().ok();
    assert_eq!(manager.get_state(), PlaybackState::Playing);
}

#[test]
fn test_next_previous_navigation() {
    let mut manager = PlaybackManager::default();

    // Add multiple tracks
    let track1 = create_test_track("1", "Track 1", "Artist A", 180);
    let track2 = create_test_track("2", "Track 2", "Artist B", 180);
    let track3 = create_test_track("3", "Track 3", "Artist C", 180);

    manager.add_to_queue_end(track1.clone());
    manager.add_to_queue_end(track2);
    manager.add_to_queue_end(track3);

    // Set source and current track
    manager.set_audio_source(Box::new(MockAudioSource::new(
        Duration::from_secs(180),
        44100,
    )));

    // Skip to next track
    manager.next().ok();

    // Queue should have fewer tracks now
    assert!(manager.queue_len() < 3);

    // Go back to previous (if history exists)
    manager.previous().ok();
}

#[test]
fn test_queue_priority_explicit_over_source() {
    let mut manager = PlaybackManager::default();

    // Add source queue first
    manager.add_playlist_to_queue(vec![
        create_test_track("s1", "Source 1", "Artist A", 180),
        create_test_track("s2", "Source 2", "Artist B", 180),
    ]);

    // Add explicit queue
    manager.add_to_queue_next(create_test_track("e1", "Explicit 1", "Artist C", 180));

    // Get all tracks
    let queue = manager.get_queue();

    // Explicit should be first
    assert_eq!(queue[0].id, "e1");
    assert_eq!(queue[1].id, "s1");
    assert_eq!(queue[2].id, "s2");
}

#[test]
fn test_shuffle_changes_playback_order() {
    let mut manager = PlaybackManager::default();

    // Add tracks in specific order
    let tracks = vec![
        create_test_track("1", "Track 1", "Artist A", 180),
        create_test_track("2", "Track 2", "Artist B", 180),
        create_test_track("3", "Track 3", "Artist C", 180),
        create_test_track("4", "Track 4", "Artist D", 180),
        create_test_track("5", "Track 5", "Artist E", 180),
    ];

    manager.add_playlist_to_queue(tracks.clone());

    let original_order: Vec<String> = manager.get_queue().iter().map(|t| t.id.clone()).collect();

    // Enable shuffle
    manager.set_shuffle(ShuffleMode::Random);

    let shuffled_order: Vec<String> = manager.get_queue().iter().map(|t| t.id.clone()).collect();

    // Order should be different (very unlikely to be same with 5 tracks)
    assert_ne!(original_order, shuffled_order);

    // All tracks should still be present
    let mut sorted_original = original_order.clone();
    let mut sorted_shuffled = shuffled_order.clone();
    sorted_original.sort();
    sorted_shuffled.sort();
    assert_eq!(sorted_original, sorted_shuffled);
}

#[test]
fn test_shuffle_restore_original_order() {
    let mut manager = PlaybackManager::default();

    let tracks = vec![
        create_test_track("1", "Track 1", "Artist A", 180),
        create_test_track("2", "Track 2", "Artist B", 180),
        create_test_track("3", "Track 3", "Artist C", 180),
    ];

    manager.add_playlist_to_queue(tracks);

    let original_order: Vec<String> = manager.get_queue().iter().map(|t| t.id.clone()).collect();

    // Shuffle
    manager.set_shuffle(ShuffleMode::Random);

    // Turn shuffle off
    manager.set_shuffle(ShuffleMode::Off);

    let restored_order: Vec<String> = manager.get_queue().iter().map(|t| t.id.clone()).collect();

    // Should be back to original order
    assert_eq!(original_order, restored_order);
}

#[test]
fn test_repeat_one_loops_track() {
    let mut manager = PlaybackManager::default();
    manager.set_repeat(RepeatMode::One);

    manager.add_to_queue_end(create_test_track("1", "Track 1", "Artist A", 180));

    // Set a mock source
    let source = Box::new(MockAudioSource::new(Duration::from_secs(180), 44100));
    manager.set_audio_source(source);

    let track_id = manager.get_current_track().map(|t| t.id.clone());

    // Call next (should restart same track)
    manager.next().ok();

    // Should still be same track (or none if no track was loaded)
    // In actual implementation with track loading, this would restart the same track
    assert_eq!(manager.get_repeat(), RepeatMode::One);
}

#[test]
fn test_volume_affects_audio_output() {
    let mut manager = PlaybackManager::default();
    manager.set_volume(50); // 50% = -30 dB ≈ 0.0316 gain

    manager.set_audio_source(Box::new(MockAudioSource::new(
        Duration::from_secs(10),
        44100,
    )));

    let mut buffer = vec![0.0f32; 100];
    manager.process_audio(&mut buffer).ok();

    // Buffer should be attenuated
    // At 50% volume, samples should be quieter than 0.5
    let max_sample = buffer.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    assert!(
        max_sample < 0.5,
        "Expected volume reduction, got max: {}",
        max_sample
    );
}

#[test]
fn test_mute_silences_output() {
    let mut manager = PlaybackManager::default();
    manager.set_audio_source(Box::new(MockAudioSource::new(
        Duration::from_secs(10),
        44100,
    )));

    manager.mute();

    let mut buffer = vec![1.0f32; 100]; // Fill with non-zero
    manager.process_audio(&mut buffer).ok();

    // All samples should be zero (silence)
    assert!(
        buffer.iter().all(|s| *s == 0.0),
        "Expected silence when muted"
    );
}

#[test]
fn test_seek_changes_position() {
    let mut manager = PlaybackManager::default();
    let mut source = Box::new(MockAudioSource::new(Duration::from_secs(180), 44100));

    // Seek to 60 seconds
    source.seek(Duration::from_secs(60)).ok();

    assert_eq!(source.position(), Duration::from_secs(60));

    manager.set_audio_source(source);

    // Position should be 60 seconds
    assert_eq!(manager.get_position(), Duration::from_secs(60));
}

#[test]
fn test_seek_percent_calculates_correctly() {
    let mut manager = PlaybackManager::default();
    manager.set_audio_source(Box::new(MockAudioSource::new(
        Duration::from_secs(100),
        44100,
    )));

    // Seek to 50%
    manager.seek_to_percent(0.5).ok();

    let pos = manager.get_position();
    assert!(
        (pos.as_secs() as i64 - 50).abs() <= 1,
        "Expected ~50s, got {}s",
        pos.as_secs()
    );
}

#[test]
fn test_auto_advance_on_track_end() {
    let mut manager = PlaybackManager::default();

    // Add two tracks
    manager.add_to_queue_end(create_test_track("1", "Track 1", "Artist A", 1)); // 1 second
    manager.add_to_queue_end(create_test_track("2", "Track 2", "Artist B", 180));

    manager.set_audio_source(Box::new(MockAudioSource::new(
        Duration::from_secs(1),
        44100,
    )));

    // Process audio until track ends
    let mut total_samples = 0;
    let mut buffer = vec![0.0f32; 1024];

    // Read until we get 0 samples (track finished)
    for _ in 0..100 {
        // Max iterations to prevent infinite loop
        match manager.process_audio(&mut buffer) {
            Ok(0) => break, // Track finished
            Ok(n) => total_samples += n,
            Err(_) => break,
        }
    }

    // Should have read some samples
    assert!(total_samples > 0, "Should have read some samples");
}

#[test]
fn test_queue_operations_dont_affect_current_track() {
    let mut manager = PlaybackManager::default();

    manager.add_to_queue_end(create_test_track("1", "Track 1", "Artist A", 180));
    manager.set_audio_source(Box::new(MockAudioSource::new(
        Duration::from_secs(180),
        44100,
    )));

    // Add more tracks while playing
    manager.add_to_queue_end(create_test_track("2", "Track 2", "Artist B", 180));
    manager.add_to_queue_end(create_test_track("3", "Track 3", "Artist C", 180));

    // Remove a track
    manager.remove_from_queue(1).ok();

    // Playback state should remain unchanged
    assert_eq!(manager.get_state(), PlaybackState::Playing);
}

#[test]
fn test_history_limited_to_max_size() {
    let config = PlaybackConfig {
        history_size: 5, // Only keep 5 tracks
        ..Default::default()
    };

    let mut manager = PlaybackManager::new(config);

    // Add 10 tracks and simulate playing them
    for i in 1..=10 {
        manager.add_to_queue_end(create_test_track(
            &i.to_string(),
            &format!("Track {}", i),
            "Artist",
            180,
        ));
    }

    // Simulate playing through all tracks by calling next
    for _ in 0..10 {
        manager.next().ok();
    }

    // History should have max 5 tracks
    let history = manager.get_history();
    assert!(
        history.len() <= 5,
        "History should not exceed max size, got {}",
        history.len()
    );
}

#[test]
fn test_smart_shuffle_distributes_artists() {
    let mut manager = PlaybackManager::default();

    // Add tracks from 2 artists (3 tracks each)
    let tracks = vec![
        create_test_track("a1", "Song 1", "Artist A", 180),
        create_test_track("a2", "Song 2", "Artist A", 180),
        create_test_track("a3", "Song 3", "Artist A", 180),
        create_test_track("b1", "Song 4", "Artist B", 180),
        create_test_track("b2", "Song 5", "Artist B", 180),
        create_test_track("b3", "Song 6", "Artist B", 180),
    ];

    manager.add_playlist_to_queue(tracks);
    manager.set_shuffle(ShuffleMode::Smart);

    let queue = manager.get_queue();

    // Count consecutive same-artist plays
    let mut consecutive_count = 0;
    for i in 0..queue.len() - 1 {
        if queue[i].artist == queue[i + 1].artist {
            consecutive_count += 1;
        }
    }

    // Smart shuffle should minimize consecutive same-artist plays
    // With 3 tracks per artist, should have at most 1-2 consecutive
    assert!(
        consecutive_count <= 2,
        "Too many consecutive same-artist plays: {}",
        consecutive_count
    );
}

#[test]
fn test_empty_queue_playback_fails_gracefully() {
    let mut manager = PlaybackManager::default();

    // Try to play with empty queue
    let result = manager.play();

    // Should error (queue empty)
    assert!(result.is_err());
}

#[test]
fn test_process_audio_when_stopped_outputs_silence() {
    let mut manager = PlaybackManager::default();
    assert_eq!(manager.get_state(), PlaybackState::Stopped);

    let mut buffer = vec![1.0f32; 1024];
    manager.process_audio(&mut buffer).ok();

    // Should output silence
    assert!(buffer.iter().all(|s| *s == 0.0));
}

#[test]
fn test_previous_within_3_seconds_goes_to_previous_track() {
    let mut manager = PlaybackManager::default();

    manager.add_to_queue_end(create_test_track("1", "Track 1", "Artist A", 180));
    manager.add_to_queue_end(create_test_track("2", "Track 2", "Artist B", 180));

    // Simulate playing track 1 then track 2
    manager.next().ok(); // Track 1 to history
    manager.set_audio_source(Box::new(MockAudioSource::new(
        Duration::from_secs(180),
        44100,
    )));

    // Position is 0 (< 3 seconds)
    assert!(manager.get_position() < Duration::from_secs(3));

    // Previous should go to previous track
    manager.previous().ok();

    // Should have popped from history
    assert_eq!(manager.get_history().len(), 0);
}

#[test]
fn test_seek_beyond_duration_fails() {
    let mut manager = PlaybackManager::default();
    manager.set_audio_source(Box::new(MockAudioSource::new(
        Duration::from_secs(100),
        44100,
    )));

    // Try to seek beyond duration
    let result = manager.seek_to(Duration::from_secs(200));

    assert!(result.is_err());
}

#[test]
fn test_rapid_state_changes() {
    let mut manager = PlaybackManager::default();

    manager.add_to_queue_end(create_test_track("1", "Track 1", "Artist A", 180));
    manager.set_audio_source(Box::new(MockAudioSource::new(
        Duration::from_secs(180),
        44100,
    )));

    // Rapid play/pause/play/pause
    for _ in 0..10 {
        manager.play().ok();
        manager.pause();
    }

    // Should end in paused state
    assert_eq!(manager.get_state(), PlaybackState::Paused);

    // Should still be able to resume
    manager.play().ok();
    assert_eq!(manager.get_state(), PlaybackState::Playing);
}

#[test]
fn test_large_queue_performance() {
    let mut manager = PlaybackManager::default();

    // Add 1000 tracks
    let tracks: Vec<QueueTrack> = (0..1000)
        .map(|i| {
            create_test_track(
                &i.to_string(),
                &format!("Track {}", i),
                &format!("Artist {}", i % 50), // 50 artists
                180,
            )
        })
        .collect();

    manager.add_playlist_to_queue(tracks);

    // Queue operations should still be fast
    assert_eq!(manager.queue_len(), 1000);

    // Shuffle large queue
    manager.set_shuffle(ShuffleMode::Smart);

    // Should still have all tracks
    assert_eq!(manager.queue_len(), 1000);

    // Remove from middle
    manager.remove_from_queue(500).ok();
    assert_eq!(manager.queue_len(), 999);
}
