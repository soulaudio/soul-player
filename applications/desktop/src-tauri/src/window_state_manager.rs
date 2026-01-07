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
        // Set position if available
        if let (Some(x), Some(y)) = (ws.x, ws.y) {
            let _ = window.set_position(Position::Physical(PhysicalPosition { x, y }));
        }

        // Set size
        let _ = window.set_size(Size::Physical(PhysicalSize {
            width: ws.width as u32,
            height: ws.height as u32,
        }));

        // Set maximized
        if ws.maximized {
            let _ = window.maximize();
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
        last_route: Some(route),
    };

    window_state::save_window_state(&state.pool, &state.user_id, &ws)
        .await
        .map_err(|e| e.to_string())
}
