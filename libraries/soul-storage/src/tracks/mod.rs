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
        eprintln!(
            "[tracks::create] Creating track_sources entry: track_id={}, source_id={}, path={}",
            track_id, track.origin_source_id, local_file_path
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
        eprintln!("[tracks::create] ✓ track_sources entry created");
    } else {
        eprintln!(
            "[tracks::create] ⚠ WARNING: No local_file_path provided, skipping track_sources entry"
        );
    }

    // Note: track_stats is now per-user and created on-demand when a user plays/rates a track
    // No automatic initialization needed here

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

/// Find track file path by content hash (for duplicate detection during import)
pub async fn find_path_by_content_hash(
    pool: &SqlitePool,
    content_hash: &str,
) -> Result<Option<String>> {
    let row = sqlx::query!(
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
    let rows = sqlx::query!(
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
    let now = chrono::Utc::now().timestamp();

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
    let rows = sqlx::query!(
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
    let now = chrono::Utc::now().timestamp();

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
        WHERE t.fingerprint IS NULL
        LIMIT ?
        "#,
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

/// Get all tracks that have fingerprints (for duplicate detection)
pub async fn get_with_fingerprints(pool: &SqlitePool) -> Result<Vec<Track>> {
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
        WHERE t.fingerprint IS NOT NULL
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

// =============================================================================

/// Get tracks by genre
pub async fn get_by_genre(pool: &SqlitePool, genre_id: GenreId) -> Result<Vec<Track>> {
    let rows = sqlx::query!(
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
