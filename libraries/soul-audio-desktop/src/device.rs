// soul-audio-desktop/src/device.rs
//
// Audio device enumeration and management

use cpal::traits::{DeviceTrait, HostTrait};
use cpal::SampleFormat;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::backend::{AudioBackend, BackendError};

/// Sample format / bit depth capabilities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SupportedBitDepth {
    /// 16-bit integer
    Int16,
    /// 24-bit integer (packed in 32-bit container)
    Int24,
    /// 32-bit integer
    Int32,
    /// 32-bit float
    Float32,
    /// 64-bit float
    Float64,
}

impl SupportedBitDepth {
    /// Get the bit depth as a number
    pub fn bits(&self) -> u8 {
        match self {
            Self::Int16 => 16,
            Self::Int24 => 24,
            Self::Int32 => 32,
            Self::Float32 => 32,
            Self::Float64 => 64,
        }
    }

    /// Check if this is an integer format
    pub fn is_integer(&self) -> bool {
        matches!(self, Self::Int16 | Self::Int24 | Self::Int32)
    }

    /// Check if this is a floating-point format
    pub fn is_float(&self) -> bool {
        matches!(self, Self::Float32 | Self::Float64)
    }

    /// Get human-readable name
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Int16 => "16-bit",
            Self::Int24 => "24-bit",
            Self::Int32 => "32-bit",
            Self::Float32 => "32-bit float",
            Self::Float64 => "64-bit float",
        }
    }

    /// Convert from CPAL SampleFormat
    fn from_cpal(format: SampleFormat) -> Option<Self> {
        match format {
            SampleFormat::I16 => Some(Self::Int16),
            SampleFormat::I32 => Some(Self::Int32),
            SampleFormat::F32 => Some(Self::Float32),
            SampleFormat::F64 => Some(Self::Float64),
            SampleFormat::U8 | SampleFormat::U16 | SampleFormat::U32 | SampleFormat::U64 => None,
            SampleFormat::I8 | SampleFormat::I64 => None,
            _ => None,
        }
    }
}

/// Detailed DAC/Device capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCapabilities {
    /// All supported sample rates (Hz)
    pub sample_rates: Vec<u32>,

    /// Supported bit depths / sample formats
    pub bit_depths: Vec<SupportedBitDepth>,

    /// Maximum number of output channels
    pub max_channels: u16,

    /// Supports exclusive mode (WASAPI exclusive, ASIO, etc.)
    pub supports_exclusive: bool,

    /// Supports DSD output (native DoP or DSD-over-USB)
    pub supports_dsd: bool,

    /// DSD rates supported (64x, 128x, 256x, 512x) - multiples of 44.1kHz
    pub dsd_rates: Vec<u32>,

    /// Minimum buffer size in frames (if known)
    pub min_buffer_frames: Option<u32>,

    /// Maximum buffer size in frames (if known)
    pub max_buffer_frames: Option<u32>,

    /// Supports hardware volume control
    pub has_hardware_volume: bool,
}

impl Default for DeviceCapabilities {
    fn default() -> Self {
        Self {
            sample_rates: vec![44100, 48000],
            bit_depths: vec![SupportedBitDepth::Float32],
            max_channels: 2,
            supports_exclusive: false,
            supports_dsd: false,
            dsd_rates: Vec::new(),
            min_buffer_frames: None,
            max_buffer_frames: None,
            has_hardware_volume: false,
        }
    }
}

/// Standard sample rates for audio devices
pub const STANDARD_SAMPLE_RATES: &[u32] = &[
    44100,  // CD quality
    48000,  // DAT / DVD
    88200,  // 2x CD
    96000,  // DVD-Audio / high-res
    176400, // 4x CD
    192000, // High-res audio
    352800, // DSD64 (DoP)
    384000, // 8x 48kHz
    705600, // DSD128 (DoP)
    768000, // 16x 48kHz
];

/// DSD rates as sample rate equivalents
pub const DSD_RATES: &[(u32, &str)] = &[
    (2822400, "DSD64"),   // 64x 44.1kHz
    (5644800, "DSD128"),  // 128x 44.1kHz
    (11289600, "DSD256"), // 256x 44.1kHz
    (22579200, "DSD512"), // 512x 44.1kHz
];

