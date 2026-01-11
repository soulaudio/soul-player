//! Playback context tracking for "Jump Back Into" and "Now Playing" context display
//!
//! Tracks what context (album, playlist, artist, etc.) a user is playing from.

use crate::StorageError;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

type Result<T> = std::result::Result<T, StorageError>;

/// Types of playback contexts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContextType {
    Album,
    Playlist,
    Artist,
    Genre,
    Tracks, // All tracks / library
}

impl ContextType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ContextType::Album => "album",
            ContextType::Playlist => "playlist",
            ContextType::Artist => "artist",
            ContextType::Genre => "genre",
            ContextType::Tracks => "tracks",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "album" => Some(ContextType::Album),
            "playlist" => Some(ContextType::Playlist),
            "artist" => Some(ContextType::Artist),
            "genre" => Some(ContextType::Genre),
            "tracks" => Some(ContextType::Tracks),
            _ => None,
        }
    }
}

/// A playback context record
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackContext {
    pub id: i64,
    pub user_id: String,
    pub context_type: ContextType,
    pub context_id: Option<String>,
    pub context_name: Option<String>,
    pub context_artwork_path: Option<String>,
    pub last_played_at: i64,
}

/// Input for recording a playback context
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordContext {
    pub context_type: ContextType,
    pub context_id: Option<String>,
    pub context_name: Option<String>,
    pub context_artwork_path: Option<String>,
}

/// Record or update a playback context (upsert)
///
/// When a user starts playing from an album/playlist/etc., call this to record it.
/// If the same context already exists, updates the last_played_at timestamp.
pub async fn record(pool: &SqlitePool, user_id: &str, context: &RecordContext) -> Result<()> {
    let context_type = context.context_type.as_str();
    let context_id = context.context_id.clone().unwrap_or_default();
    let now = chrono::Utc::now().timestamp();

    sqlx::query!(
        r#"
        INSERT INTO playback_contexts (user_id, context_type, context_id, context_name, context_artwork_path, last_played_at)
        VALUES (?, ?, ?, ?, ?, ?)
        ON CONFLICT(user_id, context_type, COALESCE(context_id, ''))
        DO UPDATE SET
            context_name = excluded.context_name,
            context_artwork_path = excluded.context_artwork_path,
            last_played_at = excluded.last_played_at
        "#,
        user_id,
        context_type,
        context_id,
        context.context_name,
        context.context_artwork_path,
        now
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Get recent playback contexts for a user
///
/// Returns contexts ordered by most recently played first.
pub async fn get_recent(
    pool: &SqlitePool,
    user_id: &str,
    limit: i32,
) -> Result<Vec<PlaybackContext>> {
    let rows = sqlx::query!(
        r#"
        SELECT id, user_id, context_type, context_id, context_name, context_artwork_path, last_played_at
        FROM playback_contexts
        WHERE user_id = ?
        ORDER BY last_played_at DESC
        LIMIT ?
        "#,
        user_id,
        limit
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .filter_map(|row| {
            let context_type = ContextType::from_str(&row.context_type)?;
            Some(PlaybackContext {
                id: row.id.expect("playback_contexts.id should not be null"),
                user_id: row.user_id,
                context_type,
                context_id: if row.context_id.as_deref() == Some("") {
                    None
                } else {
                    row.context_id
                },
                context_name: row.context_name,
                context_artwork_path: row.context_artwork_path,
                last_played_at: row.last_played_at,
            })
        })
        .collect())
}

/// Get the most recent playback context for a user (current context)
pub async fn get_current(pool: &SqlitePool, user_id: &str) -> Result<Option<PlaybackContext>> {
    let contexts = get_recent(pool, user_id, 1).await?;
    Ok(contexts.into_iter().next())
}

/// Get a specific playback context by type and ID
pub async fn get_by_type_and_id(
    pool: &SqlitePool,
    user_id: &str,
    context_type: ContextType,
    context_id: Option<&str>,
) -> Result<Option<PlaybackContext>> {
    let context_type_str = context_type.as_str();
    let context_id_str = context_id.unwrap_or("");

    let row = sqlx::query!(
        r#"
        SELECT id, user_id, context_type, context_id, context_name, context_artwork_path, last_played_at
        FROM playback_contexts
        WHERE user_id = ? AND context_type = ? AND COALESCE(context_id, '') = ?
        "#,
        user_id,
        context_type_str,
        context_id_str
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.and_then(|row| {
        let context_type = ContextType::from_str(&row.context_type)?;
        Some(PlaybackContext {
            id: row.id.expect("playback_contexts.id should not be null"),
            user_id: row.user_id,
            context_type,
            context_id: if row.context_id.as_deref() == Some("") {
                None
            } else {
                row.context_id
            },
            context_name: row.context_name,
            context_artwork_path: row.context_artwork_path,
            last_played_at: row.last_played_at,
        })
    }))
}

/// Delete a specific playback context
pub async fn delete(
    pool: &SqlitePool,
    user_id: &str,
    context_type: ContextType,
    context_id: Option<&str>,
) -> Result<bool> {
    let context_type_str = context_type.as_str();
    let context_id_str = context_id.unwrap_or("");

    let result = sqlx::query!(
        "DELETE FROM playback_contexts WHERE user_id = ? AND context_type = ? AND COALESCE(context_id, '') = ?",
        user_id,
        context_type_str,
        context_id_str
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

/// Clear all playback contexts for a user
pub async fn clear_all(pool: &SqlitePool, user_id: &str) -> Result<u64> {
    let result = sqlx::query!("DELETE FROM playback_contexts WHERE user_id = ?", user_id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected())
}
