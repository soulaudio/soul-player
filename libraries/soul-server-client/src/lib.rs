//! Soul Player Server Client
//!
//! HTTP client library for interacting with Soul Player Server API.
//!
//! # Features
//!
//! - **Authentication**: Login with username/password, token refresh
//! - **Library sync**: Fetch tracks, delta sync, search
//! - **Upload**: Upload tracks with progress reporting
//! - **Download**: Download tracks with progress reporting
//!
//! # Example
//!
//! ```ignore
//! use soul_server_client::{SoulServerClient, ServerConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create client
//!     let config = ServerConfig::new("https://music.example.com");
//!     let client = SoulServerClient::new(config)?;
//!
//!     // Test connection
//!     let info = client.test_connection().await?;
//!     println!("Connected to {} v{}", info.name, info.version);
//!
//!     // Login
//!     let login = client.login("user", "password").await?;
//!     println!("Logged in as {}", login.username);
//!
//!     // Get library
//!     let library_client = client.library().await?;
//!     let library = library_client.client().get_full_library().await?;
//!     println!("Found {} tracks", library.tracks.len());
//!
//!     Ok(())
//! }
//! ```

mod auth;
mod client;
mod download;
mod error;
mod library;
mod types;
mod upload;

// Re-export main types
pub use client::{DownloadClientHandle, LibraryClientHandle, SoulServerClient, UploadClientHandle};
pub use error::{Result, ServerClientError};
pub use types::{
    DownloadProgress, LibraryResponse, LoginResponse, RefreshTokenResponse, ServerConfig,
    ServerInfo, ServerTrack, StreamUrlResponse, SyncDelta, UploadMetadata, UploadProgress,
    UploadResponse, UserInfo,
};

// Re-export sub-clients for direct use if needed
pub use auth::AuthClient;
pub use download::DownloadClient;
pub use library::LibraryClient;
pub use upload::UploadClient;
