//! Comprehensive tests for the Soul Server Client library.
//!
//! These tests use mock servers to verify client behavior without
//! requiring a real server connection.

use soul_server_client::{ServerClientError, ServerConfig, SoulServerClient};
use wiremock::matchers::{body_json_string, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// =============================================================================
// Server Config Tests
// =============================================================================

mod server_config {
    use super::*;

    #[test]
    fn test_new_with_url() {
        let config = ServerConfig::new("https://example.com");
        assert_eq!(config.url, "https://example.com");
        assert!(config.access_token.is_none());
        assert!(config.refresh_token.is_none());
    }

    #[test]
    fn test_with_tokens() {
        let config = ServerConfig::with_tokens(
            "https://example.com",
            "access_token_123",
            Some("refresh_token_456".to_string()),
        );

        assert_eq!(config.url, "https://example.com");
        assert_eq!(config.access_token.as_deref(), Some("access_token_123"));
        assert_eq!(config.refresh_token.as_deref(), Some("refresh_token_456"));
    }

    #[test]
    fn test_with_tokens_no_refresh() {
        let config = ServerConfig::with_tokens("https://example.com", "access_token_123", None);

        assert!(config.access_token.is_some());
        assert!(config.refresh_token.is_none());
    }
}

// =============================================================================
// Client Creation Tests
// =============================================================================

mod client_creation {
    use super::*;

    #[test]
    fn test_valid_https_url() {
        let config = ServerConfig::new("https://example.com");
        let client = SoulServerClient::new(config);
        assert!(client.is_ok());
    }

    #[test]
    fn test_valid_http_url() {
        let config = ServerConfig::new("http://localhost:8080");
        let client = SoulServerClient::new(config);
        assert!(client.is_ok());
    }

    #[test]
    fn test_empty_url_rejected() {
        let config = ServerConfig::new("");
        let result = SoulServerClient::new(config);

        assert!(result.is_err());
        match result.unwrap_err() {
            ServerClientError::InvalidUrl(msg) => {
                assert!(msg.contains("empty"));
            }
            _ => panic!("Expected InvalidUrl error"),
        }
    }

    #[test]
    fn test_url_without_scheme_rejected() {
        let config = ServerConfig::new("example.com");
        let result = SoulServerClient::new(config);

        assert!(result.is_err());
        match result.unwrap_err() {
            ServerClientError::InvalidUrl(msg) => {
                assert!(msg.contains("http://") || msg.contains("https://"));
            }
            _ => panic!("Expected InvalidUrl error"),
        }
    }

    #[test]
    fn test_ftp_scheme_rejected() {
        let config = ServerConfig::new("ftp://example.com");
        let result = SoulServerClient::new(config);

        assert!(result.is_err());
        match result.unwrap_err() {
            ServerClientError::InvalidUrl(_) => {}
            _ => panic!("Expected InvalidUrl error"),
        }
    }

    #[test]
    fn test_url_normalization_trailing_slash() {
        let config = ServerConfig::new("https://example.com/");
        let client = SoulServerClient::new(config).unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();
        let url = rt.block_on(client.url());

        assert_eq!(url, "https://example.com");
        assert!(!url.ends_with('/'));
    }

    #[test]
    fn test_url_normalization_multiple_trailing_slashes() {
        let config = ServerConfig::new("https://example.com///");
        let client = SoulServerClient::new(config).unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();
        let url = rt.block_on(client.url());

        // Should remove all trailing slashes
        assert!(!url.ends_with('/'));
    }
}

// =============================================================================
// Connection Tests
// =============================================================================

mod connection {
    use super::*;

    #[tokio::test]
    async fn test_successful_connection() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/api/info"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "name": "Soul Player Server",
                "version": "1.0.0",
                "features": ["upload", "sync", "streaming"],
                "requires_auth": true,
                "max_upload_size": 104857600
            })))
            .mount(&mock_server)
            .await;

        let config = ServerConfig::new(mock_server.uri());
        let client = SoulServerClient::new(config).unwrap();

        let result = client.test_connection().await;
        assert!(result.is_ok());

        let info = result.unwrap();
        assert_eq!(info.name, "Soul Player Server");
        assert_eq!(info.version, "1.0.0");
        assert!(info.requires_auth);
        assert_eq!(info.features.len(), 3);
        assert_eq!(info.max_upload_size, Some(104857600));
    }

    #[tokio::test]
    async fn test_connection_to_unreachable_server() {
        let config = ServerConfig::new("http://localhost:99999");
        let client = SoulServerClient::new(config).unwrap();

        let result = client.test_connection().await;
        assert!(result.is_err());

        match result.unwrap_err() {
            ServerClientError::ServerUnreachable(_) | ServerClientError::Request(_) => {}
            e => panic!("Expected ServerUnreachable or Request error, got: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_connection_server_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/api/info"))
            .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
            .mount(&mock_server)
            .await;

        let config = ServerConfig::new(mock_server.uri());
        let client = SoulServerClient::new(config).unwrap();

        let result = client.test_connection().await;
        assert!(result.is_err());

        match result.unwrap_err() {
            ServerClientError::ServerError { status, message } => {
                assert_eq!(status, 500);
                assert!(message.contains("Internal Server Error"));
            }
            e => panic!("Expected ServerError, got: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_connection_invalid_json_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/api/info"))
            .respond_with(ResponseTemplate::new(200).set_body_string("not valid json"))
            .mount(&mock_server)
            .await;

        let config = ServerConfig::new(mock_server.uri());
        let client = SoulServerClient::new(config).unwrap();

        let result = client.test_connection().await;
        assert!(result.is_err());

        match result.unwrap_err() {
            ServerClientError::ParseError(_) => {}
            e => panic!("Expected ParseError, got: {:?}", e),
        }
    }
}

