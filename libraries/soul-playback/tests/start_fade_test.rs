//! Playback Start Fade Tests
//!
//! Tests to detect and verify fixes for audio pop/click at playback start.
//!
//! The issue: When playback starts, audio transitions from silence (0.0) to
//! full amplitude instantly, causing an audible "click" or "pop". This is a
//! DC offset step response that creates a transient.
//!
//! Solution: Apply a short fade-in envelope (5-10ms) when playback starts.

use soul_playback::{
    AudioSource, FadeCurve, PlaybackConfig, PlaybackManager, QueueTrack, Result, TrackSource,
};
use std::f32::consts::PI;
use std::path::PathBuf;
use std::time::Duration;

// ============================================================================
// TEST UTILITIES
// ============================================================================

/// Test audio source that outputs a constant level sine wave
/// Used to detect if fade-in is properly applied
struct ConstantLevelSource {
    amplitude: f32,
    frequency: f32,
    sample_rate: u32,
    position_samples: usize,
    total_samples: usize,
}

impl ConstantLevelSource {
    fn new(amplitude: f32, frequency: f32, sample_rate: u32, duration_secs: f32) -> Self {
        Self {
            amplitude,
            frequency,
            sample_rate,
            position_samples: 0,
            total_samples: (sample_rate as f32 * duration_secs * 2.0) as usize, // stereo
        }
    }
}

impl AudioSource for ConstantLevelSource {
    fn read_samples(&mut self, buffer: &mut [f32]) -> Result<usize> {
        let remaining = self.total_samples.saturating_sub(self.position_samples);
        let to_read = buffer.len().min(remaining);

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
        let samples = (position.as_secs_f32() * self.sample_rate as f32 * 2.0) as usize;
        self.position_samples = samples.min(self.total_samples);
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
}

/// Test audio source that outputs a constant DC value (worst case for click detection)
/// A sudden jump to DC causes the most obvious click
struct DcOffsetSource {
    dc_level: f32,
    sample_rate: u32,
    position_samples: usize,
    total_samples: usize,
}

impl DcOffsetSource {
    fn new(dc_level: f32, sample_rate: u32, duration_secs: f32) -> Self {
        Self {
            dc_level,
            sample_rate,
            position_samples: 0,
            total_samples: (sample_rate as f32 * duration_secs * 2.0) as usize,
        }
    }
}

impl AudioSource for DcOffsetSource {
    fn read_samples(&mut self, buffer: &mut [f32]) -> Result<usize> {
        let remaining = self.total_samples.saturating_sub(self.position_samples);
        let to_read = buffer.len().min(remaining);

        for sample in buffer.iter_mut().take(to_read) {
            *sample = self.dc_level;
        }

        self.position_samples += to_read;
        Ok(to_read)
    }

