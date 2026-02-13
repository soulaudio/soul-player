use soul_playback::{PlaybackConfig, PlaybackManager, QueueContext, QueueTrack, TrackSource};
use std::path::PathBuf;
use std::time::Duration;

fn create_test_track(i: usize) -> QueueTrack {
    QueueTrack {
        id: format!("track_{}", i),
        path: PathBuf::from(format!("test/track_{}.mp3", i)),
        title: format!("Track {}", i),
        artist: "Test Artist".to_string(),
        album: Some("Test Album".to_string()),
        duration: Duration::from_secs(180),
        track_number: Some((i + 1) as u32),
        source: TrackSource::Album {
            id: "1".to_string(),
            name: "Test Album".to_string(),
        },
    }
}

#[test]
fn test_lazy_queue_context_and_detection() {
    // Create manager with default config
    let config = PlaybackConfig::default();
    let mut manager = PlaybackManager::new(config);

    // Create a large queue context (500 tracks)
    let context = QueueContext::AllTracks {
        user_id: 1,
        total_count: 500,
    };

    // Create initial batch of 50 tracks
    let initial_batch: Vec<QueueTrack> = (0..50).map(create_test_track).collect();

    // Load initial batch and set lazy context
    manager.load_playlist(initial_batch.clone(), 0);
    manager.set_lazy_context(context.clone(), None);

    // Verify lazy state was set
    assert!(
        manager.get_lazy_state().is_some(),
        "Lazy state should be set"
    );

    // Verify lazy state has correct context
    let lazy_state = manager.get_lazy_state().unwrap();
    match &lazy_state.context {
        QueueContext::AllTracks {
            user_id,
            total_count,
        } => {
            assert_eq!(*user_id, 1);
            assert_eq!(*total_count, 500);
        }
        _ => panic!("Expected AllTracks context"),
    }

    println!("✓ Lazy queue context set correctly");

    // Simulate playback progression - advance to track 42
    for i in 0..45 {
        let _ = manager.next();

        // Check current position in queue
        let remaining = 50 - (i + 1);

        // Check if batch loading should trigger (< 10 tracks remaining)
        let should_load = manager.check_batch_loading();

        if remaining < 10 {
            // Should trigger batch loading (< 10 tracks remaining)
            assert!(
                should_load.is_some(),
                "Batch loading should trigger at track {} ({} remaining)",
                i + 1,
                remaining
            );

            if let Some((offset, limit)) = should_load {
                println!(
                    "✓ Batch loading triggered at track {} ({} remaining): offset={}, limit={}",
                    i + 1,
                    remaining,
                    offset,
                    limit
                );

                // Verify it's requesting the next batch (tracks 50-100)
                assert_eq!(offset, 50, "Should request next batch starting at track 50");
                assert_eq!(limit, 50, "Should request 50 tracks");
                break;
            }
        } else {
            // Should not trigger yet (>= 10 tracks remaining)
            assert!(
                should_load.is_none(),
                "Should not trigger batch loading at track {} ({} remaining)",
                i + 1,
                remaining
            );
        }
    }

    println!("✓ Forward pagination detection test PASSED");
}

#[test]
fn test_lazy_queue_jump_detection() {
    // Create manager with default config
    let config = PlaybackConfig::default();
    let mut manager = PlaybackManager::new(config);

    // Create a large queue context (500 tracks)
    let context = QueueContext::Album {
        album_id: 1,
        total_count: 500,
    };

    // Create initial batch of 50 tracks
    let initial_batch: Vec<QueueTrack> = (0..50).map(create_test_track).collect();

    // Load initial batch and set lazy context
    manager.load_playlist(initial_batch.clone(), 0);
    manager.set_lazy_context(context, None);

    // Test 1: Try to jump to track 450 (beyond loaded window of 0-49)
    let jump_result = manager.check_jump_loading(450);
    assert!(
        jump_result.is_some(),
        "Jump loading should trigger for track 450"
    );

    if let Some((offset, limit)) = jump_result {
        println!(
            "✓ Jump loading triggered for index 450: offset={}, limit={}",
            offset, limit
        );

        // Should load batch containing track 450
        assert!(
            (400..=450).contains(&offset),
            "Offset {} should be near track 450",
            offset
        );
        assert_eq!(limit, 50, "Should request 50 tracks");
    }

    // Test 2: Try to jump to track 25 (within loaded window)
    let jump_within = manager.check_jump_loading(25);
    assert!(
        jump_within.is_none(),
        "Jump loading should NOT trigger for track 25 (already loaded)"
    );

    println!("✓ Jump loading detection test PASSED");
}

#[test]
fn test_no_lazy_loading_for_small_queue() {
    // Create manager with default config
    let config = PlaybackConfig::default();
    let mut manager = PlaybackManager::new(config);

    // Create small queue (50 tracks, below threshold)
    let small_queue: Vec<QueueTrack> = (0..50).map(create_test_track).collect();

    // Load queue WITHOUT lazy context
    manager.load_playlist(small_queue, 0);

    // Verify no lazy state
    assert!(
        manager.get_lazy_state().is_none(),
        "Small queue should not have lazy state"
    );

    // Advance through most of the queue
    for i in 0..45 {
        let _ = manager.next();

        // Should never trigger batch loading (no lazy context)
        let batch_check = manager.check_batch_loading();
        assert!(
            batch_check.is_none(),
            "Should not trigger batch loading at track {}",
            i
        );

        // Jump loading should also not trigger
        let jump_check = manager.check_jump_loading(i + 10);
        assert!(
            jump_check.is_none(),
            "Should not trigger jump loading without lazy context"
        );
    }

    println!("✓ Small queue correctly skips lazy loading");
}

#[test]
fn test_lazy_context_clear() {
    // Create manager
    let config = PlaybackConfig::default();
    let mut manager = PlaybackManager::new(config);

    // Set lazy context
    let context = QueueContext::AllTracks {
        user_id: 1,
        total_count: 500,
    };
    let initial_batch: Vec<QueueTrack> = (0..50).map(create_test_track).collect();
    manager.load_playlist(initial_batch, 0);
    manager.set_lazy_context(context, None);

    // Verify it's set
    assert!(manager.get_lazy_state().is_some());

    // Clear lazy context
    manager.clear_lazy_context();

    // Verify it's cleared
    assert!(
        manager.get_lazy_state().is_none(),
        "Lazy context should be cleared"
    );

    // Batch loading should not trigger after clear
    for _ in 0..45 {
        let _ = manager.next();
        assert!(manager.check_batch_loading().is_none());
    }

    println!("✓ Lazy context clear test PASSED");
}
