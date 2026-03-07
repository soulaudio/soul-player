//! TDD tests: shuffle mode + auto-advance
//!
//! Verifies that when shuffle is enabled and a track finishes,
//! auto-advance fires LoadNext with the next shuffled track.
//!
//! Bug hypothesis: in shuffle mode, auto-advance (process_audio → handle_track_finished
//! → next → play_next_in_queue → get_next_track_from_queue → queue.pop_next) may fail
//! to produce a next track or may not trigger LoadNext correctly.

use soul_playback::{
    AudioSource, PlaybackEvent, PlaybackManager, PlaybackState, QueueTrack, ShuffleMode,
    TrackSource,
};
use std::path::PathBuf;
use std::time::Duration;

// ── Helpers ──────────────────────────────────────────────────────────────────

fn make_track(id: &str, title: &str, secs: u64) -> QueueTrack {
    QueueTrack {
        id: id.to_string(),
        path: PathBuf::from(format!("/music/{id}.wav")),
        title: title.to_string(),
        artist: "Test Artist".to_string(),
        album: Some("Test Album".to_string()),
        duration: Duration::from_secs(secs),
        track_number: None,
        source: TrackSource::Single,
    }
}

fn make_tracks(n: usize, duration_secs: u64) -> Vec<QueueTrack> {
    (1..=n)
        .map(|i| make_track(&format!("{i}"), &format!("Track {i}"), duration_secs))
        .collect()
}

/// Audio source that finishes after `duration` worth of samples.
struct ShortAudioSource {
    duration: Duration,
    position: Duration,
}

impl ShortAudioSource {
    fn new(duration: Duration) -> Self {
        Self {
            duration,
            position: Duration::ZERO,
        }
    }
}

