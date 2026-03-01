use std::path::PathBuf;
use tauri::{AppHandle, Manager};

fn get_themes_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    let themes_dir = app_data.join("themes");
    std::fs::create_dir_all(&themes_dir)
        .map_err(|e| format!("Failed to create themes directory: {}", e))?;
    Ok(themes_dir)
}

/// List all custom themes stored as JSON files in the app data themes directory.
/// Returns a list of raw JSON strings, one per theme file.
#[tauri::command]
pub async fn theme_list_custom(app: AppHandle) -> Result<Vec<String>, String> {
    let themes_dir = match get_themes_dir(&app) {
        Ok(dir) => dir,
        Err(e) => {
            tracing::warn!("[THEME] Could not get themes dir: {}", e);
            return Ok(Vec::new());
        }
    };

    let mut themes = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&themes_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                match std::fs::read_to_string(&path) {
                    Ok(content) => themes.push(content),
                    Err(e) => {
                        tracing::warn!("[THEME] Failed to read theme file {:?}: {}", path, e)
                    }
                }
            }
        }
    }
    // If read_dir fails, directory doesn't exist yet — not an error

    tracing::debug!("[THEME] Loaded {} custom theme(s) from disk", themes.len());
    Ok(themes)
}

/// Save a custom theme as a JSON file in the app data themes directory.
/// The file is named `{theme_id}.json`.
#[tauri::command]
pub async fn theme_save(
    app: AppHandle,
    theme_id: String,
    theme_json: String,
) -> Result<(), String> {
    let themes_dir = get_themes_dir(&app)?;
    let filename = format!("{}.json", theme_id);
    let path = themes_dir.join(&filename);

    std::fs::write(&path, &theme_json)
        .map_err(|e| format!("Failed to write theme file '{}': {}", filename, e))?;

    tracing::info!("[THEME] Saved custom theme: {}", theme_id);
    Ok(())
}

/// Delete a custom theme file from the app data themes directory.
/// No-op if the file does not exist.
#[tauri::command]
pub async fn theme_delete(app: AppHandle, theme_id: String) -> Result<(), String> {
    let themes_dir = get_themes_dir(&app)?;
    let filename = format!("{}.json", theme_id);
    let path = themes_dir.join(&filename);

    if path.exists() {
        std::fs::remove_file(&path)
            .map_err(|e| format!("Failed to delete theme file '{}': {}", filename, e))?;
        tracing::info!("[THEME] Deleted custom theme: {}", theme_id);
    }

    Ok(())
}
