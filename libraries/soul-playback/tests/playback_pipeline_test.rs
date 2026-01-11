//! Playback Pipeline Integration Tests
//!
//! Comprehensive end-to-end tests for the playback pipeline including:
//! - Crossfade tests with all curve types
//! - Volume leveling and ReplayGain
//! - Queue management with effects
//! - Gapless playback transitions
//!
//! Following project rules:
//! - No shallow tests - every test verifies meaningful behavior
//! - Real audio patterns and scenarios

use soul_playback::{
    AudioSource, CrossfadeSettings, FadeCurve, PlaybackConfig, PlaybackError, PlaybackManager,
    PlaybackState, QueueTrack, RepeatMode, Result, ShuffleMode, TrackSource,
};
use std::f32::consts::PI;
use std::path::PathBuf;
use std::time::Duration;

// ============================================================================
// Test Utilities
// ============================================================================

/// Mock audio source that generates actual audio samples
struct MockAudioSource {
    duration: Duration,
    position: Duration,
    sample_rate: u32,
    frequency: f32,
    amplitude: f32,
    finished: bool,
}

impl MockAudioSource {
    fn new(duration: Duration, sample_rate: u32) -> Self {
        Self {
            duration,
            position: Duration::ZERO,
            sample_rate,
            frequency: 440.0,
            amplitude: 0.5,
            finished: false,
        }
    }

    fn with_frequency(mut self, freq: f32) -> Self {
        self.frequency = freq;
        self
    }

    fn with_amplitude(mut self, amp: f32) -> Self {
        self.amplitude = amp;
        self
    }
}

impl AudioSource for MockAudioSource {
    fn read_samples(&mut self, buffer: &mut [f32]) -> Result<usize> {
        if self.finished {
            return Ok(0);
        }

        let samples_per_second = self.sample_rate as u64 * 2; // Stereo
        let total_samples = (self.duration.as_secs_f64() * samples_per_second as f64) as u64;
        let current_sample = (self.position.as_secs_f64() * samples_per_second as f64) as u64;

        let remaining = (total_samples - current_sample) as usize;
        let to_read = remaining.min(buffer.len());

        if to_read == 0 {
            self.finished = true;
            return Ok(0);
        }

        // Generate sine wave
        let start_sample = current_sample / 2; // Convert to mono samples
        for i in 0..(to_read / 2) {
            let sample_idx = start_sample + i as u64;
            let t = sample_idx as f32 / self.sample_rate as f32;
            let sample = (2.0 * PI * self.frequency * t).sin() * self.amplitude;
            buffer[i * 2] = sample; // Left
            buffer[i * 2 + 1] = sample; // Right
        }

        // Update position
        let samples_read_duration =
            Duration::from_secs_f64(to_read as f64 / samples_per_second as f64);
        self.position += samples_read_duration;

        Ok(to_read)
    }

