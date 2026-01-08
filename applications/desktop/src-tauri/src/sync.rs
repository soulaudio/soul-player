use soul_sync::{SyncManager, SyncProgress};
use sqlx::SqlitePool;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;

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
    state: State<'_, Arc<Mutex<SyncState>>>,
) -> Result<(), String> {
    let trigger_enum = match trigger.as_str() {
        "manual" => soul_sync::SyncTrigger::Manual,
        "migration" => soul_sync::SyncTrigger::SchemaMigration,
        "source_activation" => soul_sync::SyncTrigger::SourceActivation,
        _ => return Err(format!("Invalid trigger: {}", trigger)),
    };

    let state_guard = state.lock().await;
    let (mut progress_rx, handle) = state_guard
        .manager
        .start_sync(trigger_enum)
        .await
        .map_err(|e| e.to_string())?;

    // Spawn task to forward progress
    let app_clone = app.clone();
    tokio::spawn(async move {
        while let Some(progress) = progress_rx.recv().await {
            let _ = app_clone.emit("sync-progress", progress);
        }
    });

    // Wait for completion in background
    tokio::spawn(async move {
        match handle.await {
            Ok(Ok(summary)) => {
                let _ = app.emit("sync-complete", summary);
            }
            Ok(Err(e)) => {
                let _ = app.emit("sync-error", e.to_string());
            }
            Err(e) => {
                let _ = app.emit("sync-error", format!("Task panicked: {}", e));
            }
        }
    });

    Ok(())
}

#[tauri::command]
pub async fn get_sync_status(
    state: State<'_, Arc<Mutex<SyncState>>>,
) -> Result<SyncProgress, String> {
    let state_guard = state.lock().await;
    state_guard
        .manager
        .get_status()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn cancel_sync(state: State<'_, Arc<Mutex<SyncState>>>) -> Result<(), String> {
    let state_guard = state.lock().await;
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
    _state: State<'_, Arc<Mutex<SyncState>>>,
) -> Result<Vec<SyncErrorRecord>, String> {
    // TODO: Implement sync error retrieval
    // For now, return empty vector as this is not critical for MVP
    Ok(Vec::new())
}
