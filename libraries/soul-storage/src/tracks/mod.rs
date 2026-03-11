use crate::utils::time::now_timestamp;
use soul_core::{error::Result, types::*};
use sqlx::{Row, SqlitePool};

/// Helper function to batch fetch track availability data
/// Uses compile-time safe query with subquery pattern
async fn batch_fetch_availability(
    pool: &SqlitePool,
    track_ids: &[i64],
) -> Result<
    Vec<(
        i64,
        i64,
        String,
        Option<String>,
        Option<String>,
        Option<i64>,
    )>,
> {
    if track_ids.is_empty() {
        return Ok(Vec::new());
    }

    // SQLite has a limit on the number of parameters (default 999).
    // To avoid exceeding this, we chunk the IDs and run multiple queries.
    const CHUNK_SIZE: usize = 500; // Safe limit well below 999

    let mut all_results = Vec::new();

    for chunk in track_ids.chunks(CHUNK_SIZE) {
        // Convert track IDs to comma-separated string for subquery
        let ids_str = chunk
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(",");

        // Use subquery pattern with compile-time safe query
        // SQLx doesn't support array binding in SQLite, but we can use a workaround
        // by constructing the query string safely (no user input, only i64 IDs)
        let query_str = format!(
            "SELECT track_id, source_id, status, local_file_path, server_path, local_file_size \
             FROM track_sources WHERE track_id IN ({})",
            ids_str
        );

        let rows = sqlx::query(&query_str).fetch_all(pool).await?;

        let chunk_results = rows
            .into_iter()
            .map(|row| {
                (
                    row.get::<i64, _>("track_id"),
                    row.get::<i64, _>("source_id"),
                    row.get::<String, _>("status"),
                    row.get::<Option<String>, _>("local_file_path"),
                    row.get::<Option<String>, _>("server_path"),
                    row.get::<Option<i64>, _>("local_file_size"),
                )
            })
            .collect::<Vec<_>>();

        all_results.extend(chunk_results);
    }

    Ok(all_results)
}

/// Get all tracks with denormalized artist/album names
/// Supports optional pagination via limit and after_id (cursor-based)
pub async fn get_all(
    pool: &SqlitePool,
    limit: Option<i64>,
    after_id: Option<String>,
) -> Result<Vec<Track>> {
    let start = std::time::Instant::now();

    // Default limit to 1000 if not specified
    let limit = limit.unwrap_or(1000);
    let after_id_int: Option<i64> = after_id.as_ref().and_then(|id| id.parse::<i64>().ok());

    // Optimized: Single query with LEFT JOIN to fetch tracks, artists, albums, and availability
    let query_start = std::time::Instant::now();
    let rows: Vec<_> = sqlx::query!(
        r#"
        SELECT
            t.id, t.title, t.artist_id, t.album_id, t.album_artist_id,
            t.track_number, t.disc_number, t.year, t.duration_seconds,
            t.bitrate, t.sample_rate, t.channels, t.file_format,
            t.origin_source_id, t.musicbrainz_recording_id, t.fingerprint,
            t.metadata_source, t.created_at, t.updated_at,
            ar.name as artist_name,
            al.title as album_title,
            al.cover_art_path as album_cover_art_path,
            al.artwork_source as album_artwork_source,
            ts.source_id as "source_id?",
            ts.status as "status?",
            ts.local_file_path as "local_file_path?",
            ts.server_path as "server_path?",
            ts.local_file_size as "local_file_size?"
        FROM tracks t
        LEFT JOIN artists ar ON t.artist_id = ar.id
        LEFT JOIN albums al ON t.album_id = al.id
        LEFT JOIN track_sources ts ON t.id = ts.track_id
        WHERE (? IS NULL OR t.id > ?)
        ORDER BY t.id, ts.source_id
        LIMIT ?
        "#,
        after_id_int,
        after_id_int,
        limit
    )
    .fetch_all(pool)
    .await?;

    let query_duration = query_start.elapsed();
    tracing::debug!(
        query_duration_ms = query_duration.as_millis(),
        row_count = rows.len(),
        "[DB] get_all tracks query completed (optimized single query)"
    );

    // Group rows by track_id and build Track objects
    let build_start = std::time::Instant::now();
    // Pre-allocate HashMap with estimated capacity (rows typically have 1-3 availability per track)
    let estimated_capacity = rows.len() / 2;
    let mut tracks_map: std::collections::HashMap<i64, Track> =
        std::collections::HashMap::with_capacity(estimated_capacity);

    for row in rows {
        let track_id_i64 = row.id;
        let track = tracks_map.entry(track_id_i64).or_insert_with(|| Track {
            id: TrackId::new(track_id_i64.to_string()),
            title: row.title.clone(),
            artist_id: row.artist_id,
            artist_name: row.artist_name.clone(),
            album_id: row.album_id,
            album_title: row.album_title.clone(),
            album_artist_id: row.album_artist_id,
            track_number: row.track_number.map(|x| x as i32),
            disc_number: row.disc_number.map(|x| x as i32),
            year: row.year.map(|x| x as i32),
            duration_seconds: row.duration_seconds,
            bitrate: row.bitrate.map(|x| x as i32),
            sample_rate: row.sample_rate.map(|x| x as i32),
            channels: row.channels.map(|x| x as i32),
            file_format: row
                .file_format
                .clone()
                .unwrap_or_else(|| "unknown".to_string()),
            origin_source_id: row.origin_source_id,
            musicbrainz_recording_id: row.musicbrainz_recording_id.clone(),
            fingerprint: row.fingerprint.clone(),
            metadata_source: parse_metadata_source(
                row.metadata_source.as_deref().unwrap_or("file"),
            ),
            created_at: row.created_at.clone(),
            updated_at: row.updated_at.clone(),
            cover_art_path: row.album_cover_art_path.clone(),
            artwork_source: row.album_artwork_source.clone(),
            artists: Vec::new(),
            availability: Vec::new(),
        });

        // Add availability data if present in this row
        if let (Some(source_id), Some(status)) = (row.source_id, &row.status) {
            track.availability.push(TrackAvailability {
                source_id,
                status: parse_availability_status(status),
                local_file_path: row.local_file_path.clone(),
                server_path: row.server_path.clone(),
                local_file_size: row.local_file_size,
            });
        }
    }

    // Convert to Vec and preserve order
    let mut tracks: Vec<Track> = tracks_map.into_values().collect();
    tracks.sort_by_key(|t| t.id.as_str().parse::<i64>().unwrap_or(0));

    let build_duration = build_start.elapsed();
    let total_duration = start.elapsed();

    tracing::info!(
        total_duration_ms = total_duration.as_millis(),
        build_duration_ms = build_duration.as_millis(),
        track_count = tracks.len(),
        "[DB] get_all completed (optimized)"
    );

    Ok(tracks)
}

/// Search tracks by query (searches title, artist name, album title)
pub async fn search(pool: &SqlitePool, query: &str) -> Result<Vec<Track>> {
    let start = std::time::Instant::now();
    let search_pattern = format!("%{}%", query);

    // First, fetch all matching track data with artist/album info
    let query_start = std::time::Instant::now();
    let rows: Vec<_> = sqlx::query!(
        r#"
        SELECT DISTINCT
            t.id, t.title, t.artist_id, t.album_id, t.album_artist_id,
            t.track_number, t.disc_number, t.year, t.duration_seconds,
            t.bitrate, t.sample_rate, t.channels, t.file_format,
            t.origin_source_id, t.musicbrainz_recording_id, t.fingerprint,
            t.metadata_source, t.created_at, t.updated_at,
            ar.name as artist_name,
            al.title as album_title,
            al.cover_art_path as album_cover_art_path,
            al.artwork_source as album_artwork_source
        FROM tracks t
        LEFT JOIN artists ar ON t.artist_id = ar.id
        LEFT JOIN albums al ON t.album_id = al.id
        WHERE t.title LIKE ?
           OR ar.name LIKE ?
           OR al.title LIKE ?
        ORDER BY t.title
        "#,
        search_pattern,
        search_pattern,
        search_pattern
    )
    .fetch_all(pool)
    .await?;

    let query_duration = query_start.elapsed();
    tracing::debug!(
        query_duration_ms = query_duration.as_millis(),
        row_count = rows.len(),
        search_query = %query,
        "[DB] search tracks query completed"
    );

    if rows.is_empty() {
        tracing::debug!(
            total_duration_ms = start.elapsed().as_millis(),
            "[DB] search completed with no results"
        );
        return Ok(Vec::new());
    }

    // Collect track IDs for batch availability lookup
    let track_ids: Vec<i64> = rows.iter().map(|r| r.id).collect();

    // Batch fetch availability data for matched tracks using compile-time safe queries
    let availability_rows = batch_fetch_availability(pool, &track_ids).await?;

    // Group availability by track_id (pre-allocate with capacity for better performance)
    let mut availability_map: std::collections::HashMap<String, Vec<TrackAvailability>> =
        std::collections::HashMap::with_capacity(track_ids.len());
    for (track_id, source_id, status, local_file_path, server_path, local_file_size) in
        availability_rows
    {
        availability_map
            .entry(track_id.to_string())
            .or_insert_with(Vec::new)
            .push(TrackAvailability {
                source_id,
                status: parse_availability_status(&status),
                local_file_path,
                server_path,
                local_file_size,
            });
    }

    // Build track objects with availability data
    let tracks: Vec<_> = rows
        .into_iter()
        .map(|row| {
            let track_id = TrackId::new(row.id.to_string());
            let availability = availability_map
                .get(track_id.as_str())
                .cloned()
                .unwrap_or_default();

            Track {
                id: track_id,
                title: row.title,
                artist_id: row.artist_id,
                artist_name: row.artist_name,
                album_id: row.album_id,
                album_title: row.album_title,
                album_artist_id: row.album_artist_id,
                track_number: row.track_number.map(|x| x as i32),
                disc_number: row.disc_number.map(|x| x as i32),
                year: row.year.map(|x| x as i32),
                duration_seconds: row.duration_seconds,
                bitrate: row.bitrate.map(|x| x as i32),
                sample_rate: row.sample_rate.map(|x| x as i32),
                channels: row.channels.map(|x| x as i32),
                file_format: row.file_format.unwrap_or_else(|| "unknown".to_string()),
                origin_source_id: row.origin_source_id,
                musicbrainz_recording_id: row.musicbrainz_recording_id,
                fingerprint: row.fingerprint,
                metadata_source: row
                    .metadata_source
                    .and_then(|s: String| match s.as_str() {
                        "file" => Some(MetadataSource::File),
                        "enriched" => Some(MetadataSource::Enriched),
                        "user_edited" => Some(MetadataSource::UserEdited),
                        _ => None,
                    })
                    .unwrap_or(MetadataSource::File),
                created_at: row.created_at,
                updated_at: row.updated_at,
                cover_art_path: row.album_cover_art_path,
                artwork_source: row.album_artwork_source,
                artists: Vec::new(),
                availability,
            }
        })
        .collect();

    Ok(tracks)
}

