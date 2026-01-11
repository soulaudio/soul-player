//! Integration tests for sources vertical slice
//!
//! Tests the multi-source functionality including:
//! - Creating local and server sources
//! - Activating/deactivating servers
//! - Online/offline status tracking
//! - Constraint enforcement (only one active server)

mod test_helpers;

use soul_core::types::*;
use test_helpers::*;

#[tokio::test]
async fn test_get_all_sources_includes_default_local() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    // Default local source should exist (id=1 from migration)
    let sources = soul_storage::sources::get_all(pool)
        .await
        .expect("Failed to get sources");

    assert!(
        !sources.is_empty(),
        "Should have at least default local source"
    );
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
        SourceConfig::Server {
            url,
            username,
            token,
        } => {
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
    soul_storage::sources::set_active(pool, 1).await.unwrap();

    // Should have no effect (local sources are always active but not in the "active server" sense)
    let active = soul_storage::sources::get_active_server(pool)
        .await
        .unwrap();

    assert!(
        active.is_none(),
        "Local sources should not appear as active servers"
    );
}

// =============================================================================
// Delete Source Tests
// =============================================================================

#[tokio::test]
async fn test_delete_source() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    // Create a server source
    let server = soul_storage::sources::create(
        pool,
        CreateSource {
            name: "Deletable Server".to_string(),
            config: SourceConfig::Server {
                url: "https://delete.example.com".to_string(),
                username: "user".to_string(),
                token: None,
            },
        },
    )
    .await
    .unwrap();

    // Verify it exists
    let exists = soul_storage::sources::get_by_id(pool, server.id)
        .await
        .unwrap();
    assert!(exists.is_some());

    // Delete it
    soul_storage::sources::delete(pool, server.id)
        .await
        .expect("Failed to delete source");

    // Verify it's gone
    let deleted = soul_storage::sources::get_by_id(pool, server.id)
        .await
        .unwrap();
    assert!(deleted.is_none());
}

#[tokio::test]
async fn test_delete_nonexistent_source_succeeds() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    // Delete a source that doesn't exist - should not error
    let result = soul_storage::sources::delete(pool, 99999).await;
    assert!(result.is_ok());
}

// =============================================================================
// Server Credentials Tests
// =============================================================================

#[tokio::test]
async fn test_update_server_credentials() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    // Create a server source
    let server = soul_storage::sources::create(
        pool,
        CreateSource {
            name: "Credentials Test".to_string(),
            config: SourceConfig::Server {
                url: "https://creds.example.com".to_string(),
                username: "old_user".to_string(),
                token: None,
            },
        },
    )
    .await
    .unwrap();

    // Update credentials
    soul_storage::sources::update_server_credentials(
        pool,
        server.id,
        "new_user",
        Some("new_token_abc"),
    )
    .await
    .expect("Failed to update credentials");

    // Verify update
    let updated = soul_storage::sources::get_by_id(pool, server.id)
        .await
        .unwrap()
        .unwrap();

    match updated.config {
        SourceConfig::Server {
            username, token, ..
        } => {
            assert_eq!(username, "new_user");
            assert_eq!(token, Some("new_token_abc".to_string()));
        }
        _ => panic!("Expected Server config"),
    }
}

#[tokio::test]
async fn test_update_server_credentials_clear_token() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    // Create a server source with token
    let server = soul_storage::sources::create(
        pool,
        CreateSource {
            name: "Token Clear Test".to_string(),
            config: SourceConfig::Server {
                url: "https://clear.example.com".to_string(),
                username: "user".to_string(),
                token: Some("existing_token".to_string()),
            },
        },
    )
    .await
    .unwrap();

    // Update to clear token
    soul_storage::sources::update_server_credentials(pool, server.id, "user", None)
        .await
        .expect("Failed to clear token");

    // Verify token is cleared
    let updated = soul_storage::sources::get_by_id(pool, server.id)
        .await
        .unwrap()
        .unwrap();

    match updated.config {
        SourceConfig::Server { token, .. } => {
            assert!(token.is_none());
        }
        _ => panic!("Expected Server config"),
    }
}

// =============================================================================
// Last Sync Timestamp Tests
// =============================================================================

