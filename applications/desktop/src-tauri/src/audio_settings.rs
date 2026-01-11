//! Audio settings management for Tauri desktop application
//!
//! This module provides commands for managing audio backends and devices,
//! integrating with the soul-audio-desktop backend/device system.

use serde::{Deserialize, Serialize};
use soul_audio_desktop::{
    backend, device, AudioBackend, AudioDeviceInfo, BackendInfo, DeviceCapabilities,
    ExclusiveConfig, LatencyInfo, SupportedBitDepth,
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
            bit_depths: caps
                .bit_depths
                .into_iter()
                .map(FrontendBitDepth::from)
                .collect(),
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
    let frontend_backends: Vec<FrontendBackendInfo> = backends
        .into_iter()
        .map(FrontendBackendInfo::from)
        .collect();

    eprintln!(
        "[audio_settings] Found {} backends",
        frontend_backends.len()
    );
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
    eprintln!(
        "[audio_settings] Getting devices for backend: {}",
        backend_str
    );

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
            d.sample_rate
                .map(|r| r.to_string())
                .unwrap_or_else(|| "?".to_string()),
            d.channels
                .map(|c| c.to_string())
                .unwrap_or_else(|| "?".to_string()),
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
            updated_at = excluded.updated_at",
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
        "SELECT value FROM user_settings WHERE user_id = ? AND key = ?",
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
        Ok(devices) => devices
            .into_iter()
            .find(|d| d.name == device_name)
            .map(|d| (Some(d.channels), d.is_default))
            .unwrap_or((None, false)),
        Err(_) => (None, false),
    };

    eprintln!(
        "[audio_settings] Current device: {} ({}) at {} Hz",
        device_name, backend_str, active_sample_rate
    );

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
pub async fn refresh_sample_rate(playback: State<'_, PlaybackManager>) -> Result<bool, String> {
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
    let caps = device::get_device_capabilities(backend, &device_name).map_err(|e| e.to_string())?;

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

// ==============================================================================
// Exclusive Mode and Latency Monitoring
// ==============================================================================

/// Frontend-compatible latency information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontendLatencyInfo {
    /// Buffer latency in samples
    pub buffer_samples: u32,
    /// Buffer latency in milliseconds
    pub buffer_ms: f32,
    /// Total estimated output latency in milliseconds (includes DAC)
    pub total_ms: f32,
    /// Whether running in exclusive mode
    pub exclusive: bool,
}

impl From<LatencyInfo> for FrontendLatencyInfo {
    fn from(info: LatencyInfo) -> Self {
        Self {
            buffer_samples: info.buffer_samples,
            buffer_ms: info.buffer_ms,
            total_ms: info.total_ms,
            exclusive: info.exclusive,
        }
    }
}

/// Frontend-compatible exclusive mode configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontendExclusiveConfig {
    /// Target sample rate (Hz) - 0 for device native
    pub sample_rate: u32,
    /// Target bit depth: "int16", "int24", "int32", "float32", "float64"
    pub bit_depth: String,
    /// Buffer size in frames (smaller = lower latency)
    pub buffer_frames: Option<u32>,
    /// Enable exclusive mode
    pub exclusive_mode: bool,
    /// Device name (None for default device)
    pub device_name: Option<String>,
    /// Backend: "default", "asio", "jack"
    pub backend: String,
}

impl Default for FrontendExclusiveConfig {
    fn default() -> Self {
        Self {
            sample_rate: 0,
            bit_depth: "float32".to_string(),
            buffer_frames: None,
            exclusive_mode: true,
            device_name: None,
            backend: "default".to_string(),
        }
    }
}

impl TryFrom<FrontendExclusiveConfig> for ExclusiveConfig {
    type Error = String;

