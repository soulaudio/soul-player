use soul_playback::{PlaybackEvent, PlaybackManager, QueueTrack, TrackSource};
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

/// After load_playlist + play, manager is in Loading state (awaiting activate_source).
/// get_current_track() should return the track being loaded (not None).
#[test]
fn loading_state_exposes_pending_track() {
    let mut mgr = PlaybackManager::default();
    mgr.load_playlist(vec![make_track("1")], 0);
    mgr.play().unwrap();

    // After play(), a LoadNext event was emitted — manager is Loading
    let events = mgr.drain_events();
    assert!(events.iter().any(|e| matches!(e, PlaybackEvent::LoadNext(_))));

    // get_current_track() should return the track being loaded (not None)
    let track = mgr.get_current_track();
    assert!(
        track.is_some(),
        "Expected loading track visible via get_current_track"
    );
    assert_eq!(track.unwrap().id, "1");
}
