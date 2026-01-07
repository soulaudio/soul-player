/// Admin API routes
use crate::{
    error::{Result},
    middleware::AuthenticatedUser,
    state::AppState,
};
use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use soul_core::{storage::StorageContext, types::{User, UserId}};

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct CreateUserResponse {
    pub user: User,
    pub success: bool,
}

#[derive(Debug, Deserialize)]
pub struct ScanRequest {
    pub path: String,
}

#[derive(Debug, Serialize)]
pub struct ScanResponse {
    pub status: String,
    pub message: String,
}

/// POST /api/admin/users
/// Create a new user account
pub async fn create_user(
    State(app_state): State<AppState>,
    _auth: AuthenticatedUser,
    Json(req): Json<CreateUserRequest>,
) -> Result<Json<CreateUserResponse>> {
    // TODO: Verify admin role

    // Hash password
    let _password_hash = app_state.auth_service.hash_password(&req.password)?;

    // TODO: Create user - need to implement create_user in StorageContext or call soul_storage::users::create directly
    // let user = app_state.db.create_user(&req.username).await?;

    // Store credentials
    // store_user_credentials(&app_state, &user.id, &password_hash).await?;

    // Placeholder user for now
    let user = User {
        id: 1,
        name: req.username,
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    Ok(Json(CreateUserResponse {
        user,
        success: true,
    }))
}

/// GET /api/admin/users
/// List all users
pub async fn list_users(
    State(app_state): State<AppState>,
    _auth: AuthenticatedUser,
) -> Result<Json<Vec<User>>> {
    // TODO: Verify admin role

    let users = app_state.db.get_all_users().await?;
    Ok(Json(users))
}

/// DELETE /api/admin/users/:id
/// Delete a user account
pub async fn delete_user(
    Path(_user_id): Path<String>,
    State(_app_state): State<AppState>,
    _auth: AuthenticatedUser,
) -> Result<Json<serde_json::Value>> {
    // TODO: Verify admin role
    // TODO: Implement user deletion (need to add to Storage trait)

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "User deletion not yet implemented"
    })))
}

/// POST /api/admin/scan
/// Trigger a library scan
pub async fn trigger_scan(
    _auth: AuthenticatedUser,
    Json(req): Json<ScanRequest>,
) -> Result<Json<ScanResponse>> {
    // TODO: Verify admin role
    // TODO: Trigger background scan task

    Ok(Json(ScanResponse {
        status: "pending".to_string(),
        message: format!("Scan of {} queued", req.path),
    }))
}

/// GET /api/admin/scan/status
/// Get scan status
pub async fn scan_status(_auth: AuthenticatedUser) -> Result<Json<serde_json::Value>> {
    // TODO: Verify admin role
    // TODO: Get actual scan status

    Ok(Json(serde_json::json!({
        "status": "idle",
        "message": "No scan in progress"
    })))
}

/// Helper to store user credentials (placeholder)
async fn store_user_credentials(
    _app_state: &AppState,
    _user_id: &UserId,
    _password_hash: &str,
) -> Result<()> {
    // TODO: Implement actual database storage
    Ok(())
}
