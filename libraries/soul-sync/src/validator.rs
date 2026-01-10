use crate::{error::Result, state::StateManager, SyncPhase, SyncProgress};
use sqlx::SqlitePool;
use std::path::Path;
use tokio::sync::mpsc;
use tracing::{debug, warn};

/// Validate library database records and repair where possible
pub async fn validate_library(
    pool: &SqlitePool,
    state: &StateManager,
    progress_tx: &mpsc::Sender<SyncProgress>,
    session_id: &str,
) -> Result<()> {
    debug!("Starting validation phase");
    state.update_phase(SyncPhase::Validation, 0).await?;

    let mut total_issues = 0;

    // Phase 1: Check for tracks with missing files
    total_issues += validate_track_files(pool, state, session_id).await?;

    // Phase 2: Validate foreign key references
    total_issues += validate_references(pool, state, session_id).await?;

    debug!("Validation complete: found {} issues", total_issues);

    let progress = state.get_progress().await?;
    let _ = progress_tx.send(progress).await;

    Ok(())
}

/// Check for tracks pointing to non-existent files
async fn validate_track_files(
    pool: &SqlitePool,
    state: &StateManager,
    session_id: &str,
) -> Result<usize> {
    debug!("Validating track file paths");

    let track_sources = sqlx::query!(
        "SELECT track_id, local_file_path FROM track_sources
         WHERE local_file_path IS NOT NULL"
    )
    .fetch_all(pool)
    .await?;

    let mut missing_count = 0;

    for ts in track_sources {
        if let Some(ref file_path) = ts.local_file_path {
            let path = Path::new(file_path);

            if !path.exists() {
                warn!("Track {} has missing file: {}", ts.track_id, file_path);

                state
                    .log_error(
                        session_id,
                        SyncPhase::Validation,
                        Some(file_path),
                        "file_missing",
                        &format!("Track file not found: {}", file_path),
                    )
                    .await?;

                missing_count += 1;

                // TODO: Mark track as unavailable or remove it
                // For now, just log the error
            }
        }
    }

    debug!("Found {} tracks with missing files", missing_count);

    Ok(missing_count)
}

/// Validate foreign key references
async fn validate_references(
    pool: &SqlitePool,
    state: &StateManager,
    session_id: &str,
) -> Result<usize> {
    debug!("Validating foreign key references");

    let mut issue_count = 0;

    // Check for orphaned track_stats (track_id doesn't exist)
    let orphaned_stats = sqlx::query!(
        "SELECT COUNT(*) as count FROM track_stats ts
         WHERE NOT EXISTS (SELECT 1 FROM tracks t WHERE t.id = ts.track_id)"
    )
    .fetch_one(pool)
    .await?;

    if orphaned_stats.count > 0 {
        warn!(
            "Found {} orphaned track_stats records",
            orphaned_stats.count
        );

        state
            .log_error(
                session_id,
                SyncPhase::Validation,
                None,
                "orphaned_stats",
                &format!(
                    "Found {} orphaned track_stats records",
                    orphaned_stats.count
                ),
            )
            .await?;

        issue_count += orphaned_stats.count as usize;
    }

    // Check for orphaned playlist_tracks
    let orphaned_playlist_tracks = sqlx::query!(
        "SELECT COUNT(*) as count FROM playlist_tracks pt
         WHERE NOT EXISTS (SELECT 1 FROM tracks t WHERE t.id = pt.track_id)"
    )
    .fetch_one(pool)
    .await?;

    if orphaned_playlist_tracks.count > 0 {
        warn!(
            "Found {} orphaned playlist_tracks records",
            orphaned_playlist_tracks.count
        );

        state
            .log_error(
                session_id,
                SyncPhase::Validation,
                None,
                "orphaned_playlist_tracks",
                &format!(
                    "Found {} orphaned playlist_tracks records",
                    orphaned_playlist_tracks.count
                ),
            )
            .await?;

        issue_count += orphaned_playlist_tracks.count as usize;
    }

    debug!("Found {} reference issues", issue_count);

    Ok(issue_count)
}