/// Get track by ID
pub async fn get_by_id(pool: &SqlitePool, id: TrackId) -> Result<Option<Track>> {
    let start = std::time::Instant::now();
    tracing::debug!(
        track_id = %id,
        "[Storage:get_by_id] Fetching track"
    );

    let id_int: i64 = id
        .as_str()
        .parse()
        .map_err(|_| soul_core::SoulError::Storage(format!("Invalid track ID: {}", id)))?;

    let query_start = std::time::Instant::now();
    let row: Option<_> = sqlx::query!(
        r#"
        SELECT
            t.id, t.title, t.artist_id, t.album_id, t.album_artist_id,
            t.track_number, t.disc_number, t.year, t.duration_seconds,
            t.bitrate, t.sample_rate, t.channels, t.file_format,
            t.origin_source_id, t.musicbrainz_recording_id, t.fingerprint,
            t.metadata_source, t.created_at, t.updated_at,
            ar.name as artist_name,
            al.title as album_title,
            al.cover_art_path as album_cover_art_path,
            al.artwork_source as album_artwork_source
        FROM tracks t
        LEFT JOIN artists ar ON t.artist_id = ar.id
        LEFT JOIN albums al ON t.album_id = al.id
        WHERE t.id = ?
        "#,
        id_int
    )
    .fetch_optional(pool)
    .await?;
    let query_duration = query_start.elapsed();

    if let Some(row) = row {
        let track_id = TrackId::new(row.id.to_string());
        let availability = get_availability(pool, track_id.clone()).await?;

        let total_duration = start.elapsed();

        if total_duration.as_millis() > 100 {
            tracing::warn!(
                track_id = %id,
                query_ms = query_duration.as_millis(),
                total_ms = total_duration.as_millis(),
                "[Storage:get_by_id] Slow query detected"
            );
        } else {
            tracing::debug!(
                track_id = %id,
                found = true,
                query_ms = query_duration.as_millis(),
                total_ms = total_duration.as_millis(),
                "[Storage:get_by_id] Completed"
            );
        }

        Ok(Some(Track {
            id: track_id,
            title: row.title,
            artist_id: row.artist_id,
            artist_name: row.artist_name,
            album_id: row.album_id,
            album_title: row.album_title,
            album_artist_id: row.album_artist_id,
            track_number: row.track_number.map(|x| x as i32),
            disc_number: row.disc_number.map(|x| x as i32),
            year: row.year.map(|x| x as i32),
            duration_seconds: row.duration_seconds,
            bitrate: row.bitrate.map(|x| x as i32),
            sample_rate: row.sample_rate.map(|x| x as i32),
            channels: row.channels.map(|x| x as i32),
            file_format: row.file_format.unwrap_or_else(|| "unknown".to_string()),
            origin_source_id: row.origin_source_id,
            musicbrainz_recording_id: row.musicbrainz_recording_id,
            fingerprint: row.fingerprint,
            metadata_source: parse_metadata_source(
                row.metadata_source.as_deref().unwrap_or("file"),
            ),
            created_at: row.created_at,
            updated_at: row.updated_at,
            cover_art_path: row.album_cover_art_path,
            artwork_source: row.album_artwork_source,
            artists: Vec::new(),
            availability,
        }))
    } else {
        let total_duration = start.elapsed();
        tracing::debug!(
            track_id = %id,
            found = false,
            total_ms = total_duration.as_millis(),
            "[Storage:get_by_id] Completed"
        );
        Ok(None)
    }
}

/// Get multiple tracks by IDs in a single query (batch operation)
///
/// This is optimized to avoid N+1 query problems when fetching multiple tracks.
/// Uses dynamic SQL with IN clause to fetch all tracks, artist/album data, and
/// availability in a single query.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `ids` - Vector of track IDs to fetch
///
/// # Returns
///
/// Vector of tracks in the same order as the input IDs (missing tracks are filtered out)
pub async fn get_by_ids(pool: &SqlitePool, ids: &[TrackId]) -> Result<Vec<Track>> {
    let start = std::time::Instant::now();

    if ids.is_empty() {
        return Ok(Vec::new());
    }

    tracing::debug!(
        track_count = ids.len(),
        "[Storage:get_by_ids] Fetching tracks in batch"
    );

    // Convert TrackIds to i64s
    let id_ints: Vec<i64> = ids
        .iter()
        .filter_map(|id| id.as_str().parse::<i64>().ok())
        .collect();

    if id_ints.is_empty() {
        return Ok(Vec::new());
    }

    // SQLite has a limit on the number of parameters (default 999).
    // To avoid exceeding this, we chunk the IDs if needed.
    const CHUNK_SIZE: usize = 500;

    let mut all_tracks = Vec::new();

    for chunk in id_ints.chunks(CHUNK_SIZE) {
        // Build comma-separated ID list for IN clause
        // This is safe because we've validated these are i64s (no SQL injection risk)
        let ids_str = chunk
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(",");

        let query_str = format!(
            r#"
            SELECT
                t.id, t.title, t.artist_id, t.album_id, t.album_artist_id,
                t.track_number, t.disc_number, t.year, t.duration_seconds,
                t.bitrate, t.sample_rate, t.channels, t.file_format,
                t.origin_source_id, t.musicbrainz_recording_id, t.fingerprint,
                t.metadata_source, t.created_at, t.updated_at,
                ar.name as artist_name,
                al.title as album_title,
                al.cover_art_path as album_cover_art_path,
                al.artwork_source as album_artwork_source,
                ts.source_id as source_id,
                ts.status as status,
                ts.local_file_path as local_file_path,
                ts.server_path as server_path,
                ts.local_file_size as local_file_size
            FROM tracks t
            LEFT JOIN artists ar ON t.artist_id = ar.id
            LEFT JOIN albums al ON t.album_id = al.id
            LEFT JOIN track_sources ts ON t.id = ts.track_id
            WHERE t.id IN ({})
            ORDER BY t.id, ts.source_id
            "#,
            ids_str
        );

        let query_start = std::time::Instant::now();
        let rows = sqlx::query(&query_str).fetch_all(pool).await?;
        let query_duration = query_start.elapsed();

        tracing::debug!(
            chunk_size = chunk.len(),
            row_count = rows.len(),
            query_ms = query_duration.as_millis(),
            "[Storage:get_by_ids] Query chunk completed"
        );

        // Group rows by track_id and build Track objects
        let mut tracks_map: std::collections::HashMap<i64, Track> =
            std::collections::HashMap::new();

        for row in rows {
            let track_id_i64: i64 = row.try_get("id")?;
            let track = tracks_map.entry(track_id_i64).or_insert_with(|| Track {
                id: TrackId::new(track_id_i64.to_string()),
                title: row.try_get("title").unwrap_or_default(),
                artist_id: row.try_get("artist_id").ok(),
                artist_name: row.try_get("artist_name").ok(),
                album_id: row.try_get("album_id").ok(),
                album_title: row.try_get("album_title").ok(),
                album_artist_id: row.try_get("album_artist_id").ok(),
                track_number: row
                    .try_get::<Option<i64>, _>("track_number")
                    .ok()
                    .flatten()
                    .map(|x| x as i32),
                disc_number: row
                    .try_get::<Option<i64>, _>("disc_number")
                    .ok()
                    .flatten()
                    .map(|x| x as i32),
                year: row
                    .try_get::<Option<i64>, _>("year")
                    .ok()
                    .flatten()
                    .map(|x| x as i32),
                duration_seconds: row.try_get("duration_seconds").ok(),
                bitrate: row
                    .try_get::<Option<i64>, _>("bitrate")
                    .ok()
                    .flatten()
                    .map(|x| x as i32),
                sample_rate: row
                    .try_get::<Option<i64>, _>("sample_rate")
                    .ok()
                    .flatten()
                    .map(|x| x as i32),
                channels: row
                    .try_get::<Option<i64>, _>("channels")
                    .ok()
                    .flatten()
                    .map(|x| x as i32),
                file_format: row
                    .try_get::<Option<String>, _>("file_format")
                    .ok()
                    .flatten()
                    .unwrap_or_else(|| "unknown".to_string()),
                origin_source_id: row.try_get("origin_source_id").unwrap_or(1),
                musicbrainz_recording_id: row.try_get("musicbrainz_recording_id").ok(),
                fingerprint: row.try_get("fingerprint").ok(),
                metadata_source: parse_metadata_source(
                    row.try_get::<Option<String>, _>("metadata_source")
                        .ok()
                        .flatten()
                        .as_deref()
                        .unwrap_or("file"),
                ),
                created_at: row.try_get("created_at").unwrap_or_default(),
                updated_at: row.try_get("updated_at").unwrap_or_default(),
                cover_art_path: row.try_get("album_cover_art_path").ok(),
                artwork_source: row.try_get("album_artwork_source").ok(),
                artists: Vec::new(),
                availability: Vec::new(),
            });

            // Add availability data if present in this row
            if let (Ok(Some(source_id)), Ok(Some(status))) = (
                row.try_get::<Option<i64>, _>("source_id"),
                row.try_get::<Option<String>, _>("status"),
            ) {
                track.availability.push(TrackAvailability {
                    source_id,
                    status: parse_availability_status(&status),
                    local_file_path: row.try_get("local_file_path").ok(),
                    server_path: row.try_get("server_path").ok(),
                    local_file_size: row.try_get("local_file_size").ok(),
                });
            }
        }

        // Convert to Vec and preserve the original order
        all_tracks.extend(tracks_map.into_values());
    }

    // Sort by the original input order (based on position in ids vec)
    let id_to_pos: std::collections::HashMap<String, usize> = ids
        .iter()
        .enumerate()
        .map(|(i, id)| (id.as_str().to_string(), i))
        .collect();

    all_tracks.sort_by_key(|t| id_to_pos.get(t.id.as_str()).copied().unwrap_or(usize::MAX));

    let total_duration = start.elapsed();

    if total_duration.as_millis() > 100 {
        tracing::warn!(
            requested_count = ids.len(),
            found_count = all_tracks.len(),
            total_ms = total_duration.as_millis(),
            "[Storage:get_by_ids] Slow batch query detected"
        );
    } else {
        tracing::debug!(
            requested_count = ids.len(),
            found_count = all_tracks.len(),
            total_ms = total_duration.as_millis(),
            "[Storage:get_by_ids] Batch query completed"
        );
    }

    Ok(all_tracks)
}

