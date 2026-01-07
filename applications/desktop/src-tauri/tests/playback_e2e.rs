//! End-to-end tests for Tauri playback integration
//!
//! These tests verify the complete flow from Tauri commands through
//! PlaybackManager to DesktopPlayback and event emission.

use soul_audio_desktop::{DesktopPlayback, LocalAudioSource, PlaybackCommand, PlaybackEvent};
use soul_playback::{PlaybackConfig, QueueTrack, RepeatMode, ShuffleMode, TrackSource};
use std::fs::File;
use std::io::Write as IoWrite;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tempfile::TempDir;

/// Helper to generate test WAV file
fn generate_test_wav(path: &PathBuf, duration_secs: f64, frequency: f64) -> std::io::Result<()> {
    let sample_rate = 44100;
    let num_samples = (sample_rate as f64 * duration_secs) as usize;
    let channels = 2;

    let mut file = File::create(path)?;

    // RIFF header
    file.write_all(b"RIFF")?;
    let file_size = 36 + num_samples * channels * 2;
    file.write_all(&(file_size as u32).to_le_bytes())?;
    file.write_all(b"WAVE")?;

    // fmt chunk
    file.write_all(b"fmt ")?;
    file.write_all(&16u32.to_le_bytes())?;
    file.write_all(&1u16.to_le_bytes())?;
    file.write_all(&(channels as u16).to_le_bytes())?;
    file.write_all(&(sample_rate as u32).to_le_bytes())?;
    file.write_all(&((sample_rate * channels * 2) as u32).to_le_bytes())?;
    file.write_all(&((channels * 2) as u16).to_le_bytes())?;
    file.write_all(&16u16.to_le_bytes())?;

    // data chunk
    file.write_all(b"data")?;
    file.write_all(&((num_samples * channels * 2) as u32).to_le_bytes())?;

    for i in 0..num_samples {
        let t = i as f64 / sample_rate as f64;
        let sample = (t * frequency * 2.0 * std::f64::consts::PI).sin();
        let sample_i16 = (sample * 32767.0) as i16;

        file.write_all(&sample_i16.to_le_bytes())?;
        file.write_all(&sample_i16.to_le_bytes())?;
    }

    Ok(())
}

/// Simulated PlaybackManager for testing (mirrors the actual implementation)
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

    fn play(&self) -> Result<(), String> {
        let playback = self.playback.lock().map_err(|e| e.to_string())?;
        playback
            .send_command(PlaybackCommand::Play)
            .map_err(|e| e.to_string())
    }

    fn pause(&self) -> Result<(), String> {
        let playback = self.playback.lock().map_err(|e| e.to_string())?;
        playback
            .send_command(PlaybackCommand::Pause)
            .map_err(|e| e.to_string())
    }

    fn stop(&self) -> Result<(), String> {
        let playback = self.playback.lock().map_err(|e| e.to_string())?;
        playback
            .send_command(PlaybackCommand::Stop)
            .map_err(|e| e.to_string())
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

    fn seek(&self, position: f64) -> Result<(), String> {
        let playback = self.playback.lock().map_err(|e| e.to_string())?;
        playback
            .send_command(PlaybackCommand::Seek(position))
            .map_err(|e| e.to_string())
    }

    fn set_volume(&self, volume: u8) -> Result<(), String> {
        let playback = self.playback.lock().map_err(|e| e.to_string())?;
        playback
            .send_command(PlaybackCommand::SetVolume(volume.clamp(0, 100)))
            .map_err(|e| e.to_string())
    }

    fn mute(&self) -> Result<(), String> {
        let playback = self.playback.lock().map_err(|e| e.to_string())?;
        playback
            .send_command(PlaybackCommand::Mute)
            .map_err(|e| e.to_string())
    }

    fn unmute(&self) -> Result<(), String> {
        let playback = self.playback.lock().map_err(|e| e.to_string())?;
        playback
            .send_command(PlaybackCommand::Unmute)
            .map_err(|e| e.to_string())
    }

    fn set_shuffle(&self, mode: ShuffleMode) -> Result<(), String> {
        let playback = self.playback.lock().map_err(|e| e.to_string())?;
        playback
            .send_command(PlaybackCommand::SetShuffle(mode))
            .map_err(|e| e.to_string())
    }

    fn set_repeat(&self, mode: RepeatMode) -> Result<(), String> {
        let playback = self.playback.lock().map_err(|e| e.to_string())?;
        playback
            .send_command(PlaybackCommand::SetRepeat(mode))
            .map_err(|e| e.to_string())
    }

    fn add_to_queue(&self, track: QueueTrack) -> Result<(), String> {
        let playback = self.playback.lock().map_err(|e| e.to_string())?;
        playback
            .send_command(PlaybackCommand::AddToQueue(track))
            .map_err(|e| e.to_string())
    }

    fn clear_queue(&self) -> Result<(), String> {
        let playback = self.playback.lock().map_err(|e| e.to_string())?;
        playback
            .send_command(PlaybackCommand::ClearQueue)
            .map_err(|e| e.to_string())
    }

    fn try_recv_event(&self) -> Option<PlaybackEvent> {
        let playback = self.playback.lock().ok()?;
        playback.try_recv_event()
    }
}