/// Information about an audio output device
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioDeviceInfo {
    /// Device name (human-readable)
    pub name: String,

    /// Backend/host ID this device belongs to
    pub backend: AudioBackend,

    /// Is this the system default device for this backend?
    pub is_default: bool,

    /// Native sample rate (Hz)
    pub sample_rate: u32,

    /// Number of output channels
    pub channels: u16,

    /// Supported sample rates (min, max)
    pub sample_rate_range: Option<(u32, u32)>,

    /// Detailed device capabilities (optional, may require additional queries)
    #[serde(default)]
    pub capabilities: Option<DeviceCapabilities>,
}

/// Detect detailed capabilities for a CPAL device
pub fn detect_device_capabilities(device: &cpal::Device, backend: AudioBackend) -> DeviceCapabilities {
    use std::collections::HashSet;

    let mut sample_rates = HashSet::new();
    let mut bit_depths = HashSet::new();
    let mut max_channels: u16 = 2;

    // Query all supported output configurations
    if let Ok(configs) = device.supported_output_configs() {
        for config in configs {
            // Extract sample format / bit depth
            if let Some(depth) = SupportedBitDepth::from_cpal(config.sample_format()) {
                bit_depths.insert(depth);
            }

            // Track max channels
            max_channels = max_channels.max(config.channels());

            // Get min/max sample rate for this config
            let min_rate = config.min_sample_rate();
            let max_rate = config.max_sample_rate();

            // Add standard rates within the supported range
            for &rate in STANDARD_SAMPLE_RATES {
                if rate >= min_rate && rate <= max_rate {
                    sample_rates.insert(rate);
                }
            }

            // Always include the min and max if they're valid
            if min_rate >= 8000 {
                sample_rates.insert(min_rate);
            }
            if max_rate <= 768000 {
                sample_rates.insert(max_rate);
            }
        }
    }

    // Convert to sorted vectors
    let mut sample_rates: Vec<u32> = sample_rates.into_iter().collect();
    sample_rates.sort();

    let mut bit_depths: Vec<SupportedBitDepth> = bit_depths.into_iter().collect();
    bit_depths.sort_by_key(|d| d.bits());

    // Check for DSD support (indicated by very high sample rates)
    let supports_dsd = sample_rates.iter().any(|&r| r >= 352800);
    let dsd_rates: Vec<u32> = if supports_dsd {
        // Check which DSD rates are actually supported
        DSD_RATES
            .iter()
            .filter_map(|&(rate, _)| {
                // DSD over PCM (DoP) uses 176.4kHz for DSD64, 352.8kHz for DSD128, etc.
                // The actual sample rate needed is half the DSD rate
                let pcm_rate = rate / 16; // DoP frame rate
                if sample_rates.iter().any(|&r| r >= pcm_rate) {
                    Some(rate)
                } else {
                    None
                }
            })
            .collect()
    } else {
        Vec::new()
    };

    // Detect exclusive mode support based on backend
    let supports_exclusive = matches!(
        backend,
        AudioBackend::Default // WASAPI can do exclusive mode
    ) || {
        #[cfg(all(target_os = "windows", feature = "asio"))]
        { matches!(backend, AudioBackend::Asio) }
        #[cfg(not(all(target_os = "windows", feature = "asio")))]
        { false }
    };

    DeviceCapabilities {
        sample_rates,
        bit_depths,
        max_channels,
        supports_exclusive,
        supports_dsd,
        dsd_rates,
        min_buffer_frames: None, // Would require backend-specific queries
        max_buffer_frames: None,
        has_hardware_volume: false, // Would require backend-specific queries
    }
}

/// Enumerate all output devices for a specific backend
pub fn list_devices(backend: AudioBackend) -> Result<Vec<AudioDeviceInfo>, DeviceError> {
    list_devices_with_capabilities(backend, false)
}

