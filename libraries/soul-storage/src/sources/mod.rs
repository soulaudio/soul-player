use soul_core::{error::Result, types::*};
use sqlx::SqlitePool;

pub async fn get_all(pool: &SqlitePool) -> Result<Vec<Source>> {
    let rows = sqlx::query!(
        "SELECT id, name, source_type, server_url, server_username, server_token,
                is_active, is_online, last_sync_at
         FROM sources
         ORDER BY id"
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| {
            let config = if row.source_type == "local" {
                SourceConfig::Local
            } else {
                SourceConfig::Server {
                    url: row.server_url.unwrap_or_default(),
                    username: row.server_username.unwrap_or_default(),
                    token: row.server_token,
                }
            };

            Source {
                id: row.id,
                name: row.name,
                source_type: if row.source_type == "local" {
                    SourceType::Local
                } else {
                    SourceType::Server
                },
                config,
                is_active: row.is_active,
                is_online: row.is_online,
                last_sync_at: row.last_sync_at,
            }
        })
        .collect())
}

pub async fn get_by_id(pool: &SqlitePool, id: SourceId) -> Result<Option<Source>> {
    let row = sqlx::query!(
        "SELECT id, name, source_type, server_url, server_username, server_token,
                is_active, is_online, last_sync_at
         FROM sources
         WHERE id = ?",
        id
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|row| {
        let config = if row.source_type == "local" {
            SourceConfig::Local
        } else {
            SourceConfig::Server {
                url: row.server_url.unwrap_or_default(),
                username: row.server_username.unwrap_or_default(),
                token: row.server_token,
            }
        };

        Source {
            id: row.id,
            name: row.name,
            source_type: if row.source_type == "local" {
                SourceType::Local
            } else {
                SourceType::Server
            },
            config,
            is_active: row.is_active,
            is_online: row.is_online,
            last_sync_at: row.last_sync_at,
        }
    }))
}

pub async fn get_active_server(pool: &SqlitePool) -> Result<Option<Source>> {
    let row = sqlx::query!(
        "SELECT id, name, source_type, server_url, server_username, server_token,
                is_active, is_online, last_sync_at
         FROM sources
         WHERE source_type = 'server' AND is_active = 1
         LIMIT 1"
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|row| Source {
        id: row.id.expect("source id should not be null"),
        name: row.name,
        source_type: SourceType::Server,
        config: SourceConfig::Server {
            url: row.server_url.unwrap_or_default(),
            username: row.server_username.unwrap_or_default(),
            token: row.server_token,
        },
        is_active: row.is_active,
        is_online: row.is_online,
        last_sync_at: row.last_sync_at,
    }))
}

pub async fn create(pool: &SqlitePool, source: CreateSource) -> Result<Source> {
    let (source_type, server_url, server_username, server_token) = match &source.config {
        SourceConfig::Local => ("local", None, None, None),
        SourceConfig::Server {
            url,
            username,
            token,
        } => (
            "server",
            Some(url.clone()),
            Some(username.clone()),
            token.clone(),
        ),
    };

    let result = sqlx::query!(
        "INSERT INTO sources (name, source_type, server_url, server_username, server_token)
         VALUES (?, ?, ?, ?, ?)",
        source.name,
        source_type,
        server_url,
        server_username,
        server_token
    )
    .execute(pool)
    .await?;

    let id = result.last_insert_rowid();

    Ok(Source {
        id,
        name: source.name,
        source_type: if source_type == "local" {
            SourceType::Local
        } else {
            SourceType::Server
        },
        config: source.config,
        is_active: false,
        is_online: true,
        last_sync_at: None,
    })
}

