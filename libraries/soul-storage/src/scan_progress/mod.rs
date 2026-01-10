//! Scan progress tracking storage
//!
//! Monitors ongoing library scans and their progress.
//!
//! # Example
//!
//! ```rust,no_run
//! use soul_storage::scan_progress;
//!
//! # async fn example(pool: &sqlx::SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
//! // Start a new scan
//! let progress = scan_progress::start(pool, 1, Some(1000)).await?;
//!
//! // Update progress
//! scan_progress::increment_processed(pool, progress.id, 1).await?;
//! scan_progress::increment_new(pool, progress.id, 1).await?;
//!
//! // Complete the scan
//! scan_progress::complete(pool, progress.id).await?;
//! # Ok(())
//! # }
//! ```

use crate::StorageError;
use soul_core::types::{ScanProgress, ScanProgressStatus};
use sqlx::SqlitePool;

type Result<T> = std::result::Result<T, StorageError>;

/// Get a scan progress by ID
pub async fn get_by_id(pool: &SqlitePool, id: i64) -> Result<Option<ScanProgress>> {
    let row = sqlx::query!(
        r#"
        SELECT id, library_source_id, started_at, completed_at, total_files,
               processed_files, new_files, updated_files, removed_files,
               errors, status, error_message
        FROM scan_progress
        WHERE id = ?
        "#,
        id
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| ScanProgress {
        id: r.id,
        library_source_id: r.library_source_id,
        started_at: r.started_at,
        completed_at: r.completed_at,
        total_files: r.total_files,
        processed_files: r.processed_files,
        new_files: r.new_files,
        updated_files: r.updated_files,
        removed_files: r.removed_files,
        errors: r.errors,
        status: ScanProgressStatus::from_str(&r.status).unwrap_or(ScanProgressStatus::Running),
        error_message: r.error_message,
    }))
}

/// Get the currently running scan for a library source (if any)
pub async fn get_running(pool: &SqlitePool, library_source_id: i64) -> Result<Option<ScanProgress>> {
    let row = sqlx::query!(
        r#"
        SELECT id, library_source_id, started_at, completed_at, total_files,
               processed_files, new_files, updated_files, removed_files,
               errors, status, error_message
        FROM scan_progress
        WHERE library_source_id = ? AND status = 'running'
        ORDER BY started_at DESC
        LIMIT 1
        "#,
        library_source_id
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| ScanProgress {
        id: r.id.expect("scan_progress id cannot be null"),
        library_source_id: r.library_source_id,
        started_at: r.started_at,
        completed_at: r.completed_at,
        total_files: r.total_files,
        processed_files: r.processed_files,
        new_files: r.new_files,
        updated_files: r.updated_files,
        removed_files: r.removed_files,
        errors: r.errors,
        status: ScanProgressStatus::from_str(&r.status).unwrap_or(ScanProgressStatus::Running),
        error_message: r.error_message,
    }))
}

/// Get the most recent scan for a library source
pub async fn get_latest(pool: &SqlitePool, library_source_id: i64) -> Result<Option<ScanProgress>> {
    let row = sqlx::query!(
        r#"
        SELECT id, library_source_id, started_at, completed_at, total_files,
               processed_files, new_files, updated_files, removed_files,
               errors, status, error_message
        FROM scan_progress
        WHERE library_source_id = ?
        ORDER BY started_at DESC, id DESC
        LIMIT 1
        "#,
        library_source_id
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| ScanProgress {
        id: r.id.expect("scan_progress id cannot be null"),
        library_source_id: r.library_source_id,
        started_at: r.started_at,
        completed_at: r.completed_at,
        total_files: r.total_files,
        processed_files: r.processed_files,
        new_files: r.new_files,
        updated_files: r.updated_files,
        removed_files: r.removed_files,
        errors: r.errors,
        status: ScanProgressStatus::from_str(&r.status).unwrap_or(ScanProgressStatus::Running),
        error_message: r.error_message,
    }))
}

