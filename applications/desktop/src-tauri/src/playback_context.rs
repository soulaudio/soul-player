//! Playback context commands for tracking what context (album, playlist, etc.) is being played

use crate::app_state::AppState;
use serde::{Deserialize, Serialize};
use soul_storage::playback_contexts::{self, ContextType, PlaybackContext, RecordContext};
use tauri::State;

/// Frontend-friendly playback context
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontendPlaybackContext {
    pub id: i64,
    pub context_type: String,
    pub context_id: Option<String>,
    pub context_name: Option<String>,
    pub context_artwork_path: Option<String>,
    pub last_played_at: i64,
}

impl From<PlaybackContext> for FrontendPlaybackContext {
    fn from(ctx: PlaybackContext) -> Self {
        Self {
            id: ctx.id,
            context_type: ctx.context_type.as_str().to_string(),
            context_id: ctx.context_id,
            context_name: ctx.context_name,
            context_artwork_path: ctx.context_artwork_path,
            last_played_at: ctx.last_played_at,
        }
    }
}

/// Input for recording a playback context from the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordContextInput {
    pub context_type: String,
    pub context_id: Option<String>,
    pub context_name: Option<String>,
    pub context_artwork_path: Option<String>,
}

/// Record that the user started playing from a context (album, playlist, etc.)
#[tauri::command]
pub async fn record_playback_context(
    state: State<'_, AppState>,
    input: RecordContextInput,
) -> Result<(), String> {
    let context_type = ContextType::from_str(&input.context_type)
        .ok_or_else(|| format!("Invalid context type: {}", input.context_type))?;

    let context = RecordContext {
        context_type,
        context_id: input.context_id,
        context_name: input.context_name,
        context_artwork_path: input.context_artwork_path,
    };

    playback_contexts::record(&state.pool, &state.user_id, &context)
        .await
        .map_err(|e| e.to_string())
}

/// Get recent playback contexts for "Jump Back Into" section
#[tauri::command]
pub async fn get_recent_playback_contexts(
    state: State<'_, AppState>,
    limit: Option<i32>,
) -> Result<Vec<FrontendPlaybackContext>, String> {
    let limit = limit.unwrap_or(10);

    let contexts = playback_contexts::get_recent(&state.pool, &state.user_id, limit)
        .await
        .map_err(|e| e.to_string())?;

    Ok(contexts.into_iter().map(FrontendPlaybackContext::from).collect())
}

/// Get the current (most recent) playback context
#[tauri::command]
pub async fn get_current_playback_context(
    state: State<'_, AppState>,
) -> Result<Option<FrontendPlaybackContext>, String> {
    let context = playback_contexts::get_current(&state.pool, &state.user_id)
        .await
        .map_err(|e| e.to_string())?;

    Ok(context.map(FrontendPlaybackContext::from))
}

/// Clear all playback context history
#[tauri::command]
pub async fn clear_playback_context_history(state: State<'_, AppState>) -> Result<u64, String> {
    playback_contexts::clear_all(&state.pool, &state.user_id)
        .await
        .map_err(|e| e.to_string())
}
