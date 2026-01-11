//! Integration tests for audio settings persistence
//!
//! Tests meaningful behavior of audio-related settings including:
//! - Settings persistence roundtrip (set/get)
//! - Update behavior (upsert)
//! - Multi-user isolation
//! - Edge cases (empty, large values, unicode)
//! - Error paths

use soul_storage::{create_pool, run_migrations, settings};

/// Setting keys matching the desktop application
const SETTING_VOLUME_LEVELING_MODE: &str = "audio.volume_leveling_mode";
const SETTING_VOLUME_LEVELING_PREAMP: &str = "audio.volume_leveling_preamp";
const SETTING_VOLUME_LEVELING_PREVENT_CLIPPING: &str = "audio.volume_leveling_prevent_clipping";
const SETTING_DSP_CHAIN: &str = "audio.dsp_chain";

/// Desktop default user ID
const USER_ID: &str = "1";

/// Create a test user in the database
async fn create_test_user(pool: &sqlx::SqlitePool, user_id: &str) {
    sqlx::query("INSERT INTO users (id, name, created_at) VALUES (?, 'Test User', 1234567890)")
        .bind(user_id)
        .execute(pool)
        .await
        .expect("Failed to create test user");
}

/// Helper to setup a test pool with migrations and a user
async fn setup_test_pool() -> sqlx::SqlitePool {
    let pool = create_pool("sqlite::memory:").await.unwrap();
    run_migrations(&pool).await.unwrap();
    create_test_user(&pool, USER_ID).await;
    pool
}

// ============================================================================
// Core Settings Behavior Tests
// ============================================================================

#[tokio::test]
async fn test_setting_roundtrip_string_value() {
    let pool = setup_test_pool().await;

    // Test that string values roundtrip correctly
    settings::set_setting(
        &pool,
        USER_ID,
        SETTING_VOLUME_LEVELING_MODE,
        &serde_json::json!("replaygain_album"),
    )
    .await
    .unwrap();

    let mode = settings::get_setting(&pool, USER_ID, SETTING_VOLUME_LEVELING_MODE)
        .await
        .unwrap();

    assert_eq!(mode, Some(serde_json::json!("replaygain_album")));
}

#[tokio::test]
async fn test_setting_roundtrip_numeric_value() {
    let pool = setup_test_pool().await;

    // Test positive, negative, and zero values
    let test_values = [0.0f64, 6.5, -3.0, 12.0, -12.0];

    for value in test_values {
        settings::set_setting(
            &pool,
            USER_ID,
            SETTING_VOLUME_LEVELING_PREAMP,
            &serde_json::json!(value),
        )
        .await
        .unwrap();

        let result = settings::get_setting(&pool, USER_ID, SETTING_VOLUME_LEVELING_PREAMP)
            .await
            .unwrap();

        assert_eq!(
            result,
            Some(serde_json::json!(value)),
            "Failed for value: {}",
            value
        );
    }
}

#[tokio::test]
async fn test_setting_roundtrip_boolean_value() {
    let pool = setup_test_pool().await;

    for value in [true, false] {
        settings::set_setting(
            &pool,
            USER_ID,
            SETTING_VOLUME_LEVELING_PREVENT_CLIPPING,
            &serde_json::json!(value),
        )
        .await
        .unwrap();

        let result = settings::get_setting(&pool, USER_ID, SETTING_VOLUME_LEVELING_PREVENT_CLIPPING)
            .await
            .unwrap();

        assert_eq!(
            result,
            Some(serde_json::json!(value)),
            "Failed for value: {}",
            value
        );
    }
}

#[tokio::test]
async fn test_setting_update_overwrites_previous_value() {
    let pool = setup_test_pool().await;

    // Set initial value
    settings::set_setting(
        &pool,
        USER_ID,
        SETTING_VOLUME_LEVELING_MODE,
        &serde_json::json!("disabled"),
    )
    .await
    .unwrap();

    // Update to different value
    settings::set_setting(
        &pool,
        USER_ID,
        SETTING_VOLUME_LEVELING_MODE,
        &serde_json::json!("ebu_r128"),
    )
    .await
    .unwrap();

    // Verify only the new value is present
    let mode = settings::get_setting(&pool, USER_ID, SETTING_VOLUME_LEVELING_MODE)
        .await
        .unwrap();

    assert_eq!(mode, Some(serde_json::json!("ebu_r128")));

    // Verify there's only one setting (not duplicated)
    let all_settings = settings::get_all_settings(&pool, USER_ID).await.unwrap();
    let mode_count = all_settings
        .iter()
        .filter(|s| s.key == SETTING_VOLUME_LEVELING_MODE)
        .count();
    assert_eq!(mode_count, 1, "Setting should not be duplicated on update");
}

