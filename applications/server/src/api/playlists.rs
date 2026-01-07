/// Playlists API routes
use crate::{error::Result, error::ServerError, middleware::AuthenticatedUser, state::AppState};
use axum::{
    extract::{Path, State},
    Json,
};
use serde::Deserialize;
use soul_core::{
    storage::StorageContext,
    types::{Playlist, PlaylistId, TrackId, UserId, CreatePlaylist},
};

#[derive(Debug, Deserialize)]
pub struct CreatePlaylistRequest {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePlaylistRequest {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct AddTrackRequest {
    pub track_id: String,
}

#[derive(Debug, Deserialize)]
pub struct SharePlaylistRequest {
    pub user_id: String,
    pub permission: String, // "read" or "write"
}

/// GET /api/playlists
/// Get all playlists accessible to the authenticated user
pub async fn list_playlists(
    State(app_state): State<AppState>,
    _auth: AuthenticatedUser,
) -> Result<Json<Vec<Playlist>>> {
    let playlists = app_state
        .db
        .get_user_playlists()
        .await?;
    Ok(Json(playlists))
}

/// POST /api/playlists
/// Create a new playlist
pub async fn create_playlist(
    State(app_state): State<AppState>,
    auth: AuthenticatedUser,
    Json(req): Json<CreatePlaylistRequest>,
) -> Result<Json<Playlist>> {
    let create_playlist = CreatePlaylist {
        name: req.name,
        description: None,
        owner_id: auth.user_id().clone(),
        is_favorite: false,
    };
    let playlist = app_state
        .db
        .create_playlist(create_playlist)
        .await?;
    Ok(Json(playlist))
}

/// GET /api/playlists/:id
/// Get playlist details with tracks
pub async fn get_playlist(
    Path(id): Path<String>,
    State(app_state): State<AppState>,
) -> Result<Json<Playlist>> {
    let playlist_id = PlaylistId::new(id);
    let playlist = app_state
        .db
        .get_playlist_with_tracks(playlist_id)
        .await?
        .ok_or_else(|| ServerError::NotFound("Playlist not found".to_string()))?;

    Ok(Json(playlist))
}

/// PUT /api/playlists/:id
/// Update playlist name
pub async fn update_playlist(
    Path(id): Path<String>,
    State(app_state): State<AppState>,
    _auth: AuthenticatedUser,
    Json(req): Json<UpdatePlaylistRequest>,
) -> Result<Json<Playlist>> {
    let playlist_id = PlaylistId::new(id);

    // Get existing playlist
    let mut playlist = app_state
        .db
        .get_playlist(playlist_id.clone())
        .await?
        .ok_or_else(|| ServerError::NotFound("Playlist not found".to_string()))?;

    // TODO: Check ownership or write permission

    // Update name (need to add update method to Storage trait)
    playlist.name = req.name;

    // For now, return the updated playlist
    // TODO: Implement actual database update
    Ok(Json(playlist))
}

/// DELETE /api/playlists/:id
/// Delete a playlist
pub async fn delete_playlist(
    Path(id): Path<String>,
    State(app_state): State<AppState>,
    _auth: AuthenticatedUser,
) -> Result<Json<serde_json::Value>> {
    let playlist_id = PlaylistId::new(id);

    // TODO: Check ownership

    app_state.db.delete_playlist(playlist_id).await?;
    Ok(Json(serde_json::json!({ "success": true })))
}

/// POST /api/playlists/:id/tracks
/// Add a track to a playlist
pub async fn add_track_to_playlist(
    Path(id): Path<String>,
    State(app_state): State<AppState>,
    _auth: AuthenticatedUser,
    Json(req): Json<AddTrackRequest>,
) -> Result<Json<serde_json::Value>> {
    let playlist_id = PlaylistId::new(id);
    let track_id = TrackId::new(req.track_id);

    // TODO: Check write permission

    app_state
        .db
        .add_track_to_playlist(playlist_id, track_id)
        .await?;
    Ok(Json(serde_json::json!({ "success": true })))
}

/// DELETE /api/playlists/:id/tracks/:track_id
/// Remove a track from a playlist
pub async fn remove_track_from_playlist(
    Path((id, track_id)): Path<(String, String)>,
    State(app_state): State<AppState>,
    _auth: AuthenticatedUser,
) -> Result<Json<serde_json::Value>> {
    let playlist_id = PlaylistId::new(id);
    let track_id = TrackId::new(track_id);

    // TODO: Check write permission

    app_state
        .db
        .remove_track_from_playlist(playlist_id, track_id)
        .await?;
    Ok(Json(serde_json::json!({ "success": true })))
}

/// POST /api/playlists/:id/share
/// Share a playlist with another user
pub async fn share_playlist(
    Path(id): Path<String>,
    State(app_state): State<AppState>,
    auth: AuthenticatedUser,
    Json(req): Json<SharePlaylistRequest>,
) -> Result<Json<serde_json::Value>> {
    let playlist_id = PlaylistId::new(id);
    let shared_with_user_id = UserId::new(req.user_id);

    // Validate permission (just check it's either "read" or "write")
    if req.permission != "read" && req.permission != "write" {
        return Err(ServerError::BadRequest("Invalid permission. Must be 'read' or 'write'".to_string()));
    }

    // Call soul_storage function directly (bypasses the trait)
    soul_storage::playlists::share_playlist(
        app_state.db.pool(),
        playlist_id,
        shared_with_user_id,
        &req.permission,
        auth.user_id().clone(),
    )
    .await?;

    Ok(Json(serde_json::json!({ "success": true })))
}

/// DELETE /api/playlists/:id/share/:user_id
/// Unshare a playlist
pub async fn unshare_playlist(
    Path((id, user_id)): Path<(String, String)>,
    State(app_state): State<AppState>,
    auth: AuthenticatedUser,
) -> Result<Json<serde_json::Value>> {
    let playlist_id = PlaylistId::new(id);
    let shared_user_id = UserId::new(user_id);

    // Call soul_storage function directly (bypasses the trait)
    soul_storage::playlists::unshare_playlist(
        app_state.db.pool(),
        playlist_id,
        shared_user_id,
        auth.user_id().clone(),
    )
    .await?;

    Ok(Json(serde_json::json!({ "success": true })))
}
