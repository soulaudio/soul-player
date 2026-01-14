/// Common test utilities and fixtures
use anyhow::Result;
use soul_core::types::UserId;
use soul_storage::{create_pool, run_migrations, LocalStorageContext};
use sqlx::SqlitePool;
use std::sync::Arc;

// Type alias for backwards compatibility
pub type Database = LocalStorageContext;

/// Create a test database with migrations applied
/// Creates an in-memory SQLite database with the default test user (user_id = 1)
pub async fn create_test_database() -> Result<Arc<Database>> {
    // Create in-memory pool
    let pool = create_pool("sqlite::memory:").await?;

    // Run migrations
    run_migrations(&pool).await?;

    // Create storage context with default test user
    let db = LocalStorageContext::new(pool, UserId::new("1".to_string()));

    Ok(Arc::new(db))
}

/// Get the underlying pool from the database (for direct SQL operations in tests)
pub fn get_pool(db: &Database) -> &SqlitePool {
    db.pool()
}

/// Helper function to create a user directly in the database
/// This is for testing only - production code should use proper user creation APIs
/// Returns a UserId that can be used for credential storage
pub async fn create_user_in_db(pool: &SqlitePool, username: &str) -> Result<UserId> {
    let user_id = UserId::generate();
    let created_at = chrono::Utc::now().timestamp();

    sqlx::query(
        "INSERT INTO users (id, name, created_at) VALUES (?, ?, ?)"
    )
    .bind(user_id.as_str())
    .bind(username)
    .bind(created_at)
    .execute(pool)
    .await?;

    Ok(user_id)
}

// Note: The old Storage trait API (create_user, add_track with Track::new) is deprecated.
// The new StorageContext trait uses different types (CreateTrack, CreatePlaylist, etc.)
// These tests need to be updated to use the new multi-source architecture.
//
// For now, some tests may be disabled until they can be properly rewritten for the new API.

/// Test user credentials
pub mod fixtures {
    pub const TEST_USERNAME: &str = "testuser";
    pub const TEST_PASSWORD: &str = "TestPassword123!";
    pub const TEST_PASSWORD_HASH: &str =
        "$2b$12$KIXvQWqWZ8L8wJ9vL0nLxu3QZHqK4iFr9fVjQyZvZqZ8L8wJ9vL0nL"; // bcrypt hash of "TestPassword123!"

    pub const ADMIN_USERNAME: &str = "admin";
    pub const ADMIN_PASSWORD: &str = "AdminPassword456!";
}
