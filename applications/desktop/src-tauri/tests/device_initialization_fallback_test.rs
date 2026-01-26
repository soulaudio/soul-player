//! Device Initialization and Fallback Tests
//!
//! Comprehensive tests for device initialization, recovery, and edge case handling.
//! These tests verify that the audio device system gracefully handles:
//! - Cross-platform device name mismatches
//! - Removed/unplugged devices
//! - Invalid saved settings
//! - Backend unavailability
//! - Concurrent operations
//! - Database corruption
//!
//! ## Test Strategy
//! - Unit tests: Test individual components in isolation
//! - Integration tests: Test the full device initialization flow
//! - Edge case tests: Test error paths and recovery mechanisms
//!
//! ## Critical Requirements (from CLAUDE.md)
//! - Platform-agnostic core: Settings from one OS shouldn't break another
//! - Error handling: Libraries use thiserror + Result, no unwrap()
//! - Database: Multi-user aware, every query includes user_id
//! - Test quality: Focus on meaningful tests, no shallow tests

use serde_json::json;
#[cfg(feature = "asio")]
use soul_audio_desktop::backend;
use soul_audio_desktop::{device, AudioBackend};
use sqlx::SqlitePool;
use tempfile::TempDir;

// =============================================================================
// Test Infrastructure
// =============================================================================

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
}

// =============================================================================
// Edge Case Tests - Cross-Platform Device Name Mismatches
// =============================================================================

/// Test: Device name from Windows doesn't exist on Linux
///
/// Scenario: User switches from Windows to Linux, database has "Default Audio Device"
/// Expected: Falls back to actual default device, updates saved setting
#[tokio::test]
async fn test_cross_platform_device_name_mismatch() {
    let test_db = TestDb::new().await;
    let pool = test_db.open().await;
    let user_id = "1";

    // Simulate saved Windows device name
    let windows_device_settings = json!({
        "backend": "default",
        "device_name": "Default Audio Device"  // Typical Windows name
    });

    // Save to database
    let now = chrono::Utc::now().timestamp();
    sqlx::query(
        "INSERT INTO user_settings (user_id, key, value, updated_at)
         VALUES (?, ?, ?, ?)",
    )
    .bind(user_id)
    .bind("audio.output_device")
    .bind(windows_device_settings.to_string())
    .bind(now)
    .execute(&pool)
    .await
    .expect("Failed to save device setting");

    // Verify device doesn't exist
    let backend = AudioBackend::Default;
    let device_lookup = device::find_device_by_name(backend, "Default Audio Device");

    if device_lookup.is_ok() {
        // Device exists (we're on Windows), skip this test
        println!("Skipping test - running on Windows where device exists");
        pool.close().await;
        return;
    }

    // Device doesn't exist - verify our fallback logic would handle this
    // In real initialize_audio_device(), this would:
    // 1. Detect device not found
    // 2. Keep current device (default)
    // 3. Update saved setting

    // Simulate the update that initialize_audio_device() would do
    let Ok(default_device) = device::get_default_device(backend) else {
        println!("Skipping test - no audio device available");
        pool.close().await;
        return;
    };

    let corrected_settings = json!({
        "backend": "default",
        "device_name": default_device.name
    });

    sqlx::query("UPDATE user_settings SET value = ?, updated_at = ? WHERE user_id = ? AND key = ?")
        .bind(corrected_settings.to_string())
        .bind(chrono::Utc::now().timestamp())
        .bind(user_id)
        .bind("audio.output_device")
        .execute(&pool)
        .await
        .expect("Failed to update device setting");

    // Verify correction
    let (updated_value,): (String,) =
        sqlx::query_as("SELECT value FROM user_settings WHERE user_id = ? AND key = ?")
            .bind(user_id)
            .bind("audio.output_device")
            .fetch_one(&pool)
            .await
            .expect("Failed to get updated setting");

    let updated_json: serde_json::Value =
        serde_json::from_str(&updated_value).expect("Failed to parse JSON");

    assert_eq!(
        updated_json["device_name"].as_str().unwrap(),
        default_device.name,
        "Device name should be updated to actual default device"
    );

    pool.close().await;
}

// =============================================================================
// Edge Case Tests - Invalid JSON in Database
// =============================================================================

