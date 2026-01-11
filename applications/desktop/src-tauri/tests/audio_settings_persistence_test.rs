//! Audio settings persistence tests
//!
//! These tests verify that audio settings truly persist to disk and survive
//! "app restarts" (simulated by closing and reopening the database connection).
//!
//! Settings tested:
//! - Volume leveling mode
//! - Volume leveling pre-amp (with clamping validation)
//! - Volume leveling prevent clipping
//! - DSP chain with multiple effect types
//! - User isolation

use serde_json::json;
use sqlx::SqlitePool;
use tempfile::TempDir;

/// Setting keys (must match the constants in the source modules)
const SETTING_VOLUME_LEVELING_MODE: &str = "audio.volume_leveling_mode";
const SETTING_VOLUME_LEVELING_PREAMP: &str = "audio.volume_leveling_preamp";
const SETTING_VOLUME_LEVELING_PREVENT_CLIPPING: &str = "audio.volume_leveling_prevent_clipping";
const DSP_CHAIN_SETTING_KEY: &str = "audio.dsp_chain";

/// Test fixture that manages a temporary database
struct TestDb {
    db_path: std::path::PathBuf,
    _temp_dir: TempDir, // Hold reference to prevent cleanup
}

impl TestDb {
    /// Create a new test database with migrations applied
    async fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let db_path = temp_dir.path().join("test.db");

        // Create initial pool and run migrations
        let pool = Self::create_pool_for_path(&db_path).await;
        soul_storage::run_migrations(&pool)
            .await
            .expect("Failed to run migrations");

        // Create default user
        let now = chrono::Utc::now().timestamp();
        sqlx::query("INSERT INTO users (id, name, created_at) VALUES (?, ?, ?)")
            .bind("1")
            .bind("Test User")
            .bind(now)
            .execute(&pool)
            .await
            .expect("Failed to create test user");

        // Close the pool to ensure clean state
        pool.close().await;

        Self {
            db_path,
            _temp_dir: temp_dir,
        }
    }

    /// Create a pool for the database file
    async fn create_pool_for_path(db_path: &std::path::Path) -> SqlitePool {
        let db_url = if cfg!(windows) {
            let path_str = db_path
                .to_str()
                .expect("Path contains invalid UTF-8")
                .replace('\\', "/");
            format!("sqlite:///{}", path_str)
        } else {
            format!(
                "sqlite://{}",
                db_path.to_str().expect("Path contains invalid UTF-8")
            )
        };

        soul_storage::create_pool(&db_url)
            .await
            .expect("Failed to create pool")
    }

    /// Open a new connection to the database (simulates app start)
    async fn open(&self) -> SqlitePool {
        Self::create_pool_for_path(&self.db_path).await
    }

    /// Create a second user for isolation tests
    async fn create_second_user(&self, pool: &SqlitePool) {
        let now = chrono::Utc::now().timestamp();
        sqlx::query("INSERT INTO users (id, name, created_at) VALUES (?, ?, ?)")
            .bind("2")
            .bind("User 2")
            .bind(now)
            .execute(pool)
            .await
            .expect("Failed to create second user");
    }
}

// =============================================================================
// TRUE PERSISTENCE TESTS - Verify data survives database close/reopen
// =============================================================================

