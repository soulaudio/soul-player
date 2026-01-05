use sqlx::{Row, SqlitePool};
use soul_core::{error::Result, types::*};

/// Get user's playlists (owned + shared with them)
pub async fn get_user_playlists(pool: &SqlitePool, user_id: UserId) -> Result<Vec<Playlist>> {
    let rows = sqlx::query(
        r#"
        SELECT DISTINCT
            p.id, p.name, p.description, p.owner_id, p.is_public, p.is_favorite,
            p.created_at, p.updated_at
        FROM playlists p
        LEFT JOIN playlist_shares ps ON p.id = ps.playlist_id
        WHERE p.owner_id = ? OR ps.shared_with_user_id = ?
        ORDER BY p.is_favorite DESC, p.updated_at DESC
        "#,
    )
    .bind(user_id)
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|row| Playlist {
        id: row.get("id"),
        name: row.get("name"),
        description: row.get("description"),
        owner_id: row.get("owner_id"),
        is_public: row.get::<i64, _>("is_public") != 0,
        is_favorite: row.get::<i64, _>("is_favorite") != 0,
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        tracks: None,
    }).collect())
}

/// Get playlist by ID (with permission check)
pub async fn get_by_id(
    pool: &SqlitePool,
    id: PlaylistId,
    user_id: UserId,
) -> Result<Option<Playlist>> {
    let row = sqlx::query(
        r#"
        SELECT p.id, p.name, p.description, p.owner_id, p.is_public, p.is_favorite,
               p.created_at, p.updated_at
        FROM playlists p
        LEFT JOIN playlist_shares ps ON p.id = ps.playlist_id
        WHERE p.id = ? AND (p.owner_id = ? OR ps.shared_with_user_id = ? OR p.is_public = 1)
        LIMIT 1
        "#,
    )
    .bind(id)
    .bind(user_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|row| Playlist {
        id: row.get("id"),
        name: row.get("name"),
        description: row.get("description"),
        owner_id: row.get("owner_id"),
        is_public: row.get::<i64, _>("is_public") != 0,
        is_favorite: row.get::<i64, _>("is_favorite") != 0,
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        tracks: None,
    }))
}

/// Get playlist with all tracks
pub async fn get_with_tracks(
    pool: &SqlitePool,
    id: PlaylistId,
    user_id: UserId,
) -> Result<Option<Playlist>> {
    // First get the playlist
    let Some(mut playlist) = get_by_id(pool, id, user_id).await? else {
        return Ok(None);
    };

    // Then get tracks
    let track_rows = sqlx::query(
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
    )
    .bind(id)
    .fetch_all(pool)
    .await?;

    let tracks = track_rows.into_iter().map(|row| PlaylistTrack {
        track_id: row.get("track_id"),
        position: row.get("position"),
        added_at: row.get("added_at"),
        title: Some(row.get("title")),
        artist_name: row.get("artist_name"),
        duration_seconds: row.get("duration_seconds"),
    }).collect();

    playlist.tracks = Some(tracks);

    Ok(Some(playlist))
}

