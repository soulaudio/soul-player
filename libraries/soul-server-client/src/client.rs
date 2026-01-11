//! Main Soul Player Server client.

use crate::auth::AuthClient;
use crate::download::DownloadClient;
use crate::error::{Result, ServerClientError};
use crate::library::LibraryClient;
use crate::types::{LoginResponse, RefreshTokenResponse, ServerConfig, ServerInfo};
use crate::upload::UploadClient;
use reqwest::Client;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Main client for interacting with a Soul Player Server.
///
/// The client handles authentication, token refresh, and provides
/// access to library, upload, and download operations.
///
/// # Example
///
/// ```ignore
/// use soul_server_client::{SoulServerClient, ServerConfig};
///
/// // Create client
/// let config = ServerConfig::new("https://music.example.com");
/// let mut client = SoulServerClient::new(config)?;
///
/// // Test connection
/// let info = client.test_connection().await?;
/// println!("Connected to {} v{}", info.name, info.version);
///
/// // Login
/// let login = client.login("user", "password").await?;
/// println!("Logged in as {}", login.username);
///
/// // Get library
/// let library = client.library().get_full_library().await?;
/// println!("Found {} tracks", library.tracks.len());
/// ```
pub struct SoulServerClient {
    http: Client,
    config: Arc<RwLock<ServerConfig>>,
}

impl SoulServerClient {
    /// Create a new client with the given configuration.
    pub fn new(config: ServerConfig) -> Result<Self> {
        // Validate URL
        if config.url.is_empty() {
            return Err(ServerClientError::InvalidUrl("URL cannot be empty".into()));
        }

        // Parse and normalize URL
        let url = config.url.trim_end_matches('/').to_string();
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(ServerClientError::InvalidUrl(
                "URL must start with http:// or https://".into(),
            ));
        }

        let normalized_config = ServerConfig {
            url,
            access_token: config.access_token,
            refresh_token: config.refresh_token,
        };

        // Create HTTP client with reasonable defaults
        let http = Client::builder()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .user_agent(format!(
                "SoulPlayer/{} (Desktop)",
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .map_err(|e| ServerClientError::Request(e))?;

        Ok(Self {
            http,
            config: Arc::new(RwLock::new(normalized_config)),
        })
    }

    /// Get the server URL.
    pub async fn url(&self) -> String {
        self.config.read().await.url.clone()
    }

    /// Check if the client has an access token.
    pub async fn is_authenticated(&self) -> bool {
        self.config.read().await.access_token.is_some()
    }

    /// Test the connection to the server.
    ///
    /// This does not require authentication.
    pub async fn test_connection(&self) -> Result<ServerInfo> {
        let config = self.config.read().await;
        let url = format!("{}/api/info", config.url);
        drop(config);

        debug!(url = %url, "Testing server connection");

        let response = self.http.get(&url).send().await.map_err(|e| {
            if e.is_connect() || e.is_timeout() {
                ServerClientError::ServerUnreachable(e.to_string())
            } else {
                ServerClientError::Request(e)
            }
        })?;

        let status = response.status();

        if status.is_success() {
            let info: ServerInfo = response.json().await.map_err(|e| {
                ServerClientError::ParseError(format!("Failed to parse server info: {}", e))
            })?;

            info!(
                name = %info.name,
                version = %info.version,
                features = ?info.features,
                "Connected to server"
            );

            Ok(info)
        } else {
            let error_text = response.text().await.unwrap_or_default();
            Err(ServerClientError::ServerError {
                status: status.as_u16(),
                message: error_text,
            })
        }
    }

    /// Login with username and password.
    ///
    /// On success, the access token is stored for subsequent requests.
    pub async fn login(&self, username: &str, password: &str) -> Result<LoginResponse> {
        let config = self.config.read().await;
        let url = config.url.clone();
        drop(config);

        let auth_client = AuthClient::new(&self.http, &url);
        let response = auth_client.login(username, password).await?;

        // Store tokens
        let mut config = self.config.write().await;
        config.access_token = Some(response.access_token.clone());
        config.refresh_token = Some(response.refresh_token.clone());

        Ok(response)
    }

    /// Set tokens directly (e.g., from stored credentials).
    pub async fn set_tokens(&self, access_token: String, refresh_token: Option<String>) {
        let mut config = self.config.write().await;
        config.access_token = Some(access_token);
        config.refresh_token = refresh_token;
    }

    /// Get the current tokens.
    pub async fn get_tokens(&self) -> (Option<String>, Option<String>) {
        let config = self.config.read().await;
        (config.access_token.clone(), config.refresh_token.clone())
    }

    /// Clear stored tokens (logout).
    pub async fn logout(&self) {
        let mut config = self.config.write().await;
        config.access_token = None;
        config.refresh_token = None;
        info!("Logged out");
    }

    /// Refresh the access token using the refresh token.
    ///
    /// Returns the new tokens on success.
    pub async fn refresh_token(&self) -> Result<RefreshTokenResponse> {
        let config = self.config.read().await;
        let refresh_token = config
            .refresh_token
            .clone()
            .ok_or(ServerClientError::AuthRequired)?;
        let url = config.url.clone();
        drop(config);

        let auth_client = AuthClient::new(&self.http, &url);
        let response = auth_client.refresh_token(&refresh_token).await?;

        // Update stored tokens
        let mut config = self.config.write().await;
        config.access_token = Some(response.access_token.clone());
        config.refresh_token = Some(response.refresh_token.clone());

        Ok(response)
    }

    /// Validate the current access token.
    pub async fn validate_token(&self) -> Result<bool> {
        let config = self.config.read().await;
        let access_token = match &config.access_token {
            Some(t) => t.clone(),
            None => return Ok(false),
        };
        let url = config.url.clone();
        drop(config);

        let auth_client = AuthClient::new(&self.http, &url);
        auth_client.validate_token(&access_token).await
    }

    /// Get a library client for library operations.
    ///
    /// Returns an error if not authenticated.
    pub async fn library(&self) -> Result<LibraryClientHandle> {
        let config = self.config.read().await;
        let access_token = config
            .access_token
            .clone()
            .ok_or(ServerClientError::AuthRequired)?;
        let url = config.url.clone();
        drop(config);

        Ok(LibraryClientHandle {
            http: self.http.clone(),
            url,
            access_token,
        })
    }

    /// Get an upload client for uploading tracks.
    ///
    /// Returns an error if not authenticated.
    pub async fn upload(&self) -> Result<UploadClientHandle> {
        let config = self.config.read().await;
        let access_token = config
            .access_token
            .clone()
            .ok_or(ServerClientError::AuthRequired)?;
        let url = config.url.clone();
        drop(config);

        Ok(UploadClientHandle {
            http: self.http.clone(),
            url,
            access_token,
        })
    }

    /// Get a download client for downloading tracks.
    ///
    /// Returns an error if not authenticated.
    pub async fn download(&self) -> Result<DownloadClientHandle> {
        let config = self.config.read().await;
        let access_token = config
            .access_token
            .clone()
            .ok_or(ServerClientError::AuthRequired)?;
        let url = config.url.clone();
        drop(config);

        Ok(DownloadClientHandle {
            http: self.http.clone(),
            url,
            access_token,
        })
    }

    /// Execute an operation with automatic token refresh on 401.
    ///
    /// If the operation fails with `AuthRequired`, attempts to refresh
    /// the token and retry once.
    pub async fn with_auto_refresh<T, F, Fut>(&self, operation: F) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        match operation().await {
            Ok(result) => Ok(result),
            Err(ServerClientError::AuthRequired) => {
                warn!("Token expired, attempting refresh");

                // Try to refresh
                self.refresh_token().await?;

                // Retry operation
                operation().await
            }
            Err(e) => Err(e),
        }
    }
}

