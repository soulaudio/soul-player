/// Playback state API routes
use crate::{
    error::{Result, ServerError},
    middleware::AuthenticatedUser,
    state::AppState,
};
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use soul_core::types::{PlaybackState, RepeatMode, TransferPlayback, UpdatePlaybackState};

#[derive(Debug, Serialize)]
pub struct PlaybackResponse {
    pub user_id: String,
    pub active_device_id: Option<String>,
    pub is_playing: bool,
    pub current_track_id: Option<String>,
    pub position_ms: i64,
    pub volume: i32,
    pub shuffle_enabled: bool,
    pub repeat_mode: RepeatMode,
    pub queue: Option<Vec<String>>,
    pub updated_at: i64,
}

impl From<PlaybackState> for PlaybackResponse {
    fn from(state: PlaybackState) -> Self {
        let queue = state
            .queue_json
            .as_ref()
            .and_then(|json| serde_json::from_str(json).ok());

        Self {
            user_id: state.user_id,
            active_device_id: state.active_device_id,
            is_playing: state.is_playing,
            current_track_id: state.current_track_id,
            position_ms: state.position_ms,
            volume: state.volume,
            shuffle_enabled: state.shuffle_enabled,
            repeat_mode: state.repeat_mode,
            queue,
            updated_at: state.updated_at,
        }
    }
}

/// GET /api/playback - Get current playback state
pub async fn get_playback(
    State(app_state): State<AppState>,
    auth: AuthenticatedUser,
) -> Result<Json<PlaybackResponse>> {
    let user_id = auth.user_id().as_str();
    let pool = app_state.db.pool();

    let state = soul_storage::playback_state::get(pool, user_id).await?;

    Ok(Json(state.into()))
}

/// PUT /api/playback - Update playback state
pub async fn update_playback(
    State(app_state): State<AppState>,
    auth: AuthenticatedUser,
    Json(update): Json<UpdatePlaybackState>,
) -> Result<Json<PlaybackResponse>> {
    let user_id = auth.user_id().as_str();
    let pool = app_state.db.pool();

    let state = soul_storage::playback_state::update(pool, user_id, &update).await?;

    // TODO: Broadcast state change to WebSocket connections

    Ok(Json(state.into()))
}

#[derive(Debug, Deserialize)]
pub struct PlayCommand {
    pub track_id: Option<String>,
    pub queue: Option<Vec<String>>,
    pub start_index: Option<usize>,
}

/// POST /api/playback/play - Start or resume playback
pub async fn play(
    State(app_state): State<AppState>,
    auth: AuthenticatedUser,
    Json(cmd): Json<Option<PlayCommand>>,
) -> Result<Json<PlaybackResponse>> {
    let user_id = auth.user_id().as_str();
    let pool = app_state.db.pool();

    let mut update = UpdatePlaybackState {
        is_playing: Some(true),
        ..Default::default()
    };

    if let Some(cmd) = cmd {
        if let Some(track_id) = cmd.track_id {
            update.current_track_id = Some(track_id);
            update.position_ms = Some(0);
        }
        if let Some(queue) = cmd.queue {
            update.queue_json = Some(serde_json::to_string(&queue).unwrap_or_default());
            if let Some(idx) = cmd.start_index {
                if let Some(track_id) = queue.get(idx) {
                    update.current_track_id = Some(track_id.clone());
                    update.position_ms = Some(0);
                }
            }
        }
    }

    let state = soul_storage::playback_state::update(pool, user_id, &update).await?;

    // TODO: Broadcast state change to WebSocket connections

    Ok(Json(state.into()))
}

/// POST /api/playback/pause - Pause playback
pub async fn pause(
    State(app_state): State<AppState>,
    auth: AuthenticatedUser,
) -> Result<Json<PlaybackResponse>> {
    let user_id = auth.user_id().as_str();
    let pool = app_state.db.pool();

    let update = UpdatePlaybackState {
        is_playing: Some(false),
        ..Default::default()
    };

    let state = soul_storage::playback_state::update(pool, user_id, &update).await?;

    // TODO: Broadcast state change to WebSocket connections

    Ok(Json(state.into()))
}

#[derive(Debug, Deserialize)]
pub struct SeekCommand {
    pub position_ms: i64,
}

/// POST /api/playback/seek - Seek to position
pub async fn seek(
    State(app_state): State<AppState>,
    auth: AuthenticatedUser,
    Json(cmd): Json<SeekCommand>,
) -> Result<Json<PlaybackResponse>> {
    let user_id = auth.user_id().as_str();
    let pool = app_state.db.pool();

    let update = UpdatePlaybackState {
        position_ms: Some(cmd.position_ms),
        ..Default::default()
    };

    let state = soul_storage::playback_state::update(pool, user_id, &update).await?;

    // TODO: Broadcast state change to WebSocket connections

    Ok(Json(state.into()))
}

