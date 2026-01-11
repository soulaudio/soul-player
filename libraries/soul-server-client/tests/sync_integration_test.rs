//! Integration tests for the source sync flow.
//!
//! These tests verify the complete sync workflow combining
//! server client operations with storage state management.

use soul_server_client::{ServerConfig, SoulServerClient};
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Helper to create mock server track response
fn create_mock_track(id: &str, title: &str, format: &str) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "title": title,
        "artist": "Test Artist",
        "artist_id": "artist1",
        "album": "Test Album",
        "album_id": "album1",
        "album_artist": "Test Artist",
        "track_number": 1,
        "disc_number": 1,
        "year": 2024,
        "duration_seconds": 180.0,
        "file_format": format,
        "bitrate": if format == "mp3" { Some(320) } else { None },
        "sample_rate": 44100,
        "channels": 2,
        "file_size": 5000000,
        "content_hash": format!("hash_{}", id),
        "server_path": format!("/music/{}.{}", id, format),
        "created_at": "2024-01-01T00:00:00Z",
        "updated_at": "2024-01-01T00:00:00Z"
    })
}

// =============================================================================
// Full Sync Flow Tests
// =============================================================================

mod full_sync_flow {
    use super::*;

    /// Test: Complete sync workflow from login to full library fetch
    #[tokio::test]
    async fn test_login_and_full_sync() {
        let mock_server = MockServer::start().await;

        // Mock login endpoint
        Mock::given(method("POST"))
            .and(path("/api/auth/login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "sync_access_token",
                "refresh_token": "sync_refresh_token",
                "expires_in": 3600,
                "user_id": "user_sync",
                "username": "syncuser"
            })))
            .mount(&mock_server)
            .await;

        // Mock library endpoint
        Mock::given(method("GET"))
            .and(path("/api/library"))
            .and(header("Authorization", "Bearer sync_access_token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "tracks": [
                    create_mock_track("track1", "Song One", "flac"),
                    create_mock_track("track2", "Song Two", "mp3"),
                    create_mock_track("track3", "Song Three", "flac"),
                ],
                "albums": [{
                    "id": "album1",
                    "title": "Test Album",
                    "artist": "Test Artist",
                    "artist_id": "artist1",
                    "year": 2024,
                    "track_count": 3,
                    "cover_art_url": null
                }],
                "artists": [{
                    "id": "artist1",
                    "name": "Test Artist",
                    "album_count": 1,
                    "track_count": 3
                }],
                "server_timestamp": 1704067200,
                "sync_token": "initial_sync_token"
            })))
            .mount(&mock_server)
            .await;

        // Create client
        let config = ServerConfig::new(mock_server.uri());
        let client = SoulServerClient::new(config).unwrap();

        // Step 1: Login
        let login_response = client.login("syncuser", "password").await.unwrap();
        assert_eq!(login_response.username, "syncuser");
        assert!(client.is_authenticated().await);

        // Step 2: Get full library
        let library_handle = client.library().await.unwrap();
        let library = library_handle.client().get_full_library().await.unwrap();

        // Verify library contents
        assert_eq!(library.tracks.len(), 3);
        assert_eq!(library.albums.len(), 1);
        assert_eq!(library.artists.len(), 1);
        assert_eq!(library.sync_token, "initial_sync_token");

        // Verify track details
        let track1 = library.tracks.iter().find(|t| t.id == "track1").unwrap();
        assert_eq!(track1.title, "Song One");
        assert_eq!(track1.file_format, "flac");
    }

    /// Test: Delta sync after initial full sync
    #[tokio::test]
    async fn test_delta_sync_after_full_sync() {
        let mock_server = MockServer::start().await;

        // Mock delta endpoint
        Mock::given(method("GET"))
            .and(path("/api/library/delta"))
            .and(header("Authorization", "Bearer valid_token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "new_tracks": [
                    create_mock_track("new_track", "New Song", "flac"),
                ],
                "updated_tracks": [
                    create_mock_track("track1", "Updated Song One", "flac"),
                ],
                "deleted_track_ids": ["track3"],
                "server_timestamp": 1704153600,
                "sync_token": "updated_sync_token"
            })))
            .mount(&mock_server)
            .await;

        // Create authenticated client
        let config = ServerConfig::with_tokens(
            mock_server.uri(),
            "valid_token",
            Some("refresh_token".to_string()),
        );
        let client = SoulServerClient::new(config).unwrap();

        // Perform delta sync
        let library_handle = client.library().await.unwrap();
        let delta = library_handle
            .client()
            .get_library_delta(None, Some("initial_sync_token"))
            .await
            .unwrap();

        // Verify delta contents
        assert_eq!(delta.new_tracks.len(), 1);
        assert_eq!(delta.updated_tracks.len(), 1);
        assert_eq!(delta.deleted_track_ids.len(), 1);
        assert_eq!(delta.sync_token, "updated_sync_token");

        // Verify new track
        assert_eq!(delta.new_tracks[0].title, "New Song");

        // Verify updated track
        assert_eq!(delta.updated_tracks[0].title, "Updated Song One");

        // Verify deleted track
        assert_eq!(delta.deleted_track_ids[0], "track3");
    }

    /// Test: Multiple sequential delta syncs
    #[tokio::test]
    async fn test_sequential_delta_syncs() {
        let mock_server = MockServer::start().await;

        // First delta - add tracks
        Mock::given(method("GET"))
            .and(path("/api/library/delta"))
            .and(query_param("token", "sync_v1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "new_tracks": [
                    create_mock_track("delta1_track", "Delta 1 Track", "flac"),
                ],
                "updated_tracks": [],
                "deleted_track_ids": [],
                "server_timestamp": 1704100000,
                "sync_token": "sync_v2"
            })))
            .mount(&mock_server)
            .await;

        // Second delta - update and delete
        Mock::given(method("GET"))
            .and(path("/api/library/delta"))
            .and(query_param("token", "sync_v2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "new_tracks": [],
                "updated_tracks": [
                    create_mock_track("delta1_track", "Delta 1 Track Updated", "flac"),
                ],
                "deleted_track_ids": ["old_track"],
                "server_timestamp": 1704200000,
                "sync_token": "sync_v3"
            })))
            .mount(&mock_server)
            .await;

        let config =
            ServerConfig::with_tokens(mock_server.uri(), "token", Some("refresh".to_string()));
        let client = SoulServerClient::new(config).unwrap();
        let library_handle = client.library().await.unwrap();

        // First delta
        let delta1 = library_handle
            .client()
            .get_library_delta(None, Some("sync_v1"))
            .await
            .unwrap();

        assert_eq!(delta1.new_tracks.len(), 1);
        assert_eq!(delta1.sync_token, "sync_v2");

        // Second delta using token from first
        let delta2 = library_handle
            .client()
            .get_library_delta(None, Some("sync_v2"))
            .await
            .unwrap();

        assert_eq!(delta2.updated_tracks.len(), 1);
        assert_eq!(delta2.deleted_track_ids.len(), 1);
        assert_eq!(delta2.sync_token, "sync_v3");
    }
}

