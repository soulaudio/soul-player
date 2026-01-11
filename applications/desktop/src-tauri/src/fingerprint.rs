//! Background fingerprinting worker
//!
//! Processes the fingerprint queue in the background, computing audio
//! fingerprints using Chromaprint and storing them in the database.
//!
//! Fingerprints are AcoustID-compatible and can be used for:
//! - Duplicate detection (same audio, different files)
//! - Cross-format matching (FLAC vs MP3 of same song)
//! - File relocation tracking

use crate::app_state::AppState;
use serde::{Deserialize, Serialize};
use soul_audio::fingerprint::{FingerprintConfig, Fingerprinter};
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::Mutex;

/// Fingerprint worker state
pub struct FingerprintWorker {
    /// Whether the worker is currently running
    running: AtomicBool,
    /// Number of items processed in current session
    processed_count: AtomicI64,
    /// Last error message
    last_error: Mutex<Option<String>>,
    /// Cancellation token
    cancel: AtomicBool,
}

impl FingerprintWorker {
    pub fn new() -> Self {
        Self {
            running: AtomicBool::new(false),
            processed_count: AtomicI64::new(0),
            last_error: Mutex::new(None),
            cancel: AtomicBool::new(false),
        }
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    pub fn processed_count(&self) -> i64 {
        self.processed_count.load(Ordering::SeqCst)
    }

    pub fn request_cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::SeqCst)
    }

    pub fn reset(&self) {
        self.running.store(false, Ordering::SeqCst);
        self.cancel.store(false, Ordering::SeqCst);
        self.processed_count.store(0, Ordering::SeqCst);
    }
}

impl Default for FingerprintWorker {
    fn default() -> Self {
        Self::new()
    }
}

/// Fingerprinting status for frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FingerprintStatus {
    pub is_running: bool,
    pub pending_count: i64,
    pub failed_count: i64,
    pub processed_this_session: i64,
    pub last_error: Option<String>,
}

/// Get fingerprinting status
#[tauri::command]
pub async fn get_fingerprint_status(
    state: State<'_, AppState>,
    worker: State<'_, Arc<FingerprintWorker>>,
) -> Result<FingerprintStatus, String> {
    let stats = soul_storage::fingerprint_queue::get_stats(&state.pool)
        .await
        .map_err(|e| e.to_string())?;

    let last_error = worker.last_error.lock().await.clone();

    Ok(FingerprintStatus {
        is_running: worker.is_running(),
        pending_count: stats.pending,
        failed_count: stats.failed,
        processed_this_session: worker.processed_count(),
        last_error,
    })
}

/// Start the fingerprinting worker
#[tauri::command]
pub async fn start_fingerprinting(
    app: AppHandle,
    state: State<'_, AppState>,
    worker: State<'_, Arc<FingerprintWorker>>,
) -> Result<(), String> {
    if worker.is_running() {
        return Err("Fingerprinting already in progress".to_string());
    }

    worker.reset();
    worker.running.store(true, Ordering::SeqCst);

    let pool = (*state.pool).clone();
    let worker_clone = Arc::clone(&worker);

    // Spawn background task
    tokio::spawn(async move {
        run_fingerprint_worker(app, pool, worker_clone).await;
    });

    Ok(())
}

/// Stop the fingerprinting worker
#[tauri::command]
pub async fn stop_fingerprinting(worker: State<'_, Arc<FingerprintWorker>>) -> Result<(), String> {
    if !worker.is_running() {
        return Ok(());
    }

    worker.request_cancel();
    Ok(())
}

/// Retry failed fingerprints
#[tauri::command]
pub async fn retry_failed_fingerprints(state: State<'_, AppState>) -> Result<u64, String> {
    soul_storage::fingerprint_queue::retry_failed(&state.pool)
        .await
        .map_err(|e| e.to_string())
}

/// Clear failed fingerprints
#[tauri::command]
pub async fn clear_failed_fingerprints(state: State<'_, AppState>) -> Result<u64, String> {
    soul_storage::fingerprint_queue::clear_failed(&state.pool)
        .await
        .map_err(|e| e.to_string())
}

/// Compare two fingerprints and return similarity score
///
/// Returns a score from 0.0 (completely different) to 1.0 (identical).
/// Tracks are considered duplicates if similarity >= 0.85.
#[tauri::command]
pub async fn compare_fingerprints(
    fingerprint_a: String,
    fingerprint_b: String,
) -> Result<FingerprintComparison, String> {
    use soul_audio::fingerprint::FingerprintResult;

    // Decode fingerprints
    let fp_a = FingerprintResult::from_base64(&fingerprint_a, 0.0)
        .map_err(|e| format!("Invalid fingerprint A: {}", e))?;
    let fp_b = FingerprintResult::from_base64(&fingerprint_b, 0.0)
        .map_err(|e| format!("Invalid fingerprint B: {}", e))?;

    let similarity = fp_a.similarity(&fp_b);
    let is_match = similarity >= 0.85;

    Ok(FingerprintComparison {
        similarity,
        is_match,
    })
}

