/// Library Sources API routes
///
/// Manages watched folders for the server.
use crate::{error::ServerError, middleware::AuthenticatedUser, state::AppState};
use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use soul_core::types::{
    CreateLibrarySource, ScanProgress as StorageScanProgress, UpdateLibrarySource,
};

type Result<T> = std::result::Result<T, ServerError>;

/// Library source for API responses
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibrarySourceResponse {
    pub id: i64,
    pub name: String,
    pub path: String,
    pub enabled: bool,
    pub sync_deletes: bool,
    pub last_scan_at: Option<i64>,
    pub scan_status: String,
    pub error_message: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Request to create a new library source
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSourceRequest {
    pub name: String,
    pub path: String,
    #[serde(default = "default_true")]
    pub sync_deletes: bool,
}

fn default_true() -> bool {
    true
}

/// Request to update a library source
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSourceRequest {
    pub name: Option<String>,
    pub enabled: Option<bool>,
    pub sync_deletes: Option<bool>,
}

/// Scan progress for API responses
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanProgressResponse {
    pub id: i64,
    pub source_id: i64,
    pub source_name: Option<String>,
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
    pub percentage: f32,
}

impl ScanProgressResponse {
    fn from_storage(progress: StorageScanProgress, source_name: Option<String>) -> Self {
        let percentage = if let Some(total) = progress.total_files {
            if total > 0 {
                (progress.processed_files as f32 / total as f32) * 100.0
            } else {
                0.0
            }
        } else {
            0.0
        };

        Self {
            id: progress.id,
            source_id: progress.library_source_id,
            source_name,
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
        }
    }
}

/// GET /api/sources
/// List all library sources for the authenticated user
pub async fn list_sources(
    State(app_state): State<AppState>,
    auth: AuthenticatedUser,
) -> Result<Json<Vec<LibrarySourceResponse>>> {
    let device_id = get_server_device_id();
    let user_id = auth.user_id().as_str();

    let sources =
        soul_storage::library_sources::get_by_user_device(app_state.db.pool(), user_id, &device_id)
            .await
            .map_err(|e| ServerError::Internal(e.to_string()))?;

    let result: Vec<LibrarySourceResponse> = sources
        .into_iter()
        .map(|s| LibrarySourceResponse {
            id: s.id,
            name: s.name,
            path: s.path,
            enabled: s.enabled,
            sync_deletes: s.sync_deletes,
            last_scan_at: s.last_scan_at,
            scan_status: s.scan_status.as_str().to_string(),
            error_message: s.error_message,
            created_at: s.created_at,
            updated_at: s.updated_at,
        })
        .collect();

    Ok(Json(result))
}

/// POST /api/sources
/// Create a new library source
pub async fn create_source(
    State(app_state): State<AppState>,
    auth: AuthenticatedUser,
    Json(req): Json<CreateSourceRequest>,
) -> Result<Json<LibrarySourceResponse>> {
    let device_id = get_server_device_id();
    let user_id = auth.user_id().as_str();

    let create = CreateLibrarySource {
        name: req.name,
        path: req.path,
        sync_deletes: req.sync_deletes,
    };

    let source =
        soul_storage::library_sources::create(app_state.db.pool(), user_id, &device_id, &create)
            .await
            .map_err(|e| ServerError::Internal(e.to_string()))?;

    Ok(Json(LibrarySourceResponse {
        id: source.id,
        name: source.name,
        path: source.path,
        enabled: source.enabled,
        sync_deletes: source.sync_deletes,
        last_scan_at: source.last_scan_at,
        scan_status: source.scan_status.as_str().to_string(),
        error_message: source.error_message,
        created_at: source.created_at,
        updated_at: source.updated_at,
    }))
}

/// GET /api/sources/:id
/// Get a specific library source
pub async fn get_source(
    State(app_state): State<AppState>,
    auth: AuthenticatedUser,
    Path(source_id): Path<i64>,
) -> Result<Json<LibrarySourceResponse>> {
    let source = soul_storage::library_sources::get_by_id(app_state.db.pool(), source_id)
        .await
        .map_err(|e| ServerError::Internal(e.to_string()))?
        .ok_or_else(|| ServerError::NotFound("Source not found".to_string()))?;

    // Verify ownership
    if source.user_id != auth.user_id().as_str() {
        return Err(ServerError::Unauthorized("Access denied".to_string()));
    }

    Ok(Json(LibrarySourceResponse {
        id: source.id,
        name: source.name,
        path: source.path,
        enabled: source.enabled,
        sync_deletes: source.sync_deletes,
        last_scan_at: source.last_scan_at,
        scan_status: source.scan_status.as_str().to_string(),
        error_message: source.error_message,
        created_at: source.created_at,
        updated_at: source.updated_at,
    }))
}

/// PUT /api/sources/:id
/// Update a library source
pub async fn update_source(
    State(app_state): State<AppState>,
    auth: AuthenticatedUser,
    Path(source_id): Path<i64>,
    Json(req): Json<UpdateSourceRequest>,
) -> Result<Json<LibrarySourceResponse>> {
    // Get existing source and verify ownership
    let source = soul_storage::library_sources::get_by_id(app_state.db.pool(), source_id)
        .await
        .map_err(|e| ServerError::Internal(e.to_string()))?
        .ok_or_else(|| ServerError::NotFound("Source not found".to_string()))?;

    if source.user_id != auth.user_id().as_str() {
        return Err(ServerError::Unauthorized("Access denied".to_string()));
    }

    // Update fields
    let update = UpdateLibrarySource {
        name: req.name,
        enabled: req.enabled,
        sync_deletes: req.sync_deletes,
    };

    soul_storage::library_sources::update(app_state.db.pool(), source_id, &update)
        .await
        .map_err(|e| ServerError::Internal(e.to_string()))?;

    // Reload source
    let source = soul_storage::library_sources::get_by_id(app_state.db.pool(), source_id)
        .await
        .map_err(|e| ServerError::Internal(e.to_string()))?
        .ok_or_else(|| ServerError::NotFound("Source not found".to_string()))?;

    Ok(Json(LibrarySourceResponse {
        id: source.id,
        name: source.name,
        path: source.path,
        enabled: source.enabled,
        sync_deletes: source.sync_deletes,
        last_scan_at: source.last_scan_at,
        scan_status: source.scan_status.as_str().to_string(),
        error_message: source.error_message,
        created_at: source.created_at,
        updated_at: source.updated_at,
    }))
}

/// DELETE /api/sources/:id
/// Delete a library source
pub async fn delete_source(
    State(app_state): State<AppState>,
    auth: AuthenticatedUser,
    Path(source_id): Path<i64>,
) -> Result<Json<serde_json::Value>> {
    // Get existing source and verify ownership
    let source = soul_storage::library_sources::get_by_id(app_state.db.pool(), source_id)
        .await
        .map_err(|e| ServerError::Internal(e.to_string()))?
        .ok_or_else(|| ServerError::NotFound("Source not found".to_string()))?;

    if source.user_id != auth.user_id().as_str() {
        return Err(ServerError::Unauthorized("Access denied".to_string()));
    }

    soul_storage::library_sources::delete(app_state.db.pool(), source_id)
        .await
        .map_err(|e| ServerError::Internal(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Source deleted"
    })))
}

