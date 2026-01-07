use soul_core::{error::Result, types::*};
use sqlx::SqlitePool;

pub async fn get_all(pool: &SqlitePool) -> Result<Vec<Album>> {
    let rows = sqlx::query!(
        "SELECT a.id, a.title, a.artist_id, ar.name as artist_name, a.year,
                a.cover_art_path, a.musicbrainz_id, a.created_at, a.updated_at
         FROM albums a
         LEFT JOIN artists ar ON a.artist_id = ar.id
         ORDER BY a.title"
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| Album {
            id: row.id.expect("album id should not be null"),
            title: row.title,
            artist_id: row.artist_id,
            artist_name: row.artist_name,
            year: row.year.map(|y| y as i32),
            cover_art_path: row.cover_art_path,
            musicbrainz_id: row.musicbrainz_id,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
        .collect())
}

pub async fn get_by_id(pool: &SqlitePool, id: AlbumId) -> Result<Option<Album>> {
    let row = sqlx::query!(
        "SELECT a.id, a.title, a.artist_id, ar.name as artist_name, a.year,
                a.cover_art_path, a.musicbrainz_id, a.created_at, a.updated_at
         FROM albums a
         LEFT JOIN artists ar ON a.artist_id = ar.id
         WHERE a.id = ?",
        id
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|row| Album {
        id: row.id,
        title: row.title,
        artist_id: row.artist_id,
        artist_name: row.artist_name,
        year: row.year.map(|y| y as i32),
        cover_art_path: row.cover_art_path,
        musicbrainz_id: row.musicbrainz_id,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }))
}

pub async fn get_by_artist(pool: &SqlitePool, artist_id: ArtistId) -> Result<Vec<Album>> {
    let rows = sqlx::query!(
        "SELECT a.id, a.title, a.artist_id, ar.name as artist_name, a.year,
                a.cover_art_path, a.musicbrainz_id, a.created_at, a.updated_at
         FROM albums a
         LEFT JOIN artists ar ON a.artist_id = ar.id
         WHERE a.artist_id = ?
         ORDER BY a.year DESC, a.title",
        artist_id
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| Album {
            id: row.id.expect("album id should not be null"),
            title: row.title,
            artist_id: row.artist_id,
            artist_name: Some(row.artist_name),
            year: row.year.map(|y| y as i32),
            cover_art_path: row.cover_art_path,
            musicbrainz_id: row.musicbrainz_id,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
        .collect())
}

pub async fn create(pool: &SqlitePool, album: CreateAlbum) -> Result<Album> {
    let result = sqlx::query!(
        "INSERT INTO albums (title, artist_id, year, musicbrainz_id)
         VALUES (?, ?, ?, ?)",
        album.title,
        album.artist_id,
        album.year,
        album.musicbrainz_id
    )
    .execute(pool)
    .await?;

    let id = result.last_insert_rowid();

    get_by_id(pool, id).await?.ok_or_else(|| {
        soul_core::SoulError::Storage("Failed to retrieve created album".to_string())
    })
}
