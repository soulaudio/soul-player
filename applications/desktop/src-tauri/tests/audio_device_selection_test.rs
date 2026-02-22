//! Audio device selection persistence tests
//!
//! Verifies that device selection (backend + device name) persists to the database
//! and survives simulated app restarts (connection close/reopen), and is correctly
//! isolated per user.
//!
//! Uses the same `soul_storage::settings` helpers as the Tauri commands so the
//! test exercises the same code path as production.

use serde_json::json;
use sqlx::SqlitePool;
use tempfile::TempDir;

const SETTING_OUTPUT_DEVICE: &str = "audio.output_device";

// =============================================================================
// Test fixture
// =============================================================================

struct TestDb {
    db_path: std::path::PathBuf,
    _temp_dir: TempDir, // Hold reference to prevent cleanup
}

impl TestDb {
    async fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let db_path = temp_dir.path().join("test.db");

        let pool = Self::create_pool(&db_path).await;
        soul_storage::run_migrations(&pool)
            .await
            .expect("Failed to run migrations");

        let now = chrono::Utc::now().timestamp();
        sqlx::query("INSERT INTO users (id, name, created_at) VALUES (?, ?, ?)")
            .bind("1")
            .bind("Test User")
            .bind(now)
            .execute(&pool)
            .await
            .expect("Failed to create test user");

        pool.close().await;
        Self {
            db_path,
            _temp_dir: temp_dir,
        }
    }

    async fn create_pool(db_path: &std::path::Path) -> SqlitePool {
        let db_url = if cfg!(windows) {
            format!("sqlite:///{}", db_path.to_str().unwrap().replace('\\', "/"))
        } else {
            format!("sqlite://{}", db_path.to_str().unwrap())
        };
        soul_storage::create_pool(&db_url)
            .await
            .expect("Failed to create pool")
    }

    /// Open a new connection (simulates app restart)
    async fn open(&self) -> SqlitePool {
        Self::create_pool(&self.db_path).await
    }

    async fn add_user(&self, pool: &SqlitePool, user_id: &str) {
        let now = chrono::Utc::now().timestamp();
        sqlx::query("INSERT INTO users (id, name, created_at) VALUES (?, ?, ?)")
            .bind(user_id)
            .bind(format!("User {}", user_id))
            .bind(now)
            .execute(pool)
            .await
            .expect("Failed to create user");
    }
}

// =============================================================================
// Tests
// =============================================================================

/// WASAPI (default) device selection persists across a simulated app restart.
#[tokio::test]
async fn test_wasapi_device_persists_across_restart() {
    let db = TestDb::new().await;
    let user_id = "1";

    // Session 1: save WASAPI device
    {
        let pool = db.open().await;
        soul_storage::settings::set_setting(
            &pool,
            user_id,
            SETTING_OUTPUT_DEVICE,
            &json!({ "backend": "default", "device_name": "Speakers (Realtek Audio)" }),
        )
        .await
        .expect("Failed to save device");
        pool.close().await;
    }

    // Session 2: verify it survived the restart
    {
        let pool = db.open().await;
        let saved = soul_storage::settings::get_setting(&pool, user_id, SETTING_OUTPUT_DEVICE)
            .await
            .expect("Failed to get setting")
            .expect("Setting should exist after restart");

        assert_eq!(
            saved["backend"].as_str().unwrap(),
            "default",
            "Backend should be 'default' (WASAPI)"
        );
        assert_eq!(
            saved["device_name"].as_str().unwrap(),
            "Speakers (Realtek Audio)",
            "Device name should survive restart"
        );
        pool.close().await;
    }
}

/// ASIO device selection persists across a simulated app restart.
#[tokio::test]
async fn test_asio_device_persists_across_restart() {
    let db = TestDb::new().await;
    let user_id = "1";

    // Session 1: save ASIO device
    {
        let pool = db.open().await;
        soul_storage::settings::set_setting(
            &pool,
            user_id,
            SETTING_OUTPUT_DEVICE,
            &json!({ "backend": "asio", "device_name": "Focusrite USB ASIO" }),
        )
        .await
        .expect("Failed to save ASIO device");
        pool.close().await;
    }

    // Session 2: verify ASIO backend + device name both survived
    {
        let pool = db.open().await;
        let saved = soul_storage::settings::get_setting(&pool, user_id, SETTING_OUTPUT_DEVICE)
            .await
            .expect("Failed to get setting")
            .expect("ASIO setting should exist after restart");

        assert_eq!(
            saved["backend"].as_str().unwrap(),
            "asio",
            "Backend should remain 'asio' after restart"
        );
        assert_eq!(
            saved["device_name"].as_str().unwrap(),
            "Focusrite USB ASIO",
            "ASIO device name should survive restart"
        );
        pool.close().await;
    }
}

