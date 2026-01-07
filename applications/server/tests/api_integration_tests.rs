/// API integration tests
/// Tests complete HTTP request/response cycles with real database
mod common;

use axum::{
    body::Body,
    http::{header, Request, StatusCode},
    Router,
};
use common::create_test_database;
use soul_core::{Storage, Track, UserId};
use soul_server::{
    api, middleware,
    services::{AuthService, FileStorage},
    state::AppState,
};
use std::sync::Arc;
use tempfile::TempDir;
use tower::util::ServiceExt;

/// Helper to create test app router
async fn create_test_app() -> (
    Router,
    Arc<AuthService>,
    TempDir,
    Arc<soul_storage::Database>,
) {
    let db = create_test_database().await.unwrap();

    let temp_dir = TempDir::new().unwrap();
    let file_storage = FileStorage::new(temp_dir.path().to_path_buf());
    file_storage.initialize().await.unwrap();
    let file_storage = Arc::new(file_storage);

    let auth_service = Arc::new(AuthService::new(
        "test-secret-key".to_string(),
        1, // 1 hour access
        1, // 1 day refresh
    ));

    let app_state = AppState::new(db.clone(), Arc::clone(&auth_service), file_storage);

    // Build router with all routes
    let public_routes = Router::new()
        .route("/auth/login", axum::routing::post(api::auth::login))
        .route("/auth/refresh", axum::routing::post(api::auth::refresh));

    let protected_routes = Router::new()
        .route("/tracks", axum::routing::get(api::tracks::list_tracks))
        .route("/tracks/:id", axum::routing::get(api::tracks::get_track))
        .route(
            "/tracks/import",
            axum::routing::post(api::tracks::import_track),
        )
        .route(
            "/tracks/:id",
            axum::routing::delete(api::tracks::delete_track),
        )
        .route(
            "/playlists",
            axum::routing::get(api::playlists::list_playlists),
        )
        .route(
            "/playlists",
            axum::routing::post(api::playlists::create_playlist),
        )
        .route(
            "/playlists/:id",
            axum::routing::get(api::playlists::get_playlist),
        )
        .route("/admin/users", axum::routing::post(api::admin::create_user))
        .route("/admin/users", axum::routing::get(api::admin::list_users))
        .layer(axum::middleware::from_fn_with_state(
            Arc::clone(&auth_service),
            middleware::auth_middleware,
        ));

    let app = Router::new()
        .nest("/api", public_routes.merge(protected_routes))
        .with_state(app_state);

    (app, auth_service, temp_dir, db)
}

