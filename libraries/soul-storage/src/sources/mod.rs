use sqlx::{Row, SqlitePool};
use soul_core::{error::Result, types::*};

pub async fn get_all(pool: &SqlitePool) -> Result<Vec<Source>> {
    let rows = sqlx::query(
        "SELECT id, name, source_type, server_url, server_username, server_token,
                is_active, is_online, last_sync_at
         FROM sources
         ORDER BY id"
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|row| {
        let source_type: String = row.get("source_type");
        let config = if source_type == "local" {
            SourceConfig::Local
        } else {
            SourceConfig::Server {
                url: row.get::<Option<String>, _>("server_url").unwrap_or_default(),
                username: row.get::<Option<String>, _>("server_username").unwrap_or_default(),
                token: row.get("server_token"),
            }
        };

        Source {
            id: row.get("id"),
            name: row.get("name"),
            source_type: if source_type == "local" {
                SourceType::Local
            } else {
                SourceType::Server
            },
            config,
            is_active: row.get::<i64, _>("is_active") != 0,
            is_online: row.get::<i64, _>("is_online") != 0,
            last_sync_at: row.get("last_sync_at"),
        }
    }).collect())
}

pub async fn get_by_id(pool: &SqlitePool, id: SourceId) -> Result<Option<Source>> {
    let row = sqlx::query(
        "SELECT id, name, source_type, server_url, server_username, server_token,
                is_active, is_online, last_sync_at
         FROM sources
         WHERE id = ?"
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|row| {
        let source_type: String = row.get("source_type");
        let config = if source_type == "local" {
            SourceConfig::Local
        } else {
            SourceConfig::Server {
                url: row.get::<Option<String>, _>("server_url").unwrap_or_default(),
                username: row.get::<Option<String>, _>("server_username").unwrap_or_default(),
                token: row.get("server_token"),
            }
        };

        Source {
            id: row.get("id"),
            name: row.get("name"),
            source_type: if source_type == "local" {
                SourceType::Local
            } else {
                SourceType::Server
            },
            config,
            is_active: row.get::<i64, _>("is_active") != 0,
            is_online: row.get::<i64, _>("is_online") != 0,
            last_sync_at: row.get("last_sync_at"),
        }
    }))
}

pub async fn get_active_server(pool: &SqlitePool) -> Result<Option<Source>> {
    let row = sqlx::query(
        "SELECT id, name, source_type, server_url, server_username, server_token,
                is_active, is_online, last_sync_at
         FROM sources
         WHERE source_type = 'server' AND is_active = 1
         LIMIT 1"
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|row| Source {
        id: row.get("id"),
        name: row.get("name"),
        source_type: SourceType::Server,
        config: SourceConfig::Server {
            url: row.get::<Option<String>, _>("server_url").unwrap_or_default(),
            username: row.get::<Option<String>, _>("server_username").unwrap_or_default(),
            token: row.get("server_token"),
        },
        is_active: row.get::<i64, _>("is_active") != 0,
        is_online: row.get::<i64, _>("is_online") != 0,
        last_sync_at: row.get("last_sync_at"),
    }))
}

pub async fn create(pool: &SqlitePool, source: CreateSource) -> Result<Source> {
    let (source_type, server_url, server_username, server_token) = match &source.config {
        SourceConfig::Local => ("local", None, None, None),
        SourceConfig::Server { url, username, token } => {
            ("server", Some(url.clone()), Some(username.clone()), token.clone())
        }
    };

    let result = sqlx::query(
        "INSERT INTO sources (name, source_type, server_url, server_username, server_token)
         VALUES (?, ?, ?, ?, ?)"
    )
    .bind(&source.name)
    .bind(source_type)
    .bind(&server_url)
    .bind(&server_username)
    .bind(&server_token)
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
    sqlx::query("UPDATE sources SET is_active = 0 WHERE source_type = 'server'")
        .execute(pool)
        .await?;

    // Activate the specified server
    sqlx::query("UPDATE sources SET is_active = 1 WHERE id = ? AND source_type = 'server'")
        .bind(id)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn update_status(pool: &SqlitePool, id: SourceId, is_online: bool) -> Result<()> {
    sqlx::query("UPDATE sources SET is_online = ? WHERE id = ?")
        .bind(is_online)
        .bind(id)
        .execute(pool)
        .await?;

    Ok(())
}
