use tauri::{
    AppHandle, Emitter, Manager,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};

/// Create and configure the system tray icon with menu
pub fn create_tray(app: &AppHandle) -> tauri::Result<()> {
    let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let show_i = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let play_pause_i = MenuItem::with_id(app, "play_pause", "Play/Pause", true, None::<&str>)?;
    let next_i = MenuItem::with_id(app, "next", "Next", true, None::<&str>)?;
    let prev_i = MenuItem::with_id(app, "previous", "Previous", true, None::<&str>)?;

    let menu = Menu::with_items(
        app,
        &[&play_pause_i, &next_i, &prev_i, &separator, &show_i, &quit_i],
    )?;

    let _tray = TrayIconBuilder::with_id("main")
        .menu(&menu)
        .on_menu_event(move |app, event| {
            match event.id.as_ref() {
                "quit" => app.exit(0),
                "show" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                "play_pause" => {
                    // Emit event to frontend to toggle playback
                    if let Err(e) = app.emit("tray-play-pause", ()) {
                        tracing::warn!(error = %e, event = "tray-play-pause", "Failed to emit event to frontend");
                    }
                }
                "next" => {
                    if let Err(e) = app.emit("tray-next", ()) {
                        tracing::warn!(error = %e, event = "tray-next", "Failed to emit event to frontend");
                    }
                }
                "previous" => {
                    if let Err(e) = app.emit("tray-previous", ()) {
                        tracing::warn!(error = %e, event = "tray-previous", "Failed to emit event to frontend");
                    }
                }
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                tracing::debug!("[Tray] Left click detected, toggling window visibility");
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    match window.is_visible() {
                        Ok(is_visible) => {
                            if is_visible {
                                tracing::debug!("[Tray] Hiding window");
                                if let Err(e) = window.hide() {
                                    tracing::error!("[Tray] Failed to hide window: {}", e);
                                }
                            } else {
                                tracing::debug!("[Tray] Showing and focusing window");
                                if let Err(e) = window.show() {
                                    tracing::error!("[Tray] Failed to show window: {}", e);
                                }
                                if let Err(e) = window.set_focus() {
                                    tracing::error!("[Tray] Failed to focus window: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!("[Tray] Failed to check window visibility: {}", e);
                        }
                    }
                } else {
                    tracing::warn!("[Tray] Main window not found");
                }
            }
        })
        .build(app)?;

    Ok(())
}
