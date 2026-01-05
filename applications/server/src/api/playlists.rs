/// Playlists API routes
use crate::{error::Result, middleware::AuthenticatedUser, state::AppState};
use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use soul_core::{Permission, Playlist, PlaylistId, Storage, TrackId, UserId};

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
    auth: AuthenticatedUser,
) -> Result<Json<Vec<Playlist>>> {
    let playlists = app_state.db.get_accessible_playlists(auth.user_id()).await?;
    Ok(Json(playlists))
}

/// POST /api/playlists
/// Create a new playlist
pub async fn create_playlist(
    State(app_state): State<AppState>,
    auth: AuthenticatedUser,
    Json(req): Json<CreatePlaylistRequest>,
) -> Result<Json<Playlist>> {
    let playlist = app_state.db.create_playlist(auth.user_id(), &req.name).await?;
    Ok(Json(playlist))
}

/// GET /api/playlists/:id
/// Get playlist details with tracks
pub async fn get_playlist(
    Path(id): Path<String>,
    State(app_state): State<AppState>,
) -> Result<Json<PlaylistWithTracks>> {
    let playlist_id = PlaylistId::new(id);
    let playlist = app_state.db.get_playlist(&playlist_id).await?;
    let tracks = app_state.db.get_playlist_tracks(&playlist_id).await?;

    Ok(Json(PlaylistWithTracks { playlist, tracks }))
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
    let mut playlist = app_state.db.get_playlist(&playlist_id).await?;

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

    app_state.db.delete_playlist(&playlist_id).await?;
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

    app_state.db.add_track_to_playlist(&playlist_id, &track_id).await?;
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

    app_state.db.remove_track_from_playlist(&playlist_id, &track_id)
        .await?;
    Ok(Json(serde_json::json!({ "success": true })))
}

/// POST /api/playlists/:id/share
/// Share a playlist with another user
pub async fn share_playlist(
    Path(id): Path<String>,
    State(app_state): State<AppState>,
    _auth: AuthenticatedUser,
    Json(req): Json<SharePlaylistRequest>,
) -> Result<Json<serde_json::Value>> {
    let playlist_id = PlaylistId::new(id);
    let shared_with_user_id = UserId::new(req.user_id);

    // Parse permission
    let permission = Permission::from_str(&req.permission).ok_or_else(|| {
        crate::error::ServerError::BadRequest("Invalid permission".to_string())
    })?;

    // TODO: Check ownership

    app_state.db.share_playlist(&playlist_id, &shared_with_user_id, permission)
        .await?;
    Ok(Json(serde_json::json!({ "success": true })))
}

/// DELETE /api/playlists/:id/share/:user_id
/// Unshare a playlist
pub async fn unshare_playlist(
    Path((id, user_id)): Path<(String, String)>,
    State(app_state): State<AppState>,
    _auth: AuthenticatedUser,
) -> Result<Json<serde_json::Value>> {
    let playlist_id = PlaylistId::new(id);
    let user_id = UserId::new(user_id);

    // TODO: Check ownership

    app_state.db.unshare_playlist(&playlist_id, &user_id).await?;
    Ok(Json(serde_json::json!({ "success": true })))
}

#[derive(Debug, Serialize)]
pub struct PlaylistWithTracks {
    pub playlist: Playlist,
    pub tracks: Vec<soul_core::Track>,
}
