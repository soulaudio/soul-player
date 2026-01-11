//! Library settings Tauri commands
//!
//! Handles library sources (watched folders), managed library settings,
//! and external file handling preferences.

use crate::app_state::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

/// Frontend representation of a library source
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontendLibrarySource {
    pub id: i64,
    pub name: String,
    pub path: String,
    pub enabled: bool,
    pub sync_deletes: bool,
    pub last_scan_at: Option<i64>,
    pub scan_status: Option<String>,
    pub error_message: Option<String>,
}

impl From<soul_core::types::LibrarySource> for FrontendLibrarySource {
    fn from(source: soul_core::types::LibrarySource) -> Self {
        Self {
            id: source.id,
            name: source.name,
            path: source.path,
            enabled: source.enabled,
            sync_deletes: source.sync_deletes,
            last_scan_at: source.last_scan_at,
            scan_status: Some(source.scan_status.as_str().to_string()),
            error_message: source.error_message,
        }
    }
}

/// Frontend representation of managed library settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontendManagedLibrarySettings {
    pub library_path: String,
    pub path_template: String,
    pub import_action: String, // "copy" or "move"
}

/// Frontend representation of external file settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontendExternalFileSettings {
    pub default_action: String,     // "ask", "play", or "import"
    pub import_destination: String, // "managed" or "watched"
    pub import_to_source_id: Option<i64>,
    pub show_import_notification: bool,
}

/// Get the device ID for this desktop instance
fn get_device_id() -> String {
    // For desktop, we use a fixed device ID
    // In a real implementation, this would be generated once and stored
    "desktop-local".to_string()
}

// ============================================================================
// Library Sources (Watched Folders)
// ============================================================================

/// Get all library sources for the current user/device
#[tauri::command]
pub async fn get_library_sources(
    state: State<'_, AppState>,
) -> Result<Vec<FrontendLibrarySource>, String> {
    let device_id = get_device_id();

    let sources =
        soul_storage::library_sources::get_by_user_device(&state.pool, &state.user_id, &device_id)
            .await
            .map_err(|e| format!("Failed to get library sources: {}", e))?;

    Ok(sources
        .into_iter()
        .map(FrontendLibrarySource::from)
        .collect())
}

/// Add a new library source (watched folder)
#[tauri::command]
pub async fn add_library_source(
    name: String,
    path: String,
    sync_deletes: bool,
    state: State<'_, AppState>,
) -> Result<FrontendLibrarySource, String> {
    let device_id = get_device_id();

    // Verify path exists
    let path_buf = std::path::Path::new(&path);
    if !path_buf.exists() {
        return Err(format!("Path does not exist: {}", path));
    }
    if !path_buf.is_dir() {
        return Err(format!("Path is not a directory: {}", path));
    }

    let create_source = soul_core::types::CreateLibrarySource {
        name,
        path,
        sync_deletes,
    };

    let source = soul_storage::library_sources::create(
        &state.pool,
        &state.user_id,
        &device_id,
        &create_source,
    )
    .await
    .map_err(|e| {
        let err_str = e.to_string();
        // Check for UNIQUE constraint violation on path
        if err_str.contains("UNIQUE constraint failed") && err_str.contains("path") {
            "DUPLICATE_PATH".to_string()
        } else {
            format!("Failed to create library source: {}", e)
        }
    })?;

    Ok(FrontendLibrarySource::from(source))
}

/// Remove a library source
#[tauri::command]
pub async fn remove_library_source(
    source_id: i64,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    soul_storage::library_sources::delete(&state.pool, source_id)
        .await
        .map_err(|e| format!("Failed to delete library source: {}", e))
}

/// Toggle a library source enabled/disabled
#[tauri::command]
pub async fn toggle_library_source(
    source_id: i64,
    enabled: bool,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    soul_storage::library_sources::set_enabled(&state.pool, source_id, enabled)
        .await
        .map_err(|e| format!("Failed to toggle library source: {}", e))
}

