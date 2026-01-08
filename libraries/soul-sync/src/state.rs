use crate::{error::Result, SyncPhase, SyncProgress, SyncStatus};
use sqlx::SqlitePool;

/// Manages sync state persistence in the database
pub struct StateManager {
    pool: SqlitePool,
}

impl StateManager {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Check if sync is currently running
    pub async fn is_syncing(&self) -> Result<bool> {
        let row = sqlx::query!(
            "SELECT status FROM sync_state WHERE id = 1"
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(row.status != "idle")
    }

    /// Initialize a new sync session
    pub async fn start_sync(&self, _session_id: String) -> Result<()> {
        sqlx::query!(
            "UPDATE sync_state SET 
                status = 'scanning',
                phase = 'scanning',
                total_items = 0,
                processed_items = 0,
                successful_items = 0,
                failed_items = 0,
                current_item = NULL,
                started_at = datetime('now'),
                completed_at = NULL,
                last_updated_at = datetime('now'),
                error_message = NULL
            WHERE id = 1"
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Update the current phase of the sync
    pub async fn update_phase(&self, phase: SyncPhase, total_items: usize) -> Result<()> {
        let (status, phase_str) = match phase {
            SyncPhase::Scanning => ("scanning", "scanning"),
            SyncPhase::MetadataExtraction => ("extracting", "metadata_extraction"),
            SyncPhase::Validation => ("validating", "validation"),
            SyncPhase::Cleanup => ("cleaning", "cleanup"),
        };

        let total_items_i64 = total_items as i64;

        sqlx::query!(
            "UPDATE sync_state SET
                status = ?,
                phase = ?,
                total_items = ?,
                processed_items = 0,
                current_item = NULL,
                last_updated_at = datetime('now')
            WHERE id = 1",
            status,
            phase_str,
            total_items_i64
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Update progress counters
    pub async fn update_progress(
        &self,
        processed: usize,
        successful: usize,
        failed: usize,
        current_item: Option<&str>,
    ) -> Result<()> {
        let processed_i64 = processed as i64;
        let successful_i64 = successful as i64;
        let failed_i64 = failed as i64;

        sqlx::query!(
            "UPDATE sync_state SET
                processed_items = ?,
                successful_items = ?,
                failed_items = ?,
                current_item = ?,
                last_updated_at = datetime('now')
            WHERE id = 1",
            processed_i64,
            successful_i64,
            failed_i64,
            current_item
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Log an error that occurred during sync
    pub async fn log_error(
        &self,
        session_id: &str,
        phase: SyncPhase,
        item_path: Option<&str>,
        error_type: &str,
        error_message: &str,
    ) -> Result<()> {
        let phase_str = match phase {
            SyncPhase::Scanning => "scanning",
            SyncPhase::MetadataExtraction => "metadata_extraction",
            SyncPhase::Validation => "validation",
            SyncPhase::Cleanup => "cleanup",
        };

        sqlx::query!(
            "INSERT INTO sync_errors (sync_session_id, phase, item_path, error_type, error_message)
             VALUES (?, ?, ?, ?, ?)",
            session_id,
            phase_str,
            item_path,
            error_type,
            error_message
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Mark sync as failed with error
    pub async fn mark_error(&self, error_message: &str) -> Result<()> {
        sqlx::query!(
            "UPDATE sync_state SET 
                status = 'error',
                error_message = ?,
                completed_at = datetime('now'),
                last_updated_at = datetime('now')
            WHERE id = 1",
            error_message
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Mark sync as complete
    pub async fn complete_sync(&self) -> Result<()> {
        sqlx::query!(
            "UPDATE sync_state SET 
                status = 'idle',
                phase = NULL,
                current_item = NULL,
                completed_at = datetime('now'),
                last_updated_at = datetime('now')
            WHERE id = 1"
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get current sync progress
    pub async fn get_progress(&self) -> Result<SyncProgress> {
        let row = sqlx::query!(
            "SELECT status, phase, total_items, processed_items, successful_items, failed_items, current_item
             FROM sync_state WHERE id = 1"
        )
        .fetch_one(&self.pool)
        .await?;

        let status = match row.status.as_str() {
            "scanning" => SyncStatus::Scanning,
            "extracting" => SyncStatus::Extracting,
            "validating" => SyncStatus::Validating,
            "cleaning" => SyncStatus::Cleaning,
            "error" => SyncStatus::Error,
            _ => SyncStatus::Idle,
        };

        let phase = row.phase.and_then(|p| match p.as_str() {
            "scanning" => Some(SyncPhase::Scanning),
            "metadata_extraction" => Some(SyncPhase::MetadataExtraction),
            "validation" => Some(SyncPhase::Validation),
            "cleanup" => Some(SyncPhase::Cleanup),
            _ => None,
        });

        let total = row.total_items.unwrap_or(0) as usize;
        let processed = row.processed_items.unwrap_or(0) as usize;
        let percentage = if total > 0 {
            (processed as f32 / total as f32) * 100.0
        } else {
            0.0
        };

        Ok(SyncProgress {
            status,
            phase,
            total_items: total,
            processed_items: processed,
            successful_items: row.successful_items.unwrap_or(0) as usize,
            failed_items: row.failed_items.unwrap_or(0) as usize,
            current_item: row.current_item,
            percentage,
        })
    }

    /// Get the last known migration version
    pub async fn get_last_known_migration(&self) -> Result<String> {
        let row = sqlx::query!(
            "SELECT last_migration_version FROM sync_state WHERE id = 1"
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(row.last_migration_version.unwrap_or_default())
    }

    /// Record the current migration version
    pub async fn record_migration_version(&self, version: &str) -> Result<()> {
        sqlx::query!(
            "UPDATE sync_state SET last_migration_version = ? WHERE id = 1",
            version
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get count of errors for a session
    pub async fn get_error_count(&self, session_id: &str) -> Result<usize> {
        let row = sqlx::query!(
            "SELECT COUNT(*) as count FROM sync_errors WHERE sync_session_id = ?",
            session_id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(row.count as usize)
    }
}
