use soul_playback::{PlaybackEvent, PlaybackManager, PlaybackStateEvent, QueueTrack, TrackSource};
use std::path::PathBuf;
use std::time::Duration;

fn make_track(id: &str) -> QueueTrack {
    QueueTrack {
        id: id.to_string(),
        path: PathBuf::from(format!("/tmp/track{id}.mp3")),
        title: format!("Track {id}"),
        artist: "Artist".to_string(),
        album: Some("Album".to_string()),
        duration: Duration::from_secs(30),
        track_number: Some(1),
        source: TrackSource::Single,
    }
}

/// When the last track in the queue finishes auto-advancing with Repeat::Off,
/// a StateChanged(Stopped) event MUST be emitted so the UI stops the timer.
#[test]
fn queue_exhausted_emits_state_changed_stopped() {
    let mut mgr = PlaybackManager::default();
    mgr.load_playlist(vec![make_track("1")], 0);
    let _ = mgr.play();
    mgr.drain_events(); // consume LoadNext, StateChanged(Playing)

    // Simulate track finishing → next() is called → queue empty
    let _ = mgr.next();
    let events = mgr.drain_events();

    let has_stopped = events.iter().any(|e| {
        matches!(
            e,
            PlaybackEvent::StateChanged {
                state: PlaybackStateEvent::Stopped
            }
        )
    });
    assert!(
        has_stopped,
        "Expected StateChanged(Stopped) when queue exhausts, got: {events:?}"
    );
}