    fn try_from(config: FrontendExclusiveConfig) -> Result<Self, Self::Error> {
        let bit_depth = match config.bit_depth.as_str() {
            "int16" => SupportedBitDepth::Int16,
            "int24" => SupportedBitDepth::Int24,
            "int32" => SupportedBitDepth::Int32,
            "float32" => SupportedBitDepth::Float32,
            "float64" => SupportedBitDepth::Float64,
            other => return Err(format!("Unknown bit depth: {}", other)),
        };

        let backend = parse_backend(&config.backend)?;

        Ok(ExclusiveConfig {
            sample_rate: config.sample_rate,
            bit_depth,
            buffer_frames: config.buffer_frames,
            exclusive_mode: config.exclusive_mode,
            device_name: config.device_name,
            backend,
        })
    }
}

/// Get current latency information from the playback system
#[tauri::command]
pub async fn get_latency_info(
    playback: State<'_, PlaybackManager>,
) -> Result<FrontendLatencyInfo, String> {
    eprintln!("[audio_settings] Getting latency info");

    let latency = playback.get_latency_info();

    let info = FrontendLatencyInfo::from(latency);
    eprintln!(
        "[audio_settings] Latency: {} samples, {:.2}ms buffer, {:.2}ms total, exclusive={}",
        info.buffer_samples, info.buffer_ms, info.total_ms, info.exclusive
    );

    Ok(info)
}

/// Set exclusive mode configuration
///
/// Switches to exclusive mode output for bit-perfect playback and lower latency
#[tauri::command]
pub async fn set_exclusive_mode(
    config: FrontendExclusiveConfig,
    playback: State<'_, PlaybackManager>,
    app_state: State<'_, AppState>,
) -> Result<FrontendLatencyInfo, String> {
    eprintln!("[audio_settings] Setting exclusive mode: {:?}", config);

    let exclusive_config: ExclusiveConfig = config.clone().try_into()?;

    // Enable exclusive mode in playback manager
    let latency = playback
        .set_exclusive_mode(exclusive_config)
        .map_err(|e| format!("Failed to set exclusive mode: {}", e))?;

    // Save settings to database
    let user_id = &app_state.user_id;
    let settings = serde_json::json!({
        "sample_rate": config.sample_rate,
        "bit_depth": config.bit_depth,
        "buffer_frames": config.buffer_frames,
        "exclusive_mode": config.exclusive_mode,
        "device_name": config.device_name,
        "backend": config.backend,
    });

    let now = chrono::Utc::now().timestamp();

    sqlx::query(
        "INSERT INTO user_settings (user_id, key, value, updated_at)
         VALUES (?, ?, ?, ?)
         ON CONFLICT(user_id, key) DO UPDATE SET
            value = excluded.value,
            updated_at = excluded.updated_at",
    )
    .bind(user_id)
    .bind("audio.exclusive_mode")
    .bind(settings.to_string())
    .bind(now)
    .execute(&*app_state.pool)
    .await
    .map_err(|e| format!("Failed to save exclusive mode setting: {}", e))?;

    eprintln!("[audio_settings] Exclusive mode configured successfully");

    Ok(FrontendLatencyInfo::from(latency))
}

/// Disable exclusive mode (return to shared mode)
#[tauri::command]
pub async fn disable_exclusive_mode(
    playback: State<'_, PlaybackManager>,
    app_state: State<'_, AppState>,
) -> Result<(), String> {
    eprintln!("[audio_settings] Disabling exclusive mode");

    playback
        .disable_exclusive_mode()
        .map_err(|e| format!("Failed to disable exclusive mode: {}", e))?;

    // Update settings in database
    let user_id = &app_state.user_id;
    let now = chrono::Utc::now().timestamp();

    sqlx::query(
        "INSERT INTO user_settings (user_id, key, value, updated_at)
         VALUES (?, ?, ?, ?)
         ON CONFLICT(user_id, key) DO UPDATE SET
            value = excluded.value,
            updated_at = excluded.updated_at",
    )
    .bind(user_id)
    .bind("audio.exclusive_mode")
    .bind("{\"exclusive_mode\": false}")
    .bind(now)
    .execute(&*app_state.pool)
    .await
    .map_err(|e| format!("Failed to save exclusive mode setting: {}", e))?;

    eprintln!("[audio_settings] Exclusive mode disabled");
    Ok(())
}

