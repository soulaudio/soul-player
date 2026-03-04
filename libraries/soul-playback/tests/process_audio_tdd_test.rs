//! TDD tests for process_audio() edge cases in PlaybackManager
//!
//! Scenarios tested (per the task brief):
//!
//! 1. Zero-size buffer — should not panic, returns Ok
//! 2. Odd-length buffer while in stereo mode — should not produce garbage or panic
//! 3. 1000 rapid calls while Stopped — fill_underrun_buffer must be consistent
//!    (DAC keepalive noise, never silence, LFSR must not lock to 0)
//! 4. process_audio() while Playing but with a source that reads fewer samples
//!    than the buffer length — remainder must be filled with keepalive noise
//!
//! All four scenarios were expected to be safe by design; the goal is to confirm
//! that assumption and catch any regression.

use soul_playback::{
    AudioSource, PlaybackEvent, PlaybackManager, PlaybackState, QueueTrack, TrackSource,
};
use std::path::PathBuf;
use std::time::Duration;

// ──────────────────────────────────────────────────────────────────────────────
// Test helpers
// ──────────────────────────────────────────────────────────────────────────────

fn make_track(id: &str, secs: u64) -> QueueTrack {
    QueueTrack {
        id: id.to_string(),
        path: PathBuf::from(format!("/music/{id}.mp3")),
        title: format!("Track {id}"),
        artist: "Artist".to_string(),
        album: None,
        duration: Duration::from_secs(secs),
        track_number: None,
        source: TrackSource::Single,
    }
}

/// Simple mock that fills with a fixed test value and tracks how many samples were read.
struct FixedAudioSource {
    duration: Duration,
    position: Duration,
    fill_value: f32,
    /// Simulate returning fewer samples than requested (e.g. last buffer of a track)
    max_per_call: Option<usize>,
}

impl FixedAudioSource {
    fn new(duration_secs: u64) -> Self {
        Self {
            duration: Duration::from_secs(duration_secs),
            position: Duration::ZERO,
            fill_value: 0.25,
            max_per_call: None,
        }
    }

    /// Returns at most `max_samples` per `read_samples` call.
    fn with_max_per_call(duration_secs: u64, max_samples: usize) -> Self {
        Self {
            max_per_call: Some(max_samples),
            ..Self::new(duration_secs)
        }
    }
}

impl AudioSource for FixedAudioSource {
    fn read_samples(&mut self, buffer: &mut [f32]) -> soul_playback::Result<usize> {
        let sample_rate = 88200.0_f64; // 44100 Hz stereo
        let total = (self.duration.as_secs_f64() * sample_rate) as usize;
        let current = (self.position.as_secs_f64() * sample_rate) as usize;
        let remaining = total.saturating_sub(current);

        let requested = buffer.len();
        let capped = match self.max_per_call {
            Some(max) => requested.min(max),
            None => requested,
        };
        let to_read = remaining.min(capped);

        if to_read == 0 {
            return Ok(0);
        }

        for s in buffer.iter_mut().take(to_read) {
            *s = self.fill_value;
        }
        self.position += Duration::from_secs_f64(to_read as f64 / sample_rate);
        Ok(to_read)
    }

