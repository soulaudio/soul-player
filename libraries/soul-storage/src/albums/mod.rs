use crate::error::StorageError;
use soul_core::{error::Result, types::*};
use sqlx::SqlitePool;

pub async fn get_all(pool: &SqlitePool) -> Result<Vec<Album>> {
    let rows: Vec<_> = sqlx::query!(
        "SELECT a.id, a.title, a.artist_id, ar.name as artist_name, a.year,
                a.cover_art_path, a.artwork_source, a.musicbrainz_id, a.folder_path,
                a.created_at, a.updated_at
         FROM albums a
         LEFT JOIN artists ar ON a.artist_id = ar.id
         ORDER BY a.title COLLATE NOCASE"
    )
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|row| {
            Ok(Album {
                id: row
                    .id
                    .ok_or_else(|| StorageError::MissingField("album.id".to_string()))?,
                title: row.title,
                artist_id: row.artist_id,
                artist_name: row.artist_name,
                year: row.year.map(|y| y as i32),
                cover_art_path: row.cover_art_path,
                artwork_source: row.artwork_source,
                musicbrainz_id: row.musicbrainz_id,
                folder_path: row.folder_path,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
        })
        .collect()
}

pub async fn get_random(pool: &SqlitePool, limit: i64) -> Result<Vec<Album>> {
    let rows: Vec<_> = sqlx::query!(
        "SELECT a.id, a.title, a.artist_id, ar.name as artist_name, a.year,
                a.cover_art_path, a.artwork_source, a.musicbrainz_id, a.folder_path,
                a.created_at, a.updated_at
         FROM albums a
         LEFT JOIN artists ar ON a.artist_id = ar.id
         ORDER BY RANDOM()
         LIMIT ?",
        limit
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| Album {
            id: row.id,
            title: row.title,
            artist_id: row.artist_id,
            artist_name: row.artist_name,
            year: row.year.map(|y| y as i32),
            cover_art_path: row.cover_art_path,
            artwork_source: row.artwork_source,
            musicbrainz_id: row.musicbrainz_id,
            folder_path: row.folder_path,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
        .collect())
}

pub async fn get_recently_added(pool: &SqlitePool, limit: i64) -> Result<Vec<Album>> {
    let rows: Vec<_> = sqlx::query!(
        "SELECT a.id, a.title, a.artist_id, ar.name as artist_name, a.year,
                a.cover_art_path, a.artwork_source, a.musicbrainz_id, a.folder_path,
                a.created_at, a.updated_at
         FROM albums a
         LEFT JOIN artists ar ON a.artist_id = ar.id
         ORDER BY a.created_at DESC
         LIMIT ?",
        limit
    )
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|row| {
            Ok(Album {
                id: row
                    .id
                    .ok_or_else(|| StorageError::MissingField("album.id".to_string()))?,
                title: row.title,
                artist_id: row.artist_id,
                artist_name: row.artist_name,
                year: row.year.map(|y| y as i32),
                cover_art_path: row.cover_art_path,
                artwork_source: row.artwork_source,
                musicbrainz_id: row.musicbrainz_id,
                folder_path: row.folder_path,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
        })
        .collect()
}

pub async fn get_recently_added_within_days(
    pool: &SqlitePool,
    days: i64,
    limit: i64,
) -> Result<Vec<Album>> {
    let rows: Vec<_> = sqlx::query!(
        "SELECT a.id, a.title, a.artist_id, ar.name as artist_name, a.year,
                a.cover_art_path, a.artwork_source, a.musicbrainz_id, a.folder_path,
                a.created_at, a.updated_at
         FROM albums a
         LEFT JOIN artists ar ON a.artist_id = ar.id
         WHERE datetime(a.created_at) >= datetime('now', '-' || ? || ' days')
         ORDER BY a.created_at DESC
         LIMIT ?",
        days,
        limit
    )
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|row| {
            Ok(Album {
                id: row
                    .id
                    .ok_or_else(|| StorageError::MissingField("album.id".to_string()))?,
                title: row.title,
                artist_id: row.artist_id,
                artist_name: row.artist_name,
                year: row.year.map(|y| y as i32),
                cover_art_path: row.cover_art_path,
                artwork_source: row.artwork_source,
                musicbrainz_id: row.musicbrainz_id,
                folder_path: row.folder_path,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
        })
        .collect()
}

pub async fn get_least_played(pool: &SqlitePool, limit: i64, user_id: i64) -> Result<Vec<Album>> {
    // Get albums with lowest play count for this user
    let rows: Vec<_> = sqlx::query!(
        "SELECT a.id, a.title, a.artist_id, ar.name as artist_name, a.year,
                a.cover_art_path, a.artwork_source, a.musicbrainz_id, a.folder_path,
                a.created_at, a.updated_at,
                COALESCE(play_counts.count, 0) as play_count
         FROM albums a
         LEFT JOIN artists ar ON a.artist_id = ar.id
         LEFT JOIN (
             SELECT t.album_id, COUNT(*) as count
             FROM tracks t
             JOIN playback_contexts pc ON pc.context_type = 'album' AND CAST(pc.context_id AS INTEGER) = t.album_id
             WHERE pc.user_id = ?
             GROUP BY t.album_id
         ) play_counts ON play_counts.album_id = a.id
         ORDER BY play_count ASC, RANDOM()
         LIMIT ?",
        user_id,
        limit
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| Album {
            id: row.id,
            title: row.title,
            artist_id: row.artist_id,
            artist_name: row.artist_name,
            year: row.year.map(|y| y as i32),
            cover_art_path: row.cover_art_path,
            artwork_source: row.artwork_source,
            musicbrainz_id: row.musicbrainz_id,
            folder_path: row.folder_path,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
        .collect())
}

pub async fn get_time_capsule(pool: &SqlitePool, limit: i64, user_id: i64) -> Result<Vec<Album>> {
    // Get albums played on this day in previous years
    let rows: Vec<_> = sqlx::query!(
        "SELECT DISTINCT a.id, a.title, a.artist_id, ar.name as artist_name, a.year,
                a.cover_art_path, a.artwork_source, a.musicbrainz_id, a.folder_path,
                a.created_at, a.updated_at
         FROM albums a
         LEFT JOIN artists ar ON a.artist_id = ar.id
         JOIN playback_contexts pc ON pc.context_type = 'album' AND CAST(pc.context_id AS INTEGER) = a.id
         WHERE pc.user_id = ?
           AND strftime('%m-%d', datetime(pc.last_played_at, 'unixepoch')) = strftime('%m-%d', 'now')
           AND strftime('%Y', datetime(pc.last_played_at, 'unixepoch')) < strftime('%Y', 'now')
         ORDER BY pc.last_played_at DESC
         LIMIT ?",
        user_id,
        limit
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .filter_map(|row| {
            let id = row.id?; // Filter out None ids
            Some(Album {
                id,
                title: row.title,
                artist_id: row.artist_id,
                artist_name: Some(row.artist_name),
                year: row.year.map(|y| y as i32),
                cover_art_path: row.cover_art_path,
                artwork_source: row.artwork_source,
                musicbrainz_id: row.musicbrainz_id,
                folder_path: row.folder_path,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
        })
        .collect())
}

pub async fn get_by_genre(pool: &SqlitePool, genre_id: i64, limit: i64) -> Result<Vec<Album>> {
    // Get albums that have tracks with this genre
    let rows: Vec<_> = sqlx::query!(
        "SELECT DISTINCT a.id, a.title, a.artist_id, ar.name as artist_name, a.year,
                a.cover_art_path, a.artwork_source, a.musicbrainz_id, a.folder_path,
                a.created_at, a.updated_at
         FROM albums a
         LEFT JOIN artists ar ON a.artist_id = ar.id
         JOIN tracks t ON t.album_id = a.id
         JOIN track_genres tg ON tg.track_id = t.id
         WHERE tg.genre_id = ?
         ORDER BY RANDOM()
         LIMIT ?",
        genre_id,
        limit
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .filter_map(|row| {
            let id = row.id?; // Filter out None ids
            Some(Album {
                id,
                title: row.title,
                artist_id: row.artist_id,
                artist_name: Some(row.artist_name),
                year: row.year.map(|y| y as i32),
                cover_art_path: row.cover_art_path,
                artwork_source: row.artwork_source,
                musicbrainz_id: row.musicbrainz_id,
                folder_path: row.folder_path,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
        })
        .collect())
}

pub async fn get_by_id(pool: &SqlitePool, id: AlbumId) -> Result<Option<Album>> {
    let row: Option<_> = sqlx::query!(
        "SELECT a.id, a.title, a.artist_id, ar.name as artist_name, a.year,
                a.cover_art_path, a.artwork_source, a.musicbrainz_id, a.folder_path,
                a.created_at, a.updated_at
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
        artwork_source: row.artwork_source,
        musicbrainz_id: row.musicbrainz_id,
        folder_path: row.folder_path,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }))
}

pub async fn get_by_artist(pool: &SqlitePool, artist_id: ArtistId) -> Result<Vec<Album>> {
    let rows: Vec<_> = sqlx::query!(
        "SELECT a.id, a.title, a.artist_id, ar.name as artist_name, a.year,
                a.cover_art_path, a.artwork_source, a.musicbrainz_id, a.folder_path,
                a.created_at, a.updated_at
         FROM albums a
         LEFT JOIN artists ar ON a.artist_id = ar.id
         WHERE a.artist_id = ?
         ORDER BY a.year DESC, a.title COLLATE NOCASE",
        artist_id
    )
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|row| {
            Ok(Album {
                id: row
                    .id
                    .ok_or_else(|| StorageError::MissingField("album.id".to_string()))?,
                title: row.title,
                artist_id: row.artist_id,
                artist_name: Some(row.artist_name),
                year: row.year.map(|y| y as i32),
                cover_art_path: row.cover_art_path,
                artwork_source: row.artwork_source,
                musicbrainz_id: row.musicbrainz_id,
                folder_path: row.folder_path,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
        })
        .collect()
}

pub async fn create(pool: &SqlitePool, album: CreateAlbum) -> Result<Album> {
    let result: sqlx::sqlite::SqliteQueryResult = sqlx::query!(
        "INSERT INTO albums (title, artist_id, year, musicbrainz_id, folder_path)
         VALUES (?, ?, ?, ?, ?)",
        album.title,
        album.artist_id,
        album.year,
        album.musicbrainz_id,
        album.folder_path
    )
    .execute(pool)
    .await?;

    let id = result.last_insert_rowid();

    get_by_id(pool, id).await?.ok_or_else(|| {
        soul_core::SoulError::Storage("Failed to retrieve created album".to_string())
    })
}

/// Update album cover art path
pub async fn update_cover_art_path(
    pool: &SqlitePool,
    album_id: AlbumId,
    cover_art_path: Option<&str>,
) -> Result<()> {
    sqlx::query!(
        "UPDATE albums SET cover_art_path = ?, updated_at = datetime('now') WHERE id = ?",
        cover_art_path,
        album_id
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Set artwork source for an album
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `album_id` - Album ID
/// * `source` - Artwork source type: 'soul_storage', 'folder', or 'embedded'
/// * `path` - Path to the artwork file
pub async fn set_artwork_source(
    pool: &SqlitePool,
    album_id: AlbumId,
    source: &str,
    path: &str,
) -> Result<()> {
    sqlx::query!(
        "UPDATE albums SET artwork_source = ?, cover_art_path = ?, updated_at = datetime('now') WHERE id = ?",
        source,
        path,
        album_id
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Get track counts for all albums in a single query.
/// Returns a HashMap of album_id → track_count.
pub async fn get_track_counts(
    pool: &SqlitePool,
) -> Result<std::collections::HashMap<AlbumId, i32>> {
    let rows: Vec<_> = sqlx::query!(
        "SELECT album_id, COUNT(*) as count
         FROM tracks
         WHERE album_id IS NOT NULL
         GROUP BY album_id"
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .filter_map(|row| row.album_id.map(|album_id| (album_id, row.count as i32)))
        .collect())
}

/// Get artwork source information for an album
pub async fn get_artwork_source(
    pool: &SqlitePool,
    album_id: AlbumId,
) -> Result<Option<(String, Option<String>)>> {
    let row: Option<_> = sqlx::query!(
        "SELECT artwork_source, cover_art_path FROM albums WHERE id = ?",
        album_id
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.and_then(|r| r.artwork_source.map(|source| (source, r.cover_art_path))))
}

/// Update the folder_path of an album.
/// Used when a parent directory is discovered after a subfolder scan,
/// to promote the canonical path to the outermost location.
pub async fn update_folder_path(pool: &SqlitePool, id: AlbumId, folder_path: &str) -> Result<()> {
    sqlx::query!(
        "UPDATE albums SET folder_path = ? WHERE id = ?",
        folder_path,
        id
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Delete albums that have no available tracks.
/// Returns the number of albums deleted.
pub async fn delete_orphaned(pool: &SqlitePool) -> Result<i64> {
    let result = sqlx::query(
        "DELETE FROM albums
         WHERE id NOT IN (
             SELECT DISTINCT album_id FROM tracks WHERE album_id IS NOT NULL AND is_available = 1
         )",
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected() as i64)
}
