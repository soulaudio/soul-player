//! Fingerprint queue storage
//!
//! Manages the background queue for audio fingerprinting.
//!
//! # Example
//!
//! ```rust,no_run
//! use soul_storage::fingerprint_queue;
//!
//! # async fn example(pool: &sqlx::SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
//! // Add a track to the fingerprint queue
//! fingerprint_queue::enqueue(pool, "track-123", 0).await?;
//!
//! // Get next item to process
//! if let Some(item) = fingerprint_queue::get_next(pool).await? {
//!     // Process fingerprint...
//!     fingerprint_queue::complete(pool, item.id).await?;
//! }
//! # Ok(())
//! # }
//! ```

use crate::StorageError;
use sqlx::SqlitePool;

type Result<T> = std::result::Result<T, StorageError>;

/// A fingerprint queue item
#[derive(Debug, Clone)]
pub struct FingerprintQueueItem {
    pub id: i64,
    pub track_id: String,
    pub priority: i32,
    pub attempts: i32,
    pub last_error: Option<String>,
    pub created_at: i64,
}

/// Fingerprint queue statistics
#[derive(Debug, Clone, Default)]
pub struct FingerprintQueueStats {
    pub pending: i64,
    pub failed: i64,
    pub total_processed: i64,
}

/// Add a track to the fingerprint queue
pub async fn enqueue(pool: &SqlitePool, track_id: &str, priority: i32) -> Result<i64> {
    let result = sqlx::query!(
        r#"
        INSERT INTO fingerprint_queue (track_id, priority)
        VALUES (?, ?)
        ON CONFLICT(track_id) DO UPDATE SET priority = MAX(priority, excluded.priority)
        "#,
        track_id,
        priority
    )
    .execute(pool)
    .await?;

    Ok(result.last_insert_rowid())
}

/// Add multiple tracks to the fingerprint queue
pub async fn enqueue_batch(pool: &SqlitePool, track_ids: &[&str], priority: i32) -> Result<i64> {
    let mut count = 0i64;

    for track_id in track_ids {
        sqlx::query!(
            r#"
            INSERT INTO fingerprint_queue (track_id, priority)
            VALUES (?, ?)
            ON CONFLICT(track_id) DO NOTHING
            "#,
            track_id,
            priority
        )
        .execute(pool)
        .await?;
        count += 1;
    }

    Ok(count)
}

/// Get the next item to process (highest priority, oldest first)
pub async fn get_next(pool: &SqlitePool) -> Result<Option<FingerprintQueueItem>> {
    let row = sqlx::query!(
        r#"
        SELECT id, track_id, priority, attempts, last_error, created_at
        FROM fingerprint_queue
        WHERE attempts < 3
        ORDER BY priority DESC, created_at ASC
        LIMIT 1
        "#
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| FingerprintQueueItem {
        id: r.id.expect("id should not be null"),
        track_id: r.track_id,
        priority: r.priority as i32,
        attempts: r.attempts as i32,
        last_error: r.last_error,
        created_at: r.created_at,
    }))
}

/// Get a batch of items to process
pub async fn get_batch(pool: &SqlitePool, limit: i32) -> Result<Vec<FingerprintQueueItem>> {
    let rows = sqlx::query!(
        r#"
        SELECT id, track_id, priority, attempts, last_error, created_at
        FROM fingerprint_queue
        WHERE attempts < 3
        ORDER BY priority DESC, created_at ASC
        LIMIT ?
        "#,
        limit
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| FingerprintQueueItem {
            id: r.id.expect("id should not be null"),
            track_id: r.track_id,
            priority: r.priority as i32,
            attempts: r.attempts as i32,
            last_error: r.last_error,
            created_at: r.created_at,
        })
        .collect())
}

/// Mark an item as completed (remove from queue)
pub async fn complete(pool: &SqlitePool, id: i64) -> Result<()> {
    sqlx::query!("DELETE FROM fingerprint_queue WHERE id = ?", id)
        .execute(pool)
        .await?;

    Ok(())
}

/// Mark an item as failed (increment attempts, record error)
pub async fn fail(pool: &SqlitePool, id: i64, error: &str) -> Result<()> {
    sqlx::query!(
        r#"
        UPDATE fingerprint_queue
        SET attempts = attempts + 1, last_error = ?
        WHERE id = ?
        "#,
        error,
        id
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Remove a track from the queue (e.g., if track was deleted)
pub async fn remove(pool: &SqlitePool, track_id: &str) -> Result<()> {
    sqlx::query!("DELETE FROM fingerprint_queue WHERE track_id = ?", track_id)
        .execute(pool)
        .await?;

    Ok(())
}

/// Get queue statistics
pub async fn get_stats(pool: &SqlitePool) -> Result<FingerprintQueueStats> {
    let pending =
        sqlx::query_scalar!("SELECT COUNT(*) as count FROM fingerprint_queue WHERE attempts < 3")
            .fetch_one(pool)
            .await?;

    let failed =
        sqlx::query_scalar!("SELECT COUNT(*) as count FROM fingerprint_queue WHERE attempts >= 3")
            .fetch_one(pool)
            .await?;

    Ok(FingerprintQueueStats {
        pending: pending as i64,
        failed: failed as i64,
        total_processed: 0, // Would need a separate counter table
    })
}

/// Get count of pending items
pub async fn pending_count(pool: &SqlitePool) -> Result<i64> {
    let count =
        sqlx::query_scalar!("SELECT COUNT(*) as count FROM fingerprint_queue WHERE attempts < 3")
            .fetch_one(pool)
            .await?;

    Ok(count as i64)
}

/// Clear all failed items (attempts >= 3)
pub async fn clear_failed(pool: &SqlitePool) -> Result<u64> {
    let result = sqlx::query!("DELETE FROM fingerprint_queue WHERE attempts >= 3")
        .execute(pool)
        .await?;

    Ok(result.rows_affected())
}

/// Reset failed items to retry
pub async fn retry_failed(pool: &SqlitePool) -> Result<u64> {
    let result = sqlx::query!(
        "UPDATE fingerprint_queue SET attempts = 0, last_error = NULL WHERE attempts >= 3"
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fingerprint_queue_stats_default() {
        let stats = FingerprintQueueStats::default();
        assert_eq!(stats.pending, 0);
        assert_eq!(stats.failed, 0);
    }
}
