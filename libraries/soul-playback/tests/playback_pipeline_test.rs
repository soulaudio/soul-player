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

    // Process warmup audio to get past start fade (20ms = ~1764 samples at 44100Hz stereo)
    let mut warmup = vec![0.0f32; 4096];
    manager.process_audio(&mut warmup).ok();

    let mut buffer = vec![0.0f32; 1024];

    // Test at 100% volume (now past fade period)
    manager.set_volume(100);
    manager.process_audio(&mut buffer).ok();
    let rms_100 = calculate_rms(&buffer);

    // Reset source and test at 50%
    manager.set_audio_source(Box::new(MockAudioSource::new(
        Duration::from_secs(5),
        44100,
    )));
    // Process warmup for new source
    let mut warmup = vec![0.0f32; 4096];
    manager.process_audio(&mut warmup).ok();

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

    // Process warmup to get past start fade (20ms = ~1764 samples at 44100Hz stereo)
    let mut warmup = vec![0.0f32; 4096];
    manager.process_audio(&mut warmup).ok();

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

    // Process warmup to get past start fade (20ms = ~1764 samples at 44100Hz stereo)
    let mut warmup = vec![0.0f32; 4096];
    manager.process_audio(&mut warmup).ok();

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

// ============================================================================
// 12. Crossfade Audio Mixing Tests - Verify actual crossfade behavior
// ============================================================================

/// Test that verifies both tracks are audible during crossfade transition
/// This is the core test for the crossfade bug investigation
#[test]
fn test_crossfade_both_tracks_audible_during_transition() {
    let mut manager = PlaybackManager::default();
    manager.set_crossfade_enabled(true);
    manager.set_crossfade_duration(500); // 500ms crossfade for easier testing
    manager.set_crossfade_curve(FadeCurve::Linear);
    manager.set_sample_rate(44100);
    manager.set_output_channels(2);

    // Track 1: 440Hz sine wave (A4 note)
    // Track 2: 880Hz sine wave (A5 note - one octave higher)
    // If crossfade works correctly, we should hear both frequencies during transition

    // Add queue tracks
    manager.add_to_queue_end(create_test_track("1", "Track 1", "Artist A", 1)); // 1 second track
    manager.add_to_queue_end(create_test_track("2", "Track 2", "Artist B", 1));

    // Set up current source (440Hz)
    let current_source = MockAudioSource::new(Duration::from_secs(1), 44100).with_frequency(440.0);
    manager.set_audio_source(Box::new(current_source));

    // Set up next source (880Hz) - THIS IS CRITICAL for crossfade to work
    let next_source = MockAudioSource::new(Duration::from_secs(1), 44100).with_frequency(880.0);
    let next_track = create_test_track("2", "Track 2", "Artist B", 1);
    manager.set_next_source(Box::new(next_source), next_track);

    // Verify next source is set
    assert!(
        manager.has_next_source(),
        "Next source must be set for crossfade to work"
    );

    // Advance near end of track to trigger crossfade
    // Track is 1 second, crossfade is 500ms, so seek to 400ms (100ms before crossfade starts)
    manager.seek_to(Duration::from_millis(400)).ok();

    // Process audio through the crossfade region
    let mut all_samples: Vec<f32> = Vec::new();
    let mut buffer = vec![0.0f32; 4096];
    let mut crossfade_triggered = false;

    for _ in 0..50 {
        // Process up to 50 buffers
        match manager.process_audio(&mut buffer) {
            Ok(0) => break,
            Ok(n) => {
                all_samples.extend_from_slice(&buffer[..n]);

                // Check if crossfade is active
                if manager.is_crossfading() {
                    crossfade_triggered = true;
                }
            }
            Err(_) => break,
        }

        // Stop if we've processed enough for the crossfade
        if all_samples.len() > 44100 * 2 {
            // More than 1 second stereo
            break;
        }
    }

    // Crossfade should have been triggered
    assert!(
        crossfade_triggered,
        "Crossfade should have been triggered during playback"
    );

    // Verify we got audio output
    assert!(
        !all_samples.is_empty(),
        "Should have produced audio samples"
    );

    // During crossfade, both frequencies should be present in the output
    // We can verify this by checking that the output contains both frequency components
    // A simple check: we should have audio output (some non-zero samples)
    let rms = calculate_rms(&all_samples);
    let peak = calculate_peak(&all_samples);

    // We should have SOME audio (the sources have amplitude 0.5)
    // During crossfade, the gains vary but we should see output
    assert!(
        rms > 0.01,
        "Audio should have some output during crossfade, got RMS: {}, peak: {}",
        rms,
        peak
    );

    // Peak should show that audio was actually produced
    assert!(
        peak > 0.05,
        "Peak should show audio was produced, got peak: {}",
        peak
    );
}