/// Check if currently in exclusive mode
#[tauri::command]
pub async fn is_exclusive_mode(playback: State<'_, PlaybackManager>) -> Result<bool, String> {
    Ok(playback.is_exclusive_mode())
}

/// Get available buffer sizes for a device
///
/// Returns common buffer sizes and whether they're supported by the device
#[tauri::command]
pub async fn get_available_buffer_sizes(
    backend_str: String,
    device_name: String,
) -> Result<Vec<BufferSizeOption>, String> {
    eprintln!(
        "[audio_settings] Getting buffer sizes for {} ({})",
        device_name, backend_str
    );

    let backend = parse_backend(&backend_str)?;
    let caps = device::get_device_capabilities(backend, &device_name).map_err(|e| e.to_string())?;

    // Standard buffer sizes in frames
    let standard_sizes = [32, 64, 128, 256, 512, 1024, 2048, 4096];

    let options: Vec<BufferSizeOption> = standard_sizes
        .iter()
        .map(|&frames| {
            let supported = match (caps.min_buffer_frames, caps.max_buffer_frames) {
                (Some(min), Some(max)) => frames >= min && frames <= max,
                _ => true, // If unknown, assume supported
            };

            // Calculate latency at common sample rates
            let latency_ms_44100 = frames as f32 / 44100.0 * 1000.0;
            let latency_ms_48000 = frames as f32 / 48000.0 * 1000.0;

            BufferSizeOption {
                frames,
                supported,
                latency_ms_44100,
                latency_ms_48000,
            }
        })
        .collect();

    eprintln!(
        "[audio_settings] Found {} buffer size options",
        options.len()
    );

    Ok(options)
}

/// Buffer size option for frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BufferSizeOption {
    /// Buffer size in frames
    pub frames: u32,
    /// Whether this size is supported by the device
    pub supported: bool,
    /// Latency in ms at 44.1kHz
    pub latency_ms_44100: f32,
    /// Latency in ms at 48kHz
    pub latency_ms_48000: f32,
}

/// Get recommended exclusive mode preset for a use case
#[tauri::command]
pub async fn get_exclusive_preset(
    preset: String,
    backend_str: Option<String>,
    device_name: Option<String>,
) -> Result<FrontendExclusiveConfig, String> {
    eprintln!("[audio_settings] Getting exclusive preset: {}", preset);

    let backend = backend_str.unwrap_or_else(|| "default".to_string());

    let config = match preset.as_str() {
        "bit_perfect_16" => FrontendExclusiveConfig {
            bit_depth: "int16".to_string(),
            exclusive_mode: true,
            backend,
            device_name,
            ..Default::default()
        },
        "bit_perfect_24" => FrontendExclusiveConfig {
            bit_depth: "int24".to_string(),
            exclusive_mode: true,
            backend,
            device_name,
            ..Default::default()
        },
        "bit_perfect_32" => FrontendExclusiveConfig {
            bit_depth: "int32".to_string(),
            exclusive_mode: true,
            backend,
            device_name,
            ..Default::default()
        },
        "low_latency" => FrontendExclusiveConfig {
            buffer_frames: Some(128),
            exclusive_mode: true,
            backend,
            device_name,
            ..Default::default()
        },
        "ultra_low_latency" => FrontendExclusiveConfig {
            buffer_frames: Some(64),
            exclusive_mode: true,
            backend,
            device_name,
            ..Default::default()
        },
        "balanced" => FrontendExclusiveConfig {
            buffer_frames: Some(256),
            exclusive_mode: true,
            backend,
            device_name,
            ..Default::default()
        },
        _ => return Err(format!("Unknown preset: {}", preset)),
    };

    Ok(config)
}

// ==============================================================================
// Crossfade Settings
// ==============================================================================

/// Set crossfade enabled/disabled
///
/// Applies immediately without requiring app restart.
/// When enabled, tracks will blend into each other during transitions.
/// When disabled, gapless playback is used.
#[tauri::command]
pub async fn set_crossfade_enabled(
    enabled: bool,
    playback: State<'_, PlaybackManager>,
) -> Result<(), String> {
    eprintln!("[audio_settings] Setting crossfade enabled: {}", enabled);
    playback.set_crossfade_enabled(enabled);
    Ok(())
}