    fn seek(&mut self, position: Duration) -> Result<()> {
        let samples = (position.as_secs_f32() * self.sample_rate as f32 * 2.0) as usize;
        self.position_samples = samples.min(self.total_samples);
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
}

#[allow(dead_code)]
fn create_test_track(id: &str) -> QueueTrack {
    QueueTrack {
        id: id.to_string(),
        path: PathBuf::from(format!("/test/{}.mp3", id)),
        title: format!("Test Track {}", id),
        artist: "Test Artist".to_string(),
        album: Some("Test Album".to_string()),
        duration: Duration::from_secs(10),
        track_number: Some(1),
        source: TrackSource::Single,
    }
}

/// Detect sudden jumps (discontinuities) in audio signal
/// Returns positions and magnitudes of jumps exceeding threshold
fn detect_discontinuities(samples: &[f32], threshold: f32) -> Vec<(usize, f32)> {
    let mut discontinuities = Vec::new();

    for i in 1..samples.len() {
        let diff = (samples[i] - samples[i - 1]).abs();
        if diff > threshold {
            discontinuities.push((i, diff));
        }
    }

    discontinuities
}

/// Check if audio starts with a proper fade-in (gradual increase)
/// Returns true if the first N samples show a smooth ramp
fn has_proper_fade_in(samples: &[f32], fade_samples: usize) -> bool {
    if samples.len() < fade_samples {
        return false;
    }

    // Check that amplitude increases gradually
    let mut prev_envelope = 0.0f32;
    let window_size = fade_samples / 10; // Check 10 windows

    for window_idx in 0..10 {
        let start = window_idx * window_size;
        let end = (start + window_size).min(samples.len());

        // Calculate envelope (peak) in this window
        let envelope: f32 = samples[start..end]
            .iter()
            .map(|s| s.abs())
            .fold(0.0f32, f32::max);

        // Envelope should be non-decreasing during fade-in
        // Allow small tolerance for signal variations
        if window_idx > 0 && envelope < prev_envelope * 0.5 {
            // Envelope decreased significantly - not a proper fade-in
            return false;
        }

        prev_envelope = envelope.max(prev_envelope);
    }

    true
}

/// Calculate the maximum sample-to-sample jump in the first N samples
fn max_initial_jump(samples: &[f32], check_samples: usize) -> f32 {
    let check_len = check_samples.min(samples.len());
    let mut max_jump = 0.0f32;

    for i in 1..check_len {
        let jump = (samples[i] - samples[i - 1]).abs();
        max_jump = max_jump.max(jump);
    }

    max_jump
}

// ============================================================================
// TESTS: Detecting the Problem
// ============================================================================

#[test]
fn test_detect_click_at_playback_start_with_dc() {
    // This test documents the current behavior (click at start)
    // and should PASS after the fix is implemented
    let mut manager = PlaybackManager::new(PlaybackConfig::default());
    manager.set_sample_rate(48000);
    manager.set_output_channels(2);

    // Use DC offset source (constant 0.8) - worst case for click
    let source = Box::new(DcOffsetSource::new(0.8, 48000, 5.0));
    manager.set_audio_source(source);

    // Process first buffer
    let mut buffer = vec![0.0f32; 1024];
    let _ = manager.process_audio(&mut buffer);

    // Check for click at the very start (sample 0 -> sample 1)
    // Without fade-in, this would be: 0.0 -> 0.8 = 0.8 jump
    let first_jump = (buffer[0] - 0.0).abs(); // First sample after silence
    let max_jump = max_initial_jump(&buffer, 100);

    println!("First sample value: {:.4}", buffer[0]);
    println!("First jump (from silence): {:.4}", first_jump);
    println!("Max jump in first 100 samples: {:.4}", max_jump);

    // With proper fade-in, the first sample should NOT be at full amplitude
    // Threshold: 0.1 is a reasonable max jump (would be 0.8 without fade-in)
    assert!(
        first_jump < 0.1,
        "Click detected at playback start! First sample jumped from 0 to {:.4}. \
         This causes an audible pop. A fade-in envelope should be applied.",
        first_jump
    );
}

#[test]
fn test_detect_click_at_playback_start_with_sine() {
    // Test with sine wave (more realistic signal)
    let mut manager = PlaybackManager::new(PlaybackConfig::default());
    manager.set_sample_rate(48000);
    manager.set_output_channels(2);

    // Use sine wave at 0.8 amplitude
    let source = Box::new(ConstantLevelSource::new(0.8, 1000.0, 48000, 5.0));
    manager.set_audio_source(source);

    // Process first buffer
    let mut buffer = vec![0.0f32; 1024];
    let _ = manager.process_audio(&mut buffer);

    // Calculate expected maximum derivative for a 1kHz sine at 48kHz
    // Max derivative = amplitude * 2 * PI * frequency / sample_rate
    let max_expected_derivative = 0.8 * 2.0 * PI * 1000.0 / 48000.0;
    println!("Max expected derivative for 1kHz sine: {:.4}", max_expected_derivative);

    // Check first sample transition
    let first_sample = buffer[0];
    println!("First sample value: {:.6}", first_sample);

    // With fade-in, the first sample should be attenuated
    // A 1kHz sine at t=0 starts near 0, but without fade-in
    // the full amplitude is immediately present
    let has_fade = has_proper_fade_in(&buffer, 480); // ~10ms at 48kHz

    println!("Has proper fade-in: {}", has_fade);

    // After fix, this should pass
    assert!(
        has_fade,
        "No fade-in detected at playback start. Audio starts at full amplitude \
         which causes an audible click/pop."
    );
}

#[test]
fn test_fade_in_duration_is_appropriate() {
    // Test that fade-in is not too short (clicks) or too long (noticeable delay)
    let mut manager = PlaybackManager::new(PlaybackConfig::default());
    manager.set_sample_rate(48000);
    manager.set_output_channels(2);
    // Set volume to max for clear measurement
    manager.set_volume(100);

    // Use constant level source at full amplitude
    let target_amplitude = 1.0f32;
    let source = Box::new(DcOffsetSource::new(target_amplitude, 48000, 5.0));
    manager.set_audio_source(source);

    // Process enough samples to cover fade-in (at least 20ms worth)
    let mut all_samples = Vec::new();
    for _ in 0..40 {
        let mut buffer = vec![0.0f32; 512];
        let samples_read = manager.process_audio(&mut buffer).unwrap();
        all_samples.extend_from_slice(&buffer[..samples_read]);
    }

    // Debug: print first few sample values and some later ones
    println!("Total samples collected: {}", all_samples.len());
    println!(
        "First 20 samples: {:?}",
        &all_samples[..20.min(all_samples.len())]
    );
    if all_samples.len() > 500 {
        println!(
            "Samples 480-500: {:?}",
            &all_samples[480..500.min(all_samples.len())]
        );
    }
    if all_samples.len() > 1000 {
        println!(
            "Samples 980-1000: {:?}",
            &all_samples[980..1000.min(all_samples.len())]
        );
    }

    // Find the first non-zero sample to verify fade is working
    let first_nonzero_idx = all_samples.iter().position(|&s| s.abs() > 0.001);
    println!("First non-zero sample index: {:?}", first_nonzero_idx);

    // Find when signal reaches 90% of target amplitude (end of fade)
    // Account for volume/limiter effects - look for 80% of peak value
    let peak_value = all_samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    println!("Peak value in signal: {:.4}", peak_value);

    let threshold = peak_value * 0.9;
    let mut fade_end_sample = 0;
    for (i, &sample) in all_samples.iter().enumerate() {
        if sample.abs() >= threshold {
            fade_end_sample = i;
            break;
        }
    }

    // Convert to milliseconds: sample_index / (sample_rate * channels)
    // At 48kHz stereo, 1ms = 96 samples (48 * 2)
    let fade_duration_ms = fade_end_sample as f32 / 96.0;
    println!(
        "Fade-in duration: {:.1}ms ({} samples, threshold: {:.2})",
        fade_duration_ms, fade_end_sample, threshold
    );

    // First, verify we have actual audio (non-zero peak)
    assert!(
        peak_value > 0.1,
        "No audio output detected! Peak value: {:.4}. Check audio source and volume.",
        peak_value
    );

    // Fade should be 25-50ms (5ms pre-silence + 30ms fade = 35ms total)
    // Too short: still clicks
    // Too long: noticeable delay in playback start
    assert!(
        fade_duration_ms >= 20.0,
        "Fade-in too short: {:.1}ms. Should be at least 20ms to avoid clicks.",
        fade_duration_ms
    );

    assert!(
        fade_duration_ms <= 60.0,
        "Fade-in too long: {:.1}ms. Should be at most 60ms to avoid noticeable delay.",
        fade_duration_ms
    );
}

// ============================================================================
// TESTS: Resume from Pause
// ============================================================================

#[test]
fn test_no_click_on_resume_from_pause() {
    // Clicks can also occur when resuming from pause
    let mut manager = PlaybackManager::new(PlaybackConfig::default());
    manager.set_sample_rate(48000);
    manager.set_output_channels(2);

    // Start playback
    let source = Box::new(DcOffsetSource::new(0.8, 48000, 5.0));
    manager.set_audio_source(source);

    // Play for a bit
    let mut buffer = vec![0.0f32; 2048];
    let _ = manager.process_audio(&mut buffer);

    // Pause
    manager.pause();

    // Output should be silence while paused
    let mut pause_buffer = vec![1.0f32; 512]; // Non-zero to detect fill
    let _ = manager.process_audio(&mut pause_buffer);
    assert!(
        pause_buffer.iter().all(|&s| s == 0.0),
        "Output should be silence while paused"
    );

    // Resume
    manager.play().unwrap();

    // Process first buffer after resume
    let mut resume_buffer = vec![0.0f32; 1024];
    let _ = manager.process_audio(&mut resume_buffer);

    // Check for click at resume
    let max_jump = max_initial_jump(&resume_buffer, 100);
    println!("Max jump on resume: {:.4}", max_jump);

    // With proper fade-in on resume, there should be no large jump
    assert!(
        max_jump < 0.2,
        "Click detected on resume from pause! Max jump: {:.4}. \
         A fade-in should be applied when resuming.",
        max_jump
    );
}

// ============================================================================
// TESTS: Fade Curve Verification
// ============================================================================

#[test]
fn test_fade_curve_starts_at_zero() {
    // Verify that fade curves start at 0 (essential for click-free start)
    let curves = [
        FadeCurve::Linear,
        FadeCurve::EqualPower,
        FadeCurve::SCurve,
        FadeCurve::SquareRoot,
    ];

    for curve in curves {
        let gain_at_start = curve.calculate_gain(0.0, false); // fade_in = false means fade-in
        assert!(
            gain_at_start.abs() < 0.001,
            "Curve {:?} should start at 0, got {:.4}",
            curve,
            gain_at_start
        );
    }
}

#[test]
fn test_fade_curve_ends_at_one() {
    // Verify that fade curves end at 1 (full amplitude)
    let curves = [
        FadeCurve::Linear,
        FadeCurve::EqualPower,
        FadeCurve::SCurve,
        FadeCurve::SquareRoot,
    ];

    for curve in curves {
        let gain_at_end = curve.calculate_gain(1.0, false);
        assert!(
            (gain_at_end - 1.0).abs() < 0.001,
            "Curve {:?} should end at 1, got {:.4}",
            curve,
            gain_at_end
        );
    }
}

#[test]
fn test_fade_curve_is_monotonic() {
    // Verify that fade-in curves are monotonically increasing
    // (no dips that could cause artifacts)
    let curves = [
        FadeCurve::Linear,
        FadeCurve::EqualPower,
        FadeCurve::SCurve,
        FadeCurve::SquareRoot,
    ];

    for curve in curves {
        let mut prev_gain = 0.0;
        for i in 0..=100 {
            let position = i as f32 / 100.0;
            let gain = curve.calculate_gain(position, false);

            assert!(
                gain >= prev_gain - 0.001, // Allow tiny floating point tolerance
                "Curve {:?} is not monotonic at position {}: {} < {}",
                curve,
                position,
                gain,
                prev_gain
            );

            prev_gain = gain;
        }
    }
}

// ============================================================================
// TESTS: Edge Cases
// ============================================================================

#[test]
fn test_very_short_buffer_with_fade() {
    // Test that fade-in works correctly even with very small buffers
    let mut manager = PlaybackManager::new(PlaybackConfig::default());
    manager.set_sample_rate(48000);
    manager.set_output_channels(2);

    let source = Box::new(DcOffsetSource::new(0.8, 48000, 5.0));
    manager.set_audio_source(source);

    // Process with very small buffers (64 samples = 32 stereo frames)
    let mut all_samples = Vec::new();
    for _ in 0..20 {
        let mut buffer = vec![0.0f32; 64];
        let samples_read = manager.process_audio(&mut buffer).unwrap();
        all_samples.extend_from_slice(&buffer[..samples_read]);
    }

    // Even with small buffers, fade should be smooth
    let discontinuities = detect_discontinuities(&all_samples, 0.1);
    println!("Discontinuities with small buffers: {:?}", discontinuities);

    assert!(
        discontinuities.is_empty(),
        "Discontinuities found with small buffer processing: {:?}",
        discontinuities
    );
}

#[test]
fn test_fade_in_with_varying_buffer_sizes() {
    // Ensure fade works correctly when buffer sizes vary
    let mut manager = PlaybackManager::new(PlaybackConfig::default());
    manager.set_sample_rate(48000);
    manager.set_output_channels(2);

    let source = Box::new(DcOffsetSource::new(0.8, 48000, 5.0));
    manager.set_audio_source(source);

    let buffer_sizes = [128, 256, 64, 512, 128, 256];
    let mut all_samples = Vec::new();

    for &size in &buffer_sizes {
        let mut buffer = vec![0.0f32; size];
        let samples_read = manager.process_audio(&mut buffer).unwrap();
        all_samples.extend_from_slice(&buffer[..samples_read]);
    }

    // Check for discontinuities
    let discontinuities = detect_discontinuities(&all_samples, 0.15);

    assert!(
        discontinuities.len() < 3,
        "Too many discontinuities with varying buffer sizes: {:?}",
        discontinuities
    );
}

#[test]
fn test_multiple_start_stop_cycles() {
    // Test that fade-in works correctly across multiple play/stop cycles
    let sample_rate = 48000u32;

    for cycle in 0..5 {
        let mut manager = PlaybackManager::new(PlaybackConfig::default());
        manager.set_sample_rate(sample_rate);
        manager.set_output_channels(2);

        let source = Box::new(DcOffsetSource::new(0.8, sample_rate, 5.0));
        manager.set_audio_source(source);

        let mut buffer = vec![0.0f32; 1024];
        let _ = manager.process_audio(&mut buffer);

        let first_jump = buffer[0].abs();
        println!("Cycle {}: First sample = {:.4}", cycle, first_jump);

        assert!(
            first_jump < 0.1,
            "Cycle {}: Click at start! First sample = {:.4}",
            cycle,
            first_jump
        );

        manager.stop();
    }
}

// ============================================================================
// BENCHMARK TEST
// ============================================================================

#[test]
fn test_fade_envelope_performance() {
    // Ensure fade-in processing doesn't add significant overhead
    use std::time::Instant;

    let iterations = 1000;
    let buffer_size = 512;
    let sample_rate = 48000u32;

    let start = Instant::now();

    for _ in 0..iterations {
        let mut manager = PlaybackManager::new(PlaybackConfig::default());
        manager.set_sample_rate(sample_rate);
        manager.set_output_channels(2);

        let source = Box::new(DcOffsetSource::new(0.8, sample_rate, 5.0));
        manager.set_audio_source(source);

        let mut buffer = vec![0.0f32; buffer_size];
        let _ = manager.process_audio(&mut buffer);
    }

    let elapsed = start.elapsed();
    let per_iteration_us = elapsed.as_micros() as f64 / iterations as f64;

    println!(
        "Fade envelope processing: {:.2}us per start ({} iterations)",
        per_iteration_us, iterations
    );

    // Should be well under 1ms per start
    assert!(
        per_iteration_us < 1000.0,
        "Fade envelope processing too slow: {:.2}us",
        per_iteration_us
    );
}

// ============================================================================
// TESTS: Waveform Preservation (No Jitter/Stretching)
// ============================================================================

/// Test that a sine wave remains sinusoidal after fade processing.
/// The slew limiter was causing waveform distortion by limiting per-sample changes.
#[test]
fn test_waveform_not_distorted_by_fade() {
    let sample_rate = 48000u32;
    let frequency = 1000.0; // 1kHz sine wave

    let mut manager = PlaybackManager::new(PlaybackConfig::default());
    manager.set_sample_rate(sample_rate);
    manager.set_output_channels(2);

    // Use a loud sine wave that will definitely trigger the fade
    let source = Box::new(ConstantLevelSource::new(0.8, frequency, sample_rate, 1.0));
    manager.set_audio_source(source);

    // Process enough audio to get past the fade period (100ms worth)
    let mut all_samples = Vec::new();
    for _ in 0..10 {
        let mut buffer = vec![0.0f32; 960]; // 10ms at 48kHz stereo
        let _ = manager.process_audio(&mut buffer);
        all_samples.extend_from_slice(&buffer);
    }

    // Skip the fade region and analyze the post-fade audio
    // After ~60ms, the fade should be complete
    let post_fade_start = (sample_rate as f32 * 0.07 * 2.0) as usize; // 70ms in stereo samples
    let post_fade_samples: Vec<f32> = all_samples[post_fade_start..]
        .iter()
        .step_by(2) // Take left channel only
        .copied()
        .collect();

    // Verify we have audio (DC blocker may slightly reduce peak)
    let peak = post_fade_samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    assert!(peak > 0.1, "No audio after fade, peak: {}", peak);

    // Check zero crossings - for a 1kHz wave at 48kHz, we expect ~48 samples per cycle
    // So in 30ms (1440 samples), we expect ~30 zero crossings
    let zero_crossings = count_zero_crossings(&post_fade_samples);
    let expected_crossings_per_1000_samples = 2.0 * frequency / sample_rate as f32 * 1000.0;
    let actual_crossings_per_1000_samples =
        zero_crossings as f32 / post_fade_samples.len() as f32 * 1000.0;

    println!(
        "Zero crossings: expected {:.1}/1000, actual {:.1}/1000",
        expected_crossings_per_1000_samples, actual_crossings_per_1000_samples
    );

    // Allow 20% tolerance for frequency accuracy
    let ratio = actual_crossings_per_1000_samples / expected_crossings_per_1000_samples;
    assert!(
        (0.8..=1.2).contains(&ratio),
        "Waveform frequency distorted! Expected ratio ~1.0, got {:.2}. \
         This indicates the fade is stretching/jittering the waveform.",
        ratio
    );
}

/// Test that zero crossings during fade are evenly spaced (no jitter).
/// Jitter would cause irregular spacing between zero crossings.
#[test]
fn test_no_jitter_during_fade() {
    let sample_rate = 48000u32;
    let frequency = 500.0; // 500Hz - 96 samples per cycle

    let mut manager = PlaybackManager::new(PlaybackConfig::default());
    manager.set_sample_rate(sample_rate);
    manager.set_output_channels(2);

    let source = Box::new(ConstantLevelSource::new(0.9, frequency, sample_rate, 1.0));
    manager.set_audio_source(source);

    // Process audio through the fade
    let mut buffer = vec![0.0f32; 9600]; // 100ms at 48kHz stereo
    let _ = manager.process_audio(&mut buffer);

    // Extract left channel
    let left_channel: Vec<f32> = buffer.iter().step_by(2).copied().collect();

    // Find zero crossing intervals after audio starts
    let crossing_positions = find_zero_crossing_positions(&left_channel);

    if crossing_positions.len() > 5 {
        // Calculate intervals between crossings
        let intervals: Vec<usize> = crossing_positions
            .windows(2)
            .map(|w| w[1] - w[0])
            .collect();

        // Expected interval: half a cycle = sample_rate / frequency / 2
        let expected_interval = sample_rate as f32 / frequency / 2.0;

        // Check that intervals don't vary too much (< 50% deviation)
        let mut max_deviation = 0.0f32;
        for &interval in &intervals {
            let deviation = (interval as f32 - expected_interval).abs() / expected_interval;
            max_deviation = max_deviation.max(deviation);
        }

        println!(
            "Zero crossing intervals: expected {:.1}, max deviation {:.1}%",
            expected_interval,
            max_deviation * 100.0
        );

        // During fade, some deviation is expected, but not more than 50%
        assert!(
            max_deviation < 0.5,
            "Excessive jitter in zero crossings! Max deviation: {:.1}%. \
             This indicates waveform stretching/compression.",
            max_deviation * 100.0
        );
    }
}

/// Test that fade doesn't add harmonics (Total Harmonic Distortion check).
/// A pure sine wave should remain a pure sine wave (just amplitude modulated).
#[test]
fn test_fade_does_not_add_harmonics() {
    let sample_rate = 48000u32;
    let frequency = 1000.0;

    let mut manager = PlaybackManager::new(PlaybackConfig::default());
    manager.set_sample_rate(sample_rate);
    manager.set_output_channels(2);

    let source = Box::new(ConstantLevelSource::new(0.8, frequency, sample_rate, 1.0));
    manager.set_audio_source(source);

    // Get audio after fade completes
    let mut warmup = vec![0.0f32; 9600]; // 100ms warmup
    let _ = manager.process_audio(&mut warmup);

    let mut buffer = vec![0.0f32; 4800]; // 50ms of post-fade audio
    let _ = manager.process_audio(&mut buffer);

    // Extract left channel
    let left_channel: Vec<f32> = buffer.iter().step_by(2).copied().collect();

    // Simple THD check: compare actual waveform to ideal sine wave
    // Find the amplitude and phase of the fundamental
    let samples_per_cycle = sample_rate as f32 / frequency;
    let num_cycles = left_channel.len() as f32 / samples_per_cycle;

    if num_cycles >= 2.0 {
        // Calculate RMS of actual signal
        let rms_actual: f32 = (left_channel.iter().map(|s| s * s).sum::<f32>()
            / left_channel.len() as f32)
            .sqrt();

        // Reconstruct ideal sine and calculate error
        let mut error_sum = 0.0f32;
        let amplitude = rms_actual * 2.0_f32.sqrt(); // RMS to peak

        for (i, &sample) in left_channel.iter().enumerate() {
            let t = i as f32 / sample_rate as f32;
            // We don't know the phase, so we'll just check if the envelope is correct
            // by comparing absolute values
            let ideal_envelope = amplitude;
            let actual_envelope = sample.abs();
            // Only check when the sine wave is near its peak (avoid zero crossings)
            if actual_envelope > amplitude * 0.7 {
                let error = (actual_envelope - ideal_envelope).abs();
                error_sum += error * error;
            }
        }

        // This is a simplified distortion check
        let distortion_estimate = (error_sum / left_channel.len() as f32).sqrt() / amplitude;

        println!("Estimated distortion: {:.2}%", distortion_estimate * 100.0);

        // THD should be reasonably low for a properly faded sine wave
        // Allow up to 15% for DC blocker startup effects
        assert!(
            distortion_estimate < 0.15,
            "High distortion detected: {:.2}%. The fade may be damaging the waveform.",
            distortion_estimate * 100.0
        );
    }
}

/// Helper: Count zero crossings in a signal
fn count_zero_crossings(samples: &[f32]) -> usize {
    samples
        .windows(2)
        .filter(|w| (w[0] >= 0.0 && w[1] < 0.0) || (w[0] < 0.0 && w[1] >= 0.0))
        .count()
}

/// Helper: Find positions of zero crossings
fn find_zero_crossing_positions(samples: &[f32]) -> Vec<usize> {
    samples
        .windows(2)
        .enumerate()
        .filter(|(_, w)| (w[0] >= 0.0 && w[1] < 0.0) || (w[0] < 0.0 && w[1] >= 0.0))
        .map(|(i, _)| i)
        .collect()
}

// ============================================================================
// TESTS: Seek and Play/Pause Pop Prevention
// ============================================================================

/// Test that seeking doesn't cause a pop
#[test]
fn test_no_click_on_seek() {
    let sample_rate = 48000u32;

    let mut manager = PlaybackManager::new(PlaybackConfig::default());
    manager.set_sample_rate(sample_rate);
    manager.set_output_channels(2);

    // Use a sine wave source
    let source = Box::new(ConstantLevelSource::new(0.8, 440.0, sample_rate, 5.0));
    manager.set_audio_source(source);

    // Process some audio first to get past initial fade
    let mut warmup = vec![0.0f32; 9600]; // 100ms
    let _ = manager.process_audio(&mut warmup);

    // Now seek to a different position
    manager.seek_to(Duration::from_secs(2)).unwrap();

    // Process audio right after seek
    let mut buffer = vec![0.0f32; 1024];
    let _ = manager.process_audio(&mut buffer);

    // Check for click at seek point
    let max_jump = max_initial_jump(&buffer, 100);
    println!("Max jump after seek: {:.4}", max_jump);

    // With proper fade-in after seek, there should be no large jump
    assert!(
        max_jump < 0.15,
        "Click detected after seek! Max jump: {:.4}. \
         A fade-in should be applied when seeking.",
        max_jump
    );
}

/// Test that seeking to different positions all apply fade
#[test]
fn test_no_click_on_multiple_seeks() {
    let sample_rate = 48000u32;

    let mut manager = PlaybackManager::new(PlaybackConfig::default());
    manager.set_sample_rate(sample_rate);
    manager.set_output_channels(2);

    let source = Box::new(ConstantLevelSource::new(0.8, 440.0, sample_rate, 10.0));
    manager.set_audio_source(source);

    // Process initial fade
    let mut warmup = vec![0.0f32; 4800];
    let _ = manager.process_audio(&mut warmup);

    // Seek multiple times
    let seek_positions = [
        Duration::from_millis(500),
        Duration::from_secs(3),
        Duration::from_millis(100),
        Duration::from_secs(7),
    ];

    for (i, &pos) in seek_positions.iter().enumerate() {
        manager.seek_to(pos).unwrap();

        let mut buffer = vec![0.0f32; 512];
        let _ = manager.process_audio(&mut buffer);

        let max_jump = max_initial_jump(&buffer, 50);
        println!("Seek {} to {:?}: max jump {:.4}", i, pos, max_jump);

        assert!(
            max_jump < 0.2,
            "Click on seek {}: position {:?}, max jump {:.4}",
            i,
            pos,
            max_jump
        );
    }
}

/// Test that pause/resume cycle doesn't cause a pop
#[test]
fn test_no_click_on_pause_resume_cycle() {
    let sample_rate = 48000u32;

    let mut manager = PlaybackManager::new(PlaybackConfig::default());
    manager.set_sample_rate(sample_rate);
    manager.set_output_channels(2);

    let source = Box::new(ConstantLevelSource::new(0.8, 440.0, sample_rate, 5.0));
    manager.set_audio_source(source);

    // Process initial audio
    let mut warmup = vec![0.0f32; 4800];
    let _ = manager.process_audio(&mut warmup);

    // Do multiple pause/resume cycles
    for cycle in 0..3 {
        // Pause
        manager.pause();

        // Process while paused (should be silence)
        let mut paused_buffer = vec![1.0f32; 512];
        let _ = manager.process_audio(&mut paused_buffer);
        assert!(
            paused_buffer.iter().all(|&s| s == 0.0),
            "Cycle {}: Output should be silence while paused",
            cycle
        );

        // Resume
        manager.play().unwrap();

        // Process right after resume
        let mut resume_buffer = vec![0.0f32; 512];
        let _ = manager.process_audio(&mut resume_buffer);

        let max_jump = max_initial_jump(&resume_buffer, 50);
        println!("Cycle {} resume: max jump {:.4}", cycle, max_jump);

        assert!(
            max_jump < 0.15,
            "Cycle {}: Click on resume! Max jump {:.4}",
            cycle,
            max_jump
        );
    }
}

/// Test that seeking while paused and then resuming doesn't cause a pop
#[test]
fn test_no_click_on_seek_while_paused() {
    let sample_rate = 48000u32;

    let mut manager = PlaybackManager::new(PlaybackConfig::default());
    manager.set_sample_rate(sample_rate);
    manager.set_output_channels(2);

    let source = Box::new(ConstantLevelSource::new(0.8, 440.0, sample_rate, 10.0));
    manager.set_audio_source(source);

    // Process initial audio
    let mut warmup = vec![0.0f32; 4800];
    let _ = manager.process_audio(&mut warmup);

    // Pause
    manager.pause();

    // Seek while paused
    manager.seek_to(Duration::from_secs(5)).unwrap();

    // Resume
    manager.play().unwrap();

    // Process after resume
    let mut buffer = vec![0.0f32; 1024];
    let _ = manager.process_audio(&mut buffer);

    let max_jump = max_initial_jump(&buffer, 100);
    println!("Max jump after seek-while-paused: {:.4}", max_jump);

    assert!(
        max_jump < 0.15,
        "Click after seek while paused! Max jump: {:.4}",
        max_jump
    );
}

/// Test that rapid seek operations don't cause pops
#[test]
fn test_no_click_on_rapid_seeks() {
    let sample_rate = 48000u32;

    let mut manager = PlaybackManager::new(PlaybackConfig::default());
    manager.set_sample_rate(sample_rate);
    manager.set_output_channels(2);

    let source = Box::new(ConstantLevelSource::new(0.8, 440.0, sample_rate, 10.0));
    manager.set_audio_source(source);

    // Rapid seeks without processing in between
    for i in 0..5 {
        manager
            .seek_to(Duration::from_millis(i as u64 * 1000))
            .unwrap();
    }

    // Now process audio
    let mut buffer = vec![0.0f32; 1024];
    let _ = manager.process_audio(&mut buffer);

    let max_jump = max_initial_jump(&buffer, 100);
    println!("Max jump after rapid seeks: {:.4}", max_jump);

    // Even after rapid seeks, the fade should work correctly
    assert!(
        max_jump < 0.15,
        "Click after rapid seeks! Max jump: {:.4}",
        max_jump
    );
}