#[tokio::test]
async fn test_get_nonexistent_setting_returns_none() {
    let pool = setup_test_pool().await;

    // Query setting that was never set
    let mode = settings::get_setting(&pool, USER_ID, SETTING_VOLUME_LEVELING_MODE)
        .await
        .unwrap();

    assert_eq!(mode, None);
}

// ============================================================================
// Complex JSON Structure Tests
// ============================================================================

#[tokio::test]
async fn test_dsp_chain_complex_nested_structure() {
    let pool = setup_test_pool().await;

    // Full chain with multiple effects in order - tests complex nested JSON
    let chain = serde_json::json!([
        {
            "type": "eq",
            "bands": [
                {"frequency": 80.0, "gain": 2.0, "q": 0.8},
                {"frequency": 10000.0, "gain": -1.5, "q": 1.0}
            ]
        },
        {
            "type": "compressor",
            "settings": {
                "thresholdDb": -20.0,
                "ratio": 3.0,
                "attackMs": 5.0,
                "releaseMs": 80.0,
                "kneeDb": 4.0,
                "makeupGainDb": 2.0
            }
        },
        {
            "type": "crossfeed",
            "settings": {
                "preset": "light",
                "level": 0.2,
                "delay": 0.2
            }
        },
        {
            "type": "limiter",
            "settings": {
                "thresholdDb": -0.5,
                "releaseMs": 30.0,
                "lookaheadMs": 3.0
            }
        }
    ]);

    settings::set_setting(&pool, USER_ID, SETTING_DSP_CHAIN, &chain)
        .await
        .unwrap();

    let result = settings::get_setting(&pool, USER_ID, SETTING_DSP_CHAIN)
        .await
        .unwrap();

    assert_eq!(result, Some(chain));
}

#[tokio::test]
async fn test_dsp_chain_update_replaces_entirely() {
    let pool = setup_test_pool().await;

    // Set initial chain with EQ
    let initial_chain = serde_json::json!([{
        "type": "eq",
        "bands": [{"frequency": 100.0, "gain": 3.0, "q": 1.0}]
    }]);

    settings::set_setting(&pool, USER_ID, SETTING_DSP_CHAIN, &initial_chain)
        .await
        .unwrap();

    // Replace with compressor only - should not merge, should replace
    let new_chain = serde_json::json!([{
        "type": "compressor",
        "settings": {
            "thresholdDb": -10.0,
            "ratio": 2.0,
            "attackMs": 20.0,
            "releaseMs": 200.0,
            "kneeDb": 3.0,
            "makeupGainDb": 1.0
        }
    }]);

    settings::set_setting(&pool, USER_ID, SETTING_DSP_CHAIN, &new_chain)
        .await
        .unwrap();

    let result = settings::get_setting(&pool, USER_ID, SETTING_DSP_CHAIN)
        .await
        .unwrap();

    assert_eq!(result, Some(new_chain));

    // Verify initial chain is completely gone (no trace of "eq")
    let result_str = result.unwrap().to_string();
    assert!(
        !result_str.contains("\"type\":\"eq\""),
        "Old chain should be completely replaced"
    );
}

// ============================================================================
// Multiple Settings Tests
// ============================================================================