#[tokio::test]
async fn test_update_last_sync() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    // Create a server source
    let server = soul_storage::sources::create(
        pool,
        CreateSource {
            name: "Sync Timestamp Test".to_string(),
            config: SourceConfig::Server {
                url: "https://sync.example.com".to_string(),
                username: "user".to_string(),
                token: None,
            },
        },
    )
    .await
    .unwrap();

    // Initially should have no last_sync_at
    assert!(server.last_sync_at.is_none());

    // Update last sync
    soul_storage::sources::update_last_sync(pool, server.id)
        .await
        .expect("Failed to update last sync");

    // Verify last_sync_at is now set
    let updated = soul_storage::sources::get_by_id(pool, server.id)
        .await
        .unwrap()
        .unwrap();

    assert!(
        updated.last_sync_at.is_some(),
        "last_sync_at should be set after update"
    );
}

// =============================================================================
// User-Specific Server Sources Tests
// =============================================================================

#[tokio::test]
async fn test_add_server_source_for_user() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    // Add a server source for user
    let source = soul_storage::sources::add_server_source(
        pool,
        1, // user_id
        "User's Server",
        "https://user.server.example.com",
    )
    .await
    .expect("Failed to add server source");

    assert_eq!(source.name, "User's Server");
    assert_eq!(source.source_type, SourceType::Server);
    assert!(!source.is_active);
    assert!(!source.is_online); // New sources start offline

    match source.config {
        SourceConfig::Server { url, .. } => {
            assert_eq!(url, "https://user.server.example.com");
        }
        _ => panic!("Expected Server config"),
    }
}

#[tokio::test]
async fn test_get_server_sources_for_user() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    // Add multiple server sources
    soul_storage::sources::add_server_source(pool, 1, "Server A", "https://a.example.com")
        .await
        .unwrap();
    soul_storage::sources::add_server_source(pool, 1, "Server B", "https://b.example.com")
        .await
        .unwrap();
    soul_storage::sources::add_server_source(
        pool,
        2,
        "Other User Server",
        "https://other.example.com",
    )
    .await
    .unwrap();

    // Get sources for user 1
    let sources = soul_storage::sources::get_server_sources_for_user(pool, 1)
        .await
        .expect("Failed to get sources");

    // Should include user 1's servers but not user 2's
    assert_eq!(sources.len(), 2);
    assert!(sources.iter().any(|s| s.name == "Server A"));
    assert!(sources.iter().any(|s| s.name == "Server B"));
    assert!(!sources.iter().any(|s| s.name == "Other User Server"));
}

// =============================================================================
// Auth Token Management Tests
// =============================================================================

#[tokio::test]
async fn test_store_and_get_auth_token() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    // Create a server source
    let source =
        soul_storage::sources::add_server_source(pool, 1, "Auth Test", "https://auth.example.com")
            .await
            .unwrap();

    // Store auth token
    let expires_at = chrono::Utc::now().timestamp() + 3600; // 1 hour from now
    soul_storage::sources::store_auth_token(
        pool,
        source.id,
        "access_token_xyz",
        Some("refresh_token_abc"),
        Some(expires_at),
    )
    .await
    .expect("Failed to store auth token");

    // Retrieve auth token
    let token = soul_storage::sources::get_auth_token(pool, source.id)
        .await
        .expect("Failed to get auth token")
        .expect("Auth token not found");

    assert_eq!(token.source_id, source.id);
    assert_eq!(token.access_token, "access_token_xyz");
    assert_eq!(token.refresh_token, Some("refresh_token_abc".to_string()));
    assert_eq!(token.expires_at, Some(expires_at));
}

#[tokio::test]
async fn test_store_auth_token_upsert() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let source = soul_storage::sources::add_server_source(
        pool,
        1,
        "Upsert Test",
        "https://upsert.example.com",
    )
    .await
    .unwrap();

    // Store initial token
    soul_storage::sources::store_auth_token(pool, source.id, "initial_token", None, None)
        .await
        .unwrap();

    // Update with new token (should upsert, not insert duplicate)
    soul_storage::sources::store_auth_token(
        pool,
        source.id,
        "updated_token",
        Some("new_refresh"),
        Some(12345),
    )
    .await
    .expect("Upsert should succeed");

    // Verify only one token exists with updated values
    let token = soul_storage::sources::get_auth_token(pool, source.id)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(token.access_token, "updated_token");
    assert_eq!(token.refresh_token, Some("new_refresh".to_string()));
    assert_eq!(token.expires_at, Some(12345));
}

