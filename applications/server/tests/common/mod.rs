/// Common test utilities and fixtures
use anyhow::Result;
use soul_storage::Database;
use std::sync::Arc;

/// Create a test database with migrations applied
/// Uses the Database::new method which handles migrations automatically
pub async fn create_test_database() -> Result<Arc<Database>> {
    // Create in-memory database for tests
    let db = Database::new(":memory:").await?;
    Ok(Arc::new(db))
}

/// Test user credentials
pub mod fixtures {
    pub const TEST_USERNAME: &str = "testuser";
    pub const TEST_PASSWORD: &str = "TestPassword123!";
    pub const TEST_PASSWORD_HASH: &str =
        "$2b$12$KIXvQWqWZ8L8wJ9vL0nLxu3QZHqK4iFr9fVjQyZvZqZ8L8wJ9vL0nL"; // bcrypt hash of "TestPassword123!"

    pub const ADMIN_USERNAME: &str = "admin";
    pub const ADMIN_PASSWORD: &str = "AdminPassword456!";
}