// =============================================================================
// Authentication Tests
// =============================================================================

mod authentication {
    use super::*;

    #[tokio::test]
    async fn test_successful_login() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/auth/login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "new_access_token",
                "refresh_token": "new_refresh_token",
                "expires_in": 3600,
                "user_id": "user123",
                "username": "testuser"
            })))
            .mount(&mock_server)
            .await;

        let config = ServerConfig::new(mock_server.uri());
        let client = SoulServerClient::new(config).unwrap();

        let result = client.login("testuser", "password123").await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.access_token, "new_access_token");
        assert_eq!(response.refresh_token, "new_refresh_token");
        assert_eq!(response.expires_in, 3600);
        assert_eq!(response.user_id, "user123");
        assert_eq!(response.username, "testuser");

        // Verify client is now authenticated
        assert!(client.is_authenticated().await);
    }

    #[tokio::test]
    async fn test_login_invalid_credentials() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/auth/login"))
            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
                "error": "unauthorized",
                "message": "Invalid credentials"
            })))
            .mount(&mock_server)
            .await;

        let config = ServerConfig::new(mock_server.uri());
        let client = SoulServerClient::new(config).unwrap();

        let result = client.login("wronguser", "wrongpassword").await;
        assert!(result.is_err());

        match result.unwrap_err() {
            ServerClientError::AuthFailed(msg) => {
                assert!(msg.contains("Invalid") || msg.contains("password"));
            }
            e => panic!("Expected AuthFailed, got: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_logout_clears_tokens() {
        let mock_server = MockServer::start().await;

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

        let config = ServerConfig::new(mock_server.uri());
        let client = SoulServerClient::new(config).unwrap();

        // Login first
        client.login("user", "pass").await.unwrap();
        assert!(client.is_authenticated().await);

        // Logout
        client.logout().await;
        assert!(!client.is_authenticated().await);

        let (access, refresh) = client.get_tokens().await;
        assert!(access.is_none());
        assert!(refresh.is_none());
    }

    #[tokio::test]
    async fn test_set_tokens_directly() {
        let config = ServerConfig::new("https://example.com");
        let client = SoulServerClient::new(config).unwrap();

        assert!(!client.is_authenticated().await);

        client
            .set_tokens(
                "direct_access_token".to_string(),
                Some("direct_refresh_token".to_string()),
            )
            .await;

        assert!(client.is_authenticated().await);

        let (access, refresh) = client.get_tokens().await;
        assert_eq!(access.as_deref(), Some("direct_access_token"));
        assert_eq!(refresh.as_deref(), Some("direct_refresh_token"));
    }

    #[tokio::test]
    async fn test_token_refresh() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/auth/refresh"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "refreshed_access",
                "refresh_token": "refreshed_refresh",
                "expires_in": 3600
            })))
            .mount(&mock_server)
            .await;

        let config = ServerConfig::with_tokens(
            mock_server.uri(),
            "old_access",
            Some("old_refresh".to_string()),
        );
        let client = SoulServerClient::new(config).unwrap();

        let result = client.refresh_token().await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.access_token, "refreshed_access");
        assert_eq!(response.refresh_token, "refreshed_refresh");

        // Client should have updated tokens
        let (access, refresh) = client.get_tokens().await;
        assert_eq!(access.as_deref(), Some("refreshed_access"));
        assert_eq!(refresh.as_deref(), Some("refreshed_refresh"));
    }

    #[tokio::test]
    async fn test_token_refresh_without_refresh_token() {
        let config = ServerConfig::new("https://example.com");
        let client = SoulServerClient::new(config).unwrap();

        let result = client.refresh_token().await;
        assert!(result.is_err());

        match result.unwrap_err() {
            ServerClientError::AuthRequired => {}
            e => panic!("Expected AuthRequired, got: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_token_refresh_expired() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/auth/refresh"))
            .respond_with(ResponseTemplate::new(401).set_body_string("Token expired"))
            .mount(&mock_server)
            .await;

        let config = ServerConfig::with_tokens(
            mock_server.uri(),
            "expired_access",
            Some("expired_refresh".to_string()),
        );
        let client = SoulServerClient::new(config).unwrap();

        let result = client.refresh_token().await;
        assert!(result.is_err());

        match result.unwrap_err() {
            ServerClientError::TokenRefreshFailed(_) => {}
            e => panic!("Expected TokenRefreshFailed, got: {:?}", e),
        }
    }
}