pub async fn set_active(pool: &SqlitePool, id: SourceId) -> Result<()> {
    // Deactivate all servers first
    sqlx::query!("UPDATE sources SET is_active = 0 WHERE source_type = 'server'")
        .execute(pool)
        .await?;

    // Activate the specified server
    sqlx::query!(
        "UPDATE sources SET is_active = 1 WHERE id = ? AND source_type = 'server'",
        id
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn update_status(pool: &SqlitePool, id: SourceId, is_online: bool) -> Result<()> {
    sqlx::query!(
        "UPDATE sources SET is_online = ? WHERE id = ?",
        is_online,
        id
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Delete a source by ID
pub async fn delete(pool: &SqlitePool, id: SourceId) -> Result<()> {
    sqlx::query!("DELETE FROM sources WHERE id = ?", id)
        .execute(pool)
        .await?;

    Ok(())
}

/// Update server credentials (username and token)
pub async fn update_server_credentials(
    pool: &SqlitePool,
    id: SourceId,
    username: &str,
    token: Option<&str>,
) -> Result<()> {
    sqlx::query!(
        "UPDATE sources SET server_username = ?, server_token = ?, updated_at = datetime('now') WHERE id = ?",
        username,
        token,
        id
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Update the last sync timestamp
pub async fn update_last_sync(pool: &SqlitePool, id: SourceId) -> Result<()> {
    sqlx::query!(
        "UPDATE sources SET last_sync_at = datetime('now'), updated_at = datetime('now') WHERE id = ?",
        id
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Get all server sources for a user
pub async fn get_server_sources_for_user(pool: &SqlitePool, user_id: i64) -> Result<Vec<Source>> {
    let rows = sqlx::query!(
        "SELECT id, name, source_type, server_url, server_username, server_token,
                is_active, is_online, last_sync_at
         FROM sources
         WHERE source_type = 'server' AND (user_id = ? OR user_id IS NULL)
         ORDER BY name",
        user_id
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| Source {
            id: row.id.expect("source id should not be null"),
            name: row.name,
            source_type: SourceType::Server,
            config: SourceConfig::Server {
                url: row.server_url.unwrap_or_default(),
                username: row.server_username.unwrap_or_default(),
                token: row.server_token,
            },
            is_active: row.is_active,
            is_online: row.is_online,
            last_sync_at: row.last_sync_at,
        })
        .collect())
}

/// Add a new server source for a user
pub async fn add_server_source(
    pool: &SqlitePool,
    user_id: i64,
    name: &str,
    url: &str,
) -> Result<Source> {
    let result = sqlx::query!(
        "INSERT INTO sources (name, source_type, server_url, user_id, is_online)
         VALUES (?, 'server', ?, ?, 0)",
        name,
        url,
        user_id
    )
    .execute(pool)
    .await?;

    let id = result.last_insert_rowid();

    Ok(Source {
        id,
        name: name.to_string(),
        source_type: SourceType::Server,
        config: SourceConfig::Server {
            url: url.to_string(),
            username: String::new(),
            token: None,
        },
        is_active: false,
        is_online: false,
        last_sync_at: None,
    })
}

// =============================================================================
// Auth Token Management
// =============================================================================

/// Authentication token for a server source
#[derive(Debug, Clone)]
pub struct AuthToken {
    pub source_id: i64,
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<i64>,
}

/// Store authentication tokens for a source
pub async fn store_auth_token(
    pool: &SqlitePool,
    source_id: i64,
    access_token: &str,
    refresh_token: Option<&str>,
    expires_at: Option<i64>,
) -> Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO server_auth_tokens (source_id, access_token, refresh_token, token_expires_at)
        VALUES (?, ?, ?, ?)
        ON CONFLICT(source_id) DO UPDATE SET
            access_token = excluded.access_token,
            refresh_token = excluded.refresh_token,
            token_expires_at = excluded.token_expires_at,
            updated_at = datetime('now')
        "#,
        source_id,
        access_token,
        refresh_token,
        expires_at
    )
    .execute(pool)
    .await?;

    // Also update the legacy token field in sources table for compatibility
    sqlx::query!(
        "UPDATE sources SET server_token = ? WHERE id = ?",
        access_token,
        source_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Get authentication token for a source
pub async fn get_auth_token(pool: &SqlitePool, source_id: i64) -> Result<Option<AuthToken>> {
    let row = sqlx::query!(
        "SELECT source_id, access_token, refresh_token, token_expires_at
         FROM server_auth_tokens
         WHERE source_id = ?",
        source_id
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| AuthToken {
        source_id: r.source_id,
        access_token: r.access_token,
        refresh_token: r.refresh_token,
        expires_at: r.token_expires_at,
    }))
}

/// Clear authentication token for a source (logout)
pub async fn clear_auth_token(pool: &SqlitePool, source_id: i64) -> Result<()> {
    sqlx::query!(
        "DELETE FROM server_auth_tokens WHERE source_id = ?",
        source_id
    )
    .execute(pool)
    .await?;

    // Also clear the legacy token
    sqlx::query!(
        "UPDATE sources SET server_token = NULL, server_username = NULL WHERE id = ?",
        source_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Check if token is expired
pub fn is_token_expired(token: &AuthToken) -> bool {
    match token.expires_at {
        Some(expires_at) => {
            let now = chrono::Utc::now().timestamp();
            // Consider token expired if it expires within 60 seconds
            now >= expires_at - 60
        }
        None => false, // No expiry means it doesn't expire
    }
}

// =============================================================================
// Source Sync State Management
// =============================================================================

/// Sync state for a source-user combination
#[derive(Debug, Clone)]
pub struct SourceSyncState {
    pub source_id: i64,
    pub user_id: i64,
    pub last_sync_at: Option<i64>,
    pub last_sync_direction: Option<String>,
    pub sync_status: String,
    pub current_operation: Option<String>,
    pub current_item: Option<String>,
    pub total_items: i32,
    pub processed_items: i32,
    pub tracks_uploaded: i32,
    pub tracks_downloaded: i32,
    pub tracks_updated: i32,
    pub tracks_deleted: i32,
    pub error_message: Option<String>,
    pub server_sync_token: Option<String>,
}

/// Get sync state for a source-user combination
pub async fn get_sync_state(
    pool: &SqlitePool,
    source_id: i64,
    user_id: i64,
) -> Result<Option<SourceSyncState>> {
    let row = sqlx::query!(
        r#"
        SELECT source_id, user_id, last_sync_at, last_sync_direction, sync_status,
               current_operation, current_item, total_items, processed_items,
               tracks_uploaded, tracks_downloaded, tracks_updated, tracks_deleted,
               error_message, server_sync_token
        FROM source_sync_state
        WHERE source_id = ? AND user_id = ?
        "#,
        source_id,
        user_id
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| SourceSyncState {
        source_id: r.source_id,
        user_id: r.user_id,
        last_sync_at: r.last_sync_at,
        last_sync_direction: r.last_sync_direction,
        sync_status: r.sync_status,
        current_operation: r.current_operation,
        current_item: r.current_item,
        total_items: r.total_items.unwrap_or(0) as i32,
        processed_items: r.processed_items.unwrap_or(0) as i32,
        tracks_uploaded: r.tracks_uploaded.unwrap_or(0) as i32,
        tracks_downloaded: r.tracks_downloaded.unwrap_or(0) as i32,
        tracks_updated: r.tracks_updated.unwrap_or(0) as i32,
        tracks_deleted: r.tracks_deleted.unwrap_or(0) as i32,
        error_message: r.error_message,
        server_sync_token: r.server_sync_token,
    }))
}

/// Initialize or reset sync state for a source-user combination
pub async fn init_sync_state(
    pool: &SqlitePool,
    source_id: i64,
    user_id: i64,
    direction: &str,
    total_items: i32,
) -> Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO source_sync_state (source_id, user_id, sync_status, last_sync_direction, total_items, processed_items)
        VALUES (?, ?, 'syncing', ?, ?, 0)
        ON CONFLICT(source_id, user_id) DO UPDATE SET
            sync_status = 'syncing',
            last_sync_direction = excluded.last_sync_direction,
            total_items = excluded.total_items,
            processed_items = 0,
            current_operation = NULL,
            current_item = NULL,
            tracks_uploaded = 0,
            tracks_downloaded = 0,
            tracks_updated = 0,
            tracks_deleted = 0,
            error_message = NULL,
            updated_at = datetime('now')
        "#,
        source_id,
        user_id,
        direction,
        total_items
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Update sync progress
pub async fn update_sync_progress(
    pool: &SqlitePool,
    source_id: i64,
    user_id: i64,
    operation: &str,
    current_item: Option<&str>,
    processed_items: i32,
) -> Result<()> {
    sqlx::query!(
        r#"
        UPDATE source_sync_state
        SET current_operation = ?,
            current_item = ?,
            processed_items = ?,
            updated_at = datetime('now')
        WHERE source_id = ? AND user_id = ?
        "#,
        operation,
        current_item,
        processed_items,
        source_id,
        user_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Complete sync with results
pub async fn complete_sync(
    pool: &SqlitePool,
    source_id: i64,
    user_id: i64,
    tracks_uploaded: i32,
    tracks_downloaded: i32,
    tracks_updated: i32,
    tracks_deleted: i32,
    server_sync_token: Option<&str>,
) -> Result<()> {
    let now = chrono::Utc::now().timestamp();

    sqlx::query!(
        r#"
        UPDATE source_sync_state
        SET sync_status = 'idle',
            last_sync_at = ?,
            current_operation = NULL,
            current_item = NULL,
            tracks_uploaded = ?,
            tracks_downloaded = ?,
            tracks_updated = ?,
            tracks_deleted = ?,
            server_sync_token = COALESCE(?, server_sync_token),
            error_message = NULL,
            updated_at = datetime('now')
        WHERE source_id = ? AND user_id = ?
        "#,
        now,
        tracks_uploaded,
        tracks_downloaded,
        tracks_updated,
        tracks_deleted,
        server_sync_token,
        source_id,
        user_id
    )
    .execute(pool)
    .await?;

    // Also update the source's last_sync_at
    sqlx::query!(
        "UPDATE sources SET last_sync_at = datetime('now') WHERE id = ?",
        source_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Mark sync as failed
pub async fn fail_sync(
    pool: &SqlitePool,
    source_id: i64,
    user_id: i64,
    error_message: &str,
) -> Result<()> {
    sqlx::query!(
        r#"
        UPDATE source_sync_state
        SET sync_status = 'error',
            error_message = ?,
            updated_at = datetime('now')
        WHERE source_id = ? AND user_id = ?
        "#,
        error_message,
        source_id,
        user_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Cancel ongoing sync
pub async fn cancel_sync(pool: &SqlitePool, source_id: i64, user_id: i64) -> Result<()> {
    sqlx::query!(
        r#"
        UPDATE source_sync_state
        SET sync_status = 'cancelled',
            updated_at = datetime('now')
        WHERE source_id = ? AND user_id = ? AND sync_status = 'syncing'
        "#,
        source_id,
        user_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Get the server sync token for delta sync
pub async fn get_server_sync_token(
    pool: &SqlitePool,
    source_id: i64,
    user_id: i64,
) -> Result<Option<String>> {
    let row = sqlx::query!(
        "SELECT server_sync_token FROM source_sync_state WHERE source_id = ? AND user_id = ?",
        source_id,
        user_id
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.and_then(|r| r.server_sync_token))
}