/// Find potential duplicates for a track by comparing fingerprints
#[tauri::command]
pub async fn find_duplicates(
    state: State<'_, AppState>,
    track_id: String,
    threshold: Option<f64>,
) -> Result<Vec<DuplicateMatch>, String> {
    use soul_audio::fingerprint::FingerprintResult;
    use soul_core::types::TrackId;

    let threshold = threshold.unwrap_or(0.85);

    // Get the source track
    let source_track = soul_storage::tracks::get_by_id(&state.pool, TrackId::new(track_id.clone()))
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Track {} not found", track_id))?;

    let source_fingerprint = source_track
        .fingerprint
        .as_ref()
        .ok_or_else(|| format!("Track {} has no fingerprint", track_id))?;

    let source_fp = FingerprintResult::from_base64(source_fingerprint, 0.0)
        .map_err(|e| format!("Invalid source fingerprint: {}", e))?;

    // Get all tracks with fingerprints
    let all_tracks = soul_storage::tracks::get_with_fingerprints(&state.pool)
        .await
        .map_err(|e| e.to_string())?;

    let mut duplicates = Vec::new();

    for track in all_tracks {
        // Skip the source track
        if track.id.as_str() == track_id {
            continue;
        }

        if let Some(ref fp_str) = track.fingerprint {
            if let Ok(fp) = FingerprintResult::from_base64(fp_str, 0.0) {
                let similarity = source_fp.similarity(&fp);
                if similarity >= threshold {
                    duplicates.push(DuplicateMatch {
                        track_id: track.id.as_str().to_string(),
                        title: track.title.clone(),
                        artist: track.artist_name.clone(),
                        similarity,
                    });
                }
            }
        }
    }

    // Sort by similarity (highest first)
    duplicates.sort_by(|a, b| {
        b.similarity
            .partial_cmp(&a.similarity)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(duplicates)
}

/// Result of comparing two fingerprints
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FingerprintComparison {
    /// Similarity score (0.0 to 1.0)
    pub similarity: f64,
    /// Whether the tracks are considered duplicates (similarity >= 0.85)
    pub is_match: bool,
}

/// A potential duplicate match
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateMatch {
    pub track_id: String,
    pub title: String,
    pub artist: Option<String>,
    pub similarity: f64,
}

/// Background worker that processes the fingerprint queue
async fn run_fingerprint_worker(
    app: AppHandle,
    pool: sqlx::SqlitePool,
    worker: Arc<FingerprintWorker>,
) {
    tracing::info!("Fingerprint worker started");

    // Emit initial status
    let _ = app.emit("fingerprint-started", ());

    loop {
        // Check for cancellation
        if worker.is_cancelled() {
            tracing::info!("Fingerprint worker cancelled");
            break;
        }

        // Get next item to process
        let item = match soul_storage::fingerprint_queue::get_next(&pool).await {
            Ok(Some(item)) => item,
            Ok(None) => {
                // No more items, worker is done
                tracing::info!("Fingerprint queue empty, worker stopping");
                break;
            }
            Err(e) => {
                tracing::error!("Failed to get fingerprint queue item: {}", e);
                *worker.last_error.lock().await = Some(e.to_string());
                // Wait a bit before retrying
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                continue;
            }
        };

        // Process the item
        match process_fingerprint(&pool, &item).await {
            Ok(fingerprint) => {
                // Update track with fingerprint
                if let Err(e) =
                    soul_storage::tracks::set_fingerprint(&pool, &item.track_id, &fingerprint).await
                {
                    tracing::error!("Failed to save fingerprint for {}: {}", item.track_id, e);
                    let _ =
                        soul_storage::fingerprint_queue::fail(&pool, item.id, &e.to_string()).await;
                } else {
                    // Remove from queue
                    let _ = soul_storage::fingerprint_queue::complete(&pool, item.id).await;
                    worker.processed_count.fetch_add(1, Ordering::SeqCst);

                    // Emit progress event
                    let pending = soul_storage::fingerprint_queue::pending_count(&pool)
                        .await
                        .unwrap_or(0);
                    let _ = app.emit(
                        "fingerprint-progress",
                        FingerprintProgressEvent {
                            processed: worker.processed_count(),
                            pending,
                        },
                    );
                }
            }
            Err(e) => {
                tracing::warn!("Failed to fingerprint {}: {}", item.track_id, e);
                let _ = soul_storage::fingerprint_queue::fail(&pool, item.id, &e).await;
                *worker.last_error.lock().await = Some(e);
            }
        }

        // Small delay to avoid overwhelming the system
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    worker.running.store(false, Ordering::SeqCst);

    // Emit completion event
    let _ = app.emit(
        "fingerprint-complete",
        FingerprintCompleteEvent {
            processed: worker.processed_count(),
        },
    );

    tracing::info!(
        "Fingerprint worker stopped. Processed {} items.",
        worker.processed_count()
    );
}

/// Process a single fingerprint queue item
async fn process_fingerprint(
    pool: &sqlx::SqlitePool,
    item: &soul_storage::fingerprint_queue::FingerprintQueueItem,
) -> Result<String, String> {
    use soul_core::types::TrackId;
    use std::path::Path;

    // Get the track to find the file path
    let track_id = TrackId::new(item.track_id.clone());
    let track = soul_storage::tracks::get_by_id(pool, track_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Track {} not found", item.track_id))?;

    // Get file path from track availability
    let file_path = track
        .availability
        .iter()
        .find_map(|a| a.local_file_path.clone())
        .ok_or_else(|| format!("Track {} has no local file path", item.track_id))?;

    // Compute fingerprint using Chromaprint
    let path = Path::new(&file_path);
    if !path.exists() {
        return Err(format!("File not found: {}", file_path));
    }

    // Run fingerprinting in blocking task to avoid blocking async runtime
    let file_path_owned = file_path.clone();
    let result = tokio::task::spawn_blocking(move || {
        let fingerprinter = Fingerprinter::new(FingerprintConfig::default());
        fingerprinter.fingerprint_file(Path::new(&file_path_owned))
    })
    .await
    .map_err(|e| format!("Fingerprint task failed: {}", e))?
    .map_err(|e| format!("Fingerprint error: {}", e))?;

    // Encode fingerprint as base64 for storage
    Ok(result.to_base64())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct FingerprintProgressEvent {
    processed: i64,
    pending: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct FingerprintCompleteEvent {
    processed: i64,
}
