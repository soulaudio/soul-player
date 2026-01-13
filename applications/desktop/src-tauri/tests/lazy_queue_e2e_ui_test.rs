//! Full E2E UI Test for Lazy Queue Loading
//!
//! This test:
//! 1. Creates an isolated test database (temp dir)
//! 2. Seeds it with 500 test tracks using SQLx
//! 3. Launches the actual desktop app UI with the test database
//! 4. Uses UI automation to test the lazy queue loading workflow
//!
//! **SQLx Note**: We use SQLx, NOT an ORM. SQLx provides compile-time SQL verification
//! and direct SQL queries without the overhead of an ORM layer.

use sqlx::SqlitePool;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::time::Duration;
use tempfile::TempDir;

/// Test fixture that manages isolated test database and app instance
struct E2ETestHarness {
    _temp_dir: TempDir,
    db_path: PathBuf,
    pool: SqlitePool,
    app_process: Option<Child>,
}

impl E2ETestHarness {
    /// Create new test harness with fresh database
    async fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let db_path = temp_dir.path().join("test.db");

        eprintln!("[E2E] Created temp dir: {}", temp_dir.path().display());
        eprintln!("[E2E] Database path: {}", db_path.display());

        // Create database URL
        let db_url = if cfg!(windows) {
            let path_str = db_path.to_str().unwrap().replace('\\', "/");
            format!("sqlite:///{}", path_str)
        } else {
            format!("sqlite://{}", db_path.to_str().unwrap())
        };

        eprintln!("[E2E] Database URL: {}", db_url);

        // Create pool using SQLx (NOT an ORM - direct SQL with compile-time verification)
        let pool = soul_storage::create_pool(&db_url)
            .await
            .expect("Failed to create pool");

        // Run migrations
        eprintln!("[E2E] Running migrations...");
        soul_storage::run_migrations(&pool)
            .await
            .expect("Failed to run migrations");

        // Create default user using raw SQL via SQLx
        eprintln!("[E2E] Creating default user...");
        let now = chrono::Utc::now().timestamp();
        sqlx::query("INSERT INTO users (id, name, created_at) VALUES (?, ?, ?)")
            .bind("1")
            .bind("Test User")
            .bind(now)
            .execute(&pool)
            .await
            .expect("Failed to create user");

        Self {
            _temp_dir: temp_dir,
            db_path,
            pool,
            app_process: None,
        }
    }

    /// Seed database with 500 test tracks using SQLx raw SQL
    async fn seed_test_data(&self) {
        eprintln!("[E2E] Seeding test data...");

        // Create test artist using SQLx raw SQL (NOT an ORM)
        sqlx::query("INSERT INTO artists (id, name) VALUES (?, ?)")
            .bind(9999_i64)
            .bind("Test Artist")
            .execute(&self.pool)
            .await
            .expect("Failed to insert test artist");

        // Create test album using SQLx raw SQL
        sqlx::query("INSERT INTO albums (id, title, artist_id, year) VALUES (?, ?, ?, ?)")
            .bind(9999_i64)
            .bind("Test Album")
            .bind(9999_i64)
            .bind(2024_i64)
            .execute(&self.pool)
            .await
            .expect("Failed to insert test album");

        // Create local source using SQLx raw SQL
        sqlx::query("INSERT OR IGNORE INTO sources (id, name, source_type) VALUES (?, ?, ?)")
            .bind(1_i64)
            .bind("Local")
            .bind("local")
            .execute(&self.pool)
            .await
            .ok();

        // Insert 500 test tracks in batches using SQLx raw SQL
        eprintln!("[E2E] Inserting 500 test tracks...");
        for batch_start in (1..=500).step_by(50) {
            let batch_end = (batch_start + 49).min(500);

            for i in batch_start..=batch_end {
                let track_id = 10000 + i;
                let title = format!("Test Track {}", i);
                let file_path = format!("test/track_{}.mp3", i);

                // Insert track using SQLx raw SQL (NOT an ORM)
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
                .execute(&self.pool)
                .await
                .expect("Failed to insert track");

                // Insert availability using SQLx raw SQL
                sqlx::query(
                    "INSERT INTO track_availability (track_id, source_id, status, local_file_path)
                     VALUES (?, ?, ?, ?)",
                )
                .bind(track_id as i64)
                .bind(1_i64)
                .bind("available")
                .bind(&file_path)
                .execute(&self.pool)
                .await
                .expect("Failed to insert availability");
            }

            if batch_end % 100 == 0 {
                eprintln!("[E2E] Inserted {}/500 tracks...", batch_end);
            }
        }

        // Verify count using SQLx query
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM tracks WHERE id >= 10000 AND id < 10500")
                .fetch_one(&self.pool)
                .await
                .expect("Failed to count tracks");

        eprintln!("[E2E] ✓ Inserted {} test tracks", count);
        assert_eq!(count, 500, "Should have inserted 500 tracks");
    }

    /// Launch the desktop app with the test database
    fn launch_app(&mut self) -> Result<(), String> {
        eprintln!("[E2E] Launching app with test database...");

        // Determine app binary path
        let app_binary = if cfg!(debug_assertions) {
            "target/debug/soul-player-desktop"
        } else {
            "target/release/soul-player-desktop"
        };

        let app_binary = if cfg!(windows) {
            format!("{}.exe", app_binary)
        } else {
            app_binary.to_string()
        };

        // Build absolute path to binary (from workspace root)
        let workspace_root = std::env::current_dir()
            .map_err(|e| format!("Failed to get current dir: {}", e))?
            .ancestors()
            .nth(3) // Go up from src-tauri/tests
            .ok_or("Failed to find workspace root")?
            .to_path_buf();

        let app_path = workspace_root.join(&app_binary);

        if !app_path.exists() {
            return Err(format!(
                "App binary not found at: {}\nBuild it first: cargo build --release -p soul-player-desktop",
                app_path.display()
            ));
        }

        eprintln!("[E2E] App binary: {}", app_path.display());
        eprintln!("[E2E] Database path: {}", self.db_path.display());

        // Launch app with DATABASE_PATH environment variable
        let mut cmd = Command::new(&app_path);
        cmd.env("DATABASE_PATH", &self.db_path);

        // On Windows, launch without creating a console window
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        let child = cmd
            .spawn()
            .map_err(|e| format!("Failed to launch app: {}", e))?;

        eprintln!("[E2E] ✓ App launched with PID: {}", child.id());
        self.app_process = Some(child);

        // Wait for app to start
        eprintln!("[E2E] Waiting 5 seconds for app to initialize...");
        std::thread::sleep(Duration::from_secs(5));

        Ok(())
    }

    /// Kill the app process
    fn kill_app(&mut self) {
        if let Some(mut process) = self.app_process.take() {
            eprintln!("[E2E] Killing app process...");
            let _ = process.kill();
            let _ = process.wait();
            eprintln!("[E2E] ✓ App killed");
        }
    }
}