/// Enumerate all output devices with optional capability detection
///
/// If `include_capabilities` is true, detailed DAC capabilities will be queried
/// for each device. This is more expensive but provides complete information.
pub fn list_devices_with_capabilities(
    backend: AudioBackend,
    include_capabilities: bool,
) -> Result<Vec<AudioDeviceInfo>, DeviceError> {
    let host = backend
        .to_cpal_host()
        .map_err(|_| DeviceError::BackendUnavailable(backend.name()))?;

    let default_device = host.default_output_device();
    let default_name = default_device.as_ref().and_then(|d| d.name().ok());

    // Use output_devices() instead of devices() to only enumerate output devices.
    // This is more efficient and avoids issues with input-only devices.
    let devices = host
        .output_devices()
        .map_err(|e| DeviceError::EnumerationFailed(e.to_string()))?;

    let mut device_list = Vec::new();

    for device in devices {
        // Wrap device info extraction in individual error handling to prevent
        // one problematic device from failing the entire enumeration.
        // This is especially important for ASIO where some drivers are unreliable.
        let device_info = (|| -> Option<AudioDeviceInfo> {
            let name = device.name().ok()?;

            // Try to get default output config; skip device if unavailable
            let config = device.default_output_config().ok()?;
            let sample_rate = config.sample_rate();
            let channels = config.channels();

            // Get sample rate range if available (non-critical, so don't fail if missing)
            let sample_rate_range = device
                .supported_output_configs()
                .ok()
                .and_then(|mut configs| {
                    configs.next().map(|config| {
                        (config.min_sample_rate(), config.max_sample_rate())
                    })
                });

            // Optionally detect full capabilities
            let capabilities = if include_capabilities {
                Some(detect_device_capabilities(&device, backend))
            } else {
                None
            };

            Some(AudioDeviceInfo {
                name: name.clone(),
                backend,
                is_default: Some(name.clone()) == default_name,
                sample_rate,
                channels,
                sample_rate_range,
                capabilities,
            })
        })();

        if let Some(info) = device_info {
            device_list.push(info);
        }
    }

    // Sort: default first, then alphabetically
    device_list.sort_by(|a, b| {
        match (a.is_default, b.is_default) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name),
        }
    });

    Ok(device_list)
}

/// Get information about the default output device for a backend
pub fn get_default_device(backend: AudioBackend) -> Result<AudioDeviceInfo, DeviceError> {
    get_default_device_with_capabilities(backend, false)
}

/// Get information about the default output device with optional capability detection
pub fn get_default_device_with_capabilities(
    backend: AudioBackend,
    include_capabilities: bool,
) -> Result<AudioDeviceInfo, DeviceError> {
    let host = backend
        .to_cpal_host()
        .map_err(|_| DeviceError::BackendUnavailable(backend.name()))?;

    let device = host
        .default_output_device()
        .ok_or(DeviceError::NoDeviceFound)?;

    let name = device
        .name()
        .map_err(|e| DeviceError::DeviceInfoFailed(e.to_string()))?;

    let config = device
        .default_output_config()
        .map_err(|e| DeviceError::DeviceInfoFailed(e.to_string()))?;

    let sample_rate = config.sample_rate();
    let channels = config.channels();

    let sample_rate_range = device
        .supported_output_configs()
        .ok()
        .and_then(|mut configs| {
            configs.next().map(|config| {
                (config.min_sample_rate(), config.max_sample_rate())
            })
        });

    let capabilities = if include_capabilities {
        Some(detect_device_capabilities(&device, backend))
    } else {
        None
    };

    Ok(AudioDeviceInfo {
        name,
        backend,
        is_default: true,
        sample_rate,
        channels,
        sample_rate_range,
        capabilities,
    })
}

/// Get detailed capabilities for a device by name
pub fn get_device_capabilities(
    backend: AudioBackend,
    device_name: &str,
) -> Result<DeviceCapabilities, DeviceError> {
    let device = find_device_by_name(backend, device_name)?;
    Ok(detect_device_capabilities(&device, backend))
}

/// Find a device by name within a backend
pub fn find_device_by_name(
    backend: AudioBackend,
    device_name: &str,
) -> Result<cpal::Device, DeviceError> {
    let host = backend
        .to_cpal_host()
        .map_err(|_| DeviceError::BackendUnavailable(backend.name()))?;

    // Use output_devices() to only search among output devices
    let devices = host
        .output_devices()
        .map_err(|e| DeviceError::EnumerationFailed(e.to_string()))?;

    for device in devices {
        if let Ok(name) = device.name() {
            if name == device_name {
                return Ok(device);
            }
        }
    }

    Err(DeviceError::DeviceNotFound(device_name.to_string()))
}