/// Switching from WASAPI to ASIO stores only the new setting (upsert, not append).
#[tokio::test]
async fn test_device_switch_overwrites_previous() {
    let db = TestDb::new().await;
    let user_id = "1";

    {
        let pool = db.open().await;

        // Start on WASAPI
        soul_storage::settings::set_setting(
            &pool,
            user_id,
            SETTING_OUTPUT_DEVICE,
            &json!({ "backend": "default", "device_name": "Speakers" }),
        )
        .await
        .expect("Failed to save initial device");

        // Switch to ASIO — must overwrite, not append
        soul_storage::settings::set_setting(
            &pool,
            user_id,
            SETTING_OUTPUT_DEVICE,
            &json!({ "backend": "asio", "device_name": "ASIO4ALL v2" }),
        )
        .await
        .expect("Failed to save ASIO device");

        pool.close().await;
    }

    {
        let pool = db.open().await;
        let saved = soul_storage::settings::get_setting(&pool, user_id, SETTING_OUTPUT_DEVICE)
            .await
            .expect("Failed to get setting")
            .expect("Setting should exist after switch");

        // Should only have the ASIO setting (upsert behavior)
        assert_eq!(
            saved["backend"].as_str().unwrap(),
            "asio",
            "Should have ASIO backend after switch"
        );
        assert_eq!(
            saved["device_name"].as_str().unwrap(),
            "ASIO4ALL v2",
            "Should have ASIO device name after switch"
        );
        pool.close().await;
    }
}

/// Two users have independent device settings — changing one does not affect the other.
#[tokio::test]
async fn test_multi_user_device_isolation() {
    let db = TestDb::new().await;

    {
        let pool = db.open().await;
        db.add_user(&pool, "2").await;

        // User 1 uses WASAPI Speakers
        soul_storage::settings::set_setting(
            &pool,
            "1",
            SETTING_OUTPUT_DEVICE,
            &json!({ "backend": "default", "device_name": "Speakers" }),
        )
        .await
        .expect("Failed to save user 1 device");

        // User 2 uses ASIO
        soul_storage::settings::set_setting(
            &pool,
            "2",
            SETTING_OUTPUT_DEVICE,
            &json!({ "backend": "asio", "device_name": "ASIO4ALL v2" }),
        )
        .await
        .expect("Failed to save user 2 device");

        pool.close().await;
    }

    {
        let pool = db.open().await;

        let user1 = soul_storage::settings::get_setting(&pool, "1", SETTING_OUTPUT_DEVICE)
            .await
            .expect("DB error for user 1")
            .expect("User 1 setting should exist");
        let user2 = soul_storage::settings::get_setting(&pool, "2", SETTING_OUTPUT_DEVICE)
            .await
            .expect("DB error for user 2")
            .expect("User 2 setting should exist");

        assert_eq!(
            user1["backend"].as_str().unwrap(),
            "default",
            "User 1 should have WASAPI (default)"
        );
        assert_eq!(
            user2["backend"].as_str().unwrap(),
            "asio",
            "User 2 should have ASIO"
        );
        assert_ne!(
            user1["device_name"].as_str().unwrap(),
            user2["device_name"].as_str().unwrap(),
            "Users should have different device names — isolation broken"
        );

        pool.close().await;
    }
}

/// When no device has been saved, get_setting returns None (not an error).
#[tokio::test]
async fn test_no_saved_device_returns_none() {
    let db = TestDb::new().await;
    let pool = db.open().await;

    let result = soul_storage::settings::get_setting(&pool, "1", SETTING_OUTPUT_DEVICE)
        .await
        .expect("DB query should not fail");

    assert!(
        result.is_none(),
        "Missing device setting should return None, not an error or default"
    );

    pool.close().await;
}
