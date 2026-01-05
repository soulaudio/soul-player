//! Integration tests for sources vertical slice
//!
//! Tests the multi-source functionality including:
//! - Creating local and server sources
//! - Activating/deactivating servers
//! - Online/offline status tracking
//! - Constraint enforcement (only one active server)

mod test_helpers;

use test_helpers::*;
use soul_core::types::*;

#[tokio::test]
async fn test_get_all_sources_includes_default_local() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    // Default local source should exist (id=1 from migration)
    let sources = soul_storage::sources::get_all(pool)
        .await
        .expect("Failed to get sources");

    assert!(!sources.is_empty(), "Should have at least default local source");
    assert_eq!(sources[0].id, 1);
    assert_eq!(sources[0].source_type, SourceType::Local);
    assert_eq!(sources[0].name, "Local Files");
    assert!(sources[0].is_active);
    assert!(sources[0].is_online);
}

#[tokio::test]
async fn test_create_local_source() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let source = soul_storage::sources::create(
        pool,
        CreateSource {
            name: "External Drive".to_string(),
            config: SourceConfig::Local,
        },
    )
    .await
    .expect("Failed to create local source");

    assert_eq!(source.name, "External Drive");
    assert_eq!(source.source_type, SourceType::Local);
    assert_eq!(source.config, SourceConfig::Local);
    assert!(!source.is_active); // New sources start inactive
    assert!(source.is_online);
}

#[tokio::test]
async fn test_create_server_source() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let source = soul_storage::sources::create(
        pool,
        CreateSource {
            name: "Home Server".to_string(),
            config: SourceConfig::Server {
                url: "https://home.example.com".to_string(),
                username: "user".to_string(),
                token: Some("test-token".to_string()),
            },
        },
    )
    .await
    .expect("Failed to create server source");

    assert_eq!(source.name, "Home Server");
    assert_eq!(source.source_type, SourceType::Server);

    match source.config {
        SourceConfig::Server { url, username, token } => {
            assert_eq!(url, "https://home.example.com");
            assert_eq!(username, "user");
            assert_eq!(token, Some("test-token".to_string()));
        }
        _ => panic!("Expected Server config"),
    }
}

#[tokio::test]
async fn test_get_source_by_id() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    // Get default local source
    let source = soul_storage::sources::get_by_id(pool, 1)
        .await
        .expect("Failed to get source")
        .expect("Source not found");

    assert_eq!(source.id, 1);
    assert_eq!(source.source_type, SourceType::Local);
}

#[tokio::test]
async fn test_get_nonexistent_source_returns_none() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let result = soul_storage::sources::get_by_id(pool, 9999)
        .await
        .expect("Query should succeed");

    assert!(result.is_none());
}

#[tokio::test]
async fn test_set_active_server() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    // Create a server source
    let server = soul_storage::sources::create(
        pool,
        CreateSource {
            name: "Server 1".to_string(),
            config: SourceConfig::Server {
                url: "https://server1.example.com".to_string(),
                username: "user".to_string(),
                token: None,
            },
        },
    )
    .await
    .expect("Failed to create server");

    // Activate it
    soul_storage::sources::set_active(pool, server.id)
        .await
        .expect("Failed to set active server");

    // Verify it's active
    let active = soul_storage::sources::get_active_server(pool)
        .await
        .expect("Failed to get active server")
        .expect("No active server found");

    assert_eq!(active.id, server.id);
    assert!(active.is_active);
}

#[tokio::test]
async fn test_only_one_active_server_at_a_time() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    // Create two servers
    let server1 = soul_storage::sources::create(
        pool,
        CreateSource {
            name: "Server 1".to_string(),
            config: SourceConfig::Server {
                url: "https://server1.example.com".to_string(),
                username: "user1".to_string(),
                token: None,
            },
        },
    )
    .await
    .unwrap();

    let server2 = soul_storage::sources::create(
        pool,
        CreateSource {
            name: "Server 2".to_string(),
            config: SourceConfig::Server {
                url: "https://server2.example.com".to_string(),
                username: "user2".to_string(),
                token: None,
            },
        },
    )
    .await
    .unwrap();

    // Activate server 1
    soul_storage::sources::set_active(pool, server1.id)
        .await
        .unwrap();

    // Activate server 2 (should deactivate server 1)
    soul_storage::sources::set_active(pool, server2.id)
        .await
        .unwrap();

    // Verify only server 2 is active
    let active = soul_storage::sources::get_active_server(pool)
        .await
        .unwrap()
        .expect("No active server");

    assert_eq!(active.id, server2.id);

    // Verify server 1 is not active
    let server1_updated = soul_storage::sources::get_by_id(pool, server1.id)
        .await
        .unwrap()
        .unwrap();

    assert!(!server1_updated.is_active);
}

#[tokio::test]
async fn test_get_active_server_returns_none_when_no_server_active() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let result = soul_storage::sources::get_active_server(pool)
        .await
        .expect("Query should succeed");

    assert!(result.is_none());
}

#[tokio::test]
async fn test_update_source_status() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let server = soul_storage::sources::create(
        pool,
        CreateSource {
            name: "Test Server".to_string(),
            config: SourceConfig::Server {
                url: "https://test.example.com".to_string(),
                username: "user".to_string(),
                token: None,
            },
        },
    )
    .await
    .unwrap();

    // Mark as offline
    soul_storage::sources::update_status(pool, server.id, false)
        .await
        .expect("Failed to update status");

    // Verify status changed
    let updated = soul_storage::sources::get_by_id(pool, server.id)
        .await
        .unwrap()
        .unwrap();

    assert!(!updated.is_online);

    // Mark back online
    soul_storage::sources::update_status(pool, server.id, true)
        .await
        .unwrap();

    let updated = soul_storage::sources::get_by_id(pool, server.id)
        .await
        .unwrap()
        .unwrap();

    assert!(updated.is_online);
}

#[tokio::test]
async fn test_local_source_cannot_be_set_active() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    // Try to set local source (id=1) as active
    soul_storage::sources::set_active(pool, 1)
        .await
        .unwrap();

    // Should have no effect (local sources are always active but not in the "active server" sense)
    let active = soul_storage::sources::get_active_server(pool)
        .await
        .unwrap();

    assert!(active.is_none(), "Local sources should not appear as active servers");
}