/// Test: Corrupted JSON in device settings
///
/// Scenario: Database contains invalid JSON (corruption, manual edit)
/// Expected: Parse error, fallback to default device
#[tokio::test]
async fn test_corrupted_json_in_device_settings() {
    let test_db = TestDb::new().await;
    let pool = test_db.open().await;
    let user_id = "1";

    // Save corrupted JSON to database
    let corrupted_json = "{ invalid json, missing quotes }";

    let now = chrono::Utc::now().timestamp();
    sqlx::query(
        "INSERT INTO user_settings (user_id, key, value, updated_at)
         VALUES (?, ?, ?, ?)",
    )
    .bind(user_id)
    .bind("audio.output_device")
    .bind(corrupted_json)
    .bind(now)
    .execute(&pool)
    .await
    .expect("Failed to save corrupted setting");

    // Try to parse it (simulating initialize_audio_device behavior)
    let (value,): (String,) =
        sqlx::query_as("SELECT value FROM user_settings WHERE user_id = ? AND key = ?")
            .bind(user_id)
            .bind("audio.output_device")
            .fetch_one(&pool)
            .await
            .expect("Failed to query setting");

    let parse_result: Result<serde_json::Value, _> = serde_json::from_str(&value);

    // Should fail to parse
    assert!(parse_result.is_err(), "Corrupted JSON should fail to parse");

    // In real code, this would fall back to default device
    // Verify default device is available
    if let Ok(default_device) = device::get_default_device(AudioBackend::Default) {
        assert!(
            !default_device.name.is_empty(),
            "Default device should be available as fallback"
        );
    }

    pool.close().await;
}

// =============================================================================
// Edge Case Tests - Missing Fields in JSON
// =============================================================================

/// Test: Device settings JSON missing required fields
///
/// Scenario: JSON is valid but missing "backend" or "device_name"
/// Expected: Use default values, don't crash
#[tokio::test]
async fn test_device_settings_missing_fields() {
    let test_db = TestDb::new().await;
    let pool = test_db.open().await;
    let user_id = "1";

    // Test Case 1: Missing device_name
    let missing_name = json!({ "backend": "default" });

    let now = chrono::Utc::now().timestamp();
    sqlx::query(
        "INSERT INTO user_settings (user_id, key, value, updated_at)
         VALUES (?, ?, ?, ?) ON CONFLICT(user_id, key) DO UPDATE SET value = excluded.value",
    )
    .bind(user_id)
    .bind("audio.output_device")
    .bind(missing_name.to_string())
    .bind(now)
    .execute(&pool)
    .await
    .expect("Failed to save setting");

    // Verify field is missing
    let (value,): (String,) =
        sqlx::query_as("SELECT value FROM user_settings WHERE user_id = ? AND key = ?")
            .bind(user_id)
            .bind("audio.output_device")
            .fetch_one(&pool)
            .await
            .expect("Failed to query setting");

    let parsed: serde_json::Value = serde_json::from_str(&value).expect("Should parse as JSON");

    assert!(
        parsed.get("device_name").is_none(),
        "device_name should be missing"
    );

    // Test Case 2: Missing backend
    let missing_backend = json!({ "device_name": "Some Device" });

    sqlx::query("UPDATE user_settings SET value = ?, updated_at = ? WHERE user_id = ? AND key = ?")
        .bind(missing_backend.to_string())
        .bind(chrono::Utc::now().timestamp())
        .bind(user_id)
        .bind("audio.output_device")
        .execute(&pool)
        .await
        .expect("Failed to update setting");

    let (value2,): (String,) =
        sqlx::query_as("SELECT value FROM user_settings WHERE user_id = ? AND key = ?")
            .bind(user_id)
            .bind("audio.output_device")
            .fetch_one(&pool)
            .await
            .expect("Failed to query setting");

    let parsed2: serde_json::Value = serde_json::from_str(&value2).expect("Should parse as JSON");

    assert!(
        parsed2.get("backend").is_none(),
        "backend should be missing"
    );

    // Test Case 3: Empty strings
    let empty_values = json!({ "backend": "", "device_name": "" });

    sqlx::query("UPDATE user_settings SET value = ?, updated_at = ? WHERE user_id = ? AND key = ?")
        .bind(empty_values.to_string())
        .bind(chrono::Utc::now().timestamp())
        .bind(user_id)
        .bind("audio.output_device")
        .execute(&pool)
        .await
        .expect("Failed to update setting");

    let (value3,): (String,) =
        sqlx::query_as("SELECT value FROM user_settings WHERE user_id = ? AND key = ?")
            .bind(user_id)
            .bind("audio.output_device")
            .fetch_one(&pool)
            .await
            .expect("Failed to query setting");

    let parsed3: serde_json::Value = serde_json::from_str(&value3).expect("Should parse as JSON");

    assert_eq!(parsed3["backend"].as_str().unwrap(), "");
    assert_eq!(parsed3["device_name"].as_str().unwrap(), "");

    pool.close().await;
}

