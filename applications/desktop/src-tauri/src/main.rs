// Prevents additional console window on Windows in release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::case_sensitive_file_extension_comparisons)]
#![allow(clippy::match_same_arms)]
#![allow(clippy::manual_range_contains)]
#![allow(clippy::unnecessary_filter_map)]
#![allow(clippy::struct_field_names)]
#![allow(clippy::match_wildcard_for_single_variants)]
#![allow(clippy::semicolon_if_nothing_returned)]
#![allow(clippy::unused_self)]
#![allow(clippy::inefficient_to_string)]
#![allow(clippy::manual_flatten)]

mod app_state;
mod artwork;
mod audio_settings;
mod deep_link;
mod dsp_commands;
mod fingerprint;
mod import;
mod library_settings;
mod loudness;
mod playback;
mod playback_context;
mod shortcuts;
mod sources;
mod splash;
mod sync;
// mod tray; // Temporarily disabled - Tauri 2.0 API change
mod updater;
mod window_state_manager;

use app_state::AppState;
use import::ImportManager;
use playback::PlaybackManager;
use serde::{Deserialize, Serialize};
use soul_playback::{lazy_queue::QueueContext, RepeatMode, ShuffleMode};
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager, State};

// Re-export types from soul-core for frontend
// Note: We add file_path for convenience in the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
struct FrontendTrack {
    id: i64,
    title: String,
    artist_name: Option<String>,
    artist_id: Option<i64>,
    album_title: Option<String>,
    album_id: Option<i64>,
    duration_seconds: Option<f64>,
    file_path: Option<String>,
    track_number: Option<i32>,
    year: Option<i32>,
    // Audio format metadata
    file_format: String,
    bit_rate: Option<i32>,
    sample_rate: Option<i32>,
    channels: Option<i32>,
    // Whether the track is in the managed library (vs watched folder)
    is_in_managed_library: bool,
}

impl From<soul_core::types::Track> for FrontendTrack {
    fn from(track: soul_core::types::Track) -> Self {
        // Get first available local file path
        let file_path = track.availability.iter().find_map(|avail| {
            if matches!(
                avail.status,
                soul_core::types::AvailabilityStatus::LocalFile
                    | soul_core::types::AvailabilityStatus::Cached
            ) {
                avail.local_file_path.clone()
            } else {
                None
            }
        });

        Self {
            id: track.id.as_str().parse().unwrap_or(0),
            title: track.title,
            artist_name: track.artist_name,
            artist_id: track.artist_id,
            album_title: track.album_title,
            album_id: track.album_id,
            duration_seconds: track.duration_seconds,
            file_path,
            track_number: track.track_number,
            year: track.year,
            file_format: track.file_format,
            bit_rate: track.bitrate,
            sample_rate: track.sample_rate,
            channels: track.channels,
            // Default to false - will be set correctly when library path is available
            is_in_managed_library: false,
        }
    }
}