/// Get current crossfade enabled state
#[tauri::command]
pub async fn is_crossfade_enabled(
    playback: State<'_, PlaybackManager>,
) -> Result<bool, String> {
    Ok(playback.is_crossfade_enabled())
}

/// Set crossfade duration in milliseconds
///
/// Applies immediately without requiring app restart.
/// Duration is capped at 10000ms (10 seconds).
/// A duration of 0 means gapless playback (no crossfade).
#[tauri::command]
pub async fn set_crossfade_duration(
    duration_ms: u32,
    playback: State<'_, PlaybackManager>,
) -> Result<(), String> {
    eprintln!(
        "[audio_settings] Setting crossfade duration: {}ms",
        duration_ms
    );
    playback.set_crossfade_duration(duration_ms);
    Ok(())
}

/// Get crossfade duration in milliseconds
#[tauri::command]
pub async fn get_crossfade_duration(
    playback: State<'_, PlaybackManager>,
) -> Result<u32, String> {
    Ok(playback.get_crossfade_duration())
}

/// Set crossfade curve type
///
/// Applies immediately without requiring app restart.
/// Available curves:
/// - "linear": Simple linear fade
/// - "square_root": Natural-sounding transitions
/// - "s_curve": Smooth acceleration at start/end
/// - "equal_power": Constant perceived loudness (recommended)
#[tauri::command]
pub async fn set_crossfade_curve(
    curve: String,
    playback: State<'_, PlaybackManager>,
) -> Result<(), String> {
    eprintln!("[audio_settings] Setting crossfade curve: {}", curve);

    let fade_curve = match curve.as_str() {
        "linear" => soul_playback::FadeCurve::Linear,
        "square_root" | "logarithmic" => soul_playback::FadeCurve::SquareRoot,
        "s_curve" => soul_playback::FadeCurve::SCurve,
        "equal_power" => soul_playback::FadeCurve::EqualPower,
        other => return Err(format!("Unknown crossfade curve: {}", other)),
    };

    playback.set_crossfade_curve(fade_curve);
    Ok(())
}

/// Get crossfade curve type as string
#[tauri::command]
pub async fn get_crossfade_curve(
    playback: State<'_, PlaybackManager>,
) -> Result<String, String> {
    let curve = playback.get_crossfade_curve();
    let curve_str = match curve {
        soul_playback::FadeCurve::Linear => "linear",
        soul_playback::FadeCurve::SquareRoot => "square_root",
        #[allow(deprecated)]
        soul_playback::FadeCurve::Logarithmic => "square_root",
        soul_playback::FadeCurve::SCurve => "s_curve",
        soul_playback::FadeCurve::EqualPower => "equal_power",
    };
    Ok(curve_str.to_string())
}

/// Set all crossfade settings at once
///
/// This is a convenience command that updates enabled, duration, and curve
/// in a single call, avoiding multiple round-trips. All settings apply immediately.
#[tauri::command]
pub async fn set_crossfade_settings(
    enabled: bool,
    duration_ms: u32,
    curve: String,
    playback: State<'_, PlaybackManager>,
) -> Result<(), String> {
    eprintln!(
        "[audio_settings] Setting crossfade settings: enabled={}, duration={}ms, curve={}",
        enabled, duration_ms, curve
    );

    // Set curve first
    let fade_curve = match curve.as_str() {
        "linear" => soul_playback::FadeCurve::Linear,
        "square_root" | "logarithmic" => soul_playback::FadeCurve::SquareRoot,
        "s_curve" => soul_playback::FadeCurve::SCurve,
        "equal_power" => soul_playback::FadeCurve::EqualPower,
        other => return Err(format!("Unknown crossfade curve: {}", other)),
    };

    playback.set_crossfade_curve(fade_curve);
    playback.set_crossfade_duration(duration_ms);
    playback.set_crossfade_enabled(enabled);

    eprintln!("[audio_settings] Crossfade settings applied successfully");
    Ok(())
}

