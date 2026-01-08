use crate::{
    cleaner, error::Result, metadata, scanner, state::StateManager, validator, SyncError,
    SyncProgress, SyncSummary, SyncTrigger,
};
use sqlx::SqlitePool;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{debug, error, info};
use uuid::Uuid;

/// Main sync manager that orchestrates the sync/doctor process
pub struct SyncManager {
    pool: SqlitePool,
    state: StateManager,
}

impl SyncManager {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool: pool.clone(),
            state: StateManager::new(pool),
        }
    }

    /// Check if sync is needed (schema changed or source activated)
    pub async fn should_auto_sync(&self) -> Result<Option<SyncTrigger>> {
        // Check if migrations have run since last sync
        let last_migration = Self::get_last_applied_migration(&self.pool).await?;
        let recorded_migration = self.state.get_last_known_migration().await?;

        if !recorded_migration.is_empty() && last_migration != recorded_migration {
            info!("Schema change detected: {} -> {}", recorded_migration, last_migration);
            return Ok(Some(SyncTrigger::SchemaMigration));
        }

        Ok(None)
    }

    /// Start sync operation
    pub async fn start_sync(
        &self,
        trigger: SyncTrigger,
    ) -> Result<(
        mpsc::Receiver<SyncProgress>,
        JoinHandle<Result<SyncSummary>>,
    )> {
        // Check if already syncing
        if self.state.is_syncing().await? {
            return Err(SyncError::AlreadySyncing);
        }

        info!("Starting sync (trigger: {:?})", trigger);

        let (tx, rx) = mpsc::channel(100);
        let pool = self.pool.clone();
        let session_id = Uuid::new_v4().to_string();

        let handle = tokio::spawn(async move {
            Self::sync_impl(pool, session_id, trigger, tx).await
        });

        Ok((rx, handle))
    }

    /// Internal sync implementation
    async fn sync_impl(
        pool: SqlitePool,
        session_id: String,
        _trigger: SyncTrigger,
        progress_tx: mpsc::Sender<SyncProgress>,
    ) -> Result<SyncSummary> {
        let start_time = std::time::Instant::now();
        let state = StateManager::new(pool.clone());

        // Initialize sync state
        state.start_sync(session_id.clone()).await?;

        // Phase 1: Scanning
        debug!("Phase 1: Scanning files");
        let files = match scanner::scan_all_sources(&pool, &state, &progress_tx).await {
            Ok(f) => f,
            Err(e) => {
                error!("Scan phase failed: {}", e);
                state.mark_error(&e.to_string()).await?;
                return Err(e);
            }
        };

        // Phase 2: Metadata Extraction
        debug!("Phase 2: Extracting metadata");
        let updated_tracks = match metadata::extract_all(&pool, &state, &files, &progress_tx, &session_id).await {
            Ok(count) => count,
            Err(e) => {
                error!("Metadata extraction phase failed: {}", e);
                state.mark_error(&e.to_string()).await?;
                return Err(e);
            }
        };

        // Phase 3: Validation & Repair
        debug!("Phase 3: Validating library");
        if let Err(e) = validator::validate_library(&pool, &state, &progress_tx, &session_id).await {
            error!("Validation phase failed: {}", e);
            state.mark_error(&e.to_string()).await?;
            return Err(e);
        }

        // Phase 4: Cleanup Orphans
        debug!("Phase 4: Cleaning up orphans");
        let orphans_cleaned = match cleaner::cleanup_orphans(&pool, &state, &progress_tx, &session_id).await {
            Ok(count) => count,
            Err(e) => {
                error!("Cleanup phase failed: {}", e);
                state.mark_error(&e.to_string()).await?;
                return Err(e);
            }
        };

        // Update last known migration version
        let last_migration = Self::get_last_applied_migration(&pool).await?;
        state.record_migration_version(&last_migration).await?;

        // Complete sync
        let duration = start_time.elapsed().as_secs();
        let error_count = state.get_error_count(&session_id).await?;

        let summary = SyncSummary {
            session_id: session_id.clone(),
            started_at: chrono::Utc::now().to_rfc3339(),
            completed_at: chrono::Utc::now().to_rfc3339(),
            duration_seconds: duration,
            files_scanned: files.len(),
            tracks_updated: updated_tracks,
            errors_encountered: error_count,
            orphans_cleaned,
        };

        state.complete_sync().await?;

        info!(
            "Sync complete: {} files scanned, {} tracks updated, {} errors, {} orphans cleaned in {}s",
            summary.files_scanned,
            summary.tracks_updated,
            summary.errors_encountered,
            summary.orphans_cleaned,
            summary.duration_seconds
        );

        Ok(summary)
    }

    /// Cancel ongoing sync
    pub async fn cancel_sync(&self) -> Result<()> {
        if !self.state.is_syncing().await? {
            return Err(SyncError::NotSyncing);
        }

        info!("Cancelling sync");
        self.state.mark_error("Cancelled by user").await?;

        Ok(())
    }

    /// Get current sync status
    pub async fn get_status(&self) -> Result<SyncProgress> {
        self.state.get_progress().await
    }

    /// Get last applied migration version from _sqlx_migrations
    async fn get_last_applied_migration(pool: &SqlitePool) -> Result<String> {
        let row = sqlx::query!(
            "SELECT version FROM _sqlx_migrations
             WHERE success = 1
             ORDER BY version DESC
             LIMIT 1"
        )
        .fetch_one(pool)
        .await?;

        Ok(row.version.map(|v| v.to_string()).unwrap_or_default())
    }
}