#[tokio::test]
async fn test_multiple_settings_coexist_independently() {
    let pool = setup_test_pool().await;

    // Set all audio settings
    settings::set_setting(
        &pool,
        USER_ID,
        SETTING_VOLUME_LEVELING_MODE,
        &serde_json::json!("replaygain_album"),
    )
    .await
    .unwrap();

    settings::set_setting(
        &pool,
        USER_ID,
        SETTING_VOLUME_LEVELING_PREAMP,
        &serde_json::json!(3.5),
    )
    .await
    .unwrap();

    settings::set_setting(
        &pool,
        USER_ID,
        SETTING_VOLUME_LEVELING_PREVENT_CLIPPING,
        &serde_json::json!(true),
    )
    .await
    .unwrap();

    let dsp_chain = serde_json::json!([{
        "type": "eq",
        "bands": [{"frequency": 60.0, "gain": 4.0, "q": 0.7}]
    }]);
    settings::set_setting(&pool, USER_ID, SETTING_DSP_CHAIN, &dsp_chain)
        .await
        .unwrap();

    // Verify all settings independently exist and have correct values
    let mode = settings::get_setting(&pool, USER_ID, SETTING_VOLUME_LEVELING_MODE)
        .await
        .unwrap();
    let preamp = settings::get_setting(&pool, USER_ID, SETTING_VOLUME_LEVELING_PREAMP)
        .await
        .unwrap();
    let prevent = settings::get_setting(&pool, USER_ID, SETTING_VOLUME_LEVELING_PREVENT_CLIPPING)
        .await
        .unwrap();
    let chain = settings::get_setting(&pool, USER_ID, SETTING_DSP_CHAIN)
        .await
        .unwrap();

    assert_eq!(mode, Some(serde_json::json!("replaygain_album")));
    assert_eq!(preamp, Some(serde_json::json!(3.5)));
    assert_eq!(prevent, Some(serde_json::json!(true)));
    assert_eq!(chain, Some(dsp_chain));
}

#[tokio::test]
async fn test_get_all_settings_returns_all_user_settings() {
    let pool = setup_test_pool().await;

    // Set multiple audio settings
    settings::set_setting(
        &pool,
        USER_ID,
        SETTING_VOLUME_LEVELING_MODE,
        &serde_json::json!("ebu_r128"),
    )
    .await
    .unwrap();

    settings::set_setting(
        &pool,
        USER_ID,
        SETTING_VOLUME_LEVELING_PREAMP,
        &serde_json::json!(-2.0),
    )
    .await
    .unwrap();

    // Get all settings and verify all are included
    let all_settings = settings::get_all_settings(&pool, USER_ID).await.unwrap();

    assert_eq!(all_settings.len(), 2);
    assert!(all_settings
        .iter()
        .any(|s| s.key == SETTING_VOLUME_LEVELING_MODE));
    assert!(all_settings
        .iter()
        .any(|s| s.key == SETTING_VOLUME_LEVELING_PREAMP));
}

// ============================================================================
// Delete Setting Tests
// ============================================================================

#[tokio::test]
async fn test_delete_setting_removes_it() {
    let pool = setup_test_pool().await;

    // Set a setting
    settings::set_setting(
        &pool,
        USER_ID,
        SETTING_VOLUME_LEVELING_MODE,
        &serde_json::json!("streaming"),
    )
    .await
    .unwrap();

    // Delete it
    let deleted = settings::delete_setting(&pool, USER_ID, SETTING_VOLUME_LEVELING_MODE)
        .await
        .unwrap();
    assert!(deleted, "delete_setting should return true when setting exists");

    // Verify it's gone
    let mode = settings::get_setting(&pool, USER_ID, SETTING_VOLUME_LEVELING_MODE)
        .await
        .unwrap();
    assert_eq!(mode, None, "Setting should be None after deletion");
}

#[tokio::test]
async fn test_delete_nonexistent_setting_returns_false() {
    let pool = setup_test_pool().await;

    // Try to delete setting that doesn't exist
    let deleted = settings::delete_setting(&pool, USER_ID, "nonexistent.setting.key")
        .await
        .unwrap();

    assert!(
        !deleted,
        "delete_setting should return false when setting doesn't exist"
    );
}

#[tokio::test]
async fn test_delete_only_affects_target_setting() {
    let pool = setup_test_pool().await;

    // Set multiple settings
    settings::set_setting(
        &pool,
        USER_ID,
        SETTING_VOLUME_LEVELING_MODE,
        &serde_json::json!("streaming"),
    )
    .await
    .unwrap();

    settings::set_setting(
        &pool,
        USER_ID,
        SETTING_VOLUME_LEVELING_PREAMP,
        &serde_json::json!(5.0),
    )
    .await
    .unwrap();

    // Delete only one
    settings::delete_setting(&pool, USER_ID, SETTING_VOLUME_LEVELING_MODE)
        .await
        .unwrap();

    // Verify other setting still exists
    let preamp = settings::get_setting(&pool, USER_ID, SETTING_VOLUME_LEVELING_PREAMP)
        .await
        .unwrap();
    assert_eq!(
        preamp,
        Some(serde_json::json!(5.0)),
        "Other settings should not be affected"
    );
}

// ============================================================================
// Multi-User Isolation Tests
// ============================================================================

