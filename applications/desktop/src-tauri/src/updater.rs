use crate::app_state::AppState;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::UpdaterExt;

// ── E2E test state (only active when PLAYWRIGHT_TEST_DIR is set) ──────────────

/// Canned response for check_for_updates in test mode.
/// `None` = not set (use real implementation).
/// `Some(None)` = no update available.
/// `Some(Some(value))` = update available with this info.
#[allow(clippy::option_option)]
static TEST_UPDATE_RESPONSE: Mutex<Option<Option<serde_json::Value>>> = Mutex::new(None);

/// When > 0, install_update sleeps for this many milliseconds then returns Ok(())
/// instead of running the real installer.  Lets tests observe the progress-bar
/// state before the operation completes.  0 = no delay (default).
static TEST_INSTALL_DELAY_MS: Mutex<u64> = Mutex::new(0);

/// When true, install_update returns Err in test mode (simulates install failure).
static TEST_INSTALL_SHOULD_FAIL: Mutex<bool> = Mutex::new(false);

/// Extract a boolean value from an optional JSON setting, returning `default_value` when the
/// setting is missing, null, or non-boolean.
///
/// This is a pure helper extracted so it can be unit-tested independently of the Tauri app
/// context.
fn parse_bool_setting(
    result: Result<Option<serde_json::Value>, soul_storage::StorageError>,
    default_value: bool,
) -> bool {
    result
        .ok()
        .flatten()
        .and_then(|v: serde_json::Value| v.as_bool())
        .unwrap_or(default_value)
}

