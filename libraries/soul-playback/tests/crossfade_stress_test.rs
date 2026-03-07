//! Crossfade stress tests for PlaybackManager
//!
//! Verifies crossfade configuration with PlaybackManager under various setups:
//!   - Setting/getting crossfade settings
//!   - Crossfade toggle during playback state transitions
//!   - All curve types with PlaybackManager
//!   - Duration boundary values
//!   - Crossfade state consistency across next/previous/seek
//!   - Rapid crossfade setting changes
//!   - Crossfade + repeat/shuffle mode interactions

use soul_playback::{
    AudioSource, CrossfadeSettings, CrossfadeState, FadeCurve, PlaybackManager, PlaybackState,
    QueueTrack, TrackSource,
};
use std::path::PathBuf;
use std::time::Duration;

// ===== Test Helpers =====

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
            samples_per_second: sample_rate as u64 * 2,
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
        let remaining = (total_samples.saturating_sub(current_sample)) as usize;
        let to_read = remaining.min(buffer.len());
        if to_read == 0 {
            self.finished = true;
            return Ok(0);
        }
        for (i, sample) in buffer.iter_mut().enumerate().take(to_read) {
            *sample = ((i % 2) as f32 - 0.5) * 0.5;
        }
        let dur = Duration::from_secs_f64(to_read as f64 / self.samples_per_second as f64);
        self.position += dur;
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

fn track(id: &str, title: &str, secs: u64) -> QueueTrack {
    QueueTrack {
        id: id.to_string(),
        path: PathBuf::from(format!("/music/{}.mp3", id)),
        title: title.to_string(),
        artist: "Test Artist".to_string(),
        album: Some("Test Album".to_string()),
        duration: Duration::from_secs(secs),
        track_number: Some(1),
        source: TrackSource::Single,
    }
}

fn source(secs: u64) -> Box<MockAudioSource> {
    Box::new(MockAudioSource::new(Duration::from_secs(secs), 44100))
}

fn setup_playing_manager() -> PlaybackManager {
    let mut m = PlaybackManager::default();
    let t1 = track("1", "Track One", 180);
    let t2 = track("2", "Track Two", 180);
    let t3 = track("3", "Track Three", 180);
    m.add_to_queue_end(t1.clone());
    m.add_to_queue_end(t2);
    m.add_to_queue_end(t3);
    m.play().ok();
    m.activate_source(source(180), t1);
    m.drain_events();
    m
}

// ===== Tests =====

#[test]
fn crossfade_defaults_are_disabled_equal_power_3000ms() {
    let m = PlaybackManager::default();
    let s = m.get_crossfade_settings();
    assert!(!s.enabled);
    assert_eq!(s.duration_ms, 3000);
    assert_eq!(s.curve, FadeCurve::EqualPower);
    assert!(!s.on_skip);
}

#[test]
fn set_crossfade_settings_round_trips() {
    let mut m = PlaybackManager::default();
    let settings = CrossfadeSettings {
        enabled: true,
        duration_ms: 5000,
        curve: FadeCurve::SCurve,
        on_skip: true,
    };
    m.set_crossfade_settings(settings);
    let s = m.get_crossfade_settings();
    assert!(s.enabled);
    assert_eq!(s.duration_ms, 5000);
    assert_eq!(s.curve, FadeCurve::SCurve);
    assert!(s.on_skip);
}

#[test]
fn all_curves_can_be_set_and_read_back() {
    let mut m = PlaybackManager::default();
    let curves = [
        FadeCurve::Linear,
        FadeCurve::SquareRoot,
        FadeCurve::SCurve,
        FadeCurve::EqualPower,
        FadeCurve::Exponential,
    ];
    for curve in curves {
        m.set_crossfade_curve(curve);
        assert_eq!(
            m.get_crossfade_curve(),
            curve,
            "Curve mismatch for {:?}",
            curve
        );
    }
}

#[test]
fn crossfade_duration_boundary_values() {
    let mut m = PlaybackManager::default();

    // 0ms (gapless)
    m.set_crossfade_duration(0);
    assert_eq!(m.get_crossfade_duration(), 0);

    // Normal values
    m.set_crossfade_duration(1000);
    assert_eq!(m.get_crossfade_duration(), 1000);

    // Maximum
    m.set_crossfade_duration(10000);
    assert_eq!(m.get_crossfade_duration(), 10000);
}

