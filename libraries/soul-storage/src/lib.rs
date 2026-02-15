//! Soul Player Storage
//!
//! Multi-user, multi-source `SQLite` database layer for Soul Player.
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
//! use soul_core::types::UserId;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create database connection
//! let pool = create_pool("sqlite://soul.db").await?;
//! run_migrations(&pool).await?;
//!
//! // Create storage context for default local user
//! let user_id = UserId::new("1");
//! let storage = LocalStorageContext::new(pool, user_id);
//!
//! // Get all tracks
//! let tracks = storage.get_all_tracks().await?;
//! # Ok(())
//! # }
//! ```

mod context;
mod error;

// Utilities
pub mod utils;

// Vertical slices
pub mod albums;
pub mod artists;
pub mod genres;
pub mod playlists;
pub mod sources;
pub mod tracks;
pub mod users;

// User preferences and state
pub mod external_file_settings;
pub mod managed_library_settings;
pub mod settings;
pub mod shortcuts;
pub mod window_state;

// Library management (watched folders, scanning)
pub mod library_sources;
pub mod scan_progress;

// Background processing
pub mod fingerprint_queue;

// Multi-device sync
pub mod devices;
pub mod playback_contexts;
pub mod playback_state;

// Audio analysis
pub mod loudness;

pub use context::LocalStorageContext;
pub use error::StorageError;

// Type alias for backwards compatibility with server code
pub type Database = LocalStorageContext;

use sqlx::migrate::Migrator;
use sqlx::sqlite::SqlitePool;

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

/// Create a new `SQLite` pool
///
/// # Arguments
///
/// * `database_url` - `SQLite` connection string (e.g., `<sqlite://soul.db>`)
///
/// # Errors
///
/// Returns an error if the connection fails
pub async fn create_pool(database_url: &str) -> Result<SqlitePool, sqlx::Error> {
    use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
    use std::str::FromStr;

    tracing::debug!("[soul-storage] Creating pool with URL: {}", database_url);

    // Parse the URL into options so we can configure SQLite behavior
    let options = SqliteConnectOptions::from_str(database_url)?
        .create_if_missing(true) // Create database file if it doesn't exist
        .journal_mode(SqliteJournalMode::Wal) // Use WAL mode for better concurrency
        .busy_timeout(std::time::Duration::from_secs(5)) // Wait up to 5s for locks (reduced from 30s)
        // Performance optimizations for desktop use:
        .pragma("synchronous", "NORMAL") // Reduce fsync calls (safe with WAL, ~2x faster writes)
        .pragma("cache_size", "-64000") // 64MB page cache (negative = KB, default ~2MB)
        .pragma("temp_store", "MEMORY") // Keep temp tables in RAM for faster operations
        .pragma("mmap_size", "268435456"); // Enable 256MB memory-mapped I/O for faster reads
                                           // Note: SQLite defaults to UTF-8 encoding on all modern systems

    tracing::debug!("[soul-storage] Options configured");

    // Create pool with the configured options
    // Optimized for concurrent operations (library scanning, playback init, import, etc.)
    let pool = SqlitePoolOptions::new()
        .max_connections(20) // Increased from 5 to handle concurrent background tasks
        .min_connections(0) // Set to 0 for fastest startup (lazy connection, pool scales up as needed)
        .acquire_timeout(std::time::Duration::from_secs(10)) // Timeout for pool acquisition
        .connect_with(options)
        .await?;

    tracing::debug!("[soul-storage] Pool created successfully");

    Ok(pool)
}