/// Create new playlist
pub async fn create(pool: &SqlitePool, playlist: CreatePlaylist) -> Result<Playlist> {
    let result = sqlx::query(
        r#"
        INSERT INTO playlists (name, description, owner_id, is_favorite)
        VALUES (?, ?, ?, ?)
        "#,
    )
    .bind(&playlist.name)
    .bind(&playlist.description)
    .bind(playlist.owner_id)
    .bind(playlist.is_favorite)
    .execute(pool)
    .await?;

    let id = result.last_insert_rowid();

    get_by_id(pool, id, playlist.owner_id).await?.ok_or_else(|| {
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
    let has_permission = check_write_permission(pool, playlist_id, user_id).await?;
    if !has_permission {
        return Err(soul_core::SoulError::PermissionDenied);
    }

    // Get next position
    let next_position_row = sqlx::query(
        "SELECT COALESCE(MAX(position), 0) + 1 as next_pos FROM playlist_tracks WHERE playlist_id = ?",
    )
    .bind(playlist_id)
    .fetch_one(pool)
    .await?;

    let next_position: i64 = next_position_row.get("next_pos");

    // Insert track
    sqlx::query(
        r#"
        INSERT INTO playlist_tracks (playlist_id, track_id, position)
        VALUES (?, ?, ?)
        ON CONFLICT(playlist_id, track_id) DO NOTHING
        "#,
    )
    .bind(playlist_id)
    .bind(track_id)
    .bind(next_position)
    .execute(pool)
    .await?;

    // Update playlist updated_at
    sqlx::query("UPDATE playlists SET updated_at = datetime('now') WHERE id = ?")
    .bind(playlist_id)
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
    let has_permission = check_write_permission(pool, playlist_id, user_id).await?;
    if !has_permission {
        return Err(soul_core::SoulError::PermissionDenied);
    }

    let mut tx = pool.begin().await?;

    // Delete the track
    sqlx::query("DELETE FROM playlist_tracks WHERE playlist_id = ? AND track_id = ?")
    .bind(playlist_id)
    .bind(track_id)
    .execute(&mut *tx)
    .await?;

    // Reorder positions to fill gap
    sqlx::query(
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
    )
    .bind(playlist_id)
    .execute(&mut *tx)
    .await?;

    // Update playlist updated_at
    sqlx::query("UPDATE playlists SET updated_at = datetime('now') WHERE id = ?")
    .bind(playlist_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(())
}

/// Delete playlist
pub async fn delete(
    pool: &SqlitePool,
    id: PlaylistId,
    user_id: UserId,
) -> Result<()> {
    // Check if user owns the playlist
    let playlist = get_by_id(pool, id, user_id).await?;

    match playlist {
        Some(p) if p.owner_id == user_id => {
            sqlx::query("DELETE FROM playlists WHERE id = ?")
                .bind(id)
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
    let has_permission = check_write_permission(pool, playlist_id, user_id).await?;
    if !has_permission {
        return Err(soul_core::SoulError::PermissionDenied);
    }

    let mut tx = pool.begin().await?;

    // Get current position
    let current_pos = sqlx::query(
        "SELECT position FROM playlist_tracks WHERE playlist_id = ? AND track_id = ?",
    )
    .bind(playlist_id)
    .bind(track_id)
    .fetch_optional(&mut *tx)
    .await?;

    let Some(current) = current_pos else {
        return Err(soul_core::SoulError::Storage("Track not in playlist".to_string()));
    };

    let old_position: i64 = current.get("position");
    let new_position_i64 = new_position as i64;

    if old_position == new_position_i64 {
        return Ok(());
    }

    // Shift other tracks
    if new_position_i64 < old_position {
        // Moving up: shift tracks down
        sqlx::query(
            r#"
            UPDATE playlist_tracks
            SET position = position + 1
            WHERE playlist_id = ?
              AND position >= ?
              AND position < ?
            "#,
        )
        .bind(playlist_id)
        .bind(new_position_i64)
        .bind(old_position)
        .execute(&mut *tx)
        .await?;
    } else {
        // Moving down: shift tracks up
        sqlx::query(
            r#"
            UPDATE playlist_tracks
            SET position = position - 1
            WHERE playlist_id = ?
              AND position > ?
              AND position <= ?
            "#,
        )
        .bind(playlist_id)
        .bind(old_position)
        .bind(new_position_i64)
        .execute(&mut *tx)
        .await?;
    }

    // Update track position
    sqlx::query("UPDATE playlist_tracks SET position = ? WHERE playlist_id = ? AND track_id = ?")
    .bind(new_position_i64)
    .bind(playlist_id)
    .bind(track_id)
    .execute(&mut *tx)
    .await?;

    // Update playlist updated_at
    sqlx::query("UPDATE playlists SET updated_at = datetime('now') WHERE id = ?")
    .bind(playlist_id)
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
    let playlist = get_by_id(pool, playlist_id, owner_id).await?;
    match playlist {
        Some(p) if p.owner_id == owner_id => {}
        Some(_) => return Err(soul_core::SoulError::PermissionDenied),
        None => return Err(soul_core::SoulError::PlaylistNotFound(playlist_id)),
    }

    // Insert share
    sqlx::query(
        r#"
        INSERT INTO playlist_shares (playlist_id, shared_with_user_id, permission)
        VALUES (?, ?, ?)
        ON CONFLICT(playlist_id, shared_with_user_id) DO UPDATE SET permission = excluded.permission
        "#,
    )
    .bind(playlist_id)
    .bind(shared_with_user_id)
    .bind(permission)
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
    let playlist = get_by_id(pool, playlist_id, owner_id).await?;
    match playlist {
        Some(p) if p.owner_id == owner_id => {}
        Some(_) => return Err(soul_core::SoulError::PermissionDenied),
        None => return Err(soul_core::SoulError::PlaylistNotFound(playlist_id)),
    }

    sqlx::query("DELETE FROM playlist_shares WHERE playlist_id = ? AND shared_with_user_id = ?")
    .bind(playlist_id)
    .bind(shared_with_user_id)
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
    let row = sqlx::query(
        r#"
        SELECT
            CASE
                WHEN p.owner_id = ? THEN 1
                WHEN ps.permission = 'write' THEN 1
                ELSE 0
            END as has_permission
        FROM playlists p
        LEFT JOIN playlist_shares ps ON p.id = ps.playlist_id AND ps.shared_with_user_id = ?
        WHERE p.id = ?
        LIMIT 1
        "#,
    )
    .bind(user_id)
    .bind(user_id)
    .bind(playlist_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| r.get::<i64, _>("has_permission") == 1).unwrap_or(false))
}
