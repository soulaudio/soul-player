//! Library sources storage (watched folders)
//!
//! Manages watched folders that Soul Player monitors for audio files.
//!
//! # Example
//!
//! ```rust,no_run
//! use soul_storage::library_sources;
//! use soul_core::types::CreateLibrarySource;
//!
//! # async fn example(pool: &sqlx::SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
//! // Add a watched folder
//! let source = library_sources::create(pool, "1", "device-uuid", &CreateLibrarySource {
//!     name: "FLAC Collection".to_string(),
//!     path: "/home/user/Music/FLAC".to_string(),
//!     sync_deletes: true,
//! }).await?;
//!
//! // Get all sources for a user/device
//! let sources = library_sources::get_by_user_device(pool, "1", "device-uuid").await?;
//! # Ok(())
//! # }
//! ```

use crate::StorageError;
use soul_core::types::{CreateLibrarySource, LibrarySource, ScanStatus, UpdateLibrarySource};
use sqlx::SqlitePool;

type Result<T> = std::result::Result<T, StorageError>;

/// Get a library source by ID
pub async fn get_by_id(pool: &SqlitePool, id: i64) -> Result<Option<LibrarySource>> {
    let row: Option<_> = sqlx::query!(
        r#"
        SELECT id, user_id, device_id, name, path, enabled, sync_deletes,
               last_scan_at, scan_status, error_message, created_at, updated_at
        FROM library_sources
        WHERE id = ?
        "#,
        id
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| LibrarySource {
        id: r.id,
        user_id: r.user_id,
        device_id: r.device_id,
        name: r.name,
        path: r.path,
        enabled: r.enabled != 0,
        sync_deletes: r.sync_deletes != 0,
        last_scan_at: r.last_scan_at,
        scan_status: r
            .scan_status
            .as_deref()
            .and_then(ScanStatus::from_str)
            .unwrap_or(ScanStatus::Idle),
        error_message: r.error_message,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }))
}

/// Get all library sources for a user/device combination
pub async fn get_by_user_device(
    pool: &SqlitePool,
    user_id: &str,
    device_id: &str,
) -> Result<Vec<LibrarySource>> {
    let rows: Vec<_> = sqlx::query!(
        r#"
        SELECT id, user_id, device_id, name, path, enabled, sync_deletes,
               last_scan_at, scan_status, error_message, created_at, updated_at
        FROM library_sources
        WHERE user_id = ? AND device_id = ?
        ORDER BY name
        "#,
        user_id,
        device_id
    )
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|r| {
            Ok(LibrarySource {
                id: r
                    .id
                    .ok_or_else(|| StorageError::MissingField("library_source.id".to_string()))?,
                user_id: r.user_id,
                device_id: r.device_id,
                name: r.name,
                path: r.path,
                enabled: r.enabled != 0,
                sync_deletes: r.sync_deletes != 0,
                last_scan_at: r.last_scan_at,
                scan_status: r
                    .scan_status
                    .as_deref()
                    .and_then(ScanStatus::from_str)
                    .unwrap_or(ScanStatus::Idle),
                error_message: r.error_message,
                created_at: r.created_at,
                updated_at: r.updated_at,
            })
        })
        .collect()
}

/// Get only enabled library sources for a user/device
pub async fn get_enabled(
    pool: &SqlitePool,
    user_id: &str,
    device_id: &str,
) -> Result<Vec<LibrarySource>> {
    let rows: Vec<_> = sqlx::query!(
        r#"
        SELECT id, user_id, device_id, name, path, enabled, sync_deletes,
               last_scan_at, scan_status, error_message, created_at, updated_at
        FROM library_sources
        WHERE user_id = ? AND device_id = ? AND enabled = 1
        ORDER BY name
        "#,
        user_id,
        device_id
    )
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|r| {
            Ok(LibrarySource {
                id: r
                    .id
                    .ok_or_else(|| StorageError::MissingField("library_source.id".to_string()))?,
                user_id: r.user_id,
                device_id: r.device_id,
                name: r.name,
                path: r.path,
                enabled: r.enabled != 0,
                sync_deletes: r.sync_deletes != 0,
                last_scan_at: r.last_scan_at,
                scan_status: r
                    .scan_status
                    .as_deref()
                    .and_then(ScanStatus::from_str)
                    .unwrap_or(ScanStatus::Idle),
                error_message: r.error_message,
                created_at: r.created_at,
                updated_at: r.updated_at,
            })
        })
        .collect()
}