// =============================================================================
// Token Management Flow Tests
// =============================================================================

mod token_management {
    use super::*;

    /// Test: Tokens are properly stored after login and available for refresh
    #[tokio::test]
    async fn test_token_lifecycle() {
        let mock_server = MockServer::start().await;

        // Mock login
        Mock::given(method("POST"))
            .and(path("/api/auth/login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "initial_access",
                "refresh_token": "initial_refresh",
                "expires_in": 3600,
                "user_id": "user1",
                "username": "testuser"
            })))
            .mount(&mock_server)
            .await;

        // Mock token refresh
        Mock::given(method("POST"))
            .and(path("/api/auth/refresh"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "refreshed_access",
                "refresh_token": "refreshed_refresh",
                "expires_in": 3600
            })))
            .mount(&mock_server)
            .await;

        let config = ServerConfig::new(mock_server.uri());
        let client = SoulServerClient::new(config).unwrap();

        // Initially not authenticated
        assert!(!client.is_authenticated().await);

        // Login
        client.login("testuser", "password").await.unwrap();

        // Now authenticated with correct tokens
        assert!(client.is_authenticated().await);
        let (access, refresh) = client.get_tokens().await;
        assert_eq!(access.as_deref(), Some("initial_access"));
        assert_eq!(refresh.as_deref(), Some("initial_refresh"));

        // Refresh token
        client.refresh_token().await.unwrap();

        // Tokens should be updated
        let (access, refresh) = client.get_tokens().await;
        assert_eq!(access.as_deref(), Some("refreshed_access"));
        assert_eq!(refresh.as_deref(), Some("refreshed_refresh"));

        // Logout
        client.logout().await;

        // No longer authenticated
        assert!(!client.is_authenticated().await);
        let (access, refresh) = client.get_tokens().await;
        assert!(access.is_none());
        assert!(refresh.is_none());
    }

    /// Test: Client can be restored from saved tokens
    #[tokio::test]
    async fn test_restore_from_saved_tokens() {
        let mock_server = MockServer::start().await;

        // Mock library endpoint
        Mock::given(method("GET"))
            .and(path("/api/library"))
            .and(header("Authorization", "Bearer saved_access_token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "tracks": [],
                "albums": [],
                "artists": [],
                "server_timestamp": 1704000000,
                "sync_token": "token"
            })))
            .mount(&mock_server)
            .await;

        // Create client with saved tokens (simulating app restart)
        let config = ServerConfig::with_tokens(
            mock_server.uri(),
            "saved_access_token",
            Some("saved_refresh_token".to_string()),
        );
        let client = SoulServerClient::new(config).unwrap();

        // Should be authenticated immediately
        assert!(client.is_authenticated().await);

        // Should be able to fetch library
        let library_handle = client.library().await.unwrap();
        let result = library_handle.client().get_full_library().await;
        assert!(result.is_ok());
    }
}