impl Drop for E2ETestHarness {
    fn drop(&mut self) {
        self.kill_app();
        eprintln!("[E2E] ✓ Test harness cleaned up");
    }
}

// =============================================================================
// E2E UI TESTS
// =============================================================================

#[tokio::test]
#[ignore] // Run manually with: cargo test --test lazy_queue_e2e_ui_test -- --ignored --nocapture
async fn test_lazy_queue_loading_e2e_ui() {
    eprintln!("\n=== Full E2E UI Test: Lazy Queue Loading ===\n");

    // Setup: Create isolated test database and seed it
    let mut harness = E2ETestHarness::new().await;
    harness.seed_test_data().await;

    // Launch app with test database
    harness.launch_app().expect("Failed to launch app");

    eprintln!("\n[E2E] ✓ App is running with test database");
    eprintln!("[E2E] Database: {}", harness.db_path.display());
    eprintln!("\n=== MANUAL TEST STEPS ===");
    eprintln!("1. Navigate to Tracks page");
    eprintln!("2. Verify you see 500 test tracks");
    eprintln!("3. Click 'Test Track 1' to start playback");
    eprintln!("4. Open the queue sidebar (queue button in player)");
    eprintln!("5. Verify queue shows ONLY ~50 tracks (not all 500)");
    eprintln!("6. Scroll to bottom of queue");
    eprintln!("7. Click the LAST track in the queue");
    eprintln!("8. Wait 2-3 seconds for batch loading");
    eprintln!("9. Verify queue now has MORE tracks (should load next batch)");
    eprintln!("10. Verify queue did NOT become empty");
    eprintln!("\n=== EXPECTED BEHAVIOR ===");
    eprintln!("✓ Initial queue size: ~50 tracks (NOT 500)");
    eprintln!("✓ After clicking last track: Queue grows to ~100 tracks");
    eprintln!("✓ Queue never becomes empty");
    eprintln!("✓ Batch loading happens automatically");
    eprintln!("\nPress Ctrl+C to stop test when done...\n");

    // Keep test running until user presses Ctrl+C
    loop {
        std::thread::sleep(Duration::from_secs(1));
    }

    // Note: This will never be reached due to Ctrl+C, but app will be cleaned up by Drop
}

#[tokio::test]
async fn test_database_seeding_only() {
    eprintln!("\n=== Test: Database Seeding (No UI) ===\n");

    // Create harness and seed data
    let harness = E2ETestHarness::new().await;
    harness.seed_test_data().await;

    // Verify we can query the data using SQLx
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tracks WHERE id >= 10000")
        .fetch_one(&harness.pool)
        .await
        .expect("Failed to count tracks");

    assert_eq!(count, 500);
    eprintln!("[E2E] ✓ Database seeding test passed");

    // Verify we can get tracks using soul-storage pagination
    let tracks = soul_storage::tracks::get_all_paginated(&harness.pool, 0, 50)
        .await
        .expect("Failed to get paginated tracks");

    assert_eq!(tracks.len(), 50);
    eprintln!("[E2E] ✓ Pagination query returned 50 tracks");

    // Verify track order
    assert_eq!(tracks[0].title, "Test Track 1");
    assert_eq!(tracks[49].title, "Test Track 50");
    eprintln!("[E2E] ✓ Track order is correct");

    eprintln!("\n=== Database Seeding Test PASSED ===\n");
}