/// Handle for library operations.
///
/// This is returned by `SoulServerClient::library()` and provides
/// access to library-related methods.
pub struct LibraryClientHandle {
    http: Client,
    url: String,
    access_token: String,
}

impl LibraryClientHandle {
    /// Get the library client.
    pub fn client(&self) -> LibraryClient<'_> {
        LibraryClient::new(&self.http, &self.url, &self.access_token)
    }
}

// Note: We don't implement Deref because it would require unsafe lifetime extension.
// Use .client() method to get a LibraryClient with proper lifetime bounds.

/// Handle for upload operations.
pub struct UploadClientHandle {
    http: Client,
    url: String,
    access_token: String,
}

impl UploadClientHandle {
    /// Get the upload client.
    pub fn client(&self) -> UploadClient<'_> {
        UploadClient::new(&self.http, &self.url, &self.access_token)
    }
}

/// Handle for download operations.
pub struct DownloadClientHandle {
    http: Client,
    url: String,
    access_token: String,
}

impl DownloadClientHandle {
    /// Get the download client.
    pub fn client(&self) -> DownloadClient<'_> {
        DownloadClient::new(&self.http, &self.url, &self.access_token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_validation() {
        // Valid URLs
        assert!(SoulServerClient::new(ServerConfig::new("https://example.com")).is_ok());
        assert!(SoulServerClient::new(ServerConfig::new("http://localhost:8080")).is_ok());

        // Invalid URLs
        assert!(SoulServerClient::new(ServerConfig::new("")).is_err());
        assert!(SoulServerClient::new(ServerConfig::new("not-a-url")).is_err());
        assert!(SoulServerClient::new(ServerConfig::new("ftp://example.com")).is_err());
    }

    #[test]
    fn test_url_normalization() {
        let client =
            SoulServerClient::new(ServerConfig::new("https://example.com/")).expect("valid url");

        // URL should have trailing slash removed
        let url = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(client.url());
        assert_eq!(url, "https://example.com");
    }
}
