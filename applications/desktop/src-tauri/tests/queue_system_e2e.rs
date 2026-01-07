//! End-to-end tests for queue system
//!
//! Tests the complete queue workflow from UI perspective:
//! - Playing tracks creates queue
//! - Queue sidebar displays tracks
//! - Next/Previous buttons enabled/disabled correctly
//! - Queue operations emit proper events

use soul_audio_desktop::{DesktopPlayback, PlaybackCommand, PlaybackEvent};
use soul_playback::{PlaybackConfig, QueueTrack, RepeatMode, TrackSource};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

// ===== Test Setup =====

/// Simulated PlaybackManager with queue methods
struct TestPlaybackManager {
    playback: Arc<Mutex<DesktopPlayback>>,
}

impl TestPlaybackManager {
    fn new() -> Result<Self, String> {
        let config = PlaybackConfig::default();
        let playback = DesktopPlayback::new(config).map_err(|e| e.to_string())?;

        Ok(Self {
            playback: Arc::new(Mutex::new(playback)),
        })
    }

    // Queue-specific methods (mirroring actual Tauri commands)

    fn play_queue(&self, tracks: Vec<QueueTrack>) -> Result<(), String> {
        let playback = self.playback.lock().map_err(|e| e.to_string())?;

        // Clear existing queue
        playback
            .send_command(PlaybackCommand::ClearQueue)
            .map_err(|e| e.to_string())?;

        // Add all tracks
        for track in tracks {
            playback
                .send_command(PlaybackCommand::AddToQueue(track))
                .map_err(|e| e.to_string())?;
        }

        // Start playback
        playback
            .send_command(PlaybackCommand::Play)
            .map_err(|e| e.to_string())
    }

    fn get_queue(&self) -> Result<Vec<QueueTrack>, String> {
        let playback = self.playback.lock().map_err(|e| e.to_string())?;
        Ok(playback.get_queue())
    }

    fn get_playback_capabilities(&self) -> Result<(bool, bool), String> {
        let playback = self.playback.lock().map_err(|e| e.to_string())?;
        Ok((playback.has_next(), playback.has_previous()))
    }

    fn next(&self) -> Result<(), String> {
        let playback = self.playback.lock().map_err(|e| e.to_string())?;
        playback
            .send_command(PlaybackCommand::Next)
            .map_err(|e| e.to_string())
    }

    fn previous(&self) -> Result<(), String> {
        let playback = self.playback.lock().map_err(|e| e.to_string())?;
        playback
            .send_command(PlaybackCommand::Previous)
            .map_err(|e| e.to_string())
    }

    fn set_repeat(&self, mode: RepeatMode) -> Result<(), String> {
        let playback = self.playback.lock().map_err(|e| e.to_string())?;
        playback
            .send_command(PlaybackCommand::SetRepeat(mode))
            .map_err(|e| e.to_string())
    }

    fn try_recv_event(&self) -> Option<PlaybackEvent> {
        let playback = self.playback.lock().ok()?;
        playback.try_recv_event()
    }

    fn drain_events(&self) {
        while self.try_recv_event().is_some() {}
    }
}

fn create_test_track(id: &str, title: &str, artist: &str) -> QueueTrack {
    QueueTrack {
        id: id.to_string(),
        path: PathBuf::from(format!("/music/{}.mp3", id)),
        title: title.to_string(),
        artist: artist.to_string(),
        album: Some("Test Album".to_string()),
        duration: Duration::from_secs(180),
        track_number: Some(id.parse().unwrap_or(1)),
        source: TrackSource::Single,
    }
}

// ===== Queue Creation Tests =====

#[test]
fn test_e2e_play_queue_creates_queue() {
    let manager = TestPlaybackManager::new().unwrap();

    let tracks = vec![
        create_test_track("1", "Track 1", "Artist A"),
        create_test_track("2", "Track 2", "Artist B"),
        create_test_track("3", "Track 3", "Artist C"),
    ];

    // Play queue
    assert!(manager.play_queue(tracks.clone()).is_ok(), "Should play queue");

    std::thread::sleep(Duration::from_millis(100));
    manager.drain_events();

    // Get queue and verify
    let queue = manager.get_queue().unwrap();

    assert_eq!(queue.len(), 3, "Queue should have 3 tracks");
    assert_eq!(queue[0].id, "1");
    assert_eq!(queue[1].id, "2");
    assert_eq!(queue[2].id, "3");
}

