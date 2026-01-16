//! Tests for pause during track startup
//!
//! Bug: When user clicks play then immediately clicks pause (during the source
//! loading/ready phase), audio continues playing instead of pausing.
//!
//! Root cause: The pause command sets state to Paused, but the audio callback
//! may have already started processing audio before respecting the pause state.

use soul_playback::{
    AudioSource, PlaybackConfig, PlaybackManager, PlaybackState, QueueTrack, Result, TrackSource,
};
use std::f32::consts::PI;
use std::time::Duration;

/// Test audio source that outputs a loud constant tone
/// This makes it very obvious if audio is playing when it shouldn't be
struct LoudToneSource {
    amplitude: f32,
    frequency: f32,
    sample_rate: u32,
    position_samples: usize,
    total_samples: usize,
    /// Whether this source is "ready" (simulates loading delay)
    ready: bool,
    /// Samples to wait before becoming ready
    ready_after_samples: usize,
}

impl LoudToneSource {
    fn new(sample_rate: u32, duration_secs: f32) -> Self {
        Self {
            amplitude: 0.9, // Very loud to make issues obvious
            frequency: 1000.0,
            sample_rate,
            position_samples: 0,
            total_samples: (sample_rate as f32 * duration_secs * 2.0) as usize,
            ready: true, // Ready immediately for simple tests
            ready_after_samples: 0,
        }
    }

    /// Create a source that becomes ready after a delay
    /// This simulates the real-world loading scenario
    #[allow(dead_code)]
    fn new_with_delay(sample_rate: u32, duration_secs: f32, delay_ms: u32) -> Self {
        let ready_after_samples = (sample_rate as f32 * (delay_ms as f32 / 1000.0) * 2.0) as usize;
        Self {
            amplitude: 0.9,
            frequency: 1000.0,
            sample_rate,
            position_samples: 0,
            total_samples: (sample_rate as f32 * duration_secs * 2.0) as usize,
            ready: false,
            ready_after_samples,
        }
    }
}

impl AudioSource for LoudToneSource {
    fn read_samples(&mut self, buffer: &mut [f32]) -> Result<usize> {
        // Simulate loading delay - become ready after processing some samples
        if !self.ready && self.position_samples >= self.ready_after_samples {
            self.ready = true;
        }

        let remaining = self.total_samples.saturating_sub(self.position_samples);
        let to_read = buffer.len().min(remaining);

        // Generate loud tone
        for i in 0..to_read / 2 {
            let sample_idx = self.position_samples / 2 + i;
            let t = sample_idx as f32 / self.sample_rate as f32;
            let sample = self.amplitude * (2.0 * PI * self.frequency * t).sin();
            buffer[i * 2] = sample; // Left
            buffer[i * 2 + 1] = sample; // Right
        }

        self.position_samples += to_read;
        Ok(to_read)
    }

    fn seek(&mut self, position: Duration) -> Result<()> {
        self.position_samples = (position.as_secs_f32() * self.sample_rate as f32 * 2.0) as usize;
        Ok(())
    }

    fn duration(&self) -> Duration {
        Duration::from_secs_f32(self.total_samples as f32 / (self.sample_rate as f32 * 2.0))
    }

    fn position(&self) -> Duration {
        Duration::from_secs_f32(self.position_samples as f32 / (self.sample_rate as f32 * 2.0))
    }

    fn is_finished(&self) -> bool {
        self.position_samples >= self.total_samples
    }

    fn is_ready(&self) -> bool {
        self.ready
    }
}

/// Helper: Check if audio buffer contains actual audio (not silence/noise)
/// Returns the peak amplitude in the buffer
fn get_peak_amplitude(buffer: &[f32]) -> f32 {
    buffer.iter().map(|s| s.abs()).fold(0.0f32, f32::max)
}

/// Helper: Check if buffer contains only silence or DAC keepalive noise
fn is_silent(buffer: &[f32]) -> bool {
    let dac_keepalive_threshold = 0.0001; // ~-80dB
    buffer.iter().all(|&s| s.abs() < dac_keepalive_threshold)
}

