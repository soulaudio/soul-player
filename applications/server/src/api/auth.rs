/// Authentication API routes
use crate::{
    error::{Result, ServerError},
    state::AppState,
};
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use soul_core::{Storage, UserId};

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
}

#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Debug, Serialize)]
pub struct RefreshResponse {
    pub access_token: String,
    pub token_type: String,
}

/// POST /api/auth/login
pub async fn login(
    State(app_state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>> {
    // Look up user by name
    let users = app_state.db.get_all_users().await?;
    let user = users
        .iter()
        .find(|u| u.name == req.username)
        .ok_or_else(|| ServerError::Auth("Invalid username or password".to_string()))?;

    // Get password hash from database
    // TODO: Need to query user_credentials table
    // For now, this is a placeholder - we need to add the credentials query
    let password_hash = get_user_password_hash(&app_state, &user.id).await?;

    // Verify password
    if !app_state.auth_service.verify_password(&req.password, &password_hash)? {
        return Err(ServerError::Auth("Invalid username or password".to_string()));
    }

    // Create tokens
    let access_token = app_state.auth_service.create_access_token(&user.id)?;
    let refresh_token = app_state.auth_service.create_refresh_token(&user.id)?;

    Ok(Json(LoginResponse {
        access_token,
        refresh_token,
        token_type: "Bearer".to_string(),
    }))
}

/// POST /api/auth/refresh
pub async fn refresh(
    State(app_state): State<AppState>,
    Json(req): Json<RefreshRequest>,
) -> Result<Json<RefreshResponse>> {
    // Verify refresh token
    let user_id = app_state.auth_service.verify_refresh_token(&req.refresh_token)?;

    // Create new access token
    let access_token = app_state.auth_service.create_access_token(&user_id)?;

    Ok(Json(RefreshResponse {
        access_token,
        token_type: "Bearer".to_string(),
    }))
}

/// Helper to get user password hash from database
async fn get_user_password_hash(app_state: &AppState, user_id: &UserId) -> Result<String> {
    use sqlx::Row;
    let pool = app_state.db.pool();

    let row = sqlx::query(
        "SELECT password_hash FROM user_credentials WHERE user_id = ?"
    )
    .bind(user_id.as_str())
    .fetch_optional(pool)
    .await
    .map_err(|e| ServerError::Internal(format!("Database error: {}", e)))?
    .ok_or_else(|| ServerError::Auth("Invalid username or password".to_string()))?;

    Ok(row.get("password_hash"))
}
