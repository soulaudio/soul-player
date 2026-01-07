use soul_core::{error::Result, types::*};
use sqlx::SqlitePool;

/// Get all tracks with denormalized artist/album names
pub async fn get_all(pool: &SqlitePool) -> Result<Vec<Track>> {
    let rows = sqlx::query!(
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
        "#
    )
    .fetch_all(pool)
    .await?;

    let mut tracks = Vec::new();
    for row in rows {
        let track_id = TrackId::new(row.id.to_string());
        let availability = get_availability(pool, track_id.clone()).await?;

        tracks.push(Track {
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
            availability,
        });
    }

    Ok(tracks)
}

/// Search tracks by query (searches title, artist name, album title)
pub async fn search(pool: &SqlitePool, query: &str) -> Result<Vec<Track>> {
    let search_pattern = format!("%{}%", query);

    let rows = sqlx::query!(
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

    let mut tracks = Vec::new();
    for row in rows {
        let track_id = TrackId::new(row.id.to_string());
        let availability = get_availability(pool, track_id.clone()).await?;

        tracks.push(Track {
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
                .and_then(|s| match s.as_str() {
                    "file" => Some(MetadataSource::File),
                    "enriched" => Some(MetadataSource::Enriched),
                    "user_edited" => Some(MetadataSource::UserEdited),
                    _ => None,
                })
                .unwrap_or(MetadataSource::File),
            created_at: row.created_at,
            updated_at: row.updated_at,
            availability,
        });
    }

    Ok(tracks)
}

/// Get track by ID
pub async fn get_by_id(pool: &SqlitePool, id: TrackId) -> Result<Option<Track>> {
    let id_int: i64 = id
        .as_str()
        .parse()
        .map_err(|_| soul_core::SoulError::Storage(format!("Invalid track ID: {}", id)))?;

    let row = sqlx::query!(
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
        WHERE t.id = ?
        "#,
        id_int
    )
    .fetch_optional(pool)
    .await?;

    match row {
        Some(row) => {
            let track_id = TrackId::new(row.id.to_string());
            let availability = get_availability(pool, track_id.clone()).await?;

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
                availability,
            }))
        }
        None => Ok(None),
    }
}

/// Get tracks by source
pub async fn get_by_source(pool: &SqlitePool, source_id: SourceId) -> Result<Vec<Track>> {
    let rows = sqlx::query!(
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
        INNER JOIN track_sources ts ON t.id = ts.track_id
        WHERE ts.source_id = ?
        ORDER BY t.title
        "#,
        source_id
    )
    .fetch_all(pool)
    .await?;

    let mut tracks = Vec::new();
    for row in rows {
        let track_id = TrackId::new(row.id.to_string());
        let availability = get_availability(pool, track_id.clone()).await?;

        tracks.push(Track {
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
            availability,
        });
    }

    Ok(tracks)
}

/// Get tracks by artist
pub async fn get_by_artist(pool: &SqlitePool, artist_id: ArtistId) -> Result<Vec<Track>> {
    let rows = sqlx::query!(
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

    let mut tracks = Vec::new();
    for row in rows {
        let track_id = TrackId::new(row.id.to_string());
        let availability = get_availability(pool, track_id.clone()).await?;

        tracks.push(Track {
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
            availability,
        });
    }

    Ok(tracks)
}

/// Get tracks by album
pub async fn get_by_album(pool: &SqlitePool, album_id: AlbumId) -> Result<Vec<Track>> {
    let rows = sqlx::query!(
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

    let mut tracks = Vec::new();
    for row in rows {
        let track_id = TrackId::new(row.id.to_string());
        let availability = get_availability(pool, track_id.clone()).await?;

        tracks.push(Track {
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
            availability,
        });
    }

    Ok(tracks)
}

/// Create new track
pub async fn create(pool: &SqlitePool, track: CreateTrack) -> Result<Track> {
    // Start transaction
    let mut tx = pool.begin().await?;

    // Insert track
    let result = sqlx::query!(
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

    let track_id = result.last_insert_rowid();

    // Create track_sources entry
    if let Some(local_file_path) = track.local_file_path {
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
    }

    // Initialize track stats
    sqlx::query!("INSERT INTO track_stats (track_id) VALUES (?)", track_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    // Fetch and return the created track
    get_by_id(pool, TrackId::new(track_id.to_string()))
        .await?
        .ok_or_else(|| {
            soul_core::SoulError::Storage("Failed to retrieve created track".to_string())
        })
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
    if track.track_number.is_some() {
        query_parts.push("track_number = ?");
        has_updates = true;
    }
    if track.year.is_some() {
        query_parts.push("year = ?");
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
    if let Some(track_number) = track.track_number {
        query = query.bind(track_number);
    }
    if let Some(year) = track.year {
        query = query.bind(year);
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

    let rows = sqlx::query!(
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

    // Update track stats
    if completed {
        sqlx::query!(
            r#"
            INSERT INTO track_stats (track_id, play_count, last_played_at)
            VALUES (?, 1, datetime('now'))
            ON CONFLICT(track_id) DO UPDATE SET
                play_count = play_count + 1,
                last_played_at = datetime('now')
            "#,
            track_id_int
        )
        .execute(&mut *tx)
        .await?;
    } else {
        sqlx::query!(
            r#"
            INSERT INTO track_stats (track_id, skip_count)
            VALUES (?, 1)
            ON CONFLICT(track_id) DO UPDATE SET
                skip_count = skip_count + 1
            "#,
            track_id_int
        )
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

/// Get recently played tracks
pub async fn get_recently_played(
    pool: &SqlitePool,
    user_id: UserId,
    limit: i32,
) -> Result<Vec<Track>> {
    let rows = sqlx::query!(
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

    let mut tracks = Vec::new();
    for row in rows {
        let track_id = TrackId::new(row.id.to_string());
        let availability = get_availability(pool, track_id.clone()).await?;

        tracks.push(Track {
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
            availability,
        });
    }

    Ok(tracks)
}

/// Get play count for track
pub async fn get_play_count(pool: &SqlitePool, track_id: TrackId) -> Result<i32> {
    let track_id_int: i64 = track_id
        .as_str()
        .parse()
        .map_err(|_| soul_core::SoulError::Storage(format!("Invalid track ID: {}", track_id)))?;

    let row = sqlx::query!(
        "SELECT play_count FROM track_stats WHERE track_id = ?",
        track_id_int
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.map_or(0, |r| r.play_count as i32))
}

/// Find track by file hash (for duplicate detection)
pub async fn find_by_hash(pool: &SqlitePool, file_hash: &str) -> Result<Option<Track>> {
    let row = sqlx::query!(
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

    if let Some(row) = row {
        let track_id = TrackId::new(row.id.to_string());
        let availability = get_availability(pool, track_id.clone()).await?;

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
            availability,
        }))
    } else {
        Ok(None)
    }
}

// Helper functions

fn parse_metadata_source(s: &str) -> MetadataSource {
    match s {
        "enriched" => MetadataSource::Enriched,
        "user_edited" => MetadataSource::UserEdited,
        _ => MetadataSource::File,
    }
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
