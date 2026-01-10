//! External file settings storage
//!
//! Manages settings for handling audio files opened or dropped into the player
//! that are not part of the user's library.
//!
//! # Example
//!
//! ```rust,no_run
//! use soul_storage::external_file_settings;
//! use soul_core::types::{ExternalFileAction, ImportDestination, UpdateExternalFileSettings};
//!
//! # async fn example(pool: &sqlx::SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
//! // Get settings for a user/device (creates defaults if not exists)
//! let settings = external_file_settings::get_or_create(pool, "1", "device-uuid").await?;
//!
//! // Update settings
//! let update = UpdateExternalFileSettings {
//!     default_action: ExternalFileAction::Import,
//!     import_destination: ImportDestination::Managed,
//!     import_to_source_id: None,
//!     show_import_notification: true,
//! };
//! external_file_settings::upsert(pool, "1", "device-uuid", &update).await?;
//! # Ok(())
//! # }
//! ```

use crate::StorageError;
use soul_core::types::{
    ExternalFileAction, ExternalFileSettings, ImportDestination, UpdateExternalFileSettings,
};
use sqlx::SqlitePool;

type Result<T> = std::result::Result<T, StorageError>;

/// Get external file settings for a user/device combination
///
/// Returns `None` if no settings exist for this user/device.
pub async fn get(
    pool: &SqlitePool,
    user_id: &str,
    device_id: &str,
) -> Result<Option<ExternalFileSettings>> {
    let row = sqlx::query!(
        r#"
        SELECT id, user_id, device_id, default_action, import_destination,
               import_to_source_id, show_import_notification, created_at, updated_at
        FROM external_file_settings
        WHERE user_id = ? AND device_id = ?
        "#,
        user_id,
        device_id
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| ExternalFileSettings {
        id: r.id.expect("id should not be null for existing row"),
        user_id: r.user_id,
        device_id: r.device_id,
        default_action: ExternalFileAction::from_str(&r.default_action)
            .unwrap_or(ExternalFileAction::Ask),
        import_destination: ImportDestination::from_str(&r.import_destination)
            .unwrap_or(ImportDestination::Managed),
        import_to_source_id: r.import_to_source_id,
        show_import_notification: r.show_import_notification != 0,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }))
}

/// Get external file settings for a user/device, creating defaults if not exists
pub async fn get_or_create(
    pool: &SqlitePool,
    user_id: &str,
    device_id: &str,
) -> Result<ExternalFileSettings> {
    // Try to get existing settings
    if let Some(settings) = get(pool, user_id, device_id).await? {
        return Ok(settings);
    }

    // Create with defaults
    let defaults = UpdateExternalFileSettings::default();
    upsert(pool, user_id, device_id, &defaults).await?;

    // Fetch and return
    get(pool, user_id, device_id)
        .await?
        .ok_or_else(|| StorageError::not_found("external_file_settings", device_id))
}

/// Create or update external file settings
pub async fn upsert(
    pool: &SqlitePool,
    user_id: &str,
    device_id: &str,
    settings: &UpdateExternalFileSettings,
) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    let default_action = settings.default_action.as_str();
    let import_destination = settings.import_destination.as_str();
    let show_notification = if settings.show_import_notification {
        1
    } else {
        0
    };

    sqlx::query!(
        r#"
        INSERT INTO external_file_settings
            (user_id, device_id, default_action, import_destination,
             import_to_source_id, show_import_notification, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(user_id, device_id) DO UPDATE SET
            default_action = excluded.default_action,
            import_destination = excluded.import_destination,
            import_to_source_id = excluded.import_to_source_id,
            show_import_notification = excluded.show_import_notification,
            updated_at = excluded.updated_at
        "#,
        user_id,
        device_id,
        default_action,
        import_destination,
        settings.import_to_source_id,
        show_notification,
        now,
        now
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Update only the default action setting
pub async fn set_default_action(
    pool: &SqlitePool,
    user_id: &str,
    device_id: &str,
    action: ExternalFileAction,
) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    let action_str = action.as_str();

    // First ensure settings exist
    let _ = get_or_create(pool, user_id, device_id).await?;

    sqlx::query!(
        r#"
        UPDATE external_file_settings
        SET default_action = ?, updated_at = ?
        WHERE user_id = ? AND device_id = ?
        "#,
        action_str,
        now,
        user_id,
        device_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Update only the import destination setting
pub async fn set_import_destination(
    pool: &SqlitePool,
    user_id: &str,
    device_id: &str,
    destination: ImportDestination,
    source_id: Option<i64>,
) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    let destination_str = destination.as_str();

    // First ensure settings exist
    let _ = get_or_create(pool, user_id, device_id).await?;

    sqlx::query!(
        r#"
        UPDATE external_file_settings
        SET import_destination = ?, import_to_source_id = ?, updated_at = ?
        WHERE user_id = ? AND device_id = ?
        "#,
        destination_str,
        source_id,
        now,
        user_id,
        device_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Update only the notification setting
pub async fn set_show_import_notification(
    pool: &SqlitePool,
    user_id: &str,
    device_id: &str,
    show: bool,
) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    let show_int = if show { 1 } else { 0 };

    // First ensure settings exist
    let _ = get_or_create(pool, user_id, device_id).await?;

    sqlx::query!(
        r#"
        UPDATE external_file_settings
        SET show_import_notification = ?, updated_at = ?
        WHERE user_id = ? AND device_id = ?
        "#,
        show_int,
        now,
        user_id,
        device_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Delete settings for a user/device
pub async fn delete(pool: &SqlitePool, user_id: &str, device_id: &str) -> Result<bool> {
    let result = sqlx::query!(
        "DELETE FROM external_file_settings WHERE user_id = ? AND device_id = ?",
        user_id,
        device_id
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

/// Delete all settings for a user (useful when deleting user account)
pub async fn delete_all_for_user(pool: &SqlitePool, user_id: &str) -> Result<u64> {
    let result = sqlx::query!(
        "DELETE FROM external_file_settings WHERE user_id = ?",
        user_id
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}

/// Delete all settings for a device (useful when unregistering device)
pub async fn delete_all_for_device(pool: &SqlitePool, device_id: &str) -> Result<u64> {
    let result = sqlx::query!(
        "DELETE FROM external_file_settings WHERE device_id = ?",
        device_id
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Integration tests would go here using a test database
    // Following project guidelines: test real behavior, not trivial operations

    #[test]
    fn test_external_file_action_roundtrip() {
        for action in [
            ExternalFileAction::Ask,
            ExternalFileAction::Play,
            ExternalFileAction::Import,
        ] {
            let s = action.as_str();
            let parsed = ExternalFileAction::from_str(s);
            assert_eq!(parsed, Some(action));
        }
    }

    #[test]
    fn test_import_destination_roundtrip() {
        for dest in [ImportDestination::Managed, ImportDestination::Watched] {
            let s = dest.as_str();
            let parsed = ImportDestination::from_str(s);
            assert_eq!(parsed, Some(dest));
        }
    }

    #[test]
    fn test_invalid_action_returns_none() {
        assert_eq!(ExternalFileAction::from_str("invalid"), None);
        assert_eq!(ExternalFileAction::from_str(""), None);
    }

    #[test]
    fn test_invalid_destination_returns_none() {
        assert_eq!(ImportDestination::from_str("invalid"), None);
        assert_eq!(ImportDestination::from_str(""), None);
    }
}