/// Get tracks by source
pub async fn get_by_source(pool: &SqlitePool, source_id: SourceId) -> Result<Vec<Track>> {
    let rows: Vec<_> = sqlx::query!(
        r#"
        SELECT DISTINCT
            t.id, t.title, t.artist_id, t.album_id, t.album_artist_id,
            t.track_number, t.disc_number, t.year, t.duration_seconds,
            t.bitrate, t.sample_rate, t.channels, t.file_format,
            t.origin_source_id, t.musicbrainz_recording_id, t.fingerprint,
            t.metadata_source, t.created_at, t.updated_at,
            ar.name as artist_name,
            al.title as album_title,
            al.cover_art_path as album_cover_art_path,
            al.artwork_source as album_artwork_source
        FROM tracks t
        LEFT JOIN artists ar ON t.artist_id = ar.id
        LEFT JOIN albums al ON t.album_id = al.id
        INNER JOIN track_sources ts ON t.id = ts.track_id
        WHERE ts.source_id = ?
        ORDER BY t.title
        "#,
        source_id
    )
    .fetch_all(pool)
    .await?;

    if rows.is_empty() {
        return Ok(Vec::new());
    }

    // Collect track IDs for batch availability lookup
    let track_ids: Vec<i64> = rows.iter().map(|r| r.id).collect();

    // Batch fetch availability data for these tracks using compile-time safe queries
    let availability_rows = batch_fetch_availability(pool, &track_ids).await?;

    // Group availability by track_id (pre-allocate with capacity for better performance)
    let mut availability_map: std::collections::HashMap<String, Vec<TrackAvailability>> =
        std::collections::HashMap::with_capacity(track_ids.len());
    for (track_id, source_id, status, local_file_path, server_path, local_file_size) in
        availability_rows
    {
        availability_map
            .entry(track_id.to_string())
            .or_insert_with(Vec::new)
            .push(TrackAvailability {
                source_id,
                status: parse_availability_status(&status),
                local_file_path,
                server_path,
                local_file_size,
            });
    }

    // Build track objects with availability data
    let tracks: Vec<_> = rows
        .into_iter()
        .map(|row| {
            let track_id = TrackId::new(row.id.to_string());
            let availability = availability_map
                .get(track_id.as_str())
                .cloned()
                .unwrap_or_default();

            Track {
                id: track_id,
                title: row.title,
                artist_id: row.artist_id,
                artist_name: Some(row.artist_name),
                album_id: row.album_id,
                album_title: Some(row.album_title),
                album_artist_id: row.album_artist_id,
                track_number: row.track_number.map(|x| x as i32),
                disc_number: row.disc_number.map(|x| x as i32),
                year: row.year.map(|x| x as i32),
                duration_seconds: row.duration_seconds,
                bitrate: row.bitrate.map(|x| x as i32),
                sample_rate: row.sample_rate.map(|x| x as i32),
                channels: row.channels.map(|x| x as i32),
                file_format: row.file_format.unwrap_or_else(|| "unknown".to_string()),
                origin_source_id: row.origin_source_id,
                musicbrainz_recording_id: row.musicbrainz_recording_id,
                fingerprint: row.fingerprint,
                metadata_source: parse_metadata_source(
                    row.metadata_source.as_deref().unwrap_or("file"),
                ),
                created_at: row.created_at,
                updated_at: row.updated_at,
                cover_art_path: row.album_cover_art_path,
                artwork_source: row.album_artwork_source,
                artists: Vec::new(),
                availability,
            }
        })
        .collect();

    Ok(tracks)
}

/// Get tracks by artist
pub async fn get_by_artist(pool: &SqlitePool, artist_id: ArtistId) -> Result<Vec<Track>> {
    let start = std::time::Instant::now();
    tracing::debug!(
        artist_id = artist_id,
        "[Storage:get_by_artist] Fetching tracks"
    );

    // First, fetch all track data for the artist
    let query_start = std::time::Instant::now();
    let rows: Vec<_> = sqlx::query!(
        r#"
        SELECT
            t.id, t.title, t.artist_id, t.album_id, t.album_artist_id,
            t.track_number, t.disc_number, t.year, t.duration_seconds,
            t.bitrate, t.sample_rate, t.channels, t.file_format,
            t.origin_source_id, t.musicbrainz_recording_id, t.fingerprint,
            t.metadata_source, t.created_at, t.updated_at,
            ar.name as artist_name,
            al.title as album_title
        FROM tracks t
        LEFT JOIN artists ar ON t.artist_id = ar.id
        LEFT JOIN albums al ON t.album_id = al.id
        WHERE t.artist_id = ?
        ORDER BY t.album_id, t.disc_number, t.track_number, t.title
        "#,
        artist_id
    )
    .fetch_all(pool)
    .await?;
    let query_duration = query_start.elapsed();

    if rows.is_empty() {
        tracing::debug!(
            artist_id = artist_id,
            count = 0,
            duration_ms = start.elapsed().as_millis(),
            "[Storage:get_by_artist] Completed - no tracks found"
        );
        return Ok(Vec::new());
    }

    // Collect track IDs for batch availability lookup
    let track_ids: Vec<i64> = rows.iter().map(|r| r.id).collect();

    // Batch fetch availability data for these tracks using compile-time safe queries
    let availability_rows = batch_fetch_availability(pool, &track_ids).await?;

    // Group availability by track_id (pre-allocate with capacity for better performance)
    let mut availability_map: std::collections::HashMap<String, Vec<TrackAvailability>> =
        std::collections::HashMap::with_capacity(track_ids.len());
    for (track_id, source_id, status, local_file_path, server_path, local_file_size) in
        availability_rows
    {
        availability_map
            .entry(track_id.to_string())
            .or_insert_with(Vec::new)
            .push(TrackAvailability {
                source_id,
                status: parse_availability_status(&status),
                local_file_path,
                server_path,
                local_file_size,
            });
    }

    // Build track objects with availability data
    let tracks: Vec<_> = rows
        .into_iter()
        .map(|row| {
            let track_id = TrackId::new(row.id.to_string());
            let availability = availability_map
                .get(track_id.as_str())
                .cloned()
                .unwrap_or_default();

            Track {
                id: track_id,
                title: row.title,
                artist_id: row.artist_id,
                artist_name: Some(row.artist_name),
                album_id: row.album_id,
                album_title: Some(row.album_title),
                album_artist_id: row.album_artist_id,
                track_number: row.track_number.map(|x| x as i32),
                disc_number: row.disc_number.map(|x| x as i32),
                year: row.year.map(|x| x as i32),
                duration_seconds: row.duration_seconds,
                bitrate: row.bitrate.map(|x| x as i32),
                sample_rate: row.sample_rate.map(|x| x as i32),
                channels: row.channels.map(|x| x as i32),
                file_format: row.file_format.unwrap_or_else(|| "unknown".to_string()),
                origin_source_id: row.origin_source_id,
                musicbrainz_recording_id: row.musicbrainz_recording_id,
                fingerprint: row.fingerprint,
                metadata_source: parse_metadata_source(
                    row.metadata_source.as_deref().unwrap_or("file"),
                ),
                created_at: row.created_at,
                updated_at: row.updated_at,
                cover_art_path: None,
                artwork_source: None,
                artists: Vec::new(),
                availability,
            }
        })
        .collect();

    let total_duration = start.elapsed();
    if total_duration.as_millis() > 100 {
        tracing::warn!(
            artist_id = artist_id,
            count = tracks.len(),
            query_ms = query_duration.as_millis(),
            total_ms = total_duration.as_millis(),
            "[Storage:get_by_artist] Slow query detected"
        );
    } else {
        tracing::debug!(
            artist_id = artist_id,
            count = tracks.len(),
            query_ms = query_duration.as_millis(),
            total_ms = total_duration.as_millis(),
            "[Storage:get_by_artist] Completed"
        );
    }

    Ok(tracks)
}

/// Get tracks by album
pub async fn get_by_album(pool: &SqlitePool, album_id: AlbumId) -> Result<Vec<Track>> {
    let start = std::time::Instant::now();
    tracing::debug!(
        album_id = album_id,
        "[Storage:get_by_album] Fetching tracks"
    );

    // First, fetch all track data for the album
    let query_start = std::time::Instant::now();
    let rows: Vec<_> = sqlx::query!(
        r#"
        SELECT
            t.id, t.title, t.artist_id, t.album_id, t.album_artist_id,
            t.track_number, t.disc_number, t.year, t.duration_seconds,
            t.bitrate, t.sample_rate, t.channels, t.file_format,
            t.origin_source_id, t.musicbrainz_recording_id, t.fingerprint,
            t.metadata_source, t.created_at, t.updated_at,
            ar.name as artist_name,
            al.title as album_title
        FROM tracks t
        LEFT JOIN artists ar ON t.artist_id = ar.id
        LEFT JOIN albums al ON t.album_id = al.id
        WHERE t.album_id = ?
        ORDER BY t.disc_number, t.track_number, t.title
        "#,
        album_id
    )
    .fetch_all(pool)
    .await?;
    let query_duration = query_start.elapsed();

    if rows.is_empty() {
        tracing::debug!(
            album_id = album_id,
            count = 0,
            duration_ms = start.elapsed().as_millis(),
            "[Storage:get_by_album] Completed - no tracks found"
        );
        return Ok(Vec::new());
    }

    // Collect track IDs for batch availability lookup
    let track_ids: Vec<i64> = rows.iter().map(|r| r.id).collect();

    // Batch fetch availability data for these tracks using compile-time safe queries
    let availability_rows = batch_fetch_availability(pool, &track_ids).await?;

    // Group availability by track_id (pre-allocate with capacity for better performance)
    let mut availability_map: std::collections::HashMap<String, Vec<TrackAvailability>> =
        std::collections::HashMap::with_capacity(track_ids.len());
    for (track_id, source_id, status, local_file_path, server_path, local_file_size) in
        availability_rows
    {
        availability_map
            .entry(track_id.to_string())
            .or_insert_with(Vec::new)
            .push(TrackAvailability {
                source_id,
                status: parse_availability_status(&status),
                local_file_path,
                server_path,
                local_file_size,
            });
    }

    // Build track objects with availability data
    let tracks: Vec<_> = rows
        .into_iter()
        .map(|row| {
            let track_id = TrackId::new(row.id.to_string());
            let availability = availability_map
                .get(track_id.as_str())
                .cloned()
                .unwrap_or_default();

            Track {
                id: track_id,
                title: row.title,
                artist_id: row.artist_id,
                artist_name: Some(row.artist_name),
                album_id: row.album_id,
                album_title: Some(row.album_title),
                album_artist_id: row.album_artist_id,
                track_number: row.track_number.map(|x| x as i32),
                disc_number: row.disc_number.map(|x| x as i32),
                year: row.year.map(|x| x as i32),
                duration_seconds: row.duration_seconds,
                bitrate: row.bitrate.map(|x| x as i32),
                sample_rate: row.sample_rate.map(|x| x as i32),
                channels: row.channels.map(|x| x as i32),
                file_format: row.file_format.unwrap_or_else(|| "unknown".to_string()),
                origin_source_id: row.origin_source_id,
                musicbrainz_recording_id: row.musicbrainz_recording_id,
                fingerprint: row.fingerprint,
                metadata_source: parse_metadata_source(
                    row.metadata_source.as_deref().unwrap_or("file"),
                ),
                created_at: row.created_at,
                updated_at: row.updated_at,
                cover_art_path: None,
                artwork_source: None,
                artists: Vec::new(),
                availability,
            }
        })
        .collect();

    let total_duration = start.elapsed();
    if total_duration.as_millis() > 100 {
        tracing::warn!(
            album_id = album_id,
            count = tracks.len(),
            query_ms = query_duration.as_millis(),
            total_ms = total_duration.as_millis(),
            "[Storage:get_by_album] Slow query detected"
        );
    } else {
        tracing::debug!(
            album_id = album_id,
            count = tracks.len(),
            query_ms = query_duration.as_millis(),
            total_ms = total_duration.as_millis(),
            "[Storage:get_by_album] Completed"
        );
    }

    Ok(tracks)
}