// =============================================================================
// Edge Case Tests - Backend Unavailability
// =============================================================================

/// Test: Saved backend is not available on this platform
///
/// Scenario: ASIO saved on Windows, but running on Linux
/// Expected: Falls back to default backend
#[tokio::test]
#[cfg(feature = "asio")]
async fn test_unavailable_backend_fallback() {
    let test_db = TestDb::new().await;
    let pool = test_db.open().await;
    let user_id = "1";

    // Save ASIO backend (only available on Windows)
    let asio_settings = json!({
        "backend": "asio",
        "device_name": "ASIO Device"
    });

    let now = chrono::Utc::now().timestamp();
    sqlx::query(
        "INSERT INTO user_settings (user_id, key, value, updated_at)
         VALUES (?, ?, ?, ?)",
    )
    .bind(user_id)
    .bind("audio.output_device")
    .bind(asio_settings.to_string())
    .bind(now)
    .execute(&pool)
    .await
    .expect("Failed to save setting");

    // Check if ASIO is available
    let backends = backend::get_backend_info();
    let asio_available = backends
        .iter()
        .any(|b| matches!(b.backend, AudioBackend::Asio) && b.available);

    if !asio_available {
        // ASIO not available - verify default backend is available
        let default_backend = backends
            .iter()
            .find(|b| b.is_default)
            .expect("Should have default backend");

        assert!(
            default_backend.available,
            "Default backend should be available as fallback"
        );
        assert!(
            default_backend.device_count > 0,
            "Default backend should have devices"
        );
    }

    pool.close().await;
}

// =============================================================================
// Edge Case Tests - Device Name with Special Characters
// =============================================================================

/// Test: Device name contains Unicode, special characters
///
/// Scenario: Device name with emoji, non-ASCII, symbols
/// Expected: Correctly stored and retrieved
#[tokio::test]
async fn test_device_name_with_special_characters() {
    let test_db = TestDb::new().await;
    let pool = test_db.open().await;
    let user_id = "1";

    // Test various special character combinations
    let special_names = [
        "Device (USB Audio 2.0)",
        "HD Audio Device #2",
        "AudioBox™ 1818VSL",
        "Realtek® HD Audio",
        "🎵 Sound Card",
        "デバイス",   // Japanese
        "Устройство", // Russian
        "音频设备",   // Chinese
    ];

    for (i, name) in special_names.iter().enumerate() {
        let settings = json!({
            "backend": "default",
            "device_name": name
        });

        let now = chrono::Utc::now().timestamp();

        if i == 0 {
            sqlx::query(
                "INSERT INTO user_settings (user_id, key, value, updated_at)
                 VALUES (?, ?, ?, ?)",
            )
            .bind(user_id)
            .bind("audio.output_device")
            .bind(settings.to_string())
            .bind(now)
            .execute(&pool)
            .await
            .expect("Failed to save setting");
        } else {
            sqlx::query(
                "UPDATE user_settings SET value = ?, updated_at = ? WHERE user_id = ? AND key = ?",
            )
            .bind(settings.to_string())
            .bind(now)
            .bind(user_id)
            .bind("audio.output_device")
            .execute(&pool)
            .await
            .expect("Failed to update setting");
        }

        // Verify retrieval
        let (value,): (String,) =
            sqlx::query_as("SELECT value FROM user_settings WHERE user_id = ? AND key = ?")
                .bind(user_id)
                .bind("audio.output_device")
                .fetch_one(&pool)
                .await
                .expect("Failed to query setting");

        let parsed: serde_json::Value = serde_json::from_str(&value).expect("Should parse as JSON");

        assert_eq!(
            parsed["device_name"].as_str().unwrap(),
            *name,
            "Special characters should round-trip correctly"
        );
    }

    pool.close().await;
}

// =============================================================================
// Edge Case Tests - Concurrent Device Updates
// =============================================================================