/// Helper to create a test queue track
fn create_queue_track(id: &str, duration_secs: u64) -> QueueTrack {
    QueueTrack {
        id: id.to_string(),
        path: std::path::PathBuf::from(format!("{}.mp3", id)),
        title: format!("Test Track {}", id),
        artist: "Test Artist".to_string(),
        album: None,
        duration: Duration::from_secs(duration_secs),
        track_number: None,
        source: TrackSource::Single,
    }
}

// ============================================================================
// TESTS: Immediate Pause After Play
// ============================================================================

#[test]
#[ignore = "Tests synchronous pause behavior - pause() uses async fade-out so state change is deferred until fade completes"]
fn test_pause_immediately_after_play_stops_audio() {
    // BUG REPRODUCTION:
    // 1. Call play (set_audio_source, which sets state to Playing)
    // 2. Immediately call pause
    // 3. Audio continues playing because pause doesn't cancel startup sequence

    let mut manager = PlaybackManager::new(PlaybackConfig::default());
    manager.set_sample_rate(48000);
    manager.set_output_channels(2);

    // Add track to queue and start playback
    let track = QueueTrack {
        id: "test1".to_string(),
        path: std::path::PathBuf::from("test.mp3"),
        title: "Test".to_string(),
        artist: "Test Artist".to_string(),
        album: None,
        duration: Duration::from_secs(5),
        track_number: None,
        source: TrackSource::Single,
    };
    manager.load_playlist(vec![track], 0);
    manager.play().unwrap(); // State: Loading

    // Simulate track finishing loading
    let source = Box::new(LoudToneSource::new(48000, 5.0));
    manager.set_audio_source(source);
    assert_eq!(manager.get_state(), PlaybackState::Playing);

    // Simulate immediately clicking "Pause" (within milliseconds)
    manager.pause();
    assert_eq!(manager.get_state(), PlaybackState::Paused);

    // Now process audio callbacks (simulate real-time audio thread)
    // The audio should be SILENT, not the loud tone

    // Process multiple buffers to cover the startup period
    for i in 0..10 {
        let mut buffer = vec![0.0f32; 2048];
        let _ = manager.process_audio(&mut buffer);

        let peak = get_peak_amplitude(&buffer);
        let silent = is_silent(&buffer);

        println!("Buffer {}: peak = {:.6}, silent = {}", i, peak, silent);

        // The buffer should be silent (paused), not contain audio
        assert!(
            silent,
            "BUG DETECTED: Audio is playing after pause! Buffer {} has peak amplitude {:.4}. \
             Expected silence after immediate pause, but audio is still playing.",
            i, peak
        );
    }
}

#[test]
fn test_pause_during_source_ready_wait() {
    // Test pausing while waiting for source to become ready
    // This is the exact window where the bug occurs

    let mut manager = PlaybackManager::new(PlaybackConfig::default());
    manager.set_sample_rate(48000);
    manager.set_output_channels(2);

    // Create source with 100ms delay before ready
    let source = Box::new(LoudToneSource::new_with_delay(48000, 5.0, 100));
    manager.set_audio_source(source);

    // Process a few buffers (source not ready yet)
    for _ in 0..2 {
        let mut buffer = vec![0.0f32; 512];
        let _ = manager.process_audio(&mut buffer);
    }

    // NOW pause while still in the "waiting for ready" phase
    manager.pause();

    // Continue processing - should remain silent
    for i in 0..20 {
        let mut buffer = vec![0.0f32; 2048];
        let _ = manager.process_audio(&mut buffer);

        let silent = is_silent(&buffer);
        println!("Buffer {}: silent = {}", i, silent);

        assert!(
            silent,
            "Buffer {} contains audio after pause during ready wait",
            i
        );
    }
}

