// soul-audio-desktop/src/backend.rs
//
// Audio backend selection and management for multi-driver support
// (WASAPI, ASIO, JACK, CoreAudio, ALSA)

use cpal::traits::HostTrait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Audio backend / driver selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AudioBackend {
    /// System default backend (WASAPI on Windows, CoreAudio on macOS, ALSA on Linux)
    Default,

    /// ASIO (Windows only) - Ultra-low latency, exclusive mode, professional audio
    #[cfg(all(target_os = "windows", feature = "asio"))]
    Asio,

    /// JACK Audio Connection Kit - Professional routing, low-latency
    #[cfg(feature = "jack")]
    Jack,
}

impl AudioBackend {
    /// Get human-readable name of backend
    pub fn name(&self) -> &'static str {
        match self {
            Self::Default => {
                #[cfg(target_os = "windows")]
                return "WASAPI";

                #[cfg(target_os = "macos")]
                return "CoreAudio";

                #[cfg(target_os = "linux")]
                return "ALSA";

                #[cfg(not(any(
                    target_os = "windows",
                    target_os = "macos",
                    target_os = "linux"
                )))]
                return "Default";
            }

            #[cfg(all(target_os = "windows", feature = "asio"))]
            Self::Asio => "ASIO",

            #[cfg(feature = "jack")]
            Self::Jack => "JACK",
        }
    }

    /// Get detailed description of backend
    pub fn description(&self) -> &'static str {
        match self {
            Self::Default => {
                #[cfg(target_os = "windows")]
                return "Windows Audio Session API (shared mode, multi-application)";

                #[cfg(target_os = "macos")]
                return "macOS Core Audio (native, low-latency)";

                #[cfg(target_os = "linux")]
                return "Advanced Linux Sound Architecture (direct hardware access)";

                #[cfg(not(any(
                    target_os = "windows",
                    target_os = "macos",
                    target_os = "linux"
                )))]
                return "System default audio backend";
            }

            #[cfg(all(target_os = "windows", feature = "asio"))]
            Self::Asio => "Ultra-low latency (exclusive mode, professional audio interfaces)",

            #[cfg(feature = "jack")]
            Self::Jack => "Professional audio routing (cross-application, low-latency)",
        }
    }

    /// Convert backend to CPAL host
    pub fn to_cpal_host(&self) -> Result<cpal::Host, BackendError> {
        match self {
            Self::Default => Ok(cpal::default_host()),

            #[cfg(all(target_os = "windows", feature = "asio"))]
            Self::Asio => cpal::host_from_id(cpal::HostId::Asio)
                .map_err(|_| BackendError::BackendUnavailable(self.name())),

            #[cfg(feature = "jack")]
            Self::Jack => {
                // Find JACK in available hosts
                let host_id = cpal::available_hosts()
                    .into_iter()
                    .find(|id| matches!(id, cpal::HostId::Jack))
                    .ok_or_else(|| BackendError::BackendUnavailable(self.name()))?;

                cpal::host_from_id(host_id)
                    .map_err(|_| BackendError::BackendUnavailable(self.name()))
            }
        }
    }

    /// Check if backend is available on current system
    pub fn is_available(&self) -> bool {
        self.to_cpal_host().is_ok()
    }
}

/// Information about an audio backend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendInfo {
    /// Backend type
    pub backend: AudioBackend,

    /// Human-readable name
    pub name: String,

    /// Description
    pub description: String,

    /// Is this backend available?
    pub available: bool,

    /// Is this the system default backend?
    pub is_default: bool,

    /// Number of output devices available on this backend
    pub device_count: usize,
}

/// List all available audio backends on current platform
pub fn list_available_backends() -> Vec<AudioBackend> {
    #[allow(unused_mut)]
    let mut backends = vec![AudioBackend::Default];

    #[cfg(all(target_os = "windows", feature = "asio"))]
    {
        if AudioBackend::Asio.is_available() {
            backends.push(AudioBackend::Asio);
        }
    }

    #[cfg(feature = "jack")]
    {
        if AudioBackend::Jack.is_available() {
            backends.push(AudioBackend::Jack);
        }
    }

    backends
}

