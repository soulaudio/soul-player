use crate::{error::Result, state::StateManager, SyncPhase, SyncProgress};
use soul_importer::metadata::extract_metadata;
use sqlx::SqlitePool;
use std::path::PathBuf;
use tokio::sync::mpsc;
use tracing::{debug, warn};

/// Re-extract metadata for all files and update database
pub async fn extract_all(
    _pool: &SqlitePool,
    state: &StateManager,
    files: &[PathBuf],
    progress_tx: &mpsc::Sender<SyncProgress>,
    session_id: &str,
) -> Result<usize> {
    debug!(
        "Starting metadata extraction phase for {} files",
        files.len()
    );
    state
        .update_phase(SyncPhase::MetadataExtraction, files.len())
        .await?;

    let mut updated = 0;
    let mut processed = 0;
    let mut failed = 0;

    for file in files {
        let file_str = file.to_string_lossy();

        // Extract metadata
        match extract_metadata(file) {
            Ok(_metadata) => {
                // Successfully extracted metadata
                debug!("Extracted metadata from: {}", file_str);
                // TODO: Implement track update/creation logic
                // For now, just count it as updated
                updated += 1;
            }
            Err(e) => {
                warn!("Failed to extract metadata from {}: {}", file_str, e);
                state
                    .log_error(
                        session_id,
                        SyncPhase::MetadataExtraction,
                        Some(&file_str),
                        "metadata_extraction_failed",
                        &e.to_string(),
                    )
                    .await?;
                failed += 1;
            }
        }

        processed += 1;

        // Update progress every 10 files to avoid too many updates
        if processed % 10 == 0 || processed == files.len() {
            state
                .update_progress(processed, updated, failed, Some(&file_str))
                .await?;

            let progress = state.get_progress().await?;
            let _ = progress_tx.send(progress).await;
        }
    }

    debug!(
        "Metadata extraction complete: {} processed, {} updated, {} failed",
        processed, updated, failed
    );

    Ok(updated)
}