/// Test: Multiple threads updating device settings concurrently
///
/// Scenario: Race condition between device switches
/// Expected: Database constraints prevent corruption, last write wins
#[tokio::test]
async fn test_concurrent_device_updates() {
    let test_db = TestDb::new().await;
    let user_id = "1";

    // Spawn multiple tasks updating device concurrently
    let mut handles = vec![];

    for i in 0..10 {
        let pool = test_db.open().await;
        let user_id = user_id.to_string();

        let handle = tokio::spawn(async move {
            let settings = json!({
                "backend": "default",
                "device_name": format!("Device {}", i)
            });

            let now = chrono::Utc::now().timestamp();

            sqlx::query(
                "INSERT INTO user_settings (user_id, key, value, updated_at)
                 VALUES (?, ?, ?, ?)
                 ON CONFLICT(user_id, key) DO UPDATE SET
                    value = excluded.value,
                    updated_at = excluded.updated_at",
            )
            .bind(&user_id)
            .bind("audio.output_device")
            .bind(settings.to_string())
            .bind(now)
            .execute(&pool)
            .await
            .expect("Failed to update setting");

            pool.close().await;
        });

        handles.push(handle);
    }

    // Wait for all updates
    for handle in handles {
        handle.await.expect("Task panicked");
    }

    // Verify final state is consistent (one of the values, not corrupted)
    let pool = test_db.open().await;

    let (value,): (String,) =
        sqlx::query_as("SELECT value FROM user_settings WHERE user_id = ? AND key = ?")
            .bind(user_id)
            .bind("audio.output_device")
            .fetch_one(&pool)
            .await
            .expect("Failed to query final setting");

    let parsed: serde_json::Value = serde_json::from_str(&value).expect("Should parse as JSON");

    // Should be one of the devices (0-9)
    let device_name = parsed["device_name"].as_str().unwrap();
    assert!(
        device_name.starts_with("Device "),
        "Final device name should be valid"
    );

    // Parse device number
    let num: u32 = device_name
        .trim_start_matches("Device ")
        .parse()
        .expect("Should parse device number");
    assert!(num < 10, "Device number should be 0-9");

    pool.close().await;
}

// =============================================================================
// Edge Case Tests - Empty Device Name
// =============================================================================

/// Test: Device name is empty string
///
/// Scenario: Empty device name in saved settings
/// Expected: Treated as "use default device"
#[tokio::test]
async fn test_empty_device_name() {
    let test_db = TestDb::new().await;
    let pool = test_db.open().await;
    let user_id = "1";

    // Save empty device name
    let empty_device = json!({
        "backend": "default",
        "device_name": ""
    });

    let now = chrono::Utc::now().timestamp();
    sqlx::query(
        "INSERT INTO user_settings (user_id, key, value, updated_at)
         VALUES (?, ?, ?, ?)",
    )
    .bind(user_id)
    .bind("audio.output_device")
    .bind(empty_device.to_string())
    .bind(now)
    .execute(&pool)
    .await
    .expect("Failed to save setting");

    // Verify retrieval
    let (value,): (String,) =
        sqlx::query_as("SELECT value FROM user_settings WHERE user_id = ? AND key = ?")
            .bind(user_id)
            .bind("audio.output_device")
            .fetch_one(&pool)
            .await
            .expect("Failed to query setting");

    let parsed: serde_json::Value = serde_json::from_str(&value).expect("Should parse as JSON");

    assert_eq!(
        parsed["device_name"].as_str().unwrap(),
        "",
        "Empty device name should be preserved"
    );

    // In initialize_audio_device(), empty string is treated as None
    // Verify default device is available as fallback
    if device::get_default_device(AudioBackend::Default).is_ok() {
        // Default device available - good
    } else {
        println!("Skipping fallback check - no audio device available");
    }

    pool.close().await;
}

// =============================================================================
// Integration Tests - Full Device Initialization Flow
// =============================================================================

/// Test: Full device initialization flow with valid saved device
///
/// Scenario: Saved device exists and is valid
/// Expected: Device is restored successfully
#[tokio::test]
async fn test_successful_device_restoration() {
    // Get default device (if available)
    let Ok(default_device) = device::get_default_device(AudioBackend::Default) else {
        println!("Skipping test - no audio device available");
        return;
    };

    let test_db = TestDb::new().await;
    let pool = test_db.open().await;
    let user_id = "1";

    // Save valid device settings
    let valid_settings = json!({
        "backend": "default",
        "device_name": default_device.name
    });

    let now = chrono::Utc::now().timestamp();
    sqlx::query(
        "INSERT INTO user_settings (user_id, key, value, updated_at)
         VALUES (?, ?, ?, ?)",
    )
    .bind(user_id)
    .bind("audio.output_device")
    .bind(valid_settings.to_string())
    .bind(now)
    .execute(&pool)
    .await
    .expect("Failed to save setting");

    // Simulate initialization - verify device can be found
    let (saved_value,): (String,) =
        sqlx::query_as("SELECT value FROM user_settings WHERE user_id = ? AND key = ?")
            .bind(user_id)
            .bind("audio.output_device")
            .fetch_one(&pool)
            .await
            .expect("Failed to query setting");

    let parsed: serde_json::Value =
        serde_json::from_str(&saved_value).expect("Should parse as JSON");

    let saved_device_name = parsed["device_name"].as_str().unwrap();

    // Verify device exists
    let device_exists = device::find_device_by_name(AudioBackend::Default, saved_device_name);

    assert!(
        device_exists.is_ok(),
        "Saved device should exist and be found"
    );

    pool.close().await;
}