/// Start the background update checker
///
/// Checks for updates immediately on startup, then every hour if auto-update is enabled
pub fn start_update_checker(app: AppHandle) {
    // Skip background update checking in E2E test environments or when explicitly disabled
    if std::env::var("PLAYWRIGHT_TEST_DIR").is_ok() || std::env::var("SKIP_UPDATE_CHECK").is_ok() {
        tracing::info!("[UPDATER] Skipping update checker (test/disabled mode)");
        return;
    }

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

    let auto_update_enabled = parse_bool_setting(auto_update, true);

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

                let silent_mode = parse_bool_setting(silent, false);

                if silent_mode {
                    tracing::info!("[UPDATER] Starting silent install");
                    match update
                        .download_and_install(
                            |_chunk_len, _content_len| {},
                            || {
                                tracing::info!("[UPDATER] Silent download finished");
                            },
                        )
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
    // In E2E test mode: use canned response if one has been set via set_test_update_response
    if std::env::var("PLAYWRIGHT_TEST_DIR").is_ok() {
        let canned = TEST_UPDATE_RESPONSE.lock().unwrap().clone();
        if let Some(response) = canned {
            return Ok(response);
        }
    }

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

/// Tauri command used by E2E tests to set the canned response for check_for_updates.
///
/// - `no_update = true` → simulate "no update available" (Ok(None))
/// - `response = Some({...})` → simulate update available with that info (Ok(Some(v)))
/// - both absent/null → clear canned response (fall through to real implementation)
///
/// Only available when PLAYWRIGHT_TEST_DIR env var is set.
#[tauri::command]
pub async fn set_test_update_response(
    response: Option<serde_json::Value>,
    no_update: Option<bool>,
) -> Result<(), String> {
    if std::env::var("PLAYWRIGHT_TEST_DIR").is_err() {
        return Err(
            "Only available in E2E test mode (PLAYWRIGHT_TEST_DIR must be set)".to_string(),
        );
    }
    let inner: Option<Option<serde_json::Value>> = if no_update == Some(true) {
        Some(None) // simulate "no update available"
    } else {
        response.map(Some) // None = clear; Some(v) = simulate update available
    };
    *TEST_UPDATE_RESPONSE.lock().unwrap() = inner;
    Ok(())
}

/// Tauri command used by E2E tests to control install_update behavior.
/// Pass `delay_ms = n` (n > 0) to make install_update sleep for n ms then return Ok.
/// Pass `delay_ms = 0` to clear (instant Ok in test mode).
/// Pass `should_fail = true` to make install_update return Err.
/// Only available when PLAYWRIGHT_TEST_DIR env var is set.
#[tauri::command]
pub async fn set_test_install_delay(
    delay_ms: u64,
    should_fail: Option<bool>,
) -> Result<(), String> {
    if std::env::var("PLAYWRIGHT_TEST_DIR").is_err() {
        return Err(
            "Only available in E2E test mode (PLAYWRIGHT_TEST_DIR must be set)".to_string(),
        );
    }
    *TEST_INSTALL_DELAY_MS.lock().unwrap() = delay_ms;
    *TEST_INSTALL_SHOULD_FAIL.lock().unwrap() = should_fail.unwrap_or(false);
    Ok(())
}

/// Tauri command used by E2E tests to fire a fake update-available event.
/// Only available when PLAYWRIGHT_TEST_DIR env var is set (test environment).
#[tauri::command]
pub async fn emit_test_update_available(app: AppHandle) -> Result<(), String> {
    if std::env::var("PLAYWRIGHT_TEST_DIR").is_err() {
        return Err(
            "Only available in E2E test mode (PLAYWRIGHT_TEST_DIR must be set)".to_string(),
        );
    }
    let update_info = serde_json::json!({
        "version": "99.9.9",
        "date": null,
        "body": "E2E test update – this is a fake update used by automated tests."
    });
    app.emit("update-available", &update_info)
        .map_err(|e| e.to_string())
}

/// Tauri command to install an available update
#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), String> {
    // In E2E test mode: simulate install with optional delay and failure
    if std::env::var("PLAYWRIGHT_TEST_DIR").is_ok() {
        let should_fail = *TEST_INSTALL_SHOULD_FAIL.lock().unwrap();
        let delay_ms = *TEST_INSTALL_DELAY_MS.lock().unwrap();

        // Emit progress events during delay to test progress UI
        if delay_ms > 0 {
            let steps = 5u64;
            let step_delay = delay_ms / steps;
            for i in 1..=steps {
                tokio::time::sleep(Duration::from_millis(step_delay)).await;
                let progress = ((i as f64 / steps as f64) * 100.0) as u8;
                let _ = app.emit("update-progress", progress);
            }
        }

        if should_fail {
            return Err("Simulated install failure for E2E test".to_string());
        }
        return Ok(());
    }

    let updater = app
        .updater()
        .map_err(|e: tauri_plugin_updater::Error| e.to_string())?;

    // Check for available update. The user already saw the dialog, but we need
    // the Update object to call download_and_install on it. This is a lightweight
    // HTTP request to the update endpoint.
    let update = updater
        .check()
        .await
        .map_err(|e| format!("Failed to check for update: {}", e))?
        .ok_or_else(|| "No update available. The update may have been withdrawn.".to_string())?;

    tracing::info!(
        version = %update.version,
        "[UPDATER] Starting download and install"
    );

    // Track cumulative downloaded bytes for accurate progress reporting.
    // The callback receives individual chunk sizes, NOT cumulative totals.
    let downloaded = AtomicUsize::new(0);
    let app_clone = app.clone();

    update
        .download_and_install(
            move |chunk_len, content_len| {
                let total_downloaded =
                    downloaded.fetch_add(chunk_len, Ordering::Relaxed) + chunk_len;
                let progress = if let Some(total) = content_len {
                    ((total_downloaded as f64 / total as f64) * 100.0).min(100.0) as u8
                } else {
                    0
                };
                if let Err(e) = app_clone.emit("update-progress", progress) {
                    tracing::warn!(error = %e, "[UPDATER] Failed to emit progress event");
                }
            },
            || {
                tracing::info!("[UPDATER] Download finished, starting installation");
            },
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "[UPDATER] Download/install failed");
            format!("Failed to install update: {}", e)
        })?;

    tracing::info!("[UPDATER] Install completed, restarting app");
    app.restart();

    // Unreachable — restart() exits the process
    #[allow(unreachable_code)]
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::parse_bool_setting;

    // Helper: simulate a successful DB result returning a JSON bool
    fn ok_bool(v: bool) -> Result<Option<serde_json::Value>, soul_storage::StorageError> {
        Ok(Some(serde_json::Value::Bool(v)))
    }

    // Helper: simulate a successful DB result returning null (setting exists but is null)
    fn ok_null() -> Result<Option<serde_json::Value>, soul_storage::StorageError> {
        Ok(Some(serde_json::Value::Null))
    }

    // Helper: simulate a successful DB result returning a JSON number (wrong type)
    fn ok_number(n: i64) -> Result<Option<serde_json::Value>, soul_storage::StorageError> {
        Ok(Some(serde_json::Value::Number(n.into())))
    }

    // Helper: simulate a successful DB result with no value (setting not found)
    fn ok_none() -> Result<Option<serde_json::Value>, soul_storage::StorageError> {
        Ok(None)
    }

    // Helper: simulate a DB error
    fn db_error() -> Result<Option<serde_json::Value>, soul_storage::StorageError> {
        Err(soul_storage::StorageError::not_found(
            "setting",
            "auto_update",
        ))
    }

    #[test]
    fn parse_bool_setting_returns_true_when_setting_is_true() {
        assert!(parse_bool_setting(ok_bool(true), false));
    }

    #[test]
    fn parse_bool_setting_returns_false_when_setting_is_false() {
        assert!(!parse_bool_setting(ok_bool(false), true));
    }

    #[test]
    fn parse_bool_setting_uses_default_when_setting_missing() {
        assert!(parse_bool_setting(ok_none(), true));
        assert!(!parse_bool_setting(ok_none(), false));
    }

    #[test]
    fn parse_bool_setting_uses_default_when_db_errors() {
        // auto_update_enabled defaults to true on DB error (safe default: keep checking)
        assert!(parse_bool_setting(db_error(), true));
        // silent_mode defaults to false on DB error (safe default: show prompt)
        assert!(!parse_bool_setting(db_error(), false));
    }

    #[test]
    fn parse_bool_setting_uses_default_when_value_is_null() {
        assert!(parse_bool_setting(ok_null(), true));
        assert!(!parse_bool_setting(ok_null(), false));
    }

    #[test]
    fn parse_bool_setting_uses_default_when_value_is_wrong_type() {
        // A number is not a bool — should fall back to default
        assert!(parse_bool_setting(ok_number(1), true));
        assert!(!parse_bool_setting(ok_number(0), false));
    }

    #[test]
    fn auto_update_enabled_default_is_true_safe_default() {
        // When the setting is absent (first run, DB error), auto-update should be ON by default.
        // This matches the intent: users get updates unless they explicitly opt out.
        let result = parse_bool_setting(ok_none(), true);
        assert!(result, "auto_update_enabled should default to true");
    }

    #[test]
    fn silent_mode_default_is_false_safe_default() {
        // When the silent_mode setting is absent, default to showing the update prompt.
        // This is the safer default: never silently restart without user consent.
        let result = parse_bool_setting(ok_none(), false);
        assert!(
            !result,
            "silent_mode should default to false (show update prompt)"
        );
    }
}
