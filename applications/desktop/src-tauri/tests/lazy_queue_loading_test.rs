//! Lazy Queue Loading Integration Tests
//!
//! Tests that the lazy queue system correctly loads batches of tracks on demand:
//! 1. Initial batch loading when playing from large library
//! 2. Forward pagination when approaching end of loaded window
//! 3. Jump loading when skipping beyond loaded window
//! 4. Queue growth through multiple batch loads
//!
//! These tests use a real database with 500 test tracks to verify the full
//! playback pipeline from database queries through to queue management.

// FIXME: Temporarily disabled due to PlaybackManager API changes from stash merge
#![cfg(all())]

use soul_audio_desktop::{DesktopPlayback, PlaybackCommand};
use soul_playback::{lazy_queue::QueueContext, PlaybackConfig, QueueTrack, TrackSource};
use soul_storage::{create_pool, run_migrations};
use sqlx::SqlitePool;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tempfile::TempDir;

/// Test fixture that manages a temporary database with test tracks
struct TestDb {
    db_path: PathBuf,
    _temp_dir: TempDir,
    pool: SqlitePool,
}

impl TestDb {
    /// Create a new test database with 500 test tracks
    async fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let db_path = temp_dir.path().join("test.db");

        let db_url = if cfg!(windows) {
            let path_str = db_path.to_str().unwrap().replace('\\', "/");
            format!("sqlite:///{}", path_str)
        } else {
            format!("sqlite://{}", db_path.to_str().unwrap())
        };

        eprintln!("[TestDb] Creating pool at: {}", db_url);
        let pool = create_pool(&db_url).await.expect("Failed to create pool");

        eprintln!("[TestDb] Running migrations...");
        run_migrations(&pool)
            .await
            .expect("Failed to run migrations");

        eprintln!("[TestDb] Seeding test data...");
        Self::seed_test_data(&pool).await;

        Self {
            db_path,
            _temp_dir: temp_dir,
            pool,
        }
    }

    /// Seed 500 test tracks into the database
    async fn seed_test_data(pool: &SqlitePool) {
        // Create test artist
        sqlx::query("INSERT INTO artists (id, name) VALUES (?, ?)")
            .bind(9999_i64)
            .bind("Test Artist")
            .execute(pool)
            .await
            .expect("Failed to insert test artist");

        // Create test album
        sqlx::query("INSERT INTO albums (id, title, artist_id, year) VALUES (?, ?, ?, ?)")
            .bind(9999_i64)
            .bind("Test Album")
            .bind(9999_i64)
            .bind(2024_i64)
            .execute(pool)
            .await
            .expect("Failed to insert test album");

        // Create local source
        sqlx::query("INSERT OR IGNORE INTO sources (id, name, source_type) VALUES (?, ?, ?)")
            .bind(1_i64)
            .bind("Local")
            .bind("local")
            .execute(pool)
            .await
            .ok();

        // Insert 500 test tracks in batches for performance
        eprintln!("[TestDb] Inserting 500 test tracks...");
        for batch_start in (1..=500).step_by(50) {
            let batch_end = (batch_start + 49).min(500);

            for i in batch_start..=batch_end {
                let track_id = 10000 + i;
                let title = format!("Test Track {}", i);
                let file_path = format!("test/track_{}.mp3", i);

                // Insert track
                sqlx::query(
                    "INSERT INTO tracks (id, title, artist_id, album_id, track_number, disc_number, duration_seconds, file_format)
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
                )
                .bind(track_id as i64)
                .bind(&title)
                .bind(9999_i64)
                .bind(9999_i64)
                .bind(i as i64)
                .bind(1_i64)
                .bind(180.0_f64)
                .bind("mp3")
                .execute(pool)
                .await
                .expect("Failed to insert track");

                // Insert availability
                sqlx::query(
                    "INSERT INTO track_availability (track_id, source_id, status, local_file_path)
                     VALUES (?, ?, ?, ?)",
                )
                .bind(track_id as i64)
                .bind(1_i64)
                .bind("available")
                .bind(&file_path)
                .execute(pool)
                .await
                .expect("Failed to insert availability");
            }

            if batch_end % 100 == 0 {
                eprintln!("[TestDb] Inserted {}/500 tracks...", batch_end);
            }
        }

        // Verify count
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM tracks WHERE id >= 10000 AND id < 10500")
                .fetch_one(pool)
                .await
                .expect("Failed to count tracks");

        eprintln!("[TestDb] ✓ Inserted {} test tracks", count);
        assert_eq!(count, 500, "Should have inserted 500 tracks");
    }

    /// Get paginated tracks from database (simulates batch loading)
    async fn get_tracks_paginated(&self, offset: i64, limit: i64) -> Vec<soul_core::Track> {
        soul_storage::tracks::get_all_paginated(&self.pool, offset, limit)
            .await
            .expect("Failed to get paginated tracks")
    }
}