/// Trigger a rescan of a specific library source
/// If force_refresh is true, re-extracts metadata even for unchanged files
#[tauri::command]
pub async fn rescan_library_source(
    source_id: i64,
    force_refresh: Option<bool>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let device_id = get_device_id();

    // Get the source
    let source = soul_storage::library_sources::get_by_id(&state.pool, source_id)
        .await
        .map_err(|e| format!("Failed to get library source: {}", e))?
        .ok_or_else(|| "Library source not found".to_string())?;

    // Create scanner and scan
    let scanner = soul_importer::library_scanner::LibraryScanner::new(
        (*state.pool).clone(),
        state.user_id.clone(),
        device_id,
    )
    .force_metadata_refresh(force_refresh.unwrap_or(false));

    scanner
        .scan_source(&source)
        .await
        .map_err(|e| format!("Failed to scan source: {}", e))?;

    Ok(())
}

/// Trigger a rescan of all enabled library sources
/// If force_refresh is true, re-extracts metadata even for unchanged files
#[tauri::command]
pub async fn rescan_all_sources(
    force_refresh: Option<bool>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let device_id = get_device_id();

    let scanner = soul_importer::library_scanner::LibraryScanner::new(
        (*state.pool).clone(),
        state.user_id.clone(),
        device_id,
    )
    .force_metadata_refresh(force_refresh.unwrap_or(false));

    scanner
        .scan_all()
        .await
        .map_err(|e| format!("Failed to scan sources: {}", e))?;

    Ok(())
}

// ============================================================================
// Managed Library Settings
// ============================================================================

/// Get managed library settings
#[tauri::command]
pub async fn get_managed_library_settings(
    state: State<'_, AppState>,
) -> Result<Option<FrontendManagedLibrarySettings>, String> {
    let device_id = get_device_id();

    let settings =
        soul_storage::managed_library_settings::get(&state.pool, &state.user_id, &device_id)
            .await
            .map_err(|e| format!("Failed to get managed library settings: {}", e))?;

    Ok(settings.map(|s| FrontendManagedLibrarySettings {
        library_path: s.library_path,
        path_template: s.path_template,
        import_action: format!("{:?}", s.import_action).to_lowercase(),
    }))
}

/// Set managed library settings
#[tauri::command]
pub async fn set_managed_library_settings(
    library_path: String,
    path_template: String,
    import_action: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    use soul_core::types::{ImportAction, UpdateManagedLibrarySettings};

    let device_id = get_device_id();

    // Verify path exists or create it
    let path_buf = std::path::Path::new(&library_path);
    if !path_buf.exists() {
        std::fs::create_dir_all(path_buf)
            .map_err(|e| format!("Failed to create library directory: {}", e))?;
    }

    let action = match import_action.as_str() {
        "copy" => ImportAction::Copy,
        "move" => ImportAction::Move,
        _ => return Err(format!("Invalid import action: {}", import_action)),
    };

    let update = UpdateManagedLibrarySettings {
        library_path,
        path_template,
        import_action: action,
    };

    soul_storage::managed_library_settings::upsert(
        &state.pool,
        &state.user_id,
        &device_id,
        &update,
    )
    .await
    .map_err(|e| format!("Failed to save managed library settings: {}", e))?;

    Ok(())
}

// ============================================================================
// External File Settings
// ============================================================================

/// Get external file handling settings
#[tauri::command]
pub async fn get_external_file_settings(
    state: State<'_, AppState>,
) -> Result<FrontendExternalFileSettings, String> {
    let device_id = get_device_id();

    let settings =
        soul_storage::external_file_settings::get(&state.pool, &state.user_id, &device_id)
            .await
            .map_err(|e| format!("Failed to get external file settings: {}", e))?;

    match settings {
        Some(s) => Ok(FrontendExternalFileSettings {
            default_action: s.default_action.as_str().to_string(),
            import_destination: s.import_destination.as_str().to_string(),
            import_to_source_id: s.import_to_source_id,
            show_import_notification: s.show_import_notification,
        }),
        None => Ok(FrontendExternalFileSettings {
            default_action: "ask".to_string(),
            import_destination: "managed".to_string(),
            import_to_source_id: None,
            show_import_notification: true,
        }),
    }
}

