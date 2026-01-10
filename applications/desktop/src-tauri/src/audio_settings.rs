//! Audio settings management for Tauri desktop application
//!
//! This module provides commands for managing audio backends and devices,
//! integrating with the soul-audio-desktop backend/device system.

use serde::{Deserialize, Serialize};
use soul_audio_desktop::{
    backend, device, AudioBackend, AudioDeviceInfo, BackendInfo, DeviceCapabilities,
    SupportedBitDepth,
};
use tauri::State;

use crate::app_state::AppState;
use crate::playback::PlaybackManager;

/// Frontend-compatible backend info
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontendBackendInfo {
    backend: String, // "default", "asio", "jack"
    name: String,
    description: String,
    available: bool,
    is_default: bool,
    device_count: usize,
}

impl From<BackendInfo> for FrontendBackendInfo {
    fn from(info: BackendInfo) -> Self {
        let backend_str = match info.backend {
            AudioBackend::Default => "default",
            #[cfg(target_os = "windows")]
            AudioBackend::Asio => "asio",
            #[cfg(any(target_os = "linux", target_os = "macos"))]
            AudioBackend::Jack => "jack",
        };

        Self {
            backend: backend_str.to_string(),
            name: info.name,
            description: info.description,
            available: info.available,
            is_default: info.is_default,
            device_count: info.device_count,
        }
    }
}

/// Frontend-compatible bit depth info
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontendBitDepth {
    pub format: String,
    pub bits: u8,
    pub is_integer: bool,
    pub is_float: bool,
    pub display_name: String,
}

impl From<SupportedBitDepth> for FrontendBitDepth {
    fn from(depth: SupportedBitDepth) -> Self {
        let format = match depth {
            SupportedBitDepth::Int16 => "int16",
            SupportedBitDepth::Int24 => "int24",
            SupportedBitDepth::Int32 => "int32",
            SupportedBitDepth::Float32 => "float32",
            SupportedBitDepth::Float64 => "float64",
        };

        Self {
            format: format.to_string(),
            bits: depth.bits(),
            is_integer: depth.is_integer(),
            is_float: depth.is_float(),
            display_name: depth.display_name().to_string(),
        }
    }
}

/// Frontend-compatible device capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontendDeviceCapabilities {
    pub sample_rates: Vec<u32>,
    pub bit_depths: Vec<FrontendBitDepth>,
    pub max_channels: u16,
    pub supports_exclusive: bool,
    pub supports_dsd: bool,
    pub dsd_rates: Vec<DsdRateInfo>,
    pub min_buffer_frames: Option<u32>,
    pub max_buffer_frames: Option<u32>,
    pub has_hardware_volume: bool,
}

/// DSD rate information for frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DsdRateInfo {
    pub rate: u32,
    pub name: String,
}

impl From<DeviceCapabilities> for FrontendDeviceCapabilities {
    fn from(caps: DeviceCapabilities) -> Self {
        let dsd_rates: Vec<DsdRateInfo> = caps
            .dsd_rates
            .iter()
            .map(|&rate| {
                let name = soul_audio_desktop::DSD_RATES
                    .iter()
                    .find(|(r, _)| *r == rate)
                    .map(|(_, n)| n.to_string())
                    .unwrap_or_else(|| format!("DSD{}", rate / 44100));
                DsdRateInfo { rate, name }
            })
            .collect();

        Self {
            sample_rates: caps.sample_rates,
            bit_depths: caps.bit_depths.into_iter().map(FrontendBitDepth::from).collect(),
            max_channels: caps.max_channels,
            supports_exclusive: caps.supports_exclusive,
            supports_dsd: caps.supports_dsd,
            dsd_rates,
            min_buffer_frames: caps.min_buffer_frames,
            max_buffer_frames: caps.max_buffer_frames,
            has_hardware_volume: caps.has_hardware_volume,
        }
    }
}

/// Frontend-compatible device info
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontendDeviceInfo {
    name: String,
    backend: String,
    is_default: bool,
    sample_rate: Option<u32>,
    channels: Option<u16>,
    sample_rate_range: Option<(u32, u32)>,
    is_running: bool,
    capabilities: Option<FrontendDeviceCapabilities>,
}

