// soul-audio-desktop/src/device.rs
//
// Audio device enumeration and management

use cpal::traits::{DeviceTrait, HostTrait};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::backend::{AudioBackend, BackendError};

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
}

/// Enumerate all output devices for a specific backend
pub fn list_devices(backend: AudioBackend) -> Result<Vec<AudioDeviceInfo>, DeviceError> {
    let host = backend
        .to_cpal_host()
        .map_err(|_| DeviceError::BackendUnavailable(backend.name()))?;

    let default_device = host.default_output_device();
    let default_name = default_device.as_ref().and_then(|d| d.name().ok());

    let devices = host
        .devices()
        .map_err(|e| DeviceError::EnumerationFailed(e.to_string()))?;

    let mut device_list = Vec::new();

    for device in devices {
        if let Ok(name) = device.name() {
            if let Ok(config) = device.default_output_config() {
                let sample_rate = config.sample_rate().0;
                let channels = config.channels();

                // Get sample rate range if available
                let sample_rate_range = device
                    .supported_output_configs()
                    .ok()
                    .and_then(|mut configs| {
                        configs.next().map(|config| {
                            (config.min_sample_rate().0, config.max_sample_rate().0)
                        })
                    });

                device_list.push(AudioDeviceInfo {
                    name: name.clone(),
                    backend,
                    is_default: Some(name.clone()) == default_name,
                    sample_rate,
                    channels,
                    sample_rate_range,
                });
            }
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

    let sample_rate = config.sample_rate().0;
    let channels = config.channels();

    let sample_rate_range = device
        .supported_output_configs()
        .ok()
        .and_then(|mut configs| {
            configs.next().map(|config| {
                (config.min_sample_rate().0, config.max_sample_rate().0)
            })
        });

    Ok(AudioDeviceInfo {
        name,
        backend,
        is_default: true,
        sample_rate,
        channels,
        sample_rate_range,
    })
}

/// Find a device by name within a backend
pub fn find_device_by_name(
    backend: AudioBackend,
    device_name: &str,
) -> Result<cpal::Device, DeviceError> {
    let host = backend
        .to_cpal_host()
        .map_err(|_| DeviceError::BackendUnavailable(backend.name()))?;

    let devices = host
        .devices()
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
}
