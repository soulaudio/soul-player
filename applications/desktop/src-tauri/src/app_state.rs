use sqlx::SqlitePool;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::artwork::ArtworkManager;

/// Cache key for artwork - can be either a track ID or album ID
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ArtworkCacheKey {
    Track(String),
    Album(i64),
}

/// Shared application state
pub struct AppState {
    pub pool: Arc<SqlitePool>,
    pub user_id: String,
    pub library_path: PathBuf,
    pub artwork_manager: Arc<ArtworkManager>,
    /// LRU cache for base64-encoded artwork data URLs
    /// Key: ArtworkCacheKey (track ID or album ID)
    /// Value: base64 data URL string (e.g., "data:image/jpeg;base64,...")
    pub artwork_cache: Arc<Mutex<lru::LruCache<ArtworkCacheKey, String>>>,
}

impl AppState {
    /// Create a new AppState with the given database file path
    ///
    /// This will:
    /// - Create/connect to the database
    /// - Run all migrations
    /// - Create a default user if needed
    pub async fn new(db_path: PathBuf) -> Result<Self, String> {
        tracing::debug!("Initializing database at: {}", db_path.display());

        // Ensure we have an absolute path
        let db_path = if db_path.is_relative() {
            tracing::debug!("⚠ WARNING: Database path is relative, attempting to make absolute");
            std::env::current_dir()
                .ok()
                .map(|cwd| {
                    let abs = cwd.join(&db_path);
                    tracing::debug!("Converted {} to {}", db_path.display(), abs.display());
                    abs
                })
                .unwrap_or(db_path)
        } else {
            db_path
        };

        // Ensure parent directory exists (async)
        if let Some(parent) = db_path.parent() {
            tracing::debug!("Creating directory: {}", parent.display());
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                format!(
                    "Failed to create database directory '{}': {}",
                    parent.display(),
                    e
                )
            })?;
            tracing::debug!("✓ Directory created/verified");

            // Test write permissions by creating a test file (async)
            let test_file = parent.join(".write_test");
            match tokio::fs::write(&test_file, b"test").await {
                Ok(_) => {
                    tracing::debug!("✓ Write permissions verified");
                    let _ = tokio::fs::remove_file(&test_file).await; // Clean up
                }
                Err(e) => {
                    return Err(format!(
                        "Directory exists but cannot write files: '{}': {}",
                        parent.display(),
                        e
                    ));
                }
            }
        } else {
            tracing::debug!("⚠ WARNING: No parent directory for database path");
        }

        // Convert PathBuf to SQLite connection string
        // For SQLite with sqlx, we use: sqlite://path/to/file.db
        // On Windows, convert backslashes to forward slashes for URL compatibility
        let db_url = if cfg!(windows) {
            // Windows: Convert C:\path\to\file.db -> sqlite:///C:/path/to/file.db
            let path_str = db_path
                .to_str()
                .ok_or_else(|| "Database path contains invalid UTF-8".to_string())?
                .replace('\\', "/");
            format!("sqlite:///{}", path_str)
        } else {
            // Unix: Use path as-is
            format!(
                "sqlite://{}",
                db_path
                    .to_str()
                    .ok_or_else(|| "Database path contains invalid UTF-8".to_string())?
            )
        };

        tracing::debug!("Database URL: {}", db_url);
        tracing::debug!("Database file path: {}", db_path.display());

        let pool = soul_storage::create_pool(&db_url).await.map_err(|e| {
            format!(
                "Failed to create database pool at '{}': {}",
                db_path.display(),
                e
            )
        })?;

        soul_storage::run_migrations(&pool)
            .await
            .map_err(|e| format!("Failed to run migrations: {}", e))?;

        // Create default user if not exists
        let user_id = "1";
        let user_name = "Default User";
        let now = chrono::Utc::now().timestamp();

        sqlx::query("INSERT OR IGNORE INTO users (id, name, created_at) VALUES (?, ?, ?)")
            .bind(user_id)
            .bind(user_name)
            .bind(now)
            .execute(&pool)
            .await
            .map_err(|e| format!("Failed to create default user: {}", e))?;

        tracing::debug!(
            "Database initialized successfully at: {}",
            db_path.display()
        );

        // Calculate library path
        let library_path = db_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join("library");

        tracing::debug!("Library path: {}", library_path.display());

        // Calculate artwork storage path
        let artwork_storage_path = db_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join("artwork");

        tracing::debug!("Artwork storage path: {}", artwork_storage_path.display());

        // Create artwork manager with cache for 100 images (~50-100MB)
        let artwork_manager = ArtworkManager::new(pool.clone(), 100, artwork_storage_path);

        // Create LRU cache for base64-encoded artwork (150 entries)
        // Each entry is ~50-200KB (base64 encoded), so ~7.5-30MB total
        let artwork_cache = Arc::new(Mutex::new(lru::LruCache::new(
            std::num::NonZeroUsize::new(150).unwrap(),
        )));

        Ok(Self {
            pool: Arc::new(pool),
            user_id: user_id.to_string(),
            library_path,
            artwork_manager: Arc::new(artwork_manager),
            artwork_cache,
        })
    }

    /// Create AppState from environment variable or default path
    pub async fn from_env_or_default(default_path: PathBuf) -> Result<Self, String> {
        tracing::debug!("=== Database Path Resolution ===");
        tracing::debug!("Default path provided: {}", default_path.display());

        // Try to load .env file (for development)
        match dotenvy::dotenv() {
            Ok(path) => tracing::debug!("Loaded .env from: {}", path.display()),
            Err(e) => tracing::debug!("No .env file loaded: {}", e),
        }

        // Check for custom database path in environment
        let db_path = if let Ok(custom_path) = std::env::var("DATABASE_PATH") {
            tracing::debug!("Found DATABASE_PATH in environment: {}", custom_path);
            let path = PathBuf::from(&custom_path);

            // If relative path, make it absolute relative to current exe directory
            if path.is_relative() {
                tracing::debug!("Path is relative, resolving...");
                if let Ok(exe_dir) = std::env::current_exe() {
                    tracing::debug!("Executable location: {}", exe_dir.display());
                    if let Some(parent) = exe_dir.parent() {
                        let absolute = parent.join(&path);
                        tracing::debug!(
                            "✓ Resolved relative path '{}' to: {}",
                            custom_path,
                            absolute.display()
                        );
                        absolute
                    } else {
                        tracing::debug!(
                            "⚠ Could not get parent directory of exe, using relative path as-is"
                        );
                        path
                    }
                } else {
                    tracing::debug!("⚠ Could not get exe location, using relative path as-is");
                    path
                }
            } else {
                tracing::debug!(
                    "✓ Using absolute custom database path from env: {}",
                    path.display()
                );
                path
            }
        } else {
            tracing::debug!(
                "✓ No DATABASE_PATH in environment, using default: {}",
                default_path.display()
            );
            default_path
        };

        tracing::debug!("=== Final database path: {} ===", db_path.display());
        Self::new(db_path).await
    }
}
