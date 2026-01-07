use crate::app_state::AppState;
use soul_storage::shortcuts::{self, ShortcutAction};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

/// Register all enabled global shortcuts for the current user
pub async fn register_shortcuts(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let shortcuts_list = shortcuts::get_shortcuts(&state.pool, &state.user_id)
        .await
        .map_err(|e| e.to_string())?;

    for sc in shortcuts_list.iter().filter(|s| s.enabled) {
        let shortcut = sc
            .accelerator
            .parse::<Shortcut>()
            .map_err(|e| format!("Invalid shortcut '{}': {}", sc.accelerator, e))?;

        let app_clone = app.clone();
        let action = sc.action.clone();

        app.global_shortcut()
            .on_shortcut(shortcut, move |_app, _shortcut, _event| {
                handle_shortcut_action(&app_clone, &action);
            })
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// Handle a shortcut action by emitting events to the frontend
fn handle_shortcut_action(app: &AppHandle, action: &ShortcutAction) {
    let event_name = match action {
        ShortcutAction::PlayPause => "shortcut-play-pause",
        ShortcutAction::Next => "shortcut-next",
        ShortcutAction::Previous => "shortcut-previous",
        ShortcutAction::VolumeUp => "shortcut-volume-up",
        ShortcutAction::VolumeDown => "shortcut-volume-down",
        ShortcutAction::Mute => "shortcut-mute",
        ShortcutAction::ToggleShuffle => "shortcut-toggle-shuffle",
        ShortcutAction::ToggleRepeat => "shortcut-toggle-repeat",
    };

    let _ = app.emit(event_name, ());
}

/// Tauri command to get all global shortcuts
#[tauri::command]
pub async fn get_global_shortcuts(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<soul_storage::shortcuts::GlobalShortcut>, String> {
    shortcuts::get_shortcuts(&state.pool, &state.user_id)
        .await
        .map_err(|e| e.to_string())
}

/// Tauri command to set a global shortcut
#[tauri::command]
pub async fn set_global_shortcut(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    action: String,
    accelerator: String,
) -> Result<(), String> {
    let action_enum = ShortcutAction::from_str(&action).ok_or("Invalid action")?;

    // Validate accelerator
    accelerator
        .parse::<Shortcut>()
        .map_err(|e| format!("Invalid shortcut: {}", e))?;

    shortcuts::set_shortcut(&state.pool, &state.user_id, action_enum, accelerator)
        .await
        .map_err(|e| e.to_string())?;

    // Re-register all shortcuts
    app.global_shortcut()
        .unregister_all()
        .map_err(|e| e.to_string())?;
    register_shortcuts(&app).await?;

    Ok(())
}

/// Tauri command to reset shortcuts to defaults
#[tauri::command]
pub async fn reset_global_shortcuts(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    shortcuts::reset_shortcuts_to_defaults(&state.pool, &state.user_id)
        .await
        .map_err(|e| e.to_string())?;

    app.global_shortcut()
        .unregister_all()
        .map_err(|e| e.to_string())?;
    register_shortcuts(&app).await?;

    Ok(())
}