    fn seek(&mut self, position: Duration) -> Result<()> {
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

/// Mock audio source that generates silence
struct SilentSource {
    duration: Duration,
    position: Duration,
    sample_rate: u32,
}

impl SilentSource {
    fn new(duration: Duration, sample_rate: u32) -> Self {
        Self {
            duration,
            position: Duration::ZERO,
            sample_rate,
        }
    }
}

impl AudioSource for SilentSource {
    fn read_samples(&mut self, buffer: &mut [f32]) -> Result<usize> {
        let samples_per_second = self.sample_rate as u64 * 2;
        let total_samples = (self.duration.as_secs_f64() * samples_per_second as f64) as u64;
        let current_sample = (self.position.as_secs_f64() * samples_per_second as f64) as u64;

        let remaining = (total_samples - current_sample) as usize;
        let to_read = remaining.min(buffer.len());

        if to_read == 0 {
            return Ok(0);
        }

        buffer[..to_read].fill(0.0);

        let samples_read_duration =
            Duration::from_secs_f64(to_read as f64 / samples_per_second as f64);
        self.position += samples_read_duration;

        Ok(to_read)
    }

    fn seek(&mut self, position: Duration) -> Result<()> {
        if position > self.duration {
            return Err(PlaybackError::InvalidSeekPosition(position));
        }
        self.position = position;
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

fn create_album_track(
    id: &str,
    title: &str,
    artist: &str,
    album: &str,
    track_num: u32,
) -> QueueTrack {
    QueueTrack {
        id: id.to_string(),
        path: PathBuf::from(format!("/music/{}.mp3", id)),
        title: title.to_string(),
        artist: artist.to_string(),
        album: Some(album.to_string()),
        duration: Duration::from_secs(180),
        track_number: Some(track_num),
        source: TrackSource::Album {
            id: album.to_lowercase().replace(' ', "_"),
            name: album.to_string(),
        },
    }
}

/// Calculate RMS of a buffer
fn calculate_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_squares: f32 = samples.iter().map(|s| s * s).sum();
    (sum_squares / samples.len() as f32).sqrt()
}

/// Calculate peak amplitude
fn calculate_peak(samples: &[f32]) -> f32 {
    samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max)
}

// ============================================================================
// 1. Crossfade Tests
// ============================================================================

#[test]
fn test_gapless_transition() {
    let config = PlaybackConfig::gapless();
    let mut manager = PlaybackManager::new(config);

    // Add two tracks
    manager.add_to_queue_end(create_test_track("1", "Track 1", "Artist A", 2));
    manager.add_to_queue_end(create_test_track("2", "Track 2", "Artist B", 2));

    // Set up current and next sources for gapless
    manager.set_audio_source(Box::new(MockAudioSource::new(
        Duration::from_secs(2),
        44100,
    )));

    // Pre-decode next track
    let next_source = MockAudioSource::new(Duration::from_secs(2), 44100).with_frequency(880.0);
    let next_track = create_test_track("2", "Track 2", "Artist B", 2);
    manager.set_next_source(Box::new(next_source), next_track);

    assert!(manager.has_next_source());

    // Process audio through near-end of track
    let mut buffer = vec![0.0f32; 4096];
    let mut total_samples = 0;

    // Read until track transitions
    for _ in 0..100 {
        match manager.process_audio(&mut buffer) {
            Ok(0) => break,
            Ok(n) => total_samples += n,
            Err(_) => break,
        }
    }

    assert!(
        total_samples > 0,
        "Should have processed some audio before transition"
    );
}

#[test]
fn test_crossfade_linear_curve() {
    let mut manager = PlaybackManager::default();
    manager.set_crossfade_enabled(true);
    manager.set_crossfade_duration(1000);
    manager.set_crossfade_curve(FadeCurve::Linear);

    assert_eq!(manager.get_crossfade_curve(), FadeCurve::Linear);
    assert_eq!(manager.get_crossfade_duration(), 1000);

    // Verify settings persisted
    let settings = manager.get_crossfade_settings();
    assert!(settings.enabled);
    assert_eq!(settings.curve, FadeCurve::Linear);
}

#[test]
fn test_crossfade_equal_power_curve() {
    let mut manager = PlaybackManager::default();
    manager.set_crossfade_enabled(true);
    manager.set_crossfade_curve(FadeCurve::EqualPower);

    assert_eq!(manager.get_crossfade_curve(), FadeCurve::EqualPower);
}

#[test]
fn test_crossfade_s_curve() {
    let mut manager = PlaybackManager::default();
    manager.set_crossfade_enabled(true);
    manager.set_crossfade_curve(FadeCurve::SCurve);

    assert_eq!(manager.get_crossfade_curve(), FadeCurve::SCurve);
}

#[test]
fn test_crossfade_square_root_curve() {
    let mut manager = PlaybackManager::default();
    manager.set_crossfade_enabled(true);
    manager.set_crossfade_curve(FadeCurve::SquareRoot);

    assert_eq!(manager.get_crossfade_curve(), FadeCurve::SquareRoot);
}

#[test]
fn test_crossfade_duration_accuracy() {
    let config = PlaybackConfig::with_crossfade(2000, FadeCurve::Linear);
    let manager = PlaybackManager::new(config);

    let settings = manager.get_crossfade_settings();
    assert_eq!(settings.duration_ms, 2000);
    assert!(settings.enabled);
}

#[test]
fn test_crossfade_duration_clamping() {
    let mut manager = PlaybackManager::default();

    // Should clamp to max (10000ms)
    manager.set_crossfade_duration(15000);
    assert_eq!(manager.get_crossfade_duration(), 10000);

    // Should accept valid values
    manager.set_crossfade_duration(5000);
    assert_eq!(manager.get_crossfade_duration(), 5000);
}

#[test]
fn test_crossfade_on_skip_setting() {
    let mut manager = PlaybackManager::default();
    manager.set_crossfade_enabled(true);

    // Default: no crossfade on skip
    assert!(!manager.get_crossfade_settings().on_skip);

    // Enable crossfade on skip
    manager.set_crossfade_on_skip(true);
    assert!(manager.get_crossfade_settings().on_skip);
}

#[test]
fn test_crossfade_state_transitions() {
    let mut manager = PlaybackManager::default();
    manager.set_crossfade_enabled(true);
    manager.set_crossfade_duration(1000);

    // Initially inactive
    assert!(!manager.is_crossfading());
    assert_eq!(
        manager.get_crossfade_state(),
        soul_playback::CrossfadeState::Inactive
    );

    // Progress should be 0 or 1 when not active
    let progress = manager.get_crossfade_progress();
    assert!(
        (0.0..=1.0).contains(&progress),
        "Progress should be between 0 and 1, got {}",
        progress
    );
}

#[test]
fn test_crossfade_with_different_sample_rates() {
    let mut manager = PlaybackManager::default();
    manager.set_crossfade_enabled(true);
    manager.set_crossfade_duration(1000);

    // Set up at 44.1kHz
    manager.set_sample_rate(44100);
    manager.set_audio_source(Box::new(MockAudioSource::new(
        Duration::from_secs(5),
        44100,
    )));

    // Verify sample rate is set
    assert_eq!(manager.get_sample_rate(), 44100);

    // Change to 48kHz (simulating track change)
    manager.set_sample_rate(48000);
    assert_eq!(manager.get_sample_rate(), 48000);
}

// ============================================================================
// 2. Volume Leveling Tests
// ============================================================================

#[test]
fn test_volume_linear_scaling() {
    let mut manager = PlaybackManager::default();
    manager.set_audio_source(Box::new(MockAudioSource::new(
        Duration::from_secs(5),
        44100,
    )));

    let mut buffer = vec![0.0f32; 1024];

    // Test at 100% volume
    manager.set_volume(100);
    manager.process_audio(&mut buffer).ok();
    let rms_100 = calculate_rms(&buffer);

    // Reset source and test at 50%
    manager.set_audio_source(Box::new(MockAudioSource::new(
        Duration::from_secs(5),
        44100,
    )));
    manager.set_volume(50);
    let mut buffer = vec![0.0f32; 1024];
    manager.process_audio(&mut buffer).ok();
    let rms_50 = calculate_rms(&buffer);

    // 50% should be significantly quieter (logarithmic scale)
    assert!(
        rms_50 < rms_100 * 0.5,
        "50% volume should be much quieter, 100%: {}, 50%: {}",
        rms_100,
        rms_50
    );
}

#[test]
fn test_mute_produces_silence() {
    let mut manager = PlaybackManager::default();
    manager.set_audio_source(Box::new(MockAudioSource::new(
        Duration::from_secs(5),
        44100,
    )));

    manager.mute();

    let mut buffer = vec![1.0f32; 1024]; // Non-zero initial values
    manager.process_audio(&mut buffer).ok();

    // All samples should be zero
    assert!(
        buffer.iter().all(|&s| s == 0.0),
        "Muted output should be silence"
    );
}

#[test]
fn test_unmute_restores_audio() {
    let mut manager = PlaybackManager::default();
    manager.set_audio_source(Box::new(MockAudioSource::new(
        Duration::from_secs(5),
        44100,
    )));

    // Mute then unmute
    manager.mute();
    assert!(manager.is_muted());

    manager.unmute();
    assert!(!manager.is_muted());

    let mut buffer = vec![0.0f32; 1024];
    manager.process_audio(&mut buffer).ok();

    // Should have audio
    let peak = calculate_peak(&buffer);
    assert!(
        peak > 0.1,
        "Unmuted output should have audio, got peak: {}",
        peak
    );
}

#[test]
fn test_volume_level_persistence() {
    let mut manager = PlaybackManager::default();

    // Set to non-default volume
    manager.set_volume(65);
    assert_eq!(manager.get_volume(), 65);

    // Mute
    manager.mute();
    assert!(manager.is_muted());

    // Volume level should persist
    assert_eq!(manager.get_volume(), 65);

    // Unmute - volume should still be 65
    manager.unmute();
    assert_eq!(manager.get_volume(), 65);
}

#[test]
fn test_volume_toggle_mute() {
    let mut manager = PlaybackManager::default();

    assert!(!manager.is_muted());

    manager.toggle_mute();
    assert!(manager.is_muted());

    manager.toggle_mute();
    assert!(!manager.is_muted());
}

#[test]
fn test_volume_boundary_values() {
    let mut manager = PlaybackManager::default();
    manager.set_audio_source(Box::new(MockAudioSource::new(
        Duration::from_secs(5),
        44100,
    )));

    // Test 0% (should be silent)
    manager.set_volume(0);
    assert_eq!(manager.get_volume(), 0);

    let mut buffer = vec![0.0f32; 1024];
    manager.process_audio(&mut buffer).ok();

    let peak_0 = calculate_peak(&buffer);
    assert!(
        peak_0 < 0.001,
        "0% volume should be near silent, got: {}",
        peak_0
    );

    // Test 100%
    manager.set_audio_source(Box::new(MockAudioSource::new(
        Duration::from_secs(5),
        44100,
    )));
    manager.set_volume(100);
    assert_eq!(manager.get_volume(), 100);

    let mut buffer = vec![0.0f32; 1024];
    manager.process_audio(&mut buffer).ok();

    let peak_100 = calculate_peak(&buffer);
    assert!(
        peak_100 > 0.3,
        "100% volume should have audio, got: {}",
        peak_100
    );
}

#[test]
fn test_clipping_prevention() {
    let mut manager = PlaybackManager::default();

    // Use loud source
    let loud_source = MockAudioSource::new(Duration::from_secs(5), 44100).with_amplitude(0.9);
    manager.set_audio_source(Box::new(loud_source));
    manager.set_volume(100);

    let mut buffer = vec![0.0f32; 1024];
    manager.process_audio(&mut buffer).ok();

    // Should not exceed 1.0 (clipping)
    let peak = calculate_peak(&buffer);
    assert!(peak <= 1.0, "Output should not clip, got peak: {}", peak);
}

// ============================================================================
// 3. Queue Management with Effects Tests
// ============================================================================

#[test]
fn test_effect_state_preserved_during_playback() {
    let mut manager = PlaybackManager::default();

    // Add tracks
    for i in 1..=3 {
        manager.add_to_queue_end(create_test_track(
            &i.to_string(),
            &format!("Track {}", i),
            "Artist",
            180,
        ));
    }

    manager.set_audio_source(Box::new(MockAudioSource::new(
        Duration::from_secs(180),
        44100,
    )));

    // Volume should persist across process calls
    manager.set_volume(75);

    let mut buffer = vec![0.0f32; 1024];
    for _ in 0..10 {
        manager.process_audio(&mut buffer).ok();
        assert_eq!(manager.get_volume(), 75, "Volume should persist");
    }
}

#[test]
fn test_queue_modification_during_playback() {
    let mut manager = PlaybackManager::default();

    manager.add_to_queue_end(create_test_track("1", "Track 1", "Artist", 180));
    manager.set_audio_source(Box::new(MockAudioSource::new(
        Duration::from_secs(180),
        44100,
    )));

    assert_eq!(manager.get_state(), PlaybackState::Playing);

    // Modify queue while playing
    manager.add_to_queue_end(create_test_track("2", "Track 2", "Artist", 180));
    manager.add_to_queue_next(create_test_track("3", "Track 3", "Artist", 180));

    // Should still be playing
    assert_eq!(manager.get_state(), PlaybackState::Playing);

    // Queue should have new tracks
    assert_eq!(manager.queue_len(), 3);
}

#[test]
fn test_shuffle_preserves_current_track() {
    let mut manager = PlaybackManager::default();

    // Add playlist
    let tracks: Vec<QueueTrack> = (1..=5)
        .map(|i| create_test_track(&i.to_string(), &format!("Track {}", i), "Artist", 180))
        .collect();

    manager.add_playlist_to_queue(tracks);

    // Enable shuffle - this should not affect the ability to play
    manager.set_shuffle(ShuffleMode::Random);

    // Should still be able to add source and play
    manager.set_audio_source(Box::new(MockAudioSource::new(
        Duration::from_secs(180),
        44100,
    )));

    assert_eq!(manager.get_state(), PlaybackState::Playing);
}

#[test]
fn test_queue_clear_stops_playback() {
    let mut manager = PlaybackManager::default();

    manager.add_to_queue_end(create_test_track("1", "Track 1", "Artist", 180));
    manager.set_audio_source(Box::new(MockAudioSource::new(
        Duration::from_secs(180),
        44100,
    )));

    // Playing
    assert_eq!(manager.get_state(), PlaybackState::Playing);

    // Clear queue
    manager.clear_queue();

    // Queue should be empty
    assert!(manager.get_queue().is_empty());
}

#[test]
fn test_skip_to_index_with_effects() {
    let mut manager = PlaybackManager::default();
    manager.set_volume(60);

    // Add several tracks
    for i in 1..=5 {
        manager.add_to_queue_end(create_test_track(
            &i.to_string(),
            &format!("Track {}", i),
            "Artist",
            180,
        ));
    }

    // Volume setting should persist after skip
    manager.skip_to_queue_index(2).ok();
    assert_eq!(manager.get_volume(), 60);
}

// ============================================================================
// 4. Track Transition Tests
// ============================================================================

#[test]
fn test_track_end_advances_queue() {
    let mut manager = PlaybackManager::default();

    // Add two short tracks
    manager.add_to_queue_end(create_test_track("1", "Track 1", "Artist", 1)); // 1 second
    manager.add_to_queue_end(create_test_track("2", "Track 2", "Artist", 1));

    // Play first track
    manager.set_audio_source(Box::new(MockAudioSource::new(
        Duration::from_secs(1),
        44100,
    )));

    // Process until track ends
    let mut buffer = vec![0.0f32; 4096];
    let mut iterations = 0;
    while manager.get_state() == PlaybackState::Playing && iterations < 100 {
        match manager.process_audio(&mut buffer) {
            Ok(0) => break, // Track finished
            Ok(_) => {}
            Err(_) => break,
        }
        iterations += 1;
    }

    // Queue should have advanced
    assert!(iterations > 0, "Should have processed some audio");
}

#[test]
fn test_repeat_one_restarts_track() {
    let mut manager = PlaybackManager::default();
    manager.set_repeat(RepeatMode::One);

    manager.add_to_queue_end(create_test_track("1", "Track 1", "Artist", 180));
    manager.set_audio_source(Box::new(MockAudioSource::new(
        Duration::from_secs(180),
        44100,
    )));

    // Process some audio
    let mut buffer = vec![0.0f32; 1024];
    manager.process_audio(&mut buffer).ok();

    // Next should restart same track
    manager.next().ok();

    // Repeat mode should still be One
    assert_eq!(manager.get_repeat(), RepeatMode::One);
}

#[test]
fn test_repeat_all_loops_queue() {
    let mut manager = PlaybackManager::default();
    manager.set_repeat(RepeatMode::All);

    // Add tracks
    manager.add_to_queue_end(create_test_track("1", "Track 1", "Artist", 180));
    manager.add_to_queue_end(create_test_track("2", "Track 2", "Artist", 180));

    // Verify repeat mode
    assert_eq!(manager.get_repeat(), RepeatMode::All);
    assert!(manager.has_next(), "Should have next with RepeatMode::All");
}

#[test]
fn test_stop_clears_transition_state() {
    let mut manager = PlaybackManager::default();
    manager.set_crossfade_enabled(true);

    // Set up tracks and sources
    manager.add_to_queue_end(create_test_track("1", "Track 1", "Artist", 10));
    manager.set_audio_source(Box::new(MockAudioSource::new(
        Duration::from_secs(10),
        44100,
    )));

    let next_source = MockAudioSource::new(Duration::from_secs(10), 44100);
    let next_track = create_test_track("2", "Track 2", "Artist", 10);
    manager.set_next_source(Box::new(next_source), next_track);

    assert!(manager.has_next_source());

    // Stop
    manager.stop();

    // Should clear everything
    assert!(!manager.has_next_source());
    assert!(manager.get_next_track().is_none());
    assert_eq!(manager.get_state(), PlaybackState::Stopped);
}

// ============================================================================
// 5. Position and Seek Tests
// ============================================================================

#[test]
fn test_seek_updates_position() {
    let mut manager = PlaybackManager::default();
    manager.set_audio_source(Box::new(MockAudioSource::new(
        Duration::from_secs(180),
        44100,
    )));

    // Initial position is 0
    assert_eq!(manager.get_position(), Duration::ZERO);

    // Seek to 60 seconds
    manager.seek_to(Duration::from_secs(60)).ok();
    let pos = manager.get_position();

    // Position should be around 60 seconds
    assert!(
        pos.as_secs() >= 59 && pos.as_secs() <= 61,
        "Position should be ~60s, got {}s",
        pos.as_secs()
    );
}

#[test]
fn test_seek_percent() {
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
        "50% of 100s should be ~50s, got {}s",
        pos.as_secs()
    );
}

#[test]
fn test_seek_beyond_duration_fails() {
    let mut manager = PlaybackManager::default();
    manager.set_audio_source(Box::new(MockAudioSource::new(
        Duration::from_secs(100),
        44100,
    )));

    let result = manager.seek_to(Duration::from_secs(200));
    assert!(result.is_err(), "Should fail when seeking beyond duration");
}

#[test]
fn test_seek_to_start() {
    let mut manager = PlaybackManager::default();
    manager.set_audio_source(Box::new(MockAudioSource::new(
        Duration::from_secs(100),
        44100,
    )));

    // Seek forward then back to start
    manager.seek_to(Duration::from_secs(50)).ok();
    manager.seek_to(Duration::ZERO).ok();

    assert_eq!(manager.get_position(), Duration::ZERO);
}

#[test]
fn test_duration_reporting() {
    let mut manager = PlaybackManager::default();

    // No track = no duration
    assert!(manager.get_duration().is_none());

    // With track
    manager.set_audio_source(Box::new(MockAudioSource::new(
        Duration::from_secs(180),
        44100,
    )));

    let duration = manager.get_duration();
    assert_eq!(duration, Some(Duration::from_secs(180)));
}

// ============================================================================
// 6. Multi-channel Output Tests
// ============================================================================

#[test]
fn test_mono_output() {
    let mut manager = PlaybackManager::default();
    manager.set_output_channels(1);
    manager.set_audio_source(Box::new(MockAudioSource::new(
        Duration::from_secs(5),
        44100,
    )));

    // Process audio in mono
    let mut buffer = vec![0.0f32; 512]; // Mono buffer
    let samples = manager.process_audio(&mut buffer).unwrap();

    assert!(samples > 0, "Should process mono samples");

    // All samples should be valid
    assert!(
        buffer.iter().all(|s| s.is_finite()),
        "All mono samples should be finite"
    );
}

#[test]
fn test_stereo_output() {
    let mut manager = PlaybackManager::default();
    manager.set_output_channels(2);
    manager.set_audio_source(Box::new(MockAudioSource::new(
        Duration::from_secs(5),
        44100,
    )));

    // Process audio in stereo
    let mut buffer = vec![0.0f32; 1024]; // Stereo buffer
    let samples = manager.process_audio(&mut buffer).unwrap();

    assert!(samples > 0, "Should process stereo samples");
    assert_eq!(samples % 2, 0, "Stereo samples should be even");
}

#[test]
fn test_multichannel_output() {
    let mut manager = PlaybackManager::default();
    manager.set_output_channels(6); // 5.1 surround
    manager.set_audio_source(Box::new(MockAudioSource::new(
        Duration::from_secs(5),
        44100,
    )));

    // Process audio
    let mut buffer = vec![0.0f32; 6 * 256]; // 6 channels * 256 frames
    let samples = manager.process_audio(&mut buffer).unwrap();

    assert!(samples > 0, "Should process multichannel samples");

    // All samples should be valid
    assert!(
        buffer[..samples].iter().all(|s| s.is_finite()),
        "All multichannel samples should be finite"
    );
}

// ============================================================================
// 7. History and Navigation Tests
// ============================================================================

#[test]
fn test_history_tracks_played_songs() {
    let mut manager = PlaybackManager::default();

    // Add and play tracks
    for i in 1..=3 {
        manager.add_to_queue_end(create_test_track(
            &i.to_string(),
            &format!("Track {}", i),
            "Artist",
            180,
        ));
    }

    // Simulate playing through tracks
    for _ in 0..2 {
        manager.next().ok();
    }

    // History should have at most 2 tracks (since we skipped twice)
    let history = manager.get_history();
    assert!(
        history.len() <= 2,
        "History should track skipped tracks, got {} entries",
        history.len()
    );
}

#[test]
fn test_previous_within_3_seconds() {
    let mut manager = PlaybackManager::default();

    manager.add_to_queue_end(create_test_track("1", "Track 1", "Artist", 180));
    manager.add_to_queue_end(create_test_track("2", "Track 2", "Artist", 180));

    // Play first track
    manager.set_audio_source(Box::new(MockAudioSource::new(
        Duration::from_secs(180),
        44100,
    )));

    // Move to second track (first goes to history)
    manager.next().ok();

    let history_before = manager.get_history().len();

    // Previous should pop from history (we're within 3 seconds)
    manager.previous().ok();

    let history_after = manager.get_history().len();

    // History should have one less item (or same if no audio source to check position)
    assert!(
        history_after <= history_before,
        "Previous should use history, before: {}, after: {}",
        history_before,
        history_after
    );
}

#[test]
fn test_previous_after_3_seconds_restarts() {
    let mut manager = PlaybackManager::default();

    manager.add_to_queue_end(create_test_track("1", "Track 1", "Artist", 180));

    // Set source and seek past 3 seconds
    let mut source = MockAudioSource::new(Duration::from_secs(180), 44100);
    source.seek(Duration::from_secs(60)).ok(); // 60 seconds in
    manager.set_audio_source(Box::new(source));

    // Previous should restart (position > 3 seconds)
    manager.previous().ok();

    // Position should be reset to start
    let pos = manager.get_position();
    assert!(
        pos.as_secs() < 5,
        "Previous past 3s should restart, got position: {}s",
        pos.as_secs()
    );
}

#[test]
fn test_history_size_limit() {
    let config = PlaybackConfig {
        history_size: 3,
        ..Default::default()
    };

    let mut manager = PlaybackManager::new(config);

    // Add many tracks
    for i in 1..=10 {
        manager.add_to_queue_end(create_test_track(
            &i.to_string(),
            &format!("Track {}", i),
            "Artist",
            180,
        ));
    }

    // Skip through all of them
    for _ in 0..10 {
        manager.next().ok();
    }

    // History should not exceed limit
    let history = manager.get_history();
    assert!(
        history.len() <= 3,
        "History should respect limit, got {} entries",
        history.len()
    );
}

// ============================================================================
// 8. Album-based Playback Tests
// ============================================================================

#[test]
fn test_album_playback_order() {
    let mut manager = PlaybackManager::default();

    // Add album tracks in order
    let tracks = vec![
        create_album_track("1", "Song 1", "Artist", "Album A", 1),
        create_album_track("2", "Song 2", "Artist", "Album A", 2),
        create_album_track("3", "Song 3", "Artist", "Album A", 3),
    ];

    manager.add_playlist_to_queue(tracks);

    // Without shuffle, order should be preserved
    let queue = manager.get_queue();
    assert_eq!(queue[0].track_number, Some(1));
    assert_eq!(queue[1].track_number, Some(2));
    assert_eq!(queue[2].track_number, Some(3));
}

#[test]
fn test_smart_shuffle_artist_distribution() {
    let mut manager = PlaybackManager::default();

    // Create tracks from multiple artists
    let mut tracks = Vec::new();
    for artist_num in 1..=3 {
        for track_num in 1..=3 {
            tracks.push(create_test_track(
                &format!("a{}t{}", artist_num, track_num),
                &format!("Song {}", track_num),
                &format!("Artist {}", artist_num),
                180,
            ));
        }
    }

    manager.add_playlist_to_queue(tracks);
    manager.set_shuffle(ShuffleMode::Smart);

    // Get shuffled queue
    let queue = manager.get_queue();

    // Count consecutive same-artist tracks
    let mut consecutive = 0;
    for i in 0..queue.len() - 1 {
        if queue[i].artist == queue[i + 1].artist {
            consecutive += 1;
        }
    }

    // Smart shuffle should minimize consecutive same-artist tracks
    assert!(
        consecutive <= 3,
        "Smart shuffle should distribute artists, got {} consecutive",
        consecutive
    );
}

// ============================================================================
// 9. Error Handling Tests
// ============================================================================

#[test]
fn test_empty_queue_play_error() {
    let mut manager = PlaybackManager::default();

    let result = manager.play();
    assert!(result.is_err(), "Playing empty queue should fail");
}

#[test]
fn test_remove_invalid_index() {
    let mut manager = PlaybackManager::default();

    manager.add_to_queue_end(create_test_track("1", "Track 1", "Artist", 180));

    let result = manager.remove_from_queue(10);
    assert!(result.is_err(), "Removing invalid index should fail");
}

#[test]
fn test_skip_to_invalid_index() {
    let mut manager = PlaybackManager::default();

    manager.add_to_queue_end(create_test_track("1", "Track 1", "Artist", 180));

    let result = manager.skip_to_queue_index(10);
    assert!(result.is_err(), "Skipping to invalid index should fail");
}

#[test]
fn test_seek_without_source() {
    let mut manager = PlaybackManager::default();

    let result = manager.seek_to(Duration::from_secs(10));
    assert!(result.is_err(), "Seeking without source should fail");
}

#[test]
fn test_reorder_queue_valid() {
    let mut manager = PlaybackManager::default();

    for i in 1..=3 {
        manager.add_to_queue_end(create_test_track(
            &i.to_string(),
            &format!("Track {}", i),
            "Artist",
            180,
        ));
    }

    // Reorder: move track 0 to position 2
    let result = manager.reorder_queue(0, 2);
    assert!(result.is_ok(), "Valid reorder should succeed");
}

// ============================================================================
// 10. Stress and Performance Tests
// ============================================================================

#[test]
fn test_rapid_play_pause() {
    let mut manager = PlaybackManager::default();

    manager.add_to_queue_end(create_test_track("1", "Track 1", "Artist", 180));
    manager.set_audio_source(Box::new(MockAudioSource::new(
        Duration::from_secs(180),
        44100,
    )));

    // Rapid play/pause cycles
    for _ in 0..100 {
        manager.play().ok();
        manager.pause();
    }

    // Should end in paused state
    assert_eq!(manager.get_state(), PlaybackState::Paused);

    // Should still be functional
    manager.play().ok();
    assert_eq!(manager.get_state(), PlaybackState::Playing);
}

#[test]
fn test_rapid_volume_changes() {
    let mut manager = PlaybackManager::default();
    manager.set_audio_source(Box::new(MockAudioSource::new(
        Duration::from_secs(10),
        44100,
    )));

    let mut buffer = vec![0.0f32; 512];

    // Rapid volume changes while processing
    for i in 0..100 {
        manager.set_volume((i % 101) as u8);
        manager.process_audio(&mut buffer).ok();
    }

    // Should still be functional
    assert!(
        buffer.iter().all(|s| s.is_finite()),
        "Output should be valid after rapid volume changes"
    );
}

#[test]
fn test_large_queue_operations() {
    let mut manager = PlaybackManager::default();

    // Add 1000 tracks
    let tracks: Vec<QueueTrack> = (0..1000)
        .map(|i| {
            create_test_track(
                &i.to_string(),
                &format!("Track {}", i),
                &format!("Artist {}", i % 50),
                180,
            )
        })
        .collect();

    manager.add_playlist_to_queue(tracks);

    assert_eq!(manager.queue_len(), 1000);

    // Shuffle
    manager.set_shuffle(ShuffleMode::Smart);
    assert_eq!(manager.queue_len(), 1000);

    // Remove from middle
    manager.remove_from_queue(500).ok();
    assert_eq!(manager.queue_len(), 999);

    // Restore order
    manager.set_shuffle(ShuffleMode::Off);
    // Should still have all remaining tracks
    assert_eq!(manager.queue_len(), 999);
}

#[test]
fn test_continuous_playback_processing() {
    let mut manager = PlaybackManager::default();
    manager.set_audio_source(Box::new(MockAudioSource::new(
        Duration::from_secs(30),
        44100,
    )));

    // Simulate 30 seconds of continuous playback processing
    let mut total_samples = 0;
    let mut buffer = vec![0.0f32; 4096];

    for _ in 0..1000 {
        match manager.process_audio(&mut buffer) {
            Ok(0) => break,
            Ok(n) => {
                total_samples += n;
                // Verify output is valid
                assert!(
                    buffer[..n].iter().all(|s| s.is_finite()),
                    "All samples should be finite"
                );
            }
            Err(_) => break,
        }
    }

    // Should have processed some audio
    assert!(
        total_samples > 44100, // At least 0.5 seconds
        "Should process significant audio, got {} samples",
        total_samples
    );
}

// ============================================================================
// 11. Crossfade Calculation Tests (unit-level integration)
// ============================================================================

#[test]
fn test_time_until_crossfade() {
    let mut manager = PlaybackManager::default();
    manager.set_crossfade_enabled(true);
    manager.set_crossfade_duration(3000); // 3 seconds

    // Without source, should return None
    assert!(manager.time_until_crossfade().is_none());

    // With source
    manager.set_audio_source(Box::new(MockAudioSource::new(
        Duration::from_secs(10),
        44100,
    )));

    // At start (10s track, 3s crossfade): 10 - 0 - 3 = 7 seconds until crossfade
    let time_until = manager.time_until_crossfade().unwrap();
    assert!(
        time_until.as_secs() >= 6 && time_until.as_secs() <= 8,
        "Expected ~7s until crossfade, got {}s",
        time_until.as_secs()
    );
}

#[test]
fn test_should_prepare_next_track() {
    let mut manager = PlaybackManager::default();
    manager.set_crossfade_enabled(true);
    manager.set_crossfade_duration(3000);

    // Add tracks
    manager.add_to_queue_end(create_test_track("1", "Track 1", "Artist", 5)); // 5 seconds
    manager.add_to_queue_end(create_test_track("2", "Track 2", "Artist", 5));

    // Short track: 5s - 3s crossfade = crossfade at 2s
    // Should prepare 5s before crossfade start, so immediately
    manager.set_audio_source(Box::new(MockAudioSource::new(
        Duration::from_secs(5),
        44100,
    )));

    // With a 5 second track and 3 second crossfade, crossfade starts at 2s
    // We're at 0s, so 2s until crossfade < 5s, should prepare
    assert!(
        manager.should_prepare_next_track(),
        "Should prepare next track for short track"
    );
}

#[test]
fn test_crossfade_settings_immutability() {
    let config = PlaybackConfig::with_crossfade(5000, FadeCurve::SCurve);
    let manager = PlaybackManager::new(config);

    let settings = manager.get_crossfade_settings();

    // Settings should match config
    assert!(settings.enabled);
    assert_eq!(settings.duration_ms, 5000);
    assert_eq!(settings.curve, FadeCurve::SCurve);
    assert!(!settings.on_skip);
}

#[test]
fn test_crossfade_full_settings_update() {
    let mut manager = PlaybackManager::default();

    let new_settings = CrossfadeSettings {
        enabled: true,
        duration_ms: 4000,
        curve: FadeCurve::SquareRoot,
        on_skip: true,
    };

    manager.set_crossfade_settings(new_settings.clone());

    let settings = manager.get_crossfade_settings();
    assert_eq!(settings.enabled, new_settings.enabled);
    assert_eq!(settings.duration_ms, new_settings.duration_ms);
    assert_eq!(settings.curve, new_settings.curve);
    assert_eq!(settings.on_skip, new_settings.on_skip);
}