/// Get all crossfade settings at once
#[tauri::command]
pub async fn get_crossfade_settings(
    playback: State<'_, PlaybackManager>,
) -> Result<CrossfadeSettingsInfo, String> {
    let enabled = playback.is_crossfade_enabled();
    let duration_ms = playback.get_crossfade_duration();
    let curve = playback.get_crossfade_curve();

    let curve_str = match curve {
        soul_playback::FadeCurve::Linear => "linear",
        soul_playback::FadeCurve::SquareRoot => "square_root",
        #[allow(deprecated)]
        soul_playback::FadeCurve::Logarithmic => "square_root",
        soul_playback::FadeCurve::SCurve => "s_curve",
        soul_playback::FadeCurve::EqualPower => "equal_power",
    };

    Ok(CrossfadeSettingsInfo {
        enabled,
        duration_ms,
        curve: curve_str.to_string(),
    })
}

/// Crossfade settings info for frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrossfadeSettingsInfo {
    /// Whether crossfade is enabled
    pub enabled: bool,
    /// Crossfade duration in milliseconds
    pub duration_ms: u32,
    /// Crossfade curve type: "linear", "square_root", "s_curve", "equal_power"
    pub curve: String,
}

// ==============================================================================
// Resampling Settings
// ==============================================================================

/// Resampling settings info for frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResamplingSettingsInfo {
    /// Quality preset: "fast", "balanced", "high", "maximum"
    pub quality: String,
    /// Target sample rate: 0 = auto (match device), or specific rate like 96000
    pub target_rate: u32,
    /// Backend: "auto", "rubato", "r8brain"
    pub backend: String,
}

/// Set resampling quality preset
///
/// Quality presets:
/// - "fast": Low CPU, 64-tap filter, good for older hardware
/// - "balanced": Moderate CPU, 128-tap filter, good quality
/// - "high": Higher CPU, 256-tap filter, excellent quality (default)
/// - "maximum": Highest CPU, 512-tap filter, audiophile quality
///
/// Note: Changes apply to newly loaded tracks. The current track will continue
/// playing with its existing resampler settings until the next track loads.
#[tauri::command]
pub async fn set_resampling_quality(
    quality: String,
    playback: State<'_, PlaybackManager>,
    app_state: State<'_, AppState>,
) -> Result<(), String> {
    eprintln!("[audio_settings] Setting resampling quality: {}", quality);

    // Validate quality value
    let valid_qualities = ["fast", "balanced", "high", "maximum"];
    if !valid_qualities.contains(&quality.as_str()) {
        return Err(format!(
            "Invalid quality '{}'. Must be one of: {}",
            quality,
            valid_qualities.join(", ")
        ));
    }

    // Apply to playback manager
    playback.set_resampling_quality(&quality)?;

    // Persist to database
    let user_id = &app_state.user_id;
    let now = chrono::Utc::now().timestamp();

    // Load existing settings
    let existing = sqlx::query_as::<_, (String,)>(
        "SELECT value FROM user_settings WHERE user_id = ? AND key = ?",
    )
    .bind(user_id)
    .bind("audio.resampling")
    .fetch_optional(&*app_state.pool)
    .await
    .map_err(|e| format!("Failed to load existing settings: {}", e))?;

    let mut settings: serde_json::Value = existing
        .and_then(|(v,)| serde_json::from_str(&v).ok())
        .unwrap_or_else(|| {
            serde_json::json!({
                "quality": "high",
                "target_rate": 0,
                "backend": "auto"
            })
        });

    settings["quality"] = serde_json::Value::String(quality.clone());

    sqlx::query(
        "INSERT INTO user_settings (user_id, key, value, updated_at)
         VALUES (?, ?, ?, ?)
         ON CONFLICT(user_id, key) DO UPDATE SET
            value = excluded.value,
            updated_at = excluded.updated_at",
    )
    .bind(user_id)
    .bind("audio.resampling")
    .bind(settings.to_string())
    .bind(now)
    .execute(&*app_state.pool)
    .await
    .map_err(|e| format!("Failed to save resampling quality: {}", e))?;

    eprintln!("[audio_settings] Resampling quality set to '{}'. Will apply to next track.", quality);
    Ok(())
}