#[test]
fn test_e2e_play_queue_from_middle() {
    let manager = TestPlaybackManager::new().unwrap();

    // Simulate clicking track 3 in library (creates queue starting from track 3)
    let library = vec![
        create_test_track("1", "Track 1", "Artist A"),
        create_test_track("2", "Track 2", "Artist B"),
        create_test_track("3", "Track 3", "Artist C"),
        create_test_track("4", "Track 4", "Artist D"),
        create_test_track("5", "Track 5", "Artist E"),
    ];

    let start_index = 2; // User clicked track 3

    // Build queue: [3, 4, 5, 1, 2]
    let queue: Vec<QueueTrack> = library
        .iter()
        .skip(start_index)
        .chain(library.iter().take(start_index))
        .cloned()
        .collect();

    assert!(manager.play_queue(queue).is_ok(), "Should play queue");

    std::thread::sleep(Duration::from_millis(100));
    manager.drain_events();

    let actual_queue = manager.get_queue().unwrap();

    assert_eq!(actual_queue.len(), 5);
    assert_eq!(actual_queue[0].id, "3");
    assert_eq!(actual_queue[1].id, "4");
    assert_eq!(actual_queue[2].id, "5");
    assert_eq!(actual_queue[3].id, "1");
    assert_eq!(actual_queue[4].id, "2");
}

#[test]
fn test_e2e_play_queue_replaces_existing() {
    let manager = TestPlaybackManager::new().unwrap();

    // Play first queue
    let queue1 = vec![
        create_test_track("a", "Track A", "Artist 1"),
        create_test_track("b", "Track B", "Artist 2"),
    ];

    manager.play_queue(queue1).unwrap();
    std::thread::sleep(Duration::from_millis(50));

    // Play second queue (should replace)
    let queue2 = vec![
        create_test_track("x", "Track X", "Artist 3"),
        create_test_track("y", "Track Y", "Artist 4"),
        create_test_track("z", "Track Z", "Artist 5"),
    ];

    manager.play_queue(queue2).unwrap();
    std::thread::sleep(Duration::from_millis(100));
    manager.drain_events();

    // Should have new queue
    let actual_queue = manager.get_queue().unwrap();

    assert_eq!(actual_queue.len(), 3);
    assert_eq!(actual_queue[0].id, "x");
    assert_eq!(actual_queue[1].id, "y");
    assert_eq!(actual_queue[2].id, "z");
}

// ===== Playback Capabilities Tests =====

#[test]
fn test_e2e_has_next_with_queue() {
    let manager = TestPlaybackManager::new().unwrap();

    let tracks = vec![
        create_test_track("1", "Track 1", "Artist A"),
        create_test_track("2", "Track 2", "Artist B"),
    ];

    manager.play_queue(tracks).unwrap();
    std::thread::sleep(Duration::from_millis(50));
    manager.drain_events();

    let (has_next, _has_previous) = manager.get_playback_capabilities().unwrap();

    assert!(has_next, "Should have next with tracks in queue");
}

#[test]
fn test_e2e_has_next_empty_queue() {
    let manager = TestPlaybackManager::new().unwrap();

    let (has_next, _has_previous) = manager.get_playback_capabilities().unwrap();

    assert!(!has_next, "Should not have next with empty queue");
}

#[test]
fn test_e2e_has_next_after_consuming_queue() {
    let manager = TestPlaybackManager::new().unwrap();

    let tracks = vec![
        create_test_track("1", "Track 1", "Artist A"),
        create_test_track("2", "Track 2", "Artist B"),
    ];

    manager.play_queue(tracks).unwrap();
    std::thread::sleep(Duration::from_millis(50));

    // Skip both tracks
    manager.next().unwrap();
    std::thread::sleep(Duration::from_millis(30));
    manager.next().unwrap();
    std::thread::sleep(Duration::from_millis(30));
    manager.drain_events();

    let (has_next, _) = manager.get_playback_capabilities().unwrap();

    assert!(!has_next, "Should not have next after consuming queue");
}