/// Test GET /api/tracks without authentication
#[tokio::test]
async fn test_get_tracks_unauthorized() {
    let (app, _, _temp_dir, db) = create_test_app().await;

    let request = Request::builder()
        .uri("/api/tracks")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

/// Test login flow and token usage
#[tokio::test]
async fn test_login_flow() {
    let (app, auth_service, _temp_dir, db) = create_test_app().await;

    // First, create a user directly in the database
    let user = db.create_user("testuser").await.unwrap();

    // Hash and store password
    let password_hash = auth_service.hash_password("password123").unwrap();
    store_test_credentials(&db, &user.id, &password_hash).await;

    // Attempt login
    let login_body = serde_json::json!({
        "username": "testuser",
        "password": "password123"
    });

    let request = Request::builder()
        .uri("/api/auth/login")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&login_body).unwrap()))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Parse response to get tokens
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let login_response: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert!(login_response["access_token"].is_string());
    assert!(login_response["refresh_token"].is_string());

    // Use access token to access protected route
    let access_token = login_response["access_token"].as_str().unwrap();

    let request = Request::builder()
        .uri("/api/tracks")
        .header(header::AUTHORIZATION, format!("Bearer {}", access_token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

/// Test login with wrong password
#[tokio::test]
async fn test_login_wrong_password() {
    let (app, auth_service, _temp_dir, db) = create_test_app().await;

    let user = db.create_user("testuser").await.unwrap();

    let password_hash = auth_service.hash_password("correctpassword").unwrap();
    store_test_credentials(&db, &user.id, &password_hash).await;

    let login_body = serde_json::json!({
        "username": "testuser",
        "password": "wrongpassword"
    });

    let request = Request::builder()
        .uri("/api/auth/login")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&login_body).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

/// Test login with nonexistent user
#[tokio::test]
async fn test_login_nonexistent_user() {
    let (app, _, _temp_dir, db) = create_test_app().await;

    let login_body = serde_json::json!({
        "username": "nonexistent",
        "password": "password"
    });

    let request = Request::builder()
        .uri("/api/auth/login")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&login_body).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

/// Test GET /api/tracks with authentication
#[tokio::test]
async fn test_get_tracks_authenticated() {
    let (app, auth_service, _temp_dir, db) = create_test_app().await;

    let user = db.create_user("testuser").await.unwrap();

    // Create access token
    let access_token = auth_service.create_access_token(&user.id).unwrap();

    let request = Request::builder()
        .uri("/api/tracks")
        .header(header::AUTHORIZATION, format!("Bearer {}", access_token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let tracks_response: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert!(tracks_response["tracks"].is_array());
    assert_eq!(tracks_response["tracks"].as_array().unwrap().len(), 0);
    assert_eq!(tracks_response["total"], 0);
}

/// Test GET /api/tracks with tracks in database
#[tokio::test]
async fn test_get_tracks_with_data() {
    let (app, auth_service, _temp_dir, db) = create_test_app().await;

    let user = db.create_user("testuser").await.unwrap();

    // Add test tracks
    let track = Track::new(
        "Test Song".to_string(),
        std::path::PathBuf::from("/fake/path.mp3"),
    );
    db.add_track(track.clone()).await.unwrap();

    let access_token = auth_service.create_access_token(&user.id).unwrap();

    let request = Request::builder()
        .uri("/api/tracks")
        .header(header::AUTHORIZATION, format!("Bearer {}", access_token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let tracks_response: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(tracks_response["total"], 1);
    assert_eq!(tracks_response["tracks"].as_array().unwrap().len(), 1);
    assert_eq!(tracks_response["tracks"][0]["title"], "Test Song");
}

/// Test GET /api/tracks with search query
#[tokio::test]
async fn test_get_tracks_with_search() {
    let (app, auth_service, _temp_dir, db) = create_test_app().await;

    let user = db.create_user("testuser").await.unwrap();

    // Add multiple tracks
    let track1 = Track::new(
        "Rock Song".to_string(),
        std::path::PathBuf::from("/fake/rock.mp3"),
    );
    let track2 = Track::new(
        "Jazz Song".to_string(),
        std::path::PathBuf::from("/fake/jazz.mp3"),
    );
    db.add_track(track1).await.unwrap();
    db.add_track(track2).await.unwrap();

    let access_token = auth_service.create_access_token(&user.id).unwrap();

    // Search for "Rock"
    let request = Request::builder()
        .uri("/api/tracks?q=Rock")
        .header(header::AUTHORIZATION, format!("Bearer {}", access_token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let tracks_response: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(tracks_response["total"], 1);
    assert_eq!(tracks_response["tracks"][0]["title"], "Rock Song");
}

/// Test GET /api/tracks with pagination
#[tokio::test]
async fn test_get_tracks_with_pagination() {
    let (app, auth_service, _temp_dir, db) = create_test_app().await;

    let user = db.create_user("testuser").await.unwrap();

    // Add 5 tracks
    for i in 1..=5 {
        let track = Track::new(
            format!("Song {}", i),
            std::path::PathBuf::from(format!("/fake/{}.mp3", i)),
        );
        db.add_track(track).await.unwrap();
    }

    let access_token = auth_service.create_access_token(&user.id).unwrap();

    // Get first 2 tracks
    let request = Request::builder()
        .uri("/api/tracks?limit=2&offset=0")
        .header(header::AUTHORIZATION, format!("Bearer {}", access_token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let tracks_response: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(tracks_response["total"], 5);
    assert_eq!(tracks_response["tracks"].as_array().unwrap().len(), 2);
}

/// Test POST /api/playlists
#[tokio::test]
async fn test_create_playlist() {
    let (app, auth_service, _temp_dir, db) = create_test_app().await;

    let user = db.create_user("testuser").await.unwrap();

    let access_token = auth_service.create_access_token(&user.id).unwrap();

    let create_body = serde_json::json!({
        "name": "My Playlist"
    });

    let request = Request::builder()
        .uri("/api/playlists")
        .method("POST")
        .header(header::AUTHORIZATION, format!("Bearer {}", access_token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&create_body).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let playlist_response: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(playlist_response["name"], "My Playlist");
    assert!(playlist_response["id"].is_string());
}

/// Test GET /api/playlists
#[tokio::test]
async fn test_get_playlists() {
    let (app, auth_service, _temp_dir, db) = create_test_app().await;

    let user = db.create_user("testuser").await.unwrap();

    // Create a playlist
    db.create_playlist(&user.id, "Test Playlist").await.unwrap();

    let access_token = auth_service.create_access_token(&user.id).unwrap();

    let request = Request::builder()
        .uri("/api/playlists")
        .header(header::AUTHORIZATION, format!("Bearer {}", access_token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let playlists: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert!(playlists.is_array());
    assert_eq!(playlists.as_array().unwrap().len(), 1);
    assert_eq!(playlists[0]["name"], "Test Playlist");
}

/// Test POST /api/admin/users
#[tokio::test]
async fn test_create_user() {
    let (app, auth_service, _temp_dir, db) = create_test_app().await;

    let admin_user = db.create_user("admin").await.unwrap();

    let access_token = auth_service.create_access_token(&admin_user.id).unwrap();

    let create_body = serde_json::json!({
        "username": "newuser",
        "password": "password123"
    });

    let request = Request::builder()
        .uri("/api/admin/users")
        .method("POST")
        .header(header::AUTHORIZATION, format!("Bearer {}", access_token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&create_body).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let user_response: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(user_response["user"]["name"], "newuser");
    assert_eq!(user_response["success"], true);
}

/// Test GET /api/admin/users
#[tokio::test]
async fn test_list_users() {
    let (app, auth_service, _temp_dir, db) = create_test_app().await;

    let admin_user = db.create_user("admin").await.unwrap();
    db.create_user("user1").await.unwrap();
    db.create_user("user2").await.unwrap();

    let access_token = auth_service.create_access_token(&admin_user.id).unwrap();

    let request = Request::builder()
        .uri("/api/admin/users")
        .header(header::AUTHORIZATION, format!("Bearer {}", access_token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let users: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert!(users.is_array());
    assert_eq!(users.as_array().unwrap().len(), 3);
}

/// Test invalid JSON request
#[tokio::test]
async fn test_invalid_json_request() {
    let (app, _, _temp_dir, db) = create_test_app().await;

    let request = Request::builder()
        .uri("/api/auth/login")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from("not valid json"))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// Helper function to store credentials for testing
async fn store_test_credentials(
    db: &Arc<soul_storage::Database>,
    user_id: &UserId,
    password_hash: &str,
) {
    use sqlx::Row;
    let pool = db.pool();
    let now = chrono::Utc::now().timestamp();

    sqlx::query(
        "INSERT INTO user_credentials (user_id, password_hash, created_at, updated_at) VALUES (?, ?, ?, ?)"
    )
    .bind(user_id.as_str())
    .bind(password_hash)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .unwrap();
}
