use crate::error::StorageError;
use soul_core::{error::Result, types::*};
use sqlx::SqlitePool;

pub async fn get_all(pool: &SqlitePool) -> Result<Vec<Genre>> {
    let rows: Vec<_> = sqlx::query!(
        "SELECT id, name, canonical_name, created_at
         FROM genres
         ORDER BY name"
    )
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|row| {
            Ok(Genre {
                id: row
                    .id
                    .ok_or_else(|| StorageError::MissingField("genre.id".to_string()))?,
                name: row.name,
                canonical_name: row.canonical_name,
                created_at: row.created_at,
            })
        })
        .collect()
}

pub async fn get_by_id(pool: &SqlitePool, id: GenreId) -> Result<Option<Genre>> {
    let row: Option<_> = sqlx::query!(
        "SELECT id, name, canonical_name, created_at
         FROM genres
         WHERE id = ?",
        id
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|row| Genre {
        id: row.id,
        name: row.name,
        canonical_name: row.canonical_name,
        created_at: row.created_at,
    }))
}

pub async fn find_by_name(pool: &SqlitePool, name: &str) -> Result<Option<Genre>> {
    let row: Option<_> = sqlx::query!(
        "SELECT id, name, canonical_name, created_at
         FROM genres
         WHERE name = ?",
        name
    )
    .fetch_optional(pool)
    .await?;

    match row {
        Some(row) => match row.id {
            Some(id) => Ok(Some(Genre {
                id,
                name: row.name,
                canonical_name: row.canonical_name,
                created_at: row.created_at,
            })),
            None => Err(StorageError::MissingField("genre.id".to_string()).into()),
        },
        None => Ok(None),
    }
}

pub async fn find_by_canonical_name(
    pool: &SqlitePool,
    canonical_name: &str,
) -> Result<Option<Genre>> {
    let row: Option<_> = sqlx::query!(
        "SELECT id, name, canonical_name, created_at
         FROM genres
         WHERE canonical_name = ?",
        canonical_name
    )
    .fetch_optional(pool)
    .await?;

    match row {
        Some(row) => match row.id {
            Some(id) => Ok(Some(Genre {
                id,
                name: row.name,
                canonical_name: row.canonical_name,
                created_at: row.created_at,
            })),
            None => Err(StorageError::MissingField("genre.id".to_string()).into()),
        },
        None => Ok(None),
    }
}

pub async fn create(pool: &SqlitePool, genre: CreateGenre) -> Result<Genre> {
    let result: sqlx::sqlite::SqliteQueryResult = sqlx::query!(
        "INSERT INTO genres (name, canonical_name)
         VALUES (?, ?)",
        genre.name,
        genre.canonical_name
    )
    .execute(pool)
    .await?;

    let id = result.last_insert_rowid();

    get_by_id(pool, id).await?.ok_or_else(|| {
        soul_core::SoulError::Storage("Failed to retrieve created genre".to_string())
    })
}

/// Get all genres for a specific track
pub async fn get_by_track(pool: &SqlitePool, track_id: TrackId) -> Result<Vec<Genre>> {
    let rows: Vec<_> = sqlx::query!(
        "SELECT g.id, g.name, g.canonical_name, g.created_at
         FROM genres g
         INNER JOIN track_genres tg ON g.id = tg.genre_id
         WHERE tg.track_id = ?
         ORDER BY g.name",
        track_id
    )
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|row| {
            Ok(Genre {
                id: row
                    .id
                    .ok_or_else(|| StorageError::MissingField("genre.id".to_string()))?,
                name: row.name,
                canonical_name: row.canonical_name,
                created_at: row.created_at,
            })
        })
        .collect()
}

/// Add a genre to a track
pub async fn add_to_track(pool: &SqlitePool, track_id: TrackId, genre_id: GenreId) -> Result<()> {
    sqlx::query!(
        "INSERT OR IGNORE INTO track_genres (track_id, genre_id)
         VALUES (?, ?)",
        track_id,
        genre_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Remove a genre from a track
pub async fn remove_from_track(
    pool: &SqlitePool,
    track_id: TrackId,
    genre_id: GenreId,
) -> Result<()> {
    sqlx::query!(
        "DELETE FROM track_genres
         WHERE track_id = ? AND genre_id = ?",
        track_id,
        genre_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Remove all genres from a track
pub async fn clear_track_genres(pool: &SqlitePool, track_id: TrackId) -> Result<()> {
    sqlx::query!("DELETE FROM track_genres WHERE track_id = ?", track_id)
        .execute(pool)
        .await?;

    Ok(())
}