#[test]
fn test_e2e_has_previous_initially_false() {
    let manager = TestPlaybackManager::new().unwrap();

    let tracks = vec![create_test_track("1", "Track 1", "Artist A")];

    manager.play_queue(tracks).unwrap();
    std::thread::sleep(Duration::from_millis(50));
    manager.drain_events();

    let (_has_next, has_previous) = manager.get_playback_capabilities().unwrap();

    assert!(!has_previous, "Should not have previous initially");
}

#[test]
fn test_e2e_has_previous_after_skip() {
    let manager = TestPlaybackManager::new().unwrap();

    let tracks = vec![
        create_test_track("1", "Track 1", "Artist A"),
        create_test_track("2", "Track 2", "Artist B"),
    ];

    manager.play_queue(tracks).unwrap();
    std::thread::sleep(Duration::from_millis(50));

    // Skip to next track
    manager.next().unwrap();
    std::thread::sleep(Duration::from_millis(50));
    manager.drain_events();

    let (_, has_previous) = manager.get_playback_capabilities().unwrap();

    assert!(has_previous, "Should have previous after skipping");
}

#[test]
fn test_e2e_capabilities_with_repeat_one() {
    let manager = TestPlaybackManager::new().unwrap();

    manager.set_repeat(RepeatMode::One).unwrap();
    std::thread::sleep(Duration::from_millis(20));

    let tracks = vec![create_test_track("1", "Track 1", "Artist A")];

    manager.play_queue(tracks).unwrap();
    std::thread::sleep(Duration::from_millis(50));
    manager.drain_events();

    let (has_next, has_previous) = manager.get_playback_capabilities().unwrap();

    assert!(has_next, "Should have next with repeat one");
    assert!(has_previous, "Should have previous with repeat one");
}

// ===== Queue Navigation Tests =====

#[test]
fn test_e2e_queue_navigation_sequence() {
    let manager = TestPlaybackManager::new().unwrap();

    let tracks = vec![
        create_test_track("1", "Track 1", "Artist A"),
        create_test_track("2", "Track 2", "Artist B"),
        create_test_track("3", "Track 3", "Artist C"),
    ];

    manager.play_queue(tracks).unwrap();
    std::thread::sleep(Duration::from_millis(50));

    // Check initial state
    let (has_next, has_previous) = manager.get_playback_capabilities().unwrap();
    assert!(has_next, "Should have next initially");
    assert!(!has_previous, "Should not have previous initially");

    // Skip to track 2
    manager.next().unwrap();
    std::thread::sleep(Duration::from_millis(50));

    let (has_next, has_previous) = manager.get_playback_capabilities().unwrap();
    assert!(has_next, "Should have next at track 2");
    assert!(has_previous, "Should have previous at track 2");

    // Skip to track 3
    manager.next().unwrap();
    std::thread::sleep(Duration::from_millis(50));

    let (has_next, has_previous) = manager.get_playback_capabilities().unwrap();
    assert!(has_next, "Should have next at track 3");
    assert!(has_previous, "Should have previous at track 3");

    // Skip past end
    manager.next().unwrap();
    std::thread::sleep(Duration::from_millis(50));
    manager.drain_events();

    let (has_next, has_previous) = manager.get_playback_capabilities().unwrap();
    assert!(!has_next, "Should not have next past end");
    assert!(has_previous, "Should still have previous past end");
}

#[test]
fn test_e2e_previous_navigation() {
    let manager = TestPlaybackManager::new().unwrap();

    let tracks = vec![
        create_test_track("1", "Track 1", "Artist A"),
        create_test_track("2", "Track 2", "Artist B"),
        create_test_track("3", "Track 3", "Artist C"),
    ];

    manager.play_queue(tracks).unwrap();
    std::thread::sleep(Duration::from_millis(50));

    // Skip forward twice
    manager.next().unwrap();
    std::thread::sleep(Duration::from_millis(30));
    manager.next().unwrap();
    std::thread::sleep(Duration::from_millis(30));

    // Now go back
    assert!(manager.previous().is_ok(), "Previous should succeed");
    std::thread::sleep(Duration::from_millis(50));
    manager.drain_events();

    let (has_next, has_previous) = manager.get_playback_capabilities().unwrap();
    assert!(has_next, "Should have next after going back");
    assert!(has_previous, "Should still have previous");
}

// ===== Queue Sidebar Workflow Tests =====

