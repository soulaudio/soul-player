//! Import management for Soul Player desktop app
//!
//! This module handles music file importing with progress updates sent to the frontend

use serde::{Deserialize, Serialize};
use soul_importer::{ImportConfig, ImportProgress, ImportSummary, MusicImporter};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::{Mutex, RwLock};

// Re-export SqlitePool from soul_storage
type SqlitePool = sqlx::SqlitePool;

/// Import manager state
pub struct ImportManager {
    pool: SqlitePool,
    user_id: String,
    config: RwLock<ImportConfig>,
    current_import: Arc<Mutex<Option<ImportState>>>,
}

/// State of an ongoing import operation
struct ImportState {
    total_files: usize,
    processed: usize,
    successful: usize,
    failed: usize,
    duplicates: usize,
    is_running: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportProgressUpdate {
    pub total_files: usize,
    pub processed_files: usize,
    pub successful_imports: usize,
    pub skipped_duplicates: usize,
    pub failed_imports: usize,
    pub current_file: Option<String>,
    pub estimated_seconds_remaining: Option<u64>,
    pub percentage: f32,
}

impl From<ImportProgress> for ImportProgressUpdate {
    fn from(progress: ImportProgress) -> Self {
        let current_file = progress
            .current_file
            .as_ref()
            .map(|p| p.display().to_string());
        let percentage = progress.percentage();

        Self {
            total_files: progress.total_files,
            processed_files: progress.processed_files,
            successful_imports: progress.successful_imports,
            skipped_duplicates: progress.skipped_duplicates,
            failed_imports: progress.failed_imports,
            current_file,
            estimated_seconds_remaining: progress.estimated_seconds_remaining,
            percentage,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSummaryResponse {
    pub total_processed: usize,
    pub successful: usize,
    pub duplicates_skipped: usize,
    pub failed: usize,
    pub require_review_count: usize,
    pub errors: Vec<(String, String)>,
    pub duration_seconds: u64,
}

impl From<ImportSummary> for ImportSummaryResponse {
    fn from(summary: ImportSummary) -> Self {
        Self {
            total_processed: summary.total_processed,
            successful: summary.successful,
            duplicates_skipped: summary.duplicates_skipped,
            failed: summary.failed,
            require_review_count: summary.require_review.len(),
            errors: summary
                .errors
                .into_iter()
                .map(|(path, err)| (path.display().to_string(), err))
                .collect(),
            duration_seconds: summary.duration_seconds,
        }
    }
}

impl ImportManager {
    pub async fn new(
        pool: SqlitePool,
        user_id: String,
        default_library_path: PathBuf,
    ) -> Result<Self, String> {
        tracing::debug!("Initializing for user_id: {}", user_id);

        // Load config from database (or defaults for new users)
        use soul_importer::FileManagementStrategy;

        let strategy_str: String = soul_storage::settings::get_import_strategy(&pool, &user_id)
            .await
            .map_err(|e| format!("Failed to load import strategy: {}", e))?;

        let file_strategy = match strategy_str.as_str() {
            "move" => FileManagementStrategy::Move,
            "reference" => FileManagementStrategy::Reference,
            _ => FileManagementStrategy::Copy,
        };

        let library_path: PathBuf =
            soul_storage::settings::get_import_library_path(&pool, &user_id)
                .await
                .map_err(|e| format!("Failed to load library path: {}", e))?
                .map(PathBuf::from)
                .unwrap_or(default_library_path);

        let confidence_threshold: u8 =
            soul_storage::settings::get_import_confidence_threshold(&pool, &user_id)
                .await
                .map_err(|e| format!("Failed to load confidence threshold: {}", e))?;

        let file_naming_pattern: String =
            soul_storage::settings::get_import_file_naming_pattern(&pool, &user_id)
                .await
                .map_err(|e| format!("Failed to load file naming pattern: {}", e))?;

        let skip_duplicates: bool =
            soul_storage::settings::get_import_skip_duplicates(&pool, &user_id)
                .await
                .map_err(|e| format!("Failed to load skip duplicates: {}", e))?;

        let config = ImportConfig {
            library_path: library_path.clone(),
            file_strategy,
            confidence_threshold,
            file_naming_pattern,
            skip_duplicates,
        };

        tracing::debug!(
            "Loaded config: strategy={:?}, library_path={:?}",
            config.file_strategy,
            config.library_path
        );

        Ok(Self {
            pool,
            user_id,
            config: RwLock::new(config),
            current_import: Arc::new(Mutex::new(None)),
        })
    }

    pub async fn is_importing(&self) -> bool {
        self.current_import
            .lock()
            .await
            .as_ref()
            .map(|s| s.is_running)
            .unwrap_or(false)
    }

    pub async fn get_config(&self) -> ImportConfig {
        self.config.read().await.clone()
    }

    pub async fn update_config(&self, config: ImportConfig) {
        use soul_importer::FileManagementStrategy;

        tracing::debug!("Updating config: strategy={:?}", config.file_strategy);

        // Persist to database FIRST - save individual settings
        let strategy_str = match config.file_strategy {
            FileManagementStrategy::Move => "move",
            FileManagementStrategy::Copy => "copy",
            FileManagementStrategy::Reference => "reference",
        };

        // Save all settings
        if let Err(e) = soul_storage::settings::set_setting(
            &self.pool,
            &self.user_id,
            soul_storage::settings::SETTING_IMPORT_STRATEGY,
            &serde_json::json!(strategy_str),
        )
        .await
        {
            tracing::error!("Failed to persist strategy: {}", e);
            return;
        }

        if let Err(e) = soul_storage::settings::set_setting(
            &self.pool,
            &self.user_id,
            soul_storage::settings::SETTING_IMPORT_LIBRARY_PATH,
            &serde_json::json!(config.library_path.display().to_string()),
        )
        .await
        {
            tracing::error!("Failed to persist library path: {}", e);
            return;
        }

        if let Err(e) = soul_storage::settings::set_setting(
            &self.pool,
            &self.user_id,
            soul_storage::settings::SETTING_IMPORT_CONFIDENCE_THRESHOLD,
            &serde_json::json!(config.confidence_threshold),
        )
        .await
        {
            tracing::error!("Failed to persist confidence threshold: {}", e);
            return;
        }

        if let Err(e) = soul_storage::settings::set_setting(
            &self.pool,
            &self.user_id,
            soul_storage::settings::SETTING_IMPORT_FILE_NAMING_PATTERN,
            &serde_json::json!(config.file_naming_pattern.clone()),
        )
        .await
        {
            tracing::error!("Failed to persist file naming pattern: {}", e);
            return;
        }

        if let Err(e) = soul_storage::settings::set_setting(
            &self.pool,
            &self.user_id,
            soul_storage::settings::SETTING_IMPORT_SKIP_DUPLICATES,
            &serde_json::json!(config.skip_duplicates),
        )
        .await
        {
            tracing::error!("Failed to persist skip duplicates: {}", e);
            return;
        }

        tracing::debug!("Config persisted to database");

        // Update in-memory cache
        *self.config.write().await = config;

        tracing::debug!("Config updated in memory");
    }

    pub async fn import_files(&self, app: AppHandle, files: Vec<PathBuf>) -> Result<(), String> {
        tracing::debug!("Starting import of {} files", files.len());

        // Check if already importing
        if self.is_importing().await {
            tracing::warn!("Import already in progress, queueing not supported yet");
            return Err(
                "Import already in progress. Please wait for current import to complete."
                    .to_string(),
            );
        }

        // Set initial state
        *self.current_import.lock().await = Some(ImportState {
            total_files: files.len(),
            processed: 0,
            successful: 0,
            failed: 0,
            duplicates: 0,
            is_running: true,
        });
        tracing::debug!("Import state set");

        // IMPORTANT: Reload config from database to ensure we have the latest settings
        // This prevents race conditions where the UI updates the config but import starts
        // before the async update completes
        tracing::debug!("Reloading config from database...");

        let strategy_str: String =
            soul_storage::settings::get_import_strategy(&self.pool, &self.user_id)
                .await
                .map_err(|e| format!("Failed to reload import strategy: {}", e))?;

        let file_strategy = match strategy_str.as_str() {
            "move" => soul_importer::FileManagementStrategy::Move,
            "reference" => soul_importer::FileManagementStrategy::Reference,
            _ => soul_importer::FileManagementStrategy::Copy,
        };

        let library_path: PathBuf =
            soul_storage::settings::get_import_library_path(&self.pool, &self.user_id)
                .await
                .map_err(|e| format!("Failed to reload library path: {}", e))?
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    // Fallback to cached config's library path if not in database
                    self.config
                        .try_read()
                        .map(|c| c.library_path.clone())
                        .unwrap_or_default()
                });

        let confidence_threshold: u8 =
            soul_storage::settings::get_import_confidence_threshold(&self.pool, &self.user_id)
                .await
                .map_err(|e| format!("Failed to reload confidence threshold: {}", e))?;

        let file_naming_pattern: String =
            soul_storage::settings::get_import_file_naming_pattern(&self.pool, &self.user_id)
                .await
                .map_err(|e| format!("Failed to reload file naming pattern: {}", e))?;

        let skip_duplicates: bool =
            soul_storage::settings::get_import_skip_duplicates(&self.pool, &self.user_id)
                .await
                .map_err(|e| format!("Failed to reload skip duplicates: {}", e))?;

        let config = ImportConfig {
            library_path: library_path.clone(),
            file_strategy,
            confidence_threshold,
            file_naming_pattern,
            skip_duplicates,
        };

        tracing::debug!("Using strategy: {:?}", config.file_strategy);
        tracing::debug!("Library path: {:?}", config.library_path);

        let importer = MusicImporter::new(self.pool.clone(), config);

        tracing::debug!("Creating importer and starting import");
        let (mut progress_rx, handle) = importer.import_files(&files).await.map_err(|e| {
            tracing::error!("Importer error: {}", e);
            // Clear state on error
            let current_import = self.current_import.clone();
            let cleanup_handle = tokio::spawn(async move {
                *current_import.lock().await = None;
            });
            // Log errors from cleanup task
            tokio::spawn(async move {
                if let Err(e) = cleanup_handle.await {
                    tracing::error!("[IMPORT] Cleanup task panicked: {:?}", e);
                }
            });
            e.to_string()
        })?;

        tracing::debug!("Import started, spawning progress listener");

        // Spawn task to handle progress updates
        let app_clone = app.clone();
        let progress_handle = tokio::spawn(async move {
            tracing::debug!("Starting progress listener");
            while let Some(progress) = progress_rx.recv().await {
                let update = ImportProgressUpdate::from(progress);
                tracing::debug!(
                    "Progress: {}/{} files",
                    update.processed_files,
                    update.total_files
                );
                // Emit progress to frontend
                if let Err(e) = app_clone.emit("import-progress", update) {
                    tracing::warn!(error = %e, event = "import-progress", "Failed to emit event to frontend");
                }
            }
            tracing::debug!("Progress channel closed");
        });

        // Log errors from progress listener
        tokio::spawn(async move {
            if let Err(e) = progress_handle.await {
                tracing::error!("[IMPORT] Progress listener task panicked: {:?}", e);
            }
        });

        // Wait for import to complete in background
        let current_import = self.current_import.clone();
        let completion_handle = tokio::spawn(async move {
            tracing::debug!("Waiting for import to complete");
            match handle.await {
                Ok(Ok(summary)) => {
                    tracing::debug!(
                        "Import completed successfully: {} successful, {} failed",
                        summary.successful,
                        summary.failed
                    );
                    // Emit completion
                    let response = ImportSummaryResponse::from(summary);
                    if let Err(e) = app.emit("import-complete", response) {
                        tracing::error!(error = %e, event = "import-complete", "Failed to emit event to frontend");
                    }
                }
                Ok(Err(e)) => {
                    tracing::error!("Import error: {}", e);
                    if let Err(e) = app.emit("import-error", e.to_string()) {
                        tracing::error!(error = %e, event = "import-error", "Failed to emit event to frontend");
                    }
                }
                Err(e) => {
                    tracing::error!("Task panicked: {}", e);
                    if let Err(e) = app.emit("import-error", format!("Task panicked: {}", e)) {
                        tracing::error!(error = %e, event = "import-error", "Failed to emit event to frontend");
                    }
                }
            }
            // Clear import state
            tracing::debug!("Clearing import state");
            *current_import.lock().await = None;
        });

        // Log errors from completion handler
        tokio::spawn(async move {
            if let Err(e) = completion_handle.await {
                tracing::error!("[IMPORT] Completion handler task panicked: {:?}", e);
            }
        });

        tracing::debug!("Background tasks spawned, returning Ok");
        Ok(())
    }

    pub async fn import_directory(&self, app: AppHandle, directory: PathBuf) -> Result<(), String> {
        tracing::debug!("Scanning directory: {:?}", directory);

        // Scan directory first
        let scanner = soul_importer::scanner::FileScanner::new();
        let files = scanner.scan_directory(&directory).map_err(|e| {
            tracing::error!("Scan error: {}", e);
            e.to_string()
        })?;

        tracing::debug!("Found {} files", files.len());

        self.import_files(app, files).await
    }

    pub async fn cancel_import(&self) -> Result<(), String> {
        // Mark as not running
        if let Some(state) = self.current_import.lock().await.as_mut() {
            state.is_running = false;
            Ok(())
        } else {
            Err("No import in progress".to_string())
        }
    }
}

// Tauri commands

#[tauri::command]
pub async fn import_files(
    app: AppHandle,
    files: Vec<String>,
    manager: tauri::State<'_, crate::lazy_workers::LazyImportManager>,
) -> Result<(), String> {
    tracing::debug!("Received {} files", files.len());
    for (i, file) in files.iter().enumerate() {
        tracing::debug!("File {}: {}", i + 1, file);
    }

    let paths: Vec<PathBuf> = files.into_iter().map(PathBuf::from).collect();
    let manager_ref = manager.get().await?;
    let result = manager_ref.import_files(app, paths).await;

    match &result {
        Ok(_) => tracing::debug!("Import started successfully"),
        Err(e) => tracing::error!("Import failed: {}", e),
    }

    result
}

#[tauri::command]
pub async fn import_directory(
    app: AppHandle,
    directory: String,
    manager: tauri::State<'_, crate::lazy_workers::LazyImportManager>,
) -> Result<(), String> {
    tracing::debug!("Received directory: {}", directory);

    let manager_ref = manager.get().await?;
    let result = manager_ref
        .import_directory(app, PathBuf::from(directory))
        .await;

    match &result {
        Ok(_) => tracing::debug!("Import started successfully"),
        Err(e) => tracing::error!("Import failed: {}", e),
    }

    result
}

#[tauri::command]
pub async fn cancel_import(
    manager: tauri::State<'_, crate::lazy_workers::LazyImportManager>,
) -> Result<(), String> {
    let manager_ref = manager.get().await?;
    manager_ref.cancel_import().await
}

#[tauri::command]
pub async fn is_importing(
    manager: tauri::State<'_, crate::lazy_workers::LazyImportManager>,
) -> Result<bool, String> {
    let manager_ref = manager.get().await?;
    Ok(manager_ref.is_importing().await)
}

#[tauri::command]
pub async fn get_import_config(
    manager: tauri::State<'_, crate::lazy_workers::LazyImportManager>,
) -> Result<ImportConfig, String> {
    let manager_ref = manager.get().await?;
    Ok(manager_ref.get_config().await)
}

#[tauri::command]
pub async fn update_import_config(
    config: ImportConfig,
    manager: tauri::State<'_, crate::lazy_workers::LazyImportManager>,
) -> Result<(), String> {
    let manager_ref = manager.get().await?;
    manager_ref.update_config(config).await;
    Ok(())
}

/// Get all configured sources
#[tauri::command]
pub async fn get_all_sources(
    pool: tauri::State<'_, SqlitePool>,
) -> Result<Vec<SourceInfo>, String> {
    let sources = soul_storage::sources::get_all(&pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(sources
        .into_iter()
        .map(|s| {
            let source_type_str = match s.source_type {
                soul_core::types::SourceType::Local => "local".to_string(),
                soul_core::types::SourceType::Server => "server".to_string(),
            };
            SourceInfo {
                id: s.id,
                name: s.name,
                source_type: source_type_str,
                is_active: s.is_active,
                is_online: s.is_online,
            }
        })
        .collect())
}

/// Set a source as active and trigger a sync
#[tauri::command]
pub async fn set_active_source(
    app: AppHandle,
    id: i64,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<(), String> {
    // Set the source as active
    soul_storage::sources::set_active(&pool, id)
        .await
        .map_err(|e| e.to_string())?;

    // Emit event to trigger sync
    app.emit("sync-required", soul_sync::SyncTrigger::SourceActivation)
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Open file dialog for selecting audio files
#[tauri::command]
pub async fn open_file_dialog(
    _app: AppHandle,
    multiple: bool,
    filters: Vec<DialogFilter>,
) -> Result<Option<Vec<String>>, String> {
    use rfd::FileDialog;

    // Build the dialog
    let mut dialog = FileDialog::new();

    // Add filters
    for filter in filters {
        let extensions: Vec<&str> = filter.extensions.iter().map(|s| s.as_str()).collect();
        dialog = dialog.add_filter(&filter.name, &extensions);
    }

    // Show dialog and get result
    if multiple {
        let files = dialog.pick_files();
        Ok(files.map(|paths| paths.into_iter().map(|p| p.display().to_string()).collect()))
    } else {
        let file = dialog.pick_file();
        Ok(file.map(|p| vec![p.display().to_string()]))
    }
}

/// Open folder dialog
#[tauri::command]
pub async fn open_folder_dialog(_app: AppHandle) -> Result<Option<String>, String> {
    use rfd::FileDialog;

    let folder = FileDialog::new().pick_folder();
    Ok(folder.map(|p| p.display().to_string()))
}

/// Check if a path is a directory
#[tauri::command]
pub async fn is_directory(path: String) -> Result<bool, String> {
    let path_buf = std::path::PathBuf::from(&path);

    // Use async metadata to avoid blocking on slow/network storage
    match tokio::fs::metadata(&path_buf).await {
        Ok(metadata) => Ok(metadata.is_dir()),
        Err(_) => Ok(false), // Path doesn't exist or can't be read
    }
}

/// Scan a directory for audio files (for play-without-import feature)
#[tauri::command]
pub async fn scan_directory_for_audio(path: String) -> Result<Vec<String>, String> {
    // Wrap blocking WalkDir operations in spawn_blocking to avoid macOS spinning wheel
    let path_buf = std::path::PathBuf::from(&path);

    tokio::task::spawn_blocking(move || {
        let scanner = soul_importer::scanner::FileScanner::new();
        scanner
            .scan_directory(&path_buf)
            .map(|files| files.into_iter().map(|p| p.display().to_string()).collect())
    })
    .await
    .map_err(|e| format!("Scan task failed: {}", e))?
    .map_err(|e| e.to_string())
}

/// Lightweight metadata returned for externally dropped/opened files
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileMetadata {
    pub file_path: String,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration_seconds: Option<f64>,
    pub track_number: Option<u32>,
}

/// Read tag metadata for a list of file paths (used when playing dropped files)
#[tauri::command]
pub async fn get_metadata_for_paths(paths: Vec<String>) -> Result<Vec<FileMetadata>, String> {
    tokio::task::spawn_blocking(move || {
        paths
            .into_iter()
            .map(|path_str| {
                let path = std::path::Path::new(&path_str);
                match soul_importer::metadata::extract_metadata(path) {
                    Ok(meta) => FileMetadata {
                        file_path: path_str,
                        title: meta.title,
                        artist: meta.artists.into_iter().next(),
                        album: meta.album,
                        duration_seconds: meta.duration_seconds,
                        track_number: meta.track_number,
                    },
                    Err(_) => FileMetadata {
                        file_path: path_str,
                        title: None,
                        artist: None,
                        album: None,
                        duration_seconds: None,
                        track_number: None,
                    },
                }
            })
            .collect()
    })
    .await
    .map_err(|e| format!("Metadata task failed: {e}"))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogFilter {
    pub name: String,
    pub extensions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceInfo {
    pub id: i64,
    pub name: String,
    pub source_type: String,
    pub is_active: bool,
    pub is_online: bool,
}