impl AudioSource for ShortAudioSource {
    fn read_samples(&mut self, buffer: &mut [f32]) -> soul_playback::Result<usize> {
        let sample_rate = 88200.0_f64; // 44100 Hz stereo
        let total = (self.duration.as_secs_f64() * sample_rate) as usize;
        let current = (self.position.as_secs_f64() * sample_rate) as usize;
        let remaining = total.saturating_sub(current);

        let to_read = remaining.min(buffer.len());
        if to_read == 0 {
            return Ok(0);
        }

        for s in buffer.iter_mut().take(to_read) {
            *s = 0.1;
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

/// Drain events and return them.
fn drain(mgr: &mut PlaybackManager) -> Vec<PlaybackEvent> {
    mgr.drain_events()
}

/// Process audio until we get a LoadNext event or exhaust attempts.
/// Returns the LoadNext track if found.
fn process_until_load_next(mgr: &mut PlaybackManager, max_iterations: usize) -> Option<QueueTrack> {
    let mut buffer = vec![0.0f32; 4096];
    for _ in 0..max_iterations {
        let _ = mgr.process_audio(&mut buffer);
        for event in drain(mgr) {
            if let PlaybackEvent::LoadNext(track) = event {
                return Some(track);
            }
        }
    }
    None
}

// ── Test 1: Shuffle + auto-advance fires LoadNext ────────────────────────────

#[test]
fn shuffle_auto_advance_fires_load_next() {
    let mut mgr = PlaybackManager::default();

    let tracks = make_tracks(5, 1); // 5 tracks, 1 second each
    mgr.load_playlist(tracks.clone(), 0);
    mgr.set_shuffle(ShuffleMode::Random);
    mgr.play().unwrap();

    // Drain the initial LoadNext from play()
    let initial_events = drain(&mut mgr);
    let initial_load = initial_events
        .iter()
        .find(|e| matches!(e, PlaybackEvent::LoadNext(_)));
    assert!(
        initial_load.is_some(),
        "play() should emit LoadNext for first track"
    );

    // Get the first track and activate it
    let first_track = match initial_load.unwrap() {
        PlaybackEvent::LoadNext(t) => t.clone(),
        _ => unreachable!(),
    };

    // Activate with a very short source (0.01s) so it finishes quickly
    mgr.activate_source(
        Box::new(ShortAudioSource::new(Duration::from_millis(10))),
        first_track.clone(),
    );
    drain(&mut mgr); // clear activate events

    assert_eq!(mgr.get_state(), PlaybackState::Playing);

    // Process audio until the short source finishes and triggers auto-advance
    let next_track = process_until_load_next(&mut mgr, 1000);

    assert!(
        next_track.is_some(),
        "Auto-advance must fire LoadNext when track finishes in shuffle mode"
    );

    let next = next_track.unwrap();
    // The next track should be one of our 5 tracks (but not necessarily track 2)
    let valid_ids: Vec<&str> = vec!["1", "2", "3", "4", "5"];
    assert!(
        valid_ids.contains(&next.id.as_str()),
        "LoadNext track id '{}' should be one of the original tracks",
        next.id
    );
}

// ── Test 2: Full auto-advance chain through all tracks in shuffle mode ───────

#[test]
fn shuffle_auto_advance_chains_through_all_tracks() {
    let mut mgr = PlaybackManager::default();

    let tracks = make_tracks(5, 1);
    mgr.load_playlist(tracks.clone(), 0);
    mgr.set_shuffle(ShuffleMode::Random);
    mgr.play().unwrap();

    let mut played_ids: Vec<String> = Vec::new();

    // Play through all 5 tracks via auto-advance
    for i in 0..5 {
        // Get LoadNext event
        let load_next_track = if i == 0 {
            // First track comes from play()
            let events = drain(&mut mgr);
            events
                .into_iter()
                .find_map(|e| {
                    if let PlaybackEvent::LoadNext(t) = e {
                        Some(t)
                    } else {
                        None
                    }
                })
                .expect("play() should emit LoadNext")
        } else {
            // Subsequent tracks come from auto-advance
            process_until_load_next(&mut mgr, 2000)
                .unwrap_or_else(|| panic!("Auto-advance should fire LoadNext for track {}", i + 1))
        };

        played_ids.push(load_next_track.id.clone());

        // Activate with short source
        mgr.activate_source(
            Box::new(ShortAudioSource::new(Duration::from_millis(10))),
            load_next_track,
        );
        drain(&mut mgr); // clear activate events
    }

    assert_eq!(
        played_ids.len(),
        5,
        "Should have played all 5 tracks via auto-advance in shuffle mode"
    );

    // All 5 original IDs should be present (each played exactly once)
    let mut sorted = played_ids.clone();
    sorted.sort();
    assert_eq!(
        sorted,
        vec!["1", "2", "3", "4", "5"],
        "All 5 tracks should have played exactly once in shuffle mode. Got: {:?}",
        played_ids
    );

    // Order should differ from sequential (with high probability for 5 tracks)
    // This is probabilistic — there's a 1/120 chance shuffle produces 1,2,3,4,5
    // We accept this flake risk rather than over-complicating the test
    let sequential: Vec<String> = (1..=5).map(|i| i.to_string()).collect();
    if played_ids == sequential {
        eprintln!(
            "WARNING: Shuffle produced sequential order (1/120 chance). \
             This is not a failure, but if it happens repeatedly, shuffle may be broken."
        );
    }
}

// ── Test 3: Shuffle + auto-advance returns error after last track ─────────────

#[test]
fn shuffle_auto_advance_stops_after_last_track() {
    let mut mgr = PlaybackManager::default();

    let tracks = make_tracks(3, 1);
    mgr.load_playlist(tracks, 0);
    mgr.set_shuffle(ShuffleMode::Random);
    mgr.play().unwrap();

    // Play through all 3 tracks
    for i in 0..3 {
        let load_next_track = if i == 0 {
            drain(&mut mgr)
                .into_iter()
                .find_map(|e| {
                    if let PlaybackEvent::LoadNext(t) = e {
                        Some(t)
                    } else {
                        None
                    }
                })
                .expect("Should get LoadNext")
        } else {
            process_until_load_next(&mut mgr, 2000)
                .unwrap_or_else(|| panic!("Auto-advance should fire LoadNext for track {}", i + 1))
        };

        mgr.activate_source(
            Box::new(ShortAudioSource::new(Duration::from_millis(10))),
            load_next_track,
        );
        drain(&mut mgr);
    }

    // Now process audio — the 3rd track should finish and queue is exhausted.
    // process_audio should return Err(QueueEmpty) or state should become Stopped.
    let mut buffer = vec![0.0f32; 4096];
    let mut reached_end = false;
    for _ in 0..2000 {
        match mgr.process_audio(&mut buffer) {
            Err(_) => {
                // QueueEmpty error — expected
                reached_end = true;
                break;
            }
            Ok(_) => {
                // Check if state transitioned to Stopped (no more tracks)
                if mgr.get_state() == PlaybackState::Stopped {
                    reached_end = true;
                    break;
                }
                // Make sure no unexpected LoadNext
                for event in drain(&mut mgr) {
                    if let PlaybackEvent::LoadNext(_) = event {
                        panic!("Should NOT get LoadNext after all tracks played (no repeat)");
                    }
                }
            }
        }
    }

    assert!(
        reached_end,
        "Should stop or error after all shuffled tracks have been played"
    );
}

// ── Test 4: Shuffle mode set DURING playback still auto-advances ─────────────

#[test]
fn shuffle_enabled_mid_playback_still_auto_advances() {
    let mut mgr = PlaybackManager::default();

    let tracks = make_tracks(5, 1);
    mgr.load_playlist(tracks, 0);
    // Start WITHOUT shuffle
    mgr.play().unwrap();

    let first = drain(&mut mgr)
        .into_iter()
        .find_map(|e| {
            if let PlaybackEvent::LoadNext(t) = e {
                Some(t)
            } else {
                None
            }
        })
        .expect("Should get first LoadNext");

    mgr.activate_source(
        Box::new(ShortAudioSource::new(Duration::from_millis(10))),
        first,
    );
    drain(&mut mgr);

    // Enable shuffle mid-playback
    mgr.set_shuffle(ShuffleMode::Random);

    // Let the first track finish — auto-advance should still work
    let next = process_until_load_next(&mut mgr, 2000);

    assert!(
        next.is_some(),
        "Auto-advance must work after enabling shuffle mid-playback"
    );
}

// ── Test 5: Shuffle + RepeatAll auto-advance wraps around ────────────────────

#[test]
fn shuffle_repeat_all_auto_advance_wraps_around() {
    let mut mgr = PlaybackManager::default();

    let tracks = make_tracks(3, 1);
    mgr.load_playlist(tracks, 0);
    mgr.set_shuffle(ShuffleMode::Random);
    mgr.set_repeat(soul_playback::RepeatMode::All);
    mgr.play().unwrap();

    let mut played_count = 0;

    // Play through 6 tracks (3 original + 3 wrapped)
    for i in 0..6 {
        let track = if i == 0 {
            drain(&mut mgr)
                .into_iter()
                .find_map(|e| {
                    if let PlaybackEvent::LoadNext(t) = e {
                        Some(t)
                    } else {
                        None
                    }
                })
                .expect("Should get first LoadNext")
        } else {
            process_until_load_next(&mut mgr, 2000).unwrap_or_else(|| {
                panic!(
                    "Auto-advance should fire LoadNext for track {} (repeat all + shuffle)",
                    i + 1
                )
            })
        };

        played_count += 1;

        mgr.activate_source(
            Box::new(ShortAudioSource::new(Duration::from_millis(10))),
            track,
        );
        drain(&mut mgr);
    }

    assert_eq!(
        played_count, 6,
        "Should have played 6 tracks (3 + 3 repeat) in shuffle + RepeatAll mode"
    );
}