#[tokio::test]
async fn test_settings_isolated_between_users() {
    let pool = create_pool("sqlite::memory:").await.unwrap();
    run_migrations(&pool).await.unwrap();

    // Create two users
    create_test_user(&pool, "1").await;
    create_test_user(&pool, "2").await;

    // Set different modes for each user
    settings::set_setting(
        &pool,
        "1",
        SETTING_VOLUME_LEVELING_MODE,
        &serde_json::json!("replaygain_track"),
    )
    .await
    .unwrap();

    settings::set_setting(
        &pool,
        "2",
        SETTING_VOLUME_LEVELING_MODE,
        &serde_json::json!("ebu_r128"),
    )
    .await
    .unwrap();

    // Verify isolation - each user sees only their own setting
    let user1_mode = settings::get_setting(&pool, "1", SETTING_VOLUME_LEVELING_MODE)
        .await
        .unwrap();
    let user2_mode = settings::get_setting(&pool, "2", SETTING_VOLUME_LEVELING_MODE)
        .await
        .unwrap();

    assert_eq!(user1_mode, Some(serde_json::json!("replaygain_track")));
    assert_eq!(user2_mode, Some(serde_json::json!("ebu_r128")));
}

#[tokio::test]
async fn test_delete_only_affects_own_user_settings() {
    let pool = create_pool("sqlite::memory:").await.unwrap();
    run_migrations(&pool).await.unwrap();

    // Create two users
    create_test_user(&pool, "1").await;
    create_test_user(&pool, "2").await;

    // Both users set the same key
    settings::set_setting(
        &pool,
        "1",
        SETTING_VOLUME_LEVELING_MODE,
        &serde_json::json!("mode_a"),
    )
    .await
    .unwrap();

    settings::set_setting(
        &pool,
        "2",
        SETTING_VOLUME_LEVELING_MODE,
        &serde_json::json!("mode_b"),
    )
    .await
    .unwrap();

    // User 1 deletes their setting
    settings::delete_setting(&pool, "1", SETTING_VOLUME_LEVELING_MODE)
        .await
        .unwrap();

    // User 2's setting should still exist
    let user2_mode = settings::get_setting(&pool, "2", SETTING_VOLUME_LEVELING_MODE)
        .await
        .unwrap();
    assert_eq!(
        user2_mode,
        Some(serde_json::json!("mode_b")),
        "Other user's setting should not be affected by delete"
    );
}

#[tokio::test]
async fn test_get_all_settings_only_returns_own_user_settings() {
    let pool = create_pool("sqlite::memory:").await.unwrap();
    run_migrations(&pool).await.unwrap();

    // Create two users
    create_test_user(&pool, "1").await;
    create_test_user(&pool, "2").await;

    // User 1 sets 2 settings
    settings::set_setting(
        &pool,
        "1",
        SETTING_VOLUME_LEVELING_MODE,
        &serde_json::json!("mode_1"),
    )
    .await
    .unwrap();

    settings::set_setting(&pool, "1", SETTING_VOLUME_LEVELING_PREAMP, &serde_json::json!(1.0))
        .await
        .unwrap();

    // User 2 sets 1 setting
    settings::set_setting(
        &pool,
        "2",
        SETTING_DSP_CHAIN,
        &serde_json::json!([{"type": "eq"}]),
    )
    .await
    .unwrap();

    // Get all for user 1 - should not include user 2's settings
    let user1_settings = settings::get_all_settings(&pool, "1").await.unwrap();
    assert_eq!(
        user1_settings.len(),
        2,
        "User 1 should only see their 2 settings"
    );

    // Get all for user 2 - should not include user 1's settings
    let user2_settings = settings::get_all_settings(&pool, "2").await.unwrap();
    assert_eq!(
        user2_settings.len(),
        1,
        "User 2 should only see their 1 setting"
    );
}

// ============================================================================
// Edge Cases Tests
// ============================================================================

#[tokio::test]
async fn test_empty_array_json_value() {
    let pool = setup_test_pool().await;

    let empty_chain = serde_json::json!([]);

    settings::set_setting(&pool, USER_ID, SETTING_DSP_CHAIN, &empty_chain)
        .await
        .unwrap();

    let chain = settings::get_setting(&pool, USER_ID, SETTING_DSP_CHAIN)
        .await
        .unwrap();

    assert_eq!(chain, Some(empty_chain));
}