    fn seek(&mut self, pos: Duration) -> soul_playback::Result<()> {
        self.position = pos.min(self.duration);
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

// ──────────────────────────────────────────────────────────────────────────────
// Scenario 1: zero-size buffer
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn process_audio_zero_size_buffer_stopped_does_not_panic() {
    let mut manager = PlaybackManager::default();
    assert_eq!(manager.get_state(), PlaybackState::Stopped);

    let mut buffer: Vec<f32> = vec![];
    let result = manager.process_audio(&mut buffer);
    assert!(result.is_ok(), "zero-size buffer must not return Err");
}

#[test]
fn process_audio_zero_size_buffer_playing_does_not_panic() {
    let mut manager = PlaybackManager::default();

    let track = make_track("1", 60);
    manager.load_playlist(vec![track.clone()], 0);
    manager.play().unwrap();
    manager.activate_source(Box::new(FixedAudioSource::new(60)), track);

    assert_eq!(manager.get_state(), PlaybackState::Playing);

    let mut buffer: Vec<f32> = vec![];
    let result = manager.process_audio(&mut buffer);
    assert!(
        result.is_ok(),
        "zero-size buffer while Playing must not return Err"
    );
}

#[test]
fn process_audio_zero_size_buffer_paused_does_not_panic() {
    let mut manager = PlaybackManager::default();

    let track = make_track("1", 60);
    manager.load_playlist(vec![track.clone()], 0);
    manager.play().unwrap();
    manager.activate_source(Box::new(FixedAudioSource::new(60)), track);
    manager.pause();

    let mut buffer: Vec<f32> = vec![];
    let result = manager.process_audio(&mut buffer);
    assert!(
        result.is_ok(),
        "zero-size buffer while Paused must not return Err"
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// Scenario 2: odd-length buffer in stereo mode
//
// The audio pipeline operates on interleaved stereo samples.  An odd number of
// samples means the last frame is incomplete.  The code must not panic and must
// not produce NaN/Inf values in output.
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn process_audio_odd_length_buffer_stopped_does_not_panic() {
    let mut manager = PlaybackManager::default();
    let mut buffer = vec![0.0f32; 3]; // odd
    let result = manager.process_audio(&mut buffer);
    assert!(result.is_ok());
    for &s in &buffer {
        assert!(
            s.is_finite(),
            "odd-length stopped buffer must not produce NaN/Inf"
        );
    }
}

#[test]
fn process_audio_odd_length_buffer_playing_does_not_panic_or_produce_garbage() {
    let mut manager = PlaybackManager::default();

    let track = make_track("1", 60);
    manager.load_playlist(vec![track.clone()], 0);
    manager.play().unwrap();
    manager.activate_source(Box::new(FixedAudioSource::new(60)), track);

    // Use an intentionally odd-length buffer
    let mut buffer = vec![-1.0f32; 7];
    let result = manager.process_audio(&mut buffer);

    assert!(
        result.is_ok(),
        "odd-length buffer while Playing must not return Err"
    );

    for (i, &s) in buffer.iter().enumerate() {
        assert!(
            s.is_finite(),
            "sample[{i}] must be finite with odd-length buffer, got {s}"
        );
        // Samples should not remain at the sentinel -1.0 (the source filled them)
        // (The start_fade envelope may reduce amplitude, so just check finiteness
        //  rather than exact value.)
    }
}

#[test]
fn process_audio_single_sample_buffer_does_not_panic() {
    let mut manager = PlaybackManager::default();
    let mut buffer = vec![999.0f32; 1]; // single sample
    let result = manager.process_audio(&mut buffer);
    assert!(result.is_ok(), "single-sample buffer must not return Err");
    assert!(buffer[0].is_finite(), "single-sample output must be finite");
}

// ──────────────────────────────────────────────────────────────────────────────
// Scenario 3: 1000 rapid calls while Stopped — LFSR must not lock to zero
//
// The fill_underrun_buffer LFSR uses XOR-shift.  XOR of 0 with any shift is 0,
// so if the state ever reaches 0 it stays 0, producing a stream of 0.0 samples.
// Pure silence causes some DACs to enter power-save mode (the pop problem).
//
// Initial seed is 0xDEAD_BEEF (non-zero), and a correctly implemented 32-bit
// XOR-shift LFSR with non-degenerate taps (13, 17, 5) has period 2^32 - 1
// and never passes through 0.  This test verifies the invariant.
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn process_audio_1000_calls_while_stopped_never_produces_silent_buffer() {
    let mut manager = PlaybackManager::default();
    assert_eq!(manager.get_state(), PlaybackState::Stopped);

    let silence_threshold = 1e-8_f32; // below DAC keepalive level → effectively silent
    let mut silent_buffers = 0usize;
    const CALLS: usize = 1000;
    const BUF_SIZE: usize = 512;

    for _ in 0..CALLS {
        let mut buffer = vec![0.0f32; BUF_SIZE];
        manager.process_audio(&mut buffer).unwrap();

        let all_silent = buffer.iter().all(|&s| s.abs() < silence_threshold);
        if all_silent {
            silent_buffers += 1;
        }
    }

    assert_eq!(
        silent_buffers, 0,
        "No buffer out of {CALLS} should be fully silent while Stopped. \
         If the LFSR locks to 0 the DAC may enter power-save mode."
    );
}

#[test]
fn process_audio_stopped_output_is_within_dac_keepalive_amplitude() {
    let mut manager = PlaybackManager::default();

    // DAC keepalive noise should be inaudible (~-96 dB ≈ 0.000016 linear)
    // We allow up to -60 dB (0.001) as a generous upper bound
    let max_allowed = 0.001_f32;

    let mut buffer = vec![0.0f32; 1024];
    manager.process_audio(&mut buffer).unwrap();

    for (i, &s) in buffer.iter().enumerate() {
        assert!(
            s.abs() <= max_allowed,
            "sample[{i}]={s}: Stopped output exceeded expected DAC keepalive level"
        );
    }
}

#[test]
fn process_audio_1000_calls_while_stopped_output_is_consistent() {
    // Verify that the noise is always finite and never NaN across all 1000 calls
    let mut manager = PlaybackManager::default();
    const CALLS: usize = 1000;

    for call in 0..CALLS {
        let mut buffer = vec![0.0f32; 64];
        let result = manager.process_audio(&mut buffer);
        assert!(result.is_ok(), "Call {call}: process_audio returned Err");
        for (i, &s) in buffer.iter().enumerate() {
            assert!(s.is_finite(), "Call {call}, sample[{i}] is not finite: {s}");
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Scenario 4: source reads fewer samples than buffer length
//
// The remainder of the output buffer must be filled with DAC keepalive noise,
// not left as whatever was in the buffer before.
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn process_audio_partial_source_read_fills_remainder_with_keepalive_noise() {
    let mut manager = PlaybackManager::default();

    let track = make_track("1", 60);
    manager.load_playlist(vec![track.clone()], 0);
    manager.play().unwrap();

    // Source only returns 64 samples per call; our buffer is 256 samples
    let source = FixedAudioSource::with_max_per_call(60, 64);
    manager.activate_source(Box::new(source), track);

    // Pre-fill buffer with a sentinel value that would be detectable if not overwritten
    let sentinel = 999.0_f32;
    let mut buffer = vec![sentinel; 256];

    let result = manager.process_audio(&mut buffer);
    assert!(result.is_ok());

    // None of the samples should still be the sentinel value
    for (i, &s) in buffer.iter().enumerate() {
        assert!(
            s != sentinel,
            "sample[{i}] was not overwritten (still sentinel {sentinel}). \
             Partial read remainder must be filled."
        );
        assert!(s.is_finite(), "sample[{i}] is not finite: {s}");
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Scenario 5: very large buffer — ensure no out-of-bounds access
// ──────────────────────────────────────────────────────────────────────────────

// ──────────────────────────────────────────────────────────────────────────────
// Mock: source with Duration::MAX (simulates VBR MP3 / files where n_frames
// is absent from codec params → LocalAudioSource sets total_duration = Duration::MAX)
//
// This mock has a FINITE number of samples and returns 0 from read_samples()
// once they are exhausted, but its duration() is Duration::MAX.  The old
// `position >= duration` check can never fire for such a source; auto-advance
// must fall back on source.is_finished() instead.
// ──────────────────────────────────────────────────────────────────────────────

struct EofAudioSource {
    samples_remaining: usize,
    total_samples: usize,
}

impl EofAudioSource {
    /// 10 seconds of stereo silence at 44100 Hz = 882 000 samples
    fn ten_seconds() -> Self {
        let total = (10.0_f64 * 44100.0 * 2.0) as usize; // 882000
        Self {
            samples_remaining: total,
            total_samples: total,
        }
    }
}

impl AudioSource for EofAudioSource {
    fn read_samples(&mut self, buffer: &mut [f32]) -> soul_playback::Result<usize> {
        let to_read = buffer.len().min(self.samples_remaining);
        buffer[..to_read].fill(0.0);
        self.samples_remaining -= to_read;
        Ok(to_read)
    }

    fn seek(&mut self, _: Duration) -> soul_playback::Result<()> {
        Ok(())
    }

    fn duration(&self) -> Duration {
        // Deliberately report an unknown / infinite duration, as real VBR MP3
        // files do when Symphonia cannot determine n_frames from the container.
        Duration::MAX
    }

    fn position(&self) -> Duration {
        let consumed = self.total_samples - self.samples_remaining;
        // Position never reaches Duration::MAX — that is the whole problem.
        Duration::from_secs_f64(consumed as f64 / (44100.0 * 2.0))
    }

    fn is_finished(&self) -> bool {
        self.samples_remaining == 0
    }
}

#[test]
fn process_audio_large_buffer_while_stopped_does_not_panic() {
    let mut manager = PlaybackManager::default();
    // Larger than MAX_STEREO_BUFFER_SIZE (8192 * 2 = 16384)
    let mut buffer = vec![0.0f32; 32768];
    let result = manager.process_audio(&mut buffer);
    assert!(result.is_ok());
    for &s in &buffer {
        assert!(s.is_finite());
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Scenario 6: auto-advance fires when source.is_finished() but duration is MAX
//
// Regression test for VBR MP3 / container formats where Symphonia cannot
// determine n_frames → LocalAudioSource::total_duration = Duration::MAX.
// The position counter never reaches Duration::MAX, so the old condition
//   `if position >= duration { handle_track_finished() }`
// never fires.  Fix: also trigger auto-advance when source.is_finished().
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn auto_advance_fires_when_source_is_finished_but_duration_is_max() {
    // Arrange: 2-track queue; T1 uses an EofAudioSource with duration=MAX.
    let mut mgr = PlaybackManager::default();
    let track1 = make_track("1", 10);
    let track2 = make_track("2", 10);

    mgr.load_playlist(vec![track1.clone(), track2.clone()], 0);
    mgr.play().unwrap();
    mgr.drain_events(); // discard LoadNext(T1) from play()

    let accepted = mgr.activate_source(Box::new(EofAudioSource::ten_seconds()), track1);
    assert!(accepted, "activate_source must accept the first track");
    assert_eq!(mgr.get_state(), PlaybackState::Playing);
    mgr.drain_events(); // discard StateChanged(Playing)

    // Act: pump process_audio() until all samples from T1 are consumed and
    // then for a few extra callbacks so any deferred events are emitted.
    let mut buf = vec![0.0f32; 4096];
    let mut found_load_next = false;
    let max_iters = 1_000; // 1000 * 4096 >> 882_000 samples in the source

    for _ in 0..max_iters {
        mgr.process_audio(&mut buf).unwrap();
        let events = mgr.drain_events();
        if events
            .iter()
            .any(|e| matches!(e, PlaybackEvent::LoadNext(_)))
        {
            found_load_next = true;
            break;
        }
    }

    // Assert: auto-advance must have emitted LoadNext(T2).
    assert!(
        found_load_next,
        "Auto-advance (LoadNext) must fire when source.is_finished() is true, \
         even when source.duration() == Duration::MAX. \
         This regression affects VBR MP3 and other formats where Symphonia \
         cannot determine n_frames from container metadata."
    );
}
