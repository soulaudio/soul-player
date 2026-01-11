//! Library sync operations for Soul Player Server.

use crate::error::{Result, ServerClientError};
use crate::types::{LibraryResponse, ServerTrack, StreamUrlResponse, SyncDelta};
use reqwest::Client;
use tracing::debug;

/// Library client for Soul Player Server.
pub struct LibraryClient<'a> {
    http: &'a Client,
    base_url: &'a str,
    access_token: &'a str,
}

impl<'a> LibraryClient<'a> {
    pub(crate) fn new(http: &'a Client, base_url: &'a str, access_token: &'a str) -> Self {
        Self {
            http,
            base_url,
            access_token,
        }
    }

    /// Get the full library (for initial sync).
    pub async fn get_full_library(&self) -> Result<LibraryResponse> {
        let url = format!("{}/api/library", self.base_url);
        debug!(url = %url, "Fetching full library");

        let response = self
            .http
            .get(&url)
            .bearer_auth(self.access_token)
            .send()
            .await?;

        let status = response.status();

        if status.is_success() {
            let library: LibraryResponse = response.json().await.map_err(|e| {
                ServerClientError::ParseError(format!("Failed to parse library response: {}", e))
            })?;

            debug!(
                tracks = library.tracks.len(),
                albums = library.albums.len(),
                artists = library.artists.len(),
                "Fetched full library"
            );

            Ok(library)
        } else if status.as_u16() == 401 {
            Err(ServerClientError::AuthRequired)
        } else {
            let error_text = response.text().await.unwrap_or_default();
            Err(ServerClientError::ServerError {
                status: status.as_u16(),
                message: error_text,
            })
        }
    }

    /// Get library changes since last sync (delta sync).
    ///
    /// # Arguments
    /// * `since_timestamp` - Unix timestamp of last sync (None for first sync)
    /// * `sync_token` - Token from previous sync response (None for first sync)
    pub async fn get_library_delta(
        &self,
        since_timestamp: Option<i64>,
        sync_token: Option<&str>,
    ) -> Result<SyncDelta> {
        let mut url = format!("{}/api/library/delta", self.base_url);

        let mut params = Vec::new();
        if let Some(since) = since_timestamp {
            params.push(format!("since={}", since));
        }
        if let Some(token) = sync_token {
            params.push(format!("token={}", token));
        }

        if !params.is_empty() {
            url = format!("{}?{}", url, params.join("&"));
        }

        debug!(url = %url, "Fetching library delta");

        let response = self
            .http
            .get(&url)
            .bearer_auth(self.access_token)
            .send()
            .await?;

        let status = response.status();

        if status.is_success() {
            let delta: SyncDelta = response.json().await.map_err(|e| {
                ServerClientError::ParseError(format!("Failed to parse delta response: {}", e))
            })?;

            debug!(
                new_tracks = delta.new_tracks.len(),
                updated_tracks = delta.updated_tracks.len(),
                deleted_tracks = delta.deleted_track_ids.len(),
                "Fetched library delta"
            );

            Ok(delta)
        } else if status.as_u16() == 401 {
            Err(ServerClientError::AuthRequired)
        } else {
            let error_text = response.text().await.unwrap_or_default();
            Err(ServerClientError::ServerError {
                status: status.as_u16(),
                message: error_text,
            })
        }
    }

    /// Get a single track by ID.
    pub async fn get_track(&self, track_id: &str) -> Result<ServerTrack> {
        let url = format!("{}/api/library/tracks/{}", self.base_url, track_id);
        debug!(url = %url, track_id = %track_id, "Fetching track");

        let response = self
            .http
            .get(&url)
            .bearer_auth(self.access_token)
            .send()
            .await?;

        let status = response.status();

        if status.is_success() {
            let track: ServerTrack = response.json().await.map_err(|e| {
                ServerClientError::ParseError(format!("Failed to parse track response: {}", e))
            })?;

            Ok(track)
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

    /// Get a streaming URL for a track.
    ///
    /// The URL is time-limited and should be used promptly.
    pub async fn get_stream_url(&self, track_id: &str) -> Result<StreamUrlResponse> {
        let url = format!("{}/api/library/tracks/{}/stream", self.base_url, track_id);
        debug!(url = %url, track_id = %track_id, "Getting stream URL");

        let response = self
            .http
            .get(&url)
            .bearer_auth(self.access_token)
            .send()
            .await?;

        let status = response.status();

        if status.is_success() {
            let stream_info: StreamUrlResponse = response.json().await.map_err(|e| {
                ServerClientError::ParseError(format!("Failed to parse stream response: {}", e))
            })?;

            Ok(stream_info)
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

    /// Delete a track from the server.
    pub async fn delete_track(&self, track_id: &str) -> Result<()> {
        let url = format!("{}/api/library/tracks/{}", self.base_url, track_id);
        debug!(url = %url, track_id = %track_id, "Deleting track");

        let response = self
            .http
            .delete(&url)
            .bearer_auth(self.access_token)
            .send()
            .await?;

        let status = response.status();

        if status.is_success() || status.as_u16() == 204 {
            debug!(track_id = %track_id, "Track deleted");
            Ok(())
        } else if status.as_u16() == 401 {
            Err(ServerClientError::AuthRequired)
        } else if status.as_u16() == 404 {
            // Already deleted, that's fine
            Ok(())
        } else {
            let error_text = response.text().await.unwrap_or_default();
            Err(ServerClientError::ServerError {
                status: status.as_u16(),
                message: error_text,
            })
        }
    }

    /// Search tracks on the server.
    pub async fn search_tracks(&self, query: &str, limit: Option<u32>) -> Result<Vec<ServerTrack>> {
        let mut url = format!(
            "{}/api/library/tracks/search?q={}",
            self.base_url,
            urlencoding::encode(query)
        );

        if let Some(limit) = limit {
            url = format!("{}&limit={}", url, limit);
        }

        debug!(url = %url, query = %query, "Searching tracks");

        let response = self
            .http
            .get(&url)
            .bearer_auth(self.access_token)
            .send()
            .await?;

        let status = response.status();

        if status.is_success() {
            let tracks: Vec<ServerTrack> = response.json().await.map_err(|e| {
                ServerClientError::ParseError(format!("Failed to parse search response: {}", e))
            })?;

            debug!(results = tracks.len(), "Search complete");
            Ok(tracks)
        } else if status.as_u16() == 401 {
            Err(ServerClientError::AuthRequired)
        } else {
            let error_text = response.text().await.unwrap_or_default();
            Err(ServerClientError::ServerError {
                status: status.as_u16(),
                message: error_text,
            })
        }
    }
}

// URL encoding helper
mod urlencoding {
    pub fn encode(s: &str) -> String {
        url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
    }
}

#[cfg(test)]
mod tests {
    // Tests would go here with mocked HTTP responses
}