impl From<AudioDeviceInfo> for FrontendDeviceInfo {
    fn from(info: AudioDeviceInfo) -> Self {
        let backend_str = match info.backend {
            AudioBackend::Default => "default",
            #[cfg(target_os = "windows")]
            AudioBackend::Asio => "asio",
            #[cfg(any(target_os = "linux", target_os = "macos"))]
            AudioBackend::Jack => "jack",
        };

        Self {
            name: info.name,
            backend: backend_str.to_string(),
            is_default: info.is_default,
            sample_rate: Some(info.sample_rate),
            channels: Some(info.channels),
            sample_rate_range: info.sample_rate_range,
            is_running: false, // Device list items are not necessarily running
            capabilities: info.capabilities.map(FrontendDeviceCapabilities::from),
        }
    }
}

/// Parse backend string to AudioBackend enum
fn parse_backend(backend_str: &str) -> Result<AudioBackend, String> {
    match backend_str {
        "default" => Ok(AudioBackend::Default),
        #[cfg(target_os = "windows")]
        "asio" => Ok(AudioBackend::Asio),
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        "jack" => Ok(AudioBackend::Jack),
        _ => Err(format!("Unknown backend: {}", backend_str)),
    }
}

// ==============================================================================
// Tauri Commands
// ==============================================================================

/// Get all available audio backends
#[tauri::command]
pub async fn get_audio_backends() -> Result<Vec<FrontendBackendInfo>, String> {
    eprintln!("[audio_settings] Getting available backends");

    let backends = backend::get_backend_info();
    let frontend_backends: Vec<FrontendBackendInfo> =
        backends.into_iter().map(FrontendBackendInfo::from).collect();

    eprintln!("[audio_settings] Found {} backends", frontend_backends.len());
    for b in &frontend_backends {
        eprintln!(
            "  - {} ({}): available={}, devices={}",
            b.name, b.backend, b.available, b.device_count
        );
    }

    Ok(frontend_backends)
}

/// Get all audio devices for a specific backend
#[tauri::command]
pub async fn get_audio_devices(backend_str: String) -> Result<Vec<FrontendDeviceInfo>, String> {
    eprintln!("[audio_settings] Getting devices for backend: {}", backend_str);

    let backend = parse_backend(&backend_str)?;
    let devices = device::list_devices(backend).map_err(|e| e.to_string())?;

    let frontend_devices: Vec<FrontendDeviceInfo> =
        devices.into_iter().map(FrontendDeviceInfo::from).collect();

    eprintln!(
        "[audio_settings] Found {} devices for {}",
        frontend_devices.len(),
        backend_str
    );
    for d in &frontend_devices {
        eprintln!(
            "  - {}: {}Hz, {}ch{}",
            d.name,
            d.sample_rate.map(|r| r.to_string()).unwrap_or_else(|| "?".to_string()),
            d.channels.map(|c| c.to_string()).unwrap_or_else(|| "?".to_string()),
            if d.is_default { " [DEFAULT]" } else { "" }
        );
    }

    Ok(frontend_devices)
}


/// Set the audio output device
///
/// This will switch the audio device during playback if possible.
/// Note: Brief pause (~200-500ms) may occur during switch.
#[tauri::command]
pub async fn set_audio_device(
    backend_str: String,
    device_name: String,
    playback_manager: State<'_, PlaybackManager>,
    app_state: State<'_, AppState>,
) -> Result<(), String> {
    eprintln!(
        "[audio_settings] Setting audio device: backend={}, device={}",
        backend_str, device_name
    );

    let backend = parse_backend(&backend_str)?;

    // Verify device exists
    let _device = device::find_device_by_name(backend, &device_name).map_err(|e| e.to_string())?;
    eprintln!("[audio_settings] Found device: {}", device_name);

    // Switch the playback device
    playback_manager
        .switch_device(backend, Some(device_name.clone()))
        .map_err(|e| format!("Failed to switch device: {}", e))?;

    eprintln!("[audio_settings] Device switched successfully");

    // Save to settings for persistence
    let user_id = &app_state.user_id;
    let settings = serde_json::json!({
        "backend": backend_str,
        "device_name": device_name,
    });

    let now = chrono::Utc::now().timestamp();

    sqlx::query(
        "INSERT INTO user_settings (user_id, key, value, updated_at)
         VALUES (?, ?, ?, ?)
         ON CONFLICT(user_id, key) DO UPDATE SET
            value = excluded.value,
            updated_at = excluded.updated_at"
    )
        .bind(user_id)
        .bind("audio.output_device")
        .bind(settings.to_string())
        .bind(now)
        .execute(&*app_state.pool)
        .await
        .map_err(|e| format!("Failed to save device setting: {}", e))?;

    eprintln!("[audio_settings] Device setting saved to database");

    Ok(())
}

