// soul-audio-desktop/src/backend.rs
//
// Audio backend selection and management for multi-driver support
// (WASAPI, ASIO, JACK, CoreAudio, ALSA)

#[cfg(not(target_os = "windows"))]
use cpal::traits::HostTrait;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;

/// Audio backend / driver selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AudioBackend {
    /// System default backend (WASAPI on Windows, `CoreAudio` on macOS, ALSA on Linux)
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
    ///
    /// On Windows, **never** calls any CPAL host operations (not even `cpal::default_host()`).
    ///
    /// Both `cpal::host_from_id(Asio)` and `cpal::default_host()` (WASAPI) can trigger
    /// unhandled Windows SEH exceptions when ASIO drivers are installed — especially when
    /// called concurrently from `spawn_blocking` tasks while the main thread initialises
    /// audio playback. SEH exceptions bypass Rust's panic machinery and kill the process.
    ///
    /// On Windows the availability is inferred statically:
    /// - `Default` (WASAPI) is always available on Windows.
    /// - `Asio` availability is inferred from `cpal::available_hosts()`, which only
    ///   checks whether the ASIO host was compiled in — it does not initialise any drivers.
    pub fn is_available(&self) -> bool {
        #[cfg(target_os = "windows")]
        {
            match self {
                Self::Default => true, // WASAPI is always available on Windows
                #[cfg(feature = "asio")]
                Self::Asio => {
                    // Safe: available_hosts() is a compile-time check, no driver init.
                    cpal::available_hosts()
                        .into_iter()
                        .any(|id| id == cpal::HostId::Asio)
                }
                #[allow(unreachable_patterns)]
                _ => false,
            }
        }

        #[cfg(not(target_os = "windows"))]
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

            // Count devices for backend info display.
            // On Windows, avoid calling host.output_devices() for ANY backend:
            // - ASIO/JACK: can crash via SEH exceptions on some driver setups
            // - Default (WASAPI): can also trigger SEH with ASIO drivers installed,
            //   because WASAPI enumeration sometimes initializes COM objects that
            //   conflict with the ASIO runtime, causing unhandled exceptions.
            // Instead, we use a placeholder count of 1 on Windows and let the
            // actual device listing (via WinRT) provide the real device names.
            let device_count = if available {
                match backend {
                    #[cfg(target_os = "windows")]
                    AudioBackend::Default => 1, // Avoid CPAL WASAPI enumeration (SEH-prone on Windows with ASIO drivers)
                    #[cfg(not(target_os = "windows"))]
                    AudioBackend::Default => backend
                        .to_cpal_host()
                        .ok()
                        .and_then(|host| host.output_devices().ok())
                        .map(|devices| devices.count())
                        .unwrap_or(0),
                    // For ASIO/JACK, just indicate availability without counting
                    // to avoid driver crashes from repeated enumeration
                    #[cfg(all(target_os = "windows", feature = "asio"))]
                    AudioBackend::Asio => 1,
                    #[cfg(feature = "jack")]
                    AudioBackend::Jack => 1,
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

// ==============================================================================
// Async Timeout Wrappers
// ==============================================================================

/// Default timeout for backend enumeration (5 seconds)
pub const BACKEND_ENUM_TIMEOUT_SECS: u64 = 5;

/// Get backend info with timeout protection
///
/// This is the async version of `get_backend_info()` that wraps the call in a
/// timeout to prevent indefinite hangs during backend enumeration.
///
/// # Errors
/// - `BackendError::EnumerationTimeout` if enumeration takes longer than 5 seconds
/// - `BackendError::TaskJoinError` if the background task panics
pub async fn get_backend_info_async() -> Result<Vec<BackendInfo>, BackendError> {
    get_backend_info_with_timeout(BACKEND_ENUM_TIMEOUT_SECS).await
}

/// Get backend info with custom timeout
///
/// Same as `get_backend_info_async()` but allows specifying a custom timeout duration.
pub async fn get_backend_info_with_timeout(
    timeout_secs: u64,
) -> Result<Vec<BackendInfo>, BackendError> {
    tracing::debug!(
        timeout_secs,
        "[Backend] Starting async backend enumeration with timeout"
    );

    let timeout_duration = Duration::from_secs(timeout_secs);

    match tokio::time::timeout(
        timeout_duration,
        tokio::task::spawn_blocking(get_backend_info),
    )
    .await
    {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(join_err)) => {
            tracing::error!(
                error = %join_err,
                "[Backend] Backend enumeration task panicked or failed to join"
            );
            Err(BackendError::TaskJoinError(join_err.to_string()))
        }
        Err(_timeout_err) => {
            tracing::error!(
                timeout_secs,
                "[Backend] Backend enumeration timed out - audio service may be hung"
            );
            Err(BackendError::EnumerationTimeout(timeout_secs))
        }
    }
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

    /// Backend enumeration timed out
    #[error("Backend enumeration timed out after {0} seconds. The audio service may be hung. Try restarting the audio service or rebooting your system.")]
    EnumerationTimeout(u64),

    /// Task join error during async enumeration
    #[error("Failed to join backend enumeration task: {0}")]
    TaskJoinError(String),
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
    #[ignore = "Requires real audio hardware - not available in CI environments"]
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
    #[ignore = "Requires real audio hardware - not available in CI environments"]
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
    #[ignore = "Requires real audio hardware - not available in CI environments"]
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
    #[ignore = "Requires real audio hardware - not available in CI environments"]
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