/// Create new track
pub async fn create(pool: &SqlitePool, track: CreateTrack) -> Result<Track> {
    let start = std::time::Instant::now();
    tracing::debug!(
        title = %track.title,
        artist_id = ?track.artist_id,
        album_id = ?track.album_id,
        "[Storage:create] Creating track"
    );

    // Start transaction
    let mut tx = pool.begin().await?;

    // Insert track
    let insert_start = std::time::Instant::now();
    let result: sqlx::sqlite::SqliteQueryResult = sqlx::query!(
        r#"
        INSERT INTO tracks (
            title, artist_id, album_id, album_artist_id, track_number, disc_number, year,
            duration_seconds, bitrate, sample_rate, channels, file_format, file_hash,
            origin_source_id, musicbrainz_recording_id, fingerprint
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        track.title,
        track.artist_id,
        track.album_id,
        track.album_artist_id,
        track.track_number,
        track.disc_number,
        track.year,
        track.duration_seconds,
        track.bitrate,
        track.sample_rate,
        track.channels,
        track.file_format,
        track.file_hash,
        track.origin_source_id,
        track.musicbrainz_recording_id,
        track.fingerprint
    )
    .execute(&mut *tx)
    .await?;
    let insert_duration = insert_start.elapsed();

    let track_id = result.last_insert_rowid();
    tracing::debug!(
        track_id = track_id,
        insert_ms = insert_duration.as_millis(),
        "[Storage:create] Track record inserted"
    );

    // Create track_sources entry
    if let Some(local_file_path) = track.local_file_path {
        tracing::debug!(
            track_id = track_id,
            source_id = track.origin_source_id,
            path = %local_file_path,
            "[Storage:create] Creating track_sources entry"
        );
        sqlx::query!(
            r#"
            INSERT INTO track_sources (track_id, source_id, status, local_file_path)
            VALUES (?, ?, 'local_file', ?)
            "#,
            track_id,
            track.origin_source_id,
            local_file_path
        )
        .execute(&mut *tx)
        .await?;
        tracing::debug!(
            track_id = track_id,
            "[Storage:create] track_sources entry created"
        );
    } else {
        tracing::warn!(
            track_id = track_id,
            "[Storage:create] No local_file_path provided, skipping track_sources entry"
        );
    }

    // Note: track_stats is now per-user and created on-demand when a user plays/rates a track
    // No automatic initialization needed here

    let commit_start = std::time::Instant::now();
    tx.commit().await?;
    let commit_duration = commit_start.elapsed();

    // Fetch and return the created track
    let created_track = get_by_id(pool, TrackId::new(track_id.to_string()))
        .await?
        .ok_or_else(|| {
            soul_core::SoulError::Storage("Failed to retrieve created track".to_string())
        })?;

    let total_duration = start.elapsed();
    if total_duration.as_millis() > 100 {
        tracing::warn!(
            track_id = track_id,
            title = %track.title,
            insert_ms = insert_duration.as_millis(),
            commit_ms = commit_duration.as_millis(),
            total_ms = total_duration.as_millis(),
            "[Storage:create] Slow track creation detected"
        );
    } else {
        tracing::debug!(
            track_id = track_id,
            title = %track.title,
            insert_ms = insert_duration.as_millis(),
            commit_ms = commit_duration.as_millis(),
            total_ms = total_duration.as_millis(),
            "[Storage:create] Track created successfully"
        );
    }

    Ok(created_track)
}

/// Update track metadata
pub async fn update(pool: &SqlitePool, id: TrackId, track: UpdateTrack) -> Result<Track> {
    let id_clone = id.clone();
    let id_int: i64 = id
        .as_str()
        .parse()
        .map_err(|_| soul_core::SoulError::Storage(format!("Invalid track ID: {}", id)))?;

    let mut query_parts = Vec::new();
    let mut has_updates = false;

    if track.title.is_some() {
        query_parts.push("title = ?");
        has_updates = true;
    }
    if track.artist_id.is_some() {
        query_parts.push("artist_id = ?");
        has_updates = true;
    }
    if track.album_id.is_some() {
        query_parts.push("album_id = ?");
        has_updates = true;
    }
    if track.album_artist_id.is_some() {
        query_parts.push("album_artist_id = ?");
        has_updates = true;
    }
    if track.track_number.is_some() {
        query_parts.push("track_number = ?");
        has_updates = true;
    }
    if track.disc_number.is_some() {
        query_parts.push("disc_number = ?");
        has_updates = true;
    }
    if track.year.is_some() {
        query_parts.push("year = ?");
        has_updates = true;
    }
    if track.duration_seconds.is_some() {
        query_parts.push("duration_seconds = ?");
        has_updates = true;
    }
    if track.bitrate.is_some() {
        query_parts.push("bitrate = ?");
        has_updates = true;
    }
    if track.sample_rate.is_some() {
        query_parts.push("sample_rate = ?");
        has_updates = true;
    }
    if track.channels.is_some() {
        query_parts.push("channels = ?");
        has_updates = true;
    }
    if track.musicbrainz_recording_id.is_some() {
        query_parts.push("musicbrainz_recording_id = ?");
        has_updates = true;
    }
    if track.fingerprint.is_some() {
        query_parts.push("fingerprint = ?");
        has_updates = true;
    }
    if track.metadata_source.is_some() {
        query_parts.push("metadata_source = ?");
        has_updates = true;
    }

    if !has_updates {
        return get_by_id(pool, id_clone.clone())
            .await?
            .ok_or(soul_core::SoulError::TrackNotFound(id_clone));
    }

    query_parts.push("updated_at = datetime('now')");

    let query_str = format!("UPDATE tracks SET {} WHERE id = ?", query_parts.join(", "));

    let mut query = sqlx::query(&query_str);

    if let Some(title) = &track.title {
        query = query.bind(title);
    }
    if let Some(artist_id) = track.artist_id {
        query = query.bind(artist_id);
    }
    if let Some(album_id) = track.album_id {
        query = query.bind(album_id);
    }
    if let Some(album_artist_id) = track.album_artist_id {
        query = query.bind(album_artist_id);
    }
    if let Some(track_number) = track.track_number {
        query = query.bind(track_number);
    }
    if let Some(disc_number) = track.disc_number {
        query = query.bind(disc_number);
    }
    if let Some(year) = track.year {
        query = query.bind(year);
    }
    if let Some(duration_seconds) = track.duration_seconds {
        query = query.bind(duration_seconds);
    }
    if let Some(bitrate) = track.bitrate {
        query = query.bind(bitrate);
    }
    if let Some(sample_rate) = track.sample_rate {
        query = query.bind(sample_rate);
    }
    if let Some(channels) = track.channels {
        query = query.bind(channels);
    }
    if let Some(musicbrainz_recording_id) = &track.musicbrainz_recording_id {
        query = query.bind(musicbrainz_recording_id);
    }
    if let Some(fingerprint) = &track.fingerprint {
        query = query.bind(fingerprint);
    }
    if let Some(metadata_source) = &track.metadata_source {
        let metadata_str = format_metadata_source(metadata_source);
        query = query.bind(metadata_str);
    }

    query = query.bind(id_int);

    query.execute(pool).await?;

    get_by_id(pool, id_clone.clone())
        .await?
        .ok_or(soul_core::SoulError::TrackNotFound(id_clone))
}

/// Delete track
pub async fn delete(pool: &SqlitePool, id: TrackId) -> Result<()> {
    let id_int: i64 = id
        .as_str()
        .parse()
        .map_err(|_| soul_core::SoulError::Storage(format!("Invalid track ID: {}", id)))?;

    sqlx::query!("DELETE FROM tracks WHERE id = ?", id_int)
        .execute(pool)
        .await?;

    Ok(())
}

/// Get track availability across all sources
pub async fn get_availability(
    pool: &SqlitePool,
    track_id: TrackId,
) -> Result<Vec<TrackAvailability>> {
    let track_id_int: i64 = track_id
        .as_str()
        .parse()
        .map_err(|_| soul_core::SoulError::Storage(format!("Invalid track ID: {}", track_id)))?;

    let rows: Vec<_> = sqlx::query!(
        r#"
        SELECT source_id, status, local_file_path, server_path, local_file_size
        FROM track_sources
        WHERE track_id = ?
        "#,
        track_id_int
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| TrackAvailability {
            source_id: row.source_id,
            status: parse_availability_status(&row.status),
            local_file_path: row.local_file_path,
            server_path: row.server_path,
            local_file_size: row.local_file_size,
        })
        .collect())
}