#[test]
fn crossfade_toggle_during_playback_does_not_change_state() {
    let mut m = setup_playing_manager();
    assert_eq!(m.get_state(), PlaybackState::Playing);

    // Enable crossfade
    m.set_crossfade_enabled(true);
    assert_eq!(m.get_state(), PlaybackState::Playing);

    // Disable crossfade
    m.set_crossfade_enabled(false);
    assert_eq!(m.get_state(), PlaybackState::Playing);
}

#[test]
fn crossfade_settings_change_during_playback_preserves_state() {
    let mut m = setup_playing_manager();

    let configs = [
        CrossfadeSettings {
            enabled: true,
            duration_ms: 1000,
            curve: FadeCurve::Linear,
            on_skip: false,
        },
        CrossfadeSettings {
            enabled: true,
            duration_ms: 5000,
            curve: FadeCurve::EqualPower,
            on_skip: true,
        },
        CrossfadeSettings {
            enabled: false,
            duration_ms: 3000,
            curve: FadeCurve::SCurve,
            on_skip: false,
        },
        CrossfadeSettings {
            enabled: true,
            duration_ms: 10000,
            curve: FadeCurve::Exponential,
            on_skip: true,
        },
    ];

    for cfg in configs {
        m.set_crossfade_settings(cfg);
        assert_eq!(
            m.get_state(),
            PlaybackState::Playing,
            "State changed after crossfade settings update"
        );
    }
}

#[test]
fn rapid_crossfade_toggle_50_times() {
    let mut m = setup_playing_manager();

    for i in 0..50 {
        m.set_crossfade_enabled(i % 2 == 0);
    }

    // Final state: i=49, 49%2=1, so enabled=false
    assert!(!m.get_crossfade_settings().enabled);
    assert_eq!(m.get_state(), PlaybackState::Playing);
}

#[test]
fn rapid_curve_cycling_100_times() {
    let mut m = setup_playing_manager();
    m.set_crossfade_enabled(true);

    let curves = [
        FadeCurve::Linear,
        FadeCurve::SquareRoot,
        FadeCurve::SCurve,
        FadeCurve::EqualPower,
        FadeCurve::Exponential,
    ];

    for i in 0..100 {
        m.set_crossfade_curve(curves[i % curves.len()]);
    }

    // After 100 iterations, last set was curves[99 % 5] = curves[4] = Exponential
    assert_eq!(m.get_crossfade_curve(), FadeCurve::Exponential);
    assert_eq!(m.get_state(), PlaybackState::Playing);
}

#[test]
fn crossfade_settings_persist_across_next_track() {
    let mut m = setup_playing_manager();

    m.set_crossfade_settings(CrossfadeSettings {
        enabled: true,
        duration_ms: 4500,
        curve: FadeCurve::Exponential,
        on_skip: true,
    });

    // Skip to next track
    let _ = m.next();
    let events = m.drain_events();
    // Process LoadNext if present
    let has_load_next = events
        .iter()
        .any(|e| matches!(e, soul_playback::PlaybackEvent::LoadNext(_)));
    assert!(has_load_next, "next() should emit LoadNext");

    // Crossfade settings should persist
    let s = m.get_crossfade_settings();
    assert!(s.enabled);
    assert_eq!(s.duration_ms, 4500);
    assert_eq!(s.curve, FadeCurve::Exponential);
    assert!(s.on_skip);
}

#[test]
fn crossfade_settings_persist_across_previous_track() {
    let mut m = setup_playing_manager();

    // Skip forward first so we have history
    let _ = m.next();
    let events = m.drain_events();
    // Activate source for track two
    for event in &events {
        if let soul_playback::PlaybackEvent::LoadNext(t) = event {
            m.activate_source(source(180), t.clone());
            break;
        }
    }
    m.drain_events();

    // Set crossfade
    m.set_crossfade_settings(CrossfadeSettings {
        enabled: true,
        duration_ms: 2000,
        curve: FadeCurve::SCurve,
        on_skip: false,
    });

    // Go back
    let _ = m.previous();
    m.drain_events();

    // Settings persist
    let s = m.get_crossfade_settings();
    assert!(s.enabled);
    assert_eq!(s.duration_ms, 2000);
    assert_eq!(s.curve, FadeCurve::SCurve);
}

