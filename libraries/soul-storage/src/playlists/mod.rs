use soul_core::{error::Result, types::*};
use sqlx::SqlitePool;

/// Get user's playlists (owned + shared with them)
pub async fn get_user_playlists(pool: &SqlitePool, user_id: UserId) -> Result<Vec<Playlist>> {
    let rows = sqlx::query!(
        r#"
        SELECT DISTINCT
            p.id, p.name, p.description, p.owner_id, p.is_public, p.is_favorite,
            p.created_at, p.updated_at
        FROM playlists p
        LEFT JOIN playlist_shares ps ON p.id = ps.playlist_id
        WHERE p.owner_id = ? OR ps.shared_with_user_id = ?
        ORDER BY p.is_favorite DESC, p.updated_at DESC
        "#,
        user_id,
        user_id
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| {
            // Convert Unix timestamp to ISO 8601 string
            let created_at = chrono::DateTime::from_timestamp(row.created_at, 0)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_default();
            let updated_at = chrono::DateTime::from_timestamp(row.updated_at, 0)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_default();

            Playlist {
                id: PlaylistId::new(row.id),
                name: row.name,
                description: row.description,
                owner_id: UserId::new(row.owner_id),
                is_public: row.is_public != 0,
                is_favorite: row.is_favorite != 0,
                created_at,
                updated_at,
                tracks: None,
            }
        })
        .collect())
}

/// Get playlist by ID (with permission check)
pub async fn get_by_id(
    pool: &SqlitePool,
    id: PlaylistId,
    user_id: UserId,
) -> Result<Option<Playlist>> {
    let row = sqlx::query!(
        r#"
        SELECT p.id, p.name, p.description, p.owner_id, p.is_public, p.is_favorite,
               p.created_at, p.updated_at
        FROM playlists p
        LEFT JOIN playlist_shares ps ON p.id = ps.playlist_id
        WHERE p.id = ? AND (p.owner_id = ? OR ps.shared_with_user_id = ? OR p.is_public = 1)
        LIMIT 1
        "#,
        id,
        user_id,
        user_id
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|row| {
        // Convert Unix timestamp to ISO 8601 string
        let created_at = chrono::DateTime::from_timestamp(row.created_at, 0)
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_default();
        let updated_at = chrono::DateTime::from_timestamp(row.updated_at, 0)
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_default();

        Playlist {
            id: PlaylistId::new(row.id),
            name: row.name,
            description: row.description,
            owner_id: UserId::new(row.owner_id),
            is_public: row.is_public != 0,
            is_favorite: row.is_favorite != 0,
            created_at,
            updated_at,
            tracks: None,
        }
    }))
}

/// Get playlist with all tracks
pub async fn get_with_tracks(
    pool: &SqlitePool,
    id: PlaylistId,
    user_id: UserId,
) -> Result<Option<Playlist>> {
    // First get the playlist
    let Some(mut playlist) = get_by_id(pool, id.clone(), user_id).await? else {
        return Ok(None);
    };

    // Then get tracks
    let track_rows = sqlx::query!(
        r#"
        SELECT
            pt.track_id, pt.position, pt.added_at,
            t.title, t.duration_seconds,
            ar.name as artist_name
        FROM playlist_tracks pt
        INNER JOIN tracks t ON pt.track_id = t.id
        LEFT JOIN artists ar ON t.artist_id = ar.id
        WHERE pt.playlist_id = ?
        ORDER BY pt.position
        "#,
        id
    )
    .fetch_all(pool)
    .await?;

    let tracks = track_rows
        .into_iter()
        .map(|row| PlaylistTrack {
            track_id: TrackId::new(row.track_id.to_string()),
            position: row.position as i32,
            added_at: row.added_at,
            title: Some(row.title),
            artist_name: Some(row.artist_name),
            duration_seconds: row.duration_seconds,
        })
        .collect();

    playlist.tracks = Some(tracks);

    Ok(Some(playlist))
}

/// Create new playlist
pub async fn create(pool: &SqlitePool, playlist: CreatePlaylist) -> Result<Playlist> {
    // Generate a new UUID for the playlist
    let id = PlaylistId::generate();

    sqlx::query!(
        r#"
        INSERT INTO playlists (id, name, description, owner_id, is_favorite, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, strftime('%s', 'now'), strftime('%s', 'now'))
        "#,
        id,
        playlist.name,
        playlist.description,
        playlist.owner_id,
        playlist.is_favorite
    )
    .execute(pool)
    .await?;

    get_by_id(pool, id, playlist.owner_id)
        .await?
        .ok_or_else(|| {
            soul_core::SoulError::Storage("Failed to retrieve created playlist".to_string())
        })
}

