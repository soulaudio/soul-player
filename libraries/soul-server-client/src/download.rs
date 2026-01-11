//! Track download operations for Soul Player Server.

use crate::error::{Result, ServerClientError};
use crate::types::DownloadProgress;
use futures_util::StreamExt;
use reqwest::Client;
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tracing::{debug, info};

/// Download client for Soul Player Server.
pub struct DownloadClient<'a> {
    http: &'a Client,
    base_url: &'a str,
    access_token: &'a str,
}

impl<'a> DownloadClient<'a> {
    pub(crate) fn new(http: &'a Client, base_url: &'a str, access_token: &'a str) -> Self {
        Self {
            http,
            base_url,
            access_token,
        }
    }

    /// Download a track file.
    ///
    /// # Arguments
    /// * `track_id` - The server track ID
    /// * `dest_path` - Where to save the file
    /// * `progress_callback` - Called periodically with download progress
    ///
    /// # Returns
    /// Ok(()) on success, error otherwise.
    pub async fn download_track<F>(
        &self,
        track_id: &str,
        dest_path: &Path,
        mut progress_callback: F,
    ) -> Result<()>
    where
        F: FnMut(DownloadProgress),
    {
        let url = format!("{}/api/library/tracks/{}/download", self.base_url, track_id);
        debug!(url = %url, track_id = %track_id, dest = %dest_path.display(), "Downloading track");

        let response = self
            .http
            .get(&url)
            .bearer_auth(self.access_token)
            .send()
            .await?;

        let status = response.status();

        if !status.is_success() {
            if status.as_u16() == 401 {
                return Err(ServerClientError::AuthRequired);
            } else if status.as_u16() == 404 {
                return Err(ServerClientError::ServerError {
                    status: 404,
                    message: format!("Track not found: {}", track_id),
                });
            } else {
                let error_text = response.text().await.unwrap_or_default();
                return Err(ServerClientError::ServerError {
                    status: status.as_u16(),
                    message: error_text,
                });
            }
        }

        // Get content length if available
        let total_size = response.content_length();

        // Create parent directories if needed
        if let Some(parent) = dest_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Create file
        let mut file = File::create(dest_path).await?;
        let mut downloaded: u64 = 0;

        // Stream the response body
        let mut stream = response.bytes_stream();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;
            file.write_all(&chunk).await?;
            downloaded += chunk.len() as u64;

            // Report progress
            let progress = total_size
                .map(|total| downloaded as f32 / total as f32)
                .unwrap_or(0.0);

            progress_callback(DownloadProgress {
                track_id: track_id.to_string(),
                bytes_received: downloaded,
                bytes_total: total_size,
                progress,
            });
        }

        file.flush().await?;

        info!(
            track_id = %track_id,
            dest = %dest_path.display(),
            size = downloaded,
            "Track downloaded"
        );

        Ok(())
    }

    /// Download multiple tracks.
    ///
    /// # Arguments
    /// * `downloads` - List of (track_id, dest_path) tuples
    /// * `track_progress` - Called for each chunk of each track
    /// * `overall_progress` - Called when a track completes (index, total)
    ///
    /// # Returns
    /// Vec of results for each download.
    pub async fn download_tracks_batch<F, G>(
        &self,
        downloads: Vec<(String, std::path::PathBuf)>,
        mut track_progress: F,
        mut overall_progress: G,
    ) -> Vec<Result<()>>
    where
        F: FnMut(DownloadProgress),
        G: FnMut(usize, usize),
    {
        let total = downloads.len();
        let mut results = Vec::with_capacity(total);

        for (index, (track_id, dest_path)) in downloads.into_iter().enumerate() {
            let result = self
                .download_track(&track_id, &dest_path, &mut track_progress)
                .await;

            overall_progress(index + 1, total);
            results.push(result);
        }

        results
    }

    /// Get the direct download URL for a track.
    ///
    /// This returns a signed URL that can be used for direct download
    /// without passing through the client.
    pub async fn get_download_url(&self, track_id: &str) -> Result<String> {
        let url = format!(
            "{}/api/library/tracks/{}/download-url",
            self.base_url, track_id
        );

        let response = self
            .http
            .get(&url)
            .bearer_auth(self.access_token)
            .send()
            .await?;

        let status = response.status();

        if status.is_success() {
            #[derive(serde::Deserialize)]
            struct UrlResponse {
                url: String,
            }

            let url_response: UrlResponse = response.json().await.map_err(|e| {
                ServerClientError::ParseError(format!("Failed to parse URL response: {}", e))
            })?;

            Ok(url_response.url)
        } else if status.as_u16() == 401 {
            Err(ServerClientError::AuthRequired)
        } else if status.as_u16() == 404 {
            Err(ServerClientError::ServerError {
                status: 404,
                message: format!("Track not found: {}", track_id),
            })
        } else {
            let error_text = response.text().await.unwrap_or_default();
            Err(ServerClientError::ServerError {
                status: status.as_u16(),
                message: error_text,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    // Tests would go here with mocked HTTP responses
}
