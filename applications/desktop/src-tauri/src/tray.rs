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
                    let _ = app.emit("tray-play-pause", ());
                }
                "next" => {
                    let _ = app.emit("tray-next", ());
                }
                "previous" => {
                    let _ = app.emit("tray-previous", ());
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
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    if window.is_visible().unwrap_or(false) {
                        let _ = window.hide();
                    } else {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
            }
        })
        .build(app)?;

    Ok(())
}