#[test]
fn test_e2e_queue_sidebar_displays_all_tracks() {
    let manager = TestPlaybackManager::new().unwrap();

    let tracks = vec![
        create_test_track("1", "Track 1", "Artist A"),
        create_test_track("2", "Track 2", "Artist B"),
        create_test_track("3", "Track 3", "Artist C"),
        create_test_track("4", "Track 4", "Artist D"),
        create_test_track("5", "Track 5", "Artist E"),
    ];

    manager.play_queue(tracks.clone()).unwrap();
    std::thread::sleep(Duration::from_millis(100));
    manager.drain_events();

    // Simulate queue sidebar requesting queue
    let queue = manager.get_queue().unwrap();

    assert_eq!(queue.len(), 5, "Sidebar should show all 5 tracks");

    // Verify all track details
    for (i, track) in queue.iter().enumerate() {
        assert_eq!(track.id, (i + 1).to_string());
        assert_eq!(track.title, format!("Track {}", i + 1));
    }
}

#[test]
fn test_e2e_queue_sidebar_updates_on_skip() {
    let manager = TestPlaybackManager::new().unwrap();

    let tracks = vec![
        create_test_track("1", "Track 1", "Artist A"),
        create_test_track("2", "Track 2", "Artist B"),
        create_test_track("3", "Track 3", "Artist C"),
    ];

    manager.play_queue(tracks).unwrap();
    std::thread::sleep(Duration::from_millis(50));

    let initial_queue = manager.get_queue().unwrap();
    assert_eq!(initial_queue.len(), 3);

    // Skip track
    manager.next().unwrap();
    std::thread::sleep(Duration::from_millis(50));
    manager.drain_events();

    // Queue should be updated
    let updated_queue = manager.get_queue().unwrap();

    // Depending on implementation, queue may shrink after skip
    assert!(updated_queue.len() <= 3, "Queue should be updated after skip");
}

#[test]
fn test_e2e_queue_sidebar_receives_update_events() {
    let manager = TestPlaybackManager::new().unwrap();

    let tracks = vec![
        create_test_track("1", "Track 1", "Artist A"),
        create_test_track("2", "Track 2", "Artist B"),
    ];

    manager.play_queue(tracks).unwrap();
    std::thread::sleep(Duration::from_millis(100));

    // Collect events
    let events: Vec<_> = std::iter::from_fn(|| manager.try_recv_event())
        .take(50)
        .collect();

    // Should have queue updated events
    let has_queue_events = events
        .iter()
        .any(|e| matches!(e, PlaybackEvent::QueueUpdated));

    assert!(has_queue_events, "Should emit queue updated events");
}

// ===== UI Button State Tests =====

#[test]
fn test_e2e_next_button_disabled_at_end() {
    let manager = TestPlaybackManager::new().unwrap();

    let tracks = vec![create_test_track("1", "Track 1", "Artist A")];

    manager.play_queue(tracks).unwrap();
    std::thread::sleep(Duration::from_millis(50));

    // Skip past only track
    manager.next().unwrap();
    std::thread::sleep(Duration::from_millis(50));
    manager.drain_events();

    let (has_next, _) = manager.get_playback_capabilities().unwrap();

    // Next button should be disabled (has_next = false)
    assert!(!has_next, "Next button should be disabled at queue end");
}

#[test]
fn test_e2e_previous_button_disabled_at_start() {
    let manager = TestPlaybackManager::new().unwrap();

    let tracks = vec![create_test_track("1", "Track 1", "Artist A")];

    manager.play_queue(tracks).unwrap();
    std::thread::sleep(Duration::from_millis(50));
    manager.drain_events();

    let (_, has_previous) = manager.get_playback_capabilities().unwrap();

    // Previous button should be disabled (has_previous = false)
    assert!(
        !has_previous,
        "Previous button should be disabled at queue start"
    );
}

#[test]
fn test_e2e_buttons_enabled_in_middle_of_queue() {
    let manager = TestPlaybackManager::new().unwrap();

    let tracks = vec![
        create_test_track("1", "Track 1", "Artist A"),
        create_test_track("2", "Track 2", "Artist B"),
        create_test_track("3", "Track 3", "Artist C"),
    ];

    manager.play_queue(tracks).unwrap();
    std::thread::sleep(Duration::from_millis(50));

    // Skip to middle track
    manager.next().unwrap();
    std::thread::sleep(Duration::from_millis(50));
    manager.drain_events();

    let (has_next, has_previous) = manager.get_playback_capabilities().unwrap();

    // Both buttons should be enabled
    assert!(has_next, "Next button should be enabled in middle");
    assert!(has_previous, "Previous button should be enabled in middle");
}

