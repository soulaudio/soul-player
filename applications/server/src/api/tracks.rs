/// Tracks API routes
use crate::{
    error::{Result, ServerError},
    middleware::AuthenticatedUser,
    state::AppState,
};
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use soul_core::{
    storage::StorageContext,
    types::{Track, TrackId},
};

#[derive(Debug, Deserialize)]
pub struct TrackQuery {
    #[serde(default)]
    pub q: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub offset: usize,
}

fn default_limit() -> usize {
    50
}

#[derive(Debug, Serialize)]
pub struct TracksResponse {
    pub tracks: Vec<Track>,
    pub total: usize,
}

/// GET /api/tracks
pub async fn list_tracks(
    State(app_state): State<AppState>,
    Query(query): Query<TrackQuery>,
) -> Result<Json<TracksResponse>> {
    let tracks: Vec<Track> = if let Some(q) = query.q {
        app_state.db.search_tracks(&q).await?
    } else {
        app_state.db.get_all_tracks().await?
    };

    // Simple pagination
    let total = tracks.len();
    let paginated = tracks
        .into_iter()
        .skip(query.offset)
        .take(query.limit)
        .collect();

    Ok(Json(TracksResponse {
        tracks: paginated,
        total,
    }))
}

/// GET /api/tracks/:id
pub async fn get_track(
    Path(id): Path<String>,
    State(app_state): State<AppState>,
) -> Result<Json<Track>> {
    let track_id = TrackId::new(id);
    let track = app_state
        .db
        .get_track(track_id)
        .await?
        .ok_or_else(|| ServerError::NotFound("Track not found".to_string()))?;
    Ok(Json(track))
}

/// POST /api/tracks/import
/// Upload a track file with metadata
pub async fn import_track(
    State(app_state): State<AppState>,
    _auth: AuthenticatedUser,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> Result<Json<Track>> {
    // Parse multipart form
    let content_type = headers
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| ServerError::BadRequest("Missing Content-Type".to_string()))?;

    if !content_type.starts_with("multipart/form-data") {
        return Err(ServerError::BadRequest(
            "Expected multipart/form-data".to_string(),
        ));
    }

    let boundary = content_type
        .split("boundary=")
        .nth(1)
        .ok_or_else(|| ServerError::BadRequest("Missing boundary".to_string()))?;

    // Convert Bytes to a stream for multer
    let stream = futures_util::stream::once(async move { Ok::<_, std::io::Error>(body) });
    let mut multipart = multer::Multipart::new(stream, boundary);

    let mut file_data: Option<Vec<u8>> = None;
    let mut file_extension: Option<String> = None;
    let mut metadata_json: Option<String> = None;

    // Parse multipart fields
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ServerError::BadRequest(format!("Failed to parse multipart: {}", e)))?
    {
        let name = field.name().unwrap_or("").to_string();

        match name.as_str() {
            "file" => {
                let filename = field.file_name().unwrap_or("unknown").to_string();
                file_extension = std::path::Path::new(&filename)
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|s| s.to_string());

                file_data = Some(
                    field
                        .bytes()
                        .await
                        .map_err(|e| {
                            ServerError::BadRequest(format!("Failed to read file: {}", e))
                        })?
                        .to_vec(),
                );
            }
            "metadata" => {
                metadata_json = Some(field.text().await.map_err(|e| {
                    ServerError::BadRequest(format!("Failed to read metadata: {}", e))
                })?);
            }
            _ => {}
        }
    }

    let file_data = file_data.ok_or_else(|| ServerError::BadRequest("Missing file".to_string()))?;
    let file_extension = file_extension
        .ok_or_else(|| ServerError::BadRequest("Missing file extension".to_string()))?;
    let metadata_json =
        metadata_json.ok_or_else(|| ServerError::BadRequest("Missing metadata".to_string()))?;

    // Parse metadata
    let track: Track = serde_json::from_str(&metadata_json)
        .map_err(|e| ServerError::BadRequest(format!("Invalid metadata: {}", e)))?;

    // Store file
    app_state
        .file_storage
        .store_original(&track.id, &file_extension, &file_data)
        .await?;

    // TODO: Save to database - need to implement create_track with CreateTrack type
    // app_state.db.create_track(...).await?;

    // TODO: Queue transcoding job

    Ok(Json(track))
}

/// DELETE /api/tracks/:id
pub async fn delete_track(
    Path(id): Path<String>,
    State(app_state): State<AppState>,
    _auth: AuthenticatedUser,
) -> Result<Json<serde_json::Value>> {
    let track_id = TrackId::new(id);

    // Delete from storage
    app_state.file_storage.delete_track(&track_id).await?;

    // Delete from database
    app_state.db.delete_track(track_id).await?;

    Ok(Json(serde_json::json!({ "success": true })))
}
