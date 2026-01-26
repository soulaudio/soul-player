use crate::app_state::AppState;
use soul_storage::window_state::{self, WindowState};
use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, Position, Size};

/// Load and apply window state from database
pub async fn load_window_state(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let ws = window_state::get_window_state(&state.pool, &state.user_id)
        .await
        .map_err(|e| e.to_string())?;

    if let Some(window) = app.get_webview_window("main") {
        tracing::info!(
            "[window_state] Restoring window state: {}x{} at ({:?}, {:?}), maximized={}",
            ws.width,
            ws.height,
            ws.x,
            ws.y,
            ws.maximized
        );

        // On macOS, there's a known Tauri v2 bug where set_size() doesn't work reliably
        // on hidden windows (GitHub issue #12168). Note: macOS now uses native decorations,
        // but this workaround is still needed during window initialization.
        // The window will be shown first in main.rs, then this function will be called again
        // to apply the size. For now, we only set position on hidden windows.
        #[cfg(target_os = "macos")]
        {
            if !window.is_visible().unwrap_or(false) {
                tracing::debug!(
                    "[window_state] macOS: Window is hidden, deferring size until after show()"
                );
                // Only set position while hidden, size will be set after show()
                if let (Some(x), Some(y)) = (ws.x, ws.y) {
                    if let Err(e) =
                        window.set_position(Position::Physical(PhysicalPosition { x, y }))
                    {
                        tracing::warn!("[window_state] Failed to set window position: {}", e);
                    } else {
                        tracing::debug!("[window_state] Window position set to ({}, {})", x, y);
                    }
                }
                return Ok(());
            }

            // Window is visible, now set the size with reduced retries
            // First attempt is immediate (no sleep), retries only if needed
            tracing::debug!(
                "[window_state] macOS: Window is visible, applying size {}x{} with retries",
                ws.width,
                ws.height
            );

            let mut size_applied = false;
            const MAX_ATTEMPTS: u32 = 2; // Reduced from 3 to 2 (5-10ms faster)
            const RETRY_DELAY_MS: u64 = 16; // Single frame at 60fps

            for attempt in 1..=MAX_ATTEMPTS {
                match window.set_size(Size::Physical(PhysicalSize {
                    width: ws.width as u32,
                    height: ws.height as u32,
                })) {
                    Ok(_) => {
                        tracing::debug!(
                            "[window_state] macOS: set_size succeeded on attempt {}",
                            attempt
                        );
                        size_applied = true;
                        break;
                    }
                    Err(e) => {
                        tracing::debug!(
                            "[window_state] macOS: set_size failed on attempt {}: {}",
                            attempt,
                            e
                        );
                        // Only sleep AFTER failure, not before first attempt
                        if attempt < MAX_ATTEMPTS {
                            tokio::time::sleep(std::time::Duration::from_millis(RETRY_DELAY_MS))
                                .await;
                        }
                    }
                }
            }

            if !size_applied {
                tracing::warn!(
                    "[window_state] macOS: Failed to apply size after {} attempts, using default size",
                    MAX_ATTEMPTS
                );
            }
        }

        // On other platforms, set size normally
        #[cfg(not(target_os = "macos"))]
        {
            if let Err(e) = window.set_size(Size::Physical(PhysicalSize {
                width: ws.width as u32,
                height: ws.height as u32,
            })) {
                tracing::warn!("[window_state] Failed to set window size: {}", e);
            }
        }

        // Set position if available (on non-macOS or visible macOS windows)
        #[cfg(not(target_os = "macos"))]
        if let (Some(x), Some(y)) = (ws.x, ws.y) {
            if let Err(e) = window.set_position(Position::Physical(PhysicalPosition { x, y })) {
                tracing::warn!("[window_state] Failed to set window position: {}", e);
            } else {
                tracing::debug!("[window_state] Window position set to ({}, {})", x, y);
            }
        }

        // Set maximized
        if ws.maximized {
            if let Err(e) = window.maximize() {
                tracing::warn!("[window_state] Failed to maximize window: {}", e);
            } else {
                tracing::debug!("[window_state] Window maximized");
            }
        }

        // Verify the size was actually applied (log for debugging)
        if let Ok(size) = window.outer_size() {
            tracing::info!(
                "[window_state] Final window size: {}x{} (requested: {}x{})",
                size.width,
                size.height,
                ws.width,
                ws.height
            );
        }
    }

    Ok(())
}

/// Save current window state to database
pub async fn save_window_state(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let window = app
        .get_webview_window("main")
        .ok_or("Main window not found")?;

    let position = window.outer_position().ok();
    let size = window.outer_size().ok();
    let maximized = window.is_maximized().unwrap_or(false);

    let ws = WindowState {
        x: position.map(|p| p.x),
        y: position.map(|p| p.y),
        width: size.map(|s| s.width as i32).unwrap_or(1200),
        height: size.map(|s| s.height as i32).unwrap_or(800),
        maximized,
        last_route: None, // Will be set from frontend if needed
    };

    tracing::info!(
        "[window_state] Saving window state: {}x{} at ({:?}, {:?}), maximized={}",
        ws.width,
        ws.height,
        ws.x,
        ws.y,
        ws.maximized
    );

    window_state::save_window_state(&state.pool, &state.user_id, &ws)
        .await
        .map_err(|e| e.to_string())
}

/// Tauri command to save window state
#[tauri::command]
pub async fn save_window_state_cmd(app: AppHandle) -> Result<(), String> {
    save_window_state(&app).await
}

/// Tauri command to save window state with route
#[tauri::command]
pub async fn save_window_state_with_route(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    route: String,
) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or("Main window not found")?;

    let position = window.outer_position().ok();
    let size = window.outer_size().ok();
    let maximized = window.is_maximized().unwrap_or(false);

    let ws = WindowState {
        x: position.map(|p| p.x),
        y: position.map(|p| p.y),
        width: size.map(|s| s.width as i32).unwrap_or(1200),
        height: size.map(|s| s.height as i32).unwrap_or(800),
        maximized,
        last_route: Some(route.clone()),
    };

    tracing::info!(
        "[window_state] Saving window state with route '{}': {}x{} at ({:?}, {:?}), maximized={}",
        route,
        ws.width,
        ws.height,
        ws.x,
        ws.y,
        ws.maximized
    );

    window_state::save_window_state(&state.pool, &state.user_id, &ws)
        .await
        .map_err(|e| e.to_string())
}
