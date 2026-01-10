use crate::{error::Result, state::StateManager, SyncPhase, SyncProgress};
use sqlx::SqlitePool;
use tokio::sync::mpsc;
use tracing::{debug, info};

/// Clean up orphaned records from the database
pub async fn cleanup_orphans(
    pool: &SqlitePool,
    state: &StateManager,
    progress_tx: &mpsc::Sender<SyncProgress>,
    _session_id: &str,
) -> Result<usize> {
    debug!("Starting cleanup phase");
    state.update_phase(SyncPhase::Cleanup, 0).await?;

    let mut total_cleaned = 0;

    // Clean orphaned track_stats
    total_cleaned += cleanup_orphaned_track_stats(pool).await?;

    // Clean orphaned track_sources
    total_cleaned += cleanup_orphaned_track_sources(pool).await?;

    // Clean orphaned playlist_tracks
    total_cleaned += cleanup_orphaned_playlist_tracks(pool).await?;

    // Clean orphaned artists (artists with no tracks)
    total_cleaned += cleanup_orphaned_artists(pool).await?;

    // Clean orphaned albums (albums with no tracks)
    total_cleaned += cleanup_orphaned_albums(pool).await?;

    // Clean orphaned genres (genres with no tracks)
    total_cleaned += cleanup_orphaned_genres(pool).await?;

    info!(
        "Cleanup complete: removed {} orphaned records",
        total_cleaned
    );

    let progress = state.get_progress().await?;
    let _ = progress_tx.send(progress).await;

    Ok(total_cleaned)
}

/// Remove track_stats for non-existent tracks
async fn cleanup_orphaned_track_stats(pool: &SqlitePool) -> Result<usize> {
    let result = sqlx::query!(
        "DELETE FROM track_stats
         WHERE track_id NOT IN (SELECT id FROM tracks)"
    )
    .execute(pool)
    .await?;

    let count = result.rows_affected() as usize;
    if count > 0 {
        info!("Removed {} orphaned track_stats records", count);
    }

    Ok(count)
}

/// Remove track_sources for non-existent tracks or sources
async fn cleanup_orphaned_track_sources(pool: &SqlitePool) -> Result<usize> {
    let result = sqlx::query!(
        "DELETE FROM track_sources
         WHERE track_id NOT IN (SELECT id FROM tracks)
         OR source_id NOT IN (SELECT id FROM sources)"
    )
    .execute(pool)
    .await?;

    let count = result.rows_affected() as usize;
    if count > 0 {
        info!("Removed {} orphaned track_sources records", count);
    }

    Ok(count)
}

/// Remove playlist_tracks for non-existent tracks
async fn cleanup_orphaned_playlist_tracks(pool: &SqlitePool) -> Result<usize> {
    let result = sqlx::query!(
        "DELETE FROM playlist_tracks
         WHERE track_id NOT IN (SELECT id FROM tracks)"
    )
    .execute(pool)
    .await?;

    let count = result.rows_affected() as usize;
    if count > 0 {
        info!("Removed {} orphaned playlist_tracks records", count);
    }

    Ok(count)
}

/// Remove artists with no tracks
async fn cleanup_orphaned_artists(pool: &SqlitePool) -> Result<usize> {
    let result = sqlx::query!(
        "DELETE FROM artists
         WHERE id NOT IN (SELECT DISTINCT artist_id FROM tracks WHERE artist_id IS NOT NULL)"
    )
    .execute(pool)
    .await?;

    let count = result.rows_affected() as usize;
    if count > 0 {
        info!("Removed {} orphaned artists", count);
    }

    Ok(count)
}

/// Remove albums with no tracks
async fn cleanup_orphaned_albums(pool: &SqlitePool) -> Result<usize> {
    let result = sqlx::query!(
        "DELETE FROM albums
         WHERE id NOT IN (SELECT DISTINCT album_id FROM tracks WHERE album_id IS NOT NULL)"
    )
    .execute(pool)
    .await?;

    let count = result.rows_affected() as usize;
    if count > 0 {
        info!("Removed {} orphaned albums", count);
    }

    Ok(count)
}

/// Remove genres with no tracks
async fn cleanup_orphaned_genres(pool: &SqlitePool) -> Result<usize> {
    // Note: genres are linked via track_genres junction table
    let result = sqlx::query!(
        "DELETE FROM genres
         WHERE id NOT IN (SELECT DISTINCT genre_id FROM track_genres)"
    )
    .execute(pool)
    .await?;

    let count = result.rows_affected() as usize;
    if count > 0 {
        info!("Removed {} orphaned genres", count);
    }

    Ok(count)
}
