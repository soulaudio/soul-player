use sqlx::SqlitePool;
use std::sync::Once;

static INIT: Once = Once::new();

pub async fn setup_test_db() -> SqlitePool {
    // Initialize logging once
    INIT.call_once(|| {
        let _ = tracing_subscriber::fmt()
            .with_test_writer()
            .with_max_level(tracing::Level::DEBUG)
            .try_init();
    });

    // Create in-memory database
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create test database pool");

    // Run migrations
    sqlx::migrate!("../soul-storage/migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    pool
}
