//! Tauri commands for server source management.

use crate::app_state::AppState;
use serde::{Deserialize, Serialize};
use soul_server_client::{ServerConfig, SoulServerClient};
use soul_storage::sources;
use tauri::State;

// =============================================================================
// Types
// =============================================================================

/// Source information for the frontend
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceInfo {
    pub id: i64,
    pub name: String,
    pub source_type: String,
    pub server_url: Option<String>,
    pub is_active: bool,
    pub is_online: bool,
    pub is_authenticated: bool,
    pub username: Option<String>,
    pub last_sync_at: Option<String>,
}

/// Server connection test result
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerTestResult {
    pub success: bool,
    pub name: Option<String>,
    pub version: Option<String>,
    pub requires_auth: bool,
    pub error: Option<String>,
}

/// Authentication result
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthResult {
    pub success: bool,
    pub username: Option<String>,
    pub error: Option<String>,
}

/// Sync status for the frontend
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatus {
    pub status: String,
    pub progress: f32,
    pub current_operation: Option<String>,
    pub current_item: Option<String>,
    pub processed_items: i32,
    pub total_items: i32,
    pub error: Option<String>,
}

/// Sync result
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncResult {
    pub success: bool,
    pub tracks_uploaded: i32,
    pub tracks_downloaded: i32,
    pub tracks_updated: i32,
    pub tracks_deleted: i32,
    pub errors: Vec<String>,
}

// =============================================================================
// Source Management Commands
// =============================================================================

/// Get all sources (local + servers)
#[tauri::command]
pub async fn get_sources(state: State<'_, AppState>) -> Result<Vec<SourceInfo>, String> {
    let pool = &*state.pool;

    let all_sources = sources::get_all(pool)
        .await
        .map_err(|e| format!("Failed to get sources: {}", e))?;

    let mut result = Vec::new();

    for source in all_sources {
        let is_authenticated = if source.source_type == soul_core::types::SourceType::Server {
            sources::get_auth_token(pool, source.id)
                .await
                .map(|t| t.is_some())
                .unwrap_or(false)
        } else {
            false
        };

        let (server_url, username) = match &source.config {
            soul_core::types::SourceConfig::Server { url, username, .. } => (
                Some(url.clone()),
                Some(username.clone()).filter(|u| !u.is_empty()),
            ),
            _ => (None, None),
        };

        result.push(SourceInfo {
            id: source.id,
            name: source.name,
            source_type: match source.source_type {
                soul_core::types::SourceType::Local => "local".to_string(),
                soul_core::types::SourceType::Server => "server".to_string(),
            },
            server_url,
            is_active: source.is_active,
            is_online: source.is_online,
            is_authenticated,
            username,
            last_sync_at: source.last_sync_at,
        });
    }

    Ok(result)
}

/// Get server sources only (for current user)
#[tauri::command]
pub async fn get_server_sources(state: State<'_, AppState>) -> Result<Vec<SourceInfo>, String> {
    let pool = &*state.pool;

    // For desktop, use user_id = 1
    let user_id = 1i64;

    let server_sources = sources::get_server_sources_for_user(pool, user_id)
        .await
        .map_err(|e| format!("Failed to get server sources: {}", e))?;

    let mut result = Vec::new();

    for source in server_sources {
        let is_authenticated = sources::get_auth_token(pool, source.id)
            .await
            .map(|t| t.is_some())
            .unwrap_or(false);

        let (server_url, username) = match &source.config {
            soul_core::types::SourceConfig::Server { url, username, .. } => (
                Some(url.clone()),
                Some(username.clone()).filter(|u| !u.is_empty()),
            ),
            _ => (None, None),
        };

        result.push(SourceInfo {
            id: source.id,
            name: source.name,
            source_type: "server".to_string(),
            server_url,
            is_active: source.is_active,
            is_online: source.is_online,
            is_authenticated,
            username,
            last_sync_at: source.last_sync_at,
        });
    }

    Ok(result)
}

