use soul_core::{error::Result, types::*};
use sqlx::SqlitePool;

pub async fn get_all(pool: &SqlitePool) -> Result<Vec<Artist>> {
    let rows = sqlx::query!(
        "SELECT id, name, sort_name, musicbrainz_id, created_at, updated_at
         FROM artists
         ORDER BY sort_name, name"
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| Artist {
            id: row.id.expect("artist id should not be null"),
            name: row.name,
            sort_name: row.sort_name,
            musicbrainz_id: row.musicbrainz_id,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
        .collect())
}

pub async fn get_by_id(pool: &SqlitePool, id: ArtistId) -> Result<Option<Artist>> {
    let row = sqlx::query!(
        "SELECT id, name, sort_name, musicbrainz_id, created_at, updated_at
         FROM artists
         WHERE id = ?",
        id
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|row| Artist {
        id: row.id,
        name: row.name,
        sort_name: row.sort_name,
        musicbrainz_id: row.musicbrainz_id,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }))
}

pub async fn find_by_name(pool: &SqlitePool, name: &str) -> Result<Option<Artist>> {
    let row = sqlx::query!(
        "SELECT id, name, sort_name, musicbrainz_id, created_at, updated_at
         FROM artists
         WHERE name = ?",
        name
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|row| Artist {
        id: row.id.expect("artist id should not be null"),
        name: row.name,
        sort_name: row.sort_name,
        musicbrainz_id: row.musicbrainz_id,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }))
}

pub async fn create(pool: &SqlitePool, artist: CreateArtist) -> Result<Artist> {
    let result = sqlx::query!(
        "INSERT INTO artists (name, sort_name, musicbrainz_id)
         VALUES (?, ?, ?)",
        artist.name,
        artist.sort_name,
        artist.musicbrainz_id
    )
    .execute(pool)
    .await?;

    let id = result.last_insert_rowid();

    get_by_id(pool, id).await?.ok_or_else(|| {
        soul_core::SoulError::Storage("Failed to retrieve created artist".to_string())
    })
}