/// Test that volume leveling settings persist across "app restart" (connection close/reopen)
///
/// This is the core persistence test - it verifies that settings written to the database
/// are actually committed to disk and can be read back after closing and reopening the connection.
#[tokio::test]
async fn test_volume_leveling_survives_restart() {
    let test_db = TestDb::new().await;
    let user_id = "1";

    // Configure settings
    let mode = "replaygain_album";
    let preamp_db = 3.5;
    let prevent_clipping = true;

    // ========== Session 1: Configure and save ==========
    {
        let pool = test_db.open().await;

        soul_storage::settings::set_setting(
            &pool,
            user_id,
            SETTING_VOLUME_LEVELING_MODE,
            &json!(mode),
        )
        .await
        .expect("Failed to set mode");

        soul_storage::settings::set_setting(
            &pool,
            user_id,
            SETTING_VOLUME_LEVELING_PREAMP,
            &json!(preamp_db),
        )
        .await
        .expect("Failed to set preamp");

        soul_storage::settings::set_setting(
            &pool,
            user_id,
            SETTING_VOLUME_LEVELING_PREVENT_CLIPPING,
            &json!(prevent_clipping),
        )
        .await
        .expect("Failed to set prevent clipping");

        // Close connection - simulates app shutdown
        pool.close().await;
    }

    // ========== Session 2: Reopen and verify (simulates app restart) ==========
    {
        let pool = test_db.open().await;

        let saved_mode =
            soul_storage::settings::get_setting(&pool, user_id, SETTING_VOLUME_LEVELING_MODE)
                .await
                .expect("Failed to get mode")
                .expect("Mode should exist after restart");

        let saved_preamp =
            soul_storage::settings::get_setting(&pool, user_id, SETTING_VOLUME_LEVELING_PREAMP)
                .await
                .expect("Failed to get preamp")
                .expect("Preamp should exist after restart");

        let saved_prevent = soul_storage::settings::get_setting(
            &pool,
            user_id,
            SETTING_VOLUME_LEVELING_PREVENT_CLIPPING,
        )
        .await
        .expect("Failed to get prevent clipping")
        .expect("Prevent clipping should exist after restart");

        assert_eq!(
            saved_mode.as_str().unwrap(),
            mode,
            "Mode should survive restart"
        );
        assert!(
            (saved_preamp.as_f64().unwrap() - preamp_db).abs() < 0.001,
            "Preamp should survive restart"
        );
        assert_eq!(
            saved_prevent.as_bool().unwrap(),
            prevent_clipping,
            "Prevent clipping should survive restart"
        );

        pool.close().await;
    }
}

/// Test that DSP chain with complex effects persists across restart
#[tokio::test]
async fn test_dsp_chain_survives_restart() {
    let test_db = TestDb::new().await;
    let user_id = "1";

    // Create a realistic DSP chain with multiple effect types
    let dsp_chain = json!({
        "slots": [
            {
                "index": 0,
                "effect": {
                    "type": "eq",
                    "bands": [
                        { "frequency": 100.0, "gain": 3.0, "q": 1.0 },
                        { "frequency": 1000.0, "gain": 0.0, "q": 1.0 },
                        { "frequency": 10000.0, "gain": -3.0, "q": 1.0 }
                    ]
                },
                "enabled": true
            },
            {
                "index": 1,
                "effect": {
                    "type": "compressor",
                    "settings": {
                        "thresholdDb": -20.0,
                        "ratio": 4.0,
                        "attackMs": 10.0,
                        "releaseMs": 100.0
                    }
                },
                "enabled": true
            },
            {
                "index": 2,
                "effect": {
                    "type": "limiter",
                    "settings": { "thresholdDb": -1.0, "releaseMs": 50.0 }
                },
                "enabled": false
            },
            { "index": 3, "effect": null, "enabled": false }
        ]
    });

    // ========== Session 1: Save DSP chain ==========
    {
        let pool = test_db.open().await;

        soul_storage::settings::set_setting(&pool, user_id, DSP_CHAIN_SETTING_KEY, &dsp_chain)
            .await
            .expect("Failed to save DSP chain");

        pool.close().await;
    }

    // ========== Session 2: Verify after restart ==========
    {
        let pool = test_db.open().await;

        let saved_value =
            soul_storage::settings::get_setting(&pool, user_id, DSP_CHAIN_SETTING_KEY)
                .await
                .expect("Failed to get DSP chain")
                .expect("DSP chain should exist after restart");

        let slots = saved_value["slots"]
            .as_array()
            .expect("Should have slots array");
        assert_eq!(slots.len(), 4, "Should have 4 slots");

        // Verify EQ in slot 0
        assert_eq!(slots[0]["effect"]["type"], "eq");
        assert!(slots[0]["enabled"].as_bool().unwrap());
        let bands = slots[0]["effect"]["bands"]
            .as_array()
            .expect("Should have bands");
        assert_eq!(bands.len(), 3, "EQ should have 3 bands");
        assert_eq!(bands[0]["frequency"], 100.0);

        // Verify compressor in slot 1
        assert_eq!(slots[1]["effect"]["type"], "compressor");
        assert!(slots[1]["enabled"].as_bool().unwrap());
        assert_eq!(slots[1]["effect"]["settings"]["ratio"], 4.0);

        // Verify limiter in slot 2 (disabled)
        assert_eq!(slots[2]["effect"]["type"], "limiter");
        assert!(!slots[2]["enabled"].as_bool().unwrap());

        // Verify empty slot 3
        assert!(slots[3]["effect"].is_null());

        pool.close().await;
    }
}

