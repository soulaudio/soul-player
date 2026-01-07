/// Authentication API routes
use crate::{
    error::{Result, ServerError},
    state::AppState,
};
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use soul_core::{storage::StorageContext, types::UserId};

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
    let users: Vec<soul_core::types::User> = app_state.db.get_all_users().await?;
    let user = users
        .iter()
        .find(|u| u.name == req.username)
        .ok_or_else(|| ServerError::Auth("Invalid username or password".to_string()))?;

    // Convert user.id (i64) to string for get_password_hash
    let user_id_str = user.id.to_string();

    // Get password hash from database
    let password_hash = soul_storage::users::get_password_hash(app_state.db.pool(), &user_id_str)
        .await?
        .ok_or_else(|| ServerError::Auth("Invalid username or password".to_string()))?;

    // Verify password
    if !app_state
        .auth_service
        .verify_password(&req.password, &password_hash)?
    {
        return Err(ServerError::Auth(
            "Invalid username or password".to_string(),
        ));
    }

    // Create tokens - convert i64 to UserId
    let user_id = UserId::new(user_id_str.clone());
    let access_token = app_state.auth_service.create_access_token(&user_id)?;
    let refresh_token = app_state.auth_service.create_refresh_token(&user_id)?;

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
    let user_id = app_state
        .auth_service
        .verify_refresh_token(&req.refresh_token)?;

    // Create new access token
    let access_token = app_state.auth_service.create_access_token(&user_id)?;

    Ok(Json(RefreshResponse {
        access_token,
        token_type: "Bearer".to_string(),
    }))
}