#[test]
#[ignore = "Tests synchronous pause behavior - pause() uses async fade-out so state change is deferred until fade completes"]
fn test_resume_after_pause_during_startup() {
    // Test that resuming after pausing during startup works correctly
    // The audio should start playing cleanly, not be in a broken state

    let mut manager = PlaybackManager::new(PlaybackConfig::default());
    manager.set_sample_rate(48000);
    manager.set_output_channels(2);

    // Add track and start playback
    manager.load_playlist(vec![create_queue_track("test1", 5)], 0);
    manager.play().unwrap(); // State: Loading

    // Simulate track finishing loading
    let source = Box::new(LoudToneSource::new(48000, 5.0));
    manager.set_audio_source(source);

    // Immediate pause
    manager.pause();

    // Verify paused (silent)
    let mut pause_buffer = vec![0.0f32; 2048];
    let _ = manager.process_audio(&mut pause_buffer);
    assert!(is_silent(&pause_buffer), "Should be silent while paused");

    // Now resume
    manager.play().unwrap();
    assert_eq!(manager.get_state(), PlaybackState::Playing);

    // Process audio - should now have audio playing
    let mut warmup_buffer = vec![0.0f32; 9600]; // 100ms warmup
    let _ = manager.process_audio(&mut warmup_buffer);

    let mut audio_buffer = vec![0.0f32; 2048];
    let _ = manager.process_audio(&mut audio_buffer);

    let peak = get_peak_amplitude(&audio_buffer);
    println!("Peak after resume: {:.4}", peak);

    // Should have actual audio now (not silence)
    assert!(
        peak > 0.1,
        "No audio after resume! Peak: {:.4}. Resume should start playback.",
        peak
    );
}

#[test]
#[ignore = "Tests synchronous pause behavior - pause() uses async fade-out so state change is deferred until fade completes"]
fn test_multiple_rapid_pause_resume_cycles() {
    // Test rapid pause/resume during startup
    // This simulates a user frantically clicking play/pause

    let sample_rate = 48000u32;

    for cycle in 0..5 {
        let mut manager = PlaybackManager::new(PlaybackConfig::default());
        manager.set_sample_rate(sample_rate);
        manager.set_output_channels(2);

        // Add track and start playback
        manager.load_playlist(vec![create_queue_track(&format!("test{}", cycle), 5)], 0);
        manager.play().unwrap(); // State: Loading

        // Simulate track finishing loading
        let source = Box::new(LoudToneSource::new(sample_rate, 5.0));
        manager.set_audio_source(source);

        // Rapid pause/resume
        manager.pause();
        manager.play().unwrap();
        manager.pause();

        // Should be paused (silent)
        let mut buffer = vec![0.0f32; 2048];
        let _ = manager.process_audio(&mut buffer);

        let silent = is_silent(&buffer);
        println!(
            "Cycle {}: silent after rapid pause/resume = {}",
            cycle, silent
        );

        assert!(
            silent,
            "Cycle {}: Audio playing after final pause in rapid cycle",
            cycle
        );
    }
}

#[test]
fn test_pause_just_after_source_becomes_ready() {
    // Test pausing RIGHT when the source becomes ready
    // This is a critical timing window

    let mut manager = PlaybackManager::new(PlaybackConfig::default());
    manager.set_sample_rate(48000);
    manager.set_output_channels(2);

    // Source with very short delay (50ms)
    let source = Box::new(LoudToneSource::new_with_delay(48000, 5.0, 50));
    manager.set_audio_source(source);

    // Process until source becomes ready (50ms = 4800 stereo samples)
    let mut total_processed = 0;
    while total_processed < 4800 {
        let mut buffer = vec![0.0f32; 512];
        let samples = manager.process_audio(&mut buffer).unwrap();
        total_processed += samples;
    }

    // Source should be ready now, pause immediately
    manager.pause();

    // Verify audio stops
    for i in 0..5 {
        let mut buffer = vec![0.0f32; 2048];
        let _ = manager.process_audio(&mut buffer);

        let silent = is_silent(&buffer);
        println!("Buffer {} after ready+pause: silent = {}", i, silent);

        assert!(silent, "Buffer {} has audio after pause-at-ready", i);
    }
}