/// Get current resampling quality preset
#[tauri::command]
pub async fn get_resampling_quality(
    playback: State<'_, PlaybackManager>,
) -> Result<String, String> {
    Ok(playback.get_resampling_quality())
}

/// Set resampling target sample rate
///
/// Arguments:
/// - rate: Target sample rate in Hz. Use 0 for "auto" (match device native rate)
///   Common values: 44100, 48000, 88200, 96000, 176400, 192000
///
/// Note: Changes apply to newly loaded tracks.
#[tauri::command]
pub async fn set_resampling_target_rate(
    rate: u32,
    playback: State<'_, PlaybackManager>,
    app_state: State<'_, AppState>,
) -> Result<(), String> {
    eprintln!("[audio_settings] Setting resampling target rate: {} (0=auto)", rate);

    // Validate rate (0 = auto, otherwise must be a reasonable sample rate)
    if rate != 0 && (rate < 8000 || rate > 384000) {
        return Err(format!(
            "Invalid target rate {}. Must be 0 (auto) or between 8000 and 384000 Hz",
            rate
        ));
    }

    // Apply to playback manager
    playback.set_resampling_target_rate(rate)?;

    // Persist to database
    let user_id = &app_state.user_id;
    let now = chrono::Utc::now().timestamp();

    // Load existing settings
    let existing = sqlx::query_as::<_, (String,)>(
        "SELECT value FROM user_settings WHERE user_id = ? AND key = ?",
    )
    .bind(user_id)
    .bind("audio.resampling")
    .fetch_optional(&*app_state.pool)
    .await
    .map_err(|e| format!("Failed to load existing settings: {}", e))?;

    let mut settings: serde_json::Value = existing
        .and_then(|(v,)| serde_json::from_str(&v).ok())
        .unwrap_or_else(|| {
            serde_json::json!({
                "quality": "high",
                "target_rate": 0,
                "backend": "auto"
            })
        });

    settings["target_rate"] = serde_json::Value::Number(rate.into());

    sqlx::query(
        "INSERT INTO user_settings (user_id, key, value, updated_at)
         VALUES (?, ?, ?, ?)
         ON CONFLICT(user_id, key) DO UPDATE SET
            value = excluded.value,
            updated_at = excluded.updated_at",
    )
    .bind(user_id)
    .bind("audio.resampling")
    .bind(settings.to_string())
    .bind(now)
    .execute(&*app_state.pool)
    .await
    .map_err(|e| format!("Failed to save resampling target rate: {}", e))?;

    eprintln!("[audio_settings] Resampling target rate set. Will apply to next track.");
    Ok(())
}

/// Get current resampling target sample rate
///
/// Returns 0 for "auto" mode, otherwise the specific target rate in Hz
#[tauri::command]
pub async fn get_resampling_target_rate(
    playback: State<'_, PlaybackManager>,
) -> Result<u32, String> {
    Ok(playback.get_resampling_target_rate())
}

