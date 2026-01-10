use crate::app_state::AppState;
use soul_storage::shortcuts::{self, ShortcutAction};
use tauri::AppHandle;

// NOTE: We use app-level shortcuts handled in React, not OS-level global shortcuts.
// This allows shortcuts to:
// 1. Only work when the app window is focused
// 2. Respect input fields (don't fire when typing in textarea/input)
// 3. Be more predictable and not conflict with other applications
//
// The shortcuts are stored in the database and loaded by the frontend.
// See: applications/desktop/src/hooks/useKeyboardShortcuts.ts

/// Register shortcuts - now a no-op since we use app-level shortcuts in React
pub async fn register_shortcuts(_app: &AppHandle) -> Result<(), String> {
    // App-level shortcuts are handled in React via useKeyboardShortcuts hook
    // This function is kept for compatibility but does nothing
    Ok(())
}

/// Tauri command to get all shortcuts (renamed from global for clarity)
#[tauri::command]
pub async fn get_global_shortcuts(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<soul_storage::shortcuts::GlobalShortcut>, String> {
    shortcuts::get_shortcuts(&state.pool, &state.user_id)
        .await
        .map_err(|e| e.to_string())
}

/// Tauri command to set a shortcut
#[tauri::command]
pub async fn set_global_shortcut(
    state: tauri::State<'_, AppState>,
    action: String,
    accelerator: String,
) -> Result<(), String> {
    let action_enum = ShortcutAction::from_str(&action).ok_or("Invalid action")?;

    // Basic validation - accelerator must not be empty
    if accelerator.trim().is_empty() {
        return Err("Shortcut cannot be empty".to_string());
    }

    shortcuts::set_shortcut(&state.pool, &state.user_id, action_enum, accelerator)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Tauri command to reset shortcuts to defaults
#[tauri::command]
pub async fn reset_global_shortcuts(
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    shortcuts::reset_shortcuts_to_defaults(&state.pool, &state.user_id)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}