#[test]
fn crossfade_initial_state_is_inactive() {
    let m = PlaybackManager::default();
    assert_eq!(m.get_crossfade_state(), CrossfadeState::Inactive);
}

#[test]
fn crossfade_state_inactive_when_disabled() {
    let mut m = setup_playing_manager();
    m.set_crossfade_enabled(false);
    assert_eq!(m.get_crossfade_state(), CrossfadeState::Inactive);
}

#[test]
fn crossfade_on_skip_can_be_toggled() {
    let mut m = PlaybackManager::default();

    m.set_crossfade_on_skip(true);
    assert!(m.get_crossfade_settings().on_skip);

    m.set_crossfade_on_skip(false);
    assert!(!m.get_crossfade_settings().on_skip);
}

#[test]
fn crossfade_settings_with_different_configs_during_stopped_state() {
    let mut m = PlaybackManager::default();
    assert_eq!(m.get_state(), PlaybackState::Stopped);

    // Should accept settings even when stopped
    m.set_crossfade_settings(CrossfadeSettings {
        enabled: true,
        duration_ms: 7000,
        curve: FadeCurve::SquareRoot,
        on_skip: true,
    });

    let s = m.get_crossfade_settings();
    assert!(s.enabled);
    assert_eq!(s.duration_ms, 7000);
    assert_eq!(s.curve, FadeCurve::SquareRoot);
    assert!(s.on_skip);
}

#[test]
fn crossfade_progress_is_one_when_inactive() {
    // When no crossfade is active, progress reports 1.0 (fully complete / no fade)
    let m = PlaybackManager::default();
    assert_eq!(m.get_crossfade_progress(), 1.0);
}

#[test]
fn crossfade_all_curves_during_next_operations() {
    let curves = [
        FadeCurve::Linear,
        FadeCurve::SquareRoot,
        FadeCurve::SCurve,
        FadeCurve::EqualPower,
        FadeCurve::Exponential,
    ];

    for curve in curves {
        let mut m = PlaybackManager::default();
        let t1 = track("1", "Track One", 180);
        let t2 = track("2", "Track Two", 180);
        m.add_to_queue_end(t1.clone());
        m.add_to_queue_end(t2);
        m.play().ok();
        m.activate_source(source(180), t1);
        m.drain_events();

        m.set_crossfade_settings(CrossfadeSettings {
            enabled: true,
            duration_ms: 2000,
            curve,
            on_skip: true,
        });

        let _ = m.next();
        let events = m.drain_events();

        // Must emit LoadNext for the next track regardless of curve
        let has_load_next = events
            .iter()
            .any(|e| matches!(e, soul_playback::PlaybackEvent::LoadNext(_)));
        assert!(
            has_load_next,
            "next() must emit LoadNext with curve {:?}",
            curve
        );
    }
}

#[test]
fn crossfade_settings_validation_rejects_excessive_duration() {
    let settings = CrossfadeSettings {
        enabled: true,
        duration_ms: 15000, // exceeds 10000 max
        curve: FadeCurve::EqualPower,
        on_skip: false,
    };

    let result = settings.validate();
    assert!(
        result.is_err(),
        "Validation should reject duration > 10000ms"
    );
}

#[test]
fn crossfade_settings_validation_accepts_valid_config() {
    let settings = CrossfadeSettings {
        enabled: true,
        duration_ms: 5000,
        curve: FadeCurve::Linear,
        on_skip: true,
    };

    let result = settings.validate();
    assert!(result.is_ok(), "Validation should accept valid config");
}

#[test]
fn crossfade_gapless_constructor_creates_zero_duration() {
    let s = CrossfadeSettings::gapless();
    assert!(s.enabled);
    assert_eq!(s.duration_ms, 0);
}

#[test]
fn crossfade_with_duration_clamps_to_max() {
    let s = CrossfadeSettings::with_duration(20000);
    assert_eq!(s.duration_ms, 10000); // clamped to MAX_CROSSFADE_DURATION_MS
}

#[test]
fn crossfade_with_duration_and_curve() {
    let s = CrossfadeSettings::with_duration_and_curve(4000, FadeCurve::Exponential);
    assert!(s.enabled);
    assert_eq!(s.duration_ms, 4000);
    assert_eq!(s.curve, FadeCurve::Exponential);
}