/// Start a new scan for a library source
pub async fn start(
    pool: &SqlitePool,
    library_source_id: i64,
    total_files: Option<i64>,
) -> Result<ScanProgress> {
    let now = chrono::Utc::now().timestamp();

    let result = sqlx::query!(
        r#"
        INSERT INTO scan_progress (library_source_id, started_at, total_files, status)
        VALUES (?, ?, ?, 'running')
        "#,
        library_source_id,
        now,
        total_files
    )
    .execute(pool)
    .await?;

    let id = result.last_insert_rowid();

    Ok(ScanProgress {
        id,
        library_source_id,
        started_at: now,
        completed_at: None,
        total_files,
        processed_files: 0,
        new_files: 0,
        updated_files: 0,
        removed_files: 0,
        errors: 0,
        status: ScanProgressStatus::Running,
        error_message: None,
    })
}

/// Set the total file count (if discovered after scan started)
pub async fn set_total_files(pool: &SqlitePool, id: i64, total: i64) -> Result<()> {
    sqlx::query!(
        "UPDATE scan_progress SET total_files = ? WHERE id = ?",
        total,
        id
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Increment the processed files count
pub async fn increment_processed(pool: &SqlitePool, id: i64, count: i64) -> Result<()> {
    sqlx::query!(
        "UPDATE scan_progress SET processed_files = processed_files + ? WHERE id = ?",
        count,
        id
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Increment the new files count
pub async fn increment_new(pool: &SqlitePool, id: i64, count: i64) -> Result<()> {
    sqlx::query!(
        "UPDATE scan_progress SET new_files = new_files + ? WHERE id = ?",
        count,
        id
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Increment the updated files count
pub async fn increment_updated(pool: &SqlitePool, id: i64, count: i64) -> Result<()> {
    sqlx::query!(
        "UPDATE scan_progress SET updated_files = updated_files + ? WHERE id = ?",
        count,
        id
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Increment the removed files count
pub async fn increment_removed(pool: &SqlitePool, id: i64, count: i64) -> Result<()> {
    sqlx::query!(
        "UPDATE scan_progress SET removed_files = removed_files + ? WHERE id = ?",
        count,
        id
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Increment the error count
pub async fn increment_errors(pool: &SqlitePool, id: i64, count: i64) -> Result<()> {
    sqlx::query!(
        "UPDATE scan_progress SET errors = errors + ? WHERE id = ?",
        count,
        id
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Mark a scan as completed
pub async fn complete(pool: &SqlitePool, id: i64) -> Result<()> {
    let now = chrono::Utc::now().timestamp();

    sqlx::query!(
        "UPDATE scan_progress SET status = 'completed', completed_at = ? WHERE id = ?",
        now,
        id
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Mark a scan as failed
pub async fn fail(pool: &SqlitePool, id: i64, error_message: &str) -> Result<()> {
    let now = chrono::Utc::now().timestamp();

    sqlx::query!(
        "UPDATE scan_progress SET status = 'failed', completed_at = ?, error_message = ? WHERE id = ?",
        now,
        error_message,
        id
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Mark a scan as cancelled
pub async fn cancel(pool: &SqlitePool, id: i64) -> Result<()> {
    let now = chrono::Utc::now().timestamp();

    sqlx::query!(
        "UPDATE scan_progress SET status = 'cancelled', completed_at = ? WHERE id = ?",
        now,
        id
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Delete old scan progress records (keep only the last N per source)
pub async fn cleanup_old(pool: &SqlitePool, library_source_id: i64, keep_count: i64) -> Result<u64> {
    let result = sqlx::query!(
        r#"
        DELETE FROM scan_progress
        WHERE library_source_id = ?
          AND id NOT IN (
            SELECT id FROM scan_progress
            WHERE library_source_id = ?
            ORDER BY started_at DESC, id DESC
            LIMIT ?
          )
        "#,
        library_source_id,
        library_source_id,
        keep_count
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_progress_status_roundtrip() {
        for status in [
            ScanProgressStatus::Running,
            ScanProgressStatus::Completed,
            ScanProgressStatus::Failed,
            ScanProgressStatus::Cancelled,
        ] {
            let s = status.as_str();
            let parsed = ScanProgressStatus::from_str(s);
            assert_eq!(parsed, Some(status));
        }
    }

    #[test]
    fn test_invalid_status() {
        assert_eq!(ScanProgressStatus::from_str("invalid"), None);
        assert_eq!(ScanProgressStatus::from_str(""), None);
    }
}
