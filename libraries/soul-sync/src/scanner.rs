use crate::{error::Result, state::StateManager, SyncPhase, SyncProgress};
use soul_importer::scanner::FileScanner;
use sqlx::SqlitePool;
use std::path::PathBuf;
use tokio::sync::mpsc;
use tracing::{debug, warn};

/// Scan all active sources for audio files
pub async fn scan_all_sources(
    pool: &SqlitePool,
    state: &StateManager,
    progress_tx: &mpsc::Sender<SyncProgress>,
) -> Result<Vec<PathBuf>> {
    debug!("Starting file scan phase");
    state.update_phase(SyncPhase::Scanning, 0).await?;

    // Emit initial progress
    let progress = state.get_progress().await?;
    let _ = progress_tx.send(progress).await;

    // Get all active sources
    let sources = get_active_sources(pool).await?;
    debug!("Found {} active sources", sources.len());

    let scanner = FileScanner::new();
    let mut all_files = Vec::new();

    for source in sources {
        debug!("Scanning source: {}", source.name);

        // For local sources, get library path from settings
        if source.source_type == "local" {
            if let Some(library_path) = get_library_path(pool).await? {
                debug!("Scanning library path: {}", library_path.display());

                match scanner.scan_directory(&library_path) {
                    Ok(files) => {
                        debug!("Found {} files in {}", files.len(), library_path.display());
                        all_files.extend(files);
                    }
                    Err(e) => {
                        warn!("Error scanning {}: {}", library_path.display(), e);
                        // Continue with other sources even if one fails
                    }
                }
            } else {
                warn!("No library path configured for local source");
            }
        }
        // TODO: Handle remote server sources in the future
    }

    debug!("Scan complete: found {} total files", all_files.len());

    // Update phase with total count
    state
        .update_phase(SyncPhase::Scanning, all_files.len())
        .await?;

    // Emit progress
    let progress = state.get_progress().await?;
    let _ = progress_tx.send(progress).await;

    Ok(all_files)
}

/// Helper to get active sources
async fn get_active_sources(pool: &SqlitePool) -> Result<Vec<SourceInfo>> {
    let rows = sqlx::query!("SELECT id, name, source_type FROM sources WHERE is_active = 1")
        .fetch_all(pool)
        .await?;

    Ok(rows
        .into_iter()
        .map(|row| SourceInfo {
            _id: row.id,
            name: row.name,
            source_type: row.source_type,
        })
        .collect())
}

/// Helper to get library path from settings
async fn get_library_path(pool: &SqlitePool) -> Result<Option<PathBuf>> {
    // Get library path for default user (user_id = '1')
    let row = sqlx::query!(
        "SELECT value FROM user_settings
         WHERE user_id = '1' AND key = 'import.library_path'"
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| PathBuf::from(r.value)))
}

#[derive(Debug)]
struct SourceInfo {
    _id: i64,
    name: String,
    source_type: String,
}
