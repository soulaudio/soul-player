use soul_core::{error::Result, types::*};
use sqlx::SqlitePool;

pub async fn get_all(pool: &SqlitePool) -> Result<Vec<Artist>> {
    Ok(sqlx::query_as!(
        Artist,
        "SELECT id as 'id!', name, sort_name, musicbrainz_id, cover_art_path, created_at, updated_at
         FROM artists
         ORDER BY sort_name COLLATE NOCASE, name COLLATE NOCASE"
    )
    .fetch_all(pool)
    .await?)
}

pub async fn get_by_id(pool: &SqlitePool, id: ArtistId) -> Result<Option<Artist>> {
    Ok(sqlx::query_as!(
        Artist,
        "SELECT id as 'id!', name, sort_name, musicbrainz_id, cover_art_path, created_at, updated_at
         FROM artists
         WHERE id = ?",
        id
    )
    .fetch_optional(pool)
    .await?)
}

pub async fn find_by_name(pool: &SqlitePool, name: &str) -> Result<Option<Artist>> {
    Ok(sqlx::query_as!(
        Artist,
        "SELECT id as 'id!', name, sort_name, musicbrainz_id, cover_art_path, created_at, updated_at
         FROM artists
         WHERE name = ?",
        name
    )
    .fetch_optional(pool)
    .await?)
}

pub async fn create(pool: &SqlitePool, artist: CreateArtist) -> Result<Artist> {
    let result: sqlx::sqlite::SqliteQueryResult = sqlx::query!(
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

/// Update artist cover art path
pub async fn update_cover_art_path(
    pool: &SqlitePool,
    artist_id: ArtistId,
    cover_art_path: Option<&str>,
) -> Result<()> {
    sqlx::query!(
        "UPDATE artists SET cover_art_path = ?, updated_at = datetime('now') WHERE id = ?",
        cover_art_path,
        artist_id
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Get track counts for all artists in a single query
pub async fn get_track_counts(
    pool: &SqlitePool,
) -> Result<std::collections::HashMap<ArtistId, i32>> {
    let rows: Vec<_> = sqlx::query!(
        "SELECT artist_id as 'artist_id!', COUNT(DISTINCT track_id) as count
         FROM track_artists
         GROUP BY artist_id"
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| (row.artist_id, row.count as i32))
        .collect())
}

/// Get album counts for all artists in a single query
pub async fn get_album_counts(
    pool: &SqlitePool,
) -> Result<std::collections::HashMap<ArtistId, i32>> {
    let rows: Vec<_> = sqlx::query!(
        "SELECT artist_id, COUNT(*) as count
         FROM albums
         WHERE artist_id IS NOT NULL
         GROUP BY artist_id"
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .filter_map(|row| {
            // Filter out rows with missing artist_id
            row.artist_id.map(|artist_id| (artist_id, row.count as i32))
        })
        .collect::<std::collections::HashMap<_, _>>())
}

/// Delete artists that have no available tracks (neither as primary nor via track_artists).
/// Returns the number of artists deleted.
pub async fn delete_orphaned(pool: &SqlitePool) -> Result<i64> {
    let result = sqlx::query(
        "DELETE FROM artists
         WHERE id NOT IN (
             SELECT DISTINCT artist_id FROM tracks WHERE artist_id IS NOT NULL AND is_available = 1
         )
         AND id NOT IN (
             SELECT DISTINCT ta.artist_id FROM track_artists ta
             JOIN tracks t ON ta.track_id = t.id
             WHERE t.is_available = 1
         )",
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected() as i64)
}