/// Record track play
pub async fn record_play(
    pool: &SqlitePool,
    user_id: UserId,
    track_id: TrackId,
    duration_seconds: Option<f64>,
    completed: bool,
) -> Result<()> {
    let start = std::time::Instant::now();
    tracing::debug!(
        user_id = %user_id,
        track_id = %track_id,
        completed = completed,
        duration_secs = ?duration_seconds,
        "[Storage:record_play] Recording playback"
    );

    let track_id_int: i64 = track_id
        .as_str()
        .parse()
        .map_err(|_| soul_core::SoulError::Storage(format!("Invalid track ID: {}", track_id)))?;

    let mut tx = pool.begin().await?;

    // Insert play history
    sqlx::query!(
        r#"
        INSERT INTO play_history (user_id, track_id, play_duration_seconds, completed)
        VALUES (?, ?, ?, ?)
        "#,
        user_id,
        track_id_int,
        duration_seconds,
        completed
    )
    .execute(&mut *tx)
    .await?;

    // Update track stats (per-user)
    if completed {
        sqlx::query!(
            r#"
            INSERT INTO track_stats (user_id, track_id, play_count, last_played_at)
            VALUES (?, ?, 1, datetime('now'))
            ON CONFLICT(user_id, track_id) DO UPDATE SET
                play_count = play_count + 1,
                last_played_at = datetime('now')
            "#,
            user_id,
            track_id_int
        )
        .execute(&mut *tx)
        .await?;
    } else {
        sqlx::query!(
            r#"
            INSERT INTO track_stats (user_id, track_id, skip_count)
            VALUES (?, ?, 1)
            ON CONFLICT(user_id, track_id) DO UPDATE SET
                skip_count = skip_count + 1
            "#,
            user_id,
            track_id_int
        )
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    let total_duration = start.elapsed();
    if total_duration.as_millis() > 100 {
        tracing::warn!(
            user_id = %user_id,
            track_id = %track_id,
            completed = completed,
            total_ms = total_duration.as_millis(),
            "[Storage:record_play] Slow query detected"
        );
    } else {
        tracing::debug!(
            user_id = %user_id,
            track_id = %track_id,
            completed = completed,
            total_ms = total_duration.as_millis(),
            "[Storage:record_play] Completed"
        );
    }

    Ok(())
}

/// Get top tracks by artist sorted by play count
pub async fn get_top_tracks_by_artist(
    pool: &SqlitePool,
    user_id: UserId,
    artist_id: ArtistId,
    limit: i32,
) -> Result<Vec<Track>> {
    let rows: Vec<_> = sqlx::query!(
        r#"
        SELECT
            t.id, t.title, t.artist_id, t.album_id, t.album_artist_id,
            t.track_number, t.disc_number, t.year, t.duration_seconds,
            t.bitrate, t.sample_rate, t.channels, t.file_format,
            t.origin_source_id, t.musicbrainz_recording_id, t.fingerprint,
            t.metadata_source, t.created_at, t.updated_at,
            ar.name as artist_name,
            al.title as album_title
        FROM tracks t
        LEFT JOIN track_stats ts ON t.id = ts.track_id AND ts.user_id = ?
        LEFT JOIN artists ar ON t.artist_id = ar.id
        LEFT JOIN albums al ON t.album_id = al.id
        WHERE t.artist_id = ?
        ORDER BY COALESCE(ts.play_count, 0) DESC, t.title ASC
        LIMIT ?
        "#,
        user_id,
        artist_id,
        limit
    )
    .fetch_all(pool)
    .await?;

    if rows.is_empty() {
        return Ok(Vec::new());
    }

    // Collect track IDs for batch availability lookup
    let track_ids: Vec<i64> = rows.iter().map(|r| r.id).collect();

    // Batch fetch availability data for these tracks using compile-time safe queries
    let availability_rows = batch_fetch_availability(pool, &track_ids).await?;

    // Group availability by track_id (pre-allocate with capacity for better performance)
    let mut availability_map: std::collections::HashMap<String, Vec<TrackAvailability>> =
        std::collections::HashMap::with_capacity(track_ids.len());
    for (track_id, source_id, status, local_file_path, server_path, local_file_size) in
        availability_rows
    {
        availability_map
            .entry(track_id.to_string())
            .or_insert_with(Vec::new)
            .push(TrackAvailability {
                source_id,
                status: parse_availability_status(&status),
                local_file_path,
                server_path,
                local_file_size,
            });
    }

    // Build track objects with availability data
    let tracks: Vec<_> = rows
        .into_iter()
        .map(|row| {
            let track_id = TrackId::new(row.id.to_string());
            let availability = availability_map
                .get(track_id.as_str())
                .cloned()
                .unwrap_or_default();

            Track {
                id: track_id,
                title: row.title,
                artist_id: row.artist_id,
                artist_name: Some(row.artist_name),
                album_id: row.album_id,
                album_title: Some(row.album_title),
                album_artist_id: row.album_artist_id,
                track_number: row.track_number.map(|x| x as i32),
                disc_number: row.disc_number.map(|x| x as i32),
                year: row.year.map(|x| x as i32),
                duration_seconds: row.duration_seconds,
                bitrate: row.bitrate.map(|x| x as i32),
                sample_rate: row.sample_rate.map(|x| x as i32),
                channels: row.channels.map(|x| x as i32),
                file_format: row.file_format.unwrap_or_else(|| "unknown".to_string()),
                origin_source_id: row.origin_source_id,
                musicbrainz_recording_id: row.musicbrainz_recording_id,
                fingerprint: row.fingerprint,
                metadata_source: parse_metadata_source(
                    row.metadata_source.as_deref().unwrap_or("file"),
                ),
                created_at: row.created_at,
                updated_at: row.updated_at,
                cover_art_path: None,
                artwork_source: None,
                artists: Vec::new(),
                availability,
            }
        })
        .collect();

    Ok(tracks)
}

/// Get recently played tracks
pub async fn get_recently_played(
    pool: &SqlitePool,
    user_id: UserId,
    limit: i32,
) -> Result<Vec<Track>> {
    let rows: Vec<_> = sqlx::query!(
        r#"
        SELECT DISTINCT
            t.id, t.title, t.artist_id, t.album_id, t.album_artist_id,
            t.track_number, t.disc_number, t.year, t.duration_seconds,
            t.bitrate, t.sample_rate, t.channels, t.file_format,
            t.origin_source_id, t.musicbrainz_recording_id, t.fingerprint,
            t.metadata_source, t.created_at, t.updated_at,
            ar.name as artist_name,
            al.title as album_title
        FROM tracks t
        LEFT JOIN artists ar ON t.artist_id = ar.id
        LEFT JOIN albums al ON t.album_id = al.id
        INNER JOIN play_history ph ON t.id = ph.track_id
        WHERE ph.user_id = ?
        GROUP BY t.id
        ORDER BY MAX(ph.played_at) DESC
        LIMIT ?
        "#,
        user_id,
        limit
    )
    .fetch_all(pool)
    .await?;

    if rows.is_empty() {
        return Ok(Vec::new());
    }

    // Collect track IDs for batch availability lookup
    let track_ids: Vec<i64> = rows.iter().map(|r| r.id).collect();

    // Batch fetch availability data for these tracks using compile-time safe queries
    let availability_rows = batch_fetch_availability(pool, &track_ids).await?;

    // Group availability by track_id (pre-allocate with capacity for better performance)
    let mut availability_map: std::collections::HashMap<String, Vec<TrackAvailability>> =
        std::collections::HashMap::with_capacity(track_ids.len());
    for (track_id, source_id, status, local_file_path, server_path, local_file_size) in
        availability_rows
    {
        availability_map
            .entry(track_id.to_string())
            .or_insert_with(Vec::new)
            .push(TrackAvailability {
                source_id,
                status: parse_availability_status(&status),
                local_file_path,
                server_path,
                local_file_size,
            });
    }

    // Build track objects with availability data
    let tracks: Vec<_> = rows
        .into_iter()
        .map(|row| {
            let track_id = TrackId::new(row.id.to_string());
            let availability = availability_map
                .get(track_id.as_str())
                .cloned()
                .unwrap_or_default();

            Track {
                id: track_id,
                title: row.title,
                artist_id: row.artist_id,
                artist_name: Some(row.artist_name),
                album_id: row.album_id,
                album_title: Some(row.album_title),
                album_artist_id: row.album_artist_id,
                track_number: row.track_number.map(|x| x as i32),
                disc_number: row.disc_number.map(|x| x as i32),
                year: row.year.map(|x| x as i32),
                duration_seconds: row.duration_seconds,
                bitrate: row.bitrate.map(|x| x as i32),
                sample_rate: row.sample_rate.map(|x| x as i32),
                channels: row.channels.map(|x| x as i32),
                file_format: row.file_format.unwrap_or_else(|| "unknown".to_string()),
                origin_source_id: row.origin_source_id,
                musicbrainz_recording_id: row.musicbrainz_recording_id,
                fingerprint: row.fingerprint,
                metadata_source: parse_metadata_source(
                    row.metadata_source.as_deref().unwrap_or("file"),
                ),
                created_at: row.created_at,
                updated_at: row.updated_at,
                cover_art_path: None,
                artwork_source: None,
                artists: Vec::new(),
                availability,
            }
        })
        .collect();

    Ok(tracks)
}

/// Get play count for track
pub async fn get_play_count(pool: &SqlitePool, user_id: UserId, track_id: TrackId) -> Result<i32> {
    let track_id_int: i64 = track_id
        .as_str()
        .parse()
        .map_err(|_| soul_core::SoulError::Storage(format!("Invalid track ID: {}", track_id)))?;

    let user_id_str = user_id.as_str();

    let row: Option<_> = sqlx::query!(
        "SELECT play_count FROM track_stats WHERE user_id = ? AND track_id = ?",
        user_id_str,
        track_id_int
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.map_or(0, |r| r.play_count as i32))
}

/// Find track by file hash (for duplicate detection)
pub async fn find_by_hash(pool: &SqlitePool, file_hash: &str) -> Result<Option<Track>> {
    let start = std::time::Instant::now();
    tracing::debug!(
        file_hash = %file_hash,
        "[Storage:find_by_hash] Checking for duplicate"
    );

    let query_start = std::time::Instant::now();
    let row: Option<_> = sqlx::query!(
        r#"
        SELECT
            t.id, t.title, t.artist_id, t.album_id, t.album_artist_id,
            t.track_number, t.disc_number, t.year, t.duration_seconds,
            t.bitrate, t.sample_rate, t.channels, t.file_format,
            t.origin_source_id, t.musicbrainz_recording_id, t.fingerprint,
            t.metadata_source, t.created_at, t.updated_at,
            ar.name as artist_name,
            al.title as album_title
        FROM tracks t
        LEFT JOIN artists ar ON t.artist_id = ar.id
        LEFT JOIN albums al ON t.album_id = al.id
        WHERE t.file_hash = ?
        "#,
        file_hash
    )
    .fetch_optional(pool)
    .await?;
    let query_duration = query_start.elapsed();

    if let Some(row) = row {
        let track_id = TrackId::new(row.id.to_string());
        let availability = get_availability(pool, track_id.clone()).await?;

        let total_duration = start.elapsed();
        if total_duration.as_millis() > 100 {
            tracing::warn!(
                file_hash = %file_hash,
                track_id = %track_id,
                found = true,
                query_ms = query_duration.as_millis(),
                total_ms = total_duration.as_millis(),
                "[Storage:find_by_hash] Slow query detected"
            );
        } else {
            tracing::debug!(
                file_hash = %file_hash,
                track_id = %track_id,
                found = true,
                query_ms = query_duration.as_millis(),
                total_ms = total_duration.as_millis(),
                "[Storage:find_by_hash] Duplicate found"
            );
        }

        Ok(Some(Track {
            id: track_id,
            title: row.title,
            artist_id: row.artist_id,
            artist_name: Some(row.artist_name),
            album_id: row.album_id,
            album_title: Some(row.album_title),
            album_artist_id: row.album_artist_id,
            track_number: row.track_number.map(|x| x as i32),
            disc_number: row.disc_number.map(|x| x as i32),
            year: row.year.map(|x| x as i32),
            duration_seconds: row.duration_seconds,
            bitrate: row.bitrate.map(|x| x as i32),
            sample_rate: row.sample_rate.map(|x| x as i32),
            channels: row.channels.map(|x| x as i32),
            file_format: row.file_format.unwrap_or_else(|| "unknown".to_string()),
            origin_source_id: row.origin_source_id,
            musicbrainz_recording_id: row.musicbrainz_recording_id,
            fingerprint: row.fingerprint,
            metadata_source: parse_metadata_source(
                row.metadata_source.as_deref().unwrap_or("file"),
            ),
            created_at: row.created_at,
            updated_at: row.updated_at,
            cover_art_path: None,
            artwork_source: None,
            artists: Vec::new(),
            availability,
        }))
    } else {
        let total_duration = start.elapsed();
        tracing::debug!(
            file_hash = %file_hash,
            found = false,
            total_ms = total_duration.as_millis(),
            "[Storage:find_by_hash] No duplicate found"
        );
        Ok(None)
    }
}

/// Find track file path by content hash (for duplicate detection during import)
pub async fn find_path_by_content_hash(
    pool: &SqlitePool,
    content_hash: &str,
) -> Result<Option<String>> {
    let row: Option<_> = sqlx::query!(
        r#"
        SELECT file_path
        FROM tracks
        WHERE content_hash = ? AND is_available = 1
        LIMIT 1
        "#,
        content_hash
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.and_then(|r| r.file_path))
}

// =============================================================================
// Paginated Query Functions (for lazy loading)
// =============================================================================

/// Get all tracks with pagination
pub async fn get_all_paginated(pool: &SqlitePool, offset: i64, limit: i64) -> Result<Vec<Track>> {
    let rows: Vec<_> = sqlx::query!(
        r#"
        SELECT
            t.id, t.title, t.artist_id, t.album_id, t.album_artist_id,
            t.track_number, t.disc_number, t.year, t.duration_seconds,
            t.bitrate, t.sample_rate, t.channels, t.file_format,
            t.origin_source_id, t.musicbrainz_recording_id, t.fingerprint,
            t.metadata_source, t.created_at, t.updated_at,
            ar.name as artist_name,
            al.title as album_title
        FROM tracks t
        LEFT JOIN artists ar ON t.artist_id = ar.id
        LEFT JOIN albums al ON t.album_id = al.id
        ORDER BY t.title
        LIMIT ? OFFSET ?
        "#,
        limit,
        offset
    )
    .fetch_all(pool)
    .await?;

    if rows.is_empty() {
        return Ok(Vec::new());
    }

    // Collect track IDs for batch availability lookup
    let track_ids: Vec<i64> = rows.iter().map(|r| r.id).collect();

    // Batch fetch availability data for these tracks using compile-time safe queries
    let availability_rows = batch_fetch_availability(pool, &track_ids).await?;

    // Group availability by track_id (pre-allocate with capacity for better performance)
    let mut availability_map: std::collections::HashMap<String, Vec<TrackAvailability>> =
        std::collections::HashMap::with_capacity(track_ids.len());
    for (track_id, source_id, status, local_file_path, server_path, local_file_size) in
        availability_rows
    {
        availability_map
            .entry(track_id.to_string())
            .or_insert_with(Vec::new)
            .push(TrackAvailability {
                source_id,
                status: parse_availability_status(&status),
                local_file_path,
                server_path,
                local_file_size,
            });
    }

    // Build track objects with availability data
    let tracks: Vec<_> = rows
        .into_iter()
        .map(|row| {
            let track_id = TrackId::new(row.id.to_string());
            let availability = availability_map
                .get(track_id.as_str())
                .cloned()
                .unwrap_or_default();

            Track {
                id: track_id,
                title: row.title,
                artist_id: row.artist_id,
                artist_name: row.artist_name,
                album_id: row.album_id,
                album_title: row.album_title,
                album_artist_id: row.album_artist_id,
                track_number: row.track_number.map(|x| x as i32),
                disc_number: row.disc_number.map(|x| x as i32),
                year: row.year.map(|x| x as i32),
                duration_seconds: row.duration_seconds,
                bitrate: row.bitrate.map(|x| x as i32),
                sample_rate: row.sample_rate.map(|x| x as i32),
                channels: row.channels.map(|x| x as i32),
                file_format: row.file_format.unwrap_or_else(|| "unknown".to_string()),
                origin_source_id: row.origin_source_id,
                musicbrainz_recording_id: row.musicbrainz_recording_id,
                fingerprint: row.fingerprint,
                metadata_source: parse_metadata_source(
                    row.metadata_source.as_deref().unwrap_or("file"),
                ),
                created_at: row.created_at,
                updated_at: row.updated_at,
                cover_art_path: None,
                artwork_source: None,
                artists: Vec::new(),
                availability,
            }
        })
        .collect();

    Ok(tracks)
}

/// Get tracks by artist with pagination
pub async fn get_by_artist_paginated(
    pool: &SqlitePool,
    artist_id: ArtistId,
    offset: i64,
    limit: i64,
) -> Result<Vec<Track>> {
    let rows: Vec<_> = sqlx::query!(
        r#"
        SELECT
            t.id, t.title, t.artist_id, t.album_id, t.album_artist_id,
            t.track_number, t.disc_number, t.year, t.duration_seconds,
            t.bitrate, t.sample_rate, t.channels, t.file_format,
            t.origin_source_id, t.musicbrainz_recording_id, t.fingerprint,
            t.metadata_source, t.created_at, t.updated_at,
            ar.name as artist_name,
            al.title as album_title
        FROM tracks t
        LEFT JOIN artists ar ON t.artist_id = ar.id
        LEFT JOIN albums al ON t.album_id = al.id
        WHERE t.artist_id = ?
        ORDER BY t.album_id, t.disc_number, t.track_number, t.title
        LIMIT ? OFFSET ?
        "#,
        artist_id,
        limit,
        offset
    )
    .fetch_all(pool)
    .await?;

    if rows.is_empty() {
        return Ok(Vec::new());
    }

    // Collect track IDs for batch availability lookup
    let track_ids: Vec<i64> = rows.iter().map(|r| r.id).collect();

    // Batch fetch availability data for these tracks using compile-time safe queries
    let availability_rows = batch_fetch_availability(pool, &track_ids).await?;

    // Group availability by track_id (pre-allocate with capacity for better performance)
    let mut availability_map: std::collections::HashMap<String, Vec<TrackAvailability>> =
        std::collections::HashMap::with_capacity(track_ids.len());
    for (track_id, source_id, status, local_file_path, server_path, local_file_size) in
        availability_rows
    {
        availability_map
            .entry(track_id.to_string())
            .or_insert_with(Vec::new)
            .push(TrackAvailability {
                source_id,
                status: parse_availability_status(&status),
                local_file_path,
                server_path,
                local_file_size,
            });
    }

    // Build track objects with availability data
    let tracks: Vec<_> = rows
        .into_iter()
        .map(|row| {
            let track_id = TrackId::new(row.id.to_string());
            let availability = availability_map
                .get(track_id.as_str())
                .cloned()
                .unwrap_or_default();

            Track {
                id: track_id,
                title: row.title,
                artist_id: row.artist_id,
                artist_name: Some(row.artist_name),
                album_id: row.album_id,
                album_title: Some(row.album_title),
                album_artist_id: row.album_artist_id,
                track_number: row.track_number.map(|x| x as i32),
                disc_number: row.disc_number.map(|x| x as i32),
                year: row.year.map(|x| x as i32),
                duration_seconds: row.duration_seconds,
                bitrate: row.bitrate.map(|x| x as i32),
                sample_rate: row.sample_rate.map(|x| x as i32),
                channels: row.channels.map(|x| x as i32),
                file_format: row.file_format.unwrap_or_else(|| "unknown".to_string()),
                origin_source_id: row.origin_source_id,
                musicbrainz_recording_id: row.musicbrainz_recording_id,
                fingerprint: row.fingerprint,
                metadata_source: parse_metadata_source(
                    row.metadata_source.as_deref().unwrap_or("file"),
                ),
                created_at: row.created_at,
                updated_at: row.updated_at,
                cover_art_path: None,
                artwork_source: None,
                artists: Vec::new(),
                availability,
            }
        })
        .collect();

    Ok(tracks)
}

/// Get tracks by album with pagination
pub async fn get_by_album_paginated(
    pool: &SqlitePool,
    album_id: AlbumId,
    offset: i64,
    limit: i64,
) -> Result<Vec<Track>> {
    let rows: Vec<_> = sqlx::query!(
        r#"
        SELECT
            t.id, t.title, t.artist_id, t.album_id, t.album_artist_id,
            t.track_number, t.disc_number, t.year, t.duration_seconds,
            t.bitrate, t.sample_rate, t.channels, t.file_format,
            t.origin_source_id, t.musicbrainz_recording_id, t.fingerprint,
            t.metadata_source, t.created_at, t.updated_at,
            ar.name as artist_name,
            al.title as album_title
        FROM tracks t
        LEFT JOIN artists ar ON t.artist_id = ar.id
        LEFT JOIN albums al ON t.album_id = al.id
        WHERE t.album_id = ?
        ORDER BY t.disc_number, t.track_number, t.title
        LIMIT ? OFFSET ?
        "#,
        album_id,
        limit,
        offset
    )
    .fetch_all(pool)
    .await?;

    if rows.is_empty() {
        return Ok(Vec::new());
    }

    // Collect track IDs for batch availability lookup
    let track_ids: Vec<i64> = rows.iter().map(|r| r.id).collect();

    // Batch fetch availability data for these tracks using compile-time safe queries
    let availability_rows = batch_fetch_availability(pool, &track_ids).await?;

    // Group availability by track_id (pre-allocate with capacity for better performance)
    let mut availability_map: std::collections::HashMap<String, Vec<TrackAvailability>> =
        std::collections::HashMap::with_capacity(track_ids.len());
    for (track_id, source_id, status, local_file_path, server_path, local_file_size) in
        availability_rows
    {
        availability_map
            .entry(track_id.to_string())
            .or_insert_with(Vec::new)
            .push(TrackAvailability {
                source_id,
                status: parse_availability_status(&status),
                local_file_path,
                server_path,
                local_file_size,
            });
    }

    // Build track objects with availability data
    let tracks: Vec<_> = rows
        .into_iter()
        .map(|row| {
            let track_id = TrackId::new(row.id.to_string());
            let availability = availability_map
                .get(track_id.as_str())
                .cloned()
                .unwrap_or_default();

            Track {
                id: track_id,
                title: row.title,
                artist_id: row.artist_id,
                artist_name: Some(row.artist_name),
                album_id: row.album_id,
                album_title: Some(row.album_title),
                album_artist_id: row.album_artist_id,
                track_number: row.track_number.map(|x| x as i32),
                disc_number: row.disc_number.map(|x| x as i32),
                year: row.year.map(|x| x as i32),
                duration_seconds: row.duration_seconds,
                bitrate: row.bitrate.map(|x| x as i32),
                sample_rate: row.sample_rate.map(|x| x as i32),
                channels: row.channels.map(|x| x as i32),
                file_format: row.file_format.unwrap_or_else(|| "unknown".to_string()),
                origin_source_id: row.origin_source_id,
                musicbrainz_recording_id: row.musicbrainz_recording_id,
                fingerprint: row.fingerprint,
                metadata_source: parse_metadata_source(
                    row.metadata_source.as_deref().unwrap_or("file"),
                ),
                created_at: row.created_at,
                updated_at: row.updated_at,
                cover_art_path: None,
                artwork_source: None,
                artists: Vec::new(),
                availability,
            }
        })
        .collect();

    Ok(tracks)
}

/// Get tracks by playlist with pagination
pub async fn get_by_playlist_paginated(
    pool: &SqlitePool,
    playlist_id: PlaylistId,
    offset: i64,
    limit: i64,
) -> Result<Vec<Track>> {
    let rows: Vec<_> = sqlx::query!(
        r#"
        SELECT
            t.id, t.title, t.artist_id, t.album_id, t.album_artist_id,
            t.track_number, t.disc_number, t.year, t.duration_seconds,
            t.bitrate, t.sample_rate, t.channels, t.file_format,
            t.origin_source_id, t.musicbrainz_recording_id, t.fingerprint,
            t.metadata_source, t.created_at, t.updated_at,
            ar.name as "artist_name?",
            al.title as "album_title?"
        FROM tracks t
        LEFT JOIN artists ar ON t.artist_id = ar.id
        LEFT JOIN albums al ON t.album_id = al.id
        INNER JOIN playlist_tracks pt ON t.id = pt.track_id
        WHERE pt.playlist_id = ?
        ORDER BY pt.position
        LIMIT ? OFFSET ?
        "#,
        playlist_id,
        limit,
        offset
    )
    .fetch_all(pool)
    .await?;

    if rows.is_empty() {
        return Ok(Vec::new());
    }

    // Collect track IDs for batch availability lookup
    let track_ids: Vec<i64> = rows.iter().map(|r| r.id).collect();

    // Batch fetch availability data for these tracks using compile-time safe queries
    let availability_rows = batch_fetch_availability(pool, &track_ids).await?;

    // Group availability by track_id (pre-allocate with capacity for better performance)
    let mut availability_map: std::collections::HashMap<String, Vec<TrackAvailability>> =
        std::collections::HashMap::with_capacity(track_ids.len());
    for (track_id, source_id, status, local_file_path, server_path, local_file_size) in
        availability_rows
    {
        availability_map
            .entry(track_id.to_string())
            .or_insert_with(Vec::new)
            .push(TrackAvailability {
                source_id,
                status: parse_availability_status(&status),
                local_file_path,
                server_path,
                local_file_size,
            });
    }

    // Build track objects with availability data
    let tracks: Vec<_> = rows
        .into_iter()
        .map(|row| {
            let track_id = TrackId::new(row.id.to_string());
            let availability = availability_map
                .get(track_id.as_str())
                .cloned()
                .unwrap_or_default();

            Track {
                id: track_id,
                title: row.title,
                artist_id: row.artist_id,
                artist_name: row.artist_name,
                album_id: row.album_id,
                album_title: row.album_title,
                album_artist_id: row.album_artist_id,
                track_number: row.track_number.map(|x| x as i32),
                disc_number: row.disc_number.map(|x| x as i32),
                year: row.year.map(|x| x as i32),
                duration_seconds: row.duration_seconds,
                bitrate: row.bitrate.map(|x| x as i32),
                sample_rate: row.sample_rate.map(|x| x as i32),
                channels: row.channels.map(|x| x as i32),
                file_format: row.file_format.unwrap_or_else(|| "unknown".to_string()),
                origin_source_id: row.origin_source_id,
                musicbrainz_recording_id: row.musicbrainz_recording_id,
                fingerprint: row.fingerprint,
                metadata_source: parse_metadata_source(
                    row.metadata_source.as_deref().unwrap_or("file"),
                ),
                created_at: row.created_at,
                updated_at: row.updated_at,
                cover_art_path: None,
                artwork_source: None,
                artists: Vec::new(),
                availability,
            }
        })
        .collect();

    Ok(tracks)
}

// =============================================================================
// Helper functions

fn parse_metadata_source(s: &str) -> MetadataSource {
    match s {
        "enriched" => MetadataSource::Enriched,
        "user_edited" => MetadataSource::UserEdited,
        _ => MetadataSource::File,
    }
}

// =============================================================================
// Library Scanning Functions
// =============================================================================

/// Track file info for library scanning
#[derive(Debug, Clone)]
pub struct TrackFileInfo {
    pub id: i64,
    pub file_path: Option<String>,
    pub file_size: Option<i64>,
    pub file_mtime: Option<i64>,
    pub content_hash: Option<String>,
}

/// Get tracks for a library source with file metadata
pub async fn get_by_library_source(
    pool: &SqlitePool,
    source_id: i64,
) -> Result<Vec<TrackFileInfo>> {
    let rows: Vec<_> = sqlx::query!(
        r#"
        SELECT id, file_path, file_size, file_mtime, content_hash
        FROM tracks
        WHERE library_source_id = ? AND is_available = 1
        "#,
        source_id
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| TrackFileInfo {
            id: r.id,
            file_path: r.file_path,
            file_size: r.file_size,
            file_mtime: r.file_mtime,
            content_hash: r.content_hash,
        })
        .collect())
}

