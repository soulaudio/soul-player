use sqlx::SqlitePool;
use std::path::PathBuf;
use std::sync::Arc;

/// Shared application state
pub struct AppState {
    pub pool: Arc<SqlitePool>,
    pub user_id: String,
    pub library_path: PathBuf,
}

impl AppState {
    /// Create a new AppState with the given database file path
    ///
    /// This will:
    /// - Create/connect to the database
    /// - Run all migrations
    /// - Create a default user if needed
    pub async fn new(db_path: PathBuf) -> Result<Self, String> {
        eprintln!("Initializing database at: {}", db_path.display());

        // Ensure we have an absolute path
        let db_path = if db_path.is_relative() {
            eprintln!("⚠ WARNING: Database path is relative, attempting to make absolute");
            std::env::current_dir()
                .ok()
                .map(|cwd| {
                    let abs = cwd.join(&db_path);
                    eprintln!("Converted {} to {}", db_path.display(), abs.display());
                    abs
                })
                .unwrap_or(db_path)
        } else {
            db_path
        };

        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            eprintln!("Creating directory: {}", parent.display());
            std::fs::create_dir_all(parent).map_err(|e| {
                format!(
                    "Failed to create database directory '{}': {}",
                    parent.display(),
                    e
                )
            })?;
            eprintln!("✓ Directory created/verified");

            // Test write permissions by creating a test file
            let test_file = parent.join(".write_test");
            match std::fs::write(&test_file, b"test") {
                Ok(_) => {
                    eprintln!("✓ Write permissions verified");
                    let _ = std::fs::remove_file(&test_file); // Clean up
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
            eprintln!("⚠ WARNING: No parent directory for database path");
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

        eprintln!("Database URL: {}", db_url);
        eprintln!("Database file path: {}", db_path.display());

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

        eprintln!(
            "Database initialized successfully at: {}",
            db_path.display()
        );

        // Calculate library path
        let library_path = db_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join("library");

        eprintln!("Library path: {}", library_path.display());

        Ok(Self {
            pool: Arc::new(pool),
            user_id: user_id.to_string(),
            library_path,
        })
    }

    /// Create AppState from environment variable or default path
    pub async fn from_env_or_default(default_path: PathBuf) -> Result<Self, String> {
        eprintln!("=== Database Path Resolution ===");
        eprintln!("Default path provided: {}", default_path.display());

        // Try to load .env file (for development)
        match dotenvy::dotenv() {
            Ok(path) => eprintln!("Loaded .env from: {}", path.display()),
            Err(e) => eprintln!("No .env file loaded: {}", e),
        }

        // Check for custom database path in environment
        let db_path = if let Ok(custom_path) = std::env::var("DATABASE_PATH") {
            eprintln!("Found DATABASE_PATH in environment: {}", custom_path);
            let path = PathBuf::from(&custom_path);

            // If relative path, make it absolute relative to current exe directory
            if path.is_relative() {
                eprintln!("Path is relative, resolving...");
                if let Ok(exe_dir) = std::env::current_exe() {
                    eprintln!("Executable location: {}", exe_dir.display());
                    if let Some(parent) = exe_dir.parent() {
                        let absolute = parent.join(&path);
                        eprintln!(
                            "✓ Resolved relative path '{}' to: {}",
                            custom_path,
                            absolute.display()
                        );
                        absolute
                    } else {
                        eprintln!(
                            "⚠ Could not get parent directory of exe, using relative path as-is"
                        );
                        path
                    }
                } else {
                    eprintln!("⚠ Could not get exe location, using relative path as-is");
                    path
                }
            } else {
                eprintln!(
                    "✓ Using absolute custom database path from env: {}",
                    path.display()
                );
                path
            }
        } else {
            eprintln!(
                "✓ No DATABASE_PATH in environment, using default: {}",
                default_path.display()
            );
            default_path
        };

        eprintln!("=== Final database path: {} ===", db_path.display());
        Self::new(db_path).await
    }
}