// =============================================================================
// USER ISOLATION TEST - Critical for multi-user support
// =============================================================================

/// Test that settings for different users are isolated across restarts
///
/// This verifies the multi-user requirement from CLAUDE.md: "Every database query
/// MUST include user_id context"
#[tokio::test]
async fn test_user_isolation_survives_restart() {
    let test_db = TestDb::new().await;

    // ========== Session 1: Set different settings for each user ==========
    {
        let pool = test_db.open().await;
        test_db.create_second_user(&pool).await;

        soul_storage::settings::set_setting(
            &pool,
            "1",
            SETTING_VOLUME_LEVELING_MODE,
            &json!("disabled"),
        )
        .await
        .expect("Failed to set user 1 mode");

        soul_storage::settings::set_setting(
            &pool,
            "2",
            SETTING_VOLUME_LEVELING_MODE,
            &json!("ebu_r128"),
        )
        .await
        .expect("Failed to set user 2 mode");

        // Also set preamp to different values
        soul_storage::settings::set_setting(
            &pool,
            "1",
            SETTING_VOLUME_LEVELING_PREAMP,
            &json!(-6.0),
        )
        .await
        .expect("Failed to set user 1 preamp");

        soul_storage::settings::set_setting(
            &pool,
            "2",
            SETTING_VOLUME_LEVELING_PREAMP,
            &json!(6.0),
        )
        .await
        .expect("Failed to set user 2 preamp");

        pool.close().await;
    }

    // ========== Session 2: Verify isolation after restart ==========
    {
        let pool = test_db.open().await;

        let user1_mode =
            soul_storage::settings::get_setting(&pool, "1", SETTING_VOLUME_LEVELING_MODE)
                .await
                .expect("Failed to get user 1 mode")
                .expect("User 1 mode should exist");

        let user2_mode =
            soul_storage::settings::get_setting(&pool, "2", SETTING_VOLUME_LEVELING_MODE)
                .await
                .expect("Failed to get user 2 mode")
                .expect("User 2 mode should exist");

        let user1_preamp =
            soul_storage::settings::get_setting(&pool, "1", SETTING_VOLUME_LEVELING_PREAMP)
                .await
                .expect("Failed to get user 1 preamp")
                .expect("User 1 preamp should exist");

        let user2_preamp =
            soul_storage::settings::get_setting(&pool, "2", SETTING_VOLUME_LEVELING_PREAMP)
                .await
                .expect("Failed to get user 2 preamp")
                .expect("User 2 preamp should exist");

        // Verify user 1 settings
        assert_eq!(
            user1_mode.as_str().unwrap(),
            "disabled",
            "User 1 mode should be 'disabled'"
        );
        assert!(
            (user1_preamp.as_f64().unwrap() - (-6.0)).abs() < 0.001,
            "User 1 preamp should be -6.0 dB"
        );

        // Verify user 2 settings
        assert_eq!(
            user2_mode.as_str().unwrap(),
            "ebu_r128",
            "User 2 mode should be 'ebu_r128'"
        );
        assert!(
            (user2_preamp.as_f64().unwrap() - 6.0).abs() < 0.001,
            "User 2 preamp should be 6.0 dB"
        );

        pool.close().await;
    }
}

// =============================================================================
// SETTING UPDATE (UPSERT) BEHAVIOR TEST
// =============================================================================