/// Initialize audio device from saved settings
///
/// Called on app startup to restore the previously selected device
pub async fn initialize_audio_device(
    playback: &PlaybackManager,
    app_state: &AppState,
) -> Result<(), String> {
    eprintln!("[audio_settings] Initializing audio device from settings...");

    // Try to load saved device setting
    let saved_setting = sqlx::query_as::<_, (String,)>(
        "SELECT value FROM user_settings WHERE user_id = ? AND key = ?"
    )
        .bind(&app_state.user_id)
        .bind("audio.output_device")
        .fetch_optional(&*app_state.pool)
        .await
        .map_err(|e| format!("Failed to load device setting: {}", e))?;

    if let Some((value,)) = saved_setting {
        // Parse saved settings
        let settings: serde_json::Value = serde_json::from_str(&value)
            .map_err(|e| format!("Failed to parse device settings: {}", e))?;

        if let (Some(backend_str), Some(device_name)) = (
            settings.get("backend").and_then(|v| v.as_str()),
            settings.get("device_name").and_then(|v| v.as_str()),
        ) {
            let backend = parse_backend(backend_str)?;
            let device_name = if device_name.is_empty() {
                None
            } else {
                Some(device_name.to_string())
            };

            eprintln!(
                "[audio_settings] Restoring device: backend={:?}, device={:?}",
                backend, device_name
            );

            // Switch to saved device
            playback
                .switch_device(backend, device_name)
                .map_err(|e| format!("Failed to restore device: {}", e))?;

            eprintln!("[audio_settings] Device restored successfully");
            return Ok(());
        }
    }

    eprintln!("[audio_settings] No saved device found, using default");
    Ok(())
}

/// Get current audio device
#[tauri::command]
pub async fn get_current_audio_device(
    playback: State<'_, PlaybackManager>,
) -> Result<FrontendDeviceInfo, String> {
    eprintln!("[audio_settings] Getting current audio device from playback manager");

    let backend = playback.get_current_backend();
    let device_name = playback.get_current_device();
    // Get the actual sample rate from the playback system (what we're outputting at)
    let active_sample_rate = playback.get_current_sample_rate();

    let backend_str = match backend {
        AudioBackend::Default => "default",
        #[cfg(target_os = "windows")]
        AudioBackend::Asio => "asio",
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        AudioBackend::Jack => "jack",
    };

    // Try to get device info by listing all devices and finding the matching one
    let (channels, is_default) = match device::list_devices(backend) {
        Ok(devices) => {
            devices.into_iter()
                .find(|d| d.name == device_name)
                .map(|d| (Some(d.channels), d.is_default))
                .unwrap_or((None, false))
        }
        Err(_) => (None, false),
    };

    eprintln!("[audio_settings] Current device: {} ({}) at {} Hz", device_name, backend_str, active_sample_rate);

    Ok(FrontendDeviceInfo {
        name: device_name,
        backend: backend_str.to_string(),
        is_default,
        sample_rate: Some(active_sample_rate),
        channels,
        sample_rate_range: None,
        is_running: true,
        capabilities: None,
    })
}