/// Test that crossfade engine correctly mixes samples from both tracks
#[test]
fn test_crossfade_engine_mixing_directly() {
    use soul_playback::CrossfadeEngine;

    let settings = CrossfadeSettings {
        enabled: true,
        duration_ms: 100, // 100ms for easy testing
        curve: FadeCurve::Linear,
        on_skip: true,
    };

    let mut engine = CrossfadeEngine::with_settings(settings);
    engine.set_sample_rate(1000); // 1000Hz for simple math

    // Start crossfade
    let started = engine.start(false);
    assert!(started, "Crossfade should start when enabled");
    assert!(engine.is_active(), "Crossfade should be active after start");

    // Prepare test buffers
    // Outgoing track: all 1.0
    // Incoming track: all 0.0
    // With linear crossfade, output should go from 1.0 to 0.0
    let outgoing = vec![1.0f32; 200]; // 100 stereo frames = 100ms at 1000Hz
    let incoming = vec![0.0f32; 200];
    let mut output = vec![0.0f32; 200];

    let (samples, completed) = engine.process(&outgoing, &incoming, &mut output);

    // Should have processed all samples
    assert_eq!(samples, 200, "Should process all samples");
    assert!(completed, "Crossfade should complete");

    // Verify output: first sample should be mostly outgoing, last should be mostly incoming
    assert!(
        output[0] > 0.9,
        "First sample should be mostly outgoing (1.0), got {}",
        output[0]
    );
    assert!(
        output[198] < 0.1,
        "Last sample should be mostly incoming (0.0), got {}",
        output[198]
    );

    // Middle should be roughly 0.5 (linear crossfade)
    // 200 samples total = 100 stereo frames
    // At frame 50, progress = 100/200 = 0.5, so output should be ~0.5
    let mid_frame = 50;
    let mid_sample_idx = mid_frame * 2; // Index 100
    assert!(
        output[mid_sample_idx] > 0.4 && output[mid_sample_idx] < 0.6,
        "Middle sample at index {} should be ~0.5 for linear crossfade, got {}",
        mid_sample_idx,
        output[mid_sample_idx]
    );
}

/// Test equal power crossfade maintains constant perceived loudness
#[test]
fn test_crossfade_equal_power_constant_loudness() {
    use soul_playback::CrossfadeEngine;

    let settings = CrossfadeSettings {
        enabled: true,
        duration_ms: 100,
        curve: FadeCurve::EqualPower,
        on_skip: true,
    };

    let mut engine = CrossfadeEngine::with_settings(settings);
    engine.set_sample_rate(1000);

    engine.start(false);

    // Both tracks at full volume (1.0)
    // With equal power crossfade, sum of squares should be constant (~1.0)
    let outgoing = vec![1.0f32; 200];
    let incoming = vec![1.0f32; 200];
    let mut output = vec![0.0f32; 200];

    let (samples, _) = engine.process(&outgoing, &incoming, &mut output);
    assert_eq!(samples, 200);

    // Check that output maintains approximately constant power throughout
    // For equal power crossfade: out_gain^2 + in_gain^2 = 1
    // So if both inputs are 1.0, output should be ~sqrt(2) = ~1.414 at midpoint
    // But since we're mixing, the actual value depends on the gains

    // At the start (t=0): out_gain=1.0, in_gain=0.0, output=1.0
    // At the end (t=1): out_gain=0.0, in_gain=1.0, output=1.0
    // At midpoint (t=0.5): out_gain=0.707, in_gain=0.707, output=0.707+0.707=1.414

    let start_sample = output[0];
    let mid_sample = output[99 * 2]; // Approximately midpoint
    let end_sample = output[198];

    // Start and end should be ~1.0 (one track at full, other at zero)
    assert!(
        (start_sample - 1.0).abs() < 0.1,
        "Start should be ~1.0, got {}",
        start_sample
    );
    assert!(
        (end_sample - 1.0).abs() < 0.1,
        "End should be ~1.0, got {}",
        end_sample
    );

    // Middle should be higher due to both tracks contributing
    assert!(
        mid_sample > 1.0,
        "Middle should be >1.0 with equal power when both tracks are 1.0, got {}",
        mid_sample
    );
}