/// Add track to playlist
pub async fn add_track(
    pool: &SqlitePool,
    playlist_id: PlaylistId,
    track_id: TrackId,
    user_id: UserId,
) -> Result<()> {
    // Check permission (must be owner or have write permission)
    let has_permission = check_write_permission(pool, playlist_id.clone(), user_id).await?;
    if !has_permission {
        return Err(soul_core::SoulError::PermissionDenied);
    }

    // Get next position (0-based)
    let next_position_row = sqlx::query!(
        "SELECT COALESCE(MAX(position), -1) + 1 as next_pos FROM playlist_tracks WHERE playlist_id = ?",
        playlist_id
    )
    .fetch_one(pool)
    .await?;

    let next_position = next_position_row.next_pos;

    // Convert TrackId (String) to i64 for database
    let track_id_i64: i64 = track_id
        .as_str()
        .parse()
        .map_err(|_| soul_core::SoulError::InvalidInput("Invalid track ID".to_string()))?;

    // Insert track
    sqlx::query!(
        r#"
        INSERT INTO playlist_tracks (playlist_id, track_id, position)
        VALUES (?, ?, ?)
        ON CONFLICT(playlist_id, track_id) DO NOTHING
        "#,
        playlist_id,
        track_id_i64,
        next_position
    )
    .execute(pool)
    .await?;

    // Update playlist updated_at
    sqlx::query!(
        "UPDATE playlists SET updated_at = strftime('%s', 'now') WHERE id = ?",
        playlist_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Remove track from playlist
pub async fn remove_track(
    pool: &SqlitePool,
    playlist_id: PlaylistId,
    track_id: TrackId,
    user_id: UserId,
) -> Result<()> {
    // Check permission
    let has_permission = check_write_permission(pool, playlist_id.clone(), user_id).await?;
    if !has_permission {
        return Err(soul_core::SoulError::PermissionDenied);
    }

    let mut tx = pool.begin().await?;

    // Delete the track
    sqlx::query!(
        "DELETE FROM playlist_tracks WHERE playlist_id = ? AND track_id = ?",
        playlist_id,
        track_id
    )
    .execute(&mut *tx)
    .await?;

    // Reorder positions to fill gap
    sqlx::query!(
        r#"
        UPDATE playlist_tracks
        SET position = (
            SELECT COUNT(*)
            FROM playlist_tracks pt2
            WHERE pt2.playlist_id = playlist_tracks.playlist_id
              AND pt2.position < playlist_tracks.position
        )
        WHERE playlist_id = ?
        "#,
        playlist_id
    )
    .execute(&mut *tx)
    .await?;

    // Update playlist updated_at
    sqlx::query!(
        "UPDATE playlists SET updated_at = strftime('%s', 'now') WHERE id = ?",
        playlist_id
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(())
}

/// Delete playlist
pub async fn delete(pool: &SqlitePool, id: PlaylistId, user_id: UserId) -> Result<()> {
    // Check if user owns the playlist
    let playlist = get_by_id(pool, id.clone(), user_id.clone()).await?;

    match playlist {
        Some(p) if p.owner_id == user_id => {
            sqlx::query!("DELETE FROM playlists WHERE id = ?", id)
                .execute(pool)
                .await?;
            Ok(())
        }
        Some(_) => Err(soul_core::SoulError::PermissionDenied),
        None => Err(soul_core::SoulError::PlaylistNotFound(id)),
    }
}

/// Reorder tracks in playlist
pub async fn reorder_tracks(
    pool: &SqlitePool,
    playlist_id: PlaylistId,
    track_id: TrackId,
    new_position: i32,
    user_id: UserId,
) -> Result<()> {
    // Check permission
    let has_permission = check_write_permission(pool, playlist_id.clone(), user_id).await?;
    if !has_permission {
        return Err(soul_core::SoulError::PermissionDenied);
    }

    let mut tx = pool.begin().await?;

    // Get current position
    let current_pos = sqlx::query!(
        "SELECT position FROM playlist_tracks WHERE playlist_id = ? AND track_id = ?",
        playlist_id,
        track_id
    )
    .fetch_optional(&mut *tx)
    .await?;

    let Some(current) = current_pos else {
        return Err(soul_core::SoulError::Storage(
            "Track not in playlist".to_string(),
        ));
    };

    let old_position = current.position;
    let new_position_i64 = new_position as i64;

    if old_position == new_position_i64 {
        return Ok(());
    }

    // Shift other tracks
    if new_position_i64 < old_position {
        // Moving up: shift tracks down
        sqlx::query!(
            r#"
            UPDATE playlist_tracks
            SET position = position + 1
            WHERE playlist_id = ?
              AND position >= ?
              AND position < ?
            "#,
            playlist_id,
            new_position_i64,
            old_position
        )
        .execute(&mut *tx)
        .await?;
    } else {
        // Moving down: shift tracks up
        sqlx::query!(
            r#"
            UPDATE playlist_tracks
            SET position = position - 1
            WHERE playlist_id = ?
              AND position > ?
              AND position <= ?
            "#,
            playlist_id,
            old_position,
            new_position_i64
        )
        .execute(&mut *tx)
        .await?;
    }

    // Update track position
    sqlx::query!(
        "UPDATE playlist_tracks SET position = ? WHERE playlist_id = ? AND track_id = ?",
        new_position_i64,
        playlist_id,
        track_id
    )
    .execute(&mut *tx)
    .await?;

    // Update playlist updated_at
    sqlx::query!(
        "UPDATE playlists SET updated_at = strftime('%s', 'now') WHERE id = ?",
        playlist_id
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(())
}

/// Share playlist with another user
pub async fn share_playlist(
    pool: &SqlitePool,
    playlist_id: PlaylistId,
    shared_with_user_id: UserId,
    permission: &str,
    owner_id: UserId,
) -> Result<()> {
    // Verify owner
    let playlist = get_by_id(pool, playlist_id.clone(), owner_id.clone()).await?;
    match playlist {
        Some(p) if p.owner_id == owner_id => {}
        Some(_) => return Err(soul_core::SoulError::PermissionDenied),
        None => return Err(soul_core::SoulError::PlaylistNotFound(playlist_id.clone())),
    }

    // Insert share
    let now = chrono::Utc::now().timestamp();
    sqlx::query!(
        r#"
        INSERT INTO playlist_shares (playlist_id, shared_with_user_id, permission, shared_at)
        VALUES (?, ?, ?, ?)
        ON CONFLICT(playlist_id, shared_with_user_id) DO UPDATE SET
            permission = excluded.permission,
            shared_at = excluded.shared_at
        "#,
        playlist_id,
        shared_with_user_id,
        permission,
        now
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Unshare playlist
pub async fn unshare_playlist(
    pool: &SqlitePool,
    playlist_id: PlaylistId,
    shared_with_user_id: UserId,
    owner_id: UserId,
) -> Result<()> {
    // Verify owner
    let playlist = get_by_id(pool, playlist_id.clone(), owner_id.clone()).await?;
    match playlist {
        Some(p) if p.owner_id == owner_id => {}
        Some(_) => return Err(soul_core::SoulError::PermissionDenied),
        None => return Err(soul_core::SoulError::PlaylistNotFound(playlist_id.clone())),
    }

    sqlx::query!(
        "DELETE FROM playlist_shares WHERE playlist_id = ? AND shared_with_user_id = ?",
        playlist_id,
        shared_with_user_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

// Helper functions

async fn check_write_permission(
    pool: &SqlitePool,
    playlist_id: PlaylistId,
    user_id: UserId,
) -> Result<bool> {
    let row = sqlx::query!(
        r#"
        SELECT
            COALESCE(
                CASE
                    WHEN p.owner_id = ? THEN 1
                    WHEN ps.permission = 'write' THEN 1
                    ELSE 0
                END,
                0
            ) as "has_permission: i64"
        FROM playlists p
        LEFT JOIN playlist_shares ps ON p.id = ps.playlist_id AND ps.shared_with_user_id = ?
        WHERE p.id = ?
        LIMIT 1
        "#,
        user_id,
        user_id,
        playlist_id
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| r.has_permission == Some(1)).unwrap_or(false))
}