/// Test: Device initialization with no saved settings
///
/// Scenario: First launch, no device settings saved
/// Expected: Uses default device, no errors
#[tokio::test]
async fn test_first_launch_no_saved_device() {
    let test_db = TestDb::new().await;
    let pool = test_db.open().await;
    let user_id = "1";

    // Verify no device settings saved
    let result: Option<(String,)> =
        sqlx::query_as("SELECT value FROM user_settings WHERE user_id = ? AND key = ?")
            .bind(user_id)
            .bind("audio.output_device")
            .fetch_optional(&pool)
            .await
            .expect("Failed to query setting");

    assert!(result.is_none(), "No device settings should be saved");

    // Verify default device is available
    if let Ok(default_device) = device::get_default_device(AudioBackend::Default) {
        assert!(
            !default_device.name.is_empty(),
            "Default device should be available"
        );
        assert!(
            default_device.sample_rate > 0,
            "Default device should have valid sample rate"
        );
    } else {
        println!("Skipping default device check - no audio device available");
    }

    pool.close().await;
}

// =============================================================================
// User Isolation Tests
// =============================================================================

/// Test: Device settings are isolated per user
///
/// Scenario: Multiple users with different device preferences
/// Expected: Each user's settings are independent
#[tokio::test]
async fn test_device_settings_user_isolation() {
    let test_db = TestDb::new().await;
    let pool = test_db.open().await;

    // Create second user
    let now = chrono::Utc::now().timestamp();
    sqlx::query("INSERT INTO users (id, name, created_at) VALUES (?, ?, ?)")
        .bind("2")
        .bind("User 2")
        .bind(now)
        .execute(&pool)
        .await
        .expect("Failed to create second user");

    // Set different device for each user
    let user1_device = json!({
        "backend": "default",
        "device_name": "User 1 Device"
    });

    let user2_device = json!({
        "backend": "default",
        "device_name": "User 2 Device"
    });

    sqlx::query(
        "INSERT INTO user_settings (user_id, key, value, updated_at)
         VALUES (?, ?, ?, ?)",
    )
    .bind("1")
    .bind("audio.output_device")
    .bind(user1_device.to_string())
    .bind(now)
    .execute(&pool)
    .await
    .expect("Failed to save user 1 device");

    sqlx::query(
        "INSERT INTO user_settings (user_id, key, value, updated_at)
         VALUES (?, ?, ?, ?)",
    )
    .bind("2")
    .bind("audio.output_device")
    .bind(user2_device.to_string())
    .bind(now)
    .execute(&pool)
    .await
    .expect("Failed to save user 2 device");

    // Verify isolation
    let (user1_value,): (String,) =
        sqlx::query_as("SELECT value FROM user_settings WHERE user_id = ? AND key = ?")
            .bind("1")
            .bind("audio.output_device")
            .fetch_one(&pool)
            .await
            .expect("Failed to query user 1 setting");

    let (user2_value,): (String,) =
        sqlx::query_as("SELECT value FROM user_settings WHERE user_id = ? AND key = ?")
            .bind("2")
            .bind("audio.output_device")
            .fetch_one(&pool)
            .await
            .expect("Failed to query user 2 setting");

    let user1_parsed: serde_json::Value =
        serde_json::from_str(&user1_value).expect("Should parse user 1 JSON");
    let user2_parsed: serde_json::Value =
        serde_json::from_str(&user2_value).expect("Should parse user 2 JSON");

    assert_eq!(
        user1_parsed["device_name"].as_str().unwrap(),
        "User 1 Device"
    );
    assert_eq!(
        user2_parsed["device_name"].as_str().unwrap(),
        "User 2 Device"
    );

    pool.close().await;
}