/// Get detailed information about all backends
pub fn get_backend_info() -> Vec<BackendInfo> {
    let default_host_name = AudioBackend::Default.name();

    let all_backends = vec![
        AudioBackend::Default,
        #[cfg(all(target_os = "windows", feature = "asio"))]
        AudioBackend::Asio,
        #[cfg(feature = "jack")]
        AudioBackend::Jack,
    ];

    all_backends
        .into_iter()
        .map(|backend| {
            let available = backend.is_available();

            // Count devices for the default backend only during backend enumeration.
            // ASIO and JACK device enumeration can be problematic (some drivers crash
            // when enumerated multiple times or concurrently), so we defer device
            // counting for those backends to when devices are explicitly requested.
            let device_count = if available {
                match backend {
                    AudioBackend::Default => backend
                        .to_cpal_host()
                        .ok()
                        .and_then(|host| host.output_devices().ok())
                        .map(|devices| devices.count())
                        .unwrap_or(0),
                    // For ASIO/JACK, just indicate availability without counting
                    // to avoid driver crashes from repeated enumeration
                    #[cfg(all(target_os = "windows", feature = "asio"))]
                    AudioBackend::Asio => 1, // Indicate at least one device exists
                    #[cfg(feature = "jack")]
                    AudioBackend::Jack => 1, // Indicate at least one device exists
                }
            } else {
                0
            };

            BackendInfo {
                name: backend.name().to_string(),
                description: backend.description().to_string(),
                is_default: backend.name() == default_host_name,
                available,
                device_count,
                backend,
            }
        })
        .collect()
}

/// Backend-related errors
#[derive(Debug, Error)]
pub enum BackendError {
    /// Backend not available on this system
    #[error("Audio backend '{0}' is not available on this system")]
    BackendUnavailable(&'static str),

    /// No backends available
    #[error("No audio backends available")]
    NoBackendsAvailable,

    /// CPAL error
    #[error("CPAL error: {0}")]
    CpalError(String),
}

impl From<cpal::HostUnavailable> for BackendError {
    fn from(err: cpal::HostUnavailable) -> Self {
        BackendError::CpalError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_backend_available() {
        let backend = AudioBackend::Default;
        assert!(
            backend.is_available(),
            "Default backend should always be available"
        );
    }

    #[test]
    fn test_backend_name() {
        let backend = AudioBackend::Default;
        let name = backend.name();
        assert!(!name.is_empty(), "Backend name should not be empty");
    }

    #[test]
    fn test_backend_description() {
        let backend = AudioBackend::Default;
        let desc = backend.description();
        assert!(!desc.is_empty(), "Backend description should not be empty");
    }

    #[test]
    fn test_list_available_backends() {
        let backends = list_available_backends();
        assert!(
            !backends.is_empty(),
            "At least one backend should be available"
        );
        assert!(
            backends.contains(&AudioBackend::Default),
            "Default backend should always be in list"
        );
    }

    #[test]
    fn test_get_backend_info() {
        let info = get_backend_info();
        assert!(!info.is_empty(), "Should return backend info");

        // At least one backend should be available
        assert!(
            info.iter().any(|b| b.available),
            "At least one backend should be available"
        );

        // Default backend should be marked as default
        let default_info = info.iter().find(|b| b.backend == AudioBackend::Default);
        assert!(default_info.is_some(), "Default backend should be in list");
        assert!(
            default_info.unwrap().is_default,
            "Default backend should be marked as default"
        );
    }

    #[test]
    fn test_to_cpal_host() {
        let backend = AudioBackend::Default;
        let host = backend.to_cpal_host();
        assert!(
            host.is_ok(),
            "Should be able to get CPAL host for default backend"
        );
    }

    #[cfg(all(target_os = "windows", feature = "asio"))]
    #[test]
    fn test_asio_backend() {
        let backend = AudioBackend::Asio;
        assert_eq!(backend.name(), "ASIO");
        assert!(!backend.description().is_empty());

        // ASIO might not be available (depends on drivers), but should not panic
        let _ = backend.is_available();
    }

    #[cfg(feature = "jack")]
    #[test]
    fn test_jack_backend() {
        let backend = AudioBackend::Jack;
        assert_eq!(backend.name(), "JACK");
        assert!(!backend.description().is_empty());

        // JACK might not be available (depends on installation), but should not panic
        let _ = backend.is_available();
    }
}