/// POST /api/sources/:id/scan
/// Trigger a scan for a specific source
pub async fn trigger_source_scan(
    State(app_state): State<AppState>,
    auth: AuthenticatedUser,
    Path(source_id): Path<i64>,
) -> Result<Json<serde_json::Value>> {
    let device_id = get_server_device_id();

    // Get existing source and verify ownership
    let source = soul_storage::library_sources::get_by_id(app_state.db.pool(), source_id)
        .await
        .map_err(|e| ServerError::Internal(e.to_string()))?
        .ok_or_else(|| ServerError::NotFound("Source not found".to_string()))?;

    if source.user_id != auth.user_id().as_str() {
        return Err(ServerError::Unauthorized("Access denied".to_string()));
    }

    // Start scan in background
    let pool = app_state.db.pool().clone();
    let user_id = auth.user_id().as_str().to_string();

    tokio::spawn(async move {
        if let Err(e) = run_source_scan(&pool, &user_id, &device_id, source_id).await {
            tracing::error!("Scan failed for source {}: {}", source_id, e);
        }
    });

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Scan started",
        "sourceId": source_id
    })))
}

/// GET /api/sources/:id/scan/progress
/// Get scan progress for a specific source
pub async fn get_scan_progress(
    State(app_state): State<AppState>,
    auth: AuthenticatedUser,
    Path(source_id): Path<i64>,
) -> Result<Json<Option<ScanProgressResponse>>> {
    // Verify ownership
    let source = soul_storage::library_sources::get_by_id(app_state.db.pool(), source_id)
        .await
        .map_err(|e| ServerError::Internal(e.to_string()))?
        .ok_or_else(|| ServerError::NotFound("Source not found".to_string()))?;

    if source.user_id != auth.user_id().as_str() {
        return Err(ServerError::Unauthorized("Access denied".to_string()));
    }

    let progress = soul_storage::scan_progress::get_running(app_state.db.pool(), source_id)
        .await
        .map_err(|e| ServerError::Internal(e.to_string()))?;

    Ok(Json(progress.map(|p| {
        ScanProgressResponse::from_storage(p, Some(source.name.clone()))
    })))
}

/// GET /api/sources/scan/active
/// Get all active scans for the user
pub async fn get_active_scans(
    State(app_state): State<AppState>,
    auth: AuthenticatedUser,
) -> Result<Json<Vec<ScanProgressResponse>>> {
    let device_id = get_server_device_id();
    let user_id = auth.user_id().as_str();

    let sources =
        soul_storage::library_sources::get_by_user_device(app_state.db.pool(), user_id, &device_id)
            .await
            .map_err(|e| ServerError::Internal(e.to_string()))?;

    let mut active_scans = Vec::new();

    for source in sources {
        if let Some(progress) =
            soul_storage::scan_progress::get_running(app_state.db.pool(), source.id)
                .await
                .map_err(|e| ServerError::Internal(e.to_string()))?
        {
            active_scans.push(ScanProgressResponse::from_storage(
                progress,
                Some(source.name.clone()),
            ));
        }
    }

    Ok(Json(active_scans))
}

// Helper functions

fn get_server_device_id() -> String {
    // Server uses a fixed device ID
    std::env::var("SOUL_SERVER_DEVICE_ID").unwrap_or_else(|_| "soul-server".to_string())
}

async fn run_source_scan(
    pool: &sqlx::SqlitePool,
    user_id: &str,
    device_id: &str,
    source_id: i64,
) -> anyhow::Result<()> {
    use soul_importer::library_scanner::LibraryScanner;

    let source = soul_storage::library_sources::get_by_id(pool, source_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Source not found"))?;

    let scanner = LibraryScanner::new(pool.clone(), user_id, device_id);
    scanner.scan_source(&source).await?;

    Ok(())
}
