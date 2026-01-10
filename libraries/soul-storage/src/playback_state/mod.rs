//! Playback state management for multi-device sync

use crate::StorageError;
use soul_core::types::{PlaybackState, RepeatMode, UpdatePlaybackState};
use sqlx::SqlitePool;

type Result<T> = std::result::Result<T, StorageError>;

/// Get playback state for a user
///
/// Returns default state if none exists
pub async fn get(pool: &SqlitePool, user_id: &str) -> Result<PlaybackState> {
    let row = sqlx::query!(
        "SELECT user_id, active_device_id, is_playing, current_track_id, position_ms,
                volume, shuffle_enabled, repeat_mode, queue_json, updated_at
         FROM user_playback_state WHERE user_id = ?",
        user_id
    )
    .fetch_optional(pool)
    .await?;

    Ok(row
        .map(|r| PlaybackState {
            user_id: r.user_id,
            active_device_id: r.active_device_id,
            is_playing: r.is_playing != 0,
            current_track_id: r.current_track_id,
            position_ms: r.position_ms,
            volume: r.volume as i32,
            shuffle_enabled: r.shuffle_enabled != 0,
            repeat_mode: RepeatMode::from_str(&r.repeat_mode).unwrap_or(RepeatMode::Off),
            queue_json: r.queue_json,
            updated_at: r.updated_at,
        })
        .unwrap_or_else(|| PlaybackState {
            user_id: user_id.to_string(),
            ..Default::default()
        }))
}

/// Create or update playback state
pub async fn upsert(pool: &SqlitePool, state: &PlaybackState) -> Result<()> {
    let is_playing = if state.is_playing { 1 } else { 0 };
    let shuffle_enabled = if state.shuffle_enabled { 1 } else { 0 };
    let repeat_mode = state.repeat_mode.as_str();
    let now = chrono::Utc::now().timestamp();

    sqlx::query!(
        "INSERT INTO user_playback_state
         (user_id, active_device_id, is_playing, current_track_id, position_ms,
          volume, shuffle_enabled, repeat_mode, queue_json, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(user_id)
         DO UPDATE SET
            active_device_id = excluded.active_device_id,
            is_playing = excluded.is_playing,
            current_track_id = excluded.current_track_id,
            position_ms = excluded.position_ms,
            volume = excluded.volume,
            shuffle_enabled = excluded.shuffle_enabled,
            repeat_mode = excluded.repeat_mode,
            queue_json = excluded.queue_json,
            updated_at = excluded.updated_at",
        state.user_id,
        state.active_device_id,
        is_playing,
        state.current_track_id,
        state.position_ms,
        state.volume,
        shuffle_enabled,
        repeat_mode,
        state.queue_json,
        now
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Partially update playback state (only provided fields)
pub async fn update(
    pool: &SqlitePool,
    user_id: &str,
    update: &UpdatePlaybackState,
) -> Result<PlaybackState> {
    // Get current state
    let mut state = get(pool, user_id).await?;

    // Apply updates
    if let Some(is_playing) = update.is_playing {
        state.is_playing = is_playing;
    }
    if let Some(ref track_id) = update.current_track_id {
        state.current_track_id = Some(track_id.clone());
    }
    if let Some(position_ms) = update.position_ms {
        state.position_ms = position_ms;
    }
    if let Some(volume) = update.volume {
        state.volume = volume.clamp(0, 100);
    }
    if let Some(shuffle_enabled) = update.shuffle_enabled {
        state.shuffle_enabled = shuffle_enabled;
    }
    if let Some(repeat_mode) = update.repeat_mode {
        state.repeat_mode = repeat_mode;
    }
    if let Some(ref queue_json) = update.queue_json {
        state.queue_json = Some(queue_json.clone());
    }

    // Save and return
    upsert(pool, &state).await?;
    Ok(state)
}

/// Set the active device for playback
pub async fn set_active_device(
    pool: &SqlitePool,
    user_id: &str,
    device_id: Option<&str>,
) -> Result<()> {
    let now = chrono::Utc::now().timestamp();

    // First ensure a state row exists
    sqlx::query!(
        "INSERT INTO user_playback_state (user_id, updated_at)
         VALUES (?, ?)
         ON CONFLICT(user_id) DO NOTHING",
        user_id,
        now
    )
    .execute(pool)
    .await?;

    // Then update the active device
    sqlx::query!(
        "UPDATE user_playback_state SET active_device_id = ?, updated_at = ? WHERE user_id = ?",
        device_id,
        now,
        user_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Delete playback state for a user
pub async fn delete(pool: &SqlitePool, user_id: &str) -> Result<bool> {
    let result = sqlx::query!("DELETE FROM user_playback_state WHERE user_id = ?", user_id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
}