/// Create a new library source (watched folder)
pub async fn create(
    pool: &SqlitePool,
    user_id: &str,
    device_id: &str,
    source: &CreateLibrarySource,
) -> Result<LibrarySource> {
    let now = chrono::Utc::now().timestamp();
    let sync_deletes = i32::from(source.sync_deletes);

    let result: sqlx::sqlite::SqliteQueryResult = sqlx::query!(
        r#"
        INSERT INTO library_sources (user_id, device_id, name, path, sync_deletes, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        "#,
        user_id,
        device_id,
        source.name,
        source.path,
        sync_deletes,
        now,
        now
    )
    .execute(pool)
    .await?;

    let id = result.last_insert_rowid();

    Ok(LibrarySource {
        id,
        user_id: user_id.to_string(),
        device_id: device_id.to_string(),
        name: source.name.clone(),
        path: source.path.clone(),
        enabled: true,
        sync_deletes: source.sync_deletes,
        last_scan_at: None,
        scan_status: ScanStatus::Idle,
        error_message: None,
        created_at: now,
        updated_at: now,
    })
}

/// Update a library source
pub async fn update(pool: &SqlitePool, id: i64, update: &UpdateLibrarySource) -> Result<bool> {
    let now = chrono::Utc::now().timestamp();

    // Get current source
    let current = get_by_id(pool, id).await?;
    let Some(current) = current else {
        return Ok(false);
    };

    let name = update.name.as_ref().unwrap_or(&current.name);
    let enabled = update.enabled.unwrap_or(current.enabled);
    let enabled_int = i32::from(enabled);
    let sync_deletes = update.sync_deletes.unwrap_or(current.sync_deletes);
    let sync_deletes_int = i32::from(sync_deletes);

    let result: sqlx::sqlite::SqliteQueryResult = sqlx::query!(
        r#"
        UPDATE library_sources
        SET name = ?, enabled = ?, sync_deletes = ?, updated_at = ?
        WHERE id = ?
        "#,
        name,
        enabled_int,
        sync_deletes_int,
        now,
        id
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

/// Delete a library source
pub async fn delete(pool: &SqlitePool, id: i64) -> Result<bool> {
    let result: sqlx::sqlite::SqliteQueryResult =
        sqlx::query!("DELETE FROM library_sources WHERE id = ?", id)
            .execute(pool)
            .await?;

    Ok(result.rows_affected() > 0)
}

/// Set the scan status for a library source
pub async fn set_scan_status(
    pool: &SqlitePool,
    id: i64,
    status: ScanStatus,
    error_message: Option<&str>,
) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    let status_str = status.as_str();

    sqlx::query!(
        r#"
        UPDATE library_sources
        SET scan_status = ?, error_message = ?, updated_at = ?
        WHERE id = ?
        "#,
        status_str,
        error_message,
        now,
        id
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Update the last scan timestamp
pub async fn set_last_scan_at(pool: &SqlitePool, id: i64, timestamp: i64) -> Result<()> {
    let now = chrono::Utc::now().timestamp();

    sqlx::query!(
        r#"
        UPDATE library_sources
        SET last_scan_at = ?, scan_status = 'idle', error_message = NULL, updated_at = ?
        WHERE id = ?
        "#,
        timestamp,
        now,
        id
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Cleanup orphaned scans on startup
///
/// This function should be called on app startup to reset any library sources
/// that were stuck in "scanning" state due to app crash or forced quit.
/// It also cancels any running scan_progress records, even if the library_source
/// scan_status is already 'idle' (can happen if app crashed after updating library_source
/// but before completing scan_progress).
///
/// Returns the number of orphaned scans that were cleaned up.
pub async fn cleanup_orphaned_scans(
    pool: &SqlitePool,
    user_id: &str,
    device_id: &str,
) -> Result<usize> {
    // Get all sources for this user/device
    let sources = get_by_user_device(pool, user_id, device_id).await?;
    let mut cleanup_count = 0;

    for source in sources {
        // Reset library source if stuck in scanning state
        if source.scan_status == ScanStatus::Scanning {
            set_scan_status(pool, source.id, ScanStatus::Idle, None).await?;
            cleanup_count += 1;
        }

        // Cancel any orphaned running scan_progress records
        // (this can exist even if library_source.scan_status is 'idle')
        if let Some(progress) = crate::scan_progress::get_running(pool, source.id).await? {
            crate::scan_progress::cancel(pool, progress.id).await?;
            cleanup_count += 1;
        }
    }

    Ok(cleanup_count)
}

/// Check if a path already exists for this user/device
pub async fn path_exists(
    pool: &SqlitePool,
    user_id: &str,
    device_id: &str,
    path: &str,
) -> Result<bool> {
    let row: Option<_> = sqlx::query!(
        "SELECT id FROM library_sources WHERE user_id = ? AND device_id = ? AND path = ?",
        user_id,
        device_id,
        path
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.is_some())
}

/// Get count of library sources for a user/device
pub async fn count(pool: &SqlitePool, user_id: &str, device_id: &str) -> Result<i64> {
    let row = sqlx::query!(
        "SELECT COUNT(*) as count FROM library_sources WHERE user_id = ? AND device_id = ?",
        user_id,
        device_id
    )
    .fetch_one(pool)
    .await?;

    Ok(row.count as i64)
}

/// Set the enabled status of a library source
pub async fn set_enabled(pool: &SqlitePool, id: i64, enabled: bool) -> Result<bool> {
    let now = chrono::Utc::now().timestamp();
    let enabled_int = i32::from(enabled);

    let result: sqlx::sqlite::SqliteQueryResult = sqlx::query!(
        r#"
        UPDATE library_sources
        SET enabled = ?, updated_at = ?
        WHERE id = ?
        "#,
        enabled_int,
        now,
        id
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

/// Check if onboarding is needed (no library sources AND no managed library configured)
///
/// This is an optimized version that uses a single SQL query instead of two separate queries.
/// Onboarding is needed if the user has zero library sources AND no managed library settings.
pub async fn check_onboarding_needed(
    pool: &SqlitePool,
    user_id: &str,
    device_id: &str,
) -> Result<bool> {
    let result = sqlx::query!(
        r#"
        SELECT
            (SELECT COUNT(*) FROM library_sources WHERE user_id = ? AND device_id = ?) as source_count,
            (SELECT COUNT(*) FROM managed_library_settings WHERE user_id = ? AND device_id = ?) as managed_count
        "#,
        user_id,
        device_id,
        user_id,
        device_id
    )
    .fetch_one(pool)
    .await?;

    // Onboarding needed if both counts are 0
    Ok(result.source_count == 0 && result.managed_count == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_status_roundtrip() {
        for status in [ScanStatus::Idle, ScanStatus::Scanning, ScanStatus::Error] {
            let s = status.as_str();
            let parsed = ScanStatus::from_str(s);
            assert_eq!(parsed, Some(status));
        }
    }

    #[test]
    fn test_invalid_scan_status() {
        assert_eq!(ScanStatus::from_str("invalid"), None);
        assert_eq!(ScanStatus::from_str(""), None);
    }
}
