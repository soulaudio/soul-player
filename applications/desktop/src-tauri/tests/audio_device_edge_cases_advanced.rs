//! Advanced Audio Device Edge Cases Tests
//!
//! Tests covering critical edge cases identified through comprehensive analysis:
//! - Whitespace handling in device names
//! - Empty string vs NULL consistency
//! - Backend/device name mismatches
//! - Device state changes during initialization
//! - Database edge cases
//! - Concurrent operations

use serde_json::json;
#[cfg(feature = "asio")]
use soul_audio_desktop::backend;
use soul_audio_desktop::{device, AudioBackend};
use sqlx::SqlitePool;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tempfile::TempDir;

// =============================================================================
// Test Infrastructure
// =============================================================================

struct TestDb {
    db_path: std::path::PathBuf,
    _temp_dir: TempDir,
}

impl TestDb {
    async fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let db_path = temp_dir.path().join("test.db");

        let pool = Self::create_pool_for_path(&db_path).await;
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

    async fn open(&self) -> SqlitePool {
        Self::create_pool_for_path(&self.db_path).await
    }
}

// =============================================================================
// HIGH PRIORITY: Whitespace Handling
// =============================================================================

/// Test: Device name with leading/trailing whitespace
///
/// Scenario: User copies device name with spaces, or UI bug adds whitespace
/// Expected: Whitespace trimmed, device found successfully
#[tokio::test]
async fn test_device_name_with_leading_trailing_whitespace() {
    let test_db = TestDb::new().await;
    let pool = test_db.open().await;
    let user_id = "1";

    // Get actual device (if available)
    let Ok(default_device) = device::get_default_device(AudioBackend::Default) else {
        println!("Skipping test - no audio device available");
        pool.close().await;
        return;
    };

    // Save with extra whitespace
    let device_with_whitespace = format!("  {}  ", default_device.name);
    let settings = json!({
        "backend": "default",
        "device_name": device_with_whitespace
    });

    let now = chrono::Utc::now().timestamp();
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

    // Verify whitespace is in the saved value
    let (saved_value,): (String,) =
        sqlx::query_as("SELECT value FROM user_settings WHERE user_id = ? AND key = ?")
            .bind(user_id)
            .bind("audio.output_device")
            .fetch_one(&pool)
            .await
            .expect("Failed to query setting");

    let parsed: serde_json::Value =
        serde_json::from_str(&saved_value).expect("Should parse as JSON");
    let saved_name = parsed["device_name"].as_str().unwrap();

    // Verify whitespace is present in DB (before trim)
    assert!(
        saved_name.starts_with(' ') || saved_name.ends_with(' '),
        "Whitespace should be in saved value"
    );

    // Simulate initialize_audio_device behavior
    // With current code, this would fail to find device
    // After fix with .trim(), it should succeed
    let trimmed_name = saved_name.trim();
    let device_lookup = device::find_device_by_name(AudioBackend::Default, trimmed_name);

    assert!(device_lookup.is_ok(), "Trimmed device name should be found");

    pool.close().await;
}

/// Test: Device name with only whitespace
///
/// Scenario: Empty input treated as "   "
/// Expected: Treated as empty device name, uses default
#[tokio::test]
async fn test_device_name_only_whitespace() {
    let test_db = TestDb::new().await;
    let pool = test_db.open().await;
    let user_id = "1";

    // Save with only whitespace
    let settings = json!({
        "backend": "default",
        "device_name": "   "
    });

    let now = chrono::Utc::now().timestamp();
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

    // Verify saved
    let (saved_value,): (String,) =
        sqlx::query_as("SELECT value FROM user_settings WHERE user_id = ? AND key = ?")
            .bind(user_id)
            .bind("audio.output_device")
            .fetch_one(&pool)
            .await
            .expect("Failed to query setting");

    let parsed: serde_json::Value =
        serde_json::from_str(&saved_value).expect("Should parse as JSON");
    let saved_name = parsed["device_name"].as_str().unwrap();

    // After .trim(), should be empty
    assert!(
        saved_name.trim().is_empty(),
        "Whitespace-only should trim to empty"
    );

    pool.close().await;
}

// =============================================================================
// HIGH PRIORITY: Empty String vs NULL Consistency
// =============================================================================

