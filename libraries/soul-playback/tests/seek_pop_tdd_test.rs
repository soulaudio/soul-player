//! TDD tests for audio pop/click at seek.
//!
//! Root cause: `seek_to()` in PlaybackManager calls `source.seek()` but never
//! calls `self.start_fade.start()`. After seek, audio transitions abruptly
//! from silence (zero-fill during flush) to full amplitude — an audible pop.
//!
//! Fix: call `self.start_fade.start()` inside `seek_to()` after a successful
//! `source.seek()`.

use soul_playback::{
    AudioSource, PlaybackConfig, PlaybackManager, QueueTrack, Result, TrackSource,
};
use std::f32::consts::PI;
use std::path::PathBuf;
use std::time::Duration;

// ============================================================================
// Test source: constant-amplitude sine wave, supports seek.
//
// A sine wave at 440 Hz is used instead of DC because the start_fade envelope
// contains a DC blocker (highpass filter, coeff=0.9975) that would attenuate a
// true DC signal to near-zero after ~2000 samples, making it useless as a
// "full amplitude" test signal.
// ============================================================================

struct SineSource {
    amplitude: f32,
    frequency: f32,
    sample_rate: u32,
    position_samples: usize, // stereo samples
    total_samples: usize,    // stereo samples
}

impl SineSource {
    fn new(amplitude: f32, sample_rate: u32, duration_secs: f32) -> Self {
        Self {
            amplitude,
            frequency: 440.0,
            sample_rate,
            position_samples: 0,
            total_samples: (sample_rate as f32 * duration_secs * 2.0) as usize,
        }
    }
}

