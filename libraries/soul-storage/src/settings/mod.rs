//! User settings management
//!
//! This module provides persistent storage for user preferences across all platforms.
//! Settings are stored as key-value pairs with JSON-serialized values for flexibility.
//!
//! # Example
//!
//! ```rust,no_run
//! use soul_storage::settings;
//! # async fn example(pool: &sqlx::SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
//! // Set a theme preference
//! settings::set_setting(pool, "1", settings::SETTING_THEME, &serde_json::json!("dark")).await?;
//!
//! // Get the theme preference
//! let theme = settings::get_setting(pool, "1", settings::SETTING_THEME).await?;
//! # Ok(())
//! # }
//! ```

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::error::StorageError;

pub type Result<T> = std::result::Result<T, StorageError>;

// Setting key constants
/// UI theme setting (e.g., "light", "dark", "ocean")
pub const SETTING_THEME: &str = "ui.theme";

/// UI locale setting (e.g., "en-US", "de", "ja")
pub const SETTING_LOCALE: &str = "ui.locale";

/// Audio volume setting (0-100)
pub const SETTING_VOLUME: &str = "audio.volume";

/// Enable automatic update checking
pub const SETTING_AUTO_UPDATE_ENABLED: &str = "app.auto_update_enabled";

/// Install updates silently without user prompt
pub const SETTING_AUTO_UPDATE_SILENT: &str = "app.auto_update_silent";

/// Import file management strategy ("move", "copy", or "reference")
pub const SETTING_IMPORT_STRATEGY: &str = "import.file_strategy";

/// Import library path
pub const SETTING_IMPORT_LIBRARY_PATH: &str = "import.library_path";

/// Import confidence threshold (0-100)
pub const SETTING_IMPORT_CONFIDENCE_THRESHOLD: &str = "import.confidence_threshold";

/// Import file naming pattern
pub const SETTING_IMPORT_FILE_NAMING_PATTERN: &str = "import.file_naming_pattern";

/// Import skip duplicates flag
pub const SETTING_IMPORT_SKIP_DUPLICATES: &str = "import.skip_duplicates";

/// User setting entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSetting {
    /// Setting key
    pub key: String,
    /// Setting value (JSON)
    pub value: serde_json::Value,
}

/// Get a single setting value for a user
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `user_id` - User ID
/// * `key` - Setting key
///
/// # Returns
///
/// Returns `Ok(Some(value))` if the setting exists, `Ok(None)` if not found
///
/// # Errors
///
/// Returns an error if the database query fails or JSON deserialization fails
pub async fn get_setting(
    pool: &SqlitePool,
    user_id: &str,
    key: &str,
) -> Result<Option<serde_json::Value>> {
    let result = sqlx::query!(
        "SELECT value FROM user_settings WHERE user_id = ? AND key = ?",
        user_id,
        key
    )
    .fetch_optional(pool)
    .await?;

    match result {
        Some(row) => {
            let value: serde_json::Value = serde_json::from_str(&row.value)
                .map_err(|e| StorageError::SerializationError(e.to_string()))?;
            Ok(Some(value))
        }
        None => Ok(None),
    }
}

/// Set a setting value for a user
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `user_id` - User ID
/// * `key` - Setting key
/// * `value` - Setting value (will be JSON-serialized)
///
/// # Errors
///
/// Returns an error if the database query fails or JSON serialization fails
pub async fn set_setting(
    pool: &SqlitePool,
    user_id: &str,
    key: &str,
    value: &serde_json::Value,
) -> Result<()> {
    let value_str = serde_json::to_string(value)
        .map_err(|e| StorageError::SerializationError(e.to_string()))?;
    let now = chrono::Utc::now().timestamp();

    sqlx::query!(
        "INSERT INTO user_settings (user_id, key, value, updated_at)
         VALUES (?, ?, ?, ?)
         ON CONFLICT(user_id, key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
        user_id,
        key,
        value_str,
        now
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Get all settings for a user
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `user_id` - User ID
///
/// # Returns
///
/// Returns a vector of all user settings
///
/// # Errors
///
/// Returns an error if the database query fails or JSON deserialization fails
pub async fn get_all_settings(pool: &SqlitePool, user_id: &str) -> Result<Vec<UserSetting>> {
    let rows = sqlx::query!(
        "SELECT key, value FROM user_settings WHERE user_id = ?",
        user_id
    )
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|row| {
            let value: serde_json::Value = serde_json::from_str(&row.value)
                .map_err(|e| StorageError::SerializationError(e.to_string()))?;
            Ok(UserSetting {
                key: row.key,
                value,
            })
        })
        .collect()
}

/// Delete a setting for a user
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `user_id` - User ID
/// * `key` - Setting key
///
/// # Returns
///
/// Returns `Ok(true)` if a setting was deleted, `Ok(false)` if no setting was found
///
/// # Errors
///
/// Returns an error if the database query fails
pub async fn delete_setting(pool: &SqlitePool, user_id: &str, key: &str) -> Result<bool> {
    let result = sqlx::query!(
        "DELETE FROM user_settings WHERE user_id = ? AND key = ?",
        user_id,
        key
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

/// Get import file strategy setting
///
/// Returns "move", "copy", or "reference" (defaults to "copy")
pub async fn get_import_strategy(pool: &SqlitePool, user_id: &str) -> Result<String> {
    Ok(
        match get_setting(pool, user_id, SETTING_IMPORT_STRATEGY).await? {
            Some(val) => val.as_str().unwrap_or("copy").to_string(),
            None => "copy".to_string(),
        },
    )
}

/// Get import library path setting
pub async fn get_import_library_path(pool: &SqlitePool, user_id: &str) -> Result<Option<String>> {
    Ok(get_setting(pool, user_id, SETTING_IMPORT_LIBRARY_PATH)
        .await?
        .and_then(|v| v.as_str().map(|s| s.to_string())))
}

/// Get import confidence threshold setting (defaults to 85)
pub async fn get_import_confidence_threshold(pool: &SqlitePool, user_id: &str) -> Result<u8> {
    Ok(
        match get_setting(pool, user_id, SETTING_IMPORT_CONFIDENCE_THRESHOLD).await? {
            Some(val) => val.as_u64().unwrap_or(85) as u8,
            None => 85,
        },
    )
}

/// Get import file naming pattern (defaults to "{artist} - {title}.{ext}")
pub async fn get_import_file_naming_pattern(pool: &SqlitePool, user_id: &str) -> Result<String> {
    Ok(
        match get_setting(pool, user_id, SETTING_IMPORT_FILE_NAMING_PATTERN).await? {
            Some(val) => val
                .as_str()
                .unwrap_or("{artist} - {title}.{ext}")
                .to_string(),
            None => "{artist} - {title}.{ext}".to_string(),
        },
    )
}

/// Get import skip duplicates flag (defaults to true)
pub async fn get_import_skip_duplicates(pool: &SqlitePool, user_id: &str) -> Result<bool> {
    Ok(
        match get_setting(pool, user_id, SETTING_IMPORT_SKIP_DUPLICATES).await? {
            Some(val) => val.as_bool().unwrap_or(true),
            None => true,
        },
    )
}