// =============================================================================
// Multi-Source Sync Tests
// =============================================================================

mod multi_source {
    use super::*;

    /// Test: Multiple server sources can be managed independently
    #[tokio::test]
    async fn test_multiple_server_sources() {
        let mock_server1 = MockServer::start().await;
        let mock_server2 = MockServer::start().await;

        // Setup first server
        Mock::given(method("GET"))
            .and(path("/api/info"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "name": "Server 1",
                "version": "1.0.0",
                "features": [],
                "requires_auth": true,
                "max_upload_size": null
            })))
            .mount(&mock_server1)
            .await;

        // Setup second server
        Mock::given(method("GET"))
            .and(path("/api/info"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "name": "Server 2",
                "version": "2.0.0",
                "features": ["streaming"],
                "requires_auth": true,
                "max_upload_size": 524288000
            })))
            .mount(&mock_server2)
            .await;

        // Create clients for both servers
        let client1 = SoulServerClient::new(ServerConfig::new(mock_server1.uri())).unwrap();
        let client2 = SoulServerClient::new(ServerConfig::new(mock_server2.uri())).unwrap();

        // Test connections to both
        let info1 = client1.test_connection().await.unwrap();
        let info2 = client2.test_connection().await.unwrap();

        assert_eq!(info1.name, "Server 1");
        assert_eq!(info1.version, "1.0.0");
        assert_eq!(info2.name, "Server 2");
        assert_eq!(info2.version, "2.0.0");

        // Servers should be independent
        assert_ne!(client1.url().await, client2.url().await);
    }

    /// Test: Auth state is independent per source
    #[tokio::test]
    async fn test_independent_auth_state() {
        let mock_server1 = MockServer::start().await;
        let mock_server2 = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/auth/login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "server1_token",
                "refresh_token": "server1_refresh",
                "expires_in": 3600,
                "user_id": "user1",
                "username": "user"
            })))
            .mount(&mock_server1)
            .await;

        let client1 = SoulServerClient::new(ServerConfig::new(mock_server1.uri())).unwrap();
        let client2 = SoulServerClient::new(ServerConfig::new(mock_server2.uri())).unwrap();

        // Login to first server only
        client1.login("user", "pass").await.unwrap();

        // First should be authenticated
        assert!(client1.is_authenticated().await);

        // Second should not be authenticated
        assert!(!client2.is_authenticated().await);
    }
}

// =============================================================================
// Error Recovery Tests
// =============================================================================

mod error_recovery {
    use super::*;