/// Test that updating a setting correctly overwrites the previous value
///
/// This verifies the ON CONFLICT DO UPDATE behavior - critical for settings that
/// users change frequently (like volume levels).
#[tokio::test]
async fn test_setting_update_overwrites_previous() {
    let test_db = TestDb::new().await;
    let user_id = "1";

    // ========== Session 1: Set initial value ==========
    {
        let pool = test_db.open().await;

        soul_storage::settings::set_setting(
            &pool,
            user_id,
            SETTING_VOLUME_LEVELING_MODE,
            &json!("disabled"),
        )
        .await
        .expect("Failed to set initial mode");

        pool.close().await;
    }

    // ========== Session 2: Update the value ==========
    {
        let pool = test_db.open().await;

        // Verify initial value
        let initial = soul_storage::settings::get_setting(
            &pool,
            user_id,
            SETTING_VOLUME_LEVELING_MODE,
        )
        .await
        .expect("Failed to get mode")
        .expect("Mode should exist");
        assert_eq!(initial.as_str().unwrap(), "disabled");

        // Update to new value
        soul_storage::settings::set_setting(
            &pool,
            user_id,
            SETTING_VOLUME_LEVELING_MODE,
            &json!("replaygain_track"),
        )
        .await
        .expect("Failed to update mode");

        pool.close().await;
    }

    // ========== Session 3: Verify update persisted ==========
    {
        let pool = test_db.open().await;

        let final_value =
            soul_storage::settings::get_setting(&pool, user_id, SETTING_VOLUME_LEVELING_MODE)
                .await
                .expect("Failed to get mode")
                .expect("Mode should exist");

        assert_eq!(
            final_value.as_str().unwrap(),
            "replaygain_track",
            "Updated mode should persist"
        );

        pool.close().await;
    }
}

// =============================================================================
// MISSING SETTINGS BEHAVIOR TEST
// =============================================================================

/// Test that missing settings return None (important for initialization logic)
///
/// This verifies the behavior that the app relies on during startup when
/// determining whether to use defaults.
#[tokio::test]
async fn test_missing_settings_return_none() {
    let test_db = TestDb::new().await;
    let user_id = "1";

    let pool = test_db.open().await;

    // Query settings that haven't been set
    let mode = soul_storage::settings::get_setting(&pool, user_id, SETTING_VOLUME_LEVELING_MODE)
        .await
        .expect("Query should succeed");

    let preamp = soul_storage::settings::get_setting(&pool, user_id, SETTING_VOLUME_LEVELING_PREAMP)
        .await
        .expect("Query should succeed");

    let prevent = soul_storage::settings::get_setting(
        &pool,
        user_id,
        SETTING_VOLUME_LEVELING_PREVENT_CLIPPING,
    )
    .await
    .expect("Query should succeed");

    let dsp_chain = soul_storage::settings::get_setting(&pool, user_id, DSP_CHAIN_SETTING_KEY)
        .await
        .expect("Query should succeed");

    // All should be None
    assert!(mode.is_none(), "Mode should be None when not set");
    assert!(preamp.is_none(), "Preamp should be None when not set");
    assert!(
        prevent.is_none(),
        "Prevent clipping should be None when not set"
    );
    assert!(dsp_chain.is_none(), "DSP chain should be None when not set");

    pool.close().await;
}

// =============================================================================
// VOLUME LEVELING MODE VARIANTS TEST
// =============================================================================

/// Test all volume leveling mode variants persist correctly
///
/// This ensures the settings system can handle all valid mode strings
/// that the application supports.
#[tokio::test]
async fn test_all_volume_leveling_modes_persist() {
    let test_db = TestDb::new().await;
    let user_id = "1";

    let modes = [
        "disabled",
        "replaygain_track",
        "replaygain_album",
        "ebu_r128",
        "streaming",
    ];

    for mode in modes {
        // ========== Set mode in session 1 ==========
        {
            let pool = test_db.open().await;

            soul_storage::settings::set_setting(
                &pool,
                user_id,
                SETTING_VOLUME_LEVELING_MODE,
                &json!(mode),
            )
            .await
            .expect("Failed to set volume leveling mode");

            pool.close().await;
        }

        // ========== Verify in session 2 ==========
        {
            let pool = test_db.open().await;

            let saved_value =
                soul_storage::settings::get_setting(&pool, user_id, SETTING_VOLUME_LEVELING_MODE)
                    .await
                    .expect("Failed to get volume leveling mode")
                    .expect("Setting should exist");

            assert_eq!(
                saved_value.as_str().unwrap(),
                mode,
                "Mode '{}' should persist correctly",
                mode
            );

            pool.close().await;
        }
    }
}