/// Refresh sample rate - checks if device sample rate changed and updates if needed
///
/// This is useful when the user knows they've changed device settings
/// (e.g., via ASIO control panel) and wants to immediately update.
#[tauri::command]
pub async fn refresh_sample_rate(
    playback: State<'_, PlaybackManager>,
) -> Result<bool, String> {
    eprintln!("[audio_settings] Refreshing sample rate...");
    let result = playback.refresh_sample_rate()?;
    if result {
        eprintln!("[audio_settings] Sample rate changed, stream recreated");
    } else {
        eprintln!("[audio_settings] Sample rate unchanged");
    }
    Ok(result)
}

/// Check if r8brain resampling backend is available
///
/// Returns true if the application was compiled with r8brain support
#[tauri::command]
pub async fn is_r8brain_available() -> Result<bool, String> {
    #[cfg(feature = "r8brain")]
    {
        Ok(true)
    }
    #[cfg(not(feature = "r8brain"))]
    {
        Ok(false)
    }
}

/// Get detailed capabilities for a specific audio device
///
/// Returns sample rates, bit depths, DSD support, exclusive mode support, etc.
#[tauri::command]
pub async fn get_device_capabilities(
    backend_str: String,
    device_name: String,
) -> Result<FrontendDeviceCapabilities, String> {
    eprintln!(
        "[audio_settings] Getting capabilities for device: {} ({})",
        device_name, backend_str
    );

    let backend = parse_backend(&backend_str)?;
    let caps =
        device::get_device_capabilities(backend, &device_name).map_err(|e| e.to_string())?;

    let frontend_caps = FrontendDeviceCapabilities::from(caps);

    eprintln!(
        "[audio_settings] Device capabilities: {} sample rates, {} bit depths, DSD={}",
        frontend_caps.sample_rates.len(),
        frontend_caps.bit_depths.len(),
        frontend_caps.supports_dsd
    );

    Ok(frontend_caps)
}

/// Get all audio devices for a specific backend with full capabilities
///
/// This is a more expensive call than get_audio_devices() as it queries
/// detailed capabilities for each device.
#[tauri::command]
pub async fn get_audio_devices_with_capabilities(
    backend_str: String,
) -> Result<Vec<FrontendDeviceInfo>, String> {
    eprintln!(
        "[audio_settings] Getting devices with capabilities for backend: {}",
        backend_str
    );

    let backend = parse_backend(&backend_str)?;
    let devices =
        device::list_devices_with_capabilities(backend, true).map_err(|e| e.to_string())?;

    let frontend_devices: Vec<FrontendDeviceInfo> =
        devices.into_iter().map(FrontendDeviceInfo::from).collect();

    eprintln!(
        "[audio_settings] Found {} devices with capabilities",
        frontend_devices.len()
    );

    Ok(frontend_devices)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_backend_default() {
        let backend = parse_backend("default").unwrap();
        assert_eq!(backend, AudioBackend::Default);
    }

    #[test]
    fn test_parse_backend_invalid() {
        let result = parse_backend("invalid");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown backend"));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_parse_backend_asio() {
        let backend = parse_backend("asio").unwrap();
        assert_eq!(backend, AudioBackend::Asio);
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[test]
    fn test_parse_backend_jack() {
        let backend = parse_backend("jack").unwrap();
        assert_eq!(backend, AudioBackend::Jack);
    }

    #[tokio::test]
    async fn test_get_backends() {
        let backends = get_audio_backends().await.unwrap();
        assert!(!backends.is_empty(), "Should have at least one backend");
        assert!(
            backends.iter().any(|b| b.backend == "default"),
            "Should have default backend"
        );
    }

    #[tokio::test]
    async fn test_get_devices_default_backend() {
        let devices = get_audio_devices("default".to_string()).await.unwrap();
        assert!(!devices.is_empty(), "Should have at least one device");
        assert!(
            devices.iter().any(|d| d.is_default),
            "Should have a default device"
        );
    }

    #[tokio::test]
    async fn test_get_devices_invalid_backend() {
        let result = get_audio_devices("invalid".to_string()).await;
        assert!(result.is_err(), "Should fail with invalid backend");
    }

    // Note: test_get_current_device requires PlaybackManager state which isn't available in unit tests
    // This would need to be an integration test with proper Tauri state setup
}
