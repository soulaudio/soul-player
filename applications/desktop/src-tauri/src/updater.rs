use crate::app_state::AppState;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::UpdaterExt;

/// Start the background update checker
///
/// Checks for updates immediately on startup, then every hour if auto-update is enabled
pub fn start_update_checker(app: AppHandle) {
    let update_handle = tokio::spawn(async move {
        // Check immediately on startup (after a small delay to let app initialize)
        tokio::time::sleep(Duration::from_secs(3)).await;
        tracing::info!("[UPDATER] Starting initial update check on startup");
        check_and_handle_updates(&app).await;

        // Then check every hour
        let mut interval = tokio::time::interval(Duration::from_secs(3600));
        interval.tick().await; // First tick completes immediately, skip it

        loop {
            interval.tick().await;
            tracing::info!("[UPDATER] Running scheduled update check");
            check_and_handle_updates(&app).await;
        }
    });

    // Log errors from update checker (runs for app lifetime)
    tokio::spawn(async move {
        if let Err(e) = update_handle.await {
            tracing::error!("[UPDATER] Update checker task panicked: {:?}", e);
        }
    });
}

/// Internal helper to check for updates and handle the result
async fn check_and_handle_updates(app: &AppHandle) {
    // Check if auto-update is enabled in settings
    let state = app.state::<AppState>();
    let auto_update: Result<Option<serde_json::Value>, soul_storage::StorageError> =
        soul_storage::settings::get_setting(
            &state.pool,
            &state.user_id,
            soul_storage::settings::SETTING_AUTO_UPDATE_ENABLED,
        )
        .await;

    let auto_update_enabled = auto_update
        .ok()
        .flatten()
        .and_then(|v: serde_json::Value| v.as_bool())
        .unwrap_or(true);

    if !auto_update_enabled {
        tracing::debug!("[UPDATER] Auto-update disabled in settings, skipping check");
        return;
    }

    // Check for updates
    match app.updater() {
        Ok(updater) => match updater.check().await {
            Ok(Some(update)) => {
                tracing::info!("[UPDATER] Update available: v{}", update.version);

                let silent: Result<Option<serde_json::Value>, soul_storage::StorageError> =
                    soul_storage::settings::get_setting(
                        &state.pool,
                        &state.user_id,
                        soul_storage::settings::SETTING_AUTO_UPDATE_SILENT,
                    )
                    .await;

                let silent_mode = silent
                    .ok()
                    .flatten()
                    .and_then(|v: serde_json::Value| v.as_bool())
                    .unwrap_or(false);

                if silent_mode {
                    tracing::info!("[UPDATER] Starting silent install");
                    // Silent install
                    match update
                        .download_and_install(|_chunk_length, _content_length| {}, || {})
                        .await
                    {
                        Ok(()) => {
                            tracing::info!("[UPDATER] Silent install completed successfully");
                            tracing::info!("[UPDATER] Restarting app to apply update");
                            // Restart the app to apply the update (this will exit the process)
                            app.restart();
                        }
                        Err(e) => {
                            tracing::error!("[UPDATER] Silent install failed: {}", e);
                        }
                    }
                } else {
                    tracing::info!("[UPDATER] Emitting update-available event to frontend");
                    // Emit event to frontend for user prompt
                    let update_info = serde_json::json!({
                        "version": update.version,
                        "date": update.date,
                        "body": update.body
                    });
                    if let Err(e) = app.emit("update-available", &update_info) {
                        tracing::error!(error = %e, event = "update-available", "Failed to emit event to frontend");
                    }
                }
            }
            Ok(None) => {
                tracing::debug!("[UPDATER] No updates available");
            }
            Err(e) => {
                tracing::warn!("[UPDATER] Failed to check for updates: {}", e);
            }
        },
        Err(e) => {
            tracing::error!("[UPDATER] Failed to get updater instance: {}", e);
        }
    }
}

/// Tauri command to manually check for updates
#[tauri::command]
pub async fn check_for_updates(app: AppHandle) -> Result<Option<serde_json::Value>, String> {
    let updater = app
        .updater()
        .map_err(|e: tauri_plugin_updater::Error| e.to_string())?;

    match updater.check().await {
        Ok(Some(update)) => Ok(Some(serde_json::json!({
            "version": update.version,
            "date": update.date,
            "body": update.body
        }))),
        Ok(None) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

/// Tauri command to install an available update
#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), String> {
    let updater = app
        .updater()
        .map_err(|e: tauri_plugin_updater::Error| e.to_string())?;

    if let Some(update) = updater
        .check()
        .await
        .map_err(|e: tauri_plugin_updater::Error| e.to_string())?
    {
        let app_clone = app.clone();
        let restart_app = app.clone();
        update
            .download_and_install(
                move |chunk, total| {
                    let progress = if let Some(t) = total {
                        (chunk as f64 / t as f64 * 100.0) as u8
                    } else {
                        0
                    };
                    if let Err(e) = app_clone.emit("update-progress", progress) {
                        tracing::warn!(error = %e, event = "update-progress", "Failed to emit event to frontend");
                    }
                },
                || {},
            )
            .await
            .map_err(|e: tauri_plugin_updater::Error| e.to_string())?;

        tracing::info!("[UPDATER] Manual install completed, restarting app");
        // Restart the app to apply the update (this will exit the process)
        restart_app.restart();
    }

    Ok(())
}