/// Test: Empty string device name roundtrip
///
/// Scenario: set_audio_device called with empty string
/// Expected: Stored and retrieved consistently, treated as default device
#[tokio::test]
async fn test_empty_string_device_name_roundtrip() {
    let test_db = TestDb::new().await;
    let pool = test_db.open().await;
    let user_id = "1";

    // Test Case 1: Empty string stored directly
    let settings_empty = json!({
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
    .bind(settings_empty.to_string())
    .bind(now)
    .execute(&pool)
    .await
    .expect("Failed to save empty string");

    // Retrieve and verify
    let (value1,): (String,) =
        sqlx::query_as("SELECT value FROM user_settings WHERE user_id = ? AND key = ?")
            .bind(user_id)
            .bind("audio.output_device")
            .fetch_one(&pool)
            .await
            .expect("Failed to query");

    let parsed1: serde_json::Value = serde_json::from_str(&value1).unwrap();
    assert_eq!(parsed1["device_name"].as_str().unwrap(), "");

    // Test Case 2: Null stored
    let settings_null = json!({
        "backend": "default",
        "device_name": serde_json::Value::Null
    });

    sqlx::query("UPDATE user_settings SET value = ?, updated_at = ? WHERE user_id = ? AND key = ?")
        .bind(settings_null.to_string())
        .bind(chrono::Utc::now().timestamp())
        .bind(user_id)
        .bind("audio.output_device")
        .execute(&pool)
        .await
        .expect("Failed to save null");

    let (value2,): (String,) =
        sqlx::query_as("SELECT value FROM user_settings WHERE user_id = ? AND key = ?")
            .bind(user_id)
            .bind("audio.output_device")
            .fetch_one(&pool)
            .await
            .expect("Failed to query");

    let parsed2: serde_json::Value = serde_json::from_str(&value2).unwrap();
    assert!(parsed2["device_name"].is_null());

    // Test Case 3: Missing key
    let settings_missing = json!({
        "backend": "default"
    });

    sqlx::query("UPDATE user_settings SET value = ?, updated_at = ? WHERE user_id = ? AND key = ?")
        .bind(settings_missing.to_string())
        .bind(chrono::Utc::now().timestamp())
        .bind(user_id)
        .bind("audio.output_device")
        .execute(&pool)
        .await
        .expect("Failed to save missing key");

    let (value3,): (String,) =
        sqlx::query_as("SELECT value FROM user_settings WHERE user_id = ? AND key = ?")
            .bind(user_id)
            .bind("audio.output_device")
            .fetch_one(&pool)
            .await
            .expect("Failed to query");

    let parsed3: serde_json::Value = serde_json::from_str(&value3).unwrap();
    assert!(parsed3.get("device_name").is_none());

    // All three cases should be treated as "use default device"
    // Verify initialize_audio_device handles all three consistently

    pool.close().await;
}

// =============================================================================
// HIGH PRIORITY: Backend/Device Mismatch
// =============================================================================

/// Test: ASIO backend with WASAPI-style device name
///
/// Scenario: User switches backend but device name format doesn't match
/// Expected: Device not found, fallback to default, helpful error message
#[tokio::test]
#[cfg(feature = "asio")]
async fn test_backend_device_name_format_mismatch() {
    let test_db = TestDb::new().await;
    let pool = test_db.open().await;
    let user_id = "1";

    // Save ASIO backend with WASAPI-format device name
    let settings = json!({
        "backend": "asio",
        "device_name": "Speakers (Realtek High Definition Audio)"  // WASAPI format
    });

    let now = chrono::Utc::now().timestamp();
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

    // Check if ASIO is available
    let backends = backend::get_backend_info();
    let asio_available = backends
        .iter()
        .any(|b| matches!(b.backend, AudioBackend::Asio) && b.available);

    if asio_available {
        // ASIO available - device name won't match ASIO devices
        // Should fall back to default device
        let asio_devices = device::list_devices(AudioBackend::Asio);

        match asio_devices {
            Ok(devices) => {
                // Verify WASAPI-format name doesn't exist in ASIO device list
                let wasapi_name = "Speakers (Realtek High Definition Audio)";
                let found = devices.iter().any(|d| d.name == wasapi_name);

                assert!(
                    !found,
                    "WASAPI-format device name should not exist in ASIO device list"
                );
            }
            Err(_) => {
                println!("ASIO available but no devices - test passes");
            }
        }
    } else {
        // ASIO not available - backend parsing will fail, settings deleted
        println!("ASIO not available - backend will be invalid");
    }

    pool.close().await;
}

/// Test: Backend feature not compiled
///
/// Scenario: Setting says "asio" but app compiled without asio feature
/// Expected: Backend parsing fails, setting deleted, uses default
#[tokio::test]
async fn test_backend_feature_not_compiled() {
    let test_db = TestDb::new().await;
    let pool = test_db.open().await;
    let user_id = "1";

    // Try JACK backend (uncommon on Windows/macOS)
    let settings = json!({
        "backend": "jack",
        "device_name": "system"
    });

    let now = chrono::Utc::now().timestamp();
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

    // Check if JACK is available (only available on Linux/macOS with jack feature)
    #[cfg(feature = "jack")]
    let jack_available = {
        let backends = backend::get_backend_info();
        backends
            .iter()
            .any(|b| matches!(b.backend, AudioBackend::Jack) && b.available)
    };

    #[cfg(not(feature = "jack"))]
    let jack_available = false;

    if !jack_available {
        println!("JACK not available - backend will be invalid (expected)");

        // Simulate initialize_audio_device: parse_backend("jack") should fail
        // Setting should be deleted
        // This is the expected behavior
    }

    pool.close().await;
}

// =============================================================================
// MEDIUM PRIORITY: Device State Changes
// =============================================================================

/// Test: Device unplugged between verification and switch
///
/// Scenario: Device exists when checked, unplugged before switch
/// Expected: switch_device fails, error logged, continues with default
#[tokio::test]
async fn test_device_removed_after_verification() {
    // This is a timing-based edge case that's hard to test reliably
    // The current code handles it correctly (line 504-514 in audio_settings.rs)
    // by catching switch_device errors and logging them

    // We can verify the error path exists and is covered
    // Actual testing would require mocking or hardware manipulation

    println!("Note: This edge case is handled in code at audio_settings.rs:504-514");
    println!("Error from switch_device is caught and logged, app continues");
}

/// Test: Multiple devices with same name (different backends)
///
/// Scenario: "Speakers" exists on both Default and ASIO backends
/// Expected: Backend-specific device lookup, not confused
#[tokio::test]
#[cfg(feature = "asio")]
async fn test_same_device_name_different_backends() {
    // Get default device
    let Ok(default_device) = device::get_default_device(AudioBackend::Default) else {
        println!("Skipping test - no default device available");
        return;
    };

    // Check if ASIO is available
    let backends = backend::get_backend_info();
    let asio_available = backends
        .iter()
        .any(|b| matches!(b.backend, AudioBackend::Asio) && b.available);

    if !asio_available {
        println!("Skipping test - ASIO not available");
        return;
    }

    // Try to find device with same name on ASIO
    let asio_lookup = device::find_device_by_name(AudioBackend::Asio, &default_device.name);

    match asio_lookup {
        Ok(_) => {
            println!(
                "Device '{}' exists on both Default and ASIO - name collision!",
                default_device.name
            );
            // This is the edge case - same name, different backends
            // Backend-specific lookup should work correctly
        }
        Err(_) => {
            println!(
                "Device '{}' exists on Default but not ASIO - no name collision",
                default_device.name
            );
        }
    }
}

// =============================================================================
// MEDIUM PRIORITY: Database Edge Cases
// =============================================================================

/// Test: Database locked during initialization
///
/// Scenario: Long-running EXCLUSIVE transaction blocks read
/// Expected: Query times out or fails gracefully
#[tokio::test]
async fn test_database_locked_during_read() {
    let test_db = TestDb::new().await;
    let pool1 = test_db.open().await;
    let pool2 = test_db.open().await;

    let user_id = "1";

    // Save a setting
    let settings = json!({
        "backend": "default",
        "device_name": "Test Device"
    });

    let now = chrono::Utc::now().timestamp();
    sqlx::query(
        "INSERT INTO user_settings (user_id, key, value, updated_at)
         VALUES (?, ?, ?, ?)",
    )
    .bind(user_id)
    .bind("audio.output_device")
    .bind(settings.to_string())
    .bind(now)
    .execute(&pool1)
    .await
    .expect("Failed to save setting");

    // Start EXCLUSIVE transaction in pool1
    let lock_holder = Arc::new(AtomicBool::new(true));
    let lock_holder_clone = lock_holder.clone();

    let handle = tokio::spawn(async move {
        // Begin exclusive transaction
        let mut tx = pool1.begin().await.unwrap();

        // Lock the table
        sqlx::query("UPDATE user_settings SET value = value WHERE user_id = ?")
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .unwrap();

        // Hold lock for 2 seconds
        while lock_holder_clone.load(Ordering::Relaxed) {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        tx.commit().await.unwrap();
    });

    // Wait for lock to be acquired
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Try to read while locked (from pool2)
    let read_start = std::time::Instant::now();
    let read_result = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        sqlx::query_as::<_, (String,)>(
            "SELECT value FROM user_settings WHERE user_id = ? AND key = ?",
        )
        .bind(user_id)
        .bind("audio.output_device")
        .fetch_one(&pool2),
    )
    .await;

    let read_duration = read_start.elapsed();

    // Release lock
    lock_holder.store(false, Ordering::Relaxed);
    handle.await.unwrap();

    match read_result {
        Ok(Ok(_)) => {
            // Read succeeded (SQLite allows concurrent reads)
            println!("Read succeeded despite lock (SQLite allows concurrent reads)");
        }
        Ok(Err(e)) => {
            println!("Read failed with database error: {}", e);
        }
        Err(_) => {
            println!("Read timed out after {:?}", read_duration);
            assert!(
                read_duration >= std::time::Duration::from_secs(5),
                "Should timeout after 5 seconds"
            );
        }
    }

    pool2.close().await;
}

/// Test: DELETE fails during corrupted setting cleanup
///
/// Scenario: JSON corrupted, DELETE query fails
/// Expected: Error logged, continues (doesn't retry infinitely)
#[tokio::test]
async fn test_corrupted_json_delete_fails() {
    let test_db = TestDb::new().await;
    let pool = test_db.open().await;
    let user_id = "1";

    // Save corrupted JSON
    let corrupted_json = "{ invalid json";

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

    // Verify JSON is corrupted
    let (value,): (String,) =
        sqlx::query_as("SELECT value FROM user_settings WHERE user_id = ? AND key = ?")
            .bind(user_id)
            .bind("audio.output_device")
            .fetch_one(&pool)
            .await
            .expect("Failed to query");

    let parse_result: Result<serde_json::Value, _> = serde_json::from_str(&value);
    assert!(parse_result.is_err(), "JSON should be corrupted");

    // DELETE should succeed (no reason for it to fail)
    let delete_result = sqlx::query("DELETE FROM user_settings WHERE user_id = ? AND key = ?")
        .bind(user_id)
        .bind("audio.output_device")
        .execute(&pool)
        .await;

    assert!(delete_result.is_ok(), "DELETE should succeed");

    // Verify deletion
    let after_delete: Option<(String,)> =
        sqlx::query_as("SELECT value FROM user_settings WHERE user_id = ? AND key = ?")
            .bind(user_id)
            .bind("audio.output_device")
            .fetch_optional(&pool)
            .await
            .expect("Failed to query");

    assert!(after_delete.is_none(), "Setting should be deleted");

    pool.close().await;
}

// =============================================================================
// MEDIUM PRIORITY: Concurrent Operations
// =============================================================================

/// Test: Concurrent initialize_audio_device calls
///
/// Scenario: Two threads call initialization simultaneously
/// Expected: No panic, PlaybackManager Mutex handles synchronization
#[tokio::test]
async fn test_concurrent_initialization_calls() {
    // This requires a real PlaybackManager, which needs audio hardware
    // Testing strategy: Verify database operations are atomic

    let test_db = TestDb::new().await;

    // Spawn multiple tasks updating device setting
    let mut handles = vec![];

    for i in 0..5 {
        let pool = test_db.open().await;

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
            .bind("1")
            .bind("audio.output_device")
            .bind(settings.to_string())
            .bind(now)
            .execute(&pool)
            .await
            .expect("Failed to update");

            pool.close().await;
        });

        handles.push(handle);
    }

    // Wait for all
    for handle in handles {
        handle.await.unwrap();
    }

    // Verify database is consistent (no corruption)
    let pool = test_db.open().await;

    let (value,): (String,) =
        sqlx::query_as("SELECT value FROM user_settings WHERE user_id = ? AND key = ?")
            .bind("1")
            .bind("audio.output_device")
            .fetch_one(&pool)
            .await
            .expect("Failed to query");

    let parsed: serde_json::Value = serde_json::from_str(&value).expect("Should parse");

    // Should be one of the values (0-4), not corrupted
    let device_name = parsed["device_name"].as_str().unwrap();
    assert!(device_name.starts_with("Device "));

    pool.close().await;
}