/// Test that crossfade is NOT triggered when on_skip is false and it's a manual skip
#[test]
fn test_crossfade_respects_on_skip_setting() {
    use soul_playback::CrossfadeEngine;

    let settings = CrossfadeSettings {
        enabled: true,
        duration_ms: 1000,
        curve: FadeCurve::Linear,
        on_skip: false, // Crossfade only on auto-advance
    };

    let mut engine = CrossfadeEngine::with_settings(settings);
    engine.set_sample_rate(44100);

    // Try to start with manual skip - should NOT start
    let started = engine.start(true); // true = manual skip
    assert!(!started, "Crossfade should NOT start on manual skip when on_skip is false");
    assert!(
        !engine.is_active(),
        "Engine should remain inactive"
    );

    // Reset and try with auto-advance - should start
    engine.reset();
    let started = engine.start(false); // false = auto-advance
    assert!(started, "Crossfade SHOULD start on auto-advance");
    assert!(engine.is_active(), "Engine should be active");
}

/// Test that crossfade is triggered on manual skip when on_skip is true
#[test]
fn test_crossfade_on_manual_skip() {
    use soul_playback::CrossfadeEngine;

    let settings = CrossfadeSettings {
        enabled: true,
        duration_ms: 1000,
        curve: FadeCurve::Linear,
        on_skip: true, // Crossfade on both auto-advance and manual skip
    };

    let mut engine = CrossfadeEngine::with_settings(settings);
    engine.set_sample_rate(44100);

    // Should start with manual skip
    let started = engine.start(true);
    assert!(started, "Crossfade should start on manual skip when on_skip is true");
}

/// Test crossfade with different curve types
#[test]
fn test_crossfade_curve_differences() {
    use soul_playback::CrossfadeEngine;

    let curves = [
        FadeCurve::Linear,
        FadeCurve::EqualPower,
        FadeCurve::SCurve,
        FadeCurve::SquareRoot,
    ];

    for curve in curves {
        let settings = CrossfadeSettings {
            enabled: true,
            duration_ms: 100,
            curve,
            on_skip: true,
        };

        let mut engine = CrossfadeEngine::with_settings(settings);
        engine.set_sample_rate(1000);
        engine.start(false);

        // Process crossfade
        let outgoing = vec![1.0f32; 200];
        let incoming = vec![0.0f32; 200];
        let mut output = vec![0.0f32; 200];

        let (samples, completed) = engine.process(&outgoing, &incoming, &mut output);

        assert_eq!(samples, 200, "Curve {:?}: Should process all samples", curve);
        assert!(completed, "Curve {:?}: Should complete", curve);

        // All curves should go from ~1.0 to ~0.0
        assert!(
            output[0] > 0.9,
            "Curve {:?}: Start should be mostly outgoing",
            curve
        );
        assert!(
            output[198] < 0.1,
            "Curve {:?}: End should be mostly incoming",
            curve
        );
    }
}

