use soul_sync::{SyncManager, SyncProgress};
use sqlx::SqlitePool;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};

/// Sync state managed by Tauri
pub struct SyncState {
    pub manager: Arc<SyncManager>,
    pub pool: SqlitePool,
}

impl SyncState {
    pub fn new(pool: SqlitePool) -> Self {
        let manager = Arc::new(SyncManager::new(pool.clone()));
        Self { manager, pool }
    }
}

#[tauri::command]
pub async fn start_sync(
    app: AppHandle,
    trigger: String,
    state: State<'_, crate::lazy_workers::LazySyncState>,
) -> Result<(), String> {
    let trigger_enum = match trigger.as_str() {
        "manual" => soul_sync::SyncTrigger::Manual,
        "migration" => soul_sync::SyncTrigger::SchemaMigration,
        "source_activation" => soul_sync::SyncTrigger::SourceActivation,
        _ => return Err(format!("Invalid trigger: {}", trigger)),
    };

    let sync_state = state.get();
    let state_guard = sync_state.lock().await;
    let (mut progress_rx, handle) = state_guard
        .manager
        .start_sync(trigger_enum)
        .await
        .map_err(|e| e.to_string())?;

    // Spawn task to forward progress
    let app_clone = app.clone();
    let progress_handle = tokio::spawn(async move {
        while let Some(progress) = progress_rx.recv().await {
            if let Err(e) = app_clone.emit("sync-progress", progress) {
                tracing::warn!(error = %e, event = "sync-progress", "Failed to emit event to frontend");
            }
        }
    });

    // Log errors from progress forwarder
    tokio::spawn(async move {
        if let Err(e) = progress_handle.await {
            tracing::error!("[SYNC] Progress forwarder task panicked: {:?}", e);
        }
    });

    // Wait for completion in background
    let completion_handle = tokio::spawn(async move {
        match handle.await {
            Ok(Ok(summary)) => {
                if let Err(e) = app.emit("sync-complete", summary) {
                    tracing::error!(error = %e, event = "sync-complete", "Failed to emit event to frontend");
                }
            }
            Ok(Err(e)) => {
                if let Err(e) = app.emit("sync-error", e.to_string()) {
                    tracing::error!(error = %e, event = "sync-error", "Failed to emit event to frontend");
                }
            }
            Err(e) => {
                if let Err(e) = app.emit("sync-error", format!("Task panicked: {}", e)) {
                    tracing::error!(error = %e, event = "sync-error", "Failed to emit event to frontend");
                }
            }
        }
    });

    // Log errors from completion handler
    tokio::spawn(async move {
        if let Err(e) = completion_handle.await {
            tracing::error!("[SYNC] Completion handler task panicked: {:?}", e);
        }
    });

    Ok(())
}

#[tauri::command]
pub async fn get_sync_status(
    state: State<'_, crate::lazy_workers::LazySyncState>,
) -> Result<SyncProgress, String> {
    let sync_state = state.get();
    let state_guard = sync_state.lock().await;
    state_guard
        .manager
        .get_status()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn cancel_sync(
    state: State<'_, crate::lazy_workers::LazySyncState>,
) -> Result<(), String> {
    let sync_state = state.get();
    let state_guard = sync_state.lock().await;
    state_guard
        .manager
        .cancel_sync()
        .await
        .map_err(|e| e.to_string())
}

#[derive(serde::Serialize)]
pub struct SyncErrorRecord {
    pub id: i64,
    pub session_id: String,
    pub phase: String,
    pub error_message: String,
}

#[tauri::command]
pub async fn get_sync_errors(
    _session_id: Option<String>,
    _state: State<'_, crate::lazy_workers::LazySyncState>,
) -> Result<Vec<SyncErrorRecord>, String> {
    // TODO: Implement sync error retrieval
    // For now, return empty vector as this is not critical for MVP
    Ok(Vec::new())
}