/// Set external file handling settings
#[tauri::command]
pub async fn set_external_file_settings(
    default_action: String,
    import_destination: String,
    import_to_source_id: Option<i64>,
    show_import_notification: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    use soul_core::types::{ExternalFileAction, ImportDestination, UpdateExternalFileSettings};

    let device_id = get_device_id();

    let action = ExternalFileAction::from_str(&default_action)
        .ok_or_else(|| format!("Invalid default action: {}", default_action))?;

    let destination = ImportDestination::from_str(&import_destination)
        .ok_or_else(|| format!("Invalid import destination: {}", import_destination))?;

    let update = UpdateExternalFileSettings {
        default_action: action,
        import_destination: destination,
        import_to_source_id,
        show_import_notification,
    };

    soul_storage::external_file_settings::upsert(&state.pool, &state.user_id, &device_id, &update)
        .await
        .map_err(|e| format!("Failed to save external file settings: {}", e))?;

    Ok(())
}

// ============================================================================
// Path Template Presets
// ============================================================================

/// Get available path template presets
#[tauri::command]
pub fn get_path_template_presets() -> Vec<(String, String, String)> {
    vec![
        (
            "audiophile".to_string(),
            "{AlbumArtist}/{Year} - {Album}/{TrackNo} - {Title}".to_string(),
            "Pink Floyd/1977 - Animals/01 - Pigs on the Wing.flac".to_string(),
        ),
        (
            "simple".to_string(),
            "{AlbumArtist}/{Album}/{TrackNo} - {Title}".to_string(),
            "Pink Floyd/Animals/01 - Pigs on the Wing.flac".to_string(),
        ),
        (
            "genre".to_string(),
            "{Genre}/{AlbumArtist}/{Album}/{TrackNo} - {Title}".to_string(),
            "Rock/Pink Floyd/Animals/01 - Pigs on the Wing.flac".to_string(),
        ),
    ]
}

/// Preview a path template with sample metadata
#[tauri::command]
pub fn preview_path_template(template: String) -> String {
    use soul_importer::metadata::ExtractedMetadata;
    use soul_importer::path_template::PathTemplate;

    let path_template = PathTemplate::new(&template);

    // Create sample metadata for preview
    let metadata = ExtractedMetadata {
        title: Some("Pigs on the Wing (Part 1)".to_string()),
        artist: Some("Pink Floyd".to_string()),
        album: Some("Animals".to_string()),
        album_artist: Some("Pink Floyd".to_string()),
        track_number: Some(1),
        disc_number: Some(1),
        year: Some(1977),
        genres: vec!["Progressive Rock".to_string()],
        duration_seconds: Some(85.0),
        bitrate: Some(1411),
        sample_rate: Some(44100),
        channels: Some(2),
        file_format: "flac".to_string(),
        musicbrainz_recording_id: None,
        composer: Some("Roger Waters".to_string()),
        album_art: None,
    };

    let source_path = std::path::Path::new("/example/source.flac");
    let result = path_template.resolve(&metadata, source_path);

    result.to_string_lossy().to_string()
}

// ============================================================================
// Folder Picker
// ============================================================================

/// Open a folder picker dialog
/// Note: Currently returns an error, as folder picker requires additional setup
#[tauri::command]
pub async fn pick_folder() -> Result<Option<String>, String> {
    // For now, return an error indicating manual input is needed
    // In the future, we can use tauri-plugin-dialog or native-dialog
    Err("Folder picker not yet implemented - please enter the path manually".to_string())
}

// ============================================================================
// Onboarding
// ============================================================================

/// Check if onboarding is needed (first run or empty library)
#[tauri::command]
pub async fn check_onboarding_needed(state: State<'_, AppState>) -> Result<bool, String> {
    let device_id = get_device_id();

    // Check if user has any library sources configured
    let sources =
        soul_storage::library_sources::get_by_user_device(&state.pool, &state.user_id, &device_id)
            .await
            .map_err(|e| format!("Failed to check library sources: {}", e))?;

    // Check if managed library is configured
    let managed =
        soul_storage::managed_library_settings::get(&state.pool, &state.user_id, &device_id)
            .await
            .map_err(|e| format!("Failed to check managed settings: {}", e))?;

    // Onboarding needed if no sources AND no managed library configured
    Ok(sources.is_empty() && managed.is_none())
}