    /// Test: Recovery from token expiration during sync
    #[tokio::test]
    async fn test_token_expiration_during_sync() {
        let mock_server = MockServer::start().await;

        // First call fails with 401 (token expired)
        Mock::given(method("GET"))
            .and(path("/api/library"))
            .respond_with(ResponseTemplate::new(401).set_body_string("Token expired"))
            .up_to_n_times(1)
            .mount(&mock_server)
            .await;

        let config = ServerConfig::with_tokens(
            mock_server.uri(),
            "expired_token",
            Some("valid_refresh".to_string()),
        );
        let client = SoulServerClient::new(config).unwrap();

        let library_handle = client.library().await.unwrap();
        let result = library_handle.client().get_full_library().await;

        // Should fail with auth error
        assert!(result.is_err());
    }

    /// Test: Handling of network interruption during sync
    #[tokio::test]
    async fn test_network_interruption() {
        // Use an invalid port to simulate network failure
        let config = ServerConfig::with_tokens(
            "http://127.0.0.1:1", // Invalid port, will fail to connect
            "token",
            Some("refresh".to_string()),
        );
        let client = SoulServerClient::new(config).unwrap();

        let library_handle = client.library().await.unwrap();
        let result = library_handle.client().get_full_library().await;

        // Should fail with connection error
        assert!(result.is_err());
    }

    /// Test: Partial sync state preservation on failure
    #[tokio::test]
    async fn test_sync_failure_preserves_token() {
        let mock_server = MockServer::start().await;

        // Login succeeds
        Mock::given(method("POST"))
            .and(path("/api/auth/login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "access",
                "refresh_token": "refresh",
                "expires_in": 3600,
                "user_id": "user",
                "username": "user"
            })))
            .mount(&mock_server)
            .await;

        // Library fetch fails
        Mock::given(method("GET"))
            .and(path("/api/library"))
            .respond_with(ResponseTemplate::new(500).set_body_string("Server error"))
            .mount(&mock_server)
            .await;

        let client = SoulServerClient::new(ServerConfig::new(mock_server.uri())).unwrap();

        // Login
        client.login("user", "pass").await.unwrap();
        assert!(client.is_authenticated().await);

        // Try library fetch (will fail)
        let library_handle = client.library().await.unwrap();
        let result = library_handle.client().get_full_library().await;
        assert!(result.is_err());

        // Should still be authenticated despite sync failure
        assert!(client.is_authenticated().await);
        let (access, refresh) = client.get_tokens().await;
        assert!(access.is_some());
        assert!(refresh.is_some());
    }
}

// =============================================================================
// Concurrent Sync Tests
// =============================================================================

mod concurrent_sync {
    use super::*;
    use std::sync::Arc;

    /// Test: Concurrent library fetches don't interfere
    #[tokio::test]
    async fn test_concurrent_library_fetches() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/api/library"))
            .and(header("Authorization", "Bearer token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "tracks": [create_mock_track("t1", "Track", "flac")],
                "albums": [],
                "artists": [],
                "server_timestamp": 1704000000,
                "sync_token": "token"
            })))
            .mount(&mock_server)
            .await;

        let config =
            ServerConfig::with_tokens(mock_server.uri(), "token", Some("refresh".to_string()));
        let client = Arc::new(SoulServerClient::new(config).unwrap());

        // Launch multiple concurrent fetches
        let mut handles = vec![];
        for _ in 0..5 {
            let client = Arc::clone(&client);
            handles.push(tokio::spawn(async move {
                let library_handle = client.library().await.unwrap();
                library_handle.client().get_full_library().await
            }));
        }

        // All should succeed
        for handle in handles {
            let result = handle.await.unwrap();
            assert!(result.is_ok());
            assert_eq!(result.unwrap().tracks.len(), 1);
        }
    }

    /// Test: Concurrent operations on different sources
    #[tokio::test]
    async fn test_concurrent_multi_source_operations() {
        let mock_server1 = MockServer::start().await;
        let mock_server2 = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/api/library"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "tracks": [create_mock_track("s1_t1", "Server 1 Track", "flac")],
                "albums": [],
                "artists": [],
                "server_timestamp": 1704000000,
                "sync_token": "s1_token"
            })))
            .mount(&mock_server1)
            .await;

        Mock::given(method("GET"))
            .and(path("/api/library"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "tracks": [create_mock_track("s2_t1", "Server 2 Track", "mp3")],
                "albums": [],
                "artists": [],
                "server_timestamp": 1704000000,
                "sync_token": "s2_token"
            })))
            .mount(&mock_server2)
            .await;

        let client1 = Arc::new(
            SoulServerClient::new(ServerConfig::with_tokens(
                mock_server1.uri(),
                "token1",
                Some("refresh1".to_string()),
            ))
            .unwrap(),
        );

        let client2 = Arc::new(
            SoulServerClient::new(ServerConfig::with_tokens(
                mock_server2.uri(),
                "token2",
                Some("refresh2".to_string()),
            ))
            .unwrap(),
        );

        // Fetch from both servers concurrently
        let c1 = Arc::clone(&client1);
        let c2 = Arc::clone(&client2);

        let (result1, result2) = tokio::join!(
            async move {
                let h = c1.library().await.unwrap();
                h.client().get_full_library().await
            },
            async move {
                let h = c2.library().await.unwrap();
                h.client().get_full_library().await
            }
        );

        // Both should succeed with correct data
        let lib1 = result1.unwrap();
        let lib2 = result2.unwrap();

        assert_eq!(lib1.tracks[0].id, "s1_t1");
        assert_eq!(lib2.tracks[0].id, "s2_t1");
    }
}