/// Set resampling backend
///
/// Backends:
/// - "auto": Use r8brain if available, otherwise rubato
/// - "rubato": Fast, portable Sinc resampler (always available)
/// - "r8brain": Audiophile-grade resampler (requires r8brain feature)
///
/// Note: Changes apply to newly loaded tracks.
#[tauri::command]
pub async fn set_resampling_backend(
    backend: String,
    playback: State<'_, PlaybackManager>,
    app_state: State<'_, AppState>,
) -> Result<(), String> {
    eprintln!("[audio_settings] Setting resampling backend: {}", backend);

    // Validate backend value
    let valid_backends = ["auto", "rubato", "r8brain"];
    if !valid_backends.contains(&backend.as_str()) {
        return Err(format!(
            "Invalid backend '{}'. Must be one of: {}",
            backend,
            valid_backends.join(", ")
        ));
    }

    // Check if r8brain is available when explicitly requested
    if backend == "r8brain" {
        #[cfg(not(feature = "r8brain"))]
        {
            return Err("r8brain backend is not available in this build".to_string());
        }
    }

    // Apply to playback manager
    playback.set_resampling_backend(&backend)?;

    // Persist to database
    let user_id = &app_state.user_id;
    let now = chrono::Utc::now().timestamp();

    // Load existing settings
    let existing = sqlx::query_as::<_, (String,)>(
        "SELECT value FROM user_settings WHERE user_id = ? AND key = ?",
    )
    .bind(user_id)
    .bind("audio.resampling")
    .fetch_optional(&*app_state.pool)
    .await
    .map_err(|e| format!("Failed to load existing settings: {}", e))?;

    let mut settings: serde_json::Value = existing
        .and_then(|(v,)| serde_json::from_str(&v).ok())
        .unwrap_or_else(|| {
            serde_json::json!({
                "quality": "high",
                "target_rate": 0,
                "backend": "auto"
            })
        });

    settings["backend"] = serde_json::Value::String(backend);

    sqlx::query(
        "INSERT INTO user_settings (user_id, key, value, updated_at)
         VALUES (?, ?, ?, ?)
         ON CONFLICT(user_id, key) DO UPDATE SET
            value = excluded.value,
            updated_at = excluded.updated_at",
    )
    .bind(user_id)
    .bind("audio.resampling")
    .bind(settings.to_string())
    .bind(now)
    .execute(&*app_state.pool)
    .await
    .map_err(|e| format!("Failed to save resampling backend: {}", e))?;

    eprintln!("[audio_settings] Resampling backend set. Will apply to next track.");
    Ok(())
}

/// Get current resampling backend
#[tauri::command]
pub async fn get_resampling_backend(
    playback: State<'_, PlaybackManager>,
) -> Result<String, String> {
    Ok(playback.get_resampling_backend())
}

/// Set all resampling settings at once
///
/// This is a convenience command that updates quality, target rate, and backend
/// in a single call. All settings apply to newly loaded tracks.
#[tauri::command]
pub async fn set_resampling_settings(
    quality: String,
    target_rate: u32,
    backend: String,
    playback: State<'_, PlaybackManager>,
    app_state: State<'_, AppState>,
) -> Result<(), String> {
    eprintln!(
        "[audio_settings] Setting resampling settings: quality={}, target_rate={}, backend={}",
        quality, target_rate, backend
    );

    // Validate all values
    let valid_qualities = ["fast", "balanced", "high", "maximum"];
    if !valid_qualities.contains(&quality.as_str()) {
        return Err(format!(
            "Invalid quality '{}'. Must be one of: {}",
            quality,
            valid_qualities.join(", ")
        ));
    }

    if target_rate != 0 && (target_rate < 8000 || target_rate > 384000) {
        return Err(format!(
            "Invalid target rate {}. Must be 0 (auto) or between 8000 and 384000 Hz",
            target_rate
        ));
    }

    let valid_backends = ["auto", "rubato", "r8brain"];
    if !valid_backends.contains(&backend.as_str()) {
        return Err(format!(
            "Invalid backend '{}'. Must be one of: {}",
            backend,
            valid_backends.join(", ")
        ));
    }

    // Apply to playback manager
    playback.set_resampling_quality(&quality)?;
    playback.set_resampling_target_rate(target_rate)?;
    playback.set_resampling_backend(&backend)?;

    // Persist to database
    let user_id = &app_state.user_id;
    let settings = serde_json::json!({
        "quality": quality,
        "target_rate": target_rate,
        "backend": backend,
    });

    let now = chrono::Utc::now().timestamp();

    sqlx::query(
        "INSERT INTO user_settings (user_id, key, value, updated_at)
         VALUES (?, ?, ?, ?)
         ON CONFLICT(user_id, key) DO UPDATE SET
            value = excluded.value,
            updated_at = excluded.updated_at",
    )
    .bind(user_id)
    .bind("audio.resampling")
    .bind(settings.to_string())
    .bind(now)
    .execute(&*app_state.pool)
    .await
    .map_err(|e| format!("Failed to save resampling settings: {}", e))?;

    eprintln!("[audio_settings] Resampling settings saved. Will apply to next track.");
    Ok(())
}