// =============================================================================
// Library Client Tests
// =============================================================================

mod library {
    use super::*;

    async fn setup_authenticated_client() -> (MockServer, SoulServerClient) {
        let mock_server = MockServer::start().await;

        let config = ServerConfig::with_tokens(
            mock_server.uri(),
            "valid_token",
            Some("refresh_token".to_string()),
        );
        let client = SoulServerClient::new(config).unwrap();

        (mock_server, client)
    }

    #[tokio::test]
    async fn test_get_full_library() {
        let (mock_server, client) = setup_authenticated_client().await;

        Mock::given(method("GET"))
            .and(path("/api/library"))
            .and(header("Authorization", "Bearer valid_token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "tracks": [
                    {
                        "id": "track1",
                        "title": "Test Song",
                        "artist": "Test Artist",
                        "artist_id": "artist1",
                        "album": "Test Album",
                        "album_id": "album1",
                        "album_artist": "Test Artist",
                        "track_number": 1,
                        "disc_number": 1,
                        "year": 2024,
                        "duration_seconds": 180.5,
                        "file_format": "flac",
                        "bitrate": null,
                        "sample_rate": 44100,
                        "channels": 2,
                        "file_size": 30000000,
                        "content_hash": "abc123",
                        "server_path": "/music/test.flac",
                        "created_at": "2024-01-01T00:00:00Z",
                        "updated_at": "2024-01-01T00:00:00Z"
                    }
                ],
                "albums": [
                    {
                        "id": "album1",
                        "title": "Test Album",
                        "artist": "Test Artist",
                        "artist_id": "artist1",
                        "year": 2024,
                        "track_count": 10,
                        "cover_art_url": "/art/album1.jpg"
                    }
                ],
                "artists": [
                    {
                        "id": "artist1",
                        "name": "Test Artist",
                        "album_count": 5,
                        "track_count": 50
                    }
                ],
                "server_timestamp": 1704067200,
                "sync_token": "sync_token_123"
            })))
            .mount(&mock_server)
            .await;

        let library_handle = client.library().await.unwrap();
        let result = library_handle.client().get_full_library().await;
        assert!(result.is_ok());

        let library = result.unwrap();
        assert_eq!(library.tracks.len(), 1);
        assert_eq!(library.albums.len(), 1);
        assert_eq!(library.artists.len(), 1);
        assert_eq!(library.sync_token, "sync_token_123");

        let track = &library.tracks[0];
        assert_eq!(track.title, "Test Song");
        assert_eq!(track.artist.as_deref(), Some("Test Artist"));
        assert_eq!(track.file_format, "flac");
    }

    #[tokio::test]
    async fn test_get_library_requires_auth() {
        let config = ServerConfig::new("https://example.com");
        let client = SoulServerClient::new(config).unwrap();

        let result = client.library().await;
        assert!(result.is_err());

        match result.unwrap_err() {
            ServerClientError::AuthRequired => {}
            e => panic!("Expected AuthRequired, got: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_get_library_delta() {
        let (mock_server, client) = setup_authenticated_client().await;

        Mock::given(method("GET"))
            .and(path("/api/library/delta"))
            .and(header("Authorization", "Bearer valid_token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "new_tracks": [
                    {
                        "id": "new_track",
                        "title": "New Song",
                        "artist": null,
                        "artist_id": null,
                        "album": null,
                        "album_id": null,
                        "album_artist": null,
                        "track_number": null,
                        "disc_number": null,
                        "year": null,
                        "duration_seconds": 200.0,
                        "file_format": "mp3",
                        "bitrate": 320,
                        "sample_rate": 44100,
                        "channels": 2,
                        "file_size": 5000000,
                        "content_hash": "def456",
                        "server_path": "/music/new.mp3",
                        "created_at": "2024-01-02T00:00:00Z",
                        "updated_at": "2024-01-02T00:00:00Z"
                    }
                ],
                "updated_tracks": [],
                "deleted_track_ids": ["old_track_1", "old_track_2"],
                "server_timestamp": 1704153600,
                "sync_token": "new_sync_token"
            })))
            .mount(&mock_server)
            .await;

        let library_handle = client.library().await.unwrap();
        let result = library_handle.client().get_library_delta(None, None).await;
        assert!(result.is_ok());

        let delta = result.unwrap();
        assert_eq!(delta.new_tracks.len(), 1);
        assert_eq!(delta.updated_tracks.len(), 0);
        assert_eq!(delta.deleted_track_ids.len(), 2);
        assert_eq!(delta.sync_token, "new_sync_token");
    }

    #[tokio::test]
    async fn test_get_single_track() {
        let (mock_server, client) = setup_authenticated_client().await;

        Mock::given(method("GET"))
            .and(path("/api/library/tracks/track123"))
            .and(header("Authorization", "Bearer valid_token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "track123",
                "title": "Specific Track",
                "artist": "Artist",
                "artist_id": "a1",
                "album": "Album",
                "album_id": "alb1",
                "album_artist": null,
                "track_number": 5,
                "disc_number": 1,
                "year": 2023,
                "duration_seconds": 240.0,
                "file_format": "flac",
                "bitrate": null,
                "sample_rate": 96000,
                "channels": 2,
                "file_size": 60000000,
                "content_hash": "hash123",
                "server_path": "/music/track.flac",
                "created_at": "2023-01-01T00:00:00Z",
                "updated_at": "2023-06-01T00:00:00Z"
            })))
            .mount(&mock_server)
            .await;

        let library_handle = client.library().await.unwrap();
        let result = library_handle.client().get_track("track123").await;
        assert!(result.is_ok());

        let track = result.unwrap();
        assert_eq!(track.id, "track123");
        assert_eq!(track.title, "Specific Track");
        assert_eq!(track.sample_rate, Some(96000));
    }

    #[tokio::test]
    async fn test_get_track_not_found() {
        let (mock_server, client) = setup_authenticated_client().await;

        Mock::given(method("GET"))
            .and(path("/api/library/tracks/nonexistent"))
            .respond_with(ResponseTemplate::new(404).set_body_string("Not found"))
            .mount(&mock_server)
            .await;

        let library_handle = client.library().await.unwrap();
        let result = library_handle.client().get_track("nonexistent").await;
        assert!(result.is_err());

        match result.unwrap_err() {
            ServerClientError::ServerError { status, message } => {
                assert_eq!(status, 404);
                assert!(message.contains("not found") || message.contains("nonexistent"));
            }
            e => panic!("Expected ServerError with 404, got: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_search_tracks() {
        let (mock_server, client) = setup_authenticated_client().await;

        Mock::given(method("GET"))
            .and(path("/api/library/tracks/search"))
            .and(header("Authorization", "Bearer valid_token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                {
                    "id": "result1",
                    "title": "Search Result 1",
                    "artist": "Artist",
                    "artist_id": null,
                    "album": null,
                    "album_id": null,
                    "album_artist": null,
                    "track_number": null,
                    "disc_number": null,
                    "year": null,
                    "duration_seconds": 180.0,
                    "file_format": "mp3",
                    "bitrate": 320,
                    "sample_rate": 44100,
                    "channels": 2,
                    "file_size": 5000000,
                    "content_hash": "h1",
                    "server_path": "/r1.mp3",
                    "created_at": "2024-01-01T00:00:00Z",
                    "updated_at": "2024-01-01T00:00:00Z"
                },
                {
                    "id": "result2",
                    "title": "Search Result 2",
                    "artist": "Artist",
                    "artist_id": null,
                    "album": null,
                    "album_id": null,
                    "album_artist": null,
                    "track_number": null,
                    "disc_number": null,
                    "year": null,
                    "duration_seconds": 200.0,
                    "file_format": "flac",
                    "bitrate": null,
                    "sample_rate": 44100,
                    "channels": 2,
                    "file_size": 30000000,
                    "content_hash": "h2",
                    "server_path": "/r2.flac",
                    "created_at": "2024-01-01T00:00:00Z",
                    "updated_at": "2024-01-01T00:00:00Z"
                }
            ])))
            .mount(&mock_server)
            .await;

        let library_handle = client.library().await.unwrap();
        let result = library_handle
            .client()
            .search_tracks("test query", None)
            .await;
        assert!(result.is_ok());

        let tracks = result.unwrap();
        assert_eq!(tracks.len(), 2);
    }

    #[tokio::test]
    async fn test_delete_track() {
        let (mock_server, client) = setup_authenticated_client().await;

        Mock::given(method("DELETE"))
            .and(path("/api/library/tracks/track_to_delete"))
            .and(header("Authorization", "Bearer valid_token"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&mock_server)
            .await;

        let library_handle = client.library().await.unwrap();
        let result = library_handle
            .client()
            .delete_track("track_to_delete")
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_stream_url() {
        let (mock_server, client) = setup_authenticated_client().await;

        Mock::given(method("GET"))
            .and(path("/api/library/tracks/track123/stream"))
            .and(header("Authorization", "Bearer valid_token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "url": "https://cdn.example.com/stream/track123?token=xyz",
                "expires_in": 3600
            })))
            .mount(&mock_server)
            .await;

        let library_handle = client.library().await.unwrap();
        let result = library_handle.client().get_stream_url("track123").await;
        assert!(result.is_ok());

        let stream = result.unwrap();
        assert!(stream.url.contains("track123"));
        assert_eq!(stream.expires_in, 3600);
    }
}

