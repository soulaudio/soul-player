// Prevents additional console window on Windows in release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod playback;

use playback::PlaybackManager;
use serde::{Deserialize, Serialize};
use soul_playback::{RepeatMode, ShuffleMode};
use std::path::PathBuf;
use tauri::{Manager, State};

// Types matching the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Track {
    id: i64,
    title: String,
    artist: Option<String>,
    album: Option<String>,
    duration: Option<f64>,
    file_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Album {
    id: i64,
    title: String,
    artist: Option<String>,
    year: Option<i32>,
    cover_art_path: Option<String>,
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

#[tauri::command]
async fn play_track(
    file_path: String,
    playback: State<'_, PlaybackManager>,
) -> Result<(), String> {
    let path = PathBuf::from(file_path);
    playback.play_track(path)
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
async fn get_all_tracks() -> Result<Vec<Track>, String> {
    // TODO: Integrate with soul-storage
    // Return mock data for now
    Ok(vec![
        Track {
            id: 1,
            title: "Sample Track 1".to_string(),
            artist: Some("Sample Artist".to_string()),
            album: Some("Sample Album".to_string()),
            duration: Some(240.0),
            file_path: "/path/to/track1.mp3".to_string(),
        },
        Track {
            id: 2,
            title: "Sample Track 2".to_string(),
            artist: Some("Another Artist".to_string()),
            album: Some("Another Album".to_string()),
            duration: Some(180.0),
            file_path: "/path/to/track2.mp3".to_string(),
        },
    ])
}

#[tauri::command]
async fn get_track_by_id(id: i64) -> Result<Option<Track>, String> {
    // TODO: Integrate with soul-storage
    Ok(Some(Track {
        id,
        title: format!("Track {}", id),
        artist: Some("Sample Artist".to_string()),
        album: Some("Sample Album".to_string()),
        duration: Some(200.0),
        file_path: format!("/path/to/track{}.mp3", id),
    }))
}

#[tauri::command]
async fn get_all_albums() -> Result<Vec<Album>, String> {
    // TODO: Integrate with soul-storage
    Ok(vec![
        Album {
            id: 1,
            title: "Sample Album".to_string(),
            artist: Some("Sample Artist".to_string()),
            year: Some(2024),
            cover_art_path: None,
        },
    ])
}

#[tauri::command]
async fn get_all_playlists() -> Result<Vec<Playlist>, String> {
    // TODO: Integrate with soul-storage
    Ok(vec![
        Playlist {
            id: 1,
            name: "My Playlist".to_string(),
            description: Some("A sample playlist".to_string()),
            owner_id: 1,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        },
    ])
}

#[tauri::command]
async fn create_playlist(name: String, description: Option<String>) -> Result<Playlist, String> {
    // TODO: Integrate with soul-storage
    Ok(Playlist {
        id: 1,
        name,
        description,
        owner_id: 1,
        created_at: "2024-01-01T00:00:00Z".to_string(),
        updated_at: "2024-01-01T00:00:00Z".to_string(),
    })
}

#[tauri::command]
async fn add_track_to_playlist(playlist_id: i64, track_id: i64) -> Result<(), String> {
    // TODO: Integrate with soul-storage
    println!("Adding track {} to playlist {}", track_id, playlist_id);
    Ok(())
}

#[tauri::command]
async fn scan_library(path: String) -> Result<(), String> {
    // TODO: Integrate with soul-metadata
    println!("Scanning library at: {}", path);
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Initialize playback manager
            let playback_manager =
                PlaybackManager::new(app.handle().clone()).expect("Failed to initialize playback");
            app.manage(playback_manager);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Playback control
            play_track,
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
            // Library management (TODO)
            get_all_tracks,
            get_track_by_id,
            get_all_albums,
            get_all_playlists,
            create_playlist,
            add_track_to_playlist,
            scan_library,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