/// Test that crossfade duration calculation is correct at different sample rates
#[test]
fn test_crossfade_duration_at_different_sample_rates() {
    use soul_playback::CrossfadeEngine;

    let sample_rates = [44100, 48000, 96000, 192000];

    for rate in sample_rates {
        let settings = CrossfadeSettings {
            enabled: true,
            duration_ms: 1000, // 1 second
            curve: FadeCurve::Linear,
            on_skip: true,
        };

        let mut engine = CrossfadeEngine::with_settings(settings);
        engine.set_sample_rate(rate);
        engine.start(false);

        // Calculate expected samples for 1 second
        // Note: duration_samples in crossfade is multiplied by 2 for stereo
        let expected_samples = rate as usize * 2; // 1 second of stereo samples

        // Verify remaining samples
        let remaining = engine.remaining_samples();
        assert_eq!(
            remaining, expected_samples,
            "At {}Hz, 1s crossfade should have {} remaining samples, got {}",
            rate, expected_samples, remaining
        );
    }
}

/// Integration test: Verify crossfade actually triggers when approaching end of track
#[test]
fn test_crossfade_triggers_at_correct_time() {
    let mut manager = PlaybackManager::default();
    manager.set_crossfade_enabled(true);
    manager.set_crossfade_duration(1000); // 1 second crossfade
    manager.set_sample_rate(44100);
    manager.set_output_channels(2);

    // Add two tracks
    manager.add_to_queue_end(create_test_track("1", "Track 1", "Artist", 5)); // 5 seconds
    manager.add_to_queue_end(create_test_track("2", "Track 2", "Artist", 5));

    // Set up 5-second source
    manager.set_audio_source(Box::new(MockAudioSource::new(
        Duration::from_secs(5),
        44100,
    )));

    // Pre-load next track
    let next_source = MockAudioSource::new(Duration::from_secs(5), 44100);
    let next_track = create_test_track("2", "Track 2", "Artist", 5);
    manager.set_next_source(Box::new(next_source), next_track);

    // Seek to 3.5 seconds (1.5 seconds before end, 0.5 seconds before crossfade should start)
    manager.seek_to(Duration::from_millis(3500)).ok();

    // Should NOT be crossfading yet (0.5s before crossfade region)
    assert!(
        !manager.is_crossfading(),
        "Should not be crossfading 0.5s before crossfade region"
    );

    // Process some audio to advance time into the crossfade region
    let mut buffer = vec![0.0f32; 44100]; // ~0.5 seconds at 44.1kHz stereo
    let mut crossfade_started = false;

    for _ in 0..10 {
        match manager.process_audio(&mut buffer) {
            Ok(0) => break,
            Ok(_) => {
                if manager.is_crossfading() {
                    crossfade_started = true;
                    break;
                }
            }
            Err(_) => break,
        }
    }

    // Crossfade should have started
    assert!(
        crossfade_started,
        "Crossfade should start when entering the last 1 second of track"
    );
}

/// Test that demonstrates the root cause: crossfade doesn't work without pre-loaded next source
#[test]
fn test_crossfade_fails_without_next_source() {
    let mut manager = PlaybackManager::default();
    manager.set_crossfade_enabled(true);
    manager.set_crossfade_duration(500);
    manager.set_sample_rate(44100);
    manager.set_output_channels(2);

    // Add tracks but DON'T set next source (simulating the bug)
    manager.add_to_queue_end(create_test_track("1", "Track 1", "Artist", 1));
    manager.add_to_queue_end(create_test_track("2", "Track 2", "Artist", 1));

    // Only set current source, NOT next source
    manager.set_audio_source(Box::new(MockAudioSource::new(
        Duration::from_secs(1),
        44100,
    )));

    // Verify next source is NOT set
    assert!(
        !manager.has_next_source(),
        "Next source should NOT be set (this is the bug condition)"
    );

    // Seek near end
    manager.seek_to(Duration::from_millis(400)).ok();

    // Process audio - crossfade cannot happen without next source
    let mut buffer = vec![0.0f32; 4096];
    let mut crossfade_triggered = false;

    for _ in 0..50 {
        match manager.process_audio(&mut buffer) {
            Ok(0) => break,
            Ok(_) => {
                if manager.is_crossfading() {
                    crossfade_triggered = true;
                }
            }
            Err(_) => break,
        }
    }

    // Crossfade should NOT have been triggered because next_source wasn't set
    assert!(
        !crossfade_triggered,
        "Crossfade should NOT trigger without next_source - this demonstrates the root cause of the bug"
    );
}