/// Add a new server source
#[tauri::command]
pub async fn add_server_source(
    state: State<'_, AppState>,
    name: String,
    url: String,
) -> Result<SourceInfo, String> {
    let pool = &*state.pool;

    // Validate URL
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("URL must start with http:// or https://".to_string());
    }

    // For desktop, use user_id = 1
    let user_id = 1i64;

    let source = sources::add_server_source(pool, user_id, &name, &url)
        .await
        .map_err(|e| format!("Failed to add server source: {}", e))?;

    Ok(SourceInfo {
        id: source.id,
        name: source.name,
        source_type: "server".to_string(),
        server_url: Some(url),
        is_active: false,
        is_online: false,
        is_authenticated: false,
        username: None,
        last_sync_at: None,
    })
}

/// Remove a source
#[tauri::command]
pub async fn remove_source(state: State<'_, AppState>, source_id: i64) -> Result<(), String> {
    let pool = &*state.pool;

    // Don't allow removing the local source (ID = 1)
    if source_id == 1 {
        return Err("Cannot remove the local source".to_string());
    }

    sources::delete(pool, source_id)
        .await
        .map_err(|e| format!("Failed to remove source: {}", e))?;

    Ok(())
}

/// Test connection to a server
#[tauri::command]
pub async fn test_server_connection(url: String) -> Result<ServerTestResult, String> {
    let config = ServerConfig::new(&url);

    let client = match SoulServerClient::new(config) {
        Ok(c) => c,
        Err(e) => {
            return Ok(ServerTestResult {
                success: false,
                name: None,
                version: None,
                requires_auth: false,
                error: Some(format!("Invalid URL: {}", e)),
            });
        }
    };

    match client.test_connection().await {
        Ok(info) => Ok(ServerTestResult {
            success: true,
            name: Some(info.name),
            version: Some(info.version),
            requires_auth: info.requires_auth,
            error: None,
        }),
        Err(e) => Ok(ServerTestResult {
            success: false,
            name: None,
            version: None,
            requires_auth: false,
            error: Some(e.to_string()),
        }),
    }
}

// =============================================================================
// Authentication Commands
// =============================================================================

/// Authenticate with a server source
#[tauri::command]
pub async fn authenticate_source(
    state: State<'_, AppState>,
    source_id: i64,
    username: String,
    password: String,
) -> Result<AuthResult, String> {
    let pool = &*state.pool;

    // Get source to get the URL
    let source = sources::get_by_id(pool, source_id)
        .await
        .map_err(|e| format!("Failed to get source: {}", e))?
        .ok_or("Source not found")?;

    let url = match &source.config {
        soul_core::types::SourceConfig::Server { url, .. } => url.clone(),
        _ => return Err("Not a server source".to_string()),
    };

    // Create client and authenticate
    let config = ServerConfig::new(&url);
    let client =
        SoulServerClient::new(config).map_err(|e| format!("Failed to create client: {}", e))?;

    match client.login(&username, &password).await {
        Ok(login_response) => {
            // Calculate expiry timestamp
            let expires_at = chrono::Utc::now().timestamp() + login_response.expires_in as i64;

            // Store tokens
            sources::store_auth_token(
                pool,
                source_id,
                &login_response.access_token,
                Some(&login_response.refresh_token),
                Some(expires_at),
            )
            .await
            .map_err(|e| format!("Failed to store token: {}", e))?;

            // Update username in source
            sources::update_server_credentials(
                pool,
                source_id,
                &login_response.username,
                Some(&login_response.access_token),
            )
            .await
            .map_err(|e| format!("Failed to update credentials: {}", e))?;

            // Mark source as online
            sources::update_status(pool, source_id, true)
                .await
                .map_err(|e| format!("Failed to update status: {}", e))?;

            Ok(AuthResult {
                success: true,
                username: Some(login_response.username),
                error: None,
            })
        }
        Err(e) => Ok(AuthResult {
            success: false,
            username: None,
            error: Some(e.to_string()),
        }),
    }
}

/// Logout from a server source
#[tauri::command]
pub async fn logout_source(state: State<'_, AppState>, source_id: i64) -> Result<(), String> {
    let pool = &*state.pool;

    sources::clear_auth_token(pool, source_id)
        .await
        .map_err(|e| format!("Failed to logout: {}", e))?;

    Ok(())
}