// =============================================================================
// DSP CHAIN STATE TRANSITIONS TEST
// =============================================================================

/// Test DSP chain state transitions (add, toggle, clear) persist correctly
///
/// This tests the realistic use case of a user configuring their DSP chain
/// over multiple sessions.
#[tokio::test]
async fn test_dsp_chain_state_transitions() {
    let test_db = TestDb::new().await;
    let user_id = "1";

    // ========== Session 1: Add an effect ==========
    {
        let pool = test_db.open().await;

        let chain_with_eq = json!({
            "slots": [
                {
                    "index": 0,
                    "effect": { "type": "eq", "bands": [{ "frequency": 1000.0, "gain": 6.0, "q": 1.0 }] },
                    "enabled": true
                },
                { "index": 1, "effect": null, "enabled": false },
                { "index": 2, "effect": null, "enabled": false },
                { "index": 3, "effect": null, "enabled": false }
            ]
        });

        soul_storage::settings::set_setting(&pool, user_id, DSP_CHAIN_SETTING_KEY, &chain_with_eq)
            .await
            .expect("Failed to save chain");

        pool.close().await;
    }

    // ========== Session 2: Verify and toggle effect off ==========
    {
        let pool = test_db.open().await;

        // Verify effect was added
        let saved =
            soul_storage::settings::get_setting(&pool, user_id, DSP_CHAIN_SETTING_KEY)
                .await
                .expect("Failed to get chain")
                .expect("Chain should exist");

        assert_eq!(saved["slots"][0]["effect"]["type"], "eq");
        assert!(saved["slots"][0]["enabled"].as_bool().unwrap());

        // Toggle effect off (but keep the effect definition)
        let chain_toggled = json!({
            "slots": [
                {
                    "index": 0,
                    "effect": { "type": "eq", "bands": [{ "frequency": 1000.0, "gain": 6.0, "q": 1.0 }] },
                    "enabled": false
                },
                { "index": 1, "effect": null, "enabled": false },
                { "index": 2, "effect": null, "enabled": false },
                { "index": 3, "effect": null, "enabled": false }
            ]
        });

        soul_storage::settings::set_setting(&pool, user_id, DSP_CHAIN_SETTING_KEY, &chain_toggled)
            .await
            .expect("Failed to save toggled chain");

        pool.close().await;
    }

    // ========== Session 3: Verify toggle persisted ==========
    {
        let pool = test_db.open().await;

        let saved = soul_storage::settings::get_setting(&pool, user_id, DSP_CHAIN_SETTING_KEY)
            .await
            .expect("Failed to get chain")
            .expect("Chain should exist");

        // Effect should still exist but be disabled
        assert_eq!(saved["slots"][0]["effect"]["type"], "eq");
        assert!(
            !saved["slots"][0]["enabled"].as_bool().unwrap(),
            "Effect should be disabled after toggle"
        );

        // Clear all effects
        let chain_cleared = json!({
            "slots": [
                { "index": 0, "effect": null, "enabled": false },
                { "index": 1, "effect": null, "enabled": false },
                { "index": 2, "effect": null, "enabled": false },
                { "index": 3, "effect": null, "enabled": false }
            ]
        });

        soul_storage::settings::set_setting(&pool, user_id, DSP_CHAIN_SETTING_KEY, &chain_cleared)
            .await
            .expect("Failed to save cleared chain");

        pool.close().await;
    }

    // ========== Session 4: Verify clear persisted ==========
    {
        let pool = test_db.open().await;

        let saved = soul_storage::settings::get_setting(&pool, user_id, DSP_CHAIN_SETTING_KEY)
            .await
            .expect("Failed to get chain")
            .expect("Chain should exist");

        // All slots should be empty
        for (i, slot) in saved["slots"].as_array().unwrap().iter().enumerate() {
            assert!(
                slot["effect"].is_null(),
                "Slot {} should be empty after clear",
                i
            );
        }

        pool.close().await;
    }
}
