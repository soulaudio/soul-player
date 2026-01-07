/// Audio streaming API
use crate::{
    config::Quality,
    error::{Result, ServerError},
    middleware::AuthenticatedUser,
    state::AppState,
};
use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::Response,
};
use serde::Deserialize;
use soul_core::{
    storage::StorageContext,
    types::TrackId,
};
use tokio::fs::File;
use tokio_util::io::ReaderStream;

#[derive(Debug, Deserialize)]
pub struct StreamQuery {
    #[serde(default)]
    pub quality: Option<String>, // "original", "high", "medium", "low"
}

/// GET /api/stream/:track_id
/// Stream audio file with range request support
pub async fn stream_track(
    Path(track_id): Path<String>,
    State(app_state): State<AppState>,
    _auth: AuthenticatedUser,
    Query(query): Query<StreamQuery>,
    headers: HeaderMap,
) -> Result<Response> {
    let track_id = TrackId::new(track_id);

    // Verify track exists in database
    let _track = app_state
        .db
        .get_track(track_id.clone())
        .await?
        .ok_or_else(|| ServerError::NotFound("Track not found".to_string()))?;

    // Determine quality
    let quality = query
        .quality
        .as_deref()
        .and_then(parse_quality)
        .unwrap_or(Quality::High);

    // Get best available quality
    let actual_quality = app_state
        .file_storage
        .get_best_available_quality(&track_id, quality);

    // Get file path
    let file_path = app_state
        .file_storage
        .get_track_path(&track_id, actual_quality, None)?;

    // Validate path (prevent directory traversal)
    app_state.file_storage.validate_path(&file_path)?;

    // Get file metadata
    let metadata = tokio::fs::metadata(&file_path).await?;
    let file_size = metadata.len();

    // Detect MIME type
    let mime_type = mime_guess::from_path(&file_path)
        .first_or_octet_stream()
        .to_string();

    // Check for Range header
    let range_header = headers.get(header::RANGE);

    if let Some(range) = range_header {
        // Parse range header
        let range_str = range
            .to_str()
            .map_err(|_| ServerError::BadRequest("Invalid Range header".to_string()))?;

        if let Some((start, end)) = parse_range(range_str, file_size) {
            // Open file and seek to position
            let file = File::open(&file_path).await?;
            let reader = ReaderStream::new(file);

            // Create range response
            let content_length = end - start + 1;
            let body = Body::from_stream(reader);

            let response = Response::builder()
                .status(StatusCode::PARTIAL_CONTENT)
                .header(header::CONTENT_TYPE, mime_type)
                .header(header::CONTENT_LENGTH, content_length)
                .header(
                    header::CONTENT_RANGE,
                    format!("bytes {}-{}/{}", start, end, file_size),
                )
                .header(header::ACCEPT_RANGES, "bytes")
                .body(body)
                .map_err(|e| ServerError::Internal(format!("Failed to build response: {}", e)))?;

            return Ok(response);
        }
    }

    // No range request - stream entire file
    let file = File::open(&file_path).await?;
    let reader = ReaderStream::new(file);
    let body = Body::from_stream(reader);

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime_type)
        .header(header::CONTENT_LENGTH, file_size)
        .header(header::ACCEPT_RANGES, "bytes")
        .body(body)
        .map_err(|e| ServerError::Internal(format!("Failed to build response: {}", e)))?;

    Ok(response)
}

/// Parse quality string to Quality enum
fn parse_quality(s: &str) -> Option<Quality> {
    match s.to_lowercase().as_str() {
        "original" => Some(Quality::Original),
        "high" => Some(Quality::High),
        "medium" => Some(Quality::Medium),
        "low" => Some(Quality::Low),
        _ => None,
    }
}

/// Parse HTTP Range header
/// Format: "bytes=start-end"
fn parse_range(range: &str, file_size: u64) -> Option<(u64, u64)> {
    let range = range.strip_prefix("bytes=")?;

    if let Some((start_str, end_str)) = range.split_once('-') {
        let start: u64 = start_str.parse().ok()?;
        let end: u64 = if end_str.is_empty() {
            file_size - 1
        } else {
            end_str.parse().ok()?
        };

        if start <= end && end < file_size {
            return Some((start, end));
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_range() {
        assert_eq!(parse_range("bytes=0-999", 10000), Some((0, 999)));
        assert_eq!(parse_range("bytes=1000-", 10000), Some((1000, 9999)));
        assert_eq!(parse_range("bytes=0-9999", 10000), Some((0, 9999)));
        assert_eq!(parse_range("bytes=10000-", 10000), None); // Out of bounds
        assert_eq!(parse_range("invalid", 10000), None);
    }

    #[test]
    fn test_parse_quality() {
        assert_eq!(parse_quality("original"), Some(Quality::Original));
        assert_eq!(parse_quality("high"), Some(Quality::High));
        assert_eq!(parse_quality("MEDIUM"), Some(Quality::Medium));
        assert_eq!(parse_quality("invalid"), None);
    }
}