/// Helper to create QueueTrack from soul_core::Track
fn to_queue_track(track: &soul_core::Track, library_path: &str) -> QueueTrack {
    let file_path = track
        .availability
        .first()
        .and_then(|a| a.local_file_path.as_ref())
        .map(|p| PathBuf::from(library_path).join(p))
        .unwrap_or_else(|| PathBuf::from("missing.mp3"));

    QueueTrack {
        id: track.id.to_string(),
        title: track.title.clone(),
        artist: track.artist_name.clone().unwrap_or_default(),
        album: track.album_title.clone(),
        duration: Duration::from_secs(track.duration_seconds.unwrap_or(0.0) as u64),
        path: file_path,
        source: TrackSource::Single,
        track_number: track.track_number.map(|n| n as u32),
    }
}

// =============================================================================
// LAZY QUEUE LOADING TESTS
// =============================================================================

#[tokio::test]
async fn test_initial_batch_loading() {
    let test_db = TestDb::new().await;

    // Load initial batch (first 50 tracks)
    let tracks = test_db.get_tracks_paginated(0, 50).await;
    assert_eq!(tracks.len(), 50, "Should load initial batch of 50 tracks");

    // Verify tracks are in correct order
    assert_eq!(tracks[0].title, "Test Track 1");
    assert_eq!(tracks[49].title, "Test Track 50");

    eprintln!("[Test] ✓ Initial batch loading works");
}

#[tokio::test]
async fn test_forward_pagination() {
    let test_db = TestDb::new().await;

    // Load first batch
    let batch1 = test_db.get_tracks_paginated(0, 50).await;
    assert_eq!(batch1.len(), 50);

    // Load second batch (forward pagination)
    let batch2 = test_db.get_tracks_paginated(50, 50).await;
    assert_eq!(batch2.len(), 50);
    assert_eq!(batch2[0].title, "Test Track 51");
    assert_eq!(batch2[49].title, "Test Track 100");

    eprintln!("[Test] ✓ Forward pagination works");
}

#[tokio::test]
async fn test_jump_beyond_window() {
    let test_db = TestDb::new().await;

    // Jump to track 250 (way beyond initial window)
    let batch = test_db.get_tracks_paginated(250, 50).await;
    assert_eq!(batch.len(), 50);
    assert_eq!(batch[0].title, "Test Track 251");

    eprintln!("[Test] ✓ Jump beyond window works");
}

#[tokio::test]
async fn test_last_batch_partial() {
    let test_db = TestDb::new().await;

    // Load last batch (should be exactly 50 tracks since we have 500 total)
    let batch = test_db.get_tracks_paginated(450, 50).await;
    assert_eq!(batch.len(), 50);
    assert_eq!(batch[0].title, "Test Track 451");
    assert_eq!(batch[49].title, "Test Track 500");

    // Try to load beyond end (should return empty)
    let empty = test_db.get_tracks_paginated(500, 50).await;
    assert_eq!(empty.len(), 0, "Should return empty when beyond end");

    eprintln!("[Test] ✓ Last batch and boundary handling works");
}