/// Mark onboarding as complete (by setting up minimal config)
#[tauri::command]
pub async fn complete_onboarding(
    setup_type: String, // "watched", "managed", or "both"
    state: State<'_, AppState>,
) -> Result<(), String> {
    use soul_core::types::UpdateManagedLibrarySettings;

    let device_id = get_device_id();

    // If managed library was selected, set up default managed library settings
    if setup_type == "managed" || setup_type == "both" {
        let default_path = state.library_path.to_string_lossy().to_string();
        let update = UpdateManagedLibrarySettings::default();

        soul_storage::managed_library_settings::upsert(
            &state.pool,
            &state.user_id,
            &device_id,
            &UpdateManagedLibrarySettings {
                library_path: default_path,
                ..update
            },
        )
        .await
        .map_err(|e| format!("Failed to set up managed library: {}", e))?;
    }

    Ok(())
}

/// Get default library path suggestion
#[tauri::command]
pub fn get_default_library_path(state: State<'_, AppState>) -> String {
    state.library_path.to_string_lossy().to_string()
}

// ============================================================================
// Scan Progress
// ============================================================================

/// Scan progress for frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontendScanProgress {
    pub id: i64,
    pub library_source_id: i64,
    pub library_source_name: Option<String>,
    pub started_at: i64,
    pub completed_at: Option<i64>,
    pub total_files: Option<i64>,
    pub processed_files: i64,
    pub new_files: i64,
    pub updated_files: i64,
    pub removed_files: i64,
    pub errors: i64,
    pub status: String,
    pub error_message: Option<String>,
    /// Percentage (0-100)
    pub percentage: f32,
}

/// Get all currently running scans
#[tauri::command]
pub async fn get_running_scans(
    state: State<'_, AppState>,
) -> Result<Vec<FrontendScanProgress>, String> {
    let device_id = get_device_id();

    // Get all sources for this user/device
    let sources =
        soul_storage::library_sources::get_by_user_device(&state.pool, &state.user_id, &device_id)
            .await
            .map_err(|e| format!("Failed to get library sources: {}", e))?;

    let mut running_scans = Vec::new();

    for source in sources {
        if let Some(progress) = soul_storage::scan_progress::get_running(&state.pool, source.id)
            .await
            .map_err(|e| format!("Failed to get scan progress: {}", e))?
        {
            let percentage = if let Some(total) = progress.total_files {
                if total > 0 {
                    (progress.processed_files as f32 / total as f32) * 100.0
                } else {
                    0.0
                }
            } else {
                0.0
            };

            running_scans.push(FrontendScanProgress {
                id: progress.id,
                library_source_id: progress.library_source_id,
                library_source_name: Some(source.name.clone()),
                started_at: progress.started_at,
                completed_at: progress.completed_at,
                total_files: progress.total_files,
                processed_files: progress.processed_files,
                new_files: progress.new_files,
                updated_files: progress.updated_files,
                removed_files: progress.removed_files,
                errors: progress.errors,
                status: progress.status.as_str().to_string(),
                error_message: progress.error_message,
                percentage,
            });
        }
    }

    Ok(running_scans)
}

/// Get the latest scan for a specific source
#[tauri::command]
pub async fn get_latest_scan(
    source_id: i64,
    state: State<'_, AppState>,
) -> Result<Option<FrontendScanProgress>, String> {
    let progress = soul_storage::scan_progress::get_latest(&state.pool, source_id)
        .await
        .map_err(|e| format!("Failed to get scan progress: {}", e))?;

    Ok(progress.map(|p| {
        let percentage = if let Some(total) = p.total_files {
            if total > 0 {
                (p.processed_files as f32 / total as f32) * 100.0
            } else {
                0.0
            }
        } else {
            0.0
        };

        FrontendScanProgress {
            id: p.id,
            library_source_id: p.library_source_id,
            library_source_name: None,
            started_at: p.started_at,
            completed_at: p.completed_at,
            total_files: p.total_files,
            processed_files: p.processed_files,
            new_files: p.new_files,
            updated_files: p.updated_files,
            removed_files: p.removed_files,
            errors: p.errors,
            status: p.status.as_str().to_string(),
            error_message: p.error_message,
            percentage,
        }
    }))
}