// ===== E2E Tests =====

#[test]
fn test_e2e_playback_manager_creation() {
    let manager = TestPlaybackManager::new();
    assert!(manager.is_ok(), "Should create PlaybackManager");
}

#[test]
fn test_e2e_play_pause_stop_workflow() {
    let manager = TestPlaybackManager::new().unwrap();

    // Execute playback workflow
    assert!(manager.play().is_ok(), "Play should succeed");
    std::thread::sleep(Duration::from_millis(50));

    assert!(manager.pause().is_ok(), "Pause should succeed");
    std::thread::sleep(Duration::from_millis(50));

    assert!(manager.play().is_ok(), "Resume should succeed");
    std::thread::sleep(Duration::from_millis(50));

    assert!(manager.stop().is_ok(), "Stop should succeed");
    std::thread::sleep(Duration::from_millis(50));

    // Collect events
    let events: Vec<_> = std::iter::from_fn(|| manager.try_recv_event())
        .take(20)
        .collect();

    // Should have received state change events
    let has_state_events = events
        .iter()
        .any(|e| matches!(e, PlaybackEvent::StateChanged(_)));

    assert!(has_state_events, "Should receive state changed events");
}

#[test]
fn test_e2e_volume_control() {
    let manager = TestPlaybackManager::new().unwrap();

    // Test volume control sequence
    let volumes = [0, 25, 50, 75, 100];

    for vol in volumes {
        assert!(
            manager.set_volume(vol).is_ok(),
            "Should set volume to {}",
            vol
        );
        std::thread::sleep(Duration::from_millis(30));
    }

    // Collect volume events
    std::thread::sleep(Duration::from_millis(100));

    let events: Vec<_> = std::iter::from_fn(|| manager.try_recv_event())
        .take(30)
        .collect();

    let volume_events: Vec<_> = events
        .iter()
        .filter_map(|e| {
            if let PlaybackEvent::VolumeChanged(v) = e {
                Some(*v)
            } else {
                None
            }
        })
        .collect();

    assert!(
        !volume_events.is_empty(),
        "Should receive volume changed events"
    );
}

#[test]
fn test_e2e_mute_unmute() {
    let manager = TestPlaybackManager::new().unwrap();

    assert!(manager.mute().is_ok(), "Mute should succeed");
    std::thread::sleep(Duration::from_millis(50));

    assert!(manager.unmute().is_ok(), "Unmute should succeed");
    std::thread::sleep(Duration::from_millis(50));

    // Should have volume events
    let events: Vec<_> = std::iter::from_fn(|| manager.try_recv_event())
        .take(10)
        .collect();

    let has_volume_events = events
        .iter()
        .any(|e| matches!(e, PlaybackEvent::VolumeChanged(_)));

    assert!(has_volume_events, "Should have volume events");
}

#[test]
fn test_e2e_shuffle_modes() {
    let manager = TestPlaybackManager::new().unwrap();

    // Test all shuffle modes
    for mode in [ShuffleMode::Off, ShuffleMode::Random, ShuffleMode::Smart] {
        assert!(
            manager.set_shuffle(mode.clone()).is_ok(),
            "Should set shuffle mode"
        );
        std::thread::sleep(Duration::from_millis(20));
    }

    // Should complete without errors
}