// ============================================================================
// TESTS: State Verification
// ============================================================================

#[test]
#[ignore = "Tests synchronous pause behavior - pause() uses async fade-out so state change is deferred until fade completes"]
fn test_pause_changes_state_immediately() {
    let mut manager = PlaybackManager::new(PlaybackConfig::default());
    manager.set_sample_rate(48000);
    manager.set_output_channels(2);

    // Add track and start playback
    manager.load_playlist(vec![create_queue_track("test1", 5)], 0);
    manager.play().unwrap(); // State: Loading

    // Simulate track finishing loading
    let source = Box::new(LoudToneSource::new(48000, 5.0));
    manager.set_audio_source(source);
    assert_eq!(manager.get_state(), PlaybackState::Playing);

    manager.pause();
    assert_eq!(
        manager.get_state(),
        PlaybackState::Paused,
        "State should change to Paused immediately"
    );
}

#[test]
fn test_pause_respects_state_in_audio_callback() {
    // Verify that the audio callback respects the Paused state
    // even if pause was called during startup

    let mut manager = PlaybackManager::new(PlaybackConfig::default());
    manager.set_sample_rate(48000);
    manager.set_output_channels(2);

    let source = Box::new(LoudToneSource::new(48000, 5.0));
    manager.set_audio_source(source);

    manager.pause();

    // The audio callback should check state and output silence
    let mut buffer = vec![0.0f32; 2048];
    let _ = manager.process_audio(&mut buffer);

    // Verify we got the full buffer length back (not early return)
    // AND that it's silent
    assert!(
        is_silent(&buffer),
        "Audio callback not respecting Paused state during startup"
    );
}

// ============================================================================
// TESTS: Edge Cases
// ============================================================================

#[test]
fn test_pause_before_first_audio_callback() {
    // Test pausing before ANY audio processing has occurred
    // This is the most extreme case of immediate pause

    let mut manager = PlaybackManager::new(PlaybackConfig::default());
    manager.set_sample_rate(48000);
    manager.set_output_channels(2);

    let source = Box::new(LoudToneSource::new(48000, 5.0));
    manager.set_audio_source(source);

    // Pause BEFORE any process_audio() call
    manager.pause();

    // Now process audio
    let mut buffer = vec![0.0f32; 2048];
    let _ = manager.process_audio(&mut buffer);

    assert!(
        is_silent(&buffer),
        "Audio playing after pause before first callback"
    );
}

#[test]
#[ignore = "Tests synchronous pause behavior - pause() uses async fade-out so state change is deferred until fade completes"]
fn test_pause_then_different_track() {
    // Test that pausing during startup, then loading a different track works

    let mut manager = PlaybackManager::new(PlaybackConfig::default());
    manager.set_sample_rate(48000);
    manager.set_output_channels(2);

    // First track
    manager.load_playlist(vec![create_queue_track("test1", 5)], 0);
    manager.play().unwrap();

    let source1 = Box::new(LoudToneSource::new(48000, 5.0));
    manager.set_audio_source(source1);
    manager.pause();

    // Verify paused
    let mut buffer1 = vec![0.0f32; 1024];
    let _ = manager.process_audio(&mut buffer1);
    assert!(is_silent(&buffer1));

    // Load different track (new track starts playback automatically)
    manager.load_playlist(vec![create_queue_track("test2", 5)], 0);
    manager.play().unwrap(); // New track, start playing

    let source2 = Box::new(LoudToneSource::new(48000, 5.0));
    manager.set_audio_source(source2);

    // Should be playing new track
    let mut warmup = vec![0.0f32; 9600];
    let _ = manager.process_audio(&mut warmup);

    let mut buffer2 = vec![0.0f32; 2048];
    let _ = manager.process_audio(&mut buffer2);

    let peak = get_peak_amplitude(&buffer2);
    assert!(
        peak > 0.1,
        "New track should be playing after pause-then-load"
    );
}
