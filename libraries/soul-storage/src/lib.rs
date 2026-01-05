//! Soul Player Storage
//!
//! Multi-user, multi-source SQLite database layer for Soul Player.
//!
//! This crate provides persistent storage for tracks, playlists, and users
//! with support for multiple sources (local files + remote servers).
//!
//! # Architecture
//!
//! - **Multi-Source**: Tracks can exist across multiple sources (local + servers)
//! - **Multi-User**: All data supports multiple users from day one
//! - **Vertical Slicing**: Each feature owns its own queries and logic
//! - **Offline-First**: Queue operations when offline, sync when connected
//!
//! # Example
//!
//! ```rust,no_run
//! use soul_storage::{LocalStorageContext, create_pool, run_migrations};
//! use soul_core::storage::StorageContext;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create database connection
//! let pool = create_pool("sqlite://soul.db").await?;
//! run_migrations(&pool).await?;
//!
//! // Create storage context for user 1 (default local user)
//! let storage = LocalStorageContext::new(pool, 1);
//!
//! // Get all tracks
//! let tracks = storage.get_all_tracks().await?;
//! # Ok(())
//! # }
//! ```

mod context;
mod error;

// Vertical slices
pub mod tracks;
pub mod artists;
pub mod albums;
pub mod playlists;
pub mod sources;

pub use context::LocalStorageContext;
pub use error::StorageError;

use sqlx::sqlite::SqlitePool;
use sqlx::migrate::Migrator;

// Embed migrations into binary
static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

/// Run database migrations
///
/// This should be called once when the application starts to ensure
/// the database schema is up to date.
///
/// # Errors
///
/// Returns an error if migrations fail to run
pub async fn run_migrations(pool: &SqlitePool) -> Result<(), sqlx::migrate::MigrateError> {
    MIGRATOR.run(pool).await
}

/// Create a new SQLite pool
///
/// # Arguments
///
/// * `database_url` - SQLite connection string (e.g., "sqlite://soul.db")
///
/// # Errors
///
/// Returns an error if the connection fails
pub async fn create_pool(database_url: &str) -> Result<SqlitePool, sqlx::Error> {
    SqlitePool::connect(database_url).await
}
