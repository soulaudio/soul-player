/// Device management API routes
use crate::{
    error::{Result, ServerError},
    middleware::AuthenticatedUser,
    state::AppState,
};
use axum::{
    extract::{Path, State},
    Json,
};
use serde::Serialize;
use soul_core::types::{Device, DeviceType, RegisterDevice};

#[derive(Debug, Serialize)]
pub struct DevicesResponse {
    pub devices: Vec<DeviceResponse>,
}

#[derive(Debug, Serialize)]
pub struct DeviceResponse {
    pub id: String,
    pub name: String,
    pub device_type: DeviceType,
    pub is_active: bool,
    pub last_seen_at: i64,
    pub created_at: i64,
}

impl DeviceResponse {
    fn from_device(device: Device, active_device_id: Option<&str>) -> Self {
        Self {
            is_active: active_device_id == Some(&device.id),
            id: device.id,
            name: device.name,
            device_type: device.device_type,
            last_seen_at: device.last_seen_at,
            created_at: device.created_at,
        }
    }
}

/// POST /api/devices - Register a new device
pub async fn register_device(
    State(app_state): State<AppState>,
    auth: AuthenticatedUser,
    Json(body): Json<RegisterDevice>,
) -> Result<Json<DeviceResponse>> {
    let user_id = auth.user_id().as_str();
    let device_id = uuid::Uuid::new_v4().to_string();

    let pool = app_state.db.pool();
    let device =
        soul_storage::devices::register(pool, &device_id, user_id, &body.name, body.device_type)
            .await?;

    // Get current active device for this user
    let state = soul_storage::playback_state::get(pool, user_id).await?;

    Ok(Json(DeviceResponse::from_device(
        device,
        state.active_device_id.as_deref(),
    )))
}

/// GET /api/devices - List user's devices
pub async fn list_devices(
    State(app_state): State<AppState>,
    auth: AuthenticatedUser,
) -> Result<Json<DevicesResponse>> {
    let user_id = auth.user_id().as_str();
    let pool = app_state.db.pool();

    let devices = soul_storage::devices::get_by_user(pool, user_id).await?;
    let state = soul_storage::playback_state::get(pool, user_id).await?;

    let devices: Vec<DeviceResponse> = devices
        .into_iter()
        .map(|d| DeviceResponse::from_device(d, state.active_device_id.as_deref()))
        .collect();

    Ok(Json(DevicesResponse { devices }))
}

/// DELETE /api/devices/:id - Unregister a device
pub async fn unregister_device(
    Path(device_id): Path<String>,
    State(app_state): State<AppState>,
    auth: AuthenticatedUser,
) -> Result<Json<serde_json::Value>> {
    let user_id = auth.user_id().as_str();
    let pool = app_state.db.pool();

    // Verify the device belongs to this user
    let device = soul_storage::devices::get_by_id(pool, &device_id)
        .await?
        .ok_or_else(|| ServerError::NotFound("Device not found".to_string()))?;

    if device.user_id != user_id {
        return Err(ServerError::Unauthorized(
            "Device does not belong to user".to_string(),
        ));
    }

    soul_storage::devices::unregister(pool, &device_id).await?;

    Ok(Json(serde_json::json!({ "success": true })))
}

/// PUT /api/devices/:id/activate - Set this device as the active player
pub async fn activate_device(
    Path(device_id): Path<String>,
    State(app_state): State<AppState>,
    auth: AuthenticatedUser,
) -> Result<Json<serde_json::Value>> {
    let user_id = auth.user_id().as_str();
    let pool = app_state.db.pool();

    // Verify the device belongs to this user
    let device = soul_storage::devices::get_by_id(pool, &device_id)
        .await?
        .ok_or_else(|| ServerError::NotFound("Device not found".to_string()))?;

    if device.user_id != user_id {
        return Err(ServerError::Unauthorized(
            "Device does not belong to user".to_string(),
        ));
    }

    // Set this device as active
    soul_storage::playback_state::set_active_device(pool, user_id, Some(&device_id)).await?;

    // Update last seen
    soul_storage::devices::update_last_seen(pool, &device_id).await?;

    // TODO: Broadcast to WebSocket connections

    Ok(Json(serde_json::json!({ "success": true, "active_device_id": device_id })))
}

/// POST /api/devices/:id/heartbeat - Update device last seen time
pub async fn heartbeat(
    Path(device_id): Path<String>,
    State(app_state): State<AppState>,
    auth: AuthenticatedUser,
) -> Result<Json<serde_json::Value>> {
    let user_id = auth.user_id().as_str();
    let pool = app_state.db.pool();

    // Verify the device belongs to this user
    let device = soul_storage::devices::get_by_id(pool, &device_id)
        .await?
        .ok_or_else(|| ServerError::NotFound("Device not found".to_string()))?;

    if device.user_id != user_id {
        return Err(ServerError::Unauthorized(
            "Device does not belong to user".to_string(),
        ));
    }

    soul_storage::devices::update_last_seen(pool, &device_id).await?;

    Ok(Json(serde_json::json!({ "success": true })))
}