impl AudioSource for SineSource {
    fn read_samples(&mut self, buffer: &mut [f32]) -> Result<usize> {
        let remaining = self.total_samples.saturating_sub(self.position_samples);
        let to_read = buffer.len().min(remaining);
        let frames = to_read / 2;
        for i in 0..frames {
            let frame_idx = (self.position_samples / 2) + i;
            let t = frame_idx as f32 / self.sample_rate as f32;
            let s = self.amplitude * (2.0 * PI * self.frequency * t).sin();
            buffer[i * 2] = s;
            buffer[i * 2 + 1] = s;
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

fn make_track(id: &str) -> QueueTrack {
    QueueTrack {
        id: id.to_string(),
        path: PathBuf::from(format!("/test/{}.mp3", id)),
        title: format!("Track {}", id),
        artist: "Artist".to_string(),
        album: Some("Album".to_string()),
        duration: Duration::from_secs(60),
        track_number: Some(1),
        source: TrackSource::Single,
    }
}

// ============================================================================
// Helpers
// ============================================================================

/// Set up a PlaybackManager in Playing state with the given source.
fn manager_playing(source: Box<dyn AudioSource>, track: QueueTrack) -> PlaybackManager {
    let mut mgr = PlaybackManager::new(PlaybackConfig::default());
    mgr.set_sample_rate(48000);
    mgr.set_output_channels(2);
    mgr.set_volume(100);
    let t = track.clone();
    mgr.add_to_queue_end(t);
    mgr.play().expect("play() should succeed");
    mgr.activate_source(source, track);
    mgr
}

/// Drain `n_buffers` × 512-sample buffers. Returns all samples.
fn drain_buffers(manager: &mut PlaybackManager, n_buffers: usize) -> Vec<f32> {
    let mut all = Vec::new();
    for _ in 0..n_buffers {
        let mut buf = vec![0.0f32; 512];
        let _ = manager.process_audio(&mut buf);
        all.extend_from_slice(&buf);
    }
    all
}

/// Peak absolute value in a slice.
fn peak(samples: &[f32]) -> f32 {
    samples
        .iter()
        .copied()
        .fold(0.0f32, |acc, s| acc.max(s.abs()))
}

// ============================================================================
// TESTS
// ============================================================================

/// Baseline: start_fade applies at track load — first sample is attenuated.
/// Must pass both before and after our fix.
#[test]
fn baseline_start_fade_applies_at_track_load() {
    let track = make_track("baseline");
    let source = Box::new(SineSource::new(0.8, 48000, 30.0));
    let mut mgr = manager_playing(source, track);

    let mut buf = vec![0.0f32; 512];
    let _ = mgr.process_audio(&mut buf);

    // First sample must be attenuated — start_fade is ramping up.
    // 440 Hz sine at t=0 has value 0, but at t=small it's close to 0 anyway.
    // Use peak of first 50 stereo samples (25 frames ≈ 0.5ms) as the indicator.
    let first_peak = peak(&buf[..50]);
    assert!(
        first_peak < 0.1,
        "baseline: first 50 samples peak should be attenuated by start_fade, got {:.4}",
        first_peak
    );
}

/// Sanity: after ~100ms, audio is at full amplitude (start_fade completed).
#[test]
fn sanity_audio_at_full_amplitude_after_warmup() {
    let track = make_track("sanity");
    let source = Box::new(SineSource::new(0.8, 48000, 30.0));
    let mut mgr = manager_playing(source, track);

    // ~100ms = 4800 frames = 9600 stereo samples. start_fade = 30ms.
    let warmup = drain_buffers(&mut mgr, 20);
    let p = peak(&warmup);
    // 440 Hz sine at 0.8 amplitude × volume 100 = 0.8 peak expected
    assert!(
        p > 0.5,
        "sanity: after warmup audio should be at full amplitude, got peak={:.4}",
        p
    );
}

/// THE KEY TEST: after a seek, the first post-seek buffer must start near zero
/// (fade-in applied), NOT jump immediately to full amplitude (pop).
///
/// Without the fix: `seek_to()` never restarts start_fade; first post-seek
/// buffer immediately produces full-amplitude audio → audible pop.
///
/// With the fix: `seek_to()` calls `self.start_fade.start()`; first
/// post-seek buffer is faded in from silence → no pop.
#[test]
fn seek_triggers_start_fade_no_pop() {
    let track = make_track("seek-pop");
    let source = Box::new(SineSource::new(0.8, 48000, 30.0));
    let mut mgr = manager_playing(source, track);

    // Warm up: ~100ms so start_fade completes and audio reaches full amplitude.
    let warmup = drain_buffers(&mut mgr, 20);
    assert!(
        peak(&warmup) > 0.5,
        "sanity: audio should be at full amplitude after warmup"
    );

    // Seek to 10 seconds.
    mgr.seek_to(Duration::from_secs(10))
        .expect("seek_to should succeed");

    // Capture the first post-seek buffer.
    let mut post_seek_buf = vec![0.0f32; 512];
    let _ = mgr.process_audio(&mut post_seek_buf);

    // Use peak of first 50 stereo samples as the "pop detector".
    // After seek, the sine continues at the new phase — but with fade-in
    // the first 50 samples should be near zero.
    let first_peak = peak(&post_seek_buf[..50]);
    println!("Peak of first 50 post-seek samples: {:.4}", first_peak);
    println!(
        "Peak of entire post-seek buffer: {:.4}",
        peak(&post_seek_buf)
    );

    // Without fix: first_peak ≈ 0.8 (no fade, full amplitude immediately)
    // With fix:    first_peak < 0.1 (start_fade attenuates to near zero)
    assert!(
        first_peak < 0.1,
        "POP DETECTED after seek: peak of first 50 samples is {:.4} (should be < 0.1). \
         Fix: call self.start_fade.start() in seek_to() after source.seek().",
        first_peak
    );
}

/// Multiple consecutive seeks should each trigger a fresh fade-in.
#[test]
fn multiple_seeks_each_trigger_fade_in() {
    let track = make_track("multi-seek");
    let source = Box::new(SineSource::new(0.8, 48000, 60.0));
    let mut mgr = manager_playing(source, track);

    // Warm up.
    drain_buffers(&mut mgr, 20);

    let seek_targets_secs: &[u64] = &[5, 20, 35, 50];

    for &t in seek_targets_secs {
        mgr.seek_to(Duration::from_secs(t))
            .expect("seek_to should succeed");

        // Capture the first post-seek buffer immediately.
        let mut buf = vec![0.0f32; 512];
        let _ = mgr.process_audio(&mut buf);

        let first_peak = peak(&buf[..50]);
        println!(
            "seek to {}s: peak of first 50 samples = {:.4}",
            t, first_peak
        );
        assert!(
            first_peak < 0.1,
            "POP at seek to {}s: peak of first 50 samples = {:.4} (should be < 0.1)",
            t,
            first_peak
        );

        // Let audio reach full amplitude again before next seek.
        drain_buffers(&mut mgr, 20);
    }
}