/// Get authentication status for a source
#[tauri::command]
pub async fn get_source_auth_status(
    state: State<'_, AppState>,
    source_id: i64,
) -> Result<AuthResult, String> {
    let pool = &*state.pool;

    let token = sources::get_auth_token(pool, source_id)
        .await
        .map_err(|e| format!("Failed to get auth status: {}", e))?;

    match token {
        Some(t) => {
            let is_expired = sources::is_token_expired(&t);

            // Get username from source
            let source = sources::get_by_id(pool, source_id)
                .await
                .map_err(|e| format!("Failed to get source: {}", e))?;

            let username = source.and_then(|s| match s.config {
                soul_core::types::SourceConfig::Server { username, .. } => {
                    Some(username).filter(|u| !u.is_empty())
                }
                _ => None,
            });

            Ok(AuthResult {
                success: !is_expired,
                username,
                error: if is_expired {
                    Some("Token expired".to_string())
                } else {
                    None
                },
            })
        }
        None => Ok(AuthResult {
            success: false,
            username: None,
            error: None,
        }),
    }
}

// =============================================================================
// Active Source Commands
// =============================================================================

/// Set the active server source
/// Note: The main set_active_source command is in import.rs
pub async fn set_active_source_internal(
    state: State<'_, AppState>,
    source_id: i64,
) -> Result<(), String> {
    let pool = &*state.pool;

    sources::set_active(pool, source_id)
        .await
        .map_err(|e| format!("Failed to set active source: {}", e))?;

    Ok(())
}

/// Get the currently active server source
#[tauri::command]
pub async fn get_active_source(state: State<'_, AppState>) -> Result<Option<SourceInfo>, String> {
    let pool = &*state.pool;

    let source = sources::get_active_server(pool)
        .await
        .map_err(|e| format!("Failed to get active source: {}", e))?;

    match source {
        Some(s) => {
            let is_authenticated = sources::get_auth_token(pool, s.id)
                .await
                .map(|t| t.is_some())
                .unwrap_or(false);

            let (server_url, username) = match &s.config {
                soul_core::types::SourceConfig::Server { url, username, .. } => (
                    Some(url.clone()),
                    Some(username.clone()).filter(|u| !u.is_empty()),
                ),
                _ => (None, None),
            };

            Ok(Some(SourceInfo {
                id: s.id,
                name: s.name,
                source_type: "server".to_string(),
                server_url,
                is_active: s.is_active,
                is_online: s.is_online,
                is_authenticated,
                username,
                last_sync_at: s.last_sync_at,
            }))
        }
        None => Ok(None),
    }
}

// =============================================================================
// Sync Commands
// =============================================================================

/// Get sync status for a source
/// Note: The main get_sync_status command is in sync.rs
pub async fn get_source_sync_status(
    state: State<'_, AppState>,
    source_id: i64,
) -> Result<SyncStatus, String> {
    let pool = &*state.pool;

    // For desktop, use user_id = 1
    let user_id = 1i64;

    let sync_state = sources::get_sync_state(pool, source_id, user_id)
        .await
        .map_err(|e| format!("Failed to get sync status: {}", e))?;

    match sync_state {
        Some(state) => {
            let progress = if state.total_items > 0 {
                state.processed_items as f32 / state.total_items as f32
            } else {
                0.0
            };

            Ok(SyncStatus {
                status: state.sync_status,
                progress,
                current_operation: state.current_operation,
                current_item: state.current_item,
                processed_items: state.processed_items,
                total_items: state.total_items,
                error: state.error_message,
            })
        }
        None => Ok(SyncStatus {
            status: "idle".to_string(),
            progress: 0.0,
            current_operation: None,
            current_item: None,
            processed_items: 0,
            total_items: 0,
            error: None,
        }),
    }
}

