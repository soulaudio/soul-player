use soul_core::{
    error::Result,
    types::{ArtistId, TrackArtist, TrackId},
};
use sqlx::SqlitePool;
use std::collections::HashMap;

/// Link an artist to a track at a given position.
/// Silently ignores duplicate (track_id, artist_id) pairs via INSERT OR IGNORE.
pub async fn add_to_track(
    pool: &SqlitePool,
    track_id: &TrackId,
    artist_id: ArtistId,
    position: i64,
) -> Result<()> {
    let track_id_str = track_id.as_str();
    sqlx::query!(
        "INSERT OR IGNORE INTO track_artists (track_id, artist_id, position)
         VALUES (?, ?, ?)",
        track_id_str,
        artist_id,
        position
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Remove all artist associations for a track (call before re-importing).
pub async fn clear_for_track(pool: &SqlitePool, track_id: &TrackId) -> Result<()> {
    let track_id_str = track_id.as_str();
    sqlx::query!(
        "DELETE FROM track_artists WHERE track_id = ?",
        track_id_str
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Fetch all artists for multiple tracks in one query.
/// Returns a map from track_id → ordered Vec<TrackArtist> (ordered by position).
///
/// Uses raw query_as (not compile-time macro) because the IN list is dynamic.
/// The list is built from typed TrackId values — no user strings are interpolated.
pub async fn get_for_tracks(
    pool: &SqlitePool,
    track_ids: &[TrackId],
) -> Result<HashMap<TrackId, Vec<TrackArtist>>> {
    if track_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let placeholders = track_ids
        .iter()
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "SELECT ta.track_id, ta.artist_id, ar.name
         FROM track_artists ta
         JOIN artists ar ON ar.id = ta.artist_id
         WHERE ta.track_id IN ({})
         ORDER BY ta.track_id, ta.position",
        placeholders
    );

    let mut query = sqlx::query_as::<_, (String, i64, String)>(&sql);
    for id in track_ids {
        query = query.bind(id.as_str());
    }

    let rows = query.fetch_all(pool).await?;

    let mut map: HashMap<TrackId, Vec<TrackArtist>> = HashMap::new();
    for (track_id_str, artist_id, name) in rows {
        let track_id = TrackId::new(track_id_str);
        map.entry(track_id).or_default().push(TrackArtist {
            id: artist_id,
            name,
        });
    }
    Ok(map)
}

/// Populate the `artists` field on a collection of tracks from the junction table.
pub async fn populate_for_tracks(
    pool: &SqlitePool,
    tracks: &mut Vec<soul_core::types::Track>,
) -> Result<()> {
    let ids: Vec<TrackId> = tracks.iter().map(|t| t.id.clone()).collect();
    let mut map = get_for_tracks(pool, &ids).await?;
    for track in tracks.iter_mut() {
        if let Some(artists) = map.remove(&track.id) {
            track.artists = artists;
        }
    }
    Ok(())
}