#[tokio::test]
async fn test_clear_auth_token() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let source = soul_storage::sources::add_server_source(
        pool,
        1,
        "Clear Test",
        "https://clear.example.com",
    )
    .await
    .unwrap();

    // Store token
    soul_storage::sources::store_auth_token(
        pool,
        source.id,
        "to_be_cleared",
        Some("refresh"),
        None,
    )
    .await
    .unwrap();

    // Clear token
    soul_storage::sources::clear_auth_token(pool, source.id)
        .await
        .expect("Failed to clear auth token");

    // Verify token is gone
    let token = soul_storage::sources::get_auth_token(pool, source.id)
        .await
        .unwrap();
    assert!(token.is_none());
}

#[tokio::test]
async fn test_is_token_expired() {
    use soul_storage::sources::{is_token_expired, AuthToken};

    // Token that expired in the past
    let expired_token = AuthToken {
        source_id: 1,
        access_token: "test".to_string(),
        refresh_token: None,
        expires_at: Some(chrono::Utc::now().timestamp() - 3600), // 1 hour ago
    };
    assert!(is_token_expired(&expired_token));

    // Token that expires soon (within 60 seconds buffer)
    let soon_expired_token = AuthToken {
        source_id: 1,
        access_token: "test".to_string(),
        refresh_token: None,
        expires_at: Some(chrono::Utc::now().timestamp() + 30), // 30 seconds from now
    };
    assert!(is_token_expired(&soon_expired_token));

    // Token that's still valid
    let valid_token = AuthToken {
        source_id: 1,
        access_token: "test".to_string(),
        refresh_token: None,
        expires_at: Some(chrono::Utc::now().timestamp() + 3600), // 1 hour from now
    };
    assert!(!is_token_expired(&valid_token));

    // Token with no expiry (never expires)
    let no_expiry_token = AuthToken {
        source_id: 1,
        access_token: "test".to_string(),
        refresh_token: None,
        expires_at: None,
    };
    assert!(!is_token_expired(&no_expiry_token));
}

// =============================================================================
// Sync State Management Tests
// =============================================================================

#[tokio::test]
async fn test_init_sync_state() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let source = soul_storage::sources::add_server_source(
        pool,
        1,
        "Sync State Test",
        "https://sync.example.com",
    )
    .await
    .unwrap();

    // Initialize sync state
    soul_storage::sources::init_sync_state(pool, source.id, 1, "download", 100)
        .await
        .expect("Failed to init sync state");

    // Get sync state
    let state = soul_storage::sources::get_sync_state(pool, source.id, 1)
        .await
        .expect("Failed to get sync state")
        .expect("Sync state not found");

    assert_eq!(state.source_id, source.id);
    assert_eq!(state.user_id, 1);
    assert_eq!(state.sync_status, "syncing");
    assert_eq!(state.last_sync_direction, Some("download".to_string()));
    assert_eq!(state.total_items, 100);
    assert_eq!(state.processed_items, 0);
}

#[tokio::test]
async fn test_update_sync_progress() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let source = soul_storage::sources::add_server_source(
        pool,
        1,
        "Progress Test",
        "https://progress.example.com",
    )
    .await
    .unwrap();

    // Initialize
    soul_storage::sources::init_sync_state(pool, source.id, 1, "upload", 50)
        .await
        .unwrap();

    // Update progress
    soul_storage::sources::update_sync_progress(
        pool,
        source.id,
        1,
        "uploading",
        Some("track_123.flac"),
        25,
    )
    .await
    .expect("Failed to update progress");

    // Verify
    let state = soul_storage::sources::get_sync_state(pool, source.id, 1)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(state.current_operation, Some("uploading".to_string()));
    assert_eq!(state.current_item, Some("track_123.flac".to_string()));
    assert_eq!(state.processed_items, 25);
}

#[tokio::test]
async fn test_complete_sync() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let source = soul_storage::sources::add_server_source(
        pool,
        1,
        "Complete Test",
        "https://complete.example.com",
    )
    .await
    .unwrap();

    // Initialize
    soul_storage::sources::init_sync_state(pool, source.id, 1, "download", 100)
        .await
        .unwrap();

    // Complete sync
    soul_storage::sources::complete_sync(
        pool,
        source.id,
        1,
        10, // uploaded
        80, // downloaded
        5,  // updated
        5,  // deleted
        Some("sync_token_xyz"),
    )
    .await
    .expect("Failed to complete sync");

    // Verify
    let state = soul_storage::sources::get_sync_state(pool, source.id, 1)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(state.sync_status, "idle");
    assert!(state.last_sync_at.is_some());
    assert!(state.current_operation.is_none());
    assert_eq!(state.tracks_uploaded, 10);
    assert_eq!(state.tracks_downloaded, 80);
    assert_eq!(state.tracks_updated, 5);
    assert_eq!(state.tracks_deleted, 5);
    assert_eq!(state.server_sync_token, Some("sync_token_xyz".to_string()));
}

