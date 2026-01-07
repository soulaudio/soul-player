use crate::app_state::AppState;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::UpdaterExt;

/// Start the background update checker
///
/// Checks for updates every hour if auto-update is enabled in settings
pub fn start_update_checker(app: AppHandle) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(3600)); // Check every hour

        loop {
            interval.tick().await;

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
                continue;
            }

            // Check for updates
            if let Ok(updater) = app.updater() {
                if let Ok(Some(update)) = updater.check().await {
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
                        // Silent install
                        let _install_result: Result<(), tauri_plugin_updater::Error> = update
                            .download_and_install(
                                |_chunk_length, _content_length| {},
                                || {}
                            )
                            .await;
                    } else {
                        // Emit event to frontend for user prompt
                        let update_info = serde_json::json!({
                            "version": update.version,
                            "date": update.date,
                            "body": update.body
                        });
                        let _ = app.emit("update-available", &update_info);
                    }
                }
            }
        }
    });
}

/// Tauri command to manually check for updates
#[tauri::command]
pub async fn check_for_updates(
    app: AppHandle,
) -> Result<Option<serde_json::Value>, String> {
    let updater = app.updater().map_err(|e: tauri_plugin_updater::Error| e.to_string())?;

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
    let updater = app.updater().map_err(|e: tauri_plugin_updater::Error| e.to_string())?;

    if let Some(update) = updater
        .check()
        .await
        .map_err(|e: tauri_plugin_updater::Error| e.to_string())?
    {
        let app_clone = app.clone();
        update
            .download_and_install(
                move |chunk, total| {
                    let progress = if let Some(t) = total {
                        (chunk as f64 / t as f64 * 100.0) as u8
                    } else {
                        0
                    };
                    let _ = app_clone.emit("update-progress", progress);
                },
                || {}
            )
            .await
            .map_err(|e: tauri_plugin_updater::Error| e.to_string())?;
    }

    Ok(())
}