impl FrontendTrack {
    /// Create from a Track with library path context to determine if in managed library
    fn from_track_with_library_path(
        track: soul_core::types::Track,
        library_path: &std::path::Path,
    ) -> Self {
        let mut frontend_track = Self::from(track);

        // Check if track's file path is inside the managed library
        if let Some(ref path) = frontend_track.file_path {
            let track_path = std::path::Path::new(path);
            frontend_track.is_in_managed_library = track_path.starts_with(library_path);
        }

        frontend_track
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FrontendAlbum {
    id: i64,
    title: String,
    artist_name: Option<String>,
    artist_id: Option<i64>,
    year: Option<i32>,
    cover_art_path: Option<String>,
}

impl From<soul_core::types::Album> for FrontendAlbum {
    fn from(album: soul_core::types::Album) -> Self {
        Self {
            id: album.id,
            title: album.title,
            artist_name: album.artist_name,
            artist_id: album.artist_id,
            year: album.year,
            cover_art_path: album.cover_art_path,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FrontendArtist {
    id: i64,
    name: String,
    sort_name: Option<String>,
    track_count: i32,
    album_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FrontendGenre {
    id: i64,
    name: String,
    track_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FrontendPlaylist {
    id: String,
    name: String,
    description: Option<String>,
    owner_id: String,
    is_public: bool,
    is_favorite: bool,
    track_count: i32,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Playlist {
    id: i64,
    name: String,
    description: Option<String>,
    owner_id: i64,
    created_at: String,
    updated_at: String,
}

// Tauri commands - Playback control

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct TrackData {
    track_id: String,
    title: String,
    artist: String,
    album: Option<String>,
    album_id: Option<i64>,
    file_path: String,
    duration_seconds: Option<f64>,
    track_number: Option<u32>,
    cover_art_path: Option<String>,
}

impl TrackData {
    fn to_queue_track(&self) -> soul_playback::QueueTrack {
        use std::time::Duration;

        // Set source based on album_id if available
        let source = if let Some(album_id) = self.album_id {
            soul_playback::TrackSource::Album {
                id: album_id.to_string(),
                name: self.album.clone().unwrap_or_default(),
            }
        } else {
            soul_playback::TrackSource::Single
        };

        soul_playback::QueueTrack {
            id: self.track_id.clone(),
            path: PathBuf::from(&self.file_path),
            title: self.title.clone(),
            artist: self.artist.clone(),
            album: self.album.clone(),
            duration: self
                .duration_seconds
                .map(|s| Duration::from_secs_f64(s))
                .unwrap_or(Duration::from_secs(0)),
            track_number: self.track_number,
            source,
        }
    }
}

#[tauri::command]
async fn play_track(
    track_id: String,
    title: String,
    artist: String,
    album: Option<String>,
    file_path: String,
    duration_seconds: Option<f64>,
    track_number: Option<u32>,
    playback: State<'_, PlaybackManager>,
) -> Result<(), String> {
    use std::time::Duration;

    let track = soul_playback::QueueTrack {
        id: track_id,
        path: PathBuf::from(file_path),
        title,
        artist,
        album,
        duration: duration_seconds
            .map(|s| Duration::from_secs_f64(s))
            .unwrap_or(Duration::from_secs(0)),
        track_number,
        source: soul_playback::TrackSource::Single,
    };

    playback.play_track(track)
}

#[tauri::command(rename_all = "camelCase")]
async fn play_queue(
    queue: Vec<TrackData>,
    start_index: usize,
    playback: State<'_, PlaybackManager>,
) -> Result<(), String> {
    tracing::info!(
        "[play_queue] Called with {} tracks, start_index: {}",
        queue.len(),
        start_index
    );

    if queue.is_empty() {
        return Err("Queue is empty".to_string());
    }

    if start_index >= queue.len() {
        return Err("Start index out of bounds".to_string());
    }

    // Debug: print first track info
    if let Some(first) = queue.first() {
        tracing::info!(
            "[play_queue] First track: {}, path: {}",
            first.title,
            first.file_path
        );
    }

    // Convert to QueueTrack format
    let tracks: Vec<soul_playback::QueueTrack> = queue
        .iter()
        .map(|track_data| track_data.to_queue_track())
        .collect();

    tracing::info!(
        "[play_queue] Loading {} tracks as playlist (source queue)",
        tracks.len()
    );

    // Stop current playback
    tracing::debug!("[play_queue] Calling stop()...");
    let stop_result = playback.stop();
    tracing::debug!("[play_queue] stop() returned: {:?}", stop_result);
    stop_result?;

    // Load playlist as source queue (Spotify-style context)
    // This replaces the source queue tier, keeping explicit queue separate
    tracing::debug!(
        "[play_queue] Calling load_playlist() with {} tracks...",
        tracks.len()
    );
    let load_result = playback.load_playlist(tracks);
    tracing::debug!("[play_queue] load_playlist() returned: {:?}", load_result);
    load_result?;

    // Start playback (will play first track in source queue)
    tracing::debug!("[play_queue] Calling play()...");
    let play_result = playback.play();
    tracing::debug!("[play_queue] play() returned: {:?}", play_result);
    play_result?;
    tracing::info!("[play_queue] All commands sent successfully");

    Ok(())
}

#[tauri::command(rename_all = "camelCase")]
async fn play_queue_with_context(
    context: QueueContext,
    initial_batch: Vec<TrackData>,
    start_index: usize,
    enable_shuffle: bool,
    playback: State<'_, PlaybackManager>,
) -> Result<(), String> {
    tracing::info!(
        "[play_queue_with_context] Context: {:?}, batch size: {}, start_index: {}, shuffle: {}",
        context.type_name(),
        initial_batch.len(),
        start_index,
        enable_shuffle
    );

    if initial_batch.is_empty() {
        return Err("Initial batch is empty".to_string());
    }

    // Convert to QueueTrack format
    let tracks: Vec<soul_playback::QueueTrack> = initial_batch
        .iter()
        .map(|track_data| track_data.to_queue_track())
        .collect();

    // Generate shuffle seed on backend if shuffle is enabled
    let shuffle_seed = if enable_shuffle {
        use std::time::{SystemTime, UNIX_EPOCH};
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_secs();
        tracing::info!("[play_queue_with_context] Generated shuffle seed: {}", seed);
        Some(seed)
    } else {
        None
    };

    // Stop current playback
    playback.stop()?;

    // Load initial batch as source queue
    playback.load_playlist(tracks)?;

    // Set lazy context with backend-generated seed
    playback.set_lazy_context(context.clone(), shuffle_seed)?;

    // Apply shuffle if enabled
    if enable_shuffle {
        playback.set_shuffle(ShuffleMode::Random)?;
    }

    // Skip to start index if needed
    if start_index > 0 {
        playback.skip_to_index(start_index)?;
    }

    // Start playback
    playback.play()?;

    tracing::info!(
        "[play_queue_with_context] Lazy queue initialized: context={:?}, total_count={}",
        context.type_name(),
        context.total_count()
    );

    Ok(())
}

#[tauri::command]
async fn play(playback: State<'_, PlaybackManager>) -> Result<(), String> {
    playback.play()
}

#[tauri::command]
async fn pause_playback(playback: State<'_, PlaybackManager>) -> Result<(), String> {
    playback.pause()
}

#[tauri::command]
async fn resume_playback(playback: State<'_, PlaybackManager>) -> Result<(), String> {
    playback.play()
}

#[tauri::command]
async fn stop_playback(playback: State<'_, PlaybackManager>) -> Result<(), String> {
    playback.stop()
}

#[tauri::command]
async fn next_track(playback: State<'_, PlaybackManager>) -> Result<(), String> {
    playback.next()
}

#[tauri::command]
async fn previous_track(playback: State<'_, PlaybackManager>) -> Result<(), String> {
    playback.previous()
}

#[tauri::command]
async fn set_volume(volume: u8, playback: State<'_, PlaybackManager>) -> Result<(), String> {
    playback.set_volume(volume)
}

#[tauri::command]
async fn mute(playback: State<'_, PlaybackManager>) -> Result<(), String> {
    playback.mute()
}

#[tauri::command]
async fn unmute(playback: State<'_, PlaybackManager>) -> Result<(), String> {
    playback.unmute()
}

#[tauri::command]
async fn seek_to(position: f64, playback: State<'_, PlaybackManager>) -> Result<(), String> {
    playback.seek(position)
}

#[tauri::command]
async fn set_shuffle(mode: String, playback: State<'_, PlaybackManager>) -> Result<(), String> {
    let shuffle_mode = match mode.as_str() {
        "off" => ShuffleMode::Off,
        "random" => ShuffleMode::Random,
        "smart" => ShuffleMode::Smart,
        _ => return Err("Invalid shuffle mode".to_string()),
    };
    playback.set_shuffle(shuffle_mode)
}

#[tauri::command]
async fn set_repeat(mode: String, playback: State<'_, PlaybackManager>) -> Result<(), String> {
    let repeat_mode = match mode.as_str() {
        "off" => RepeatMode::Off,
        "all" => RepeatMode::All,
        "one" => RepeatMode::One,
        _ => return Err("Invalid repeat mode".to_string()),
    };
    playback.set_repeat(repeat_mode)
}

#[tauri::command]
async fn clear_queue(playback: State<'_, PlaybackManager>) -> Result<(), String> {
    playback.clear_queue()
}

#[tauri::command]
async fn add_play_next(
    track: TrackData,
    playback: State<'_, PlaybackManager>,
) -> Result<(), String> {
    let queue_track = track.to_queue_track();
    playback.add_play_next(queue_track)
}

#[tauri::command]
async fn add_to_queue_end(
    track: TrackData,
    playback: State<'_, PlaybackManager>,
) -> Result<(), String> {
    let queue_track = track.to_queue_track();
    playback.add_to_queue_end(queue_track)
}

#[tauri::command]
async fn clear_play_next(playback: State<'_, PlaybackManager>) -> Result<(), String> {
    playback.clear_play_next()
}

#[tauri::command]
async fn clear_add_to_queue(playback: State<'_, PlaybackManager>) -> Result<(), String> {
    playback.clear_add_to_queue()
}

#[tauri::command]
async fn cycle_shuffle(playback: State<'_, PlaybackManager>) -> Result<String, String> {
    playback.cycle_shuffle()
}

#[tauri::command]
async fn get_shuffle(playback: State<'_, PlaybackManager>) -> Result<String, String> {
    Ok(playback.get_shuffle().as_str().to_string())
}

#[tauri::command]
async fn get_repeat(playback: State<'_, PlaybackManager>) -> Result<String, String> {
    Ok(playback.get_repeat().as_str().to_string())
}

#[tauri::command]
async fn get_queue(playback: State<'_, PlaybackManager>) -> Result<Vec<TrackData>, String> {
    use soul_playback::TrackSource;

    let queue = playback.get_queue();
    let queue_data = queue
        .iter()
        .map(|track| {
            // Extract album_id and cover_art_path from source
            let (album_id, cover_art_path) = match &track.source {
                TrackSource::Album { id, .. } => {
                    let album_id = id.parse::<i64>().ok();
                    (album_id, Some(format!("artwork://album/{}", id)))
                }
                _ => (None, Some(format!("artwork://track/{}", track.id))),
            };

            TrackData {
                track_id: track.id.clone(),
                title: track.title.clone(),
                artist: track.artist.clone(),
                album: track.album.clone(),
                album_id,
                file_path: track.path.to_string_lossy().to_string(),
                duration_seconds: Some(track.duration.as_secs_f64()),
                track_number: track.track_number,
                cover_art_path,
            }
        })
        .collect();
    Ok(queue_data)
}

#[tauri::command]
async fn skip_to_queue_index(
    index: usize,
    playback: State<'_, PlaybackManager>,
) -> Result<(), String> {
    playback.skip_to_queue_index(index)
}

#[tauri::command]
async fn get_playback_capabilities(
    playback: State<'_, PlaybackManager>,
) -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "hasNext": playback.has_next(),
        "hasPrevious": playback.has_previous(),
    }))
}

/// Get current playback state (for syncing UI with audio layer)
#[tauri::command]
async fn get_playback_state(playback: State<'_, PlaybackManager>) -> Result<String, String> {
    let state = playback.get_state();
    // Return state as string matching what's emitted in events
    let state_str = match state {
        soul_playback::PlaybackState::Playing => "Playing",
        soul_playback::PlaybackState::Paused => "Paused",
        soul_playback::PlaybackState::Stopped => "Stopped",
        soul_playback::PlaybackState::Loading => "Loading",
    };
    Ok(state_str.to_string())
}

#[tauri::command]
async fn get_all_tracks(state: State<'_, AppState>) -> Result<Vec<FrontendTrack>, String> {
    let tracks = soul_storage::tracks::get_all(&state.pool)
        .await
        .map_err(|e| e.to_string())?;
    let frontend_tracks: Vec<FrontendTrack> = tracks
        .into_iter()
        .map(|t| FrontendTrack::from_track_with_library_path(t, &state.library_path))
        .collect();

    // Debug: Log tracks without file paths
    let tracks_without_paths = frontend_tracks
        .iter()
        .filter(|t| t.file_path.is_none())
        .count();
    if tracks_without_paths > 0 {
        tracing::warn!(
            "[get_all_tracks] WARNING: {} out of {} tracks have no file path",
            tracks_without_paths,
            frontend_tracks.len()
        );
    } else {
        tracing::debug!(
            "[get_all_tracks] All {} tracks have file paths",
            frontend_tracks.len()
        );
    }

    Ok(frontend_tracks)
}

#[tauri::command]
async fn get_track_by_id(
    id: i64,
    state: State<'_, AppState>,
) -> Result<Option<FrontendTrack>, String> {
    let track_id = soul_core::types::TrackId::new(id.to_string());
    let track = soul_storage::tracks::get_by_id(&state.pool, track_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(track.map(FrontendTrack::from))
}

#[tauri::command]
async fn delete_track(id: i64, state: State<'_, AppState>) -> Result<(), String> {
    tracing::info!("[delete_track] Starting deletion for track ID: {}", id);

    let track_id = soul_core::types::TrackId::new(id.to_string());

    // Get track info before deletion (need file path)
    let track = soul_storage::tracks::get_by_id(&state.pool, track_id.clone())
        .await
        .map_err(|e| {
            tracing::error!("[delete_track] Failed to fetch track: {}", e);
            format!("Failed to fetch track: {}", e)
        })?
        .ok_or_else(|| {
            tracing::error!("[delete_track] Track not found: {}", id);
            format!("Track not found with ID: {}", id)
        })?;

    tracing::info!("[delete_track] Found track: {}", track.title);

    // Get file path from availability
    let file_path = track
        .availability
        .iter()
        .find_map(|avail| avail.local_file_path.clone());

    tracing::debug!("[delete_track] File path: {:?}", file_path);
    tracing::debug!("[delete_track] Library path: {:?}", state.library_path);

    // Determine if file should be deleted (library-owned vs external)
    let should_delete_file = if let Some(ref path) = file_path {
        let path_buf = std::path::PathBuf::from(path);
        let is_library_owned = path_buf.starts_with(&state.library_path);
        tracing::debug!("[delete_track] Is library-owned: {}", is_library_owned);
        is_library_owned
    } else {
        tracing::debug!("[delete_track] No file path found, skipping file deletion");
        false
    };

    // Start database transaction
    tracing::debug!("[delete_track] Starting transaction");
    let mut tx = state.pool.begin().await.map_err(|e| {
        tracing::error!("[delete_track] Failed to start transaction: {}", e);
        format!("Database error: {}", e)
    })?;

    // Delete from database (CASCADE handles related tables)
    tracing::debug!("[delete_track] Deleting from database");
    let id_int: i64 = id;
    sqlx::query!("DELETE FROM tracks WHERE id = ?", id_int)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            tracing::error!("[delete_track] Database deletion failed: {}", e);
            format!("Database deletion failed: {}", e)
        })?;

    tracing::debug!("[delete_track] Database record deleted");

    // If library-owned file, attempt deletion
    if should_delete_file {
        if let Some(path) = file_path {
            tracing::info!("[delete_track] Attempting to delete file: {}", path);

            match std::fs::remove_file(&path) {
                Ok(_) => {
                    tracing::info!("[delete_track] File deleted successfully");
                    // Commit transaction
                    tx.commit().await.map_err(|e| {
                        tracing::error!("[delete_track] Transaction commit failed: {}", e);
                        format!("Failed to commit transaction: {}", e)
                    })?;
                }
                Err(e) => {
                    tracing::error!(
                        "[delete_track] File deletion failed: {}, rolling back transaction",
                        e
                    );

                    // Rollback transaction
                    tx.rollback().await.map_err(|e| {
                        tracing::error!("[delete_track] Rollback failed: {}", e);
                        format!("Rollback failed: {}", e)
                    })?;

                    // Return error with context
                    return Err(format!(
                        "Failed to delete file '{}': {}. Database changes were rolled back.",
                        path, e
                    ));
                }
            }
        }
    } else {
        // External file - just commit database changes
        tracing::debug!("[delete_track] External file, only removing from database");
        tx.commit().await.map_err(|e| {
            tracing::error!("[delete_track] Transaction commit failed: {}", e);
            format!("Failed to commit transaction: {}", e)
        })?;
    }

    tracing::info!("[delete_track] Track deletion completed successfully");
    Ok(())
}

/// Show a file in the system file explorer
#[tauri::command]
async fn show_in_file_explorer(path: String) -> Result<(), String> {
    use std::process::Command;

    let path = std::path::Path::new(&path);

    if !path.exists() {
        return Err(format!("File not found: {}", path.display()));
    }

    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .args(["/select,", &path.to_string_lossy()])
            .spawn()
            .map_err(|e| format!("Failed to open explorer: {}", e))?;
        Ok(())
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .args(["-R", &path.to_string_lossy()])
            .spawn()
            .map_err(|e| format!("Failed to open Finder: {}", e))?;
        Ok(())
    }

    #[cfg(target_os = "linux")]
    {
        // Try different file managers in order of preference
        let parent = path.parent().unwrap_or(path);
        let file_managers = [
            ("xdg-open", vec![parent.to_string_lossy().to_string()]),
            (
                "nautilus",
                vec!["--select".to_string(), path.to_string_lossy().to_string()],
            ),
            (
                "dolphin",
                vec!["--select".to_string(), path.to_string_lossy().to_string()],
            ),
            ("nemo", vec![path.to_string_lossy().to_string()]),
            ("thunar", vec![parent.to_string_lossy().to_string()]),
        ];

        for (cmd, args) in file_managers {
            if Command::new(cmd).args(&args).spawn().is_ok() {
                return Ok(());
            }
        }

        return Err("No supported file manager found".to_string());
    }
}

/// Diagnostic command to check database state
#[tauri::command]
async fn check_database_health(state: State<'_, AppState>) -> Result<DatabaseHealthReport, String> {
    let tracks = soul_storage::tracks::get_all(&state.pool)
        .await
        .map_err(|e| e.to_string())?;

    let total_tracks = tracks.len();
    let tracks_with_file_paths = tracks.iter().filter(|t| !t.availability.is_empty()).count();
    let tracks_with_local_files = tracks
        .iter()
        .filter(|t| t.availability.iter().any(|a| a.local_file_path.is_some()))
        .count();

    Ok(DatabaseHealthReport {
        total_tracks,
        tracks_with_availability: tracks_with_file_paths,
        tracks_with_local_files,
        issues: if total_tracks > 0 && tracks_with_local_files == 0 {
            vec![
                "No tracks have local file paths set. You may need to re-import your library."
                    .to_string(),
            ]
        } else {
            vec![]
        },
    })
}

#[derive(Debug, Clone, serde::Serialize)]
struct DatabaseHealthReport {
    total_tracks: usize,
    tracks_with_availability: usize,
    tracks_with_local_files: usize,
    issues: Vec<String>,
}

#[tauri::command]
async fn get_all_albums(state: State<'_, AppState>) -> Result<Vec<FrontendAlbum>, String> {
    let albums = soul_storage::albums::get_all(&state.pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(albums.into_iter().map(FrontendAlbum::from).collect())
}

#[tauri::command]
async fn get_random_albums(
    state: State<'_, AppState>,
    limit: i64,
) -> Result<Vec<FrontendAlbum>, String> {
    let albums = soul_storage::albums::get_random(&state.pool, limit)
        .await
        .map_err(|e| e.to_string())?;
    Ok(albums.into_iter().map(FrontendAlbum::from).collect())
}

#[tauri::command]
async fn get_recently_added_albums(
    state: State<'_, AppState>,
    limit: i64,
) -> Result<Vec<FrontendAlbum>, String> {
    let albums = soul_storage::albums::get_recently_added(&state.pool, limit)
        .await
        .map_err(|e| e.to_string())?;
    Ok(albums.into_iter().map(FrontendAlbum::from).collect())
}

#[tauri::command]
async fn get_recently_added_albums_within_days(
    state: State<'_, AppState>,
    days: i64,
    limit: i64,
) -> Result<Vec<FrontendAlbum>, String> {
    let albums = soul_storage::albums::get_recently_added_within_days(&state.pool, days, limit)
        .await
        .map_err(|e| e.to_string())?;
    Ok(albums.into_iter().map(FrontendAlbum::from).collect())
}

#[tauri::command]
async fn get_least_played_albums(
    state: State<'_, AppState>,
    limit: i64,
) -> Result<Vec<FrontendAlbum>, String> {
    const DEFAULT_USER_ID: i64 = 1;
    let albums = soul_storage::albums::get_least_played(&state.pool, limit, DEFAULT_USER_ID)
        .await
        .map_err(|e| e.to_string())?;
    Ok(albums.into_iter().map(FrontendAlbum::from).collect())
}

#[tauri::command]
async fn get_time_capsule_albums(
    state: State<'_, AppState>,
    limit: i64,
) -> Result<Vec<FrontendAlbum>, String> {
    const DEFAULT_USER_ID: i64 = 1;
    let albums = soul_storage::albums::get_time_capsule(&state.pool, limit, DEFAULT_USER_ID)
        .await
        .map_err(|e| e.to_string())?;
    Ok(albums.into_iter().map(FrontendAlbum::from).collect())
}

#[tauri::command]
async fn get_genre_albums(
    state: State<'_, AppState>,
    genre_id: i64,
    limit: i64,
) -> Result<Vec<FrontendAlbum>, String> {
    let albums = soul_storage::albums::get_by_genre(&state.pool, genre_id, limit)
        .await
        .map_err(|e| e.to_string())?;
    Ok(albums.into_iter().map(FrontendAlbum::from).collect())
}

// ============================================================================
// Artist commands
// ============================================================================

#[tauri::command]
async fn get_all_artists(state: State<'_, AppState>) -> Result<Vec<FrontendArtist>, String> {
    let artists = soul_storage::artists::get_all(&state.pool)
        .await
        .map_err(|e| e.to_string())?;

    let mut frontend_artists = Vec::new();
    for artist in artists {
        // Count tracks for this artist
        let tracks = soul_storage::tracks::get_by_artist(&state.pool, artist.id)
            .await
            .map_err(|e| e.to_string())?;
        let track_count = tracks.len() as i32;

        // Count albums for this artist
        let albums = soul_storage::albums::get_by_artist(&state.pool, artist.id)
            .await
            .map_err(|e| e.to_string())?;
        let album_count = albums.len() as i32;

        frontend_artists.push(FrontendArtist {
            id: artist.id,
            name: artist.name,
            sort_name: artist.sort_name,
            track_count,
            album_count,
        });
    }

    Ok(frontend_artists)
}

#[tauri::command]
async fn get_artist_by_id(
    id: i64,
    state: State<'_, AppState>,
) -> Result<Option<FrontendArtist>, String> {
    let artist = soul_storage::artists::get_by_id(&state.pool, id)
        .await
        .map_err(|e| e.to_string())?;

    match artist {
        Some(artist) => {
            let tracks = soul_storage::tracks::get_by_artist(&state.pool, artist.id)
                .await
                .map_err(|e| e.to_string())?;
            let albums = soul_storage::albums::get_by_artist(&state.pool, artist.id)
                .await
                .map_err(|e| e.to_string())?;

            Ok(Some(FrontendArtist {
                id: artist.id,
                name: artist.name,
                sort_name: artist.sort_name,
                track_count: tracks.len() as i32,
                album_count: albums.len() as i32,
            }))
        }
        None => Ok(None),
    }
}

#[tauri::command]
async fn get_artist_albums(
    artist_id: i64,
    state: State<'_, AppState>,
) -> Result<Vec<FrontendAlbum>, String> {
    let albums = soul_storage::albums::get_by_artist(&state.pool, artist_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(albums.into_iter().map(FrontendAlbum::from).collect())
}

#[tauri::command]
async fn get_artist_tracks(
    artist_id: i64,
    state: State<'_, AppState>,
) -> Result<Vec<FrontendTrack>, String> {
    let tracks = soul_storage::tracks::get_by_artist(&state.pool, artist_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(tracks.into_iter().map(FrontendTrack::from).collect())
}

#[tauri::command]
async fn get_artist_top_tracks(
    artist_id: i64,
    limit: Option<i32>,
    state: State<'_, AppState>,
) -> Result<Vec<FrontendTrack>, String> {
    let user_id = soul_core::types::UserId::new(state.user_id.clone());
    let limit = limit.unwrap_or(10); // Default top 10

    let tracks =
        soul_storage::tracks::get_top_tracks_by_artist(&state.pool, user_id, artist_id, limit)
            .await
            .map_err(|e| e.to_string())?;

    Ok(tracks.into_iter().map(FrontendTrack::from).collect())
}

// ============================================================================
// Album commands
// ============================================================================

#[tauri::command]
async fn get_album_by_id(
    id: i64,
    state: State<'_, AppState>,
) -> Result<Option<FrontendAlbum>, String> {
    let album = soul_storage::albums::get_by_id(&state.pool, id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(album.map(FrontendAlbum::from))
}

#[tauri::command]
async fn get_album_tracks(
    album_id: i64,
    state: State<'_, AppState>,
) -> Result<Vec<FrontendTrack>, String> {
    let tracks = soul_storage::tracks::get_by_album(&state.pool, album_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(tracks.into_iter().map(FrontendTrack::from).collect())
}

// ============================================================================
// Genre commands
// ============================================================================

#[tauri::command]
async fn get_all_genres(state: State<'_, AppState>) -> Result<Vec<FrontendGenre>, String> {
    let genres = soul_storage::genres::get_all(&state.pool)
        .await
        .map_err(|e| e.to_string())?;

    let mut frontend_genres = Vec::new();
    for genre in genres {
        // Count tracks for this genre
        let tracks = soul_storage::tracks::get_by_genre(&state.pool, genre.id)
            .await
            .map_err(|e| e.to_string())?;

        frontend_genres.push(FrontendGenre {
            id: genre.id,
            name: genre.name,
            track_count: tracks.len() as i32,
        });
    }

    Ok(frontend_genres)
}

#[tauri::command]
async fn get_genre_by_id(
    id: i64,
    state: State<'_, AppState>,
) -> Result<Option<FrontendGenre>, String> {
    let genre = soul_storage::genres::get_by_id(&state.pool, id)
        .await
        .map_err(|e| e.to_string())?;

    match genre {
        Some(genre) => {
            let tracks = soul_storage::tracks::get_by_genre(&state.pool, genre.id)
                .await
                .map_err(|e| e.to_string())?;

            Ok(Some(FrontendGenre {
                id: genre.id,
                name: genre.name,
                track_count: tracks.len() as i32,
            }))
        }
        None => Ok(None),
    }
}

#[tauri::command]
async fn get_genre_tracks(
    genre_id: i64,
    state: State<'_, AppState>,
) -> Result<Vec<FrontendTrack>, String> {
    let tracks = soul_storage::tracks::get_by_genre(&state.pool, genre_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(tracks.into_iter().map(FrontendTrack::from).collect())
}

// ============================================================================
// Playlist commands
// ============================================================================

#[tauri::command]
async fn get_all_playlists(state: State<'_, AppState>) -> Result<Vec<FrontendPlaylist>, String> {
    let user_id = soul_core::types::UserId::new(state.user_id.clone());
    let playlists = soul_storage::playlists::get_user_playlists(&state.pool, user_id.clone())
        .await
        .map_err(|e| e.to_string())?;

    let mut frontend_playlists = Vec::new();
    for playlist in playlists {
        // Get track count
        let with_tracks = soul_storage::playlists::get_with_tracks(
            &state.pool,
            playlist.id.clone(),
            user_id.clone(),
        )
        .await
        .map_err(|e| e.to_string())?;
        let track_count = with_tracks
            .and_then(|p| p.tracks.map(|t| t.len()))
            .unwrap_or(0) as i32;

        frontend_playlists.push(FrontendPlaylist {
            id: playlist.id.as_str().to_string(),
            name: playlist.name,
            description: playlist.description,
            owner_id: playlist.owner_id.as_str().to_string(),
            is_public: playlist.is_public,
            is_favorite: playlist.is_favorite,
            track_count,
            created_at: playlist.created_at,
            updated_at: playlist.updated_at,
        });
    }

    Ok(frontend_playlists)
}

#[tauri::command]
async fn get_playlist_by_id(
    id: String,
    state: State<'_, AppState>,
) -> Result<Option<FrontendPlaylist>, String> {
    let user_id = soul_core::types::UserId::new(state.user_id.clone());
    let playlist_id = soul_core::types::PlaylistId::new(id);

    let playlist = soul_storage::playlists::get_with_tracks(&state.pool, playlist_id, user_id)
        .await
        .map_err(|e| e.to_string())?;

    Ok(playlist.map(|p| {
        let track_count = p.tracks.as_ref().map(|t| t.len()).unwrap_or(0) as i32;
        FrontendPlaylist {
            id: p.id.as_str().to_string(),
            name: p.name,
            description: p.description,
            owner_id: p.owner_id.as_str().to_string(),
            is_public: p.is_public,
            is_favorite: p.is_favorite,
            track_count,
            created_at: p.created_at,
            updated_at: p.updated_at,
        }
    }))
}

#[tauri::command]
async fn get_playlist_tracks(
    id: String,
    state: State<'_, AppState>,
) -> Result<Vec<FrontendTrack>, String> {
    let playlist_id = soul_core::types::PlaylistId::new(id);

    let tracks = soul_storage::tracks::get_by_playlist(&state.pool, playlist_id)
        .await
        .map_err(|e| e.to_string())?;

    Ok(tracks
        .into_iter()
        .map(|t| FrontendTrack::from_track_with_library_path(t, &state.library_path))
        .collect())
}

#[tauri::command]
async fn create_playlist(
    name: String,
    description: Option<String>,
    state: State<'_, AppState>,
) -> Result<FrontendPlaylist, String> {
    let user_id = soul_core::types::UserId::new(state.user_id.clone());

    let create_playlist = soul_core::types::CreatePlaylist {
        name,
        description,
        owner_id: user_id.clone(),
        is_favorite: false,
    };

    let playlist = soul_storage::playlists::create(&state.pool, create_playlist)
        .await
        .map_err(|e| e.to_string())?;

    Ok(FrontendPlaylist {
        id: playlist.id.as_str().to_string(),
        name: playlist.name,
        description: playlist.description,
        owner_id: playlist.owner_id.as_str().to_string(),
        is_public: playlist.is_public,
        is_favorite: playlist.is_favorite,
        track_count: 0,
        created_at: playlist.created_at,
        updated_at: playlist.updated_at,
    })
}

#[tauri::command]
async fn delete_playlist(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let user_id = soul_core::types::UserId::new(state.user_id.clone());
    let playlist_id = soul_core::types::PlaylistId::new(id);

    soul_storage::playlists::delete(&state.pool, playlist_id, user_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn add_track_to_playlist(
    playlist_id: String,
    track_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let user_id = soul_core::types::UserId::new(state.user_id.clone());
    let playlist_id = soul_core::types::PlaylistId::new(playlist_id);
    let track_id = soul_core::types::TrackId::new(track_id);

    soul_storage::playlists::add_track(&state.pool, playlist_id, track_id, user_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn remove_track_from_playlist(
    playlist_id: String,
    track_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let user_id = soul_core::types::UserId::new(state.user_id.clone());
    let playlist_id = soul_core::types::PlaylistId::new(playlist_id);
    let track_id = soul_core::types::TrackId::new(track_id);

    soul_storage::playlists::remove_track(&state.pool, playlist_id, track_id, user_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn reorder_playlist_track(
    playlist_id: String,
    track_id: String,
    new_position: i32,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let user_id = soul_core::types::UserId::new(state.user_id.clone());
    let playlist_id = soul_core::types::PlaylistId::new(playlist_id);
    let track_id = soul_core::types::TrackId::new(track_id);

    soul_storage::playlists::reorder_tracks(
        &state.pool,
        playlist_id,
        track_id,
        new_position,
        user_id,
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_playlists_containing_track(
    track_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let user_id = soul_core::types::UserId::new(state.user_id.clone());
    let track_id = soul_core::types::TrackId::new(track_id);

    let playlist_ids =
        soul_storage::playlists::get_playlists_containing_track(&state.pool, track_id, user_id)
            .await
            .map_err(|e| e.to_string())?;

    Ok(playlist_ids
        .into_iter()
        .map(|id| id.as_str().to_string())
        .collect())
}

#[tauri::command]
async fn scan_library(path: String) -> Result<(), String> {
    // TODO: Integrate with soul-metadata
    println!("Scanning library at: {}", path);
    Ok(())
}

// File association handler
fn handle_file_associations(app: AppHandle, files: Vec<PathBuf>) {
    if files.is_empty() {
        return;
    }

    // Filter to only audio files
    let audio_files: Vec<PathBuf> = files
        .into_iter()
        .filter(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| {
                    matches!(
                        ext.to_lowercase().as_str(),
                        "mp3"
                            | "flac"
                            | "wav"
                            | "ogg"
                            | "oga"
                            | "m4a"
                            | "mp4"
                            | "aac"
                            | "opus"
                            | "wma"
                            | "aiff"
                            | "aif"
                            | "ape"
                            | "wv"
                    )
                })
                .unwrap_or(false)
        })
        .collect();

    if audio_files.is_empty() {
        return;
    }

    // Emit event to frontend with the files to open
    let file_paths: Vec<String> = audio_files
        .iter()
        .filter_map(|p| p.to_str().map(String::from))
        .collect();

    if let Err(e) = app.emit("files-opened", file_paths) {
        tracing::error!("Failed to emit files-opened event: {}", e);
    }
}

// Settings commands

#[tauri::command]
async fn get_user_settings(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<soul_storage::settings::UserSetting>, String> {
    soul_storage::settings::get_all_settings(&state.pool, &state.user_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn set_user_setting(
    state: tauri::State<'_, AppState>,
    key: String,
    value: serde_json::Value,
) -> Result<(), String> {
    soul_storage::settings::set_setting(&state.pool, &state.user_id, &key, &value)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_user_setting(
    state: tauri::State<'_, AppState>,
    key: String,
) -> Result<Option<serde_json::Value>, String> {
    soul_storage::settings::get_setting(&state.pool, &state.user_id, &key)
        .await
        .map_err(|e| e.to_string())
}

/// Get artwork as data URL for a track
#[tauri::command]
async fn get_track_artwork(
    track_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<Option<String>, String> {
    let track_id_parsed = soul_core::types::TrackId::new(track_id);

    match state
        .artwork_manager
        .get_track_artwork_with_mime(track_id_parsed)
        .await
    {
        Ok(Some((data, mime_type))) => {
            // Convert to base64 data URL
            let base64_data =
                base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &data);
            let data_url = format!("data:{};base64,{}", mime_type, base64_data);
            Ok(Some(data_url))
        }
        Ok(None) => Ok(None),
        Err(e) => {
            tracing::error!("[get_track_artwork] Error: {}", e);
            Ok(None)
        }
    }
}

/// Get artwork as data URL for an album
#[tauri::command]
async fn get_album_artwork(
    album_id: i64,
    state: tauri::State<'_, AppState>,
) -> Result<Option<String>, String> {
    match state
        .artwork_manager
        .get_album_artwork_with_mime(album_id)
        .await
    {
        Ok(Some((data, mime_type))) => {
            // Convert to base64 data URL
            let base64_data =
                base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &data);
            let data_url = format!("data:{};base64,{}", mime_type, base64_data);
            Ok(Some(data_url))
        }
        Ok(None) => Ok(None),
        Err(e) => {
            tracing::error!("[get_album_artwork] Error: {}", e);
            Ok(None)
        }
    }
}

/// Response structure for artwork with source info
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ArtworkResponse {
    data_url: String,
    is_custom: bool,
}

/// Get artwork with source info for an album (for edit dialog)
#[tauri::command]
async fn get_album_artwork_with_source(
    album_id: i64,
    state: tauri::State<'_, AppState>,
) -> Result<Option<ArtworkResponse>, String> {
    match state
        .artwork_manager
        .get_album_artwork_with_source(album_id)
        .await
    {
        Ok(Some((data, mime_type, is_custom))) => {
            let base64_data =
                base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &data);
            let data_url = format!("data:{};base64,{}", mime_type, base64_data);
            Ok(Some(ArtworkResponse {
                data_url,
                is_custom,
            }))
        }
        Ok(None) => Ok(None),
        Err(e) => {
            tracing::error!("[get_album_artwork_with_source] Error: {}", e);
            Ok(None)
        }
    }
}

/// Get artwork for an artist
#[tauri::command]
async fn get_artist_artwork(
    artist_id: i64,
    state: tauri::State<'_, AppState>,
) -> Result<Option<String>, String> {
    match state
        .artwork_manager
        .get_artist_artwork_with_mime(artist_id)
        .await
    {
        Ok(Some((data, mime_type))) => {
            let base64_data =
                base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &data);
            let data_url = format!("data:{};base64,{}", mime_type, base64_data);
            Ok(Some(data_url))
        }
        Ok(None) => Ok(None),
        Err(e) => {
            tracing::error!("[get_artist_artwork] Error: {}", e);
            Ok(None)
        }
    }
}

/// Get artwork with source info for an artist (for edit dialog)
#[tauri::command]
async fn get_artist_artwork_with_source(
    artist_id: i64,
    state: tauri::State<'_, AppState>,
) -> Result<Option<ArtworkResponse>, String> {
    match state
        .artwork_manager
        .get_artist_artwork_with_source(artist_id)
        .await
    {
        Ok(Some((data, mime_type, is_custom))) => {
            let base64_data =
                base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &data);
            let data_url = format!("data:{};base64,{}", mime_type, base64_data);
            Ok(Some(ArtworkResponse {
                data_url,
                is_custom,
            }))
        }
        Ok(None) => Ok(None),
        Err(e) => {
            tracing::error!("[get_artist_artwork_with_source] Error: {}", e);
            Ok(None)
        }
    }
}

/// Get artwork for a playlist
#[tauri::command]
async fn get_playlist_artwork(
    playlist_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<Option<String>, String> {
    let playlist_id = soul_core::types::PlaylistId::new(playlist_id);
    match state
        .artwork_manager
        .get_playlist_artwork_with_mime(&playlist_id)
        .await
    {
        Ok(Some((data, mime_type))) => {
            let base64_data =
                base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &data);
            let data_url = format!("data:{};base64,{}", mime_type, base64_data);
            Ok(Some(data_url))
        }
        Ok(None) => Ok(None),
        Err(e) => {
            tracing::error!("[get_playlist_artwork] Error: {}", e);
            Ok(None)
        }
    }
}

/// Get artwork with source info for a playlist (for edit dialog)
#[tauri::command]
async fn get_playlist_artwork_with_source(
    playlist_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<Option<ArtworkResponse>, String> {
    let playlist_id = soul_core::types::PlaylistId::new(playlist_id);
    match state
        .artwork_manager
        .get_playlist_artwork_with_source(&playlist_id)
        .await
    {
        Ok(Some((data, mime_type, is_custom))) => {
            let base64_data =
                base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &data);
            let data_url = format!("data:{};base64,{}", mime_type, base64_data);
            Ok(Some(ArtworkResponse {
                data_url,
                is_custom,
            }))
        }
        Ok(None) => Ok(None),
        Err(e) => {
            tracing::error!("[get_playlist_artwork_with_source] Error: {}", e);
            Ok(None)
        }
    }
}

/// Request structure for setting artwork
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetArtworkRequest {
    entity_type: String,
    entity_id: String,
    artwork_base64: String,
    mime_type: String,
    write_to_files: Option<bool>,
    use_soul_storage: Option<bool>,
}

/// Event payload for artwork changes
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ArtworkChangedEvent {
    entity_type: String,
    entity_id: String,
}

/// Set artwork for an entity (album, artist, or playlist)
#[tauri::command]
async fn set_artwork(
    request: SetArtworkRequest,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    use base64::Engine;

    let artwork_data = base64::engine::general_purpose::STANDARD
        .decode(&request.artwork_base64)
        .map_err(|e| format!("Invalid base64 data: {}", e))?;

    let result = match request.entity_type.as_str() {
        "album" => {
            let album_id: i64 = request.entity_id.parse().map_err(|_| "Invalid album ID")?;
            state
                .artwork_manager
                .set_album_artwork(
                    album_id,
                    artwork_data,
                    &request.mime_type,
                    request.write_to_files.unwrap_or(false),
                    request.use_soul_storage.unwrap_or(false),
                )
                .await
        }
        "artist" => {
            let artist_id: i64 = request.entity_id.parse().map_err(|_| "Invalid artist ID")?;
            state
                .artwork_manager
                .set_artist_artwork(artist_id, artwork_data, &request.mime_type)
                .await
        }
        "playlist" => {
            let playlist_id = soul_core::types::PlaylistId::new(request.entity_id.clone());
            let user_id = soul_core::types::UserId::new(state.user_id.clone());
            state
                .artwork_manager
                .set_playlist_artwork(&user_id, &playlist_id, artwork_data, &request.mime_type)
                .await
        }
        _ => Err(format!("Invalid entity type: {}", request.entity_type)),
    };

    // Emit event if successful
    if result.is_ok() {
        let event = ArtworkChangedEvent {
            entity_type: request.entity_type.clone(),
            entity_id: request.entity_id.clone(),
        };
        if let Err(e) = app.emit("artwork-changed", event) {
            tracing::error!("Failed to emit artwork-changed event: {}", e);
        }
    }

    result
}

/// Remove artwork from an entity
#[tauri::command]
async fn remove_artwork(
    entity_type: String,
    entity_id: String,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let result = match entity_type.as_str() {
        "album" => {
            let album_id: i64 = entity_id.parse().map_err(|_| "Invalid album ID")?;
            state.artwork_manager.remove_album_artwork(album_id).await
        }
        "artist" => {
            let artist_id: i64 = entity_id.parse().map_err(|_| "Invalid artist ID")?;
            state.artwork_manager.remove_artist_artwork(artist_id).await
        }
        "playlist" => {
            let playlist_id = soul_core::types::PlaylistId::new(entity_id.clone());
            let user_id = soul_core::types::UserId::new(state.user_id.clone());
            state
                .artwork_manager
                .remove_playlist_artwork(&user_id, &playlist_id)
                .await
        }
        _ => Err(format!("Invalid entity type: {}", entity_type)),
    };

    // Emit event if successful
    if result.is_ok() {
        let event = ArtworkChangedEvent {
            entity_type: entity_type.clone(),
            entity_id: entity_id.clone(),
        };
        if let Err(e) = app.emit("artwork-changed", event) {
            tracing::error!("Failed to emit artwork-changed event: {}", e);
        }
    }

    result
}

/// Debug command to test artwork extraction for a specific track
#[tauri::command]
async fn test_artwork_extraction(
    track_id: i64,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    tracing::info!(
        "[test_artwork_extraction] Testing artwork for track {}",
        track_id
    );

    let track_id_str = soul_core::types::TrackId::new(track_id.to_string());

    // Get track info
    let track = soul_storage::tracks::get_by_id(&state.pool, track_id_str.clone())
        .await
        .map_err(|e| format!("Failed to get track: {}", e))?;

    let Some(track) = track else {
        return Err(format!("Track {} not found", track_id));
    };

    tracing::info!("[test_artwork_extraction] Track title: {}", track.title);
    tracing::debug!(
        "[test_artwork_extraction] Availability count: {}",
        track.availability.len()
    );

    // Find file path
    let file_path = track.availability.iter().find_map(|avail| {
        tracing::debug!(
            "[test_artwork_extraction] Checking availability: status={:?}, path={:?}",
            avail.status,
            avail.local_file_path
        );
        if matches!(
            avail.status,
            soul_core::types::AvailabilityStatus::LocalFile
                | soul_core::types::AvailabilityStatus::Cached
        ) {
            avail.local_file_path.clone()
        } else {
            None
        }
    });

    let Some(file_path) = file_path else {
        return Err(format!("No local file path found for track {}", track_id));
    };

    tracing::info!("[test_artwork_extraction] File path: {}", file_path);

    // Check if file exists
    let path = std::path::Path::new(&file_path);
    if !path.exists() {
        return Err(format!("File does not exist: {}", file_path));
    }

    tracing::info!("[test_artwork_extraction] File exists, extracting artwork...");

    // Try to extract artwork
    match state
        .artwork_manager
        .get_track_artwork_with_mime(track_id_str)
        .await
    {
        Ok(Some((data, mime_type))) => {
            let msg = format!(
                "SUCCESS: Found artwork for '{}'\nFile: {}\nSize: {} bytes\nType: {}",
                track.title,
                file_path,
                data.len(),
                mime_type
            );
            tracing::info!("[test_artwork_extraction] {}", msg);
            Ok(msg)
        }
        Ok(None) => {
            let msg = format!(
                "No artwork found in file: {}\nThe file may not have embedded artwork.",
                file_path
            );
            tracing::warn!("[test_artwork_extraction] {}", msg);
            Err(msg)
        }
        Err(e) => {
            let msg = format!("Failed to extract artwork: {}", e);
            tracing::error!("[test_artwork_extraction] ERROR: {}", msg);
            Err(msg)
        }
    }
}

/// Get platform-specific app data directory
/// Windows: %APPDATA%\Soul Player\
/// macOS: ~/Library/Application Support/soul-player/
/// Linux: ~/.config/soul-player/
fn get_app_data_dir() -> std::path::PathBuf {
    if cfg!(target_os = "windows") {
        let roaming = std::env::var("APPDATA").expect("APPDATA environment variable not found");
        std::path::PathBuf::from(roaming).join("Soul Player")
    } else if cfg!(target_os = "macos") {
        let home = std::env::var("HOME").expect("HOME environment variable not found");
        std::path::PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join("soul-player")
    } else {
        let config_dir = if let Ok(xdg_config) = std::env::var("XDG_CONFIG_HOME") {
            std::path::PathBuf::from(xdg_config)
        } else {
            let home = std::env::var("HOME").expect("HOME environment variable not found");
            std::path::PathBuf::from(home).join(".config")
        };
        config_dir.join("soul-player")
    }
}

/// Initialize logging system with optional file output
fn init_logging(enable_file_logging: bool) {
    use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

    if enable_file_logging {
        // Get app data directory for logs
        let app_data_dir = get_app_data_dir();
        let logs_dir = app_data_dir.join("logs");

        // Create logs directory if it doesn't exist
        if let Err(e) = std::fs::create_dir_all(&logs_dir) {
            eprintln!("Failed to create logs directory: {}", e);
            // Fall back to console-only logging
            init_console_logging();
            return;
        }

        // Set up file appender with daily rotation
        let file_appender = tracing_appender::rolling::daily(logs_dir.clone(), "soul-player.log");
        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

        // Keep the guard alive for the lifetime of the program
        // This is necessary to ensure logs are flushed to disk
        std::mem::forget(guard);

        // Set up layers: console + file
        let env_filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("info,soul_importer=debug"));

        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt::layer().with_writer(std::io::stderr)) // Console output
            .with(fmt::layer().with_writer(non_blocking).with_ansi(false)) // File output (no colors)
            .init();

        eprintln!("[LOGGING] File logging enabled: {}", logs_dir.display());
        eprintln!("[LOGGING] Log files are saved as: soul-player.log.YYYY-MM-DD");
    } else {
        init_console_logging();
    }
}

/// Initialize console-only logging
fn init_console_logging() {
    use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,soul_importer=debug"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt::layer().with_writer(std::io::stderr))
        .init();
}

fn main() {
    // Check user's logging preference from database before initializing logging
    // This allows the UI toggle to override the --logs command-line flag
    let enable_file_logging = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async {
            // Get database path (same logic as in setup closure)
            let app_data_dir = get_app_data_dir();
            let db_path = app_data_dir.join("soul-player.db");

            // Only check database if it exists (avoid creating DB just to check logging)
            if !db_path.exists() {
                return None;
            }

            // Create temporary pool to check setting
            let db_url = if cfg!(windows) {
                let path_str = db_path.to_str()?.replace('\\', "/");
                format!("sqlite:///{}", path_str)
            } else {
                format!("sqlite://{}", db_path.to_str()?)
            };

            let pool = soul_storage::create_pool(&db_url).await.ok()?;

            // Check user's logging preference (user_id = "1" for desktop)
            soul_storage::settings::get_logging_enabled(&pool, "1")
                .await
                .ok()
                .flatten()
        })
        .unwrap_or_else(|| {
            // Fall back to --logs flag if:
            // - Database doesn't exist yet (first run)
            // - Database check failed
            // - User hasn't set preference (returns None)
            std::env::args().any(|arg| arg == "--logs")
        });

    // Initialize logging system
    init_logging(enable_file_logging);

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            // Called when a second instance is launched
            // Focus the main window
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_focus();
                let _ = window.unminimize();
            }

            // Handle file arguments passed to the second instance
            let files: Vec<PathBuf> = args
                .iter()
                .skip(1) // Skip the executable path
                .filter(|arg| !arg.starts_with('-'))
                .filter_map(|arg| {
                    if let Ok(url) = url::Url::parse(arg) {
                        url.to_file_path().ok()
                    } else {
                        Some(PathBuf::from(arg))
                    }
                })
                .collect();

            if !files.is_empty() {
                handle_file_associations(app.clone(), files);
            }
        }))
        .register_asynchronous_uri_scheme_protocol("artwork", |app, request, responder| {
            let uri = request.uri().to_string();
            tracing::debug!("[artwork protocol] Request: {}", uri);

            // Get the artwork manager from app state
            let app_handle = app.app_handle().clone();
            tauri::async_runtime::spawn(async move {
                let state = app_handle.state::<AppState>();
                let manager = &state.artwork_manager;

                match artwork::handle_artwork_request(manager, &uri).await {
                    Ok(response) => responder.respond(response),
                    Err(e) => {
                        tracing::error!("[artwork protocol] Error: {}", e);
                        let error_response = tauri::http::Response::builder()
                            .status(500)
                            .body(format!("Error: {}", e).into_bytes())
                            .unwrap();
                        responder.respond(error_response)
                    }
                }
            });
        })
        .setup(|app| {
            let app_handle = app.handle().clone();

            // Handle file associations from command line args (Windows/Linux)
            #[cfg(not(any(target_os = "macos", target_os = "ios")))]
            {
                let mut files = Vec::new();
                for maybe_file in std::env::args().skip(1) {
                    // Skip flags and options
                    if maybe_file.starts_with('-') {
                        continue;
                    }
                    // Try parsing as URL first (for file:// URLs)
                    if let Ok(url) = url::Url::parse(&maybe_file) {
                        if let Ok(path) = url.to_file_path() {
                            files.push(path);
                        }
                    } else {
                        // Otherwise treat as direct file path
                        files.push(PathBuf::from(maybe_file));
                    }
                }
                if !files.is_empty() {
                    handle_file_associations(app_handle.clone(), files);
                }
            }

            // Initialize app state with progress tracking
            tauri::async_runtime::block_on(async move {
                use splash::emit_init_progress;

                emit_init_progress(&app_handle, "Initializing database...", 10).await;

                // Get platform-specific app data directory
                let app_data_dir = get_app_data_dir();
                let db_path = app_data_dir.join("soul-player.db");
                tracing::info!("App data directory: {}", db_path.display());

                // Create AppState (handles migrations and default user)
                // Uses .env file if available (for development)
                let app_state = AppState::from_env_or_default(db_path)
                    .await
                    .expect("Failed to initialize app state");

                let pool = app_state.pool.as_ref().clone();

                emit_init_progress(&app_handle, "Loading settings...", 30).await;
                app_handle.manage(app_state);

                // Cleanup any orphaned scans from previous app crash/quit
                let device_id = library_settings::get_device_id();
                match soul_storage::library_sources::cleanup_orphaned_scans(
                    &pool, "1", // Desktop uses user_id = "1" as default user
                    &device_id,
                )
                .await
                {
                    Ok(count) if count > 0 => {
                        tracing::info!(
                            "Cleaned up {} orphaned scan(s) from previous session",
                            count
                        );
                    }
                    Err(e) => {
                        tracing::warn!("Failed to cleanup orphaned scans: {}", e);
                    }
                    _ => {}
                }

                emit_init_progress(&app_handle, "Initializing audio engine...", 50).await;

                // Initialize playback manager
                let playback_manager = PlaybackManager::new(app_handle.clone())
                    .expect("Failed to initialize playback");

                // Restore saved audio device (if any)
                {
                    let app_state_for_init = app_handle.state::<AppState>();
                    if let Err(e) = audio_settings::initialize_audio_device(
                        &playback_manager,
                        &app_state_for_init,
                    )
                    .await
                    {
                        tracing::warn!("[main] Warning: Failed to restore audio device: {}", e);
                    }
                }

                // Restore saved volume leveling mode (if any)
                {
                    let app_state_for_init = app_handle.state::<AppState>();
                    if let Err(e) = loudness::initialize_volume_leveling_mode(
                        &playback_manager,
                        &app_state_for_init,
                    )
                    .await
                    {
                        tracing::warn!(
                            "[main] Warning: Failed to restore volume leveling mode: {}",
                            e
                        );
                    }
                }

                // Restore saved DSP effect chain
                {
                    let app_state_for_init = app_handle.state::<AppState>();
                    dsp_commands::restore_dsp_chain_from_database(
                        &playback_manager,
                        &app_state_for_init.pool,
                        &app_state_for_init.user_id,
                    )
                    .await;
                }

                app_handle.manage(playback_manager);

                emit_init_progress(&app_handle, "Initializing loudness analyzer...", 55).await;

                // Initialize loudness analysis worker
                let analysis_worker =
                    std::sync::Arc::new(tokio::sync::Mutex::new(loudness::AnalysisWorker::new()));
                app_handle.manage(analysis_worker);

                emit_init_progress(&app_handle, "Configuring import system...", 60).await;

                // Initialize import manager
                // Use the same platform-specific directory as the database
                let library_path = app_data_dir.join("library");

                let import_manager = ImportManager::new(
                    pool.clone(),
                    "1".to_string(), // Desktop uses user_id = "1" as default user
                    library_path,
                )
                .await
                .expect("Failed to initialize import manager");
                app_handle.manage(import_manager);

                emit_init_progress(&app_handle, "Initializing sync system...", 65).await;

                // Initialize sync manager
                let sync_state = std::sync::Arc::new(tokio::sync::Mutex::new(
                    sync::SyncState::new(pool.clone()),
                ));
                app_handle.manage(sync_state.clone());

                // Check if auto-sync is needed (schema changes)
                {
                    let sync_guard = sync_state.lock().await;
                    if let Ok(Some(trigger)) = sync_guard.manager.should_auto_sync().await {
                        drop(sync_guard);
                        let app_clone = app_handle.clone();
                        tokio::spawn(async move {
                            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                            let _ = app_clone.emit("sync-required", trigger);
                        });
                    }
                }

                emit_init_progress(&app_handle, "Initializing fingerprint worker...", 68).await;

                // Initialize fingerprint worker
                let fingerprint_worker = std::sync::Arc::new(fingerprint::FingerprintWorker::new());
                app_handle.manage(fingerprint_worker);

                emit_init_progress(&app_handle, "Setting up system tray...", 70).await;

                // Setup system tray (temporarily disabled - Tauri 2.0 API change)
                // TODO: Fix tray imports for Tauri 2.0
                // if let Err(e) = tray::create_tray(&app_handle) {
                //     eprintln!("Failed to create tray: {}", e);
                // }

                emit_init_progress(&app_handle, "Registering shortcuts...", 80).await;

                // Register global shortcuts
                if let Err(e) = shortcuts::register_shortcuts(&app_handle).await {
                    tracing::warn!("Failed to register shortcuts: {}", e);
                }

                emit_init_progress(&app_handle, "Configuring deep links...", 85).await;

                // Setup deep link handler
                if let Err(e) = deep_link::setup(&app_handle) {
                    tracing::warn!("Failed to setup deep links: {}", e);
                }

                emit_init_progress(&app_handle, "Loading window state...", 90).await;

                // Load window state
                if let Err(e) = window_state_manager::load_window_state(&app_handle).await {
                    tracing::warn!("Failed to load window state: {}", e);
                }

                emit_init_progress(&app_handle, "Starting update checker...", 95).await;

                // Start update checker
                updater::start_update_checker(app_handle.clone());

                emit_init_progress(&app_handle, "Ready!", 100).await;

                // Close splash screen and show main window after a short delay
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                if let Some(splash) = app_handle.get_webview_window("splash") {
                    let _ = splash.close();
                }
                if let Some(main) = app_handle.get_webview_window("main") {
                    let _ = main.show();
                }
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                // Save window state on close
                let app = window.app_handle();
                tauri::async_runtime::block_on(async {
                    let _ = window_state_manager::save_window_state(&app).await;
                });
            }
        })
        .invoke_handler(tauri::generate_handler![
            // Playback control
            play_track,
            play_queue,
            play_queue_with_context,
            play,
            pause_playback,
            resume_playback,
            stop_playback,
            next_track,
            previous_track,
            set_volume,
            mute,
            unmute,
            seek_to,
            set_shuffle,
            set_repeat,
            clear_queue,
            add_play_next,
            add_to_queue_end,
            clear_play_next,
            clear_add_to_queue,
            cycle_shuffle,
            get_shuffle,
            get_repeat,
            get_queue,
            skip_to_queue_index,
            get_playback_capabilities,
            get_playback_state,
            // Audio settings
            audio_settings::get_audio_backends,
            audio_settings::get_audio_devices,
            audio_settings::get_audio_devices_with_capabilities,
            audio_settings::get_current_audio_device,
            audio_settings::get_device_capabilities,
            audio_settings::set_audio_device,
            audio_settings::refresh_sample_rate,
            audio_settings::is_r8brain_available,
            // Exclusive mode / Latency
            audio_settings::get_latency_info,
            audio_settings::set_exclusive_mode,
            audio_settings::disable_exclusive_mode,
            audio_settings::is_exclusive_mode,
            audio_settings::get_available_buffer_sizes,
            audio_settings::get_exclusive_preset,
            // Crossfade settings
            audio_settings::set_crossfade_enabled,
            audio_settings::is_crossfade_enabled,
            audio_settings::set_crossfade_duration,
            audio_settings::get_crossfade_duration,
            audio_settings::set_crossfade_curve,
            audio_settings::get_crossfade_curve,
            audio_settings::set_crossfade_settings,
            audio_settings::get_crossfade_settings,
            // Resampling settings
            audio_settings::set_resampling_quality,
            audio_settings::get_resampling_quality,
            audio_settings::set_resampling_target_rate,
            audio_settings::get_resampling_target_rate,
            audio_settings::set_resampling_backend,
            audio_settings::get_resampling_backend,
            audio_settings::set_resampling_settings,
            audio_settings::get_resampling_settings,
            // Headroom management
            audio_settings::get_headroom_settings,
            audio_settings::set_headroom_mode,
            audio_settings::set_headroom_enabled,
            audio_settings::set_headroom_eq_boost,
            audio_settings::set_headroom_preamp,
            // DSP effects chain
            dsp_commands::get_available_effects,
            dsp_commands::get_dsp_chain,
            dsp_commands::add_effect_to_chain,
            dsp_commands::remove_effect_from_chain,
            dsp_commands::toggle_effect,
            dsp_commands::update_effect_parameters,
            dsp_commands::clear_dsp_chain,
            dsp_commands::get_eq_presets,
            dsp_commands::get_compressor_presets,
            dsp_commands::get_limiter_presets,
            dsp_commands::get_crossfeed_presets,
            dsp_commands::get_stereo_presets,
            dsp_commands::get_graphic_eq_presets,
            dsp_commands::get_dsp_chain_presets,
            dsp_commands::save_dsp_chain_preset,
            dsp_commands::delete_dsp_chain_preset,
            dsp_commands::load_dsp_chain_preset,
            // Library management
            get_all_tracks,
            get_track_by_id,
            delete_track,
            show_in_file_explorer,
            check_database_health,
            // Albums
            get_all_albums,
            get_random_albums,
            get_recently_added_albums,
            get_recently_added_albums_within_days,
            get_least_played_albums,
            get_time_capsule_albums,
            get_genre_albums,
            get_album_by_id,
            get_album_tracks,
            // Artists
            get_all_artists,
            get_artist_by_id,
            get_artist_albums,
            get_artist_tracks,
            get_artist_top_tracks,
            // Genres
            get_all_genres,
            get_genre_by_id,
            get_genre_tracks,
            // Playlists
            get_all_playlists,
            get_playlist_by_id,
            get_playlist_tracks,
            create_playlist,
            delete_playlist,
            add_track_to_playlist,
            remove_track_from_playlist,
            reorder_playlist_track,
            get_playlists_containing_track,
            scan_library,
            // Library settings
            library_settings::get_library_sources,
            library_settings::add_library_source,
            library_settings::remove_library_source,
            library_settings::toggle_library_source,
            library_settings::rescan_library_source,
            library_settings::rescan_all_sources,
            library_settings::get_managed_library_settings,
            library_settings::set_managed_library_settings,
            library_settings::get_external_file_settings,
            library_settings::set_external_file_settings,
            library_settings::get_path_template_presets,
            library_settings::preview_path_template,
            library_settings::pick_folder,
            library_settings::check_onboarding_needed,
            library_settings::complete_onboarding,
            library_settings::get_default_library_path,
            library_settings::get_running_scans,
            library_settings::get_latest_scan,
            // Import management
            import::import_files,
            import::import_directory,
            import::cancel_import,
            import::is_importing,
            import::get_import_config,
            import::update_import_config,
            import::get_all_sources,
            import::set_active_source,
            import::open_file_dialog,
            import::open_folder_dialog,
            import::is_directory,
            import::scan_directory_for_audio,
            // Sync/doctor
            sync::start_sync,
            sync::get_sync_status,
            sync::cancel_sync,
            sync::get_sync_errors,
            // Fingerprinting
            fingerprint::get_fingerprint_status,
            fingerprint::start_fingerprinting,
            fingerprint::stop_fingerprinting,
            fingerprint::retry_failed_fingerprints,
            fingerprint::clear_failed_fingerprints,
            fingerprint::compare_fingerprints,
            fingerprint::find_duplicates,
            // Settings
            get_user_settings,
            set_user_setting,
            get_user_setting,
            // Artwork
            get_track_artwork,
            get_album_artwork,
            get_album_artwork_with_source,
            get_artist_artwork,
            get_artist_artwork_with_source,
            get_playlist_artwork,
            get_playlist_artwork_with_source,
            set_artwork,
            remove_artwork,
            // Debug/Testing
            test_artwork_extraction,
            // Global shortcuts
            shortcuts::get_global_shortcuts,
            shortcuts::set_global_shortcut,
            shortcuts::reset_global_shortcuts,
            // Window state
            window_state_manager::save_window_state_cmd,
            window_state_manager::save_window_state_with_route,
            // Updater
            updater::check_for_updates,
            updater::install_update,
            // Loudness analysis
            loudness::get_track_loudness,
            loudness::analyze_track,
            loudness::queue_track_analysis,
            loudness::queue_all_unanalyzed,
            loudness::get_analysis_queue_stats,
            loudness::start_analysis_worker,
            loudness::stop_analysis_worker,
            loudness::get_analysis_worker_status,
            loudness::set_volume_leveling_mode,
            loudness::set_volume_leveling_preamp,
            loudness::set_volume_leveling_prevent_clipping,
            loudness::clear_completed_analysis,
            // Server sources
            sources::get_sources,
            sources::get_server_sources,
            sources::add_server_source,
            sources::remove_source,
            sources::test_server_connection,
            sources::authenticate_source,
            sources::logout_source,
            sources::get_source_auth_status,
            sources::get_active_source,
            sources::sync_from_server,
            sources::upload_to_server,
            // Playback context (Jump Back Into, Now Playing context)
            playback_context::record_playback_context,
            playback_context::get_recent_playback_contexts,
            playback_context::get_current_playback_context,
            playback_context::clear_playback_context_history,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app, _event| {
            // Handle file associations on macOS/iOS (runtime events)
            #[cfg(any(target_os = "macos", target_os = "ios"))]
            if let tauri::RunEvent::Opened { urls } = _event {
                let files = urls
                    .into_iter()
                    .filter_map(|url| url.to_file_path().ok())
                    .collect::<Vec<_>>();
                handle_file_associations(_app.clone(), files);
            }
        });
}