#[test]
fn test_e2e_repeat_modes() {
    let manager = TestPlaybackManager::new().unwrap();

    // Test all repeat modes
    for mode in [RepeatMode::Off, RepeatMode::All, RepeatMode::One] {
        assert!(manager.set_repeat(mode).is_ok(), "Should set repeat mode");
        std::thread::sleep(Duration::from_millis(20));
    }

    // Should complete without errors
}

#[test]
fn test_e2e_queue_management() {
    let manager = TestPlaybackManager::new().unwrap();

    // Create test tracks
    let tracks: Vec<_> = (1..=5)
        .map(|i| QueueTrack {
            id: format!("track{}", i),
            path: PathBuf::from(format!("/test/track{}.mp3", i)),
            title: format!("Track {}", i),
            artist: "Test Artist".to_string(),
            album: Some("Test Album".to_string()),
            duration: Duration::from_secs(180),
            track_number: Some(i),
            source: TrackSource::Single,
        })
        .collect();

    // Add all tracks
    for track in tracks {
        assert!(manager.add_to_queue(track).is_ok(), "Should add track");
        std::thread::sleep(Duration::from_millis(10));
    }

    // Clear queue
    assert!(manager.clear_queue().is_ok(), "Should clear queue");
    std::thread::sleep(Duration::from_millis(50));

    // Should have queue updated events
    let events: Vec<_> = std::iter::from_fn(|| manager.try_recv_event())
        .take(30)
        .collect();

    let queue_events = events
        .iter()
        .filter(|e| matches!(e, PlaybackEvent::QueueUpdated))
        .count();

    assert!(
        queue_events >= 5,
        "Should have multiple queue update events"
    );
}

