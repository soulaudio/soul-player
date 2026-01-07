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