#[derive(Debug, Deserialize)]
pub struct VolumeCommand {
    pub volume: i32,
}

/// POST /api/playback/volume - Set volume
pub async fn set_volume(
    State(app_state): State<AppState>,
    auth: AuthenticatedUser,
    Json(cmd): Json<VolumeCommand>,
) -> Result<Json<PlaybackResponse>> {
    let user_id = auth.user_id().as_str();
    let pool = app_state.db.pool();

    let update = UpdatePlaybackState {
        volume: Some(cmd.volume.clamp(0, 100)),
        ..Default::default()
    };

    let state = soul_storage::playback_state::update(pool, user_id, &update).await?;

    // TODO: Broadcast state change to WebSocket connections

    Ok(Json(state.into()))
}

/// POST /api/playback/skip/next - Skip to next track
pub async fn skip_next(
    State(app_state): State<AppState>,
    auth: AuthenticatedUser,
) -> Result<Json<PlaybackResponse>> {
    let user_id = auth.user_id().as_str();
    let pool = app_state.db.pool();

    let state = soul_storage::playback_state::get(pool, user_id).await?;

    // Parse queue and find next track
    if let Some(queue_json) = &state.queue_json {
        if let Ok(queue) = serde_json::from_str::<Vec<String>>(queue_json) {
            if let Some(current_id) = &state.current_track_id {
                if let Some(current_idx) = queue.iter().position(|id| id == current_id) {
                    let next_idx = (current_idx + 1) % queue.len();
                    if let Some(next_track) = queue.get(next_idx) {
                        let update = UpdatePlaybackState {
                            current_track_id: Some(next_track.clone()),
                            position_ms: Some(0),
                            ..Default::default()
                        };
                        let state =
                            soul_storage::playback_state::update(pool, user_id, &update).await?;
                        return Ok(Json(state.into()));
                    }
                }
            }
        }
    }

    // No change if no queue or can't find next
    Ok(Json(state.into()))
}

/// POST /api/playback/skip/previous - Skip to previous track
pub async fn skip_previous(
    State(app_state): State<AppState>,
    auth: AuthenticatedUser,
) -> Result<Json<PlaybackResponse>> {
    let user_id = auth.user_id().as_str();
    let pool = app_state.db.pool();

    let state = soul_storage::playback_state::get(pool, user_id).await?;

    // Parse queue and find previous track
    if let Some(queue_json) = &state.queue_json {
        if let Ok(queue) = serde_json::from_str::<Vec<String>>(queue_json) {
            if let Some(current_id) = &state.current_track_id {
                if let Some(current_idx) = queue.iter().position(|id| id == current_id) {
                    let prev_idx = if current_idx == 0 {
                        queue.len() - 1
                    } else {
                        current_idx - 1
                    };
                    if let Some(prev_track) = queue.get(prev_idx) {
                        let update = UpdatePlaybackState {
                            current_track_id: Some(prev_track.clone()),
                            position_ms: Some(0),
                            ..Default::default()
                        };
                        let state =
                            soul_storage::playback_state::update(pool, user_id, &update).await?;
                        return Ok(Json(state.into()));
                    }
                }
            }
        }
    }

    // No change if no queue or can't find previous
    Ok(Json(state.into()))
}

/// POST /api/playback/transfer - Transfer playback to another device
pub async fn transfer(
    State(app_state): State<AppState>,
    auth: AuthenticatedUser,
    Json(transfer): Json<TransferPlayback>,
) -> Result<Json<PlaybackResponse>> {
    let user_id = auth.user_id().as_str();
    let pool = app_state.db.pool();

    // Verify the target device belongs to this user
    let device = soul_storage::devices::get_by_id(pool, &transfer.device_id)
        .await?
        .ok_or_else(|| ServerError::NotFound("Device not found".to_string()))?;

    if device.user_id != user_id {
        return Err(ServerError::Unauthorized(
            "Device does not belong to user".to_string(),
        ));
    }

    // Set the new active device
    soul_storage::playback_state::set_active_device(pool, user_id, Some(&transfer.device_id))
        .await?;

    // Optionally start playing
    if transfer.play {
        let update = UpdatePlaybackState {
            is_playing: Some(true),
            ..Default::default()
        };
        soul_storage::playback_state::update(pool, user_id, &update).await?;
    }

    let state = soul_storage::playback_state::get(pool, user_id).await?;

    // TODO: Broadcast transfer event to WebSocket connections

    Ok(Json(state.into()))
}
