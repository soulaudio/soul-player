//! Managed library settings storage
//!
//! Stores configuration for the managed library folder where imports are organized.
//!
//! # Example
//!
//! ```rust,no_run
//! use soul_storage::managed_library_settings;
//! use soul_core::types::UpdateManagedLibrarySettings;
//!
//! # async fn example(pool: &sqlx::SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
//! // Configure managed library
//! managed_library_settings::upsert(pool, "1", "device-uuid", &UpdateManagedLibrarySettings {
//!     library_path: "/home/user/Music/Soul Player".to_string(),
//!     path_template: "{AlbumArtist}/{Year} - {Album}/{TrackNo} - {Title}".to_string(),
//!     import_action: soul_core::types::ImportAction::Copy,
//! }).await?;
//!
//! // Get settings (creates defaults if needed)
//! let settings = managed_library_settings::get_or_create(pool, "1", "device-uuid", "/home/user/Music/Soul Player").await?;
//! # Ok(())
//! # }
//! ```

use crate::StorageError;
use soul_core::types::{ImportAction, ManagedLibrarySettings, UpdateManagedLibrarySettings};
use sqlx::SqlitePool;

type Result<T> = std::result::Result<T, StorageError>;

/// Get managed library settings for a user/device
pub async fn get(
    pool: &SqlitePool,
    user_id: &str,
    device_id: &str,
) -> Result<Option<ManagedLibrarySettings>> {
    let row = sqlx::query!(
        r#"
        SELECT id, user_id, device_id, library_path, path_template,
               import_action, created_at, updated_at
        FROM managed_library_settings
        WHERE user_id = ? AND device_id = ?
        "#,
        user_id,
        device_id
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| ManagedLibrarySettings {
        id: r.id.expect("managed_library_settings id cannot be null"),
        user_id: r.user_id,
        device_id: r.device_id,
        library_path: r.library_path,
        path_template: r.path_template,
        import_action: ImportAction::from_str(&r.import_action).unwrap_or(ImportAction::Copy),
        created_at: r.created_at,
        updated_at: r.updated_at,
    }))
}

/// Get managed library settings, creating with defaults if not exists
///
/// # Arguments
///
/// * `pool` - Database connection
/// * `user_id` - User ID
/// * `device_id` - Device ID
/// * `default_path` - Default library path to use if creating new settings
pub async fn get_or_create(
    pool: &SqlitePool,
    user_id: &str,
    device_id: &str,
    default_path: &str,
) -> Result<ManagedLibrarySettings> {
    // Try to get existing
    if let Some(settings) = get(pool, user_id, device_id).await? {
        return Ok(settings);
    }

    // Create with defaults
    let defaults = UpdateManagedLibrarySettings {
        library_path: default_path.to_string(),
        ..Default::default()
    };
    upsert(pool, user_id, device_id, &defaults).await?;

    get(pool, user_id, device_id)
        .await?
        .ok_or_else(|| StorageError::not_found("managed_library_settings", device_id))
}

/// Create or update managed library settings
pub async fn upsert(
    pool: &SqlitePool,
    user_id: &str,
    device_id: &str,
    settings: &UpdateManagedLibrarySettings,
) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    let import_action = settings.import_action.as_str();

    sqlx::query!(
        r#"
        INSERT INTO managed_library_settings
            (user_id, device_id, library_path, path_template, import_action, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(user_id, device_id) DO UPDATE SET
            library_path = excluded.library_path,
            path_template = excluded.path_template,
            import_action = excluded.import_action,
            updated_at = excluded.updated_at
        "#,
        user_id,
        device_id,
        settings.library_path,
        settings.path_template,
        import_action,
        now,
        now
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Update only the library path
pub async fn set_library_path(
    pool: &SqlitePool,
    user_id: &str,
    device_id: &str,
    path: &str,
) -> Result<bool> {
    let now = chrono::Utc::now().timestamp();

    let result = sqlx::query!(
        r#"
        UPDATE managed_library_settings
        SET library_path = ?, updated_at = ?
        WHERE user_id = ? AND device_id = ?
        "#,
        path,
        now,
        user_id,
        device_id
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

/// Update only the path template
pub async fn set_path_template(
    pool: &SqlitePool,
    user_id: &str,
    device_id: &str,
    template: &str,
) -> Result<bool> {
    let now = chrono::Utc::now().timestamp();

    let result = sqlx::query!(
        r#"
        UPDATE managed_library_settings
        SET path_template = ?, updated_at = ?
        WHERE user_id = ? AND device_id = ?
        "#,
        template,
        now,
        user_id,
        device_id
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

/// Update only the import action
pub async fn set_import_action(
    pool: &SqlitePool,
    user_id: &str,
    device_id: &str,
    action: ImportAction,
) -> Result<bool> {
    let now = chrono::Utc::now().timestamp();
    let action_str = action.as_str();

    let result = sqlx::query!(
        r#"
        UPDATE managed_library_settings
        SET import_action = ?, updated_at = ?
        WHERE user_id = ? AND device_id = ?
        "#,
        action_str,
        now,
        user_id,
        device_id
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

/// Delete settings for a user/device
pub async fn delete(pool: &SqlitePool, user_id: &str, device_id: &str) -> Result<bool> {
    let result = sqlx::query!(
        "DELETE FROM managed_library_settings WHERE user_id = ? AND device_id = ?",
        user_id,
        device_id
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

/// Check if managed library is configured for a user/device
pub async fn is_configured(pool: &SqlitePool, user_id: &str, device_id: &str) -> Result<bool> {
    let row = sqlx::query!(
        "SELECT id FROM managed_library_settings WHERE user_id = ? AND device_id = ?",
        user_id,
        device_id
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.is_some())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_import_action_roundtrip() {
        for action in [ImportAction::Copy, ImportAction::Move] {
            let s = action.as_str();
            let parsed = ImportAction::from_str(s);
            assert_eq!(parsed, Some(action));
        }
    }

    #[test]
    fn test_invalid_import_action() {
        assert_eq!(ImportAction::from_str("invalid"), None);
        assert_eq!(ImportAction::from_str(""), None);
    }
}