/// Device-related errors
#[derive(Debug, Error)]
pub enum DeviceError {
    /// Backend not available
    #[error("Audio backend '{0}' is not available")]
    BackendUnavailable(&'static str),

    /// Failed to enumerate devices
    #[error("Failed to enumerate audio devices: {0}")]
    EnumerationFailed(String),

    /// No devices found
    #[error("No audio output devices found")]
    NoDeviceFound,

    /// Device with specified name not found
    #[error("Audio device '{0}' not found")]
    DeviceNotFound(String),

    /// Failed to get device information
    #[error("Failed to get device information: {0}")]
    DeviceInfoFailed(String),
}

impl From<BackendError> for DeviceError {
    fn from(err: BackendError) -> Self {
        match err {
            BackendError::BackendUnavailable(name) => DeviceError::BackendUnavailable(name),
            _ => DeviceError::EnumerationFailed(err.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_devices_default_backend() {
        let backend = AudioBackend::Default;
        let devices = list_devices(backend);

        // Should succeed on any system with audio
        assert!(
            devices.is_ok(),
            "Should be able to list devices for default backend"
        );

        let devices = devices.unwrap();
        assert!(!devices.is_empty(), "Should find at least one audio device");

        // At least one device should be default
        assert!(
            devices.iter().any(|d| d.is_default),
            "At least one device should be marked as default"
        );

        // All devices should have valid sample rates
        for device in &devices {
            assert!(device.sample_rate > 0, "Sample rate should be positive");
            assert!(device.channels > 0, "Channel count should be positive");
        }
    }

    #[test]
    fn test_get_default_device() {
        let backend = AudioBackend::Default;
        let device = get_default_device(backend);

        assert!(device.is_ok(), "Should be able to get default device");

        let device = device.unwrap();
        assert!(device.is_default, "Device should be marked as default");
        assert!(!device.name.is_empty(), "Device should have a name");
        assert!(device.sample_rate > 0, "Device should have valid sample rate");
    }

    #[test]
    fn test_find_device_by_name() {
        let backend = AudioBackend::Default;
        let devices = list_devices(backend).unwrap();

        if let Some(first_device) = devices.first() {
            let found = find_device_by_name(backend, &first_device.name);
            assert!(
                found.is_ok(),
                "Should be able to find device by name: {}",
                first_device.name
            );
        }
    }

    #[test]
    fn test_find_nonexistent_device() {
        let backend = AudioBackend::Default;
        let result = find_device_by_name(backend, "Nonexistent Device 12345");

        assert!(result.is_err(), "Should fail to find nonexistent device");
        assert!(
            matches!(result, Err(DeviceError::DeviceNotFound(_))),
            "Should return DeviceNotFound error"
        );
    }

    #[test]
    fn test_device_sorting() {
        let backend = AudioBackend::Default;
        let devices = list_devices(backend).unwrap();

        if devices.len() > 1 {
            // Default device should be first
            assert!(
                devices[0].is_default,
                "First device in list should be the default"
            );

            // Remaining devices should be alphabetically sorted
            for i in 1..devices.len() {
                if !devices[i].is_default {
                    assert!(
                        devices[i - 1].name <= devices[i].name || devices[i - 1].is_default,
                        "Devices should be sorted alphabetically after default"
                    );
                }
            }
        }
    }

    #[cfg(all(target_os = "windows", feature = "asio"))]
    #[test]
    fn test_asio_devices() {
        let backend = AudioBackend::Asio;

        if backend.is_available() {
            let devices = list_devices(backend);
            // ASIO might not have devices if no ASIO drivers installed
            // Just check it doesn't panic
            let _ = devices;
        }
    }

    #[cfg(feature = "jack")]
    #[test]
    fn test_jack_devices() {
        let backend = AudioBackend::Jack;

        if backend.is_available() {
            let devices = list_devices(backend);
            // JACK might not be running, just check it doesn't panic
            let _ = devices;
        }
    }

    #[test]
    fn test_supported_bit_depth() {
        // Test bit depth values
        assert_eq!(SupportedBitDepth::Int16.bits(), 16);
        assert_eq!(SupportedBitDepth::Int24.bits(), 24);
        assert_eq!(SupportedBitDepth::Int32.bits(), 32);
        assert_eq!(SupportedBitDepth::Float32.bits(), 32);
        assert_eq!(SupportedBitDepth::Float64.bits(), 64);

        // Test integer/float classification
        assert!(SupportedBitDepth::Int16.is_integer());
        assert!(SupportedBitDepth::Int24.is_integer());
        assert!(SupportedBitDepth::Int32.is_integer());
        assert!(!SupportedBitDepth::Float32.is_integer());
        assert!(!SupportedBitDepth::Float64.is_integer());

        assert!(!SupportedBitDepth::Int16.is_float());
        assert!(SupportedBitDepth::Float32.is_float());
        assert!(SupportedBitDepth::Float64.is_float());

        // Test display names
        assert_eq!(SupportedBitDepth::Int16.display_name(), "16-bit");
        assert_eq!(SupportedBitDepth::Int24.display_name(), "24-bit");
        assert_eq!(SupportedBitDepth::Float32.display_name(), "32-bit float");
    }

    #[test]
    fn test_device_capabilities_default() {
        let caps = DeviceCapabilities::default();

        assert!(!caps.sample_rates.is_empty(), "Should have default sample rates");
        assert!(!caps.bit_depths.is_empty(), "Should have default bit depths");
        assert_eq!(caps.max_channels, 2, "Default should be stereo");
        assert!(!caps.supports_dsd, "Default should not support DSD");
        assert!(caps.dsd_rates.is_empty(), "Default should have no DSD rates");
    }

    #[test]
    fn test_list_devices_with_capabilities() {
        let backend = AudioBackend::Default;
        let devices = list_devices_with_capabilities(backend, true);

        assert!(devices.is_ok(), "Should be able to list devices with capabilities");

        let devices = devices.unwrap();
        if !devices.is_empty() {
            // At least one device should have capabilities when requested
            let device = &devices[0];
            assert!(
                device.capabilities.is_some(),
                "Device should have capabilities when requested"
            );

            let caps = device.capabilities.as_ref().unwrap();
            assert!(!caps.sample_rates.is_empty(), "Should detect sample rates");
            assert!(!caps.bit_depths.is_empty(), "Should detect bit depths");
            assert!(caps.max_channels > 0, "Should detect channels");
        }
    }

    #[test]
    fn test_get_default_device_with_capabilities() {
        let backend = AudioBackend::Default;
        let device = get_default_device_with_capabilities(backend, true);

        assert!(device.is_ok(), "Should be able to get default device with capabilities");

        let device = device.unwrap();
        assert!(device.capabilities.is_some(), "Should have capabilities");

        let caps = device.capabilities.as_ref().unwrap();
        assert!(!caps.sample_rates.is_empty(), "Should detect sample rates");

        // Sample rates should be sorted
        for i in 1..caps.sample_rates.len() {
            assert!(
                caps.sample_rates[i - 1] <= caps.sample_rates[i],
                "Sample rates should be sorted"
            );
        }
    }

    #[test]
    fn test_get_device_capabilities_by_name() {
        let backend = AudioBackend::Default;
        let devices = list_devices(backend).unwrap();

        if let Some(first_device) = devices.first() {
            let caps = get_device_capabilities(backend, &first_device.name);
            assert!(caps.is_ok(), "Should be able to get capabilities by name");

            let caps = caps.unwrap();
            assert!(!caps.sample_rates.is_empty(), "Should detect sample rates");
            assert!(!caps.bit_depths.is_empty(), "Should detect bit depths");
        }
    }

    #[test]
    fn test_standard_sample_rates() {
        // Verify standard sample rates are defined correctly
        assert!(STANDARD_SAMPLE_RATES.contains(&44100), "Should include CD quality");
        assert!(STANDARD_SAMPLE_RATES.contains(&48000), "Should include DAT quality");
        assert!(STANDARD_SAMPLE_RATES.contains(&96000), "Should include high-res");
        assert!(STANDARD_SAMPLE_RATES.contains(&192000), "Should include high-res 192k");

        // Should be sorted
        for i in 1..STANDARD_SAMPLE_RATES.len() {
            assert!(
                STANDARD_SAMPLE_RATES[i - 1] < STANDARD_SAMPLE_RATES[i],
                "Standard rates should be sorted"
            );
        }
    }

    #[test]
    fn test_dsd_rates() {
        // Verify DSD rates
        assert_eq!(DSD_RATES.len(), 4, "Should have 4 DSD rate levels");

        // Check DSD64
        assert!(DSD_RATES.iter().any(|(r, n)| *r == 2822400 && *n == "DSD64"));
        // Check DSD128
        assert!(DSD_RATES.iter().any(|(r, n)| *r == 5644800 && *n == "DSD128"));
        // Check DSD256
        assert!(DSD_RATES.iter().any(|(r, n)| *r == 11289600 && *n == "DSD256"));
        // Check DSD512
        assert!(DSD_RATES.iter().any(|(r, n)| *r == 22579200 && *n == "DSD512"));
    }
}
