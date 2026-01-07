//! Soul Server Library
//!
//! Multi-user music streaming server with authentication, file storage, and transcoding.
//!
//! This library exposes the core components for testing purposes.

pub mod api;
pub mod config;
pub mod error;
pub mod jobs;
pub mod middleware;
pub mod services;
pub mod state;

// Re-export commonly used types for convenience
pub use config::{AudioFormat, Quality, ServerConfig};
pub use error::{Result, ServerError};
pub use services::{auth::AuthService, file_storage::FileStorage, transcoding::TranscodingService};
pub use state::AppState;