/// Update track file metadata after file change
pub async fn update_file_metadata(
    pool: &SqlitePool,
    track_id: i64,
    title: Option<&str>,
    track_number: Option<u32>,
    disc_number: Option<u32>,
    year: Option<i32>,
    duration_seconds: Option<f64>,
    bitrate: Option<u32>,
    sample_rate: Option<u32>,
    channels: Option<u8>,
    file_format: &str,
    file_size: i64,
    file_mtime: i64,
    content_hash: Option<&str>,
) -> Result<()> {
    sqlx::query!(
        r#"
        UPDATE tracks
        SET title = COALESCE(?, title),
            track_number = ?,
            disc_number = ?,
            year = ?,
            duration_seconds = ?,
            bitrate = ?,
            sample_rate = ?,
            channels = ?,
            file_format = ?,
            file_size = ?,
            file_mtime = ?,
            content_hash = COALESCE(?, content_hash),
            updated_at = datetime('now')
        WHERE id = ?
        "#,
        title,
        track_number,
        disc_number,
        year,
        duration_seconds,
        bitrate,
        sample_rate,
        channels,
        file_format,
        file_size,
        file_mtime,
        content_hash,
        track_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Update artist and album for a track
pub async fn update_artist_album(
    pool: &SqlitePool,
    track_id: i64,
    artist_id: Option<ArtistId>,
    album_id: Option<AlbumId>,
) -> Result<()> {
    sqlx::query!(
        r#"
        UPDATE tracks
        SET artist_id = COALESCE(?, artist_id),
            album_id = COALESCE(?, album_id),
            updated_at = datetime('now')
        WHERE id = ?
        "#,
        artist_id,
        album_id,
        track_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Update track path after file relocation
pub async fn update_file_path(
    pool: &SqlitePool,
    track_id: &str,
    file_path: &str,
    source_id: i64,
    file_size: i64,
    file_mtime: i64,
) -> Result<()> {
    sqlx::query!(
        r#"
        UPDATE tracks
        SET file_path = ?,
            library_source_id = ?,
            file_size = ?,
            file_mtime = ?,
            is_available = 1,
            unavailable_since = NULL,
            updated_at = datetime('now')
        WHERE id = ?
        "#,
        file_path,
        source_id,
        file_size,
        file_mtime,
        track_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Update library source for a track
pub async fn set_library_source(
    pool: &SqlitePool,
    track_id: i64,
    source_id: i64,
    file_size: i64,
    file_mtime: i64,
) -> Result<()> {
    sqlx::query!(
        r#"
        UPDATE tracks
        SET library_source_id = ?, file_size = ?, file_mtime = ?, is_available = 1
        WHERE id = ?
        "#,
        source_id,
        file_size,
        file_mtime,
        track_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Mark a track as unavailable (soft delete)
pub async fn mark_unavailable(pool: &SqlitePool, track_id: i64) -> Result<()> {
    let now = now_timestamp();

    sqlx::query!(
        r#"
        UPDATE tracks
        SET is_available = 0, unavailable_since = ?, updated_at = datetime('now')
        WHERE id = ?
        "#,
        now,
        track_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Mark a track as available again
pub async fn mark_available(pool: &SqlitePool, track_id: i64) -> Result<()> {
    sqlx::query!(
        r#"
        UPDATE tracks
        SET is_available = 1, unavailable_since = NULL, updated_at = datetime('now')
        WHERE id = ?
        "#,
        track_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Get unavailable tracks for a library source
pub async fn get_unavailable_by_source(
    pool: &SqlitePool,
    source_id: i64,
) -> Result<Vec<TrackFileInfo>> {
    let rows: Vec<_> = sqlx::query!(
        r#"
        SELECT id, file_path, file_size, file_mtime, content_hash
        FROM tracks
        WHERE library_source_id = ? AND is_available = 0
        "#,
        source_id
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| TrackFileInfo {
            id: r.id,
            file_path: r.file_path,
            file_size: r.file_size,
            file_mtime: r.file_mtime,
            content_hash: r.content_hash,
        })
        .collect())
}

// =============================================================================
// Helper Functions
/// Set the audio fingerprint for a track
pub async fn set_fingerprint(pool: &SqlitePool, track_id: &str, fingerprint: &str) -> Result<()> {
    let now = now_timestamp();

    sqlx::query!(
        r#"
        UPDATE tracks
        SET fingerprint = ?, updated_at = ?
        WHERE id = ?
        "#,
        fingerprint,
        now,
        track_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Get tracks that don't have fingerprints yet
pub async fn get_without_fingerprint(pool: &SqlitePool, limit: i32) -> Result<Vec<Track>> {
    let rows: Vec<_> = sqlx::query!(
        r#"
        SELECT
            t.id, t.title, t.artist_id, t.album_id, t.album_artist_id,
            t.track_number, t.disc_number, t.year, t.duration_seconds,
            t.bitrate, t.sample_rate, t.channels, t.file_format,
            t.origin_source_id, t.musicbrainz_recording_id, t.fingerprint,
            t.metadata_source, t.created_at, t.updated_at,
            ar.name as artist_name,
            al.title as album_title
        FROM tracks t
        LEFT JOIN artists ar ON t.artist_id = ar.id
        LEFT JOIN albums al ON t.album_id = al.id
        WHERE t.fingerprint IS NULL
        LIMIT ?
        "#,
        limit
    )
    .fetch_all(pool)
    .await?;

    if rows.is_empty() {
        return Ok(Vec::new());
    }

    // Collect track IDs for batch availability lookup
    let track_ids: Vec<i64> = rows.iter().map(|r| r.id).collect();

    // Batch fetch availability data for these tracks using compile-time safe queries
    let availability_rows = batch_fetch_availability(pool, &track_ids).await?;

    // Group availability by track_id (pre-allocate with capacity for better performance)
    let mut availability_map: std::collections::HashMap<String, Vec<TrackAvailability>> =
        std::collections::HashMap::with_capacity(track_ids.len());
    for (track_id, source_id, status, local_file_path, server_path, local_file_size) in
        availability_rows
    {
        availability_map
            .entry(track_id.to_string())
            .or_insert_with(Vec::new)
            .push(TrackAvailability {
                source_id,
                status: parse_availability_status(&status),
                local_file_path,
                server_path,
                local_file_size,
            });
    }

    // Build track objects with availability data
    let tracks: Vec<_> = rows
        .into_iter()
        .map(|row| {
            let track_id = TrackId::new(row.id.to_string());
            let availability = availability_map
                .get(track_id.as_str())
                .cloned()
                .unwrap_or_default();

            Track {
                id: track_id,
                title: row.title,
                artist_id: row.artist_id,
                artist_name: row.artist_name,
                album_id: row.album_id,
                album_title: row.album_title,
                album_artist_id: row.album_artist_id,
                track_number: row.track_number.map(|x| x as i32),
                disc_number: row.disc_number.map(|x| x as i32),
                year: row.year.map(|x| x as i32),
                duration_seconds: row.duration_seconds,
                bitrate: row.bitrate.map(|x| x as i32),
                sample_rate: row.sample_rate.map(|x| x as i32),
                channels: row.channels.map(|x| x as i32),
                file_format: row.file_format.unwrap_or_else(|| "unknown".to_string()),
                origin_source_id: row.origin_source_id,
                musicbrainz_recording_id: row.musicbrainz_recording_id,
                fingerprint: row.fingerprint,
                metadata_source: parse_metadata_source(
                    row.metadata_source.as_deref().unwrap_or("file"),
                ),
                created_at: row.created_at,
                updated_at: row.updated_at,
                cover_art_path: None,
                artwork_source: None,
                artists: Vec::new(),
                availability,
            }
        })
        .collect();

    Ok(tracks)
}

/// Get all tracks that have fingerprints (for duplicate detection)
/// Supports optional pagination via limit and after_id (cursor-based)
pub async fn get_with_fingerprints(
    pool: &SqlitePool,
    limit: Option<i64>,
    after_id: Option<String>,
) -> Result<Vec<Track>> {
    // Default limit to 1000 if not specified
    let limit = limit.unwrap_or(1000);
    let after_id_int: Option<i64> = after_id.as_ref().and_then(|id| id.parse::<i64>().ok());

    let rows: Vec<_> = sqlx::query!(
        r#"
        SELECT
            t.id, t.title, t.artist_id, t.album_id, t.album_artist_id,
            t.track_number, t.disc_number, t.year, t.duration_seconds,
            t.bitrate, t.sample_rate, t.channels, t.file_format,
            t.origin_source_id, t.musicbrainz_recording_id, t.fingerprint,
            t.metadata_source, t.created_at, t.updated_at,
            ar.name as artist_name,
            al.title as album_title
        FROM tracks t
        LEFT JOIN artists ar ON t.artist_id = ar.id
        LEFT JOIN albums al ON t.album_id = al.id
        WHERE t.fingerprint IS NOT NULL
        AND (? IS NULL OR t.id > ?)
        ORDER BY t.id
        LIMIT ?
        "#,
        after_id_int,
        after_id_int,
        limit
    )
    .fetch_all(pool)
    .await?;

    if rows.is_empty() {
        return Ok(Vec::new());
    }

    // Collect track IDs for batch availability lookup
    let track_ids: Vec<i64> = rows.iter().map(|r| r.id).collect();

    // Batch fetch availability data for these tracks using compile-time safe queries
    let availability_rows = batch_fetch_availability(pool, &track_ids).await?;

    // Group availability by track_id (pre-allocate with capacity for better performance)
    let mut availability_map: std::collections::HashMap<String, Vec<TrackAvailability>> =
        std::collections::HashMap::with_capacity(track_ids.len());
    for (track_id, source_id, status, local_file_path, server_path, local_file_size) in
        availability_rows
    {
        availability_map
            .entry(track_id.to_string())
            .or_insert_with(Vec::new)
            .push(TrackAvailability {
                source_id,
                status: parse_availability_status(&status),
                local_file_path,
                server_path,
                local_file_size,
            });
    }

    // Build track objects with availability data
    let tracks: Vec<_> = rows
        .into_iter()
        .map(|row| {
            let track_id = TrackId::new(row.id.to_string());
            let availability = availability_map
                .get(track_id.as_str())
                .cloned()
                .unwrap_or_default();

            Track {
                id: track_id,
                title: row.title,
                artist_id: row.artist_id,
                artist_name: row.artist_name,
                album_id: row.album_id,
                album_title: row.album_title,
                album_artist_id: row.album_artist_id,
                track_number: row.track_number.map(|x| x as i32),
                disc_number: row.disc_number.map(|x| x as i32),
                year: row.year.map(|x| x as i32),
                duration_seconds: row.duration_seconds,
                bitrate: row.bitrate.map(|x| x as i32),
                sample_rate: row.sample_rate.map(|x| x as i32),
                channels: row.channels.map(|x| x as i32),
                file_format: row.file_format.unwrap_or_else(|| "unknown".to_string()),
                origin_source_id: row.origin_source_id,
                musicbrainz_recording_id: row.musicbrainz_recording_id,
                fingerprint: row.fingerprint,
                metadata_source: parse_metadata_source(
                    row.metadata_source.as_deref().unwrap_or("file"),
                ),
                created_at: row.created_at,
                updated_at: row.updated_at,
                cover_art_path: None,
                artwork_source: None,
                artists: Vec::new(),
                availability,
            }
        })
        .collect();

    Ok(tracks)
}

// =============================================================================

/// Get tracks by genre
pub async fn get_by_genre(pool: &SqlitePool, genre_id: GenreId) -> Result<Vec<Track>> {
    let rows: Vec<_> = sqlx::query!(
        r#"
        SELECT
            t.id, t.title, t.artist_id, t.album_id, t.album_artist_id,
            t.track_number, t.disc_number, t.year, t.duration_seconds,
            t.bitrate, t.sample_rate, t.channels, t.file_format,
            t.origin_source_id, t.musicbrainz_recording_id, t.fingerprint,
            t.metadata_source, t.created_at, t.updated_at,
            ar.name as "artist_name?",
            al.title as "album_title?"
        FROM tracks t
        LEFT JOIN artists ar ON t.artist_id = ar.id
        LEFT JOIN albums al ON t.album_id = al.id
        INNER JOIN track_genres tg ON t.id = tg.track_id
        WHERE tg.genre_id = ?
        ORDER BY ar.name, al.title, t.disc_number, t.track_number, t.title
        "#,
        genre_id
    )
    .fetch_all(pool)
    .await?;

    if rows.is_empty() {
        return Ok(Vec::new());
    }

    // Collect track IDs for batch availability lookup
    let track_ids: Vec<i64> = rows.iter().map(|r| r.id).collect();

    // Batch fetch availability data for these tracks using compile-time safe queries
    let availability_rows = batch_fetch_availability(pool, &track_ids).await?;

    // Group availability by track_id (pre-allocate with capacity for better performance)
    let mut availability_map: std::collections::HashMap<String, Vec<TrackAvailability>> =
        std::collections::HashMap::with_capacity(track_ids.len());
    for (track_id, source_id, status, local_file_path, server_path, local_file_size) in
        availability_rows
    {
        availability_map
            .entry(track_id.to_string())
            .or_insert_with(Vec::new)
            .push(TrackAvailability {
                source_id,
                status: parse_availability_status(&status),
                local_file_path,
                server_path,
                local_file_size,
            });
    }

    // Build track objects with availability data
    let tracks: Vec<_> = rows
        .into_iter()
        .map(|row| {
            let track_id = TrackId::new(row.id.to_string());
            let availability = availability_map
                .get(track_id.as_str())
                .cloned()
                .unwrap_or_default();

            Track {
                id: track_id,
                title: row.title,
                artist_id: row.artist_id,
                artist_name: row.artist_name,
                album_id: row.album_id,
                album_title: row.album_title,
                album_artist_id: row.album_artist_id,
                track_number: row.track_number.map(|x| x as i32),
                disc_number: row.disc_number.map(|x| x as i32),
                year: row.year.map(|x| x as i32),
                duration_seconds: row.duration_seconds,
                bitrate: row.bitrate.map(|x| x as i32),
                sample_rate: row.sample_rate.map(|x| x as i32),
                channels: row.channels.map(|x| x as i32),
                file_format: row.file_format.unwrap_or_else(|| "unknown".to_string()),
                origin_source_id: row.origin_source_id,
                musicbrainz_recording_id: row.musicbrainz_recording_id,
                fingerprint: row.fingerprint,
                metadata_source: parse_metadata_source(
                    row.metadata_source.as_deref().unwrap_or("file"),
                ),
                created_at: row.created_at,
                updated_at: row.updated_at,
                cover_art_path: None,
                artwork_source: None,
                artists: Vec::new(),
                availability,
            }
        })
        .collect();

    Ok(tracks)
}

/// Get tracks by playlist
pub async fn get_by_playlist(pool: &SqlitePool, playlist_id: PlaylistId) -> Result<Vec<Track>> {
    let rows: Vec<_> = sqlx::query!(
        r#"
        SELECT
            t.id, t.title, t.artist_id, t.album_id, t.album_artist_id,
            t.track_number, t.disc_number, t.year, t.duration_seconds,
            t.bitrate, t.sample_rate, t.channels, t.file_format,
            t.origin_source_id, t.musicbrainz_recording_id, t.fingerprint,
            t.metadata_source, t.created_at, t.updated_at,
            ar.name as "artist_name?",
            al.title as "album_title?"
        FROM tracks t
        LEFT JOIN artists ar ON t.artist_id = ar.id
        LEFT JOIN albums al ON t.album_id = al.id
        INNER JOIN playlist_tracks pt ON t.id = pt.track_id
        WHERE pt.playlist_id = ?
        ORDER BY pt.position
        "#,
        playlist_id
    )
    .fetch_all(pool)
    .await?;

    if rows.is_empty() {
        return Ok(Vec::new());
    }

    // Collect track IDs for batch availability lookup
    let track_ids: Vec<i64> = rows.iter().map(|r| r.id).collect();

    // Batch fetch availability data for these tracks using compile-time safe queries
    let availability_rows = batch_fetch_availability(pool, &track_ids).await?;

    // Group availability by track_id (pre-allocate with capacity for better performance)
    let mut availability_map: std::collections::HashMap<String, Vec<TrackAvailability>> =
        std::collections::HashMap::with_capacity(track_ids.len());
    for (track_id, source_id, status, local_file_path, server_path, local_file_size) in
        availability_rows
    {
        availability_map
            .entry(track_id.to_string())
            .or_insert_with(Vec::new)
            .push(TrackAvailability {
                source_id,
                status: parse_availability_status(&status),
                local_file_path,
                server_path,
                local_file_size,
            });
    }

    // Build track objects with availability data
    let tracks: Vec<_> = rows
        .into_iter()
        .map(|row| {
            let track_id = TrackId::new(row.id.to_string());
            let availability = availability_map
                .get(track_id.as_str())
                .cloned()
                .unwrap_or_default();

            Track {
                id: track_id,
                title: row.title,
                artist_id: row.artist_id,
                artist_name: row.artist_name,
                album_id: row.album_id,
                album_title: row.album_title,
                album_artist_id: row.album_artist_id,
                track_number: row.track_number.map(|x| x as i32),
                disc_number: row.disc_number.map(|x| x as i32),
                year: row.year.map(|x| x as i32),
                duration_seconds: row.duration_seconds,
                bitrate: row.bitrate.map(|x| x as i32),
                sample_rate: row.sample_rate.map(|x| x as i32),
                channels: row.channels.map(|x| x as i32),
                file_format: row.file_format.unwrap_or_else(|| "unknown".to_string()),
                origin_source_id: row.origin_source_id,
                musicbrainz_recording_id: row.musicbrainz_recording_id,
                fingerprint: row.fingerprint,
                metadata_source: parse_metadata_source(
                    row.metadata_source.as_deref().unwrap_or("file"),
                ),
                created_at: row.created_at,
                updated_at: row.updated_at,
                cover_art_path: None,
                artwork_source: None,
                artists: Vec::new(),
                availability,
            }
        })
        .collect();

    Ok(tracks)
}

fn format_metadata_source(source: &MetadataSource) -> &'static str {
    match source {
        MetadataSource::File => "file",
        MetadataSource::Enriched => "enriched",
        MetadataSource::UserEdited => "user_edited",
    }
}

fn parse_availability_status(s: &str) -> AvailabilityStatus {
    match s {
        "cached" => AvailabilityStatus::Cached,
        "stream_only" => AvailabilityStatus::StreamOnly,
        "unavailable" => AvailabilityStatus::Unavailable,
        _ => AvailabilityStatus::LocalFile,
    }
}
