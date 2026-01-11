//! Track upload operations for Soul Player Server.

use crate::error::{Result, ServerClientError};
use crate::types::{UploadMetadata, UploadProgress, UploadResponse};
use reqwest::multipart::{Form, Part};
use reqwest::Client;
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tracing::{debug, info};

/// Upload client for Soul Player Server.
pub struct UploadClient<'a> {
    http: &'a Client,
    base_url: &'a str,
    access_token: &'a str,
}

impl<'a> UploadClient<'a> {
    pub(crate) fn new(http: &'a Client, base_url: &'a str, access_token: &'a str) -> Self {
        Self {
            http,
            base_url,
            access_token,
        }
    }

    /// Upload a single track file.
    ///
    /// # Arguments
    /// * `file_path` - Path to the audio file
    /// * `metadata` - Optional metadata to associate with the track
    ///
    /// # Returns
    /// The uploaded track info, or error if upload fails.
    pub async fn upload_track(
        &self,
        file_path: &Path,
        metadata: Option<&UploadMetadata>,
    ) -> Result<UploadResponse> {
        if !file_path.exists() {
            return Err(ServerClientError::FileNotFound(
                file_path.display().to_string(),
            ));
        }

        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("track")
            .to_string();

        debug!(file = %file_path.display(), "Uploading track");

        // Read file contents
        let mut file = File::open(file_path).await?;
        let mut contents = Vec::new();
        file.read_to_end(&mut contents).await?;

        let file_size = contents.len();

        // Create multipart form
        let file_part = Part::bytes(contents)
            .file_name(file_name.clone())
            .mime_str(mime_type_for_file(file_path))?;

        let mut form = Form::new().part("file", file_part);

        // Add metadata if provided
        if let Some(meta) = metadata {
            let meta_json = serde_json::to_string(meta)
                .map_err(|e| ServerClientError::ParseError(e.to_string()))?;
            form = form.text("metadata", meta_json);
        }

        let url = format!("{}/api/library/tracks", self.base_url);

        let response = self
            .http
            .post(&url)
            .bearer_auth(self.access_token)
            .multipart(form)
            .send()
            .await?;

        let status = response.status();

        if status.is_success() {
            let upload_response: UploadResponse = response.json().await.map_err(|e| {
                ServerClientError::ParseError(format!("Failed to parse upload response: {}", e))
            })?;

            info!(
                track_id = %upload_response.track.id,
                file = %file_name,
                size = file_size,
                already_existed = upload_response.already_existed,
                "Track uploaded"
            );

            Ok(upload_response)
        } else if status.as_u16() == 401 {
            Err(ServerClientError::AuthRequired)
        } else if status.as_u16() == 413 {
            Err(ServerClientError::ServerError {
                status: 413,
                message: "File too large".to_string(),
            })
        } else if status.as_u16() == 429 {
            // Rate limited
            let retry_after = response
                .headers()
                .get("Retry-After")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse().ok())
                .unwrap_or(60);

            Err(ServerClientError::RateLimited {
                retry_after_secs: retry_after,
            })
        } else {
            let error_text = response.text().await.unwrap_or_default();
            Err(ServerClientError::ServerError {
                status: status.as_u16(),
                message: error_text,
            })
        }
    }

    /// Upload multiple tracks with progress reporting.
    ///
    /// # Arguments
    /// * `files` - List of (file_path, optional metadata) tuples
    /// * `progress_callback` - Called for each file as it's uploaded
    ///
    /// # Returns
    /// Vec of results for each file (success or error).
    pub async fn upload_tracks_batch<F>(
        &self,
        files: Vec<(std::path::PathBuf, Option<UploadMetadata>)>,
        mut progress_callback: F,
    ) -> Vec<Result<UploadResponse>>
    where
        F: FnMut(UploadProgress),
    {
        let total_tracks = files.len();
        let mut results = Vec::with_capacity(total_tracks);

        for (index, (file_path, metadata)) in files.into_iter().enumerate() {
            let file_name = file_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            // Report progress before starting
            progress_callback(UploadProgress {
                track_index: index,
                total_tracks,
                current_file: file_name.clone(),
                bytes_sent: 0,
                bytes_total: file_path.metadata().map(|m| m.len()).unwrap_or(0),
            });

            let result = self.upload_track(&file_path, metadata.as_ref()).await;

            // Report completion
            let bytes_total = file_path.metadata().map(|m| m.len()).unwrap_or(0);
            progress_callback(UploadProgress {
                track_index: index,
                total_tracks,
                current_file: file_name,
                bytes_sent: bytes_total,
                bytes_total,
            });

            results.push(result);
        }

        results
    }
}

/// Get MIME type for audio file.
fn mime_type_for_file(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("mp3") => "audio/mpeg",
        Some("flac") => "audio/flac",
        Some("ogg") => "audio/ogg",
        Some("opus") => "audio/opus",
        Some("wav") => "audio/wav",
        Some("m4a") | Some("aac") => "audio/mp4",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mime_types() {
        assert_eq!(mime_type_for_file(Path::new("song.mp3")), "audio/mpeg");
        assert_eq!(mime_type_for_file(Path::new("song.flac")), "audio/flac");
        assert_eq!(mime_type_for_file(Path::new("song.ogg")), "audio/ogg");
        assert_eq!(mime_type_for_file(Path::new("song.m4a")), "audio/mp4");
        assert_eq!(
            mime_type_for_file(Path::new("song.unknown")),
            "application/octet-stream"
        );
    }
}