#[tokio::test]
async fn test_playback_manager_with_lazy_context() {
    let test_db = TestDb::new().await;
    let library_path = test_db.db_path.parent().unwrap().to_str().unwrap();

    // Create playback config (use defaults)
    let config = PlaybackConfig::default();

    // Create playback manager
    let playback = DesktopPlayback::new(config).expect("Failed to create playback");
    let manager = Arc::new(Mutex::new(playback));

    // Load initial batch
    let initial_tracks = test_db.get_tracks_paginated(0, 50).await;
    let queue_tracks: Vec<QueueTrack> = initial_tracks
        .iter()
        .map(|t| to_queue_track(t, library_path))
        .collect();

    // Load playlist (simulates play_queue_with_context)
    {
        let pb = manager.lock().unwrap();
        pb.send_command(PlaybackCommand::LoadPlaylist(queue_tracks))
            .expect("Failed to load playlist");

        // Set lazy context
        let mut mgr = pb.get_playback_manager().lock().unwrap();
        mgr.set_lazy_context(
            QueueContext::AllTracks {
                user_id: 1,
                total_count: 500,
            },
            None, // No shuffle for this test
        );

        // Verify lazy state is set
        assert!(mgr.get_lazy_state().is_some(), "Lazy state should be set");

        let lazy_state = mgr.get_lazy_state().unwrap();
        assert_eq!(lazy_state.window_start, 0);
        assert_eq!(lazy_state.window_end, 50);

        eprintln!("[Test] ✓ Lazy context set correctly");
    }

    // Simulate playing through tracks and checking for batch loading
    {
        let pb = manager.lock().unwrap();
        let mut mgr = pb.get_playback_manager().lock().unwrap();

        // Simulate being at track 44 (near end of window)
        let batch_needed = mgr.check_batch_loading();
        assert!(
            batch_needed.is_some(),
            "Should detect batch loading needed at track 44"
        );

        let (offset, limit) = batch_needed.unwrap();
        assert_eq!(offset, 50, "Should request batch starting at offset 50");
        assert_eq!(limit, 50, "Should request 50 tracks");

        eprintln!("[Test] ✓ Batch loading detection works");
    }

    // Simulate appending the next batch
    {
        let next_batch = test_db.get_tracks_paginated(50, 50).await;
        let queue_tracks: Vec<QueueTrack> = next_batch
            .iter()
            .map(|t| to_queue_track(t, library_path))
            .collect();

        let pb = manager.lock().unwrap();
        pb.send_command(PlaybackCommand::AppendToSource(queue_tracks))
            .expect("Failed to append batch");

        // Verify queue grew
        let mgr = pb.get_playback_manager().lock().unwrap();
        let queue_len = mgr.queue_len();
        assert!(
            queue_len >= 100,
            "Queue should have grown to at least 100 tracks, got {}",
            queue_len
        );

        eprintln!("[Test] ✓ Batch appending works");
    }

    eprintln!("[Test] ✓ Full lazy loading flow works end-to-end");
}

#[tokio::test]
async fn test_jump_loading_triggers_batch_request() {
    let test_db = TestDb::new().await;
    let library_path = test_db.db_path.parent().unwrap().to_str().unwrap();

    let config = PlaybackConfig::default();

    let playback = DesktopPlayback::new(config).expect("Failed to create playback");
    let manager = Arc::new(Mutex::new(playback));

    // Load initial batch and set lazy context
    {
        let initial_tracks = test_db.get_tracks_paginated(0, 50).await;
        let queue_tracks: Vec<QueueTrack> = initial_tracks
            .iter()
            .map(|t| to_queue_track(t, library_path))
            .collect();

        let pb = manager.lock().unwrap();
        pb.send_command(PlaybackCommand::LoadPlaylist(queue_tracks))
            .expect("Failed to load playlist");

        let mut mgr = pb.get_playback_manager().lock().unwrap();
        mgr.set_lazy_context(
            QueueContext::AllTracks {
                user_id: 1,
                total_count: 500,
            },
            None,
        );
    }

    // Try to skip to track 250 (way beyond loaded window)
    {
        let pb = manager.lock().unwrap();
        let mut mgr = pb.get_playback_manager().lock().unwrap();

        let jump_needed = mgr.check_jump_loading(249); // 0-indexed, so 249 = track 250
        assert!(
            jump_needed.is_some(),
            "Should detect jump loading needed for track 250"
        );

        let (offset, limit) = jump_needed.unwrap();
        assert_eq!(
            offset, 200,
            "Should request batch starting at offset 200 (batch containing track 250)"
        );
        assert_eq!(limit, 50, "Should request 50 tracks");

        eprintln!("[Test] ✓ Jump loading detection works");
    }

    eprintln!("[Test] ✓ Jump loading triggers correct batch request");
}