#[tokio::test]
async fn test_empty_string_value() {
    let pool = setup_test_pool().await;

    settings::set_setting(
        &pool,
        USER_ID,
        SETTING_VOLUME_LEVELING_MODE,
        &serde_json::json!(""),
    )
    .await
    .unwrap();

    let mode = settings::get_setting(&pool, USER_ID, SETTING_VOLUME_LEVELING_MODE)
        .await
        .unwrap();

    assert_eq!(mode, Some(serde_json::json!("")));
}

#[tokio::test]
async fn test_unicode_value() {
    let pool = setup_test_pool().await;

    // Test various unicode including emoji, CJK, RTL
    let unicode_value = serde_json::json!("Volume: 音量 🔊 صوت");

    settings::set_setting(&pool, USER_ID, SETTING_VOLUME_LEVELING_MODE, &unicode_value)
        .await
        .unwrap();

    let mode = settings::get_setting(&pool, USER_ID, SETTING_VOLUME_LEVELING_MODE)
        .await
        .unwrap();

    assert_eq!(mode, Some(unicode_value));
}

#[tokio::test]
async fn test_null_json_value() {
    let pool = setup_test_pool().await;

    settings::set_setting(
        &pool,
        USER_ID,
        SETTING_VOLUME_LEVELING_MODE,
        &serde_json::Value::Null,
    )
    .await
    .unwrap();

    let mode = settings::get_setting(&pool, USER_ID, SETTING_VOLUME_LEVELING_MODE)
        .await
        .unwrap();

    assert_eq!(mode, Some(serde_json::Value::Null));
}

#[tokio::test]
async fn test_special_characters_in_key() {
    let pool = setup_test_pool().await;

    // Keys with dots, underscores, and dashes (common in namespaced settings)
    let key = "audio.dsp.eq_band-1.gain";

    settings::set_setting(&pool, USER_ID, key, &serde_json::json!(3.5))
        .await
        .unwrap();

    let value = settings::get_setting(&pool, USER_ID, key).await.unwrap();

    assert_eq!(value, Some(serde_json::json!(3.5)));
}

#[tokio::test]
async fn test_large_json_value() {
    let pool = setup_test_pool().await;

    // Create a large DSP chain with many bands (realistic 31-band graphic EQ)
    // Use integer values to avoid floating-point precision issues in comparison
    let mut bands = vec![];
    for i in 0..31 {
        bands.push(serde_json::json!({
            "frequency": 20 * (i + 1),  // Integer frequencies: 20, 40, 60, ...
            "gain": i as i32 - 15,       // Integer gains: -15 to +15
            "q": 1                        // Integer Q
        }));
    }

    let large_chain = serde_json::json!([{
        "type": "graphic_eq",
        "bands": bands
    }]);

    settings::set_setting(&pool, USER_ID, SETTING_DSP_CHAIN, &large_chain)
        .await
        .unwrap();

    let result = settings::get_setting(&pool, USER_ID, SETTING_DSP_CHAIN)
        .await
        .unwrap();

    assert_eq!(result, Some(large_chain));
}

#[tokio::test]
async fn test_deeply_nested_json_value() {
    let pool = setup_test_pool().await;

    // Create deeply nested structure
    let deep_json = serde_json::json!({
        "level1": {
            "level2": {
                "level3": {
                    "level4": {
                        "level5": {
                            "value": "deep"
                        }
                    }
                }
            }
        }
    });

    settings::set_setting(&pool, USER_ID, "test.deep.key", &deep_json)
        .await
        .unwrap();

    let result = settings::get_setting(&pool, USER_ID, "test.deep.key")
        .await
        .unwrap();

    assert_eq!(result, Some(deep_json));
}

// ============================================================================
// Error Path Tests
// ============================================================================

#[tokio::test]
async fn test_get_setting_for_nonexistent_user_returns_none() {
    let pool = setup_test_pool().await;

    // Query setting for user that doesn't exist in our test setup
    let result = settings::get_setting(&pool, "nonexistent_user_999", SETTING_VOLUME_LEVELING_MODE)
        .await
        .unwrap();

    // Should return None, not error (user doesn't need to exist to query their settings)
    assert_eq!(result, None);
}

#[tokio::test]
async fn test_get_all_settings_for_user_with_no_settings() {
    let pool = setup_test_pool().await;

    // User exists but has no settings
    let all_settings = settings::get_all_settings(&pool, USER_ID).await.unwrap();

    assert!(
        all_settings.is_empty(),
        "User with no settings should return empty vec"
    );
}