/// Get all resampling settings at once
#[tauri::command]
pub async fn get_resampling_settings(
    playback: State<'_, PlaybackManager>,
) -> Result<ResamplingSettingsInfo, String> {
    Ok(ResamplingSettingsInfo {
        quality: playback.get_resampling_quality(),
        target_rate: playback.get_resampling_target_rate(),
        backend: playback.get_resampling_backend(),
    })
}

// ===== Headroom Management =====

/// Headroom mode for frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontendHeadroomMode {
    /// Mode type: "auto", "manual", or "disabled"
    pub mode: String,
    /// Manual headroom value in dB (only used when mode is "manual")
    pub manual_db: Option<f64>,
}

/// Headroom settings info for frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeadroomSettingsInfo {
    pub enabled: bool,
    pub mode: FrontendHeadroomMode,
    pub total_gain_db: f64,
    pub attenuation_db: f64,
}

/// Get headroom settings
#[tauri::command]
pub async fn get_headroom_settings(
    playback: State<'_, PlaybackManager>,
) -> Result<HeadroomSettingsInfo, String> {
    use soul_playback::HeadroomMode;

    let mode = playback.get_headroom_mode();
    let frontend_mode = match mode {
        HeadroomMode::Auto => FrontendHeadroomMode {
            mode: "auto".to_string(),
            manual_db: None,
        },
        HeadroomMode::Manual(db) => FrontendHeadroomMode {
            mode: "manual".to_string(),
            manual_db: Some(db),
        },
        HeadroomMode::Disabled => FrontendHeadroomMode {
            mode: "disabled".to_string(),
            manual_db: None,
        },
    };

    Ok(HeadroomSettingsInfo {
        enabled: playback.is_headroom_enabled(),
        mode: frontend_mode,
        total_gain_db: playback.get_headroom_total_gain_db(),
        attenuation_db: playback.get_headroom_attenuation_db(),
    })
}

/// Set headroom mode
#[tauri::command]
pub async fn set_headroom_mode(
    playback: State<'_, PlaybackManager>,
    mode: String,
    manual_db: Option<f64>,
) -> Result<(), String> {
    use soul_playback::HeadroomMode;

    let headroom_mode = match mode.as_str() {
        "auto" => HeadroomMode::Auto,
        "manual" => {
            let db = manual_db.unwrap_or(-6.0);
            HeadroomMode::Manual(db)
        }
        "disabled" => HeadroomMode::Disabled,
        _ => return Err(format!("Invalid headroom mode: {}", mode)),
    };

    playback.set_headroom_mode(headroom_mode);
    eprintln!("[audio_settings] Headroom mode set to: {:?}", mode);
    Ok(())
}

/// Enable or disable headroom management
#[tauri::command]
pub async fn set_headroom_enabled(
    playback: State<'_, PlaybackManager>,
    enabled: bool,
) -> Result<(), String> {
    playback.set_headroom_enabled(enabled);
    eprintln!("[audio_settings] Headroom enabled: {}", enabled);
    Ok(())
}

/// Set headroom EQ boost (called when EQ settings change)
#[tauri::command]
pub async fn set_headroom_eq_boost(
    playback: State<'_, PlaybackManager>,
    boost_db: f64,
) -> Result<(), String> {
    playback.set_headroom_eq_boost_db(boost_db);
    Ok(())
}

/// Set headroom preamp gain
#[tauri::command]
pub async fn set_headroom_preamp(
    playback: State<'_, PlaybackManager>,
    preamp_db: f64,
) -> Result<(), String> {
    playback.set_headroom_preamp_db(preamp_db);
    Ok(())
}