#[tokio::test]
async fn test_fail_sync() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let source =
        soul_storage::sources::add_server_source(pool, 1, "Fail Test", "https://fail.example.com")
            .await
            .unwrap();

    // Initialize
    soul_storage::sources::init_sync_state(pool, source.id, 1, "download", 100)
        .await
        .unwrap();

    // Fail sync
    soul_storage::sources::fail_sync(pool, source.id, 1, "Connection timeout")
        .await
        .expect("Failed to fail sync");

    // Verify
    let state = soul_storage::sources::get_sync_state(pool, source.id, 1)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(state.sync_status, "error");
    assert_eq!(state.error_message, Some("Connection timeout".to_string()));
}

#[tokio::test]
async fn test_cancel_sync() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let source = soul_storage::sources::add_server_source(
        pool,
        1,
        "Cancel Test",
        "https://cancel.example.com",
    )
    .await
    .unwrap();

    // Initialize
    soul_storage::sources::init_sync_state(pool, source.id, 1, "upload", 50)
        .await
        .unwrap();

    // Cancel sync
    soul_storage::sources::cancel_sync(pool, source.id, 1)
        .await
        .expect("Failed to cancel sync");

    // Verify
    let state = soul_storage::sources::get_sync_state(pool, source.id, 1)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(state.sync_status, "cancelled");
}

#[tokio::test]
async fn test_cancel_only_affects_syncing_state() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let source = soul_storage::sources::add_server_source(
        pool,
        1,
        "Cancel Check",
        "https://check.example.com",
    )
    .await
    .unwrap();

    // Initialize and complete sync first
    soul_storage::sources::init_sync_state(pool, source.id, 1, "download", 10)
        .await
        .unwrap();
    soul_storage::sources::complete_sync(pool, source.id, 1, 0, 10, 0, 0, None)
        .await
        .unwrap();

    // Try to cancel (should not change from 'idle')
    soul_storage::sources::cancel_sync(pool, source.id, 1)
        .await
        .unwrap();

    // Verify state is still idle
    let state = soul_storage::sources::get_sync_state(pool, source.id, 1)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(state.sync_status, "idle");
}

#[tokio::test]
async fn test_get_server_sync_token() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let source = soul_storage::sources::add_server_source(
        pool,
        1,
        "Token Test",
        "https://token.example.com",
    )
    .await
    .unwrap();

    // Initially no sync token
    let token = soul_storage::sources::get_server_sync_token(pool, source.id, 1)
        .await
        .unwrap();
    assert!(token.is_none());

    // Complete a sync with a token
    soul_storage::sources::init_sync_state(pool, source.id, 1, "download", 10)
        .await
        .unwrap();
    soul_storage::sources::complete_sync(pool, source.id, 1, 0, 10, 0, 0, Some("token_abc123"))
        .await
        .unwrap();

    // Should now have the token
    let token = soul_storage::sources::get_server_sync_token(pool, source.id, 1)
        .await
        .unwrap();
    assert_eq!(token, Some("token_abc123".to_string()));
}

#[tokio::test]
async fn test_sync_state_per_user_isolation() {
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    let source = soul_storage::sources::add_server_source(
        pool,
        1,
        "Multi User",
        "https://multi.example.com",
    )
    .await
    .unwrap();

    // Init sync for user 1
    soul_storage::sources::init_sync_state(pool, source.id, 1, "download", 100)
        .await
        .unwrap();

    // Init sync for user 2
    soul_storage::sources::init_sync_state(pool, source.id, 2, "upload", 50)
        .await
        .unwrap();

    // Verify isolation
    let state1 = soul_storage::sources::get_sync_state(pool, source.id, 1)
        .await
        .unwrap()
        .unwrap();
    let state2 = soul_storage::sources::get_sync_state(pool, source.id, 2)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(state1.last_sync_direction, Some("download".to_string()));
    assert_eq!(state1.total_items, 100);

    assert_eq!(state2.last_sync_direction, Some("upload".to_string()));
    assert_eq!(state2.total_items, 50);
}