#[test]
fn test_e2e_seek_command() {
    let manager = TestPlaybackManager::new().unwrap();

    // Test seeking to various positions
    for position in [0.0, 30.5, 60.0, 120.0] {
        assert!(
            manager.seek(position).is_ok(),
            "Should accept seek to {}",
            position
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn test_e2e_navigation_commands() {
    let manager = TestPlaybackManager::new().unwrap();

    // Add some tracks to enable navigation
    for i in 1..=3 {
        let track = QueueTrack {
            id: format!("track{}", i),
            path: PathBuf::from(format!("/test/track{}.mp3", i)),
            title: format!("Track {}", i),
            artist: "Test Artist".to_string(),
            album: None,
            duration: Duration::from_secs(180),
            track_number: Some(i),
            source: TrackSource::Single,
        };
        manager.add_to_queue(track).unwrap();
    }

    std::thread::sleep(Duration::from_millis(50));

    // Test navigation
    assert!(manager.next().is_ok(), "Next should succeed");
    std::thread::sleep(Duration::from_millis(30));

    assert!(manager.next().is_ok(), "Next should succeed");
    std::thread::sleep(Duration::from_millis(30));

    assert!(manager.previous().is_ok(), "Previous should succeed");
    std::thread::sleep(Duration::from_millis(50));

    // Should have received events
    let events: Vec<_> = std::iter::from_fn(|| manager.try_recv_event())
        .take(50)
        .collect();

    assert!(!events.is_empty(), "Should have received events");
}

#[test]
fn test_e2e_complete_user_session() {
    let manager = TestPlaybackManager::new().unwrap();

    // Simulate a complete user session

    // 1. Set initial volume
    manager.set_volume(80).unwrap();
    std::thread::sleep(Duration::from_millis(30));

    // 2. Enable shuffle
    manager.set_shuffle(ShuffleMode::Random).unwrap();
    std::thread::sleep(Duration::from_millis(20));

    // 3. Add playlist
    for i in 1..=10 {
        let track = QueueTrack {
            id: format!("track{}", i),
            path: PathBuf::from(format!("/music/track{}.mp3", i)),
            title: format!("Song {}", i),
            artist: "Artist".to_string(),
            album: Some("Album".to_string()),
            duration: Duration::from_secs(180),
            track_number: Some(i),
            source: TrackSource::Playlist {
                id: "playlist1".to_string(),
                name: "Test Playlist".to_string(),
            },
        };
        manager.add_to_queue(track).unwrap();
        std::thread::sleep(Duration::from_millis(5));
    }

    // 4. Start playback
    manager.play().unwrap();
    std::thread::sleep(Duration::from_millis(50));

    // 5. Skip a few tracks
    manager.next().unwrap();
    std::thread::sleep(Duration::from_millis(30));
    manager.next().unwrap();
    std::thread::sleep(Duration::from_millis(30));

    // 6. Adjust volume
    manager.set_volume(60).unwrap();
    std::thread::sleep(Duration::from_millis(30));

    // 7. Seek within track
    manager.seek(45.0).unwrap();
    std::thread::sleep(Duration::from_millis(30));

    // 8. Pause
    manager.pause().unwrap();
    std::thread::sleep(Duration::from_millis(50));

    // 9. Resume
    manager.play().unwrap();
    std::thread::sleep(Duration::from_millis(50));

    // 10. Stop
    manager.stop().unwrap();
    std::thread::sleep(Duration::from_millis(50));

    // Collect all events
    let events: Vec<_> = std::iter::from_fn(|| manager.try_recv_event())
        .take(100)
        .collect();

    // Should have received various event types
    let has_state_events = events
        .iter()
        .any(|e| matches!(e, PlaybackEvent::StateChanged(_)));
    let has_volume_events = events
        .iter()
        .any(|e| matches!(e, PlaybackEvent::VolumeChanged(_)));
    let has_queue_events = events
        .iter()
        .any(|e| matches!(e, PlaybackEvent::QueueUpdated));

    assert!(has_state_events, "Should have state events");
    assert!(has_volume_events, "Should have volume events");
    assert!(has_queue_events, "Should have queue events");
}

#[test]
fn test_e2e_error_handling() {
    let manager = TestPlaybackManager::new().unwrap();

    // Test that invalid operations don't crash the system

    // Try to play without any tracks
    assert!(manager.play().is_ok(), "Empty play should be accepted");
    std::thread::sleep(Duration::from_millis(30));

    // Try to navigate with empty queue
    assert!(manager.next().is_ok(), "Empty next should be accepted");
    std::thread::sleep(Duration::from_millis(20));

    assert!(
        manager.previous().is_ok(),
        "Empty previous should be accepted"
    );
    std::thread::sleep(Duration::from_millis(50));

    // System should still be responsive
    assert!(
        manager.set_volume(50).is_ok(),
        "Should still respond to commands"
    );
}

#[test]
fn test_e2e_concurrent_operations() {
    let manager = Arc::new(TestPlaybackManager::new().unwrap());

    // Simulate concurrent UI interactions
    let handles: Vec<_> = (0..3)
        .map(|i| {
            let mgr = manager.clone();
            std::thread::spawn(move || {
                for j in 0..20 {
                    let vol = ((i * 20 + j) % 100) as u8;
                    mgr.set_volume(vol).unwrap();
                    std::thread::sleep(Duration::from_millis(5));
                }
            })
        })
        .collect();

    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }

    // System should have handled concurrent access
    std::thread::sleep(Duration::from_millis(100));

    // Should still be responsive
    assert!(
        manager.set_volume(70).is_ok(),
        "Should still respond after concurrent access"
    );
}

#[test]
fn test_e2e_volume_boundary_values() {
    let manager = TestPlaybackManager::new().unwrap();

    // Test boundary values
    for vol in [0, 1, 50, 99, 100, 150, 255] {
        assert!(
            manager.set_volume(vol).is_ok(),
            "Should handle volume value {}",
            vol
        );
        std::thread::sleep(Duration::from_millis(20));
    }

    std::thread::sleep(Duration::from_millis(100));

    // Collect volume events and verify they're in valid range
    let events: Vec<_> = std::iter::from_fn(|| manager.try_recv_event())
        .take(30)
        .collect();

    for event in events {
        if let PlaybackEvent::VolumeChanged(vol) = event {
            assert!(vol <= 100, "Volume should be clamped to 100, got {}", vol);
        }
    }
}