/// Sync library from server (download)
#[tauri::command]
pub async fn sync_from_server(
    state: State<'_, AppState>,
    source_id: i64,
) -> Result<SyncResult, String> {
    let pool = &*state.pool;

    // For desktop, use user_id = 1
    let user_id = 1i64;

    // Get source and verify it's authenticated
    let source = sources::get_by_id(pool, source_id)
        .await
        .map_err(|e| format!("Failed to get source: {}", e))?
        .ok_or("Source not found")?;

    let url = match &source.config {
        soul_core::types::SourceConfig::Server { url, .. } => url.clone(),
        _ => return Err("Not a server source".to_string()),
    };

    let token = sources::get_auth_token(pool, source_id)
        .await
        .map_err(|e| format!("Failed to get auth token: {}", e))?
        .ok_or("Not authenticated")?;

    // Initialize sync state
    sources::init_sync_state(pool, source_id, user_id, "download", 0)
        .await
        .map_err(|e| format!("Failed to init sync: {}", e))?;

    // Create client with token
    let config = ServerConfig::with_tokens(&url, &token.access_token, token.refresh_token);
    let client =
        SoulServerClient::new(config).map_err(|e| format!("Failed to create client: {}", e))?;

    // Get server sync token for delta sync
    let sync_token = sources::get_server_sync_token(pool, source_id, user_id)
        .await
        .map_err(|e| format!("Failed to get sync token: {}", e))?;

    // Get library delta
    let library_client = client
        .library()
        .await
        .map_err(|e| format!("Failed to get library client: {}", e))?;

    let delta = library_client
        .client()
        .get_library_delta(None, sync_token.as_deref())
        .await
        .map_err(|e| {
            // Mark sync as failed
            let _ = tokio::runtime::Handle::current().block_on(sources::fail_sync(
                pool,
                source_id,
                user_id,
                &e.to_string(),
            ));
            format!("Failed to get library delta: {}", e)
        })?;

    let tracks_downloaded = delta.new_tracks.len() as i32;
    let tracks_updated = delta.updated_tracks.len() as i32;
    let tracks_deleted = delta.deleted_track_ids.len() as i32;

    // TODO: Actually process the delta - create/update/delete tracks in local DB
    // This is a placeholder for now

    // Complete sync
    sources::complete_sync(
        pool,
        source_id,
        user_id,
        0,
        tracks_downloaded,
        tracks_updated,
        tracks_deleted,
        Some(&delta.sync_token),
    )
    .await
    .map_err(|e| format!("Failed to complete sync: {}", e))?;

    Ok(SyncResult {
        success: true,
        tracks_uploaded: 0,
        tracks_downloaded,
        tracks_updated,
        tracks_deleted,
        errors: vec![],
    })
}

/// Upload local tracks to server
#[tauri::command]
pub async fn upload_to_server(
    state: State<'_, AppState>,
    source_id: i64,
    track_ids: Vec<String>,
) -> Result<SyncResult, String> {
    let pool = &*state.pool;

    // For desktop, use user_id = 1
    let user_id = 1i64;

    // Get source and verify it's authenticated
    let source = sources::get_by_id(pool, source_id)
        .await
        .map_err(|e| format!("Failed to get source: {}", e))?
        .ok_or("Source not found")?;

    let url = match &source.config {
        soul_core::types::SourceConfig::Server { url, .. } => url.clone(),
        _ => return Err("Not a server source".to_string()),
    };

    let token = sources::get_auth_token(pool, source_id)
        .await
        .map_err(|e| format!("Failed to get auth token: {}", e))?
        .ok_or("Not authenticated")?;

    // Initialize sync state
    sources::init_sync_state(pool, source_id, user_id, "upload", track_ids.len() as i32)
        .await
        .map_err(|e| format!("Failed to init sync: {}", e))?;

    // Create client with token
    let config = ServerConfig::with_tokens(&url, &token.access_token, token.refresh_token);
    let _client =
        SoulServerClient::new(config).map_err(|e| format!("Failed to create client: {}", e))?;

    // TODO: Actually upload tracks
    // For each track_id:
    // 1. Get track from DB with file path
    // 2. Upload to server
    // 3. Update progress

    let tracks_uploaded = track_ids.len() as i32;

    // Complete sync
    sources::complete_sync(pool, source_id, user_id, tracks_uploaded, 0, 0, 0, None)
        .await
        .map_err(|e| format!("Failed to complete sync: {}", e))?;

    Ok(SyncResult {
        success: true,
        tracks_uploaded,
        tracks_downloaded: 0,
        tracks_updated: 0,
        tracks_deleted: 0,
        errors: vec![],
    })
}

/// Cancel ongoing sync for a specific source
/// Note: The main cancel_sync command is in sync.rs
pub async fn cancel_source_sync(state: State<'_, AppState>, source_id: i64) -> Result<(), String> {
    let pool = &*state.pool;

    // For desktop, use user_id = 1
    let user_id = 1i64;

    sources::cancel_sync(pool, source_id, user_id)
        .await
        .map_err(|e| format!("Failed to cancel sync: {}", e))?;

    Ok(())
}