// =============================================================================
// Upload Client Tests
// =============================================================================

mod upload {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    async fn setup_authenticated_client() -> (MockServer, SoulServerClient) {
        let mock_server = MockServer::start().await;

        let config = ServerConfig::with_tokens(
            mock_server.uri(),
            "valid_token",
            Some("refresh_token".to_string()),
        );
        let client = SoulServerClient::new(config).unwrap();

        (mock_server, client)
    }

    fn create_temp_audio_file(extension: &str) -> NamedTempFile {
        let mut file = tempfile::Builder::new()
            .suffix(&format!(".{}", extension))
            .tempfile()
            .unwrap();

        // Write some dummy data
        file.write_all(b"fake audio content").unwrap();
        file
    }

    #[tokio::test]
    async fn test_upload_requires_auth() {
        let config = ServerConfig::new("https://example.com");
        let client = SoulServerClient::new(config).unwrap();

        let result = client.upload().await;
        assert!(result.is_err());

        match result.unwrap_err() {
            ServerClientError::AuthRequired => {}
            e => panic!("Expected AuthRequired, got: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_upload_file_not_found() {
        let (_, client) = setup_authenticated_client().await;

        let upload_handle = client.upload().await.unwrap();
        let result = upload_handle
            .client()
            .upload_track(std::path::Path::new("/nonexistent/file.mp3"), None)
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ServerClientError::FileNotFound(path) => {
                assert!(path.contains("nonexistent"));
            }
            e => panic!("Expected FileNotFound, got: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_successful_upload() {
        let (mock_server, client) = setup_authenticated_client().await;

        Mock::given(method("POST"))
            .and(path("/api/library/tracks"))
            .and(header("Authorization", "Bearer valid_token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "track": {
                    "id": "new_track_id",
                    "title": "Uploaded Track",
                    "artist": null,
                    "artist_id": null,
                    "album": null,
                    "album_id": null,
                    "album_artist": null,
                    "track_number": null,
                    "disc_number": null,
                    "year": null,
                    "duration_seconds": 180.0,
                    "file_format": "mp3",
                    "bitrate": 320,
                    "sample_rate": 44100,
                    "channels": 2,
                    "file_size": 18,
                    "content_hash": "content_hash",
                    "server_path": "/uploads/track.mp3",
                    "created_at": "2024-01-01T00:00:00Z",
                    "updated_at": "2024-01-01T00:00:00Z"
                },
                "already_existed": false
            })))
            .mount(&mock_server)
            .await;

        let temp_file = create_temp_audio_file("mp3");

        let upload_handle = client.upload().await.unwrap();
        let result = upload_handle
            .client()
            .upload_track(temp_file.path(), None)
            .await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.track.id, "new_track_id");
        assert!(!response.already_existed);
    }

    #[tokio::test]
    async fn test_upload_duplicate_file() {
        let (mock_server, client) = setup_authenticated_client().await;

        Mock::given(method("POST"))
            .and(path("/api/library/tracks"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "track": {
                    "id": "existing_track_id",
                    "title": "Existing Track",
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
                    "file_size": 18,
                    "content_hash": "existing_hash",
                    "server_path": "/uploads/existing.flac",
                    "created_at": "2024-01-01T00:00:00Z",
                    "updated_at": "2024-01-01T00:00:00Z"
                },
                "already_existed": true
            })))
            .mount(&mock_server)
            .await;

        let temp_file = create_temp_audio_file("flac");

        let upload_handle = client.upload().await.unwrap();
        let result = upload_handle
            .client()
            .upload_track(temp_file.path(), None)
            .await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.already_existed);
    }

    #[tokio::test]
    async fn test_upload_file_too_large() {
        let (mock_server, client) = setup_authenticated_client().await;

        Mock::given(method("POST"))
            .and(path("/api/library/tracks"))
            .respond_with(ResponseTemplate::new(413).set_body_string("File too large"))
            .mount(&mock_server)
            .await;

        let temp_file = create_temp_audio_file("flac");

        let upload_handle = client.upload().await.unwrap();
        let result = upload_handle
            .client()
            .upload_track(temp_file.path(), None)
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ServerClientError::ServerError { status, message } => {
                assert_eq!(status, 413);
                assert!(message.contains("large"));
            }
            e => panic!("Expected ServerError with 413, got: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_upload_rate_limited() {
        let (mock_server, client) = setup_authenticated_client().await;

        Mock::given(method("POST"))
            .and(path("/api/library/tracks"))
            .respond_with(
                ResponseTemplate::new(429)
                    .insert_header("Retry-After", "120")
                    .set_body_string("Too many requests"),
            )
            .mount(&mock_server)
            .await;

        let temp_file = create_temp_audio_file("mp3");

        let upload_handle = client.upload().await.unwrap();
        let result = upload_handle
            .client()
            .upload_track(temp_file.path(), None)
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ServerClientError::RateLimited { retry_after_secs } => {
                assert_eq!(retry_after_secs, 120);
            }
            e => panic!("Expected RateLimited, got: {:?}", e),
        }
    }
}

// =============================================================================
// Error Type Tests
// =============================================================================

mod errors {
    use super::*;

    #[test]
    fn test_error_display() {
        let error = ServerClientError::AuthRequired;
        assert_eq!(format!("{}", error), "Authentication required");

        let error = ServerClientError::AuthFailed("Invalid password".to_string());
        assert!(format!("{}", error).contains("Invalid password"));

        let error = ServerClientError::ServerError {
            status: 500,
            message: "Internal error".to_string(),
        };
        assert!(format!("{}", error).contains("500"));
        assert!(format!("{}", error).contains("Internal error"));

        let error = ServerClientError::InvalidUrl("bad url".to_string());
        assert!(format!("{}", error).contains("bad url"));

        let error = ServerClientError::RateLimited {
            retry_after_secs: 60,
        };
        assert!(format!("{}", error).contains("60"));
    }

    #[test]
    fn test_error_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<ServerClientError>();
    }
}