// =============================================================================
// Format and Quality Sync Tests
// =============================================================================

mod format_quality_sync {
    use super::*;

    /// Test: Track format information is correctly synced
    #[tokio::test]
    async fn test_format_info_sync() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/api/library"))
            .and(header("Authorization", "Bearer token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "tracks": [
                    {
                        "id": "flac_track",
                        "title": "Lossless Track",
                        "artist": "Artist",
                        "artist_id": "a1",
                        "album": "Album",
                        "album_id": "alb1",
                        "album_artist": null,
                        "track_number": 1,
                        "disc_number": 1,
                        "year": 2024,
                        "duration_seconds": 180.0,
                        "file_format": "flac",
                        "bitrate": null,
                        "sample_rate": 96000,
                        "channels": 2,
                        "file_size": 50000000,
                        "content_hash": "hash1",
                        "server_path": "/music/lossless.flac",
                        "created_at": "2024-01-01T00:00:00Z",
                        "updated_at": "2024-01-01T00:00:00Z"
                    },
                    {
                        "id": "mp3_track",
                        "title": "Lossy Track",
                        "artist": "Artist",
                        "artist_id": "a1",
                        "album": "Album",
                        "album_id": "alb1",
                        "album_artist": null,
                        "track_number": 2,
                        "disc_number": 1,
                        "year": 2024,
                        "duration_seconds": 200.0,
                        "file_format": "mp3",
                        "bitrate": 320,
                        "sample_rate": 44100,
                        "channels": 2,
                        "file_size": 8000000,
                        "content_hash": "hash2",
                        "server_path": "/music/lossy.mp3",
                        "created_at": "2024-01-01T00:00:00Z",
                        "updated_at": "2024-01-01T00:00:00Z"
                    },
                    {
                        "id": "hires_track",
                        "title": "Hi-Res Track",
                        "artist": "Artist",
                        "artist_id": "a1",
                        "album": "Album",
                        "album_id": "alb1",
                        "album_artist": null,
                        "track_number": 3,
                        "disc_number": 1,
                        "year": 2024,
                        "duration_seconds": 240.0,
                        "file_format": "flac",
                        "bitrate": null,
                        "sample_rate": 192000,
                        "channels": 2,
                        "file_size": 100000000,
                        "content_hash": "hash3",
                        "server_path": "/music/hires.flac",
                        "created_at": "2024-01-01T00:00:00Z",
                        "updated_at": "2024-01-01T00:00:00Z"
                    }
                ],
                "albums": [],
                "artists": [],
                "server_timestamp": 1704000000,
                "sync_token": "format_sync_token"
            })))
            .mount(&mock_server)
            .await;

        let config =
            ServerConfig::with_tokens(mock_server.uri(), "token", Some("refresh".to_string()));
        let client = SoulServerClient::new(config).unwrap();

        let library_handle = client.library().await.unwrap();
        let library = library_handle.client().get_full_library().await.unwrap();

        // Verify FLAC track
        let flac = library
            .tracks
            .iter()
            .find(|t| t.id == "flac_track")
            .unwrap();
        assert_eq!(flac.file_format, "flac");
        assert!(flac.bitrate.is_none()); // Lossless doesn't have bitrate
        assert_eq!(flac.sample_rate, Some(96000));

        // Verify MP3 track
        let mp3 = library.tracks.iter().find(|t| t.id == "mp3_track").unwrap();
        assert_eq!(mp3.file_format, "mp3");
        assert_eq!(mp3.bitrate, Some(320));
        assert_eq!(mp3.sample_rate, Some(44100));

        // Verify Hi-Res track
        let hires = library
            .tracks
            .iter()
            .find(|t| t.id == "hires_track")
            .unwrap();
        assert_eq!(hires.file_format, "flac");
        assert_eq!(hires.sample_rate, Some(192000)); // Hi-Res sample rate
    }

    /// Test: Channel configuration is correctly synced
    #[tokio::test]
    async fn test_channel_info_sync() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/api/library"))
            .and(header("Authorization", "Bearer token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "tracks": [
                    {
                        "id": "stereo",
                        "title": "Stereo Track",
                        "artist": null,
                        "artist_id": null,
                        "album": null,
                        "album_id": null,
                        "album_artist": null,
                        "track_number": null,
                        "disc_number": null,
                        "year": null,
                        "duration_seconds": 180.0,
                        "file_format": "flac",
                        "bitrate": null,
                        "sample_rate": 44100,
                        "channels": 2,
                        "file_size": 30000000,
                        "content_hash": "h1",
                        "server_path": "/stereo.flac",
                        "created_at": "2024-01-01T00:00:00Z",
                        "updated_at": "2024-01-01T00:00:00Z"
                    },
                    {
                        "id": "mono",
                        "title": "Mono Track",
                        "artist": null,
                        "artist_id": null,
                        "album": null,
                        "album_id": null,
                        "album_artist": null,
                        "track_number": null,
                        "disc_number": null,
                        "year": null,
                        "duration_seconds": 60.0,
                        "file_format": "mp3",
                        "bitrate": 128,
                        "sample_rate": 44100,
                        "channels": 1,
                        "file_size": 1000000,
                        "content_hash": "h2",
                        "server_path": "/mono.mp3",
                        "created_at": "2024-01-01T00:00:00Z",
                        "updated_at": "2024-01-01T00:00:00Z"
                    },
                    {
                        "id": "surround",
                        "title": "5.1 Surround Track",
                        "artist": null,
                        "artist_id": null,
                        "album": null,
                        "album_id": null,
                        "album_artist": null,
                        "track_number": null,
                        "disc_number": null,
                        "year": null,
                        "duration_seconds": 300.0,
                        "file_format": "flac",
                        "bitrate": null,
                        "sample_rate": 48000,
                        "channels": 6,
                        "file_size": 150000000,
                        "content_hash": "h3",
                        "server_path": "/surround.flac",
                        "created_at": "2024-01-01T00:00:00Z",
                        "updated_at": "2024-01-01T00:00:00Z"
                    }
                ],
                "albums": [],
                "artists": [],
                "server_timestamp": 1704000000,
                "sync_token": "channel_sync_token"
            })))
            .mount(&mock_server)
            .await;

        let config =
            ServerConfig::with_tokens(mock_server.uri(), "token", Some("refresh".to_string()));
        let client = SoulServerClient::new(config).unwrap();

        let library_handle = client.library().await.unwrap();
        let library = library_handle.client().get_full_library().await.unwrap();

        // Verify channel configurations
        let stereo = library.tracks.iter().find(|t| t.id == "stereo").unwrap();
        assert_eq!(stereo.channels, Some(2));

        let mono = library.tracks.iter().find(|t| t.id == "mono").unwrap();
        assert_eq!(mono.channels, Some(1));

        let surround = library.tracks.iter().find(|t| t.id == "surround").unwrap();
        assert_eq!(surround.channels, Some(6)); // 5.1 = 6 channels
    }
}