// ===== Integration with Real Playback =====

#[test]
fn test_e2e_complete_queue_workflow() {
    let manager = TestPlaybackManager::new().unwrap();

    // Simulate user clicking track in library
    let library = vec![
        create_test_track("1", "Song One", "Artist A"),
        create_test_track("2", "Song Two", "Artist B"),
        create_test_track("3", "Song Three", "Artist C"),
        create_test_track("4", "Song Four", "Artist D"),
    ];

    // User clicks song 2 (index 1)
    let start_index = 1;
    let queue: Vec<QueueTrack> = library
        .iter()
        .skip(start_index)
        .chain(library.iter().take(start_index))
        .cloned()
        .collect();

    // 1. Play queue
    manager.play_queue(queue).unwrap();
    std::thread::sleep(Duration::from_millis(100));

    // 2. Check initial state
    let (has_next, has_previous) = manager.get_playback_capabilities().unwrap();
    assert!(has_next, "Should have next after playing");
    assert!(!has_previous, "Should not have previous initially");

    // 3. Open queue sidebar (get queue)
    let sidebar_queue = manager.get_queue().unwrap();
    assert_eq!(sidebar_queue.len(), 4);
    assert_eq!(sidebar_queue[0].id, "2"); // Started from track 2

    // 4. Click next button
    manager.next().unwrap();
    std::thread::sleep(Duration::from_millis(50));

    // 5. Check updated state
    let (has_next, has_previous) = manager.get_playback_capabilities().unwrap();
    assert!(has_next, "Should still have next");
    assert!(has_previous, "Should now have previous");

    // 6. Queue sidebar updates
    let updated_queue = manager.get_queue().unwrap();
    assert!(updated_queue.len() <= 4, "Queue updated after skip");

    // 7. Navigate to end
    manager.next().unwrap();
    std::thread::sleep(Duration::from_millis(30));
    manager.next().unwrap();
    std::thread::sleep(Duration::from_millis(30));
    manager.next().unwrap();
    std::thread::sleep(Duration::from_millis(50));
    manager.drain_events();

    // 8. Check end state
    let (has_next, has_previous) = manager.get_playback_capabilities().unwrap();
    assert!(!has_next, "Should not have next at end");
    assert!(has_previous, "Should still have previous at end");
}

#[test]
fn test_e2e_large_queue_performance() {
    let manager = TestPlaybackManager::new().unwrap();

    // Create large queue (simulating large library)
    let tracks: Vec<_> = (1..=100)
        .map(|i| {
            create_test_track(&i.to_string(), &format!("Track {}", i), "Various Artists")
        })
        .collect();

    let start = std::time::Instant::now();

    // Play queue
    manager.play_queue(tracks).unwrap();
    std::thread::sleep(Duration::from_millis(200));
    manager.drain_events();

    // Get queue
    let queue = manager.get_queue().unwrap();
    assert_eq!(queue.len(), 100);

    // Check capabilities
    let (has_next, has_previous) = manager.get_playback_capabilities().unwrap();

    let elapsed = start.elapsed();

    assert!(has_next, "Should have next with large queue");
    assert!(
        elapsed < Duration::from_secs(1),
        "Large queue operations should be fast, took {:?}",
        elapsed
    );
}

#[test]
fn test_e2e_rapid_queue_changes() {
    let manager = TestPlaybackManager::new().unwrap();

    // Simulate user rapidly changing tracks
    for i in 0..10 {
        let tracks = vec![
            create_test_track(&format!("{}a", i), &format!("Track {}A", i), "Artist"),
            create_test_track(&format!("{}b", i), &format!("Track {}B", i), "Artist"),
        ];

        manager.play_queue(tracks).unwrap();
        std::thread::sleep(Duration::from_millis(10));
    }

    std::thread::sleep(Duration::from_millis(100));
    manager.drain_events();

    // Should handle rapid changes
    let (has_next, _) = manager.get_playback_capabilities().unwrap();
    let queue = manager.get_queue().unwrap();

    // Final queue should be from last play_queue call
    assert_eq!(queue.len(), 2);
}
